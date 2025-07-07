use std::num::NonZero;

use crate::repo::{AsText, SignatureDisplay};
use chrono::DateTime;
use eyre::Result;
use iroha::client::Client;
use iroha::data_model::prelude::*;
use itertools::Itertools as _;
use sqlx::query_builder::Separated;
use sqlx::types::Json;
use sqlx::{query, QueryBuilder, Sqlite, SqliteConnection};
use tracing::debug;

async fn insert_in_batches<'args, const N: usize, I, F>(
    conn: &mut SqliteConnection,
    table: &str,
    fields: [&str; N],
    tuples: I,
    mut push_tuple: F,
) -> Result<(), sqlx::Error>
where
    I: ExactSizeIterator,
    I::Item: 'args,
    F: FnMut(Separated<'_, 'args, Sqlite, &'static str>, I::Item),
{
    // https://sqlite.org/limits.html
    // NOTE: it could be 32k, depending on version
    const SQLITE_LIMIT: usize = 999;

    for chunk in &tuples.chunks(const { SQLITE_LIMIT / N }) {
        let mut builder = QueryBuilder::new("insert into ");
        builder.push(table).push("(");
        let mut sep = builder.separated(", ");
        for i in fields {
            sep.push(i);
        }
        builder.push(") ");
        builder.push_values(chunk, &mut push_tuple);
        builder.build().execute(&mut *conn).await?;
    }

    Ok(())
}

/// Scan Iroha into an `SQLite` database.
#[allow(clippy::too_many_lines)]
pub async fn scan_into(client: &Client, conn: &mut SqliteConnection) -> Result<()> {
    debug!("Scanning Iroha into an in-memory SQLite database");

    const CHUNK: usize = 100;
    const BLOCKS_CHUNK: usize = 5;
    let fetch_size = FetchSize::new(NonZero::new(CHUNK as u64));

    query(include_str!("./create_tables.sql"))
        .execute(&mut *conn)
        .await?;
    query("PRAGMA foreign_keys=OFF").execute(&mut *conn).await?;
    query("BEGIN TRANSACTION").execute(&mut *conn).await?;

    for chunk in &client
        .query(FindDomains)
        .with_fetch_size(fetch_size)
        .execute()?
        .chunks(CHUNK)
    {
        debug!("domains chunk");
        let chunk = chunk.collect::<Result<Vec<_>, _>>()?;
        insert_in_batches(
            conn,
            "domains",
            ["name", "logo", "metadata"],
            chunk.iter(),
            |mut b, value| {
                b.push_bind(value.id().name().as_ref())
                    .push_bind(value.logo().as_ref().map(AsText))
                    .push_bind(Json(value.metadata()));
            },
        )
        .await?;
        insert_in_batches(
            conn,
            "domain_owners",
            ["account_signatory", "account_domain", "domain"],
            chunk.iter(),
            |mut b, value| {
                b.push_bind(AsText(value.owned_by().signatory()))
                    .push_bind(AsText(value.owned_by().domain()))
                    .push_bind(AsText(value.id()));
            },
        )
        .await?;
    }

    for chunk in &client.query(FindAccounts).execute()?.chunks(CHUNK) {
        debug!("accounts chunk");
        let chunk = chunk.collect::<Result<Vec<_>, _>>()?;
        insert_in_batches(
            conn,
            "accounts",
            ["signatory", "domain", "metadata"],
            chunk.iter(),
            |mut b, value| {
                b.push_bind(AsText(value.signatory()))
                    .push_bind(AsText(value.id().domain()))
                    .push_bind(Json(value.metadata()));
            },
        )
        .await?;
    }

    for chunk in &client
        .query(FindBlocks)
        .with_fetch_size(FetchSize::new(NonZero::new(BLOCKS_CHUNK as u64)))
        .execute()?
        .chunks(BLOCKS_CHUNK)
    {
        debug!("blocks chunk");
        let chunk = chunk.collect::<Result<Vec<_>, _>>()?;
        insert_in_batches(
            conn,
            "blocks",
            [
                "hash",
                "height",
                "created_at",
                "prev_block_hash",
                "transactions_hash",
            ],
            chunk.iter(),
            |mut b, value| {
                b.push_bind(AsText(value.hash()))
                    .push_bind(value.header().height().get() as i32)
                    .push_bind(DateTime::from_timestamp_millis(
                        value.header().creation_time().as_millis() as i64,
                    ))
                    .push_bind(value.header().prev_block_hash().map(AsText))
                    .push_bind(value.header().transactions_hash().map(AsText));
            },
        )
        .await?;
        insert_in_batches(
            conn,
            "transactions",
            [
                "hash",
                "block",
                "created_at",
                "time_to_live_ms",
                "authority_signatory",
                "authority_domain",
                "signature",
                "nonce",
                "metadata",
                "error",
                "executable",
            ],
            chunk
                .iter()
                .flat_map(|block| {
                    block
                        .transactions()
                        .enumerate()
                        .map(move |(i, tx)| (block, tx, i))
                })
                // FIXME: have to collect to avoid type pain
                .collect::<Vec<_>>()
                .iter(),
            |mut b, (block, tx, tx_index)| {
                let error = block.error(*tx_index);
                let height = block.header().height();

                b.push_bind(AsText(tx.hash()))
                    .push_bind(height.get() as u32)
                    .push_bind(DateTime::from_timestamp_millis(
                        tx.creation_time().as_millis() as i64,
                    ))
                    .push_bind(tx.time_to_live().map(|dur| dur.as_millis() as i64))
                    .push_bind(AsText(tx.authority().signatory()))
                    .push_bind(AsText(tx.authority().domain()))
                    .push_bind(AsText(SignatureDisplay(tx.signature().payload().clone())))
                    .push_bind(tx.nonce().map(|num| i64::from(num.get())))
                    .push_bind(Json(tx.metadata()))
                    .push_bind(error.map(Json))
                    .push_bind(match tx.instructions() {
                        Executable::Instructions(_) => "Instructions",
                        Executable::Wasm(_) => "WASM",
                    });
            },
        )
        .await?;
        insert_in_batches(
            conn,
            "instructions",
            ["transaction_hash", "value"],
            chunk
                .iter()
                .flat_map(|block| {
                    block
                        .transactions()
                        .filter_map(|tx| match tx.instructions() {
                            Executable::Instructions(isi_vec) => {
                                Some(isi_vec.iter().map(|isi| (tx.hash(), isi)))
                            }
                            Executable::Wasm(_) => None,
                        })
                        .flatten()
                })
                .collect::<Vec<_>>()
                .iter(),
            |mut b, (tx_hash, value)| {
                b.push_bind(AsText(tx_hash)).push_bind(Json(value));
            },
        )
        .await?;
    }

    for chunk in &client
        .query(FindAssetsDefinitions)
        .with_fetch_size(fetch_size)
        .execute()?
        .chunks(CHUNK)
    {
        debug!("asset definitions chunk");
        let chunk = chunk.collect::<Result<Vec<_>, _>>()?;
        insert_in_batches(
            conn,
            "asset_definitions",
            [
                "name",
                "domain",
                "metadata",
                "mintable",
                "owned_by_signatory",
                "owned_by_domain",
                "logo",
            ],
            chunk.iter(),
            |mut b, value| {
                b.push_bind(AsText(value.id().name()))
                    .push_bind(AsText(value.id().domain()))
                    .push_bind(Json(value.metadata()))
                    .push_bind(match value.mintable() {
                        Mintable::Not => "Not",
                        Mintable::Once => "Once",
                        Mintable::Infinitely => "Infinitely",
                    })
                    .push_bind(AsText(value.owned_by().signatory()))
                    .push_bind(AsText(value.owned_by().domain()))
                    .push_bind(value.logo().as_ref().map(AsText));
            },
        )
        .await?;
    }

    for chunk in &client
        .query(FindAssets)
        .with_fetch_size(fetch_size)
        .execute()?
        .chunks(CHUNK)
    {
        debug!("assets chunk");
        let chunk = chunk.collect::<Result<Vec<_>, _>>()?;
        insert_in_batches(
            conn,
            "assets",
            [
                "definition_name",
                "definition_domain",
                "owned_by_signatory",
                "owned_by_domain",
                "value",
            ],
            chunk.iter(),
            |mut b, value| {
                b.push_bind(AsText(value.id().definition().name()))
                    .push_bind(AsText(value.id().definition().domain()))
                    .push_bind(AsText(value.id().account().signatory()))
                    .push_bind(AsText(value.id().account().domain()))
                    .push_bind(Json(value.value()));
            },
        )
        .await?;
    }

    for chunk in &client
        .query(FindNfts)
        .with_fetch_size(fetch_size)
        .execute()?
        .chunks(CHUNK)
    {
        debug!("nfts chunk");
        let chunk = chunk.collect::<Result<Vec<_>, _>>()?;
        insert_in_batches(
            conn,
            "nfts",
            [
                "name",
                "domain",
                "content",
                "owned_by_signatory",
                "owned_by_domain",
            ],
            chunk.iter(),
            |mut b, value| {
                b.push_bind(AsText(value.id().name()))
                    .push_bind(AsText(value.id().domain()))
                    .push_bind(Json(value.content()))
                    .push_bind(AsText(value.owned_by().signatory()))
                    .push_bind(AsText(value.owned_by().domain()));
            },
        )
        .await?;
    }

    query("COMMIT").execute(&mut *conn).await?;
    query("PRAGMA foreign_keys=ON").execute(&mut *conn).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iroha_client_wrap::ClientWrap;
    use iroha::crypto::KeyPair;
    use iroha::data_model::Level;
    use serde_json::json;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::{ConnectOptions, Connection};
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    /// This function automates the creation of `test_dump.sql`, and is meant to be run manually.
    ///
    /// **Prerequisites:**
    ///
    /// - Installed `sqlite3` (to make dumps);
    /// - Running one of the Docker Compose configs from Iroha repo, e.g.:
    ///
    /// ```sh
    /// docker-compose -f defaults/docker-compose.local.yml up --force-recreate
    /// ```
    ///
    /// When run, it fills Iroha with data, scans into an SQLite database, saves it into `test_dump_db.sqlite`,
    /// and dumps it into `src/repo/test_dump.sql`.
    ///
    /// The saved `.sqlite` file could be useful for debugging.
    #[ignore]
    #[tokio::test]
    async fn create_test_dump() -> Result<()> {
        // NOTE: Uncomment for troubleshooting
        // crate::init_test_logger();

        let db_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_dump_db.sqlite");
        let dump_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/repo/test_dump.sql");

        // copy of `defaults/client.toml` (in Iroha repo)
        let key_pair: KeyPair = serde_json::from_value(serde_json::json!({
            "public_key": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03",
            "private_key": "802620CCF31D85E3B32A4BEA59987CE0C78E3B8E2DB93881468AB2435FE45D5C9DCD53"
        }))?;
        let account = AccountId::new("wonderland".parse()?, key_pair.public_key().clone());
        let torii_api_url = "http://127.0.0.1:8080/".parse()?;
        let client = Client::new(iroha::config::Config {
            account,
            key_pair,
            torii_api_url,
            basic_auth: None,
            chain: ChainId::from("00000000-0000-0000-0000-000000000000"),
            transaction_add_nonce: false,
            transaction_status_timeout: Duration::from_secs(10),
            transaction_ttl: Duration::from_secs(300),
        });
        let client_wrap = ClientWrap::from(client.clone());

        debug!("Filling Iroha...");
        tokio::task::spawn_blocking(move || fill_iroha(&client)).await??;

        if db_path.exists() {
            debug!("Removing previous DB file");
            tokio::fs::remove_file(&db_path).await?;
        }

        let mut conn = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .connect()
            .await?;
        scan_into(&client_wrap, &mut conn).await?;
        redact_wasm_blobs(&mut conn).await?;
        conn.close().await?;
        sqlite3_dump(db_path, dump_path).await?;

        debug!("Dump is written!");

        Ok(())
    }

    async fn redact_wasm_blobs(conn: &mut SqliteConnection) -> Result<()> {
        sqlx::query(r#"update instructions set value = '{"Upgrade":"MHgwMDk5MjI="}' from json_each(instructions.value) where json_each.key = 'Upgrade'"#)
        .execute(conn)
        .await?;

        Ok(())
    }

    async fn sqlite3_dump(db_path: impl AsRef<Path>, dump_path: impl AsRef<Path>) -> Result<()> {
        let output = tokio::process::Command::new("sqlite3")
            .arg(db_path.as_ref())
            .arg(".dump")
            .output()
            .await?;

        let content = String::from_utf8(output.stdout)?;
        tokio::fs::write(dump_path, content.as_bytes()).await?;

        Ok(())
    }

    /// Goals:
    ///
    /// - Around 20-25 blocks
    /// - Several domains and accounts
    /// - Fungible and non-fungible assets
    /// - Successful and failed transactions
    /// - **All kinds of instructions**
    /// - Metadata for most of the entities
    fn fill_iroha(client: &Client) -> Result<()> {
        let acc1_key = KeyPair::from_seed(vec![0, 4, 1, 2, 5, 2, 5, 2, 1, 2], <_>::default());
        let acc1 = AccountId::new("wonderland".parse()?, acc1_key.public_key().clone());
        let acc2_key = KeyPair::from_seed(vec![0, 111, 222, 41, 1, 2], <_>::default());
        let acc2 = AccountId::new("looking_glass".parse()?, acc2_key.public_key().clone());

        client.submit_blocking(Register::domain(
            Domain::new("looking_glass".parse()?)
                .with_logo("/ipns/QmSrPmbaUKA3ZodhzPWZnpFgcPMFWF4QsxXbkWfEptTBJd".parse()?)
                .with_metadata(
                    Metadata::default()
                        .put("important_data".parse()?, json!(["secret-code", 1, 2, 3]))
                        .put(
                            "very_important_data".parse()?,
                            json!({"very":{"important":{"data":{"is":{"deep":{"inside":42}}}}}}),
                        ),
                ),
        ))?;

        client.submit_blocking(Register::account(
            Account::new(acc1.clone())
                .with_metadata(Metadata::default().put("alias".parse()?, json!("bob"))),
        ))?;
        client
            .submit_blocking(Register::account(Account::new(acc2.clone()).with_metadata(
                Metadata::default().put("alias".parse()?, json!("mad_hatter")),
            )))?;
        client.submit_blocking(Register::nft(Nft::new(
            "snowflake$wonderland".parse()?,
            Metadata::default().put("what-am-i".parse()?, json!("an nft, unique as a snowflake")),
        )))?;
        client.submit_blocking(SetKeyValue::account(
            client.account.clone(),
            "alias".parse()?,
            json!("alice"),
        ))?;
        client.submit_blocking(SetKeyValue::nft(
            "snowflake$wonderland".parse()?,
            "another-rather-unique-metadata-set-later".parse()?,
            json!([5, 1, 2, 3, 4]),
        ))?;
        let _ = client
            .submit_blocking(RemoveKeyValue::account(
                acc2.clone(),
                "non-existing".parse()?,
            ))
            .unwrap_err();
        client.submit_blocking(Mint::asset_numeric(
            100_123u64,
            AssetId::new("rose#wonderland".parse()?, acc1.clone()),
        ))?;
        client.submit_blocking(Burn::asset_numeric(
            123u64,
            AssetId::new("rose#wonderland".parse()?, acc1.clone()),
        ))?;
        client.submit_blocking(Transfer::nft(
            client.account.clone(),
            "snowflake$wonderland".parse()?,
            acc2.clone(),
        ))?;

        let _ = client
            .submit_blocking(Revoke::account_role(
                "RoleThatDoesNotExist".parse()?,
                acc1.clone(),
            ))
            .unwrap_err();
        client.submit_blocking(Log::new(
            Level::ERROR,
            "A disrupting message of sorts".to_owned(),
        ))?;
        let _ = client
            .submit_blocking(CustomInstruction::new(
                json!({ "kind": "custom", "value": false }),
            ))
            .unwrap_err();
        let _ = client
            .submit_blocking(ExecuteTrigger::new("ping".parse()?).with_args(&json!([
                "do this",
                "then this",
                "and that afterwards"
            ])))
            .unwrap_err();

        // let empty block to appear
        std::thread::sleep(Duration::from_secs(2));

        Ok(())
    }

    trait MetadataExt {
        fn put(self, key: Name, value: impl Into<iroha::data_model::prelude::Json>) -> Self;
    }

    impl MetadataExt for Metadata {
        fn put(mut self, key: Name, value: impl Into<iroha::data_model::prelude::Json>) -> Self {
            self.insert(key, value);
            self
        }
    }
}
