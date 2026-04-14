# Rust validator

This crate is the **next** validator implementation. It is intended to replace the Python prototype for any serious testnet.

## Current scope

- typed config loading
- key handling and address derivation
- transaction, vote, and block types
- placeholder consensus loop
- placeholder TCP network listener
- structured logging

## Immediate next steps

1. complete persistent state backend
2. finalize canonical serialization for all signed objects
3. enforce deterministic block building from the mempool
4. add authenticated peer sessions and bounded message decoding
5. add storage snapshots and crash recovery
6. add fuzzing and property tests

## Local commands

```bash
cargo fmt
cargo check
cargo run -- --config examples/node1.toml
```
