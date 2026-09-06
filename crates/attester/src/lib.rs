//! Reference attester for spl-por. NOT part of the thesis contribution: it
//! only exercises the public oracle interface (program + sum-tree crate).
//! The program and the verifier treat it as untrusted.
pub mod attestation;
pub mod cid;
pub mod ipfs;
pub mod keys;
pub mod snapshot;
pub mod tx;
