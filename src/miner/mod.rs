pub mod worker;
extern crate rand;
use rand::Rng;
use log::info;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
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
use crate::types::transaction::SignedTransaction;
use crate::types::merkle::{MerkleTree};


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
    blockchain: Arc<Mutex<Blockchain>>,
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    finished_block_chan: Sender<Block>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(blockchain:&Arc<Mutex<Blockchain>>) -> (Context, Handle, Receiver<Block>) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let (finished_block_sender, finished_block_receiver) = unbounded();

    let ctx = Context {
        blockchain: Arc::clone(blockchain),
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        finished_block_chan: finished_block_sender,
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle, finished_block_receiver)
}

#[cfg(any(test,test_utilities))]
fn test_new() -> (Context, Handle, Receiver<Block>) {
    let blockchain = Blockchain::new();
    let arc_blockchain = Arc::new(Mutex::new(blockchain));
    new(&arc_blockchain)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
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
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn miner_loop(&mut self) {
        // main mining loop
        let chain = self.blockchain.lock().unwrap();
        let mut parent = chain.tip();
        drop(chain);
        loop {
            
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    match signal {
                        ControlSignal::Exit => {
                            info!("Miner shutting down");
                            self.operating_state = OperatingState::ShutDown;
                        }
                        ControlSignal::Start(i) => {
                            info!("Miner starting in continuous mode with lambda {}", i);
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
                                info!("Miner shutting down");
                                self.operating_state = OperatingState::ShutDown;
                            }
                            ControlSignal::Start(i) => {
                                info!("Miner starting in continuous mode with lambda {}", i);
                                self.operating_state = OperatingState::Run(i);
                            }
                            ControlSignal::Update => {
                                let chain = self.blockchain.lock().unwrap();
                                parent = chain.tip();
                                drop(chain);
                            }
                        };
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            // TODO for student: actual mining, create a block
            let nonce: u32 = rand::thread_rng().gen();
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
            let vec: Vec<SignedTransaction> = Vec::new();
            let data = Data{data: vec};
            let difficulty: H256 = [255u8; 32].into();
            let merkle_tree = MerkleTree::new(&data.data);
            let merkle_root = merkle_tree.root();
            let header = Header{parent: parent, nonce, difficulty, timestamp, merkle_root};
            let block = Block{header, data: data};
            if block.hash() <= difficulty {
                // drop(chain);
                self.finished_block_chan.send(block.clone()).expect("Send finished block error");
                parent = block.hash();
            }
            // TODO for student: if block mining finished, you can have something like: self.finished_block_chan.send(block.clone()).expect("Send finished block error");

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use ntest::timeout;
    use crate::types::hash::Hashable;

    #[test]
    #[timeout(60000)]
    fn miner_three_block() {
        let (miner_ctx, miner_handle, finished_block_chan) = super::test_new();
        miner_ctx.start();
        miner_handle.start(0);
        let mut block_prev = finished_block_chan.recv().unwrap();
        print!("Block_prev: {:?}", block_prev);
        for _ in 0..2 {
            let block_next = finished_block_chan.recv().unwrap();
            print!("Block_next: {:?}", block_next);
            assert_eq!(block_prev.hash(), block_next.get_parent());
            block_prev = block_next;
        }
    }
    // thread 'miner' panicked at 'Send finished block error: "SendError(..)"', src\miner\mod.rs:160:62
    // thread 'miner::test::miner_three_block' panicked at 'timeout: the function call took 2 ms. Max time 60000 ms', src\miner\mod.rs:183:5
    // I'm updating the parent through &chain.tip(), however I'm not sure that the block ever get's inserted, as we don't have an instance of Worker
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST