#![allow(unused, private_interfaces)]

use anyhow::{Result, anyhow};
use crossbeam_skiplist::SkipMap;
use kanal::AsyncSender;
use lib::{
    crypto::{PrivateKey, PublicKey, Signature},
    network::Message,
    types::{Transaction, TransactionInput, TransactionOutput},
    util::Saveable,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Arc};
use tokio::net::TcpStream;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Key {
    public: PathBuf,
    private: PathBuf,
}

#[derive(Clone)]
struct LoadedKeys {
    public: PublicKey,
    private: PrivateKey,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Recipient {
    pub name: String,
    pub key: PathBuf,
}

#[derive(Clone)]
pub struct LoadedRecipient {
    pub name: String,
    pub key: PublicKey,
}

impl Recipient {
    pub fn load(&self) -> Result<LoadedRecipient> {
        let public_key = PublicKey::load_from_file(&self.key)?;

        Ok(LoadedRecipient {
            name: self.name.clone(),
            key: public_key,
        })
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub enum FeeType {
    Fixed,
    Percent,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FeeConfig {
    pub fee_type: FeeType,
    pub value: f64,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub keys: Vec<Key>,
    pub contacts: Vec<Recipient>,
    pub default_node: String,
    pub fee_config: FeeConfig,
}

#[derive(Clone)]
pub struct UtxosStore {
    pub keys: Vec<LoadedKeys>,
    pub utxos: Arc<SkipMap<PublicKey, Vec<(bool, TransactionOutput)>>>,
}

#[derive(Clone)]
pub struct Core {
    pub config: Config,
    pub utxos: UtxosStore,
    pub tx_sender: AsyncSender<Transaction>,
}

impl UtxosStore {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            utxos: Arc::new(SkipMap::new()),
        }
    }
    pub fn add_key(&mut self, keys: LoadedKeys) {
        self.keys.push(keys);
    }
}

impl Core {
    fn new(config: Config, utxos: UtxosStore) -> Self {
        let (sender, _) = kanal::bounded(10);
        Self {
            config,
            utxos,
            tx_sender: sender.clone_async(),
        }
    }
    pub fn load(config: PathBuf) -> Result<Self> {
        let config: Config = toml::from_str(&fs::read_to_string(config)?)?;
        let mut utxos = UtxosStore::new();
        for key in &config.keys {
            let public_key = PublicKey::load_from_file(&key.public)?;
            let private_key = PrivateKey::load_from_file(&key.private)?;
            utxos.add_key(LoadedKeys {
                public: public_key,
                private: private_key,
            });
        }
        Ok(Core::new(config, utxos))
    }
    pub async fn fetch_utxos(&self) -> Result<()> {
        let mut stream = TcpStream::connect(&self.config.default_node).await?;
        for key in &self.utxos.keys {
            let message = Message::FetchUTXOs(key.public.clone());
            message.send_async(&mut stream).await?;
            if let Message::UTXOs(utxos) = Message::receive_async(&mut stream).await? {
                // Replace the entire utxos set for this key.
                let value: Vec<(bool, TransactionOutput)> = utxos
                    .into_iter()
                    .map(|(tx_output, marked)| (marked, tx_output))
                    .collect();

                self.utxos.utxos.insert(key.public.clone(), value);
            } else {
                return Err(anyhow!("unexpected response receive from node."));
            }
        }
        Ok(())
    }
    pub async fn send_transaction(&self, tx: Transaction) -> Result<()> {
        let mut stream = TcpStream::connect(&self.config.default_node).await?;
        let message = Message::SubmitTransaction(tx);
        message.send_async(&mut stream).await?;
        Ok(())
    }
    pub async fn create_transaction(
        &self,
        recipient: &PublicKey,
        amount: u64,
    ) -> Result<Transaction> {
        let fee = self.calculate_fee(amount);
        let total_amount = fee + amount;
        let mut inputs = Vec::new();
        let mut input_sum = 0;

        for entry in self.utxos.utxos.iter() {
            let pubkey = entry.key();
            let utxos = entry.value();
            for (marked, tx_output) in utxos.iter() {
                if *marked {
                    continue;
                }
                if input_sum >= total_amount {
                    break;
                }

                inputs.push(TransactionInput {
                    prev_transaction_output_hash: tx_output.hash(),
                    signature: Signature::sign_output(
                        &tx_output.hash(),
                        &self
                            .utxos
                            .keys
                            .iter()
                            .find(|keys| keys.public == *pubkey)
                            .expect("Couldn't find the key")
                            .private,
                    ),
                });

                input_sum += tx_output.value;
            }
            if input_sum >= total_amount {
                break;
            }
        }
        if input_sum < total_amount {
            return Err(anyhow!("Insufficient fund"));
        }
        let mut output = vec![TransactionOutput {
            value: amount,
            unique_id: uuid::Uuid::new_v4(),
            pub_key: self.utxos.keys[0].public.clone(),
        }];

        if input_sum > total_amount {
            output.push(TransactionOutput {
                value: input_sum - total_amount,
                unique_id: uuid::Uuid::new_v4(),
                pub_key: self.utxos.keys[0].public.clone(),
            });
        }

        Ok(Transaction::new(inputs, output))
    }

    pub fn get_balance(&self) -> u64 {
        self.utxos
            .utxos
            .iter()
            .map(|entry| {
                entry
                    .value()
                    .iter()
                    .map(|(marked, tx_output)| return tx_output.value)
                    .sum::<u64>()
            })
            .sum::<u64>()
    }

    pub fn calculate_fee(&self, amount: u64) -> u64 {
        match self.config.fee_config.fee_type {
            FeeType::Fixed => self.config.fee_config.value as u64,
            FeeType::Percent => (amount as f64 * self.config.fee_config.value / 100.0) as u64,
        }
    }
}
