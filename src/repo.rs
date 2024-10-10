//! Database operations

mod from_iroha;
mod types;
mod util;

use crate::schema::{InstructionKind, Page};
use crate::schema::{PaginationQueryParams, TransactionStatus};
use crate::util::{DirectPagination, ReversePagination, ReversePaginationError};
pub use from_iroha::scan_into as scan_iroha;
use iroha_data_model::prelude as data_model;
use nonzero_ext::nonzero;
use sqlx::{QueryBuilder, Sqlite, SqliteConnection};
use std::{num::NonZero, sync::Arc};
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard, Notify};
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
    conn: Arc<Mutex<Option<SqliteConnection>>>,
    available: Arc<Notify>,
}

impl Repo {
    pub fn new(conn: Option<SqliteConnection>) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
            available: Arc::new(Notify::new()),
        }
    }

    pub async fn swap(&self, conn: SqliteConnection) {
        let mut guard = self.conn.lock().await;
        *guard = Some(conn);
        self.available.notify_waiters();
    }

    pub async fn list_blocks(&self, pagination: PaginationQueryParams) -> Result<Page<Block>> {
        let mut conn = self.acquire_conn().await;
        let (total,): (u64,) = QueryBuilder::new("with main as (")
            .push_custom(SelectBlocks {
                with_where: PushDisplay("true"),
            })
            .push(") select count(*) from main")
            .build_query_as()
            .fetch_one(&mut *conn)
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
            .fetch_all(&mut *conn)
            .await?;

        Ok(Page::new(blocks, pagination.into()))
    }

    pub async fn find_block(&self, params: FindBlockParams) -> Result<Block> {
        let mut conn = self.acquire_conn().await;
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
            .fetch_one(&mut *conn)
            .await?;
        Ok(item)
    }

    pub async fn list_transactions(
        &self,
        params: ListTransactionsParams,
    ) -> Result<Page<TransactionBase>> {
        let mut conn = self.acquire_conn().await;

        let (total,): (u64,) = QueryBuilder::new("with main as (")
            .push_custom(SelectTransactions {
                with_where: &params,
                detailed: false,
            })
            .push(") select count(*) from main")
            .build_query_as()
            .fetch_one(&mut *conn)
            .await?;
        let Some(total) = NonZero::new(total) else {
            return Ok(Page::empty(params.pagination.per_page));
        };
        let pagination =
            ReversePagination::new(total, params.pagination.per_page, params.pagination.page)?;

        let txs = QueryBuilder::new("with main as (")
            .push_custom(SelectTransactions {
                with_where: &params,
                detailed: false,
            })
            .push(") select * from main ")
            .push_custom(LimitOffset::from(pagination.range()))
            .build_query_as()
            .fetch_all(&mut *conn)
            .await?;

        Ok(Page::new(txs, pagination.into()))
    }

    pub async fn find_transaction_by_hash(
        &self,
        hash: iroha_crypto::Hash,
    ) -> Result<TransactionDetailed> {
        let mut conn = self.acquire_conn().await;
        let tx = QueryBuilder::new("")
            .push_custom(SelectTransactions {
                with_where: util::push_fn(|builder| {
                    builder.push("hash like ").push_bind(AsText(hash));
                }),
                detailed: true,
            })
            .build_query_as()
            .fetch_one(&mut *conn)
            .await?;
        Ok(tx)
    }

    pub async fn list_instructions(
        &self,
        params: ListInstructionParams,
    ) -> Result<Page<Instruction>> {
        let mut conn = self.acquire_conn().await;

        let (total,): (u64,) = QueryBuilder::new("with main as (")
            .push_custom(SelectInstructions { params: &params })
            .push(") select count(*) from main")
            .build_query_as()
            .fetch_one(&mut *conn)
            .await?;
        let Some(total) = NonZero::new(total) else {
            return Ok(Page::empty(params.pagination.per_page));
        };
        let pagination =
            ReversePagination::new(total, params.pagination.per_page, params.pagination.page)?;

        let items = QueryBuilder::new("with main as (")
            .push_custom(SelectInstructions { params: &params })
            .push(") select * from main ")
            .push_custom(LimitOffset::from(pagination.range()))
            .build_query_as()
            .fetch_all(&mut *conn)
            .await?;

        Ok(Page::new(items, pagination.into()))
    }

    pub async fn list_domains(&self, params: ListDomainParams) -> Result<Page<Domain>> {
        let mut conn = self.acquire_conn().await;

        let (total,): (u64,) = QueryBuilder::new("with grouped as (")
            .push_custom(SelectDomains {
                with_where: &params,
            })
            .push(") select count(*) from grouped")
            .build_query_as()
            .fetch_one(&mut *conn)
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
            .fetch_all(&mut *conn)
            .await?;

        Ok(Page::new(res, pagination.into()))
    }

    pub async fn find_domain(&self, id: data_model::DomainId) -> Result<Domain> {
        let mut conn = self.acquire_conn().await;
        let item = QueryBuilder::new("")
            .push_custom(SelectDomains {
                with_where: util::push_fn(|builder| {
                    builder.push("domains.name = ").push_bind(AsText(id));
                }),
            })
            .build_query_as()
            .fetch_one(&mut *conn)
            .await?;
        Ok(item)
    }

    pub async fn list_accounts(&self, params: ListAccountsParams) -> Result<Page<Account>> {
        let mut conn = self.acquire_conn().await;

        let (total,): (u64,) = QueryBuilder::new("with grouped as (")
            .push_custom(SelectAccounts {
                with_where: &params,
            })
            .push(") select count(*) from grouped")
            .build_query_as()
            .fetch_one(&mut *conn)
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
            .push_custom(SelectAccounts {
                with_where: &params,
            })
            .push(") select * from grouped")
            .push_custom(LimitOffset::from(pagination.range()))
            .build_query_as()
            .fetch_all(&mut *conn)
            .await?;

        Ok(Page::new(res, pagination.into()))
    }

    pub async fn find_account(&self, id: data_model::AccountId) -> Result<Account> {
        let mut conn = self.acquire_conn().await;
        let item = QueryBuilder::new("")
            .push_custom(SelectAccounts {
                with_where: util::push_fn(|builder| {
                    builder
                        .separated(" and ")
                        .push("accounts.signatory like  ")
                        .push_bind_unseparated(id.signatory().to_string())
                        .push("accounts.domain = ")
                        .push_bind_unseparated(id.domain().to_string());
                }),
            })
            .build_query_as()
            .fetch_one(&mut *conn)
            .await?;
        Ok(item)
    }

    pub async fn list_assets_definitions(
        &self,
        params: ListAssetDefinitionParams,
    ) -> Result<Page<AssetDefinition>> {
        let mut conn = self.acquire_conn().await;
        let (total,): (u64,) = QueryBuilder::new("with main as (")
            .push_custom(SelectAssetsDefinitions {
                with_where: &params,
            })
            .push(") select count(*) from main")
            .build_query_as()
            .fetch_one(&mut *conn)
            .await?;
        let Some(total) = NonZero::new(total) else {
            return Ok(Page::empty(params.pagination.per_page));
        };
        let pagination = DirectPagination::new(
            params.pagination.page.unwrap_or(nonzero!(1u64)),
            params.pagination.per_page,
            total,
        );
        let items = QueryBuilder::new("with main as (")
            .push_custom(SelectAssetsDefinitions {
                with_where: &params,
            })
            .push(") select * from main ")
            .push_custom(LimitOffset::from(pagination.range()))
            .build_query_as()
            .fetch_all(&mut *conn)
            .await?;
        Ok(Page::new(items, pagination.into()))
    }

    pub async fn find_asset_definition(
        &self,
        id: data_model::AssetDefinitionId,
    ) -> Result<AssetDefinition> {
        Ok(QueryBuilder::new("")
            .push_custom(SelectAssetsDefinitions {
                with_where: push_fn(|builder| {
                    let mut sep = builder.separated(" and ");
                    sep.push("asset_definitions.name = ")
                        .push_bind_unseparated(AsText(id.name()))
                        .push("asset_definitions.domain = ")
                        .push_bind_unseparated(AsText(id.domain()));
                }),
            })
            .build_query_as()
            .fetch_one(&mut *(self.acquire_conn().await))
            .await?)
    }

    pub async fn list_assets(&self, params: ListAssetsParams) -> Result<Page<Asset>> {
        let mut conn = self.acquire_conn().await;
        let (total,): (u64,) = QueryBuilder::new("with main as (")
            .push_custom(SelectAssets {
                with_where: &params,
            })
            .push(") select count(*) from main")
            .build_query_as()
            .fetch_one(&mut *conn)
            .await?;
        let Some(total) = NonZero::new(total) else {
            return Ok(Page::empty(params.pagination.per_page));
        };
        let pagination = DirectPagination::new(
            params.pagination.page.unwrap_or(nonzero!(1u64)),
            params.pagination.per_page,
            total,
        );
        let items = QueryBuilder::new("with main as (")
            .push_custom(SelectAssets {
                with_where: &params,
            })
            .push(") select * from main ")
            .push_custom(LimitOffset::from(pagination.range()))
            .build_query_as()
            .fetch_all(&mut *conn)
            .await?;
        Ok(Page::new(items, pagination.into()))
    }

    pub async fn find_asset(&self, id: data_model::AssetId) -> Result<Asset> {
        Ok(QueryBuilder::new("")
            .push_custom(SelectAssets {
                with_where: push_fn(|builder| {
                    builder
                        .separated(" and ")
                        .push("owned_by_signatory like  ")
                        .push_bind_unseparated(AsText(id.account().signatory()))
                        .push("owned_by_domain = ")
                        .push_bind_unseparated(id.account().domain().name().as_ref())
                        .push("definition_name = ")
                        .push_bind_unseparated(id.definition().name().as_ref())
                        .push("definition_domain = ")
                        .push_bind_unseparated(id.definition().domain().name().as_ref());
                }),
            })
            .build_query_as()
            .fetch_one(&mut *(self.acquire_conn().await))
            .await?)
    }

    async fn acquire_conn(&self) -> MappedMutexGuard<'_, SqliteConnection> {
        loop {
            let guard = self.conn.lock().await;
            match MutexGuard::try_map(guard, Option::as_mut) {
                Ok(mapped) => return mapped,
                // dropping
                Err(_guard) => {}
            }
            self.available.notified().await;
        }
    }
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
     left join assets on assets.owned_by_signatory like  accounts.signatory
and assets.owned_by_domain = accounts.domain
     left join domain_owners on domain_owners.account_domain = accounts.domain
and domain_owners.account_signatory like  accounts.signatory
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
  count(transactions.hash) as transactions_total,
  count(case 1 when transactions.error is not null then 1 else null end) as transactions_rejected
FROM
  blocks
JOIN
  transactions ON transactions.block = blocks.height
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

pub struct ListTransactionsParams {
    pub pagination: PaginationQueryParams,
    pub block: Option<u64>,
    pub authority: Option<data_model::AccountId>,
    pub status: Option<TransactionStatus>,
}

impl<'a> PushCustom<'a> for &'a ListTransactionsParams {
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        let mut sep = builder.separated(" and ");
        sep.push("true");
        if let Some(height) = self.block {
            sep.push("block = ").push_bind_unseparated(height as u32);
        }
        if let Some(id) = &self.authority {
            sep.push("authority like  ")
                .push_bind_unseparated(AsText(id));
        }
        if let Some(status) = self.status {
            sep.push("status = ").push_bind_unseparated(status);
        }
    }
}

struct SelectTransactions<W> {
    with_where: W,
    detailed: bool,
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
            .push("block")
            .push("authority")
            .push("created_at")
            .push("executable")
            .push("status");

        if self.detailed {
            select
                .push("metadata")
                .push("nonce")
                .push("time_to_live_ms")
                .push("signature")
                .push("error as rejection_reason")
        } else {
            select.push("error is not null as error")
        };
        builder
            .push(" from v_transactions where ")
            .push_custom(self.with_where)
            .push(" order by created_at desc");
    }
}

pub struct ListDomainParams {
    pub pagination: PaginationQueryParams,
    pub owned_by: Option<data_model::AccountId>,
}

impl<'a> PushCustom<'a> for &'a ListDomainParams {
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        let mut sep = builder.separated(" and ");
        sep.push("true");
        if let Some(id) = &self.owned_by {
            sep.push("domain_owners.account_signatory like  ")
                .push_bind_unseparated(id.signatory().to_string())
                .push("domain_owners.account_domain = ")
                .push_bind_unseparated(id.domain().name().as_ref());
        }
    }
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

impl<'a> PushCustom<'a> for &'a ListAccountsParams {
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        let mut sep = builder.separated(" and ");
        sep.push("true");
        if let Some(id) = &self.with_asset {
            sep.push("assets.owned_by_signatory like  accounts.signatory")
                .push("assets.owned_by_domain = accounts.domain")
                .push("assets.definition_name = ")
                .push_bind_unseparated(id.name().as_ref())
                .push("assets.definition_domain = ")
                .push_bind_unseparated(id.domain().name().as_ref());
        }
        if let Some(domain) = &self.domain {
            sep.push("accounts.domain = ")
                .push_bind_unseparated(domain.name().as_ref());
        }
    }
}

#[derive(Copy, Clone)]
pub struct SelectAssetsDefinitions<W> {
    pub with_where: W,
}

impl<'a, W: PushCustom<'a>> PushCustom<'a> for SelectAssetsDefinitions<W> {
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        builder.push("\
select format('%s#%s', name, domain)                                                            as id,
       format('%s@%s', asset_definitions.owned_by_signatory, asset_definitions.owned_by_domain) as owned_by,
       logo,
       metadata,
       mintable,
       type,
       count(assets.definition_name)                                                            as assets
from asset_definitions
         left join assets on asset_definitions.name = assets.definition_name and
                             asset_definitions.domain = assets.definition_domain
where ")
            .push_custom(self.with_where).push(" group by asset_definitions.name, asset_definitions.domain");
    }
}

pub struct ListAssetDefinitionParams {
    pub pagination: PaginationQueryParams,
    pub domain: Option<data_model::DomainId>,
    pub owned_by: Option<data_model::AccountId>,
}

impl<'a> PushCustom<'a> for &'a ListAssetDefinitionParams {
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        let mut sep = builder.separated(" and ");
        sep.push("true");
        if let Some(id) = &self.domain {
            sep.push("asset_definitions.domain = ")
                .push_bind_unseparated(id.name().as_ref());
        }
        if let Some(id) = &self.owned_by {
            sep.push("asset_definitions.owned_by_signatory like ")
                .push_bind_unseparated(AsText(id.signatory()))
                .push("asset_definitions.owned_by_domain = ")
                .push_bind_unseparated(id.domain().name().as_ref());
        }
    }
}

#[derive(Copy, Clone)]
pub struct SelectAssets<W> {
    pub with_where: W,
}

impl<'a, W: PushCustom<'a>> PushCustom<'a> for SelectAssets<W> {
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        builder.push("\
select case assets.definition_domain = assets.owned_by_domain
           when true then format('%s##%s@%s', assets.definition_name, assets.owned_by_signatory, assets.owned_by_domain)
           else format('%s#%s#%s@%s', assets.definition_name, assets.definition_domain, assets.owned_by_signatory,
                       assets.owned_by_domain) end as id,
       value
from assets
where ")
            .push_custom(self.with_where);
    }
}

pub struct ListAssetsParams {
    pub pagination: PaginationQueryParams,
    pub owned_by: Option<data_model::AccountId>,
    pub definition: Option<data_model::AssetDefinitionId>,
}

impl<'a> PushCustom<'a> for &'a ListAssetsParams {
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        let mut sep = builder.separated(" and ");
        sep.push("true");
        if let Some(id) = &self.owned_by {
            sep.push("owned_by_signatory like ")
                .push_bind_unseparated(AsText(id.signatory()))
                .push("owned_by_domain = ")
                .push_bind_unseparated(id.domain().name().as_ref());
        }
        if let Some(id) = &self.definition {
            sep.push("definition_name = ")
                .push_bind_unseparated(id.name().as_ref())
                .push("definition_domain = ")
                .push_bind_unseparated(id.domain().name().as_ref());
        }
    }
}

pub struct ListInstructionParams {
    pub pagination: PaginationQueryParams,
    pub transaction_hash: Option<iroha_crypto::Hash>,
    pub transaction_status: Option<TransactionStatus>,
    pub block: Option<u64>,
    pub kind: Option<InstructionKind>,
    pub authority: Option<data_model::AccountId>,
}

impl<'a> PushCustom<'a> for &'a ListInstructionParams {
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        let mut sep = builder.separated(" and ");
        sep.push("true");
        if let Some(hash) = &self.transaction_hash {
            sep.push("transaction_hash like ")
                .push_bind_unseparated(AsText(hash));
        }
        if let Some(kind) = &self.kind {
            sep.push("kind = ").push_bind_unseparated(kind);
        }
        if let Some(id) = &self.authority {
            sep.push("authority like ")
                .push_bind_unseparated(AsText(id));
        }
        if let Some(block) = self.block {
            sep.push("block = ").push_bind_unseparated(block as i64);
        }
        if let Some(status) = self.transaction_status {
            sep.push("transaction_status = ")
                .push_bind_unseparated(status);
        }
    }
}

#[derive(Copy, Clone)]
struct SelectInstructions<'a> {
    params: &'a ListInstructionParams,
}

impl<'a> PushCustom<'a> for SelectInstructions<'a> {
    fn push_custom(self, builder: &mut QueryBuilder<'a, Sqlite>) {
        builder
            .push("select * from v_instructions where ")
            .push_custom(self.params)
            .push(" order by created_at desc ");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::types::JsonValue;
    use sqlx::{query, query_as, Connection};

    async fn test_repo() -> Repo {
        let mut conn = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        query(include_str!("./repo/create_tables.sql"))
            .execute(&mut conn)
            .await
            .unwrap();
        query(include_str!("./repo/test-dump.sql"))
            .execute(&mut conn)
            .await
            .unwrap();

        Repo::new(Some(conn))
    }

    fn default_pagination() -> PaginationQueryParams {
        PaginationQueryParams {
            page: None,
            per_page: nonzero!(10u64),
        }
    }

    #[tokio::test]
    async fn list_txs() {
        let repo = test_repo().await;

        let txs = repo
            .list_transactions(ListTransactionsParams {
                pagination: PaginationQueryParams {
                    page: None,
                    per_page: nonzero!(5u64),
                },
                block: None,
                authority: None,
                status: None,
            })
            .await
            .unwrap();

        assert_eq!(txs.pagination.page.0, 18);
        assert_eq!(txs.pagination.per_page.0, 5);
        assert_eq!(txs.pagination.total_pages.0, 18);
        assert_eq!(txs.pagination.total_items.0, 90);
        assert_eq!(txs.items.len(), 5);
    }

    #[tokio::test]
    async fn filter_txs_by_block_hash() {
        let repo = test_repo().await;

        let txs = repo
            .list_transactions(ListTransactionsParams {
                pagination: default_pagination(),
                block: Some(1),
                authority: None,
                status: None,
            })
            .await
            .unwrap();

        assert_eq!(txs.pagination.page.0, 1);
        assert_eq!(txs.pagination.total_pages.0, 1);
        assert_eq!(txs.pagination.total_items.0, 3);
        assert_eq!(txs.items.len(), 3);
    }

    #[tokio::test]
    async fn filter_txs_by_status() {
        let repo = test_repo().await;

        let data = repo
            .list_transactions(ListTransactionsParams {
                pagination: default_pagination(),
                block: None,
                authority: None,
                status: Some(TransactionStatus::Rejected),
            })
            .await
            .unwrap();

        assert_eq!(data.pagination.total_items.0, 60);
        assert!(data
            .items
            .iter()
            .all(|x| x.status == TransactionStatus::Rejected));

        let data = repo
            .list_transactions(ListTransactionsParams {
                pagination: default_pagination(),
                block: None,
                authority: None,
                status: Some(TransactionStatus::Committed),
            })
            .await
            .unwrap();

        assert_eq!(data.pagination.total_items.0, 30);
        assert!(data
            .items
            .iter()
            .all(|x| x.status == TransactionStatus::Committed));
    }

    #[tokio::test]
    async fn filter_isi_by_kind() {
        let repo = test_repo().await;

        let data = repo
            .list_instructions(ListInstructionParams {
                pagination: default_pagination(),
                transaction_hash: None,
                transaction_status: None,
                block: None,
                kind: Some(InstructionKind::Transfer),
                authority: None,
            })
            .await
            .unwrap();

        assert_eq!(data.pagination.page.0, 2);
        assert_eq!(data.pagination.total_pages.0, 2);
        assert_eq!(data.pagination.total_items.0, 15);
        assert_eq!(data.items.len(), 15);
        assert!(data
            .items
            .iter()
            .all(|x| x.kind == InstructionKind::Transfer));
    }

    #[tokio::test]
    async fn filter_isi_by_kind_and_authority() {
        let repo = test_repo().await;
        let account_id: data_model::AccountId =
            "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis"
                .parse()
                .unwrap();

        let data = repo
            .list_instructions(ListInstructionParams {
                pagination: default_pagination(),
                transaction_hash: None,
                transaction_status: None,
                block: None,
                kind: Some(InstructionKind::Register),
                authority: Some(account_id.clone()),
            })
            .await
            .unwrap();

        assert_eq!(data.items.len(), 3);
        assert_eq!(data.pagination.total_pages.0, 1);
        assert!(data
            .items
            .iter()
            .all(|x| x.kind == InstructionKind::Register && x.authority.0 .0 == account_id));
    }

    #[tokio::test]
    async fn filter_isi_by_block() {
        let repo = test_repo().await;

        let data = repo
            .list_instructions(ListInstructionParams {
                pagination: default_pagination(),
                transaction_hash: None,
                transaction_status: None,
                block: Some(6),
                kind: None,
                authority: None,
            })
            .await
            .unwrap();

        assert_eq!(data.items.len(), 1);
    }

    #[tokio::test]
    async fn filter_isi_by_status() {
        let repo = test_repo().await;

        let data = repo
            .list_instructions(ListInstructionParams {
                pagination: default_pagination(),
                transaction_hash: None,
                transaction_status: Some(TransactionStatus::Committed),
                block: None,
                kind: None,
                authority: None,
            })
            .await
            .unwrap();

        dbg!(data);
    }

    #[tokio::test]
    async fn list_blocks() {
        let repo = test_repo().await;

        let data = repo.list_blocks(default_pagination()).await.unwrap();

        assert_eq!(data.pagination.total_pages.0, 3);
        assert_eq!(data.pagination.total_items.0, 21);
    }

    #[tokio::test]
    async fn list_assets() {
        let repo = test_repo().await;

        let data = repo
            .list_assets(ListAssetsParams {
                pagination: default_pagination(),
                owned_by: None,
                definition: None,
            })
            .await
            .unwrap();

        assert_eq!(data.items.len(), 1);
    }

    #[tokio::test]
    async fn json_payloads_in_v_instructions() -> eyre::Result<()> {
        let mut conn = SqliteConnection::connect("sqlite::memory:").await?;
        query(include_str!("./repo/create_tables.sql"))
            .execute(&mut conn)
            .await?;
        query("PRAGMA foreign_keys=OFF").execute(&mut conn).await?;

        let values = [
            json!(null),
            json!(412),
            json!(false),
            json!(true),
            json!(42.0),
            json!("whatever\""),
            json!([1, 2, 3]),
            json!(["foo", "bar", false, null]),
            json!({"foo": "bar"}),
        ];

        let mut b = QueryBuilder::new("insert into transactions (hash, block, created_at, authority_signatory, authority_domain, signature, executable) values (");
        b.separated(", ")
            .push_bind("dummy")
            .push_bind(1)
            .push_bind("now()")
            .push_bind("dummy")
            .push_bind("dummy")
            .push_bind("dummy")
            .push_bind("Instructions");
        b.push(")").build().execute(&mut conn).await?;
        QueryBuilder::new("insert into instructions ")
            .push_values(&values, |mut b, value| {
                b.push_bind("dummy")
                    .push_bind(json!({"kind": value.clone()}));
            })
            .build()
            .execute(&mut conn)
            .await?;

        let rows: Vec<(JsonValue,)> = query_as("select payload from v_instructions")
            .fetch_all(&mut conn)
            .await?;
        let rows: Vec<JsonValue> = rows.into_iter().map(|(x,)| x).collect();

        assert_eq!(&rows, &values);

        // for row in query("select payload, payload_quoted from v_instructions")
        //     .fetch_all(&mut conn)
        //     .await?
        // {
        //     for column in row.columns() {
        //         let column_name = column.name();
        //         let value: String = row.try_get_unchecked(column_name)?;
        //         println!("{}: {}", column_name, value);
        //     }
        // }

        Ok(())
    }
}
