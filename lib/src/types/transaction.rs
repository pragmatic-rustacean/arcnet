use serde::{Deserialize, Serialize};

use crate::{
    crypto::{PrivateKey, PublicKey, Signature},
    sha256::Hash,
};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub(super) input: Vec<TransactionInput>,
    pub(super) output: Vec<TransactionOutput>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionInput {
    pub(super) prev_transaction_output_hash: Hash,
    pub(super) signature: Signature,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionOutput {
    pub(super) value: u64,
    pub(super) unique_id: Uuid,
    pub(super) pub_key: PublicKey,
}

impl TransactionOutput {
    pub(crate) fn hash(&self) -> Hash {
        Hash::hash(&self)
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
