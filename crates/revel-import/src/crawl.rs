//! Lecture d'un site publié (7.4, import depuis un lien) : pages suivies depuis l'adresse donnée,
//! le plan du site (`sitemap.xml`) et les liens internes ; feuilles de style, scripts, images,
//! polices et vidéos téléchargés, y compris ceux des serveurs de l'outil d'origine (Framer, Webflow,
//! Google Fonts), rangés dans `_externe/<serveur>/…`. Les adresses des pages sont réécrites vers les
//! copies locales ; le dossier obtenu est un site statique, importé ensuite comme un dossier (7.1).
//!
//! `robots.txt` est respecté (règles `User-agent: *`). Le réseau passe par [`Fetch`] : en essai,
//! des pages en mémoire, jamais un site en ligne.

use dom_query::{Document, NodeData, NodeRef};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
pub use url::Url;

/// Réponse d'un serveur.
#[derive(Debug, Clone)]
pub struct Fetched {
    pub status: u16,
    /// Adresse finale (après redirections).
    pub url: Url,
    pub content_type: String,
    pub body: Vec<u8>,
}

/// Accès au réseau.
pub trait Fetch: Sync {
    fn get(&self, url: &Url) -> Result<Fetched, String>;

    /// Lecture avec des en-têtes (jeton d'une API). Par défaut : en-têtes ignorés.
    fn get_with(&self, url: &Url, headers: &[(&str, &str)]) -> Result<Fetched, String> {
        let _ = headers;
        self.get(url)
    }
}

/// Lecture avec deux nouvelles tentatives en cas d'échec passager (réseau, serveur occupé :
/// statut 429 ou 5xx).
fn get_retry(fetch: &(impl Fetch + ?Sized), url: &Url) -> Result<Fetched, String> {
    let mut last = fetch.get(url);
    for attempt in 1..=2u64 {
        let passing = match &last {
            Err(_) => true,
            Ok(r) => r.status == 429 || (500..600).contains(&r.status),
        };
        if !passing {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(300 * attempt));
        last = fetch.get(url);
    }
    last
}

/// Lit un document JSON (API) : erreur si le statut n'est pas 2xx (statut et début du corps).
pub fn get_json(
    fetch: &dyn Fetch,
    url: &str,
    headers: &[(&str, &str)],
) -> Result<serde_json::Value, (u16, String)> {
    let u = Url::parse(url).map_err(|e| (0, e.to_string()))?;
    let r = fetch.get_with(&u, headers).map_err(|e| (0, e))?;
    if !(200..300).contains(&r.status) {
        let body = String::from_utf8_lossy(&r.body);
        return Err((r.status, body.chars().take(300).collect()));
    }
    serde_json::from_slice(&r.body).map_err(|e| (r.status, e.to_string()))
}

/// Accès HTTP réel (certificats du système, délai de 30 s, 100 Mo au plus par fichier).
pub struct Http {
    agent: ureq::Agent,
}

impl Default for Http {
    fn default() -> Self {
        Self::new()
    }
}

impl Http {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .user_agent(concat!(
                "Mozilla/5.0 (compatible; Revel/",
                env!("CARGO_PKG_VERSION"),
                "; import)"
            ))
            .timeout_global(Some(std::time::Duration::from_secs(30)))
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                    .build(),
            )
            .build()
            .into();
        Self { agent }
    }
}

impl Fetch for Http {
    fn get(&self, url: &Url) -> Result<Fetched, String> {
        self.get_with(url, &[])
    }

    fn get_with(&self, url: &Url, headers: &[(&str, &str)]) -> Result<Fetched, String> {
        use ureq::ResponseExt;
        let mut req = self.agent.get(url.as_str());
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let mut res = req.call().map_err(|e| e.to_string())?;
        let status = res.status().as_u16();
        let final_url = Url::parse(&res.get_uri().to_string()).unwrap_or_else(|_| url.clone());
        let content_type = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        let body = res
            .body_mut()
            .with_config()
            .limit(MAX_FILE)
            .read_to_vec()
            .map_err(|e| e.to_string())?;
        Ok(Fetched {
            status,
            url: final_url,
            content_type,
            body,
        })
    }
}

/// Télécharge des fichiers (adresse, chemin relatif à `dest` avec `/`) par lots de `threads` ;
/// réussite de chacun, dans l'ordre. `progress` : fichiers écrits jusqu'ici.
pub fn download(
    fetch: &dyn Fetch,
    jobs: &[(Url, String)],
    dest: &Path,
    threads: usize,
    cancel: Option<&AtomicBool>,
    progress: &mut dyn FnMut(usize),
) -> Vec<bool> {
    let mut ok = vec![false; jobs.len()];
    let mut written = 0;
    for (n, batch) in jobs.chunks(threads.max(1)).enumerate() {
        if cancel.is_some_and(|c| c.load(Ordering::SeqCst)) {
            break;
        }
        let results: Vec<Option<Vec<u8>>> = std::thread::scope(|s| {
            let handles: Vec<_> = batch
                .iter()
                .map(|(u, _)| {
                    s.spawn(move || {
                        fetch
                            .get(u)
                            .ok()
                            .filter(|r| (200..300).contains(&r.status))
                            .map(|r| r.body)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().ok().flatten())
                .collect()
        });
        for (i, (body, (_, rel))) in results.into_iter().zip(batch).enumerate() {
            let Some(body) = body else { continue };
            let mut path = dest.to_path_buf();
            for seg in rel.split('/').filter(|s| !s.is_empty()) {
                path.push(safe(seg));
            }
            if path == dest {
                continue;
            }
            if path
                .parent()
                .is_some_and(|p| std::fs::create_dir_all(p).is_ok())
                && std::fs::write(&path, body).is_ok()
            {
                ok[n * threads.max(1) + i] = true;
                written += 1;
            }
        }
        progress(written);
    }
    ok
}

/// Taille maximale d'un fichier téléchargé.
const MAX_FILE: u64 = 100 * 1024 * 1024;

/// Réglages de la lecture.
pub struct Options<'a> {
    /// Nombre maximal de pages.
    pub max_pages: usize,
    /// Taille totale maximale des fichiers téléchargés.
    pub max_bytes: u64,
    /// Téléchargements simultanés.
    pub threads: usize,
    /// Demande d'interruption.
    pub cancel: Option<&'a AtomicBool>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self {
            max_pages: 2000,
            max_bytes: 2 * 1024 * 1024 * 1024,
            threads: 6,
            cancel: None,
        }
    }
}

/// Avancement : pages et fichiers lus, restant à lire, octets reçus.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    pub pages: usize,
    pub files: usize,
    pub queued: usize,
    pub bytes: u64,
}

/// Bilan de la lecture.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Adresse finale de la page de départ.
    pub start: String,
    pub pages: usize,
    pub files: usize,
    pub bytes: u64,
    /// Outil qui a produit le site (balise `generator`) : `Framer`, `Webflow`, `WordPress`…
    pub tool: Option<String>,
    /// Programme qui affiche les pages dans le navigateur (SvelteKit, Next.js…), gardé.
    pub runtime: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CrawlError {
    #[error("adresse non reconnue : {0}")]
    BadUrl(String),
    #[error("site injoignable : {0}")]
    Unreachable(String),
    #[error("l'adresse ne mène pas à une page HTML")]
    NotHtml,
    #[error("lecture du site annulée")]
    Cancelled,
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// Adresse donnée par l'utilisateur (`exemple.com`, `https://exemple.com/page`).
pub fn parse_start(input: &str) -> Option<Url> {
    let s = input.trim();
    if s.is_empty() || s.contains(char::is_whitespace) {
        return None;
    }
    let s = if s.contains("://") {
        s.to_string()
    } else {
        format!("https://{s}")
    };
    let u = Url::parse(&s).ok()?;
    (matches!(u.scheme(), "http" | "https")
        && u.host_str().is_some_and(|h| {
            h.contains('.') || h == "localhost" || h.parse::<std::net::IpAddr>().is_ok()
        }))
    .then_some(u)
}

/// Nom de dossier proposé pour un site (`www.atelier.fr` → `atelier.fr`).
pub fn site_name(u: &Url) -> String {
    u.host_str()
        .unwrap_or("site")
        .trim_start_matches("www.")
        .to_string()
}

/// Règles de `robots.txt` pour tous les robots : (autorisé, début d'adresse).
fn robots_rules(text: &str) -> (Vec<(bool, String)>, Vec<String>, Option<f64>) {
    let mut rules = Vec::new();
    let mut sitemaps = Vec::new();
    let mut delay = None;
    let mut applies = false;
    let mut in_group = false;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
        match k.as_str() {
            "user-agent" => {
                if !in_group {
                    applies = false;
                }
                in_group = true;
                if v == "*" {
                    applies = true;
                }
            }
            "disallow" | "allow" => {
                in_group = false;
                if applies && !v.is_empty() {
                    rules.push((k == "allow", v.to_string()));
                }
            }
            "sitemap" => sitemaps.push(v.to_string()),
            "crawl-delay" => {
                in_group = false;
                if applies {
                    delay = v.parse::<f64>().ok().filter(|d| *d > 0.0);
                }
            }
            _ => in_group = false,
        }
    }
    (rules, sitemaps, delay)
}

/// Adresse autorisée par `robots.txt` (règle la plus longue).
fn allowed(rules: &[(bool, String)], path: &str) -> bool {
    rules
        .iter()
        .filter(|(_, p)| path.starts_with(p.trim_end_matches('*')))
        .max_by_key(|(_, p)| p.len())
        .is_none_or(|(allow, _)| *allow)
}

/// Site lu : hôtes du site (avec et sans `www.`).
struct Site {
    hosts: Vec<String>,
    scheme: String,
}

impl Site {
    /// Adresse du site ramenée à la forme du domaine de la page de départ.
    fn canon(&self, u: &mut Url) {
        if self.owns(u) {
            let _ = u.set_host(Some(&self.hosts[0]));
            let _ = u.set_scheme(&self.scheme);
        }
    }

    fn owns(&self, u: &Url) -> bool {
        matches!(u.scheme(), "http" | "https")
            && u.host_str()
                .is_some_and(|h| self.hosts.iter().any(|x| x.eq_ignore_ascii_case(h)))
    }
}

/// Clé d'une page : chemin sans requête ni ancre, `/index.html` ramené au dossier.
fn page_key(u: &Url) -> String {
    let p = u.path();
    let p = p
        .strip_suffix("index.html")
        .or_else(|| p.strip_suffix("index.htm"))
        .unwrap_or(p);
    p.to_string()
}

/// Nom de fichier sûr sous Windows, macOS et Linux.
fn safe(seg: &str) -> String {
    let s: String = seg
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\\' => '_',
            c if (c as u32) < 32 => '_',
            c => c,
        })
        .collect();
    let s = s.trim_end_matches(['.', ' ']).to_string();
    if s.is_empty() || s == "." || s == ".." {
        "_".into()
    } else {
        s
    }
}

fn short_hash(s: &str) -> String {
    let h = s.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    format!("{:08x}", h as u32)
}

/// Extension d'un type de contenu.
fn ext_for(content_type: &str) -> Option<&'static str> {
    let t = content_type.split(';').next().unwrap_or("").trim();
    Some(match t {
        "text/css" => "css",
        "text/javascript" | "application/javascript" => "js",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/avif" => "avif",
        "image/svg+xml" => "svg",
        "image/x-icon" | "image/vnd.microsoft.icon" => "ico",
        "font/woff2" => "woff2",
        "font/woff" => "woff",
        "font/ttf" => "ttf",
        "font/otf" => "otf",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "application/json" => "json",
        "application/pdf" => "pdf",
        _ => return None,
    })
}

/// Chemin local (sans `/` initial) d'un fichier ; `html` : page.
/// Segment de chemin raccourci (noms de fichiers limités par les systèmes : 255 octets, et
/// chemins courts sous Windows) : début du nom, empreinte, extension.
fn short_seg(s: String) -> String {
    const MAX: usize = 80;
    if s.len() <= MAX {
        return s;
    }
    let ext = s
        .rsplit_once('.')
        .map(|(_, e)| e)
        .filter(|e| !e.is_empty() && e.len() <= 8 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    let mut cut = 48;
    while !s.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}~{}{ext}", &s[..cut], short_hash(&s))
}

fn local_path(site: &Site, u: &Url, html: bool, content_type: &str) -> String {
    let mut segs: Vec<String> = u
        .path_segments()
        .map(|s| {
            s.filter(|x| !x.is_empty())
                .map(|x| {
                    short_seg(safe(
                        &percent_encoding::percent_decode_str(x).decode_utf8_lossy(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let prefix = if site.owns(u) {
        String::new()
    } else {
        format!("_externe/{}/", safe(u.host_str().unwrap_or("serveur")))
    };
    if html {
        let last = segs
            .last()
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if u.path().ends_with('/') || segs.is_empty() {
            segs.push("index.html".into());
        } else if !(last.ends_with(".html") || last.ends_with(".htm")) {
            // `/agence`, `/contact.php` : dossier `agence/`, `contact.php/`.
            segs.push("index.html".into());
        }
        return format!("{prefix}{}", segs.join("/"));
    }
    if segs.is_empty() {
        segs.push("index".into());
    }
    let mut name = segs.pop().unwrap_or_default();
    if let Some(q) = u.query() {
        let tag = short_hash(q);
        name = match name.rsplit_once('.') {
            Some((stem, ext)) => format!("{stem}~{tag}.{ext}"),
            None => format!("{name}~{tag}"),
        };
    }
    if !name.contains('.')
        && let Some(ext) = ext_for(content_type)
    {
        name = format!("{name}.{ext}");
    }
    segs.push(short_seg(name));
    format!("{prefix}{}", segs.join("/"))
}

/// Adresse publiée d'un chemin local de page (`a/index.html` → `/a/`).
fn local_url(path: &str) -> String {
    let p = match path.strip_suffix("index.html") {
        Some(dir) => format!("/{dir}"),
        None => format!("/{path}"),
    };
    encode_path(&p)
}

/// Chemin local écrit dans une adresse : espaces, `%`, `#`, `?`, virgules, parenthèses et
/// guillemets codés (adresses de `srcset`, de `url()` sans guillemets).
fn encode_path(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    for c in p.chars() {
        match c {
            ' ' | '%' | '#' | '?' | ',' | '(' | ')' | '"' | '\'' | '<' | '>' => {
                out.push_str(&format!("%{:02X}", c as u32));
            }
            c if (c as u32) < 0x20 => out.push_str(&format!("%{:02X}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Attributs contenant une adresse.
const URL_ATTRS: &[&str] = &[
    "href",
    "src",
    "poster",
    "data",
    "action",
    "formaction",
    "background",
    "xlink:href",
];

/// Adresses citées par une feuille de style (`url(…)`, `@import "…"`).
/// Fin de l'argument de `url(` : indice de la parenthèse fermante, en sautant une adresse entre
/// guillemets (`url("photo (1080p).jpg")`).
pub(crate) fn url_close(rest: &str) -> Option<usize> {
    let t = rest.trim_start();
    let lead = rest.len() - t.len();
    if let Some(q @ ('"' | '\'')) = t.chars().next() {
        let b = t.as_bytes();
        let mut k = 1;
        while k < b.len() {
            if b[k] == b'\\' {
                k += 2;
                continue;
            }
            if b[k] == q as u8 {
                return t[k + 1..].find(')').map(|e| lead + k + 1 + e);
            }
            k += 1;
        }
        return None;
    }
    rest.find(')')
}

fn css_refs(css: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = css;
    while let Some(i) = rest.find("url(") {
        rest = &rest[i + 4..];
        let Some(end) = url_close(rest) else { break };
        let u = rest[..end].trim().trim_matches(['"', '\'']).trim();
        if !u.is_empty() && !u.starts_with("data:") && !u.starts_with('#') {
            out.push(u.to_string());
        }
        rest = &rest[end..];
    }
    let mut rest = css;
    while let Some(i) = rest.find("@import") {
        rest = &rest[i + 7..];
        let t = rest.trim_start();
        if let Some(q @ ('"' | '\'')) = t.chars().next()
            && let Some(end) = t[1..].find(q)
        {
            out.push(t[1..1 + end].to_string());
        }
    }
    out
}

/// Remplace les adresses d'une feuille de style selon `map` (adresse absolue → adresse locale).
fn rewrite_css(css: &str, base: &Url, map: &HashMap<String, String>) -> String {
    let resolve = |u: &str| -> Option<String> {
        let abs = base.join(u.trim()).ok()?;
        map.get(abs.as_str()).cloned()
    };
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(i) = rest.find("url(") {
        out.push_str(&rest[..i + 4]);
        rest = &rest[i + 4..];
        let Some(end) = url_close(rest) else { break };
        let inner = &rest[..end];
        let t = inner.trim();
        let q = t.chars().next().filter(|c| *c == '"' || *c == '\'');
        let u = t.trim_matches(['"', '\'']);
        match resolve(u) {
            Some(local) => {
                let q = q.map(String::from).unwrap_or_default();
                out.push_str(&format!("{q}{local}{q}"));
            }
            None => out.push_str(inner),
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    // @import "…"
    let mut done = String::with_capacity(out.len());
    let mut rest = out.as_str();
    while let Some(i) = rest.find("@import") {
        done.push_str(&rest[..i + 7]);
        rest = &rest[i + 7..];
        let t = rest.trim_start();
        let ws = &rest[..rest.len() - t.len()];
        if let Some(q @ ('"' | '\'')) = t.chars().next()
            && let Some(end) = t[1..].find(q)
            && let Some(local) = resolve(&t[1..1 + end])
        {
            done.push_str(ws);
            done.push(q);
            done.push_str(&local);
            done.push(q);
            rest = &t[2 + end..];
        }
    }
    done.push_str(rest);
    done
}

/// Adresses d'un attribut `srcset`.
fn srcset_urls(v: &str) -> Vec<String> {
    v.split(',')
        .filter_map(|p| p.split_whitespace().next().map(str::to_string))
        .filter(|u| !u.is_empty())
        .collect()
}

/// Liens et fichiers d'une page.
#[derive(Default)]
struct Found {
    pages: Vec<Url>,
    files: Vec<Url>,
}

fn element_name(n: &NodeRef) -> Option<String> {
    n.query(|t| match &t.data {
        NodeData::Element(e) => Some(e.name.local.to_string()),
        _ => None,
    })
    .flatten()
}

/// Extensions de fichiers liés par `<a>` à télécharger (et non à suivre comme des pages).
const FILE_EXT: &[&str] = &[
    "pdf", "zip", "png", "jpg", "jpeg", "gif", "webp", "avif", "svg", "mp4", "webm", "mp3", "doc",
    "docx", "xls", "xlsx", "ppt", "pptx", "txt", "csv", "ico",
];

fn is_file_link(u: &Url) -> bool {
    u.path()
        .rsplit('/')
        .next()
        .and_then(|l| l.rsplit_once('.'))
        .is_some_and(|(_, e)| FILE_EXT.contains(&e.to_ascii_lowercase().as_str()))
}

fn scan_page(doc: &Document, base: &Url, site: &Site) -> Found {
    let mut f = Found::default();
    let join = |v: &str| -> Option<Url> {
        let v = v.trim();
        if v.is_empty()
            || v.starts_with('#')
            || v.starts_with("data:")
            || v.starts_with("javascript:")
            || v.starts_with("mailto:")
            || v.starts_with("tel:")
        {
            return None;
        }
        let mut u = base.join(v).ok()?;
        u.set_fragment(None);
        site.canon(&mut u);
        matches!(u.scheme(), "http" | "https").then_some(u)
    };
    for n in doc.root().descendants() {
        let Some(tag) = element_name(&n) else {
            continue;
        };
        let attr = |name: &str| n.attr(name).map(|v| v.to_string());
        match tag.as_str() {
            "a" | "area" => {
                if let Some(u) = attr("href").and_then(|v| join(&v))
                    && site.owns(&u)
                {
                    if is_file_link(&u) {
                        f.files.push(u);
                    } else {
                        f.pages.push(u);
                    }
                }
            }
            "link" => {
                let rel = attr("rel").unwrap_or_default().to_ascii_lowercase();
                if let Some(u) = attr("href").and_then(|v| join(&v)) {
                    let keep = rel.split_whitespace().any(|r| {
                        matches!(
                            r,
                            "stylesheet"
                                | "icon"
                                | "apple-touch-icon"
                                | "mask-icon"
                                | "manifest"
                                | "preload"
                                | "modulepreload"
                        )
                    });
                    // Scripts du moteur de Framer : retirés à l'import (animations reprises).
                    let framer = u.host_str().is_some_and(|h| h.contains("framer"));
                    let script = rel.contains("modulepreload")
                        || (rel.contains("preload") && attr("as").as_deref() == Some("script"));
                    if keep && !(script && framer) {
                        f.files.push(u);
                    }
                }
                for u in attr("imagesrcset")
                    .map(|v| srcset_urls(&v))
                    .unwrap_or_default()
                {
                    if let Some(u) = join(&u) {
                        f.files.push(u);
                    }
                }
            }
            _ => {
                for a in ["src", "poster", "data"] {
                    // `data` : fichier d'un `<object>` seulement (ailleurs : données d'un
                    // composant).
                    if a == "data" && tag != "object" {
                        continue;
                    }
                    if let Some(u) = attr(a).and_then(|v| join(&v))
                        && tag != "iframe"
                    {
                        f.files.push(u);
                    }
                }
                for u in attr("srcset").map(|v| srcset_urls(&v)).unwrap_or_default() {
                    if let Some(u) = join(&u) {
                        f.files.push(u);
                    }
                }
                if let Some(style) = attr("style") {
                    for u in css_refs(&style) {
                        if let Some(u) = join(&u) {
                            f.files.push(u);
                        }
                    }
                }
                if tag == "style" {
                    for u in css_refs(&n.text()) {
                        if let Some(u) = join(&u) {
                            f.files.push(u);
                        }
                    }
                }
                // Script écrit dans la page : modules qu'il charge (`import("./app.js")`).
                if tag == "script" && attr("src").is_none() {
                    let text = n.text();
                    for m in js_modules(&text) {
                        if let Some(u) = join(&m) {
                            f.files.push(u);
                        }
                    }
                }
            }
        }
    }
    f
}

/// Réécrit les adresses d'une page vers les copies locales.
fn rewrite_page(doc: &Document, base: &Url, site: &Site, map: &HashMap<String, String>) {
    let resolve = |v: &str| -> Option<String> {
        let t = v.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("data:") {
            return None;
        }
        let mut u = base.join(t).ok()?;
        let frag = u.fragment().map(|f| format!("#{f}"));
        u.set_fragment(None);
        site.canon(&mut u);
        if let Some(local) = map.get(u.as_str()) {
            return Some(format!("{local}{}", frag.unwrap_or_default()));
        }
        // Page du site non lue : adresse depuis la racine.
        if site.owns(&u) {
            let mut p = u.path().to_string();
            if let Some(q) = u.query() {
                p.push('?');
                p.push_str(q);
            }
            return Some(format!("{p}{}", frag.unwrap_or_default()));
        }
        None
    };
    for n in doc.root().descendants() {
        let Some(tag) = element_name(&n) else {
            continue;
        };
        for a in URL_ATTRS {
            if *a == "data" && tag != "object" {
                continue;
            }
            if let Some(v) = n.attr(a)
                && let Some(local) = resolve(&v)
            {
                n.set_attr(a, &local);
            }
        }
        for a in ["srcset", "imagesrcset"] {
            if let Some(v) = n.attr(a) {
                let out: Vec<String> = v
                    .split(',')
                    .map(|p| {
                        let p = p.trim();
                        match p.split_once(char::is_whitespace) {
                            Some((u, d)) => format!(
                                "{} {}",
                                resolve(u).unwrap_or_else(|| u.to_string()),
                                d.trim()
                            ),
                            None => resolve(p).unwrap_or_else(|| p.to_string()),
                        }
                    })
                    .collect();
                n.set_attr(a, &out.join(", "));
            }
        }
        if let Some(style) = n.attr("style") {
            n.set_attr("style", &rewrite_css(&style, base, map));
        }
        if tag == "style" {
            let css = n.text().to_string();
            let new = rewrite_css(&css, base, map);
            if new != css {
                n.set_text(new);
            }
        }
        if tag == "base" {
            n.remove_from_parent();
        }
        // Connexions préparées vers des serveurs extérieurs : inutiles, les fichiers sont copiés.
        if tag == "link"
            && n.attr("rel").is_some_and(|r| {
                r.split_whitespace().any(|t| {
                    t.eq_ignore_ascii_case("preconnect") || t.eq_ignore_ascii_case("dns-prefetch")
                })
            })
        {
            n.remove_from_parent();
        }
    }
}

/// Outil d'origine d'après la balise `generator` (ou ses marques) de la page de départ.
/// Modules cités par un script : `import … from "…"`, `export … from "…"`, `import("…")`,
/// `new URL("…", import.meta.url)`, `import "…"` (adresses relatives ou absolues seulement : les
/// noms de paquets nus ne sont pas des fichiers).
pub(crate) fn js_modules(js: &str) -> Vec<String> {
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r#"(?:\bfrom\s*|\bimport\s*\(\s*|\bimport\s+|new\s+URL\s*\(\s*)["'`]([^"'`\s]+)["'`]"#,
        )
        .expect("expression des modules")
    });
    let mut out: Vec<String> = Vec::new();
    for c in RE.captures_iter(js) {
        let spec = &c[1];
        if (spec.starts_with("./")
            || spec.starts_with("../")
            || spec.starts_with('/')
            || spec.starts_with("http://")
            || spec.starts_with("https://"))
            && !spec.contains("${")
            // Données du CMS de Framer : adresse calculée autrement par le script (`/cms/`),
            // lues par l'import Framer (D-109).
            && !spec.ends_with(".framercms")
            && !out.iter().any(|o| o == spec)
        {
            out.push(spec.to_string());
        }
    }
    out
}

/// Extensions des fichiers qu'un script charge par leur adresse (sons, vidéos, images, données,
/// polices, modèles 3D, animations).
const SCRIPT_ASSETS: &str = "mp3|wav|ogg|oga|m4a|aac|flac|mp4|webm|mov|png|jpe?g|gif|webp|avif|svg|ico|json|woff2?|ttf|otf|glb|gltf|bin|wasm|lottie|riv|css|js|mjs|txt|xml|pdf";

/// Fichiers cités par une adresse depuis la racine dans un script (`"/sfx/navigation.wav"`).
pub(crate) fn js_root_assets(js: &str) -> Vec<String> {
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(&format!(
            r#"["'`](/[^"'`\s<>{{}}$\\]+?\.(?:{SCRIPT_ASSETS}))(?:\?[^"'`\s]*)?["'`]"#
        ))
        .expect("expression des fichiers")
    });
    let mut out: Vec<String> = Vec::new();
    for c in RE.captures_iter(js) {
        let p = c[1].to_string();
        if !p.starts_with("//") && !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

/// Fichiers dont l'adresse est composée dans un script (`` `/music/${pistes[i]}` ``) : chaque
/// début d'adresse est essayé avec les noms de fichiers cités dans le même script. Adresses
/// supposées, ignorées si le site ne les a pas.
pub(crate) fn js_composed_assets(js: &str) -> Vec<String> {
    static PREFIX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r#"`(/[^`$\s]*/)\$\{"#).expect("expression des débuts d'adresse")
    });
    static NAME: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(&format!(
            r#"["']([^"'/\\\n<>]{{1,120}}\.(?:{SCRIPT_ASSETS}))["']"#
        ))
        .expect("expression des noms de fichiers")
    });
    let prefixes: Vec<&str> = PREFIX
        .captures_iter(js)
        .filter_map(|c| c.get(1).map(|m| m.as_str()))
        .collect();
    if prefixes.is_empty() {
        return Vec::new();
    }
    let names: Vec<&str> = NAME
        .captures_iter(js)
        .filter_map(|c| c.get(1).map(|m| m.as_str()))
        .filter(|n| !n.starts_with('.'))
        .collect();
    let mut out: Vec<String> = Vec::new();
    for p in &prefixes {
        for n in &names {
            let u = format!("{p}{n}");
            if !out.contains(&u) && out.len() < 300 {
                out.push(u);
            }
        }
    }
    out
}

/// Programme qui affiche et anime les pages dans le navigateur (gardé avec le site).
pub(crate) fn app_runtime(html: &str) -> Option<&'static str> {
    [
        ("__sveltekit_", "SvelteKit"),
        ("data-sveltekit", "SvelteKit"),
        ("__NEXT_DATA__", "Next.js"),
        ("self.__next_f", "Next.js"),
        ("window.__NUXT__", "Nuxt"),
        ("id=\"__nuxt\"", "Nuxt"),
        ("<astro-island", "Astro"),
        ("ng-version=", "Angular"),
        ("data-server-rendered", "Vue"),
        ("__remixContext", "Remix"),
        ("__reactRouterContext", "React Router"),
        ("___gatsby", "Gatsby"),
    ]
    .iter()
    .find(|(m, _)| html.contains(m))
    .map(|(_, n)| *n)
}

fn detect_tool(doc: &Document) -> Option<String> {
    let gen_text = doc
        .select("meta[name=generator]")
        .attr("content")
        .map(|s| s.to_string())
        .unwrap_or_default();
    let g = gen_text.to_ascii_lowercase();
    let html = doc.html_root();
    Some(
        if g.starts_with("framer") || doc.select("[data-framer-hydrate-v2]").exists() {
            "Framer"
        } else if g.contains("webflow") || html.has_attr("data-wf-site") {
            "Webflow"
        } else if g.starts_with("wordpress") {
            "WordPress"
        } else if g.contains("wix.com") {
            "Wix"
        } else if g.contains("squarespace") {
            "Squarespace"
        } else if g.contains("shopify") {
            "Shopify"
        } else {
            return None;
        }
        .to_string(),
    )
}

/// Lit le site de `start` et l'écrit dans `dest` (dossier vide ou absent).
pub fn crawl(
    start: &Url,
    dest: &Path,
    fetch: &dyn Fetch,
    opts: &Options,
    progress: &mut dyn FnMut(Progress),
) -> Result<Report, CrawlError> {
    let stopped = || opts.cancel.is_some_and(|c| c.load(Ordering::SeqCst));
    let first = fetch
        .get(start)
        .map_err(|e| CrawlError::Unreachable(e.to_string()))?;
    if first.status >= 400 {
        return Err(CrawlError::Unreachable(format!(
            "{} ({})",
            start, first.status
        )));
    }
    if !first.content_type.contains("html") && !first.body.starts_with(b"<") {
        return Err(CrawlError::NotHtml);
    }
    let origin = first.url.clone();
    let host = origin.host_str().unwrap_or_default().to_string();
    let alias = match host.strip_prefix("www.") {
        Some(h) => h.to_string(),
        None => format!("www.{host}"),
    };
    let site = Site {
        hosts: vec![host.clone(), alias],
        scheme: origin.scheme().to_string(),
    };
    let root =
        |p: &str| -> Option<Url> { Url::parse(&format!("{}://{host}{p}", site.scheme)).ok() };
    let mut report = Report {
        start: origin.to_string(),
        ..Default::default()
    };

    // robots.txt et plan du site.
    let (rules, mut sitemaps, delay) = match root("/robots.txt").map(|u| fetch.get(&u)) {
        Some(Ok(r)) if r.status < 400 => robots_rules(&String::from_utf8_lossy(&r.body)),
        _ => (Vec::new(), Vec::new(), None),
    };
    // Rythme demandé par le site (`Crawl-delay`) : pages lues une à une, pause plafonnée à 2 s,
    // fichiers deux par deux.
    let pause = delay.map(|d| std::time::Duration::from_secs_f64(d.min(2.0)));
    let page_threads = if pause.is_some() {
        1
    } else {
        opts.threads.max(1)
    };
    let file_threads = if pause.is_some() {
        2
    } else {
        opts.threads.max(1)
    };
    if let Some(d) = delay {
        report.warnings.push(format!(
            "rythme de lecture réduit à la demande du site (robots.txt : Crawl-delay {d} s, pause de {:.1} s au plus)",
            d.min(2.0)
        ));
    }
    if sitemaps.is_empty() {
        sitemaps.push(format!("{}://{host}/sitemap.xml", site.scheme));
    }
    let mut pages: VecDeque<Url> = VecDeque::new();
    let mut seen_pages: HashSet<String> = HashSet::new();
    let canonical_host = host.clone();
    let canonical_scheme = site.scheme.clone();
    let add_page = |mut u: Url, pages: &mut VecDeque<Url>, seen: &mut HashSet<String>| {
        if u.query().is_some() || !allowed(&rules, u.path()) {
            return;
        }
        // Autre forme du domaine (`www.`) : ramenée à celle du site.
        let _ = u.set_host(Some(&canonical_host));
        let _ = u.set_scheme(&canonical_scheme);
        if seen.insert(page_key(&u)) {
            pages.push_back(u);
        }
    };
    add_page(origin.clone(), &mut pages, &mut seen_pages);
    let mut sm_queue: VecDeque<(String, usize)> = sitemaps.into_iter().map(|s| (s, 0)).collect();
    let mut sm_seen = HashSet::new();
    while let Some((s, depth)) = sm_queue.pop_front() {
        if depth > 2 || !sm_seen.insert(s.clone()) {
            continue;
        }
        let Ok(u) = Url::parse(&s) else { continue };
        let Ok(r) = fetch.get(&u) else { continue };
        if r.status >= 400 {
            continue;
        }
        let text = String::from_utf8_lossy(&r.body);
        let index = text.contains("<sitemapindex");
        let mut rest = text.as_ref();
        while let Some(i) = rest.find("<loc>") {
            rest = &rest[i + 5..];
            let Some(end) = rest.find("</loc>") else {
                break;
            };
            let loc = rest[..end].trim().replace("&amp;", "&");
            rest = &rest[end..];
            if index {
                sm_queue.push_back((loc, depth + 1));
            } else if let Ok(pu) = Url::parse(&loc)
                && site.owns(&pu)
            {
                add_page(pu, &mut pages, &mut seen_pages);
            }
        }
    }

    std::fs::create_dir_all(dest)?;
    // Adresse absolue → adresse locale ; contenus des pages et feuilles de style à réécrire.
    let mut map: HashMap<String, String> = HashMap::new();
    let mut saved_pages: Vec<(Url, String, String)> = Vec::new();
    let mut saved_css: Vec<(Url, String, String)> = Vec::new();
    let mut files: VecDeque<Url> = VecDeque::new();
    let mut seen_files: HashSet<String> = HashSet::new();
    let mut used_paths: HashSet<String> = HashSet::new();
    let mut prog = Progress::default();
    let mut warned_size = false;
    // Adresses supposées (composées dans un script) : absentes sans avertissement.
    let mut guessed: HashSet<String> = HashSet::new();
    let mut runtime: Option<&'static str> = None;

    let mut prefetched = Some(first);
    // Pages, par lots lus en parallèle.
    let mut first_batch = true;
    while !pages.is_empty() && report.pages < opts.max_pages {
        if stopped() {
            return Err(CrawlError::Cancelled);
        }
        if let Some(p) = pause
            && !first_batch
        {
            std::thread::sleep(p);
        }
        first_batch = false;
        let batch: Vec<Url> = (0..page_threads)
            .filter_map(|_| pages.pop_front())
            .collect();
        let results: Vec<(Url, Result<Fetched, String>)> = std::thread::scope(|s| {
            let handles: Vec<_> = batch
                .iter()
                .map(|u| {
                    let pre = if prefetched
                        .as_ref()
                        .is_some_and(|p| p.url == *u || *u == origin)
                    {
                        prefetched.take()
                    } else {
                        None
                    };
                    let u = u.clone();
                    s.spawn(move || {
                        let r = match pre {
                            Some(p) => Ok(p),
                            None => get_retry(fetch, &u),
                        };
                        (u, r)
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        for (u, r) in results {
            let Ok(r) = r else {
                report.warnings.push(format!("{u} : page illisible"));
                continue;
            };
            if r.status >= 400 || !(r.content_type.contains("html") || r.body.starts_with(b"<")) {
                continue;
            }
            // Redirection vers une autre page du site : la page d'arrivée.
            if !site.owns(&r.url) {
                continue;
            }
            let text = String::from_utf8_lossy(&r.body).into_owned();
            let doc = Document::from(text.as_str());
            if runtime.is_none() {
                runtime = app_runtime(&text);
            }
            if report.pages == 0 {
                report.tool = detect_tool(&doc);
                let words = doc.select("body").text().split_whitespace().count();
                if words < 30 && doc.select("script[src]").exists() {
                    report.warnings.push(
                        "page d'accueil presque vide : contenu produit par JavaScript dans le navigateur, peu de texte repris".into(),
                    );
                }
            }
            let found = scan_page(&doc, &r.url, &site);
            for p in found.pages {
                if site.owns(&p) && report.pages + pages.len() < opts.max_pages * 2 {
                    add_page(p, &mut pages, &mut seen_pages);
                }
            }
            for f in found.files {
                if seen_files.insert(f.as_str().to_string()) {
                    files.push_back(f);
                }
            }
            let mut path = local_path(&site, &r.url, true, &r.content_type);
            if !used_paths.insert(path.clone()) {
                continue;
            }
            let url_local = local_url(&path);
            map.insert(u.as_str().to_string(), url_local.clone());
            map.insert(r.url.as_str().to_string(), url_local);
            if path.is_empty() {
                path = "index.html".into();
            }
            saved_pages.push((r.url.clone(), path, text));
            report.pages += 1;
            prog.pages = report.pages;
            prog.bytes += r.body.len() as u64;
            prog.queued = pages.len() + files.len();
            progress(prog);
        }
    }
    if !pages.is_empty() {
        report.warnings.push(format!(
            "plus de {} pages : les suivantes ne sont pas reprises",
            opts.max_pages
        ));
    }

    // Fichiers (et ceux cités par les feuilles de style), par lots.
    while !files.is_empty() {
        if stopped() {
            return Err(CrawlError::Cancelled);
        }
        let batch: Vec<Url> = (0..file_threads)
            .filter_map(|_| files.pop_front())
            .collect();
        let results: Vec<(Url, Result<Fetched, String>)> = std::thread::scope(|s| {
            let handles: Vec<_> = batch
                .iter()
                .map(|u| {
                    let u = u.clone();
                    s.spawn(move || {
                        let r = get_retry(fetch, &u);
                        (u, r)
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        for (u, r) in results {
            let r = match r {
                Ok(r) if r.status < 400 => r,
                _ => {
                    if !guessed.contains(u.as_str()) {
                        report.warnings.push(format!("{u} : fichier introuvable"));
                    }
                    continue;
                }
            };
            if prog.bytes + r.body.len() as u64 > opts.max_bytes {
                if !warned_size {
                    report.warnings.push(
                        "taille maximale atteinte : certains fichiers ne sont pas repris".into(),
                    );
                    warned_size = true;
                }
                continue;
            }
            let mut path = local_path(&site, &u, false, &r.content_type);
            while !used_paths.insert(path.clone()) {
                path = match path.rsplit_once('.') {
                    Some((a, b)) => format!("{a}-1.{b}"),
                    None => format!("{path}-1"),
                };
            }
            map.insert(u.as_str().to_string(), encode_path(&format!("/{path}")));
            // Script : modules qu'il charge et fichiers qu'il cite, copiés aux mêmes adresses
            // (le script n'est pas réécrit).
            let is_js = r.content_type.contains("javascript")
                || path.ends_with(".js")
                || path.ends_with(".mjs");
            if is_js {
                let text = String::from_utf8_lossy(&r.body);
                let mut add = |cu: Url, guess: bool| {
                    if matches!(cu.scheme(), "http" | "https")
                        && seen_files.insert(cu.as_str().to_string())
                    {
                        if guess {
                            guessed.insert(cu.as_str().to_string());
                        }
                        files.push_back(cu);
                    }
                };
                for m in js_modules(&text) {
                    if let Ok(mut cu) = r.url.join(&m) {
                        cu.set_fragment(None);
                        add(cu, false);
                    }
                }
                for a in js_root_assets(&text) {
                    if let Some(cu) = root(&a) {
                        add(cu, true);
                    }
                }
                for a in js_composed_assets(&text) {
                    if let Some(cu) = root(&a) {
                        add(cu, true);
                    }
                }
            }
            let is_css = r.content_type.contains("css") || path.ends_with(".css");
            if is_css {
                let text = String::from_utf8_lossy(&r.body).into_owned();
                for c in css_refs(&text) {
                    if let Ok(mut cu) = r.url.join(&c) {
                        cu.set_fragment(None);
                        if matches!(cu.scheme(), "http" | "https")
                            && seen_files.insert(cu.as_str().to_string())
                        {
                            files.push_back(cu);
                        }
                    }
                }
                saved_css.push((r.url.clone(), path, text));
            } else {
                let full = dest.join(&path);
                let written = full
                    .parent()
                    .is_none_or(|p| std::fs::create_dir_all(p).is_ok())
                    && std::fs::write(&full, &r.body).is_ok();
                if !written {
                    report
                        .warnings
                        .push(format!("{} : fichier non écrit ({path})", r.url));
                    continue;
                }
            }
            report.files += 1;
            prog.files = report.files;
            prog.bytes += r.body.len() as u64;
            prog.queued = files.len();
            progress(prog);
        }
    }

    // Écriture des pages et feuilles de style, adresses réécrites.
    for (u, path, css) in &saved_css {
        let full = dest.join(path);
        let written = full
            .parent()
            .is_none_or(|p| std::fs::create_dir_all(p).is_ok())
            && std::fs::write(&full, rewrite_css(css, u, &map)).is_ok();
        if !written {
            report
                .warnings
                .push(format!("{u} : feuille de style non écrite ({path})"));
        }
    }
    for (u, path, text) in &saved_pages {
        let doc = Document::from(text.as_str());
        rewrite_page(&doc, u, &site, &map);
        let full = dest.join(path);
        let written = full
            .parent()
            .is_none_or(|p| std::fs::create_dir_all(p).is_ok())
            && std::fs::write(&full, doc.html().as_bytes()).is_ok();
        if !written {
            report
                .warnings
                .push(format!("{u} : page non écrite ({path})"));
        }
    }
    if let Some(rt) = runtime {
        report.runtime = Some(rt.to_string());
        report.warnings.push(format!(
            "pages affichées et animées par le programme du site ({rt}) : ses scripts sont gardés ; dans le site publié, il peut réafficher les parties qu'il produit et remplacer les modifications faites dans Revel"
        ));
    }
    report.bytes = prog.bytes;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chemins_locaux_codes() {
        assert_eq!(
            local_url("_externe/cdn.example/a_Team 1.webp"),
            "/_externe/cdn.example/a_Team%201.webp"
        );
        assert_eq!(local_url("a (1080p),x.jpg"), "/a%20%281080p%29%2Cx.jpg");
        assert_eq!(local_url("blog/index.html"), "/blog/");
        assert_eq!(local_url("café/index.html"), "/café/");
    }

    #[test]
    fn adresses_entre_guillemets() {
        let css = r#"a{background:url("https://x.example/a (1080p)-poster.jpg")}b{background:url(https://x.example/b.png)}"#;
        assert_eq!(
            css_refs(css),
            vec![
                "https://x.example/a (1080p)-poster.jpg",
                "https://x.example/b.png"
            ]
        );
        assert_eq!(url_close(r#""a)b" ) x"#), Some(6));
    }

    #[test]
    fn noms_longs_raccourcis() {
        let long = format!("{}.webp", "a".repeat(200));
        let s = short_seg(long.clone());
        assert!(s.len() <= 80, "{s}");
        assert!(s.ends_with(".webp") && s.starts_with("aaaa"));
        assert_ne!(short_seg(format!("{}.webp", "b".repeat(200))), s);
        assert_eq!(short_seg("court.png".into()), "court.png");
        let accents = "é".repeat(60);
        assert!(short_seg(accents).len() <= 80);
    }

    /// Site en mémoire.
    struct Mem(HashMap<String, (u16, &'static str, Vec<u8>)>);

    impl Fetch for Mem {
        fn get(&self, url: &Url) -> Result<Fetched, String> {
            let (status, ct, body) = self.0.get(url.as_str()).cloned().unwrap_or((
                404,
                "text/plain",
                b"absent".to_vec(),
            ));
            // Redirection simulée : `http://` vers `https://www.`.
            Ok(Fetched {
                status,
                url: url.clone(),
                content_type: ct.into(),
                body,
            })
        }
    }

    fn mem(entries: &[(&str, &'static str, &str)]) -> Mem {
        Mem(entries
            .iter()
            .map(|(u, ct, b)| (u.to_string(), (200, *ct, b.as_bytes().to_vec())))
            .collect())
    }

    const HTML: &str = "text/html; charset=utf-8";

    #[test]
    fn modules_et_fichiers_des_scripts() {
        let entry = r#"import{l as o,a as r}from"../chunks/B6.js";export{o as load_css}from"./x.js";const n=()=>import("../nodes/0.js");import "./side.js";new URL("./w.wasm",import.meta.url);import x from "svelte";"#;
        assert_eq!(
            js_modules(entry),
            vec![
                "../chunks/B6.js",
                "./x.js",
                "../nodes/0.js",
                "./side.js",
                "./w.wasm"
            ]
        );
        let page = r#"const s=new Howl({src:["/sfx/navigation.wav"]});const t=["Changing Seasons -Reload-.mp3","Full Moon Full Life.mp3"];h(`/music/${t[i]}`);fetch("/api/data");el.src="//cdn.x/a.png""#;
        assert_eq!(js_root_assets(page), vec!["/sfx/navigation.wav"]);
        assert_eq!(
            js_composed_assets(page),
            vec![
                "/music/Changing Seasons -Reload-.mp3",
                "/music/Full Moon Full Life.mp3"
            ]
        );
        assert_eq!(
            app_runtime("<script>__sveltekit_1we8 = {}</script>"),
            Some("SvelteKit")
        );
        assert_eq!(app_runtime("<p>simple</p>"), None);
    }

    #[test]
    fn programme_du_site_garde() {
        let f = mem(&[
            (
                "https://app.fr/",
                HTML,
                r#"<html><head><link rel="modulepreload" href="/_app/entry/start.js"></head><body><h1>Menu</h1><div>
<script>__sveltekit_x={};Promise.all([import("./_app/entry/start.js"),import("./_app/entry/app.js")])</script></div></body></html>"#,
            ),
            (
                "https://app.fr/_app/entry/start.js",
                "text/javascript",
                r#"import{a}from"../chunks/c1.js";"#,
            ),
            (
                "https://app.fr/_app/entry/app.js",
                "text/javascript",
                r#"const n=()=>import("../nodes/2.js");"#,
            ),
            (
                "https://app.fr/_app/chunks/c1.js",
                "text/javascript",
                "export const a=1;",
            ),
            (
                "https://app.fr/_app/nodes/2.js",
                "text/javascript",
                r#"const s=["/sfx/nav.wav"];const t=["Piste un.mp3","Absente.mp3"];play(`/music/${t[0]}`);"#,
            ),
            ("https://app.fr/sfx/nav.wav", "audio/wav", "RIFF"),
            ("https://app.fr/music/Piste%20un.mp3", "audio/mpeg", "ID3"),
        ]);
        let d = tempfile::tempdir().unwrap();
        let r = crawl(
            &parse_start("app.fr").unwrap(),
            d.path(),
            &f,
            &Options::default(),
            &mut |_| {},
        )
        .unwrap();
        for p in [
            "_app/entry/start.js",
            "_app/entry/app.js",
            "_app/chunks/c1.js",
            "_app/nodes/2.js",
            "sfx/nav.wav",
            "music/Piste un.mp3",
        ] {
            assert!(d.path().join(p).is_file(), "{p} absent");
        }
        assert_eq!(r.runtime.as_deref(), Some("SvelteKit"));
        // Adresse supposée absente : pas d'avertissement « fichier introuvable ».
        assert!(
            !r.warnings.iter().any(|w| w.contains("Absente")),
            "{:?}",
            r.warnings
        );
        assert!(r.warnings.iter().any(|w| w.contains("SvelteKit")));
    }

    #[test]
    fn adresses_de_depart() {
        assert_eq!(
            parse_start("atelier.fr").unwrap().as_str(),
            "https://atelier.fr/"
        );
        assert!(parse_start("ftp://x.fr").is_none());
        assert!(parse_start("pas une adresse").is_none());
        assert_eq!(
            site_name(&parse_start("https://www.atelier.fr/a").unwrap()),
            "atelier.fr"
        );
    }

    #[test]
    fn robots() {
        let (r, s, d) = robots_rules(
            "User-agent: Googlebot\nDisallow: /\n\nUser-agent: *\nDisallow: /prive/\nAllow: /prive/public\nCrawl-delay: 10\nSitemap: https://x.fr/plan.xml\n",
        );
        assert_eq!(d, Some(10.0));
        assert!(allowed(&r, "/a/"));
        assert!(!allowed(&r, "/prive/a"));
        assert!(allowed(&r, "/prive/public/a"));
        assert_eq!(s, vec!["https://x.fr/plan.xml".to_string()]);
    }

    #[test]
    fn site_lu_et_adresses_reecrites() {
        let f = mem(&[
            (
                "https://atelier.fr/",
                HTML,
                r#"<!DOCTYPE html><html><head><meta name="generator" content="Framer 1a2b"><link rel="stylesheet" href="/css/site.css"><link rel="icon" href="https://atelier.fr/favicon.png"><style>@font-face{src:url(https://fonts.gstatic.com/s/inter.woff2)}</style></head><body><a href="/agence">Agence</a> <a href="https://www.atelier.fr/contact#form">Contact</a> <a href="/prive/x">Privé</a> <a href="https://ailleurs.com/">Ailleurs</a> <a href="/doc.pdf">PDF</a><img src="https://cdn.framerusercontent.com/images/a.png?width=40" srcset="https://cdn.framerusercontent.com/images/a.png?width=40 1x, https://cdn.framerusercontent.com/images/a.png?width=80 2x" alt=""></body></html>"#,
            ),
            (
                "https://atelier.fr/agence",
                HTML,
                r#"<html><head><link rel="stylesheet" href="css/site.css"></head><body><h1>Agence</h1><a href="/">Accueil</a></body></html>"#,
            ),
            (
                "https://atelier.fr/contact",
                HTML,
                "<html><body><h1>Contact</h1></body></html>",
            ),
            (
                "https://atelier.fr/plan-secret/",
                HTML,
                "<html><body><h1>Seulement dans le plan du site</h1></body></html>",
            ),
            (
                "https://atelier.fr/robots.txt",
                "text/plain",
                "User-agent: *\nDisallow: /prive/\n",
            ),
            (
                "https://atelier.fr/sitemap.xml",
                "application/xml",
                "<urlset><url><loc>https://atelier.fr/plan-secret/</loc></url><url><loc>https://atelier.fr/</loc></url></urlset>",
            ),
            (
                "https://atelier.fr/css/site.css",
                "text/css",
                "body{background:url(../img/fond.jpg)} @import \"theme.css\";",
            ),
            (
                "https://atelier.fr/css/theme.css",
                "text/css",
                "h1{color:red}",
            ),
            ("https://atelier.fr/img/fond.jpg", "image/jpeg", "jpg"),
            ("https://atelier.fr/favicon.png", "image/png", "png"),
            ("https://atelier.fr/doc.pdf", "application/pdf", "pdf"),
            (
                "https://fonts.gstatic.com/s/inter.woff2",
                "font/woff2",
                "woff2",
            ),
            (
                "https://cdn.framerusercontent.com/images/a.png?width=40",
                "image/png",
                "a40",
            ),
            (
                "https://cdn.framerusercontent.com/images/a.png?width=80",
                "image/png",
                "a80",
            ),
        ]);
        let dir = tempfile::tempdir().unwrap();
        let mut events = 0;
        let r = crawl(
            &parse_start("atelier.fr").unwrap(),
            dir.path(),
            &f,
            &Options::default(),
            &mut |_| events += 1,
        )
        .unwrap();
        assert_eq!(r.tool.as_deref(), Some("Framer"));
        assert_eq!(r.pages, 4, "{:?}", r.warnings);
        assert!(events > 0);
        let read = |p: &str| {
            std::fs::read_to_string(dir.path().join(p)).unwrap_or_else(|e| panic!("{p} : {e}"))
        };
        let home = read("index.html");
        assert!(home.contains(r#"href="/agence/""#), "{home}");
        assert!(home.contains(r#"href="/contact/#form""#), "{home}");
        assert!(
            home.contains(r#"href="/prive/x""#),
            "page interdite : adresse depuis la racine : {home}"
        );
        assert!(home.contains(r#"href="https://ailleurs.com/""#), "{home}");
        assert!(home.contains(r#"href="/doc.pdf""#), "{home}");
        assert!(
            home.contains("/_externe/fonts.gstatic.com/s/inter.woff2"),
            "{home}"
        );
        assert!(
            home.contains("/_externe/cdn.framerusercontent.com/images/a~"),
            "{home}"
        );
        assert!(!dir.path().join("prive").exists());
        assert!(read("plan-secret/index.html").contains("Seulement"));
        assert!(read("agence/index.html").contains(r#"href="/css/site.css""#));
        let css = read("css/site.css");
        assert!(
            css.contains("url(/img/fond.jpg)") && css.contains("@import \"/css/theme.css\""),
            "{css}"
        );
        assert_eq!(read("css/theme.css"), "h1{color:red}");
        assert_eq!(read("img/fond.jpg"), "jpg");
        assert_eq!(read("doc.pdf"), "pdf");
        // Deux tailles de la même image : deux fichiers.
        let imgs: Vec<_> =
            std::fs::read_dir(dir.path().join("_externe/cdn.framerusercontent.com/images"))
                .unwrap()
                .flatten()
                .collect();
        assert_eq!(imgs.len(), 2);
        // Le dossier obtenu s'importe comme un site statique.
        assert_eq!(
            crate::detect(dir.path()),
            crate::SiteKind::Static { pages: 4 }
        );
    }

    #[test]
    fn interruption_et_site_injoignable() {
        let f = mem(&[(
            "https://a.fr/",
            HTML,
            "<html><body><a href=\"/b\">b</a></body></html>",
        )]);
        let dir = tempfile::tempdir().unwrap();
        let stop = AtomicBool::new(true);
        let o = Options {
            cancel: Some(&stop),
            ..Default::default()
        };
        assert!(matches!(
            crawl(
                &parse_start("a.fr").unwrap(),
                dir.path(),
                &f,
                &o,
                &mut |_| {}
            ),
            Err(CrawlError::Cancelled)
        ));
        assert!(matches!(
            crawl(
                &parse_start("b.fr").unwrap(),
                dir.path(),
                &f,
                &Options::default(),
                &mut |_| {}
            ),
            Err(CrawlError::Unreachable(_))
        ));
    }
}
