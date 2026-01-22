#![allow(unused)]

use anyhow::Result;
use argh::FromArgs;
use dashmap::DashMap;
use lib::types::Blockchain;
use static_init::dynamic;
use std::path::Path;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

mod handler;
mod util;

#[dynamic]
static BLOCKCHAIN: RwLock<Blockchain> = RwLock::new(Blockchain::new());
#[dynamic]
static NODES: DashMap<String, TcpStream> = DashMap::new();

#[derive(FromArgs)]
/// A toy blockchain node
struct Args {
    #[argh(option, default = "9000")]
    /// port number
    port: u16,
    #[argh(option, default = "String::from(\"./blockchain.cbor\")")]
    /// blockchain file location
    blockchain_file: String,
    #[argh(positional)]
    /// addresses to the initial nodes
    nodes: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // parse cli args.
    let args: Args = argh::from_env();
    let port = args.port;
    let blockchain_file = args.blockchain_file;
    let nodes = args.nodes;
    let addr = format!("0.0.0.0:{}", port);
    // check if the blockchain file exists.
    if Path::new(&blockchain_file).exists() {
        println!("loading blockchain");
        util::load_blockchain(&blockchain_file).await?;
        println!("populating connections");
        util::populate_connections(&nodes).await?;
        println!("total amount of known nodes: {}", nodes.len());
    } else {
        println!("blockchain file does not exist");
        if nodes.is_empty() {
            println!("No nodes provided, starting as seed node...");
        } else {
            let (longest_name, longest_count) = util::find_longest_node()
                .await
                .expect("failed to find longest node");
            // request the blockchain from the node with the longest blockchain.
            util::download_blockchain(&longest_name, longest_count)
                .await
                .expect("failed to download block");
            println!("blockchain downloaded from : {}", longest_name);

            // recalculate utxos
            {
                let mut blockchain = BLOCKCHAIN.write().await;
                blockchain.rebuild_utxos();
            }
            // try adjust difficuly
            {
                let mut blockchain = BLOCKCHAIN.write().await;
                blockchain.try_adjust_target();
            }
        }
    }

    let listener = TcpListener::bind(&addr)
        .await
        .expect(&format!("Failed to bind the address: {}", &addr));
    println!("listening on address: {}", addr);

    loop {
        let (socket, _) = listener
            .accept()
            .await
            .expect("listener failed to accept the connection");
        // start a task to periodically cleanup the mempool.
        util::cleanup().await;
        println!("saving the mess");
        util::save(blockchain_file.clone()).await;

        tokio::spawn(handler::handle_connection(socket));
    }
    Ok(())
}
