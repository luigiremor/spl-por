#!/usr/bin/env bash
# Boots a fresh local validator and deploys the por program.
# Usage: ./scripts/localnet.sh   (Ctrl+C stops the validator)
set -euo pipefail
cd "$(dirname "$0")/.."

LEDGER_DIR=".localnet-ledger"
RPC_URL="http://127.0.0.1:8899"
WALLET="${ANCHOR_WALLET:-$HOME/.config/solana/id.json}" # same as [provider].wallet

anchor build

solana-test-validator --reset --quiet --ledger "$LEDGER_DIR" &
VALIDATOR_PID=$!
trap 'kill "$VALIDATOR_PID" 2>/dev/null || true' EXIT

echo "waiting for validator..."
for _ in $(seq 1 30); do
  if solana cluster-version --url "$RPC_URL" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
solana cluster-version --url "$RPC_URL"

solana airdrop 10 --url "$RPC_URL" --keypair "$WALLET" >/dev/null
anchor deploy --provider.cluster localnet

echo "program deployed:"
anchor keys list
echo "validator running (pid $VALIDATOR_PID); Ctrl+C to stop."
wait "$VALIDATOR_PID"
