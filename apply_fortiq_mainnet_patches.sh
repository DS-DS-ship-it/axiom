#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="${1:-/Users/derekwardlaw/Downloads/axiom_public_repo}"
NODE_RUST_DIR="$REPO_ROOT/node-rust"

if [[ ! -d "$NODE_RUST_DIR/src" ]]; then
  echo "Could not find node-rust/src under: $NODE_RUST_DIR"
  exit 1
fi

cd "$NODE_RUST_DIR"

python3 - <<'PY'
from pathlib import Path
import re

ROOT = Path(".").resolve()

FILES = {
    "axiom1": ROOT / "src" / "axiom1.rs",
    "state": ROOT / "src" / "state.rs",
    "state_transition": ROOT / "src" / "state_transition.rs",
    "devnet_runtime": ROOT / "src" / "devnet_runtime.rs",
}

for p in FILES.values():
    if not p.exists():
        raise SystemExit(f"Missing file: {p}")

def backup(path: Path) -> None:
    bak = path.with_suffix(path.suffix + ".bak.mainnetpatch")
    if not bak.exists():
        bak.write_text(path.read_text())

def find_matching_brace(text: str, open_idx: int) -> int:
    depth = 0
    i = open_idx
    n = len(text)
    in_string = False
    in_char = False
    in_line_comment = False
    in_block_comment = False
    escape = False

    while i < n:
        ch = text[i]
        nxt = text[i + 1] if i + 1 < n else ""

        if in_line_comment:
            if ch == "\n":
                in_line_comment = False
            i += 1
            continue

        if in_block_comment:
            if ch == "*" and nxt == "/":
                in_block_comment = False
                i += 2
                continue
            i += 1
            continue

        if in_string:
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                in_string = False
            i += 1
            continue

        if in_char:
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == "'":
                in_char = False
            i += 1
            continue

        if ch == "/" and nxt == "/":
            in_line_comment = True
            i += 2
            continue

        if ch == "/" and nxt == "*":
            in_block_comment = True
            i += 2
            continue

        if ch == '"':
            in_string = True
            i += 1
            continue

        if ch == "'":
            in_char = True
            i += 1
            continue

        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1

    raise ValueError("No matching brace found")

def replace_named_block(text: str, marker: str, replacement: str) -> str:
    idx = text.find(marker)
    if idx < 0:
        raise ValueError(f"Could not find marker: {marker}")
    open_idx = text.find("{", idx)
    if open_idx < 0:
        raise ValueError(f"Could not find opening brace after marker: {marker}")
    close_idx = find_matching_brace(text, open_idx)
    return text[:idx] + replacement + text[close_idx + 1:]

def replace_or_insert_struct(text: str, struct_name: str, replacement: str, insert_before: str) -> str:
    marker = f"pub struct {struct_name}"
    if marker in text:
        return replace_named_block(text, marker, replacement)
    idx = text.find(insert_before)
    if idx < 0:
        raise ValueError(f"Could not find insertion point for {struct_name}: {insert_before}")
    return text[:idx] + replacement + "\n\n" + text[idx:]

def insert_before(text: str, marker: str, insertion: str) -> str:
    if insertion.strip() in text:
        return text
    idx = text.find(marker)
    if idx < 0:
        raise ValueError(f"Could not find insertion marker: {marker}")
    return text[:idx] + insertion + text[idx:]

def ensure_use_block(text: str, want: str, after_last_use: bool = True) -> str:
    if want in text:
        return text
    uses = list(re.finditer(r"^use .*?;\n", text, flags=re.M))
    if uses and after_last_use:
        last = uses[-1]
        return text[:last.end()] + want + "\n" + text[last.end():]
    return want + "\n" + text

# ------------------------------------------------------------------
# axiom1.rs
# ------------------------------------------------------------------
path = FILES["axiom1"]
backup(path)
text = path.read_text()

text = text.replace("use anyhow::Result;", "use anyhow::{ensure, Result};")
text = text.replace("use anyhow::{anyhow, Result};", "use anyhow::{anyhow, ensure, Result};")
text = ensure_use_block(text, "use crate::crypto::hash_hex;")

consts = """pub const FORTIQ_MAINNET_CHAIN_ID: &str = "fortiq-mainnet";
pub const FORTIQ_MAINNET_PROTOCOL_VERSION: u32 = 1;
pub const FORTIQ_MAINNET_SPEC_HASH: &str = "REPLACE_WITH_CANONICAL_SPEC_HASH";

fn default_decimals() -> u8 {
    8
}

"""
if "FORTIQ_MAINNET_CHAIN_ID" not in text:
    uses = list(re.finditer(r"^use .*?;\n", text, flags=re.M))
    if uses:
        last = uses[-1]
        text = text[:last.end()] + "\n" + consts + text[last.end():]
    else:
        text = consts + text

genesis_validator_block = """#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenesisValidator {
    pub name: String,
    pub public_key: String,
    pub bind_addr: String,
    pub power: u64,
    pub stake: u64,
    #[serde(default)]
    pub reward_address: String,
}"""
text = replace_or_insert_struct(
    text,
    "GenesisValidator",
    genesis_validator_block,
    "pub struct Axiom1Genesis"
)

axiom1_block = """#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Axiom1Genesis {
    pub chain_id: String,
    pub protocol_version: u32,
    pub genesis_time_ms: u64,
    pub base_fee: u64,
    pub checkpoint_interval: u64,

    #[serde(default = "default_decimals")]
    pub decimals: u8,

    #[serde(default)]
    pub max_supply: u128,

    #[serde(default)]
    pub initial_block_subsidy: u128,

    #[serde(default)]
    pub subsidy_halving_interval: u64,

    pub validators: Vec<GenesisValidator>,
    pub accounts: Vec<GenesisAccount>,
    pub metadata: std::collections::BTreeMap<String, String>,
}"""
text = replace_named_block(text, "pub struct Axiom1Genesis", axiom1_block)

extra_methods = """
    pub fn canonical_hash(&self) -> Result<String> {
        Ok(hash_hex(&serde_json::to_vec(self)?))
    }

    pub fn verify_mainnet_identity(&self) -> Result<()> {
        ensure!(
            self.chain_id == FORTIQ_MAINNET_CHAIN_ID,
            "unexpected mainnet chain_id"
        );
        ensure!(
            self.protocol_version == FORTIQ_MAINNET_PROTOCOL_VERSION,
            "unexpected mainnet protocol_version"
        );
        ensure!(self.decimals == 8, "mainnet decimals must be 8");
        ensure!(self.max_supply > 0, "mainnet max_supply must be > 0");
        ensure!(
            self.initial_block_subsidy > 0,
            "mainnet initial_block_subsidy must be > 0"
        );
        ensure!(
            self.subsidy_halving_interval > 0,
            "mainnet subsidy_halving_interval must be > 0"
        );

        let genesis_alloc: u128 = self
            .accounts
            .iter()
            .map(|a| u128::from(a.balance))
            .sum();

        ensure!(
            genesis_alloc <= self.max_supply,
            "genesis allocations exceed max_supply"
        );

        ensure!(
            FORTIQ_MAINNET_SPEC_HASH != "REPLACE_WITH_CANONICAL_SPEC_HASH",
            "set FORTIQ_MAINNET_SPEC_HASH before launching mainnet"
        );

        let actual = self.canonical_hash()?;
        ensure!(
            actual == FORTIQ_MAINNET_SPEC_HASH,
            "mainnet spec hash mismatch: expected {}, got {}",
            FORTIQ_MAINNET_SPEC_HASH,
            actual
        );

        Ok(())
    }

"""
if "pub fn canonical_hash(&self) -> Result<String>" not in text:
    text = insert_before(text, "    pub fn checkpoint_interval(&self) -> u64 {", extra_methods)

to_chain_state_block = """    pub fn to_chain_state(&self) -> crate::state::ChainState {
        let mut state = crate::state::ChainState::default();
        state.chain_id = self.chain_id.clone();
        state.protocol_version = self.protocol_version;
        state.height = 0;
        state.tip_hash = "GENESIS".to_string();
        state.decimals = self.decimals;
        state.max_supply = self.max_supply;

        let genesis_issued: u128 = self
            .accounts
            .iter()
            .map(|a| u128::from(a.balance))
            .sum();

        state.total_issued = genesis_issued;
        state.initial_block_subsidy = self.initial_block_subsidy;
        state.subsidy_halving_interval = self.subsidy_halving_interval;

        for v in &self.validators {
            state.validators.insert(
                v.name.clone(),
                crate::state::Validator {
                    public_key: v.public_key.clone(),
                    power: v.power,
                    stake: u128::from(v.stake),
                    reward_address: v.reward_address.clone(),
                },
            );
        }

        for a in &self.accounts {
            state.accounts.insert(
                a.address.clone(),
                crate::state::Account {
                    balance: u128::from(a.balance),
                    nonce: a.nonce,
                    staked: u128::from(a.staked),
                },
            );
        }

        state
    }"""
text = replace_named_block(text, "    pub fn to_chain_state(&self)", to_chain_state_block)
path.write_text(text)

# ------------------------------------------------------------------
# state.rs
# ------------------------------------------------------------------
path = FILES["state"]
backup(path)
text = path.read_text()

validator_block = """#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub struct Validator {
    pub public_key: String,
    pub power: u64,
    pub stake: u128,
    #[serde(default)]
    pub reward_address: String,
}"""
text = replace_named_block(text, "pub struct Validator", validator_block)

chain_state_block = """#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ChainState {
    pub chain_id: String,
    pub protocol_version: u32,

    pub height: u64,
    pub tip_hash: String,

    pub accounts: std::collections::BTreeMap<String, Account>,
    pub validators: std::collections::BTreeMap<String, Validator>,

    pub total_burned: u128,
    pub total_tipped: u128,

    #[serde(default)]
    pub decimals: u8,

    #[serde(default)]
    pub max_supply: u128,

    #[serde(default)]
    pub total_issued: u128,

    #[serde(default)]
    pub initial_block_subsidy: u128,

    #[serde(default)]
    pub subsidy_halving_interval: u64,
}"""
text = replace_named_block(text, "pub struct ChainState", chain_state_block)

default_block = """impl Default for ChainState {
    fn default() -> Self {
        Self {
            chain_id: String::new(),
            protocol_version: 0,
            height: 0,
            tip_hash: "GENESIS".to_string(),
            accounts: std::collections::BTreeMap::new(),
            validators: std::collections::BTreeMap::new(),
            total_burned: 0,
            total_tipped: 0,
            decimals: 8,
            max_supply: 0,
            total_issued: 0,
            initial_block_subsidy: 0,
            subsidy_halving_interval: 0,
        }
    }
}"""
text = replace_named_block(text, "impl Default for ChainState", default_block)
path.write_text(text)

# ------------------------------------------------------------------
# state_transition.rs
# ------------------------------------------------------------------
path = FILES["state_transition"]
backup(path)
text = path.read_text()

helpers = """
fn block_subsidy(height: u64, initial: u128, halving_interval: u64) -> u128 {
    if height == 0 || initial == 0 || halving_interval == 0 {
        return 0;
    }

    let halvings = (height - 1) / halving_interval;
    if halvings >= 128 {
        0
    } else {
        initial >> halvings
    }
}

fn proposer_reward_address(
    state: &crate::state::ChainState,
    proposer_name: &str,
) -> Result<String> {
    let v = state
        .validators
        .get(proposer_name)
        .ok_or_else(|| anyhow!("unknown proposer"))?;

    ensure!(
        !v.reward_address.is_empty(),
        "missing reward_address for proposer {}",
        proposer_name
    );

    Ok(v.reward_address.clone())
}

"""
if "fn block_subsidy(height: u64, initial: u128, halving_interval: u64) -> u128" not in text:
    text = insert_before(text, "#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]", helpers)

execute_block = """pub fn execute_block_deterministic(
    block: &Block,
    pre_state: &ChainState,
    expected_chain_id: &str,
) -> Result<BlockExecutionResult> {
    ensure!(block.header.chain_id == expected_chain_id, "wrong block chain id");

    let mut working_state = pre_state.clone();
    let mut receipts = Vec::with_capacity(block.txs.len());
    let mut gas_used = 0u64;

    for tx in &block.txs {
        let receipt = apply_transaction_deterministic(
            tx,
            &mut working_state,
            expected_chain_id,
            block.header.base_fee,
        );
        gas_used = gas_used.saturating_add(receipt.gas_used);
        receipts.push(receipt);
    }

    let tipped_total: u128 = receipts.iter().map(|r| r.tipped_fee).sum();
    let subsidy = block_subsidy(
        block.header.height,
        working_state.initial_block_subsidy,
        working_state.subsidy_halving_interval,
    );

    if subsidy > 0 {
        ensure!(
            working_state.total_issued.saturating_add(subsidy) <= working_state.max_supply,
            "supply cap exceeded"
        );
        working_state.total_issued = working_state.total_issued.saturating_add(subsidy);
    }

    let proposer_payout = tipped_total.saturating_add(subsidy);
    if proposer_payout > 0 {
        let reward_address = proposer_reward_address(&working_state, &block.header.proposer)?;
        let acct = working_state.accounts.entry(reward_address).or_default();
        acct.balance = acct.balance.saturating_add(proposer_payout);
    }

    let tx_root = canonical_receipt_root(&receipts)?;
    let state_root = canonical_state_root(&working_state)?;
    let block_hash = canonical_block_hash(block)?;

    Ok(BlockExecutionResult {
        block_hash,
        tx_root,
        state_root,
        gas_used,
        receipts,
    })
}"""
text = replace_named_block(text, "pub fn execute_block_deterministic(", execute_block)
path.write_text(text)

# ------------------------------------------------------------------
# devnet_runtime.rs
# ------------------------------------------------------------------
path = FILES["devnet_runtime"]
backup(path)
text = path.read_text()

text = text.replace(
    "    axiom1::Axiom1Genesis,",
    "    axiom1::{Axiom1Genesis, FORTIQ_MAINNET_CHAIN_ID},"
)

bootstrap_old = """        if config.chain_id != genesis.chain_id {
            return Err(anyhow!("config/genesis chain id mismatch"));
        }
"""
bootstrap_new = """        if config.chain_id != genesis.chain_id {
            return Err(anyhow!("config/genesis chain id mismatch"));
        }

        if genesis.chain_id == FORTIQ_MAINNET_CHAIN_ID {
            genesis.verify_mainnet_identity()?;
        }
"""
if bootstrap_new not in text:
    if bootstrap_old not in text:
        raise ValueError("Could not find bootstrap chain-id block")
    text = text.replace(bootstrap_old, bootstrap_new, 1)

commit_old = """    pub fn commit_block(&mut self, block: &Block, votes: &[Vote]) -> Result<CommitCertificate> {
        let cert = build_commit_certificate(block, votes, &self.state)?;
"""
commit_new = """    pub fn commit_block(&mut self, block: &Block, votes: &[Vote]) -> Result<CommitCertificate> {
        let execution = execute_block_deterministic(block, &self.state, &self.config.chain_id)?;
        ensure!(
            block.header.state_root == execution.state_root,
            "block state_root mismatch"
        );
        ensure!(
            block.header.tx_root == execution.tx_root,
            "block tx_root mismatch"
        );
        ensure!(
            block.header.gas_used == execution.gas_used,
            "block gas_used mismatch"
        );

        let cert = build_commit_certificate(block, votes, &self.state)?;
"""
if commit_new not in text:
    if commit_old not in text:
        raise ValueError("Could not find commit_block marker")
    text = text.replace(commit_old, commit_new, 1)

path.write_text(text)

print("Patched:")
for name, path in FILES.items():
    print(" -", path)
print("\nBackups created with suffix .bak.mainnetpatch")
PY

echo
echo "Patch complete."
echo "Next:"
echo "  cd \"$NODE_RUST_DIR\""
echo "  cargo fmt"
echo "  cargo build"
