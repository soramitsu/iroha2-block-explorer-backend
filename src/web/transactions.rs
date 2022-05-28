use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ops::Deref,
};

use crate::web::etc::HashDeser;

use super::{
    etc::{SerializeScaleIntoHex, Timestamp},
    get, web, AppData, Paginated, PaginationQueryParams, Scope, WebError,
};
use color_eyre::{eyre::Context, Report, Result};
use iroha_core::tx::{
    Executable, RejectedTransaction, TransactionRejectionReason, TransactionValue,
};
use iroha_crypto::{Hash, PublicKey, Signature, SignatureOf};
use iroha_data_model::prelude::{
    AccountId, FindAllTransactions, FindTransactionByHash, Instruction, Payload, Transaction,
    UnlimitedMetadata,
};
use serde::Serialize;

#[derive(Serialize)]
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
pub struct CommittedTransactionDTO {
    #[serde(flatten)]
    shared: TransactionSharedDTO,
    signatures: BTreeSet<Signature>,
}

impl TryFrom<Transaction> for CommittedTransactionDTO {
    type Error = Report;

    fn try_from(value: Transaction) -> Result<Self, Self::Error> {
        Ok(Self {
            shared: TransactionSharedDTO::from_payload(value.payload)?,
            signatures: into_signatures_set(value.signatures),
        })
    }
}

#[derive(Serialize)]
pub struct RejectedTransactionDTO {
    #[serde(flatten)]
    shared: TransactionSharedDTO,
    signatures: BTreeSet<Signature>,
    rejection_reason: TransactionRejectionReason,
}

impl TryFrom<RejectedTransaction> for RejectedTransactionDTO {
    type Error = Report;

    fn try_from(value: RejectedTransaction) -> Result<Self, Self::Error> {
        Ok(Self {
            shared: TransactionSharedDTO::from_payload(value.payload)?,
            signatures: into_signatures_set(value.signatures),
            rejection_reason: value.rejection_reason,
        })
    }
}

#[derive(Serialize)]
struct TransactionSharedDTO {
    block_hash: Hash,
    payload: TransactionPayloadDTO,
}

impl TransactionSharedDTO {
    fn from_payload(payload: Payload) -> Result<Self> {
        Ok(Self {
            // FIXME
            block_hash: Hash::zeroed(),
            payload: payload.try_into().wrap_err("Failed to map Payload")?,
        })
    }
}

#[derive(Serialize)]
pub struct TransactionPayloadDTO {
    account_id: AccountId,
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
            account_id: payload.account_id,
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
pub enum TransactionInstructionsDTO {
    Instructions(Vec<SerializeScaleIntoHex<Instruction>>),
    // For now WASM binary content isn't exposed to frontend
    Wasm,
}

impl From<Executable> for TransactionInstructionsDTO {
    fn from(value: Executable) -> Self {
        match value {
            Executable::Instructions(items) => {
                Self::Instructions(items.into_iter().map(SerializeScaleIntoHex).collect())
            }
            Executable::Wasm(_) => Self::Wasm,
        }
    }
}

fn into_signatures_set<T, U>(value: T) -> BTreeSet<Signature>
where
    T: IntoIterator<Item = U>,
    U: Into<Signature>,
{
    value.into_iter().map(Into::into).collect()
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
