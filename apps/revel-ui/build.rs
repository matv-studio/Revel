// Pile du module WebAssembly portée à 8 Mio (1 Mio par défaut) : une construction de
// développement, aux vues non optimisées, la dépassait en affichant certains panneaux (la page
// s'arrêtait net). Écrit ici plutôt que dans `.cargo/config.toml`, que la variable RUSTFLAGS de
// l'intégration continue remplacerait.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg=-zstack-size=8388608");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
