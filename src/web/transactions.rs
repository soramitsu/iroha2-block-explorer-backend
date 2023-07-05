use std::collections::BTreeSet;

use crate::{iroha_client_wrap::QueryBuilder, web::etc::HashDeser};

use super::{
    etc::{SerScaleHex, SignatureDTO, Timestamp},
    get, web, AppData, Paginated, PaginationQueryParams, Scope, WebError,
};
use color_eyre::{eyre::Context, Result};
use iroha_core::tx::{
    Executable, AcceptedTransaction, TransactionValue, 
};
use iroha_crypto::{Hash, HashOf, Signature,SignaturesOf};
use iroha_data_model::prelude::{
    FindAllTransactions, FindTransactionByHash, InstructionBox,TransactionQueryResult,   UnlimitedMetadata,
};
use iroha_data_model::transaction::model::SignedTransaction;
use iroha_data_model::transaction::model::TransactionPayload;
use iroha_data_model::transaction::error::model::TransactionRejectionReason;
use serde::Serialize;
use core::num::{NonZeroU64,NonZeroU32};

#[derive(Serialize)]
#[serde(tag = "t", content = "c")]
pub enum TransactionDTO {
    Committed(CommittedTransactionDTO),
    Rejected(RejectedTransactionDTO),
}

impl TryFrom<TransactionQueryResult> for TransactionDTO {
    type Error = color_eyre::Report;

    // fn try_from(tx: TransactionValue) -> Result<Self> {
    //     match tx {
    //         TransactionValue::Transaction(versioned) => {
    //             let tx = versioned.into_v1();
    //             let base = TransactionBase::new(tx.hash(), tx.payload, tx.signatures)
    //                 .wrap_err("Failed to make TransactionBase")?;
    //             Ok(Self::Committed(CommittedTransactionDTO { base }))
    //         }
    //         TransactionValue::AcceptedTransaction(versioned) => {
    //             let tx = versioned.into_v1();
    //             let hash = tx.hash();
    //             let AcceptedTransaction {
    //                 payload,
    //                 signatures,
    //                 rejection_reason,
    //             } = tx;
    //             let base = TransactionBase::new(hash, payload, signatures)
    //                 .wrap_err("Failed to make TransactionBase")?;
    //             Ok(Self::Rejected(RejectedTransactionDTO {
    //                 base,
    //                 rejection_reason: rejection_reason.into(),
    //             }))
    //         }
    //     }
    // }

    fn try_from(tx_result: TransactionQueryResult) -> Result<Self> {
        // transactionQueryResult -> transactionValue
        let tx_value = tx_result.transaction();
        // transactionValue -> {VersionedSignedTransaction,error}
        let TransactionValue { tx, error } = tx_value;
        let hash = tx.hash();
        let base = TransactionBase::new(hash, tx.payload(), tx.signatures())
            .wrap_err("Failed to make TransactionBase")?;

        match error {
            Some(rejection_reason) => Ok(Self::Rejected(RejectedTransactionDTO {
                base,
                rejection_reason: rejection_reason.into(),
            })),
            None => Ok(Self::Committed(CommittedTransactionDTO { base })),
        }
    }
}

#[derive(Serialize)]
struct TransactionBase {
    hash: SerScaleHex<HashOf<VersionedSignedTransaction>>,
    block_hash: SerScaleHex<Hash>,
    // payload: TransactionPayloadDTO,
    payload: TransactionPayload,
    signatures: SignaturesOf<TransactionPayload>,
}

impl TransactionBase {
    fn new<T, U>(hash: HashOf<SignedTransaction>, payload: TransactionPayload, signatures: T) -> Result<Self>
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
    time_to_live_ms:  Option<NonZeroU64>,
    nonce: Option<NonZeroU32>,
    metadata: UnlimitedMetadata,
}



impl TryFrom<TransactionPayload> for TransactionPayloadDTO {
    type Error = color_eyre::Report;

    fn try_from(payload: TransactionPayload) -> Result<Self, Self::Error> {
        Ok(Self {
            account_id: payload.authority.to_string(),
            instructions: payload.instructions.into(),
            creation_time: Timestamp::try_from(payload.creation_time_ms)
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
    Instructions(Vec<SerScaleHex< InstructionBox>>),
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
        .request(QueryBuilder::new(FindTransactionByHash::new(HashOf::from_untyped_unchecked(hash)))) // deprecated 
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
        .map(|x| TryInto::try_into(x)) // transactionQueryResult returned 
        .collect::<Result<Vec<_>>>()
        .wrap_err("Failed to construct TransactionDTO")?;

    Ok(web::Json(Paginated::new(data, pagination)))
}

pub fn scope() -> Scope {
    web::scope("/transactions").service(index).service(show)
}
