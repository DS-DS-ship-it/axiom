#!/usr/bin/env bash
set -euo pipefail
cd /Users/derekwardlaw/Downloads/axiom_public_repo
rm -rf testnet/devnet/node1-data testnet/devnet/node2-data testnet/devnet/node3-data testnet/devnet/node4-data
rm -f testnet/devnet/node1-state.json testnet/devnet/node2-state.json testnet/devnet/node3-state.json testnet/devnet/node4-state.json
rm -f testnet/devnet/node1.wal.jsonl testnet/devnet/node2.wal.jsonl testnet/devnet/node3.wal.jsonl testnet/devnet/node4.wal.jsonl
python3 scripts/generate_axiom1_genesis.py --out testnet/devnet
echo "devnet reset complete"
