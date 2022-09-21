use std::ops::Add;
use rand::{thread_rng, Rng};
use ring::signature::{self,Ed25519KeyPair, EdDSAParameters, KeyPair, Signature, VerificationAlgorithm, UnparsedPublicKey};
use serde::{Deserialize, Serialize};
extern crate bincode;
use bincode::{serialize, deserialize};

use crate::types::address::Address;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    sender: Address,
    receiver: Address,
    value: u128, // Not sure if u128 is necessary, but I assume it has to be very large (lot of Satoshis in one bitcoin)
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    transaction: Transaction,
    signature: Vec<u8>,
    pubkey: Vec<u8>,
    // signature and pubkey represented as Vec<u8> for convenience --> check these structs as a part
    // of ring crate
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    // convert Transaction to Vec<u8>
    let encoded: Vec<u8> = serialize(t).unwrap();
    // convert encoded to &[u8]
    let transaction = &encoded[..];

    // sign the transaction using key.sign(transaction)
    key.sign(transaction)
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &[u8], signature: &[u8]) -> bool {
    // convert Transaction to Vec<u8>
    let encoded: Vec<u8>  = serialize(t).unwrap();
    // convert encoded to &[u8]
    let transaction = &encoded[..];
    // convert public_key to ring public key
    let peer_public_key =
        UnparsedPublicKey::new(&signature::ED25519, public_key);

    // verify the signed message using public key
    peer_public_key.verify(transaction, signature).is_ok()
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_transaction() -> Transaction {
    // initialize two arrays of 20 u8 0s
    let mut ar1 = [0u8;20];
    let mut ar2 = [0u8;20];
    // fill the arrays with 20 random values
    thread_rng().fill(&mut ar1[..]);
    thread_rng().fill(&mut ar2[..]);
    // return a transaction passing ar as a constructor argument
    let sender: Address = Address::from(ar1);
    let receiver: Address = Address::from(ar2);

    let value: u128 = thread_rng().gen();
    let transaction = Transaction{sender, receiver, value};
    transaction

}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use ring::signature::KeyPair;

    use crate::types::key_pair;

    use super::*;

    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, key.public_key().as_ref(), signature.as_ref()));
    }

    #[test]
    fn sign_verify_two() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        let key_2 = key_pair::random();
        let t_2 = generate_random_transaction();
        assert!(!verify(&t_2, key.public_key().as_ref(), signature.as_ref()));
        assert!(!verify(&t, key_2.public_key().as_ref(), signature.as_ref()));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
