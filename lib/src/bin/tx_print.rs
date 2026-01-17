use lib::{types::Transaction, util::Saveable};
use std::{env, fs::File, process::exit};

fn main() {
    let path = if let Some(path) = env::args().nth(1) {
        path
    } else {
        println!("Usage: transaction_print <transaction_file>");
        exit(1);
    };
    if let Ok(file) = File::open(path) {
        let transaction = Transaction::load(file).expect("Failed to load transaction");
        println!("{:#?}", transaction);
    }
}
