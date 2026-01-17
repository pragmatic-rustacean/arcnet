use lib::{types::Block, util::Saveable};
use std::{env, fs::File, process::exit};

fn main() {
    let path = if let Some(path) = env::args().nth(1) {
        path
    } else {
        println!("Usage: block_print <block_file>");
        exit(1);
    };
    if let Ok(file) = File::open(path) {
        let block = Block::load(file).expect("Failed to load block");
        println!("{:#?}", block);
    }
}
