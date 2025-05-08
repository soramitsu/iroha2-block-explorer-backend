use crate::schema::ToriiUrl;
use iroha::client::Client;
use iroha::crypto::KeyPair;
use iroha::data_model::account::AccountId;
use iroha::data_model::ChainId;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ClientWrap(Arc<Client>);

impl Deref for ClientWrap {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ClientWrap {
    pub fn new(authority: AccountId, key_pair: KeyPair, torii_url: ToriiUrl) -> Self {
        Client::new(iroha::config::Config {
            account: authority,
            key_pair,
            torii_api_url: torii_url.0,
            basic_auth: None,

            // we only use queries, and these fields are unused
            chain: ChainId::from("whichever"),
            transaction_add_nonce: false,
            transaction_status_timeout: Duration::from_secs(0),
            transaction_ttl: Duration::from_secs(0),
        })
        .into()
    }

    pub fn torii_url(&self) -> ToriiUrl {
        ToriiUrl(self.torii_url.clone())
    }
}

impl From<Client> for ClientWrap {
    fn from(value: Client) -> Self {
        Self(Arc::new(value))
    }
}
