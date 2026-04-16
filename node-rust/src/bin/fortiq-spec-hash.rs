use axiom_node::axiom1::Axiom1Genesis;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: fortiq-spec-hash <path-to-fortiq-mainnet.json>");

    let spec = Axiom1Genesis::from_path(&path).expect("failed to load spec");
    let hash = spec.canonical_hash().expect("failed to hash spec");
    println!("{}", hash);
}
