pub mod query;
pub mod state;
mod storage;
mod sync;

use std::{future::Future, num::NonZero, path::PathBuf, sync::Arc, time::Duration};

use futures_util::{Stream, StreamExt as _};
use iroha_config::base::WithOrigin;
use iroha_core::{
    block::{CommittedBlock, ValidBlock},
    kura::Kura,
    query::store::{LiveQueryStore, LiveQueryStoreHandle},
    state::{State, StateBlock, StateReadOnly, StateView, World},
};
use iroha_crypto::{HashOf, PublicKey};
use iroha_data_model::prelude::*;
use iroha_futures::supervisor::{Child, OnShutdown, ShutdownSignal, Supervisor};
use nonzero_ext::nonzero;
pub use query::QueryExecutor;
use tokio::sync::watch;
use tracing::{debug, error, info};

const KURA_BLOCKS_IN_MEMORY: NonZero<usize> = nonzero!(128usize);

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(
        "block is too far - state height is {state_height}, while block height is {block_height}"
    )]
    ReceivedBlockHeightIsTooFar {
        state_height: usize,
        block_height: NonZero<usize>,
    },
    #[error("todo")]
    ReceivedBlockPreviousBlockHashNotFound {
        actual_prev_block_hash: HashOf<BlockHeader>,
        block_prev_block_hash: HashOf<BlockHeader>,
    },
    #[error("todo")]
    ReceivedBlockIsAlreadyInBlockChain,
    #[error("todo")]
    KuraInit(#[source] iroha_core::kura::Error),
    #[error("todo")]
    IrohaHasNoGenesis,
    #[error("todo")]
    GenesisNoTransactions,
}

type Result<T, E = Error> = core::result::Result<T, E>;

trait StateViewExt {
    fn block_hash(&self, height: NonZero<usize>) -> Option<HashOf<BlockHeader>>;
}

impl StateViewExt for StateView<'_> {
    fn block_hash(&self, height: NonZero<usize>) -> Option<HashOf<BlockHeader>> {
        self.block_hashes().get(height.get() - 1).map(|x| *x)
    }
}

pub fn start(
    store_dir: impl Into<PathBuf>,
    telemetry: crate::telemetry::Telemetry,
    client: crate::iroha_client::Client,
    signal: ShutdownSignal,
) -> (
    state::State,
    impl Future<Output = Result<(), iroha_futures::supervisor::Error>> + Sized + Send + Sync,
) {
    let (storage, storage_fut) = storage::Storage::new(store_dir, signal);
    let (state, state_fut) = state::State::new(storage.clone());
    let sync_fut = async move { sync::run(&state, &storage, &client).await };

    let mut sup = Supervisor::new();

    sup.monitor(Child::new(
        tokio::spawn(storage_fut),
        OnShutdown::Wait(Duration::from_secs(5)),
    ));
    sup.monitor(Child::new(tokio::spawn(state_fut), OnShutdown::Abort));
    sup.monitor(Child::new(tokio::spawn(sync_fut), OnShutdown::Abort));

    sup.shutdown_on_external_signal(signal);

    (state, sup.start())
}
