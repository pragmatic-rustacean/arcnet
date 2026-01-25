#![allow(unused)]

use crate::{
    core::{Config, Core, FeeConfig, FeeType, Recipient},
    ui::run_ui,
    utils::generate_dummy_config,
};
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
use tokio::{task::JoinHandle, time};
use tracing::*;

#[derive(Clone, Copy)]
pub enum Unit {
    Sats,
    Arcs,
}

pub async fn update_balance(core: Arc<Core>) {
    let mut interval = time::interval(Duration::from_secs(20));
    loop {
        interval.tick();
        if let Err(er) = core.fetch_utxos().await {
            error!("Failed to update the utxos: {}", er);
        }
    }
}

pub async fn handle_transaction(rx: AsyncReceiver<Transaction>, core: Arc<Core>) {
    while let Ok(transaction) = rx.recv().await {
        if let Err(er) = core.send_transaction(transaction).await {
            error!("Failed to send transaction: {}", er);
        }
    }
}

pub async fn run_cli(core: Arc<Core>) -> Result<()> {
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
                core.tx_sender.send(transaction)?;
                println!("transaction sent succesfully");
                core.fetch_utxos().await?;
            }
            "exit" => break,
            _ => println!("unknown command"),
        }
    }

    Ok(())
}

pub async fn ui_task(core: Arc<Core>, balance_content: TextContent) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        info!("Running UI");
        if let Err(er) = run_ui(core, balance_content) {
            eprintln!("UI ended with an error");
        };
    })
}

pub fn convert_amount(amount: f64, from: Unit, to: Unit) -> f64 {
    match (to, from) {
        (Unit::Arcs, Unit::Sats) => amount * 100_000_000.0,
        (Unit::Sats, Unit::Sats) => amount / 100_000_000.0,
        _ => amount,
    }
}
