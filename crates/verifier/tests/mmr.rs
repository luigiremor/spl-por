mod common;

use verifier::chain::{load_state, FixtureFetcher, OnChainState};
use verifier::checks::mmr::check_mmr;

fn history3_state() -> OnChainState {
    let f = FixtureFetcher::load(&common::fixtures_dir().join("history3.json")).unwrap();
    load_state(&f, &por::ID, &common::mint(), 2).unwrap()
}

#[test]
fn three_attestation_history_matches_log_state() {
    let st = history3_state();
    assert_eq!(st.history_roots.len(), 3);
    assert_eq!(st.log.leaf_count, 3);
    let res = check_mmr(&st.history_roots, &st.log);
    assert!(res.passed, "{}", res.detail);
}

#[test]
fn history_is_the_same_whichever_sequence_is_verified() {
    // Verifying sequence 0 while 3 attestations exist still checks the whole log.
    let f = FixtureFetcher::load(&common::fixtures_dir().join("history3.json")).unwrap();
    let st = load_state(&f, &por::ID, &common::mint(), 0).unwrap();
    assert_eq!(st.record.sequence, 0);
    assert_eq!(st.history_roots.len(), 3);
    assert!(check_mmr(&st.history_roots, &st.log).passed);
}

#[test]
fn tampered_root_in_history_fails() {
    let st = history3_state();
    let mut roots = st.history_roots.clone();
    roots[1][0] ^= 0x01; // tampers the root of sequence 1
    let res = check_mmr(&roots, &st.log);
    assert!(
        !res.passed,
        "tampered history must diverge from anchored peaks"
    );
}

#[test]
fn truncated_history_fails_leaf_count() {
    let st = history3_state();
    let res = check_mmr(&st.history_roots[..2], &st.log);
    assert!(!res.passed, "truncated history must fail on leaf_count");
    assert!(res.detail.contains("leaf_count"));
}

#[test]
fn single_attestation_base_fixture_matches_log_state() {
    let f = FixtureFetcher::load(&common::fixtures_dir().join("base.json")).unwrap();
    let st = load_state(&f, &por::ID, &common::mint(), 0).unwrap();
    assert_eq!(st.log.leaf_count, 1);
    assert!(check_mmr(&st.history_roots, &st.log).passed);
}
