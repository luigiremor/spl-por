use crate::hash::{leaf_hash, node_hash};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
    Empty,
    NotStrictlyIncreasing { index: usize },
    BalanceAboveMax { index: usize },
    SumOverflow,
    SumAboveMax,
}

impl core::fmt::Display for TreeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TreeError::Empty => write!(f, "attestation requires at least one leaf"),
            TreeError::NotStrictlyIncreasing { index } => {
                write!(f, "account_id at index {index} is not strictly increasing")
            }
            TreeError::BalanceAboveMax { index } => {
                write!(f, "balance at index {index} exceeds max_value")
            }
            TreeError::SumOverflow => write!(f, "subtotal overflows u64"),
            TreeError::SumAboveMax => write!(f, "subtotal exceeds max_value"),
        }
    }
}

impl std::error::Error for TreeError {}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum TreeNode {
    Leaf {
        hash: [u8; 32],
        sum: u64,
    },
    Internal {
        hash: [u8; 32],
        sum: u64,
        leaf_count: usize,
        left: Box<TreeNode>,
        right: Box<TreeNode>,
    },
}

impl TreeNode {
    pub(crate) fn hash(&self) -> &[u8; 32] {
        match self {
            TreeNode::Leaf { hash, .. } | TreeNode::Internal { hash, .. } => hash,
        }
    }

    pub(crate) fn sum(&self) -> u64 {
        match self {
            TreeNode::Leaf { sum, .. } | TreeNode::Internal { sum, .. } => *sum,
        }
    }

    pub(crate) fn leaf_count(&self) -> usize {
        match self {
            TreeNode::Leaf { .. } => 1,
            TreeNode::Internal { leaf_count, .. } => *leaf_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SumTree {
    pub root_hash: [u8; 32],
    pub total: u64,
    pub(crate) root: TreeNode,
}

impl SumTree {
    pub fn leaf_count(&self) -> usize {
        self.root.leaf_count()
    }
}

pub fn build(leaves: &[([u8; 32], u64)], max_value: u64) -> Result<SumTree, TreeError> {
    if leaves.is_empty() {
        return Err(TreeError::Empty);
    }
    for (i, (account_id, balance)) in leaves.iter().enumerate() {
        if i > 0 && leaves[i - 1].0 >= *account_id {
            return Err(TreeError::NotStrictlyIncreasing { index: i });
        }
        if *balance > max_value {
            return Err(TreeError::BalanceAboveMax { index: i });
        }
    }
    let root = build_node(leaves, max_value)?;
    Ok(SumTree {
        root_hash: *root.hash(),
        total: root.sum(),
        root,
    })
}

fn build_node(leaves: &[([u8; 32], u64)], max_value: u64) -> Result<TreeNode, TreeError> {
    if let [(account_id, balance)] = leaves {
        return Ok(TreeNode::Leaf {
            hash: leaf_hash(account_id, *balance),
            sum: *balance,
        });
    }
    let split = largest_power_of_two_below(leaves.len());
    let left = build_node(&leaves[..split], max_value)?;
    let right = build_node(&leaves[split..], max_value)?;
    let sum = left
        .sum()
        .checked_add(right.sum())
        .ok_or(TreeError::SumOverflow)?;
    if sum > max_value {
        return Err(TreeError::SumAboveMax);
    }
    let hash = node_hash(left.sum(), right.sum(), left.hash(), right.hash());
    Ok(TreeNode::Internal {
        hash,
        sum,
        leaf_count: leaves.len(),
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn largest_power_of_two_below(n: usize) -> usize {
    debug_assert!(n >= 2);
    1usize << (n - 1).ilog2()
}
