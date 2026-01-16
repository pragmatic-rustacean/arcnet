#![allow(unused)]
use std::fmt::Display;

use serde::{Deserialize, Serialize};
use sha256::digest;

use super::U256;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub(crate) struct Hash(U256);

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl Hash {
    pub(crate) fn hash<T: Serialize>(data: &T) -> Self {
        let mut serialized: Vec<u8> = Vec::new();
        if let Err(err) = ciborium::into_writer(&data, &mut serialized) {
            panic!(
                "Failed to serialize the data {:#?}\n This should not happen",
                err
            );
        }

        let hash = digest(&serialized);
        let hash_bytes = hex::decode(hash).expect("Failed to decode the data..(@sha256.rs/ln21)");
        let hash_array: [u8; 32] = hash_bytes
            .as_slice()
            .try_into()
            .expect("Failed to convert a vector into a [u8;32]");

        Self(U256::from_big_endian(&hash_array))
    }
    pub(self) fn check_matches(&self, target: U256) {
        self.0 <= target;
    }
    pub(crate) fn zero_hash(&self) -> Self {
        Self(U256::zero())
    }
}
