# Testnet layout

This directory is for deterministic local-devnet and public-testnet configs.

## Stages

- `local-devnet`: 4 validators on localhost
- `closed-testnet`: partner nodes, limited access
- `public-testnet`: open participation, public docs, metrics, and bug reporting

## Rules

- never mix testnet and mainnet keys
- pin a genesis hash per network
- tag every network config change
- store validator inventories and seed peers explicitly
