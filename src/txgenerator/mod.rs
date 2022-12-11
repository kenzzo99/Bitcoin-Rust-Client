pub mod worker;
extern crate rand;
use rand::Rng;
use log::info;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::collections::HashMap;
use std::ops::Deref;
use std::time;
use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use crate::blockchain::Blockchain;
use crate::types::block::{Block, Data, Header};
use crate::types::hash::{H256, Hashable};
// use crate::types::merkle::verify;
use crate::types::transaction::{SignedTransaction, Transaction, sign, verify}; // state later 
use crate::types::merkle::{MerkleTree};
use crate::types::address::{Address};
use crate::types::key_pair;
use ring::signature::{self, KeyPair, Ed25519KeyPair, Signature};


enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Update, // update the block in mining, it may due to new blockchain tip or new transaction
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    chain: Arc<Mutex<Blockchain>>,
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    tx_chan: Sender<SignedTransaction>,
    // state later
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(blockchain:&Arc<Mutex<Blockchain>>, mem_pool: &Arc<Mutex<HashMap<H256, SignedTransaction>>>) -> (Context, Handle, Receiver<SignedTransaction>) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let (tx_chan_sender, tx_chan_receiver) = unbounded();

    let ctx = Context {
        chain: Arc::clone(blockchain),
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        tx_chan: tx_chan_sender,
        // state later
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };



    (ctx, handle, tx_chan_receiver)
}


impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, theta: u64) {
        self.control_chan
            .send(ControlSignal::Start(theta))
            .unwrap();
    }

    pub fn update(&self) {
        self.control_chan.send(ControlSignal::Update).unwrap();
    }
}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.generator_loop();
            })
            .unwrap();
        info!("Tx generator initialized into paused mode");
    }

    fn generator_loop(&mut self) {
        // main mining loop
        let chain = self.chain.lock().unwrap();
        let mut parent = chain.tip();
        drop(chain);
        loop {
            
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    match signal {
                        ControlSignal::Exit => {
                            info!("Transaction generator shutting down");
                            self.operating_state = OperatingState::ShutDown;
                        }
                        ControlSignal::Start(i) => {
                            info!("Transaction generator starting in continuous mode with theta {}", i);
                            self.operating_state = OperatingState::Run(i);
                        }
                        ControlSignal::Update => {
                            // in paused state, don't need to update
                        }
                    };
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        match signal {
                            ControlSignal::Exit => {
                                info!("Transaction generator shutting down");
                                self.operating_state = OperatingState::ShutDown;
                            }
                            ControlSignal::Start(i) => {
                                info!("Transaction generator starting in continuous mode with lambda {}", i);
                                self.operating_state = OperatingState::Run(i);
                            }
                            ControlSignal::Update => {
                            }
                        };
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Transaction generator control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }
            // get wallets later
            // state later

            // generate addresses
            let mut rng = rand::thread_rng();
            let key1 = key_pair::random();
            let key2 = key_pair::random();
            let public_key1 = key1.public_key();
            let public_key2 = key2.public_key();
            let mut sender: Address = Address::from_public_key_bytes(public_key1.as_ref());
            let mut receiver: Address = Address::from_public_key_bytes(public_key2.as_ref());
            let mut amount = 0;
            let mut nonce = 0;
            // generate the transaction
            let transaction = Transaction{
                sender, 
                receiver, 
                value: amount,  
                nonce};
            let signature = sign(&transaction, &key1);
            let stx = SignedTransaction {
                transaction,
                signature: signature.as_ref().to_vec(),
                pubkey: key1.public_key().as_ref().to_vec(),
            };
            self.tx_chan.send(stx.clone()).expect("Send finished transaction error");

            // TODO for student: if block mining finished, you can have something like: self.finished_block_chan.send(block.clone()).expect("Send finished block error");

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    // CHANGE TRANSACTION GENERATION SPEED HERE 
                    let interval = time::Duration::from_micros((i * 30000) as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}
