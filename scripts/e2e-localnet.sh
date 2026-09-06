#!/usr/bin/env bash
# End-to-end test on localnet for spl-por:
#   deploy -> initialize_config -> conforming attestation (status OK)
#   -> under-collateralized snapshot (status DIVERGENTE).
# Prereqs on PATH: cargo, anchor, solana, solana-keygen, solana-test-validator,
#                  spl-token, ipfs (kubo), jq.
# The program keypair target/deploy/por-keypair.json (gitignored) must match
# the id in Anchor.toml; otherwise anchor build would mint a fresh id.
set -euo pipefail
cd "$(dirname "$0")/.."

RPC=http://127.0.0.1:8899
WORK=$(mktemp -d)
export IPFS_PATH="$WORK/ipfs"
LEDGER="$WORK/ledger"
KEYPAIR="$WORK/oracle.json"
ATTEST=target/debug/por-attest
PROGRAM_ID=$(sed -n 's/^por = "\(.*\)"/\1/p' Anchor.toml)

for bin in cargo anchor solana solana-keygen solana-test-validator spl-token ipfs jq; do
  command -v "$bin" >/dev/null || { echo "[e2e] FAIL: missing prerequisite: $bin"; exit 1; }
done
[ -f target/deploy/por-keypair.json ] && [ "$(solana-keygen pubkey target/deploy/por-keypair.json)" = "$PROGRAM_ID" ] \
  || { echo "[e2e] FAIL: target/deploy/por-keypair.json missing or not $PROGRAM_ID"; exit 1; }

cleanup() {
  kill "${VALIDATOR_PID:-0}" 2>/dev/null || true
  rm -rf "$WORK"
}
trap cleanup EXIT

echo "[e2e] versions: $(solana-test-validator --version | head -1); $(anchor --version); $(spl-token --version); $(ipfs version)"
echo "[e2e] building workspace"
cargo build -q -p attester
anchor build > "$WORK/anchor-build.log" 2>&1 || { cat "$WORK/anchor-build.log"; exit 1; }

echo "[e2e] initializing throwaway kubo repo (no daemon needed: ipfs add works on the repo)"
ipfs init --profile=test >/dev/null

echo "[e2e] starting solana-test-validator"
solana-test-validator --reset --ledger "$LEDGER" --quiet &
VALIDATOR_PID=$!
until solana -u "$RPC" slot >/dev/null 2>&1; do sleep 1; done
echo "[e2e] validator ready"

solana-keygen new --no-bip39-passphrase --silent -o "$KEYPAIR"
ORACLE=$(solana-keygen pubkey "$KEYPAIR")
solana -u "$RPC" airdrop 10 "$ORACLE" >/dev/null

echo "[e2e] deploying program"
anchor deploy --provider.cluster "$RPC" --provider.wallet "$KEYPAIR" | grep -E 'Program Id|Deploy success'

echo "[e2e] creating mint (decimals=2) and token account"
# spl-token-cli 5.3.0: --mint-authority/--owner go AFTER the subcommand;
# JSON field confirmed on 2026-09-05: .commandOutput.address
MINT=$(spl-token -u "$RPC" --fee-payer "$KEYPAIR" create-token --decimals 2 \
  --mint-authority "$KEYPAIR" --output json | jq -r '.commandOutput.address')
spl-token -u "$RPC" --fee-payer "$KEYPAIR" create-account "$MINT" --owner "$KEYPAIR" >/dev/null
echo "[e2e] mint: $MINT"

echo "[e2e] initialize_config"
"$ATTEST" init-config --mint "$MINT" --keypair "$KEYPAIR" --rpc "$RPC"

echo "[e2e] scenario 1: conforming snapshot (n=100, seed=42)"
SNAP1=$("$ATTEST" snapshot --n 100 --seed 42 --out "$WORK/s1")
echo "[e2e] snapshot: $SNAP1"
TOTAL=$(echo "$SNAP1" | jq -r '.total')
UI_AMOUNT=$(printf '%d.%02d' $((TOTAL / 100)) $((TOTAL % 100)))
echo "[e2e] minting supply = $UI_AMOUNT (R_norm == S; coverage >= holds, so OK)"
spl-token -u "$RPC" --fee-payer "$KEYPAIR" mint "$MINT" "$UI_AMOUNT" \
  --recipient-owner "$ORACLE" --mint-authority "$KEYPAIR" >/dev/null

SUB0=$("$ATTEST" submit --dataset "$WORK/s1/dataset.jsonl" --out "$WORK/s1" \
  --mint "$MINT" --keypair "$KEYPAIR" --rpc "$RPC")
echo "[e2e] submit[0]: $SUB0"
echo "[e2e] compute: $(solana -u "$RPC" confirm -v "$(echo "$SUB0" | jq -r '.signature')" | grep -E 'consumed' | tr -s ' ' | paste -sd ';' -)"
echo "[e2e] manifest[0]: $(cat "$WORK/s1/manifest.json")"
REC0=$("$ATTEST" show --mint "$MINT" --sequence 0 --rpc "$RPC")
echo "[e2e] AuditRecord[0]: $REC0"
STATUS0=$(echo "$REC0" | jq -r '.status')
TOTAL0=$(echo "$REC0" | jq -r '.total_reservas')
CID0=$(echo "$REC0" | jq -r '.ipfs_cid')
[ "$STATUS0" = "OK" ] || { echo "[e2e] FAIL: expected OK, got $STATUS0"; exit 1; }
[ "$TOTAL0" = "$TOTAL" ] || { echo "[e2e] FAIL: anchored total $TOTAL0 != dataset total $TOTAL"; exit 1; }
[ "$(echo "$REC0" | jq -r '.supply_at_ingestion')" = "$TOTAL" ] || { echo "[e2e] FAIL: supply_at_ingestion != minted total"; exit 1; }
[ "$(echo "$REC0" | jq -r '.dataset_sha256')" = "$(echo "$SNAP1" | jq -r '.dataset_sha256')" ] || { echo "[e2e] FAIL: dataset_sha256 mismatch"; exit 1; }
echo "[e2e] sequence 0: status OK (as expected)"

echo "[e2e] scenario 2: under-collateralized snapshot (haircut 100 bps, supply unchanged)"
"$ATTEST" snapshot --n 100 --seed 42 --haircut-bps 100 --out "$WORK/s2" >/dev/null
SUB1=$("$ATTEST" submit --dataset "$WORK/s2/dataset.jsonl" --out "$WORK/s2" \
  --mint "$MINT" --keypair "$KEYPAIR" --rpc "$RPC")
echo "[e2e] submit[1]: $SUB1"
REC1=$("$ATTEST" show --mint "$MINT" --sequence 1 --rpc "$RPC")
echo "[e2e] AuditRecord[1]: $REC1"
STATUS1=$(echo "$REC1" | jq -r '.status')
CID1=$(echo "$REC1" | jq -r '.ipfs_cid')
[ "$STATUS1" = "DIVERGENTE" ] || { echo "[e2e] FAIL: expected DIVERGENTE, got $STATUS1"; exit 1; }
[ "$CID0" != "$CID1" ] || { echo "[e2e] FAIL: different datasets must yield different CIDs"; exit 1; }
for c in "$CID0" "$CID1"; do
  [ "${#c}" -eq 59 ] && [ "${c:0:1}" = "b" ] || { echo "[e2e] FAIL: CID $c is not the fixed profile (b + 58 chars)"; exit 1; }
done
echo "[e2e] sequence 1: status DIVERGENTE (as expected)"

echo "[e2e] PASS: end-to-end attestation on localnet (OK + DIVERGENTE)"
