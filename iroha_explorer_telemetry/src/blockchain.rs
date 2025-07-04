use std::time::Duration;

#[derive(Clone, Debug, Default)]
pub struct Metrics {
    pub block: usize,
    pub block_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub domains: usize,
    pub accounts: usize,
    pub assets: usize,
    pub txs_accepted: usize,
    pub txs_rejected: usize,
    pub parameter_max_block_time: Duration,
    pub parameter_max_commit_time: Duration,
    pub parameter_max_txs_per_block: usize,
    pub avg_block_time: Duration,
}
