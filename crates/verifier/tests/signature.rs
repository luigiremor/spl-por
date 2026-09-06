mod common;

use ed25519_dalek::SigningKey;
use solana_sdk::pubkey::Pubkey;
use verifier::chain::{load_state, FixtureFetcher, OnChainState};
use verifier::checks::signature::check_signature;

fn base_state_and_sig() -> (OnChainState, [u8; 64]) {
    let f = FixtureFetcher::load(&common::fixtures_dir().join("base.json")).unwrap();
    let st = load_state(&f, &por::ID, &common::mint(), 0).unwrap();
    (st, common::signature("dataset-0"))
}

/// Group order L of Ed25519, little-endian (RFC 8032).
const L_LE: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

/// Malleability S' = S + L: same signature modulo L, non-canonical encoding.
fn add_l_to_s(sig: &[u8; 64]) -> [u8; 64] {
    let mut out = *sig;
    let mut carry = 0u16;
    for i in 0..32 {
        let v = out[32 + i] as u16 + L_LE[i] as u16 + carry;
        out[32 + i] = v as u8;
        carry = v >> 8;
    }
    assert_eq!(carry, 0, "S + L fits in 32 bytes when S < L");
    out
}

#[test]
fn valid_signature_passes() {
    let (st, sig) = base_state_and_sig();
    let res = check_signature(&st.config, &st.record, st.mint_decimals, &sig);
    assert!(res.passed, "detail: {}", res.detail);
}

#[test]
fn s_plus_l_malleated_signature_is_rejected() {
    let (st, sig) = base_state_and_sig();
    let res = check_signature(&st.config, &st.record, st.mint_decimals, &add_l_to_s(&sig));
    assert!(!res.passed, "S + L must be rejected (S < L rule)");
}

#[test]
fn swapped_signer_key_is_rejected() {
    let (mut st, sig) = base_state_and_sig();
    let wrong = SigningKey::from_bytes(&[7u8; 32]);
    st.record.signer_pubkey = Pubkey::new_from_array(wrong.verifying_key().to_bytes());
    let res = check_signature(&st.config, &st.record, st.mint_decimals, &sig);
    assert!(
        !res.passed,
        "signature under a swapped key must be rejected"
    );
}

#[test]
fn tampered_anchored_field_breaks_signature() {
    // The verifier rebuilds the signed bytes from the AuditRecord itself: a
    // tampered total no longer matches the bytes the oracle signed.
    let (mut st, sig) = base_state_and_sig();
    st.record.total_reservas += 1;
    let res = check_signature(&st.config, &st.record, st.mint_decimals, &sig);
    assert!(!res.passed);
}

#[test]
fn current_oracle_authority_is_not_used_for_verification() {
    // Rotation survives: signer_pubkey in the record wins over PorConfig.oracle_authority.
    let (mut st, sig) = base_state_and_sig();
    st.config.oracle_authority = Pubkey::new_unique();
    let res = check_signature(&st.config, &st.record, st.mint_decimals, &sig);
    assert!(res.passed, "detail: {}", res.detail);
}
