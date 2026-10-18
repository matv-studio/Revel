//! Redirections du site (Paramètres › Navigation › Redirections) écrites dans le site produit.
//!
//! - **Apache** (`.htaccess`) : règles `mod_rewrite`, seules capables de lire les paramètres d'une
//!   ancienne adresse (`/?p=12`) ; `Redirect` de `mod_alias` ne lit pas la chaîne de requête
//!   (<https://simonecarletti.com/blog/2009/01/apache-query-string-redirects/>). Les règles se
//!   placent avant celles déjà écrites dans le fichier (une règle finale de type « contrôleur
//!   frontal » les masquerait sinon).
//! - **Tout hébergeur** (GitHub Pages ignore `.htaccess`) : page de redirection immédiate à
//!   l'ancienne adresse, `meta refresh` à 0 seconde, que Google lit comme une redirection
//!   permanente (<https://developers.google.com/search/docs/crawling-indexing/301-redirects>).
//!   Rien n'est écrit là où le site produit déjà un fichier.

use crate::{OutputFile, html::escape};
use revel_model::Site;

/// Ancienne adresse découpée : chemin (`/ancien/`) et paramètres (`p=12`).
fn split_from(from: &str) -> (&str, Option<&str>) {
    let from = from.split('#').next().unwrap_or(from);
    match from.split_once('?') {
        Some((p, q)) => (p, Some(q).filter(|q| !q.is_empty())),
        None => (from, None),
    }
}

/// `%xx` décodés (Apache compare le chemin décodé).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Texte littéral dans une expression régulière d'Apache (PCRE).
fn pcre_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '.' | '^' | '$' | '|' | '?' | '*' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            ' ' => out.push_str("\\x20"),
            _ => out.push(c),
        }
    }
    out
}

/// Adresse d'arrivée écrite dans une règle : espaces encodés ; `?` final ajouté quand
/// l'ancienne adresse avait des paramètres (ils ne sont pas repris), avant l'ancre éventuelle.
fn target(to: &str, drop_query: bool) -> String {
    let to = to.trim().replace(' ', "%20");
    if !drop_query || to.contains('?') {
        return to;
    }
    match to.split_once('#') {
        Some((a, h)) => format!("{a}?#{h}"),
        None => format!("{to}?"),
    }
}

/// Règles Apache des redirections (vide si aucune).
pub(crate) fn htaccess_rules(site: &Site) -> String {
    let mut rules = String::new();
    for r in &site.redirects {
        let (path, query) = split_from(r.from.trim());
        let Some(rel) = path.strip_prefix('/') else {
            continue;
        };
        if r.to.trim().is_empty() || r.from.trim() == r.to.trim() {
            continue;
        }
        let rel = percent_decode(rel);
        let pattern = match rel.strip_suffix('/') {
            Some(dir) => format!("^{}/?$", pcre_escape(dir)),
            None if rel.is_empty() => "^$".to_string(),
            None => format!("^{}$", pcre_escape(&rel)),
        };
        if let Some(q) = query {
            for pair in q.split('&').filter(|p| !p.is_empty()) {
                rules.push_str(&format!(
                    "RewriteCond %{{QUERY_STRING}} (^|&){}(&|$)\n",
                    pcre_escape(&percent_decode(pair))
                ));
            }
        }
        rules.push_str(&format!(
            "RewriteRule {pattern} {} [R=301,L,NE]\n",
            target(&r.to, query.is_some())
        ));
    }
    if rules.is_empty() {
        return rules;
    }
    format!(
        "# Redirections (Revel : Paramètres › Navigation › Redirections)\n<IfModule mod_rewrite.c>\nRewriteEngine On\n{rules}</IfModule>\n"
    )
}

/// Page de redirection immédiate vers `to`.
fn refresh_page(site: &Site, to: &str) -> String {
    let to = escape(to.trim());
    format!(
        "<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n<title>{to}</title>\n<meta name=\"robots\" content=\"noindex\">\n<link rel=\"canonical\" href=\"{to}\">\n<meta http-equiv=\"refresh\" content=\"0; url={to}\">\n</head>\n<body>\n<p><a href=\"{to}\">{to}</a></p>\n</body>\n</html>\n",
        escape(&site.default_locale)
    )
}

/// Fichier de la page de redirection pour une ancienne adresse sans paramètres : `/ancien/` et
/// `/ancien` -> `ancien/index.html`, `/ancien.html` -> `ancien.html` ; rien pour un autre
/// fichier (`/ancien.php`), servi autrement qu'en HTML.
fn refresh_path(path: &str) -> Option<String> {
    let rel = percent_decode(path.strip_prefix('/')?);
    if rel.split('/').any(|s| s == ".." || s == ".") || rel.contains('\\') {
        return None;
    }
    if rel.is_empty() {
        return None;
    }
    if rel.ends_with('/') {
        return Some(format!("{rel}index.html"));
    }
    let last = rel.rsplit('/').next().unwrap_or("");
    if last.ends_with(".html") || last.ends_with(".htm") {
        Some(rel)
    } else if !last.contains('.') {
        Some(format!("{rel}/index.html"))
    } else {
        None
    }
}

/// Ajoute au site produit les règles Apache et les pages de redirection.
pub(crate) fn redirect_files(site: &Site, files: &mut Vec<OutputFile>, warnings: &mut Vec<String>) {
    if site.redirects.is_empty() {
        return;
    }
    let rules = htaccess_rules(site);
    if !rules.is_empty() {
        match files.iter_mut().find(|f| f.path == ".htaccess") {
            Some(f) => {
                let text = String::from_utf8_lossy(&f.bytes).into_owned();
                // Avant la première règle du fichier (et les commentaires qui la précèdent
                // immédiatement), sinon à la fin.
                let lines: Vec<&str> = text.split_inclusive('\n').collect();
                let rule = lines.iter().position(|l| {
                    let t = l.trim_start().to_ascii_lowercase();
                    t.starts_with("rewritecond") || t.starts_with("rewriterule")
                });
                let at = rule.map(|mut i| {
                    while i > 0 && lines[i - 1].trim_start().starts_with('#') {
                        i -= 1;
                    }
                    lines[..i].iter().map(|l| l.len()).sum::<usize>()
                });
                let mut out = text.clone();
                match at {
                    Some(i) => out.insert_str(i, &format!("{rules}\n")),
                    None => {
                        if !out.is_empty() && !out.ends_with('\n') {
                            out.push('\n');
                        }
                        out.push('\n');
                        out.push_str(&rules);
                    }
                }
                f.bytes = out.into_bytes();
            }
            None => files.push(OutputFile {
                path: ".htaccess".into(),
                bytes: rules.into_bytes(),
            }),
        }
    }
    for r in &site.redirects {
        let (path, query) = split_from(r.from.trim());
        if query.is_some() || r.to.trim().is_empty() || r.from.trim() == r.to.trim() {
            continue;
        }
        let Some(out) = refresh_path(path) else {
            continue;
        };
        if files.iter().any(|f| f.path == out) {
            warnings.push(format!(
                "redirection de {} : une page du site a cette adresse, elle reste affichée (hors Apache)",
                r.from
            ));
            continue;
        }
        files.push(OutputFile {
            path: out,
            bytes: refresh_page(site, &r.to).into_bytes(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use revel_model::Redirect;

    fn site(r: &[(&str, &str)]) -> Site {
        let mut s = Site::new("Essai", "fr");
        s.redirects = r
            .iter()
            .map(|(f, t)| Redirect {
                from: (*f).into(),
                to: (*t).into(),
            })
            .collect();
        s
    }

    #[test]
    fn regles_apache() {
        let s = site(&[
            ("/ancien/", "/nouveau/"),
            ("/a.html", "/b/"),
            ("/?p=12", "/projets/x/"),
            ("/?page_id=4&lang=fr", "/agence/#rejoindre"),
            ("/vieux nom/", "https://ailleurs.example/x"),
        ]);
        let r = htaccess_rules(&s);
        assert!(r.contains("RewriteRule ^ancien/?$ /nouveau/ [R=301,L,NE]\n"));
        assert!(r.contains("RewriteRule ^a\\.html$ /b/ [R=301,L,NE]\n"));
        assert!(r.contains(
            "RewriteCond %{QUERY_STRING} (^|&)p=12(&|$)\nRewriteRule ^$ /projets/x/? [R=301,L,NE]\n"
        ));
        assert!(r.contains(
            "RewriteCond %{QUERY_STRING} (^|&)page_id=4(&|$)\nRewriteCond %{QUERY_STRING} (^|&)lang=fr(&|$)\nRewriteRule ^$ /agence/?#rejoindre [R=301,L,NE]\n"
        ));
        assert!(
            r.contains("RewriteRule ^vieux\\x20nom/?$ https://ailleurs.example/x [R=301,L,NE]")
        );
        assert!(r.starts_with("# Redirections"));
        assert!(htaccess_rules(&site(&[])).is_empty());
    }

    #[test]
    fn fichiers_produits() {
        let s = site(&[
            ("/ancien/", "/nouveau/"),
            ("/court", "/nouveau/"),
            ("/a.html", "/b/"),
            ("/b.php", "/b/"),
            ("/?p=12", "/projets/x/"),
            ("/existe/", "/nouveau/"),
        ]);
        let mut files = vec![
            OutputFile {
                path: ".htaccess".into(),
                bytes: b"Options -Indexes\nRewriteEngine On\n\n# R\xc3\xa8gle\nRewriteRule ^x$ /y [L]\n"
                    .to_vec(),
            },
            OutputFile {
                path: "existe/index.html".into(),
                bytes: b"page".to_vec(),
            },
        ];
        let mut w = Vec::new();
        redirect_files(&s, &mut files, &mut w);
        let get = |p: &str| {
            files
                .iter()
                .find(|f| f.path == p)
                .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
        };
        let h = get(".htaccess").unwrap();
        let rules = h.find("# Redirections").unwrap();
        assert!(rules > h.find("RewriteEngine On").unwrap());
        assert!(rules < h.find("# Règle").unwrap());
        let p = get("ancien/index.html").unwrap();
        assert!(p.contains("<meta http-equiv=\"refresh\" content=\"0; url=/nouveau/\">"));
        assert!(p.contains("<link rel=\"canonical\" href=\"/nouveau/\">"));
        assert!(get("court/index.html").is_some());
        assert!(get("a.html").is_some());
        assert!(get("b.php").is_none());
        assert_eq!(get("existe/index.html").unwrap(), "page");
        assert_eq!(w.len(), 1);
        // Sans `.htaccess` : le fichier ne contient que les redirections.
        let mut files = Vec::new();
        redirect_files(&site(&[("/a/", "/b/")]), &mut files, &mut w);
        assert!(get_in(&files, ".htaccess").starts_with("# Redirections"));
    }

    fn get_in(files: &[OutputFile], p: &str) -> String {
        files
            .iter()
            .find(|f| f.path == p)
            .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
            .unwrap_or_default()
    }
}
