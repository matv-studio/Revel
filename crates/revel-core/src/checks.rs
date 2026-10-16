//! Contrôles avant publication (5.4), sur le site généré : liens internes cassés, images trop
//! lourdes, images sans texte alternatif, pages sans titre.

use revel_protocol::{CheckKind, SiteCheck};
use std::collections::{BTreeSet, HashSet};
use std::path::Path;

/// Image trop lourde au-delà de 1 Mo (fichier publié tel quel).
pub(crate) const HEAVY: u64 = 1024 * 1024;

/// Contrôles au plus par type (au-delà, la liste serait illisible).
const MAX_PER_KIND: usize = 200;

/// Fichiers du dossier (chemins relatifs, `/`).
fn files(dir: &Path, base: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, base, out);
        } else if let Ok(rel) = p.strip_prefix(base) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// Balises ouvrantes `<nom …>` : (nom en minuscules, attributs bruts).
fn tags(html: &str) -> impl Iterator<Item = (String, &str)> {
    let mut rest = html;
    std::iter::from_fn(move || {
        loop {
            let i = rest.find('<')?;
            let after = &rest[i + 1..];
            // Commentaires et scripts : sautés en entier.
            if let Some(c) = after.strip_prefix("!--") {
                let end = c.find("-->").map(|e| e + 3).unwrap_or(c.len());
                rest = &c[end..];
                continue;
            }
            let name_len = after
                .find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(after.len());
            if name_len == 0 {
                rest = after;
                continue;
            }
            let name = after[..name_len].to_ascii_lowercase();
            // Fin de balise hors guillemets.
            let body = &after[name_len..];
            let mut quote: Option<char> = None;
            let mut end = body.len();
            for (j, c) in body.char_indices() {
                match (quote, c) {
                    (Some(q), c) if c == q => quote = None,
                    (Some(_), _) => {}
                    (None, '"' | '\'') => quote = Some(c),
                    (None, '>') => {
                        end = j;
                        break;
                    }
                    _ => {}
                }
            }
            let attrs = &body[..end];
            rest = &body[(end + 1).min(body.len())..];
            if name == "script" || name == "style" {
                let close = format!("</{name}");
                if let Some(k) = rest.to_ascii_lowercase().find(&close) {
                    rest = &rest[k..];
                }
            }
            return Some((name, attrs));
        }
    })
}

/// Valeur d'un attribut (`None` : absent).
fn attr(attrs: &str, name: &str) -> Option<String> {
    let lower = attrs.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let at = from + i;
        from = at + name.len();
        let before_ok = at == 0
            || lower[..at]
                .chars()
                .last()
                .is_some_and(|c| c.is_whitespace() || c == '/');
        let rest = &attrs[at + name.len()..];
        let after = rest.trim_start();
        let boundary = rest
            .chars()
            .next()
            .is_none_or(|c| c.is_whitespace() || c == '=' || c == '/');
        if !before_ok || !boundary {
            continue;
        }
        let Some(v) = after.strip_prefix('=') else {
            return Some(String::new());
        };
        let v = v.trim_start();
        return Some(match v.chars().next() {
            Some(q @ ('"' | '\'')) => v[1..].split(q).next().unwrap_or_default().to_string(),
            _ => v
                .split(|c: char| c.is_whitespace())
                .next()
                .unwrap_or_default()
                .to_string(),
        });
    }
    None
}

/// Chemin publié visé par une adresse locale (`/a/b/`, `../x.html`, `img.jpg`), sans requête
/// ni ancre ; `None` pour une adresse externe ou spéciale.
fn local_target(page_dir: &str, href: &str) -> Option<String> {
    let h = href.trim();
    if h.is_empty()
        || h.starts_with('#')
        || h.starts_with("//")
        || h.contains("{{")
        || h.split_once(':').is_some_and(|(s, _)| {
            !s.contains('/')
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        })
    {
        return None;
    }
    let path = h.split(['?', '#']).next().unwrap_or_default();
    if path.is_empty() {
        return None;
    }
    let path = percent_decode(path);
    let joined = if path.starts_with('/') {
        path
    } else {
        format!("{page_dir}{path}")
    };
    // Segments `.` et `..`.
    let mut parts: Vec<&str> = Vec::new();
    for s in joined.split('/') {
        match s {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    let mut out = parts.join("/");
    if joined.ends_with('/') && !out.is_empty() {
        out.push('/');
    }
    Some(out)
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Le chemin publié existe-t-il (fichier, ou dossier avec `index.html`) ?
fn exists(all: &HashSet<String>, p: &str) -> bool {
    if p.is_empty() {
        return all.contains("index.html");
    }
    let p = p.trim_end_matches('/');
    all.contains(p)
        || all.contains(&format!("{p}/index.html"))
        || all.contains(&format!("{p}.html"))
}

/// Contrôles du site généré dans `out`.
pub(crate) fn check_site(out: &Path) -> Vec<SiteCheck> {
    let mut list = Vec::new();
    files(out, out, &mut list);
    let all: HashSet<String> = list.iter().cloned().collect();
    let mut seen = BTreeSet::new();
    let mut checks = Vec::new();
    let mut count = std::collections::HashMap::<CheckKind, usize>::new();
    let mut push = |c: SiteCheck, checks: &mut Vec<SiteCheck>| {
        let n = count.entry(c.kind).or_default();
        if *n < MAX_PER_KIND && seen.insert((c.kind, c.page.clone(), c.target.clone())) {
            *n += 1;
            checks.push(c);
        }
    };
    let mut pages: Vec<&String> = list.iter().filter(|f| f.ends_with(".html")).collect();
    pages.sort();
    for rel in pages {
        let Ok(html) = std::fs::read_to_string(out.join(rel)) else {
            continue;
        };
        let Some(url) = revel_project::page_url(rel) else {
            continue;
        };
        let page_dir = match rel.rsplit_once('/') {
            Some((d, _)) => format!("/{d}/"),
            None => "/".to_string(),
        };
        let mut title: Option<bool> = None;
        let mut redirect = false;
        let lower = html.to_ascii_lowercase();
        if let Some(i) = lower.find("<title") {
            let after = &html[i..];
            let content = after
                .find('>')
                .map(|s| &after[s + 1..])
                .and_then(|c| c.find("</").map(|e| c[..e].trim().to_string()))
                .unwrap_or_default();
            title = Some(!content.is_empty());
        }
        let mut found = Vec::new();
        for (name, attrs) in tags(&html) {
            match name.as_str() {
                "meta" => {
                    if attr(attrs, "http-equiv").is_some_and(|v| v.eq_ignore_ascii_case("refresh"))
                    {
                        redirect = true;
                    }
                }
                "a" => {
                    if let Some(href) = attr(attrs, "href")
                        && let Some(t) = local_target(&page_dir, &href)
                        && !exists(&all, &t)
                    {
                        found.push(SiteCheck {
                            kind: CheckKind::BrokenLink,
                            page: url.clone(),
                            target: href,
                            size: 0,
                        });
                    }
                }
                "img" => {
                    let src = attr(attrs, "src").unwrap_or_default();
                    if attr(attrs, "alt").is_none() {
                        found.push(SiteCheck {
                            kind: CheckKind::MissingAlt,
                            page: url.clone(),
                            target: src.clone(),
                            size: 0,
                        });
                    }
                    if let Some(t) = local_target(&page_dir, &src)
                        && let Ok(m) = std::fs::metadata(out.join(&t))
                        && m.len() > HEAVY
                    {
                        found.push(SiteCheck {
                            kind: CheckKind::HeavyImage,
                            page: url.clone(),
                            target: src,
                            size: m.len(),
                        });
                    }
                }
                _ => {}
            }
        }
        // Page de redirection : rien à contrôler.
        if redirect {
            continue;
        }
        if title != Some(true) {
            push(
                SiteCheck {
                    kind: CheckKind::NoTitle,
                    page: url.clone(),
                    target: String::new(),
                    size: 0,
                },
                &mut checks,
            );
        }
        for c in found {
            push(c, &mut checks);
        }
    }
    checks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controles_du_site_genere() {
        let d = tempfile::tempdir().unwrap();
        let w = |p: &str, c: &[u8]| {
            let f = d.path().join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, c).unwrap();
        };
        w(
            "index.html",
            br##"<!doctype html><html><head><title>Accueil</title></head><body>
            <a href="/agence/">Agence</a> <a href="/disparue/">X</a> <a href="https://ex.com/">E</a>
            <a href="#haut">H</a> <a href="mailto:a@b.c">M</a> <a href='contact.html?x=1#f'>C</a>
            <a href="/media/plan%20du%20site.pdf">P</a> <a href="../hors.html">O</a>
            <img src="/media/lourde.jpg" alt=""> <img src="/media/legere.jpg">
            <!-- <a href="/commentee/">c</a> --><script>var s = '<a href="/script/">';</script>
            </body></html>"##,
        );
        w("agence/index.html", b"<html><head><title>  </title></head><body><img alt=\"Logo\" src=\"../media/legere.jpg\"></body></html>");
        w("contact.html", b"<title>Contact</title>");
        w(
            "ancien/index.html",
            b"<meta http-equiv=\"refresh\" content=\"0; url=/agence/\"><a href=\"/nulle/\">x</a>",
        );
        w("media/plan du site.pdf", b"%PDF");
        w("media/legere.jpg", &[0u8; 1000]);
        w("media/lourde.jpg", &vec![0u8; (HEAVY + 1) as usize]);
        let c = check_site(d.path());
        let got: Vec<(CheckKind, &str, &str)> = c
            .iter()
            .map(|c| (c.kind, c.page.as_str(), c.target.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                (CheckKind::NoTitle, "/agence/", ""),
                (CheckKind::BrokenLink, "/", "/disparue/"),
                (CheckKind::BrokenLink, "/", "../hors.html"),
                (CheckKind::HeavyImage, "/", "/media/lourde.jpg"),
                (CheckKind::MissingAlt, "/", "/media/legere.jpg"),
            ],
            "{c:?}"
        );
        assert_eq!(c[3].size, HEAVY + 1);
    }

    #[test]
    fn attributs() {
        assert_eq!(attr(r#" class="a" alt=''"#, "alt").as_deref(), Some(""));
        assert_eq!(attr(r#" data-alt="x" src=a.jpg"#, "alt"), None);
        assert_eq!(attr(r#" src=a.jpg alt"#, "alt").as_deref(), Some(""));
        assert_eq!(attr(r#" HREF = "/x/" "#, "href").as_deref(), Some("/x/"));
        assert_eq!(local_target("/a/", "b/"), Some("a/b/".into()));
        assert_eq!(local_target("/a/", "/"), Some(String::new()));
        assert_eq!(local_target("/", "tel:+33"), None);
    }
}
