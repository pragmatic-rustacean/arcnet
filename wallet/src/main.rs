#![allow(unused)]

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use cursive::views::TextContent;
use kanal::{AsyncReceiver, Sender};
use lib::types::Transaction;
use std::{
    io::{self, Read, Write},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::time;
use tracing::*;

use crate::{
    core::{Config, Core, FeeConfig, FeeType, Recipient},
    tasks::{handle_transaction, run_cli, ui_task, update_balance},
    utils::{big_mode_arc, generate_dummy_config, setup_panic_hook, setup_tracing},
};

mod core;
mod tasks;
mod ui;
mod utils;
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(short, long, value_name = "FILE", default_value_os_t = PathBuf::from("wallet_config.toml"))]
    config: PathBuf,
    #[arg(short, long, value_name = "ADDRESS")]
    node: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    GenerateConfig {
        #[arg(short, long, value_name = "FILE", default_value_os_t = PathBuf::from("wallet_config.toml"))]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing()?;
    setup_panic_hook();
    info!("Starting wallet application");
    let cli = Cli::parse();

    // if let Some(Commands::GenerateConfig { output }) = &cli.command {
    //     debug!("Generating dummy config at: {:?}", output);
    //     generate_dummy_config(output.clone())?;
    // }

    match &cli.command {
        Some(Commands::GenerateConfig { output }) => {
            println!("Worked out fine");
            generate_dummy_config(output.clone())?;
        }
        None => {
            println!("Encountered a nasty error.")
        }
    }

    let config_path = cli.config;
    info!("Loading config from: {:?}", config_path);
    let mut core = Core::load(config_path).await?;

    if let Some(node) = cli.node {
        info!("Overriding default node with: {}", node);
        core.config.default_node = node;
    }

    let (tx_send, tx_recv) = kanal::bounded(10);
    core.tx_sender = tx_send.clone();
    let core = Arc::new(core);
    let balance_content = TextContent::new(big_mode_arc(&core));
    info!("Starting background tasks");

    tokio::select! {
      _ = ui_task(core.clone() , balance_content.clone()) => (),
      _ = update_balance(core.clone()) => (),
      _ = handle_transaction(tx_recv.clone_async(), core.clone()) => (),
      _ = update_balance(core.clone()) => ()
    };

    info!("Application shutting down");
    Ok(())
}
