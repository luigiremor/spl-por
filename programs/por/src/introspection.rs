use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions::{
    load_current_index_checked, load_instruction_at_checked,
};

use crate::errors::PorError;

/// Native Ed25519 precompile (`Ed25519SigVerify111111111111111111111111111`);
/// solana-program 2.3 no longer re-exports the `ed25519_program` module.
pub const ED25519_PROGRAM_ID: Pubkey = pubkey!("Ed25519SigVerify111111111111111111111111111");

const HEADER_LEN: usize = 2; // num_signatures (1) + padding (1)
const OFFSETS_LEN: usize = 14; // 7 x u16 LE
const SELF_IX: u16 = u16::MAX; // precompile convention: refers to the instruction itself
const PUBKEY_LEN: usize = 32;
const SIG_LEN: usize = 64;

fn read_u16(data: &[u8], pos: usize) -> Result<u16> {
    let bytes: [u8; 2] = data
        .get(pos..pos + 2)
        .ok_or(PorError::MalformedEd25519Ix)?
        .try_into()
        .expect("slice of length 2");
    Ok(u16::from_le_bytes(bytes))
}

/// Conditions (a) structure, (b) authority and (c) message (monograph §4.7.1)
/// over the payload of an Ed25519Program instruction. Returns the verified pubkey.
pub fn check_ed25519_ix_data(
    data: &[u8],
    oracle_authority: &Pubkey,
    expected_message: &[u8],
) -> Result<Pubkey> {
    // (a) exactly one verified triple, no extra descriptors
    require!(
        data.len() >= HEADER_LEN + OFFSETS_LEN,
        PorError::MalformedEd25519Ix
    );
    require!(data[0] == 1, PorError::MalformedEd25519Ix);

    let sig_off = usize::from(read_u16(data, 2)?);
    let sig_ix = read_u16(data, 4)?;
    let pk_off = usize::from(read_u16(data, 6)?);
    let pk_ix = read_u16(data, 8)?;
    let msg_off = usize::from(read_u16(data, 10)?);
    let msg_len = usize::from(read_u16(data, 12)?);
    let msg_ix = read_u16(data, 14)?;

    // (a) self-referential indices: the triple lives entirely inside #0,
    // preventing relocation of pubkey/message/signature to another instruction
    require!(
        sig_ix == SELF_IX && pk_ix == SELF_IX && msg_ix == SELF_IX,
        PorError::MalformedEd25519Ix
    );

    // (a) offsets/lengths within the payload
    let pk_end = pk_off
        .checked_add(PUBKEY_LEN)
        .ok_or(PorError::MalformedEd25519Ix)?;
    let sig_end = sig_off
        .checked_add(SIG_LEN)
        .ok_or(PorError::MalformedEd25519Ix)?;
    let msg_end = msg_off
        .checked_add(msg_len)
        .ok_or(PorError::MalformedEd25519Ix)?;
    require!(
        pk_end <= data.len() && sig_end <= data.len() && msg_end <= data.len(),
        PorError::MalformedEd25519Ix
    );

    // (b) pubkey verified by the precompile == PorConfig.oracle_authority
    let pubkey = Pubkey::try_from(&data[pk_off..pk_end])
        .map_err(|_| error!(PorError::MalformedEd25519Ix))?;
    require_keys_eq!(pubkey, *oracle_authority, PorError::WrongSigner);

    // (c) verified message == byte-for-byte reconstruction
    require!(
        &data[msg_off..msg_end] == expected_message,
        PorError::MessageMismatch
    );

    Ok(pubkey)
}

/// Loads the PREVIOUS instruction through the Instructions sysvar and requires
/// it to be addressed to the native Ed25519Program before checking its payload.
pub fn verify_ed25519_introspection(
    instructions_sysvar: &AccountInfo,
    oracle_authority: &Pubkey,
    expected_message: &[u8],
) -> Result<Pubkey> {
    let current = usize::from(load_current_index_checked(instructions_sysvar)?);
    require!(current >= 1, PorError::MissingEd25519Ix);
    let prev = load_instruction_at_checked(current - 1, instructions_sysvar)?;
    require_keys_eq!(
        prev.program_id,
        ED25519_PROGRAM_ID,
        PorError::MissingEd25519Ix
    );
    check_ed25519_ix_data(&prev.data, oracle_authority, expected_message)
}
