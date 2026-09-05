use sha2::{Digest, Sha256};

/// Domain octets keep leaves, internal nodes and the MMR layer in disjoint
/// preimage spaces.
pub const LEAF_DOMAIN: u8 = 0x00;
pub const NODE_DOMAIN: u8 = 0x01;
pub const MMR_LEAF_DOMAIN: u8 = 0x02;
pub const MMR_NODE_DOMAIN: u8 = 0x03;
pub const MMR_BAG_DOMAIN: u8 = 0x04;

/// SHA-256(0x00 || account_id[32] || balance[u64 BE]).
pub fn leaf_hash(account_id: &[u8; 32], balance: u64) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([LEAF_DOMAIN]);
    h.update(account_id);
    h.update(balance.to_be_bytes());
    h.finalize().into()
}

/// SHA-256(0x01 || sum_left[u64 BE] || sum_right[u64 BE] || h_left || h_right).
pub fn node_hash(sum_left: u64, sum_right: u64, h_left: &[u8; 32], h_right: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([NODE_DOMAIN]);
    h.update(sum_left.to_be_bytes());
    h.update(sum_right.to_be_bytes());
    h.update(h_left);
    h.update(h_right);
    h.finalize().into()
}
