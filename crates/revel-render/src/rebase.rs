//! Adaptation des adresses d'un site généré à un sous-dossier.
//!
//! Un site dont les liens partent de la racine (`/assets/site.css`) ne fonctionne pas tel quel
//! sous une adresse de la forme `proprietaire.github.io/depot/` : ces fonctions ajoutent le
//! préfixe (`/depot`) aux adresses depuis la racine des pages HTML et des feuilles de style.
//! Les adresses complètes (`https://…`), relatives (`img/a.jpg`), de protocole (`//cdn…`) et
//! les ancres (`#haut`) ne changent pas.

use regex::{Captures, Regex};
use std::sync::LazyLock;

/// Attributs contenant une seule adresse.
static ATTR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)(\s(?:href|src|action|poster|formaction|data-src|data-href|data-poster)\s*=\s*)(["'])/([^/"'][^"']*|)(["'])"#,
    )
    .expect("expression valide")
});

/// Attributs contenant une liste d'adresses avec largeurs (`srcset`).
static SRCSET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(\s(?:srcset|data-srcset|imagesrcset)\s*=\s*)(["'])([^"']*)(["'])"#)
        .expect("expression valide")
});

/// Redirection par balise `<meta http-equiv="refresh" content="0; url=/…">`.
static REFRESH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(content\s*=\s*["']\s*\d+\s*;\s*url\s*=\s*)/([^/])"#)
        .expect("expression valide")
});

/// `url(/…)` dans du CSS.
static CSS_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)url\((\s*)(["']?)/([^/"')])"#).expect("expression valide"));

/// Préfixe normalisé : `/depot` (commence par `/`, sans `/` final) ; vide si aucun.
fn normalize(prefix: &str) -> String {
    let p = prefix.trim().trim_matches('/');
    if p.is_empty() {
        String::new()
    } else {
        format!("/{p}")
    }
}

/// Feuille de style (ou contenu d'un attribut `style`).
pub fn rebase_css(css: &str, prefix: &str) -> String {
    let p = normalize(prefix);
    if p.is_empty() {
        return css.to_string();
    }
    CSS_URL
        .replace_all(css, |c: &Captures<'_>| {
            format!("url({}{}{p}/{}", &c[1], &c[2], &c[3])
        })
        .into_owned()
}

/// Page HTML : attributs d'adresse, `srcset`, redirections `meta refresh`, CSS intégré.
pub fn rebase_html(html: &str, prefix: &str) -> String {
    let p = normalize(prefix);
    if p.is_empty() {
        return html.to_string();
    }
    let s = ATTR.replace_all(html, |c: &Captures<'_>| {
        format!("{}{}{p}/{}{}", &c[1], &c[2], &c[3], &c[4])
    });
    let s = SRCSET.replace_all(&s, |c: &Captures<'_>| {
        let list = c[3]
            .split(',')
            .map(|cand| {
                let t = cand.trim_start();
                let lead = &cand[..cand.len() - t.len()];
                if t.starts_with('/') && !t.starts_with("//") {
                    format!("{lead}{p}{t}")
                } else {
                    cand.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("{}{}{list}{}", &c[1], &c[2], &c[4])
    });
    let s = REFRESH.replace_all(&s, |c: &Captures<'_>| format!("{}{p}/{}", &c[1], &c[2]));
    rebase_css(&s, &p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresses_html() {
        let html = r##"<link rel="stylesheet" href="/assets/site.css?v=1">
<a href="/">Accueil</a> <a href="/agence/#equipe">Agence</a>
<a href="https://exemple.fr/x">ext</a> <a href="//cdn.x/y">cdn</a> <a href="#haut">haut</a>
<img src='/img/a-480.webp' srcset="/img/a-480.webp 480w, /img/a-800.webp 800w, https://x/y.jpg 2x">
<video poster="/media/p.jpg" data-poster="/media/p.jpg"></video>
<meta http-equiv="refresh" content="0; url=/nouvelle/">
<div style="background:url(/img/fond.jpg)"></div>"##;
        let out = rebase_html(html, "alunites-revel-lab/");
        let p = "/alunites-revel-lab";
        assert!(out.contains(&format!(r#"href="{p}/assets/site.css?v=1""#)));
        assert!(out.contains(&format!(r#"href="{p}/">Accueil"#)));
        assert!(out.contains(&format!(r#"href="{p}/agence/#equipe""#)));
        assert!(out.contains(r#"href="https://exemple.fr/x""#));
        assert!(out.contains(r#"href="//cdn.x/y""#));
        assert!(out.contains(r##"href="#haut""##));
        assert!(out.contains(&format!("src='{p}/img/a-480.webp'")));
        assert!(out.contains(&format!(
            r#"srcset="{p}/img/a-480.webp 480w, {p}/img/a-800.webp 800w, https://x/y.jpg 2x""#
        )));
        assert!(out.contains(&format!(
            r#"poster="{p}/media/p.jpg" data-poster="{p}/media/p.jpg""#
        )));
        assert!(out.contains(&format!("url={p}/nouvelle/")));
        assert!(out.contains(&format!("url({p}/img/fond.jpg)")));
        // Idempotence non requise, mais un préfixe vide ne change rien.
        assert_eq!(rebase_html(html, ""), html);
    }

    #[test]
    fn adresses_css() {
        let css = r#"@font-face{src:url("/assets/f.woff2") format("woff2")}
.a{background:url( '/img/a.jpg' )} .b{background:url(img/b.jpg)} .c{background:url(//cdn/c.jpg)}
.d{background:url(data:image/png;base64,AAA)}"#;
        let out = rebase_css(css, "/depot");
        assert!(out.contains(r#"url("/depot/assets/f.woff2")"#));
        assert!(out.contains("url( '/depot/img/a.jpg' )"));
        assert!(out.contains("url(img/b.jpg)"));
        assert!(out.contains("url(//cdn/c.jpg)"));
        assert!(out.contains("url(data:image/png"));
    }
}
