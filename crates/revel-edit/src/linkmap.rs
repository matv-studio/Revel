//! Plan du site (D-103) : pages, contenus des collections, menus et parties communes (gabarits,
//! composants) reliés par les liens qu'ils contiennent.
//!
//! - lien écrit (`<a href="/contact/">`, `{{ '/contact/' | url }}`) : relié à la page de cette
//!   adresse, sinon à la collection dont le modèle d'adresse la reçoit, sinon montré comme une
//!   page introuvable ; une adresse redirigée mène à la page de destination ;
//! - lien d'une liste : vers la collection parcourue (`{{ p.url }}` dans une liste de projets) ou
//!   vers le menu du site parcouru ;
//! - valeur d'un composant (`href: "'/contact/'"`), lien d'un menu du site, redirection ;
//! - adresse d'un autre site (D-107) : lien, média intégré (`<iframe src>`, `<video src>`…),
//!   adresse écrite dans un champ ou le texte d'un contenu du CMS, information du site. Les
//!   plateformes de vidéos et les réseaux sociaux ([`PLATFORMS`]) sont regroupés à part.
//!
//! Chaque lien garde ce qu'il faut pour le changer de destination ([`retarget`]) ou remplacer son
//! adresse partout où elle sert ([`replace_url`]).

use crate::address::{Owner, Target};
use crate::collection::{FieldList, Query};
use crate::command::Command;
use crate::siteinfo::{self, MenuLink};
use revel_model::{Node, Site};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Sorte d'un nœud du plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// Page du site.
    Page,
    /// Collection qui a une page par contenu (modèle d'adresse).
    Collection,
    /// Contenu d'une collection (sa page).
    Entry,
    /// Gabarit dont des liens sont sur plusieurs pages.
    Layout,
    /// Composant qui contient des liens.
    Component,
    /// Menu du site (informations du site).
    Menu,
    /// Adresse du site sans page.
    Missing,
    /// Autre site (par domaine).
    External,
    /// Plateforme de vidéos ou réseau social ([`PLATFORMS`], D-107).
    Media,
}

/// Nœud du plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapNode {
    /// Clé stable : `page:<id>`, `col:<id>`, `entry:<collection>/<fichier>`, `layout:<id>`,
    /// `comp:<id>`, `menu:<nom>`, `missing:<adresse>`, `ext:<domaine>`, `media:<plateforme>`.
    pub key: String,
    pub kind: NodeKind,
    /// Nom affiché (nom de la page, de la collection, du menu…).
    pub label: String,
    /// Adresse (page, contenu, modèle d'adresse d'une collection, adresse introuvable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Page en brouillon.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub draft: bool,
    /// Collection d'un contenu ; nombre de pages d'une partie commune ; contenus d'une collection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

/// D'où vient un lien, et comment le changer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "via", rename_all = "snake_case")]
pub enum LinkVia {
    /// Attribut `href` d'un élément.
    Attr { at: Target },
    /// Valeur d'une propriété d'un composant utilisé.
    Arg { at: Target, name: String },
    /// Lien d'un menu du site (position dans le menu).
    MenuItem { menu: String, index: usize },
    /// Redirection du site (position dans la liste).
    Redirect { index: usize },
    /// Liste qui affiche un lien par contenu d'une collection.
    List { at: Target },
    /// Liste qui affiche les liens d'un menu du site.
    Menu { at: Target },
    /// Média intégré : attribut (`src`) d'un `<iframe>`, `<video>`, `<audio>`, `<source>`…
    Src { at: Target, attr: String },
    /// Adresse écrite dans un champ d'un contenu du CMS (`pointer` : pointeur JSON dans son
    /// en-tête, `/video` ou `/blocks/2/video`).
    Field {
        folder: String,
        file: String,
        pointer: String,
    },
    /// Adresse écrite dans le texte (Markdown) d'un contenu du CMS.
    Body { folder: String, file: String },
    /// Information du site (chemin sous `site`).
    Info { path: Vec<String> },
}

/// Lien du plan : d'un nœud vers un autre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapLink {
    pub from: String,
    pub to: String,
    /// Adresse écrite (vide pour une liste).
    pub href: String,
    /// Texte du lien (ou nom du composant, du menu).
    pub text: String,
    pub via: LinkVia,
    /// L'adresse écrite est redirigée vers la destination.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub redirected: bool,
}

impl MapLink {
    /// Destination changeable (lien écrit, valeur d'un composant, lien d'un menu, redirection).
    pub fn retargetable(&self) -> bool {
        matches!(
            self.via,
            LinkVia::Attr { .. }
                | LinkVia::Arg { .. }
                | LinkVia::MenuItem { .. }
                | LinkVia::Redirect { .. }
        )
    }
}

/// Contenu d'une collection (pour relier les liens écrits vers sa page).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryRef {
    pub collection: String,
    pub file: String,
    pub url: String,
    pub title: String,
}

/// Contenu d'une collection du CMS lu pour les adresses écrites dans ses champs et son texte
/// (D-107). `collection` : identifiant de la collection du site (clé `entry:` du plan), sinon
/// celui du CMS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryText {
    pub collection: String,
    pub folder: String,
    pub file: String,
    pub title: String,
    pub data: Value,
    #[serde(default)]
    pub body: String,
}

/// Sorte de plateforme en ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformKind {
    Video,
    Audio,
    Social,
}

/// Plateformes de vidéos, de sons et réseaux sociaux : identifiant, nom, sorte, domaines (le
/// domaine ou l'un de ses sous-domaines).
pub const PLATFORMS: &[(&str, &str, PlatformKind, &[&str])] = &[
    (
        "youtube",
        "YouTube",
        PlatformKind::Video,
        &["youtube.com", "youtu.be", "youtube-nocookie.com"],
    ),
    (
        "vimeo",
        "Vimeo",
        PlatformKind::Video,
        &["vimeo.com", "vimeocdn.com"],
    ),
    (
        "dailymotion",
        "Dailymotion",
        PlatformKind::Video,
        &["dailymotion.com", "dai.ly"],
    ),
    ("twitch", "Twitch", PlatformKind::Video, &["twitch.tv"]),
    (
        "soundcloud",
        "SoundCloud",
        PlatformKind::Audio,
        &["soundcloud.com"],
    ),
    ("spotify", "Spotify", PlatformKind::Audio, &["spotify.com"]),
    (
        "instagram",
        "Instagram",
        PlatformKind::Social,
        &["instagram.com"],
    ),
    (
        "facebook",
        "Facebook",
        PlatformKind::Social,
        &["facebook.com", "fb.com", "fb.watch"],
    ),
    (
        "linkedin",
        "LinkedIn",
        PlatformKind::Social,
        &["linkedin.com", "lnkd.in"],
    ),
    (
        "x",
        "X (Twitter)",
        PlatformKind::Social,
        &["x.com", "twitter.com"],
    ),
    ("tiktok", "TikTok", PlatformKind::Social, &["tiktok.com"]),
    (
        "pinterest",
        "Pinterest",
        PlatformKind::Social,
        &["pinterest.com", "pinterest.fr", "pin.it"],
    ),
    ("behance", "Behance", PlatformKind::Social, &["behance.net"]),
    (
        "threads",
        "Threads",
        PlatformKind::Social,
        &["threads.net", "threads.com"],
    ),
    ("bluesky", "Bluesky", PlatformKind::Social, &["bsky.app"]),
    (
        "whatsapp",
        "WhatsApp",
        PlatformKind::Social,
        &["wa.me", "whatsapp.com"],
    ),
];

/// Plateforme d'un domaine (`www.` retiré) : identifiant, nom, sorte.
pub fn platform_of(host: &str) -> Option<(&'static str, &'static str, PlatformKind)> {
    let h = host.to_ascii_lowercase();
    let h = h.strip_prefix("www.").unwrap_or(&h);
    PLATFORMS
        .iter()
        .find(|(_, _, _, domains)| {
            domains
                .iter()
                .any(|d| h == *d || h.ends_with(&format!(".{d}")))
        })
        .map(|(id, name, kind, _)| (*id, *name, *kind))
}

/// Plateforme d'un nœud `media:<id>`.
pub fn platform(key: &str) -> Option<(&'static str, &'static str, PlatformKind)> {
    let id = key.strip_prefix("media:")?;
    PLATFORMS
        .iter()
        .find(|p| p.0 == id)
        .map(|(id, name, kind, _)| (*id, *name, *kind))
}

/// Adresses absolues (`http://`, `https://`) écrites dans un texte (HTML, Markdown compris).
pub fn urls_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("http") {
        let tail = &rest[i..];
        if !(tail.starts_with("https://") || tail.starts_with("http://")) {
            rest = &rest[i + 4..];
            continue;
        }
        let end = tail
            .find(|c: char| c.is_whitespace() || "\"'<>()[]{}\\`".contains(c))
            .unwrap_or(tail.len());
        let url = tail[..end].trim_end_matches(['.', ',', ';', ':', '!', '?']);
        // Adresse calculée (`…/video/{{ vid }}`) : pas une adresse écrite.
        let computed = tail[end..].starts_with("{{") || tail[end..].starts_with("{%");
        if url.len() > "https://".len() && !computed {
            out.push(url.to_string());
        }
        rest = &tail[end.max(1)..];
    }
    out
}

/// Plan du site.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LinkMap {
    pub nodes: Vec<MapNode>,
    pub links: Vec<MapLink>,
}

impl LinkMap {
    pub fn node(&self, key: &str) -> Option<&MapNode> {
        self.nodes.iter().find(|n| n.key == key)
    }

    /// Pages (et contenus) sans lien qui y mène, sauf la page d'accueil et la page 404.
    pub fn orphans(&self) -> BTreeSet<String> {
        let reached: HashSet<&str> = self.links.iter().map(|l| l.to.as_str()).collect();
        self.nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Page | NodeKind::Collection))
            .filter(|n| !matches!(n.path.as_deref(), Some("/") | Some("/404.html")))
            .filter(|n| !reached.contains(n.key.as_str()))
            .filter(|n| {
                // Collection : un de ses contenus atteint suffit.
                n.kind != NodeKind::Collection
                    || !self.nodes.iter().any(|e| {
                        e.parent.as_deref() == Some(n.key.as_str())
                            && reached.contains(e.key.as_str())
                    })
            })
            .map(|n| n.key.clone())
            .collect()
    }
}

/// Clé d'une page.
pub fn page_key(id: &str) -> String {
    format!("page:{id}")
}

/// Adresse sans origine du site, sans requête ni ancre ; `index.html` retiré ; barre finale
/// ajoutée à une adresse sans extension.
fn normalize(path: &str) -> String {
    let p = path.split(['?', '#']).next().unwrap_or("");
    let mut p = if p.is_empty() {
        "/".to_string()
    } else {
        p.to_string()
    };
    if let Some(stripped) = p.strip_suffix("index.html") {
        p = stripped.to_string();
    }
    let last = p.rsplit('/').next().unwrap_or("");
    if !p.ends_with('/') && !last.contains('.') {
        p.push('/');
    }
    p
}

/// Partie après l'adresse (requête, ancre) : gardée quand la destination change.
fn suffix(href: &str) -> &str {
    href.find(['?', '#']).map_or("", |i| &href[i..])
}

/// Adresse relative résolue depuis le dossier de `base`.
fn join(base: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    if !base.ends_with('/') {
        parts.pop();
    }
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    let mut out = format!("/{}", parts.join("/"));
    if rel.ends_with('/') && !out.ends_with('/') {
        out.push('/');
    }
    out
}

/// Domaine d'une adresse absolue (`https://exemple.fr/x` -> `exemple.fr`) et son chemin.
fn split_absolute(href: &str) -> Option<(String, String)> {
    let rest = href
        .strip_prefix("https://")
        .or_else(|| href.strip_prefix("http://"))
        .or_else(|| href.strip_prefix("//"))?;
    let (host, path) = rest.split_at(rest.find(['/', '?', '#']).unwrap_or(rest.len()));
    let host = host.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    Some((
        host,
        if path.is_empty() {
            "/".into()
        } else {
            path.into()
        },
    ))
}

/// Adresse littérale d'une expression `'/x/' | url` (ou `"/x/"`).
fn literal_url(expr: &str) -> Option<String> {
    let e = expr.trim();
    let e = e
        .strip_suffix("| url")
        .or_else(|| e.strip_suffix("|url"))
        .unwrap_or(e)
        .trim();
    let inner = e
        .strip_prefix('\'')
        .and_then(|x| x.strip_suffix('\''))
        .or_else(|| e.strip_prefix('"').and_then(|x| x.strip_suffix('"')))?;
    (!inner.contains(['\'', '"'])).then(|| inner.to_string())
}

/// Valeur d'un attribut `href` : adresse écrite, `{{ '/x/' | url }}` ; `None` si calculée.
fn static_href(v: &str) -> Option<String> {
    let t = v.trim();
    if let Some(inner) = t.strip_prefix("{{").and_then(|x| x.strip_suffix("}}")) {
        return literal_url(inner);
    }
    (!t.contains("{{") && !t.contains("{%")).then(|| t.to_string())
}

/// Variable de la valeur `{{ x.url }}` (`{{ x.url | url }}`, `{{ x.data.url }}`, `{{ x.page.url }}`).
fn item_url_var(v: &str) -> Option<String> {
    let inner = v.trim().strip_prefix("{{")?.strip_suffix("}}")?.trim();
    let head = inner.split('|').next()?.trim();
    let var = head.strip_suffix(".url").map(|h| {
        h.strip_suffix(".data")
            .or(h.strip_suffix(".page"))
            .unwrap_or(h)
    })?;
    (!var.is_empty() && var.chars().all(|c| c.is_alphanumeric() || c == '_')).then(|| var.into())
}

/// Modèle d'adresse (`/project/{{ slug }}/`) reconnaissant `path`.
fn matches_template(template: &str, path: &str) -> bool {
    // Parties littérales entre les valeurs `{{ … }}` ; une valeur : un ou plusieurs caractères
    // sans `/`.
    let mut lits: Vec<&str> = Vec::new();
    let mut rest = template;
    while let Some(i) = rest.find("{{") {
        lits.push(&rest[..i]);
        match rest[i..].find("}}") {
            Some(j) => rest = &rest[i + j + 2..],
            None => return false,
        }
    }
    lits.push(rest);
    if lits.len() == 1 {
        return normalize(template) == path;
    }
    let mut pos = 0usize;
    for (k, lit) in lits.iter().enumerate() {
        if k == 0 {
            if !path.starts_with(lit) {
                return false;
            }
            pos = lit.len();
            continue;
        }
        let last = k == lits.len() - 1;
        // Valeur : jusqu'au prochain littéral (le dernier finit l'adresse).
        let found = if last {
            if lit.is_empty() {
                Some(path.len())
            } else {
                path.ends_with(lit).then(|| path.len() - lit.len())
            }
        } else if lit.is_empty() {
            return false;
        } else {
            path[pos..].find(lit).map(|i| pos + i)
        };
        let Some(end) = found else {
            return false;
        };
        if end <= pos || path[pos..end].contains('/') {
            return false;
        }
        pos = end + lit.len();
    }
    pos == path.len()
}

/// Texte d'un lien : textes et valeurs de ses enfants.
fn text_of(nodes: &[Node]) -> String {
    fn go(nodes: &[Node], out: &mut String) {
        for n in nodes {
            match n {
                Node::Text { text } => out.push_str(text),
                Node::Out { expr } => {
                    let e = expr.split('|').next().unwrap_or("").trim();
                    out.push_str(&format!("{{{e}}}"));
                }
                Node::Element { children, .. } => go(children, out),
                Node::If { then, .. } => go(then, out),
                Node::Image(img) => {
                    if out.trim().is_empty()
                        && let Some(a) = &img.alt
                    {
                        out.push_str(literal_url(a).as_deref().unwrap_or(a.trim_matches('"')));
                    }
                }
                _ => {}
            }
            out.push(' ');
        }
    }
    let mut s = String::new();
    go(nodes, &mut s);
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() > 60 {
        format!("{}…", s.chars().take(59).collect::<String>())
    } else {
        s
    }
}

/// Balises d'un média intégré dont l'adresse (`src`) peut venir d'un autre site.
const EMBEDS: &[&str] = &["iframe", "video", "audio", "source", "embed", "track"];

/// Liste en cours : variable de l'élément et ce qu'elle parcourt.
#[derive(Clone)]
enum Loop {
    Collection(Vec<String>),
    Menu(String),
    Other,
}

struct Builder<'a> {
    site: &'a Site,
    pages: Vec<(String, String)>,
    /// Modèles d'adresse : (clé du nœud, modèle). Une collection dont les contenus sont connus
    /// n'y est pas : ses adresses sont celles des contenus.
    templates: Vec<(String, String)>,
    entries: HashMap<String, String>,
    redirects: HashMap<String, String>,
    host: Option<String>,
    locales: Vec<String>,
    nodes: BTreeMap<String, MapNode>,
    links: Vec<MapLink>,
}

/// Destination d'une adresse écrite.
enum Dest {
    Node(String, bool),
    Ignore,
}

impl Builder<'_> {
    fn resolve(&mut self, from_path: Option<&str>, href: &str) -> Dest {
        let h = href.trim();
        if h.is_empty()
            || h.starts_with('#')
            || h.contains(':') && !h.starts_with("http://") && !h.starts_with("https://")
        {
            return Dest::Ignore;
        }
        let path = if let Some((host, path)) = split_absolute(h) {
            if self.host.as_deref() != Some(host.as_str()) {
                let (key, kind, label) = match platform_of(&host) {
                    Some((id, name, _)) => {
                        (format!("media:{id}"), NodeKind::Media, name.to_string())
                    }
                    None => (format!("ext:{host}"), NodeKind::External, host),
                };
                self.nodes.entry(key.clone()).or_insert(MapNode {
                    key: key.clone(),
                    kind,
                    label,
                    path: None,
                    draft: false,
                    parent: None,
                    count: None,
                });
                return Dest::Node(key, false);
            }
            path
        } else if h.starts_with('/') {
            h.to_string()
        } else {
            join(from_path.unwrap_or("/"), h)
        };
        let mut path = normalize(&path);
        // Fichier (document, image) : pas une page.
        let last = path.rsplit('/').next().unwrap_or("");
        if last.contains('.') && !last.ends_with(".html") {
            return Dest::Ignore;
        }
        let mut redirected = false;
        if let Some(to) = self.redirects.get(&path) {
            path = normalize(to);
            redirected = true;
        }
        // Adresse d'une autre langue (`/en/contact/`) : même page.
        let candidates: Vec<String> = std::iter::once(path.clone())
            .chain(self.locales.iter().filter_map(|l| {
                path.strip_prefix(&format!("/{l}/"))
                    .map(|rest| format!("/{rest}"))
                    .or_else(|| (path == format!("/{l}/")).then(|| "/".to_string()))
            }))
            .collect();
        for p in &candidates {
            if let Some((id, _)) = self.pages.iter().find(|(_, pp)| pp == p) {
                return Dest::Node(page_key(id), redirected);
            }
            if let Some(key) = self.entries.get(p) {
                return Dest::Node(key.clone(), redirected);
            }
            if let Some((key, _)) = self.templates.iter().find(|(_, t)| matches_template(t, p)) {
                return Dest::Node(key.clone(), redirected);
            }
        }
        let key = format!("missing:{path}");
        self.nodes.entry(key.clone()).or_insert(MapNode {
            key: key.clone(),
            kind: NodeKind::Missing,
            label: path.clone(),
            path: Some(path),
            draft: false,
            parent: None,
            count: None,
        });
        Dest::Node(key, redirected)
    }

    fn add(&mut self, from: &str, from_path: Option<&str>, href: &str, text: String, via: LinkVia) {
        if let Dest::Node(to, redirected) = self.resolve(from_path, href)
            && to != from
        {
            self.links.push(MapLink {
                from: from.to_string(),
                to,
                href: href.to_string(),
                text,
                via,
                redirected,
            });
        }
    }

    fn walk(
        &mut self,
        owner: &Owner,
        from: &str,
        from_path: Option<&str>,
        nodes: &[Node],
        loops: &mut Vec<(String, Loop, Target)>,
    ) {
        for n in nodes {
            match n {
                Node::Element {
                    id,
                    tag,
                    attrs,
                    children,
                } => {
                    if tag.eq_ignore_ascii_case("a")
                        && let Some(id) = id
                        && let Some(href) = attrs
                            .iter()
                            .find(|a| a.name == "href")
                            .and_then(|a| a.value.as_deref())
                    {
                        let at = Target {
                            owner: owner.clone(),
                            node: id.clone(),
                        };
                        if let Some(lit) = static_href(href) {
                            let text = text_of(children);
                            self.add(from, from_path, &lit, text, LinkVia::Attr { at });
                        } else if let Some(var) = item_url_var(href)
                            && let Some((_, lp, list_at)) =
                                loops.iter().rev().find(|(v, _, _)| *v == var).cloned()
                        {
                            self.loop_link(from, &lp, list_at);
                        }
                    }
                    // Média intégré d'un autre site (D-107).
                    if let Some(id) = id
                        && EMBEDS.iter().any(|t| tag.eq_ignore_ascii_case(t))
                    {
                        for attr in ["src", "data-src"] {
                            let Some(lit) = attrs
                                .iter()
                                .find(|a| a.name == attr)
                                .and_then(|a| a.value.as_deref())
                                .and_then(static_href)
                            else {
                                continue;
                            };
                            if split_absolute(&lit).is_some() {
                                let text = attrs
                                    .iter()
                                    .find(|a| a.name == "title")
                                    .and_then(|a| a.value.clone())
                                    .unwrap_or_else(|| format!("<{tag}>"));
                                let at = Target {
                                    owner: owner.clone(),
                                    node: id.clone(),
                                };
                                self.add(
                                    from,
                                    from_path,
                                    &lit,
                                    text,
                                    LinkVia::Src {
                                        at,
                                        attr: attr.to_string(),
                                    },
                                );
                            }
                        }
                    }
                    self.walk(owner, from, from_path, children, loops);
                }
                Node::Use {
                    id: Some(id),
                    args,
                    component,
                    ..
                } => {
                    for (name, v) in args {
                        if let Some(lit) = literal_url(v)
                            && (lit.starts_with('/')
                                || lit.starts_with("http://")
                                || lit.starts_with("https://"))
                            && !lit.contains(char::is_whitespace)
                        {
                            let at = Target {
                                owner: owner.clone(),
                                node: id.clone(),
                            };
                            let text = self
                                .site
                                .components
                                .iter()
                                .find(|c| c.id == *component)
                                .and_then(|c| c.name.clone())
                                .unwrap_or_else(|| component.clone());
                            self.add(
                                from,
                                from_path,
                                &lit,
                                text,
                                LinkVia::Arg {
                                    at,
                                    name: name.clone(),
                                },
                            );
                        } else if let Some(var) = item_url_var(&format!("{{{{ {v} }}}}"))
                            && let Some((_, lp, list_at)) =
                                loops.iter().rev().find(|(x, _, _)| *x == var).cloned()
                        {
                            // Carte d'une liste (`url: p.url`) : lien vers la collection.
                            self.loop_link(from, &lp, list_at);
                        }
                    }
                }
                Node::Each {
                    id,
                    expr,
                    item,
                    children,
                    empty,
                } => {
                    let lp = if let Some(menu) = siteinfo::menu_of(expr) {
                        Loop::Menu(menu)
                    } else if let Some(q) = Query::parse(expr) {
                        Loop::Collection(std::iter::once(q.collection).chain(q.also).collect())
                    } else if let Some(FieldList::Refs { collection, .. }) = FieldList::parse(expr)
                    {
                        Loop::Collection(vec![collection])
                    } else {
                        Loop::Other
                    };
                    let var = item
                        .split(',')
                        .next_back()
                        .unwrap_or(item)
                        .trim()
                        .to_string();
                    let at = Target {
                        owner: owner.clone(),
                        node: id.clone().unwrap_or_default(),
                    };
                    loops.push((var, lp, at));
                    self.walk(owner, from, from_path, children, loops);
                    loops.pop();
                    self.walk(owner, from, from_path, empty, loops);
                }
                Node::If {
                    then, otherwise, ..
                } => {
                    self.walk(owner, from, from_path, then, loops);
                    self.walk(owner, from, from_path, otherwise, loops);
                }
                Node::SetBlock { children, .. } => {
                    self.walk(owner, from, from_path, children, loops)
                }
                _ => {}
            }
        }
    }

    fn loop_link(&mut self, from: &str, lp: &Loop, at: Target) {
        match lp {
            Loop::Collection(cols) => {
                for c in cols {
                    let key = format!("col:{c}");
                    if self.nodes.contains_key(&key)
                        && key != from
                        && !self.links.iter().any(|l| {
                            l.from == from
                                && l.to == key
                                && matches!(&l.via, LinkVia::List { at: a } if *a == at)
                        })
                    {
                        self.links.push(MapLink {
                            from: from.to_string(),
                            to: key,
                            href: String::new(),
                            text: String::new(),
                            via: LinkVia::List { at: at.clone() },
                            redirected: false,
                        });
                    }
                }
            }
            Loop::Menu(m) => {
                let key = format!("menu:{m}");
                if self.nodes.contains_key(&key)
                    && !self.links.iter().any(|l| l.from == from && l.to == key)
                {
                    self.links.push(MapLink {
                        from: from.to_string(),
                        to: key,
                        href: String::new(),
                        text: String::new(),
                        via: LinkVia::Menu { at },
                        redirected: false,
                    });
                }
            }
            Loop::Other => {}
        }
    }
}

/// Nom d'une page : son nom, sinon le dernier dossier de son adresse, sinon « / ».
pub fn page_label(p: &revel_model::Page) -> String {
    if let Some(n) = p.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        return n.to_string();
    }
    p.path
        .trim_matches('/')
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("/")
        .to_string()
}

/// Pages qui utilisent chaque gabarit et chaque composant (gabarits parents et composants
/// imbriqués compris).
fn reach(site: &Site) -> HashMap<String, usize> {
    fn uses(nodes: &[Node], out: &mut Vec<String>) {
        revel_model::ids::walk(nodes, &mut |n| {
            if let Node::Use { component, .. } = n {
                out.push(component.clone());
            }
        });
    }
    let mut count: HashMap<String, usize> = HashMap::new();
    let mut visit = |layout: Option<&str>, body: &[Node], weight: usize| {
        let mut seen: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = Vec::new();
        uses(body, &mut stack);
        let mut l = layout.map(str::to_string);
        while let Some(id) = l.take() {
            if !seen.insert(format!("layout:{id}")) {
                break;
            }
            if let Some(lay) = site.layouts.iter().find(|x| x.id == id) {
                uses(&lay.body, &mut stack);
                l = lay.parent.clone();
            }
        }
        while let Some(c) = stack.pop() {
            if seen.insert(format!("comp:{c}"))
                && let Some(comp) = site.components.iter().find(|x| x.id == c)
            {
                uses(&comp.body, &mut stack);
            }
        }
        for k in seen {
            *count.entry(k).or_default() += weight;
        }
    };
    for p in &site.pages {
        if !crate::textfile::is_server_file(&p.path) {
            visit(p.layout.as_deref(), &p.body, 1);
        }
    }
    for c in &site.collections {
        if c.path.is_some() {
            visit(c.layout.as_deref(), &[], 1);
        }
    }
    count
}

/// Plan du site. `info` : informations du site (menus) ; `entries` : contenus des collections
/// qui ont une page (pour relier les liens écrits vers l'un d'eux).
pub fn build(site: &Site, info: Option<&Value>, entries: &[EntryRef]) -> LinkMap {
    build_with(site, info, entries, &[])
}

/// Adresses de chaînes d'une valeur (pointeur JSON de chacune).
fn strings_of(v: &Value, pointer: &mut String, out: &mut Vec<(String, String)>) {
    match v {
        Value::String(s) => out.push((pointer.clone(), s.clone())),
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                let len = pointer.len();
                pointer.push_str(&format!("/{i}"));
                strings_of(x, pointer, out);
                pointer.truncate(len);
            }
        }
        Value::Object(m) => {
            for (k, x) in m {
                let len = pointer.len();
                pointer.push('/');
                pointer.push_str(&k.replace('~', "~0").replace('/', "~1"));
                strings_of(x, pointer, out);
                pointer.truncate(len);
            }
        }
        _ => {}
    }
}

/// Comme [`build`], avec les adresses d'autres sites écrites dans les contenus du CMS (`texts`)
/// et dans les informations du site (D-107).
pub fn build_with(
    site: &Site,
    info: Option<&Value>,
    entries: &[EntryRef],
    texts: &[EntryText],
) -> LinkMap {
    let host = site
        .base_url
        .as_deref()
        .and_then(split_absolute)
        .map(|(h, _)| h);
    let mut b = Builder {
        site,
        pages: Vec::new(),
        templates: Vec::new(),
        entries: HashMap::new(),
        redirects: site
            .redirects
            .iter()
            .map(|r| (normalize(&r.from), r.to.clone()))
            .collect(),
        host,
        locales: site.locales.iter().map(|l| l.code.clone()).collect(),
        nodes: BTreeMap::new(),
        links: Vec::new(),
    };
    for p in &site.pages {
        if crate::textfile::is_server_file(&p.path) || p.path.ends_with(".xml") {
            continue;
        }
        let key = page_key(&p.id);
        if p.path.contains("{{") {
            b.templates.push((key.clone(), p.path.clone()));
        } else {
            b.pages.push((p.id.clone(), normalize(&p.path)));
        }
        b.nodes.insert(
            key.clone(),
            MapNode {
                key,
                kind: NodeKind::Page,
                label: page_label(p),
                path: Some(p.path.clone()),
                draft: p.draft,
                parent: None,
                count: None,
            },
        );
    }
    // Pages à modèle d'adresse : reliées comme une collection (clé de page gardée).
    let page_templates: Vec<(String, String)> = std::mem::take(&mut b.templates);
    for c in &site.collections {
        let Some(path) = &c.path else { continue };
        let key = format!("col:{}", c.id);
        let n = entries.iter().filter(|e| e.collection == c.id).count();
        b.nodes.insert(
            key.clone(),
            MapNode {
                key: key.clone(),
                kind: NodeKind::Collection,
                label: c.id.clone(),
                path: Some(path.clone()),
                draft: false,
                parent: None,
                count: Some(n),
            },
        );
        if n == 0 {
            b.templates.push((key.clone(), path.clone()));
        }
        for e in entries.iter().filter(|e| e.collection == c.id) {
            let ek = format!("entry:{}/{}", c.id, e.file);
            b.entries.insert(normalize(&e.url), ek.clone());
            b.nodes.insert(
                ek.clone(),
                MapNode {
                    key: ek,
                    kind: NodeKind::Entry,
                    label: if e.title.trim().is_empty() {
                        e.file.clone()
                    } else {
                        e.title.clone()
                    },
                    path: Some(e.url.clone()),
                    draft: false,
                    parent: Some(key.clone()),
                    count: None,
                },
            );
        }
    }
    let menus = info.map(siteinfo::menus).unwrap_or_default();
    for (name, links) in &menus {
        let key = format!("menu:{name}");
        // Nom donné au menu (Paramètres › Navigation), sinon sa clé.
        let label = site
            .settings
            .site_labels
            .get(&format!("{}.{name}", siteinfo::MENUS))
            .cloned()
            .unwrap_or_else(|| {
                let mut c = name.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            });
        b.nodes.insert(
            key.clone(),
            MapNode {
                key,
                kind: NodeKind::Menu,
                label,
                path: None,
                draft: false,
                parent: None,
                count: Some(links.len()),
            },
        );
    }
    let reached = reach(site);
    // Gabarit d'une seule collection : ses liens sont ceux de la page de chaque contenu.
    let layout_source = |id: &str| -> String {
        let users: Vec<&str> = site
            .collections
            .iter()
            .filter(|c| c.path.is_some() && c.layout.as_deref() == Some(id))
            .map(|c| c.id.as_str())
            .collect();
        let pages = site.pages.iter().any(|p| p.layout.as_deref() == Some(id));
        let parent = site.layouts.iter().any(|l| l.parent.as_deref() == Some(id));
        match users.as_slice() {
            [one] if !pages && !parent => format!("col:{one}"),
            _ => format!("layout:{id}"),
        }
    };
    let mut loops = Vec::new();
    for p in &site.pages {
        let key = page_key(&p.id);
        if !b.nodes.contains_key(&key) {
            continue;
        }
        let path = (!p.path.contains("{{")).then(|| normalize(&p.path));
        b.walk(
            &Owner::Page(p.id.clone()),
            &key,
            path.as_deref(),
            &p.body,
            &mut loops,
        );
    }
    let mut shared: Vec<(String, NodeKind, String, usize)> = Vec::new();
    for l in &site.layouts {
        let from = layout_source(&l.id);
        let before = b.links.len();
        b.walk(
            &Owner::Layout(l.id.clone()),
            &from,
            None,
            &l.body,
            &mut loops,
        );
        if b.links.len() > before && from.starts_with("layout:") {
            let n = reached.get(&from).copied().unwrap_or(0);
            shared.push((from, NodeKind::Layout, l.id.clone(), n));
        }
    }
    for c in &site.components {
        let from = format!("comp:{}", c.id);
        let before = b.links.len();
        b.walk(
            &Owner::Component(c.id.clone()),
            &from,
            None,
            &c.body,
            &mut loops,
        );
        if b.links.len() > before {
            let n = reached.get(&from).copied().unwrap_or(0);
            let label = c.name.clone().unwrap_or_else(|| c.id.clone());
            shared.push((from, NodeKind::Component, label, n));
        }
    }
    for (key, kind, label, n) in shared {
        b.nodes.insert(
            key.clone(),
            MapNode {
                key,
                kind,
                label,
                path: None,
                draft: false,
                parent: None,
                count: Some(n),
            },
        );
    }
    // Pages à modèle d'adresse : reconnues après les collections.
    b.templates.extend(page_templates);
    for (name, links) in &menus {
        let from = format!("menu:{name}");
        for (index, l) in links.iter().enumerate() {
            b.add(
                &from,
                None,
                &l.url,
                l.label.clone(),
                LinkVia::MenuItem {
                    menu: name.clone(),
                    index,
                },
            );
        }
    }
    let redirects = site.redirects.clone();
    for (index, r) in redirects.iter().enumerate() {
        // Redirection : de l'ancienne adresse (introuvable) vers la nouvelle.
        let from = format!("missing:{}", normalize(&r.from));
        b.nodes.entry(from.clone()).or_insert(MapNode {
            key: from.clone(),
            kind: NodeKind::Missing,
            label: normalize(&r.from),
            path: Some(normalize(&r.from)),
            draft: false,
            parent: None,
            count: None,
        });
        let saved = b.redirects.remove(&normalize(&r.from));
        b.add(
            &from,
            None,
            &r.to,
            String::new(),
            LinkVia::Redirect { index },
        );
        if let Some(s) = saved {
            b.redirects.insert(normalize(&r.from), s);
        }
    }
    // Adresses d'autres sites écrites dans les contenus du CMS et les informations du site.
    let host = b.host.clone();
    let external =
        |u: &str| split_absolute(u).is_some_and(|(h, _)| host.as_deref() != Some(h.as_str()));
    for t in texts {
        let entry = format!("entry:{}/{}", t.collection, t.file);
        let from = if b.nodes.contains_key(&entry) {
            entry
        } else {
            format!("cms:{}/{}", t.folder, t.file)
        };
        let mut strings = Vec::new();
        strings_of(&t.data, &mut String::new(), &mut strings);
        for (pointer, text) in strings {
            for u in urls_in(&text).into_iter().filter(|u| external(u)) {
                b.add(
                    &from,
                    None,
                    &u,
                    t.title.clone(),
                    LinkVia::Field {
                        folder: t.folder.clone(),
                        file: t.file.clone(),
                        pointer: pointer.clone(),
                    },
                );
            }
        }
        for u in urls_in(&t.body).into_iter().filter(|u| external(u)) {
            b.add(
                &from,
                None,
                &u,
                t.title.clone(),
                LinkVia::Body {
                    folder: t.folder.clone(),
                    file: t.file.clone(),
                },
            );
        }
    }
    if let Some(info) = info {
        for v in siteinfo::values(info) {
            for u in urls_in(&v.value).into_iter().filter(|u| external(u)) {
                b.add(
                    "info:site",
                    None,
                    &u,
                    v.path.join(" › "),
                    LinkVia::Info {
                        path: v.path.clone(),
                    },
                );
            }
        }
    }
    LinkMap {
        nodes: b.nodes.into_values().collect(),
        links: b.links,
    }
}

/// Adresse écrite pour aller vers `node` (page, contenu) ; `None` pour une collection ou un menu.
pub fn address_of(map: &LinkMap, node: &str) -> Option<String> {
    let n = map.node(node)?;
    match n.kind {
        NodeKind::Page | NodeKind::Entry | NodeKind::Missing => {
            n.path.clone().filter(|p| !p.contains("{{"))
        }
        _ => None,
    }
}

/// Commande qui change la destination des liens `links` vers l'adresse `to` (requête et ancre
/// gardées). Les liens d'une liste ne changent pas.
pub fn retarget(
    site: &Site,
    info: Option<&Value>,
    links: &[&MapLink],
    to: &str,
) -> Option<Command> {
    let mut commands = Vec::new();
    let mut menus: BTreeMap<String, Vec<MenuLink>> = BTreeMap::new();
    let mut redirects = site.redirects.clone();
    let mut redirects_changed = false;
    for l in links {
        let new = format!("{to}{}", suffix(&l.href));
        match &l.via {
            LinkVia::Attr { at } => {
                let old = crate::address::node(site, at).ok().and_then(|n| match n {
                    Node::Element { attrs, .. } => attrs
                        .iter()
                        .find(|a| a.name == "href")
                        .and_then(|a| a.value.clone()),
                    _ => None,
                });
                // Forme `{{ '/x/' | url }}` gardée.
                let value = match old.as_deref().map(str::trim) {
                    Some(o) if o.starts_with("{{") => format!("{{{{ '{new}' | url }}}}"),
                    _ => new,
                };
                commands.push(Command::SetAttr {
                    at: at.clone(),
                    name: "href".into(),
                    value: Some(value),
                });
            }
            LinkVia::Arg { at, name } => commands.push(Command::SetUseArg {
                at: at.clone(),
                name: name.clone(),
                value: Some(format!("\"{new}\"")),
            }),
            LinkVia::MenuItem { menu, index } => {
                let list = menus.entry(menu.clone()).or_insert_with(|| {
                    info.map(siteinfo::menus)
                        .unwrap_or_default()
                        .into_iter()
                        .find(|(n, _)| n == menu)
                        .map(|(_, l)| l)
                        .unwrap_or_default()
                });
                if let Some(item) = list.get_mut(*index) {
                    item.url = new;
                }
            }
            LinkVia::Redirect { index } => {
                if let Some(r) = redirects.get_mut(*index) {
                    r.to = new;
                    redirects_changed = true;
                }
            }
            LinkVia::List { .. }
            | LinkVia::Menu { .. }
            | LinkVia::Src { .. }
            | LinkVia::Field { .. }
            | LinkVia::Body { .. }
            | LinkVia::Info { .. } => {}
        }
    }
    for (menu, list) in menus {
        commands.push(siteinfo::set_value(
            site,
            &[siteinfo::MENUS.to_string(), menu],
            Some(siteinfo::menu_value(&list)),
        ));
    }
    if redirects_changed {
        commands.push(Command::SetRedirects { redirects });
    }
    match commands.len() {
        0 => None,
        1 => commands.pop(),
        _ => Some(Command::Batch { commands }),
    }
}

/// Valeur d'un attribut d'un élément.
fn attr_value(site: &Site, at: &Target, name: &str) -> Option<String> {
    match crate::address::node(site, at).ok()? {
        Node::Element { attrs, .. } => attrs
            .iter()
            .find(|a| a.name == name)
            .and_then(|a| a.value.clone()),
        _ => None,
    }
}

/// Caractère qui peut continuer une adresse.
fn url_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-._~/?#[]@!$&*+,;=%:".contains(c)
}

/// Texte où l'adresse `old` est remplacée par `new` là où elle est écrite entière (pas au début
/// d'une adresse plus longue ; ponctuation d'une phrase après elle admise).
pub fn replace_link_text(text: &str, old: &str, new: &str) -> String {
    if old.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(old) {
        let before = if i > 0 {
            rest[..i].chars().next_back()
        } else {
            out.chars().next_back()
        };
        let tail = &rest[i + old.len()..];
        let mut it = tail.chars();
        let after = it.next();
        let ends = |c: Option<char>| c.is_none_or(|c| !url_char(c));
        let whole = before.is_none_or(|c| !c.is_ascii_alphanumeric())
            && (ends(after) || after.is_some_and(|c| ".,;:!?".contains(c)) && ends(it.next()));
        out.push_str(&rest[..i]);
        out.push_str(if whole { new } else { old });
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// Commande qui remplace l'adresse `old` par `new` partout où les liens `links` l'écrivent (liens,
/// médias intégrés, valeurs de composant, menus, redirections, champs et texte des contenus du CMS,
/// informations du site ; D-107). `texts` : contenus du CMS lus pour le plan.
pub fn replace_url(
    site: &Site,
    info: Option<&Value>,
    texts: &[EntryText],
    links: &[&MapLink],
    old: &str,
    new: &str,
) -> Option<Command> {
    if old.is_empty() || old == new {
        return None;
    }
    let swap = |s: &str| replace_link_text(s, old, new);
    let mut commands = Vec::new();
    let mut menus: BTreeMap<String, Vec<MenuLink>> = BTreeMap::new();
    let mut redirects = site.redirects.clone();
    let mut redirects_changed = false;
    // Contenus du CMS où l'adresse est écrite (champs, texte) : remplacée dans leur fichier.
    let mut entries: BTreeSet<(String, String)> = BTreeSet::new();
    let mut infos: BTreeMap<Vec<String>, String> = BTreeMap::new();
    let mut seen: HashSet<String> = HashSet::new();
    for l in links {
        match &l.via {
            LinkVia::Attr { at } | LinkVia::Src { at, .. } => {
                let name = match &l.via {
                    LinkVia::Src { attr, .. } => attr.as_str(),
                    _ => "href",
                };
                if !seen.insert(format!("{at:?}#{name}")) {
                    continue;
                }
                if let Some(v) = attr_value(site, at, name)
                    && v.contains(old)
                {
                    commands.push(Command::SetAttr {
                        at: at.clone(),
                        name: name.into(),
                        value: Some(swap(&v)),
                    });
                }
            }
            LinkVia::Arg { at, name } => {
                if !seen.insert(format!("{at:?}@{name}")) {
                    continue;
                }
                let v = match crate::address::node(site, at).ok() {
                    Some(Node::Use { args, .. }) => args.get(name).cloned(),
                    _ => None,
                };
                if let Some(v) = v
                    && v.contains(old)
                {
                    commands.push(Command::SetUseArg {
                        at: at.clone(),
                        name: name.clone(),
                        value: Some(swap(&v)),
                    });
                }
            }
            LinkVia::MenuItem { menu, index } => {
                let list = menus.entry(menu.clone()).or_insert_with(|| {
                    info.map(siteinfo::menus)
                        .unwrap_or_default()
                        .into_iter()
                        .find(|(n, _)| n == menu)
                        .map(|(_, l)| l)
                        .unwrap_or_default()
                });
                if let Some(item) = list.get_mut(*index) {
                    item.url = swap(&item.url);
                }
            }
            LinkVia::Redirect { index } => {
                if let Some(r) = redirects.get_mut(*index)
                    && r.to.contains(old)
                {
                    r.to = swap(&r.to);
                    redirects_changed = true;
                }
            }
            LinkVia::Field { folder, file, .. } | LinkVia::Body { folder, file } => {
                entries.insert((folder.clone(), file.clone()));
            }
            LinkVia::Info { path } => {
                let current = info
                    .map(siteinfo::values)
                    .unwrap_or_default()
                    .into_iter()
                    .find(|v| v.path == *path)
                    .map(|v| v.value);
                if let Some(v) = current {
                    infos.insert(path.clone(), swap(&v));
                }
            }
            LinkVia::List { .. } | LinkVia::Menu { .. } => {}
        }
    }
    for (menu, list) in menus {
        commands.push(siteinfo::set_value(
            site,
            &[siteinfo::MENUS.to_string(), menu],
            Some(siteinfo::menu_value(&list)),
        ));
    }
    if redirects_changed {
        commands.push(Command::SetRedirects { redirects });
    }
    for (folder, file) in entries {
        if texts.iter().any(|t| t.folder == folder && t.file == file) {
            commands.push(Command::ReplaceInEntry {
                folder,
                file,
                find: old.to_string(),
                replace: new.to_string(),
            });
        }
    }
    for (path, value) in infos {
        commands.push(siteinfo::set_value(site, &path, Some(Value::String(value))));
    }
    match commands.len() {
        0 => None,
        1 => commands.pop(),
        _ => Some(Command::Batch { commands }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use revel_model::{Attr, Page};
    use serde_json::json;

    fn a(id: &str, href: &str, text: &str) -> Node {
        Node::Element {
            id: Some(id.into()),
            tag: "a".into(),
            attrs: vec![Attr {
                name: "href".into(),
                value: Some(href.into()),
                when: None,
            }],
            children: vec![Node::Text { text: text.into() }],
        }
    }

    fn page(id: &str, path: &str, body: Vec<Node>) -> Page {
        Page {
            id: id.into(),
            path: path.into(),
            layout: None,
            props: Default::default(),
            computed: Default::default(),
            body,
            draft: false,
            name: None,
            code: Default::default(),
        }
    }

    fn site() -> Site {
        let mut s = Site::new("Essai", "fr");
        s.base_url = Some("https://www.exemple.fr".into());
        s.pages = vec![
            page(
                "index",
                "/",
                vec![
                    a("l1", "/contact", "Écrire"),
                    a("l2", "https://exemple.fr/agence/#equipe", "Agence"),
                    a("l3", "{{ '/project/alpha/' | url }}", "Alpha"),
                    a("l4", "/ancienne/", "Ancienne"),
                    a("l5", "/nulle-part/", "Perdu"),
                    a("l6", "https://autre.org/x", "Ailleurs"),
                    a("l7", "mailto:a@b.c", "Courriel"),
                    a("l8", "/media/plaquette.pdf", "PDF"),
                    Node::Each {
                        id: Some("e1".into()),
                        expr: "collections.projets".into(),
                        item: "p".into(),
                        children: vec![a("l9", "{{ p.url }}", "{{ p.data.title }}")],
                        empty: vec![],
                    },
                ],
            ),
            page(
                "liste",
                "/liste/",
                vec![Node::Each {
                    id: Some("e2".into()),
                    expr: "collections.projets".into(),
                    item: "p".into(),
                    children: vec![Node::Use {
                        id: Some("u1".into()),
                        component: "carte".into(),
                        args: [("url".to_string(), "p.url".to_string())].into(),
                        variant: None,
                        attrs: vec![],
                        children: vec![],
                    }],
                    empty: vec![],
                }],
            ),
            page(
                "contact",
                "/contact/",
                vec![a("c1", "../agence/", "Agence")],
            ),
            page("agence", "/agence/", vec![]),
            page("seule", "/seule/", vec![]),
        ];
        s.collections = vec![revel_model::Collection {
            id: "projets".into(),
            folder: "content/projets".into(),
            path: Some("/project/{{ slug }}/".into()),
            layout: Some("projet".into()),
            source: Default::default(),
            props: Default::default(),
            filter: None,
            sort: None,
            fields: vec![],
        }];
        s.layouts = vec![revel_model::Layout {
            id: "projet".into(),
            parent: None,
            props: Default::default(),
            fields: vec![],
            body: vec![a("y1", "/projets-tous/", "Tous")],
        }];
        s.redirects = vec![revel_model::Redirect {
            from: "/ancienne".into(),
            to: "/agence/".into(),
        }];
        s
    }

    #[test]
    fn liens_relies() {
        let s = site();
        let info = json!({ "menus": { "principal": [
            { "label": "Contact", "url": "/contact/" },
            { "label": "Agence", "url": "/agence/" }
        ] } });
        let entries = vec![EntryRef {
            collection: "projets".into(),
            file: "alpha".into(),
            url: "/project/alpha/".into(),
            title: "Alpha".into(),
        }];
        let m = build(&s, Some(&info), &entries);
        let to = |id: &str| {
            m.links
                .iter()
                .find(|l| matches!(&l.via, LinkVia::Attr { at } if at.node == id))
                .map(|l| (l.to.clone(), l.redirected))
        };
        assert_eq!(to("l1"), Some(("page:contact".into(), false)));
        // Même site (www, ancre) ; relative depuis /contact/.
        assert_eq!(to("l2"), Some(("page:agence".into(), false)));
        assert_eq!(to("c1"), Some(("page:agence".into(), false)));
        assert_eq!(to("l3"), Some(("entry:projets/alpha".into(), false)));
        assert_eq!(to("l4"), Some(("page:agence".into(), true)));
        assert_eq!(to("l5"), Some(("missing:/nulle-part/".into(), false)));
        assert_eq!(to("l6"), Some(("ext:autre.org".into(), false)));
        assert_eq!(to("l7"), None);
        assert_eq!(to("l8"), None);
        // Liste de projets : vers la collection.
        assert!(m.links.iter().any(|l| l.from == "page:index"
            && l.to == "col:projets"
            && matches!(l.via, LinkVia::List { .. })));
        // Carte d'une liste (valeur d'un composant) : vers la collection.
        assert!(
            m.links
                .iter()
                .any(|l| l.from == "page:liste" && l.to == "col:projets")
        );
        // Gabarit d'une seule collection : liens du modèle de la page d'un projet.
        assert!(
            m.links
                .iter()
                .any(|l| l.from == "col:projets" && l.to == "missing:/projets-tous/")
        );
        // Menu ; redirection.
        assert_eq!(
            m.links
                .iter()
                .filter(|l| l.from == "menu:principal")
                .map(|l| l.to.as_str())
                .collect::<Vec<_>>(),
            vec!["page:contact", "page:agence"]
        );
        assert!(m.links.iter().any(|l| l.from == "missing:/ancienne/"
            && l.to == "page:agence"
            && matches!(l.via, LinkVia::Redirect { index: 0 })));
        // Sans lien vers elle.
        assert_eq!(
            m.orphans(),
            BTreeSet::from(["page:seule".to_string(), "page:liste".to_string()])
        );
        // Sans les contenus : la collection reçoit le lien.
        let m2 = build(&s, None, &[]);
        assert!(
            m2.links.iter().any(|l| l.to == "col:projets"
                && matches!(&l.via, LinkVia::Attr { at } if at.node == "l3"))
        );
    }

    #[test]
    fn destination_changee() {
        let mut s = site();
        let info =
            json!({ "menus": { "principal": [{ "label": "Contact", "url": "/contact/" }] } });
        s.data.insert("site".into(), info.clone());
        let m = build(&s, Some(&info), &[]);
        let to_contact: Vec<&MapLink> = m.links.iter().filter(|l| l.to == "page:contact").collect();
        assert_eq!(to_contact.len(), 2);
        let cmd = retarget(&s, Some(&info), &to_contact, "/agence/").unwrap();
        crate::command::apply(&mut s, &cmd).unwrap();
        let info2 = s.data.get("site").cloned();
        let m = build(&s, info2.as_ref(), &[]);
        assert!(!m.links.iter().any(|l| l.to == "page:contact"));
        // Forme `| url` gardée ; ancre gardée.
        let l2: Vec<&MapLink> = m
            .links
            .iter()
            .filter(
                |l| matches!(&l.via, LinkVia::Attr { at } if at.node == "l2" || at.node == "l3"),
            )
            .collect();
        let cmd = retarget(&s, None, &l2, "/contact/").unwrap();
        crate::command::apply(&mut s, &cmd).unwrap();
        let href = |id: &str| {
            let at = Target {
                owner: Owner::Page("index".into()),
                node: id.into(),
            };
            match crate::address::node(&s, &at).unwrap() {
                Node::Element { attrs, .. } => attrs[0].value.clone().unwrap(),
                _ => unreachable!(),
            }
        };
        assert_eq!(href("l2"), "/contact/#equipe");
        assert_eq!(href("l3"), "{{ '/contact/' | url }}");
    }

    #[test]
    fn modeles_d_adresse() {
        assert!(matches_template("/project/{{ slug }}/", "/project/alpha/"));
        assert!(!matches_template("/project/{{ slug }}/", "/project/a/b/"));
        assert!(!matches_template("/project/{{ slug }}/", "/project//"));
        assert!(matches_template("/{{ slug }}/", "/actu-1/"));
        assert!(matches_template("/{{ y }}/{{ slug }}/", "/2026/titre/"));
        assert_eq!(join("/a/b/", "../c/"), "/a/c/");
        assert_eq!(join("/a/b", "c"), "/a/c");
        assert_eq!(normalize("/x/index.html?y#z"), "/x/");
        assert_eq!(item_url_var("{{ p.url | url }}").as_deref(), Some("p"));
        assert_eq!(item_url_var("{{ p.data.url }}").as_deref(), Some("p"));
        assert_eq!(static_href("{{ \"/a/\" | url }}").as_deref(), Some("/a/"));
    }

    #[test]
    fn plateformes_reconnues() {
        assert_eq!(platform_of("www.youtube.com").map(|p| p.0), Some("youtube"));
        assert_eq!(platform_of("player.vimeo.com").map(|p| p.0), Some("vimeo"));
        assert_eq!(platform_of("m.facebook.com").map(|p| p.0), Some("facebook"));
        assert_eq!(platform_of("notyoutube.com"), None);
        assert_eq!(platform("media:linkedin").map(|p| p.1), Some("LinkedIn"));
        assert_eq!(
            urls_in(
                "<p><a href=\"https://www.instagram.com/sherpa/\">x</a> https://youtu.be/abc.</p>\\"
            ),
            vec!["https://www.instagram.com/sherpa/", "https://youtu.be/abc"]
        );
        assert!(urls_in("https://player.vimeo.com/video/{{ vid }}").is_empty());
    }

    fn el(id: &str, tag: &str, attrs: &[(&str, &str)]) -> Node {
        Node::Element {
            id: Some(id.into()),
            tag: tag.into(),
            attrs: attrs
                .iter()
                .map(|(n, v)| Attr {
                    name: (*n).into(),
                    value: Some((*v).into()),
                    when: None,
                })
                .collect(),
            children: vec![],
        }
    }

    fn medias() -> (Site, Value, Vec<EntryText>) {
        let mut s = Site::new("Essai", "fr");
        s.base_url = Some("https://exemple.fr".into());
        s.pages = vec![page(
            "index",
            "/",
            vec![
                el(
                    "v1",
                    "iframe",
                    &[
                        ("src", "https://www.youtube-nocookie.com/embed/abc"),
                        ("title", "Film"),
                    ],
                ),
                el(
                    "v2",
                    "iframe",
                    &[("src", "https://www.google.com/maps/embed?pb=1")],
                ),
                el(
                    "v3",
                    "iframe",
                    &[("src", "https://player.vimeo.com/video/{{ vid | vimeo_id }}")],
                ),
                a("l1", "https://www.instagram.com/agence/", "Instagram"),
                a("l2", "https://exemple.fr/contact/", "Contact"),
            ],
        )];
        let info = json!({ "social": { "linkedin": "https://www.linkedin.com/company/agence" } });
        let texts = vec![EntryText {
            collection: "projets".into(),
            folder: "src/content/projets".into(),
            file: "force".into(),
            title: "Force".into(),
            data: json!({
                "title": "Force",
                "home_video": "https://player.vimeo.com/progressive_redirect/playback/1/file.mp4?s=1",
                "blocks": [{ "type": "text" }, { "type": "video", "video": "https://www.youtube.com/watch?v=abc" }]
            }),
            body: "Voir https://www.instagram.com/agence/ et https://exemple.fr/agence/.".into(),
        }];
        (s, info, texts)
    }

    #[test]
    fn medias_et_reseaux() {
        let (s, info, texts) = medias();
        let m = build_with(&s, Some(&info), &[], &texts);
        let kind = |k: &str| m.node(k).map(|n| n.kind);
        assert_eq!(kind("media:youtube"), Some(NodeKind::Media));
        assert_eq!(kind("media:vimeo"), Some(NodeKind::Media));
        assert_eq!(kind("media:instagram"), Some(NodeKind::Media));
        assert_eq!(kind("media:linkedin"), Some(NodeKind::Media));
        assert_eq!(kind("ext:google.com"), Some(NodeKind::External));
        assert_eq!(
            m.node("media:youtube").map(|n| n.label.as_str()),
            Some("YouTube")
        );
        let to = |k: &str| m.links.iter().filter(|l| l.to == k).collect::<Vec<_>>();
        // Iframe de la page, champ imbriqué du contenu.
        let yt = to("media:youtube");
        assert!(yt.iter().any(
            |l| matches!(&l.via, LinkVia::Src { attr, .. } if attr == "src") && l.text == "Film"
        ));
        assert!(yt.iter().any(
            |l| matches!(&l.via, LinkVia::Field { pointer, .. } if pointer == "/blocks/1/video")
        ));
        // Lien de la page et texte du contenu (sans page : source `cms:`).
        let ig = to("media:instagram");
        assert!(ig.iter().any(|l| l.from == "page:index"));
        assert!(
            ig.iter().any(|l| l.from == "cms:src/content/projets/force"
                && matches!(l.via, LinkVia::Body { .. }))
        );
        // Information du site.
        assert!(
            to("media:linkedin")
                .iter()
                .any(|l| l.from == "info:site" && l.text == "social › linkedin")
        );
        // Adresse calculée, adresse du site : ignorées.
        assert!(!m.links.iter().any(|l| l.href.contains("{{")));
        assert!(!m.links.iter().any(|l| l.href.contains("exemple.fr/agence")));
        // Contenu qui a une page : relié depuis son nœud.
        let e = vec![EntryRef {
            collection: "projets".into(),
            file: "force".into(),
            url: "/projets/force/".into(),
            title: "Force".into(),
        }];
        let mut s2 = s.clone();
        s2.collections.push(revel_model::Collection {
            id: "projets".into(),
            folder: "src/content/projets".into(),
            path: Some("/projets/{{ slug }}/".into()),
            layout: None,
            source: Default::default(),
            props: Default::default(),
            filter: None,
            sort: None,
            fields: vec![],
        });
        let m2 = build_with(&s2, Some(&info), &e, &texts);
        assert!(
            m2.links
                .iter()
                .any(|l| l.to == "media:vimeo" && l.from == "entry:projets/force")
        );
    }

    #[test]
    fn adresse_remplacee_partout() {
        let (s, info, texts) = medias();
        let m = build_with(&s, Some(&info), &[], &texts);
        let old = "https://www.instagram.com/agence/";
        let links: Vec<&MapLink> = m.links.iter().filter(|l| l.href == old).collect();
        assert_eq!(links.len(), 2);
        let Some(Command::Batch { commands }) = replace_url(
            &s,
            Some(&info),
            &texts,
            &links,
            old,
            "https://www.instagram.com/agence.officielle/",
        ) else {
            panic!("suite attendue")
        };
        assert!(commands.iter().any(|c| matches!(c, Command::SetAttr { name, value: Some(v), .. } if name == "href" && v == "https://www.instagram.com/agence.officielle/")));
        assert!(commands.iter().any(|c| matches!(c, Command::ReplaceInEntry { file, replace, .. } if file == "force" && replace.ends_with("agence.officielle/"))));
        // Champ imbriqué : remplacée dans le fichier du contenu (mise en forme gardée).
        let yt = "https://www.youtube.com/watch?v=abc";
        let links: Vec<&MapLink> = m.links.iter().filter(|l| l.href == yt).collect();
        assert!(matches!(
            replace_url(&s, Some(&info), &texts, &links, yt, "https://youtu.be/xyz"),
            Some(Command::ReplaceInEntry { find, .. }) if find == yt
        ));
        // Adresse entière seulement.
        assert_eq!(
            replace_link_text(
                "a https://x.fr/a. b https://x.fr/ab \"https://x.fr/a\"",
                "https://x.fr/a",
                "https://y.fr/"
            ),
            "a https://y.fr/. b https://x.fr/ab \"https://y.fr/\""
        );
        // Information du site, média intégré.
        let li = "https://www.linkedin.com/company/agence";
        let links: Vec<&MapLink> = m.links.iter().filter(|l| l.href == li).collect();
        assert!(matches!(
            replace_url(
                &s,
                Some(&info),
                &texts,
                &links,
                li,
                "https://www.linkedin.com/company/agence-2"
            ),
            Some(Command::SetGlobal { .. } | Command::SetDataValue { .. })
        ));
        let src = "https://www.youtube-nocookie.com/embed/abc";
        let links: Vec<&MapLink> = m.links.iter().filter(|l| l.href == src).collect();
        assert!(matches!(
            replace_url(&s, Some(&info), &texts, &links, src, "https://www.youtube-nocookie.com/embed/xyz"),
            Some(Command::SetAttr { name, .. }) if name == "src"
        ));
        assert!(replace_url(&s, Some(&info), &texts, &links, src, src).is_none());
    }
}
