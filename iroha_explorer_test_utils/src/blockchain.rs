use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};

use iroha_core::{block::BlockBuilder, tx::AcceptedTransaction};
use iroha_data_model::prelude::*;
use iroha_primitives::time::{MockTimeHandle, TimeSource};
pub use iroha_test_samples::{
    ALICE_ID as ALICE, ALICE_KEYPAIR as ALICE_KEY, CARPENTER_ID as CARPENTER,
    CARPENTER_KEYPAIR as CARPENTER_KEY, SAMPLE_GENESIS_ACCOUNT_ID as GENESIS,
    SAMPLE_GENESIS_ACCOUNT_KEYPAIR as GENESIS_KEY,
};
use serde_json::json;

macro_rules! parse {
    ($raw:expr) => {
        $raw.parse().unwrap()
    };
}

pub struct Factory {
    time_handle: MockTimeHandle,
    time_source: TimeSource,
    blocks: Vec<Arc<SignedBlock>>,
}

impl Factory {
    pub fn new() -> Self {
        let (time_handle, time_source) = TimeSource::new_mock(Duration::ZERO);
        let blocks: Vec<Arc<SignedBlock>> = vec![];
        Self {
            time_handle,
            time_source,
            blocks,
        }
    }

    pub fn transaction<I: Instruction>(
        &self,
        authority: AccountId,
        key: &PrivateKey,
        instructions: impl IntoIterator<Item = I>,
    ) -> SignedTransaction {
        TransactionBuilder::new_with_time_source(
            ChainId::from("test"),
            authority,
            &self.time_source,
        )
        .with_instructions(instructions)
        .sign(key)
    }

    pub fn block(&mut self, transactions: Vec<SignedTransaction>) -> &mut Self {
        let block: SignedBlock = BlockBuilder::new_with_time_source(
            transactions
                .into_iter()
                .map(AcceptedTransaction::new_unchecked)
                .collect(),
            self.time_source.to_owned(),
        )
        .chain(0, self.blocks.last().as_ref().map(|x| x.as_ref()))
        .sign(GENESIS_KEY.private_key())
        .unpack(|_| {})
        .into();
        let block = Arc::new(block);
        self.blocks.push(block);
        self
    }
}

pub trait MetadataExt {
    fn put(self, key: &str, value: impl Into<Json>) -> Self;
}

impl MetadataExt for Metadata {
    fn put(mut self, key: &str, value: impl Into<Json>) -> Self {
        self.insert(key.parse().unwrap(), value);
        self
    }
}

static SAMPLE: LazyLock<Vec<Arc<SignedBlock>>> = LazyLock::new(|| {
    let mut factory = Factory::new();

    let garden = CARPENTER.domain().to_owned();
    let garden_metadata = Metadata::default()
        .put("important_data", json!(["secret-code", 1, 2, 3]))
        .put(
            "very_important_data",
            json!({"very":{"important":{"data":{"is":{"deep":{"inside":42}}}}}}),
        );
    let snowflake: NftId = parse!(format!("snowflake${garden}"));

    factory.block(vec![factory.transaction::<InstructionBox>(
        GENESIS.to_owned(),
        GENESIS_KEY.private_key(),
        [
            Register::domain(Domain::new(parse!("wonderland"))).into(),
            Register::account(
                Account::new(ALICE.to_owned())
                    .with_metadata(Metadata::default().put("alias", json!("Alice"))),
            )
            .into(),
            Register::asset_definition(AssetDefinition::numeric(parse!("rose#wonderland"))).into(),
            Register::asset_definition(AssetDefinition::new(
                parse!("tulip#wonderland"),
                NumericSpec::integer(),
            ))
            .into(),
        ],
    )]);

    factory.time_handle.advance(Duration::from_secs(5));
    let tx1 = factory.transaction::<InstructionBox>(
        ALICE.to_owned(),
        ALICE_KEY.private_key(),
        [
            Register::domain(
                Domain::new(garden.to_owned())
                    .with_logo(parse!(
                        "/ipns/QmSrPmbaUKA3ZodhzPWZnpFgcPMFWF4QsxXbkWfEptTBJd"
                    ))
                    .with_metadata(garden_metadata),
            )
            .into(),
            Register::account(
                Account::new(CARPENTER.to_owned())
                    .with_metadata(Metadata::default().put("alias", json!("Carpenter"))),
            )
            .into(),
        ],
    );

    factory.time_handle.advance(Duration::from_secs(2));
    let tx2 = factory.transaction(
        CARPENTER.to_owned(),
        CARPENTER_KEY.private_key(),
        [Register::nft(Nft::new(
            snowflake.to_owned(),
            Metadata::default().put("what-am-i", json!("An NFT, unique as a snowflake")),
        ))],
    );

    factory.time_handle.advance(Duration::from_secs(7));
    let tx3 = factory.transaction::<InstructionBox>(
        CARPENTER.to_owned(),
        CARPENTER_KEY.private_key(),
        [
            SetKeyValue::account(ALICE.to_owned(), parse!("alias"), json!("Alice (mutated)"))
                .into(),
            SetKeyValue::account(GENESIS.to_owned(), parse!("alias"), json!("Genesis")).into(),
            SetKeyValue::nft(
                snowflake.to_owned(),
                parse!("another-rather-unique-metadata-set-later"),
                json!([5, 1, 2, 3, 4]),
            )
            .into(),
        ],
    );

    factory.time_handle.advance(Duration::from_millis(100));
    factory.block(vec![tx1, tx2, tx3]);

    // Time & Data triggers

    factory.time_handle.advance(Duration::from_secs(3));
    factory.block(vec![factory.transaction::<InstructionBox>(
        ALICE.to_owned(),
        ALICE_KEY.private_key(),
        [
            Register::asset_definition(AssetDefinition::numeric(parse!("pre-commit#wonderland")))
                .into(),
            Register::asset_definition(AssetDefinition::numeric(parse!(
                "time-schedule#wonderland"
            )))
            .into(),
            Register::trigger(Trigger::new(
                parse!("pre-commit"),
                Action::new(
                    Executable::Instructions(
                        vec![Mint::asset_numeric(
                            1u32,
                            parse!(format!("pre-commit#wonderland#{}", *ALICE)),
                        )
                        .into()]
                        .into(),
                    ),
                    Repeats::Exactly(5),
                    ALICE.to_owned(),
                    EventFilterBox::Time(TimeEventFilter(ExecutionTime::PreCommit)),
                ),
            ))
            .into(),
            Register::trigger(Trigger::new(
                parse!("time-schedule"),
                Action::new(
                    Executable::Instructions(
                        vec![Mint::asset_numeric(
                            1u32,
                            parse!(format!("time-schedule#wonderland#{}", *ALICE)),
                        )
                        .into()]
                        .into(),
                    ),
                    Repeats::Indefinitely,
                    ALICE.to_owned(),
                    EventFilterBox::Time(TimeEventFilter(ExecutionTime::Schedule(
                        TimeSchedule::starting_at(Duration::from_secs(35))
                            .with_period(Duration::from_secs(5)),
                    ))),
                ),
            ))
            .into(),
            // TODO: more triggers!
        ],
    )]);

    // Empty block!
    factory.time_handle.advance(Duration::from_secs(2));
    factory.block(vec![]);

    // Various instructions
    factory.time_handle.advance(Duration::from_secs(20));
    factory.block(vec![
        factory.transaction(
            CARPENTER.to_owned(),
            CARPENTER_KEY.private_key(),
            [
                Mint::asset_numeric(100u32, parse!(format!("rose#wonderland#{}", *ALICE))),
                Mint::asset_numeric(200u32, parse!(format!("rose#wonderland#{}", *CARPENTER))),
            ],
        ),
        factory.transaction(
            ALICE.to_owned(),
            ALICE_KEY.private_key(),
            [Burn::asset_numeric(
                25u32,
                parse!(format!("rose#wonderland#{}", *ALICE)),
            )],
        ),
        factory.transaction(
            ALICE.to_owned(),
            ALICE_KEY.private_key(),
            [Transfer::asset_numeric(
                parse!(format!("rose#wonderland#{}", *CARPENTER)),
                125u32,
                ALICE.to_owned(),
            )],
        ),
        // fails - no such key
        factory.transaction(
            ALICE.to_owned(),
            ALICE_KEY.private_key(),
            [RemoveKeyValue::domain(
                parse!("wonderland"),
                parse!("keys_from_all_secrets"),
            )],
        ),
        // fails - wrong signature
        factory.transaction::<InstructionBox>(ALICE.to_owned(), CARPENTER_KEY.private_key(), []),
        factory.transaction(
            CARPENTER.to_owned(),
            CARPENTER_KEY.private_key(),
            [Log::new(
                Level::ERROR,
                "A disrupting message of sorts".to_owned(),
            )],
        ),
        // TODO: upgrade to custom executor, test this
        // factory.transaction(
        //     ALICE.to_owned(),
        //     ALICE_KEY.private_key(),
        //     [CustomInstruction::new(
        //         json!({ "kind": "custom", "value": false }),
        //     )],
        // ),
        factory.transaction(
            ALICE.to_owned(),
            ALICE_KEY.private_key(),
            [ExecuteTrigger::new(parse!("ping")).with_args(&json!([
                "do this",
                "then this",
                "and that afterwards"
            ]))],
        ),
    ]);

    // TODO: grant/revoke

    factory.blocks
});

pub fn sample() -> &'static Vec<Arc<SignedBlock>> {
    &*SAMPLE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_creates_normally() {
        let _ = sample();
    }
}
