#![allow(unused)]
use anyhow::{Result, anyhow};
use lib::{crypto::PublicKey, network::Message, types::Block};
use std::{
    sync::{Arc, atomic::AtomicBool},
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::{net::TcpStream, sync::Mutex, time::interval};

pub struct Miner {
    pub_key: PublicKey,
    stream: Mutex<TcpStream>,
    current_template: Arc<std::sync::Mutex<Option<Block>>>,
    mining: Arc<AtomicBool>,
    mined_block_sender: flume::Sender<Block>,
    mined_block_receiver: flume::Receiver<Block>,
}

impl Miner {
    pub async fn new(addr: String, pub_key: PublicKey) -> Result<Self> {
        let stream = TcpStream::connect(&addr).await?;
        let (mined_block_sender, mined_block_receiver) = flume::unbounded();

        Ok(Self {
            pub_key,
            stream: Mutex::new(stream),
            current_template: Arc::new(std::sync::Mutex::new(None)),
            mining: Arc::new(AtomicBool::new(false)),
            mined_block_sender,
            mined_block_receiver,
        })
    }
    pub async fn run(&self) -> Result<()> {
        self.spawn_mining_thread();
        let mut template_interval = interval(Duration::from_millis(5));
        loop {
            let recv_clone = self.mined_block_receiver.clone();
            tokio::select! {
                _= template_interval.tick() =>  {
                self.fetch_and_validate_template().await?
              }
              Ok(mined_block) = recv_clone.recv_async() => {
                self.submit_block(mined_block).await?
              }
            }
        }
    }
    fn spawn_mining_thread(&self) -> JoinHandle<()> {
        let template = self.current_template.clone();
        let mining = self.mining.clone();
        let sender = self.mined_block_sender.clone();

        thread::spawn(move || {
            loop {
                if mining.load(std::sync::atomic::Ordering::Relaxed) {
                    if let Some(mut block) = template.lock().unwrap().clone() {
                        println!("Mining block with target: {}", block.header.target);
                        if block.header.mine(20_000_000) {
                            println!("Block mined: {}", block.hash());
                        }
                        sender.send(block).expect("Failed to send mined block");
                        mining.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
            thread::yield_now()
        })
    }
    async fn fetch_and_validate_template(&self) -> Result<()> {
        if !self.mining.load(std::sync::atomic::Ordering::Relaxed) {
            self.fetch_template().await
        } else {
            self.validate_template().await
        }
    }
    async fn fetch_template(&self) -> Result<()> {
        println!("Fetching new template!!!");
        let message = Message::FetchTemplate(self.pub_key.clone());
        let mut stream_lock = self.stream.lock().await;
        let _ = message.send_async(&mut *stream_lock).await;
        drop(stream_lock);
        let mut stream_lock = self.stream.lock().await;
        match Message::receive_async(&mut *stream_lock).await {
            Ok(message) => {
                match message {
                    Message::Template(template) => {
                        drop(stream_lock);
                        println!(
                            "Received new template with target: {}",
                            template.header.target
                        );
                        *self.current_template.lock().expect(
                            "Failed to update the current template @miner/main.rs/line 106",
                        ) = Some(template);
                        self.mining
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                        Ok(())
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }
            Err(err) => Err(anyhow!(
                "Unexpect message received when fetching templates. {:#?}",
                err
            )),
        }
    }
    async fn validate_template(&self) -> Result<()> {
        if let Some(template) = self.current_template.lock().unwrap().clone() {
            let message = Message::ValidateTemplate(template);
            let mut stream_lock = self.stream.lock().await;
            message.send_async(&mut *stream_lock).await?;
            drop(stream_lock);

            let mut stream_lock = self.stream.lock().await;
            match Message::receive_async(&mut *stream_lock).await {
                Ok(message) => match message {
                    Message::TemplateValidity(valid) => {
                        drop(stream_lock);
                        if !valid {
                            eprintln!("Current template is no longer valid");
                            self.mining
                                .store(false, std::sync::atomic::Ordering::Relaxed);
                        } else {
                            println!("Current template is still valid.");
                        }

                        Ok(())
                    }
                    _ => {
                        unreachable!();
                    }
                },
                Err(_) => Err(anyhow!(
                    "Unexpected message received when validating the template"
                )),
            }
        } else {
            Ok(())
        }
    }
    async fn submit_block(&self, block: Block) -> Result<()> {
        println!("Submitting mined block");
        let message = Message::SubmitTemplate(block);
        let mut stream_lock = self.stream.lock().await;
        message.send_async(&mut *stream_lock).await?;
        self.mining
            .store(false, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }
}
