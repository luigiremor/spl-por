mod common;

use anchor_lang::{AnchorDeserialize, Discriminator};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

fn find_event(logs: &[String]) -> Option<por::AttestationSubmitted> {
    for log in logs {
        if let Some(b64) = log.strip_prefix("Program data: ") {
            if let Ok(bytes) = B64.decode(b64) {
                if bytes.len() >= 8 && &bytes[..8] == por::AttestationSubmitted::DISCRIMINATOR {
                    return por::AttestationSubmitted::try_from_slice(&bytes[8..]).ok();
                }
            }
        }
    }
    None
}

#[test]
fn peaks_match_sum_tree_reference_after_three_appends() {
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    let mut reference = sum_tree::Mmr {
        peaks: vec![],
        leaf_count: 0,
    };

    for seq in 0..3u64 {
        let mut args = common::default_args(seq);
        args.merkle_root = [seq as u8 + 1; 32]; // distinct leaves
        let root = args.merkle_root;
        common::send_submit(&mut env, &oracle, args).unwrap();
        // MMR leaf = the attestation merkle_root (spec §4.5)
        reference.append(&root);
    }

    let log = common::get_log(&env);
    assert_eq!(log.leaf_count, 3);
    assert_eq!(usize::from(log.peak_count), reference.peaks.len()); // 3 = 0b11 => 2 peaks
    for (i, peak) in reference.peaks.iter().enumerate() {
        assert_eq!(&log.peaks[i], peak, "peak {i} differs from reference");
    }
}

#[test]
fn log_full_when_peak_capacity_exhausted() {
    // Forged state (unreachable by execution in a test: it would take ~2^33
    // appends): peak_count = 32 and an EVEN leaf_count with popcount 32
    // (0x1_FFFF_FFFE) => the append does not merge and would push a 33rd peak => LogFull.
    const FULL: u64 = 0x1_FFFF_FFFE;
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();

    let mut log_acc = env.svm.get_account(&env.log_pda).unwrap();
    let mut log = common::get_log(&env);
    log.peak_count = 32;
    log.leaf_count = FULL;
    for (i, peak) in log.peaks.iter_mut().enumerate() {
        *peak = [i as u8 + 1; 32];
    }
    let mut data = Vec::new();
    anchor_lang::AccountSerialize::try_serialize(&log, &mut data).unwrap();
    data.resize(por::state::LogState::SPACE, 0);
    log_acc.data = data;
    env.svm.set_account(env.log_pda, log_acc).unwrap();

    let mut cfg_acc = env.svm.get_account(&env.config_pda).unwrap();
    let mut config = common::get_config(&env);
    config.next_sequence = FULL; // sequence tracks leaf_count
    let mut data = Vec::new();
    anchor_lang::AccountSerialize::try_serialize(&config, &mut data).unwrap();
    data.resize(por::state::PorConfig::SPACE, 0);
    cfg_acc.data = data;
    env.svm.set_account(env.config_pda, cfg_acc).unwrap();

    let res = common::send_submit(&mut env, &oracle, common::default_args(FULL));
    common::assert_por_error(res, 8); // LogFull = 6008
}

#[test]
fn event_is_emitted_with_expected_fields() {
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    let meta = common::send_submit(&mut env, &oracle, common::default_args(0)).unwrap();
    let ev = find_event(&meta.logs).expect("AttestationSubmitted event not found");
    assert_eq!(ev.mint, env.mint);
    assert_eq!(ev.sequence, 0);
    assert_eq!(ev.merkle_root, [7u8; 32]);
    assert_eq!(ev.total_reservas, 200);
    assert_eq!(ev.supply_at_ingestion, 1_000_000);
    assert_eq!(ev.status, por::state::STATUS_OK);
}
