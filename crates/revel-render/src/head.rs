//! En-tête des pages écrit à la main (titre, description, partage, adresse, type, robots,
//! icônes, couleur du navigateur) : remplacé par les réglages du site et des pages, d'après
//! lesquels la génération l'écrit (D-92). Chaque groupe de balises n'est remplacé que si le site
//! produit reste le même (en-tête comparé balise par balise, sans leur ordre) ; sinon il reste
//! tel quel.

use crate::BuildInput;
use revel_model::{Node, Site};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

/// Empreinte du site produit : fichier -> contenu (en-tête des pages HTML mis en ordre).
type Prints = BTreeMap<String, Print>;

const TITLE: &[&str] = &[
    "title",
    "meta:og:title",
    "meta:twitter:title",
    "meta:og:site_name",
];
const DESCRIPTION: &[&str] = &[
    "meta:description",
    "meta:og:description",
    "meta:twitter:description",
];
const IMAGE: &[&str] = &["meta:og:image", "meta:twitter:image", "meta:twitter:card"];
const ADDRESS: &[&str] = &["link:canonical", "meta:og:url"];
const KIND: &[&str] = &["meta:og:type"];
const DATE: &[&str] = &["meta:article:published_time"];
const ROBOTS: &[&str] = &["meta:robots"];
const LOCALE: &[&str] = &["meta:og:locale"];
const BROWSER: &[&str] = &["meta:theme-color", "link:icon", "link:apple-touch-icon"];
const FEEDS: &[&str] = &["link:alternate-feed"];

static TAG_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?is)<!--.*?-->|<title\b[^>]*>.*?</title\s*>|<script\b[^>]*>.*?</script\s*>|<style\b[^>]*>.*?</style\s*>|<noscript\b[^>]*>.*?</noscript\s*>|<[a-z][^>]*>",
    )
    .expect("regex")
});
static ATTR_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"([^\s=/>"']+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>"']+)))?"#)
        .expect("regex")
});
static LITERAL_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#""((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)'"#).expect("regex")
});

/// Remplace l'en-tête écrit à la main par les réglages, groupe par groupe.
pub(crate) fn head_settings(input: &BuildInput, site: &mut Site, notes: &mut Vec<String>) {
    let mut kinds_seen = BTreeSet::new();
    for l in &site.layouts {
        for n in head_children(&l.body) {
            kinds_seen.extend(kind_of_node(n));
        }
    }
    let page_code = site.pages.iter().any(|p| !p.code.head.trim().is_empty());
    if kinds_seen.is_empty() && !page_code {
        return;
    }
    let Some(base) = prints(input, site) else {
        return;
    };
    let mut done: Vec<&str> = Vec::new();
    let base_url = base_url(site);
    // Balises écrites dans le code ajouté de chaque page.
    if page_code && let Some(n) = page_tags(input, site, &base, &base_url) {
        notes.push(format!(
            "en-tête : balises de partage de {n} page(s) remplacées par leurs réglages (Paramètres de la page › Référencement)"
        ));
    }
    let groups: [(&str, &[&str]); 10] = [
        ("adresse canonique", ADDRESS),
        ("robots", ROBOTS),
        ("langue au partage", LOCALE),
        ("icônes et couleur du navigateur", BROWSER),
        ("flux RSS", FEEDS),
        ("description", DESCRIPTION),
        ("image de partage", IMAGE),
        ("type au partage", KIND),
        ("date de publication", DATE),
        ("titre", TITLE),
    ];
    for (label, kinds) in groups {
        if !kinds.iter().any(|k| kinds_seen.contains(*k)) {
            continue;
        }
        for v in variants(site, kinds, &base_url) {
            if prints(input, &v).is_some_and(|p| same_site(&base, &p)) {
                *site = v;
                done.push(label);
                break;
            }
        }
    }
    if !done.is_empty() {
        notes.push(format!(
            "en-tête des gabarits écrit d'après les réglages (Paramètres › Référencement) : {}",
            done.join(", ")
        ));
    }
}

/// Adresse du site sans `/` final (vide si inconnue).
fn base_url(site: &Site) -> String {
    site.base_url
        .clone()
        .or_else(|| {
            site.data
                .get("site")
                .and_then(|s| s.get("url"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default()
        .trim()
        .trim_end_matches('/')
        .to_string()
}

// ------------------------------------------------------------------------------------------------
// Empreinte du site produit

/// Fichier produit : contenu hors en-tête, et balises de l'en-tête (forme comparable, nature).
#[derive(Debug, Clone, PartialEq)]
struct Print {
    rest: String,
    head: Vec<(String, Option<String>)>,
}

/// Empreinte du site produit, dans chaque environnement de publication (une balise écrite
/// seulement en recette compte).
fn prints(input: &BuildInput, site: &Site) -> Option<Prints> {
    let files = crate::build_all_envs(input, site)?;
    Some(
        files
            .into_iter()
            .map(|f| {
                let text = String::from_utf8_lossy(&f.bytes).into_owned();
                let v = if f.path.ends_with(".html") || f.path.ends_with(".htm") {
                    print_of(&text)
                } else {
                    Print {
                        rest: text,
                        head: Vec::new(),
                    }
                };
                (f.path, v)
            })
            .collect(),
    )
}

/// Page HTML : en-tête ramené à la liste triée de ses balises (attributs triés, entités
/// décodées, blancs ignorés).
fn print_of(html: &str) -> Print {
    let whole = || Print {
        rest: html.to_string(),
        head: Vec::new(),
    };
    let lower = html.to_ascii_lowercase();
    let (Some(s), Some(e)) = (lower.find("<head"), lower.find("</head")) else {
        return whole();
    };
    let Some(open_end) = html[s..].find('>').map(|i| s + i + 1) else {
        return whole();
    };
    if open_end > e {
        return whole();
    }
    let mut head: Vec<(String, Option<String>)> = TAG_RE
        .find_iter(&html[open_end..e])
        .map(|m| m.as_str())
        .filter(|t| !t.starts_with("<!--"))
        .map(|t| {
            let (name, attrs) = parse_tag(t);
            (norm_tag(t), kind_of(&name, &attrs))
        })
        .collect();
    head.sort();
    Print {
        rest: format!("{}{}", &html[..open_end], &html[e..]),
        head,
    }
}

/// Même fichier : tout pareil, sauf des balises que la génération ajoute d'après les réglages
/// (D-92) là où la page n'en avait pas de cette nature.
fn same(a: &Print, b: &Print) -> bool {
    if a.rest != b.rest {
        return false;
    }
    let mut extra = b.head.clone();
    for t in &a.head {
        match extra.iter().position(|x| x.0 == t.0) {
            Some(i) => {
                extra.remove(i);
            }
            None => return false,
        }
    }
    let kinds: BTreeSet<&str> = a.head.iter().filter_map(|t| t.1.as_deref()).collect();
    extra.iter().all(|(_, k)| {
        k.as_deref()
            .is_some_and(|k| (automatic(k) || FEEDS.contains(&k)) && !kinds.contains(k))
    })
}

fn same_site(a: &Prints, b: &Prints) -> bool {
    a.len() == b.len() && a.iter().all(|(k, v)| b.get(k).is_some_and(|w| same(v, w)))
}

/// Balise ramenée à une forme comparable.
fn norm_tag(t: &str) -> String {
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("<title") {
        let inner = t
            .find('>')
            .and_then(|i| lower.rfind("</title").map(|j| &t[i + 1..j]))
            .unwrap_or("");
        return format!("title {}", unescape(inner.trim()));
    }
    if lower.starts_with("<script") || lower.starts_with("<style") || lower.starts_with("<noscript")
    {
        return t.trim().to_string();
    }
    let (name, attrs) = parse_tag(t);
    let mut list: Vec<String> = attrs
        .into_iter()
        .map(|(k, v)| format!("{k}={v:?}"))
        .collect();
    list.sort();
    format!("{name} {}", list.join(" "))
}

/// Nom et attributs (noms en minuscules, valeurs décodées) d'une balise ouvrante.
fn parse_tag(t: &str) -> (String, Vec<(String, String)>) {
    let inner = t
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim_end_matches('/');
    let name_end = inner
        .find(|c: char| c.is_whitespace())
        .unwrap_or(inner.len());
    let name = inner[..name_end].to_ascii_lowercase();
    let attrs = ATTR_RE
        .captures_iter(&inner[name_end..])
        .map(|c| {
            let v = c
                .get(2)
                .or(c.get(3))
                .or(c.get(4))
                .map(|m| unescape(m.as_str()))
                .unwrap_or_default();
            (c[1].to_ascii_lowercase(), v)
        })
        .collect();
    (name, attrs)
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

// ------------------------------------------------------------------------------------------------
// Balises de l'en-tête des gabarits

/// Nature d'une balise : `title`, `meta:<name ou property>`, `link:<rel>` (`alternate-feed`
/// pour un flux RSS ou Atom).
fn kind_of(tag: &str, attrs: &[(String, String)]) -> Option<String> {
    let get = |k: &str| {
        attrs
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.trim().to_ascii_lowercase())
            .filter(|v| !v.contains("{{") && !v.contains("{%"))
    };
    match tag {
        "title" => Some("title".into()),
        "meta" => get("name")
            .or_else(|| get("property"))
            .map(|k| format!("meta:{k}")),
        "link" => {
            let rel = get("rel")?;
            if rel == "alternate"
                && get("type").is_some_and(|t| t.contains("rss") || t.contains("atom"))
            {
                Some("link:alternate-feed".into())
            } else if rel == "shortcut icon" {
                Some("link:icon".into())
            } else {
                Some(format!("link:{rel}"))
            }
        }
        _ => None,
    }
}

/// Nature d'un nœud de l'en-tête : élément, ou condition qui n'affiche qu'un élément.
fn kind_of_node(n: &Node) -> Option<String> {
    match n {
        Node::Element { tag, attrs, .. } => {
            let list: Vec<(String, String)> = attrs
                .iter()
                .filter(|a| a.when.is_none())
                .map(|a| {
                    (
                        a.name.to_ascii_lowercase(),
                        a.value.clone().unwrap_or_default(),
                    )
                })
                .collect();
            kind_of(&tag.to_ascii_lowercase(), &list)
        }
        Node::If {
            then, otherwise, ..
        } if otherwise.is_empty() => {
            let mut inner = then.iter().filter(|c| !is_blank(c));
            let first = inner.next()?;
            if inner.next().is_some() {
                return None;
            }
            match first {
                Node::Element { .. } => kind_of_node(first),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_blank(n: &Node) -> bool {
    matches!(n, Node::Text { text } if text.trim().is_empty())
}

/// Enfants de l'élément `head` d'un gabarit.
fn head_children(body: &[Node]) -> &[Node] {
    fn find(nodes: &[Node]) -> Option<&[Node]> {
        for n in nodes {
            if let Node::Element { tag, children, .. } = n {
                if tag.eq_ignore_ascii_case("head") {
                    return Some(children);
                }
                if let Some(c) = find(children) {
                    return Some(c);
                }
            }
        }
        None
    }
    find(body).unwrap_or(&[])
}

fn head_children_mut(body: &mut [Node]) -> Option<&mut Vec<Node>> {
    for n in body {
        if let Node::Element { tag, children, .. } = n {
            if tag.eq_ignore_ascii_case("head") {
                return Some(children);
            }
            if let Some(c) = head_children_mut(children) {
                return Some(c);
            }
        }
    }
    None
}

/// Nœuds des natures `kinds` retirés de l'en-tête des gabarits (avec le blanc qui les
/// précède) ; rendus.
fn remove_kinds(site: &mut Site, kinds: &[&str]) -> Vec<Node> {
    let mut removed = Vec::new();
    for l in &mut site.layouts {
        let Some(children) = head_children_mut(&mut l.body) else {
            continue;
        };
        let mut out: Vec<Node> = Vec::with_capacity(children.len());
        for n in std::mem::take(children) {
            if kind_of_node(&n).is_some_and(|k| kinds.contains(&k.as_str())) {
                if out.last().is_some_and(is_blank) {
                    out.pop();
                }
                removed.push(n);
            } else {
                out.push(n);
            }
        }
        *children = out;
    }
    removed
}

/// Élément d'un nœud retiré (lui-même ou celui de sa condition) et condition éventuelle.
fn element_of(n: &Node) -> Option<(&Node, Option<&str>)> {
    match n {
        Node::Element { .. } => Some((n, None)),
        Node::If { cond, then, .. } => then
            .iter()
            .find(|c| matches!(c, Node::Element { .. }))
            .map(|e| (e, Some(cond.as_str()))),
        _ => None,
    }
}

/// Valeur d'un attribut d'un nœud retiré : écrite telle quelle, ou texte littéral de
/// l'expression (valeur de repli) qui satisfait `accept`.
fn value_of(n: &Node, attr: &str, accept: &dyn Fn(&str) -> bool) -> Option<String> {
    let (e, cond) = element_of(n)?;
    let Node::Element { attrs, .. } = e else {
        return None;
    };
    let v = attrs
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(attr))?
        .value
        .clone()
        .unwrap_or_default();
    if !v.contains("{{") && !v.contains("{%") {
        return accept(&v).then_some(v);
    }
    let mut lits: Vec<String> = Vec::new();
    for text in [Some(v.as_str()), cond].into_iter().flatten() {
        for c in LITERAL_RE.captures_iter(text) {
            let s = c.get(1).or(c.get(2)).map(|m| m.as_str()).unwrap_or("");
            lits.push(s.replace("\\\"", "\"").replace("\\'", "'"));
        }
    }
    lits.into_iter()
        .filter(|s| accept(s))
        .max_by_key(|s| s.chars().count())
}

fn kind_is(n: &Node, k: &str) -> bool {
    kind_of_node(n).as_deref() == Some(k)
}

fn is_image(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    (l.starts_with('/') || l.starts_with("http"))
        && [".jpg", ".jpeg", ".png", ".webp", ".gif", ".avif"]
            .iter()
            .any(|e| l.ends_with(e))
}

/// Adresse absolue du site ramenée à un chemin (`https://site/x.jpg` -> `/x.jpg`).
fn local(v: &str, base_url: &str) -> String {
    match v.strip_prefix(base_url) {
        Some(rest) if !base_url.is_empty() && rest.starts_with('/') => rest.to_string(),
        _ => v.to_string(),
    }
}

/// Façons de remplacer un groupe de balises, dans l'ordre de préférence.
fn variants(site: &Site, kinds: &[&str], base_url: &str) -> Vec<Site> {
    let mut plain = site.clone();
    let removed = remove_kinds(&mut plain, kinds);
    if removed.is_empty() {
        return Vec::new();
    }
    let seo = &site.settings.seo;
    let mut out = Vec::new();
    if kinds == TITLE {
        let name = removed
            .iter()
            .find(|n| kind_is(n, "meta:og:site_name"))
            .and_then(|n| value_of(n, "content", &|s| !s.trim().is_empty()));
        if let Some(name) = name.filter(|_| seo.site_title.is_none()) {
            let mut a = plain.clone();
            a.settings.seo.site_title = Some(name.clone());
            // Titres des pages écrits avec le nom du site à la fin : nom retiré (la génération
            // l'ajoute).
            let mut b = a.clone();
            let suffix = format!(" | {name}");
            for p in &mut b.pages {
                if let Some(Value::Object(s)) = p.props.get_mut("seo")
                    && let Some(Value::String(t)) = s.get_mut("title")
                    && let Some(short) = t.strip_suffix(&suffix)
                {
                    *t = short.to_string();
                }
            }
            out.push(a);
            out.push(b);
        }
        // Sans le nom du site : titres seulement.
        let mut c = site.clone();
        if !remove_kinds(&mut c, &["title", "meta:og:title", "meta:twitter:title"]).is_empty() {
            out.push(c);
        }
    } else if kinds == DESCRIPTION {
        let text = removed
            .iter()
            .find(|n| kind_is(n, "meta:description"))
            .and_then(|n| value_of(n, "content", &|s| s.trim().chars().count() > 20));
        if let Some(t) = text.filter(|_| seo.description.is_none()) {
            let mut a = plain.clone();
            a.settings.seo.description = Some(t);
            out.push(a);
        }
        out.push(plain);
    } else if kinds == IMAGE {
        let img = removed
            .iter()
            .find(|n| kind_is(n, "meta:og:image"))
            .and_then(|n| value_of(n, "content", &is_image));
        if let Some(i) = img.filter(|_| seo.share_image.is_none()) {
            let mut a = plain.clone();
            a.settings.seo.share_image = Some(local(&i, base_url));
            out.push(a);
        }
        out.push(plain);
    } else if kinds == KIND {
        // Type par défaut des gabarits : « website », comme le gabarit d'origine.
        let mut b = plain.clone();
        for l in &mut b.layouts {
            if l.parent.is_none() && !l.props.contains_key("ogType") {
                l.props
                    .insert("ogType".into(), Value::String("website".into()));
            }
        }
        out.push(plain);
        out.push(b);
    } else if kinds == BROWSER {
        let mut a = plain;
        let s = &mut a.settings.seo;
        for n in &removed {
            let Some((Node::Element { tag, attrs, .. }, None)) = element_of(n) else {
                continue;
            };
            let get = |k: &str| {
                attrs
                    .iter()
                    .find(|x| x.name.eq_ignore_ascii_case(k))
                    .and_then(|x| x.value.clone())
                    .filter(|v| !v.contains("{{"))
            };
            match (tag.as_str(), kind_of_node(n).as_deref()) {
                ("meta", Some("meta:theme-color")) if s.theme_color.is_none() => {
                    s.theme_color = get("content");
                }
                ("link", Some("link:apple-touch-icon")) if s.touch_icon.is_none() => {
                    s.touch_icon = get("href");
                }
                ("link", Some("link:icon")) => {
                    let href = get("href");
                    let png192 = get("sizes").is_some_and(|z| z == "192x192");
                    if png192 && s.favicon_png.is_none() {
                        s.favicon_png = href;
                    } else if s.favicon.is_none() {
                        s.favicon = href;
                    }
                }
                _ => {}
            }
        }
        out.push(a);
    } else {
        out.push(plain);
    }
    out
}

// ------------------------------------------------------------------------------------------------
// Balises écrites dans le code ajouté des pages

/// Natures que la génération écrit d'après les réglages de la page et du site.
fn automatic(kind: &str) -> bool {
    [
        TITLE,
        DESCRIPTION,
        IMAGE,
        ADDRESS,
        KIND,
        DATE,
        ROBOTS,
        LOCALE,
        BROWSER,
    ]
    .iter()
    .any(|g| g.contains(&kind))
}

/// Code ajouté de chaque page : balises de partage retirées, image, type et date devenus des
/// réglages de la page. Pages dont le résultat change : essayées sans le type et la date, puis
/// sans les robots ; sinon laissées telles quelles. Rend le nombre de pages changées.
fn page_tags(input: &BuildInput, site: &mut Site, base: &Prints, base_url: &str) -> Option<usize> {
    let mut todo: Vec<usize> = site
        .pages
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.code.head.trim().is_empty())
        .map(|(i, _)| i)
        .collect();
    let mut total = 0;
    let skips: [&[&[&str]]; 4] = [&[], &[KIND, DATE], &[ROBOTS], &[KIND, DATE, ROBOTS]];
    for skip in skips {
        if todo.is_empty() {
            break;
        }
        let mut cand = site.clone();
        let changed: Vec<usize> = todo
            .iter()
            .copied()
            .filter(|&i| convert_page(&mut cand.pages[i], skip, base_url))
            .collect();
        if changed.is_empty() {
            continue;
        }
        let now = prints(input, &cand)?;
        let differs: BTreeSet<&String> = now
            .iter()
            .filter(|(k, v)| !base.get(*k).is_some_and(|b| same(b, v)))
            .map(|(k, _)| k)
            .chain(base.keys().filter(|k| !now.contains_key(*k)))
            .collect();
        let ok: Vec<usize> = changed
            .into_iter()
            .filter(|&i| {
                crate::output_path(&cand.pages[i].path)
                    .map(|o| !differs.contains(&o))
                    .unwrap_or(false)
            })
            .collect();
        if ok.is_empty() {
            continue;
        }
        // Pages gardées seules : le site produit doit rester le même.
        let mut kept = site.clone();
        for &i in &ok {
            kept.pages[i] = cand.pages[i].clone();
        }
        if same_site(base, &prints(input, &kept)?) {
            *site = kept;
            total += ok.len();
            todo.retain(|i| !ok.contains(i));
        }
    }
    (total > 0).then_some(total)
}

/// Balises de partage du code ajouté d'une page retirées (sauf celles des groupes `skip`) ;
/// image, type et date écrits dans les réglages de la page. Vrai si la page a changé.
fn convert_page(p: &mut revel_model::Page, skip: &[&[&str]], base_url: &str) -> bool {
    let code = p.code.head.clone();
    let mut kept = String::new();
    let mut last = 0;
    let mut any = false;
    for m in TAG_RE.find_iter(&code) {
        let t = m.as_str();
        let (name, attrs) = parse_tag(t);
        let Some(kind) = kind_of(&name, &attrs)
            .filter(|k| automatic(k) && !skip.iter().any(|g| g.contains(&k.as_str())))
        else {
            continue;
        };
        if name == "title" {
            continue;
        }
        let content = attrs
            .iter()
            .find(|(k, _)| k == "content" || k == "href")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        match kind.as_str() {
            "meta:og:image" | "meta:twitter:image" => {
                let seo = p
                    .props
                    .entry("seo".to_string())
                    .or_insert_with(|| Value::Object(Default::default()));
                if let Value::Object(s) = seo
                    && !s.contains_key("image")
                {
                    s.insert("image".into(), Value::String(local(&content, base_url)));
                }
            }
            "meta:og:type" if content != "website" && !p.props.contains_key("ogType") => {
                p.props.insert("ogType".into(), Value::String(content));
            }
            "meta:article:published_time" if !p.props.contains_key("date") => {
                p.props.insert("date".into(), Value::String(content));
            }
            _ => {}
        }
        kept.push_str(&code[last..m.start()]);
        last = m.end();
        // Fin de ligne qui suit la balise : retirée avec elle.
        if code[last..].starts_with('\n') {
            last += 1;
        }
        any = true;
    }
    if any {
        kept.push_str(&code[last..]);
        p.code.head = kept.trim().to_string();
    }
    any
}
