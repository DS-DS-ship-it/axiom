use axiom_node::{
    crypto::{address_from_vk, generate_key, public_key_hex, sign_bytes},
    state::{Account, ChainState},
    types::Transaction,
    validation::{signing_bytes, validate_transaction},
};

fn make_signed_transfer() -> (ChainState, Transaction) {
    let sender_key = generate_key();
    let recipient_key = generate_key();

    let sender = address_from_vk(&sender_key.verifying_key());
    let recipient = address_from_vk(&recipient_key.verifying_key());

    let mut state = ChainState::default();
    state.accounts.insert(
        sender.clone(),
        Account {
            balance: 10_000,
            nonce: 0,
            staked: 0,
        },
    );

    let mut tx = Transaction {
        chain_id: "axiom-local".to_string(),
        kind: "transfer".to_string(),
        sender,
        sender_pubkey: public_key_hex(&sender_key),
        nonce: 0,
        gas_limit: 10,
        max_fee_per_gas: 2,
        value: 100,
        recipient: Some(recipient),
        data: None,
        timestamp_ms: 1_700_000_000_000,
        signature: String::new(),
    };

    let sig = sign_bytes(&sender_key, &signing_bytes(&tx).unwrap());
    tx.signature = sig;

    (state, tx)
}

#[test]
fn valid_signed_transfer_is_accepted() {
    let (state, tx) = make_signed_transfer();
    validate_transaction(&tx, &state, "axiom-local").unwrap();
}

#[test]
fn wrong_nonce_is_rejected() {
    let (state, mut tx) = make_signed_transfer();
    tx.nonce = 1;

    let sender_key = generate_key();
    let bad_sig = sign_bytes(&sender_key, &signing_bytes(&tx).unwrap());
    tx.signature = bad_sig;

    assert!(validate_transaction(&tx, &state, "axiom-local").is_err());
}

#[test]
fn insufficient_balance_is_rejected() {
    let (mut state, tx) = make_signed_transfer();
    let sender = tx.sender.clone();
    state.accounts.get_mut(&sender).unwrap().balance = 1;

    let err = validate_transaction(&tx, &state, "axiom-local")
        .unwrap_err()
        .to_string();

    assert!(err.contains("insufficient balance"));
}

#[test]
fn sender_pubkey_mismatch_is_rejected() {
    let (state, mut tx) = make_signed_transfer();
    let other_key = generate_key();
    tx.sender_pubkey = public_key_hex(&other_key);

    let err = validate_transaction(&tx, &state, "axiom-local")
        .unwrap_err()
        .to_string();

    assert!(err.contains("sender/pubkey mismatch"));
}

#[test]
fn wrong_chain_id_is_rejected() {
    let (state, tx) = make_signed_transfer();

    let err = validate_transaction(&tx, &state, "wrong-chain")
        .unwrap_err()
        .to_string();

    assert!(err.contains("wrong chain id"));
}
