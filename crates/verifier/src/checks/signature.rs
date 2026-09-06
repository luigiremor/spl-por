use crate::report::CheckResult;
use ed25519_dalek::{Signature, VerifyingKey};
use por::state::{AuditRecord, PorConfig};
use sum_tree::AttestationMessage;

/// Check 1 (§4.4.1): rebuilds canonical_bytes from authoritative sources
/// (PorConfig + AuditRecord + decimals read from the mint) and verifies
/// Ed25519 under the signer_pubkey PERSISTED in the record (robust to
/// oracle_authority rotation). `verify_strict` is exactly the monograph's
/// canonical acceptance rule: S < L (signature deserialization in
/// ed25519-dalek 2.x rejects a non-canonical scalar) + rejection of
/// small-order points in A and R — the same criterion as the platform's
/// Ed25519 precompile.
pub fn check_signature(
    config: &PorConfig,
    record: &AuditRecord,
    mint_decimals: u8,
    signature: &[u8; 64],
) -> CheckResult {
    let bytes = AttestationMessage {
        schema_version: config.schema_version,
        mint: config.mint.to_bytes(),
        sequence: record.sequence,
        merkle_root: record.merkle_root,
        total_reservas: record.total_reservas,
        currency: config.currency,
        decimals: mint_decimals,
        reserve_decimals: config.reserve_decimals,
        ipfs_cid: record.ipfs_cid.clone(),
        dataset_sha256: record.dataset_sha256,
    }
    .canonical_bytes();
    let vk = match VerifyingKey::from_bytes(&record.signer_pubkey.to_bytes()) {
        Ok(vk) => vk,
        Err(e) => return CheckResult::fail("signature", format!("invalid signer_pubkey: {e}")),
    };
    match vk.verify_strict(&bytes, &Signature::from_bytes(signature)) {
        Ok(()) => CheckResult::pass(
            "signature",
            format!(
                "Ed25519 verify_strict ok over {} canonical bytes under recorded signer_pubkey {}",
                bytes.len(),
                record.signer_pubkey
            ),
        ),
        Err(_) => CheckResult::fail(
            "signature",
            "Ed25519 verify_strict rejected (bad signature, non-canonical S, or small-order point)",
        ),
    }
}
