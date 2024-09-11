use crate::iroha::Client;
use crate::repo::{AsText, SignatureDisplay};
use chrono::DateTime;
use color_eyre::SectionExt;
use eyre::Result;
use iroha_data_model::prelude::*;
use serde_json::json;
use sqlx::types::Json;
use sqlx::{query, Connection, QueryBuilder, SqliteConnection};
use tracing::debug;

pub async fn scan(client: &Client) -> Result<SqliteConnection> {
    debug!("Scanning Iroha into an in-memory SQLite database");

    debug!("Fetching data from Iroha...");
    let domains = client.query(FindDomains).all().await?;
    let accounts = client.query(FindAccounts).all().await?;
    let blocks = client.query(FindBlocks).all().await?;
    let assets_definitions = client.query(FindAssetsDefinitions).all().await?;
    let assets = client.query(FindAssets).all().await?;
    debug!("Done fetching");

    let mut conn = SqliteConnection::connect("sqlite::memory:").await?;
    query(include_str!("./create_tables.sql"))
        .execute(&mut conn)
        .await?;
    // query("PRAGMA foreign_keys=OFF").execute(&mut conn).await?;
    query("BEGIN TRANSACTION").execute(&mut conn).await?;

    /// todo: handle empty data
    debug!("Inserting domains & accounts...");
    QueryBuilder::new("insert into domains(name, logo, metadata) ")
        .push_values(&domains, |mut b, value| {
            b.push_bind(value.id().name().as_ref())
                .push_bind(value.logo().as_ref().map(AsText))
                .push_bind(Json(value.metadata()));
        })
        .build()
        .execute(&mut conn)
        .await?;
    QueryBuilder::new("insert into accounts(signatory, domain, metadata) ")
        .push_values(&accounts, |mut b, value| {
            b.push_bind(AsText(value.signatory()))
                .push_bind(AsText(value.id().domain()))
                .push_bind(Json(value.metadata()));
        })
        .build()
        .execute(&mut conn)
        .await?;
    QueryBuilder::new("insert into domain_owners(account_signatory, account_domain, domain) ")
        .push_values(&domains, |mut b, value| {
            b.push_bind(AsText(value.owned_by().signatory()))
                .push_bind(AsText(value.owned_by().domain()))
                .push_bind(AsText(value.id()));
        })
        .build()
        .execute(&mut conn)
        .await?;

    /// TODO: handle empty blocks, txs, isis
    debug!("Inserting blocks, transactions, instructions...");
    let mut b = QueryBuilder::new("insert into blocks(");
    let mut sep = b.separated(", ");
    for i in [
        "hash",
        "height",
        "created_at",
        "consensus_estimation_ms",
        // "view_change_index",
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
                .push_bind(value.header().consensus_estimation().as_millis() as i64)
                // .push_bind(value.header().view_change_index())
                .push_bind(value.header().prev_block_hash().map(AsText))
                .push_bind(AsText(value.header().transactions_hash()));
        })
        .build()
        .execute(&mut conn)
        .await?;

    let mut b = QueryBuilder::new("insert into transactions(");
    let mut sep = b.separated(", ");
    [
        "hash",
        "block_hash",
        "created_at",
        "time_to_live_ms",
        "authority_signatory",
        "authority_domain",
        "signature",
        "nonce",
        "metadata",
        "error",
        "instructions",
    ]
    .iter()
    .for_each(|i| {
        sep.push(i);
    });
    b.push(") ")
        .push_values(
            blocks
                .iter()
                .flat_map(|block| block.transactions().map(|tx| (block.hash(), tx))),
            |mut b, (block_hash, value)| {
                let error = value.error();
                let value = &value.value;
                b.push_bind(AsText(value.hash()))
                    .push_bind(AsText(block_hash))
                    .push_bind(DateTime::from_timestamp_millis(
                        value.creation_time().as_millis() as i64,
                    ))
                    .push_bind(value.time_to_live().map(|dur| dur.as_millis() as i64))
                    .push_bind(AsText(value.authority().signatory()))
                    .push_bind(AsText(value.authority().domain()))
                    .push_bind(AsText(SignatureDisplay(
                        value.signature().payload().clone(),
                    )))
                    .push_bind(value.nonce().map(|num| num.get() as i64))
                    .push_bind(Json(value.metadata()))
                    .push_bind(error.as_ref().map(Json))
                    .push_bind(match value.instructions() {
                        Executable::Instructions(_) => "Instructions",
                        Executable::Wasm(_) => "WASM",
                    });
            },
        )
        .build()
        .execute(&mut conn)
        .await?;

    QueryBuilder::new("insert into instructions(transaction_hash, value) ")
        .push_values(
            blocks.iter().flat_map(|block| {
                block
                    .transactions()
                    .flat_map(|tx| match &tx.value.instructions() {
                        Executable::Instructions(isi) => {
                            Some(isi.iter().map(|i| (tx.value.hash(), i)))
                        }
                        Executable::Wasm(_) => None,
                    })
                    .flatten()
            }),
            |mut b, (tx_hash, value)| {
                b.push_bind(AsText(tx_hash)).push_bind(Json(value));
            },
        )
        .build()
        .execute(&mut conn)
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
                    .push_bind(match &value.mintable {
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
            .execute(&mut conn)
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
                    .push_bind(match &value.value {
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
            .execute(&mut conn)
            .await?;
    }

    query("COMMIT").execute(&mut conn).await?;
    // query("PRAGMA foreign_keys=ON").execute(&mut conn).await?;

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroha_crypto::KeyPair;
    use iroha_data_model::prelude::{AccountId, DomainId};
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    // This is more for the development of scanning rather than for its testing
    #[tokio::test]
    async fn sandbox() {
        let key_pair: KeyPair = serde_json::from_value(serde_json::json!({
            "public_key": "ed0120B23E14F659B91736AAB980B6ADDCE4B1DB8A138AB0267E049C082A744471714E",
            "private_key": "802620E28031CC65994ADE240E32FCFD0405DF30A47BDD6ABAF76C8C3C5A4F3DE96F75"
        }))
        .unwrap();

        let client = Client::new(
            AccountId::new("wonderland".parse().unwrap(), key_pair.public_key().clone()),
            key_pair,
            "http://localhost:8080".parse().unwrap(),
        );

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "iroha_explorer=debug,sqlx=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        let _conn = scan(&client).await.expect("should scan without errors");
    }
}
