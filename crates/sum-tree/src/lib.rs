//! Cryptographic library for a proof-of-reserves proof of concept:
//! Merkle sum tree with per-node binding, append-only MMR accumulator and
//! canonical serialization of attestation messages (fixed-width binary
//! struct signed on-chain + JCS manifest published off-chain).
//!
//! Academic prototype; not audited, not production-ready.

mod hash;

pub use hash::{
    leaf_hash, node_hash, LEAF_DOMAIN, MMR_BAG_DOMAIN, MMR_LEAF_DOMAIN, MMR_NODE_DOMAIN,
    NODE_DOMAIN,
};
