//! Scenario matrix S04–S08 (Quadro 4, vectors (b) and (f)): tampered
//! ingestion never reaches the record. Requires a validator.
use scenarios::{gen::*, harness::*};

const N: usize = 10;

/// Config with a valid attestation 0 anchored; returns (ctx, tree, dataset).
fn ctx_with_seq0() -> (Ctx, sum_tree::SumTree, Vec<u8>) {
    let ctx = setup(&rpc_url(), 6, 2, MAX_VALUE);
    let rows = generate(N, SEED);
    let tree = sum_tree::build(&rows, MAX_VALUE).unwrap();
    mint_supply_to(&ctx, tree.total * 10u64.pow(4));
    let ds = serialize_dataset(&rows);
    let msg = attestation_msg(&ctx, 0, &tree, &ds);
    submit(
        &ctx,
        &build_submit_tx(&ctx, &msg, &ctx.oracle, Tamper::None),
    )
    .unwrap();
    (ctx, tree, ds)
}

fn rejected(ctx: &Ctx, msg: &sum_tree::AttestationMessage, tamper: Tamper) -> String {
    let err = submit(ctx, &build_submit_tx(ctx, msg, &ctx.oracle, tamper))
        .expect_err("transaction must be rejected");
    eprintln!("rejected as expected: {err}");
    err.to_string()
}

#[test]
#[ignore]
fn s04_replay_rejected() {
    let (ctx, tree, ds) = ctx_with_seq0();
    let replayed = attestation_msg(&ctx, 0, &tree, &ds); // sequence 0 again
    rejected(&ctx, &replayed, Tamper::None);
    assert_eq!(
        read_config(&ctx).next_sequence,
        1,
        "replay must not advance the log"
    );
    record(&matrix_csv(), "s04_replay_rejected", N, "pass", 1.0, "bool");
}

#[test]
#[ignore]
fn s05_sequence_gap_rejected() {
    let (ctx, tree, ds) = ctx_with_seq0();
    let gapped = attestation_msg(&ctx, 2, &tree, &ds); // skips 1
    let err = rejected(&ctx, &gapped, Tamper::None);
    assert!(
        err.contains("0x1770"),
        "SequenceMismatch = 6000 (0x1770): {err}"
    );
    record(
        &matrix_csv(),
        "s05_sequence_gap_rejected",
        N,
        "pass",
        1.0,
        "bool",
    );
}

#[test]
#[ignore]
fn s06_wrong_mint_rejected() {
    let (ctx, tree, ds) = ctx_with_seq0();
    let msg = attestation_msg(&ctx, 1, &tree, &ds);
    let other = create_mint(&ctx, ctx.decimals); // a real mint != PorConfig.mint
    rejected(&ctx, &msg, Tamper::WrongMint(other));
    record(
        &matrix_csv(),
        "s06_wrong_mint_rejected",
        N,
        "pass",
        1.0,
        "bool",
    );
}

#[test]
#[ignore]
fn s07_wrong_decimals_rejected() {
    let (ctx, tree, ds) = ctx_with_seq0();
    let msg = attestation_msg(&ctx, 1, &tree, &ds);
    let err = rejected(&ctx, &msg, Tamper::WrongDecimals(ctx.decimals + 1));
    assert!(
        err.contains("0x1771"),
        "DecimalsMismatch = 6001 (0x1771): {err}"
    );
    record(
        &matrix_csv(),
        "s07_wrong_decimals_rejected",
        N,
        "pass",
        1.0,
        "bool",
    );
}

#[test]
#[ignore]
fn s08_corrupt_signature_rejected() {
    let (ctx, tree, ds) = ctx_with_seq0();
    let msg = attestation_msg(&ctx, 1, &tree, &ds);
    let err = rejected(&ctx, &msg, Tamper::CorruptSig);
    // Attributed to instruction 0 (the precompile; PrecompileError::InvalidSignature
    // = custom 0x2), never to the program (ix#1).
    assert!(
        err.contains("Instruction 0"),
        "failure must come from the Ed25519 precompile (ix#0): {err}"
    );
    record(
        &matrix_csv(),
        "s08_corrupt_signature_rejected",
        N,
        "pass",
        1.0,
        "bool",
    );
}
