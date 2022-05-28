
use super::{
    etc::{HashDeser, SerializeScaleIntoHex, Timestamp},
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
    tx::{Pagination, VersionedRejectedTransaction, VersionedTransaction},
};
use iroha_crypto::{Hash, HashOf, MerkleTree};
use iroha_data_model::prelude::{BlockValue, FindAllBlocks};
use serde::{de, Serialize};
use std::fmt;

#[derive(Serialize)]
pub struct BlockShallowDTO {
    height: u64,
    timestamp: Timestamp,
    block_hash: Hash,
    transactions: u32,
    rejected_transactions: u32,
}

impl TryFrom<BlockValue> for BlockShallowDTO {
    type Error = color_eyre::Report;

    fn try_from(block: BlockValue) -> Result<Self> {
        Ok(Self {
            height: block.header.height,

            // FIXME https://github.com/hyperledger/iroha/issues/2276
            block_hash: Hash::zeroed(),

            timestamp: Timestamp::try_from(block.header.timestamp)?,
            transactions: block.transactions.len().try_into()?,
            rejected_transactions: block.rejected_transactions.len().try_into()?,
        })
    }
}

#[derive(Serialize)]
pub struct BlockDTO {
    height: u64,
    timestamp: Timestamp,
    block_hash: Hash,
    parent_block_hash: Hash,
    transactions_merkle_root_hash: HashOf<MerkleTree<VersionedTransaction>>,
    rejected_transactions_merkle_root_hash: HashOf<MerkleTree<VersionedTransaction>>,
    invalidated_blocks_hashes: Vec<Hash>,
    transactions: Vec<SerializeScaleIntoHex<VersionedValidTransaction>>,
    rejected_transactions: Vec<SerializeScaleIntoHex<VersionedRejectedTransaction>>,
    view_change_proofs: Vec<Hash>,
}

impl TryFrom<BlockValue> for BlockDTO {
    type Error = color_eyre::Report;

    fn try_from(block: BlockValue) -> Result<Self> {
        Ok(Self {
            height: block.header.height,
            timestamp: Timestamp::try_from(block.header.timestamp)?,

            // FIXME https://github.com/hyperledger/iroha/issues/2276
            block_hash: Hash::zeroed(),

            parent_block_hash: block.header.previous_block_hash,
            transactions_merkle_root_hash: block.header.transactions_hash,
            rejected_transactions_merkle_root_hash: block.header.rejected_transactions_hash,
            invalidated_blocks_hashes: block.header.invalidated_blocks_hashes,
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

/// FIXME use [`actix_web::Either`]?
#[derive(Copy, Clone, Debug, Eq, PartialOrd, Ord, PartialEq)]
pub enum HeightOrHash {
    Height(u64),
    Hash(Hash),
}

impl<'de> de::Deserialize<'de> for HeightOrHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        const HASH_HEX_LENGTH: usize = Hash::LENGTH * 2;

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = HeightOrHash;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "a block height or a {}-byte hex string",
                    Hash::LENGTH
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v.len() == HASH_HEX_LENGTH {
                    let mut slice = [0u8; Hash::LENGTH];
                    hex::decode_to_slice(v, &mut slice).map_err(|_from_hex_error| {
                        E::invalid_value(de::Unexpected::Str(v), &self)
                    })?;
                    let hash = Hash::prehashed(slice);
                    Ok(HeightOrHash::Hash(hash))
                } else {
                    let height: u64 = v
                        .parse()
                        .map_err(|_parse_error| E::invalid_value(de::Unexpected::Str(v), &self))?;
                    Ok(HeightOrHash::Height(height))
                }
            }
        }

        deserializer.deserialize_string(Visitor)
    }
}

#[get("/{height_or_hash}")]
async fn show(
    app: web::Data<AppData>,

    block_id: web::Either<web::Path<u64>, web::Path<HashDeser>>,
) -> Result<web::Json<BlockDTO>, WebError> {
    match block_id {
        web::Either::Left(height) => {
            let height = height.into_inner();
            let blocks = app
                .iroha_client
                .request_with_pagination(
                    FindAllBlocks,
                    Pagination::new(
                        Some(height.try_into().wrap_err("Failed to convert height")?),
                        Some(1),
                    ),
                )
                .await
                .map_err(WebError::expect_iroha_any_error)?
                .only_output();

            let block = match blocks.len() {
                0 => return Err(WebError::NotFound),
                1 => blocks.into_iter().nth(0).expect("Blocks len should be 1"),
                x => return Err(eyre!("Expected to get 0 or 1 block, got: {x}").into()),
            };

            Ok(web::Json(
                block.try_into().wrap_err("Failed to construct BlockDTO")?,
            ))
        }
        web::Either::Right(_hash) => {
            return Err(WebError::NotImplemented(format!(
                "Fetching block by hash is not yet implemented"
            )))
        }
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

#[cfg(test)]
mod tests {
    use super::{Hash, HeightOrHash};

    #[test]
    fn block_height_or_hash_from_height() {
        let value: HeightOrHash = serde_json::from_str("\"575712\"").unwrap();
        assert_eq!(value, HeightOrHash::Height(575712));
    }

    #[test]
    fn block_height_or_hash_from_hash() {
        let mut bytes = [0u8; Hash::LENGTH];
        for i in 0..Hash::LENGTH {
            bytes[i] = i as u8;
        }
        let bytes_hex = hex::encode(&bytes);

        let value: HeightOrHash = serde_json::from_str(&format!("\"{}\"", &bytes_hex)).unwrap();
        assert_eq!(value, HeightOrHash::Hash(Hash::prehashed(bytes)));
    }
}
