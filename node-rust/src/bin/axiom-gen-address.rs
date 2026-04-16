use axiom_node::crypto::{address_from_vk, generate_key, public_key_hex};

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn main() {
    let sk = generate_key();
    let vk = sk.verifying_key();

    let private_key_hex = hex_encode(&sk.to_bytes());
    let public_key = public_key_hex(&sk);
    let address = address_from_vk(&vk);

    println!("private_key_hex={}", private_key_hex);
    println!("public_key={}", public_key);
    println!("address={}", address);
}
