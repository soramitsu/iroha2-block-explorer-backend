use iroha::client::Client;
use iroha::crypto::KeyPair;
use iroha::data_model::account::AccountId;
use iroha::data_model::ChainId;
use std::ops::Deref;
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone)]
pub struct ClientWrap(Client);

impl Deref for ClientWrap {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ClientWrap {
    pub fn new(authority: AccountId, key_pair: KeyPair, torii_url: Url) -> Self {
        let client = Client::new(iroha::config::Config {
            account: authority,
            key_pair,
            torii_api_url: torii_url,
            basic_auth: None,

            // we only use queries, and these fields are unused
            chain: ChainId::from("whichever"),
            transaction_add_nonce: false,
            transaction_status_timeout: Duration::from_secs(0),
            transaction_ttl: Duration::from_secs(0),
        });
        Self(client)
    }
}

impl From<Client> for ClientWrap {
    fn from(value: Client) -> Self {
        Self(value)
    }
}
