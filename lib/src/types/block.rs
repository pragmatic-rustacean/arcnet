use crate::{
    U256,
    error::{ArcNetError, Result as ArcResult},
    sha256::Hash,
    util::{MarkleRoot, Saveable},
};

use std::io::{Error as IOError, ErrorKind as IOErrorKind, Read, Result as IOResult, Write};

use super::transaction::{Transaction, TransactionInput, TransactionOutput};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    pub(crate) header: BlockHeader,
    pub(crate) transactions: Vec<Transaction>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockHeader {
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

/// Save and load expect CBOR from ciborium as format
impl Saveable for Block {
    fn load<R: std::io::Read>(reader: R) -> std::io::Result<Self> {
        ciborium::from_reader(reader)
            .map_err(|_| IOError::new(IOErrorKind::InvalidData, "Failed to deserialize block"))
    }

    fn save<W: std::io::Write>(&self, writer: W) -> std::io::Result<()> {
        ciborium::ser::into_writer(self, writer)
            .map_err(|_| IOError::new(IOErrorKind::InvalidData, "Failed to serialize block"))
    }
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self {
            header,
            transactions,
        }
    }
    pub fn hash(&self) -> Hash {
        Hash::hash(self)
    }

    pub fn verify_transaction(
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

    pub fn verify_coinbase_transaction(
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

    pub fn calculate_miners_fee(
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
    pub fn new(
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
    pub fn hash(&self) -> Hash {
        Hash::hash(self)
    }

    pub fn mine(&mut self, steps: usize) -> bool {
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
