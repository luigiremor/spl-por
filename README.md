# spl-por

Proof of concept of a proof-of-reserves (PoR) infrastructure for
BRL-denominated stablecoins on Solana: an attester commits a reserve snapshot
into a Merkle sum tree, anchors the Ed25519-signed root on-chain, and any
third party re-verifies signature, inclusion, per-node binding and supply
coverage without trusting a privileged party.

> **Academic proof of concept.** This code accompanies an undergraduate
> thesis (Information Systems, UFSC, 2026) on proof of reserves for BRL
> stablecoins on Solana. The monograph is the normative specification for
> every rule implemented here. Not audited; do not use in production.

## Layout

- `crates/sum-tree` — Merkle sum tree (domain separation, checked sums,
  inclusion proofs), MMR accumulator and canonical attestation
  serialization (binary struct + JCS manifest).
- `programs/por` — Anchor program (planned).
- `crates/attester`, `crates/verifier`, `api/`, `scenarios/` — planned.

## License

MIT.
