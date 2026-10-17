//! Passe de vérification sur un site réel (7.8), sans le reste de Revel : lit le site publié à
//! son adresse (copie brute dans `<dossier>/brut`), l'importe (`<dossier>/site`) et génère le
//! site produit par Revel (`<dossier>/produit`).
//!
//! `cargo run --release -p revel-import --example verifier -- <adresse> <dossier> [langue] [pages]`

use std::path::Path;
use std::time::Instant;

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()))?;
        } else {
            std::fs::copy(&p, to.join(e.file_name()))?;
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(url), Some(dest)) = (args.get(1), args.get(2)) else {
        eprintln!("usage : verifier <adresse> <dossier> [langue]");
        std::process::exit(2);
    };
    let lang = args.get(3).cloned().unwrap_or_else(|| "en".into());
    let max_pages: usize = args.get(4).and_then(|v| v.parse().ok()).unwrap_or(2000);
    let dest = Path::new(dest);
    let start = revel_import::crawl::parse_start(url).ok_or("adresse non reconnue")?;
    let raw = dest.join("brut");
    let t = Instant::now();
    let r = revel_import::crawl::crawl(
        &start,
        &raw,
        &revel_import::crawl::Http::new(),
        &revel_import::crawl::Options {
            max_pages,
            ..Default::default()
        },
        &mut |p| {
            eprint!(
                "\r{} pages · {} fichiers · {} Ko   ",
                p.pages,
                p.files,
                p.bytes / 1024
            )
        },
    )?;
    eprintln!();
    println!(
        "lecture : {} pages, {} fichiers, {} Ko, outil : {}, {:.1} s",
        r.pages,
        r.files,
        r.bytes / 1024,
        r.tool.as_deref().unwrap_or("?"),
        t.elapsed().as_secs_f32()
    );
    for w in &r.warnings {
        println!("! {w}");
    }
    let site_dir = dest.join("site");
    copy_dir(&raw, &site_dir)?;
    let t = Instant::now();
    let (site, report) = revel_import::html::import(
        &site_dir,
        &revel_import::html::Options {
            name: revel_import::crawl::site_name(&start),
            lang,
            merge_styles: true,
            ..Default::default()
        },
    )?;
    println!(
        "import : {} pages, {} gabarits, {} règles de style, {} animations, {} formulaires, outil : {}, {:.1} s",
        report.pages,
        report.layouts,
        report.style_rules,
        report.animations,
        report.forms,
        report.tool.as_deref().unwrap_or("?"),
        t.elapsed().as_secs_f32()
    );
    for w in &report.warnings {
        println!("! {w}");
    }
    revel_project::Project::create(&site_dir, site)?;
    let t = Instant::now();
    let b = revel_project::Project::open(&site_dir)?.build(
        &dest.join("produit"),
        &revel_project::BuildOptions {
            environment: "production".into(),
            images: false,
            progress: None,
        },
    )?;
    println!(
        "génération : {} pages, {} fichiers copiés, {:.1} s",
        b.generated,
        b.copied,
        t.elapsed().as_secs_f32()
    );
    for w in b.warnings.iter().take(20) {
        println!("! {w}");
    }
    Ok(())
}
