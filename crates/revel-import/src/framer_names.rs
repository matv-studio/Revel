//! Classes de Framer renommées d'après les calques (D-112).
//!
//! Framer nomme ses classes par empreinte (`framer-1p11shv`) ; le panneau Styles les montrerait
//! telles quelles. Chaque classe prend le nom du calque de l'élément qui la porte
//! (`data-framer-name` « Intro » → `intro`), précédé de celui du calque parent pour les noms
//! génériques (« Title » dans « About » → `about-title`), ou son rôle quand le calque n'a pas de
//! nom (texte, image). Les sélecteurs de la feuille du site, les classes des pages, gabarits et
//! composants et les cibles d'interaction suivent ; le site produit reste le même.

use crate::Report;
use revel_model::{Node, Site, StyleItem};
use std::collections::{HashMap, HashSet};

/// Noms de calque trop généraux seuls : précédés du nom du calque parent.
const GENERIC: [&str; 16] = [
    "content",
    "container",
    "title",
    "text",
    "image",
    "name",
    "icon",
    "line",
    "info",
    "details",
    "bg",
    "description",
    "variant-1",
    "default",
    "wrapper",
    "stack",
];

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("n-{out}")
    } else {
        out
    }
}

/// Classe d'empreinte de Framer (`framer-1p11shv`, `framer-1p11shv-container`).
fn hashed(c: &str) -> bool {
    let Some(rest) = c.strip_prefix("framer-") else {
        return false;
    };
    let h = rest.strip_suffix("-container").unwrap_or(rest);
    (5..=8).contains(&h.len())
        && h.chars()
            .all(|x| x.is_ascii_lowercase() || x.is_ascii_digit())
        && !matches!(
            h,
            "text" | "image" | "page" | "body" | "root" | "badge" | "cursor" | "mask"
        )
}

fn attr<'a>(attrs: &'a [revel_model::Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .and_then(|a| a.value.as_deref())
}

/// Nom proposé pour la classe d'un élément.
fn proposal(
    attrs: &[revel_model::Attr],
    children: &[Node],
    class: &str,
    parent: Option<&str>,
) -> String {
    let own = attr(attrs, "data-framer-name")
        .map(slug)
        .filter(|s| !s.is_empty());
    let base = own.unwrap_or_else(|| {
        if attr(attrs, "data-framer-component-type") == Some("RichTextContainer") {
            "text".into()
        } else if children.iter().any(|c| matches!(c, Node::Element { attrs, .. } if attr(attrs, "data-framer-background-image-wrapper").is_some())) {
            "image".into()
        } else {
            "box".into()
        }
    });
    let base = if class.ends_with("-container") && !base.ends_with("container") {
        format!("{base}-container")
    } else {
        base
    };
    match parent {
        Some(p)
            if GENERIC.contains(&base.as_str())
                || base == "box"
                || base.ends_with("-container") =>
        {
            format!("{p}-{base}")
        }
        _ => base,
    }
}

fn collect(
    nodes: &[Node],
    parent: Option<&str>,
    taken: &mut HashSet<String>,
    map: &mut HashMap<String, String>,
) {
    for n in nodes {
        match n {
            Node::Element {
                attrs, children, ..
            } => {
                let classes: Vec<&str> = attr(attrs, "class")
                    .unwrap_or("")
                    .split_whitespace()
                    .collect();
                let mut mine: Option<String> = None;
                for c in classes.iter().filter(|c| hashed(c)) {
                    if let Some(done) = map.get(*c) {
                        mine.get_or_insert(done.clone());
                        continue;
                    }
                    let want = proposal(attrs, children, c, parent);
                    let mut name = want.clone();
                    let mut k = 2;
                    while taken.contains(&name) {
                        name = format!("{want}-{k}");
                        k += 1;
                    }
                    taken.insert(name.clone());
                    map.insert(c.to_string(), name.clone());
                    mine.get_or_insert(name);
                }
                // Parent des enfants : nom du calque de cet élément, sinon celui reçu.
                let layer = attr(attrs, "data-framer-name")
                    .map(slug)
                    .filter(|s| !s.is_empty() && !GENERIC.contains(&s.as_str()));
                let next = layer.or(mine
                    .filter(|m| !m.ends_with("-container"))
                    .map(|m| m.to_string()));
                collect(children, next.as_deref().or(parent), taken, map);
            }
            Node::Each { children, .. } => collect(children, parent, taken, map),
            Node::If {
                then, otherwise, ..
            } => {
                collect(then, parent, taken, map);
                collect(otherwise, parent, taken, map);
            }
            _ => {}
        }
    }
}

/// Renomme les classes d'empreinte de Framer ; renvoie le nombre de classes renommées.
pub fn readable_classes(site: &mut Site, report: &mut Report) -> usize {
    let mut taken: HashSet<String> = HashSet::new();
    // Classes déjà présentes (autres que celles de Framer) : jamais reprises.
    let class_re = regex::Regex::new(r"\.([A-Za-z_][\w-]*)").expect("regex");
    if let Some(sheet) = &site.stylesheet {
        for item in &sheet.items {
            if let StyleItem::Rule(r) = item {
                for c in class_re.captures_iter(&r.selector) {
                    if !hashed(&c[1]) {
                        taken.insert(c[1].to_string());
                    }
                }
            }
        }
    }
    let mut map = HashMap::new();
    for p in &site.pages {
        collect(&p.body, None, &mut taken, &mut map);
    }
    for l in &site.layouts {
        collect(&l.body, None, &mut taken, &mut map);
    }
    for c in &site.components {
        collect(&c.body, None, &mut taken, &mut map);
    }
    if map.is_empty() {
        return 0;
    }
    // `.framer-x` dans les sélecteurs, ` framer-x` dans les classes : mot entier.
    let token =
        regex::Regex::new(r"(^|[\s.:,>+~(])(framer-[a-z0-9]+(?:-container)?)\b").expect("regex");
    let fixed = |s: &str| -> String {
        // `\b` s'arrête au tiret : vérifier que la classe ne continue pas (`framer-abc-def`).
        let mut out = String::new();
        let mut last = 0;
        for c in token.captures_iter(s) {
            let m = c.get(2).expect("m");
            let next = s[m.end()..].chars().next();
            if next.is_some_and(|x| x == '-' || x == '_' || x.is_ascii_alphanumeric()) {
                continue;
            }
            if let Some(new) = map.get(m.as_str()) {
                out.push_str(&s[last..m.start()]);
                out.push_str(new);
                last = m.end();
            }
        }
        out.push_str(&s[last..]);
        out
    };
    fn walk(nodes: &mut [Node], f: &dyn Fn(&str) -> String) {
        for n in nodes {
            match n {
                Node::Element {
                    attrs, children, ..
                } => {
                    for a in attrs.iter_mut() {
                        if (a.name == "class" || a.name == "data-revel-on")
                            && let Some(v) = &a.value
                        {
                            a.value = Some(f(v));
                        }
                    }
                    walk(children, f);
                }
                Node::Use { attrs, .. } => {
                    for a in attrs.iter_mut() {
                        if a.name == "class"
                            && let Some(v) = &a.value
                        {
                            a.value = Some(f(v));
                        }
                    }
                }
                Node::Each { children, .. } => walk(children, f),
                Node::If {
                    then, otherwise, ..
                } => {
                    walk(then, f);
                    walk(otherwise, f);
                }
                _ => {}
            }
        }
    }
    for p in &mut site.pages {
        walk(&mut p.body, &fixed);
    }
    for l in &mut site.layouts {
        walk(&mut l.body, &fixed);
    }
    for c in &mut site.components {
        walk(&mut c.body, &fixed);
    }
    if let Some(sheet) = site.stylesheet.as_mut() {
        for item in &mut sheet.items {
            match item {
                StyleItem::Rule(r) => r.selector = fixed(&r.selector),
                StyleItem::Raw { css } => *css = fixed(css),
                StyleItem::Comment { .. } => {}
            }
        }
    }
    report.warnings.push(format!(
        "Framer : {} classes renommées d'après les calques (« intro », « about-title »…)",
        map.len()
    ));
    map.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use revel_model::{Attr, StyleRule};

    fn el(tag: &str, attrs: &[(&str, &str)], children: Vec<Node>) -> Node {
        Node::Element {
            id: None,
            tag: tag.into(),
            attrs: attrs
                .iter()
                .map(|(k, v)| Attr {
                    name: (*k).into(),
                    value: Some((*v).into()),
                    when: None,
                })
                .collect(),
            children,
        }
    }

    #[test]
    fn classes_lisibles() {
        let mut site = Site::new("t", "fr");
        let mut page: revel_model::Page =
            serde_json::from_value(serde_json::json!({"id": "index", "path": "/", "body": []}))
                .unwrap();
        page.body = vec![el(
            "div",
            &[("class", "framer-1p11shv"), ("data-framer-name", "About")],
            vec![
                el(
                    "div",
                    &[("class", "framer-udddqf"), ("data-framer-name", "Title")],
                    vec![],
                ),
                el(
                    "div",
                    &[
                        ("class", "framer-38hsop framer-text"),
                        ("data-framer-component-type", "RichTextContainer"),
                    ],
                    vec![],
                ),
                el(
                    "a",
                    &[
                        ("class", "framer-1an41qh"),
                        (
                            "data-revel-on",
                            "click class:is-active closest:.framer-1p11shv",
                        ),
                    ],
                    vec![],
                ),
            ],
        )];
        site.pages.push(page);
        site.stylesheet = Some(revel_model::Stylesheet {
            url: "/s.css".into(),
            source: None,
            items: vec![
                StyleItem::Rule(StyleRule {
                    selector: ".framer-1p11shv > .framer-udddqf, .framer-1p11shv-x".into(),
                    ..Default::default()
                }),
                StyleItem::Raw {
                    css: "@media (max-width:800px){.framer-38hsop{color:red}}".into(),
                },
            ],
        });
        let mut report = Report::default();
        assert_eq!(readable_classes(&mut site, &mut report), 4);
        let html = serde_json::to_string(&site.pages[0].body).unwrap();
        assert!(
            html.contains("\"about\"")
                && html.contains("\"about-title\"")
                && html.contains("\"about-text framer-text\""),
            "{html}"
        );
        assert!(html.contains("closest:.about"), "{html}");
        let StyleItem::Rule(r) = &site.stylesheet.as_ref().unwrap().items[0] else {
            panic!()
        };
        assert_eq!(r.selector, ".about > .about-title, .framer-1p11shv-x");
        let StyleItem::Raw { css } = &site.stylesheet.as_ref().unwrap().items[1] else {
            panic!()
        };
        assert!(css.contains(".about-text{"), "{css}");
    }
}
