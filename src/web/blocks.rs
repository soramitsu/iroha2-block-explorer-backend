use crate::iroha_client_wrap::QueryBuilder;

use super::{
    etc::{HashDeser, SerScaleHex, Timestamp},
    get,
    pagination::{Paginated, PaginationQueryParams},
    web, AppData, Scope, WebError,
};
use color_eyre::{
    eyre::{eyre, Context},
    Result,
};
use iroha_core::tx::{Pagination, VersionedSignedTransaction};
use iroha_crypto::Hash;
use iroha_crypto::{HashOf, MerkleTree};
use iroha_data_model::block::VersionedCommittedBlock;
use iroha_data_model::prelude::{FindAllBlocks, TransactionValue};
use iroha_data_model::{block::CommittedBlock, SignaturesOf};
use serde::Serialize;
use std::convert::TryInto;
use std::num::NonZeroU64;

/// Block DTO intended to be lightweight and to have only simple aggregated data.
/// Detailed data is contained within [`BlockDTO`]
#[derive(Serialize)]
pub struct BlockShallowDTO {
    /// See [`BlockDTO`]'s height
    height: u32,
    timestamp: Timestamp,
    block_hash: SerScaleHex<Hash>,
    transactions: u32,
    // rejected_transactions: u32,
    signature: SignaturesOf<CommittedBlock>,
}
impl TryFrom<VersionedCommittedBlock> for BlockShallowDTO {
    type Error = color_eyre::Report;
    fn try_from(block: VersionedCommittedBlock) -> Result<Self> {
        let committed_block = block.into_v1();
        Ok(Self {
            height: committed_block.header.height.try_into()?,
            block_hash: committed_block.hash().into(),
            timestamp: Timestamp::try_from(committed_block.header.timestamp)?,
            transactions: committed_block.transactions.len().try_into()?,
            // rejected_transactions:  committed_block.rejected_transactions.len().try_into()?,
            signature: committed_block.signatures,
        })
    }
}

/// Full Block DTO
#[derive(Serialize)]
pub struct BlockDTO {
    // Original height value is u64, but u64 can't fit into JS `number`
    height: u32,
    timestamp: Timestamp,
    block_hash: SerScaleHex<Hash>,
    parent_block_hash: Option<HashOf<VersionedCommittedBlock>>,
    transactions_merkle_root_hash: Option<HashOf<MerkleTree<VersionedSignedTransaction>>>,
    rejected_transactions_merkle_root_hash: Option<HashOf<MerkleTree<VersionedSignedTransaction>>>,
    // invalidated_blocks_hashes: Vec<SerScaleHex<Hash>>,
    transactions: Vec<TransactionValue>,
    // rejected_transactions: Vec<SerScaleHex<VersionedSignedTransaction>>,
    // view_change_proofs: Vec<SerScaleHex<Hash>>,
}

impl TryFrom<VersionedCommittedBlock> for BlockDTO {
    type Error = color_eyre::Report;

    fn try_from(block: VersionedCommittedBlock) -> Result<Self> {
        let committed_block = block.into_v1();
        // the querybox output is  VersionedCommittedBlock -> committedBlock
        Ok(Self {
            height: committed_block.header.height.try_into()?,
            timestamp: Timestamp::try_from(committed_block.header.timestamp)?,
            block_hash: committed_block.hash().into(),
            parent_block_hash: committed_block.header.previous_block_hash,
            transactions_merkle_root_hash: committed_block.header.transactions_hash,
            rejected_transactions_merkle_root_hash: committed_block
                .header
                .rejected_transactions_hash,
            // invalidated_blocks_hashes:  committed_block
            //     .header
            //     .invalidated_blocks_hashescl
            //     .into_iter()
            //     .map(Into::into)
            //     .collect(),
            transactions: committed_block
                .transactions
                .into_iter()
                .map(Into::into)
                .collect(),
            // rejected_transactions:  committed_block
            //     .rejected_transactions
            //     .into_iter()
            //     .map(Into::into)
            //     .collect(),

            // FIXME https://github.com/hyperledger/iroha/issues/2277
            // view_change_proofs: Vec::new(),
        })
    }
}

#[get("/{height_or_hash}")]
async fn show(
    app: web::Data<AppData>,
    block_id: web::Either<web::Path<NonZeroU64>, web::Path<HashDeser>>,
) -> Result<web::Json<BlockDTO>, WebError> {
    match block_id {
        web::Either::Left(height) => {
            let height = height.into_inner();

            // -1 because of how blocks pagination works
            let pagination_offset: u32 = (height.get() - 1)
                .try_into()
                .wrap_err("Failed to convert height")?;

            let blocks = app
                .iroha_client
                .request(
                    QueryBuilder::new(FindAllBlocks)
                        .with_pagination(Pagination::new(Some(pagination_offset), Some(1))),
                )
                .await
                .map_err(WebError::expect_iroha_any_error)?
                .only_output();

            let block = match blocks.len() {
                0 => return Err(WebError::NotFound),
                1 => blocks.into_iter().next().expect("Blocks len should be 1"),
                x => return Err(eyre!("Expected to get 0 or 1 block, got: {x}").into()),
            };

            Ok(web::Json(
                block.try_into().wrap_err("Failed to construct BlockDTO")?,
            ))
        }
        web::Either::Right(_hash) => Err(WebError::not_implemented(
            "Fetching block by hash is not yet implemented".to_string(),
        )),
    }
}

#[get("")]
async fn index(
    app: web::Data<AppData>,
    pagination: web::Query<PaginationQueryParams>,
) -> Result<web::Json<Paginated<Vec<BlockShallowDTO>>>, WebError> {
    let Paginated {
        data: blocks,
        pagination,
    } = app
        .iroha_client
        .request(QueryBuilder::new(FindAllBlocks).with_pagination(pagination.0.into()))
        .await
        .map_err(WebError::expect_iroha_any_error)?
        .try_into()?;

    let blocks = blocks
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<BlockShallowDTO>>>()
        .wrap_err("Failed to construct BlockShallowDTO")?;

    Ok(web::Json(Paginated::new(blocks, pagination)))
}

pub fn scope() -> Scope {
    web::scope("/blocks").service(index).service(show)
}
