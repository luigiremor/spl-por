mod common;

use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

fn send_two_ix(
    env: &mut common::Env,
    ix0: Instruction,
    ix1: Instruction,
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    env.svm.expire_blockhash();
    let payer = env.payer.insecure_clone();
    let tx = Transaction::new_signed_with_payer(
        &[ix0, ix1],
        Some(&payer.pubkey()),
        &[&payer],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx)
}

#[test]
fn missing_ed25519_ix_rejected() {
    // ONE-instruction transaction: there is no #0 to introspect
    let mut env = common::setup(1_000_000, 6);
    let ix1 = common::submit_ix(&env, common::default_args(0));
    env.svm.expire_blockhash();
    let payer = env.payer.insecure_clone();
    let tx = Transaction::new_signed_with_payer(
        &[ix1],
        Some(&payer.pubkey()),
        &[&payer],
        env.svm.latest_blockhash(),
    );
    let res = env.svm.send_transaction(tx);
    common::assert_por_error(res, 3); // MissingEd25519Ix = 6003
}

#[test]
fn previous_ix_not_ed25519_program_rejected() {
    // #0 addressed to another program (no-op self transfer via system program)
    let mut env = common::setup(1_000_000, 6);
    let payer_pk = env.payer.pubkey();
    let ix0 = solana_sdk::system_instruction::transfer(&payer_pk, &payer_pk, 1);
    let ix1 = common::submit_ix(&env, common::default_args(0));
    let res = send_two_ix(&mut env, ix0, ix1);
    common::assert_por_error(res, 3); // MissingEd25519Ix = 6003
}

#[test]
fn two_signatures_in_ix0_rejected() {
    // Payload with TWO identical, valid descriptors pointing at the SAME
    // triple: the precompile passes, the introspection rejects (num_signatures != 1).
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    let args = common::default_args(0);
    let msg = common::expected_message(&env, &args);
    let sig: [u8; 64] = oracle.sign_message(&msg).as_ref().try_into().unwrap();

    // two descriptors (2 x 14 bytes) pointing at the SAME triple
    let pk_off: u16 = 2 + 28; // header + 2 descriptors
    let sig_off: u16 = pk_off + 32;
    let msg_off: u16 = sig_off + 64;
    let mut data = vec![2u8, 0u8];
    for _ in 0..2 {
        data.extend(sig_off.to_le_bytes());
        data.extend(u16::MAX.to_le_bytes());
        data.extend(pk_off.to_le_bytes());
        data.extend(u16::MAX.to_le_bytes());
        data.extend(msg_off.to_le_bytes());
        data.extend((msg.len() as u16).to_le_bytes());
        data.extend(u16::MAX.to_le_bytes());
    }
    data.extend_from_slice(&oracle.pubkey().to_bytes());
    data.extend_from_slice(&sig);
    data.extend_from_slice(&msg);
    let ix0 = Instruction {
        program_id: solana_sdk::ed25519_program::id(),
        accounts: vec![],
        data,
    };
    let ix1 = common::submit_ix(&env, args);
    let res = send_two_ix(&mut env, ix0, ix1);
    common::assert_por_error(res, 4); // MalformedEd25519Ix = 6004
}

#[test]
fn wrong_signer_rejected() {
    // VALID triple (precompile passes), but signed by another key
    let mut env = common::setup(1_000_000, 6);
    let intruder = Keypair::new();
    let res = common::send_submit(&mut env, &intruder, common::default_args(0));
    common::assert_por_error(res, 5); // WrongSigner = 6005
}

#[test]
fn message_diverging_by_one_byte_rejected() {
    // The legitimate oracle signs a message with one flipped byte: the
    // precompile passes (signature matches the tampered message), but the
    // byte-for-byte reconstruction in #1 diverges.
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    let args = common::default_args(0);
    let mut msg = common::expected_message(&env, &args);
    msg[0] ^= 0x01;
    let ix0 = common::ed25519_ix(&oracle, &msg);
    let ix1 = common::submit_ix(&env, args);
    let res = send_two_ix(&mut env, ix0, ix1);
    common::assert_por_error(res, 6); // MessageMismatch = 6006
}

#[test]
fn invalid_signature_fails_at_precompile() {
    // Sanity: a corrupted signature must fail the transaction already at #0
    // (precompile error, not a por custom error). Confirms that the pinned
    // LiteSVM executes precompile verification.
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    let args = common::default_args(0);
    let msg = common::expected_message(&env, &args);
    let mut sig: [u8; 64] = oracle.sign_message(&msg).as_ref().try_into().unwrap();
    sig[0] ^= 0x01;
    let ix0 = Instruction {
        program_id: solana_sdk::ed25519_program::id(),
        accounts: vec![],
        data: common::ed25519_ix_data(&oracle.pubkey().to_bytes(), &sig, &msg),
    };
    let ix1 = common::submit_ix(&env, args);
    let res = send_two_ix(&mut env, ix0, ix1);
    let failed = res.expect_err("corrupted signature must fail precompile verification");
    // failure attributed to instruction #0 (the precompile), not to por (#1)
    assert!(
        matches!(
            failed.err,
            solana_sdk::transaction::TransactionError::InstructionError(0, _)
        ),
        "unexpected error: {:?}",
        failed.err
    );
}
