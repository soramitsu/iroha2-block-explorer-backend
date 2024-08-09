use std::borrow::Cow;

use iroha_data_model::{HasMetadata as _, Identifiable as _};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
