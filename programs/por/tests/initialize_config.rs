mod common;

use solana_sdk::signature::Signer;

#[test]
fn initialize_config_creates_config_and_log() {
    let env = common::setup(1_000_000, 6);

    let config = common::get_config(&env);
    assert_eq!(config.mint, env.mint);
    assert_eq!(config.oracle_authority, env.oracle.pubkey());
    assert_eq!(config.admin, env.admin.pubkey());
    assert_eq!(config.schema_version, por::state::SCHEMA_VERSION);
    assert_eq!(config.currency, *b"BRL");
    assert_eq!(config.reserve_decimals, 2);
    assert_eq!(config.max_value, 100_000_000_000);
    assert_eq!(config.next_sequence, 0);
    assert_eq!(config.log_state, env.log_pda);

    let log = common::get_log(&env);
    assert_eq!(log.peak_count, 0);
    assert_eq!(log.leaf_count, 0);
    assert_eq!(log.peaks, [[0u8; 32]; 32]);
}
