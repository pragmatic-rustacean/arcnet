use lib::{crypto::PublicKey, util::Saveable};
use std::{env, process::exit};

mod handler;
mod util;

fn usage() -> ! {
    eprintln!(
        "usage: {} <address> <public-key-file>",
        env::args().next().unwrap()
    );
    exit(1)
}

fn main() {
    let address = match env::args().nth(1) {
        Some(addr) => addr,
        None => usage(),
    };
    let public_key_file = match env::args().nth(2) {
        Some(pkey) => pkey,
        None => usage(),
    };

    let Ok(public_key) = PublicKey::load_from_file(&public_key_file) else {
        eprintln!("Error reading public key from file: {}", &public_key_file);
        exit(1)
    };

    println!(
        "Connecting to address {address} to mine  whith {:#?}",
        public_key
    );
}
