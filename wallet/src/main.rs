#![allow(unused)]

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use kanal::{AsyncReceiver, Sender};
use lib::types::Transaction;
use std::{
    io::{self, Read, Write},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::time;

use crate::core::{Config, Core, FeeConfig, FeeType, Recipient};

mod core;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    #[arg(short, long, value_name = "ADDRESS")]
    node: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    GenerateConfig {
        #[arg(short, long, value_name = "FILE")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::GenerateConfig { output }) = &cli.command {
        return generate_dummy_config(output.clone());
    }

    let config_path = cli
        .config
        .unwrap_or_else(|| PathBuf::from("wallet_config.toml"));
    let mut core = Core::load(config_path).expect("failed to load wallet config");

    if let Some(node) = cli.node {
        core.config.default_node = node;
    }

    let (tx_send, tx_recv) = kanal::bounded(10);
    core.tx_sender = tx_send.clone_async();
    let core = Arc::new(core);
    tokio::spawn(update_utxos(core.clone()));
    tokio::spawn(handle_transaction(tx_recv.clone_async(), core.clone()));

    run_cli(core.clone()).await?;

    Ok(())
}

fn generate_dummy_config(path: PathBuf) -> Result<()> {
    let dummy_config = Config {
        keys: vec![],
        contacts: vec![
            Recipient {
                name: "Niko".to_string(),
                key: PathBuf::from("rose.pub.poem"),
            },
            Recipient {
                name: "James".to_string(),
                key: PathBuf::from("niko.pub.poem"),
            },
        ],
        default_node: "127.0.0.1:9000".to_string(),
        fee_config: FeeConfig {
            fee_type: FeeType::Percent,
            value: 0.1,
        },
    };
    let config_pr = toml::to_string_pretty(&dummy_config)?;
    std::fs::write(&path, config_pr)?;
    println!("Dummy config generated at : {}", path.display());

    Ok(())
}

async fn update_utxos(core: Arc<Core>) {
    let mut interval = time::interval(Duration::from_secs(20));
    loop {
        interval.tick();
        if let Err(er) = core.fetch_utxos().await {
            eprintln!("Failed to update UTXOs {}", er);
        }
    }
}

async fn handle_transaction(rx: AsyncReceiver<Transaction>, core: Arc<Core>) {
    while let Ok(transaction) = rx.recv().await {
        if let Err(er) = core.send_transaction(transaction).await {
            eprintln!("failed to send transaction {}", er);
        }
    }
}

async fn run_cli(core: Arc<Core>) -> Result<()> {
    loop {
        println!(">!");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .expect("Failed to read the input to a string...");
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "balance" => {
                println!("Your current balance is {} satoshis", core.get_balance())
            }
            "send" => {
                if parts.len() != 3 {
                    println!("Usage: send <recipient>: <amount>");
                    continue;
                }
                let recipient = parts[1];
                let amount: u64 = parts[2].parse()?;
                let recipient_key = core
                    .config
                    .contacts
                    .iter()
                    .find(|rec| rec.name == recipient)
                    .ok_or_else(|| anyhow!("Recipient not found"))?
                    .load()?
                    .key;

                if let Err(err) = core.fetch_utxos().await {
                    eprintln!("failed to fetch utxos {}", err);
                }

                let transaction = core.create_transaction(&recipient_key, amount).await?;
                core.tx_sender.send(transaction).await?;
                println!("transaction sent succesfully");
                core.fetch_utxos().await?;
            }
            "exit" => break,
            _ => println!("unknown command"),
        }
    }

    Ok(())
}
