//! Scenario matrix S09, S14–S16 (third-party reproducibility and the
//! availability premise): the verifier rejects what the program cannot see and
//! approves legitimate history after key rotation. Requires a validator.
use scenarios::{gen::*, harness::*};
use solana_sdk::signature::{Keypair, Signer};
use std::path::Path;

const N: usize = 10;

/// Config + attestation 0 anchored with supply = R_norm + `extra_supply`.
fn anchored(extra_supply: u64) -> (Ctx, Vec<u8>, [u8; 64]) {
    let ctx = setup(&rpc_url(), 6, 2, MAX_VALUE);
    let rows = generate(N, SEED);
    let tree = sum_tree::build(&rows, MAX_VALUE).unwrap();
    mint_supply_to(&ctx, tree.total * 10u64.pow(4) + extra_supply);
    let ds = serialize_dataset(&rows);
    let msg = attestation_msg(&ctx, 0, &tree, &ds);
    submit(
        &ctx,
        &build_submit_tx(&ctx, &msg, &ctx.oracle, Tamper::None),
    )
    .unwrap();
    let sig = sign(&ctx.oracle, &msg);
    (ctx, ds, sig)
}

#[test]
#[ignore]
fn s09_rotated_key_history_approved() {
    let (ctx, ds, sig) = anchored(0); // sequence 0 signed by the ORIGINAL key
    let new_oracle = Keypair::new();
    rotate_oracle_authority(&ctx, new_oracle.pubkey());
    assert_eq!(read_config(&ctx).oracle_authority, new_oracle.pubkey());
    // the historical record stays verifiable: the signature is checked under
    // the signer_pubkey RECORDED in the AuditRecord, not the current authority
    let dir = write_dataset_dir("tmp-s09", &ds, &sig);
    assert_eq!(run_verify(&ctx, 0, Some(&dir)), 0);
    record(
        &matrix_csv(),
        "s09_rotated_key_history_approved",
        N,
        "pass",
        1.0,
        "bool",
    );
}

#[test]
#[ignore]
fn s14_dataset_one_byte_tamper_rejected() {
    let (ctx, ds, sig) = anchored(0);
    let mut tampered = ds.clone();
    let mid = tampered.len() / 2;
    tampered[mid] ^= 0x01; // one byte, inside a balance digit or hex id
    let dir = write_dataset_dir("tmp-s14", &tampered, &sig);
    assert_eq!(
        run_verify(&ctx, 0, Some(&dir)),
        1,
        "sha256(dataset) != dataset_sha256 must fail verification"
    );
    record(
        &matrix_csv(),
        "s14_dataset_one_byte_tamper_rejected",
        N,
        "pass",
        1.0,
        "bool",
    );
}

#[test]
#[ignore]
fn s15_dataset_unavailable_exit2() {
    let (ctx, _ds, _sig) = anchored(0);
    let missing = Path::new("/nonexistent/por-dataset");
    assert_eq!(
        run_verify(&ctx, 0, Some(missing)),
        2,
        "unavailability is an operational failure: exit 2, never tacit approval"
    );
    record(
        &matrix_csv(),
        "s15_dataset_unavailable_exit2",
        N,
        "pass",
        1.0,
        "bool",
    );
}

#[test]
#[ignore]
fn s16_incoherent_status_rejected() {
    // The on-chain status cannot be tampered without the program's authority;
    // the scenario confronts the verifier with an incoherent record in a LOCAL
    // fixture: under-collateralized dataset, but AuditRecord claiming OK.
    let (ctx, ds, sig) = anchored(1); // R_norm < S
    assert_eq!(read_audit(&ctx, 0).status, por::state::STATUS_DIVERGENTE);
    let mut fixture = snapshot_fixture(&ctx);
    override_fixture_status(&mut fixture, &ctx, 0, por::state::STATUS_OK);
    let fixture_path = write_fixture("tmp-s16", &fixture);
    let dir = write_dataset_dir("tmp-s16", &ds, &sig);
    assert_eq!(
        run_verify_fixture(&fixture_path, &ctx.mint, 0, Some(&dir)),
        1,
        "the verifier's own verdict must contradict the recorded status"
    );
    record(
        &matrix_csv(),
        "s16_incoherent_status_rejected",
        N,
        "pass",
        1.0,
        "bool",
    );
}
