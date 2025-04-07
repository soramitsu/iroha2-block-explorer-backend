use crate::repo;
use crate::util::{DirectPagination, ReversePagination};
use base64::Engine;
use chrono::{DateTime, Utc};
use nonzero_ext::nonzero;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::json;
use serde_with::DeserializeFromStr;
use sqlx::prelude::FromRow;
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::num::NonZero;
use std::ops::Deref;
use std::str::FromStr;
use url::Url;
use utoipa::openapi::{RefOr, Schema};
use utoipa::{schema, IntoParams, PartialSchema, ToSchema};

mod iroha {
    pub use iroha::client::ConfigGetDTO;
    pub use iroha::crypto::*;
    pub use iroha::data_model::prelude::*;
}

/// Domain
#[derive(ToSchema, Serialize, FromRow)]
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
    id: DomainId,
    /// Domain logo path
    logo: Option<IpfsPath>,
    /// Domain metadata
    metadata: Metadata,
    /// Domain's owner
    owned_by: AccountId,
    /// Total number of accounts in this domain
    accounts: u32,
    /// Total number of assets _definitions_ in this domain
    assets: u32,
    /// Total number of NFTs in this domain
    nfts: u32,
}

impl From<repo::Domain> for Domain {
    fn from(value: repo::Domain) -> Self {
        Self {
            id: DomainId(value.id.0 .0),
            logo: value.logo.map(|x| IpfsPath(x.0)),
            metadata: Metadata(value.metadata.into()),
            owned_by: AccountId(value.owned_by.0 .0),
            accounts: value.accounts,
            assets: value.assets,
            nfts: value.nfts,
        }
    }
}

/// Domain ID
#[derive(Debug, ToSchema, Serialize, Deserialize)]
#[schema(example = "genesis", value_type = String)]
pub struct DomainId(pub iroha::DomainId);

/// Account
#[derive(Serialize, ToSchema)]
pub struct Account {
    id: AccountId,
    metadata: Metadata,
    owned_domains: u32,
    owned_assets: u32,
    owned_nfts: u32,
}

impl From<repo::Account> for Account {
    fn from(value: repo::Account) -> Self {
        Self {
            id: AccountId(value.id.0 .0),
            metadata: Metadata(value.metadata.into()),
            owned_assets: value.owned_assets,
            owned_nfts: value.owned_nfts,
            owned_domains: value.owned_domains,
        }
    }
}

/// Account ID. Represented as `signatory@domain`.
#[derive(ToSchema, Serialize, Deserialize)]
#[schema(
    example = "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis",
    value_type = String
)]
pub struct AccountId(pub iroha::AccountId);

impl From<repo::AccountId> for AccountId {
    fn from(value: repo::AccountId) -> Self {
        Self(value.0 .0)
    }
}

#[derive(ToSchema, Serialize)]
pub struct AssetDefinition {
    id: AssetDefinitionId,
    mintable: Mintable,
    logo: Option<IpfsPath>,
    metadata: Metadata,
    owned_by: AccountId,
    assets: u32,
}

impl From<repo::AssetDefinition> for AssetDefinition {
    fn from(value: repo::AssetDefinition) -> Self {
        Self {
            id: AssetDefinitionId(value.id.0 .0),
            mintable: match value.mintable {
                repo::Mintable::Infinitely => Mintable::Infinitely,
                repo::Mintable::Once => Mintable::Once,
                repo::Mintable::Not => Mintable::Not,
            },
            logo: value.logo.map(|x| IpfsPath(x.0)),
            metadata: Metadata(value.metadata.into()),
            owned_by: AccountId(value.owned_by.0 .0),
            assets: value.assets,
        }
    }
}

#[derive(ToSchema, Serialize)]
pub struct Nft {
    id: NftId,
    owned_by: AccountId,
    content: Metadata,
}

impl From<repo::Nft> for Nft {
    fn from(value: repo::Nft) -> Self {
        Self {
            id: NftId(value.id.0 .0),
            content: Metadata(value.content.into()),
            owned_by: AccountId(value.owned_by.0 .0),
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

#[derive(ToSchema, Serialize)]
pub struct Asset {
    id: AssetId,
    value: Decimal,
}

impl From<repo::Asset> for Asset {
    fn from(value: repo::Asset) -> Self {
        Self {
            id: AssetId(value.id.0 .0),
            value: Decimal::from(&value.value.0),
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
pub struct Metadata(iroha::Metadata);

impl From<repo::Metadata> for Metadata {
    fn from(value: repo::Metadata) -> Self {
        Self(value.into())
    }
}

/// IPFS path
#[derive(Serialize, ToSchema)]
#[schema(value_type = String)]
pub struct IpfsPath(String);

// #[derive(ToSchema)]
// #[schema(bound = "T: parity_scale_codec::Encode")]
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

// /// Generic container of some value, representing it in both JSON and SCALE
// #[derive(ToSchema, Serialize)]
// #[schema(
//     example = json!({"scale": "AQIMd3Vm","json": {"level": "INFO","msg": "wuf"}})
// )]
// pub struct ReprScaleJson2 {
//     /// JSON representation of the value
//     #[schema(value_type = Object)]
//     json: Box<dyn Serialize>,
//     /// SCALE representation of the value, `base64`-encoded
//     #[schema(value_type = String, content_encoding = "base64")]
//     scale: Box<dyn ScaleEncode>,
// }

// impl ReprScaleJson2 {
//     fn
// }

pub struct ReprScaleJson<T>(T);

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
    pub total_pages: u64,
    /// Total number of items. Not always available.
    pub total_items: u64,
}

impl Pagination {
    pub fn new(
        page: PositiveInteger,
        per_page: PositiveInteger,
        total_items: u64,
        total_pages: u64,
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
            page: PositiveInteger(nonzero!(1u64)),
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
#[schema(value_type = u64)]
pub struct PositiveInteger(pub NonZero<u64>);

impl Deref for PositiveInteger {
    type Target = NonZero<u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for PositiveInteger {
    type Err = <NonZero<u64> as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = NonZero::from_str(s)?;
        Ok(Self(value))
    }
}

impl From<PositiveInteger> for NonZero<u64> {
    fn from(value: PositiveInteger) -> Self {
        value.0
    }
}

impl From<NonZero<u64>> for PositiveInteger {
    fn from(value: NonZero<u64>) -> Self {
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

const fn default_per_page() -> PositiveInteger {
    // FIXME: does it work as `const VAR = ...; VAR`?
    PositiveInteger(const { nonzero!(10u64) })
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

/// Transaction status
#[derive(
    Serialize, Deserialize, ToSchema, Debug, sqlx::Type, Ord, PartialOrd, Eq, PartialEq, Copy, Clone,
)]
#[serde(rename = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum TransactionStatus {
    Committed,
    Rejected,
}

#[derive(Serialize, ToSchema)]
pub struct TransactionBase {
    hash: Hash,
    block: BigInt,
    created_at: TimeStamp,
    authority: AccountId,
    executable: Executable,
    status: TransactionStatus,
}

impl From<repo::TransactionBase> for TransactionBase {
    fn from(value: repo::TransactionBase) -> Self {
        Self {
            hash: value.hash.into(),
            block: BigInt(value.block as u128),
            created_at: TimeStamp(value.created_at),
            authority: value.authority.into(),
            executable: value.executable.into(),
            status: value.status,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct TransactionDetailed {
    #[serde(flatten)]
    base: TransactionBase,
    signature: Signature,
    nonce: Option<PositiveInteger>,
    metadata: Metadata,
    time_to_live: TimeDuration,
    // FIXME: make it nullable
    #[schema(schema_with = rejection_reason_schema)]
    rejection_reason: Option<ReprScaleJson<iroha::TransactionRejectionReason>>,
}

fn rejection_reason_schema() -> impl Into<RefOr<Schema>> {
    let RefOr::T(Schema::Object(mut object)) = ReprScaleJson::<()>::schema() else {
        unreachable!()
    };
    object.description = Some("_(nullable)_ Corresponding type: [`TransactionRejectionReason`](https://jsr.io/@iroha/core@0.3.1/doc/data-model/~/TransactionRejectionReason)".to_owned());
    object
}

impl From<repo::TransactionDetailed> for TransactionDetailed {
    fn from(value: repo::TransactionDetailed) -> Self {
        Self {
            base: value.base.into(),
            signature: value.signature.into(),
            nonce: value.nonce.map(|int| {
                PositiveInteger(NonZero::new(int.get() as u64).expect("it is non-zero"))
            }),
            metadata: value.metadata.into(),
            time_to_live: TimeDuration {
                ms: BigInt(value.time_to_live_ms as u128),
            },
            rejection_reason: value.rejection_reason.map(|reason| ReprScaleJson(reason.0)),
        }
    }
}

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

impl From<repo::Executable> for Executable {
    fn from(value: repo::Executable) -> Self {
        match value {
            repo::Executable::Instructions => Self::Instructions,
            repo::Executable::WASM => Self::Wasm,
        }
    }
}

/// Iroha Special Instruction (ISI)
#[derive(Serialize, ToSchema)]
#[schema(bound = "")]
pub struct Instruction {
    /// Kind of instruction.
    kind: InstructionKind,
    #[schema(schema_with = isi_box_schema)]
    r#box: ReprScaleJson<iroha::InstructionBox>,
    transaction_hash: Hash,
    transaction_status: TransactionStatus,
    block: BigInt,
    authority: AccountId,
    created_at: TimeStamp,
}

fn isi_box_schema() -> impl Into<RefOr<Schema>> {
    let RefOr::T(Schema::Object(mut object)) = ReprScaleJson::<()>::schema() else {
        unreachable!()
    };
    object.description = Some("Corresponding type: [`InstructionBox`](https://jsr.io/@iroha/core@0.3.1/doc/data-model/~/InstructionBox)".to_owned());
    object
}

impl From<repo::Instruction> for Instruction {
    fn from(value: repo::Instruction) -> Self {
        Self {
            kind: value.kind,
            r#box: ReprScaleJson(value.r#box.0),
            transaction_hash: Hash(value.transaction_hash.0 .0),
            transaction_status: value.transaction_status,
            block: BigInt(value.block as u128),
            authority: AccountId(value.authority.0 .0),
            created_at: TimeStamp(value.created_at),
        }
    }
}

/// Kind of instruction
#[derive(Deserialize, Serialize, ToSchema, sqlx::Type, Debug, Ord, PartialOrd, Eq, PartialEq)]
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

/// Block
#[derive(Serialize, ToSchema)]
pub struct Block {
    /// Number of blocks in the chain including this block
    height: BigInt,
    /// Block hash
    hash: Hash,
    /// Hash of the previous block in the chain
    prev_block_hash: Option<Hash>,
    /// Hash of merkle tree root of transactions' hashes.
    ///
    /// The block is _empty_ if this is `null`.
    transactions_hash: Option<Hash>,
    /// Timestamp of creation
    created_at: TimeStamp,
    transactions_total: u32,
    transactions_rejected: u32,
}

impl From<repo::Block> for Block {
    fn from(value: repo::Block) -> Self {
        Self {
            hash: Hash(value.hash.0 .0),
            height: BigInt(value.height.get() as u128),
            prev_block_hash: value.prev_block_hash.map(Hash::from),
            transactions_hash: value.transactions_hash.map(Hash::from),
            created_at: TimeStamp(value.created_at),
            transactions_total: value.transactions_total,
            transactions_rejected: value.transactions_rejected,
        }
    }
}

/// Hex-encoded hash
#[derive(Deserialize, Serialize, ToSchema)]
#[schema(value_type = String, example = "1B0A52DBDC11EAE39DD0524AD5146122351527CE00D161EA8263EA7ADE4164AF")]
pub struct Hash(pub iroha::Hash);

impl From<repo::Hash> for Hash {
    fn from(value: repo::Hash) -> Self {
        Self(value.0 .0)
    }
}

impl<T> From<iroha::HashOf<T>> for Hash {
    fn from(value: iroha::HashOf<T>) -> Self {
        Self(value.into())
    }
}

/// Hex-encoded signature
#[derive(Serialize, ToSchema)]
#[schema(
    value_type = Object,
    example = json!({
        "payload": "19569E8D7A44AE93972D66BFF9B5316587CC80907B52CB667BB525B152B1591D1B1E9D89E1A67F534CE040E1FB9F18DA3B546553E111020DEFF859094FEE7A0B"
    })
)]
// FIXME: utoipa doesn't display example
pub struct Signature(iroha::Signature);

impl From<repo::Signature> for Signature {
    fn from(value: repo::Signature) -> Self {
        Self(value.0 .0 .0)
    }
}

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
    pub peers: u32,
    /// Count of registered domains
    pub domains: u32,
    /// Count of registered accounts
    pub accounts: u32,
    /// Count of registered assets (definitions) and NFTs
    pub assets: u32,
    /// Accepted transactions
    pub transactions_accepted: u32,
    /// Rejected transactions
    pub transactions_rejected: u32,
    /// Height of the latest committed block
    pub block: u32,
    /// Timestamp when the last block was created (not committed)
    pub block_created_at: TimeStamp,
    /// Finalized block, the one that __cannot be reverted__ under normal network conditions
    ///
    /// Might be not available if there are not enough metrics from peers
    pub finalized_block: Option<u32>,
    /// Average commit time among all peers during a certain observation period
    ///
    /// Might be not available if there are not enough metrics from peers
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
    pub block: u32,
    /// Commit time of the last block
    pub commit_time: TimeDuration,
    /// Average commit time on this peer during a certain observation period
    pub avg_commit_time: TimeDuration,
    /// Current queue size
    pub queue_size: u32,
    /// Uptime since genesis block commit
    pub uptime: TimeDuration,
}

/// Static information about peer
#[derive(Serialize, ToSchema, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct PeerInfo {
    /// Peer URL
    pub url: ToriiUrl,
    /// Connection status to the peer
    pub connected: bool,
    /// Peer configuration, including its public key and some other parameters.
    ///
    /// Always present when connected, but could be null if peer was never connected.
    pub config: Option<PeerConfig>,
    /// Location of the peer, if known
    pub location: Option<GeoLocation>,
    /// List of peers it is connected to
    pub connected_peers: Option<BTreeSet<PublicKey>>,
}

// TODO: use config dto from iroha directly
#[derive(Serialize, ToSchema, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct PeerConfig {
    pub public_key: PublicKey,
    pub queue_capacity: u32,
    pub network_block_gossip_size: u32,
    pub network_block_gossip_period: TimeDuration,
    pub network_tx_gossip_size: u32,
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

#[derive(Clone, Serialize)]
pub enum TelemetryStreamMessage {
    NetworkStatus(NetworkStatus),
    PeerStatus(PeerStatus),
    PeerInfo(PeerInfo),
}

/// Public key multihash
#[derive(Serialize, ToSchema, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
#[schema(value_type = String)]
pub struct PublicKey(pub iroha::PublicKey);

/// Geographical location
#[derive(Serialize, ToSchema, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct GeoLocation {
    /// Latitude
    pub lat: u32,
    /// Longitude
    pub long: u32,
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
    Serialize,
)]
#[schema(value_type = String)]
pub struct ToriiUrl(pub Url);

#[cfg(test)]
mod tests {
    use super::iroha::KeyPair;
    use insta::assert_json_snapshot;
    use serde_json::json;

    use super::*;

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
        assert_eq!(value.0, nonzero!(412u64));

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
            KeyPair::from_seed(vec![1, 2, 3], iroha::Algorithm::Secp256k1).private_key(),
            &[5, 4, 3, 2, 1],
        );

        expect_test::expect![[r#""A19E05FFE0939F8B7952819E64B9637A500D767519274F21763E8B4283A77E01223D35FE6DFEC6D513D17E1D902791B6D637AD447E9548767948F5A36B652906""#]]
            .assert_eq(&serde_json::to_string(&Signature(value)).unwrap());
    }

    #[test]
    fn serialize_unified_repr_int() {
        let value = ReprScaleJson(5);
        assert_json_snapshot!(value);
    }

    #[test]
    fn serialize_unified_repr_str() {
        let value = ReprScaleJson("test string");
        assert_json_snapshot!(value);
    }

    #[test]
    fn serialize_unified_repr_iroha() {
        let value = ReprScaleJson(Some(iroha::Log::new(
            "INFO".parse().unwrap(),
            "wuf".to_owned(),
        )));
        assert_json_snapshot!(value);
    }
}
