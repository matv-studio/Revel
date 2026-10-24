//! Relevé de ce que les modules d'un site Framer copié disent : `framer_modules <dossier> [fichier.framercms]`.
fn main() {
    let mut args = std::env::args().skip(1);
    let root = args.next().expect("dossier");
    let m = revel_import::framer::Modules::load(std::path::Path::new(&root));
    println!("apparitions : {}", m.appear.len());
    for (c, a) in m.appear.iter().take(5) {
        println!(
            "  {c} {:?} {:?} once={}",
            a.enter.get("opacity"),
            a.transition,
            a.once
        );
    }
    println!("survol :\n{}", m.hover_css);
    println!("barre de progression : {}", m.scroll_progress);
    println!("défilement doux : {}", m.smooth_scroll);
    println!("au clic : {:?}", m.click_components);
    for c in &m.cms {
        println!("cms {} {:?}", c.module_url, c.chunks);
    }
    for f in args {
        let d = std::fs::read(&f).expect("fichier");
        match revel_import::framer::decode_chunk(&d) {
            Ok(items) => println!("{f} : {} contenus", items.len()),
            Err(e) => println!("{f} : {e}"),
        }
    }
}
