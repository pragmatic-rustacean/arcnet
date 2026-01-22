use anyhow::{Context, Result};
use lib::{network::Message, types::Blockchain, util::Saveable};
use std::net::SocketAddr;
use tokio::{net::TcpStream, time};

pub async fn load_blockchain(path: &str) -> Result<()> {
    println!("blockchain file exists. Loading...");
    let new_blockchain =
        Blockchain::load_from_file(path).expect("Failed to load blockchain file. @node/util.rs 8");
    println!("blockchain loaded");
    let mut blockchain = crate::BLOCKCHAIN.write().await;
    *blockchain = new_blockchain;
    println!("rebuilding utxos");
    blockchain.rebuild_utxos();
    println!("utxos rebuilt");
    println!("checking whether the target needs to be adjusted...");
    println!("current target: {:#?}", blockchain.target());
    blockchain.try_adjust_target();
    println!("new target: {:#?}", blockchain.target());
    println!("Initialization complete");

    Ok(())
}

pub async fn populate_connections(nodes: &[String]) -> Result<()> {
    println!("trying to connect to other nodes");
    for node in nodes {
        println!("connecting to node: {}", node);

        // `TcpStream::connect` expects a valid socket address like "127.0.0.1:9000".
        // If CLI args are mis-specified (e.g. "./blockchain.cbor" passed as a positional "node"),
        // skip it rather than panic.
        let _addr: SocketAddr = match node.parse() {
            Ok(a) => a,
            Err(_) => {
                println!("skipping invalid node address (expected host:port): {}", node);
                continue;
            }
        };

        let mut stream = TcpStream::connect(node)
            .await
            .with_context(|| format!("Failed connecting to node {node} (@node/util.rs populate_connections)"))?;
        let message = Message::DiscoverNodes;
        message.send_async(&mut stream).await?;
        println!("sent discover nodes to {}", node);
        let message = Message::receive_async(&mut stream).await?;
        match message {
            Message::NodeList(child_nodes) => {
                println!("received nodelist from: {}", node);
                for c_node in child_nodes {
                    println!("adding node {}", c_node);
                    let _c_addr: SocketAddr = match c_node.parse() {
                        Ok(a) => a,
                        Err(_) => {
                            println!(
                                "skipping invalid child node address (expected host:port): {}",
                                c_node
                            );
                            continue;
                        }
                    };
                    let new_stream = TcpStream::connect(&c_node)
                        .await
                        .expect("Failed to connect with child node");
                    crate::NODES.insert(c_node, new_stream);
                }
            }
            _ => {
                println!("unexpected results returned: {}", node);
            }
        }
        crate::NODES.insert(node.clone(), stream);
    }

    Ok(())
}

pub async fn find_longest_node() -> Result<(String, u32)> {
    println!("finding nodes with highest blockchain length");
    let mut longest_name = String::new();
    let mut longest_count = 0;
    let all_nodes = crate::NODES
        .iter()
        .map(|node| node.key().clone())
        .collect::<Vec<_>>();

    for node in all_nodes {
        println!("asking {} for blockchain length", node);
        let mut stream = crate::NODES.get_mut(&node).context("no node")?;
        let message = Message::AskDifference(0);
        message
            .send_async(&mut *stream)
            .await
            .expect("failed to send node stream");
        println!("sent AskDifference to: {}", node);
        let message = Message::receive_async(&mut *stream)
            .await
            .expect("failed message received");
        match message {
            Message::Difference(count) => {
                println!("received difference from node: {}", node);
                if count > longest_count {
                    println!("new longest blockchain: {} blocks from {node}", count);
                    longest_count = count;
                    longest_name = node;
                }
            }
            msg => {
                println!("unexpected mesage received from {}: {:#?}", node, msg)
            }
        }
    }

    Ok((longest_name, longest_count as u32))
}

pub async fn download_blockchain(node: &str, count: u32) -> Result<()> {
    let mut stream = crate::NODES.get_mut(node).expect("no nodes found");
    for bk in 0..count as usize {
        let message = Message::FetchBlock(bk);
        message
            .send_async(&mut *stream)
            .await
            .expect("failed to send the stream for verifcation");
        let message = Message::receive_async(&mut *stream)
            .await
            .expect("failed to receive verified stream");
        match message {
            Message::NewBlock(block) => {
                let mut new_block = crate::BLOCKCHAIN.write().await;
                new_block.add_block(block)?;
            }
            msg => {
                println!("unexpected message received from {}: {:#?}", node, msg)
            }
        }
    }
    Ok(())
}

pub async fn cleanup() {
    let mut interval = time::interval(time::Duration::from_secs(30));
    loop {
        interval.tick().await;
        println!("cleaning the mempool of old transactions...");
        let mut blockchain = crate::BLOCKCHAIN.write().await;

        blockchain.clean_mempool();
    }
}

pub async fn save(name: String) {
    let mut interval = time::interval(time::Duration::from_secs(30));
    loop {
        interval.tick().await;
        println!("saving the blockchain to drive...");
        let mut blockchain = crate::BLOCKCHAIN.write().await;
        blockchain
            .save_to_file(name.clone())
            .expect("failed to save blockchain to drive");
    }
}
