use crate::hash::{leaf_hash, node_hash};
use crate::tree::{SumTree, TreeNode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofStep {
    pub sibling_hash: [u8; 32],
    pub sibling_sum: u64,
    pub sibling_is_left: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InclusionProof(pub Vec<ProofStep>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    ValueAboveMax,
    SumOverflow,
    RootMismatch,
    TotalMismatch,
}

impl core::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VerifyError::ValueAboveMax => write!(f, "balance or subtotal exceeds max_value"),
            VerifyError::SumOverflow => write!(f, "subtotal overflows u64"),
            VerifyError::RootMismatch => write!(f, "recomputed root does not match"),
            VerifyError::TotalMismatch => write!(f, "recomputed total does not match"),
        }
    }
}

impl std::error::Error for VerifyError {}

/// Builds a bottom-up inclusion path for the leaf at `index`.
///
/// # Panics
/// Panics if `index >= tree.leaf_count()`.
pub fn prove(tree: &SumTree, index: usize) -> InclusionProof {
    assert!(index < tree.leaf_count(), "leaf index out of range");
    let mut steps = Vec::new();
    let mut node = &tree.root;
    let mut i = index;
    while let TreeNode::Internal { left, right, .. } = node {
        if i < left.leaf_count() {
            steps.push(ProofStep {
                sibling_hash: *right.hash(),
                sibling_sum: right.sum(),
                sibling_is_left: false,
            });
            node = left.as_ref();
        } else {
            i -= left.leaf_count();
            steps.push(ProofStep {
                sibling_hash: *left.hash(),
                sibling_sum: left.sum(),
                sibling_is_left: true,
            });
            node = right.as_ref();
        }
    }
    steps.reverse();
    InclusionProof(steps)
}

pub fn verify_inclusion(
    account_id: &[u8; 32],
    balance: u64,
    proof: &InclusionProof,
    root_hash: &[u8; 32],
    total: u64,
    max_value: u64,
) -> Result<(), VerifyError> {
    if balance > max_value {
        return Err(VerifyError::ValueAboveMax);
    }
    let mut hash = leaf_hash(account_id, balance);
    let mut sum = balance;
    for step in &proof.0 {
        if step.sibling_sum > max_value {
            return Err(VerifyError::ValueAboveMax);
        }
        let (sum_left, sum_right, h_left, h_right) = if step.sibling_is_left {
            (step.sibling_sum, sum, &step.sibling_hash, &hash)
        } else {
            (sum, step.sibling_sum, &hash, &step.sibling_hash)
        };
        let parent_sum = sum_left
            .checked_add(sum_right)
            .ok_or(VerifyError::SumOverflow)?;
        if parent_sum > max_value {
            return Err(VerifyError::ValueAboveMax);
        }
        hash = node_hash(sum_left, sum_right, h_left, h_right);
        sum = parent_sum;
    }
    if hash != *root_hash {
        return Err(VerifyError::RootMismatch);
    }
    if sum != total {
        return Err(VerifyError::TotalMismatch);
    }
    Ok(())
}
