//! Langues livrées dans les installateurs (`.github/workflows/release.yml`) : téléchargées et
//! préparées comme depuis les réglages de l'application.
//!
//! `cargo run --release -p revel-translate --features download --example preparer -- <dossier> <langue>…`

use revel_translate::{catalog, install};
use std::sync::atomic::AtomicBool;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((out, codes)) = args.split_first().filter(|(_, c)| !c.is_empty()) else {
        eprintln!("usage : preparer <dossier> <langue>…");
        std::process::exit(2);
    };
    let stop = AtomicBool::new(false);
    for code in codes {
        let Some(pack) = catalog::pack(code) else {
            eprintln!("langue absente du catalogue : {code}");
            std::process::exit(2);
        };
        eprintln!(
            "{code} : {} Mo à télécharger",
            pack.download_size() / 1_000_000
        );
        let mut last = 0;
        let r = install::install(pack, out.as_ref(), &stop, &mut |p| {
            if let install::Progress::Download { done, total } = p {
                let pct = done * 100 / total.max(1);
                if pct >= last + 10 {
                    last = pct;
                    eprintln!("  {pct} %");
                }
            }
        });
        if let Err(e) = r {
            eprintln!("{code} : {e}");
            std::process::exit(1);
        }
    }
}
