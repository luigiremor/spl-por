//! Cryptographic library for a proof-of-reserves proof of concept:
//! Merkle sum tree with per-node binding, append-only MMR accumulator and
//! canonical serialization of attestation messages (fixed-width binary
//! struct signed on-chain + JCS manifest published off-chain).
//!
//! Academic prototype; not audited, not production-ready.
