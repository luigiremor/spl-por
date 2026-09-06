use scenarios::gen::{generate, serialize_dataset, MAX_VALUE, SEED};
use sha2::{Digest, Sha256};

#[test]
fn generate_is_deterministic_for_fixed_seed() {
    let a = generate(1_000, SEED);
    let b = generate(1_000, SEED);
    assert_eq!(a, b);
    let c = generate(1_000, SEED + 1);
    assert_ne!(a, c);
}

#[test]
fn generate_yields_n_strictly_increasing_unique_ids_within_max_value() {
    for n in [10usize, 100, 1_000] {
        let rows = generate(n, SEED);
        assert_eq!(rows.len(), n);
        for w in rows.windows(2) {
            assert!(w[0].0 < w[1].0, "account_id must be strictly increasing");
        }
        assert!(rows.iter().all(|(_, v)| *v <= MAX_VALUE));
    }
}

#[test]
fn every_dataset_size_builds_a_sum_tree_under_max_value() {
    // The largest N of the matrix must fit the declared max_value ceiling,
    // otherwise the attestation could not be built at all.
    for n in [10usize, 100_000] {
        let rows = generate(n, SEED);
        let tree = sum_tree::build(&rows, MAX_VALUE).expect("dataset must be buildable");
        let expected: u64 = rows.iter().map(|(_, v)| v).sum();
        assert_eq!(tree.total, expected);
    }
}

#[test]
fn serialized_dataset_hash_is_stable() {
    // Known-answer (generate-and-freeze): pins the canonical JSONL format
    // shared with the attester, so dataset_sha256 is machine-independent.
    let rows = generate(10, SEED);
    let bytes = serialize_dataset(&rows);
    let digest = hex::encode(Sha256::digest(&bytes));
    const EXPECTED: &str = "a98f0a6a1edfd5177a18302b5f295b31e56ad922e01bd76ddcb7561223d36ff4";
    assert_eq!(digest, EXPECTED);
}
