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
use iroha_core::{
    prelude::VersionedValidTransaction,
    tx::{Pagination, VersionedRejectedTransaction},
};
use iroha_crypto::Hash;
use iroha_data_model::prelude::{BlockValue, FindAllBlocks};
use serde::Serialize;
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
    rejected_transactions: u32,
}

impl TryFrom<BlockValue> for BlockShallowDTO {
    type Error = color_eyre::Report;

    fn try_from(block: BlockValue) -> Result<Self> {
        Ok(Self {
            height: block.header.height.try_into()?,

            // FIXME https://github.com/hyperledger/iroha/issues/2276
            block_hash: Hash::zeroed().into(),

            timestamp: Timestamp::try_from(block.header.timestamp)?,
            transactions: block.transactions.len().try_into()?,
            rejected_transactions: block.rejected_transactions.len().try_into()?,
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
    parent_block_hash: SerScaleHex<Hash>,
    transactions_merkle_root_hash: SerScaleHex<Hash>,
    rejected_transactions_merkle_root_hash: SerScaleHex<Hash>,
    invalidated_blocks_hashes: Vec<SerScaleHex<Hash>>,
    transactions: Vec<SerScaleHex<VersionedValidTransaction>>,
    rejected_transactions: Vec<SerScaleHex<VersionedRejectedTransaction>>,
    view_change_proofs: Vec<SerScaleHex<Hash>>,
}

impl TryFrom<BlockValue> for BlockDTO {
    type Error = color_eyre::Report;

    fn try_from(block: BlockValue) -> Result<Self> {
        Ok(Self {
            height: block.header.height.try_into()?,
            timestamp: Timestamp::try_from(block.header.timestamp)?,

            // FIXME https://github.com/hyperledger/iroha/issues/2276
            block_hash: Hash::zeroed().into(),

            parent_block_hash: block.header.previous_block_hash.into(),
            transactions_merkle_root_hash: block.header.transactions_hash.into(),
            rejected_transactions_merkle_root_hash: block.header.rejected_transactions_hash.into(),
            invalidated_blocks_hashes: block
                .header
                .invalidated_blocks_hashes
                .into_iter()
                .map(Into::into)
                .collect(),
            transactions: block.transactions.into_iter().map(Into::into).collect(),
            rejected_transactions: block
                .rejected_transactions
                .into_iter()
                .map(Into::into)
                .collect(),

            // FIXME https://github.com/hyperledger/iroha/issues/2277
            view_change_proofs: Vec::new(),
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
                .request_with_pagination(
                    FindAllBlocks,
                    Pagination::new(Some(pagination_offset), Some(1)),
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
        .request_with_pagination(FindAllBlocks, pagination.0.into())
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
