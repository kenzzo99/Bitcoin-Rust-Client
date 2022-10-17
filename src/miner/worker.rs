use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::{debug, info};
use std::sync::{Arc, Mutex};
use crate::blockchain::Blockchain;
use crate::types::block::Block;
use crate::network::server::Handle as ServerHandle;
use std::thread;

#[derive(Clone)]
pub struct Worker {
    blockchain: Arc<Mutex<Blockchain>>,
    server: ServerHandle,
    finished_block_chan: Receiver<Block>,
}

impl Worker {
    pub fn new(
        server: &ServerHandle,
        finished_block_chan: Receiver<Block>,
        blockchain: &Arc<Mutex<Blockchain>>,
    ) -> Self {
        Self {
            blockchain: Arc::clone(blockchain),
            server: server.clone(),
            finished_block_chan,
        }
    }

    pub fn start(self) {
        thread::Builder::new()
            .name("miner-worker".to_string())
            .spawn(move || {
                self.worker_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn worker_loop(&self) {
        loop {
            let _block = self.finished_block_chan.recv().expect("Receive finished block error");
            // TODO for student: insert this finished block to blockchain, and broadcast this block hash
            let mut chain = self.blockchain.lock().unwrap();
            chain.insert(&_block);
            drop(chain);
        }
    }
}