use crate::types::block::{Block, Header, Data};
use crate::types::hash::H256;
use crate::types::merkle::MerkleTree;
use crate::types::hash::Hashable;
use crate::types::transaction::SignedTransaction;
use std::collections::HashMap;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

pub struct Blockchain {
    pub blocks: HashMap<H256, Block>,
    heights: HashMap<H256, u128>,
    tip: H256
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let genesis_timestamp = 0;
        let data = Data{data: Vec::new()};
        let merkle_root = MerkleTree::new(&data.data).root();
        let parent: H256 = [0u8; 32].into();
        // when changing the difficulty make sure to change it in miner/mod.rs as well!!! 
        let difficulty: H256 = [60u8; 32].into();
        let genesis_header = Header{parent, nonce: 0, difficulty, timestamp:genesis_timestamp, merkle_root};
        let genesis = Block{header: genesis_header, data};
        let mut blocks: HashMap<H256, Block> = HashMap::new();
        let hash = genesis.hash();
        blocks.insert(hash, genesis);
        let mut heights: HashMap<H256, u128> = HashMap::new();
        heights.insert(hash, 0);        
        Blockchain{blocks,  heights, tip: hash}
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let new_block: Block = block.clone();
        let hash = block.hash();
        self.blocks.insert(hash, new_block);
        let new_block_height = self.heights.get(&block.get_parent()).unwrap() + 1;
        let longest_chain_height: u128 = self.heights.get(&self.tip).unwrap().clone();
        self.heights.insert(hash, new_block_height);
        
        if new_block_height > longest_chain_height {
            self.tip = hash;
        }
    }

    /// Get the hash of the last block in the longest chain
    pub fn tip(&self) -> H256 {
        self.tip
    }

    /// Get all blocks' hashes of the longest chain, ordered from genesis to the tip
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut vec: Vec<H256> = Vec::new();
        let mut tip = self.tip;
        for _i in 0..(self.heights.get(&self.tip).unwrap().clone()) {
            vec.push(self.blocks.get(&tip).unwrap().hash());
            tip = self.blocks.get(&tip).unwrap().get_parent();
        }
        vec.push(self.blocks.get(&tip).unwrap().hash());
        vec.reverse();
        vec
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());

    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST