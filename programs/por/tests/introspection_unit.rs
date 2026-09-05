mod common;

use por::introspection::check_ed25519_ix_data;
use por::PorError;
use solana_sdk::pubkey::Pubkey;

const DUMMY_SIG: [u8; 64] = [0u8; 64];
const MSG: &[u8] = b"attestation-canonical-bytes";

fn oracle() -> Pubkey {
    Pubkey::new_unique()
}

/// Exact variant: compares the Anchor error code (6000 + index).
fn assert_err(result: anchor_lang::Result<Pubkey>, expected: PorError) {
    let err = result.expect_err("expected rejection");
    let expected = anchor_lang::error::Error::from(expected);
    assert_eq!(
        error_code(&err),
        error_code(&expected),
        "got {err:?}, expected {expected:?}"
    );
}

fn error_code(err: &anchor_lang::error::Error) -> u32 {
    match err {
        anchor_lang::error::Error::AnchorError(e) => e.error_code_number,
        other => panic!("not an anchor error: {other:?}"),
    }
}

#[test]
fn accepts_wellformed_payload_and_returns_pubkey() {
    let oracle = oracle();
    let data = common::ed25519_ix_data(&oracle.to_bytes(), &DUMMY_SIG, MSG);
    assert_eq!(check_ed25519_ix_data(&data, &oracle, MSG).unwrap(), oracle);
}

#[test]
fn rejects_two_signatures() {
    let oracle = oracle();
    let mut data = common::ed25519_ix_data(&oracle.to_bytes(), &DUMMY_SIG, MSG);
    data[0] = 2; // num_signatures != 1
    assert_err(
        check_ed25519_ix_data(&data, &oracle, MSG),
        PorError::MalformedEd25519Ix,
    );
}

#[test]
fn rejects_non_self_referential_instruction_index() {
    let oracle = oracle();
    let mut data = common::ed25519_ix_data(&oracle.to_bytes(), &DUMMY_SIG, MSG);
    // message_instruction_index (bytes 14..16) pointing at another instruction
    data[14] = 1;
    data[15] = 0;
    assert_err(
        check_ed25519_ix_data(&data, &oracle, MSG),
        PorError::MalformedEd25519Ix,
    );
}

#[test]
fn rejects_out_of_bounds_offset() {
    let oracle = oracle();
    let mut data = common::ed25519_ix_data(&oracle.to_bytes(), &DUMMY_SIG, MSG);
    // message_data_size (bytes 12..14) larger than the payload
    let huge = (data.len() as u16) + 1;
    data[12..14].copy_from_slice(&huge.to_le_bytes());
    assert_err(
        check_ed25519_ix_data(&data, &oracle, MSG),
        PorError::MalformedEd25519Ix,
    );
}

#[test]
fn rejects_truncated_payload() {
    let oracle = oracle();
    assert_err(
        check_ed25519_ix_data(&[1u8, 0u8], &oracle, MSG),
        PorError::MalformedEd25519Ix,
    );
}

#[test]
fn rejects_wrong_pubkey() {
    let oracle = oracle();
    let other = Pubkey::new_unique();
    let data = common::ed25519_ix_data(&other.to_bytes(), &DUMMY_SIG, MSG);
    assert_err(
        check_ed25519_ix_data(&data, &oracle, MSG),
        PorError::WrongSigner,
    );
}

#[test]
fn rejects_message_diverging_by_one_byte() {
    let oracle = oracle();
    let mut tampered = MSG.to_vec();
    tampered[0] ^= 0x01;
    let data = common::ed25519_ix_data(&oracle.to_bytes(), &DUMMY_SIG, &tampered);
    assert_err(
        check_ed25519_ix_data(&data, &oracle, MSG),
        PorError::MessageMismatch,
    );
}
