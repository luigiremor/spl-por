use crate::snapshot::SnapshotEntry;

/// Range ceiling declared well below 2^64 - 1 (10^12 cents = R$ 10 billion).
/// The SAME value is passed to initialize_config in the E2E script.
pub const MAX_VALUE: u64 = 1_000_000_000_000;
pub const SCHEMA_VERSION: u16 = 1;
/// On-chain limit for `ipfs_cid` (`por::state::MAX_CID_LEN`).
pub const MAX_CID_LEN: usize = 64;

/// Builds the canonical AttestationMessage for a snapshot by delegating the
/// commitment entirely to sum_tree::build (no tree logic in the attester).
pub fn build_message(
    entries: &[SnapshotEntry],
    mint: [u8; 32],
    sequence: u64,
    decimals: u8,
    ipfs_cid: String,
    dataset_sha256: [u8; 32],
) -> anyhow::Result<sum_tree::AttestationMessage> {
    anyhow::ensure!(
        ipfs_cid.len() <= MAX_CID_LEN,
        "ipfs_cid exceeds {MAX_CID_LEN} bytes (on-chain limit)"
    );
    let leaves: Vec<([u8; 32], u64)> = entries.iter().map(|e| (e.account_id, e.balance)).collect();
    let tree = sum_tree::build(&leaves, MAX_VALUE)
        .map_err(|e| anyhow::anyhow!("sum tree build failed: {e}"))?;
    Ok(sum_tree::AttestationMessage {
        schema_version: SCHEMA_VERSION,
        mint,
        sequence,
        merkle_root: tree.root_hash,
        total_reservas: tree.total,
        currency: *b"BRL",
        decimals,
        reserve_decimals: 2,
        ipfs_cid,
        dataset_sha256,
    })
}

/// JCS manifest (RFC 8785) published off-chain next to the dataset. This is a
/// verifiable dataset, NOT the signed message (the on-chain signature covers
/// AttestationMessage::canonical_bytes()). Advisory fields do not gate the
/// on-chain reconciliation. Shape fixed by the contract; serialization
/// delegated to `sum_tree::Manifest::to_jcs` (stage 1).
pub fn build_manifest(
    msg: &sum_tree::AttestationMessage,
    supply_snapshot: u64,
    snapshot_slot: u64,
    issued_at: &str,
) -> anyhow::Result<String> {
    Ok(sum_tree::Manifest {
        schema_version: msg.schema_version,
        // Contract (monograph Table "payload"): mint is base58 in the manifest.
        mint: bs58::encode(msg.mint).into_string(),
        sequence: msg.sequence.to_string(),
        merkle_root: sum_tree::hex32(&msg.merkle_root),
        total_reservas: msg.total_reservas.to_string(),
        currency: String::from_utf8(msg.currency.to_vec())?,
        decimals: msg.decimals,
        reserve_decimals: msg.reserve_decimals,
        ipfs_cid: msg.ipfs_cid.clone(),
        dataset_sha256: sum_tree::hex32(&msg.dataset_sha256),
        supply_snapshot: supply_snapshot.to_string(),
        snapshot_slot: snapshot_slot.to_string(),
        hash_alg: "SHA-256".to_string(),
        issued_at: issued_at.to_string(),
    }
    .to_jcs()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::generate_snapshot;

    fn sample_message() -> sum_tree::AttestationMessage {
        let entries = generate_snapshot(10, 42, 0);
        build_message(
            &entries,
            [9u8; 32],
            0,
            2,
            "bafytestcid".to_string(),
            [7u8; 32],
        )
        .unwrap()
    }

    #[test]
    fn message_total_equals_checked_sum_of_balances() {
        let entries = generate_snapshot(10, 42, 0);
        let expected: u64 = entries.iter().map(|e| e.balance).sum();
        let msg = sample_message();
        assert_eq!(msg.total_reservas, expected);
        assert_eq!(msg.schema_version, 1);
        assert_eq!(&msg.currency, b"BRL");
        assert_eq!(msg.reserve_decimals, 2);
        assert_eq!(msg.decimals, 2);
        assert_ne!(
            msg.merkle_root, [0u8; 32],
            "root must be computed by sum_tree::build"
        );
    }

    #[test]
    fn message_root_matches_direct_sum_tree_build() {
        let entries = generate_snapshot(10, 42, 0);
        let leaves: Vec<([u8; 32], u64)> =
            entries.iter().map(|e| (e.account_id, e.balance)).collect();
        let tree = sum_tree::build(&leaves, MAX_VALUE).unwrap();
        let msg = sample_message();
        assert_eq!(msg.merkle_root, tree.root_hash);
        assert_eq!(msg.total_reservas, tree.total);
    }

    #[test]
    fn single_leaf_snapshot_builds() {
        let entries = generate_snapshot(1, 42, 0);
        let msg = build_message(&entries, [9u8; 32], 0, 2, "bafy".into(), [0u8; 32]).unwrap();
        assert_eq!(msg.total_reservas, entries[0].balance);
    }

    #[test]
    fn cid_longer_than_on_chain_limit_is_rejected() {
        let entries = generate_snapshot(1, 42, 0);
        assert!(build_message(&entries, [9u8; 32], 0, 2, "b".repeat(65), [0u8; 32]).is_err());
    }

    #[test]
    fn manifest_serializes_u64_as_json_strings() {
        let msg = sample_message();
        let manifest = build_manifest(&msg, 123_456, 42, "2026-08-01T12:00:00Z").unwrap();
        let v: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        // Contract rule: every u64 appears as a STRING in the JCS manifest.
        assert!(v["total_reservas"].is_string());
        assert!(v["sequence"].is_string());
        assert!(v["supply_snapshot"].is_string());
        assert!(v["snapshot_slot"].is_string());
        assert_eq!(v["hash_alg"], "SHA-256");
        assert_eq!(v["currency"], "BRL");
        assert_eq!(v["issued_at"], "2026-08-01T12:00:00Z");
        assert_eq!(v["mint"], bs58::encode([9u8; 32]).into_string());
        assert_eq!(v["merkle_root"], hex::encode(msg.merkle_root));
        assert_eq!(v["dataset_sha256"], "07".repeat(32));
        assert_eq!(v["ipfs_cid"], "bafytestcid");
    }

    #[test]
    fn canonical_bytes_is_deterministic_over_same_snapshot() {
        assert_eq!(
            sample_message().canonical_bytes(),
            sample_message().canonical_bytes()
        );
    }
}
