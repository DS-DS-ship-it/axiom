# AXIOM

AXIOM is a blockchain project scaffold with four layers:

- `spec/` — the protocol source of truth
- `prototype-python/` — a readable reference implementation for experiments
- `node-rust/` — the next validator implementation in Rust
- `docs/` — architecture, threat model, audit plan, release process
- `testnet/` — a reproducible local/public-testnet layout

## Status

This repository is **ready to push to GitHub**, but it is **not production-certified**.

What is included:
- a v0.1 protocol spec
- a packaged Python prototype using `src/` layout
- a Rust validator scaffold with consensus, state, crypto, and network modules
- a testnet directory and Docker Compose skeleton
- CI workflow examples for Python and Rust
- an audit playbook and RFP checklist

What is not claimed:
- no external security audit has been completed
- no guarantee of liveness or safety under adversarial conditions
- no legal review for token issuance, exchange listing, or public sale

## Suggested repository flow

1. Treat `spec/protocol-v0.1.md` as the source of truth.
2. Keep Python only as a reference model and test oracle.
3. Move all validator hardening into `node-rust/`.
4. Run a public testnet before any token launch.
5. Freeze a release candidate commit before commissioning an external audit.

## GitHub publish commands

```bash
cd axiom_public_repo
git init
git add .
git commit -m "Initial AXIOM public scaffold"
# Requires GitHub CLI:
gh repo create axiom --public --source=. --remote=origin --push
```

## Local quick start

### Python reference

```bash
cd prototype-python
python3 -m venv .venv
source .venv/bin/activate
python3 -m pip install -U pip
python3 -m pip install -e .[dev]
pytest
python3 -m axiom_py.cli demo
```

### Rust validator scaffold

```bash
cd node-rust
cargo fmt
cargo check
cargo run -- --config examples/node1.toml
```

## Recommended next milestones

- complete deterministic state transitions in Rust
- replace placeholder networking with authenticated peer sessions
- add RocksDB or equivalent persistent storage
- add snapshotting and crash recovery
- add fuzzing, load tests, and property tests
- run a public incentivized testnet before any mainnet decision
