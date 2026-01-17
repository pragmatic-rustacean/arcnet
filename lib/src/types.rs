#![allow(unused)]
use std::collections::{HashMap, HashSet};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize, de::value::I128Deserializer};
use uuid::Uuid;

use crate::{
    crypto::{PublicKey, Signature},
    error::ArcNetError,
    sha256::Hash,
    util::MarkleRoot,
};

use super::{U256, error::Result as ArcResult};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Blockchain {
    blocks: Vec<Block>,
    utxos: HashMap<Hash, (bool, TransactionOutput)>,
    target: U256,
    #[serde(default, skip_serializing)]
    mempool: Vec<(DateTime<Utc>, Transaction)>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Block {
    pub(crate) header: BlockHeader,
    pub(crate) transactions: Vec<Transaction>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct BlockHeader {
    /// Timestamp of the block
    pub(super) timestamp: DateTime<Utc>,
    /// Nonce used to mine the block
    pub(super) nonce: u64,
    /// Hash of the previous block
    pub(super) prev_hash_block: Hash,
    /// Merkle root of the block transaction.
    pub(super) markle_root: MarkleRoot,
    /// Target
    pub(super) target: U256,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Transaction {
    pub(super) input: Vec<TransactionInput>,
    pub(super) output: Vec<TransactionOutput>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct TransactionInput {
    pub(super) prev_transaction_output_hash: Hash,
    pub(super) signature: Signature,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct TransactionOutput {
    pub(super) value: u64,
    pub(super) unique_id: Uuid,
    pub(super) pub_key: PublicKey,
}

impl TransactionOutput {
    pub(crate) fn hash(&self) -> Hash {
        self.hash()
    }
}

impl Blockchain {
    pub(crate) fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            blocks: Vec::new(),
            target: crate::MIN_TARGET,
            mempool: Vec::new(),
        }
    }
    pub(crate) fn utxos(&self) -> &HashMap<Hash, (bool, TransactionOutput)> {
        &self.utxos
    }
    pub(crate) fn target(&self) -> U256 {
        self.target
    }
    pub(crate) fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.blocks.iter()
    }
    /// Try to add a new block to the blockchain, return an error if it is not valid to insert this block in the blockchain.
    pub(crate) fn add_block(&mut self, block: Block) -> ArcResult<()> {
        if self.blocks.is_empty() {
            if block.header.hash() != Hash::zero_hash() {
                println!("Zero hash");
                return Err(ArcNetError::InvalidBlock);
            }
        } else {
            let mut last_block = self.blocks.last().expect("Failed to get the last block");
            if block.header.prev_hash_block != last_block.hash() {
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
        let tx_block: HashSet<_> = block.transactions.iter().map(|tx| tx.hash()).collect();
        // self.mempool.retain(|(_, tx)| );
        self.blocks.push(block.clone());
        self.try_adjust_target();

        Ok(())
    }

    pub(crate) fn rebuild_utxos(&mut self) {
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

    pub(self) fn block_height(&self) -> u64 {
        self.blocks.len() as u64
    }

    pub(crate) fn try_adjust_target(&mut self) {
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
    pub(crate) fn mempool(&self) -> &[(DateTime<Utc>, Transaction)] {
        todo!("Remember to keep track of time for this function. @types.rs/line 198++");
        &self.mempool
    }

    pub(crate) fn add_to_mempool(&mut self, tx: Transaction) -> ArcResult<()> {
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
                    for tx_inputs in &ref_tx.input {
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

            let all_output_values = tx.output.iter().map(|(tx_output)|tx_output.value).sum::<u64>();

            let miners_fee =  all_input_values - all_output_values;

            miners_fee
        });

        Ok(())
    }

    pub(crate) fn clean_mempool(&mut self) {
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
                        .map(|(tx_input)| tx_input.prev_transaction_output_hash),
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
}

impl Block {
    pub(crate) fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self {
            header,
            transactions,
        }
    }
    pub(crate) fn hash(&self) -> Hash {
        Hash::hash(self)
    }

    pub(crate) fn verify_transaction(
        &self,
        block_height: u64,
        utxos: &HashMap<Hash, (bool, TransactionOutput)>,
    ) -> ArcResult<()> {
        let mut tx_inputs: HashMap<Hash, TransactionOutput> = HashMap::new();
        if self.transactions.is_empty() {
            return Err(ArcNetError::InvalidTransaction);
        }

        self.verify_coinbase_transaction(block_height, &utxos);

        for transaction in self.transactions.iter().skip(1) {
            let mut tx_output = 0;
            let mut tx_input = 0;
            for input in &transaction.input {
                let prev_transaction_output = utxos.get(&input.prev_transaction_output_hash);

                if prev_transaction_output.is_none() {
                    return Err(ArcNetError::InvalidTransaction);
                }

                let prev_transaction_output = prev_transaction_output.expect(
                    "No transactions present. So we should have  an invalid transaction error",
                );

                // Prevent same block-double spending
                if tx_inputs.contains_key(&input.prev_transaction_output_hash) {
                    return Err(ArcNetError::InvalidTransaction);
                }

                // check if the signature is valid.
                if !input.signature.verify(
                    &input.prev_transaction_output_hash,
                    &prev_transaction_output.1.pub_key,
                ) {
                    return Err(ArcNetError::InvalidSignature);
                }

                tx_input += prev_transaction_output.1.value;
                tx_inputs.insert(
                    input.prev_transaction_output_hash,
                    prev_transaction_output.1.clone(),
                );
            }

            for output in &transaction.output {
                tx_output += output.value;
            }

            if tx_input < tx_output {
                return Err(ArcNetError::InvalidTransaction);
            }
        }

        Ok(())
    }

    pub(super) fn verify_coinbase_transaction(
        &self,
        predicted_block_height: u64,
        utxos: &HashMap<Hash, (bool, TransactionOutput)>,
    ) -> ArcResult<()> {
        let coinbase_transaction = &self.transactions[0];
        if coinbase_transaction.input.len() != 0 {
            return Err(ArcNetError::InvalidTransaction);
        }
        if coinbase_transaction.output.len() == 0 {
            return Err(ArcNetError::InvalidTransaction);
        }

        let miners_fee = self
            .calculate_miners_fee(utxos)
            .expect("Failed to calculate miner's fee. @types/line 216");
        let block_reward = crate::INITIAL_BLOCK_REWARD * 10u64.pow(8)
            / 2u64.pow((predicted_block_height / crate::HALVING_INTERVAL) as u32);

        let total_coinbase_output = coinbase_transaction
            .output
            .iter()
            .map(|coin| coin.value)
            .sum::<u64>();

        if total_coinbase_output != miners_fee + block_reward {
            return Err(ArcNetError::InvalidTransaction);
        }

        Ok(())
    }

    pub(super) fn calculate_miners_fee(
        &self,
        utxos: &HashMap<Hash, (bool, TransactionOutput)>,
    ) -> ArcResult<u64> {
        let mut tx_output: HashMap<Hash, TransactionOutput> = HashMap::new();
        let mut tx_input: HashMap<Hash, TransactionOutput> = HashMap::new();
        // Check every transaction after coinbase.
        for transaction in self.transactions.iter().skip(1) {
            for inputs in &transaction.input {
                let prev_tx_output = utxos.get(&inputs.prev_transaction_output_hash);
                if prev_tx_output.is_none() {
                    return Err(ArcNetError::InvalidTransaction);
                }
                let prev_tx_output = prev_tx_output
                    .expect("Couldn't retrive the previous transaction @types/line 229");
                if tx_input.contains_key(&inputs.prev_transaction_output_hash) {
                    return Err(ArcNetError::InvalidTransaction);
                }

                tx_input.insert(
                    inputs.prev_transaction_output_hash,
                    prev_tx_output.1.clone(),
                );
            }

            for outputs in &transaction.output {
                if tx_output.contains_key(&self.hash()) {
                    return Err(ArcNetError::InvalidTransaction);
                }
                tx_output.insert(self.hash(), outputs.clone());
            }
        }

        let tx_input_value = tx_input.values().map(|val| val.value).sum::<u64>();
        let tx_output_value = tx_output.values().map(|val| val.value).sum::<u64>();

        Ok(tx_input_value - tx_output_value)
    }
}

impl BlockHeader {
    pub(crate) fn new(
        timestamp: DateTime<Utc>,
        nonce: u64,
        prev_hash_block: Hash,
        markle_root: MarkleRoot,
        target: U256,
    ) -> Self {
        Self {
            timestamp,
            nonce,
            prev_hash_block,
            markle_root,
            target,
        }
    }
    pub(crate) fn hash(&self) -> Hash {
        Hash::hash(self)
    }

    pub(crate) fn mine(&mut self, steps: usize) -> bool {
        // If the block already matches the hash return early.
        if self.hash().matches_target(self.target) {
            return true;
        }

        for _ in 0..steps {
            if let Some(new_nonce) = self.nonce.checked_add(1) {
                self.nonce = new_nonce
            } else {
                self.nonce = 0;
                self.timestamp = Utc::now();
            };

            if self.hash().matches_target(self.target) {
                return true;
            }
        }

        false
    }
}

impl Transaction {
    pub(crate) fn new(input: Vec<TransactionInput>, output: Vec<TransactionOutput>) -> Self {
        Self { input, output }
    }
    pub(crate) fn hash(&self) -> Hash {
        Hash::hash(self)
    }
}
