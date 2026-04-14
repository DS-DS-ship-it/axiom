use axiom_node::state::{Account, ChainState, Contract, Validator};

#[test]
fn chain_state_defaults_are_empty() {
    let state = ChainState::default();

    assert!(state.accounts.is_empty());
    assert!(state.contracts.is_empty());
    assert!(state.validators.is_empty());
    assert_eq!(state.total_burned, 0);
    assert_eq!(state.total_tipped, 0);
    assert_eq!(state.height, 0);
    assert_eq!(state.tip_hash, "");
}

#[test]
fn account_and_contract_can_be_inserted_and_updated() {
    let mut state = ChainState::default();

    state.accounts.insert(
        "alice".to_string(),
        Account {
            balance: 1_000,
            nonce: 0,
            staked: 0,
        },
    );

    state.contracts.insert(
        "contract1".to_string(),
        Contract {
            owner: "alice".to_string(),
            code_hash: "codehash1".to_string(),
            balance: 50,
            storage: Default::default(),
        },
    );

    let alice = state.accounts.get_mut("alice").unwrap();
    alice.balance -= 125;
    alice.nonce += 1;

    let contract = state.contracts.get_mut("contract1").unwrap();
    contract.storage.insert("counter".to_string(), 7);

    assert_eq!(state.accounts["alice"].balance, 875);
    assert_eq!(state.accounts["alice"].nonce, 1);
    assert_eq!(state.contracts["contract1"].storage["counter"], 7);
}

#[test]
fn validators_are_stored_by_name() {
    let mut state = ChainState::default();

    state.validators.insert(
        "val_a".to_string(),
        Validator {
            public_key: "pk_a".to_string(),
            power: 10,
            stake: 1_000,
        },
    );

    state.validators.insert(
        "val_b".to_string(),
        Validator {
            public_key: "pk_b".to_string(),
            power: 20,
            stake: 2_000,
        },
    );

    assert_eq!(state.validators["val_a"].power, 10);
    assert_eq!(state.validators["val_b"].stake, 2_000);
    assert_eq!(state.validators.len(), 2);
}
