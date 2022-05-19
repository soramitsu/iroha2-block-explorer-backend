use std::num::NonZeroUsize;

use clap::Parser;
use color_eyre::Result;

/// Generator main stuff
mod gen;

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
    /// How many chunks (transactions) to generate. Due to Iroha limitations,
    /// each transaction runs in a clean World-State-View, and transaction size is limited
    /// for some reason to ~1400 instructions. Thus, you can specify how many random separate
    /// data chunks to generate
    #[clap(long, default_value = "1")]
    pub chunks: NonZeroUsize,
}

impl CLI {
    fn accounts_per_chunk(&self) -> usize {
        self.domains * self.accounts_per_domain
    }

    fn assets_per_chunk(&self) -> usize {
        self.domains * self.assets_per_domain
    }

    fn total_domain_names(&self) -> usize {
        self.domains * self.chunks.get()
    }

    fn total_account_names(&self) -> usize {
        self.accounts_per_chunk() & self.chunks.get()
    }

    fn total_asset_names(&self) -> usize {
        self.assets_per_chunk() & self.chunks.get()
    }
}

fn main() -> Result<()> {
    let args = CLI::parse();
    let generated = gen::generate(&args)?;
    println!("{generated}");
    Ok(())
}
