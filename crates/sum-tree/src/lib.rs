//! Cryptographic library for a proof-of-reserves proof of concept:
//! Merkle sum tree with per-node binding, append-only MMR accumulator and
//! canonical serialization of attestation messages (fixed-width binary
//! struct signed on-chain + JCS manifest published off-chain).
//!
//! Academic prototype; not audited, not production-ready.

mod attestation;
mod hash;
mod mmr;
mod proof;
mod tree;

pub use attestation::AttestationMessage;
pub use hash::{
    leaf_hash, node_hash, LEAF_DOMAIN, MMR_BAG_DOMAIN, MMR_LEAF_DOMAIN, MMR_NODE_DOMAIN,
    NODE_DOMAIN,
};
pub use mmr::{mmr_leaf_hash, mmr_node_hash, Mmr};
pub use proof::{prove, verify_inclusion, InclusionProof, ProofStep, VerifyError};
pub use tree::{build, SumTree, TreeError};
