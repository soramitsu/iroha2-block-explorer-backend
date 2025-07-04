use std::time::Duration;

#[derive(Clone, Debug, Default)]
pub struct Metrics {
    pub block: u32,
    pub block_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub domains: u32,
    pub accounts: u32,
    pub assets: u32,
    pub txs_accepted: u32,
    pub txs_rejected: u32,
    pub parameter_max_block_time: Duration,
    pub parameter_max_commit_time: Duration,
    pub parameter_max_txs_per_block: u32,
    pub avg_block_time: Duration,
}
