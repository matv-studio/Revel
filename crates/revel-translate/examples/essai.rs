//! Essais de traduction (développement) :
//! - `essai phrases <dossier d'un modèle> <phrase>…` : phrases traduites une à une, avec la durée ;
//! - `essai fichier <dossier des modèles> <de> <vers> <fichier> [md|html|texte]` : un texte entier.

use revel_translate::{Engine, Format, Item, model::Model};
use std::path::Path;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("phrases") if args.len() > 2 => {
            let t0 = Instant::now();
            let mut m = Model::open(Path::new(&args[1])).expect("modèle");
            eprintln!("chargé en {:?}", t0.elapsed());
            for p in &args[2..] {
                let t = Instant::now();
                let r = m.sentence(p, 1, None).expect("traduction");
                eprintln!("{:>5} ms | {p}\n        → {r}", t.elapsed().as_millis());
            }
        }
        Some("fichier") if args.len() > 4 => {
            let e = Engine::new([std::path::PathBuf::from(&args[1])]);
            let text = std::fs::read_to_string(&args[4]).expect("fichier");
            let format = match args.get(5).map(String::as_str) {
                Some("md") => Format::Markdown,
                Some("html") => Format::Html,
                _ => Format::Plain,
            };
            let t = Instant::now();
            let item = Item {
                text: text.clone(),
                format,
            };
            let r = e
                .translate(&args[2], &args[3], &[item], &[])
                .expect("traduction");
            println!("{}", r[0]);
            eprintln!("{} caractères en {:?}", text.chars().count(), t.elapsed());
        }
        _ => eprintln!(
            "usage : essai phrases <modèle> <phrase>… | essai fichier <modèles> <de> <vers> <fichier> [md|html|texte]"
        ),
    }
}
