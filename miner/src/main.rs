use anyhow::{Result, anyhow};
use clap::Parser;
use lib::{crypto::PublicKey, util::Saveable};
use miner::Miner;

mod miner;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    address: String,
    #[arg(short, long)]
    pub_key_file: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let public_key = PublicKey::load_from_file(&cli.pub_key_file)
        .map_err(|_| {
            let _ = anyhow!("Error reading public key");
        })
        .expect("Failed at @miner/main.rs/line 24");

    let miner = Miner::new(cli.address, public_key).await?;

    miner.run().await
}
