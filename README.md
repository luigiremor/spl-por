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
- `crates/attester` — reference attester CLI `por-attest` (**not part of the
  contribution**; exercises the oracle interface): deterministic synthetic
  snapshot, canonical JSONL dataset, sum tree + JCS manifest via `sum-tree`,
  Ed25519 signature, IPFS publish (local kubo, fixed CIDv1 profile) and the
  two-instruction `submit_attestation` transaction.
- `crates/verifier`, `api/`, `scenarios/` — planned.

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

## End-to-end on localnet (etapa 3)

`./scripts/e2e-localnet.sh` boots a fresh validator and a throwaway kubo
repo, deploys the program, creates a mint (`decimals = 2`), runs
`initialize_config`, submits a conforming attestation (`status OK`) and then
an under-collateralized one (`--haircut-bps 100`, `status DIVERGENTE`).
Needs `ipfs` (kubo), `spl-token` and `jq` on PATH, plus
`target/deploy/por-keypair.json` matching the id in `Anchor.toml`.

```sh
por-attest snapshot --n 100 --seed 42 [--haircut-bps 0] --out DIR
por-attest init-config --mint MINT --keypair KP [--rpc URL]
por-attest submit --dataset DIR/dataset.jsonl --out DIR --mint MINT --keypair KP [--rpc URL] [--sequence N]
por-attest show --mint MINT --sequence N [--rpc URL]
```

## License

MIT.
