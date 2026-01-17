#![allow(unused)]
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ArcNetError {
    #[error("Invalid transaction")]
    InvalidTransaction,
    #[error("Invalid block")]
    InvalidBlock,
    #[error("Invalid block header")]
    InvalidBlockHeader,
    #[error("Invalid transaction input")]
    InvalidTransactionInput,
    #[error("Invalid transaction output")]
    InvalidTransactionOutput,
    #[error("Invalid merkle root")]
    InvalidMerkleError,
    #[error("Invalid hash")]
    InvalidHash,
    #[error("Invalid private key")]
    InvalidPrivateKey,
    #[error("Invalid public key")]
    InvalidPublicKey,
    #[error("Invalid signature")]
    InvalidSignature,
}

pub(crate) type Result<T> = std::result::Result<T, ArcNetError>;
