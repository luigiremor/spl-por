//! Reference attester for spl-por. NOT part of the thesis contribution: it
//! only exercises the public oracle interface (program + sum-tree crate).
//! The program and the verifier treat it as untrusted.
pub mod attestation;
pub mod keys;
pub mod snapshot;
