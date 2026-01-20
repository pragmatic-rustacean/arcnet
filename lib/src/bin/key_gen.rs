use std::env;

use lib::{crypto::PrivateKey, util::Saveable};

fn main() {
    let name = env::args().nth(1).expect("Please provide a name");
    let private_key = PrivateKey::new();
    let public_key = private_key.public_key();
    let public_key_file_name = name.clone() + ".pub.pem";
    let private_key_file_name = name + ".priv.cbor";
    
    private_key
        .save_to_file(&private_key_file_name)
        .expect("Failed to save private key");
    public_key
        .save_to_file(&public_key_file_name)
        .expect("Failed to save public key")
}
