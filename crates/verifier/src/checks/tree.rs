use crate::report::CheckResult;
use sum_tree::{build, prove, verify_inclusion};

pub struct TreeChecks {
    pub binding: CheckResult,
    pub inclusion: CheckResult,
}

/// Checks 2 and 3 (§4.4.2–§4.4.3). Rebuilds the WHOLE tree with
/// sum_tree::build (strict ordering, checked sums, per-value range — the
/// producer's own rules) and confronts root/total with the anchored values;
/// then re-applies the per-node binding on inclusion proofs through
/// verify_inclusion. `account` restricts inclusion to one account_id; without
/// it every leaf is checked.
pub fn check_tree(
    leaves: &[([u8; 32], u64)],
    expected_root: &[u8; 32],
    expected_total: u64,
    max_value: u64,
    account: Option<[u8; 32]>,
) -> TreeChecks {
    let tree = match build(leaves, max_value) {
        Ok(t) => t,
        Err(e) => {
            // Broken ordering, duplicate, out-of-range balance or sum overflow:
            // the published dataset admits no canonical tree -> rejected.
            return TreeChecks {
                binding: CheckResult::fail("binding", format!("tree reconstruction rejected: {e}")),
                inclusion: CheckResult::fail("inclusion", "tree not reconstructed"),
            };
        }
    };
    let binding = if &tree.root_hash != expected_root {
        CheckResult::fail(
            "binding",
            format!(
                "recomputed root {} differs from anchored merkle_root {}",
                hex::encode(tree.root_hash),
                hex::encode(expected_root)
            ),
        )
    } else if tree.total != expected_total {
        CheckResult::fail(
            "binding",
            format!(
                "recomputed total {} differs from attested total_reservas {}",
                tree.total, expected_total
            ),
        )
    } else {
        CheckResult::pass(
            "binding",
            format!(
                "root and total {} confirmed over {} leaves (checked sums, max_value {})",
                tree.total,
                leaves.len(),
                max_value
            ),
        )
    };

    let targets: Vec<usize> = match account {
        Some(id) => match leaves.iter().position(|(c, _)| *c == id) {
            Some(i) => vec![i],
            None => {
                return TreeChecks {
                    binding,
                    inclusion: CheckResult::fail(
                        "inclusion",
                        format!(
                            "requested account_id {} not present in dataset",
                            hex::encode(id)
                        ),
                    ),
                }
            }
        },
        None => (0..leaves.len()).collect(),
    };
    let count = targets.len();
    for i in targets {
        let (c, b) = leaves[i];
        let proof = prove(&tree, i);
        if let Err(e) = verify_inclusion(&c, b, &proof, expected_root, expected_total, max_value) {
            return TreeChecks {
                binding,
                inclusion: CheckResult::fail(
                    "inclusion",
                    format!("inclusion proof for leaf {i} rejected: {e}"),
                ),
            };
        }
    }
    TreeChecks {
        binding,
        inclusion: CheckResult::pass(
            "inclusion",
            format!("{count} inclusion proof(s) verified against anchored root and total"),
        ),
    }
}
