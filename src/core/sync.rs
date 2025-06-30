use std::num::NonZero;
use std::time::Duration;

use futures_util::StreamExt as _;
use iroha_core::state::StateReadOnly;
use nonzero_ext::nonzero;
use tracing::{debug, error, info};

use crate::core::state::State;
use crate::iroha_client::Client;

pub async fn run(state: &State, client: &Client) -> ! {
    sync_loop(state, client).await
}

async fn sync_loop(state: &State, client: &Client) -> ! {
    const RETRY: Duration = Duration::from_secs(5);

    info!("Entering sync loop");

    loop {
        let sync_from = match find_last_matching_block(state, client).await {
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
    state: &State,
    client: &Client,
) -> eyre::Result<Option<NonZero<usize>>> {
    let guard = state.acquire_guard().await;
    let kura = guard.kura();

    let Some(local_height) = NonZero::new(kura.blocks_count()) else {
        return Ok(None);
    };
    let local_genesis_hash = kura
        .get_block_hash(nonzero!(1usize))
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
            let local = kura
                .get_block_hash(info.height)
                .expect("less than local height");
            if local == info.hash {
                return Ok(Some(info.height));
            }
        }
    }

    Ok(None)
}
