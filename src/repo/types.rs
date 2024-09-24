use crate::repo::util::AsText;
use crate::schema;
use crate::schema::TransactionStatus;
use chrono::{DateTime, Utc};
use iroha_data_model::{account, asset, domain, metadata, prelude};
use serde::Deserialize;
use sqlx::types::Json;
use sqlx::{FromRow, Type};
use std::fmt::{Display, Formatter};
use std::num::NonZero;
use std::str::FromStr;

#[derive(Debug, FromRow)]
pub struct Domain {
    pub id: DomainId,
    pub logo: Option<IpfsPath>,
    pub metadata: Metadata,
    pub owned_by: AccountId,
    pub accounts: u32,
    pub assets: u32,
}

#[derive(Debug, Type)]
#[sqlx(transparent)]
pub struct DomainId(pub AsText<domain::DomainId>);

#[derive(Debug, Type)]
#[sqlx(transparent)]
pub struct AccountId(pub AsText<account::AccountId>);

#[derive(Debug, Type)]
#[sqlx(transparent)]
pub struct IpfsPath(pub String);

#[derive(Debug, Type, Deserialize)]
#[sqlx(transparent)]
pub struct Metadata(pub Option<Json<metadata::Metadata>>);

impl From<Metadata> for metadata::Metadata {
    fn from(value: Metadata) -> Self {
        value.0.map(|x| x.0).unwrap_or_default()
    }
}

#[derive(Debug, FromRow)]
pub struct Block {
    pub hash: Hash,
    pub height: NonZero<u64>,
    pub prev_block_hash: Option<Hash>,
    pub transactions_hash: Hash,
    pub created_at: DateTime<Utc>,
    pub consensus_estimation_ms: u64,
    pub transactions_total: u32,
    pub transactions_rejected: u32,
}

#[derive(Debug, Type)]
#[sqlx(transparent)]
pub struct Hash(pub AsText<iroha_crypto::Hash>);

#[derive(Debug, Type)]
#[sqlx(transparent)]
pub struct Signature(pub AsText<SignatureDisplay>);

// FIXME: remove when Iroha Signature impls FromStr
#[derive(Debug)]
pub struct SignatureDisplay(pub iroha_crypto::Signature);

impl FromStr for SignatureDisplay {
    type Err = iroha_crypto::error::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(iroha_crypto::Signature::from_hex(s)?))
    }
}

impl Display for SignatureDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // FIXME: extract hex directly, not via serde_json
        let serde_json::Value::String(value) = serde_json::to_value(&self.0)
            .expect("nothing could fail during signature serialisation to JSON")
        else {
            unreachable!("should always be a string")
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Type)]
pub enum Executable {
    Instructions,
    #[allow(clippy::upper_case_acronyms)]
    WASM,
}

#[derive(Debug, FromRow)]
pub struct TransactionBase {
    pub hash: Hash,
    pub block: u32,
    pub created_at: DateTime<Utc>,
    pub authority: AccountId,
    pub executable: Executable,
    pub status: TransactionStatus,
}

#[derive(Debug, FromRow)]
pub struct TransactionDetailed {
    #[sqlx(flatten)]
    pub base: TransactionBase,
    pub nonce: Option<NonZero<u32>>,
    pub metadata: Metadata,
    pub time_to_live_ms: u64,
    pub signature: Signature,
    pub rejection_reason: Option<Json<prelude::TransactionRejectionReason>>,
}

#[derive(Debug, FromRow)]
pub struct Account {
    pub id: AccountId,
    pub metadata: Metadata,
    pub owned_domains: u32,
    pub owned_assets: u32,
}

#[derive(Debug, FromRow)]
pub struct AssetDefinition {
    pub id: AssetDefinitionId,
    pub owned_by: AccountId,
    pub logo: Option<IpfsPath>,
    pub metadata: Metadata,
    pub mintable: Mintable,
    pub r#type: AssetType,
    pub assets: u32,
}

#[derive(Debug, Type)]
pub enum Mintable {
    Infinitely,
    Once,
    Not,
}

#[derive(Debug, Type)]
pub enum AssetType {
    Numeric,
    Store,
}

#[derive(Debug, Type)]
#[sqlx(transparent)]
pub struct AssetId(pub AsText<prelude::AssetId>);

#[derive(Debug, FromRow)]
pub struct Asset {
    pub id: AssetId,
    pub value: Json<AssetValue>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum AssetValue {
    Numeric(prelude::Numeric),
    Store(Metadata),
}

#[derive(Debug, Type)]
#[sqlx(transparent)]
pub struct AssetDefinitionId(pub AsText<asset::AssetDefinitionId>);

#[derive(Debug, FromRow)]
pub struct Instruction {
    pub transaction_hash: Hash,
    pub transaction_status: TransactionStatus,
    pub created_at: DateTime<Utc>,
    pub kind: schema::InstructionKind,
    pub payload: Json<serde_json::Value>,
    pub authority: AccountId,
}
