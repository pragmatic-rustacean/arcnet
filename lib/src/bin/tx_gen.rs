use lib::{
    crypto::PrivateKey,
    types::{Transaction, TransactionOutput},
    util::Saveable,
};
use std::{env, process::exit};
use uuid::Uuid;

fn main() {
    let path = if let Some(path) = env::args().nth(1) {
        path
    } else {
        println!("usage: tx_gen <tx_gen>");
        exit(1)
    };
    let private_key = PrivateKey::new();
    let transaction = Transaction::new(
        vec![],
        vec![TransactionOutput {
            value: lib::INITIAL_BLOCK_REWARD * 10u64.pow(8),
            unique_id: Uuid::new_v4(),
            pub_key: private_key.public_key(),
        }],
    );

    transaction
        .save_to_file(path)
        .expect("Failed to save transaction")
}
