//! Parametric synthetic datasets (monograph §5.3). Reuses the attester's
//! deterministic snapshot generator (SHA-256 counter-mode PRF keyed by the
//! seed) and its canonical JSONL serialization, so `dataset_sha256` is
//! byte-identical on any machine and the verifier consumes the very same
//! format the attester publishes.
use attester::snapshot::{self, SnapshotEntry};

/// Seed of every dataset of the evaluation.
pub const SEED: u64 = 42;
/// Cardinalities N of the scalability axis.
pub const SIZES: [usize; 5] = [10, 100, 1_000, 10_000, 100_000];
/// Range ceiling passed to `initialize_config` and to `sum_tree::build`:
/// 10^14 cents (R$ 1 trilhão). The attester's balances lie in [1, 10^8]
/// cents, so N = 100 000 accounts sum to at most 10^13 < max_value.
pub const MAX_VALUE: u64 = 100_000_000_000_000;

/// `n` pairs (account_id, balance in cents), strictly increasing by
/// account_id, deterministic in `seed`.
pub fn generate(n: usize, seed: u64) -> Vec<([u8; 32], u64)> {
    snapshot::generate_snapshot(u32::try_from(n).expect("n fits u32"), seed, 0)
        .into_iter()
        .map(|e| (e.account_id, e.balance))
        .collect()
}

/// Canonical JSONL bytes (the attester's fixed format); `dataset_sha256` is
/// computed over exactly these bytes.
pub fn serialize_dataset(rows: &[([u8; 32], u64)]) -> Vec<u8> {
    let entries: Vec<SnapshotEntry> = rows
        .iter()
        .map(|(account_id, balance)| SnapshotEntry {
            account_id: *account_id,
            balance: *balance,
        })
        .collect();
    snapshot::serialize_dataset(&entries)
}
