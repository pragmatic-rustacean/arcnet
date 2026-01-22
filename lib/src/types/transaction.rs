use serde::{Deserialize, Serialize};
use std::io::{Error as IOError, ErrorKind as IOErrorKind};

use crate::{
    crypto::{PublicKey, Signature},
    sha256::Hash,
    util::Saveable,
};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub input: Vec<TransactionInput>,
    pub output: Vec<TransactionOutput>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionInput {
    pub prev_transaction_output_hash: Hash,
    pub signature: Signature,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionOutput {
    pub value: u64,
    pub unique_id: Uuid,
    pub pub_key: PublicKey,
}

impl TransactionOutput {
    pub fn hash(&self) -> Hash {
        Hash::hash(&self)
    }
}

impl Transaction {
    pub fn new(input: Vec<TransactionInput>, output: Vec<TransactionOutput>) -> Self {
        Self { input, output }
    }
    pub fn hash(&self) -> Hash {
        Hash::hash(self)
    }
}

/// Save and load expect CBOR from ciborium as format
impl Saveable for Transaction {
    fn load<R: std::io::Read>(reader: R) -> std::io::Result<Self> {
        ciborium::from_reader(reader).map_err(|_| {
            IOError::new(
                IOErrorKind::InvalidData,
                "Failed to deserialize transaction",
            )
        })
    }

    fn save<W: std::io::Write>(&self, writer: W) -> std::io::Result<()> {
        ciborium::ser::into_writer(self, writer)
            .map_err(|_| IOError::new(IOErrorKind::InvalidData, "Failed to serialize transaction"))
    }
}
