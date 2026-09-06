use sum_tree::{build, prove, verify_inclusion};
use verifier::checks::tree::check_tree;

const MAX_VALUE: u64 = 1_000_000_000_000;

/// 4 leaves with strictly increasing account_id; balances in cents; total = 10_000.
fn leaves() -> Vec<([u8; 32], u64)> {
    (1..=4u8).map(|i| ([i; 32], 1_000u64 * i as u64)).collect()
}

fn root_total() -> ([u8; 32], u64) {
    let tree = build(&leaves(), MAX_VALUE).unwrap();
    (tree.root_hash, tree.total)
}

#[test]
fn intact_dataset_passes_inclusion_and_binding() {
    let (root, total) = root_total();
    let res = check_tree(&leaves(), &root, total, MAX_VALUE, None);
    assert!(res.binding.passed, "binding: {}", res.binding.detail);
    assert!(res.inclusion.passed, "inclusion: {}", res.inclusion.detail);
}

#[test]
fn tampered_total_fails_binding() {
    let (root, total) = root_total();
    // Attester declares a total below the true sum (Hu et al. structure:
    // total not bound to the set): binding fails.
    let res = check_tree(&leaves(), &root, total - 1, MAX_VALUE, None);
    assert!(!res.binding.passed);
}

#[test]
fn tampered_root_fails_binding() {
    let (mut root, total) = root_total();
    root[0] ^= 0x01;
    let res = check_tree(&leaves(), &root, total, MAX_VALUE, None);
    assert!(!res.binding.passed);
}

#[test]
fn tampered_proof_subtotal_is_rejected_by_node_binding() {
    let (root, total) = root_total();
    let tree = build(&leaves(), MAX_VALUE).unwrap();
    let mut proof = prove(&tree, 0);
    // Tamper the sibling subtotal at the first step: verify_inclusion
    // re-applies the checked per-node sum and must reject (anti-Hu).
    proof.0[0].sibling_sum += 1;
    let (c, b) = leaves()[0];
    assert!(verify_inclusion(&c, b, &proof, &root, total, MAX_VALUE).is_err());
}

#[test]
fn out_of_order_dataset_is_rejected() {
    let (root, total) = root_total();
    let mut l = leaves();
    l.swap(0, 1); // breaks strictly increasing account_id order
    let res = check_tree(&l, &root, total, MAX_VALUE, None);
    assert!(!res.binding.passed, "out-of-order dataset must be rejected");
}

#[test]
fn balance_above_max_value_is_rejected() {
    let (root, total) = root_total();
    let res = check_tree(&leaves(), &root, total, 2_500, None);
    assert!(!res.binding.passed);
}

#[test]
fn specific_account_inclusion_and_absent_account_fails() {
    let (root, total) = root_total();
    let ok = check_tree(&leaves(), &root, total, MAX_VALUE, Some([2u8; 32]));
    assert!(ok.inclusion.passed);
    let missing = check_tree(&leaves(), &root, total, MAX_VALUE, Some([200u8; 32]));
    assert!(
        !missing.inclusion.passed,
        "account absent from the dataset must fail inclusion"
    );
    assert!(
        missing.binding.passed,
        "binding is independent of the requested account"
    );
}
