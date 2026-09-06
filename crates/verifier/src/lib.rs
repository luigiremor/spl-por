//! Public proof-of-reserves verifier (`por-verify`). Re-executes, from public
//! data only (on-chain accounts + published dataset), the four chained checks
//! of the monograph (§4.4) plus the MMR history consistency, and confronts the
//! recomputed verdict with the `status` anchored in the `AuditRecord`.
//!
//! Academic prototype; not audited, not production-ready.
pub mod cli;
