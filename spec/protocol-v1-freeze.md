# AXIOM-1 Protocol Freeze

This document freezes the first public AXIOM development network protocol target.

## Network identity
- Chain ID: `axiom-1`
- Protocol version: `1`
- Network type: validator-based BFT Layer-1 development network
- Native asset ticker: `AXM`

## Validator set
- Initial validator count: 4
- Initial voting power: 100 each
- Initial stake: 1,000,000 AXM each

## Timing targets
- Slot target: 1 second
- Deterministic finality target: 2-4 seconds
- Base fee at genesis: 2
- Checkpoint interval: 10 committed blocks

## Launch transaction support
- `transfer`
- Validator staking data is static at genesis for the first devnet release
- Governance is disabled for `axiom-1`

## Slashing
- Duplicate-vote / equivocation evidence only
- Slashing reduces stake and proportional voting power
- Duplicate evidence must not be applied twice

## Persistence
- WAL-backed commit persistence
- Persistent chain state cursor
- Named checkpoints
- Checkpoint bundle manifest + state snapshot

## Networking
- Authenticated peer hello/ack handshake
- Session-bound signed envelopes
- Chain ID and version enforcement
- Bounded frame sizes
- Timestamp freshness enforcement

## Devnet release gate
Before public testnet:
- deterministic execution covered by tests
- authenticated peer sessions covered by tests
- storage/checkpoint path covered by tests
- WAL replay covered by tests
- abuse-limit tests passing
- soak testing passing
