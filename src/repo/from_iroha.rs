use crate::repo::{AsText, SignatureDisplay};
use chrono::DateTime;
use eyre::Result;
use iroha::client::Client;
use iroha::data_model::prelude::*;
use serde_json::json;
use sqlx::types::Json;
use sqlx::{query, QueryBuilder, SqliteConnection};
use tokio::task::spawn_blocking;
use tracing::debug;

/// Scan Iroha into an `SQLite` database.
#[allow(clippy::too_many_lines)]
pub async fn scan_into(client: &Client, conn: &mut SqliteConnection) -> Result<()> {
    debug!("Scanning Iroha into an in-memory SQLite database");

    debug!("Fetching data from Iroha...");
    let client = client.clone();
    let (domains, accounts, blocks, assets_definitions, assets) = spawn_blocking(move || {
        let domains = client.query(FindDomains).execute_all()?;
        let accounts = client.query(FindAccounts).execute_all()?;
        let blocks = client.query(FindBlocks).execute_all()?;
        let assets_definitions = client.query(FindAssetsDefinitions).execute_all()?;
        let assets = client.query(FindAssets).execute_all()?;
        Ok::<_, eyre::Report>((domains, accounts, blocks, assets_definitions, assets))
    })
    .await??;
    debug!("Done fetching");

    query(include_str!("./create_tables.sql"))
        .execute(&mut *conn)
        .await?;
    query("PRAGMA foreign_keys=OFF").execute(&mut *conn).await?;
    query("BEGIN TRANSACTION").execute(&mut *conn).await?;

    // todo: handle empty data
    debug!("Inserting domains & accounts...");
    QueryBuilder::new("insert into domains(name, logo, metadata) ")
        .push_values(&domains, |mut b, value| {
            b.push_bind(value.id().name().as_ref())
                .push_bind(value.logo().as_ref().map(AsText))
                .push_bind(Json(value.metadata()));
        })
        .build()
        .execute(&mut *conn)
        .await?;
    QueryBuilder::new("insert into accounts(signatory, domain, metadata) ")
        .push_values(&accounts, |mut b, value| {
            b.push_bind(AsText(value.signatory()))
                .push_bind(AsText(value.id().domain()))
                .push_bind(Json(value.metadata()));
        })
        .build()
        .execute(&mut *conn)
        .await?;
    QueryBuilder::new("insert into domain_owners(account_signatory, account_domain, domain) ")
        .push_values(&domains, |mut b, value| {
            b.push_bind(AsText(value.owned_by().signatory()))
                .push_bind(AsText(value.owned_by().domain()))
                .push_bind(AsText(value.id()));
        })
        .build()
        .execute(&mut *conn)
        .await?;

    // TODO: handle empty blocks, txs, isis
    debug!("Inserting blocks, transactions, instructions...");
    let mut b = QueryBuilder::new("insert into blocks(");
    let mut sep = b.separated(", ");
    for i in [
        "hash",
        "height",
        "created_at",
        "prev_block_hash",
        "transactions_hash",
    ] {
        sep.push(i);
    }
    b.push(") ")
        .push_values(&blocks, |mut b, value| {
            b.push_bind(AsText(value.hash()))
                .push_bind(value.header().height().get() as i32)
                .push_bind(DateTime::from_timestamp_millis(
                    value.header().creation_time().as_millis() as i64,
                ))
                .push_bind(value.header().prev_block_hash().map(AsText))
                .push_bind(AsText(value.header().transactions_hash()));
        })
        .build()
        .execute(&mut *conn)
        .await?;

    let mut b = QueryBuilder::new("insert into transactions(");
    let mut sep = b.separated(", ");
    for i in &[
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
    ] {
        sep.push(i);
    }
    b.push(") ")
        .push_values(
            blocks.iter().flat_map(|block| {
                block
                    .transactions()
                    .enumerate()
                    .map(move |(i, tx)| (block, tx, i))
            }),
            |mut b, (block, tx, tx_index)| {
                let error = block.error(tx_index);
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
        .build()
        .execute(&mut *conn)
        .await?;

    QueryBuilder::new("insert into instructions(transaction_hash, value) ")
        .push_values(
            blocks.iter().flat_map(|block| {
                block
                    .transactions()
                    .filter_map(|tx| match tx.instructions() {
                        Executable::Instructions(isi_vec) => {
                            Some(isi_vec.iter().map(|isi| (tx.hash(), isi)))
                        }
                        Executable::Wasm(_) => None,
                    })
                    .flatten()
            }),
            |mut b, (tx_hash, value)| {
                let json = match value {
                    // https://github.com/hyperledger-iroha/iroha/issues/5305
                    InstructionBox::Log(log) => Json(json!({
                        "Log": {
                            "level": log.level(),
                            "msg": "[Message could not be extracted at this moment]"
                        }
                    })),
                    other => Json(json!(other)),
                };
                // dbg!(&(tx_hash, value));
                b.push_bind(AsText(tx_hash)).push_bind(json);
            },
        )
        .build()
        .execute(&mut *conn)
        .await?;

    if !assets_definitions.is_empty() {
        debug!("Inserting assets...");
        let mut b = QueryBuilder::new("insert into asset_definitions(");
        b.separated(", ")
            .push("name")
            .push("domain")
            .push("metadata")
            .push("mintable")
            .push("owned_by_signatory")
            .push("owned_by_domain")
            .push("logo")
            .push("type");
        b.push(") ")
            .push_values(&assets_definitions, |mut b, value| {
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
                    .push_bind(value.logo().as_ref().map(AsText))
                    .push_bind(match &value.type_() {
                        AssetType::Store => "Store",
                        AssetType::Numeric(_) => "Numeric",
                    });
            })
            .build()
            .execute(&mut *conn)
            .await?;
        let mut b = QueryBuilder::new("insert into assets(");
        b.separated(", ")
            .push("definition_name")
            .push("definition_domain")
            .push("owned_by_signatory")
            .push("owned_by_domain")
            .push("value");
        b.push(") ")
            .push_values(&assets, |mut b, value| {
                b.push_bind(AsText(value.id().definition().name()))
                    .push_bind(AsText(value.id().definition().domain()))
                    .push_bind(AsText(value.id().account().signatory()))
                    .push_bind(AsText(value.id().account().domain()))
                    .push_bind(match value.value() {
                        AssetValue::Store(metadata) => Json(json!({
                            "kind": "Store",
                            "value": metadata
                        })),
                        AssetValue::Numeric(num) => Json(json!({
                            "kind": "Numeric",
                            "value": num
                        })),
                    });
            })
            .build()
            .execute(&mut *conn)
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
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::{ConnectOptions, Connection};
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    // This is more for the development of scanning rather than for its testing
    #[ignore]
    #[tokio::test]
    async fn sandbox() {
        let key_pair: KeyPair = serde_json::from_value(serde_json::json!({
            "public_key": "ed0120B23E14F659B91736AAB980B6ADDCE4B1DB8A138AB0267E049C082A744471714E",
            "private_key": "802620E28031CC65994ADE240E32FCFD0405DF30A47BDD6ABAF76C8C3C5A4F3DE96F75"
        }))
        .unwrap();

        let client = ClientWrap::new(
            AccountId::new("wonderland".parse().unwrap(), key_pair.public_key().clone()),
            key_pair,
            "http://fujiwara.sora.org/v5/".parse().unwrap(),
        );

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "iroha_explorer=debug,sqlx=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        let mut conn = SqliteConnectOptions::new()
            .in_memory(true)
            // .filename("./sandbox.sqlite")
            .connect()
            .await
            .unwrap();
        scan_into(&client, &mut conn)
            .await
            .expect("should scan without errors");

        conn.close().await.unwrap();
    }
}
