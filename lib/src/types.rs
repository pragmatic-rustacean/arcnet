#![allow(unused)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    crypto::{PublicKey, Signature},
    sha256::Hash,
    util::MarkleRoot,
};

use super::U256;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Blockchain {
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
        Self { blocks: Vec::new() }
    }
    pub(crate) fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
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
