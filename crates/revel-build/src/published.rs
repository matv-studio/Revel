//! Adresse du site publié d'un projet, lue dans ses fichiers : `homepage` de `package.json`,
//! `CNAME` (GitHub Pages), configuration de l'outil (`site` d'Astro, `baseURL` de Hugo, `url` de
//! Jekyll et de Docusaurus) et liens du README vers la racine d'un site. Les adresses proposées
//! sont ensuite vérifiées en ligne par l'appelant.

use std::path::Path;

/// Domaines qui ne sont jamais le site du projet (dépôts, badges, documentation des outils,
/// réseaux sociaux).
const NOT_SITES: &[&str] = &[
    "github.com",
    "raw.githubusercontent.com",
    "user-images.githubusercontent.com",
    "camo.githubusercontent.com",
    "gist.github.com",
    "docs.github.com",
    "pages.github.com",
    "gitlab.com",
    "bitbucket.org",
    "shields.io",
    "img.shields.io",
    "badge.fury.io",
    "npmjs.com",
    "www.npmjs.com",
    "npm.im",
    "opensource.org",
    "choosealicense.com",
    "creativecommons.org",
    "svelte.dev",
    "kit.svelte.dev",
    "vitejs.dev",
    "vite.dev",
    "astro.build",
    "docs.astro.build",
    "nextjs.org",
    "nuxt.com",
    "gohugo.io",
    "jekyllrb.com",
    "react.dev",
    "reactjs.org",
    "vuejs.org",
    "tailwindcss.com",
    "vercel.com",
    "netlify.com",
    "developer.mozilla.org",
    "youtube.com",
    "www.youtube.com",
    "youtu.be",
    "twitter.com",
    "x.com",
    "linkedin.com",
    "www.linkedin.com",
    "discord.gg",
    "discord.com",
    "wikipedia.org",
    "codesandbox.io",
    "stackblitz.com",
    "localhost",
    "127.0.0.1",
    "example.com",
    "example.org",
    "exemple.fr",
];

/// Hébergements de sites dont l'adresse est forcément celle d'un site publié.
const SITE_HOSTS: &[&str] = &[
    ".vercel.app",
    ".netlify.app",
    ".github.io",
    ".pages.dev",
    ".surge.sh",
    ".onrender.com",
    ".fly.dev",
    ".web.app",
    ".firebaseapp.com",
];

fn host_of(url: &str) -> Option<&str> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit('@').next()?;
    let host = host.split(':').next()?;
    (host.contains('.') || host == "localhost").then_some(host)
}

fn path_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    rest.find('/').map_or("", |i| &rest[i..])
}

fn excluded(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    NOT_SITES
        .iter()
        .any(|d| h == *d || h.ends_with(&format!(".{d}")))
}

/// Adresse nettoyée (sans ponctuation finale, `/` ajouté à un domaine nu).
fn clean(url: &str) -> Option<String> {
    let u = url
        .trim()
        .trim_end_matches(['.', ',', ')', ';', '>', '"', '\''])
        .to_string();
    let host = host_of(&u)?;
    if excluded(host) {
        return None;
    }
    if path_of(&u).is_empty() {
        Some(format!("{u}/"))
    } else {
        Some(u)
    }
}

/// Adresse de la racine d'un site (ou d'un site de projet `compte.github.io/projet/`).
fn is_site_root(url: &str) -> bool {
    let p = path_of(url).split(['?', '#']).next().unwrap_or("");
    let segs: Vec<&str> = p.split('/').filter(|s| !s.is_empty()).collect();
    segs.is_empty() || (segs.len() == 1 && host_of(url).is_some_and(|h| h.ends_with(".github.io")))
}

/// Valeur d'une clé simple dans un fichier de configuration (`clé = "valeur"`, `clé: valeur`,
/// `clé: 'valeur',`).
fn config_value(text: &str, keys: &[&str]) -> Option<String> {
    for line in text.lines() {
        let l = line.trim();
        for k in keys {
            let Some(rest) = l.strip_prefix(k) else {
                continue;
            };
            let rest = rest.trim_start();
            let Some(rest) = rest.strip_prefix('=').or_else(|| rest.strip_prefix(':')) else {
                continue;
            };
            let v = rest
                .trim()
                .trim_end_matches(',')
                .trim()
                .trim_matches(['"', '\'', '`']);
            if v.starts_with("http") {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Liens d'un texte Markdown : `[texte](adresse)` (hors images), `<adresse>` et adresses nues ;
/// avec le texte du lien quand il y en a un.
fn markdown_links(md: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let b = md.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'[' && (i == 0 || b[i - 1] != b'!') {
            if let Some(close) = md[i..].find("](").map(|c| i + c)
                && let Some(end) = md[close + 2..].find(')').map(|e| close + 2 + e)
                && !md[i + 1..close].contains('\n')
            {
                let text = md[i + 1..close].to_string();
                let url = md[close + 2..end].split_whitespace().next().unwrap_or("");
                if url.starts_with("http") {
                    out.push((text, url.to_string()));
                }
                i = end + 1;
                continue;
            }
        } else if md.is_char_boundary(i)
            && (md[i..].starts_with("http://") || md[i..].starts_with("https://"))
        {
            let prev = if i > 0 { b[i - 1] } else { b' ' };
            let end = md[i..]
                .find(|c: char| c.is_whitespace() || matches!(c, ')' | '>' | '"' | '\'' | ']'))
                .map_or(md.len(), |e| i + e);
            if prev != b'(' {
                out.push((String::new(), md[i..end].to_string()));
            }
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// Adresses possibles du site publié, de la plus sûre à la moins sûre, sans doublon.
pub fn candidates(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut push = |u: Option<String>| {
        if let Some(u) = u
            && !found
                .iter()
                .any(|f| f.trim_end_matches('/') == u.trim_end_matches('/'))
        {
            found.push(u);
        }
    };
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).ok();
    if let Some(pkg) =
        read("package.json").and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        && let Some(h) = pkg.get("homepage").and_then(|h| h.as_str())
    {
        push(clean(h));
    }
    if let Some(c) = read("CNAME")
        && let Some(domain) = c.lines().map(str::trim).find(|l| !l.is_empty())
    {
        push(clean(&format!("https://{domain}/")));
    }
    for (file, keys) in [
        ("astro.config.mjs", &["site"][..]),
        ("astro.config.ts", &["site"]),
        ("astro.config.js", &["site"]),
        ("hugo.toml", &["baseURL", "baseurl"]),
        ("config.toml", &["baseURL", "baseurl"]),
        ("hugo.yaml", &["baseURL"]),
        ("config.yaml", &["baseURL"]),
        ("docusaurus.config.js", &["url"]),
        ("docusaurus.config.ts", &["url"]),
    ] {
        if let Some(t) = read(file) {
            push(config_value(&t, keys).and_then(|u| clean(&u)));
        }
    }
    if let Some(t) = read("_config.yml")
        && let Some(url) = config_value(&t, &["url"])
    {
        let base = t
            .lines()
            .find_map(|l| l.trim().strip_prefix("baseurl:"))
            .map(|b| {
                b.trim()
                    .trim_matches(['"', '\''])
                    .trim_matches('/')
                    .to_string()
            })
            .unwrap_or_default();
        let u = if base.is_empty() {
            url
        } else {
            format!("{}/{base}/", url.trim_end_matches('/'))
        };
        push(clean(&u));
    }
    // README : liens vers la racine d'un site, les plus parlants d'abord.
    let readme = ["README.md", "readme.md", "README.markdown", "README"]
        .iter()
        .find_map(|f| read(f));
    if let Some(md) = readme {
        let mut links: Vec<(i32, String)> = markdown_links(&md)
            .into_iter()
            .filter_map(|(text, url)| {
                let u = clean(&url)?;
                if !is_site_root(&u) {
                    return None;
                }
                let host = host_of(&u)?.to_ascii_lowercase();
                let t = text.to_lowercase();
                let mut score = 0;
                if SITE_HOSTS.iter().any(|s| host.ends_with(s)) {
                    score += 2;
                }
                if [
                    "here", "demo", "live", "site", "website", "preview", "check", "ici", "démo",
                    "ligne", "voir",
                ]
                .iter()
                .any(|w| t.contains(w))
                {
                    score += 2;
                }
                Some((score, u))
            })
            .collect();
        links.sort_by_key(|(s, _)| -s);
        for (_, u) in links {
            push(Some(u));
        }
    }
    found
}

/// Site de projet GitHub Pages d'un dépôt `github.com/compte/dépôt` : `compte.github.io/dépôt/`
/// (ou `compte.github.io/` pour le dépôt `compte.github.io`).
pub fn github_pages(owner: &str, repo: &str) -> String {
    let owner = owner.to_ascii_lowercase();
    if repo.eq_ignore_ascii_case(&format!("{owner}.github.io")) {
        format!("https://{owner}.github.io/")
    } else {
        format!("https://{owner}.github.io/{repo}/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn readme_et_configuration() {
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("README.md"),
            "# persona 3 reload pause menu web remake!\n\ncheck it out [here](https://p3r.deltea.space)!\n\n\
             read more about it [here](https://www.deltea.space/blog/p3r-pause-menu)!\n\
             ![badge](https://img.shields.io/x.svg) Built with [SvelteKit](https://kit.svelte.dev).\n",
        )
        .unwrap();
        assert_eq!(candidates(d.path()), vec!["https://p3r.deltea.space/"]);
        fs::write(
            d.path().join("package.json"),
            r#"{"homepage":"https://github.com/deltea/p3r-pause-menu#readme"}"#,
        )
        .unwrap();
        fs::write(d.path().join("CNAME"), "www.atelier.fr\n").unwrap();
        fs::write(
            d.path().join("astro.config.mjs"),
            "export default defineConfig({\n  site: 'https://atelier.fr',\n});",
        )
        .unwrap();
        assert_eq!(
            candidates(d.path()),
            vec![
                "https://www.atelier.fr/",
                "https://atelier.fr/",
                "https://p3r.deltea.space/"
            ]
        );
    }

    #[test]
    fn jekyll_et_hugo() {
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("_config.yml"),
            "url: \"https://compte.github.io\"\nbaseurl: \"/blog\"\n",
        )
        .unwrap();
        fs::write(
            d.path().join("hugo.toml"),
            "baseURL = 'https://example.org/'\n",
        )
        .unwrap();
        assert_eq!(candidates(d.path()), vec!["https://compte.github.io/blog/"]);
        assert_eq!(
            github_pages("Compte", "site"),
            "https://compte.github.io/site/"
        );
        assert_eq!(
            github_pages("compte", "Compte.github.io"),
            "https://compte.github.io/"
        );
    }
}
