use axiom_node::{
    crypto::{address_from_vk, generate_key, public_key_hex, sign_bytes},
    state::{Account, ChainState},
    state_transition::{canonical_state_root, execute_block_deterministic},
    types::{Block, BlockHeader, Transaction},
    validation::signing_bytes,
};

fn signed_transfer(
    sender_key: &ed25519_dalek::SigningKey,
    sender: &str,
    recipient: &str,
    nonce: u64,
    value: u64,
) -> Transaction {
    let mut tx = Transaction {
        chain_id: "axiom-local".to_string(),
        kind: "transfer".to_string(),
        sender: sender.to_string(),
        sender_pubkey: public_key_hex(sender_key),
        nonce,
        gas_limit: 10,
        max_fee_per_gas: 3,
        value,
        recipient: Some(recipient.to_string()),
        data: None,
        timestamp_ms: 1_700_000_000_000 + nonce,
        signature: String::new(),
    };
    tx.signature = sign_bytes(sender_key, &signing_bytes(&tx).unwrap());
    tx
}

#[test]
fn deterministic_execution_produces_identical_results() {
    let alice_key = generate_key();
    let bob_key = generate_key();

    let alice = address_from_vk(&alice_key.verifying_key());
    let bob = address_from_vk(&bob_key.verifying_key());

    let mut state = ChainState {
        tip_hash: "GENESIS".to_string(),
        ..Default::default()
    };
    state.accounts.insert(
        alice.clone(),
        Account {
            balance: 10_000,
            nonce: 0,
            staked: 0,
        },
    );

    let block = Block {
        header: BlockHeader {
            chain_id: "axiom-local".to_string(),
            height: 1,
            parent_hash: "GENESIS".to_string(),
            proposer: "node1".to_string(),
            slot: 1,
            round: 0,
            timestamp_ms: 1_700_000_000_100,
            state_root: "".to_string(),
            tx_root: "".to_string(),
            base_fee: 2,
            gas_used: 0,
        },
        txs: vec![
            signed_transfer(&alice_key, &alice, &bob, 0, 100),
            signed_transfer(&alice_key, &alice, &bob, 1, 50),
        ],
        signature: String::new(),
    };

    let a = execute_block_deterministic(&block, &state, "axiom-local").unwrap();
    let b = execute_block_deterministic(&block, &state, "axiom-local").unwrap();

    assert_eq!(a, b);
}

#[test]
fn canonical_state_root_changes_when_balances_change() {
    let mut state = ChainState::default();
    state.accounts.insert(
        "alice".to_string(),
        Account {
            balance: 5,
            nonce: 0,
            staked: 0,
        },
    );
    let root_a = canonical_state_root(&state).unwrap();

    state.accounts.get_mut("alice").unwrap().balance = 6;
    let root_b = canonical_state_root(&state).unwrap();

    assert_ne!(root_a, root_b);
}

#[test]
fn invalid_transaction_yields_failed_receipt_but_deterministic_output() {
    let alice_key = generate_key();
    let bob_key = generate_key();
    let alice = address_from_vk(&alice_key.verifying_key());
    let bob = address_from_vk(&bob_key.verifying_key());

    let mut state = ChainState {
        tip_hash: "GENESIS".to_string(),
        ..Default::default()
    };
    state.accounts.insert(
        alice.clone(),
        Account {
            balance: 50,
            nonce: 0,
            staked: 0,
        },
    );

    let block = Block {
        header: BlockHeader {
            chain_id: "axiom-local".to_string(),
            height: 1,
            parent_hash: "GENESIS".to_string(),
            proposer: "node1".to_string(),
            slot: 1,
            round: 0,
            timestamp_ms: 1_700_000_000_100,
            state_root: "".to_string(),
            tx_root: "".to_string(),
            base_fee: 2,
            gas_used: 0,
        },
        txs: vec![signed_transfer(&alice_key, &alice, &bob, 0, 10_000)],
        signature: String::new(),
    };

    let out = execute_block_deterministic(&block, &state, "axiom-local").unwrap();
    assert_eq!(out.receipts.len(), 1);
    assert!(!out.receipts[0].success);
    assert!(out.receipts[0]
        .error
        .as_deref()
        .unwrap_or("")
        .contains("insufficient balance"));
}
