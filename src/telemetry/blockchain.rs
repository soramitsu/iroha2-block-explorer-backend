use crate::core::state::StateReader;
// use crate::repo::Repo;
use crate::telemetry::AVG_COMMIT_BLOCK_TIME_WINDOW;
use iroha_core::kura::KuraReadOnly;
use iroha_core::state::{StateReadOnly, WorldReadOnly};
use sqlx::FromRow;
use std::convert::Infallible;
use std::num::NonZero;
use std::time::Duration;

const QUERY: &str = "\
select (select count() from blocks)                                            as block,
       (select created_at from blocks order by blocks.created_at desc limit 1) as block_created_at,
       (select count() from domains)                                           as domains,
       (select count() from accounts)                                          as accounts,
       (select count() from assets) + (select count() from nfts)               as assets,
       (select count() from transactions where error is null)                  as txs_accepted,
       (select count() from transactions where error is not null)              as txs_rejected,
       (select cast(((max(unixepoch(created_at, 'subsec')) -
                      min(unixepoch(created_at, 'subsec'))) * 1000) / count(distinct created_at) as integer)
        from (select created_at
              from blocks
              order by created_at desc
              limit ?))                                                       as avg_block_time_ms";

#[derive(Clone, Debug)]
pub struct Metrics {
    pub block: u32,
    pub block_created_at: chrono::DateTime<chrono::Utc>,
    pub domains: u32,
    pub accounts: u32,
    pub assets: u32,
    pub txs_accepted: u32,
    pub txs_rejected: u32,
    // TODO: could be enabled now!
    // pub parameter_max_block_time: Duration,
    // pub parameter_max_commit_time: Duration,
    // pub parameter_max_txs_per_block: u32,
    pub avg_block_time: DurationMillis,
}

// TODO: create with state view and readonly kura
// update incrementally with new blocks (re-use AvgCommitTime helper)
impl Metrics {
    //     /// Initiate with the current state view
    // pub fn new(reader: &StateReader) -> Self {
    //
    //     }
    //
    //     pub fn update(&mut self,
}

// // PERF: loads all blocks on every new block; not optimal
// // FIXME: have a running State which is updated with every new block
// impl From<&StateReader> for State {
//     fn from(reader: &StateReader) -> Self {
//         let view = reader.view();
//         let storage = reader.storage();
//
//         let Some(height) = NonZero::new(view.height()) else {
//             todo!()
//         };
//
//         let (txs_accepted, txs_rejected, avg_block_time) = view.all_blocks(&storage, nonzero!(1usize))
//
//         Self {
//             block: height.get() as u32,
//             block_created_at: storage
//                 .get_block(height)
//                 .unwrap()
//                 .header()
//                 .creation_time()
//                 .into(),
//             domains: view.world().domains().len(),
//             accounts: view.world().accounts().len(),
//             assets: view.world().assets().len() + view.world().nfts().len(),
//         }
//     }
// }

#[derive(Copy, Clone, Debug)]
pub struct DurationMillis(pub Duration);

impl TryFrom<u64> for DurationMillis {
    type Error = Infallible;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(Self(Duration::from_millis(value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::tests::test_repo;
    use insta::assert_debug_snapshot;

    #[tokio::test]
    async fn scan_test_repo() {
        let repo = test_repo().await;

        let state = Metrics::scan(&repo).await.unwrap();
        assert_debug_snapshot!(state);
    }
}
