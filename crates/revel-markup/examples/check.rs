//! Analyse des gabarits d'un dossier : `cargo run -p revel-markup --example check -- <dossier>`.
fn main() {
    let dir = std::env::args().nth(1).expect("dossier");
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().and_then(|x| x.to_str()) != Some("njk") {
                continue;
            }
            let src = std::fs::read_to_string(&p).unwrap();
            let body = src
                .strip_prefix("---")
                .and_then(|r| r.split_once("\n---").map(|(_, b)| b))
                .unwrap_or(&src);
            match revel_markup::parse(
                body,
                &revel_markup::Options {
                    dialect: revel_markup::Dialect::Nunjucks,
                    mode: if p.to_string_lossy().contains("feed")
                        || p.to_string_lossy().contains("sitemap")
                    {
                        revel_markup::Mode::Xml
                    } else if p.to_string_lossy().contains("htaccess")
                        || p.to_string_lossy().contains("robots")
                    {
                        revel_markup::Mode::Text
                    } else {
                        revel_markup::Mode::Html
                    },
                },
            ) {
                Ok(r) => {
                    println!("OK   {} ({} nœuds)", p.display(), r.nodes.len());
                    for w in r.warnings {
                        println!("     ! {w}");
                    }
                }
                Err(e) => println!("ERR  {} : {e}", p.display()),
            }
        }
    }
}
