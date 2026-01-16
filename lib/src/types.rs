#![allow(unused)]

use std::collections::HashMap;

use chrono::{DateTime, Utc};
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
    pub(crate) utxos: HashMap<Hash, TransactionOutput>,
    pub(crate) blocks: Vec<Block>,
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

impl Blockchain {
    pub(crate) fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            blocks: Vec::new(),
        }
    }
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
            // if !block.header.hash().matches_target() {}
            todo!("confirm the matches_target method. @types/line 84");

            let calculated_markle_root = MarkleRoot::calculate(&block.transactions);

            if calculated_markle_root != block.header.markle_root {
                println!("Invalid merkle error");
                return Err(ArcNetError::InvalidMerkleError);
            }

            if block.header.timestamp <= last_block.header.timestamp {
                return Err(ArcNetError::InvalidBlock);
            }

            // Verify transaction...
            todo!("Add the verification of transaction logic, @types/line 91");
        };
        self.blocks.push(block);

        Ok(())
    }

    pub(crate) fn rebuild_utxos(&mut self) {
        for block in &self.blocks {
            for transaction in &block.transactions {
                for input in &transaction.input {
                    self.utxos.remove(&input.prev_transaction_output_hash);
                }
                for output in transaction.output.iter() {
                    self.utxos.insert(transaction.hash(), output.clone());
                }
            }
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
        utxos: HashMap<Hash, TransactionOutput>,
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
                    &prev_transaction_output.pub_key,
                ) {
                    return Err(ArcNetError::InvalidSignature);
                }

                tx_input += prev_transaction_output.value;
                tx_inputs.insert(
                    input.prev_transaction_output_hash,
                    prev_transaction_output.clone(),
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
        utxos: &HashMap<Hash, TransactionOutput>,
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
            .expect("Failed to calculate miner's fee. @types/line 194");
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
        utxos: &HashMap<Hash, TransactionOutput>,
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

                tx_input.insert(inputs.prev_transaction_output_hash, prev_tx_output.clone());
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
}

impl Transaction {
    pub(crate) fn new(input: Vec<TransactionInput>, output: Vec<TransactionOutput>) -> Self {
        Self { input, output }
    }
    pub(crate) fn hash(&self) -> Hash {
        Hash::hash(self)
    }
}
