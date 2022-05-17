use super::logger;
use crate::iroha_client_wrap::IrohaClientWrap;
use actix::{
    prelude::{Actor, Addr, AsyncContext, Context, Handler, Message},
    ActorFutureExt, ResponseActFuture, WrapFuture,
};
use color_eyre::{eyre::eyre, Result};
use core::time::Duration;
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
use std::{str::FromStr, sync::Arc};

const DEV_ACTOR_WORK_INTERVAL_MS: u64 = 1500;

pub struct DevActor {
    work: Option<Box<RandomWorkState>>,
}

impl DevActor {
    pub fn start(client: Arc<iroha_client::client::Client>, account_id: AccountId) -> Addr<Self> {
        let work = RandomWorkState {
            client: IrohaClientWrap::new(client),
            account_id,
            rng: rand::thread_rng(),
        };

        Actor::start(Self {
            work: Some(Box::new(work)),
        })
    }
}

impl Actor for DevActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        logger::info!("DevActor is started");
        ctx.notify(DoRandomStuff);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct DoRandomStuff;

struct RandomWorkState {
    client: IrohaClientWrap,
    account_id: AccountId,
    rng: ThreadRng,
}

impl RandomWorkState {
    async fn do_it(&mut self) -> Result<()> {
        use faker_rand::fr_fr::{internet::Username, names::FirstName};

        let mut rng = &mut self.rng;

        let some_action: RandomAction = rng.gen();
        logger::info!("Doing: {:?}", some_action);
        let client = &self.client;

        match some_action {
            RandomAction::RegisterDomain => {
                let domain_name: FirstName = rng.gen();
                let new_domain_id: DomainId = domain_name.to_string().to_lowercase().parse()?;
                let create_domain = RegisterBox::new(Domain::new(new_domain_id));

                client.submit(create_domain).await?;
            }
            RandomAction::MintAsset => {
                // The goal is to find an existing mintable asset and.. mint it with some value

                let asset = client
                    .request(FindAssetsByAccountId::new(self.account_id.clone()))
                    .await?
                    .into_iter()
                    .find(|asset| matches!(asset.value(), AssetValue::Quantity(_)));

                if let Some(asset) = asset {
                    let value = Value::U32(rng.gen());
                    let mint = MintBox::new(value, IdBox::AssetId(asset.id().clone()));

                    logger::info!("Minting: {:?}", mint);

                    client.submit(mint).await?;
                }
            }
            RandomAction::RegisterAsset => {
                let domain_id = self.account_id.domain_id.clone();
                let asset_name: Username = rng.gen();
                let definition_id =
                    AssetDefinitionId::from_str(format!("{}#{}", asset_name, domain_id).as_ref())?;
                let asset_value_type = RandomAssetValueType::new(&mut rng)?.0;

                let new_asset_definition = match asset_value_type {
                    AssetValueType::Quantity => AssetDefinition::quantity(definition_id),
                    AssetValueType::BigQuantity => AssetDefinition::big_quantity(definition_id),
                    AssetValueType::Fixed => AssetDefinition::fixed(definition_id),
                    AssetValueType::Store => AssetDefinition::store(definition_id),
                };
                let create_asset = RegisterBox::new(new_asset_definition.build());

                logger::info!("Create asset: {:?}", create_asset);

                client.submit(create_asset).await?;
            }
            RandomAction::RegisterAccount => {
                let domain_id = self.account_id.domain_id.clone();
                let account_name: FirstName = rng.gen();

                let account_id: AccountId = format!("{}@{}", account_name, domain_id).parse()?;
                let create_account = RegisterBox::new(Account::new(account_id, []));

                client.submit(create_account).await?;
            }
        }

        Ok(())
    }
}

impl Handler<DoRandomStuff> for DevActor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: DoRandomStuff, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(mut work) = self.work.take() {
            let fut = async {
                if let Err(err) = (*work).do_it().await {
                    logger::error!("Failed to do random stuff: {}", err);
                }

                work
            }
            .into_actor(self)
            .map(|work, actor, ctx| {
                actor.work = Some(work);

                ctx.notify_later(
                    DoRandomStuff,
                    Duration::from_millis(DEV_ACTOR_WORK_INTERVAL_MS),
                );
            });

            Box::pin(fut)
        } else {
            logger::warn!(
                "While hanlding DoRandomStuff message, it turned out there work state is empty",
            );
            Box::pin(async {}.into_actor(self))
        }
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
