use axiom_node::{
    crypto::{address_from_vk, generate_key, public_key_hex, sign_bytes},
    execution::apply_transfer,
    state::{Account, ChainState},
    types::Transaction,
    validation::signing_bytes,
};

fn make_signed_transfer() -> (ChainState, Transaction, String) {
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
        max_fee_per_gas: 3,
        value: 100,
        recipient: Some(recipient.clone()),
        data: None,
        timestamp_ms: 1_700_000_000_000,
        signature: String::new(),
    };

    tx.signature = sign_bytes(&sender_key, &signing_bytes(&tx).unwrap());

    (state, tx, recipient)
}

#[test]
fn apply_transfer_updates_balances_nonce_and_fee_counters() {
    let (mut state, tx, recipient) = make_signed_transfer();

    let result = apply_transfer(&tx, &mut state, "axiom-local", 2).unwrap();

    assert_eq!(result.burned_fee, 20);
    assert_eq!(result.tipped_fee, 10);
    assert_eq!(result.transferred_value, 100);

    let sender = state.accounts.get(&tx.sender).unwrap();
    assert_eq!(sender.balance, 9870);
    assert_eq!(sender.nonce, 1);

    let recipient_acct = state.accounts.get(&recipient).unwrap();
    assert_eq!(recipient_acct.balance, 100);

    assert_eq!(state.total_burned, 20);
    assert_eq!(state.total_tipped, 10);
}

#[test]
fn apply_transfer_rejects_when_max_fee_below_base_fee() {
    let (mut state, tx, _) = make_signed_transfer();

    let err = apply_transfer(&tx, &mut state, "axiom-local", 4)
        .unwrap_err()
        .to_string();

    assert!(err.contains("max_fee_per_gas below base fee"));
}

#[test]
fn apply_transfer_rejects_replay_after_nonce_increment() {
    let (mut state, tx, _) = make_signed_transfer();

    apply_transfer(&tx, &mut state, "axiom-local", 2).unwrap();

    let err = apply_transfer(&tx, &mut state, "axiom-local", 2)
        .unwrap_err()
        .to_string();

    assert!(err.contains("bad nonce"));
}

#[test]
fn apply_transfer_creates_missing_recipient_account() {
    let (mut state, tx, recipient) = make_signed_transfer();

    assert!(!state.accounts.contains_key(&recipient));
    apply_transfer(&tx, &mut state, "axiom-local", 2).unwrap();
    assert!(state.accounts.contains_key(&recipient));
}
