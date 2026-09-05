use sha2::{Digest, Sha256};

/// One reserve account: opaque 32-byte id + balance in cents (u64).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotEntry {
    pub account_id: [u8; 32],
    pub balance: u64,
}

/// Deterministic synthetic snapshot: SHA-256 counter-mode PRF keyed by `seed`
/// (`SHA256(seed_be || label || i_be)`). No RNG crate: reproducibility is
/// guaranteed by the hash function itself. `haircut_bps` applies an integer
/// basis-point discount to every balance (under-collateralized scenario;
/// 0 = identity).
pub fn generate_snapshot(n: u32, seed: u64, haircut_bps: u16) -> Vec<SnapshotEntry> {
    assert!(n >= 1, "n must be >= 1");
    assert!(haircut_bps <= 10_000, "haircut_bps must be <= 10000");
    let prf = |label: &[u8], i: u32| -> [u8; 32] {
        Sha256::new()
            .chain_update(seed.to_be_bytes())
            .chain_update(label)
            .chain_update(u64::from(i).to_be_bytes())
            .finalize()
            .into()
    };
    let mut entries: Vec<SnapshotEntry> = (0..n)
        .map(|i| {
            let account_id = prf(b"account", i);
            let digest = prf(b"balance", i);
            let raw = u64::from_be_bytes(digest[0..8].try_into().expect("8 bytes"));
            // balance in [1, 100_000_000] cents (<= R$ 1.000.000,00 per account)
            let balance = 1 + (raw % 100_000_000);
            let balance = balance * u64::from(10_000 - haircut_bps) / 10_000;
            SnapshotEntry {
                account_id,
                balance,
            }
        })
        .collect();
    entries.sort_by(|a, b| a.account_id.cmp(&b.account_id));
    // SHA-256 collision among ids has negligible probability; dedup keeps the
    // strictly-increasing invariant that sum_tree::build enforces.
    entries.dedup_by(|a, b| a.account_id == b.account_id);
    entries
}

/// Canonical JSONL serialization (FIXED format — dataset_sha256 depends on it):
/// one leaf per line, strictly increasing account_id, lexicographic keys,
/// no whitespace, lowercase hex, u64 as JSON string, LF after every line.
pub fn serialize_dataset(entries: &[SnapshotEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    for e in entries {
        out.extend_from_slice(
            format!(
                "{{\"account_id\":\"{}\",\"balance\":\"{}\"}}\n",
                hex::encode(e.account_id),
                e.balance
            )
            .as_bytes(),
        );
    }
    out
}

/// Strict parser: rejects malformed lines, non-hex ids, non-u64 balances and
/// any violation of the strictly-increasing order. Re-serialization of the
/// result must be byte-identical to the input (canonical fixed point).
pub fn parse_dataset(bytes: &[u8]) -> anyhow::Result<Vec<SnapshotEntry>> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Line {
        account_id: String,
        balance: String,
    }
    let text = std::str::from_utf8(bytes)?;
    let mut entries = Vec::new();
    let mut prev: Option<[u8; 32]> = None;
    for (idx, line) in text.lines().enumerate() {
        let n = idx + 1;
        let l: Line = serde_json::from_str(line)
            .map_err(|e| anyhow::anyhow!("line {n}: invalid JSON: {e}"))?;
        let account_id: [u8; 32] = hex::decode(&l.account_id)
            .map_err(|e| anyhow::anyhow!("line {n}: invalid hex account_id: {e}"))?
            .try_into()
            .map_err(|_| anyhow::anyhow!("line {n}: account_id must be 32 bytes"))?;
        let balance: u64 = l
            .balance
            .parse()
            .map_err(|e| anyhow::anyhow!("line {n}: invalid u64 balance: {e}"))?;
        if let Some(p) = prev {
            anyhow::ensure!(
                account_id > p,
                "line {n}: account_id not strictly increasing"
            );
        }
        prev = Some(account_id);
        entries.push(SnapshotEntry {
            account_id,
            balance,
        });
    }
    anyhow::ensure!(!entries.is_empty(), "dataset must have at least one leaf");
    Ok(entries)
}

/// SHA-256 over the exact bytes of the canonical dataset file.
pub fn dataset_sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_is_deterministic_for_same_seed() {
        let a = generate_snapshot(50, 42, 0);
        let b = generate_snapshot(50, 42, 0);
        assert_eq!(a, b);
        let c = generate_snapshot(50, 43, 0);
        assert_ne!(a, c, "different seed must yield a different snapshot");
    }

    #[test]
    fn account_ids_are_strictly_increasing_and_unique() {
        let entries = generate_snapshot(1000, 7, 0);
        for w in entries.windows(2) {
            assert!(
                w[0].account_id < w[1].account_id,
                "dataset order must be strictly increasing"
            );
        }
    }

    #[test]
    fn balances_are_in_declared_range() {
        let entries = generate_snapshot(1000, 7, 0);
        for e in &entries {
            assert!(e.balance >= 1 && e.balance <= 100_000_000);
        }
    }

    #[test]
    fn haircut_strictly_reduces_total() {
        let full: u64 = generate_snapshot(100, 42, 0)
            .iter()
            .map(|e| e.balance)
            .sum();
        let cut: u64 = generate_snapshot(100, 42, 100)
            .iter()
            .map(|e| e.balance)
            .sum();
        assert!(cut < full, "100 bps haircut must strictly reduce the total");
    }

    #[test]
    fn serialize_parse_roundtrip_is_byte_identical() {
        let entries = generate_snapshot(10, 42, 0);
        let bytes = serialize_dataset(&entries);
        let parsed = parse_dataset(&bytes).unwrap();
        assert_eq!(parsed, entries);
        assert_eq!(
            serialize_dataset(&parsed),
            bytes,
            "canonical form must be a fixed point"
        );
    }

    #[test]
    fn parse_rejects_out_of_order_dataset() {
        let mut entries = generate_snapshot(5, 42, 0);
        entries.swap(0, 1);
        let bytes = serialize_dataset(&entries);
        assert!(
            parse_dataset(&bytes).is_err(),
            "non-increasing order must be rejected"
        );
    }

    #[test]
    fn dataset_sha256_known_answer_n3_seed42() {
        let bytes = serialize_dataset(&generate_snapshot(3, 42, 0));
        // Frozen known-answer (generate-and-freeze): obtained once with
        // --nocapture and pasted here. Freezes the dataset format: any change
        // in serialization or generation breaks this test — by design.
        println!(
            "dataset_sha256(n=3, seed=42) = {}",
            hex::encode(dataset_sha256(&bytes))
        );
        assert_eq!(
            hex::encode(dataset_sha256(&bytes)),
            "8691be1bb3e7ef581ab75f89cc52d9884a9a4d437785cf4682e29f1f9680c899"
        );
    }
}
