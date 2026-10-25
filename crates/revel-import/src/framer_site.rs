//! Site Framer reconstruit (D-109) d'après ses modules ([`crate::framer`]).
//!
//! - [`apply_page`] (chaque page) : effets d'apparition au défilement en animations Revel,
//!   états `.hover` de la feuille de style en `:hover`, barre de progression de code en barre
//!   Revel (`data-revel-progress`).
//! - [`import_cms`] (le site) : collections du CMS lues dans les fichiers `.framercms` (toutes
//!   les langues), gabarit des pages de contenu tiré de la page la plus complète (champs reliés,
//!   parties sans valeur affichées à condition), listes des autres pages reliées à la
//!   collection (maçonnerie de Framer reprise), textes des autres langues repris dans les
//!   traductions du site (pages des autres langues retirées).
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use revel_model::{Attr, Layout, Locale, Node, Site, Sort, SortKind};
use serde_json::{Map, Value, json};

use crate::Report;
use crate::adapt::{appear_value, attr, clear_initial_state, preset_for, set_attr};
use crate::crawl::Fetch;
use crate::framer::{CmsValue, Modules, decode_chunk, image_src, rich_to_html};
use crate::webflow::{self, Kind};

// -------------------------------------------------------------------------------------------------
// Page

/// Effets lus dans les modules appliqués à une page ; rend le nombre d'animations reprises.
pub fn apply_page(head: &mut [Node], body: &mut [Node], m: &Modules, css: &mut String) -> usize {
    for n in head.iter_mut() {
        if let Node::Element { tag, children, .. } = n
            && tag == "style"
        {
            for c in children.iter_mut() {
                match c {
                    Node::Text { text } => *text = crate::framer::hover_classes_to_pseudo(text),
                    Node::Html { html } => *html = crate::framer::hover_classes_to_pseudo(html),
                    _ => {}
                }
            }
        }
    }
    let mut count = 0;
    walk_mut(body, &mut |n| {
        let Node::Element {
            tag,
            attrs,
            children,
            ..
        } = n
        else {
            return;
        };
        // Vidéo lancée seule : Framer coupe son son par le script (propriété `muted`, absente du
        // HTML publié) ; sans cela, le navigateur bloque la lecture automatique.
        if tag == "video" && attr(attrs, "autoplay").is_some() && attr(attrs, "muted").is_none() {
            set_attr(attrs, "muted", String::new());
        }
        // Apparition au défilement (sauf si une autre animation est déjà reprise).
        let found = classes(attrs).find_map(|c| m.appear.get(c)).cloned();
        if attr(attrs, "data-revel-appear").is_none()
            && let Some(a) = found
        {
            let num = |o: &Map<String, Value>, k: &str, d: f64| {
                o.get(k).and_then(Value::as_f64).unwrap_or(d)
            };
            let (preset, dist) = preset_for(
                num(&a.enter, "x", 0.0),
                num(&a.enter, "y", 0.0),
                num(&a.enter, "scale", 1.0),
                num(&a.enter, "rotate", 0.0),
            );
            let t = &a.transition;
            let spring = t.get("type").and_then(Value::as_str) == Some("spring");
            let dur = num(t, "duration", 0.6);
            let ease = if spring && num(t, "bounce", 0.0) > 0.3 {
                "bounce"
            } else {
                "smooth"
            };
            let mut v = appear_value(preset, dur, num(t, "delay", 0.0), ease, dist);
            if !a.once {
                v.push_str(" replay");
            }
            set_attr(attrs, "data-revel-appear", v);
            clear_initial_state(attrs);
            count += 1;
        }
        // Barre de progression de code : piste masquée aux lecteurs d'écran, un remplissage
        // dont la largeur change avec transition.
        if m.scroll_progress
            && attr(attrs, "aria-hidden") == Some("true")
            && attr(attrs, "style").is_some_and(|s| compact(s).contains("overflow:hidden"))
            && let [Node::Element { attrs: fill, .. }] = children.as_mut_slice()
            && attr(fill, "style").is_some_and(|s| compact(s).contains("transition:width"))
        {
            add_class(attrs, "barre-progression");
            set_attr(attrs, "data-revel-progress", "parent".into());
            add_class(fill, "barre-progression__remplissage");
            let st = attr(fill, "style").unwrap_or_default().to_string();
            let st: Vec<String> = st
                .split(';')
                .filter(|d| !d.trim().is_empty() && !d.trim_start().starts_with("width"))
                .map(str::to_string)
                .chain(std::iter::once(
                    "width:calc(var(--revel-progress, 0) * 100%)".to_string(),
                ))
                .collect();
            set_attr(fill, "style", st.join(";"));
        }
        // Composant qui suit la section affichée : liens de la page qu'il contient, dans l'ordre
        // des variantes ; chaque variante devient le style de la section (D-111).
        if let Some(spy) = m.spy.iter().find(|s| classes(attrs).any(|c| c == s.root)) {
            let mut ids = Vec::new();
            hash_links(children, &mut ids);
            if ids.len() > spy.steps.len() {
                set_attr(attrs, "data-revel-spy", "section".into());
                let mut add = |rule: String| {
                    if !css.contains(&rule) {
                        css.push_str(&rule);
                        css.push('\n');
                    }
                };
                for (i, step) in spy.steps.iter().enumerate() {
                    for (el, decls) in step {
                        let props: Vec<String> = decls
                            .iter()
                            .filter_map(|d| d.split(':').next())
                            .filter(|p| !p.starts_with("--"))
                            .map(|p| format!("{p} {}s cubic-bezier(.33,1,.68,1)", spy.duration))
                            .collect();
                        add(format!(
                            ".{} .{el}{{transition:{}}}",
                            spy.root,
                            props.join(",")
                        ));
                        add(format!(
                            ".{}[data-revel-section=\"{}\"] .{el}{{{}}}",
                            spy.root,
                            ids[i + 1],
                            decls
                                .iter()
                                .map(|d| format!("{d}!important"))
                                .collect::<Vec<_>>()
                                .join(";")
                        ));
                    }
                }
            }
        }
    });
    count
}

/// Cibles des liens vers une section de la page (`#travaux`, `./#travaux`), dans l'ordre.
fn hash_links(nodes: &[Node], out: &mut Vec<String>) {
    for n in nodes {
        if let Node::Element {
            tag,
            attrs,
            children,
            ..
        } = n
        {
            if tag == "a"
                && let Some(h) = attr(attrs, "href")
                && let Some((page, id)) = h.split_once('#')
                && matches!(page, "" | "./" | "/" | ".")
                && !id.is_empty()
                && !out.iter().any(|x| x == id)
            {
                out.push(id.to_string());
            }
            hash_links(children, out);
        }
    }
}

fn compact(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn classes(attrs: &[Attr]) -> impl Iterator<Item = &str> {
    attr(attrs, "class").unwrap_or("").split_whitespace()
}

fn add_class(attrs: &mut Vec<Attr>, c: &str) {
    let cur = attr(attrs, "class").unwrap_or("").to_string();
    if !cur.split_whitespace().any(|x| x == c) {
        set_attr(attrs, "class", format!("{cur} {c}").trim().to_string());
    }
}

fn walk_mut(nodes: &mut [Node], f: &mut dyn FnMut(&mut Node)) {
    for n in nodes.iter_mut() {
        f(n);
        match n {
            Node::Element { children, .. } | Node::Each { children, .. } => walk_mut(children, f),
            Node::If {
                then, otherwise, ..
            } => {
                walk_mut(then, f);
                walk_mut(otherwise, f);
            }
            _ => {}
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Valeurs d'un contenu

/// Contenu lu dans un fichier du CMS : (identifiant du champ, valeur).
type RawItem = Vec<(String, CmsValue)>;

/// Champ d'une collection : clé dans les contenus, identifiant de Framer, type.
#[derive(Debug, Clone)]
struct FieldDef {
    key: String,
    id: String,
    kind: Kind,
}

/// Valeur de recherche : texte normalisé, image (nom de fichier sans extension), média, lien.
fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Nom de fichier de Framer d'une image (`2HRwYs18….png` → `2HRwYs18…`).
fn stem(u: &str) -> Option<String> {
    let last = u.split(['?', '#']).next()?.rsplit('/').next()?;
    let s = last.split(['.', '~']).next()?;
    (s.len() >= 12).then(|| s.to_string())
}

/// Dernier segment décodé d'une adresse (`…/intro.mp4?x` → `intro.mp4`).
fn tail(u: &str) -> String {
    let last = u
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("");
    percent_encoding::percent_decode_str(last)
        .decode_utf8_lossy()
        .to_string()
}

/// Texte visible d'un nœud, espaces regroupés.
fn text_of(nodes: &[Node]) -> String {
    let mut out = String::new();
    fn go(nodes: &[Node], out: &mut String) {
        for n in nodes {
            match n {
                Node::Text { text } => {
                    out.push(' ');
                    out.push_str(text);
                }
                Node::Element { children, .. } => go(children, out),
                _ => {}
            }
        }
    }
    go(nodes, &mut out);
    norm(&out)
}

/// Ce qu'un contenu affiche, pour le retrouver dans une page.
#[derive(Default)]
struct Probe {
    /// Texte → expression.
    texts: Vec<(String, String, String)>,
    /// Nom d'image → expression.
    images: Vec<(String, String, String)>,
    /// Fichier média (vidéo) → expression.
    media: Vec<(String, String, String)>,
    /// Adresse → expression.
    links: Vec<(String, String, String)>,
    /// Texte du texte riche → expression.
    rich: Option<(String, String, String)>,
    /// Noms des images de la galerie (liste d'images) et sa clé.
    gallery: Option<(Vec<String>, String)>,
}

impl Probe {
    /// `prefix` : `""` (gabarit du contenu) ou `item.data.` (élément d'une liste).
    fn new(fields: &[FieldDef], values: &Map<String, Value>, prefix: &str, body: &str) -> Self {
        let mut p = Probe::default();
        let expr = |k: &str| format!("{prefix}{k}");
        for f in fields {
            let v = values.get(&f.key);
            match f.kind {
                Kind::PlainText | Kind::Number | Kind::Date | Kind::Option => {
                    if let Some(s) = v.and_then(Value::as_str).map(norm)
                        && !s.is_empty()
                        && !p.texts.iter().any(|(t, _, _)| *t == s)
                    {
                        let is_media = matches!(
                            tail(&s).rsplit('.').next(),
                            Some("mp4" | "webm" | "mov" | "m4v")
                        );
                        if is_media || s.starts_with("http") {
                            p.media.push((tail(&s), expr(&f.key), f.key.clone()));
                        } else {
                            p.texts.push((s, expr(&f.key), f.key.clone()));
                        }
                    }
                }
                Kind::Link => {
                    if let Some(s) = v.and_then(Value::as_str).filter(|s| !s.is_empty()) {
                        p.links.push((s.to_string(), expr(&f.key), f.key.clone()));
                    }
                }
                Kind::Image => {
                    if let Some(s) = v.and_then(Value::as_str).and_then(stem) {
                        p.images.push((s, expr(&f.key), f.key.clone()));
                    }
                }
                Kind::MultiImage => {
                    if let Some(a) = v.and_then(Value::as_array) {
                        let stems: Vec<String> = a
                            .iter()
                            .filter_map(Value::as_str)
                            .filter_map(stem)
                            .collect();
                        if !stems.is_empty() {
                            p.gallery = Some((stems, f.key.clone()));
                        }
                    }
                }
                Kind::RichText => {
                    let html = if values.contains_key(&f.key) {
                        v.and_then(Value::as_str).unwrap_or("").to_string()
                    } else {
                        body.to_string()
                    };
                    let t = norm(&strip_tags(&html));
                    if !t.is_empty() {
                        let e = if prefix.is_empty() {
                            "content | safe".to_string()
                        } else {
                            format!("{prefix}{} | safe", f.key)
                        };
                        p.rich = Some((t, e, f.key.clone()));
                    }
                }
                _ => {}
            }
        }
        p
    }
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in html.chars() {
        match c {
            '<' => {
                inside = true;
                out.push(' ');
            }
            '>' => inside = false,
            c if !inside => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// Relie les éléments d'un rendu (`nodes`) aux valeurs qu'ils affichent ; rend les clés reliées.
/// Conteneur d'un texte riche de Framer : ses balises reçoivent les styles de texte.
const RICH_WRAPPER: &str = "rich-text-wrapper";

/// Règles de base de Framer pour les textes (`p.framer-text`…) étendues aux balises d'un texte
/// riche venu du CMS (`.rich-text-wrapper > p`), qui n'ont pas la classe `framer-text`.
fn rich_text_rules(sheet: &mut revel_model::Stylesheet) -> usize {
    let re = regex::Regex::new(r"^([a-z][a-z0-9]*)\.framer-text((?::[\w-]+(?:\([^)]*\))?)*)$")
        .expect("regex");
    let mut n = 0;
    for item in &mut sheet.items {
        let revel_model::StyleItem::Rule(r) = item else {
            continue;
        };
        let extra: Vec<String> = r
            .selector
            .split(',')
            .filter_map(|s| re.captures(s.trim()))
            .map(|c| format!(".{RICH_WRAPPER} > {}{}", &c[1], &c[2]))
            .filter(|s| !r.selector.contains(s.as_str()))
            .collect();
        if !extra.is_empty() {
            r.selector = format!("{},{}", r.selector, extra.join(","));
            n += 1;
        }
    }
    n
}

fn bind(nodes: &mut [Node], p: &Probe, url_expr: Option<(&str, &str)>) -> HashSet<String> {
    let mut bound = HashSet::new();
    walk_mut(nodes, &mut |n| match n {
        Node::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            if tag == "img" {
                let src = format!(
                    "{} {}",
                    attr(attrs, "src").unwrap_or(""),
                    attr(attrs, "srcset").unwrap_or("")
                );
                if let Some((_, e, k)) = p.images.iter().find(|(s, _, _)| src.contains(s.as_str()))
                {
                    attrs.retain(|a| a.name != "srcset" && a.name != "sizes");
                    set_attr(attrs, "src", format!("{{{{ {e} }}}}"));
                    bound.insert(k.clone());
                }
            } else if matches!(tag.as_str(), "video" | "source") {
                let src = attr(attrs, "src").map(tail).unwrap_or_default();
                if let Some((_, e, k)) = p.media.iter().find(|(t, _, _)| !t.is_empty() && *t == src)
                {
                    set_attr(attrs, "src", format!("{{{{ {e} }}}}"));
                    bound.insert(k.clone());
                }
            } else if tag == "a" {
                let href = attr(attrs, "href").unwrap_or("").to_string();
                if let Some((_, e, k)) = p.links.iter().find(|(u, _, _)| *u == href) {
                    set_attr(attrs, "href", format!("{{{{ {e} }}}}"));
                    bound.insert(k.clone());
                } else if let Some((url, e)) = url_expr
                    && normalize_url(&href) == normalize_url(url)
                {
                    set_attr(attrs, "href", format!("{{{{ {e} }}}}"));
                }
            }
            if let Some((t, e, k)) = &p.rich
                && !children.is_empty()
                && children.iter().any(|c| matches!(c, Node::Element { tag, .. } if matches!(tag.as_str(), "p" | "h1" | "h2" | "h3" | "h4" | "ul" | "ol")))
                && text_of(children) == *t
            {
                // Styles de texte de Framer par balise : posés sur le conteneur
                // (`rich-text-wrapper`, comme Framer), ils s'appliquent à tout contenu.
                let presets: Vec<String> = children
                    .iter()
                    .filter_map(|c| match c {
                        Node::Element { attrs, .. } => Some(classes(attrs).filter(|c| c.starts_with("framer-styles-preset-")).map(str::to_string).collect::<Vec<_>>()),
                        _ => None,
                    })
                    .flatten()
                    .collect();
                add_class(attrs, RICH_WRAPPER);
                for c in presets {
                    add_class(attrs, &c);
                }
                *children = vec![Node::Out { expr: e.clone() }];
                bound.insert(k.clone());
            }
        }
        Node::Text { text } => {
            let t = norm(text);
            if let Some((_, e, k)) = p.texts.iter().find(|(v, _, _)| *v == t) {
                *n = Node::Out { expr: e.clone() };
                bound.insert(k.clone());
            }
        }
        _ => {}
    });
    bound
}

/// Image reliée à un champ : la hauteur de son cadre suit l'image de chaque contenu. Framer écrit
/// le rapport de l'image de la page (`aspect-ratio` du cadre, image en position absolue) ; il
/// est retiré et l'image remise dans le flux (`height:auto`), si bien que chaque image donne sa
/// propre hauteur, comme dans Framer.
fn natural_ratio(nodes: &mut [Node]) {
    for n in nodes.iter_mut() {
        let Node::Element {
            attrs, children, ..
        } = n
        else {
            continue;
        };
        let ratio = attr(attrs, "style").and_then(|s| {
            s.split(';').find_map(|d| {
                d.trim()
                    .strip_prefix("aspect-ratio:")
                    .map(|v| v.trim().to_string())
            })
        });
        if let Some(r) = ratio.and_then(|r| r.parse::<f64>().ok())
            && let Some((w, h)) = bound_img_size(children, 3)
            && h > 0.0
            && ((w / h) - r).abs() / r < 0.02
        {
            set_style(attrs, "aspect-ratio", None);
            unpin(children, 3);
            continue;
        }
        natural_ratio(children);
    }
}

/// Taille écrite (`width`, `height`) de l'image reliée (`{{ … }}`) trouvée sous `depth` niveaux.
fn bound_img_size(nodes: &[Node], depth: usize) -> Option<(f64, f64)> {
    for n in nodes {
        if let Node::Element {
            tag,
            attrs,
            children,
            ..
        } = n
        {
            if tag == "img" && attr(attrs, "src").is_some_and(|s| s.contains("{{")) {
                let w = attr(attrs, "width")?.parse().ok()?;
                let h = attr(attrs, "height")?.parse().ok()?;
                return Some((w, h));
            }
            if depth > 0
                && let Some(x) = bound_img_size(children, depth - 1)
            {
                return Some(x);
            }
        }
    }
    None
}

/// Enveloppe d'image posée sur tout son parent (`position:absolute` et bords à 0) remise dans
/// le flux ; image à hauteur libre, sans taille écrite.
fn unpin(nodes: &mut [Node], depth: usize) {
    for n in nodes.iter_mut() {
        if let Node::Element {
            tag,
            attrs,
            children,
            ..
        } = n
        {
            if tag == "img" {
                attrs.retain(|a| a.name != "width" && a.name != "height");
                set_style(attrs, "height", Some("auto"));
            } else if attr(attrs, "style").is_some_and(|s| compact(s).contains("position:absolute"))
            {
                set_style(attrs, "position", Some("relative"));
                for k in ["top", "right", "bottom", "left", "inset"] {
                    set_style(attrs, k, None);
                }
            }
            if depth > 0 {
                unpin(children, depth - 1);
            }
        }
    }
}

/// Déclaration d'un style écrit sur l'élément changée (`None` : retirée).
fn set_style(attrs: &mut Vec<Attr>, prop: &str, value: Option<&str>) {
    let st = attr(attrs, "style").unwrap_or_default().to_string();
    let mut decls: Vec<String> = st
        .split(';')
        .filter(|d| {
            let k = d.split(':').next().unwrap_or("").trim();
            !d.trim().is_empty() && k != prop
        })
        .map(|d| d.trim().to_string())
        .collect();
    if let Some(v) = value {
        decls.push(format!("{prop}:{v}"));
    }
    if decls.is_empty() {
        attrs.retain(|a| a.name != "style");
    } else {
        set_attr(attrs, "style", decls.join(";"));
    }
}

/// Clés relues dans un sous-arbre (`{{ … }}` et sorties).
/// `s` cite le nom `name` en entier (`video_url` n'est pas cité par `video_url_2`).
fn mentions(s: &str, name: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_' || c == '.';
    s.match_indices(name).any(|(i, _)| {
        !s[..i].chars().next_back().is_some_and(ident)
            && !s[i + name.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

fn keys_in(nodes: &[Node], keys: &[String], prefix: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut look = |s: &str| {
        for k in keys {
            if mentions(s, &format!("{prefix}{k}")) {
                out.insert(k.clone());
            }
        }
    };
    fn go(nodes: &[Node], look: &mut dyn FnMut(&str)) {
        for n in nodes {
            match n {
                Node::Out { expr } => look(expr),
                Node::Element {
                    attrs, children, ..
                } => {
                    for a in attrs {
                        if let Some(v) = &a.value {
                            look(v);
                        }
                    }
                    go(children, look);
                }
                Node::Each { children, .. } => go(children, look),
                Node::If {
                    then, otherwise, ..
                } => {
                    go(then, look);
                    go(otherwise, look);
                }
                _ => {}
            }
        }
    }
    go(nodes, &mut look);
    out
}

/// Parties reliées à des champs parfois vides : affichées seulement quand le champ a une valeur
/// (bouton sans lien, seconde vidéo…). L'élément entouré est le plus haut qui ne contient que
/// cette partie (enveloppes à un seul enfant).
fn wrap_optional(
    nodes: &mut Vec<Node>,
    optional: &[String],
    prefix: &str,
    filled: &HashMap<String, Vec<bool>>,
) {
    let all = optional.to_vec();
    let mut i = 0;
    while i < nodes.len() {
        let keys = keys_in(std::slice::from_ref(&nodes[i]), &all, prefix);
        let is_el = matches!(nodes[i], Node::Element { .. });
        // Champs vides pour les mêmes contenus : une seule condition ; sinon chaque partie.
        let same = {
            let mut pats = keys.iter().map(|k| filled.get(k));
            let first = pats.next().flatten();
            pats.all(|p| p == first)
        };
        if is_el && !keys.is_empty() && same && only_optional(&nodes[i], &all, prefix) {
            let mut ks: Vec<String> = keys.into_iter().collect();
            ks.sort();
            let cond = ks
                .iter()
                .map(|k| format!("{prefix}{k}"))
                .collect::<Vec<_>>()
                .join(" and ");
            let node = nodes.remove(i);
            nodes.insert(
                i,
                Node::If {
                    id: None,
                    cond,
                    then: vec![node],
                    otherwise: vec![],
                },
            );
        } else if let Node::Element { children, .. } = &mut nodes[i] {
            wrap_optional(children, optional, prefix, filled);
        } else if let Node::Each { children, .. } = &mut nodes[i] {
            wrap_optional(children, optional, prefix, filled);
        }
        i += 1;
    }
}

/// Le sous-arbre n'affiche que des valeurs de champs facultatifs (pas d'autre texte relu,
/// pas de texte écrit) : il peut disparaître avec elles.
fn only_optional(n: &Node, optional: &[String], prefix: &str) -> bool {
    fn go(n: &Node, optional: &[String], prefix: &str) -> bool {
        match n {
            Node::Out { expr } => optional
                .iter()
                .any(|k| mentions(expr, &format!("{prefix}{k}"))),
            Node::Text { text } => text.trim().is_empty(),
            Node::Element { children, .. } => children.iter().all(|c| go(c, optional, prefix)),
            Node::Each { .. } | Node::If { .. } => false,
            _ => true,
        }
    }
    go(n, optional, prefix)
}

// -------------------------------------------------------------------------------------------------
// Listes (cartes répétées) et maçonnerie

/// Chemins (indices d'enfants) des nœuds qui vérifient `pred`.
fn paths_of(nodes: &[Node], pred: &dyn Fn(&Node) -> bool) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    fn go(
        nodes: &[Node],
        path: &mut Vec<usize>,
        pred: &dyn Fn(&Node) -> bool,
        out: &mut Vec<Vec<usize>>,
    ) {
        for (i, n) in nodes.iter().enumerate() {
            path.push(i);
            if pred(n) {
                out.push(path.clone());
            } else if let Node::Element { children, .. } = n {
                go(children, path, pred, out);
            }
            path.pop();
        }
    }
    go(nodes, &mut Vec::new(), pred, &mut out);
    out
}

fn common_prefix(paths: &[Vec<usize>]) -> Vec<usize> {
    let Some(first) = paths.first() else {
        return Vec::new();
    };
    let mut n = first.len();
    for p in paths {
        n = n.min(p.iter().zip(first).take_while(|(a, b)| a == b).count());
    }
    first[..n].to_vec()
}

fn node_at_mut<'a>(nodes: &'a mut [Node], path: &[usize]) -> Option<&'a mut Node> {
    let (first, rest) = path.split_first()?;
    let n = nodes.get_mut(*first)?;
    if rest.is_empty() {
        return Some(n);
    }
    match n {
        Node::Element { children, .. } => node_at_mut(children, rest),
        _ => None,
    }
}

/// Groupes de cartes d'une même liste : les copies d'une liste (un rendu par point de rupture
/// de Framer) sont séparées ; les colonnes d'une maçonnerie restent ensemble. `key(path)` :
/// contenu montré par la carte.
fn groups(paths: Vec<Vec<usize>>, key: &dyn Fn(&[usize]) -> String) -> Vec<Vec<Vec<usize>>> {
    if paths.len() < 2 {
        return if paths.is_empty() {
            vec![]
        } else {
            vec![paths]
        };
    }
    let lca = common_prefix(&paths);
    let mut parts: BTreeMap<usize, Vec<Vec<usize>>> = BTreeMap::new();
    for p in &paths {
        if let Some(i) = p.get(lca.len()) {
            parts.entry(*i).or_default().push(p.clone());
        }
    }
    let sets: Vec<Vec<String>> = parts
        .values()
        .map(|ps| {
            let mut v: Vec<String> = ps.iter().map(|p| key(p)).collect();
            v.sort();
            v
        })
        .collect();
    // Copies de la même liste (mêmes contenus) : chacune à part.
    if parts.len() > 1 && sets.iter().all(|s| *s == sets[0]) && sets[0].len() > 1 {
        return parts.into_values().flat_map(|ps| groups(ps, key)).collect();
    }
    vec![paths]
}

/// Racine de chaque carte : ancêtres à la plus petite profondeur (sous l'ancêtre commun, de
/// longueur `lca`) où chaque carte a le sien.
fn card_roots(cards: &[Vec<usize>], lca: usize) -> Option<Vec<Vec<usize>>> {
    let max = cards.iter().map(Vec::len).min()?;
    (lca + 1..=max).find_map(|d| {
        let roots: Vec<Vec<usize>> = cards.iter().map(|c| c[..d].to_vec()).collect();
        let unique: HashSet<&Vec<usize>> = roots.iter().collect();
        (unique.len() == roots.len()).then_some(roots)
    })
}

/// Remplace un groupe de cartes par une liste (`each`) : `make(card)` donne le modèle d'une carte
/// d'après la première ; colonnes d'une maçonnerie réunies en une grille `data-revel-masonry`.
/// Rend vrai si la liste a été écrite.
fn rebuild_group(
    root: &mut [Node],
    cards: &[Vec<usize>],
    order: &dyn Fn(&[usize]) -> usize,
    each: &dyn Fn(Node) -> Node,
) -> bool {
    // Carte : plus haut ancêtre qui ne contient qu'un lien de la liste.
    let lca = common_prefix(cards);
    let Some(card_roots) = card_roots(cards, lca.len()) else {
        return false;
    };
    let depths: HashSet<usize> = card_roots.iter().map(Vec::len).collect();
    if depths.len() != 1 {
        return false;
    }
    let depth = *depths.iter().next().expect("profondeur");
    // Cartes directement dans le conteneur, ou dans ses colonnes (maçonnerie de Framer).
    let (container, masonry) = if depth == lca.len() + 1 {
        (lca.clone(), false)
    } else if depth == lca.len() + 2 {
        (lca.clone(), true)
    } else {
        return false;
    };
    let mut sorted: Vec<&Vec<usize>> = card_roots.iter().collect();
    sorted.sort_by_key(|p| order(p));
    let first = sorted[0].clone();
    let Some(Node::Element {
        children, attrs, ..
    }) = node_at_mut(root, &container)
    else {
        return false;
    };
    let rel = &first[container.len()..];
    let template = if masonry {
        let Some(Node::Element { children: col, .. }) = children.get(rel[0]) else {
            return false;
        };
        col.get(rel[1]).cloned()
    } else {
        children.get(rel[0]).cloned()
    };
    let Some(template) = template else {
        return false;
    };
    if masonry {
        // Écart des colonnes : écart des lignes de la grille.
        let gap = children.iter().find_map(|c| match c {
            Node::Element { attrs, .. } => attr(attrs, "style").and_then(|s| {
                s.split(';').find_map(|d| {
                    d.trim()
                        .strip_prefix("row-gap:")
                        .map(|v| v.trim().to_string())
                })
            }),
            _ => None,
        });
        let st = attr(attrs, "style").unwrap_or_default().to_string();
        let mut st: Vec<String> = st
            .split(';')
            .filter(|d| !d.trim().is_empty() && !d.trim_start().starts_with("row-gap"))
            .map(str::to_string)
            .collect();
        if let Some(g) = gap {
            st.push(format!("row-gap:{g}"));
        }
        set_attr(attrs, "style", st.join(";"));
        set_attr(attrs, "data-revel-masonry", "order".into());
        *children = vec![each(template)];
    } else {
        let idx: HashSet<usize> = card_roots.iter().map(|p| p[container.len()]).collect();
        let at = *idx.iter().min().expect("carte");
        let mut kept = Vec::new();
        for (i, c) in std::mem::take(children).into_iter().enumerate() {
            if i == at {
                kept.push(each(template.clone()));
            } else if !idx.contains(&i) {
                kept.push(c);
            }
        }
        *children = kept;
    }
    true
}

// -------------------------------------------------------------------------------------------------
// Collections

/// Champ de Framer → type Revel (`None` : non repris).
fn kind_of(ty: &str, gallery_like: bool) -> Option<Kind> {
    Some(match ty {
        "String" => Kind::PlainText,
        "Link" => Kind::Link,
        "ResponsiveImage" | "Image" => Kind::Image,
        "RichText" | "FormattedText" => Kind::RichText,
        "Number" => Kind::Number,
        "Boolean" => Kind::Switch,
        "Date" => Kind::Date,
        "Enum" => Kind::Option,
        "Color" => Kind::PlainText,
        "File" => Kind::File,
        "Array" if gallery_like => Kind::MultiImage,
        _ => return None,
    })
}

fn key_of(title: &str) -> String {
    let k = revel_model::slugify(title).replace('-', "_");
    if k.is_empty() { "champ".into() } else { k }
}

/// Valeur d'un champ du CMS de Framer pour un contenu Revel.
fn value_of(v: &CmsValue, kind: &Kind) -> Option<Value> {
    Some(match (v, kind) {
        (CmsValue::Null, _) => return None,
        (CmsValue::String(s), _) => Value::String(s.clone()),
        (CmsValue::Link(s), _) => {
            Value::String(serde_json::from_str::<String>(s).unwrap_or_else(|_| s.clone()))
        }
        (CmsValue::Image(j), _) => Value::String(image_src(j)?),
        (CmsValue::Rich(j), _) => Value::String(rich_to_html(j)),
        (CmsValue::Date(ms), _) => {
            let d = chrono::DateTime::from_timestamp_millis(*ms as i64)?;
            Value::String(d.format("%Y-%m-%d").to_string())
        }
        (CmsValue::Array(a), Kind::MultiImage) => Value::Array(
            a.iter()
                .filter_map(|x| match x {
                    CmsValue::Object(o) => o.iter().find_map(|(_, v)| match v {
                        CmsValue::Image(j) => image_src(j).map(Value::String),
                        _ => None,
                    }),
                    CmsValue::Image(j) => image_src(j).map(Value::String),
                    _ => None,
                })
                .collect(),
        ),
        _ => return None,
    })
}

/// Collection de Framer lue : champs, contenus par langue (`id` de langue → valeurs par champ).
struct Read {
    label: String,
    url_prefix: String,
    fields: Vec<FieldDef>,
    by_locale: Vec<(String, Vec<Map<String, Value>>)>,
}

fn read_collections(m: &Modules, fetch: &dyn Fetch, report: &mut Report) -> Vec<Read> {
    let mut out = Vec::new();
    for r in &m.cms {
        let schema = url::Url::parse(&r.module_url)
            .ok()
            .and_then(|u| fetch.get(&u).ok())
            .map(|f| String::from_utf8_lossy(&f.body).to_string())
            .map(|js| crate::framer::cms_fields(&js))
            .unwrap_or_default();
        let mut by_locale = Vec::new();
        let mut raw_items: Vec<(String, Vec<RawItem>)> = Vec::new();
        for (loc, u) in &r.chunks {
            let Some(bytes) = url::Url::parse(u)
                .ok()
                .and_then(|u| fetch.get(&u).ok())
                .filter(|f| f.status == 200)
            else {
                report
                    .warnings
                    .push(format!("Framer : données du CMS introuvables ({u})"));
                continue;
            };
            match decode_chunk(&bytes.body) {
                Ok(items) => raw_items.push((loc.clone(), items)),
                Err(e) => report
                    .warnings
                    .push(format!("Framer : données du CMS illisibles ({e})")),
            }
        }
        let Some((_, first)) = raw_items.first() else {
            continue;
        };
        // Route : `/projects/:LU2lHdkQv` → préfixe et champ de l'adresse.
        let route = m
            .routes
            .iter()
            .find(|(_, id)| schema.iter().any(|(fid, _, _)| fid == id));
        let slug_id = route.map(|(_, id)| id.clone());
        let url_prefix = route
            .map(|(p, _)| p.trim_matches('/').to_string())
            .unwrap_or_default();
        let label = url_prefix
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| {
                let mut c = s.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            })
            .unwrap_or_else(|| "Collection".into());
        // Champs : titre d'abord (`name`), adresse (`slug`), puis les autres dans l'ordre.
        let mut fields = Vec::new();
        let mut used = HashSet::new();
        let mut name_done = false;
        for (id, title, ty) in &schema {
            if matches!(id.as_str(), "createdAt" | "updatedAt") || id.ends_with("ItemId") {
                continue;
            }
            let gallery_like = ty == "Array"
                && first.iter().any(|it| {
                    it.iter().any(|(k, v)| {
                        k == id
                            && matches!(v, CmsValue::Array(a) if a.iter().all(|x| matches!(x, CmsValue::Object(o) if o.iter().any(|(_, v)| matches!(v, CmsValue::Image(_))))))
                    })
                });
            let Some(kind) = kind_of(ty, gallery_like) else {
                continue;
            };
            let key = if Some(id) == slug_id.as_ref() {
                "slug".to_string()
            } else if !name_done && kind == Kind::PlainText {
                name_done = true;
                "name".to_string()
            } else {
                let mut k = key_of(title);
                while matches!(k.as_str(), "name" | "slug" | "draft" | "order" | "content")
                    || !used.insert(k.clone())
                {
                    k.push('_');
                }
                k
            };
            fields.push(FieldDef {
                key,
                id: id.clone(),
                kind,
            });
            let _ = title;
        }
        for (loc, items) in &raw_items {
            let list: Vec<Map<String, Value>> = items
                .iter()
                .map(|it| {
                    let mut vals = Map::new();
                    for f in &fields {
                        if let Some((_, v)) = it.iter().find(|(k, _)| *k == f.id)
                            && let Some(v) = value_of(v, &f.kind)
                        {
                            vals.insert(f.key.clone(), v);
                        }
                    }
                    vals
                })
                .collect();
            by_locale.push((loc.clone(), list));
        }
        let labels: HashMap<&str, &str> = schema
            .iter()
            .map(|(id, t, _)| (id.as_str(), t.as_str()))
            .collect();
        let _ = labels;
        out.push(Read {
            label,
            url_prefix,
            fields,
            by_locale,
        });
    }
    out
}

/// Collections du CMS de Framer, gabarit des pages de contenu, listes reliées, autres langues.
pub fn import_cms(site: &mut Site, dest: &Path, fetch: &dyn Fetch, report: &mut Report) {
    let m = Modules::load(dest);
    let cols = read_collections(&m, fetch, report);
    let main_locale = m.locales.iter().find(|l| l.slug.is_empty());
    let others: Vec<_> = m.locales.iter().filter(|l| !l.slug.is_empty()).collect();
    let main_id = main_locale
        .map(|l| l.id.clone())
        .unwrap_or_else(|| "default".into());
    // Pages des autres langues : relues pour les traductions, puis retirées.
    let other_prefixes: Vec<String> = others.iter().map(|l| format!("/{}/", l.slug)).collect();
    let cms_prefixes: Vec<String> = cols
        .iter()
        .map(|c| c.url_prefix.split('/').next().unwrap_or("").to_string())
        .collect();
    // Traductions des gabarits, ajoutées après celles des pages.
    let mut later: Vec<(String, BTreeMap<String, String>)> = Vec::new();
    for c in cols {
        let Some((_, main_items)) = c.by_locale.iter().find(|(l, _)| *l == main_id).cloned() else {
            continue;
        };
        let titles: HashMap<String, String> = HashMap::new();
        let _ = titles;
        // Champ « ordre » : l'ordre du CMS de Framer.
        let wf_fields: Vec<webflow::Field> = c
            .fields
            .iter()
            .filter(|f| f.key != "slug")
            .map(|f| webflow::Field {
                key: f.key.clone(),
                label: label_for(&f.key, &m, &c),
                kind: f.kind.clone(),
            })
            .chain(std::iter::once(webflow::Field {
                key: "order".into(),
                label: "Order".into(),
                kind: Kind::Number,
            }))
            .collect();
        let items: Vec<webflow::Item> = main_items
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let mut values = v.clone();
                // Vidéos copiées avec le site : adresse de la copie (D-36).
                for x in values.values_mut() {
                    if let Some(local) = x.as_str().and_then(|u| local_copy(dest, u)) {
                        *x = Value::String(local);
                    }
                }
                values.insert("order".into(), json!(i + 1));
                webflow::Item {
                    id: String::new(),
                    slug: v
                        .get("slug")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    draft: false,
                    values,
                }
            })
            .collect();
        let before = site.collections.len();
        let wf = webflow::Collection {
            id: String::new(),
            slug: c.url_prefix.clone(),
            label: c.label.clone(),
            fields: wf_fields,
            items,
        };
        if let Err(e) = webflow::apply(site, dest, vec![wf], fetch, None, &mut |_| {}, report) {
            report.warnings.push(format!(
                "Framer : collection « {} » non écrite ({e})",
                c.label
            ));
            continue;
        }
        // Avertissement de Webflow sans objet ici (le gabarit vient des pages de contenu).
        report
            .warnings
            .retain(|w| !w.contains("pas de page modèle dans l'export"));
        let Some(col) = site.collections.get_mut(before) else {
            continue;
        };
        col.sort = Some(Sort {
            field: "order".into(),
            descending: false,
            kind: SortKind::Number,
        });
        let col_id = col.id.clone();
        let folder = col.folder.clone();
        // Valeurs écrites (images téléchargées) relues pour retrouver les pages.
        let written = read_entries(dest, &folder);
        let slugs: Vec<String> = main_items
            .iter()
            .filter_map(|v| v.get("slug").and_then(Value::as_str).map(str::to_string))
            .collect();
        let page_url = |slug: &str| format!("/{}/{}/", c.url_prefix, slug);
        let probe_for = |slug: &str, prefix: &str| -> Option<Probe> {
            let (data, body) = written.get(slug)?;
            Some(Probe::new(&c.fields, data, prefix, body))
        };

        // Gabarit : page du contenu le plus complet.
        let best = main_items
            .iter()
            .max_by_key(|v| v.values().filter(|x| !is_empty(x)).count())
            .and_then(|v| v.get("slug").and_then(Value::as_str))
            .map(str::to_string);
        let optional: Vec<String> = c
            .fields
            .iter()
            .filter(|f| {
                main_items
                    .iter()
                    .any(|v| v.get(&f.key).is_none_or(is_empty))
            })
            .map(|f| {
                if f.kind == Kind::RichText {
                    "content".to_string()
                } else {
                    f.key.clone()
                }
            })
            .collect();
        // Contenus où chaque champ a une valeur (même motif : même condition).
        let filled: HashMap<String, Vec<bool>> = c
            .fields
            .iter()
            .map(|f| {
                let k = if f.kind == Kind::RichText {
                    "content".to_string()
                } else {
                    f.key.clone()
                };
                (
                    k,
                    main_items
                        .iter()
                        .map(|v| v.get(&f.key).is_some_and(|x| !is_empty(x)))
                        .collect(),
                )
            })
            .collect();
        if !c.url_prefix.is_empty()
            && let Some(best) = best
            && let Some(pi) = site.pages.iter().position(|p| p.path == page_url(&best))
            && let Some(probe) = probe_for(&best, "")
        {
            let mut page = site.pages[pi].clone();
            let mut body = std::mem::take(&mut page.body);
            let bound = bind(&mut body, &probe, None);
            // Galerie : cartes des images de la liste réunies en une liste (une par rendu de
            // point de rupture).
            if let Some((stems, key)) = &probe.gallery {
                let imgs = paths_of(&body, &|n| img_matches(n, stems));
                let ro = body.clone();
                let stem_at = |p: &[usize]| -> String {
                    node_at(&ro, p)
                        .and_then(first_img_src)
                        .and_then(|s| stems.iter().find(|x| s.contains(x.as_str())).cloned())
                        .unwrap_or_default()
                };
                let mut gs = groups(imgs, &stem_at);
                gs.sort_by_key(|g| std::cmp::Reverse(common_prefix(g)));
                let mut done = 0;
                for g in gs {
                    let order_paths: Vec<(Vec<usize>, usize)> = g
                        .iter()
                        .map(|p| {
                            let s = stem_at(p);
                            (
                                p.clone(),
                                stems.iter().position(|x| *x == s).unwrap_or(usize::MAX),
                            )
                        })
                        .collect();
                    let order = move |p: &[usize]| -> usize {
                        order_paths
                            .iter()
                            .filter(|(q, _)| q.starts_with(p))
                            .map(|(_, o)| *o)
                            .min()
                            .unwrap_or(usize::MAX)
                    };
                    // Cartes voisines (image d'aperçu) : gardées en tête de la grille.
                    let lca = common_prefix(&g);
                    let siblings: Vec<Vec<usize>> = paths_of(
                        &body,
                        &|n| matches!(n, Node::Element { tag, .. } if tag == "img"),
                    )
                    .into_iter()
                    .filter(|p| p.starts_with(&lca) && !g.contains(p))
                    .collect();
                    if gallery_rebuild(&mut body, &g, &siblings, &order, key) {
                        done += 1;
                    }
                }
                if done > 0 {
                    report.warnings.push(format!(
                        "CMS « {} » : galerie reliée au champ « {key} » ({done} rendu(s))",
                        c.label
                    ));
                }
            }
            natural_ratio(&mut body);
            wrap_optional(&mut body, &optional, "", &filled);
            let layout_id = format!("cms-{}", col_id.replace('_', "-"));
            site.layouts.push(Layout {
                id: layout_id.clone(),
                parent: page.layout.clone(),
                props: Default::default(),
                fields: Vec::new(),
                body: body.clone(),
            });
            report.layouts += 1;
            if let Some(col) = site.collections.iter_mut().find(|c2| c2.id == col_id) {
                col.path = Some(format!("/{}/{{{{ slug }}}}/", c.url_prefix));
                col.layout = Some(layout_id);
            }
            // Traductions du gabarit : page du même contenu dans chaque autre langue.
            for (l, pre) in others.iter().zip(&other_prefixes) {
                let url = format!("{}{}", pre.trim_end_matches('/'), page_url(&best));
                if let Some(other) = site.pages.iter().find(|p| p.path == url) {
                    let mut tr = BTreeMap::new();
                    let raw = site.pages[pi].body.clone();
                    align(&raw, &other.body, &mut tr);
                    // Après celles des pages : un même texte traduit autrement dans un
                    // gabarit garde la traduction des pages.
                    let skip: HashSet<&str> =
                        probe.texts.iter().map(|(t, _, _)| t.as_str()).collect();
                    tr.retain(|k, _| !skip.contains(k.as_str()));
                    later.push((l.slug.clone(), tr));
                }
            }
            let _ = bound;
            // Pages de contenu retirées : produites par le gabarit.
            let urls: HashSet<String> = slugs.iter().map(|s| page_url(s)).collect();
            let before = site.pages.len();
            site.pages.retain(|p| !urls.contains(&p.path));
            report.pages = report.pages.saturating_sub(before - site.pages.len());
        }

        // Listes des autres pages : cartes qui mènent aux pages des contenus.
        let by_url: HashMap<String, usize> = slugs
            .iter()
            .enumerate()
            .map(|(i, s)| (page_url(s), i))
            .collect();
        let mut lists = 0;
        for page in site.pages.iter_mut() {
            if other_prefixes
                .iter()
                .any(|p| page.path.starts_with(p.as_str()))
            {
                continue;
            }
            let links = paths_of(&page.body, &|n| match n {
                Node::Element { tag, attrs, .. } if tag == "a" => {
                    attr(attrs, "href").is_some_and(|h| by_url.contains_key(&normalize_url(h)))
                }
                _ => false,
            });
            if links.len() < 2 {
                continue;
            }
            let body_ro = page.body.clone();
            let slug_at = |p: &[usize]| -> String {
                node_at(&body_ro, p)
                    .and_then(|n| match n {
                        Node::Element { attrs, .. } => attr(attrs, "href").map(normalize_url),
                        _ => None,
                    })
                    .unwrap_or_default()
            };
            let mut gs = groups(links, &slug_at);
            // Du plus profond au moins profond : les chemins restent valables.
            gs.sort_by_key(|g| std::cmp::Reverse(common_prefix(g)));
            for g in gs {
                let first_url = g
                    .iter()
                    .map(|p| slug_at(p))
                    .min_by_key(|u| by_url.get(u).copied().unwrap_or(usize::MAX))
                    .unwrap_or_default();
                let Some(first_slug) = first_url
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .map(str::to_string)
                else {
                    continue;
                };
                let Some(probe) = probe_for(&first_slug, "item.data.") else {
                    continue;
                };
                let order = |p: &[usize]| -> usize {
                    g.iter()
                        .filter(|q| q.starts_with(p))
                        .filter_map(|q| by_url.get(&slug_at(q)).copied())
                        .min()
                        .unwrap_or(usize::MAX)
                };
                let col_expr = format!("collections.{col_id}");
                let optional = optional.clone();
                let each = |template: Node| -> Node {
                    let mut t = vec![template];
                    bind(&mut t, &probe, Some((&first_url, "item.url")));
                    natural_ratio(&mut t);
                    let opt: Vec<String> = optional
                        .iter()
                        .map(|k| {
                            if k == "content" {
                                "content".to_string()
                            } else {
                                k.clone()
                            }
                        })
                        .collect();
                    wrap_optional(&mut t, &opt, "item.data.", &filled);
                    Node::Each {
                        id: None,
                        expr: col_expr.clone(),
                        item: "item".into(),
                        children: t,
                        empty: vec![],
                    }
                };
                if rebuild_group(&mut page.body, &g, &order, &each) {
                    lists += 1;
                }
            }
        }
        if lists > 0 {
            report.warnings.push(format!(
                "CMS « {} » : {lists} liste(s) reliée(s) à la collection (tous les contenus montrés)",
                c.label
            ));
        }

        // Contenus des autres langues : valeurs qui diffèrent de la langue principale.
        for l in &others {
            let Some((_, items)) = c.by_locale.iter().find(|(id, _)| *id == l.id) else {
                continue;
            };
            write_locale_entries(dest, &folder, &c, &main_items, items, &l.slug, report);
        }
    }

    // Dossiers des pages d'origine produites désormais par Revel (contenus, autres langues) :
    // plus copiés tels quels ; images du CMS copiées.
    let generated: Vec<String> = cms_prefixes
        .iter()
        .cloned()
        .chain(others.iter().map(|l| l.slug.clone()))
        .filter(|p| !p.is_empty())
        .collect();
    site.passthrough.retain(|p| {
        !generated
            .iter()
            .any(|g| p.from.trim_matches('/') == g.as_str())
    });
    if dest.join("images").is_dir() && !site.passthrough.iter().any(|p| p.from == "images") {
        site.passthrough.push(revel_model::Passthrough {
            from: "images".into(),
            to: "images".into(),
        });
    }
    if (serde_json::to_string(&site.layouts).is_ok_and(|j| j.contains(RICH_WRAPPER))
        || serde_json::to_string(&site.pages).is_ok_and(|j| j.contains(RICH_WRAPPER)))
        && let Some(sheet) = site.stylesheet.as_mut()
    {
        rich_text_rules(sheet);
    }
    // Langues : textes des pages traduits (même structure dans chaque langue), puis pages des
    // autres langues retirées (produites par Revel).
    for (l, pre) in others.iter().zip(&other_prefixes) {
        let mut tr = BTreeMap::new();
        let mains: Vec<(String, Vec<Node>)> = site
            .pages
            .iter()
            .filter(|p| {
                !other_prefixes
                    .iter()
                    .any(|x| p.path.starts_with(x.as_str()))
            })
            .map(|p| (p.path.clone(), p.body.clone()))
            .collect();
        for (path, body) in &mains {
            let url = format!("{}{}", pre.trim_end_matches('/'), path);
            if let Some(o) = site.pages.iter().find(|p| p.path == url) {
                align(body, &o.body, &mut tr);
            }
        }
        add_texts(site, &l.slug, tr, &Probe::default());
        for (code, tr) in later.iter().filter(|(c, _)| *c == l.slug) {
            let l = site.locales.iter_mut().find(|x| &x.code == code);
            if let Some(l) = l {
                for (k, v) in tr {
                    match l.texts.get(k) {
                        Some(have) if have != v => report.warnings.push(format!(
                            "Langue « {code} » : « {k} » traduit « {have} » dans les pages et « {v} » dans un gabarit ; une seule traduction par texte, celle des pages est gardée"
                        )),
                        Some(_) => {}
                        None => {
                            l.texts.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
        }
        let before = site.pages.len();
        site.pages.retain(|p| !p.path.starts_with(pre.as_str()));
        report.pages = report.pages.saturating_sub(before - site.pages.len());
        report.warnings.push(format!(
            "Framer : langue « {} » reprise dans Paramètres › Langues ({} textes)",
            l.name,
            site.locales
                .iter()
                .find(|x| x.code == l.slug)
                .map(|x| x.texts.len())
                .unwrap_or(0)
        ));
    }
}

/// Copie locale (`/_externe/<hôte>/<chemin>`) d'un fichier lu avec le site, s'il y en a une.
fn local_copy(dest: &Path, u: &str) -> Option<String> {
    if !u.starts_with("http") {
        return None;
    }
    let url = url::Url::parse(u).ok()?;
    let path = percent_encoding::percent_decode_str(url.path())
        .decode_utf8_lossy()
        .to_string();
    let rel = format!("_externe/{}{}", url.host_str()?, path);
    dest.join(&rel).is_file().then(|| format!("/{rel}"))
}

fn label_for(key: &str, _m: &Modules, c: &Read) -> String {
    let _ = c;
    let mut s = key.replace('_', " ");
    if let Some(f) = s.get(..1) {
        s = f.to_uppercase() + &s[1..];
    }
    s
}

fn is_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Array(a) => a.is_empty(),
        _ => false,
    }
}

fn normalize_url(h: &str) -> String {
    let h = h.split(['?', '#']).next().unwrap_or(h);
    let h = h.strip_prefix('.').unwrap_or(h);
    let mut s = if h.starts_with('/') {
        h.to_string()
    } else {
        format!("/{h}")
    };
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}

fn node_at<'a>(nodes: &'a [Node], path: &[usize]) -> Option<&'a Node> {
    let (first, rest) = path.split_first()?;
    let n = nodes.get(*first)?;
    if rest.is_empty() {
        return Some(n);
    }
    match n {
        Node::Element { children, .. } => node_at(children, rest),
        _ => None,
    }
}

fn first_img_src(n: &Node) -> Option<String> {
    match n {
        Node::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            if tag == "img" {
                return Some(format!(
                    "{} {}",
                    attr(attrs, "src").unwrap_or(""),
                    attr(attrs, "srcset").unwrap_or("")
                ));
            }
            children.iter().find_map(first_img_src)
        }
        _ => None,
    }
}

fn img_matches(n: &Node, stems: &[String]) -> bool {
    match n {
        Node::Element { tag, attrs, .. } if tag == "img" => {
            let s = format!(
                "{} {}",
                attr(attrs, "src").unwrap_or(""),
                attr(attrs, "srcset").unwrap_or("")
            );
            stems.iter().any(|x| s.contains(x.as_str()))
        }
        _ => false,
    }
}

/// Galerie : cartes des images de la liste (`imgs`) et cartes voisines (`siblings`, image
/// d'aperçu) dans une grille ; la première carte de la liste devient le modèle d'une boucle sur
/// le champ `key`.
fn gallery_rebuild(
    body: &mut [Node],
    imgs: &[Vec<usize>],
    siblings: &[Vec<usize>],
    order: &dyn Fn(&[usize]) -> usize,
    key: &str,
) -> bool {
    if imgs.len() < 2 {
        return false;
    }
    let all: Vec<Vec<usize>> = imgs.iter().chain(siblings).cloned().collect();
    let lca = common_prefix(&all);
    let Some(all_roots) = card_roots(&all, lca.len()) else {
        return false;
    };
    let roots: Vec<Vec<usize>> = all_roots[..imgs.len()].to_vec();
    let sib_roots: Vec<Vec<usize>> = all_roots[imgs.len()..].to_vec();
    let depth = roots[0].len();
    if roots.iter().chain(&sib_roots).any(|r| r.len() != depth) {
        return false;
    }
    let masonry = depth == lca.len() + 2;
    if !masonry && depth != lca.len() + 1 {
        return false;
    }
    let mut sorted = roots.clone();
    sorted.sort_by_key(|p| order(p));
    let first = sorted[0].clone();
    let sib_nodes: Vec<Node> = {
        let mut v: Vec<(Vec<usize>, Node)> = sib_roots
            .iter()
            .filter_map(|p| node_at(body, p).cloned().map(|n| (p.clone(), n)))
            .collect();
        // Ordre de lecture : colonne puis rang (l'aperçu vient en tête).
        v.sort_by(|a, b| {
            a.0[lca.len() + 1..]
                .cmp(&b.0[lca.len() + 1..])
                .then(a.0.cmp(&b.0))
        });
        v.into_iter().map(|(_, n)| n).collect()
    };
    let Some(mut template) = node_at(body, &first).cloned() else {
        return false;
    };
    walk_mut(std::slice::from_mut(&mut template), &mut |n| {
        if let Node::Element { tag, attrs, .. } = n
            && tag == "img"
        {
            attrs.retain(|a| a.name != "srcset" && a.name != "sizes");
            set_attr(attrs, "src", "{{ image }}".into());
        }
    });
    natural_ratio(std::slice::from_mut(&mut template));
    let Some(Node::Element {
        children, attrs, ..
    }) = node_at_mut(body, &lca)
    else {
        return false;
    };
    if masonry {
        let gap = children.iter().find_map(|c| match c {
            Node::Element { attrs, .. } => attr(attrs, "style").and_then(|s| {
                s.split(';').find_map(|d| {
                    d.trim()
                        .strip_prefix("row-gap:")
                        .map(|v| v.trim().to_string())
                })
            }),
            _ => None,
        });
        let st = attr(attrs, "style").unwrap_or_default().to_string();
        let mut st: Vec<String> = st
            .split(';')
            .filter(|d| !d.trim().is_empty() && !d.trim_start().starts_with("row-gap"))
            .map(str::to_string)
            .collect();
        if let Some(g) = gap {
            st.push(format!("row-gap:{g}"));
        }
        set_attr(attrs, "style", st.join(";"));
        set_attr(attrs, "data-revel-masonry", "order".into());
    }
    let mut kids = sib_nodes;
    kids.push(Node::Each {
        id: None,
        expr: key.to_string(),
        item: "image".into(),
        children: vec![template],
        empty: vec![],
    });
    if masonry {
        *children = kids;
    } else {
        let idx: HashSet<usize> = roots
            .iter()
            .chain(&sib_roots)
            .map(|p| p[lca.len()])
            .collect();
        let at = *idx.iter().min().expect("carte");
        let mut out = Vec::new();
        for (i, c) in std::mem::take(children).into_iter().enumerate() {
            if i == at {
                out.append(&mut kids);
            } else if !idx.contains(&i) {
                out.push(c);
            }
        }
        *children = out;
    }
    true
}

/// Contenus écrits : fichier (sans extension) → (valeurs, corps).
fn read_entries(dest: &Path, folder: &str) -> HashMap<String, (Map<String, Value>, String)> {
    let mut out = HashMap::new();
    let Ok(rd) = std::fs::read_dir(dest.join(folder)) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "md") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        let Some((data, body)) = parse_entry(&text) else {
            continue;
        };
        let Some(obj) = data.as_object() else {
            continue;
        };
        let slug = obj
            .get("slug")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| {
                p.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            });
        out.insert(slug, (obj.clone(), body));
    }
    out
}

/// En-tête YAML et corps d'un fichier de contenu (`---\n…\n---\ncorps`).
fn parse_entry(text: &str) -> Option<(Value, String)> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let data = revel_project::parse_yaml(&rest[..end]).ok()?;
    let body = rest[end + 4..].trim_start_matches(['\r', '\n']).to_string();
    Some((data, body))
}

/// Contenus d'une autre langue : champs de texte qui diffèrent, dans `content/<langue>/…`.
fn write_locale_entries(
    dest: &Path,
    folder: &str,
    c: &Read,
    main: &[Map<String, Value>],
    other: &[Map<String, Value>],
    code: &str,
    report: &mut Report,
) {
    let local = revel_model::locale_folder(folder, code);
    let dir = dest.join(&local);
    let mut n = 0;
    for (a, b) in main.iter().zip(other) {
        let Some(slug) = a.get("slug").and_then(Value::as_str) else {
            continue;
        };
        let mut data = Map::new();
        let mut body = String::new();
        for f in &c.fields {
            if !matches!(f.kind, Kind::PlainText | Kind::RichText | Kind::Link) || f.key == "slug" {
                continue;
            }
            let (Some(va), Some(vb)) = (a.get(&f.key), b.get(&f.key)) else {
                continue;
            };
            if va == vb {
                continue;
            }
            if f.kind == Kind::RichText {
                body = vb.as_str().unwrap_or("").to_string();
            } else {
                data.insert(f.key.clone(), vb.clone());
            }
        }
        if data.is_empty() && body.is_empty() {
            continue;
        }
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let file = revel_model::slugify(slug);
        let text = revel_project::entries::entry_text(&Value::Object(data), &body);
        if std::fs::write(dir.join(format!("{file}.md")), text).is_ok() {
            n += 1;
        }
    }
    if n > 0 {
        add_locale_to_pages_yml(dest, folder, &local, code);
        report.warnings.push(format!(
            "CMS « {} » : {n} contenu(s) traduit(s) ({})",
            c.label,
            code.to_uppercase()
        ));
    }
}

/// Entrée de Pages CMS de la langue (« Projects (FR) »), comme à la première traduction (D-35).
fn add_locale_to_pages_yml(dest: &Path, primary: &str, local: &str, code: &str) {
    let Ok(Some(text)) = revel_project::pagesyml::read(dest) else {
        return;
    };
    let Ok(doc) = revel_project::parse_yaml(&text) else {
        return;
    };
    let Some(list) = doc.get("content").and_then(Value::as_array) else {
        return;
    };
    let norm = |p: &str| p.trim_matches('/').to_string();
    let path_of = |x: &Value| x.get("path").and_then(Value::as_str).map(norm);
    if list.iter().any(|x| path_of(x) == Some(norm(local))) {
        return;
    }
    let Some(src) = list.iter().find(|x| path_of(x) == Some(norm(primary))) else {
        return;
    };
    let mut def = src.clone();
    let name = src
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("collection");
    let label = src.get("label").and_then(Value::as_str).unwrap_or(name);
    if let Some(o) = def.as_object_mut() {
        o.insert("name".into(), Value::String(format!("{name}-{code}")));
        o.insert(
            "label".into(),
            Value::String(format!("{label} ({})", code.to_uppercase())),
        );
        o.insert("path".into(), Value::String(local.to_string()));
    }
    if let Ok(new) = revel_project::pagesyml::add_collection(&text, &def) {
        let _ = revel_project::pagesyml::write(dest, Some(&new));
    }
}

/// Textes de deux rendus de même structure : texte de la langue principale → traduction.
fn align(a: &[Node], b: &[Node], out: &mut BTreeMap<String, String>) {
    if a.len() != b.len() {
        return;
    }
    for (x, y) in a.iter().zip(b) {
        match (x, y) {
            (Node::Text { text: t1 }, Node::Text { text: t2 }) => {
                let (n1, n2) = (norm(t1), norm(t2));
                if !n1.is_empty() && !n2.is_empty() && n1 != n2 {
                    out.entry(n1).or_insert(n2);
                }
            }
            (
                Node::Element {
                    tag: g1,
                    attrs: a1,
                    children: c1,
                    ..
                },
                Node::Element {
                    tag: g2,
                    attrs: a2,
                    children: c2,
                    ..
                },
            ) if g1 == g2 => {
                for name in ["alt", "title", "aria-label", "placeholder"] {
                    if let (Some(v1), Some(v2)) = (attr(a1, name), attr(a2, name)) {
                        let (n1, n2) = (norm(v1), norm(v2));
                        if !n1.is_empty() && !n2.is_empty() && n1 != n2 {
                            out.entry(n1).or_insert(n2);
                        }
                    }
                }
                if same_shape(c1, c2)
                    || !inline_only(c1)
                    || !inline_only(c2)
                    || !align_pieces(c1, c2, out)
                {
                    align(c1, c2, out);
                }
            }
            _ => {}
        }
    }
}

/// Mêmes nœuds aux mêmes places (textes et balises), à la traduction près.
fn same_shape(a: &[Node], b: &[Node]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| match (x, y) {
            (Node::Text { .. }, Node::Text { .. }) => true,
            (
                Node::Element {
                    tag: g1,
                    children: c1,
                    ..
                },
                Node::Element {
                    tag: g2,
                    children: c2,
                    ..
                },
            ) => g1 == g2 && same_shape(c1, c2),
            _ => false,
        })
}

/// Texte d'un paragraphe : morceaux de texte et balises de mise en forme seulement.
fn inline_only(nodes: &[Node]) -> bool {
    nodes.iter().all(|n| match n {
        Node::Text { .. } => true,
        Node::Element { tag, children, .. } => {
            matches!(
                tag.as_str(),
                "span" | "strong" | "em" | "b" | "i" | "u" | "br"
            ) && inline_only(children)
        }
        _ => false,
    })
}

/// Paragraphe mis en forme autrement dans l'autre langue (couleurs par morceaux d'un côté, un
/// seul texte de l'autre) : chaque morceau est traduit par la partie de même place, repérée
/// par les tirets et deux-points qui bordent les morceaux, quand les deux textes en ont autant.
fn align_pieces(a: &[Node], b: &[Node], out: &mut BTreeMap<String, String>) -> bool {
    fn pieces(nodes: &[Node], out: &mut Vec<String>) {
        for n in nodes {
            match n {
                Node::Text { text } => out.push(text.clone()),
                Node::Element { children, .. } => pieces(children, out),
                _ => {}
            }
        }
    }
    const MARKS: [char; 4] = ['—', '–', ':', '|'];
    let (mut pa, mut pb) = (Vec::new(), Vec::new());
    pieces(a, &mut pa);
    pieces(b, &mut pb);
    let whole_b: String = pb.concat();
    let marks_a: usize = pa.iter().map(|p| p.matches(MARKS).count()).sum();
    let at_b: Vec<usize> = whole_b
        .match_indices(MARKS)
        .map(|(i, m)| i + m.len())
        .collect();
    let starts_b: Vec<usize> = whole_b.match_indices(MARKS).map(|(i, _)| i).collect();
    if marks_a == 0 || marks_a != at_b.len() {
        return false;
    }
    // Bord de chaque morceau : juste après (ou juste avant) la k-ième marque.
    let mut seen = 0;
    let mut cuts = vec![0usize];
    for (i, p) in pa.iter().enumerate() {
        let n = p.matches(MARKS).count();
        let t = p.trim();
        if i + 1 < pa.len() {
            let next = pa[i + 1..].iter().find(|q| !q.trim().is_empty());
            let cut = if n > 0 && t.ends_with(MARKS) {
                Some(at_b[seen + n - 1])
            } else if next.is_some_and(|q| q.trim_start().starts_with(MARKS)) {
                Some(starts_b[seen + n])
            } else if t.is_empty() {
                cuts.last().copied()
            } else {
                None
            };
            match cut {
                Some(c) => cuts.push(c),
                None => return false,
            }
        }
        seen += n;
    }
    cuts.push(whole_b.len());
    for (i, p) in pa.iter().enumerate() {
        let (n1, n2) = (norm(p), norm(&whole_b[cuts[i]..cuts[i + 1]]));
        if !n1.is_empty() && !n2.is_empty() && n1 != n2 {
            out.entry(n1).or_insert(n2);
        }
    }
    true
}

/// Traductions ajoutées à la langue `code` (créée au besoin) ; textes des contenus exclus (ils
/// se traduisent dans les contenus de la langue).
fn add_texts(site: &mut Site, code: &str, tr: BTreeMap<String, String>, probe: &Probe) {
    let skip: HashSet<&str> = probe.texts.iter().map(|(t, _, _)| t.as_str()).collect();
    let l = match site.locales.iter().position(|l| l.code == code) {
        Some(i) => &mut site.locales[i],
        None => {
            site.locales.push(Locale::new(code));
            site.locales.last_mut().expect("langue")
        }
    };
    for (k, v) in tr {
        if !skip.contains(k.as_str()) {
            l.texts.entry(k).or_insert(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el(tag: &str, attrs: &[(&str, &str)], children: Vec<Node>) -> Node {
        Node::Element {
            id: None,
            tag: tag.into(),
            attrs: attrs
                .iter()
                .map(|(n, v)| Attr {
                    name: (*n).into(),
                    value: Some((*v).into()),
                    when: None,
                })
                .collect(),
            children,
        }
    }
    fn txt(t: &str) -> Node {
        Node::Text { text: t.into() }
    }
    fn card(slug: &str, title: &str) -> Node {
        el(
            "div",
            &[("class", "framer-c-container")],
            vec![el(
                "a",
                &[("href", &format!("./projects/{slug}"))],
                vec![el("h3", &[], vec![txt(title)])],
            )],
        )
    }

    #[test]
    fn page_apparition_et_barre() {
        let mut m = Modules::default();
        m.read(
            r#"U={opacity:0,y:40},W={bounce:.1,delay:.1,duration:1.5,type:`spring`};o(R,{__framer__animate:{transition:W},__framer__animateOnce:!0,__framer__enter:U,__framer__styleAppearEffectEnabled:!0,className:`framer-dhvub1`,children:[]});function S(){getComputedStyle(e).overflowY;e.scrollHeight-e.clientHeight}"#,
            "a.mjs",
        );
        let mut head = vec![el(
            "style",
            &[],
            vec![txt(".framer-a.framer-v-b.hover .framer-c{padding:0}")],
        )];
        let mut body = vec![
            el(
                "div",
                &[
                    ("class", "framer-dhvub1"),
                    ("style", "opacity:0;transform:none"),
                ],
                vec![],
            ),
            el(
                "div",
                &[
                    ("aria-hidden", "true"),
                    ("style", "width:100%;height:2px;overflow:hidden"),
                ],
                vec![el(
                    "div",
                    &[("style", "width:0%;height:100%;transition:width 0.45s ease")],
                    vec![],
                )],
            ),
        ];
        let mut css = String::new();
        assert_eq!(apply_page(&mut head, &mut body, &m, &mut css), 1);
        assert!(css.is_empty());
        let Node::Element { children, .. } = &head[0] else {
            panic!()
        };
        assert_eq!(
            children[0],
            txt(".framer-a.framer-v-b:hover .framer-c{padding:0}")
        );
        let Node::Element { attrs, .. } = &body[0] else {
            panic!()
        };
        assert_eq!(
            attr(attrs, "data-revel-appear"),
            Some("up dur=1.5 delay=0.1 dist=40")
        );
        assert!(attr(attrs, "style").is_none_or(|s| !s.contains("opacity:0")));
        let Node::Element {
            attrs, children, ..
        } = &body[1]
        else {
            panic!()
        };
        assert_eq!(attr(attrs, "data-revel-progress"), Some("parent"));
        let Node::Element { attrs: fill, .. } = &children[0] else {
            panic!()
        };
        assert!(
            attr(fill, "style")
                .unwrap()
                .contains("width:calc(var(--revel-progress, 0) * 100%)")
        );
    }

    #[test]
    fn texte_riche_du_cms() {
        let mut sheet = revel_model::Stylesheet::default();
        sheet
            .items
            .push(revel_model::StyleItem::Rule(revel_model::StyleRule {
            selector:
                "p.framer-text,h3.framer-text,span.framer-text:not([data-text-fill]),.framer-a p"
                    .into(),
            ..Default::default()
        }));
        assert_eq!(rich_text_rules(&mut sheet), 1);
        assert_eq!(rich_text_rules(&mut sheet), 0);
        let revel_model::StyleItem::Rule(r) = &sheet.items[0] else {
            panic!()
        };
        assert!(r.selector.ends_with(".rich-text-wrapper > p,.rich-text-wrapper > h3,.rich-text-wrapper > span:not([data-text-fill])"), "{}", r.selector);
        let probe = Probe {
            rich: Some(("Un. Deux".into(), "content | safe".into(), "content".into())),
            ..Default::default()
        };
        let mut body = vec![el(
            "div",
            &[("class", "framer-dfoox8")],
            vec![
                el(
                    "p",
                    &[("class", "framer-text framer-styles-preset-p1")],
                    vec![txt("Un.")],
                ),
                el(
                    "h3",
                    &[("class", "framer-text framer-styles-preset-h3")],
                    vec![txt("Deux")],
                ),
            ],
        )];
        bind(&mut body, &probe, None);
        let Node::Element {
            attrs, children, ..
        } = &body[0]
        else {
            panic!()
        };
        assert_eq!(
            attr(attrs, "class"),
            Some("framer-dfoox8 rich-text-wrapper framer-styles-preset-p1 framer-styles-preset-h3")
        );
        assert_eq!(
            children,
            &vec![Node::Out {
                expr: "content | safe".into()
            }]
        );
    }

    #[test]
    fn navigation_qui_suit_la_section() {
        let mut m = Modules::default();
        // Utilisation (module de la page) puis composant (autre module).
        m.read(
            r#"a(Hi,{__framer__animateOnce:!1,__framer__targets:[{offset:40,ref:j,target:`y0KXVwxnJ`},{offset:40,ref:M,target:`JA4JVtwmY`}],__framer__threshold:0,__framer__variantAppearEffectEnabled:!0,variant:Q(`y0KXVwxnJ`)})"#,
            "page.mjs",
        );
        m.read(
            r#"Pe={JA4JVtwmY:`framer-v-rqk8pu`,y0KXVwxnJ:`framer-v-1gebcye`},Z={bounce:.1,delay:.1,duration:1.5,type:`spring`};u(se,{href:v,children:u(x.a,{className:`framer-1an41qh framer-1vzucgo`,style:{backgroundColor:`rgb(18, 18, 18)`},variants:{JA4JVtwmY:{backgroundColor:`rgba(0, 0, 0, 0)`}},children:u(ee,{className:`framer-worfxv`,style:{opacity:1},variants:{JA4JVtwmY:{opacity:.65}}})})})"#,
            "nav.mjs",
        );
        m.finish();
        assert_eq!(m.spy.len(), 1, "{:?}", m.spy);
        let mut body = vec![el(
            "div",
            &[("class", "framer-LmkBZ framer-v-1gebcye")],
            vec![
                el(
                    "a",
                    &[("class", "framer-1an41qh"), ("href", "./#intro")],
                    vec![txt("About")],
                ),
                el(
                    "a",
                    &[("class", "framer-11nwwps"), ("href", "./#work")],
                    vec![txt("Work")],
                ),
            ],
        )];
        let mut css = String::new();
        apply_page(&mut [], &mut body, &m, &mut css);
        let Node::Element { attrs, .. } = &body[0] else {
            panic!()
        };
        assert_eq!(attr(attrs, "data-revel-spy"), Some("section"));
        let mut body = vec![el("video", &[("src", "a.mp4"), ("autoplay", "")], vec![])];
        apply_page(&mut [], &mut body, &m, &mut css);
        let Node::Element { attrs, .. } = &body[0] else {
            panic!()
        };
        assert_eq!(attr(attrs, "muted"), Some(""));
        assert!(
            css.contains(".framer-v-1gebcye[data-revel-section=\"work\"] .framer-1an41qh{background-color:rgba(0, 0, 0, 0)!important}"),
            "{css}"
        );
        assert!(css.contains(".framer-v-1gebcye[data-revel-section=\"work\"] .framer-worfxv{opacity:0.65!important}"), "{css}");
        assert!(
            css.contains(".framer-v-1gebcye .framer-1an41qh{transition:background-color 1.5s"),
            "{css}"
        );
    }

    #[test]
    fn liste_en_maconnerie_et_copies() {
        // Deux rendus (points de rupture) : deux colonnes (1, 3 | 2), puis une colonne.
        let masonry = el(
            "div",
            &[("class", "grille"), ("style", "display:grid")],
            vec![
                el(
                    "div",
                    &[("style", "display:flex;row-gap:8px")],
                    vec![card("a", "A"), card("c", "C")],
                ),
                el(
                    "div",
                    &[("style", "display:flex;row-gap:8px")],
                    vec![card("b", "B")],
                ),
            ],
        );
        let single = el(
            "div",
            &[("class", "pile")],
            vec![card("a", "A"), card("b", "B"), card("c", "C")],
        );
        let mut body = vec![el(
            "main",
            &[],
            vec![
                el("div", &[("class", "ssr-variant hidden-tel")], vec![masonry]),
                el(
                    "div",
                    &[("class", "ssr-variant hidden-bureau")],
                    vec![single],
                ),
            ],
        )];
        let order = |s: &str| {
            ["a", "b", "c"]
                .iter()
                .position(|x| s.contains(&format!("/{x}")))
                .unwrap_or(9)
        };
        let links = paths_of(
            &body,
            &|n| matches!(n, Node::Element { tag, .. } if tag == "a"),
        );
        let ro = body.clone();
        let key = |p: &[usize]| match node_at(&ro, p) {
            Some(Node::Element { attrs, .. }) => normalize_url(attr(attrs, "href").unwrap_or("")),
            _ => String::new(),
        };
        let mut gs = groups(links, &key);
        assert_eq!(gs.len(), 2);
        gs.sort_by_key(|g| std::cmp::Reverse(common_prefix(g)));
        let probe = Probe {
            texts: vec![("A".into(), "item.data.name".into(), "name".into())],
            ..Default::default()
        };
        for g in gs {
            let ord = |p: &[usize]| {
                g.iter()
                    .filter(|q| q.starts_with(p))
                    .map(|q| order(&key(q)))
                    .min()
                    .unwrap_or(9)
            };
            let each = |t: Node| {
                let mut t = vec![t];
                bind(&mut t, &probe, Some(("/projects/a/", "item.url")));
                Node::Each {
                    id: None,
                    expr: "collections.projects".into(),
                    item: "item".into(),
                    children: t,
                    empty: vec![],
                }
            };
            assert!(rebuild_group(&mut body, &g, &ord, &each));
        }
        let out = serde_json::to_string(&body).unwrap();
        assert_eq!(out.matches("\"collections.projects\"").count(), 2, "{out}");
        assert!(
            out.contains("data-revel-masonry") && out.contains("row-gap:8px"),
            "{out}"
        );
        assert!(
            out.contains("{{ item.url }}") && out.contains("item.data.name"),
            "{out}"
        );
        assert!(!out.contains("./projects/c"), "{out}");
    }

    #[test]
    fn champs_facultatifs_et_traductions() {
        let mut body = vec![el(
            "div",
            &[],
            vec![
                el(
                    "p",
                    &[],
                    vec![Node::Out {
                        expr: "year".into(),
                    }],
                ),
                el(
                    "div",
                    &[("class", "framer-b-container")],
                    vec![el(
                        "a",
                        &[("href", "{{ button_link }}")],
                        vec![Node::Out {
                            expr: "button_text".into(),
                        }],
                    )],
                ),
            ],
        )];
        let filled: HashMap<String, Vec<bool>> = [
            ("button_link".to_string(), vec![true, false]),
            ("button_text".to_string(), vec![true, false]),
        ]
        .into();
        wrap_optional(
            &mut body,
            &["button_link".into(), "button_text".into()],
            "",
            &filled,
        );
        let out = serde_json::to_string(&body).unwrap();
        assert!(
            out.contains("\"cond\":\"button_link and button_text\""),
            "{out}"
        );
        assert!(!out.contains("\"cond\":\"year\""), "{out}");
        // Deux vidéos dont la seconde est souvent vide : chacune sa condition.
        let video = |k: &str| {
            el(
                "div",
                &[],
                vec![el("video", &[("src", &format!("{{{{ {k} }}}}"))], vec![])],
            )
        };
        let mut body = vec![el(
            "div",
            &[],
            vec![video("video_url"), video("video_url_2")],
        )];
        let filled: HashMap<String, Vec<bool>> = [
            ("video_url".to_string(), vec![true, true]),
            ("video_url_2".to_string(), vec![true, false]),
        ]
        .into();
        wrap_optional(
            &mut body,
            &["video_url".into(), "video_url_2".into()],
            "",
            &filled,
        );
        let out = serde_json::to_string(&body).unwrap();
        assert!(
            out.contains("\"cond\":\"video_url\"") && out.contains("\"cond\":\"video_url_2\""),
            "{out}"
        );
        assert!(
            mentions("item.data.video_url", "item.data.video_url")
                && !mentions("{{ video_url_2 }}", "video_url")
        );
        let mut tr = BTreeMap::new();
        align(
            &[el("h2", &[("alt", "Me")], vec![txt("About me.")])],
            &[el("h2", &[("alt", "Moi")], vec![txt(" À propos. ")])],
            &mut tr,
        );
        assert_eq!(tr.get("About me.").map(String::as_str), Some("À propos."));
        assert_eq!(tr.get("Me").map(String::as_str), Some("Moi"));
        let grey = |t: &str| el("span", &[("style", "color:grey")], vec![txt(t)]);
        let mut tr = BTreeMap::new();
        align(
            &[el(
                "p",
                &[],
                vec![
                    txt("I solve problems — "),
                    grey("from 3D to web "),
                    txt("—"),
                    grey(" "),
                    txt("focused."),
                ],
            )],
            &[el(
                "p",
                &[],
                vec![txt("Je résous — de la 3D au web — concentré.")],
            )],
            &mut tr,
        );
        assert_eq!(
            tr.get("I solve problems —").map(String::as_str),
            Some("Je résous —")
        );
        assert_eq!(
            tr.get("from 3D to web").map(String::as_str),
            Some("de la 3D au web")
        );
        assert_eq!(tr.get("focused.").map(String::as_str), Some("concentré."));
        assert!(!tr.contains_key("—"));
    }
}
