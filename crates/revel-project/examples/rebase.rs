//! Essai : `cargo run -p revel-project --example rebase -- <site généré> <destination> </prefixe>`
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let n = revel_project::rebase_dir(a[0].as_ref(), a[1].as_ref(), &a[2]).expect("réécriture");
    println!("{n} fichiers réécrits");
}
