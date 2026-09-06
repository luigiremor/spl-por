//! Scenario matrix S01–S03 (Quadro 4, vector (c)): conformant and
//! under-collateralized attestations, verdict of the program AND of the
//! public verifier. Requires a validator (scripts/eval-localnet.sh).
use scenarios::{gen::*, harness::*};

const N: usize = 100;

/// Config + dataset of N accounts, supply = R_norm * num / den, attestation 0
/// submitted; returns (ctx, on-chain status, verifier exit code, r_norm, supply).
fn conform_case(supply_factor: (u64, u64)) -> (Ctx, u8, i32) {
    let ctx = setup(&rpc_url(), 6, 2, MAX_VALUE);
    let rows = generate(N, SEED);
    let tree = sum_tree::build(&rows, MAX_VALUE).unwrap();
    let r_norm = tree.total * 10u64.pow(4); // 10^(decimals - reserve_decimals)
    mint_supply_to(&ctx, r_norm * supply_factor.0 / supply_factor.1);
    let ds = serialize_dataset(&rows);
    let msg = attestation_msg(&ctx, 0, &tree, &ds);
    submit(
        &ctx,
        &build_submit_tx(&ctx, &msg, &ctx.oracle, Tamper::None),
    )
    .unwrap();
    let status = read_audit(&ctx, 0).status;
    let dir = write_dataset_dir("tmp-conform", &ds, &sign(&ctx.oracle, &msg));
    let exit = run_verify(&ctx, 0, Some(&dir));
    (ctx, status, exit)
}

#[test]
#[ignore]
fn s01_exact_coverage_ok() {
    let (_c, status, exit) = conform_case((1, 1)); // S == R_norm
    assert_eq!(status, por::state::STATUS_OK);
    assert_eq!(exit, 0);
    record(
        &matrix_csv(),
        "s01_exact_coverage_ok",
        N,
        "pass",
        1.0,
        "bool",
    );
}

#[test]
#[ignore]
fn s02_overcollateralized_ok() {
    let (_c, status, exit) = conform_case((1, 2)); // S = R_norm / 2 < R_norm
    assert_eq!(
        status,
        por::state::STATUS_OK,
        "over-collateralization is conformant, never strict equality"
    );
    assert_eq!(exit, 0);
    record(
        &matrix_csv(),
        "s02_overcollateralized_ok",
        N,
        "pass",
        1.0,
        "bool",
    );
}

#[test]
#[ignore]
fn s03_undercollateralized_divergent() {
    let ctx = setup(&rpc_url(), 6, 2, MAX_VALUE);
    let rows = generate(N, SEED);
    let tree = sum_tree::build(&rows, MAX_VALUE).unwrap();
    let r_norm = tree.total * 10u64.pow(4);
    let supply = r_norm + 1; // minimal injected deficit: R_norm < S
    mint_supply_to(&ctx, supply);
    let ds = serialize_dataset(&rows);
    let msg = attestation_msg(&ctx, 0, &tree, &ds);
    submit(
        &ctx,
        &build_submit_tx(&ctx, &msg, &ctx.oracle, Tamper::None),
    )
    .expect("a divergent attestation still lands (records DIVERGENTE)");
    let audit = read_audit(&ctx, 0);
    assert_eq!(
        audit.status,
        por::state::STATUS_DIVERGENTE,
        "program must record DIVERGENTE"
    );
    // observable delta, derivable from the public fields of the record alone
    let recorded_r_norm = audit.total_reservas * 10u64.pow(4);
    assert_eq!(audit.supply_at_ingestion - recorded_r_norm, 1);
    // the verifier independently agrees: coherent DIVERGENTE is APPROVED (exit 0)
    // with status_onchain DIVERGENTE; an incoherent one is what S16 rejects.
    let dir = write_dataset_dir("tmp-under", &ds, &sign(&ctx.oracle, &msg));
    assert_eq!(run_verify(&ctx, 0, Some(&dir)), 0);
    record(
        &matrix_csv(),
        "s03_undercollateralized_divergent",
        N,
        "pass",
        1.0,
        "bool",
    );
}
