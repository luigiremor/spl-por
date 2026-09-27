#!/usr/bin/env bash
# Evaluation driver on localnet (monograph §5.4): boots a fresh validator,
# deploys `por`, then runs whatever command line follows (default: the whole
# scenario matrix). Examples:
#   ./scripts/eval-localnet.sh                       # matrix S01-S16 -> results/matrix-localnet.csv
#   ./scripts/eval-localnet.sh cargo run --release -p scenarios --bin por-measure
# Prereqs on PATH: cargo, anchor, solana, solana-keygen, solana-test-validator, ipfs (kubo).
# target/deploy/por-keypair.json (gitignored) must match the id in Anchor.toml.
set -euo pipefail
cd "$(dirname "$0")/.."

RPC=http://127.0.0.1:8899
WORK=$(mktemp -d)
export IPFS_PATH="$WORK/ipfs"   # kubo repo for multi-block datasets (N >= 10k)
PROGRAM_ID=$(sed -n 's/^por = "\(.*\)"/\1/p' Anchor.toml)
DEPLOYER="$WORK/deployer.json"

for bin in cargo anchor solana solana-keygen solana-test-validator ipfs; do
  command -v "$bin" >/dev/null || { echo "[eval] FAIL: missing prerequisite: $bin"; exit 1; }
done
[ -f target/deploy/por.so ] && [ "$(solana-keygen pubkey target/deploy/por-keypair.json)" = "$PROGRAM_ID" ] \
  || { echo "[eval] FAIL: target/deploy/{por.so,por-keypair.json} missing or not $PROGRAM_ID (run anchor build)"; exit 1; }

cleanup() { kill "${VALIDATOR_PID:-0}" 2>/dev/null || true; rm -rf "$WORK"; }
trap cleanup EXIT

echo "[eval] versions: $(solana-test-validator --version | head -1); $(anchor --version); $(ipfs version)"
ipfs init --profile=test >/dev/null 2>&1
solana-keygen new --no-bip39-passphrase --silent -o "$DEPLOYER"

echo "[eval] starting solana-test-validator"
solana-test-validator --reset --ledger "$WORK/ledger" --quiet &
VALIDATOR_PID=$!
until solana -u "$RPC" slot >/dev/null 2>&1; do sleep 1; done
solana -u "$RPC" airdrop 10 "$(solana-keygen pubkey "$DEPLOYER")" >/dev/null
echo "[eval] deploying program"
anchor deploy --provider.cluster "$RPC" --provider.wallet "$DEPLOYER" | grep -E 'Program Id|Deploy success'

export POR_RPC="$RPC" POR_CLUSTER=localnet POR_RECORD=1
if [ $# -eq 0 ]; then
  rm -f results/matrix-localnet.csv
  cargo test -p scenarios -- --ignored --test-threads=1
  cargo test -p scenarios --test matrix_lib
else
  "$@"
fi
echo "[eval] done"
