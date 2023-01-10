use std::collections::BTreeSet;

use crate::{iroha_client_wrap::QueryBuilder, web::etc::HashDeser};

use super::{
    etc::{SerScaleHex, SignatureDTO, Timestamp},
    get, web, AppData, Paginated, PaginationQueryParams, Scope, WebError,
};
use color_eyre::{eyre::Context, Result};
use iroha_core::tx::{
    Executable, RejectedTransaction, TransactionRejectionReason, TransactionValue, Txn,
};
use iroha_crypto::{Hash, HashOf, Signature};
use iroha_data_model::prelude::{
    FindAllTransactions, FindTransactionByHash, Instruction, Payload, SignedTransaction, UnlimitedMetadata,
};
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "t", content = "c")]
pub enum TransactionDTO {
    Committed(CommittedTransactionDTO),
    Rejected(RejectedTransactionDTO),
}

impl TryFrom<TransactionValue> for TransactionDTO {
    type Error = color_eyre::Report;

    fn try_from(tx: TransactionValue) -> Result<Self> {
        match tx {
            TransactionValue::Transaction(versioned) => {
                let tx = versioned.into_v1();
                let base = TransactionBase::new(tx.hash(), tx.payload, tx.signatures)
                    .wrap_err("Failed to make TransactionBase")?;
                Ok(Self::Committed(CommittedTransactionDTO { base }))
            }
            TransactionValue::RejectedTransaction(versioned) => {
                let tx = versioned.into_v1();
                let hash = tx.hash();
                let RejectedTransaction {
                    payload,
                    signatures,
                    rejection_reason,
                } = tx;
                let base = TransactionBase::new(hash, payload, signatures)
                    .wrap_err("Failed to make TransactionBase")?;
                Ok(Self::Rejected(RejectedTransactionDTO {
                    base,
                    rejection_reason: rejection_reason.into(),
                }))
            }
        }
    }
}

#[derive(Serialize)]
struct TransactionBase {
    hash: SerScaleHex<HashOf<SignedTransaction>>,
    block_hash: SerScaleHex<Hash>,
    payload: TransactionPayloadDTO,
    signatures: BTreeSet<SignatureDTO>,
}

impl TransactionBase {
    fn new<T, U>(hash: HashOf<SignedTransaction>, payload: Payload, signatures: T) -> Result<Self>
    where
        T: IntoIterator<Item = U>,
        U: Into<Signature>,
    {
        Ok(Self {
            hash: hash.into(),

            // FIXME https://github.com/hyperledger/iroha/issues/2301
            block_hash: Hash::zeroed().into(),

            payload: payload.try_into().wrap_err("Failed to map Payload")?,
            signatures: signatures
                .into_iter()
                .map(Into::<Signature>::into)
                .map(Into::into)
                .collect(),
        })
    }
}

#[derive(Serialize)]
pub struct CommittedTransactionDTO {
    #[serde(flatten)]
    base: TransactionBase,
}

/// Just as [`CommittedTransactionDTO`], but with rejection reason
#[derive(Serialize)]
pub struct RejectedTransactionDTO {
    #[serde(flatten)]
    base: TransactionBase,
    rejection_reason: SerScaleHex<TransactionRejectionReason>,
}

#[derive(Serialize)]
pub struct TransactionPayloadDTO {
    account_id: String,
    instructions: ExecutableDTO,
    creation_time: Timestamp,
    time_to_live_ms: u32,
    nonce: Option<u32>,
    metadata: UnlimitedMetadata,
}

impl TryFrom<Payload> for TransactionPayloadDTO {
    type Error = color_eyre::Report;

    fn try_from(payload: Payload) -> Result<Self, Self::Error> {
        Ok(Self {
            account_id: payload.account_id.to_string(),
            instructions: payload.instructions.into(),
            creation_time: Timestamp::try_from(payload.creation_time)
                .wrap_err("Failed to map creation_time")?,
            time_to_live_ms: payload.time_to_live_ms.try_into()?,
            nonce: payload.nonce,
            metadata: payload.metadata,
        })
    }
}

/// Reflection of [`Executable`].
#[derive(Serialize)]
#[serde(tag = "t", content = "c")]
pub enum ExecutableDTO {
    Instructions(Vec<SerScaleHex<Instruction>>),
    /// WASM binary content is omitted for frontend
    Wasm,
}

impl From<Executable> for ExecutableDTO {
    fn from(value: Executable) -> Self {
        match value {
            Executable::Instructions(items) => {
                Self::Instructions(items.into_iter().map(SerScaleHex).collect())
            }
            Executable::Wasm(_) => Self::Wasm,
        }
    }
}

#[get("/{hash}")]
async fn show(
    app: web::Data<AppData>,
    hash: web::Path<HashDeser>,
) -> Result<web::Json<TransactionDTO>, WebError> {
    let hash = hash.into_inner().0;
    let tx = app
        .iroha_client
        .request(QueryBuilder::new(FindTransactionByHash::new(hash)))
        .await
        .map_err(WebError::expect_iroha_find_error)?
        .only_output();

    Ok(web::Json(
        tx.try_into().wrap_err("Failed to map TransactionValue")?,
    ))
}

#[get("")]
async fn index(
    app: web::Data<AppData>,
    pagination: web::Query<PaginationQueryParams>,
) -> Result<web::Json<Paginated<Vec<TransactionDTO>>>, WebError> {
    let Paginated { data, pagination } = app
        .iroha_client
        .request(QueryBuilder::new(FindAllTransactions).with_pagination(pagination.0.into()))
        .await
        .map_err(WebError::expect_iroha_any_error)?
        .try_into()?;

    let data = data
        .into_iter()
        .map(|x| TryInto::try_into(x.tx_value))
        .collect::<Result<Vec<_>>>()
        .wrap_err("Failed to construct TransactionDTO")?;

    Ok(web::Json(Paginated::new(data, pagination)))
}

pub fn scope() -> Scope {
    web::scope("/transactions").service(index).service(show)
}
