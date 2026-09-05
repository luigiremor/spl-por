use anchor_lang::prelude::*;

/// Normative order: Anchor code = 6000 + variant index (interface contract).
#[error_code]
pub enum PorError {
    #[msg("sequence does not match next_sequence")]
    SequenceMismatch, // 6000
    #[msg("declared decimals do not match the mint account")]
    DecimalsMismatch, // 6001
    #[msg("mint account does not match PorConfig.mint")]
    MintMismatch, // 6002
    #[msg("previous instruction is not addressed to the Ed25519 program")]
    MissingEd25519Ix, // 6003
    #[msg("malformed Ed25519 instruction payload")]
    MalformedEd25519Ix, // 6004
    #[msg("verified pubkey is not the oracle authority")]
    WrongSigner, // 6005
    #[msg("signed message does not match the reconstructed bytes")]
    MessageMismatch, // 6006
    #[msg("checked arithmetic overflow")]
    Overflow, // 6007
    #[msg("MMR peak capacity exhausted")]
    LogFull, // 6008
    #[msg("signer is not the admin")]
    Unauthorized, // 6009
}
