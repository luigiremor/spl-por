use sum_tree::{build, leaf_hash, prove, verify_inclusion, InclusionProof, ProofStep, VerifyError};

fn fixture(n: u8) -> Vec<([u8; 32], u64)> {
    (1..=n).map(|i| ([i; 32], 100 * u64::from(i))).collect()
}

const MAX: u64 = 1_000_000;

#[test]
fn roundtrip_all_indices_for_all_shapes() {
    for n in [1u8, 2, 3, 6, 8] {
        let leaves = fixture(n);
        let tree = build(&leaves, MAX).unwrap();
        for (index, (account_id, balance)) in leaves.iter().enumerate() {
            let proof = prove(&tree, index);
            verify_inclusion(
                account_id,
                *balance,
                &proof,
                &tree.root_hash,
                tree.total,
                MAX,
            )
            .unwrap_or_else(|e| panic!("n={n} index={index}: {e:?}"));
        }
    }
}

#[test]
fn proof_length_is_logarithmic() {
    let tree = build(&fixture(8), MAX).unwrap();
    assert_eq!(prove(&tree, 0).0.len(), 3);
    let tree1 = build(&fixture(1), MAX).unwrap();
    assert_eq!(prove(&tree1, 0).0.len(), 0);
}

#[test]
fn tampered_sibling_sum_is_detected_anti_hu() {
    let leaves = fixture(6);
    let tree = build(&leaves, MAX).unwrap();
    let mut proof = prove(&tree, 0);
    proof.0[0].sibling_sum -= 1;
    let against_true_total = verify_inclusion(
        &leaves[0].0,
        leaves[0].1,
        &proof,
        &tree.root_hash,
        tree.total,
        MAX,
    );
    assert_eq!(against_true_total.unwrap_err(), VerifyError::RootMismatch);
    let against_understated_total = verify_inclusion(
        &leaves[0].0,
        leaves[0].1,
        &proof,
        &tree.root_hash,
        tree.total - 1,
        MAX,
    );
    assert_eq!(
        against_understated_total.unwrap_err(),
        VerifyError::RootMismatch
    );
}

#[test]
fn tampered_sibling_hash_is_detected() {
    let leaves = fixture(6);
    let tree = build(&leaves, MAX).unwrap();
    let mut proof = prove(&tree, 2);
    proof.0[1].sibling_hash[0] ^= 0x01;
    let err = verify_inclusion(
        &leaves[2].0,
        leaves[2].1,
        &proof,
        &tree.root_hash,
        tree.total,
        MAX,
    )
    .unwrap_err();
    assert_eq!(err, VerifyError::RootMismatch);
}

#[test]
fn valid_proof_against_wrong_root_fails() {
    let leaves = fixture(3);
    let tree = build(&leaves, MAX).unwrap();
    let proof = prove(&tree, 1);
    let err = verify_inclusion(
        &leaves[1].0,
        leaves[1].1,
        &proof,
        &[0xff; 32],
        tree.total,
        MAX,
    )
    .unwrap_err();
    assert_eq!(err, VerifyError::RootMismatch);
}

#[test]
fn wrong_total_fails_even_with_right_root() {
    let leaves = fixture(3);
    let tree = build(&leaves, MAX).unwrap();
    let proof = prove(&tree, 1);
    let err = verify_inclusion(
        &leaves[1].0,
        leaves[1].1,
        &proof,
        &tree.root_hash,
        tree.total + 1,
        MAX,
    )
    .unwrap_err();
    assert_eq!(err, VerifyError::TotalMismatch);
}

#[test]
fn range_condition_is_enforced_during_verification() {
    let leaves = fixture(3);
    let tree = build(&leaves, MAX).unwrap();
    let proof = prove(&tree, 0);
    let err = verify_inclusion(
        &leaves[0].0,
        leaves[0].1,
        &proof,
        &tree.root_hash,
        tree.total,
        250,
    )
    .unwrap_err();
    assert_eq!(err, VerifyError::ValueAboveMax);
}

#[test]
fn overflow_during_verification_aborts() {
    let account_id = [1u8; 32];
    let balance = 2u64;
    let forged = InclusionProof(vec![ProofStep {
        sibling_hash: leaf_hash(&[2u8; 32], 1),
        sibling_sum: u64::MAX,
        sibling_is_left: false,
    }]);
    let err = verify_inclusion(&account_id, balance, &forged, &[0u8; 32], 0, u64::MAX).unwrap_err();
    assert_eq!(err, VerifyError::SumOverflow);
}
