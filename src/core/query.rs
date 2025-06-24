use std::{num::NonZero, ops::Deref, sync::Arc};

use eyre::eyre;
use iroha_core::{
    kura::KuraReadOnly,
    state::{
        State, StateReadOnly, StateReadOnlyWithTransactions, StateView, StorageReadOnly as _,
        TransactionsReadOnly as _, WorldReadOnly,
    },
    tx::{
        AssetDefinition, Domain, Executable, InstructionBox, SignedTransaction,
        TransactionRejectionReason,
    },
};
use iroha_crypto::{Hash, HashOf};
use iroha_data_model::block::SignedBlock;
use iroha_data_model::Identifiable;
use nonzero_ext::nonzero;

use crate::{
    core::storage,
    schema::{self, PaginationOrEmpty},
    util::{OffsetLimitIteratorExt as _, ReversePaginationError},
};

use super::state::StateReader;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("Bad pagination: {0}")]
    BadReversePagination(#[from] ReversePaginationError),
    #[error("Bad parameters: {message}")]
    BadParams { message: String },
    #[error("Entity not found: {entity}")]
    NotFound { entity: String },
}

pub type Result<T> = core::result::Result<T, Error>;

pub struct QueryExecutor(StateReader);

impl QueryExecutor {
    pub fn new(view: StateReader) -> Self {
        Self(view)
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
            .all_blocks(&self.storage, nonzero!(1_usize))
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
                .all_blocks(&self.storage, *height)
                .next()
                .map(|x| schema::Block::from(x.as_ref()))
                .ok_or_else(|| Error::NotFound {
                    entity: format!("block with height \"{}\"", height.get()),
                }),
            schema::BlockHeightOrHash::Hash(hash) => view
                .all_blocks(&self.storage, nonzero!(1_usize))
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
                Box::new(view.all_blocks(&self.storage, height).take(1))
            } else {
                Box::new(view.all_blocks(&self.storage, nonzero!(1_usize)))
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
        let tx_ref =
            try_retrieve_transaction_ref(&view, &self.storage, &hash).ok_or_else(|| {
                Error::NotFound {
                    entity: format!("transaction with hash \"{}\"", hash),
                }
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
                try_retrieve_transaction_ref(&view, &self.storage, &hash).ok_or_else(|| {
                    Error::NotFound {
                        entity: format!("transaction with hash \"{hash}\""),
                    }
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
                        Box::new(view.all_blocks(&self.storage, *height).take(1))
                    } else {
                        Box::new(view.all_blocks(&self.storage, nonzero!(1usize)))
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
        todo!()
    }

    pub fn accounts_show(&self, id: &schema::AccountId) -> Result<schema::Account> {
        todo!()
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
    kura: &impl KuraReadOnly,
    hash: &HashOf<SignedTransaction>,
) -> Option<BlockTransactionRef> {
    let height = view.transactions().get(&hash)?;
    let block = view
        .all_blocks(kura, height)
        .next()
        .expect("Bug: block must exist since it is in the transactions storage");
    let tx_ref = BlockTransactionIter::new(block)
        .find(|tx_ref| tx_ref.transaction().hash() == *hash)
        .expect("Bug: transaction must be in the block");
    Some(tx_ref)
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
struct BlockTransactionIter(Arc<SignedBlock>, usize);

impl BlockTransactionIter {
    fn new(block: Arc<SignedBlock>) -> Self {
        let n_transactions = block.transactions_vec().len();
        Self(block, n_transactions)
    }
}

impl Iterator for BlockTransactionIter {
    type Item = BlockTransactionRef;

    fn next(&mut self) -> Option<Self::Item> {
        if self.1 != 0 {
            self.1 -= 1;
            return Some(BlockTransactionRef(Arc::clone(&self.0), self.1));
        }

        None
    }
}

#[derive(Clone)]
pub struct BlockTransactionRef(Arc<SignedBlock>, usize);

impl Deref for BlockTransactionRef {
    type Target = SignedTransaction;

    fn deref(&self) -> &Self::Target {
        self.transaction()
    }
}

impl BlockTransactionRef {
    pub fn block(&self) -> &SignedBlock {
        &self.0
    }

    pub fn error(&self) -> Option<&TransactionRejectionReason> {
        self.0.error(self.1)
    }

    pub fn status(&self) -> schema::TransactionStatus {
        if self.error().is_some() {
            schema::TransactionStatus::Rejected
        } else {
            schema::TransactionStatus::Committed
        }
    }

    pub fn transaction(&self) -> &SignedTransaction {
        &self.0.transactions_vec()[self.1]
    }

    pub fn instructions_iter(&self) -> Option<BlockTransactionInstructionIter> {
        if matches!(
            self.transaction().instructions(),
            Executable::Instructions(_)
        ) {
            Some(BlockTransactionInstructionIter {
                tx_ref: self.clone(),
                index: 0,
            })
        } else {
            None
        }
    }
}

struct BlockTransactionInstructionIter {
    tx_ref: BlockTransactionRef,
    index: usize,
}

impl Iterator for BlockTransactionInstructionIter {
    type Item = BlockTransactionInstructionRef;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

#[derive(Clone)]
pub struct BlockTransactionInstructionRef {
    block: Arc<SignedBlock>,
    tx: usize,
    isi: usize,
}

impl Deref for BlockTransactionInstructionRef {
    type Target = InstructionBox;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}

// TODO: test condition: state is re-initialising (kura is shut down and started again) while query
// is holding state view.
