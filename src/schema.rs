use std::{num::NonZero, str::FromStr};

use crate::repo;
use crate::util::{DirectPagination, ReversePagination};
use chrono::Utc;
use nonzero_ext::nonzero;
use serde::{Deserialize, Serialize};
use serde_with::DeserializeFromStr;
use sqlx::prelude::FromRow;
use utoipa::{IntoParams, ToSchema};

mod iroha {
    pub use iroha_crypto::Hash;
    pub use iroha_data_model::prelude::*;
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
}

impl From<repo::Account> for Account {
    fn from(value: repo::Account) -> Self {
        Self {
            id: AccountId(value.id.0 .0),
            metadata: Metadata(value.metadata.into()),
            owned_assets: value.owned_assets,
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
    r#type: AssetType,
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
            r#type: match value.r#type {
                repo::AssetType::Numeric => AssetType::Numeric,
                repo::AssetType::Store => AssetType::Store,
            },
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

/// Asset Definition ID. Represented in a form of `asset#domain`.
#[derive(ToSchema, Serialize, Deserialize)]
#[schema(value_type = String, example = "roses#wonderland")]
pub struct AssetDefinitionId(pub iroha::AssetDefinitionId);

#[derive(ToSchema, Serialize)]
pub enum AssetType {
    Numeric,
    Store,
}

#[derive(ToSchema, Serialize)]
pub enum Mintable {
    Infinitely,
    Once,
    Not,
}

#[derive(ToSchema, Serialize)]
pub struct Asset {
    id: AssetId,
    value: AssetValue,
}

impl From<repo::Asset> for Asset {
    fn from(value: repo::Asset) -> Self {
        Self {
            id: AssetId(value.id.0 .0),
            value: match value.value.0 {
                repo::AssetValue::Numeric(numeric) => AssetValue::Numeric {
                    value: Decimal::from(&numeric),
                },
                repo::AssetValue::Store(map) => AssetValue::Store {
                    metadata: Metadata(map.into()),
                },
            },
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

#[derive(ToSchema, Serialize)]
#[serde(tag = "kind")]
pub enum AssetValue {
    Numeric { value: Decimal },
    Store { metadata: Metadata },
}

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

/// Big integer numeric value.
///
/// Serialized as a **number** when safely fits into `f64` max safe integer
/// (less than `pow(2, 53) - 1`, i.e. `9007199254740991`), and as a **string** otherwise.
///
/// On JavaScript side is recommended to parse with `BigInt`.
#[derive(Debug, ToSchema)]
// TODO set `value_type` to union of string and number
#[schema(example = 42)]
pub struct BigInt(pub u128);

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
    pub page: BigInt,
    /// Items per page, starts from 1
    pub per_page: BigInt,
    /// Total number of pages. Not always available.
    pub total_pages: BigInt,
    /// Total number of items. Not always available.
    pub total_items: BigInt,
}

impl Pagination {
    pub fn new(
        page: NonZero<u64>,
        per_page: NonZero<u64>,
        total_items: u64,
        total_pages: u64,
    ) -> Self {
        Self {
            page: BigInt::from(page.get()),
            per_page: BigInt::from(per_page.get()),
            total_items: BigInt::from(total_items),
            total_pages: BigInt::from(total_pages),
        }
    }

    pub fn for_empty_data(per_page: NonZero<u64>) -> Self {
        Self {
            // "there is one page, it's just empty"
            page: BigInt(1),
            per_page: BigInt::from(per_page.get()),
            // "but there are zero pages of data"
            total_pages: BigInt(0),
            total_items: BigInt(0),
        }
    }
}

impl From<ReversePagination> for Pagination {
    fn from(value: ReversePagination) -> Self {
        Self::new(
            value.page(),
            value.per_page(),
            value.total_items().get(),
            value.total_pages().get(),
        )
    }
}

impl From<DirectPagination> for Pagination {
    fn from(value: DirectPagination) -> Self {
        Self::new(
            value.page(),
            value.per_page(),
            value.total_items().get(),
            value.total_pages().get(),
        )
    }
}

/// Generic paginated data container
#[derive(Debug, Serialize, ToSchema)]
#[aliases(DomainsPage = Page<Domain>)]
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

    pub fn empty(per_page: NonZero<u64>) -> Self {
        Self::new(vec![], Pagination::for_empty_data(per_page))
    }

    pub fn map<U>(self, f: impl Fn(T) -> U) -> Page<U> {
        Page {
            pagination: self.pagination,
            items: self.items.into_iter().map(f).collect(),
        }
    }
}

// FIXME: params details is not rendered fully, only docs
/// Pagination query parameters
#[derive(Deserialize, IntoParams, Clone, Copy)]
pub struct PaginationQueryParams {
    /// Page number, optional. Different endpoints interpret value absense differently.
    #[param(example = 3, minimum = 1)]
    pub page: Option<NonZero<u64>>,
    /// Items per page
    #[param(example = 15, minimum = 1)]
    #[serde(default = "default_per_page")]
    pub per_page: NonZero<u64>,
}

const fn default_per_page() -> NonZero<u64> {
    // FIXME: does it work as `const VAR = ...; VAR`?
    const { nonzero!(10u64) }
}

/// Timestamp
#[derive(Serialize, ToSchema, Debug)]
#[schema(example = "2024-08-11T23:08:58Z")]
pub struct TimeStamp(chrono::DateTime<Utc>);

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
    nonce: Option<NonZero<u32>>,
    metadata: Metadata,
    time_to_live: Duration,
    rejection_reason: Option<TransactionRejectionReason>,
}

impl From<repo::TransactionDetailed> for TransactionDetailed {
    fn from(value: repo::TransactionDetailed) -> Self {
        Self {
            base: value.base.into(),
            signature: value.signature.into(),
            nonce: value.nonce,
            metadata: value.metadata.into(),
            time_to_live: Duration {
                ms: BigInt(value.time_to_live_ms as u128),
            },
            rejection_reason: value
                .rejection_reason
                .map(|reason| TransactionRejectionReason(reason.0)),
        }
    }
}

/// Transaction rejection reason
#[derive(Serialize, ToSchema)]
#[schema(value_type = Object)]
pub struct TransactionRejectionReason(iroha::TransactionRejectionReason);

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
pub struct Instruction {
    /// Kind of instruction. TODO: add strict enumeration
    kind: InstructionKind,
    /// Instruction payload, some JSON. TODO: add typed output
    payload: serde_json::Value,
    transaction_hash: Hash,
    transaction_status: TransactionStatus,
    block: BigInt,
    authority: AccountId,
    created_at: TimeStamp,
}

impl From<repo::Instruction> for Instruction {
    fn from(value: repo::Instruction) -> Self {
        Self {
            kind: value.kind,
            payload: value.payload.0,
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
    /// Hash of merkle tree root of transactions' hashes
    transactions_hash: Hash,
    /// Timestamp of creation
    created_at: TimeStamp,
    /// Estimation of consensus duration
    consensus_estimation: Duration,
    transactions_total: u32,
    transactions_rejected: u32,
}

impl From<repo::Block> for Block {
    fn from(value: repo::Block) -> Self {
        Self {
            hash: Hash(value.hash.0 .0),
            height: BigInt(value.height.get() as u128),
            prev_block_hash: value.prev_block_hash.map(Hash::from),
            transactions_hash: Hash(value.transactions_hash.0 .0),
            created_at: TimeStamp(value.created_at),
            consensus_estimation: Duration {
                ms: BigInt(value.consensus_estimation_ms as u128),
            },
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

impl<T> From<iroha_crypto::HashOf<T>> for Hash {
    fn from(value: iroha_crypto::HashOf<T>) -> Self {
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
pub struct Signature(iroha_crypto::Signature);

impl From<repo::Signature> for Signature {
    fn from(value: repo::Signature) -> Self {
        Self(value.0 .0 .0)
    }
}

/// Duration
#[derive(ToSchema, Serialize)]
pub struct Duration {
    /// Number of milliseconds
    ms: BigInt,
}

impl From<std::time::Duration> for Duration {
    fn from(value: std::time::Duration) -> Self {
        Self {
            ms: BigInt(value.as_millis()),
        }
    }
}

#[derive(DeserializeFromStr)]
pub enum BlockHeightOrHash {
    Height(NonZero<u64>),
    Hash(iroha::Hash),
}

impl FromStr for BlockHeightOrHash {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(value) = s.parse::<NonZero<u64>>() {
            return Ok(Self::Height(value));
        }
        if let Ok(value) = s.parse::<iroha::Hash>() {
            return Ok(Self::Hash(value));
        }
        Err("value should be either a non-zero positive integer or a hash")
    }
}

/// Peer status
#[derive(Serialize, ToSchema)]
pub struct Status {
    peers: u32,
    blocks: u32,
    txs_accepted: u32,
    txs_rejected: u32,
    view_changes: u32,
    queue_size: u32,
    uptime: Duration,
}

impl From<iroha_telemetry::metrics::Status> for Status {
    fn from(value: iroha_telemetry::metrics::Status) -> Self {
        Self {
            peers: value.peers as u32,
            blocks: value.blocks as u32,
            txs_accepted: value.txs_accepted as u32,
            txs_rejected: value.txs_rejected as u32,
            view_changes: value.view_changes,
            queue_size: value.queue_size as u32,
            uptime: Duration::from(value.uptime.0),
        }
    }
}

#[cfg(test)]
mod test {
    use iroha_crypto::KeyPair;
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
        assert_eq!(value, nonzero!(412u64));

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
        let value = iroha_crypto::Signature::new(
            KeyPair::from_seed(vec![1, 2, 3], iroha_crypto::Algorithm::Secp256k1).private_key(),
            &[5, 4, 3, 2, 1],
        );

        expect_test::expect![[r#""A19E05FFE0939F8B7952819E64B9637A500D767519274F21763E8B4283A77E01223D35FE6DFEC6D513D17E1D902791B6D637AD447E9548767948F5A36B652906""#]]
            .assert_eq(&serde_json::to_string(&Signature(value)).unwrap());
    }
}
