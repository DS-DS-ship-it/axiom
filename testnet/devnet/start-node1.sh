#!/usr/bin/env bash
set -euo pipefail
cd /Users/derekwardlaw/Downloads/axiom_public_repo
cargo run --manifest-path node-rust/Cargo.toml --bin axiom-devnet -- \
  --config testnet/devnet/node1.toml \
  --data-dir testnet/devnet/node1-data
