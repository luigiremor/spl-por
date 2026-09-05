mod common;

use solana_sdk::signature::Signer;

#[test]
fn happy_path_persists_record_and_advances_sequence() {
    // supply 1_000_000 (decimals 6); R_norm = 200 * 10^4 = 2_000_000 >= supply => OK
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    let args = common::default_args(0);

    let meta = common::send_submit(&mut env, &oracle, args.clone()).unwrap();

    let record = common::get_record(&env, 0);
    assert_eq!(record.sequence, 0);
    assert_eq!(record.merkle_root, [7u8; 32]);
    assert_eq!(record.total_reservas, 200);
    assert_eq!(record.supply_at_ingestion, 1_000_000);
    assert_eq!(record.ipfs_cid, "bafy-test-cid");
    assert_eq!(record.dataset_sha256, [9u8; 32]);
    assert_eq!(record.signer_pubkey, oracle.pubkey());
    assert_eq!(record.status, por::state::STATUS_OK);

    let config = common::get_config(&env);
    assert_eq!(config.next_sequence, 1);

    let log = common::get_log(&env);
    assert_eq!(log.leaf_count, 1);
    assert_eq!(log.peak_count, 1);

    // event emitted (decoded in the MMR tests; here only presence of the data log)
    assert!(meta.logs.iter().any(|l| l.starts_with("Program data: ")));
}

#[test]
fn sequence_gap_rejected() {
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    common::send_submit(&mut env, &oracle, common::default_args(0)).unwrap();
    // gap: skips sequence 1
    let res = common::send_submit(&mut env, &oracle, common::default_args(2));
    common::assert_por_error(res, 0); // SequenceMismatch = 6000
}

#[test]
fn replay_of_same_sequence_rejected() {
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    common::send_submit(&mut env, &oracle, common::default_args(0)).unwrap();
    // replay: the AuditRecord PDA for sequence 0 already exists => `init` fails
    // (system program error, not a por custom error) — asserting failure suffices.
    let res = common::send_submit(&mut env, &oracle, common::default_args(0));
    assert!(res.is_err(), "replay must fail");
}

#[test]
fn diverging_decimals_rejected() {
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    let mut args = common::default_args(0);
    args.decimals = 9; // mint has 6
    let res = common::send_submit(&mut env, &oracle, args);
    common::assert_por_error(res, 1); // DecimalsMismatch = 6001
}

#[test]
fn mint_mismatch_rejected() {
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    // tamper with PorConfig.mint (defense in depth beyond the seed):
    let mut acc = env.svm.get_account(&env.config_pda).unwrap();
    let mut config = common::get_config(&env);
    config.mint = solana_sdk::pubkey::Pubkey::new_unique();
    let mut data = Vec::new();
    anchor_lang::AccountSerialize::try_serialize(&config, &mut data).unwrap();
    data.resize(por::state::PorConfig::SPACE, 0);
    acc.data = data;
    env.svm.set_account(env.config_pda, acc).unwrap();

    let res = common::send_submit(&mut env, &oracle, common::default_args(0));
    common::assert_por_error(res, 2); // MintMismatch = 6002
}
