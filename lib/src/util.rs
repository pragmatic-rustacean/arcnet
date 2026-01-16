#![allow(unused)]

use crate::{U256, sha256::Hash, types::Transaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct MarkleRoot(Hash);
impl MarkleRoot {
    pub(crate) fn calculate(transactions: &[Transaction]) -> Self {
        let mut layer = vec![];
        for transact in transactions {
            layer.push(Hash::hash(transact));
        }

        while layer.len() > 1 {
            let mut new_layer = vec![];
            for pair in layer.chunks(2) {
                let left = pair[0];
                let right = pair.get(1).unwrap_or(&left);
                new_layer.push(Hash::hash(&[left, *right]));
            }
            layer = new_layer;
        }
        Self(layer[0])
    }
}
