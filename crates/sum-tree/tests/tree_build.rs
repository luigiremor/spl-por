mod common;

use common::h32;
use sum_tree::{build, leaf_hash, node_hash, TreeError};

fn fixture(n: u8) -> Vec<([u8; 32], u64)> {
    (1..=n).map(|i| ([i; 32], 100 * u64::from(i))).collect()
}

const MAX: u64 = 1_000_000;

#[test]
fn n1_root_is_the_leaf_itself() {
    let tree = build(&fixture(1), MAX).unwrap();
    assert_eq!(tree.root_hash, leaf_hash(&[1; 32], 100));
    assert_eq!(tree.total, 100);
    assert_eq!(tree.leaf_count(), 1);
}

#[test]
fn n2_root_combines_the_two_leaves() {
    let tree = build(&fixture(2), MAX).unwrap();
    let l1 = leaf_hash(&[1; 32], 100);
    let l2 = leaf_hash(&[2; 32], 200);
    assert_eq!(tree.root_hash, node_hash(100, 200, &l1, &l2));
    assert_eq!(tree.total, 300);
}

#[test]
fn n3_uses_rfc6962_partition_left2_right1() {
    let tree = build(&fixture(3), MAX).unwrap();
    let l1 = leaf_hash(&[1; 32], 100);
    let l2 = leaf_hash(&[2; 32], 200);
    let l3 = leaf_hash(&[3; 32], 300);
    let left = node_hash(100, 200, &l1, &l2);
    assert_eq!(tree.root_hash, node_hash(300, 300, &left, &l3));
    assert_eq!(tree.total, 600);
    assert_eq!(
        tree.root_hash,
        h32("049fe1ee46b049d8184aa564b0e45a683478fdd07350fbef0e4f5a94b62efc8e")
    );
}

#[test]
fn n6_uses_rfc6962_partition_left4_right2() {
    let tree = build(&fixture(6), MAX).unwrap();
    let l: Vec<[u8; 32]> = (1..=6u8)
        .map(|i| leaf_hash(&[i; 32], 100 * u64::from(i)))
        .collect();
    let na = node_hash(100, 200, &l[0], &l[1]);
    let nb = node_hash(300, 400, &l[2], &l[3]);
    let nl = node_hash(300, 700, &na, &nb);
    let nr = node_hash(500, 600, &l[4], &l[5]);
    assert_eq!(tree.root_hash, node_hash(1000, 1100, &nl, &nr));
    assert_eq!(tree.total, 2100);
    assert_eq!(
        tree.root_hash,
        h32("79b3ee22ba61a53f5d3ffa111d41c10fabb9f461a5d888570f1afdafa41afea4")
    );
}

#[test]
fn rejects_empty_input() {
    assert_eq!(build(&[], MAX).unwrap_err(), TreeError::Empty);
}

#[test]
fn rejects_non_increasing_order() {
    let leaves = vec![([2u8; 32], 100), ([1u8; 32], 200)];
    assert_eq!(
        build(&leaves, MAX).unwrap_err(),
        TreeError::NotStrictlyIncreasing { index: 1 }
    );
}

#[test]
fn rejects_duplicate_account_id() {
    let leaves = vec![([1u8; 32], 100), ([1u8; 32], 200)];
    assert_eq!(
        build(&leaves, MAX).unwrap_err(),
        TreeError::NotStrictlyIncreasing { index: 1 }
    );
}

#[test]
fn rejects_balance_above_max_value() {
    let leaves = vec![([1u8; 32], MAX + 1)];
    assert_eq!(
        build(&leaves, MAX).unwrap_err(),
        TreeError::BalanceAboveMax { index: 0 }
    );
}

#[test]
fn rejects_sum_overflow_with_checked_add() {
    let leaves = vec![([1u8; 32], u64::MAX), ([2u8; 32], 1)];
    assert_eq!(
        build(&leaves, u64::MAX).unwrap_err(),
        TreeError::SumOverflow
    );
}

#[test]
fn rejects_subtotal_above_max_value() {
    let leaves = vec![([1u8; 32], 800), ([2u8; 32], 800)];
    assert_eq!(build(&leaves, 1_000).unwrap_err(), TreeError::SumAboveMax);
}
