//! Schemas defining Explorer's HTTP API types

// FIXME: resolve todos
#![allow(unused)]

pub mod pagination;

use crate::pagination::{DirectPagination, ReversePagination, ReversePaginationError};
use base64::Engine;
use chrono::{DateTime, Utc};
use nonzero_ext::nonzero;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::json;
use serde_with::DeserializeFromStr;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::Display;
use std::num::NonZero;
use std::ops::Deref;
use std::str::FromStr;
use url::Url;
use utoipa::openapi::{RefOr, Schema};
use utoipa::{schema, IntoParams, PartialSchema, ToSchema};

mod iroha {
    pub use iroha_config::client_api::ConfigGetDTO;
    pub use iroha_data_model::prelude::*;
    pub use iroha_data_model::{asset::AssetEntry, ipfs::IpfsPath, nft::NftEntry};
}

/// Domain
#[derive(ToSchema, Serialize)]
#[schema(
    example = json!({
        "id": "genesis",
        "logo": null,
        "metadata": {},
        "owned_by": "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis"
    })
)]
pub struct Domain {
    /// Domain ID
    pub id: DomainId,
    /// Domain logo path
    pub logo: Option<IpfsPath>,
    /// Domain metadata
    pub metadata: Metadata,
    /// Domain's owner
    pub owned_by: AccountId,
    /// Total number of accounts in this domain
    pub accounts: usize,
    /// Total number of assets _definitions_ in this domain
    pub assets: usize,
    /// Total number of NFTs in this domain
    pub nfts: usize,
}

// impl<W: WorldReadOnly> From<query::DomainWorldRef<'_, W>> for Domain {
//     fn from(value: query::DomainWorldRef<'_, W>) -> Self {
//         Self {
//             id: DomainId(value.id().clone()),
//             logo: value.logo().as_ref().map(|x| IpfsPath(x.to_string())),
//             metadata: Metadata(value.metadata().clone()),
//             owned_by: AccountId(value.owned_by().clone()),
//             accounts: value.accounts(),
//             assets: value.assets(),
//             nfts: value.nfts(),
//         }
//     }
// }

/// Domain ID
#[derive(Debug, ToSchema, Serialize, Deserialize)]
#[schema(example = "genesis", value_type = String)]
pub struct DomainId(pub iroha::DomainId);

/// Account
#[derive(Serialize, ToSchema)]
pub struct Account {
    pub id: AccountId,
    pub metadata: Metadata,
    pub owned_domains: usize,
    pub owned_assets: usize,
    pub owned_nfts: usize,
}

/// Account ID. Represented as `signatory@domain`.
#[derive(ToSchema, Serialize, Deserialize, Debug)]
#[schema(
    example = "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis",
    value_type = String
)]
pub struct AccountId(pub iroha::AccountId);

// impl From<repo::AccountId> for AccountId {V
//     fn from(value: repo::AccountId) -> Self {
//         Self(value.0 .0)
//     }
// }

impl From<&iroha::AccountId> for AccountId {
    fn from(value: &iroha::AccountId) -> Self {
        Self(value.clone())
    }
}

#[derive(ToSchema, Serialize)]
pub struct AssetDefinition {
    id: AssetDefinitionId,
    mintable: Mintable,
    logo: Option<IpfsPath>,
    metadata: Metadata,
    owned_by: AccountId,
    total_quantity: Decimal,
}

impl From<&iroha::AssetDefinition> for AssetDefinition {
    fn from(value: &iroha::AssetDefinition) -> Self {
        Self {
            id: AssetDefinitionId(value.id.to_owned()),
            mintable: value.mintable.into(),
            logo: value.logo.clone().map(Into::into),
            metadata: Metadata(value.metadata.to_owned()),
            owned_by: AccountId(value.owned_by.to_owned()),
            total_quantity: Decimal::from(&value.total_quantity()),
        }
    }
}

#[derive(ToSchema, Serialize)]
pub struct Nft {
    id: NftId,
    owned_by: AccountId,
    content: Metadata,
}

impl From<iroha::NftEntry<'_>> for Nft {
    fn from(value: iroha::NftEntry<'_>) -> Self {
        Self {
            id: NftId(value.id().to_owned()),
            owned_by: AccountId(value.owned_by().to_owned()),
            content: Metadata(value.content().to_owned()),
        }
    }
}

/// Asset Definition ID. Represented in a form of `asset#domain`.
#[derive(ToSchema, Serialize, Deserialize)]
#[schema(value_type = String, example = "roses#wonderland")]
pub struct AssetDefinitionId(pub iroha::AssetDefinitionId);

/// Non-fungible token ID. Represented in a form of `nft$domain`.
#[derive(ToSchema, Serialize, Deserialize)]
#[schema(value_type = String, example = "rose$wonderland")]
pub struct NftId(pub iroha::NftId);

#[derive(ToSchema, Serialize)]
pub enum Mintable {
    Infinitely,
    Once,
    Not,
}

impl From<iroha::Mintable> for Mintable {
    fn from(value: iroha::Mintable) -> Self {
        match value {
            iroha::Mintable::Infinitely => Self::Infinitely,
            iroha::Mintable::Once => Self::Once,
            iroha::Mintable::Not => Self::Not,
        }
    }
}

#[derive(ToSchema, Serialize)]
pub struct Asset {
    id: AssetId,
    value: Decimal,
}

impl From<iroha::AssetEntry<'_>> for Asset {
    fn from(value: iroha::AssetEntry<'_>) -> Self {
        Self {
            id: AssetId(value.id().to_owned()),
            value: value.value().into(),
        }
    }
}

/// Asset ID. Union of [`AssetDefinitionId`] (`name#domain`) and [`AccountId`] (`signatory@domain`).
///
/// Represented as:
///
/// - `asset#asset_domain#signatory@account_domain`
/// - `asset##signatory@domain` - when both the asset definition and the account are in the same domain
#[derive(ToSchema, Serialize, Deserialize)]
#[schema(value_type = String, example = "roses##ed0120B23E14F659B91736AAB980B6ADDCE4B1DB8A138AB0267E049C082A744471714E@wonderland")]
pub struct AssetId(pub iroha::AssetId);

// TODO: figure out how to represent decimal
#[derive(ToSchema, Serialize)]
pub struct Decimal(String);

impl From<&iroha::Numeric> for Decimal {
    fn from(value: &iroha::Numeric) -> Self {
        // TODO check in tests
        Self(format!("{value}"))
    }
}

/// Key-value map with arbitrary data
#[derive(Serialize, ToSchema)]
#[schema(
    value_type = Object,
    example = json!({
        "test": {
            "whatever": ["foo","bar"]
        }
    })
)]
pub struct Metadata(pub iroha::Metadata);

/// IPFS path
#[derive(Serialize, ToSchema)]
#[schema(value_type = String)]
pub struct IpfsPath(pub String);

impl From<iroha::IpfsPath> for IpfsPath {
    fn from(value: iroha::IpfsPath) -> Self {
        Self(value.to_string())
    }
}

pub struct ScaleBinary<T>(T);

impl<T> PartialSchema for ScaleBinary<T> {
    fn schema() -> RefOr<Schema> {
        utoipa::openapi::ObjectBuilder::new()
            .description(Some(
                "Value represented as SCALE-encoded binary data in base64.\n\n\
                 Should usually be decoded on the client side using [`@iroha/core`](https://jsr.io/@iroha/core).",
            ))
            .schema_type(utoipa::openapi::schema::Type::String)
            .content_encoding("base64")
            .examples([json!("AQIMd3Vm")])
            .into()
    }
}

impl<T> ToSchema for ScaleBinary<T> {
    fn name() -> Cow<'static, str> {
        Cow::Borrowed("ScaleBinary")
    }
}

impl<T> Serialize for ScaleBinary<T>
where
    T: parity_scale_codec::Encode,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let scale = self.0.encode();
        let as_str = base64::prelude::BASE64_STANDARD.encode(&scale);
        serializer.serialize_str(&as_str)
    }
}

#[derive(Debug)]
pub struct ReprScaleJson<T>(pub T);

impl<T> PartialSchema for ReprScaleJson<T> {
    fn schema() -> RefOr<Schema> {
        utoipa::openapi::ObjectBuilder::new()
            .description(Some(
                "Generic container of some value, representing it in both JSON and SCALE",
            ))
            .property("scale", ScaleBinary::<T>::schema())
            .property(
                "json",
                utoipa::openapi::ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::SchemaType::AnyValue)
                    .description(Some("Value represented as JSON")),
            )
            .examples([json!({
                "scale": "AQIMd3Vm",
                "json": {"level": "INFO","msg": "wuf"}
            })])
            .into()
    }
}

impl<T> ToSchema for ReprScaleJson<T> {
    fn name() -> Cow<'static, str> {
        Cow::Borrowed("ReprScaleJson")
    }

    // fn schemas(schemas: &mut Vec<(String, RefOr<Schema>)>) {
    //     schemas.push((
    //         "Object".to_string(),
    //         schema!(
    //             #[inline]
    //             Value
    //         )
    //         .description(Some("TODO schemas desc"))
    //         .into(),
    //     ));
    //     schemas.push((
    //         ScaleBinary::<()>::name().into(),
    //         ScaleBinary::<()>::schema(),
    //     ));
    // }
}

impl<T> Serialize for ReprScaleJson<T>
where
    T: parity_scale_codec::Encode + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("scale", &ScaleBinary(&self.0))?;
        map.serialize_entry("json", &self.0)?;
        map.end()
    }
}

/// Big integer numeric value.
///
/// Serialized as a **number** when safely fits into `f64` max safe integer
/// (less than `pow(2, 53) - 1`, i.e. `9007199254740991`), and as a **string** otherwise.
///
/// On the JavaScript side is recommended to parse with `BigInt`.
#[derive(Debug, ToSchema, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
// TODO set `value_type` to union of string and number
#[schema(example = 42)]
pub struct BigInt(pub u128);

impl From<u32> for BigInt {
    fn from(value: u32) -> Self {
        Self(value as u128)
    }
}

impl From<u64> for BigInt {
    fn from(value: u64) -> Self {
        Self(value as u128)
    }
}

impl From<NonZero<u64>> for BigInt {
    fn from(value: NonZero<u64>) -> Self {
        Self::from(value.get())
    }
}

impl BigInt {
    pub const MAX_SAFE_INTEGER: u128 = 2u128.pow(53) - 1;
}

impl Serialize for BigInt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.0 > Self::MAX_SAFE_INTEGER {
            // TODO: optimise
            serializer.serialize_str(&self.0.to_string())
        } else {
            serializer.serialize_u128(self.0)
        }
    }
}

/// Information on data pagination
#[derive(Debug, Serialize, ToSchema)]
#[schema(
    example = json!({
        "page": 1,
        "page_size": 10,
        "total_items": 89
    })
)]
pub struct Pagination {
    /// Page number, starts from 1
    pub page: PositiveInteger,
    /// Items per page, starts from 1
    pub per_page: PositiveInteger,
    /// Total number of pages. Not always available.
    pub total_pages: usize,
    /// Total number of items. Not always available.
    pub total_items: usize,
}

impl Pagination {
    pub fn new(
        page: PositiveInteger,
        per_page: PositiveInteger,
        total_items: usize,
        total_pages: usize,
    ) -> Self {
        Self {
            page,
            per_page,
            total_items,
            total_pages,
        }
    }

    pub fn for_empty_data(per_page: PositiveInteger) -> Self {
        Self {
            // "there is one page, it's just empty"
            page: PositiveInteger(nonzero!(1usize)),
            per_page,
            // "but there are zero pages of data"
            total_pages: 0,
            total_items: 0,
        }
    }
}

impl From<ReversePagination> for Pagination {
    fn from(value: ReversePagination) -> Self {
        Self::new(
            PositiveInteger(value.page()),
            PositiveInteger(value.per_page()),
            value.total_items().get(),
            value.total_pages().get(),
        )
    }
}

impl From<DirectPagination> for Pagination {
    fn from(value: DirectPagination) -> Self {
        Self::new(
            PositiveInteger(value.page()),
            PositiveInteger(value.per_page()),
            value.total_items().get(),
            value.total_pages().get(),
        )
    }
}

/// Generic paginated data container
#[derive(Debug, Serialize, ToSchema)]
pub struct Page<T> {
    /// Pagination info
    pub pagination: Pagination,
    /// Page items
    pub items: Vec<T>,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, pagination: Pagination) -> Self {
        Self { pagination, items }
    }

    pub fn empty(per_page: PositiveInteger) -> Self {
        Self::new(vec![], Pagination::for_empty_data(per_page))
    }

    pub fn map<U>(self, f: impl Fn(T) -> U) -> Page<U> {
        Page {
            pagination: self.pagination,
            items: self.items.into_iter().map(f).collect(),
        }
    }
}

/// Integer greater than zero
#[derive(ToSchema, Copy, Clone, Debug, Serialize, Deserialize)]
#[schema(value_type = usize)]
pub struct PositiveInteger(pub NonZero<usize>);

impl Deref for PositiveInteger {
    type Target = NonZero<usize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for PositiveInteger {
    type Err = <NonZero<usize> as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = NonZero::from_str(s)?;
        Ok(Self(value))
    }
}

impl From<PositiveInteger> for NonZero<usize> {
    fn from(value: PositiveInteger) -> Self {
        value.0
    }
}

impl From<NonZero<usize>> for PositiveInteger {
    fn from(value: NonZero<usize>) -> Self {
        Self(value)
    }
}

// FIXME: params details is not rendered fully, only docs
/// Pagination query parameters
#[derive(Deserialize, IntoParams, Clone, Copy)]
pub struct PaginationQueryParams {
    /// Page number, optional. Different endpoints interpret value absense differently.
    #[param(example = 3, minimum = 1)]
    pub page: Option<PositiveInteger>,
    /// Items per page
    #[param(example = 15, minimum = 1)]
    #[serde(default = "default_per_page")]
    pub per_page: PositiveInteger,
}

/// Encapsulates a repetitive pattern
pub enum PaginationOrEmpty<P, T> {
    Some(P),
    Empty(Page<T>),
}

impl PaginationQueryParams {
    pub fn parse_into_direct<T>(&self, total: usize) -> PaginationOrEmpty<DirectPagination, T> {
        match NonZero::new(total) {
            Some(total) => PaginationOrEmpty::Some(DirectPagination::new(
                self.page
                    .unwrap_or(PositiveInteger(nonzero!(1usize)))
                    .into(),
                self.per_page.into(),
                total,
            )),
            None => PaginationOrEmpty::Empty(Page::empty(self.per_page)),
        }
    }

    pub fn parse_into_reverse<T>(
        &self,
        total: usize,
    ) -> Result<PaginationOrEmpty<ReversePagination, T>, ReversePaginationError> {
        let res = match NonZero::new(total) {
            Some(total) => PaginationOrEmpty::Some(ReversePagination::new(
                total,
                self.per_page.into(),
                self.page.map(From::from),
            )?),
            None => PaginationOrEmpty::Empty(Page::empty(self.per_page)),
        };
        Ok(res)
    }
}

const fn default_per_page() -> PositiveInteger {
    // FIXME: does it work as `const VAR = ...; VAR`?
    PositiveInteger(const { nonzero!(10usize) })
}

/// Timestamp
#[derive(Serialize, ToSchema, Debug, Clone)]
#[schema(example = "2024-08-11T23:08:58Z")]
pub struct TimeStamp(chrono::DateTime<Utc>);

impl From<DateTime<Utc>> for TimeStamp {
    fn from(value: DateTime<Utc>) -> Self {
        Self(value)
    }
}

impl TimeStamp {
    pub fn from_duration_timestamp(value: std::time::Duration) -> Self {
        let datetime = DateTime::from_timestamp(
            value
                .as_secs()
                .try_into()
                .expect("not handling invalid timestamps"),
            value.subsec_nanos(),
        )
        .expect("not handling invalid timestamps");
        Self(datetime)
    }
}

/// Transaction status
#[derive(Serialize, Deserialize, ToSchema, Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
#[serde(rename = "lowercase")]
pub enum TransactionStatus {
    Committed,
    Rejected,
}

// TODO: autogenerate?
impl Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Committed => "committed",
            Self::Rejected => "rejected",
        })
    }
}

#[derive(Serialize, ToSchema)]
pub struct TransactionBase {
    pub hash: Hash,
    pub block: BigInt,
    pub created_at: TimeStamp,
    pub authority: AccountId,
    pub executable: Executable,
    pub status: TransactionStatus,
}

// impl From<&query::BlockTransactionRef> for TransactionBase {
//     fn from(value: &query::BlockTransactionRef) -> Self {
//         Self {
//             hash: value.transaction().hash().into(),
//             block: BigInt(value.block().header().height().get() as u128),
//             created_at: TimeStamp::from_duration_timestamp(value.transaction().creation_time()),
//             authority: value.transaction().authority().into(),
//             executable: value.transaction().instructions().into(),
//             status: value.status(),
//         }
//     }
// }
//
// impl From<query::BlockTransactionRef> for TransactionBase {
//     fn from(value: query::BlockTransactionRef) -> Self {
//         Self::from(&value)
//     }
// }

// impl From<repo::TransactionBase> for TransactionBase {
//     fn from(value: repo::TransactionBase) -> Self {
//         Self {
//             hash: value.hash.into(),
//             block: BigInt(value.block as u128),
//             created_at: TimeStamp(value.created_at),
//             authority: value.authority.into(),
//             executable: value.executable.into(),
//             status: value.status,
//         }
//     }
// }

#[derive(Serialize, ToSchema)]
pub struct TransactionDetailed {
    #[serde(flatten)]
    pub base: TransactionBase,
    pub signature: Signature,
    pub nonce: Option<PositiveInteger>,
    pub metadata: Metadata,
    pub time_to_live: Option<TimeDuration>,
    // FIXME: make it nullable
    #[schema(schema_with = rejection_reason_schema)]
    pub rejection_reason: Option<ReprScaleJson<iroha::TransactionRejectionReason>>,
}

fn rejection_reason_schema() -> impl Into<RefOr<Schema>> {
    let RefOr::T(Schema::Object(mut object)) = ReprScaleJson::<()>::schema() else {
        unreachable!()
    };
    object.description = Some("_(nullable)_ Corresponding type: [`TransactionRejectionReason`](https://jsr.io/@iroha/core@0.3.1/doc/data-model/~/TransactionRejectionReason)".to_owned());
    object
}

// impl From<&query::BlockTransactionRef> for TransactionDetailed {
//     fn from(value: &query::BlockTransactionRef) -> Self {
//         Self {
//             base: value.into(),
//             signature: value.signature().into(),
//             nonce: value.nonce().map(|int| PositiveInteger(int)),
//             metadata: value.metadata().into(),
//             time_to_live: value.time_to_live().map(TimeDuration::from),
//             rejection_reason: value.error().map(|reason| ReprScaleJson(reason)),
//         }
//     }
// }
//
// impl From<query::BlockTransactionRef> for TransactionDetailed {
//     fn from(value: query::BlockTransactionRef) -> Self {
//         Self::from(&value)
//     }
// }

// /// Transaction rejection reason
// #[derive(Serialize, ToSchema)]
// // #[schema(value_type = Object)]
// // #[schema(schema_with = ReprScaleJson::<iroha::InstructionBox>::schema)]
// pub struct TransactionRejectionReason(
//     // #[schema(schema_with = ReprScaleJson::<iroha::TransactionRejectionReason>::schema)]
//     // ReprScaleJson<iroha::TransactionRejectionReason>,
//     // ScaleBinary<iroha::TransactionRejectionReason>,
// );

/// Operations executable on-chain
#[derive(Serialize, ToSchema)]
pub enum Executable {
    /// Array of instructions
    Instructions,
    /// WebAssembly smart contract
    Wasm,
}

// impl From<repo::Executable> for Executable {
//     fn from(value: repo::Executable) -> Self {
//         match value {
//             repo::Executable::Instructions => Self::Instructions,
//             repo::Executable::WASM => Self::Wasm,
//         }
//     }
// }

impl From<&iroha::Executable> for Executable {
    fn from(value: &iroha::Executable) -> Self {
        match value {
            iroha::Executable::Instructions(_) => Self::Instructions,
            iroha::Executable::Wasm(_) => Self::Wasm,
        }
    }
}

/// Iroha Special Instruction (ISI)
#[derive(Serialize, ToSchema, Debug)]
#[schema(bound = "")]
pub struct Instruction {
    /// Kind of instruction.
    pub kind: InstructionKind,
    #[schema(schema_with = isi_box_schema)]
    pub r#box: ReprScaleJson<iroha::InstructionBox>,
    pub transaction_hash: Hash,
    pub transaction_status: TransactionStatus,
    pub block: BigInt,
    pub authority: AccountId,
    pub created_at: TimeStamp,
}

fn isi_box_schema() -> impl Into<RefOr<Schema>> {
    let RefOr::T(Schema::Object(mut object)) = ReprScaleJson::<()>::schema() else {
        unreachable!()
    };
    object.description = Some("Corresponding type: [`InstructionBox`](https://jsr.io/@iroha/core@0.3.1/doc/data-model/~/InstructionBox)".to_owned());
    object
}

// impl From<repo::Instruction> for Instruction {
//     fn from(value: repo::Instruction) -> Self {
//         Self {
//             kind: value.kind,
//             r#box: ReprScaleJson(value.r#box.0),
//             transaction_hash: Hash(value.transaction_hash.0 .0),
//             transaction_status: value.transaction_status,
//             block: BigInt(value.block as u128),
//             authority: AccountId(value.authority.0 .0),
//             created_at: TimeStamp(value.created_at),
//         }
//     }
// }

/// Kind of instruction
#[derive(Deserialize, Serialize, ToSchema, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum InstructionKind {
    Register,
    Unregister,
    Mint,
    Burn,
    Transfer,
    SetKeyValue,
    RemoveKeyValue,
    Grant,
    Revoke,
    ExecuteTrigger,
    SetParameter,
    Upgrade,
    Log,
    Custom,
}

impl InstructionKind {
    pub fn matches_original(&self, other: &iroha::InstructionBox) -> bool {
        match (self, other) {
            (Self::Register, iroha::InstructionBox::Register(_)) => true,
            (Self::Unregister, iroha::InstructionBox::Unregister(_)) => true,
            (Self::Mint, iroha::InstructionBox::Mint(_)) => true,
            (Self::Burn, iroha::InstructionBox::Burn(_)) => true,
            (Self::Transfer, iroha::InstructionBox::Transfer(_)) => true,
            (Self::SetKeyValue, iroha::InstructionBox::SetKeyValue(_)) => true,
            (Self::RemoveKeyValue, iroha::InstructionBox::RemoveKeyValue(_)) => true,
            (Self::Grant, iroha::InstructionBox::Grant(_)) => true,
            (Self::Revoke, iroha::InstructionBox::Revoke(_)) => true,
            (Self::ExecuteTrigger, iroha::InstructionBox::ExecuteTrigger(_)) => true,
            (Self::SetParameter, iroha::InstructionBox::SetParameter(_)) => true,
            (Self::Upgrade, iroha::InstructionBox::Upgrade(_)) => true,
            (Self::Log, iroha::InstructionBox::Log(_)) => true,
            (Self::Custom, iroha::InstructionBox::Custom(_)) => true,
            _ => false,
        }
    }
}

impl From<&iroha::InstructionBox> for InstructionKind {
    fn from(value: &iroha::InstructionBox) -> Self {
        match value {
            iroha::InstructionBox::Register(_) => Self::Register,
            iroha::InstructionBox::Unregister(_) => Self::Unregister,
            iroha::InstructionBox::Mint(_) => Self::Mint,
            iroha::InstructionBox::Burn(_) => Self::Burn,
            iroha::InstructionBox::Transfer(_) => Self::Transfer,
            iroha::InstructionBox::SetKeyValue(_) => Self::SetKeyValue,
            iroha::InstructionBox::RemoveKeyValue(_) => Self::RemoveKeyValue,
            iroha::InstructionBox::Grant(_) => Self::Grant,
            iroha::InstructionBox::Revoke(_) => Self::Revoke,
            iroha::InstructionBox::ExecuteTrigger(_) => Self::ExecuteTrigger,
            iroha::InstructionBox::SetParameter(_) => Self::SetParameter,
            iroha::InstructionBox::Upgrade(_) => Self::Upgrade,
            iroha::InstructionBox::Log(_) => Self::Log,
            iroha::InstructionBox::Custom(_) => Self::Custom,
        }
    }
}

/// Block
#[derive(Serialize, ToSchema)]
pub struct Block {
    /// Number of blocks in the chain including this block
    pub height: BigInt,
    /// Block hash
    pub hash: Hash,
    /// Hash of the previous block in the chain
    pub prev_block_hash: Option<Hash>,
    /// Hash of merkle tree root of transactions' hashes.
    ///
    /// The block is _empty_ if this is `null`.
    pub transactions_hash: Option<Hash>,
    /// Timestamp of creation
    pub created_at: TimeStamp,
    pub transactions_total: u32,
    pub transactions_rejected: u32,
}

impl From<&iroha_data_model::block::SignedBlock> for Block {
    fn from(value: &iroha_data_model::block::SignedBlock) -> Self {
        let transactions_total = value.payload().transactions.len() as u32;
        let transactions_rejected = value.errors().len() as u32;

        Self {
            hash: Hash(value.hash().into()),
            height: BigInt(value.header().height().get() as u128),
            prev_block_hash: value.header().prev_block_hash().map(|x| Hash(x.into())),
            transactions_hash: value.header().transactions_hash().map(|x| Hash(x.into())),
            transactions_total,
            transactions_rejected,
            created_at: TimeStamp(
                DateTime::from_timestamp_millis(value.header().creation_time_ms as i64).unwrap(),
            ),
        }
    }
}

/// Hex-encoded hash
#[derive(Deserialize, Serialize, ToSchema, Debug)]
#[schema(value_type = String, example = "1B0A52DBDC11EAE39DD0524AD5146122351527CE00D161EA8263EA7ADE4164AF")]
pub struct Hash(pub iroha::Hash);

impl<T> From<iroha::HashOf<T>> for Hash {
    fn from(value: iroha::HashOf<T>) -> Self {
        Self(value.into())
    }
}

/// Hex-encoded signature
#[derive(Serialize, ToSchema, Debug)]
#[schema(
    value_type = Object,
    example = json!({
        "payload": "19569E8D7A44AE93972D66BFF9B5316587CC80907B52CB667BB525B152B1591D1B1E9D89E1A67F534CE040E1FB9F18DA3B546553E111020DEFF859094FEE7A0B"
    })
)]
// FIXME: utoipa doesn't display example
pub struct Signature(pub iroha::Signature);

/// Duration
#[derive(ToSchema, Serialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct TimeDuration {
    /// Number of milliseconds
    ms: BigInt,
}

impl From<std::time::Duration> for TimeDuration {
    fn from(value: std::time::Duration) -> Self {
        Self {
            ms: BigInt(value.as_millis()),
        }
    }
}

impl TimeDuration {
    pub fn from_millis(ms: impl Into<BigInt>) -> Self {
        Self { ms: ms.into() }
    }
}

#[derive(DeserializeFromStr)]
pub enum BlockHeightOrHash {
    Height(PositiveInteger),
    Hash(iroha::Hash),
}

impl FromStr for BlockHeightOrHash {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(value) = s.parse::<PositiveInteger>() {
            return Ok(Self::Height(value));
        }
        if let Ok(value) = s.parse::<iroha::Hash>() {
            return Ok(Self::Hash(value));
        }
        Err("value should be either a non-zero positive integer or a hash")
    }
}

/// Metrics of the network as a whole
#[derive(Serialize, ToSchema, Clone)]
pub struct NetworkStatus {
    /// Count of peers in the network
    pub peers: usize,
    /// Count of registered domains
    pub domains: usize,
    /// Count of registered accounts
    pub accounts: usize,
    /// Count of assets and NFTs
    pub assets: usize,
    /// Accepted transactions
    pub transactions_accepted: usize,
    /// Rejected transactions
    pub transactions_rejected: usize,
    /// Height of the latest committed block
    pub block: usize,
    /// Timestamp when the last block was created (not committed)
    pub block_created_at: Option<TimeStamp>,
    /// Finalized block, the one that __cannot be reverted__ under normal network conditions.
    ///
    /// Might be not available if there are not enough metrics from peers.
    pub finalized_block: Option<usize>,
    /// Average commit time among all peers during a certain observation period.
    ///
    /// Might be not available if there are not enough metrics from peers.
    pub avg_commit_time: Option<TimeDuration>,
    /// Average time between created blocks during a certain observation period
    pub avg_block_time: TimeDuration,
    // /// Pipeline time calculated from Sumeragi parameters
    // pub pipeline_time: TimeDuration,
    // TODO: pipeline time? txs per block?
}

// On frontend, create two maps: Map<url, peer info> | Map<pub key, peer status>
// In the main table, display based on the info map (url, connected, lookup metrics)
// Also collect "unknown" peers and display them in a separate space
// When info

/// Metrics of a single peer
#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct PeerStatus {
    /// Peer URL
    pub url: ToriiUrl,
    /// Block height
    pub block: usize,
    /// Commit time of the last block
    pub commit_time: TimeDuration,
    /// Average commit time on this peer during a certain observation period
    pub avg_commit_time: TimeDuration,
    /// Current queue size
    pub queue_size: usize,
    /// Uptime since genesis block commit
    pub uptime: TimeDuration,
}

impl PartialEq for PeerStatus {
    fn eq(&self, other: &Self) -> bool {
        self.url.eq(&other.url)
    }
}

impl PartialOrd for PeerStatus {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for PeerStatus {}

impl Ord for PeerStatus {
    fn cmp(&self, other: &Self) -> Ordering {
        self.url.cmp(&other.url)
    }
}

/// Static information about peer
#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct PeerInfo {
    /// Peer URL
    pub url: ToriiUrl,
    /// Connection status to the peer
    pub connected: bool,
    /// Peer does not support telemetry.
    ///
    /// Therefore, its status is not available.
    pub telemetry_unsupported: bool,
    /// Peer configuration, including its public key and some other parameters.
    ///
    /// Always present when connected, but could be null if peer was never connected.
    pub config: Option<PeerConfig>,
    /// Location of the peer, if known
    pub location: Option<GeoLocation>,
    /// Set of connected peers
    pub connected_peers: Option<BTreeSet<PublicKey>>,
}

impl PartialEq for PeerInfo {
    fn eq(&self, other: &Self) -> bool {
        self.url.eq(&other.url)
    }
}

impl PartialOrd for PeerInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for PeerInfo {}

impl Ord for PeerInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.url.cmp(&other.url)
    }
}

/// Peer configuration
#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct PeerConfig {
    /// Public key of the peer
    pub public_key: PublicKey,
    /// Queue capacity
    pub queue_capacity: u32,
    /// Block gossip batch size
    pub network_block_gossip_size: u32,
    /// Block gossip period
    pub network_block_gossip_period: TimeDuration,
    /// Transactions gossip batch size
    pub network_tx_gossip_size: u32,
    /// Transactions gossip period
    pub network_tx_gossip_period: TimeDuration,
}

impl From<iroha::ConfigGetDTO> for PeerConfig {
    fn from(value: iroha::ConfigGetDTO) -> Self {
        Self {
            public_key: PublicKey(value.public_key),
            queue_capacity: value.queue.capacity.get() as u32,
            network_block_gossip_size: value.network.block_gossip_size.get(),
            network_block_gossip_period: TimeDuration::from_millis(
                value.network.block_gossip_period_ms,
            ),
            network_tx_gossip_size: value.network.transaction_gossip_size.get(),
            network_tx_gossip_period: TimeDuration::from_millis(
                value.network.transaction_gossip_period_ms,
            ),
        }
    }
}

/// Container for possible messages returned from the telemetry live updates stream.
///
/// Variants are distinguished by the `kind` tag.
#[derive(Clone, Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TelemetryStreamMessage {
    /// First message, reflecting system metrics at the beginning of the connection.
    ///
    /// Sent immediately upon connection.
    First(TelemetryStreamFirstMessage),
    /// Network status update
    NetworkStatus(NetworkStatus),
    /// Peer status (i.e. dynamic metrics) update
    PeerStatus(PeerStatus),
    /// Peer info (i.e. more static data) update
    PeerInfo(PeerInfo),
}

#[derive(Serialize, ToSchema, Clone)]
pub struct TelemetryStreamFirstMessage {
    /// Available peers info
    pub peers_info: BTreeSet<PeerInfo>,
    /// Available peers status
    pub peers_status: BTreeSet<PeerStatus>,
    /// Available network status
    pub network_status: Option<NetworkStatus>,
}

/// Public key multihash
#[derive(Serialize, ToSchema, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
#[schema(value_type = String)]
pub struct PublicKey(pub iroha::PublicKey);

/// Geographical location
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq)]
pub struct GeoLocation {
    /// Latitude
    pub lat: f64,
    /// Longitude
    pub lon: f64,
    /// Country name
    pub country: String,
    /// City name
    pub city: String,
}

/// Torii URL
#[derive(
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
    Clone,
    Debug,
    derive_more::Display,
    derive_more::FromStr,
    ToSchema,
)]
#[schema(value_type = String)]
pub struct ToriiUrl(pub Url);

impl Serialize for ToriiUrl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        todo!()
    }
}

#[derive(IntoParams, Deserialize)]
pub struct AccountsIndexFilter {
    /// Select accounts owning specified asset
    pub with_asset: Option<AssetDefinitionId>,
    /// Select accounts from specified domain
    pub domain: Option<DomainId>,
}

#[derive(IntoParams, Deserialize)]
pub struct AssetDefinitionsIndexFilter {
    /// Filter by domain
    pub domain: Option<DomainId>,
    /// Filter by owner
    pub owned_by: Option<AccountId>,
}

#[derive(Deserialize, IntoParams)]
pub struct TransactionsIndexFilter {
    /// Select by authority
    pub authority: Option<AccountId>,
    /// Select by block
    // FIX: this must be non-zero
    pub block: Option<u64>,
    /// Filter by transaction status
    pub status: Option<TransactionStatus>,
}

#[derive(Deserialize, IntoParams)]
pub struct InstructionsIndexFilter {
    pub transaction_hash: Option<Hash>,
    /// Select by transaction status
    pub transaction_status: Option<TransactionStatus>,
    /// Select by block
    pub block: Option<PositiveInteger>,
    /// Filter by a kind of instruction
    pub kind: Option<InstructionKind>,
    /// Filter by the creator of the parent transaction
    pub authority: Option<AccountId>,
}

#[derive(Deserialize, IntoParams)]
pub struct AssetsIndexFilter {
    /// Filter by an owning account
    pub owned_by: Option<AccountId>,
    /// Filter by asset definition
    pub definition: Option<AssetDefinitionId>,
}

#[cfg(test)]
mod tests {
    use super::{
        iroha::{Algorithm, KeyPair},
        *,
    };

    use iroha_explorer_test_utils::ExpectExt as _;
    use serde_json::json;

    #[test]
    fn serialize_bigint() {
        assert_eq!(json!(BigInt(0)), json!(0));
        assert_eq!(
            json!(BigInt(9_007_199_254_740_991)),
            json!(9_007_199_254_740_991_u64)
        );
        assert_eq!(
            json!(BigInt(9_007_199_254_740_991 + 1)),
            json!("9007199254740992")
        );
        assert_eq!(
            json!(BigInt(10_000_000_000_000_000_000_000u128)),
            json!("10000000000000000000000")
        );
    }

    #[test]
    fn deserialize_block_height_or_hash() {
        let BlockHeightOrHash::Height(value) =
            serde_json::from_value(json!("412")).expect("should parse")
        else {
            panic!("should be height")
        };
        assert_eq!(value.0, nonzero!(412usize));

        let BlockHeightOrHash::Hash(_) = serde_json::from_value(json!(
            "3E75E5A0277C34756C2FF702963C4B9024A5E00C327CC682D9CA222EB5589DB1"
        ))
        .expect("should parse") else {
            panic!("should be hash")
        };
    }

    #[test]
    fn serialize_asset_id_canonically() {
        let short = "roses##ed0120B23E14F659B91736AAB980B6ADDCE4B1DB8A138AB0267E049C082A744471714E@wonderland";
        let full = "roses#looking_glass#ed0120B23E14F659B91736AAB980B6ADDCE4B1DB8A138AB0267E049C082A744471714E@wonderland";
        for expected in [short, full] {
            let id = iroha::AssetId::from_str(expected).expect("input is valid");
            let value = AssetId(id);
            let serialized = serde_json::to_string(&value).expect("no possible errors expected");
            assert_eq!(serialized, format!("\"{expected}\""));
        }
    }

    #[test]
    fn serialize_asset_definition_id_canonically() {
        let expected = "rose#wonderland";
        let id = iroha::AssetDefinitionId::from_str(expected).expect("input is valid");
        let value = AssetDefinitionId(id);
        let serialized = serde_json::to_string(&value).expect("no possible errors expected");
        assert_eq!(serialized, format!("\"{expected}\""));
    }

    #[test]
    fn serialize_signature() {
        let value = iroha::Signature::new(
            KeyPair::from_seed(vec![1, 2, 3], Algorithm::Secp256k1).private_key(),
            &[5, 4, 3, 2, 1],
        );

        expect_test::expect![[r#""A19E05FFE0939F8B7952819E64B9637A500D767519274F21763E8B4283A77E01223D35FE6DFEC6D513D17E1D902791B6D637AD447E9548767948F5A36B652906""#]]
            .assert_eq(&serde_json::to_string(&Signature(value)).unwrap());
    }

    #[test]
    fn serialize_unified_repr_int() {
        let value = ReprScaleJson(5);
        expect_test::expect![[r#"
            {
              "scale": "BQAAAA==",
              "json": 5
            }"#]]
        .assert_json_eq(value);
    }

    #[test]
    fn serialize_unified_repr_str() {
        let value = ReprScaleJson("test string");
        expect_test::expect![[r#"
            {
              "scale": "LHRlc3Qgc3RyaW5n",
              "json": "test string"
            }"#]]
        .assert_json_eq(value);
    }

    #[test]
    fn serialize_unified_repr_iroha() {
        let value = ReprScaleJson(Some(iroha::Log::new(
            "INFO".parse().unwrap(),
            "wuf".to_owned(),
        )));
        expect_test::expect![[r#"
            {
              "scale": "AQIMd3Vm",
              "json": {
                "level": "INFO",
                "msg": "wuf"
              }
            }"#]]
        .assert_json_eq(value);
    }
}
