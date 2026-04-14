# AXIOM Protocol v0.1

## 1. Purpose

AXIOM is a BFT-style layer-1 protocol intended to optimize for:

1. fast deterministic finality
2. bounded and auditable execution semantics
3. native token demand tied to real network use
4. implementation diversity through a reference model and a hardened validator

This document is the source of truth for protocol behavior. Client implementations must follow this spec rather than each other.

## 2. Terminology

- **AXM**: the native asset
- **validator**: a node allowed to propose and vote on blocks
- **epoch**: a fixed validator-schedule interval
- **slot**: a time window in which exactly one proposer is expected
- **quorum certificate (QC)**: aggregated evidence that >= 2/3 of validator power voted for a block
- **finalized block**: a block committed by a valid QC according to the protocol rules
- **payment lane**: deterministic transfer path for simple asset movement
- **contract lane**: deterministic execution path for VM transactions

## 3. Chain parameters

Initial reference values:

- chain id: `axiom-testnet-1`
- target slot time: `500 ms`
- target deterministic finality: `~2 s`
- quorum threshold: `>= 2/3` of active voting power
- max supply: `42,000,000 AXM`
- minimum validator stake: chain-configured
- base fee: dynamically adjusted per block
- state rent: charged on persistent bytes and burned

Any value changes must be introduced by an explicit governance upgrade or a new chain configuration release.

## 4. Accounts and objects

AXIOM uses a hybrid state model:

### 4.1 Account state

Each account has:

- address
- balance
- nonce
- optional staking metadata

### 4.2 Contract object state

Each contract object has:

- address
- owner
- code hash
- code bytes
- persistent key/value storage
- balance
- storage rent metadata

### 4.3 Address derivation

Externally owned addresses are derived from a public key hash.
Contract addresses are derived from:

`contract_address = H(sender_address || sender_nonce || deploy_salt)`

## 5. Transactions

A valid transaction contains:

- chain id
- kind: `transfer`, `deploy`, `call`, `stake`, `unstake`, `vote-delegate`, or future versioned kind
- sender address
- sender public key
- nonce
- gas limit
- max fee per gas
- value
- recipient when applicable
- data payload when applicable
- timestamp or validity window
- signature over canonical transaction bytes

### 5.1 Validation rules

A transaction is invalid if any of the following holds:

- wrong chain id
- invalid signature
- nonce mismatch
- gas limit below intrinsic gas
- sender balance below `value + gas_limit * max_fee_per_gas`
- recipient required but missing
- unknown transaction kind
- contract deploy or call payload not valid for the VM version

## 6. Fees and value accrual

### 6.1 Base fee

Each block has a protocol-calculated base fee.
The base fee is burned.

### 6.2 Priority fee

Users may pay a priority fee above the base fee.
The priority portion goes to the block proposer and later may be split with the validator set if governance chooses.

### 6.3 State rent

Persistent contract storage accrues rent.
Rent is burned.
A contract with unpaid rent may become read-only, then evictable, according to governance-defined grace periods.

### 6.4 Supply policy

- hard cap: `42,000,000 AXM`
- no hidden mint key
- no balance rewrite authority
- any treasury allocation must appear in genesis and be time-locked on-chain

## 7. Consensus

### 7.1 Validator set

The active validator set is versioned by epoch.
Each validator has:

- address
- consensus public key
- voting power
- staked balance

### 7.2 Proposer schedule

The proposer for slot `s` is selected deterministically from the active validator set.
A simple round-robin schedule is acceptable for v0.1 testnet.
Weighted proposer selection may be introduced later but must preserve determinism.

### 7.3 Voting

Validators vote for at most one block per height and round.
A valid vote includes:

- chain id
- block hash
- height
- round
- validator address
- signature

### 7.4 Finality rule

A block is finalized when a quorum certificate proves votes representing at least two-thirds of active voting power for that block at that height and round.
Fork-choice for v0.1 is “highest finalized height, then highest QC height”.

### 7.5 Slashing

Slashable offenses include:

- double proposal for the same height and round
- double vote for conflicting blocks at the same height and round
- invalid state transition proposal
- provable censorship after policy-defined thresholds

v0.1 requires slashing evidence to be included as a transaction or governance action with canonical proof bytes.

## 8. Block structure

Each block consists of:

- header
  - chain id
  - height
  - parent hash
  - proposer
  - slot
  - round
  - timestamp
  - state root
  - tx root
  - receipts root
  - validator set root
  - base fee
  - gas used
- body
  - ordered transaction list
  - optional evidence list

## 9. Execution model

### 9.1 Lanes

Transactions are separated into deterministic lanes:

- lane 0: simple transfers and staking operations
- lane 1..N: contract transactions sharded by sender or object affinity

A block builder may execute lanes in parallel only if the final ordering is identical across all correct nodes.

### 9.2 VM

The v0.1 VM is stack-based and deterministic.
It must not expose wall clock, randomness, host filesystem, network, or floating-point nondeterminism.

Supported capabilities:

- integer arithmetic with checked overflow rules
- storage read/write
- event emission
- bounded contract-to-contract call depth
- bounded memory growth
- deterministic gas charging per opcode

Forbidden for v0.1:

- unrestricted recursion
- dynamic native extensions
- nondeterministic syscalls
- direct validator or consensus-state mutation from contracts

## 10. Canonical serialization

All signed data uses canonical serialization.
Reference clients may use canonical JSON for test vectors, but the production Rust node should migrate to a compact binary encoding with an unambiguous canonical form.

## 11. Genesis

Genesis must be reproducible from public inputs:

- chain id
- validator set
- initial account balances
- treasury addresses and time locks if any
- chain parameters

A deterministic genesis hash must be published before any public testnet or mainnet launch.

## 12. RPC and networking

v0.1 testnet supports at least:

- peer handshake
- block announcement
- vote relay
- transaction gossip
- health/status query
- block query by height/hash

Authenticated peer identities are recommended.
Untrusted data must be size-bounded and decoded defensively.

## 13. Upgrades

Breaking changes require:

- a versioned spec update
- a new test vector set
- a migration plan
- a client compatibility matrix
- a public activation policy

## 14. Security invariants

The protocol should preserve these invariants:

1. no account balance becomes negative
2. total supply changes only through genesis-defined supply or explicit fee burn accounting
3. finalized history does not revert without a protocol-defined catastrophic recovery procedure
4. the same block input always yields the same post-state on correct implementations
5. a valid signature binds the sender to the exact transaction bytes

## 15. Non-goals for v0.1

- anonymous transaction privacy
- full cross-chain interoperability
- complex governance machinery
- optimized mainnet economics
- permissionless validator admission without stake and policy controls
