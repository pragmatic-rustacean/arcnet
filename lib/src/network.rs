use std::io::{Error as IOError, Read, Write};

use crate::{
    crypto::PublicKey,
    types::{Block, Transaction, TransactionOutput},
};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Message {
    // Fetch all UTXO's belonging to a specific public key.
    FetchUTXOs(PublicKey),
    // UTXOs belonging to a certail public key. Bool determines if it is marked
    UTXOs(Vec<(TransactionOutput, bool)>),
    // Send a transaction over a network.
    SubmitTransaction(Transaction),
    // Broadcast a transaction to other nodes.
    NewTransaction(Transaction),
    // Ask the node to prepare an optimal block template with coinbase transaction paying the specified public key
    FetchTemplate(PublicKey),
    // The Template
    Template(Block),
    // Ask the node to validate the block template. This is to prevent a node from mining an invalid block.
    ValidateTemplate(Block),
    // If template is valid
    TemplateValidity(bool),
    // Submit a mined block to node.
    SubmitTemplate(Block),
    // Ask a node to report to all other nodes it knows about.
    DiscoverNodes,
    // Response from Discover node.
    NodeList(Vec<String>),
    // Ask a node what is the highest block it knows compared to the local blockchain.
    AskDifference(u32),
    // Response from Ask difference,
    Difference(i32),
    // Ask a node to send a block with specified height.
    FetchBlock(usize),
    // Bloadcast the new block to the other nodes
    NewBlock(Block),
}

impl Message {
    pub fn encode(&self) -> Result<Vec<u8>, ciborium::ser::Error<IOError>> {
        let mut bytes = Vec::new();
        let _ = ciborium::into_writer(&self, &mut bytes);

        Ok(bytes)
    }
    pub fn decode(reader: &[u8]) -> Result<Self, ciborium::de::Error<IOError>> {
        ciborium::from_reader(reader)
    }
    pub fn send(&self, stream: &mut impl Write) -> Result<(), ciborium::ser::Error<IOError>> {
        let bytes = self.encode()?;
        let len = bytes.len() as u64;
        stream.write_all(&len.to_be_bytes())?;
        stream.write_all(&bytes)?;

        Ok(())
    }
    pub fn receive(stream: &mut impl Read) -> Result<Self, ciborium::de::Error<IOError>> {
        let mut len_bytes = [0u8; 8];
        stream.read_exact(&mut len_bytes)?;
        let len = u64::from_be_bytes(len_bytes) as usize;
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data)?;
        Self::decode(&data)
    }
    pub async fn send_async(
        &self,
        stream: &mut (impl AsyncWrite + Unpin),
    ) -> Result<(), ciborium::ser::Error<IOError>> {
        let bytes = self.encode()?;
        let byte_len = bytes.len() as u64;
        stream.write_all(&byte_len.to_be_bytes()).await?;
        stream.write_all(&bytes).await?;

        Ok(())
    }
    pub async fn receive_async(
        stream: &mut (impl AsyncRead + Unpin),
    ) -> Result<Self, ciborium::de::Error<IOError>> {
        let mut bytes_len = [0u8; 8];
        stream.read_exact(&mut bytes_len).await?;
        let len = u64::from_be_bytes(bytes_len) as usize;
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data).await?;

        Self::decode(&data)
    }
}
