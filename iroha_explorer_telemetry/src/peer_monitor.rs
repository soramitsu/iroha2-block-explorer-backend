use crate::{AverageBlockTime, ConfigGetDTO, BLOCK_TIME_AVG_WINDOW};

use eyre::eyre;
use http::StatusCode;
use iroha::{client::Status, data_model::prelude::*};

use iroha_explorer_schema::{GeoLocation, ToriiUrl};
use reqwest::Client;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tokio::time::{Instant, MissedTickBehavior};
use tracing::{info_span, instrument, Instrument};
use url::Url;

const GET_STATUS_INTERVAL: Duration = Duration::from_secs(5);
const GET_STATUS_DISCONNECT_TIMEOUT: Duration = Duration::from_secs(60);
const GET_PEERS_INTERVAL: Duration = Duration::from_secs(60);
const TELEMETRY_UNSUPPORTED_CHECK_INTERVAL: Duration = Duration::from_secs(300);
const GET_GEO_RETRY_INTERVAL: Duration = Duration::from_secs(60);
const GET_CONFIG_INIT_INTERVAL: Duration = Duration::from_secs(15);
const GET_CONFIG_MAX_INTERVAL: Duration = Duration::from_secs(120);
const GET_CONFIG_INTERVAL_MULTIPLIER: f64 = 1.67;

#[instrument(fields(%torii_url))]
pub fn run(torii_url: ToriiUrl) -> (mpsc::Receiver<Update>, impl Future<Output = ()> + Sized) {
    let (tx, rx) = mpsc::channel(128);
    let url = Arc::new(torii_url);

    let url2 = url.clone();
    let fut = async move {
        let mut set = JoinSet::new();

        set.spawn({
            let tx = tx.clone();
            let url3 = url2.clone();
            async move {
                let geo = match collect_geo(&url3).await {
                    Ok(x) => x,
                    Err(err) => {
                        tracing::error!(%err, "Failed to collect geo, quitting");
                        return;
                    }
                };
                tracing::debug!(?geo, "Collected peer geo data");
                let _: Result<_, _> = tx.send(Update::Geo(geo)).await;
            }
            .instrument(info_span!("geo_future", torii_url = %url2))
        });

        set.spawn(
            {
                let url3 = url2.clone();
                async move {
                    loop {
                        let cfg = get_config_with_retry(&url3).await;
                        tracing::debug!(?cfg, "Peer connected");
                        let _ = tx.send(Update::Connected(cfg)).await;

                        let (status_fin_tx, status_fin_rx) = oneshot::channel();
                        let mut set = JoinSet::new();
                        set.spawn({
                            let tx = tx.clone();
                            let url = url3.clone();
                            async move {
                                get_peers_periodic(&url, tx).await;
                                unreachable!("should never stop")
                            }
                        });
                        set.spawn({
                            let tx = tx.clone();
                            let url = url3.clone();
                            async move {
                                get_metrics_periodic_timeout(&url, tx).await;
                                let _: Result<_, _> = status_fin_tx.send(());
                            }
                        });
                        status_fin_rx.await.expect("sender should not be dropped");
                        tracing::warn!(
                            "Peer stopped responding with status, marking as disconnected"
                        );
                        let _ = tx.send(Update::Disconnected).await;
                    }
                }
            }
            .instrument(info_span!("peer_future", torii_url = %url2)),
        );

        while set.join_next().await.is_some() {}
        unreachable!("loop should never end");
    };

    (rx, fut)
}

async fn collect_geo(torii_url: &ToriiUrl) -> eyre::Result<GeoLocation> {
    #[derive(Deserialize, Debug)]
    #[serde(tag = "status", rename_all = "lowercase")]
    enum IpApiComResponse {
        Success(GeoLocation),
        Fail { message: String },
    }

    #[derive(thiserror::Error, Debug)]
    enum RequestError {
        #[error("Request to ip-api.com failed: {0:?}")]
        Http(#[from] reqwest::Error),
        #[error("Request to ip-api.com failed with message: {message}")]
        FailResponse { message: String },
    }

    let client = Client::new();
    let url = construct_ip_api_com_query(torii_url)?;
    let do_request = || {
        let client = client.clone();
        let url = url.clone();
        async move {
            let response: IpApiComResponse = client.get(url).send().await?.json().await?;
            match response {
                IpApiComResponse::Success(data) => Ok(data),
                IpApiComResponse::Fail { message } => Err(RequestError::FailResponse { message }),
            }
        }
    };

    let data = backoff::future::retry(
        backoff::ExponentialBackoffBuilder::new()
            .with_initial_interval(GET_GEO_RETRY_INTERVAL)
            .with_multiplier(1.0)
            .with_max_elapsed_time(None)
            .build(),
        || async {
            match do_request().await {
                Ok(data) => Ok(data),
                Err(RequestError::Http(err)) => {
                    tracing::warn!(%err, "Failed to get geo location (http error, will retry)");
                    Err(backoff::Error::transient(eyre!("{err}")))
                }
                Err(RequestError::FailResponse { message }) => {
                    tracing::error!(response = %message, "Failed to get geo location (service error, will not retry)");
                    Err(backoff::Error::permanent(eyre!("Got fail response")))
                }
            }
        },
    )
    .await?;

    Ok(data)
}

fn construct_ip_api_com_query(torii_url: &ToriiUrl) -> Result<Url, eyre::Error> {
    let Some(host) = torii_url.0.host_str() else {
        return Err(eyre!("Torii URL does not have host"));
    };
    let mut url = Url::parse("http://ip-api.com/json").expect("valid");
    url.path_segments_mut().expect("url has a base").push(host);
    url.query_pairs_mut()
        .append_pair("fields", "status,message,lat,lon,country,city");
    Ok(url)
}

async fn get_config_with_retry(torii_url: &ToriiUrl) -> ConfigGetDTO {
    let client = Client::new();
    let url = torii_url.0.join("/configuration").expect("valid url");

    let do_request = || {
        let client = client.clone();
        let url = url.clone();
        async move {
            let config: ConfigGetDTO = client.get(url).send().await?.json().await?;
            Ok::<_, reqwest::Error>(config)
        }
    };

    backoff::future::retry(
        backoff::ExponentialBackoffBuilder::new()
            .with_initial_interval(GET_CONFIG_INIT_INTERVAL)
            .with_max_interval(GET_CONFIG_MAX_INTERVAL)
            .with_multiplier(GET_CONFIG_INTERVAL_MULTIPLIER)
            .with_max_elapsed_time(None)
            .build(),
        || async {
            tracing::debug!("Trying to get configuration");
            match do_request().await {
                Ok(x) => Ok(x),
                Err(err) => {
                    tracing::warn!(?err, "Failed to get configuration");
                    Err(err)?
                }
            }
        },
    )
    .await
    .expect("there is no retry limit")
}

#[instrument(skip(tx), fields(%torii_url))]
async fn get_peers_periodic(torii_url: &ToriiUrl, tx: mpsc::Sender<Update>) -> NoReturn {
    let client = Client::new();
    let url = torii_url.0.join("/peers").expect("valid url");
    let get = || async {
        let peers: Vec<Peer> = client.get(url.clone()).send().await?.json().await?;
        Ok::<_, reqwest::Error>(peers)
    };

    let mut interval = tokio::time::interval(GET_PEERS_INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tracing::trace!("Collecting peers");
        match get().await {
            Ok(peers) => {
                tracing::trace!(?peers, "Collected peers from peer");
                let _ = tx
                    .send(Update::Peers(
                        peers
                            .into_iter()
                            .map(|x| x.id().public_key().clone())
                            .collect(),
                    ))
                    .await;
            }
            Err(err) => {
                tracing::warn!(?err, "Failed to get peers from peer");
            }
        }
        interval.tick().await;
    }
}

enum NoReturn {}

#[instrument(skip(tx), fields(%torii_url))]
async fn get_metrics_periodic_timeout(torii_url: &ToriiUrl, tx: mpsc::Sender<Update>) {
    #[derive(thiserror::Error, Debug)]
    enum GetError {
        #[error("http error: {0}")]
        Http(#[from] reqwest::Error),
        #[error("telemetry is not available")]
        NotImplemented,
    }

    let mut avg_commit_time = AverageBlockTime::<BLOCK_TIME_AVG_WINDOW>::new();
    let client = Client::new();
    let url = torii_url.0.join("/status").expect("valid url");
    let get = || async {
        let resp = client.get(url.clone()).send().await?;
        if resp.status() == StatusCode::NOT_IMPLEMENTED {
            return Err(GetError::NotImplemented);
        }
        let status: Status = resp.json().await?;
        Ok(status)
    };

    let mut interval = tokio::time::interval(GET_STATUS_INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut count_failure_from: Instant = Instant::now();

    loop {
        tracing::trace!("Collecting status");
        match get().await {
            Ok(status) => {
                tracing::trace!(?status, "Collected status");
                count_failure_from = Instant::now();
                let block_commit_time = Duration::from_millis(status.commit_time_ms);
                avg_commit_time.observe(status.blocks, block_commit_time);
                let metrics = Metrics {
                    // peers: status.peers as u32,
                    block: status.blocks as u32,
                    block_commit_time,
                    avg_commit_time: avg_commit_time.calculate().expect("BUG: just updated"),
                    queue_size: status.queue_size as u32,
                    uptime: status.uptime.0,
                };
                let _ = tx.send(Update::Metrics(metrics)).await;
            }
            Err(GetError::Http(err)) => {
                tracing::warn!(?err, "Failed to get status");
                let elapsed = Instant::now() - count_failure_from;
                if elapsed >= GET_STATUS_DISCONNECT_TIMEOUT {
                    tracing::warn!(%url, "Peer does not respond for too long, terminate status checks");
                    // disconnected message is sent externally
                    break;
                }
            }
            Err(GetError::NotImplemented) => {
                tracing::info!("Peer does not implement telemetry");
                let _ = tx.send(Update::TelemetryUnsupported).await;
                tokio::time::sleep(TELEMETRY_UNSUPPORTED_CHECK_INTERVAL).await;
            }
        }
        interval.tick().await;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    // pub peers: u32,
    pub block: u32,
    pub block_commit_time: Duration,
    pub avg_commit_time: Duration,
    pub queue_size: u32,
    pub uptime: Duration,
}

#[derive(Clone, Debug)]
pub enum Update {
    Connected(ConfigGetDTO),
    Disconnected,
    TelemetryUnsupported,
    Metrics(Metrics),
    Geo(GeoLocation),
    Peers(BTreeSet<PublicKey>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::spawn;

    #[test]
    fn construct_ip_api_com_urls() {
        let result = construct_ip_api_com_query(&"http://iroha.tech/".parse().unwrap()).unwrap();
        assert_eq!(
            result.to_string(),
            "http://ip-api.com/json/iroha.tech?fields=status%2Cmessage%2Clat%2Clon%2Ccountry%2Ccity"
        );

        let result =
            construct_ip_api_com_query(&"https://fujiwara.sora.org/v5".parse().unwrap()).unwrap();
        assert_eq!(
            result.to_string(),
            "http://ip-api.com/json/fujiwara.sora.org?fields=status%2Cmessage%2Clat%2Clon%2Ccountry%2Ccity"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn debug_monitor() {
        // crate::init_test_logger();
        let (mut rx, fut) = run("http://localhost:8080".parse().unwrap());
        spawn(fut);

        while let Some(msg) = rx.recv().await {
            tracing::info!(?msg, "Message from monitor");
        }
    }
}
