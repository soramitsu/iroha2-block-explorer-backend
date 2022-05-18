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
}

fn main() -> Result<()> {
    let args = CLI::parse();
    let generated = gen::generate(&args)?;
    println!("{generated}");
    Ok(())
}
