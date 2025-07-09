use std::{num::NonZero, ops::Deref, sync::Arc};

use eyre::eyre;
use iroha_core::state::{
    StateReadOnly, StateReadOnlyWithTransactions, StateView, TransactionsReadOnly as _,
    WorldReadOnly,
};
use iroha_data_model::{account::AccountEntry, prelude::*};
use iroha_explorer_schema::{
    self as schema,
    pagination::{OffsetLimitIteratorExt as _, ReversePaginationError},
    PaginationOrEmpty,
};
use mv::storage::StorageReadOnly as _;
use nonzero_ext::nonzero;

use super::state::StateGuard;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Bad pagination: {0}")]
    BadReversePagination(#[from] ReversePaginationError),
    #[error("Bad parameters: {message}")]
    BadParams { message: String },
    #[error("Entity not found: {entity}")]
    NotFound { entity: String },
}

pub type Result<T> = core::result::Result<T, Error>;

pub struct QueryExecutor(StateGuard);

impl QueryExecutor {
    pub fn new(guard: StateGuard) -> Self {
        Self(guard)
    }

    fn view(&self) -> StateView<'_> {
        self.0.view()
    }

    pub fn blocks_index(
        &self,
        pagination: &schema::PaginationQueryParams,
    ) -> Result<schema::Page<schema::Block>> {
        let view = self.view();

        let total = view.height();
        let pagination = match pagination.parse_into_reverse(total)? {
            PaginationOrEmpty::Empty(x) => return Ok(x),
            PaginationOrEmpty::Some(p) => p,
        };

        let items = view
            .all_blocks(nonzero!(1_usize))
            .rev()
            .offset_limit(pagination.to_offset_limit_for_rev_iter())
            .map(|x| schema::Block::from(x.as_ref()))
            .collect::<Vec<_>>();

        Ok(schema::Page::new(items, pagination.into()))
    }

    pub fn blocks_show(&self, id: &schema::BlockHeightOrHash) -> Result<schema::Block> {
        let view = self.view();
        match id {
            schema::BlockHeightOrHash::Height(schema::PositiveInteger(height)) => view
                .all_blocks(*height)
                .next()
                .map(|x| schema::Block::from(x.as_ref()))
                .ok_or_else(|| Error::NotFound {
                    entity: format!("block with height \"{}\"", height.get()),
                }),
            schema::BlockHeightOrHash::Hash(hash) => view
                .all_blocks(nonzero!(1_usize))
                .filter(|block| *block.hash() == *hash)
                .next()
                .map(|x| schema::Block::from(x.as_ref()))
                .ok_or_else(|| Error::NotFound {
                    entity: format!("block with hash \"{}\"", hash),
                }),
        }
    }

    pub fn transactions_index(
        &self,
        filter: &schema::TransactionsIndexFilter,
        pagination: &schema::PaginationQueryParams,
    ) -> eyre::Result<schema::Page<schema::TransactionBase>> {
        let view = self.view();
        let by_height = filter
            .block
            .map(|x| NonZero::new(x as usize).ok_or_else(|| eyre!("zero height is not ok")))
            .transpose()?;

        let produce_iter = || {
            let iter: Box<dyn Iterator<Item = Arc<SignedBlock>>> = if let Some(height) = by_height {
                Box::new(view.all_blocks(height).take(1))
            } else {
                Box::new(view.all_blocks(nonzero!(1_usize)).rev())
            };

            iter.flat_map(BlockTransactionIter::new).filter(|tx_ref| {
                if let Some(account) = &filter.authority {
                    if tx_ref.transaction().authority() != &account.0 {
                        return false;
                    }
                }

                if let Some(status_filter) = &filter.status {
                    if tx_ref.status() != *status_filter {
                        return false;
                    }
                }

                true
            })
        };

        let total = produce_iter().count();
        let pagination = match pagination.parse_into_reverse(total)? {
            PaginationOrEmpty::Empty(x) => return Ok(x),
            PaginationOrEmpty::Some(p) => p,
        };

        let items = produce_iter()
            .offset_limit(pagination.to_offset_limit_for_rev_iter())
            .map(schema::TransactionBase::from)
            .collect();

        Ok(schema::Page::new(items, pagination.into()))
    }

    pub fn transactions_show(&self, hash: &Hash) -> Result<schema::TransactionDetailed> {
        let view = self.view();
        let hash = HashOf::from_untyped_unchecked(*hash);
        let tx_ref = try_retrieve_transaction_ref(&view, &hash).ok_or_else(|| Error::NotFound {
            entity: format!("transaction with hash \"{}\"", hash),
        })?;
        Ok(schema::TransactionDetailed::from(tx_ref))
    }

    pub fn instructions_index(
        &self,
        filter: &schema::InstructionsIndexFilter,
        pagination: &schema::PaginationQueryParams,
    ) -> Result<schema::Page<schema::Instruction>> {
        let view = self.view();
        let produce_isi_iter: Box<
            dyn Fn() -> Box<dyn Iterator<Item = BlockTransactionInstructionRef>>,
        > = if let Some(hash) = &filter.transaction_hash {
            let hash = HashOf::from_untyped_unchecked(hash.0);
            let tx_ref =
                try_retrieve_transaction_ref(&view, &hash).ok_or_else(|| Error::NotFound {
                    entity: format!("transaction with hash \"{hash}\""),
                })?;

            // These filters have no effect in case when tx hash is specified.
            // However, if provided, we validate that they make sense.
            if let Some(status_filter) = &filter.transaction_status {
                if tx_ref.status() != *status_filter {
                    return Err(Error::BadParams { message: format!("transaction with hash \"{hash}\" has status \"{}\", but \"{status_filter}\" is specified; consider removing this filter", tx_ref.status()) });
                }
            }
            if let Some(account) = &filter.authority {
                if tx_ref.authority() != &account.0 {
                    return Err(Error::BadParams { message: format!("transaction with hash \"{hash}\" is authored by account \"{}\", but \"{}\" was specified; consider removing this filter", tx_ref.authority(), account.0) });
                }
            }
            if let Some(height) = &filter.block {
                let actual = tx_ref.block().header().height().get() as usize;
                if actual != height.get() {
                    return Err(Error::BadParams { message: format!("transaction with hash \"{hash}\" is in the block with height \"{actual}\", but \"{}\" was specified; consider removing this filter", height.get()) });
                }
            }

            if !matches!(tx_ref.instructions(), Executable::Instructions(_)) {
                return Err(Error::BadParams {
                    message: format!("transaction with hash \"{hash}\" does not have instructions"),
                });
            }

            Box::new(move || Box::new(tx_ref.instructions_iter().expect("pre-checked")))
        } else {
            Box::new(|| {
                let iter_blocks: Box<dyn Iterator<Item = Arc<SignedBlock>>> =
                    if let Some(height) = filter.block {
                        Box::new(view.all_blocks(*height).take(1))
                    } else {
                        Box::new(view.all_blocks(nonzero!(1usize)).rev())
                    };

                let iter = iter_blocks
                    .flat_map(BlockTransactionIter::new)
                    .filter(|tx_ref| {
                        if let Some(status_filter) = &filter.transaction_status {
                            if tx_ref.status() != *status_filter {
                                return false;
                            }
                        }
                        if let Some(account) = &filter.authority {
                            if tx_ref.authority() != &account.0 {
                                return false;
                            }
                        }
                        true
                    })
                    .filter_map(|x| x.instructions_iter())
                    .flatten();

                Box::new(iter)
            })
        };

        let produce_iter = || {
            produce_isi_iter().filter(|isi| {
                if let Some(value) = &filter.kind {
                    if !value.matches_original(isi) {
                        return false;
                    }
                }
                true
            })
        };

        let total = produce_iter().count();
        let pagination = match pagination.parse_into_reverse(total)? {
            PaginationOrEmpty::Empty(x) => return Ok(x),
            PaginationOrEmpty::Some(p) => p,
        };

        let items = produce_iter()
            .offset_limit(pagination.to_offset_limit_for_rev_iter())
            .map(schema::Instruction::from)
            .collect();

        Ok(schema::Page::new(items, pagination.into()))
    }

    pub fn domains_index(
        &self,
        owned_by: Option<&schema::AccountId>,
        pagination: &schema::PaginationQueryParams,
    ) -> schema::Page<schema::Domain> {
        let view = self.view();
        let produce_iter = || {
            view.world().domains_iter().filter(|domain| {
                if let Some(owner) = &owned_by {
                    if domain.owned_by() != &owner.0 {
                        return false;
                    }
                }
                true
            })
        };

        let total = produce_iter().count();
        let pagination = match pagination.parse_into_direct(total) {
            PaginationOrEmpty::Empty(x) => return x,
            PaginationOrEmpty::Some(p) => p,
        };

        let items = produce_iter()
            .offset_limit(pagination.to_limit_offset())
            .map(|domain| DomainWorldRef {
                domain,
                world: view.world(),
            })
            .map(schema::Domain::from)
            .collect();
        schema::Page::new(items, pagination.into())
    }

    pub fn domains_show(&self, id: &schema::DomainId) -> Result<schema::Domain> {
        let view = self.view();
        view.world()
            .domains()
            .get(&id.0)
            .map(|domain| DomainWorldRef {
                domain,
                world: view.world(),
            })
            .map(schema::Domain::from)
            .ok_or_else(|| Error::NotFound {
                entity: format!("domain \"{}\"", id.0),
            })
    }

    pub fn asset_defs_index(
        &self,
        filter: &schema::AssetDefinitionsIndexFilter,
        pagination: &schema::PaginationQueryParams,
    ) -> schema::Page<schema::AssetDefinition> {
        let view = self.view();
        let produce_iter = || {
            let iter: Box<dyn Iterator<Item = &'_ AssetDefinition>> =
                if let Some(domain) = &filter.domain {
                    Box::new(view.world().asset_definitions_in_domain_iter(&domain.0))
                } else {
                    Box::new(view.world().asset_definitions_iter())
                };

            iter.filter(|item| {
                if let Some(account) = &filter.owned_by {
                    if item.owned_by() != &account.0 {
                        return false;
                    }
                }
                true
            })
        };

        let total = produce_iter().count();
        let pagination = match pagination.parse_into_direct(total) {
            PaginationOrEmpty::Empty(x) => return x,
            PaginationOrEmpty::Some(p) => p,
        };

        let items = produce_iter()
            .offset_limit(pagination.to_limit_offset())
            .map(schema::AssetDefinition::from)
            .collect();

        schema::Page::new(items, pagination.into())
    }

    pub fn asset_defs_show(
        &self,
        id: &schema::AssetDefinitionId,
    ) -> Result<schema::AssetDefinition> {
        let view = self.view();
        view.world()
            .asset_definitions_in_domain_iter(id.0.domain())
            .filter(|x| x.id().name() == id.0.name())
            .next()
            .map(schema::AssetDefinition::from)
            .ok_or_else(|| Error::NotFound {
                entity: format!("asset definition with id \"{}\"", id.0),
            })
    }

    pub fn accounts_index(
        &self,
        filter: &schema::AccountsIndexFilter,
        pagination: &schema::PaginationQueryParams,
    ) -> Result<schema::Page<schema::Account>> {
        let view = self.view();

        let produce_iter = || {
            let accounts: Box<dyn Iterator<Item = AccountEntry>> =
                if let Some(domain) = &filter.domain {
                    Box::new(view.world().accounts_in_domain_iter(&domain.0))
                } else {
                    Box::new(view.world().accounts_iter())
                };

            accounts.filter(|x| {
                if let Some(def_id) = &filter.with_asset {
                    let id = AssetId::new(def_id.0.to_owned(), x.id().to_owned());
                    let exists = view.world().assets().get(&id).is_some();
                    if !exists {
                        return false;
                    }
                }

                true
            })
        };

        let total = produce_iter().count();
        let pagination = match pagination.parse_into_direct(total) {
            PaginationOrEmpty::Empty(x) => return Ok(x),
            PaginationOrEmpty::Some(p) => p,
        };

        let items = produce_iter()
            .offset_limit(pagination.to_limit_offset())
            .map(|entry| AccountWorldRef {
                entry,
                world: view.world(),
            })
            .map(schema::Account::from)
            .collect();

        Ok(schema::Page::new(items, pagination.into()))
    }

    pub fn accounts_show(&self, id: &schema::AccountId) -> Result<schema::Account> {
        let view = self.view();

        view.world()
            .accounts()
            .get(&id.0)
            .map(|value| AccountEntry::new(&id.0, value))
            .map(|entry| AccountWorldRef {
                entry,
                world: view.world(),
            })
            .map(schema::Account::from)
            .ok_or_else(|| Error::NotFound {
                entity: format!("account with id \"{}\"", id.0),
            })
    }

    pub fn assets_index(
        &self,
        filter: &schema::AssetsIndexFilter,
        pagination: &schema::PaginationQueryParams,
    ) -> Result<schema::Page<schema::Asset>> {
        todo!()
    }

    pub fn assets_show(&self, id: &schema::AssetId) -> Result<schema::Asset> {
        todo!()
    }

    pub fn nfts_index(
        &self,
        filter: &schema::AssetDefinitionsIndexFilter,
        pagination: &schema::PaginationQueryParams,
    ) -> Result<schema::Page<schema::Nft>> {
        todo!()
    }

    pub fn nfts_show(&self, id: &schema::NftId) -> Result<schema::Nft> {
        todo!()
    }
}

fn try_retrieve_transaction_ref(
    view: &StateView<'_>,
    hash: &HashOf<SignedTransaction>,
) -> Option<BlockTransactionRef> {
    let height = view.transactions().get(&hash)?;
    let block = view
        .all_blocks(height)
        .next()
        .expect("Bug: block must exist since it is in the transactions storage");
    let tx_ref = BlockTransactionIter::new(block)
        .find(|tx_ref| tx_ref.transaction().hash() == *hash)
        .expect("Bug: transaction must be in the block");
    Some(tx_ref)
}

struct AccountWorldRef<'a, W> {
    world: &'a W,
    entry: AccountEntry<'a>,
}

impl<'a, W> AccountWorldRef<'a, W>
where
    W: WorldReadOnly,
{
    fn domains(&self) -> usize {
        self.world
            .domains_iter()
            .filter(|x| x.owned_by() == self.entry.id())
            .count()
    }

    fn assets(&self) -> usize {
        self.world
            .assets_iter()
            .filter(|x| x.id().account() == self.entry.id())
            .count()
    }

    fn nfts(&self) -> usize {
        self.world
            .nfts_iter()
            .filter(|x| x.owned_by() == self.entry.id())
            .count()
    }
}

impl<W> From<AccountWorldRef<'_, W>> for schema::Account
where
    W: WorldReadOnly,
{
    fn from(value: AccountWorldRef<'_, W>) -> Self {
        Self {
            id: schema::AccountId(value.entry.id().to_owned()),
            metadata: schema::Metadata(value.entry.metadata().to_owned()),
            owned_domains: value.domains(),
            owned_assets: value.assets(),
            owned_nfts: value.nfts(),
        }
    }
}

pub struct DomainWorldRef<'a, W> {
    domain: &'a Domain,
    world: &'a W,
}

impl<W> Deref for DomainWorldRef<'_, W> {
    type Target = Domain;

    fn deref(&self) -> &Self::Target {
        &self.domain
    }
}

impl<'a, W> DomainWorldRef<'a, W>
where
    W: WorldReadOnly,
{
    pub fn domain(&self) -> &Domain {
        &self.domain
    }

    pub fn accounts(&self) -> usize {
        self.world.accounts_in_domain_iter(self.domain.id()).count()
    }

    pub fn assets(&self) -> usize {
        self.world
            .asset_definitions_in_domain_iter(self.domain.id())
            .count()
    }

    pub fn nfts(&self) -> usize {
        self.world.nfts_in_domain_iter(self.domain.id()).count()
    }
}

/// Iterates transactions of a block in reverse order
struct BlockTransactionIter {
    block: Arc<SignedBlock>,
    pointer: usize,
}

impl BlockTransactionIter {
    fn new(block: Arc<SignedBlock>) -> Self {
        let n_transactions = block.transactions_vec().len();
        Self {
            block,
            pointer: n_transactions,
        }
    }
}

impl Iterator for BlockTransactionIter {
    type Item = BlockTransactionRef;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pointer != 0 {
            self.pointer -= 1;
            return Some(BlockTransactionRef {
                block: Arc::clone(&self.block),
                index: self.pointer,
            });
        }

        None
    }
}

#[derive(Clone)]
pub struct BlockTransactionRef {
    block: Arc<SignedBlock>,
    index: usize,
}

impl Deref for BlockTransactionRef {
    type Target = SignedTransaction;

    fn deref(&self) -> &Self::Target {
        self.transaction()
    }
}

impl BlockTransactionRef {
    pub fn block(&self) -> &SignedBlock {
        &self.block
    }

    pub fn error(&self) -> Option<&TransactionRejectionReason> {
        self.block.error(self.index)
    }

    pub fn status(&self) -> schema::TransactionStatus {
        if self.error().is_some() {
            schema::TransactionStatus::Rejected
        } else {
            schema::TransactionStatus::Committed
        }
    }

    pub fn transaction(&self) -> &SignedTransaction {
        &self.block.transactions_vec()[self.index]
    }

    pub fn instructions_iter(&self) -> Option<BlockTransactionInstructionIter> {
        if let Executable::Instructions(isi) = self.transaction().instructions() {
            Some(BlockTransactionInstructionIter {
                tx_ref: self.clone(),
                pointer: isi.len(),
            })
        } else {
            None
        }
    }
}

pub struct BlockTransactionInstructionIter {
    tx_ref: BlockTransactionRef,
    pointer: usize,
}

impl Iterator for BlockTransactionInstructionIter {
    type Item = BlockTransactionInstructionRef;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pointer == 0 {
            None
        } else {
            self.pointer -= 1;
            Some(BlockTransactionInstructionRef {
                tx_ref: self.tx_ref.clone(),
                index: self.pointer,
            })
        }
    }
}

#[derive(Clone)]
pub struct BlockTransactionInstructionRef {
    tx_ref: BlockTransactionRef,
    index: usize,
}

impl Deref for BlockTransactionInstructionRef {
    type Target = InstructionBox;

    fn deref(&self) -> &Self::Target {
        let Executable::Instructions(isis) = self.tx_ref.transaction().instructions() else {
            unreachable!()
        };
        &isis[self.index]
    }
}

impl From<BlockTransactionInstructionRef> for schema::Instruction {
    fn from(value: BlockTransactionInstructionRef) -> Self {
        let tx = value.tx_ref.transaction();
        let isi = value.deref();

        iroha_explorer_schema::Instruction {
            kind: schema::InstructionKind::from(isi),
            r#box: schema::ReprScaleJson(isi.to_owned()),
            transaction_hash: schema::Hash(value.tx_ref.hash().into()),
            transaction_status: value.tx_ref.status(),
            block: value.tx_ref.block().header().height().into(),
            authority: schema::AccountId(value.tx_ref.authority().to_owned()),
            created_at: schema::TimeStamp::from_duration_timestamp(value.tx_ref.creation_time()),
        }
    }
}

impl From<&BlockTransactionRef> for schema::TransactionBase {
    fn from(value: &BlockTransactionRef) -> Self {
        Self {
            hash: value.transaction().hash().into(),
            block: schema::BigInt(value.block().header().height().get() as u128),
            created_at: schema::TimeStamp::from_duration_timestamp(
                value.transaction().creation_time(),
            ),
            authority: value.transaction().authority().into(),
            executable: value.transaction().instructions().into(),
            status: value.status(),
        }
    }
}

impl From<BlockTransactionRef> for schema::TransactionBase {
    fn from(value: BlockTransactionRef) -> Self {
        Self::from(&value)
    }
}

impl From<&BlockTransactionRef> for schema::TransactionDetailed {
    fn from(value: &BlockTransactionRef) -> Self {
        Self {
            base: value.into(),
            signature: schema::Signature(value.signature().0.to_owned().into()),
            nonce: value.nonce().map(|int| {
                schema::PositiveInteger(NonZero::new(int.get() as usize).expect("from non zero"))
            }),
            metadata: schema::Metadata(value.metadata().clone()),
            time_to_live: value.time_to_live().map(schema::TimeDuration::from),
            rejection_reason: value
                .error()
                .map(|reason| schema::ReprScaleJson(reason.to_owned())),
        }
    }
}

impl From<BlockTransactionRef> for schema::TransactionDetailed {
    fn from(value: BlockTransactionRef) -> Self {
        Self::from(&value)
    }
}

impl<W: WorldReadOnly> From<DomainWorldRef<'_, W>> for schema::Domain {
    fn from(value: DomainWorldRef<'_, W>) -> Self {
        Self {
            id: schema::DomainId(value.id().to_owned()),
            logo: value
                .logo()
                .as_ref()
                .map(|x| schema::IpfsPath(x.to_string())),
            metadata: schema::Metadata(value.metadata.to_owned()),
            owned_by: schema::AccountId(value.owned_by().to_owned()),
            accounts: value.accounts(),
            assets: value.assets(),
            nfts: value.nfts(),
        }
    }
}

// TODO: test condition: state is re-initialising (kura is shut down and started again) while query
// is holding state view.

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::state::State;

    use super::*;
    use expect_test::expect;
    use eyre::Result;
    use iroha_explorer_telemetry::Telemetry;
    use iroha_explorer_test_utils::blockchain::{ALICE, CARPENTER, GENESIS};
    use iroha_explorer_test_utils::{init_test_logger, ExpectExt as _};
    use iroha_futures::supervisor::ShutdownSignal;
    use iroha_primitives::time::TimeSource;
    use iroha_test_samples::{ALICE_ID, SAMPLE_GENESIS_ACCOUNT_ID};
    use tempfile::TempDir;
    use tokio::sync::mpsc;

    struct Sandbox {
        dir: TempDir,
        state: State,
    }

    impl Sandbox {
        async fn new() -> Self {
            // TODO: use in-memory storage
            let dir = TempDir::new().unwrap();

            let (state, sup_fut) = State::new(
                dir.path().to_path_buf(),
                Telemetry::new_dummy(mpsc::channel(1).0),
                ShutdownSignal::new(),
            );
            let sup_join = tokio::spawn(sup_fut);

            for block in iroha_explorer_test_utils::blockchain::sample() {
                state.insert_block(block.clone()).await.unwrap();
            }

            Self { dir, state }
        }

        async fn query(&self) -> QueryExecutor {
            QueryExecutor::new(self.state.acquire_guard().await)
        }
    }

    fn pagination(page: Option<usize>, per_page: usize) -> schema::PaginationQueryParams {
        schema::PaginationQueryParams {
            page: page.map(|x| schema::PositiveInteger(NonZero::new(x).unwrap())),
            per_page: schema::PositiveInteger(NonZero::new(per_page).unwrap()),
        }
    }

    #[tokio::test]
    async fn all_blocks() {
        let sandbox = Sandbox::new().await;

        let page = sandbox
            .query()
            .await
            .blocks_index(&pagination(None, 10))
            .unwrap();

        expect![[r#"
            {
              "pagination": {
                "page": 1,
                "per_page": 10,
                "total_pages": 1,
                "total_items": 5
              },
              "items": [
                {
                  "height": 5,
                  "hash": "75657FE498E07AABD589B0D92EE4E1A5D4B89ACED673A43FD3C76EFF87C3C6F7",
                  "prev_block_hash": "50EA8FFF8C75C7393B3E38E0591883EC53EE5E4424615EE707CF7343216635C9",
                  "transactions_hash": "7C5455612C0C6BA29BDAA50FF1F35B26785B6653AEEE3BA1B9AF23473234CB69",
                  "created_at": "1970-01-01T00:00:39.101Z",
                  "transactions_total": 7,
                  "transactions_rejected": 2
                },
                {
                  "height": 4,
                  "hash": "50EA8FFF8C75C7393B3E38E0591883EC53EE5E4424615EE707CF7343216635C9",
                  "prev_block_hash": "B3B8ABCE1A2F040AA243881B582AFF205CD2BECAFAE42A4867F2795DE6ED1ABB",
                  "transactions_hash": null,
                  "created_at": "1970-01-01T00:00:19.100Z",
                  "transactions_total": 0,
                  "transactions_rejected": 0
                },
                {
                  "height": 3,
                  "hash": "B3B8ABCE1A2F040AA243881B582AFF205CD2BECAFAE42A4867F2795DE6ED1ABB",
                  "prev_block_hash": "D2CCDFB950818B8529900468B947C46D2603A8BD90B6FEE370A5476E5098E8C5",
                  "transactions_hash": "C313C434B5637C054F4014CC77B1793F03B5C977E52CA1015036EF66ED112B41",
                  "created_at": "1970-01-01T00:00:17.101Z",
                  "transactions_total": 1,
                  "transactions_rejected": 0
                },
                {
                  "height": 2,
                  "hash": "D2CCDFB950818B8529900468B947C46D2603A8BD90B6FEE370A5476E5098E8C5",
                  "prev_block_hash": "7D6BA357E8F55A27AFF9B47D67D93BBAE006D9EA5CDBC102A147BF446DAF3D41",
                  "transactions_hash": "E7E1248B19EB6FFD7ABCACBAE6EAF8D76295807F5697758515EEDEAB71CEA36B",
                  "created_at": "1970-01-01T00:00:14.100Z",
                  "transactions_total": 3,
                  "transactions_rejected": 0
                },
                {
                  "height": 1,
                  "hash": "7D6BA357E8F55A27AFF9B47D67D93BBAE006D9EA5CDBC102A147BF446DAF3D41",
                  "prev_block_hash": null,
                  "transactions_hash": "F6AC949C6F976E4409511F84B8625C053C5728A0B15A34426318F748E2E10FBF",
                  "created_at": "1970-01-01T00:00:00.001Z",
                  "transactions_total": 1,
                  "transactions_rejected": 0
                }
              ]
            }"#]].assert_json_eq(&page);
    }

    #[tokio::test]
    async fn blocks_pages() {
        let sandbox = Sandbox::new().await;

        let page = sandbox
            .query()
            .await
            .blocks_index(&pagination(None, 2))
            .unwrap()
            .map(|x| x.height);

        expect![[r#"
            {
              "pagination": {
                "page": 3,
                "per_page": 2,
                "total_pages": 3,
                "total_items": 5
              },
              "items": [
                5,
                4,
                3
              ]
            }"#]]
        .assert_json_eq(&page);

        let page = sandbox
            .query()
            .await
            .blocks_index(&pagination(Some(3), 2))
            .unwrap()
            .map(|x| x.height);

        expect![[r#"
            {
              "pagination": {
                "page": 3,
                "per_page": 2,
                "total_pages": 3,
                "total_items": 5
              },
              "items": [
                5
              ]
            }"#]]
        .assert_json_eq(&page);

        let page = sandbox
            .query()
            .await
            .blocks_index(&pagination(Some(2), 2))
            .unwrap()
            .map(|x| x.height);

        expect![[r#"
            {
              "pagination": {
                "page": 2,
                "per_page": 2,
                "total_pages": 3,
                "total_items": 5
              },
              "items": [
                4,
                3
              ]
            }"#]]
        .assert_json_eq(&page);

        let page = sandbox
            .query()
            .await
            .blocks_index(&pagination(Some(1), 2))
            .unwrap()
            .map(|x| x.height);

        expect![[r#"
            {
              "pagination": {
                "page": 1,
                "per_page": 2,
                "total_pages": 3,
                "total_items": 5
              },
              "items": [
                2,
                1
              ]
            }"#]]
        .assert_json_eq(&page);
    }

    #[tokio::test]
    async fn domains() {
        let sandbox = Sandbox::new().await;

        let page = sandbox
            .query()
            .await
            .domains_index(None, &pagination(None, 10));

        expect![[r#"
            {
              "pagination": {
                "page": 1,
                "per_page": 10,
                "total_pages": 1,
                "total_items": 3
              },
              "items": [
                {
                  "id": "garden_of_live_flowers",
                  "logo": "/ipns/QmSrPmbaUKA3ZodhzPWZnpFgcPMFWF4QsxXbkWfEptTBJd",
                  "metadata": {
                    "important_data": [
                      "secret-code",
                      1,
                      2,
                      3
                    ],
                    "very_important_data": {
                      "very": {
                        "important": {
                          "data": {
                            "is": {
                              "deep": {
                                "inside": 42
                              }
                            }
                          }
                        }
                      }
                    }
                  },
                  "owned_by": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland",
                  "accounts": 1,
                  "assets": 0,
                  "nfts": 1
                },
                {
                  "id": "genesis",
                  "logo": null,
                  "metadata": {},
                  "owned_by": "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis",
                  "accounts": 1,
                  "assets": 0,
                  "nfts": 0
                },
                {
                  "id": "wonderland",
                  "logo": null,
                  "metadata": {},
                  "owned_by": "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis",
                  "accounts": 1,
                  "assets": 4,
                  "nfts": 0
                }
              ]
            }"#]].assert_json_eq(&page);
    }

    #[tokio::test]
    async fn domains_by_owner() {
        let query = Sandbox::new().await.query().await;

        let page = query.domains_index(
            Some(&schema::AccountId(GENESIS.to_owned())),
            &pagination(None, 10),
        );
        assert_eq!(page.pagination.total_items, 2);

        let page = query.domains_index(
            Some(&schema::AccountId(ALICE.to_owned())),
            &pagination(None, 10),
        );
        assert_eq!(page.pagination.total_items, 1);

        let page = query.domains_index(
            Some(&schema::AccountId(CARPENTER.to_owned())),
            &pagination(None, 10),
        );
        assert_eq!(page.pagination.total_items, 0);
    }

    #[tokio::test]
    async fn instructions() {
        let query = Sandbox::new().await.query().await;

        let page = query
            .instructions_index(
                &schema::InstructionsIndexFilter {
                    transaction_hash: None,
                    transaction_status: None,
                    block: None,
                    kind: None,
                    authority: None,
                },
                &pagination(None, 50),
            )
            .unwrap()
            .map(|x| format!("{} ({})", x.r#box.0, x.transaction_status));

        expect![[r#"
            {
              "pagination": {
                "page": 1,
                "per_page": 50,
                "total_pages": 1,
                "total_items": 21
              },
              "items": [
                "EXECUTE `ping` (rejected)",
                "LOG(ERROR): A disrupting message of sorts (committed)",
                "REMOVE `keys_from_all_secrets` from `wonderland` (rejected)",
                "TRANSFER `125` FROM `rose#wonderland#ed0120E9F632D3034BAB6BB26D92AC8FD93EF878D9C5E69E01B61B4C47101884EE2F99@garden_of_live_flowers` TO `ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland` (committed)",
                "BURN `25` FROM `rose##ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland` (committed)",
                "MINT `200` TO `rose#wonderland#ed0120E9F632D3034BAB6BB26D92AC8FD93EF878D9C5E69E01B61B4C47101884EE2F99@garden_of_live_flowers` (committed)",
                "MINT `100` TO `rose##ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland` (committed)",
                "REGISTER `time-schedule` (committed)",
                "REGISTER `pre-commit` (committed)",
                "REGISTER `time-schedule#wonderland +Numeric` (committed)",
                "REGISTER `pre-commit#wonderland +Numeric` (committed)",
                "SET `another-rather-unique-metadata-set-later` = `[5,1,2,3,4]` IN `snowflake$garden_of_live_flowers` (committed)",
                "SET `alias` = `\"Genesis\"` IN `ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis` (committed)",
                "SET `alias` = `\"Alice (mutated)\"` IN `ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland` (committed)",
                "REGISTER `snowflake$garden_of_live_flowers` (committed)",
                "REGISTER `[ed0120E9F632D3034BAB6BB26D92AC8FD93EF878D9C5E69E01B61B4C47101884EE2F99@garden_of_live_flowers]` (committed)",
                "REGISTER `[garden_of_live_flowers]` (committed)",
                "REGISTER `tulip#wonderland +Numeric(0)` (committed)",
                "REGISTER `rose#wonderland +Numeric` (committed)",
                "REGISTER `[ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland]` (committed)",
                "REGISTER `[wonderland]` (committed)"
              ]
            }"#]].assert_json_eq(&page);
    }

    #[tokio::test]
    async fn instructions_tx_not_found() {
        let query = Sandbox::new().await.query().await;

        let err = query
            .instructions_index(
                &schema::InstructionsIndexFilter {
                    transaction_hash: Some(schema::Hash(Hash::prehashed([0; 32]))),
                    transaction_status: None,
                    block: None,
                    kind: None,
                    authority: None,
                },
                &pagination(None, 50),
            )
            .unwrap_err();
        expect![[r#"Entity not found: transaction with hash "0000000000000000000000000000000000000000000000000000000000000001""#]].assert_eq(&format!("{err}"));
    }

    #[tokio::test]
    async fn instructions_rejected() {
        let query = Sandbox::new().await.query().await;

        let page = query
            .instructions_index(
                &schema::InstructionsIndexFilter {
                    transaction_hash: None,
                    transaction_status: Some(schema::TransactionStatus::Rejected),
                    block: None,
                    kind: None,
                    authority: None,
                },
                &pagination(None, 50),
            )
            .unwrap()
            .map(|x| format!("{}", x.r#box.0));
        expect![[r#"
            [
              "EXECUTE `ping`",
              "REMOVE `keys_from_all_secrets` from `wonderland`"
            ]"#]]
        .assert_json_eq(&page.items);
    }

    #[tokio::test]
    async fn instructions_by_tx_hash() {
        let query = Sandbox::new().await.query().await;

        let page = query
            .instructions_index(
                &schema::InstructionsIndexFilter {
                    transaction_hash: Some(schema::Hash(
                        "730A906F4D57452AEA712934A14CD4D66B0696B869BCA99CFEF1FCFA5014A97F"
                            .parse()
                            .unwrap(),
                    )),
                    transaction_status: None,
                    block: None,
                    kind: None,
                    authority: None,
                },
                &pagination(None, 50),
            )
            .unwrap()
            .map(|x| format!("{}", x.r#box.0));
        expect![[r#"
            [
              "SET `another-rather-unique-metadata-set-later` = `[5,1,2,3,4]` IN `snowflake$garden_of_live_flowers`",
              "SET `alias` = `\"Genesis\"` IN `ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis`",
              "SET `alias` = `\"Alice (mutated)\"` IN `ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland`"
            ]"#]].assert_json_eq(&page.items);
    }

    #[tokio::test]
    async fn instructions_by_kind() {
        let query = Sandbox::new().await.query().await;

        let page = query
            .instructions_index(
                &schema::InstructionsIndexFilter {
                    transaction_hash: None,
                    transaction_status: None,
                    block: None,
                    kind: Some(schema::InstructionKind::Burn),
                    authority: None,
                },
                &pagination(None, 50),
            )
            .unwrap()
            .map(|x| format!("{}", x.r#box.0));

        expect![[r#"
            [
              "BURN `25` FROM `rose##ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland`"
            ]"#]].assert_json_eq(&page.items);
    }

    #[tokio::test]
    async fn transactions() {
        let query = Sandbox::new().await.query().await;

        let page = query
            .transactions_index(
                &schema::TransactionsIndexFilter {
                    authority: None,
                    status: None,
                    block: None,
                },
                &pagination(None, 20),
            )
            .unwrap()
            .map(|tx| format!("{:?} {:?} {:?}", tx.block, tx.created_at, tx.status));

        expect![[r#"
            {
              "pagination": {
                "page": 1,
                "per_page": 20,
                "total_pages": 1,
                "total_items": 12
              },
              "items": [
                "BigInt(5) TimeStamp(1970-01-01T00:00:39.100Z) Rejected",
                "BigInt(5) TimeStamp(1970-01-01T00:00:39.100Z) Committed",
                "BigInt(5) TimeStamp(1970-01-01T00:00:39.100Z) Committed",
                "BigInt(5) TimeStamp(1970-01-01T00:00:39.100Z) Rejected",
                "BigInt(5) TimeStamp(1970-01-01T00:00:39.100Z) Committed",
                "BigInt(5) TimeStamp(1970-01-01T00:00:39.100Z) Committed",
                "BigInt(5) TimeStamp(1970-01-01T00:00:39.100Z) Committed",
                "BigInt(3) TimeStamp(1970-01-01T00:00:17.100Z) Committed",
                "BigInt(2) TimeStamp(1970-01-01T00:00:14Z) Committed",
                "BigInt(2) TimeStamp(1970-01-01T00:00:07Z) Committed",
                "BigInt(2) TimeStamp(1970-01-01T00:00:05Z) Committed",
                "BigInt(1) TimeStamp(1970-01-01T00:00:00Z) Committed"
              ]
            }"#]]
        .assert_json_eq(&page);
    }

    #[tokio::test]
    async fn transactions_block_status() {
        let query = Sandbox::new().await.query().await;

        let page = query
            .transactions_index(
                &schema::TransactionsIndexFilter {
                    authority: None,
                    status: Some(schema::TransactionStatus::Committed),
                    block: Some(5),
                },
                &pagination(None, 20),
            )
            .unwrap()
            .map(|tx| format!("{:?} {:?} {:?}", tx.block, tx.created_at, tx.status));

        expect![[r#"
            [
              "BigInt(5) TimeStamp(1970-01-01T00:00:39.100Z) Committed",
              "BigInt(5) TimeStamp(1970-01-01T00:00:39.100Z) Committed",
              "BigInt(5) TimeStamp(1970-01-01T00:00:39.100Z) Committed",
              "BigInt(5) TimeStamp(1970-01-01T00:00:39.100Z) Committed",
              "BigInt(5) TimeStamp(1970-01-01T00:00:39.100Z) Committed"
            ]"#]]
        .assert_json_eq(&page.items);
    }

    #[tokio::test]
    async fn accounts() {
        let query = Sandbox::new().await.query().await;

        let page = query
            .accounts_index(
                &schema::AccountsIndexFilter {
                    with_asset: None,
                    domain: None,
                },
                &pagination(None, 10),
            )
            .unwrap();

        expect![[r#"
            {
              "pagination": {
                "page": 1,
                "per_page": 10,
                "total_pages": 1,
                "total_items": 3
              },
              "items": [
                {
                  "id": "ed0120E9F632D3034BAB6BB26D92AC8FD93EF878D9C5E69E01B61B4C47101884EE2F99@garden_of_live_flowers",
                  "metadata": {
                    "alias": "Carpenter"
                  },
                  "owned_domains": 0,
                  "owned_assets": 1,
                  "owned_nfts": 1
                },
                {
                  "id": "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis",
                  "metadata": {
                    "alias": "Genesis"
                  },
                  "owned_domains": 2,
                  "owned_assets": 0,
                  "owned_nfts": 0
                },
                {
                  "id": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland",
                  "metadata": {
                    "alias": "Alice (mutated)"
                  },
                  "owned_domains": 1,
                  "owned_assets": 3,
                  "owned_nfts": 0
                }
              ]
            }"#]].assert_json_eq(&page);
    }

    #[tokio::test]
    async fn accounts_in_domain() {
        let query = Sandbox::new().await.query().await;

        let page = query
            .accounts_index(
                &schema::AccountsIndexFilter {
                    with_asset: None,
                    domain: Some(schema::DomainId("genesis".parse().unwrap())),
                },
                &pagination(None, 10),
            )
            .unwrap()
            .map(|x| x.metadata);

        expect![[r#"
            [
              {
                "alias": "Genesis"
              }
            ]"#]]
        .assert_json_eq(page.items);
    }

    #[tokio::test]
    async fn account_by_id() {
        let query = Sandbox::new().await.query().await;

        let item = query
            .accounts_show(&schema::AccountId(ALICE.to_owned()))
            .unwrap();

        expect![[r#"
            {
              "id": "ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland",
              "metadata": {
                "alias": "Alice (mutated)"
              },
              "owned_domains": 1,
              "owned_assets": 3,
              "owned_nfts": 0
            }"#]].assert_json_eq(item);
    }
}
