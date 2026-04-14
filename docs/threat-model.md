# Threat model

## Assumed adversaries

- remote unauthenticated peers sending malformed messages
- malicious validators equivocating or withholding votes
- contract authors attempting VM escape or resource exhaustion
- dependency compromise or malicious crate insertion
- operators leaking private keys or misconfiguring public endpoints

## Critical assets

- validator signing keys
- chain state and snapshots
- genesis integrity
- release artifacts
- CI secrets and package registries

## High-priority failure modes

- consensus safety failure
- liveness loss under partial partitions
- signature verification bugs
- nondeterministic execution between nodes
- storage corruption or crash-recovery divergence
- network amplification or denial-of-service
