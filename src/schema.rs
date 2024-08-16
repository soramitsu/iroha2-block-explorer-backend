use std::{borrow::Cow, num::NonZero, str::FromStr};

use chrono::Utc;
use iroha_data_model::{HasMetadata as _, Identifiable as _};
use nonzero_ext::nonzero;
use serde::{Deserialize, Serialize};
use serde_with::DeserializeFromStr;
use utoipa::{IntoParams, ToSchema};

use crate::util::{DirectPagination, ReversePagination};

mod iroha {
    pub use iroha_crypto::Hash;
    pub use iroha_data_model::prelude::*;
    pub use iroha_data_model::{
        block::{BlockHeader, SignedBlock},
        ipfs::IpfsPath,
    };
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
pub struct Domain<'a> {
    /// Domain ID
    id: DomainId<'a>,
    /// Domain logo path
    logo: Option<IpfsPath<'a>>,
    /// Domain metadata
    metadata: Metadata<'a>,
    /// Domain's owner
    owned_by: AccountId<'a>,
}

impl<'a> From<&'a iroha_data_model::domain::Domain> for Domain<'a> {
    fn from(value: &'a iroha_data_model::domain::Domain) -> Self {
        Self {
            id: DomainId(Cow::Borrowed(value.id())),
            logo: value.logo().as_ref().map(IpfsPath),
            metadata: Metadata(value.metadata()),
            owned_by: AccountId(Cow::Borrowed(value.owned_by())),
        }
    }
}

/// Domain ID
#[derive(ToSchema, Serialize, Deserialize)]
#[schema(example = "genesis", value_type = String)]
pub struct DomainId<'a>(pub Cow<'a, iroha::DomainId>);

/// Account
#[derive(Serialize, ToSchema)]
pub struct Account<'a> {
    id: AccountId<'a>,
    metadata: Metadata<'a>,
}

impl<'a> From<&'a iroha::Account> for Account<'a> {
    fn from(value: &'a iroha::Account) -> Self {
        Self {
            id: AccountId(Cow::Borrowed(value.id())),
            metadata: Metadata(value.metadata()),
        }
    }
}

/// Account ID. Represented as `signatory@domain`.
#[derive(ToSchema, Serialize, Deserialize)]
#[schema(
    example = "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis",
    value_type = String
)]
pub struct AccountId<'a>(pub Cow<'a, iroha::AccountId>);

#[derive(ToSchema, Serialize)]
pub struct AssetDefinition<'a> {
    id: AssetDefinitionId<'a>,
    r#type: AssetType,
    mintable: Mintable,
    logo: Option<IpfsPath<'a>>,
    metadata: Metadata<'a>,
    owned_by: AccountId<'a>,
}

impl<'a> From<&'a iroha::AssetDefinition> for AssetDefinition<'a> {
    fn from(value: &'a iroha::AssetDefinition) -> Self {
        Self {
            id: AssetDefinitionId::from(value.id()),
            r#type: match value.type_() {
                iroha::AssetType::Numeric(_spec) => AssetType::Numeric {
                    scale: None, // FIXME: private field, no access - spec.scale(),
                },
                iroha::AssetType::Store => AssetType::Store,
            },
            mintable: match value.mintable() {
                iroha::Mintable::Infinitely => Mintable::Infinitely,
                iroha::Mintable::Once => Mintable::Once,
                iroha::Mintable::Not => Mintable::Not,
            },
            logo: value.logo().as_ref().map(IpfsPath),
            metadata: Metadata(value.metadata()),
            owned_by: AccountId(Cow::Borrowed(value.owned_by())),
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize)]
pub struct AssetDefinitionId<'a> {
    domain: DomainId<'a>,
    name: Cow<'a, str>,
}

impl<'a> AssetDefinitionId<'a> {
    pub fn into_owned(self) -> iroha::AssetDefinitionId {
        iroha::AssetDefinitionId::new(
            self.domain.0.into_owned(),
            self.name
                .into_owned()
                .parse()
                .expect("it was constructed from name, reverse conversion should not fail"),
        )
    }
}

impl<'a> From<&'a iroha::AssetDefinitionId> for AssetDefinitionId<'a> {
    fn from(value: &'a iroha::AssetDefinitionId) -> Self {
        Self {
            domain: DomainId(Cow::Borrowed(value.domain())),
            name: Cow::Borrowed(value.name().as_ref()),
        }
    }
}

#[derive(ToSchema, Serialize)]
#[serde(tag = "kind")]
pub enum AssetType {
    Numeric { scale: Option<u32> },
    Store,
}

#[derive(ToSchema, Serialize)]
pub enum Mintable {
    Infinitely,
    Once,
    Not,
}

#[derive(ToSchema, Serialize)]
pub struct Asset<'a> {
    id: AssetId<'a>,
    value: AssetValue<'a>,
}

impl<'a> From<&'a iroha::Asset> for Asset<'a> {
    fn from(value: &'a iroha::Asset) -> Self {
        Self {
            id: AssetId::from(value.id()),
            value: match value.value() {
                iroha::AssetValue::Numeric(numeric) => AssetValue::Numeric {
                    value: Decimal::from(numeric),
                },
                iroha::AssetValue::Store(map) => AssetValue::Store {
                    metadata: Metadata(map),
                },
            },
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize)]
pub struct AssetId<'a> {
    definition: AssetDefinitionId<'a>,
    account: AccountId<'a>,
}

impl<'a> From<&'a iroha::AssetId> for AssetId<'a> {
    fn from(value: &'a iroha::AssetId) -> Self {
        AssetId {
            account: AccountId(Cow::Borrowed(value.account())),
            definition: AssetDefinitionId::from(value.definition()),
        }
    }
}

impl<'a> AssetId<'a> {
    pub fn into_owned(self) -> iroha::AssetId {
        iroha::AssetId::new(self.definition.into_owned(), self.account.0.into_owned())
    }
}

#[derive(ToSchema, Serialize)]
#[serde(tag = "kind")]
pub enum AssetValue<'a> {
    Numeric { value: Decimal },
    Store { metadata: Metadata<'a> },
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
pub struct Metadata<'a>(&'a iroha::Metadata);

/// IPFS path
#[derive(Serialize, ToSchema)]
#[schema(value_type = String)]
pub struct IpfsPath<'a>(&'a iroha::IpfsPath);

/// Big integer numeric value.
///
/// Serialized as a **number** when safely fits into `f64` max safe integer
/// (less than `pow(2, 53) - 1`, i.e. `9007199254740991`), and as a **string** otherwise.
///
/// On JavaScript side is recommended to parse with `BigInt`.
#[derive(ToSchema)]
// TODO set `value_type` to union of string and number
#[schema(example = 42)]
pub struct BigInt(u128);

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
#[derive(Serialize, ToSchema)]
#[schema(
    example = json!({
        "page": 1,
        "page_size": 10,
        "total_items": 89
    })
)]
pub struct Pagination {
    /// Page number, starts from 1
    page: BigInt,
    /// Items per page, starts from 1
    per_page: BigInt,
    /// Total number of pages. Not always available.
    total_pages: Option<BigInt>,
    /// Total number of items. Not always available.
    total_items: Option<BigInt>,
}

impl Pagination {
    pub fn new(
        page: NonZero<u64>,
        per_page: NonZero<u64>,
        total_items: Option<u64>,
        total_pages: Option<u64>,
    ) -> Self {
        Self {
            page: BigInt::from(page.get()),
            per_page: BigInt::from(per_page.get()),
            total_items: total_items.map(BigInt::from),
            total_pages: total_pages.map(BigInt::from),
        }
    }

    pub fn for_empty_data(per_page: NonZero<u64>) -> Self {
        Self {
            // "there is one page, it's just empty"
            page: BigInt(1),
            per_page: BigInt::from(per_page.get()),
            // "but there are zero pages of data"
            total_pages: Some(BigInt(0)),
            total_items: Some(BigInt(0)),
        }
    }
}

impl From<ReversePagination> for Pagination {
    fn from(value: ReversePagination) -> Self {
        Self::new(
            value.page(),
            value.per_page(),
            Some(value.len().get()),
            Some(
                value
                    .total_pages()
                    .get()
                    .try_into()
                    .expect("should fit into u32"),
            ),
        )
    }
}

impl From<DirectPagination> for Pagination {
    fn from(value: DirectPagination) -> Self {
        Self::new(value.page(), value.per_page(), None, None)
    }
}

/// Generic paginated data container
#[derive(Serialize, ToSchema)]
#[aliases(DomainsPage = Page<Domain<'a>>)]
pub struct Page<T> {
    /// Pagination info
    pagination: Pagination,
    /// Page items
    items: Vec<T>,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, pagination: Pagination) -> Self {
        Self { pagination, items }
    }
}

impl Page<()> {
    pub fn empty(per_page: NonZero<u64>) -> Self {
        Self::new(vec![], Pagination::for_empty_data(per_page))
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

impl TimeStamp {
    fn from_duration_as_epoch(value: std::time::Duration) -> Option<Self> {
        chrono::DateTime::from_timestamp_millis(value.as_millis().try_into().ok()?).map(Self)
    }
}

/// Like [`Transaction`], but with a block hash included
#[derive(Serialize, ToSchema)]
pub struct TransactionWithHash<'a> {
    #[serde(flatten)]
    base: Transaction<'a>,
    block_hash: Hash,
}

impl<'a> From<&'a iroha::TransactionQueryOutput> for TransactionWithHash<'a> {
    fn from(value: &'a iroha::TransactionQueryOutput) -> Self {
        Self {
            base: Transaction::from(value.as_ref()),
            block_hash: Hash::from(*value.block_hash()),
        }
    }
}

/// Transaction
#[derive(Serialize, ToSchema)]
pub struct Transaction<'a> {
    /// Transaction hash
    hash: Hash,
    /// Transaction payload
    payload: TransactionPayload<'a>,
    /// Transaction signature
    signature: Signature<'a>,
    /// If exists, transaction has been rejected
    error: Option<TransactionRejectionReason<'a>>,
}

impl<'a> From<&'a iroha::CommittedTransaction> for Transaction<'a> {
    fn from(value: &'a iroha::CommittedTransaction) -> Self {
        let signed: &iroha::SignedTransaction = value.as_ref();

        Self {
            hash: signed.hash().into(),
            payload: TransactionPayload {
                chain: signed.chain(),
                authority: AccountId(Cow::Borrowed(signed.authority())),
                instructions: match signed.instructions() {
                    iroha::Executable::Instructions(isis) => {
                        Executable::Instructions(isis.iter().map(Instruction).collect())
                    }
                    iroha::Executable::Wasm(_) => Executable::Wasm,
                },
                nonce: signed.nonce(),
                metadata: Metadata(signed.metadata()),
                created_at: TimeStamp::from_duration_as_epoch(signed.creation_time())
                    .expect("creation time should fit into date time"),
                time_to_live: signed.time_to_live().map(Duration::from),
            },
            error: value.error().as_ref().map(TransactionRejectionReason),
            signature: signed.signature().payload().into(),
        }
    }
}

/// Transaction rejection reason
#[derive(Serialize, ToSchema)]
#[schema(value_type = Object)]
pub struct TransactionRejectionReason<'a>(&'a iroha::TransactionRejectionReason);

/// Payload of transaction
#[derive(Serialize, ToSchema)]
pub struct TransactionPayload<'a> {
    /// Unique id of the blockchain. used for simple replay attack protection.
    chain: &'a iroha::ChainId,
    /// The creator of the transactions
    authority: AccountId<'a>,
    /// Instructions of the transaction
    instructions: Executable<'a>,
    /// Random value to make different hashes for transactions
    /// which occur repeatedly and simultaneously
    nonce: Option<NonZero<u32>>,
    /// Arbitrary additional information
    metadata: Metadata<'a>,
    /// Creation timestamp
    created_at: TimeStamp,
    /// After which time span since creation transaction is dismissed if not committed yet
    time_to_live: Option<Duration>,
}

/// Operations executable on-chain
#[derive(Serialize, ToSchema)]
#[serde(tag = "kind", content = "value")]
pub enum Executable<'a> {
    /// Array of instructions
    Instructions(Vec<Instruction<'a>>),
    /// WebAssembly smart contract
    Wasm,
}

/// Iroha Special Instruction (ISI)
#[derive(Serialize, ToSchema)]
#[schema(value_type = Object)]
pub struct Instruction<'a>(&'a iroha::InstructionBox);

/// Block
#[derive(Serialize, ToSchema)]
pub struct Block<'a> {
    /// Block hash
    hash: Hash,
    /// Block header
    header: BlockHeader,
    /// Signatures of peers which approved this block
    signatures: Vec<BlockSignature<'a>>,
    /// Transactions which successfully passed validation & consensus step.
    transactions: Vec<Transaction<'a>>,
    // TODO event recommendations?
}

impl<'a> From<&'a iroha::SignedBlock> for Block<'a> {
    fn from(value: &'a iroha::SignedBlock) -> Self {
        Self {
            hash: Hash(value.hash().into()),
            header: BlockHeader::from(value.header()),
            signatures: value
                .signatures()
                .map(|x| BlockSignature {
                    topology_index: BigInt(x.index() as u128),
                    payload: x.payload().into(),
                })
                .collect(),
            transactions: value.transactions().map(Transaction::from).collect(),
        }
    }
}

/// Signature of block
#[derive(Serialize, ToSchema)]
pub struct BlockSignature<'a> {
    /// Index of the peer in the topology
    topology_index: BigInt,
    /// The signature itself
    payload: Signature<'a>,
}

/// Header of block
#[derive(Serialize, ToSchema)]
#[schema(
    example = json!({
        "height": 4,
        "prev_block_hash": "9FC55BD948D0CDE0838F6D86FA069A258F033156EE9ACEF5A5018BC9589473F3",
        "transactions_hash": "6D8C110F75E7447D1495FE419C212ABA5DA31F940B85C8598D76A11C5D60AEFB",
        "created_at": "2024-08-15T00:40:02.324Z",
        "consensus_estimation": {
            "ms": 0
        }
    })
)]
pub struct BlockHeader {
    /// Number of blocks in the chain including this block
    height: NonZero<u64>,
    /// Hash of the previous block in the chain
    prev_block_hash: Option<Hash>,
    /// Hash of merkle tree root of transactions' hashes
    transactions_hash: Hash,
    /// Timestamp of creation
    created_at: TimeStamp,
    /// Estimation of consensus duration
    consensus_estimation: Duration,
}

impl From<&iroha::BlockHeader> for BlockHeader {
    fn from(value: &iroha::BlockHeader) -> Self {
        Self {
            height: value.height(),
            prev_block_hash: value.prev_block_hash().map(Hash::from),
            transactions_hash: value.transactions_hash().into(),
            created_at: TimeStamp::from_duration_as_epoch(value.creation_time())
                .expect("creation time should fit into datetime"),
            consensus_estimation: Duration::from(value.consensus_estimation()),
        }
    }
}

/// Hex-encoded hash
#[derive(Deserialize, Serialize, ToSchema)]
#[schema(value_type = String, example = "1B0A52DBDC11EAE39DD0524AD5146122351527CE00D161EA8263EA7ADE4164AF")]
pub struct Hash(pub iroha::Hash);

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
pub struct Signature<'a>(Cow<'a, iroha_crypto::Signature>);

impl<T> From<iroha_crypto::SignatureOf<T>> for Signature<'_> {
    fn from(value: iroha_crypto::SignatureOf<T>) -> Self {
        Self(Cow::Owned(value.into()))
    }
}

impl<'a, T> From<&'a iroha_crypto::SignatureOf<T>> for Signature<'a> {
    fn from(value: &'a iroha_crypto::SignatureOf<T>) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl<'a> From<&'a iroha_crypto::Signature> for Signature<'a> {
    fn from(value: &'a iroha_crypto::Signature) -> Self {
        Self(Cow::Borrowed(value))
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
        return Err("value should be either a non-zero positive integer or a hash");
    }
}

/// Shows a relationship between a type and its reflection
pub trait ToAppSchema<'a>
where
    Self: 'a,
{
    type Output: From<&'a Self>;

    /// Reflects the type in its app schema representation
    fn to_app_schema(&'a self) -> Self::Output {
        Self::Output::from(&self)
    }
}

macro_rules! impl_to_app_schema {
    ($from:ty => $into:tt) => {
        impl<'a> ToAppSchema<'a> for $from {
            type Output = $into<'a>;
        }
    };
}

impl_to_app_schema!(iroha::Domain => Domain);
impl_to_app_schema!(iroha::Account => Account);
impl_to_app_schema!(iroha::Asset => Asset);
impl_to_app_schema!(iroha::AssetDefinition => AssetDefinition);
impl_to_app_schema!(iroha::SignedBlock => Block);
impl_to_app_schema!(iroha::CommittedTransaction => Transaction);
impl_to_app_schema!(iroha::TransactionQueryOutput => TransactionWithHash);

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    #[test]
    fn serialize_bigint() {
        assert_eq!(json!(BigInt(0)), json!(0));
        assert_eq!(json!(BigInt(9007199254740991)), json!(9007199254740991u64));
        assert_eq!(
            json!(BigInt(9007199254740991 + 1)),
            json!("9007199254740992")
        );
        assert_eq!(
            json!(BigInt(10_000_000_000_000_000_000_000u128)),
            json!("10000000000000000000000")
        )
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

    // TODO
    #[test]
    fn serialize_asset_id_canonically() {
        let expected =
            "roses##ed0120B23E14F659B91736AAB980B6ADDCE4B1DB8A138AB0267E049C082A744471714E@wonderland";
        let id = iroha::AssetId::from_str(expected).expect("input is valid");
        let value = AssetId::from(&id);
        let serialized = serde_json::to_string(&value).expect("no possible errors expected");
        assert_eq!(serialized, format!("\"{expected}\""));
    }
}
