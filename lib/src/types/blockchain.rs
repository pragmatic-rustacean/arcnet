#![allow(unused)]

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::{Error as IOError, ErrorKind as IOErrorKind},
};

use super::{
    block::Block,
    transaction::{Transaction, TransactionOutput},
};
use crate::{
    U256,
    error::{ArcNetError, Result as ArcResult},
    sha256::Hash,
    util::{MarkleRoot, Saveable},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Blockchain {
    #[serde(default)]
    blocks: Vec<Block>,
    #[serde(default)]
    utxos: HashMap<Hash, (bool, TransactionOutput)>,
    #[serde(default = "default_target")]
    target: U256,
    #[serde(default, skip_serializing)]
    mempool: Vec<(DateTime<Utc>, Transaction)>,
}

fn default_target() -> U256 {
    crate::MIN_TARGET
}

/// Save and load expect CBOR from ciborium as format
impl Saveable for Blockchain {
    fn load<R: std::io::Read>(mut reader: R) -> std::io::Result<Self> {
        ciborium::de::from_reader(reader).map_err(|err| {
            IOError::new(
                IOErrorKind::InvalidData,
                format!("failed to deserialize blockchain: {err}"),
            )
        })
    }

    fn save<W: std::io::Write>(&self, writer: W) -> std::io::Result<()> {
        ciborium::ser::into_writer(self, writer)
            .map_err(|_| IOError::new(IOErrorKind::InvalidData, "Failed to serialize blockchain"))
    }
}

impl Blockchain {
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            blocks: Vec::new(),
            target: crate::MIN_TARGET,
            mempool: Vec::new(),
        }
    }
    pub fn utxos(&self) -> &HashMap<Hash, (bool, TransactionOutput)> {
        &self.utxos
    }
    pub fn target(&self) -> U256 {
        self.target
    }
    pub fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.blocks.iter()
    }
    /// Try to add a new block to the blockchain, return an error if it is not valid to insert this block in the blockchain.
    pub fn add_block(&mut self, block: Block) -> ArcResult<()> {
        if self.blocks.is_empty() {
            if block.header.hash() != Hash::zero() {
                println!("Zero hash");
                return Err(ArcNetError::InvalidBlock);
            }
        } else {
            let last_block = self.blocks.last().expect("Failed to get the last block");
            if block.header.prev_block_hash != last_block.hash() {
                println!("Hashes did not match.");
                return Err(ArcNetError::InvalidBlock);
            }

            /*
              => Check whether the block hash is less than the target.
            */
            if !block.header.hash().matches_target(block.header.target) {
                println!("Does not match target...");
                return Err(ArcNetError::InvalidBlock);
            };

            let calculated_markle_root = MarkleRoot::calculate(&block.transactions);

            if calculated_markle_root != block.header.markle_root {
                println!("Invalid merkle error");
                return Err(ArcNetError::InvalidMerkleError);
            }

            if block.header.timestamp <= last_block.header.timestamp {
                return Err(ArcNetError::InvalidBlock);
            }

            // Verify transaction...
            block
                .verify_transaction(self.block_height(), &self.utxos)
                .expect("Failed to verify transactions. @types.rs/line 103");
        };
        // Remove transactions from mempo ol that are now part in the block.
        let _tx_block: HashSet<_> = block.transactions.iter().map(|tx| tx.hash()).collect();
        // self.mempool.retain(|(_, tx)| );
        self.blocks.push(block.clone());
        self.try_adjust_target();

        Ok(())
    }

    pub fn rebuild_utxos(&mut self) {
        for block in &self.blocks {
            for transaction in &block.transactions {
                for input in &transaction.input {
                    self.utxos.remove(&input.prev_transaction_output_hash);
                }
                for output in transaction.output.iter() {
                    self.utxos
                        .insert(transaction.hash(), (false, output.clone()));
                }
            }
        }
    }

    pub fn block_height(&self) -> u64 {
        self.blocks.len() as u64
    }

    pub fn try_adjust_target(&mut self) {
        if self.blocks.is_empty() {
            return;
        }
        if self.blocks.len() % crate::DIFFICULTY_UPDATE_INTERVAL as usize != 0 {
            return;
        }

        // Measure the time it took to mine the last crate::DIFFICULTY_UPDATE_INTERVAL blocks with chrono.
        let start_time = self.blocks
            [self.blocks.len() - crate::DIFFICULTY_UPDATE_INTERVAL as usize]
            .header
            .timestamp;
        let end_time = self
            .blocks
            .last()
            .expect("Last block not found.")
            .header
            .timestamp;
        let time_diff = end_time - start_time;
        // convert time difference into seconds.
        let time_diff_secs = time_diff.num_seconds();
        // calculate the ideal time, for the target.
        let target_time = crate::IDEAL_BLOCK_TIME * crate::DIFFICULTY_UPDATE_INTERVAL;
        let new_target = BigDecimal::parse_bytes(&self.target.to_string().as_bytes(), 10)
            .expect("Bug: impossible")
            * (BigDecimal::from(time_diff_secs) / BigDecimal::from(target_time));

        let new_target_str = new_target
            .to_string()
            .split(".")
            .next()
            .expect("Bug: Expected a decimal point. @types/line 165")
            .to_string();
        let new_target =
            U256::from_str_radix(&new_target_str, 10).expect("Bug: Impossible. @types/line 167");

        // Clamp the new target to be within the range of self.target / 4 and self.target * 4.
        let new_target = if new_target < self.target / 4 {
            self.target / 4
        } else if new_target > self.target * 4 {
            self.target * 4
        } else {
            new_target
        };

        // If the new target it greater than the minimum target, then set it to the minimum target.
        self.target = new_target.min(crate::MIN_TARGET);
    }

    // mempool
    pub fn mempool(&self) -> &[(DateTime<Utc>, Transaction)] {
        &self.mempool
    }

    pub fn add_to_mempool(&mut self, tx: Transaction) -> ArcResult<()> {
        // Validate transactions before insertion.
        // All inputs must match known UTXOs, and must be unique.
        let mut known_inputs: HashSet<Hash> = HashSet::new();
        for tx_input in &tx.input {
            if !self
                .utxos
                .contains_key(&tx_input.prev_transaction_output_hash)
            {
                println!("UTXO not found");
                return Err(ArcNetError::InvalidTransaction);
            }

            if known_inputs.contains(&tx_input.prev_transaction_output_hash) {
                println!("Duplicate input");
                return Err(ArcNetError::InvalidTransaction);
            }

            known_inputs.insert(tx_input.prev_transaction_output_hash);
        }

        // check if there is any of the UTXOS that have a bool mark set to true, and if there is, find the transaction that references it in the mempool, remove it, and set all the UTXOS it references to false.
        for tx_input in &tx.input {
            if let Some((true, _)) = self.utxos.get(&tx_input.prev_transaction_output_hash) {
                // find the transaction that references the UTXO that we are trying to reference
                let ref_tx = self.mempool.iter().enumerate().find(|(_, (_, tx))| {
                    tx.output
                        .iter()
                        .any(|tx_output| tx_output.hash() == tx_input.prev_transaction_output_hash)
                });

                // if we've found one unmark all of it's UTXOS.
                if let Some((idx, (_, ref_tx))) = ref_tx {
                    for _tx_inputs in &ref_tx.input {
                        // set all UTXO's from this transaction to false.
                        self.utxos
                            .entry(tx_input.prev_transaction_output_hash)
                            .and_modify(|(marked, _)| *marked = false);
                    }

                    self.mempool.remove(idx);
                } else {
                    // If somehow there is no transaction set this UTXO to false.
                    self.utxos
                        .entry(tx_input.prev_transaction_output_hash)
                        .and_modify(|(marked, _)| *marked = false);
                }
            }
        }

        // All inputs must be lower than the output
        let all_input_values = tx
            .input
            .iter()
            .map(|tx_input| {
                self.utxos
                    .get(&tx_input.prev_transaction_output_hash)
                    .expect("Bug: Failed")
                    .1
                    .value
            })
            .sum::<u64>();

        let all_output_values = tx
            .output
            .iter()
            .map(|tx_output| tx_output.value)
            .sum::<u64>();

        if all_output_values > all_input_values {
            println!("Inputs are lower than output");
            return Err(ArcNetError::InvalidTransaction);
        }

        // Mark the UTXO's as used.
        for tx_input in &tx.input {
            self.utxos
                .entry(tx_input.prev_transaction_output_hash)
                .and_modify(|(marked, _)| *marked = true);
        }

        self.mempool.push((Utc::now(), tx));
        // Sort the transactions by miner's fee.
        self.mempool.sort_by_key(|(_, tx)| {
            let all_input_values = tx.input.iter().map(|tx_input| {
                self.utxos
                    .get(&tx_input.prev_transaction_output_hash)
                    .expect("Bug: Couldn't retrieve the previous transaction output hash @types.rs/line 209")
                    .1
                    .value
            }).sum::<u64>();

            let all_output_values = tx.output.iter().map(|tx_output|tx_output.value).sum::<u64>();

            let miners_fee =  all_input_values - all_output_values;

            miners_fee
        });

        Ok(())
    }

    pub fn clean_mempool(&mut self) {
        let now = Utc::now();
        let mut utxo_hashes_to_unmark: Vec<Hash> = vec![];
        self.mempool.retain(|(timestamp, tx)| {
            if now - *timestamp
                < chrono::Duration::seconds(crate::MAX_MEMPOOL_TRANSACTION_AGE as i64)
            {
                // send all the utxos to the vector so that we can unmark them later.
                utxo_hashes_to_unmark.extend(
                    tx.input
                        .iter()
                        .map(|tx_input| tx_input.prev_transaction_output_hash),
                );
                false
            } else {
                true
            }
        });
        // Unmark all the transaction.
        for utxo_hash in utxo_hashes_to_unmark {
            self.utxos
                .entry(utxo_hash)
                .and_modify(|(marked, _)| *marked = false);
        }
    }
    pub fn calculate_block_reward(&self) -> u64 {
        let block_height = self.block_height();
        let halvings = block_height / crate::HALVING_INTERVAL;
        (crate::INITIAL_BLOCK_REWARD * 10u64.pow(8)) >> halvings
    }
}
