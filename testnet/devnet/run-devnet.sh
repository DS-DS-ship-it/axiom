#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT/node-rust"

python3 ../scripts/generate_axiom1_genesis.py --out ../testnet/devnet

cargo run --bin axiom-devnet -- --config ../testnet/devnet/node1.toml --data-dir ../testnet/devnet/node1-data &
cargo run --bin axiom-devnet -- --config ../testnet/devnet/node2.toml --data-dir ../testnet/devnet/node2-data &
cargo run --bin axiom-devnet -- --config ../testnet/devnet/node3.toml --data-dir ../testnet/devnet/node3-data &
cargo run --bin axiom-devnet -- --config ../testnet/devnet/node4.toml --data-dir ../testnet/devnet/node4-data &

wait
