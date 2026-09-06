mod common;

use solana_sdk::pubkey::Pubkey;
use verifier::chain::{AccountFetcher, FetchError, FixtureFetcher};

#[test]
fn fixture_fetcher_parses_accounts_and_reports_not_found() {
    let json = r#"{
        "slot": 7,
        "accounts": {
            "11111111111111111111111111111111": { "data_base64": "AAEC" }
        }
    }"#;
    let f = FixtureFetcher::from_json(json).unwrap();
    let known = Pubkey::new_from_array([0u8; 32]);
    let snap = f.fetch(&known).unwrap();
    assert_eq!(snap.data, vec![0u8, 1, 2]);
    assert_eq!(snap.slot, 7);
    let unknown = Pubkey::new_from_array([255u8; 32]);
    assert!(matches!(f.fetch(&unknown), Err(FetchError::NotFound(_))));
}

#[test]
fn fixture_fetcher_serves_historical_supply_only_for_recorded_slot() {
    let json = r#"{
        "slot": 7,
        "accounts": {},
        "historical_supplies": {
            "11111111111111111111111111111111": { "5": "90000000" }
        }
    }"#;
    let f = FixtureFetcher::from_json(json).unwrap();
    let mint = Pubkey::new_from_array([0u8; 32]);
    assert_eq!(f.supply_at_slot(&mint, 5).unwrap(), Some(90_000_000));
    assert_eq!(f.supply_at_slot(&mint, 6).unwrap(), None);
}

#[test]
fn load_state_from_base_fixture() {
    let f = FixtureFetcher::load(&common::fixtures_dir().join("base.json")).unwrap();
    let st = verifier::chain::load_state(&f, &por::ID, &common::mint(), 0).unwrap();
    assert_eq!(st.record.sequence, 0);
    assert_eq!(st.record.total_reservas, 4_660_446_420);
    assert_eq!(st.record.supply_at_ingestion, 4_660_446_420);
    assert_eq!(st.record.status, por::state::STATUS_OK);
    assert_eq!(st.record.ipfs_cid.len(), 59);
    assert_eq!(st.record.signer_pubkey.to_string(), common::ORACLE);
    assert_eq!(st.config.currency, *b"BRL");
    assert_eq!(st.config.reserve_decimals, 2);
    assert_eq!(st.config.next_sequence, 1);
    assert_eq!(st.mint_decimals, 2);
    assert_eq!(st.current_supply, 4_660_446_420);
    assert_eq!(st.historical_supply, Some(4_660_446_420));
    assert_eq!(st.history_roots.len(), 1);
    assert_eq!(st.log.leaf_count, 1);
}

#[test]
fn load_state_reads_whole_history_and_the_requested_record() {
    let f = FixtureFetcher::load(&common::fixtures_dir().join("history3.json")).unwrap();
    let st = verifier::chain::load_state(&f, &por::ID, &common::mint(), 1).unwrap();
    assert_eq!(st.record.sequence, 1);
    assert_eq!(st.record.status, por::state::STATUS_DIVERGENTE);
    assert_eq!(st.history_roots.len(), 3);
    assert_eq!(st.history_roots[1], st.record.merkle_root);
}

#[test]
fn load_state_missing_account_is_operational_error() {
    let f = FixtureFetcher::from_json(r#"{ "slot": 1, "accounts": {} }"#).unwrap();
    assert!(verifier::chain::load_state(&f, &por::ID, &common::mint(), 0).is_err());
    // sequence beyond next_sequence: not found, not a panic
    let f = FixtureFetcher::load(&common::fixtures_dir().join("base.json")).unwrap();
    assert!(matches!(
        verifier::chain::load_state(&f, &por::ID, &common::mint(), 1),
        Err(FetchError::NotFound(_))
    ));
}
