use std::future::Future;
use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

use eyre::{eyre, Context as _};
use futures_util::StreamExt as _;
use iroha_core::state::{StateReadOnly, StateView};
use iroha_futures::supervisor::ShutdownSignal;
use nonzero_ext::nonzero;
use tokio::sync::Mutex;
use tokio::task::LocalSet;
use tracing::{debug, error, info};

use crate::core::state::State;
use crate::core::storage::Storage;
use crate::iroha_client::Client;

use super::StateViewExt as _;

pub async fn run(state: &State, storage: &Storage, client: &Client) -> ! {
    sync_loop(state, storage, client).await
}

// /// Sync loop runs in a [`LocalSet`] to allow `!Send` futures.
// ///
// /// It is required because [`StateView`] is not [`Send`], but is held across await
// /// on [`Client`] calls while establishing the last matching block.
// pub fn start(
//     state: Arc<State>,
//     storage: Arc<Storage>,
//     client: &Client,
//     signal: ShutdownSignal,
// ) -> std::thread::JoinHandle<()> {
//     let rt = tokio::runtime::Builder::new_current_thread()
//         .enable_all()
//         .build()
//         .unwrap();
//
//     let handle = std::thread::spawn(move || {
//         let local = LocalSet::new();
//
//         local.spawn_local(async move {
//             tokio::select! {
//               _ = sync_loop(state, client) => { unreachable!() },
//               _ = signal => {}
//             };
//             debug!("Terminating sync loop");
//         });
//
//         rt.block_on(local);
//     });
//
//     handle
// }

async fn sync_loop(state: &State, storage: &Storage, client: &Client) -> ! {
    const RETRY: Duration = Duration::from_secs(5);

    info!("Entering sync loop");

    loop {
        let sync_from = match find_last_matching_block(storage, client).await {
            Err(error) => {
                error!(?error, retry=?RETRY, "Cannot determine last matching block, waiting before retrying");
                tokio::time::sleep(RETRY).await;
                continue;
            }
            Ok(None) => {
                info!("No matching blocks, synchronizing from the beginning");
                nonzero!(1usize)
            }
            Ok(Some(height)) => {
                info!(
                    height,
                    "Determined last matching block, synchronizing from it"
                );
                height.checked_add(1).unwrap()
            }
        };

        debug!(height = sync_from, "Opening block stream");
        let mut stream = client.lazy_block_stream(sync_from).await;
        loop {
            let Some(block) = stream.next().await else {
                error!("Block stream is closed");
                break;
            };
            debug!(height=%block.header().height(), hash=%block.hash(), "Received block");
            if let Err(error) = state.insert_block(block).await {
                error!(?error, "Local state failed to accept block");
                break;
            }
        }
    }
}

async fn find_last_matching_block(
    storage: &Storage,
    client: &Client,
) -> eyre::Result<Option<NonZero<usize>>> {
    let storage = storage.acquire_read().await?;

    let Some(local_height) = NonZero::new(storage.height()) else {
        return Ok(None);
    };
    let local_genesis_hash = storage
        .block_hash(nonzero!(1usize))
        .expect("height is non-zero");

    // Genesis check to cover a common corner case
    let iroha_genesis_hash = client.get_genesis_hash().await?;
    if local_genesis_hash != iroha_genesis_hash {
        return Ok(None);
    }

    // NOTE: naive implementation - naive checks of every block
    // PERF: could be optimised as some sort of binary search
    let mut batches = client.blocks_info_from_end(local_height);
    while let Some(batch) = batches.next().await {
        for info in batch.iter_from_last() {
            let local = storage
                .block_hash(info.height)
                .expect("less than local height");
            if local == info.hash {
                return Ok(Some(info.height));
            }
        }
    }

    Ok(None)
}
