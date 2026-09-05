mod common;

use common::h32;
use sha2::{Digest, Sha256};
use sum_tree::{
    leaf_hash, node_hash, LEAF_DOMAIN, MMR_BAG_DOMAIN, MMR_LEAF_DOMAIN, MMR_NODE_DOMAIN,
    NODE_DOMAIN,
};

#[test]
fn domain_constants_match_spec() {
    assert_eq!(LEAF_DOMAIN, 0x00);
    assert_eq!(NODE_DOMAIN, 0x01);
    assert_eq!(MMR_LEAF_DOMAIN, 0x02);
    assert_eq!(MMR_NODE_DOMAIN, 0x03);
    assert_eq!(MMR_BAG_DOMAIN, 0x04);
}

#[test]
fn leaf_hash_known_answers() {
    assert_eq!(
        leaf_hash(&[0x00; 32], 0),
        h32("9e1736c43d19118e6ce4302118af337109491ecc52757dfb949bad6a7940b0c2")
    );
    assert_eq!(
        leaf_hash(&[0x11; 32], 1_000_000),
        h32("70d5e17a860ce0d24d7bd8d1c6420140166bb242b16d0633e1e58f46f7db6bb5")
    );
}

#[test]
fn node_hash_known_answer() {
    assert_eq!(
        node_hash(1, 2, &[0xaa; 32], &[0xbb; 32]),
        h32("147af570a419f2faab624113f3beecec67525e2019dd0eb9a5a9a9d484ca18ab")
    );
}

#[test]
fn leaf_and_node_domains_are_disjoint() {
    let account_id = [0x11u8; 32];
    let balance = 1_000_000u64;
    let mut h = Sha256::new();
    h.update([0x01u8]);
    h.update(account_id);
    h.update(balance.to_be_bytes());
    let node_interpretation: [u8; 32] = h.finalize().into();
    assert_ne!(leaf_hash(&account_id, balance), node_interpretation);
    assert_eq!(
        node_interpretation,
        h32("895cd34e4c1e202158ead708de3e52ed78597651120bbefb036c09e6a4fca364")
    );
}

#[test]
fn leaf_hash_is_sensitive_to_every_input() {
    let base = leaf_hash(&[0x11; 32], 42);
    assert_ne!(base, leaf_hash(&[0x12; 32], 42));
    assert_ne!(base, leaf_hash(&[0x11; 32], 43));
}
