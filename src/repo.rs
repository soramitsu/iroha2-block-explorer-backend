//! Database operations

mod types;
mod util;

use crate::schema::Page;
use crate::schema::PaginationQueryParams;
use crate::util::{DirectPagination, ReversePagination, ReversePaginationError};
use iroha_data_model::prelude as data_model;
use nonzero_ext::nonzero;
use serde::Deserialize;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{
    prelude::{FromRow, Type},
    ConnectOptions, Connection, Database, Decode, Encode, QueryBuilder, Sqlite, SqliteConnection,
};
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::{num::NonZero, sync::Arc};
use tokio::sync::Mutex;
pub use types::*;
pub use util::*;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Bad pagination: {0}")]
    Pagination(#[from] ReversePaginationError),
}

type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Repo {
    conn: Arc<Mutex<SqliteConnection>>,
}

impl Repo {
    pub async fn new(opts: SqliteConnectOptions) -> Result<Self> {
        let conn = opts.connect().await?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn list_blocks(&self, pagination: PaginationQueryParams) -> Result<Page<Block>> {
        let mut conn = self.conn.lock().await;
        let (total,): (u64,) = QueryBuilder::new("with main as (")
            .push_custom(SelectBlocks {
                with_where: PushDisplay("true"),
            })
            .push(") select count(*) from main")
            .build_query_as()
            .fetch_one(&mut (*conn))
            .await?;
        let Some(total) = NonZero::new(total) else {
            return Ok(Page::empty(pagination.per_page));
        };
        let pagination = ReversePagination::new(total, pagination.per_page, pagination.page)?;
        let blocks = QueryBuilder::new("")
            .push_custom(SelectBlocks {
                with_where: PushDisplay("true"),
            })
            .push_custom(LimitOffset::from(pagination.range()))
            .build_query_as()
            .fetch_all(&mut (*conn))
            .await?;

        Ok(Page::new(blocks, pagination.into()))
    }

    pub async fn find_block(&self, params: FindBlockParams) -> Result<Block> {
        let mut conn = self.conn.lock().await;
        let item = QueryBuilder::new("")
            .push_custom(SelectBlocks {
                with_where: util::push_fn(|builder| {
                    match params {
                        FindBlockParams::Height(height) => {
                            builder.push("blocks.height = ").push_bind(height)
                        }
                        FindBlockParams::Hash(hash) => {
                            builder.push("blocks.hash like ").push_bind(AsText(hash))
                        }
                    };
                }),
            })
            .build_query_as()
            .fetch_one(&mut (*conn))
            .await?;
        Ok(item)
    }

    pub async fn list_transactions(
        &self,
        params: ListTransactionsParams,
    ) -> Result<Page<TransactionInList>> {
        let mut conn = self.conn.lock().await;

        let (total,): (u64,) = QueryBuilder::new("with main as (")
            .push_custom(SelectTransactions {
                with_where: &params,
                full: false,
            })
            .push(") select count(*) from main")
            .build_query_as()
            .fetch_one(&mut (*conn))
            .await?;
        let Some(total) = NonZero::new(total) else {
            return Ok(Page::empty(params.pagination.per_page));
        };
        let pagination = dbg!(ReversePagination::new(
            dbg!(total),
            params.pagination.per_page,
            params.pagination.page
        )?);

        let txs = QueryBuilder::new("with main as (")
            .push_custom(SelectTransactions {
                with_where: &params,
                full: false,
            })
            .push(") select * from main ")
            .push_custom(LimitOffset::from(pagination.range()))
            .build_query_as()
            .fetch_all(&mut (*conn))
            .await?;

        Ok(Page::new(txs, pagination.into()))
    }

    pub async fn find_transaction_by_hash(&self, hash: iroha_crypto::Hash) -> Result<Transaction> {
        let mut conn = self.conn.lock().await;
        let tx = QueryBuilder::new("")
            .push_custom(SelectTransactions {
                with_where: util::push_fn(|builder| {
                    builder.push("hash like ").push_bind(AsText(hash));
                }),
                full: true,
            })
            .build_query_as()
            .fetch_one(&mut (*conn))
            .await?;
        Ok(tx)
    }

    pub async fn list_instructions(
        &self,
        params: ListInstructionParams,
    ) -> Result<Page<InstructionInList>> {
        let mut conn = self.conn.lock().await;

        let mut builder = QueryBuilder::new("select count(*) from instructions");
        params.push_where(&mut builder);
        let (total,): (u64,) = builder.build_query_as().fetch_one(&mut (*conn)).await?;
        let Some(total) = NonZero::new(total) else {
            return Ok(Page::empty(params.pagination.per_page));
        };
        let pagination =
            ReversePagination::new(total, params.pagination.per_page, params.pagination.page)?;

        let mut builder = QueryBuilder::new("select transaction_hash, value from instructions");
        params.push_where(&mut builder);
        builder.push_custom(LimitOffset::from(pagination.range()));
        let items = builder.build_query_as().fetch_all(&mut (*conn)).await?;

        Ok(Page::new(items, pagination.into()))
    }

    pub async fn list_domains(&self, params: ListDomainParams) -> Result<Page<Domain>> {
        let mut conn = self.conn.lock().await;

        let (total,): (u64,) = QueryBuilder::new("with grouped as (")
            .push_custom(SelectDomains {
                with_where: &params,
            })
            .push(") select count(*) from grouped")
            .build_query_as()
            .fetch_one(&mut (*conn))
            .await?;
        let Some(total) = NonZero::new(total) else {
            return Ok(Page::empty(params.pagination.per_page));
        };
        let pagination = DirectPagination::new(
            params.pagination.page.unwrap_or(nonzero!(1u64)),
            params.pagination.per_page,
            total,
        );

        let res = QueryBuilder::new("with grouped as (")
            .push_custom(SelectDomains {
                with_where: &params,
            })
            .push(") select * from grouped")
            .push_custom(LimitOffset::from(pagination.range()))
            .build_query_as()
            .fetch_all(&mut (*conn))
            .await?;

        Ok(Page::new(res, pagination.into()))
    }

    pub async fn find_domain(&self, id: data_model::DomainId) -> Result<Domain> {
        let mut conn = self.conn.lock().await;
        let item = QueryBuilder::new("")
            .push_custom(SelectDomains {
                with_where: util::push_fn(|builder| {
                    builder.push("domains.name = ").push_bind(AsText(id));
                }),
            })
            .build_query_as()
            .fetch_one(&mut (*conn))
            .await?;
        Ok(item)
    }

    pub async fn list_accounts(&self, params: ListAccountsParams) -> Result<Page<Account>> {
        let mut conn = self.conn.lock().await;

        let (total,): (u64,) = QueryBuilder::new("with grouped as (")
            .push_custom(SelectAccounts {
                with_where: &params,
            })
            .push(") select count(*) from grouped")
            .build_query_as()
            .fetch_one(&mut (*conn))
            .await?;
        let Some(total) = NonZero::new(total) else {
            return Ok(Page::empty(params.pagination.per_page));
        };
        let pagination = dbg!(DirectPagination::new(
            params.pagination.page.unwrap_or(nonzero!(1u64)),
            params.pagination.per_page,
            total,
        ));

        let res = QueryBuilder::new("with grouped as (")
            .push_custom(SelectAccounts {
                with_where: &params,
            })
            .push(") select * from grouped")
            .push_custom(LimitOffset::from(pagination.range()))
            .build_query_as()
            .fetch_all(&mut (*conn))
            .await?;

        Ok(Page::new(res, pagination.into()))
    }

    pub async fn find_account(&self, id: data_model::AccountId) -> Result<Account> {
        let mut conn = self.conn.lock().await;
        let item = QueryBuilder::new("")
            .push_custom(SelectAccounts {
                with_where: util::push_fn(|builder| {
                    builder
                        .separated(" and ")
                        .push("accounts.signatory = ")
                        .push_bind_unseparated(id.signatory().to_string())
                        .push("accounts.domain = ")
                        .push_bind_unseparated(id.domain().to_string());
                }),
            })
            .build_query_as()
            .fetch_one(&mut (*conn))
            .await?;
        Ok(item)
    }

    // pub async fn list_assets_definitions(
    //     &self,
    //     params: ListAssetDefinitionParams,
    // ) -> Result<Page<AssetDefinition>> {
    // }
    //
    // pub async fn find_asset_definition(
    //     &self,
    //     id: AssetDefinitionId,
    // ) -> Result<Option<AssetDefinition>> {
    // }
    //
    // pub async fn list_assets(&self, params: ListAssetsParams) -> Result<Page<AssetDefinition>> {}
    //
    // pub async fn find_asset(&self, id: AssetId) -> Result<Option<AssetDefinition>> {}
}

struct SelectAccounts<W> {
    with_where: W,
}

impl<'a, W> PushCustom<'a> for SelectAccounts<W>
where
    W: PushCustom<'a>,
{
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        builder
            .push(
                "\
select format('%s@%s', accounts.signatory, accounts.domain) as id,
       accounts.metadata,
       count(distinct assets.definition_name)               as owned_assets,
       count(distinct domain_owners.domain)                 as owned_domains
from accounts
     left join assets on assets.owned_by_signatory = accounts.signatory
and assets.owned_by_domain = accounts.domain
     left join domain_owners on domain_owners.account_domain = accounts.domain
and domain_owners.account_signatory = accounts.signatory
where ",
            )
            .push_custom(self.with_where)
            .push(" group by accounts.signatory, accounts.domain ");
    }
}

pub enum FindBlockParams {
    Height(u32),
    Hash(iroha_crypto::Hash),
}

struct SelectBlocks<W> {
    with_where: W,
}

pub struct ListTransactionsParams {
    pub pagination: PaginationQueryParams,
    pub block_hash: Option<iroha_crypto::Hash>,
    pub authority: Option<data_model::AccountId>,
}

impl<'a, W> PushCustom<'a> for SelectBlocks<W>
where
    W: PushCustom<'a>,
{
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        builder
            .push(
                "\
SELECT
  blocks.hash,
  blocks.height,
  blocks.created_at,
  prev_block_hash,
  transactions_hash,
  consensus_estimation_ms,
  count(transactions.hash) as transactions_total,
  count(case 1 when transactions.error is not null then 1 else null end) as transactions_rejected
FROM
  blocks
JOIN
  transactions ON transactions.block_hash = blocks.hash
WHERE ",
            )
            .push_custom(self.with_where)
            .push(
                "
GROUP BY
  blocks.hash
ORDER BY
  blocks.height DESC
",
            );
    }
}

struct SelectTransactions<W> {
    with_where: W,
    full: bool,
}

impl<'a, W> PushCustom<'a> for SelectTransactions<W>
where
    W: PushCustom<'a>,
{
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        builder.push("select ");
        let mut select = builder.separated(", ");
        select
            .push("hash")
            .push("block_hash")
            .push("format('%s@%s', authority_signatory, authority_domain) as authority")
            .push("created_at")
            .push("instructions");
        if self.full {
            select
                .push("metadata")
                .push("nonce")
                .push("time_to_live_ms")
                .push("signature")
                .push("error")
        } else {
            select.push("error is not null as error")
        };
        builder
            .push(" from transactions where ")
            .push_custom(self.with_where)
            .push(" order by created_at desc");
    }
}

pub struct ListDomainParams {
    pub pagination: PaginationQueryParams,
    pub owned_by: Option<data_model::AccountId>,
}

struct SelectDomains<W> {
    with_where: W,
}

impl<'a, W: PushCustom<'a>> PushCustom<'a> for SelectDomains<W> {
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        builder
            .push(
                "\
select
    domains.name as id,
    domains.logo,
    domains.metadata,
    format('%s@%s', account_signatory, account_domain) as owned_by,
    count(distinct accounts.signatory) as accounts,
    count(distinct asset_definitions.name) as assets
from domains
         join domain_owners on domain_owners.domain = domains.name
left join accounts on accounts.domain = domains.name
left join asset_definitions on asset_definitions.domain = domains.name
where ",
            )
            .push_custom(self.with_where)
            .push(" group by domains.name");
    }
}

pub struct ListAccountsParams {
    pub pagination: PaginationQueryParams,
    pub with_asset: Option<data_model::AssetDefinitionId>,
    pub domain: Option<data_model::DomainId>,
}

pub struct ListAssetDefinitionParams {
    pub pagination: PaginationQueryParams,
    pub domain: Option<DomainId>,
    pub owned_by: Option<AccountId>,
}

pub struct ListAssetsParams {
    pub pagination: PaginationQueryParams,
    pub owned_by: Option<AccountId>,
    pub definition: Option<AssetDefinitionId>,
}

pub struct ListInstructionParams {
    pub pagination: PaginationQueryParams,
    pub transaction_hash: Option<Hash>,
}

impl ListInstructionParams {
    fn push_where<'a>(&'a self, builder: &mut QueryBuilder<'a, Sqlite>) {
        if let Some(hash) = &self.transaction_hash {
            builder.push(" where transaction_hash = ").push_bind(hash);
        }
    }
}
