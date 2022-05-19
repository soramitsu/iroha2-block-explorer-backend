use crate::CLI;
use color_eyre::{eyre::Context, Result};
use serde::Serialize;

/// Generator data model.
///
/// Based on Iroha Data Model, but simpler.
mod model {
    use crate::CLI;
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
    use itertools::Itertools;
    use rand::{
        prelude::{IteratorRandom, ThreadRng},
        thread_rng, Rng,
    };
    use serde::Serialize;
    use std::{collections::HashSet, hash::Hash, str::FromStr};

    /// When there are too many ISI in a single transaction, Iroha doesn't accept it.
    ///
    /// [issue](https://github.com/hyperledger/iroha/issues/2232)
    const TRANSACTION_ISI_CHUNK_SIZE: usize = 1400;

    pub struct View {
        chunks: Vec<SingleChunkView>,
    }

    impl View {
        pub fn generate(cfg: &CLI) -> Result<Self> {
            let mut rand_help = RandHelp::with_used_names(UsedNames::with_capacity(
                cfg.total_domain_names(),
                cfg.total_account_names(),
                cfg.total_asset_names(),
            ));

            Ok(Self {
                chunks: (0..cfg.chunks.get())
                    .into_iter()
                    .map(|_| SingleChunkView::generate(cfg, &mut rand_help))
                    .collect::<Result<Vec<SingleChunkView>>>()?,
            })
        }

        pub fn build(self) -> ViewBuilt {
            let (accounts, txs): (Vec<_>, Vec<_>) = self
                .chunks
                .into_iter()
                .map(|view| {
                    let accounts = view.accounts.clone();
                    let isi = view.into_isis().take(TRANSACTION_ISI_CHUNK_SIZE);
                    let tx = GenesisTransaction { isi: isi.collect() };
                    (accounts, tx)
                })
                .multiunzip();

            ViewBuilt {
                raw_genesis_block: RawGenesisBlock {
                    transactions: txs.into(),
                },
                accounts: accounts.into_iter().flatten().collect(),
            }
        }
    }

    struct SingleChunkView {
        domains: Vec<Domain>,
        accounts: Vec<Account>,
        asset_definitions: Vec<AssetDefinition>,
        asset_actions: Vec<Instruction>,
    }

    impl SingleChunkView {
        fn generate(config: &CLI, rand_help: &mut RandHelp) -> Result<Self> {
            let mut domains: Vec<Domain> = Vec::with_capacity(config.domains);
            let mut accounts: Vec<Account> = Vec::with_capacity(config.accounts_per_chunk());
            let mut asset_definitions: Vec<AssetDefinition> =
                Vec::with_capacity(config.assets_per_chunk());
            let mut asset_actions: Vec<Instruction> = Vec::with_capacity(config.asset_actions);

            for _ in 0..config.domains {
                let domain = Domain::new(rand_help).wrap_err("Failed to generate domain")?;

                for _ in 0..config.accounts_per_domain {
                    let account = Account::new(rand_help, &domain.id)
                        .wrap_err("Failed to generate account")?;
                    accounts.push(account);
                }

                for _ in 0..config.assets_per_domain {
                    let definition = AssetDefinition::new(rand_help, &domain.id)
                        .wrap_err("Failed to generage asset definition")?;
                    asset_definitions.push(definition);
                }

                domains.push(domain);
            }

            let mut action_gen = AssetActionGen::new(&asset_definitions, &accounts);

            for _ in 0..config.asset_actions {
                asset_actions.push(
                    action_gen
                        .gen(&mut rand_help.rng)
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

        fn into_isis(self) -> impl Iterator<Item = Instruction> {
            self.domains
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

    /// For used names tracking
    #[derive(Default)]
    struct UsedNames {
        domains: HashSet<String>,
        accounts: HashSet<String>,
        asset_definitions: HashSet<String>,
    }

    impl UsedNames {
        fn with_capacity(domains: usize, accounts: usize, definitions: usize) -> Self {
            Self {
                domains: HashSet::with_capacity(domains),
                accounts: HashSet::with_capacity(accounts),
                asset_definitions: HashSet::with_capacity(definitions),
            }
        }
    }

    struct RandHelp<'a> {
        rng: ThreadRng,
        petnames: petname::Petnames<'a>,
        used_names: UsedNames,
    }

    impl<'a> RandHelp<'a> {
        fn with_used_names(used_names: UsedNames) -> Self {
            Self {
                rng: thread_rng(),
                petnames: petname::Petnames::large(),
                used_names,
            }
        }
    }

    struct Domain {
        id: DomainId,
    }

    impl Domain {
        fn new(rnd: &mut RandHelp) -> Result<Self> {
            let name = try_gen_rand_non_repetitive_value(&mut rnd.used_names.domains, || {
                let name: faker_rand::en_us::internet::Domain = rnd.rng.gen();
                let name = name.to_string().replace(' ', "_");
                let name = construct_iroha_name(&name)?;
                Ok((name.to_string(), name))
            })
            .wrap_err("Failed to generate domain name")?;

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
        fn new(rnd: &mut RandHelp, domain_id: &DomainId) -> Result<Self> {
            let kp: KeyPair = {
                let seed_len: usize = rnd.rng.gen_range(5..30);
                let mut seed: Vec<u8> = Vec::with_capacity(seed_len);
                for _ in 0..seed_len {
                    seed.push(rnd.rng.gen());
                }

                let config = KeyGenConfiguration::default().use_seed(seed);
                KeyPair::generate_with_configuration(config)?
            };
            let keys = vec![kp];

            let id = Self::gen_account_id(rnd, domain_id)?;

            Ok(Self { id, keys })
        }

        fn gen_account_id(rnd: &mut RandHelp, domain_id: &DomainId) -> Result<AccountId> {
            try_gen_rand_non_repetitive_value(&mut rnd.used_names.accounts, || {
                let name = rnd.petnames.generate(&mut rnd.rng, 3, "_");
                let name = construct_iroha_name(&name)?;
                let id = AccountId::new(name, domain_id.clone());
                let id_str = id.to_string();
                Ok((id_str, id))
            })
            .wrap_err("Failed to generate account id")
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
        fn new(rnd: &mut RandHelp, domain_id: &DomainId) -> Result<Self> {
            let mintable: Mintable = match rnd.rng.gen_range(0..10u32) {
                5..=9 => Mintable::Infinitely,
                2..=4 => Mintable::Not,
                0..=1 => Mintable::Once,
                x => return Err(eyre!("Unexpected random num: {x}")),
            };

            let value_type: AssetValueType = match rnd.rng.gen_range(0..10u32) {
                7..=9 => AssetValueType::Quantity,
                4..=6 => AssetValueType::BigQuantity,
                1..=3 => AssetValueType::Fixed,
                0 => AssetValueType::Store,
                x => return Err(eyre!("Unexpected random num: {x}")),
            };

            let id =
                try_gen_rand_non_repetitive_value(&mut rnd.used_names.asset_definitions, || {
                    let name = rnd.petnames.generate(&mut rnd.rng, 3, "-");
                    let name = construct_iroha_name(&name)?;
                    let id = AssetDefinitionId::new(name, domain_id.clone());
                    let id_str = id.to_string();
                    Ok((id_str, id))
                })
                .wrap_err("Failed to generate definition id")?;

            Ok(Self {
                id,
                value_type,
                mintable,
            })
        }
    }

    fn construct_iroha_name(input: &str) -> Result<Name> {
        Name::from_str(input)
            .wrap_err_with(|| format!("Failed to construct Iroha Name from \"{input}\""))
    }

    /// Useful for random names generation in Domain, Account etc
    ///
    /// # Errors
    /// Fails if there are too much attempts or if value generation fails
    fn try_gen_rand_non_repetitive_value<T, K, F>(used: &mut HashSet<K>, mut f: F) -> Result<T>
    where
        K: Hash + Eq,
        F: FnMut() -> Result<(K, T)>,
    {
        const MAX_ATTEMPTS: usize = 100;

        let mut iterations: usize = 0;

        loop {
            let (key, value) = f()?;

            if !used.contains(&key) {
                used.insert(key);
                return Ok(value);
            }

            iterations += 1;
            if iterations >= MAX_ATTEMPTS {
                return Err(eyre!(
                    "Too much attempts to generate random non-repetitive value (max: {MAX_ATTEMPTS})"
                ));
            }
        }
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

        /// TODO add Store registration [issue](https://github.com/hyperledger/iroha/issues/2227)
        /// TODO add assets transfers
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

        #[test]
        fn isis_are_chunked() {
            let ViewBuilt {
                raw_genesis_block, ..
            } = View::generate(&CLI {
                domains: 1,
                accounts_per_domain: 1,
                assets_per_domain: 50,
                asset_actions: 5_000,
                minify: false,
                only_genesis: false,
                chunks: 5.try_into().unwrap(),
            })
            .unwrap()
            .build();

            for GenesisTransaction { isi } in raw_genesis_block.transactions {
                more_asserts::assert_le!(isi.len(), TRANSACTION_ISI_CHUNK_SIZE);
            }
        }

        #[test]
        fn domains_are_not_repeated() {
            repetition_test_factory(
                |rnd| {
                    let Domain { id } = Domain::new(rnd).unwrap();
                    id.to_string()
                },
                500,
            );
        }

        #[test]
        fn accounts_are_not_repeated() {
            let domain_id = DomainId::from_str("wonderland").unwrap();

            repetition_test_factory(
                |rnd| {
                    // Testing `gen_account_id` to avoid keys generation
                    let id = Account::gen_account_id(rnd, &domain_id).unwrap();
                    id.to_string()
                },
                100_000,
            );
        }

        #[test]
        fn asset_definitions_are_not_repeated() {
            let domain_id = DomainId::from_str("wonderland").unwrap();

            repetition_test_factory(
                |rnd| {
                    let AssetDefinition { id, .. } = AssetDefinition::new(rnd, &domain_id).unwrap();
                    id.to_string()
                },
                10_000,
            );
        }

        fn repetition_test_factory<F>(f: F, repeat_count: usize)
        where
            F: Fn(&mut RandHelp) -> String,
        {
            let mut rnd = RandHelp::with_used_names(UsedNames::with_capacity(
                repeat_count,
                repeat_count,
                repeat_count,
            ));
            let mut occured = HashSet::with_capacity(repeat_count);

            for _ in 0..repeat_count {
                let id = f(&mut rnd);
                assert!(!occured.contains(&id));
                occured.insert(id);
            }
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
