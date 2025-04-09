use crate::iroha_client_wrap::ClientWrap;
use crate::repo::{scan_iroha, Repo};
use crate::telemetry::blockchain::State;
use crate::telemetry::Telemetry;
use eyre::WrapErr;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::ConnectOptions;
use std::time::Duration;
use tokio::time::sleep;

pub struct DatabaseUpdateLoop {
    repo: Repo,
    client: ClientWrap,
    telemetry: Telemetry,
    last_update_block: u32,
}

impl DatabaseUpdateLoop {
    pub fn new(repo: Repo, client: ClientWrap, telemetry: Telemetry) -> Self {
        Self {
            repo,
            client,
            telemetry,
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
        let Some(metrics) = self.telemetry.single_peer(&self.client.torii_url()).await? else {
            tracing::warn!("Skipping database update - peer metrics are not available");
            return Ok(());
        };

        if metrics.block == self.last_update_block {
            tracing::debug!("Skipping database update - no blocks difference");
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

        self.last_update_block = metrics.block;
        tracing::info!(%self.last_update_block, "Updated the database");

        if let Err(err) = self.update_telemetry().await {
            tracing::error!(?err, "Failed to update blockchain state in telemetry")
        }

        Ok(())
    }

    async fn update_telemetry(&self) -> eyre::Result<()> {
        let state = State::scan(&self.repo).await?;
        self.telemetry.update_blockchain_state(state).await
    }
}
