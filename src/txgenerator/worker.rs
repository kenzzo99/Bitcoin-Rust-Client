use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::{debug, info};
// use core::num::flt2dec::Sign;
use std::sync::{Arc, Mutex};
use crate::blockchain::Blockchain;
use crate::types::block::Block;
use crate::network::server::Handle as ServerHandle;
use crate::network::message::Message;
use crate::types::hash::{Hashable, H256};
use crate::types::transaction::{SignedTransaction};
use std::thread;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Worker {
    mem_pool: Arc<Mutex<HashMap<H256, SignedTransaction>>>,
    server: ServerHandle,
    tx_chan: Receiver<SignedTransaction>,
}

impl Worker {
    pub fn new(
        server: &ServerHandle,
        tx_chan: Receiver<SignedTransaction>,
        mem_pool: &Arc<Mutex<HashMap<H256, SignedTransaction>>>,
    ) -> Self {
        Self {
            server: server.clone(),
            tx_chan,
            mem_pool: Arc::clone(&mem_pool),
        }
    }

    pub fn start(self) {
        thread::Builder::new()
            .name("txgen-worker".to_string())
            .spawn(move || {
                self.worker_loop();
            })
            .unwrap();
        info!("Transaction generator initialized into paused mode");
    }

    fn worker_loop(&self) {
        loop {
            let transaction: SignedTransaction = self.tx_chan.recv().expect("Receive finished transaction error");
            // TODO for student: insert this finished block to blockchain, and broadcast this block hash
            let mut mem_pool = self.mem_pool.lock().unwrap();
            // insert tx to mempool
            mem_pool.insert(transaction.clone().hash(), transaction.clone());
            // broadcast the hash of the new block
            let mut vec = Vec::new();
            vec.push(transaction.clone().hash());
            self.server.broadcast(Message::NewTransactionHashes(vec));
            drop(mem_pool);
        }   
    }
}