use crate::hash::{MMR_BAG_DOMAIN, MMR_LEAF_DOMAIN, MMR_NODE_DOMAIN};
use sha2::{Digest, Sha256};

pub fn mmr_leaf_hash(value: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([MMR_LEAF_DOMAIN]);
    h.update(value);
    h.finalize().into()
}

pub fn mmr_node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([MMR_NODE_DOMAIN]);
    h.update(left);
    h.update(right);
    h.finalize().into()
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Mmr {
    pub peaks: Vec<[u8; 32]>,
    pub leaf_count: u64,
}

impl Mmr {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, value: &[u8; 32]) {
        let mut node = mmr_leaf_hash(value);
        let mut n = self.leaf_count;
        while n & 1 == 1 {
            let left = self
                .peaks
                .pop()
                .expect("peak invariant: one peak per set bit");
            node = mmr_node_hash(&left, &node);
            n >>= 1;
        }
        self.peaks.push(node);
        self.leaf_count += 1;
    }

    pub fn bag_peaks(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update([MMR_BAG_DOMAIN]);
        h.update(self.leaf_count.to_be_bytes());
        for peak in &self.peaks {
            h.update(peak);
        }
        h.finalize().into()
    }
}
