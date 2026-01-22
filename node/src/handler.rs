use anyhow::Result;
use chrono::Utc;
use lib::{
    BLOCK_TRANSACTION_CAP,
    network::Message,
    sha256::Hash,
    types::{Block, BlockHeader, Transaction, TransactionOutput},
    util::MarkleRoot,
};
use tokio::net::TcpStream;
use uuid::Uuid;

pub async fn handle_connection(mut socket: TcpStream) {
    loop {
        // handle the message from the socket
        let message = match Message::receive_async(&mut socket).await {
            Ok(message) => message,
            Err(er) => {
                eprintln!("invalid message from peer: {}, closing that connection", er);
                return;
            }
        };
        match message {
            Message::UTXOs(_)
            | Message::Template(_)
            | Message::Difference(_)
            | Message::TemplateValidity(_)
            | Message::NodeList(_) => {
                println!("i'm neither a miner nor a wallet! goodbye");
                return;
            }
            Message::FetchBlock(height) => {
                let blockchain = crate::BLOCKCHAIN.read().await;
                let Some(block) = blockchain.blocks().nth(height).cloned() else {
                    return;
                };
                let message = Message::NewBlock(block);
                message
                    .send_async(&mut socket)
                    .await
                    .expect("failed to send the block for verification");
            }
            Message::FetchUTXOs(public_key) => {
                println!("received a request to fetch utxos.");
                let mut blockchain = crate::BLOCKCHAIN.read().await;
                let utxos = blockchain
                    .utxos()
                    .iter()
                    .filter(|(_, (_, tx_out))| tx_out.pub_key == public_key)
                    .map(|(_, (marked, tx_out))| (tx_out.clone(), *marked))
                    .collect::<Vec<_>>();
                let message = Message::UTXOs(utxos);
                message
                    .send_async(&mut socket)
                    .await
                    .expect("failed to send utxos for verification");
            }
            Message::SubmitTransaction(transaction) => {
                println!("submit transaction");
                let mut blockchain = crate::BLOCKCHAIN.write().await;
                if let Err(er) = blockchain.add_to_mempool(transaction.clone()) {
                    println!("transaction rejected, closing connection: {}", er);
                    return;
                }
                println!("added transaction to mempool");
                // send transaction to all node friends.
                let nodes = crate::NODES
                    .iter()
                    .map(|node| node.key().clone())
                    .collect::<Vec<_>>();
                for node in nodes {
                    println!("sending to friend: {}", node);
                    if let Some(mut stream) = crate::NODES.get_mut(&node) {
                        let message = Message::NewTransaction(transaction.clone());
                        if message.send_async(&mut *stream).await.is_err() {
                            println!("failed to send transaction: {}", node);
                        }
                    }
                }
                println!("transaction sent to friend")
            }
            Message::NewTransaction(transaction) => {
                let mut blockchain = crate::BLOCKCHAIN.write().await;
                println!("received a transaction from a friend");
                if blockchain.add_to_mempool(transaction).is_err() {
                    println!("transaction rejected, closing connection");
                    return;
                }
            }
            Message::FetchTemplate(pub_key) => {
                let mut blockchain = crate::BLOCKCHAIN.read().await;
                let mut transactions = vec![];
                // insert transaction from memepool
                transactions.extend(
                    blockchain
                        .mempool()
                        .iter()
                        .take(BLOCK_TRANSACTION_CAP)
                        .map(|(_, tx)| tx)
                        .cloned()
                        .collect::<Vec<_>>(),
                );
                // insert transaction coinbase with pubkey.
                transactions.insert(
                    0,
                    Transaction {
                        input: vec![],
                        output: vec![TransactionOutput {
                            value: 0,
                            unique_id: Uuid::new_v4(),
                            pub_key,
                        }],
                    },
                );
                let markle_root = MarkleRoot::calculate(&transactions);
                let mut block = Block::new(
                    BlockHeader {
                        timestamp: Utc::now(),
                        nonce: 0,
                        prev_block_hash: blockchain
                            .blocks()
                            .last()
                            .map(|last| last.hash())
                            .unwrap_or(Hash::zero()),
                        markle_root,
                        target: blockchain.target(),
                    },
                    transactions,
                );
                let miner_fee = match block.calculate_miners_fee(blockchain.utxos()) {
                    Ok(fee) => fee,
                    Err(err) => {
                        eprintln!("{err}");
                        return;
                    }
                };
                let reward = blockchain.calculate_block_reward();
                // update coinbase transaction with reward
                block.transactions[0].output[0].value = miner_fee + reward;
                // recalculate merkle fee.
                block.header.markle_root = MarkleRoot::calculate(&block.transactions);
                let message = Message::Template(block);
                message
                    .send_async(&mut socket)
                    .await
                    .expect("failed to send new template")
            }
            Message::ValidateTemplate(block_template) => {
                let blockchain = crate::BLOCKCHAIN.read().await;
                let status = block_template.header.prev_block_hash
                    == blockchain
                        .blocks()
                        .last()
                        .map(|lst_block| lst_block.hash())
                        .unwrap_or(Hash::zero());
                let message = Message::TemplateValidity(status);
                message
                    .send_async(&mut socket)
                    .await
                    .expect("failed to send the status for verification");
            }
            Message::SubmitTemplate(block) => {
                println!("received allegedly mined template");
                let mut blockchain = crate::BLOCKCHAIN.write().await;
                if let Err(err) = blockchain.add_block(block.clone()) {
                    println!("block rejected: {err}. closing connection");
                    return;
                }
                blockchain.rebuild_utxos();
                println!("block looks good. Broadcasting...");
                // send block to all friend nodes.
                let nodes = crate::NODES
                    .iter()
                    .map(|node| node.key().clone())
                    .collect::<Vec<_>>();

                for node in nodes {
                    if let Some(mut stream) = crate::NODES.get_mut(&node) {
                        let message = Message::NewBlock(block.clone());
                        if message.send_async(&mut *stream).await.is_err() {
                            println!("failed to send block to: {}", node);
                        }
                    }
                }
            }
            Message::DiscoverNodes => {
                let nodes = crate::NODES
                    .iter()
                    .map(|node| node.key().clone())
                    .collect::<Vec<_>>();
                let message = Message::NodeList(nodes);
                message
                    .send_async(&mut socket)
                    .await
                    .expect("failed at sending nodes for verfication");
            }
            Message::AskDifference(height) => {
                let mut blockchain = crate::BLOCKCHAIN.read().await;
                let count = blockchain.block_height() as i32 - height as i32;
                let message = Message::Difference(count);
                message
                    .send_async(&mut socket)
                    .await
                    .expect("failed at sending the block height diffrence");
            }
            Message::NewBlock(block) => {
                let mut blockchain = crate::BLOCKCHAIN.write().await;
                println!("received a new block");
                if blockchain.add_block(block).is_err() {
                    println!("block rejected")
                }
            }
        }
    }
}
