use std::{
    borrow::Cow,
    num::{NonZero, NonZeroU32},
};

use iroha_data_model::{HasMetadata as _, Identifiable as _};
use nonzero_ext::nonzero;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

mod iroha {
    pub use iroha_data_model::{
        account::AccountId, domain::DomainId, ipfs::IpfsPath, metadata::Metadata,
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
            owned_by: AccountId(value.owned_by()),
        }
    }
}

/// Domain ID
#[derive(ToSchema, Serialize, Deserialize)]
#[schema(example = "genesis", value_type = String)]
pub struct DomainId<'a>(pub Cow<'a, iroha::DomainId>);

/// Account ID. Represented as `signatory@domain`.
#[derive(ToSchema, Serialize)]
#[schema(
    example = "ed01204164BF554923ECE1FD412D241036D863A6AE430476C898248B8237D77534CFC4@genesis",
    value_type = String
)]
pub struct AccountId<'a>(&'a iroha::AccountId);

/// Key-value map with arbitrary data
#[derive(Serialize, ToSchema)]
#[schema(
    value_type = Object
)]
pub struct Metadata<'a>(&'a iroha::Metadata);

/// IPFS path
#[derive(Serialize, ToSchema)]
#[schema(value_type = String)]
pub struct IpfsPath<'a>(&'a iroha::IpfsPath);

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
    page: NonZeroU32,
    /// Items per page, starts from 1
    per_page: NonZeroU32,
    /// Total amount of items. Infer pages count using this and `page_size`.
    total_items: u32,
}

impl Pagination {
    // FIXME: pass `total_items` too
    pub fn from_query(params: PaginationQueryParams) -> Self {
        Self {
            page: params.page,
            per_page: params.per_page,
            // FIXME
            total_items: 999,
        }
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
    pub fn new(items: impl Into<Vec<T>>, pagination: impl Into<Pagination>) -> Self {
        Self {
            pagination: pagination.into(),
            items: items.into(),
        }
    }
}

// FIXME: params details is not rendered fully, only docs
/// Pagination query parameters
#[derive(Deserialize, IntoParams, Clone, Copy)]
pub struct PaginationQueryParams {
    /// Page number
    #[param(example = 3, minimum = 1)]
    #[serde(default = "default_page")]
    pub page: NonZero<u32>,
    /// Items per page
    #[param(example = 15, minimum = 1)]
    #[serde(default = "default_per_page")]
    pub per_page: NonZeroU32,
}

fn default_page() -> NonZero<u32> {
    const VALUE: NonZero<u32> = nonzero!(1u32);
    VALUE
}

fn default_per_page() -> NonZero<u32> {
    const VALUE: NonZero<u32> = nonzero!(10u32);
    VALUE
}

impl From<PaginationQueryParams> for iroha_data_model::query::parameters::Pagination {
    fn from(value: PaginationQueryParams) -> Self {
        let offset = NonZero::new((value.per_page.get() * (value.page.get() - 1)) as u64);

        // FIXME: impossible to construct in otherway without enabling `transparent_api` feature of the data model
        serde_json::from_value(json!({
            "limit": value.per_page,
            "start": offset
        }))
        .expect("should deserialize fine")
    }
}
