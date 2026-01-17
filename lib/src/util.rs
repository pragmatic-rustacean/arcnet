use std::{
    fs::File,
    io::{Read, Result as IOResult, Write},
    path::Path,
};

use crate::{U256, sha256::Hash, types::Transaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkleRoot(Hash);
impl MarkleRoot {
    pub fn calculate(transactions: &[Transaction]) -> Self {
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

pub trait Saveable
where
    Self: Sized,
{
    fn load<R: Read>(reader: R) -> IOResult<Self>;
    fn save<W: Write>(&self, writer: W) -> IOResult<()>;

    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> IOResult<()> {
        let file = File::create(path).expect("Failed to create a file @util/line 40");
        self.save(file)
    }
    fn load_from_file<P: AsRef<Path> + Read>(&self, path: P) -> IOResult<Self> {
        let file =
            File::open(&path).expect("Failed to open the file, maybe the file doesn't exist.");
        Self::load(path)
    }
}
