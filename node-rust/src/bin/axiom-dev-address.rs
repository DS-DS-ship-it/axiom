use axiom_node::crypto::{address_from_vk, signing_key_from_hex};

fn main() {
    let keys = [
        ("dev1", "5555555555555555555555555555555555555555555555555555555555555555"),
        ("dev2", "6666666666666666666666666666666666666666666666666666666666666666"),
    ];

    for (name, hex) in keys {
        let sk = signing_key_from_hex(hex).unwrap();
        let addr = address_from_vk(&sk.verifying_key());
        println!("{name} {hex} {addr}");
    }
}
