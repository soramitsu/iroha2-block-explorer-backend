pub mod query;
pub mod state;
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
use tokio::sync::{mpsc, oneshot, watch};
use tracing::{debug, error, info};

const KURA_BLOCKS_IN_MEMORY: NonZero<usize> = nonzero!(128usize);

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // #[error(
    //     "block is too far - state height is {state_height}, while block height is {block_height}"
    // )]
    // ReceivedBlockHeightIsTooFar {
    //     state_height: usize,
    //     block_height: NonZero<usize>,
    // },
    // #[error("todo")]
    // ReceivedBlockPreviousBlockHashNotFound {
    //     actual_prev_block_hash: HashOf<BlockHeader>,
    //     block_prev_block_hash: HashOf<BlockHeader>,
    // },
    // #[error("todo")]
    // ReceivedBlockIsAlreadyInBlockChain,
    #[error("todo")]
    NotConfirmed,
    #[error("todo")]
    ReceivedBlockHeightMismatch {
        expected: NonZero<usize>,
        actual: NonZero<usize>,
    },
    #[error("todo")]
    ReceivedBlockPreviousBlockHashMismatch {
        actual_prev_block_hash: HashOf<BlockHeader>,
        block_prev_block_hash: HashOf<BlockHeader>,
    },
    #[error("todo")]
    KuraInit(#[source] iroha_core::kura::Error),
    #[error("todo")]
    IrohaHasNoGenesis,
    #[error("todo")]
    GenesisNoTransactions,
}

impl<T> From<mpsc::error::SendError<T>> for Error {
    fn from(value: mpsc::error::SendError<T>) -> Self {
        todo!()
    }
}

impl From<oneshot::error::RecvError> for Error {
    fn from(value: oneshot::error::RecvError) -> Self {
        todo!()
    }
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
    let (state, state_fut) = state::State::new(store_dir, telemetry, signal.clone());
    let state4sync = state.clone();
    let sync_fut = async move { sync::run(&state4sync, &client).await };

    let mut sup = Supervisor::new();

    sup.monitor(Child::new(
        tokio::spawn(async move {
            if let Err(err) = state_fut.await {
                tracing::error!(error=%err, "State shut down with error");
            }
        }),
        OnShutdown::Wait(Duration::from_secs(5)),
    ));
    sup.monitor(Child::new(tokio::spawn(sync_fut), OnShutdown::Abort));

    sup.shutdown_on_external_signal(signal);

    (state, sup.start())
}
