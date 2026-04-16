use std::env;

use axiom_node::{
    crypto::{address_from_vk, public_key_hex, sign_bytes, signing_key_from_hex},
    types::Transaction,
    validation::signing_bytes,
};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "usage: cargo run --bin axiom-dev-submit -- <nonce> [value] [recipient]\n\
             example: cargo run --bin axiom-dev-submit -- 2\n\
             example: cargo run --bin axiom-dev-submit -- 3 2500\n\
             example: cargo run --bin axiom-dev-submit -- 4 1000 axm_f7426c6c4cd4c0e171778352ea7315e4ea089ca6"
        );
        std::process::exit(1);
    }

    let nonce: u64 = args[1].parse().expect("nonce must be a u64");
    let value: u64 = if args.len() >= 3 {
        args[2].parse().expect("value must be a u64")
    } else {
        1000
    };

    let recipient = if args.len() >= 4 {
        args[3].clone()
    } else {
        "axm_f7426c6c4cd4c0e171778352ea7315e4ea089ca6".to_string()
    };

    let key_hex = "5555555555555555555555555555555555555555555555555555555555555555";
    let sk = signing_key_from_hex(key_hex).unwrap();
    let sender = address_from_vk(&sk.verifying_key());

    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let mut tx = Transaction {
        chain_id: "axiom-1".to_string(),
        kind: "transfer".to_string(),
        sender,
        sender_pubkey: public_key_hex(&sk),
        nonce,
        gas_limit: 10,
        max_fee_per_gas: 3,
        value,
        recipient: Some(recipient),
        data: None,
        timestamp_ms,
        signature: String::new(),
    };

    tx.signature = sign_bytes(&sk, &signing_bytes(&tx).unwrap());

    let body = serde_json::json!({ "tx": tx });
    println!("{}", serde_json::to_string_pretty(&body).unwrap());
}
