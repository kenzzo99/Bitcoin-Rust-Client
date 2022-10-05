extern crate rand;
use rand::Rng;
use std::hash::Hash;
use ring::digest;
use serde::{Serialize, Deserialize};
use crate::types::hash::{H256, Hashable};
use crate::types::merkle::{MerkleTree};
use bincode::{serialize, deserialize};
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use super::transaction::SignedTransaction;

// struct for holding data to be recorded in the block
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Data {
    pub data: Vec<SignedTransaction>
}

impl Hashable for Data {
    fn hash(&self) -> H256 {
    let mut ctx = digest::Context::new(&digest::SHA256);
    for transaction in self.data.clone() {
        // convert transaction to Vec<u8>
        let encoded: Vec<u8>  = serialize(&transaction).unwrap();
        // convert encoded to &[u8]
        let bytes = &encoded[..];
        ctx.update(bytes);
    };
    let temp: H256 = ctx.finish().into();
    temp
    }
}

// struct containing header of the block
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub parent: H256,
    pub nonce: u32,
    pub difficulty: H256,
    pub timestamp: u128, 
    pub merkle_root: H256
}

impl Hashable for Header {
    fn hash(&self) -> H256 {
    // Is this passing a reference or an object?
    let encoded: Vec<u8>  = serialize(&self.clone()).unwrap();
    // convert encoded to &[u8]
    let bytes = &encoded[..];
    ring::digest::digest(&digest::SHA256, bytes).into()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub data: Data
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

impl Block {
    pub fn get_parent(&self) -> H256 {
        self.header.parent
    }

    pub fn get_difficulty(&self) -> H256 {
        self.header.difficulty
    }
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_block(parent: &H256) -> Block {

    let nonce: u32 = rand::thread_rng().gen();
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let vec: Vec<SignedTransaction> = Vec::new();
    let data = Data{data: vec};
    let difficulty: H256 = [255u8; 32].into(); // ????????????
    let merkle_tree = MerkleTree::new(&data.data);
    let merkle_root = merkle_tree.root();
    let mut header = Header{parent: *parent, nonce, difficulty, timestamp, merkle_root};
    let mut block = Block{header, data: data};
    block
}