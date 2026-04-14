# Independent audit plan

## What “independent audit” should mean

An audit is not a badge. It is a scoped security review by a party that did not build the system and is paid to find flaws, not to market the project.

## When to request the audit

Do **not** request a full audit before the following exist:

- frozen protocol spec revision
- stable release candidate commit hash
- threat model
- architecture diagram
- test suite and reproducible build instructions
- public or at least partner-accessible testnet
- dependency inventory and known-issue triage

## Audit package to prepare

Send the auditor:

1. repository URL and exact commit hash
2. protocol spec revision
3. threat model
4. architecture overview
5. validator startup and test instructions
6. list of trust assumptions
7. list of known limitations
8. out-of-scope components
9. severity rubric for findings
10. preferred report publication terms

## Scope to request

At minimum, request review of:

- consensus safety and finality logic
- signature handling and canonical serialization
- P2P message parsing and peer authentication
- mempool validation
- state transition determinism
- VM sandboxing and gas accounting
- key-management assumptions
- crash recovery and storage integrity
- slashing evidence validation
- upgrade and genesis handling

## Minimum pre-audit checklist

- CI green on every supported platform
- fuzzers running on parsers and VM
- dependency scan complete
- static analysis complete
- load test summary available
- known issues documented
- rate limits and bounds added to networking code

## How to choose an auditor

Request proposals from at least three firms or independent specialists.
Compare them on:

- relevant blockchain and Rust experience
- methodology, not just branding
- ability to review both consensus and VM code
- clarity about what is in scope
- retest terms after fixes
- whether findings can be published
- timeline and staffing

## Sample RFP questions

- Have you audited BFT consensus or validator software before?
- Do you review Rust unsafe code, serialization, and cryptography usage directly?
- What fuzzing or symbolic tools do you typically use?
- Will you provide a retest window after remediation?
- Can you review the exact tagged release and publish a report tied to that hash?
- How do you separate informational issues from exploitable defects?

## After the audit

1. convert every finding into a tracked issue
2. fix and test
3. request a retest on the exact patched tag
4. publish the report or a transparent summary
5. do not claim “fully secure” or “can never fail”
