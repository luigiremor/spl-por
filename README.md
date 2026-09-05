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
- `programs/por` — Anchor program: `initialize_config`, `submit_attestation`
  (Ed25519 precompile + Instructions-sysvar introspection, own supply read,
  coverage reconciliation, MMR append), `set_oracle_authority` and the
  optional `verify_inclusion`. Tests run in Rust on LiteSVM.
- `crates/attester`, `crates/verifier`, `api/`, `scenarios/` — planned.

## Build and test

Requires `anchor-cli` 0.32.x and `solana-cli` 2.2.x. The program pins
platform-tools `v1.56` (Rust 1.89) through `package.metadata.solana` in
`programs/por/Cargo.toml`, because the default `v1.48` cannot parse the
edition-2024 crates in the dependency graph.

```sh
anchor build            # produces target/deploy/por.so (needed by the SVM tests)
cargo test --workspace  # sum-tree unit tests + por LiteSVM tests
```

## Localnet smoke test (etapa 2)

1. `./scripts/localnet.sh` — starts a fresh `solana-test-validator` and
   deploys the `por` program. Note the program id printed by `anchor keys list`.
2. In another terminal, confirm the program on-chain:
   `solana program show <PROGRAM_ID> --url http://127.0.0.1:8899`
   (lists the executable, the BPF upgradeable loader as owner and the deploy slot).
3. Follow logs: `solana logs <PROGRAM_ID> --url http://127.0.0.1:8899`.
4. The SVM suite (`cargo test -p por`) covers the instruction behaviour; the
   first end-to-end attestation on localnet (Ed25519 ix + submit via CLI) is
   the milestone of etapa 3 (attester).

Record in the smoke: `solana-test-validator` version, program id and deploy
slot (they become environment parameters of the evaluation).

## License

MIT.
