use super::message::Message;
use super::peer;
use super::server::Handle as ServerHandle;
use crate::types::hash::{H256, Hashable};
use crate::types::block::Block;
use crate::Blockchain;
use std::sync::{Arc, Mutex};

use log::{debug, warn, error};

use std::thread;

#[cfg(any(test,test_utilities))]
use super::peer::TestReceiver as PeerTestReceiver;
#[cfg(any(test,test_utilities))]
use super::server::TestReceiver as ServerTestReceiver;
#[derive(Clone)]
pub struct Worker {
    msg_chan: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    chain: Arc<Mutex<Blockchain>>
}


impl Worker {
    pub fn new(
        num_worker: usize,
        msg_src: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
        server: &ServerHandle,
        chain: &Arc<Mutex<Blockchain>>
    ) -> Self {
        Self {
            msg_chan: msg_src,
            num_worker,
            server: server.clone(),
            chain: Arc::clone(chain)
        }
    }

    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {
        let mut orphan_buffer: Vec<Block> = Vec::new();
        loop {
            let result = smol::block_on(self.msg_chan.recv());
            if let Err(e) = result {
                error!("network worker terminated {}", e);
                break;
            }
            let msg = result.unwrap();
            let (msg, mut peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }

                // Takes the vec of block hashes, checks if they are already in the chain
                // For those that are not, it requests those blocks so it can add them 
                // to the chain
                Message::NewBlockHashes(hash_vec) => {
                    println!("Received hashes: {:?}",hash_vec);
                    // get the lock
                    let chain = self.chain.lock().unwrap();
                    // create a vector for new hashes
                    let mut new_hashes: Vec<H256> = Vec::new();

                    // iterate through hash_vec and add all hashes that are not 
                    // already in the chain
                    for i in 0..hash_vec.len(){
                        if !(chain.blocks.contains_key(&hash_vec[i])){
                            new_hashes.push(hash_vec[i]);
                        }
                    }

                    // send any new hashes that weren't already in the chain by GetBlocks msg
                    if new_hashes.len() != 0 {
                        peer.write(Message::GetBlocks(new_hashes));
                    }
                    // drop the lock on the chain
                    // drop(chain);
                }

                Message::GetBlocks(block_hashes) => {
                    // get the lock
                    let chain = self.chain.lock().unwrap();

                    // vec to store requested blocks
                    let mut blocks = Vec::new();

                    // iterate through requested blocks 
                    for i in 0..block_hashes.len() {
                        // check if the block is in the chain
                        if chain.blocks.contains_key(&block_hashes[i]) {
                            blocks.push(chain.blocks.get(&block_hashes[i]).unwrap().clone());
                        }
                    }
                    println!("Blocks len: {:?}", blocks.len());
                    // send the blocks if any are in the chain
                    if blocks.len() != 0 {
                        println!("Requesting blocks:{:?}",blocks);
                        peer.write(Message::Blocks(blocks));
                    }
                    // drop the lock on the chain
                    // drop(chain); 
                }

                // handles received blocks
                Message::Blocks(blocks) =>{
                    let mut chain = self.chain.lock().unwrap();
                    // vec of new blocks
                    let mut new_blocks = Vec::new();
                    // iterate over received blocks
                    for i in 0..blocks.len() {                        
                        // check if the chain already contains the block. If not, proceed with validity checks
                        if !(chain.blocks.contains_key(&blocks[i].hash())) {
                            println!("Check 1 Passed: Block is not already in chain.");
                            // check if the chain contains the blocks parent
                            if chain.blocks.contains_key(&blocks[i].get_parent()) {
                                println!("Check 2 Passed: Chain does contain block's parent");
                                // get parents/current difficulty
                                let difficulty = chain.blocks.get(&blocks[i].get_parent()).unwrap().get_difficulty();
                                // println!("difficulty = {:?}", difficulty);
                                // perform PoW validity check. If passed, insert block to chain and add it to new_blocks
                                if blocks[i].hash() <= difficulty {
                                    println!("Check 3 Passed: PoW validity check for difficulty={:?}", difficulty);
                                    chain.insert(&blocks[i]); // (&blocks[i].clone());
                                    new_blocks.push(blocks[i].hash());
                                    
                                    // check if the processed block is the parent of any of the blocks in orphan_buffer
                                    // if so, process the orphan block
                                    for j in 0..orphan_buffer.len() {
                                        if &blocks[i].hash() == &orphan_buffer[j].get_parent() {
                                            let mut orphans: Vec<Block> = Vec::new();
                                            orphans.push(orphan_buffer[j].clone());
                                            orphan_buffer.remove(j);
                                            self.server.broadcast(Message::Blocks(orphans));
                                        }
                                    }
                                }
                            
                            }
                            else {
                                orphan_buffer.push(blocks[i].clone());
                                let mut orphans: Vec<H256> = Vec::new();
                                orphans.push(blocks[i].get_parent());
                                self.server.broadcast(Message::GetBlocks(orphans));
                            }
                        }

                    }
                    // broadcast all inserted blocks
                    if new_blocks.len() != 0 {
                        println!("Broadcasting the new blocks in Blocks()");
                        self.server.broadcast(Message::NewBlockHashes(new_blocks));
                    }
                    
                    // drop the lock
                    // drop(chain); 
                }

                Message::NewTransactionHashes(transaction_hashes) => {
                    unimplemented!()
                }
                _ => unimplemented!()
            }
        }
    }
}

#[cfg(any(test,test_utilities))]
struct TestMsgSender {
    s: smol::channel::Sender<(Vec<u8>, peer::Handle)>
}
#[cfg(any(test,test_utilities))]
impl TestMsgSender {
    fn new() -> (TestMsgSender, smol::channel::Receiver<(Vec<u8>, peer::Handle)>) {
        let (s,r) = smol::channel::unbounded();
        (TestMsgSender {s}, r)
    }

    fn send(&self, msg: Message) -> PeerTestReceiver {
        let bytes = bincode::serialize(&msg).unwrap();
        let (handle, r) = peer::Handle::test_handle();
        smol::block_on(self.s.send((bytes, handle))).unwrap();
        r
    }
}
#[cfg(any(test,test_utilities))]
/// returns two structs used by tests, and an ordered vector of hashes of all blocks in the blockchain
fn generate_test_worker_and_start() -> (TestMsgSender, ServerTestReceiver, Vec<H256>) {
    let (server, server_receiver) = ServerHandle::new_for_test();
    
    let mut blockchain = Blockchain::new();
    let longest_chain = blockchain.all_blocks_in_longest_chain();
    let blockchain = Arc::new(Mutex::new(blockchain));
    let (test_msg_sender, msg_chan) = TestMsgSender::new();
    let worker = Worker::new(1, msg_chan, &server, &blockchain);
    worker.start(); 
    (test_msg_sender, server_receiver, longest_chain)
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use ntest::timeout;
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;

    use super::super::message::Message;
    use super::generate_test_worker_and_start;

    #[test]
    #[timeout(60000)]
    fn reply_new_block_hashes() {
        let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
        let random_block = generate_random_block(v.last().unwrap());
        let mut peer_receiver = test_msg_sender.send(Message::NewBlockHashes(vec![random_block.hash()]));
        let reply = peer_receiver.recv();
        if let Message::GetBlocks(v) = reply {
            assert_eq!(v, vec![random_block.hash()]);
        } else {
            panic!();
        }
    }
    #[test]
    #[timeout(60000)]
    fn reply_get_blocks() {
        let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
        let h = v.last().unwrap().clone();
        let mut peer_receiver = test_msg_sender.send(Message::GetBlocks(vec![h.clone()]));
        let reply = peer_receiver.recv();
        if let Message::Blocks(v) = reply {
            assert_eq!(1, v.len());
            assert_eq!(h, v[0].hash())
        } else {
            panic!();
        }
    }
    #[test]
    #[timeout(60000)]
    fn reply_blocks() {
        let (test_msg_sender, server_receiver, v) = generate_test_worker_and_start();
        let random_block = generate_random_block(v.last().unwrap());
        let mut _peer_receiver = test_msg_sender.send(Message::Blocks(vec![random_block.clone()]));
        let reply = server_receiver.recv().unwrap();
        if let Message::NewBlockHashes(v) = reply {
            assert_eq!(v, vec![random_block.hash()]);
        } else {
            panic!();
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST