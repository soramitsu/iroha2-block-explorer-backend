pub mod blockchain;
mod peer_monitor;

use crate::schema::{
    GeoLocation, NetworkStatus, PeerInfo, PeerStatus, TelemetryStreamMessage, ToriiUrl,
};
use async_stream::stream;
use circular_buffer::CircularBuffer;
use futures_util::stream::Stream;
use iroha::client::ConfigGetDTO;
use iroha::crypto::PublicKey;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;

pub struct TelemetryConfig {
    /// List of Torii URLs to gather metrics from
    pub urls: BTreeSet<ToriiUrl>,
}

#[derive(Clone)]
pub struct Telemetry {
    actor: mpsc::Sender<ActorMessage>,
}

impl Telemetry {
    pub fn start(config: TelemetryConfig) -> (Self, impl Future<Output = ()> + Sized) {
        let (actor, handle) = mpsc::channel(512);

        #[cfg(debug_assertions)]
        if config.urls.is_empty() {
            tracing::warn!("No URLs provided to gather telemetry from")
        }

        let actor_clone = actor.clone();
        let fut = async move {
            let mut set = JoinSet::new();

            set.spawn({
                let urls = config.urls.clone();
                async move {
                    TelemetryActor::new(urls, handle).run().await;
                    tracing::debug!("Actor terminated");
                }
            });

            for url in config.urls {
                let actor = actor_clone.clone();
                let (mut recv, monitor_fut) = peer_monitor::run(url.clone());
                set.spawn(monitor_fut);
                set.spawn(async move {
                    while let Some(message) = recv.recv().await {
                        if let Err(err) = actor
                            .send(ActorMessage::HandlePeerMonitorUpdate(url.clone(), message))
                            .await
                        {
                            tracing::error!(?err, "Actor is down");
                            break;
                        };
                    }
                });
            }

            loop {
                match set.join_next().await {
                    Some(Ok(())) => {}
                    Some(Err(err)) => {
                        tracing::error!(?err, "Join error, aborting");
                        panic!("not a recoverable error");
                    }
                    None => break,
                }
            }
        };

        (Self { actor }, fut)
    }

    pub async fn network(&self) -> eyre::Result<Option<NetworkStatus>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.actor
            .send(ActorMessage::GetNetworkStatus { reply: reply_tx })
            .await?;
        let reply = reply_rx.await?;
        Ok(reply)
    }

    pub async fn peers(&self) -> eyre::Result<Vec<PeerStatus>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.actor
            .send(ActorMessage::GetPeersStatus { reply: reply_tx })
            .await?;
        let reply = reply_rx.await?;
        Ok(reply)
    }

    pub async fn peers_info(&self) -> eyre::Result<Vec<PeerInfo>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.actor
            .send(ActorMessage::GetPeersInfo { reply: reply_tx })
            .await?;
        let reply = reply_rx.await?;
        Ok(reply)
    }

    pub async fn live(&self) -> eyre::Result<impl Stream<Item = TelemetryStreamMessage>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.actor
            .send(ActorMessage::Stream { reply: reply_tx })
            .await?;
        let mut rx = reply_rx.await?;
        let stream = stream! {
            loop {
                match rx.recv().await {
                    Ok(data) => yield data,
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::error!("telemetry stopped streaming metrics, which is abnormal");
                        break
                    },
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(
                            skipped_messages=skipped,
                            "peers metrics stream lagged too far behind; \
                             client might result in an inconsistent state; \
                             continuing streaming nonetheless"
                        );
                    },

                }
            }
        };
        Ok(stream)
    }

    pub async fn single_peer(&self, url: &ToriiUrl) -> eyre::Result<Option<PeerStatus>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.actor
            .send(ActorMessage::GetSinglePeerStatus {
                url: url.clone(),
                reply: reply_tx,
            })
            .await?;
        let reply = reply_rx.await?;
        Ok(reply)
    }

    pub async fn update_blockchain_state(&self, state: blockchain::State) -> eyre::Result<()> {
        self.actor
            .send(ActorMessage::UpdateBlockchainState(state))
            .await?;
        Ok(())
    }
}

enum ActorMessage {
    HandlePeerMonitorUpdate(ToriiUrl, peer_monitor::Update),
    UpdateBlockchainState(blockchain::State),
    GetNetworkStatus {
        reply: oneshot::Sender<Option<NetworkStatus>>,
    },
    GetPeersInfo {
        reply: oneshot::Sender<Vec<PeerInfo>>,
    },
    GetPeersStatus {
        reply: oneshot::Sender<Vec<PeerStatus>>,
    },
    GetSinglePeerStatus {
        url: ToriiUrl,
        reply: oneshot::Sender<Option<PeerStatus>>,
    },
    Stream {
        reply: oneshot::Sender<broadcast::Receiver<TelemetryStreamMessage>>,
    },
}

struct TelemetryActor {
    state: State,
    handle: mpsc::Receiver<ActorMessage>,
    live: broadcast::Sender<TelemetryStreamMessage>,
}

impl TelemetryActor {
    fn new(peers: BTreeSet<ToriiUrl>, handle: mpsc::Receiver<ActorMessage>) -> Self {
        const CAPACITY: usize = 512;
        let state = State::new(peers);
        let (live_tx, _rx) = broadcast::channel(CAPACITY);
        Self {
            state,
            handle,
            live: live_tx,
        }
    }

    async fn run(mut self) {
        while let Some(message) = self.handle.recv().await {
            match message {
                ActorMessage::GetNetworkStatus { reply } => {
                    let _: Result<_, _> = reply.send(self.state.network_status());
                }
                ActorMessage::GetPeersStatus { reply } => {
                    let _: Result<_, _> = reply.send(self.state.peers_status().collect());
                }
                ActorMessage::GetPeersInfo { reply } => {
                    let _: Result<_, _> = reply.send(self.state.peers_info().collect());
                }
                ActorMessage::GetSinglePeerStatus { url, reply } => {
                    let _: Result<_, _> = reply.send(self.state.single_peer_status(&url));
                }
                ActorMessage::Stream { reply } => {
                    let rx = self.live.subscribe();
                    let _: Result<_, _> = reply.send(rx);
                }
                ActorMessage::UpdateBlockchainState(state) => {
                    self.state.update_blockchain(state);
                    let updated_status = self
                        .state
                        .network_status()
                        .expect("must exist after update_blockchain() call");
                    tracing::trace!("Broadcast live update of network status");
                    let _: Result<_, _> = self
                        .live
                        .send(TelemetryStreamMessage::NetworkStatus(updated_status));
                }
                ActorMessage::HandlePeerMonitorUpdate(url, update) => {
                    tracing::trace!(%url, ?update, "Peer update");
                    match self.state.update_peer(&url, update) {
                        Ok(segment) => {
                            tracing::trace!(?segment, "Broadcast live update of peer");
                            let _: Result<_, _> = self.live.send(segment.into());
                        }
                        Err(PeerNotFound) => {
                            tracing::error!(%url, "Received peer update for an unknown peer");
                        }
                    }
                }
            }
        }
        tracing::debug!("Telemetry actor handle dropped, exiting");
    }
}

struct State {
    blockchain: Option<blockchain::State>,
    peers: BTreeMap<ToriiUrl, PeerState>,
}

impl State {
    fn new(peers: BTreeSet<ToriiUrl>) -> Self {
        let peers = peers
            .into_iter()
            .map(|url| (url.clone(), PeerState::new(url)))
            .collect();

        Self {
            blockchain: None,
            peers,
        }
    }

    /// Compute based on the supermajority rule - the block height by ⅔ nodes of the network
    /// (see
    /// [Fault Tolerance](https://hyperledger-iroha.github.io/iroha-2-docs/get-started/iroha-2.html#fault-tolerance)
    /// article).
    fn finalized_block(&self) -> Option<u32> {
        let mut blocks: Vec<_> = self
            .peers
            .values()
            .filter_map(|peer| peer.metrics.as_ref().map(|metrics| metrics.block))
            .map(std::cmp::Reverse)
            .collect();
        blocks.sort_unstable();
        let supermajority_index = self.peers.len() * 2 / 3;
        let value = blocks[..].get(supermajority_index).map(|x| x.0);
        value
    }

    fn avg_commit_time(&self) -> Option<Duration> {
        if self.peers.is_empty() {
            return None;
        };
        let averages: Vec<_> = self
            .peers
            .values()
            .filter_map(|peer| peer.metrics.as_ref().map(|metrics| metrics.avg_commit_time))
            .collect();
        averages
            .iter()
            .sum::<Duration>()
            .checked_div(averages.len() as u32)
    }

    fn update_peer(
        &mut self,
        peer: &ToriiUrl,
        update: peer_monitor::Update,
    ) -> Result<UpdatedSegment, PeerNotFound> {
        let state = self.peers.get_mut(peer).ok_or(PeerNotFound)?;

        let ret = match update {
            peer_monitor::Update::Connected(config) => {
                debug_assert!(!state.connected);
                state.connected = true;
                state.config = Some(config);
                UpdatedSegment::Info(state.info())
            }
            peer_monitor::Update::Disconnected => {
                debug_assert!(state.connected);
                state.connected = false;
                state.connected_peers = None;
                state.metrics = None;
                UpdatedSegment::Info(state.info())
            }
            peer_monitor::Update::TelemetryUnsupported => {
                debug_assert!(state.connected);
                state.telemetry_unsupported = true;
                UpdatedSegment::Info(state.info())
            }
            peer_monitor::Update::Metrics(metrics) => {
                debug_assert!(state.connected);
                state.metrics = Some(metrics);
                state.telemetry_unsupported = false;
                UpdatedSegment::Status(state.status().expect("must exists after setting metrics"))
            }
            peer_monitor::Update::Peers(peers) => {
                debug_assert!(state.connected);
                state.connected_peers = Some(peers);
                UpdatedSegment::Info(state.info())
            }
            peer_monitor::Update::Geo(geo) => {
                state.geo = Some(geo);
                UpdatedSegment::Info(state.info())
            }
        };
        Ok(ret)
    }

    fn update_blockchain(&mut self, state: blockchain::State) {
        self.blockchain = Some(state);
    }

    fn network_status(&self) -> Option<NetworkStatus> {
        self.blockchain.as_ref().map(|state| NetworkStatus {
            peers: self.total_peers() as u32,
            domains: state.domains,
            accounts: state.accounts,
            assets: state.assets,
            transactions_accepted: state.txs_accepted,
            transactions_rejected: state.txs_rejected,
            block: state.block,
            block_created_at: state.block_created_at.into(),
            avg_block_time: state.avg_block_time.into(),
            avg_commit_time: self.avg_commit_time().map(From::from),
            finalized_block: self.finalized_block(),
        })
    }

    fn peers_status(&self) -> impl Iterator<Item = PeerStatus> + use<'_> {
        self.peers.values().filter_map(PeerState::status)
    }

    fn single_peer_status(&self, peer: &ToriiUrl) -> Option<PeerStatus> {
        self.peers.get(peer).and_then(PeerState::status)
    }

    fn peers_info(&self) -> impl Iterator<Item = PeerInfo> + use<'_> {
        self.peers.values().map(PeerState::info)
    }

    fn total_peers(&self) -> usize {
        let all_pub_keys: BTreeSet<_> = self
            .peers
            .values()
            .flat_map(|peer| {
                peer.config
                    .as_ref()
                    .map(|cfg| &cfg.public_key)
                    .into_iter()
                    .chain(peer.connected_peers.iter().flat_map(|x| x.iter()))
            })
            .collect();

        all_pub_keys.len()
    }
}

#[derive(Debug)]
enum UpdatedSegment {
    Info(PeerInfo),
    Status(PeerStatus),
}

impl From<UpdatedSegment> for TelemetryStreamMessage {
    fn from(value: UpdatedSegment) -> Self {
        match value {
            UpdatedSegment::Info(data) => Self::PeerInfo(data),
            UpdatedSegment::Status(data) => Self::PeerStatus(data),
        }
    }
}

#[derive(Debug)]
struct PeerNotFound;

struct PeerState {
    url: ToriiUrl,
    connected: bool,
    telemetry_unsupported: bool,
    config: Option<ConfigGetDTO>,
    geo: Option<GeoLocation>,
    connected_peers: Option<BTreeSet<PublicKey>>,
    metrics: Option<peer_monitor::Metrics>,
}

impl PeerState {
    fn new(url: ToriiUrl) -> Self {
        Self {
            url,
            connected: false,
            telemetry_unsupported: false,
            config: None,
            geo: None,
            connected_peers: None,
            metrics: None,
        }
    }

    fn info(&self) -> PeerInfo {
        PeerInfo {
            url: self.url.clone(),
            connected: self.connected,
            telemetry_unsupported: self.telemetry_unsupported,
            config: self.config.as_ref().map(|x| x.clone().into()),
            location: self.geo.clone(),
            connected_peers: self.connected_peers.as_ref().map(|x| {
                x.iter()
                    .map(|y| crate::schema::PublicKey(y.clone()))
                    .collect()
            }),
        }
    }

    fn status(&self) -> Option<PeerStatus> {
        self.metrics.as_ref().map(|metrics| PeerStatus {
            url: self.url.clone(),
            block: metrics.block,
            commit_time: crate::schema::TimeDuration::from(metrics.block_commit_time),
            avg_commit_time: crate::schema::TimeDuration::from(metrics.avg_commit_time),
            queue_size: metrics.queue_size,
            uptime: crate::schema::TimeDuration::from(metrics.uptime),
        })
    }
}

#[derive(Default)]
struct AverageCommitTime<const N: usize> {
    buff: CircularBuffer<N, Duration>,
    last_height: Option<u64>,
}

impl<const N: usize> AverageCommitTime<N> {
    fn new() -> Self {
        Self::default()
    }

    fn observe(&mut self, height: u64, block_time: Duration) {
        if self.last_height.map(|x| x == height).unwrap_or(false) {
            return;
        }
        self.last_height = Some(height);
        self.buff.push_back(block_time);
    }

    fn calculate(&self) -> Option<Duration> {
        let sum = self
            .buff
            .iter()
            .fold(None, |acc, x| Some(acc.unwrap_or(Duration::ZERO) + *x));
        sum.map(|sum| {
            sum.checked_div(self.buff.len() as u32)
                .expect("non-zero if sum exists")
        })
    }
}

#[cfg(test)]
mod avg_commit_tests {
    use super::*;

    #[test]
    fn avg_commit_time_is_empty() {
        let avg = AverageCommitTime::<5>::new();
        assert!(avg.calculate().is_none())
    }

    #[test]
    fn avg_commit_time_calculates_latest_n_window() {
        // duration, windowed mean (4)
        let series = [
            (100u64, 100u128),
            (150, 125),
            (200, 150),
            (10, 115),
            (45, 101),
            (120, 93),
            (350, 131),
        ];

        let mut avg = AverageCommitTime::<4>::new();

        for (i, (ms, mean_ms)) in series.iter().enumerate() {
            avg.observe(i as u64 + 1, Duration::from_millis(*ms));
            let value = avg.calculate().unwrap();
            assert_eq!(value.as_millis(), *mean_ms);
        }
    }

    #[test]
    fn avg_commit_time_deduplicates_by_height() {
        let mut avg = AverageCommitTime::<10>::new();

        avg.observe(1, Duration::from_millis(100));
        avg.observe(2, Duration::from_millis(200));
        avg.observe(2, Duration::from_millis(300)); // ignore
        avg.observe(3, Duration::from_millis(400));

        assert_eq!(avg.calculate().unwrap().as_millis(), (100 + 200 + 400) / 3)
    }
}

#[cfg(test)]
mod state_tests {
    use super::*;
    use crate::telemetry::peer_monitor::{Metrics, Update};
    use insta::assert_json_snapshot;
    use iroha::client::ConfigGetDTO;
    use serde_json::json;

    fn factory_key(seed: impl AsRef<[u8]>) -> PublicKey {
        iroha_crypto::KeyPair::from_seed(seed.as_ref().into(), <_>::default())
            .public_key()
            .clone()
    }

    fn factory_url(id: impl std::fmt::Display) -> ToriiUrl {
        ToriiUrl(format!("http://iroha.tech/{}", id).parse().unwrap())
    }

    fn factory_block_state() -> blockchain::State {
        blockchain::State {
            block: 0,
            block_created_at: <_>::default(),
            domains: 0,
            accounts: 0,
            assets: 0,
            txs_accepted: 0,
            txs_rejected: 0,
            // parameter_max_block_time: Duration::ZERO,
            // parameter_max_commit_time: Duration::ZERO,
            // parameter_max_txs_per_block: 0,
            avg_block_time: Duration::ZERO,
        }
    }

    fn factory_config(seed: impl AsRef<[u8]>) -> ConfigGetDTO {
        serde_json::from_value(json!({
            "public_key": factory_key(seed),
            "logger": {
                "level": "INFO",
                "filter": null
            },
            "network": {
                "block_gossip_size": 4,
                "block_gossip_period_ms": 10000,
                "transaction_gossip_size": 500,
                "transaction_gossip_period_ms": 1000
            },
            "queue": {
                "capacity": 65536
            }
        }))
        .unwrap()
    }

    fn factory_metrics() -> Metrics {
        Metrics {
            // peers: 0,
            block: 0,
            block_commit_time: Duration::ZERO,
            avg_commit_time: Duration::ZERO,
            queue_size: 0,
            uptime: Duration::ZERO,
        }
    }

    #[test]
    fn empty() {
        let peer_0 = factory_url("0");
        let peer_1 = factory_url("1");
        let peer_2 = factory_url("2");
        let peer_3 = factory_url("3");
        let state = State::new([peer_0, peer_1, peer_2, peer_3].into_iter().collect());

        assert!(state.network_status().is_none());
        let info: Vec<_> = state.peers_info().collect();
        assert_json_snapshot!(info);
        let peers: Vec<_> = state.peers_status().collect();
        assert!(peers.is_empty())
    }

    #[test]
    fn counts_all_peers_via_public_keys() {
        let url1 = factory_url("1");
        let key1 = factory_key(b"1");
        let url2 = factory_url("2");
        let key2 = factory_key(b"2");
        let key3 = factory_key(b"3");
        let key4 = factory_key(b"4");
        let mut state = State::new([&url1, &url2].into_iter().cloned().collect());

        // 1 -> 2, 3
        // 2 -> 4
        // total - 4 different peers
        let _ = state.update_peer(
            &url1,
            Update::Connected(ConfigGetDTO {
                public_key: key1.clone().into(),
                ..factory_config(b"key 1")
            }),
        );
        let _ = state.update_peer(
            &url1,
            Update::Peers([&key2, &key3].into_iter().cloned().collect()),
        );
        let _ = state.update_peer(
            &url2,
            Update::Connected(ConfigGetDTO {
                public_key: key2.clone().into(),
                ..factory_config(b"key 2")
            }),
        );
        let _ = state.update_peer(&url2, Update::Peers([&key4].into_iter().cloned().collect()));
        state.update_blockchain(factory_block_state());

        let network = state.network_status().unwrap();
        assert_eq!(network.peers, 4);
    }

    struct FinalizedBlockHelper {
        state: State,
    }

    impl FinalizedBlockHelper {
        fn new<const N: usize>(urls: [&ToriiUrl; N]) -> Self {
            let mut state = State::new(urls.clone().into_iter().cloned().collect());
            for (i, url) in urls.into_iter().enumerate() {
                state
                    .update_peer(url, Update::Connected(factory_config([i as u8])))
                    .unwrap();
            }
            Self { state }
        }

        fn update_block(&mut self, url: &ToriiUrl, block: u32) {
            self.state
                .update_peer(
                    &url,
                    Update::Metrics(Metrics {
                        block,
                        ..factory_metrics()
                    }),
                )
                .unwrap();
        }

        fn assert(&self, expected: Option<u32>) {
            assert_eq!(self.state.finalized_block(), expected);
        }
    }

    #[test]
    fn finalized_block_by_supermajority() {
        let url1 = factory_url("1");
        let url2 = factory_url("2");
        let url3 = factory_url("3");
        let url4 = factory_url("4");
        let mut helper = FinalizedBlockHelper::new([&url1, &url2, &url3, &url4]);

        helper.assert(None); // no peers data yet
        helper.update_block(&url1, 1);
        helper.assert(None); // minority
        helper.update_block(&url2, 1);
        helper.assert(None); // still minority
        helper.update_block(&url3, 1);
        helper.assert(Some(1));
        helper.update_block(&url4, 1);
        helper.assert(Some(1));
        helper.update_block(&url1, 4);
        helper.assert(Some(1));
        helper.update_block(&url2, 4);
        helper.assert(Some(1));
        helper.update_block(&url3, 2);
        helper.assert(Some(2));
        helper.update_block(&url4, 3);
        helper.assert(Some(3));
        helper.update_block(&url3, 4);
        helper.assert(Some(4));
        helper.update_block(&url1, 5);
        helper.assert(Some(4));
    }

    #[test]
    fn finalized_block_2_peers() {
        let url1 = factory_url("1");
        let url2 = factory_url("2");
        let mut helper = FinalizedBlockHelper::new([&url1, &url2]);

        helper.update_block(&url1, 1);
        helper.assert(None);
        helper.update_block(&url1, 2);
        helper.assert(None);
        helper.update_block(&url2, 1);
        helper.assert(Some(1));
    }

    #[test]
    fn finalized_block_1_peer() {
        let url = factory_url("only");
        let mut helper = FinalizedBlockHelper::new([&url]);

        helper.update_block(&url, 1);
        helper.assert(Some(1));
        helper.update_block(&url, 2);
        helper.assert(Some(2));
    }

    #[test]
    fn full_peer_update_cycle() {
        let url = factory_url("test");
        let mut state = State::new([&url].into_iter().cloned().collect());

        let _ = state.update_peer(&url, Update::Connected(factory_config(b"test")));
        let _ = state.update_peer(
            &url,
            Update::Geo(GeoLocation {
                lat: 55.0,
                lon: 32.0,
                country: "Wonderland".to_owned(),
                city: "Makondo".to_owned(),
            }),
        );
        let _ = state.update_peer(
            &url,
            Update::Metrics(Metrics {
                avg_commit_time: Duration::from_millis(122),
                ..factory_metrics()
            }),
        );
        let _ = state.update_peer(
            &url,
            Update::Peers(
                [factory_key(b"one"), factory_key(b"two")]
                    .into_iter()
                    .collect(),
            ),
        );

        let info = state.peers_info().find(|x| &x.url == &url).unwrap();
        let status = state.single_peer_status(&url).expect("must be");
        assert_json_snapshot!(info);
        assert_json_snapshot!(status);

        let _ = state.update_peer(&url, Update::Disconnected);

        let info = state.peers_info().find(|x| &x.url == &url).unwrap();
        assert!(info.config.is_some());
        assert!(info.location.is_some());
        assert!(info.connected_peers.is_none());

        let status = state.single_peer_status(&url);
        assert!(status.is_none());
    }

    #[test]
    fn no_peers_no_avg_block() {
        let state = State::new(<_>::default());
        assert_eq!(state.avg_commit_time(), None);
    }

    #[test]
    fn geo_could_arrive_before_connection() {
        let url = factory_url("test");
        let mut state = State::new([&url].into_iter().cloned().collect());

        let geo = GeoLocation {
            lat: 5.0,
            lon: 3.0,
            city: "test".to_owned(),
            country: "test".to_owned(),
        };
        state.update_peer(&url, Update::Geo(geo.clone())).unwrap();

        let info = state.peers_info().find(|x| &x.url == &url).unwrap();
        assert_eq!(info.location, Some(geo));
    }

    #[test]
    fn telemetry_unsupported() {
        let url = factory_url("test");
        let mut state = State::new([&url].into_iter().cloned().collect());

        state
            .update_peer(&url, Update::Connected(factory_config(b"test")))
            .unwrap();
        state
            .update_peer(&url, Update::TelemetryUnsupported)
            .unwrap();

        let info = state.peers_info().find(|x| &x.url == &url).unwrap();
        assert!(info.telemetry_unsupported);

        state
            .update_peer(&url, Update::Metrics(factory_metrics()))
            .unwrap();

        let info = state.peers_info().find(|x| &x.url == &url).unwrap();
        assert!(!info.telemetry_unsupported);
    }
}
