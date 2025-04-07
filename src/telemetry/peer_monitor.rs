use crate::schema::GeoLocation;
use crate::schema::ToriiUrl;
use iroha::client::ConfigGetDTO;
use iroha::crypto::PublicKey;
use std::collections::BTreeSet;
use std::future::Future;
use std::time::Duration;
use tokio::sync::broadcast;

pub fn run(
    url: ToriiUrl,
) -> (
    broadcast::Receiver<Update>,
    impl Future<Output = ()> + Sized,
) {
    let (tx, rx) = broadcast::channel(128);

    let fut = async move { todo!() };

    (rx, fut)
}

#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub peers: u32,
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
    Metrics(Metrics),
    Geo(GeoLocation),
    Peers(BTreeSet<PublicKey>),
}
