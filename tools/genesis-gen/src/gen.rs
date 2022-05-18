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
        tx::MintBox,
    };
    use iroha_crypto::{KeyGenConfiguration, KeyPair};
    use iroha_data_model::{
        account::NewAccount,
        asset::Mintable,
        prelude::{
            Account as OriginAccount, AccountId, AssetDefinition as OriginAssetDefinition,
            AssetDefinitionId, AssetId, AssetValueType, Domain as OriginDomain, DomainId,
            Instruction, Name, RegisterBox, Value,
        },
    };
    use rand::{prelude::IteratorRandom, thread_rng, Rng};
    use serde::Serialize;
    use std::{collections::HashSet, str::FromStr};

    pub struct View {
        domains: Vec<Domain>,
        accounts: Vec<Account>,
        asset_definitions: Vec<AssetDefinition>,
        asset_actions: Vec<Instruction>,
    }

    impl View {
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

            let mut action_gen = AssetActionGen::new(&asset_definitions, &accounts);

            for _ in 0..config.asset_actions {
                asset_actions.push(
                    action_gen
                        .gen(&mut rng)
                        .wrap_err("Failed to generate random action")?,
                );
            }

            Ok(Self {
                domains,
                accounts,
                asset_definitions,
                asset_actions,
            })
        }

        pub fn build(self) -> ViewBuilt {
            let isi: Vec<_> = self
                .domains
                .into_iter()
                .map(|x| RegisterBox::new(OriginDomain::new(x.id)).into())
                .chain(
                    self.accounts
                        .clone()
                        .into_iter()
                        .map(|x| RegisterBox::new(Into::<NewAccount>::into(x)).into()),
                )
                .chain(
                    self.asset_definitions
                        .into_iter()
                        .map(|x| RegisterBox::new(Into::<OriginAssetDefinition>::into(x)).into()),
                )
                .chain(self.asset_actions.into_iter())
                .collect();

            ViewBuilt {
                raw_genesis_block: RawGenesisBlock {
                    transactions: (vec![GenesisTransaction { isi: isi.into() }]).into(),
                },
                accounts: self.accounts,
            }
        }
    }

    #[derive(Serialize)]
    pub struct ViewBuilt {
        raw_genesis_block: RawGenesisBlock,
        accounts: Vec<Account>,
    }

    impl ViewBuilt {
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
            let name = construct_iroha_name(&name)?;

            Ok(Domain {
                id: DomainId::new(name),
            })
        }
    }

    impl From<Domain> for OriginDomain {
        fn from(domain: Domain) -> Self {
            OriginDomain::new(domain.id).build()
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

                let config = KeyGenConfiguration::default().use_seed(seed);
                KeyPair::generate_with_configuration(config)?
            };
            let keys = vec![kp];

            let name: faker_rand::fr_fr::internet::Username = rng.gen();
            let name = construct_iroha_name(&name.to_string())?;

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
            use AssetValueType::{BigQuantity, Fixed, Quantity, Store};
            use Mintable::{Infinitely, Not, Once};

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
            let name = construct_iroha_name(&name.to_string())?;

            Ok(Self {
                id: AssetDefinitionId::new(name, domain_id.clone()),
                mintable,
                value_type,
            })
        }
    }

    fn construct_iroha_name(input: &str) -> Result<Name> {
        Name::from_str(input)
            .wrap_err_with(|| format!("Failed to construct Iroha Name from \"{input}\""))
    }

    struct AssetActionGen<'a> {
        assets: &'a Vec<AssetDefinition>,
        accounts: &'a Vec<Account>,
        minted_assets: HashSet<AssetDefinitionId>,
    }

    impl<'a> AssetActionGen<'a> {
        fn new(assets: &'a Vec<AssetDefinition>, accounts: &'a Vec<Account>) -> Self {
            Self {
                assets,
                accounts,
                minted_assets: HashSet::new(),
            }
        }

        /// TODO add Store registration (also link issue)
        ///
        /// For now, only minting assets
        fn gen<R: Rng>(&mut self, rng: &mut R) -> Result<Instruction> {
            let some_asset_definition = self
                .assets
                .iter()
                .filter(|x| {
                    x.value_type != AssetValueType::Store
                        && (match x.mintable {
                            Mintable::Not => false,
                            Mintable::Once => !self.minted_assets.contains(&x.id),
                            Mintable::Infinitely => true,
                        })
                })
                .choose(rng)
                .ok_or_else(|| eyre!("Failed to choose random asset definition"))?;

            let some_account = self
                .accounts
                .iter()
                .choose(rng)
                .ok_or_else(|| eyre!("Failed to choose random account"))?;

            // It should be not a very big value to avoid overflow while composing them
            let value = match some_asset_definition.value_type {
                AssetValueType::Quantity => Value::U32(rng.gen_range(0..1_000)),
                AssetValueType::BigQuantity => Value::U128(rng.gen_range(0..1_000_000)),
                AssetValueType::Fixed => {
                    Value::Fixed(TryFrom::try_from(f64::powf(rng.gen(), 0.1))?)
                }
                AssetValueType::Store => return Err(eyre!("This arm should be unreachable")),
            };

            let asset_id = AssetId::new(some_asset_definition.id.clone(), some_account.id.clone());
            let instruction = Instruction::Mint(MintBox::new(value, asset_id));

            if some_asset_definition.mintable == Mintable::Once {
                self.minted_assets.insert(some_asset_definition.id.clone());
            }

            Ok(instruction)
        }
    }

    #[cfg(test)]
    mod tests {
        use iroha_core::tx::Expression;
        use iroha_data_model::IdBox;

        use super::*;

        #[test]
        fn once_mintable_asset_minted_only_once() {
            const ISI_COUNT: usize = 1000; // enough to reduce flakyness

            let mut rng = thread_rng();
            let assets = vec![
                AssetDefinition {
                    id: "rose#wonderland".parse().unwrap(),
                    value_type: AssetValueType::Quantity,
                    mintable: Mintable::Infinitely,
                },
                AssetDefinition {
                    id: "tulip#wonderland".parse().unwrap(),
                    value_type: AssetValueType::Quantity,
                    mintable: Mintable::Once,
                },
            ];
            let accounts = vec![Account {
                id: "alice@wonderland".parse().unwrap(),
                keys: vec![],
            }];
            let mut gen = AssetActionGen::new(&assets, &accounts);

            let mut isi: Vec<_> = Vec::with_capacity(ISI_COUNT);
            for _ in 0..ISI_COUNT {
                isi.push(gen.gen(&mut rng).unwrap());
            }

            let mut tulip_mint_count = 0usize;
            for item in isi {
                if let Instruction::Mint(MintBox { destination_id, .. }) = item {
                    if let Expression::Raw(value) = *destination_id.expression {
                        if let Value::Id(IdBox::AssetId(id)) = *value {
                            if id.definition_id.to_string() == "tulip#wonderland" {
                                tulip_mint_count += 1;
                            }
                        }
                    }
                }
            }
            assert_eq!(tulip_mint_count, 1);
        }
    }
}

pub fn generate(args: &CLI) -> Result<String> {
    let model = model::View::generate(args)
        .wrap_err("Failed to generate model")?
        .build();

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
