use scenarios::{gen::*, harness::*};
use solana_sdk::signature::Signer;

#[test]
#[ignore] // requires solana-test-validator + program `por` deployed (scripts/eval-localnet.sh)
fn harness_submits_a_conformant_attestation_and_verifier_approves_it() {
    let ctx = setup(&rpc_url(), 6, 2, MAX_VALUE);
    let rows = generate(10, SEED);
    let tree = sum_tree::build(&rows, MAX_VALUE).unwrap();
    // supply exactly covered: S = R_norm = total * 10^(6-2)
    mint_supply_to(&ctx, tree.total * 10u64.pow(4));
    let ds = serialize_dataset(&rows);
    let msg = attestation_msg(&ctx, 0, &tree, &ds);
    let tx = build_submit_tx(&ctx, &msg, &ctx.oracle, Tamper::None);
    submit(&ctx, &tx).expect("conformant attestation must land");
    let audit = read_audit(&ctx, 0);
    assert_eq!(audit.status, por::state::STATUS_OK, "status must be OK");
    assert_eq!(audit.signer_pubkey, ctx.oracle.pubkey());
    let dir = write_dataset_dir("harness", &ds, &sign(&ctx.oracle, &msg));
    assert_eq!(run_verify(&ctx, 0, Some(&dir)), 0);
}
