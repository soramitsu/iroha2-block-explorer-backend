use std::collections::BTreeSet;

use crate::web::etc::HashDeser;

use super::{
    etc::{SerScaleHex, SignatureDTO, Timestamp},
    get, web, AppData, Paginated, PaginationQueryParams, Scope, WebError,
};
use color_eyre::{eyre::Context, Report, Result};
use iroha_core::tx::{
    Executable, RejectedTransaction, TransactionRejectionReason, TransactionValue,
};
use iroha_crypto::{Hash, Signature};
use iroha_data_model::prelude::{
    FindAllTransactions, FindTransactionByHash, Instruction, Payload, Transaction,
    UnlimitedMetadata,
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
                Ok(Self::Committed(
                    tx.try_into()
                        .wrap_err("Failed to make CommittedTransactiondTO")?,
                ))
            }
            TransactionValue::RejectedTransaction(versioned) => {
                let tx = versioned.into_v1();
                Ok(Self::Rejected(
                    tx.try_into()
                        .wrap_err("Failed to make RejectedTransactionDTO")?,
                ))
            }
        }
    }
}

#[derive(Serialize)]
pub struct RejectedTransactionDTO {
    #[serde(flatten)]
    base: CommittedTransactionDTO,
    rejection_reason: SerScaleHex<TransactionRejectionReason>,
}

impl TryFrom<RejectedTransaction> for RejectedTransactionDTO {
    type Error = Report;

    fn try_from(value: RejectedTransaction) -> Result<Self, Self::Error> {
        Ok(Self {
            base: CommittedTransactionDTO::from_payload_and_signatures(
                value.payload,
                value.signatures,
            )?,
            rejection_reason: value.rejection_reason.into(),
        })
    }
}

#[derive(Serialize)]
struct CommittedTransactionDTO {
    block_hash: SerScaleHex<Hash>,
    payload: TransactionPayloadDTO,
    signatures: BTreeSet<SignatureDTO>,
}

impl CommittedTransactionDTO {
    fn from_payload_and_signatures<T, U>(payload: Payload, signatures: T) -> Result<Self>
    where
        T: IntoIterator<Item = U>,
        U: Into<Signature>,
    {
        Ok(Self {
            // FIXME
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

impl TryFrom<Transaction> for CommittedTransactionDTO {
    type Error = Report;

    fn try_from(value: Transaction) -> Result<Self, Self::Error> {
        Self::from_payload_and_signatures(value.payload, value.signatures)
    }
}

#[derive(Serialize)]
pub struct TransactionPayloadDTO {
    account_id: String,
    instructions: TransactionInstructionsDTO,
    creation_time: Timestamp,
    time_to_live_ms: u64,
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
            time_to_live_ms: payload.time_to_live_ms,
            nonce: payload.nonce,
            metadata: payload.metadata,
        })
    }
}

#[derive(Serialize)]
#[serde(tag = "t", content = "c")]
pub enum TransactionInstructionsDTO {
    Instructions(Vec<SerScaleHex<Instruction>>),
    // For now WASM binary content isn't exposed to frontend
    Wasm,
}

impl From<Executable> for TransactionInstructionsDTO {
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
        .request(FindTransactionByHash::new(hash))
        .await
        .map_err(WebError::expect_iroha_find_error)?;

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
        .request_with_pagination(FindAllTransactions, pagination.0.into())
        .await
        .map_err(WebError::expect_iroha_any_error)?
        .try_into()?;

    let data = data
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<_>>>()
        .wrap_err("Failed to construct TransactionDTO")?;

    Ok(web::Json(Paginated::new(data, pagination)))
}

pub fn scope() -> Scope {
    web::scope("/transactions").service(index).service(show)
}
