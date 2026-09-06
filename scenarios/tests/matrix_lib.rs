//! Scenario matrix S10–S13 (Quadro 4, vectors (a)/(d), (d), (f), (e)):
//! guardrails of the sum-tree library. No validator needed.
use scenarios::{gen::*, harness::*};
use sum_tree::{build, leaf_hash, node_hash, prove, verify_inclusion, InclusionProof, ProofStep};

const CSV: &str = "matrix-localnet.csv";

fn pass(name: &str, n: usize) {
    record(&results_path(CSV), name, n, "pass", 1.0, "bool");
}

#[test]
fn s10_corrupt_inclusion_proof_rejected() {
    let rows = generate(100, SEED);
    let tree = build(&rows, MAX_VALUE).unwrap();
    let mut proof = prove(&tree, 7);
    proof.0[0].sibling_hash[0] ^= 0x01; // corrupt the first step
    let (id, bal) = rows[7];
    let err = verify_inclusion(&id, bal, &proof, &tree.root_hash, tree.total, MAX_VALUE);
    assert!(err.is_err(), "corrupted proof must fail root recomputation");
    pass("s10_corrupt_inclusion_proof_rejected", 100);
}

#[test]
fn s11_hu_subtotal_tamper_rejected() {
    // STRUCTURAL attack of Hu et al. 2019: the sibling subtotal is altered
    // without touching the hash. In a scheme without per-node binding the hash
    // root would still close; here node_hash covers (sum_left, sum_right).
    let rows = generate(100, SEED);
    let tree = build(&rows, MAX_VALUE).unwrap();
    let mut proof = prove(&tree, 7);
    proof.0[0].sibling_sum = proof.0[0].sibling_sum.wrapping_sub(1);
    let (id, bal) = rows[7];
    let err = verify_inclusion(&id, bal, &proof, &tree.root_hash, tree.total, MAX_VALUE);
    assert!(
        err.is_err(),
        "tampered sibling subtotal must break per-node binding"
    );
    pass("s11_hu_subtotal_tamper_rejected", 100);
}

#[test]
fn s12_subtotal_overflow_aborts() {
    // ARITHMETIC vector, distinct from the structural S11: sums near u64::MAX.
    // (a) construction aborts via checked_add:
    let rows = vec![([0u8; 32], u64::MAX), ([1u8; 32], 2u64)];
    assert_eq!(
        build(&rows, u64::MAX).unwrap_err(),
        sum_tree::TreeError::SumOverflow,
        "checked_add must abort on overflow"
    );
    // (b) a forged proof with a huge sibling_sum fails verification:
    let rows = generate(4, SEED);
    let tree = build(&rows, MAX_VALUE).unwrap();
    let mut proof = prove(&tree, 0);
    proof.0[0].sibling_sum = u64::MAX; // exceeds max_value and the checked sum
    let (id, bal) = rows[0];
    assert!(verify_inclusion(&id, bal, &proof, &tree.root_hash, tree.total, MAX_VALUE).is_err());
    pass("s12_subtotal_overflow_aborts", 4);
}

#[test]
fn s13_node_as_leaf_rejected() {
    // Second preimage: an inner node presented as a leaf. Domain separation
    // (0x00 leaf, 0x01 node) makes the spaces disjoint: an "account_id" equal
    // to an inner-node hash never recomposes the same root through leaf_hash.
    let rows = generate(4, SEED);
    let tree = build(&rows, MAX_VALUE).unwrap();
    let l0 = leaf_hash(&rows[0].0, rows[0].1);
    let l1 = leaf_hash(&rows[1].0, rows[1].1);
    let inner = node_hash(rows[0].1, rows[1].1, &l0, &l1); // left node (leaves 0,1)
    let fake_balance = rows[0].1 + rows[1].1;
    let l23 = node_hash(
        rows[2].1,
        rows[3].1,
        &leaf_hash(&rows[2].0, rows[2].1),
        &leaf_hash(&rows[3].0, rows[3].1),
    );
    let forged = InclusionProof(vec![ProofStep {
        sibling_hash: l23,
        sibling_sum: rows[2].1 + rows[3].1,
        sibling_is_left: false,
    }]);
    // sanity: without domain separation this pair WOULD recompose the root
    assert_eq!(
        node_hash(fake_balance, rows[2].1 + rows[3].1, &inner, &l23),
        tree.root_hash
    );
    let err = verify_inclusion(
        &inner,
        fake_balance,
        &forged,
        &tree.root_hash,
        tree.total,
        MAX_VALUE,
    );
    assert!(
        err.is_err(),
        "domain separation must reject an inner node presented as leaf"
    );
    pass("s13_node_as_leaf_rejected", 4);
}
