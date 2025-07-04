// FIXME: temp
#![allow(unused)]

use futures_util::Stream;
use iroha_data_model::prelude::*;
use iroha_explorer_schema::ToriiUrl;
use std::num::NonZero;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Client {
    // TODO
}

// impl Deref for ClientWrap {
//     type Target = Client;
//
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

impl Client {
    pub fn new(authority: AccountId, key_pair: KeyPair, torii_url: ToriiUrl) -> Self {
        todo!()
        // Client::new(iroha::config::Config {
        //     account: authority,
        //     key_pair,
        //     torii_api_url: torii_url.0,
        //     basic_auth: None,
        //
        //     // we only use queries, and these fields are unused
        //     chain: ChainId::from("whichever"),
        //     transaction_add_nonce: false,
        //     transaction_status_timeout: Duration::from_secs(0),
        //     transaction_ttl: Duration::from_secs(0),
        // })
        // .into()
    }

    pub fn torii_url(&self) -> ToriiUrl {
        // ToriiUrl(self.torii_url.clone())
        todo!()
    }

    pub async fn lazy_block_stream(
        &self,
        from_block: NonZero<usize>,
    ) -> impl Stream<Item = Arc<SignedBlock>> {
        todo!();
        tokio_stream::iter(None)
    }

    pub async fn get_block_hash(
        &self,
        height: NonZero<usize>,
    ) -> eyre::Result<Option<HashOf<BlockHeader>>> {
        todo!()
    }

    pub fn blocks_info_from_end(
        &self,
        max_height: NonZero<usize>,
    ) -> impl Stream<Item = BlockInfoBatch> {
        todo!();
        tokio_stream::iter(None)
    }
}

pub struct BlockHashAndHeight {
    pub hash: HashOf<BlockHeader>,
    pub height: NonZero<usize>,
}

pub struct BlockInfoBatch(Vec<BlockHashAndHeight>);

impl BlockInfoBatch {
    pub fn iter_from_last(&self) -> impl Iterator<Item = &BlockHashAndHeight> {
        self.0.iter()
    }
}

// impl From<Client> for Client {
//     fn from(value: Client) -> Self {
//         Self(Arc::new(value))
//     }
// }
