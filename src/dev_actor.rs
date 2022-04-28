use super::logger;
use actix::prelude::{Actor, Addr, AsyncContext, Context, Handler, Message};
use color_eyre::{eyre::eyre, Result};
use core::time::Duration;
use iroha_client::client::Client as IrohaClient;
use iroha_data_model::{
    prelude::{
        Account, AccountId, AssetDefinition, AssetDefinitionId, AssetValue, AssetValueType, Domain,
        DomainId, FindAssetsByAccountId, MintBox, RegisterBox, Value,
    },
    IdBox,
};
use rand::{
    distributions::{Distribution, Standard},
    rngs::ThreadRng,
    seq::IteratorRandom,
    Rng,
};
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

const DEV_ACTOR_WORK_INTERVAL_MS: u64 = 1500;

pub struct DevActor {
    client: Arc<Mutex<IrohaClient>>,
    account_id: AccountId,
    rng: ThreadRng,
}

impl DevActor {
    pub fn start(client: Arc<Mutex<IrohaClient>>, account_id: AccountId) -> Addr<Self> {
        Actor::start(Self {
            client,
            account_id,
            rng: rand::thread_rng(),
        })
    }

    fn do_random_stuff(&mut self) -> Result<()> {
        let some_action: RandomAction = self.rng.gen();
        logger::info!("Doing: {:?}", some_action);
        let mut client = self
            .client
            .lock()
            .map_err(|err| eyre!("Failed to lock client mutex: {:?}", err))?;

        use faker_rand::fr_fr::{internet::Username, names::FirstName};

        match some_action {
            RandomAction::RegisterDomain => {
                let domain_name: FirstName = self.rng.gen();
                let new_domain_id: DomainId = domain_name.to_string().to_lowercase().parse()?;
                let create_domain = RegisterBox::new(Domain::new(new_domain_id));

                client.submit(create_domain)?;

                // do
            }
            RandomAction::MintAsset => {
                // The goal is to find an existing mintable asset and.. mint it with some value

                let asset = client
                    .request(FindAssetsByAccountId::new(self.account_id.clone()))?
                    .into_iter()
                    .find(|asset| match asset.value() {
                        AssetValue::Quantity(_) => true,
                        _ => false,
                    });

                if let Some(asset) = asset {
                    let value = Value::U32(self.rng.gen());
                    let mint = MintBox::new(value, IdBox::AssetId(asset.id().clone()));

                    logger::info!("Minting: {:?}", mint);

                    client.submit(mint)?;
                }
            }
            RandomAction::RegisterAsset => {
                let domain_id = self.account_id.domain_id.clone();
                let asset_name: Username = self.rng.gen();
                let definition_id =
                    AssetDefinitionId::from_str(format!("{}#{}", asset_name, domain_id).as_ref())?;
                let asset_value_type = RandomAssetValueType::new(&mut self.rng)?.0;

                let new_asset_definition = match asset_value_type {
                    AssetValueType::Quantity => AssetDefinition::quantity(definition_id),
                    AssetValueType::BigQuantity => AssetDefinition::big_quantity(definition_id),
                    AssetValueType::Fixed => AssetDefinition::fixed(definition_id),
                    AssetValueType::Store => AssetDefinition::store(definition_id),
                };
                let create_asset = RegisterBox::new(new_asset_definition.build());

                client.submit(create_asset)?;
            }
            RandomAction::RegisterAccount => {
                let domain_id = self.account_id.domain_id.clone();
                let account_name: FirstName = self.rng.gen();

                let account_id: AccountId = format!("{}@{}", account_name, domain_id).parse()?;
                let create_account = RegisterBox::new(Account::new(account_id, []));

                client.submit(create_account)?;
            }
        }

        Ok(())
    }
}

impl Actor for DevActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        logger::info!("DevActor is started");
        ctx.notify(DoSomething);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct DoSomething;

impl Handler<DoSomething> for DevActor {
    type Result = ();

    fn handle(&mut self, _msg: DoSomething, ctx: &mut Self::Context) -> Self::Result {
        match self.do_random_stuff() {
            Ok(()) => (),
            Err(err) => logger::error!("Failed to do random stuff: {}", err),
        };

        ctx.notify_later(
            DoSomething,
            Duration::from_millis(DEV_ACTOR_WORK_INTERVAL_MS),
        );
    }
}

#[derive(Clone, Copy, Debug)]
enum RandomAction {
    RegisterDomain,
    MintAsset,
    RegisterAsset,
    RegisterAccount,
}

impl Distribution<RandomAction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RandomAction {
        let num: f64 = rng.gen();

        if num < 0.1 {
            RandomAction::RegisterDomain
        } else if num < 0.3 {
            RandomAction::RegisterAsset
        } else if num < 0.5 {
            RandomAction::RegisterAccount
        } else {
            RandomAction::MintAsset
        }
    }
}

struct RandomAssetValueType(AssetValueType);

impl RandomAssetValueType {
    fn new<R: Rng + ?Sized>(rng: &mut R) -> Result<Self> {
        let value = IteratorRandom::choose(ASSET_VALUE_TYPES.iter(), rng)
            .ok_or_else(|| eyre!("Failed to generate random asset value type"))?;

        Ok(Self(*value))
    }
}

const ASSET_VALUE_TYPES: [AssetValueType; 4] = [
    AssetValueType::Quantity,
    AssetValueType::BigQuantity,
    AssetValueType::Fixed,
    AssetValueType::Store,
];
