#![allow(unused, private_interfaces)]

use anyhow::{Result, anyhow};
use crossbeam_skiplist::SkipMap;
use kanal::Sender;
use lib::{
    crypto::{PrivateKey, PublicKey, Signature},
    network::Message,
    types::{Transaction, TransactionInput, TransactionOutput},
    util::Saveable,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Arc};
use tokio::{net::TcpStream, sync::Mutex};
use tracing::{debug, error, info};

/// Represent a key pair with public and private keys
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Key {
    public: PathBuf,
    pub private: PathBuf,
}
/// Represent a loaded key pair with actual public and private keys.
#[derive(Clone)]
struct LoadedKeys {
    public: PublicKey,
    private: PrivateKey,
}
/// Represent a recipient with the name and path to the public key.
#[derive(Serialize, Deserialize, Clone)]
pub struct Recipient {
    pub name: String,
    pub key: PathBuf,
}

/// Represent a loaded recipient with actual keyI
#[derive(Clone)]
pub struct LoadedRecipient {
    pub key: PublicKey,
}

impl Recipient {
    pub fn load(&self) -> Result<LoadedRecipient> {
        debug!("Loading recipient key from : {:?}", self.key);
        let public_key = PublicKey::load_from_file(&self.key)?;

        Ok(LoadedRecipient { key: public_key })
    }
}
/// Define the types of fee calculations
#[derive(Deserialize, Serialize, Clone)]
pub enum FeeType {
    Fixed,
    Percent,
}
/// Configure the fees calculations
#[derive(Serialize, Deserialize, Clone)]
pub struct FeeConfig {
    pub fee_type: FeeType,
    pub value: f64,
}
/// Store the configuration for the core.
#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub keys: Vec<Key>,
    pub contacts: Vec<Recipient>,
    pub default_node: String,
    pub fee_config: FeeConfig,
}
/// Store and manage Unspent Transaction Output (utxos)
#[derive(Clone)]
pub struct UtxosStore {
    pub keys: Vec<LoadedKeys>,
    pub utxos: Arc<SkipMap<PublicKey, Vec<(bool, TransactionOutput)>>>,
}
/// Represents the core functionality of the wallet.
pub struct Core {
    pub config: Config,
    pub utxos: UtxosStore,
    pub tx_sender: Sender<Transaction>,
    pub stream: Mutex<TcpStream>,
}

impl UtxosStore {
    pub fn new() -> Self {
        // create a new utxos store.
        Self {
            keys: Vec::new(),
            utxos: Arc::new(SkipMap::new()),
        }
    }
    /// Add a new key to store.
    pub fn add_key(&mut self, keys: LoadedKeys) {
        debug!("Adding key to utxos store: {:?}", keys.public);
        self.keys.push(keys);
    }
}

impl Core {
    /// Create a new core instance
    fn new(config: Config, utxos: UtxosStore, stream: TcpStream) -> Self {
        let (tx_sendr, _) = kanal::bounded(10);
        Self {
            config,
            utxos,
            tx_sender: tx_sendr,
            stream: Mutex::new(stream),
        }
    }
    /// Load core's configurations from a file
    pub async fn load(config: PathBuf) -> Result<Self> {
        info!("Loading from config: {:?}", config);
        let config: Config = toml::from_str(&fs::read_to_string(config)?)?;
        let mut utxos = UtxosStore::new();
        let stream = TcpStream::connect(&config.default_node).await?;
        // load config keys
        for key in &config.keys {
            let public_key = PublicKey::load_from_file(&key.public)?;
            let private_key = PrivateKey::load_from_file(&key.private)?;
            utxos.add_key(LoadedKeys {
                public: public_key,
                private: private_key,
            });
        }

        Ok(Core::new(config, utxos, stream))
    }
    /// Fetch UTXOs from the node for all keys.
    pub async fn fetch_utxos(&self) -> Result<()> {
        debug!("Fetching UTXOs from node: {}", self.config.default_node);

        for key in &self.utxos.keys {
            let message = Message::FetchUTXOs(key.public.clone());
            message.send_async(&mut *self.stream.lock().await).await?;
            if let Message::UTXOs(utxos) =
                Message::receive_async(&mut *self.stream.lock().await).await?
            {
                debug!("Received {} UTXOs for key {:?}", utxos.len(), key.public);
                // Replace the entire utxos set for this key.
                let value: Vec<(bool, TransactionOutput)> = utxos
                    .into_iter()
                    .map(|(tx_output, marked)| (marked, tx_output))
                    .collect();

                self.utxos.utxos.insert(key.public.clone(), value);
            } else {
                error!("Unexpected response from node");
                return Err(anyhow!("unexpected response receive from node."));
            }
        }
        info!("UTXOs fetched successfully");
        Ok(())
    }
    pub async fn send_transaction(&self, tx: Transaction) -> Result<()> {
        debug!("Sending transaction to node: {}", self.config.default_node);
        let message = Message::SubmitTransaction(tx);
        message.send_async(&mut *self.stream.lock().await).await?;
        info!("Transaction sent successfully");
        Ok(())
    }
    // Prepare and send transaction asynchronously
    pub async fn send_transaction_async(&self, recipient: &str, amount: u64) -> Result<()> {
        debug!("Preparing to send {} satoshis to {}", amount, recipient);
        let receiver_key = self
            .config
            .contacts
            .iter()
            .find(|rec| rec.name == recipient)
            .ok_or_else(|| anyhow!("Recipient not found"))?
            .load()?
            .key;
        let transaction = self.create_transaction(&receiver_key, amount).await?;
        debug!("Sending transaction asynchronously");
        self.tx_sender.send(transaction)?;
        Ok(())
    }

    /// Create a new transaction
    pub async fn create_transaction(
        &self,
        recipient: &PublicKey,
        amount: u64,
    ) -> Result<Transaction> {
        debug!(
            "Creating a transaction for {} satoshis to {:?}",
            amount, recipient
        );
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
            error!(
                "Insufficient funds: have {} arcs and you need {} arcs",
                input_sum, total_amount
            );
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
        info!("Transaction created successfully");
        Ok(Transaction::new(inputs, output))
    }

    /// Get the current balance of all UTXOs
    pub fn get_balance(&self) -> u64 {
        let balance = self
            .utxos
            .utxos
            .iter()
            .map(|entry| {
                entry
                    .value()
                    .iter()
                    .map(|(marked, tx_output)| return tx_output.value)
                    .sum::<u64>()
            })
            .sum::<u64>();
        debug!("Current balance: {} satoshis", balance);
        balance
    }

    /// Calculate fee for a transaction
    pub fn calculate_fee(&self, amount: u64) -> u64 {
        let fee = match self.config.fee_config.fee_type {
            FeeType::Fixed => self.config.fee_config.value as u64,
            FeeType::Percent => (amount as f64 * self.config.fee_config.value / 100.0) as u64,
        };
        debug!("Calculated fee: {} arcs", fee);
        fee
    }
}
