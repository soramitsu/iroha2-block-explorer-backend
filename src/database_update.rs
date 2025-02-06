use crate::iroha_client_wrap::ClientWrap;
use crate::repo::{scan_iroha, Repo};
use eyre::WrapErr;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::ConnectOptions;
use std::time::Duration;
use tokio::task::spawn_blocking;
use tokio::time::sleep;

pub struct DatabaseUpdateLoop {
    repo: Repo,
    client: ClientWrap,
    last_update_block: u64,
}

impl DatabaseUpdateLoop {
    pub fn new(repo: Repo, client: ClientWrap) -> Self {
        Self {
            repo,
            client,
            last_update_block: 0,
        }
    }

    pub async fn run(mut self) {
        const TICK: Duration = Duration::from_secs(10);

        loop {
            if let Err(err) = self.attempt().await {
                tracing::error!(
                    ?err,
                    "Error while attempting to update the database. Will try again."
                );
            }

            tracing::debug!("Sleep for {TICK:?}");
            sleep(TICK).await;
        }
    }

    async fn attempt(&mut self) -> eyre::Result<()> {
        let client_clone = self.client.clone();
        let status = spawn_blocking(move || client_clone.get_status())
            .await?
            .wrap_err("Failed to fetch Iroha status")?;

        if status.blocks == self.last_update_block {
            tracing::debug!("No new blocks - skipping update");
            return Ok(());
        }

        tracing::debug!("Updating the database");
        let mut conn = SqliteConnectOptions::new()
            .in_memory(true)
            .connect()
            .await?;
        scan_iroha(&self.client, &mut conn)
            .await
            .wrap_err("Failed to scan Iroha")?;
        self.repo.swap(conn).await;

        self.last_update_block = status.blocks;
        tracing::info!(%self.last_update_block, "Updated the database");

        Ok(())
    }
}
