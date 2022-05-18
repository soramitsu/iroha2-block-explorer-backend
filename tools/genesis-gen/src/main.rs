use clap::Parser;
use color_eyre::Result;

mod gen {
    use crate::CLI;
    use color_eyre::{eyre::Context, Result};
    use serde::Serialize;

    /// Generator data model.
    ///
    /// Based on Iroha Data Model, but simpler.
    mod model {
        use super::super::CLI;
        use color_eyre::{
            eyre::{eyre, Context},
            Result,
        };
        use iroha_core::{
            genesis::{GenesisTransaction, RawGenesisBlock},
            tx::{AssetValue, MintBox},
        };
        use iroha_crypto::{KeyGenConfiguration, KeyPair};
        use iroha_data_model::{
            account::NewAccount,
            asset::Mintable,
            domain::NewDomain,
            prelude::{
                Account as OriginAccount, AccountId, Asset,
                AssetDefinition as OriginAssetDefinition, AssetDefinitionId, AssetId,
                AssetValueType, Domain as OriginDomain, DomainId, Instruction, Metadata, Name,
                RegisterBox, Value,
            },
        };
        use rand::{prelude::IteratorRandom, thread_rng, Rng};
        use serde::Serialize;
        use std::str::FromStr;

        pub struct Model {
            domains: Vec<Domain>,
            accounts: Vec<Account>,
            asset_definitions: Vec<AssetDefinition>,
            asset_actions: Vec<Instruction>,
        }

        impl Model {
            pub fn generate(config: &CLI) -> Result<Self> {
                let mut rng = thread_rng();

                let mut domains: Vec<Domain> = Vec::with_capacity(config.domains);
                let mut accounts: Vec<Account> =
                    Vec::with_capacity(config.domains * config.accounts_per_domain);
                let mut asset_definitions: Vec<AssetDefinition> =
                    Vec::with_capacity(config.domains * config.assets_per_domain);
                let mut asset_actions: Vec<Instruction> = Vec::with_capacity(config.asset_actions);

                for _ in 0..config.domains {
                    let domain = Domain::new(&mut rng).wrap_err("Failed to generate domain")?;

                    for _ in 0..config.accounts_per_domain {
                        let account = Account::new(&mut rng, &domain.id)
                            .wrap_err("Failed to generate account")?;
                        accounts.push(account);
                    }

                    for _ in 0..config.assets_per_domain {
                        let definition = AssetDefinition::new(&mut rng, &domain.id)
                            .wrap_err("Failed to generage asset definition")?;
                        asset_definitions.push(definition);
                    }

                    domains.push(domain);
                }

                for _ in 0..config.asset_actions {
                    asset_actions.push(
                        random_asset_action(&mut rng, &accounts, &asset_definitions)
                            .wrap_err("Failed to generate random action")?,
                    );
                }

                Ok(Self {
                    domains,
                    accounts,
                    asset_actions,
                    asset_definitions,
                })
            }

            pub fn build(self) -> Result<ModelBuilt> {
                let transactions: Vec<GenesisTransaction> = self
                    .domains
                    .into_iter()
                    .map(|x| Instruction::Register(RegisterBox::new(Into::<NewDomain>::into(x))))
                    .chain(self.accounts.clone().into_iter().map(|x| {
                        Instruction::Register(RegisterBox::new(Into::<NewAccount>::into(x)))
                    }))
                    .chain(self.asset_definitions.into_iter().map(|x| {
                        Instruction::Register(RegisterBox::new(
                            Into::<OriginAssetDefinition>::into(x),
                        ))
                    }))
                    .chain(self.asset_actions.into_iter())
                    .map(|instruction| GenesisTransaction {
                        isi: vec![instruction].into(),
                    })
                    .collect();

                Ok(ModelBuilt {
                    raw_genesis_block: RawGenesisBlock {
                        transactions: transactions.into(),
                    },
                    accounts: self.accounts,
                })
            }
        }

        #[derive(Serialize)]
        pub struct ModelBuilt {
            raw_genesis_block: RawGenesisBlock,
            accounts: Vec<Account>,
        }

        impl ModelBuilt {
            pub fn only_genesis(self) -> RawGenesisBlock {
                self.raw_genesis_block
            }
        }

        struct Domain {
            id: DomainId,
        }

        impl Domain {
            fn new<R: Rng>(rng: &mut R) -> Result<Self> {
                let name: faker_rand::fr_fr::company::CompanyName = rng.gen();
                let name = name.to_string().replace(' ', "_");
                let name = construct_iroha_name(name)?;

                Ok(Domain {
                    id: DomainId::new(name),
                })
            }
        }

        impl From<Domain> for NewDomain {
            fn from(domain: Domain) -> Self {
                OriginDomain::new(domain.id)
            }
        }

        #[derive(Serialize, Clone)]
        struct Account {
            id: AccountId,
            keys: Vec<KeyPair>,
        }

        impl From<Account> for NewAccount {
            fn from(Account { id, keys }: Account) -> Self {
                OriginAccount::new(id, keys.iter().map(|x| x.public_key().clone()))
            }
        }

        impl Account {
            fn new<R: Rng>(rng: &mut R, domain_id: &DomainId) -> Result<Self> {
                let kp: KeyPair = {
                    let seed_len: usize = rng.gen_range(5..30);
                    let mut seed: Vec<u8> = Vec::with_capacity(seed_len);
                    for _ in 0..seed_len {
                        seed.push(rng.gen());
                    }

                    let config = KeyGenConfiguration::default().use_seed(vec![]);
                    KeyPair::generate_with_configuration(config)?
                };
                let keys = vec![kp];

                let name: faker_rand::fr_fr::internet::Username = rng.gen();
                let name = construct_iroha_name(name.to_string())?;

                Ok(Self {
                    id: AccountId::new(name, domain_id.clone()),
                    keys,
                })
            }
        }

        struct AssetDefinition {
            id: AssetDefinitionId,
            value_type: AssetValueType,
            mintable: Mintable,
        }

        impl From<AssetDefinition> for OriginAssetDefinition {
            fn from(
                AssetDefinition {
                    id,
                    value_type,
                    mintable,
                }: AssetDefinition,
            ) -> Self {
                use AssetValueType::*;
                use Mintable::*;

                let mut definition = match value_type {
                    Quantity => OriginAssetDefinition::quantity(id),
                    BigQuantity => OriginAssetDefinition::big_quantity(id),
                    Fixed => OriginAssetDefinition::fixed(id),
                    Store => OriginAssetDefinition::store(id),
                };

                let definition = match mintable {
                    Not => {
                        definition.mintable(false);
                        definition
                    }
                    Once => definition.mintable_once(),
                    Infinitely => {
                        definition.mintable(true);
                        definition
                    }
                };

                definition.build()
            }
        }

        impl AssetDefinition {
            fn new<R: Rng>(rng: &mut R, domain_id: &DomainId) -> Result<Self> {
                let mintable: Mintable = match rng.gen_range(0..10u32) {
                    5..=9 => Mintable::Infinitely,
                    2..=4 => Mintable::Not,
                    0..=1 => Mintable::Once,
                    x => return Err(eyre!("Unexpected random num: {x}")),
                };

                let value_type: AssetValueType = match rng.gen_range(0..10u32) {
                    7..=9 => AssetValueType::Quantity,
                    4..=6 => AssetValueType::BigQuantity,
                    1..=3 => AssetValueType::Fixed,
                    0 => AssetValueType::Store,
                    x => return Err(eyre!("Unexpected random num: {x}")),
                };

                let name: faker_rand::en_us::names::LastName = rng.gen();
                let name = construct_iroha_name(name.to_string())?;

                Ok(Self {
                    id: AssetDefinitionId::new(name, domain_id.clone()),
                    mintable,
                    value_type,
                })
            }
        }

        fn construct_iroha_name(input: String) -> Result<Name> {
            Name::from_str(&input)
                .wrap_err_with(|| format!("Failed to construct Iroha Name from \"{input}\""))
        }

        fn random_asset_action<R: Rng>(
            rng: &mut R,
            accounts: &Vec<Account>,
            assets: &Vec<AssetDefinition>,
        ) -> Result<Instruction> {
            let some_asset_definition = assets
                .iter()
                .filter(|x| x.mintable != Mintable::Not || x.value_type == AssetValueType::Store)
                .choose(rng)
                .ok_or_else(|| eyre!("Failed to choose random asset definition"))?;

            let some_account = accounts
                .iter()
                .choose(rng)
                .ok_or_else(|| eyre!("Failed to choose random account"))?;

            let asset_id = AssetId::new(some_asset_definition.id.clone(), some_account.id.clone());

            if some_asset_definition.value_type == AssetValueType::Store {
                let instruction = Instruction::Register(RegisterBox::new(Asset::new(
                    asset_id,
                    AssetValue::Store(Metadata::new()),
                )));

                Ok(instruction)
            } else {
                // Then mintable asset
                let value = match some_asset_definition.value_type {
                    AssetValueType::Quantity => Value::U32(rng.gen()),
                    AssetValueType::BigQuantity => Value::U128(rng.gen()),
                    AssetValueType::Fixed => Value::Fixed(TryFrom::<f64>::try_from(rng.gen())?),
                    _ => return Err(eyre!("This arm should be unreachable")),
                };

                let instruction = Instruction::Mint(MintBox::new(value, asset_id));
                Ok(instruction)
            }
        }
    }

    pub fn generate(args: &CLI) -> Result<String> {
        let model = model::Model::generate(args)
            .wrap_err("Failed to generate model")?
            .build()
            .wrap_err("Failed to build model")?;

        if args.only_genesis {
            to_json(&model.only_genesis(), args.minify)
        } else {
            to_json(&model, args.minify)
        }
    }

    fn to_json<S: Serialize>(data: &S, minify: bool) -> Result<String> {
        if minify {
            serde_json::to_string(&data)
        } else {
            serde_json::to_string_pretty(&data)
        }
        .wrap_err("Failed to serialize JSON")
    }
}

/// `RawGenesisBlock` generator.
///
/// Generated stuff:
///
/// - Domains, Accounts and Asset Definitions
/// - Asset lifecycle actions (mints, burns, transfers)
///
/// TODO:
///
/// - Generate random (or not so) metadata
#[derive(Parser, Debug)]
pub struct CLI {
    /// Minify output JSON
    #[clap(long, short)]
    minify: bool,
    /// Print only genesis, without generated accounts data
    #[clap(long, short = 'g')]
    only_genesis: bool,
    /// How many domains to generate
    #[clap(long, short, default_value = "5")]
    pub domains: usize,
    /// How many accounts to generate per each domain
    #[clap(long, default_value = "7")]
    pub accounts_per_domain: usize,
    /// How many asset definitions to generate per each asset
    #[clap(long, default_value = "3")]
    pub assets_per_domain: usize,
    /// How many asset actions (mints, burns, transfers) to perform
    #[clap(long, default_value = "50")]
    pub asset_actions: usize,
}

fn main() -> Result<()> {
    let args = CLI::parse();
    let generated = gen::generate(&args)?;
    println!("{generated}");
    Ok(())
}
