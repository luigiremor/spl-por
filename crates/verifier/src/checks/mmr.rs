use crate::report::CheckResult;
use por::state::LogState;
use sum_tree::Mmr;

/// Accumulator consistency (§5.2, stage 4): rebuilds the expected MMR by
/// appending the merkle_root of every anchored attestation (sequences
/// 0..next_sequence, in order; the crate applies the 0x02 leaf domain) and
/// compares peaks + leaf_count with the anchored LogState. Detects a forged
/// append and inconsistent peaks.
pub fn check_mmr(history_roots: &[[u8; 32]], log: &LogState) -> CheckResult {
    let mut mmr = Mmr::new();
    for root in history_roots {
        mmr.append(root);
    }
    if mmr.leaf_count != log.leaf_count {
        return CheckResult::fail(
            "mmr_consistency",
            format!(
                "recomputed leaf_count {} differs from LogState leaf_count {}",
                mmr.leaf_count, log.leaf_count
            ),
        );
    }
    let onchain = log.peaks.get(..usize::from(log.peak_count)).unwrap_or(&[]);
    if mmr.peaks.as_slice() != onchain {
        return CheckResult::fail(
            "mmr_consistency",
            "recomputed MMR peaks differ from anchored LogState peaks",
        );
    }
    CheckResult::pass(
        "mmr_consistency",
        format!(
            "{} attestation(s) chained; {} peak(s) match LogState",
            history_roots.len(),
            mmr.peaks.len()
        ),
    )
}
