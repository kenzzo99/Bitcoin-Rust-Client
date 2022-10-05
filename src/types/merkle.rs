use std::convert::TryInto;
use super::hash::{Hashable, H256};
use ring::{digest};
extern crate bincode;
use bincode::{serialize, deserialize};

use crate::types::address::Address;
/// A Merkle tree.
#[derive(Debug, Default)]
pub struct MerkleTree {
    hashes: Vec<Vec<H256>>, // do these need to be type Vec<H256> ??

}

impl MerkleTree {
    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        let mut len = data.len(); // number of files to store in a merkle tree
        let mut level = 0; // current level we're populating
        let mut pointer: usize = 0;
        let mut hashes: Vec<Vec<H256>> = Vec::new();
        // hash all the provided data
        // if input is empty return an empty Vec of Vec<H256>
        if len > 0 {
            hashes.push(Vec::new());
        } else { return MerkleTree { hashes } }
        // if input contains just one element return a tree with just the hash of the element
        if len == 1 {
            hashes[0].push(data[0].hash());
            return MerkleTree { hashes }
        }

        // populate the first level of the tree with hashes of data
        for i in 0..len {
            hashes[level].push(data[i].hash());
        };

        // if there's an odd number of hashes, we duplicate the last one to make the number even
        if len % 2 == 1 {
            len = len + 1;
            let temp = hashes[level][hashes[level].len() - 1].clone();
            hashes[level].push(temp);
        };
        // add another level to the tree
        hashes.push(Vec::new());
        level = level + 1;
        // loop until the length of the current level of the tree is 1, i.e. until we compute
        // the root

        loop {
            // compute the hash of pairs of hashes, incrementing the pointer along the way
            for i in 0..len/2  {

                // create a hash of two child hashes
                let mut ctx = digest::Context::new(&digest::SHA256);
                ctx.update(hashes[level - 1][i].as_ref()); // maybe will not work
                ctx.update(hashes[level - 1][i + 1].as_ref());
                let temp: H256 = ctx.finish().into();
                hashes[level].push(temp);
            };

            // halve the size as the current level is half the size of the one before;
            len = len / 2;


            // if the length of the upper level is odd, check if we reached the root, if so break
            // if not, create a duplicate of the last node in the tree and increment the length
            if len % 2 == 1 {
                if len == 1 { break };
                let temp = hashes[level][len - 1].clone();
                hashes[level].push(temp);
                len = len + 1;
            };

            // switch to upper level
            level = level + 1;
            hashes.push(Vec::new());

        };
        // return the tree
        MerkleTree { hashes }
    }

    pub fn root(&self) -> H256 {
        if self.hashes.len() == 0 {
            let hash: H256 = [0u8; 32].into();
            return hash;
        }
        self.hashes[self.hashes.len() - 1][0]
    }

    /// Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {
        let mut proof: Vec<H256> = Vec::new(); // vector to return
        let mut pointer: usize = index;

        if self.hashes.len() == 1 {
            if self.hashes[0].len() == 1 {
                return proof
            }
            else if index % 2 == 0 {
                proof.push(self.hashes[0][1]);
                return { proof }
            } else {
                proof.push(self.hashes[0][0]);
                return { proof }
            };
        }

        for i in 0..(self.hashes.len() - 1) {
            if pointer % 2 == 0 {
                proof.push(self.hashes[i][pointer + 1]);
                pointer = pointer + 1;
            }
            else {
                proof.push(self.hashes[i][pointer - 1]);
                pointer - 1;
            }
            pointer = pointer / 2;
        };

        proof

    }
}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.
pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, leaf_size: usize) -> bool {
    let zero_hash: H256 = [0u8; 32].into();
    if *root == zero_hash {
        return false
    }
    let mut test: &H256 = datum;
    let mut temp: H256;
    let mut pointer = index;
    for i in 0..proof.len() {
        let mut ctx = digest::Context::new(&digest::SHA256);
        if pointer % 2 == 0 {
            ctx.update(test.as_ref());
            ctx.update(&proof[i].as_ref());
        } else {
            ctx.update(&proof[i].as_ref());
            ctx.update(test.as_ref());
        }
        pointer = pointer / 2;

        temp = ctx.finish().into();
        test = &temp;
    }
    if root == test {
        return true
    }
    return false
}
// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use crate::types::hash::H256;
    use super::*;

    macro_rules! gen_merkle_tree_data {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    #[test]
    fn merkle_root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn merkle_proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(proof,
                   vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn merkle_verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST