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
- `crates/verifier` — public verifier CLI `por-verify`: from public data
  only (on-chain accounts + published dataset) re-executes the four chained
  checks of the monograph (§4.4: strict Ed25519 signature under the recorded
  `signer_pubkey`, per-account inclusion, per-node binding over the whole
  tree, coverage `R_norm >= S` confronted with the recorded `status`) plus
  the MMR history consistency against `LogState`. Reuses `sum-tree` and the
  program's account structs; never trusts the attester.
- `api/`, `scenarios/` — planned.

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
repo, deploys the program, creates a mint (`decimals = 2`, fixed test keys
under `crates/verifier/tests/fixtures/keys`), runs `initialize_config`,
submits a conforming attestation (`status OK`), an under-collateralized one
(`--haircut-bps 100`, `status DIVERGENTE`) and a third one, then runs
`por-verify` on every sequence (see below). Needs `ipfs` (kubo),
`spl-token`, `jq` and `curl` on PATH, plus `target/deploy/por-keypair.json`
matching the id in `Anchor.toml`. `RECORD_FIXTURES=1` re-records the
verifier's frozen fixtures from that run.

```sh
por-attest snapshot --n 100 --seed 42 [--haircut-bps 0] --out DIR
por-attest init-config --mint MINT --keypair KP [--rpc URL]
por-attest submit --dataset DIR/dataset.jsonl --out DIR --mint MINT --keypair KP [--rpc URL] [--sequence N]
por-attest show --mint MINT --sequence N [--rpc URL]
```

## Public verification (etapa 4)

```sh
por-verify attestation --mint MINT --sequence N [--rpc URL] [--dataset DIR|CID]
    [--signature PATH] [--gateway URL] [--program-id ID] [--account HEX64]
    [--fixtures FILE] [--json]
```

- `--dataset`: local directory with `dataset.jsonl` (+ `signature.json`) or
  an IPFS CID fetched from `--gateway` (default: the `ipfs_cid` anchored in
  the `AuditRecord`). The CID addresses only the dataset bytes, so a CID
  source needs `--signature PATH` to the `signature.json` sidecar.
- `--account`: restrict the inclusion check to one `account_id`; default
  verifies an inclusion proof for every leaf.
- `--fixtures`: read accounts from a recorded JSON file instead of the RPC
  (offline tests; see `crates/verifier/tests/fixtures`).
- Exit codes: `0` approved (a coherent `DIVERGENTE` record **is** approved;
  the report says `status_onchain: DIVERGENTE`), `1` rejected (any check
  fails, including `dataset_sha256` mismatch or tampered status), `2`
  operational error (RPC down, unknown sequence, bad arguments, dataset
  unavailable — reported as `VERIFICATION FAILURE`, never tacit approval).
- `--json` emits the `por-verify/1` report: `program_id`, `mint`,
  `sequence`, `verdict` (`APPROVED`/`REJECTED`), `status_onchain`
  (`OK`/`DIVERGENTE`), `checks[]` (`dataset_integrity`, `signature`,
  `inclusion`, `binding`, `coverage`, `mmr_consistency`, each with `passed`
  and `detail`) and `new_observation` (current supply read when no
  historical read at `ingestion_slot` is available). Golden copy in
  `crates/verifier/tests/fixtures/golden_report.json`.

## License

MIT.
