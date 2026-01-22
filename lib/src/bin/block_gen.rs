use chrono::Utc;
use lib::{
    crypto::PrivateKey,
    sha256::Hash,
    types::{Block, BlockHeader, Transaction, TransactionOutput},
    util::{MarkleRoot, Saveable},
};
use std::{env, process::exit};
use uuid::Uuid;

fn main() {
    let path = if let Some(path) = env::args().nth(1) {
        path
    } else {
        println!("usage: block_gen <block_gen>");
        exit(1)
    };
    let private_key = PrivateKey::new();
    let transactions = vec![Transaction::new(
        vec![],
        vec![TransactionOutput {
            value: lib::INITIAL_BLOCK_REWARD * 10u64.pow(8),
            unique_id: Uuid::new_v4(),
            pub_key: private_key.public_key(),
        }],
    )];
    let markle_root = MarkleRoot::calculate(&transactions[..]);
    let block = Block::new(
        BlockHeader::new(
            Utc::now(),
            0,
            Hash::zero(),
            markle_root,
            lib::MIN_TARGET,
        ),
        transactions,
    );

    block.save_to_file(path).expect("Failed to save block");
}
