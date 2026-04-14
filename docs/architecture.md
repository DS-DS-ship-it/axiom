# Architecture

## Repository layers

### `spec/`
The normative protocol documents, test vectors, and wire-format definitions.

### `prototype-python/`
A readable reference model for proving transaction rules, block validity, fee accounting, and VM semantics.

### `node-rust/`
The validator implementation intended for hardening. This is where persistence, network resilience, observability, and performance work belong.

### `testnet/`
A reproducible environment for local devnet, CI smoke tests, and staged public testnets.

## Design rule

The Rust node must follow the spec, and the Python prototype must act as an oracle for deterministic state-transition tests.

## Security posture

- no secret admin balance rewrite path
- no hidden mint path
- no nondeterministic VM host functions
- explicit chain config and genesis hash
- signed transactions, blocks, and votes
- reproducible build and release pipeline before any public launch
