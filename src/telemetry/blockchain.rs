use crate::repo::Repo;
use crate::telemetry::AVG_COMMIT_BLOCK_TIME_WINDOW;
use sqlx::FromRow;
use std::convert::Infallible;
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

#[derive(Clone, Debug, FromRow)]
pub struct State {
    pub block: u32,
    pub block_created_at: chrono::DateTime<chrono::Utc>,
    pub domains: u32,
    pub accounts: u32,
    pub assets: u32,
    pub txs_accepted: u32,
    pub txs_rejected: u32,
    // pub parameter_max_block_time: Duration,
    // pub parameter_max_commit_time: Duration,
    // pub parameter_max_txs_per_block: u32,
    #[sqlx(rename = "avg_block_time_ms", try_from = "u64")]
    pub avg_block_time: DurationMillis,
}

#[derive(Copy, Clone, Debug)]
pub struct DurationMillis(pub Duration);

impl TryFrom<u64> for DurationMillis {
    type Error = Infallible;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(Self(Duration::from_millis(value)))
    }
}

impl State {
    pub async fn scan(repo: &Repo) -> Result<Self, crate::repo::Error> {
        let mut conn = repo.acquire_conn().await;
        let state: State = sqlx::query_as(QUERY)
            .bind(AVG_COMMIT_BLOCK_TIME_WINDOW as u32)
            .fetch_one(&mut *conn)
            .await?;
        Ok(state)
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

        let state = State::scan(&repo).await.unwrap();
        assert_debug_snapshot!(state);
    }
}
