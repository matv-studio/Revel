//! Styles d'un site Framer rapprochés des outils de Revel (D-155), sans changer le rendu :
//!
//! - couleurs de Framer (`var(--token-…, #hex)`) → couleurs du site (`--couleur-1`…, Paramètres ›
//!   Couleurs) ;
//! - variables de Framer jamais déclarées (`--framer-aspect-ratio-supported`,
//!   `--framer-will-change-override`…) → leur valeur de repli ;
//! - propriétés sans effet visible retirées ;
//! - remplissage des images de fond (style écrit sur l'enveloppe et l'image) → classes partagées.
//!
//! Le reste des styles de fonctionnement de Framer (texte, composants) demande les styles calculés
//! de chaque élément : voir D-155.

use crate::Report;
use revel_model::{Attr, Decl, Node, Site, StyleItem, StyleRule};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Propriétés d'un style écrit sans effet visible ou gérées par un outil de Revel : retirées.
const DROPPED: &[&str] = &[
    "corner-shape",
    "-webkit-touch-callout",
    "-webkit-user-select",
    "user-select",
    "touch-action",
    "will-change",
    "-webkit-font-smoothing",
    "-moz-osx-font-smoothing",
    "--font-selector",
    "-webkit-backdrop-filter",
    "-webkit-mask",
    "-webkit-mask-image",
];

/// `a:b;c:d` → déclarations (parenthèses et guillemets respectés).
fn parse_style(v: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let (mut depth, mut cur, mut q) = (0i32, String::new(), None::<char>);
    let flush = |cur: &mut String, out: &mut Vec<(String, String)>| {
        if let Some((k, v)) = cur.split_once(':') {
            let (k, v) = (k.trim(), v.trim());
            if !k.is_empty() {
                out.push((k.to_string(), v.to_string()));
            }
        }
        cur.clear();
    };
    for c in v.chars() {
        match (q, c) {
            (Some(x), _) if c == x => q = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => q = Some(c),
            (None, '(') => depth += 1,
            (None, ')') => depth -= 1,
            (None, ';') if depth == 0 => {
                flush(&mut cur, &mut out);
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    flush(&mut cur, &mut out);
    out
}

/// Remplace chaque `var(--x[, repli])` : `keep(x)` vrai → gardé (sans repli) ; sinon valeur de
/// `lookup(x)`, sinon le repli (lui-même résolu), sinon rien.
fn resolve(
    v: &str,
    keep: &dyn Fn(&str) -> bool,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> String {
    let mut out = String::new();
    let mut rest = v;
    while let Some(i) = rest.find("var(") {
        out.push_str(&rest[..i]);
        let body_start = i + 4;
        let mut depth = 1i32;
        let mut j = body_start;
        let b = rest.as_bytes();
        while j < b.len() && depth > 0 {
            match b[j] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            j += 1;
        }
        let body = &rest[body_start..j.saturating_sub(1)];
        let (name, fallback) = match split_first_comma(body) {
            Some((n, f)) => (n.trim(), Some(f.trim())),
            None => (body.trim(), None),
        };
        let name = name.trim_start_matches("--");
        if keep(name) {
            out.push_str(&format!("var(--{name})"));
        } else if let Some(v) = lookup(name) {
            out.push_str(&resolve(&v, keep, lookup));
        } else if let Some(f) = fallback {
            out.push_str(&resolve(f, keep, lookup));
        }
        rest = &rest[j.min(rest.len())..];
    }
    out.push_str(rest);
    out.trim().to_string()
}

fn split_first_comma(s: &str) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => return Some((&s[..i], &s[i + 1..])),
            _ => {}
        }
    }
    None
}

fn walk_mut(nodes: &mut [Node], f: &mut dyn FnMut(&mut Node)) {
    for n in nodes.iter_mut() {
        f(n);
        for list in revel_model::ids::child_lists_mut(n) {
            walk_mut(list, f);
        }
    }
}

fn attr<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .and_then(|a| a.value.as_deref())
}

fn classes(attrs: &[Attr]) -> Vec<String> {
    attr(attrs, "class")
        .filter(|v| !v.contains("{{") && !v.contains("{%"))
        .map(|v| v.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

/// Résultat du nettoyage (comptes pour le bilan d'import).
#[derive(Debug, Default)]
pub struct Cleaned {
    pub colors: usize,
    pub inline: usize,
    pub presets: usize,
    pub removed: usize,
}

/// Noms des variables déclarées quelque part (règle ou style écrit d'un élément).
fn declared_vars(site: &Site) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Some(sh) = &site.stylesheet {
        for item in &sh.items {
            if let StyleItem::Rule(r) = item {
                for d in &r.decls {
                    if let Some(n) = d.prop.strip_prefix("--") {
                        out.insert(n.to_string());
                    }
                }
            }
        }
    }
    let mut note = |nodes: &Vec<Node>| {
        revel_model::ids::walk(nodes, &mut |n| {
            if let Node::Element { attrs, .. } = n
                && let Some(s) = attr(attrs, "style")
            {
                for (k, _) in parse_style(s) {
                    if let Some(n) = k.strip_prefix("--") {
                        out.insert(n.to_string());
                    }
                }
            }
        });
    };
    for p in &site.pages {
        note(&p.body);
    }
    for l in &site.layouts {
        note(&l.body);
    }
    for c in &site.components {
        note(&c.body);
    }
    out
}

/// Nettoie les styles d'un site importé de Framer (voir l'en-tête du module). Seules les
/// transformations qui gardent le même rendu sont faites : couleurs du site, variables jamais
/// déclarées remplacées par leur valeur de repli, propriétés sans effet retirées, remplissage des
/// images de fond en classes.
pub fn clean(site: &mut Site, report: &mut Report) -> Cleaned {
    let mut done = Cleaned::default();
    if site.stylesheet.is_none() {
        return done;
    }
    // 1. Couleurs de Framer (`var(--token-<id>, <repli>)`) → couleurs du site.
    let token_re = regex::Regex::new(r"var\(--token-([0-9a-f-]{36})\s*,\s*").expect("regex");
    let mut tokens: BTreeMap<String, String> = BTreeMap::new();
    let note_tokens = |text: &str, tokens: &mut BTreeMap<String, String>| {
        for m in token_re.captures_iter(text) {
            let start = m.get(0).expect("m").end();
            let mut depth = 1i32;
            let b = text.as_bytes();
            let mut j = start;
            while j < b.len() && depth > 0 {
                match b[j] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            let fb = text[start..j.saturating_sub(1)].trim().to_string();
            tokens.entry(m[1].to_string()).or_insert(fb);
        }
    };
    if let Some(sh) = &site.stylesheet {
        for item in &sh.items {
            if let StyleItem::Rule(r) = item {
                for d in &r.decls {
                    note_tokens(&d.value, &mut tokens);
                }
            }
        }
    }
    for body in site
        .pages
        .iter()
        .map(|p| &p.body)
        .chain(site.layouts.iter().map(|l| &l.body))
        .chain(site.components.iter().map(|c| &c.body))
    {
        revel_model::ids::walk(body, &mut |n| {
            if let Node::Element { attrs, .. } = n
                && let Some(s) = attr(attrs, "style")
            {
                note_tokens(s, &mut tokens);
            }
        });
    }
    // Thème de Framer : jetons écrits sur `body` (ou `:root`), valeur sombre dans
    // `prefers-color-scheme: dark` ; ils deviennent des couleurs du site.
    let mut theme: BTreeMap<String, String> = BTreeMap::new();
    let mut theme_dark: BTreeMap<String, String> = BTreeMap::new();
    if let Some(sh) = &site.stylesheet {
        for item in &sh.items {
            if let StyleItem::Rule(r) = item
                && matches!(r.selector.trim(), "body" | ":root" | "html")
            {
                let dark = r.media.as_deref().is_some_and(|m| {
                    m.contains("prefers-color-scheme: dark")
                        || m.contains("prefers-color-scheme:dark")
                });
                if r.media.is_some() && !dark {
                    continue;
                }
                for d in &r.decls {
                    if let Some(id) = d.prop.strip_prefix("--token-") {
                        let target = if dark { &mut theme_dark } else { &mut theme };
                        target.insert(id.to_string(), d.value.trim().to_string());
                        tokens
                            .entry(id.to_string())
                            .or_insert_with(|| d.value.trim().to_string());
                    }
                }
            }
        }
    }
    let declared = declared_vars(site);
    let mut names: HashMap<String, String> = HashMap::new();
    let mut root_decls: Vec<Decl> = Vec::new();
    let mut dark_decls: Vec<Decl> = Vec::new();
    for (id, fb) in &tokens {
        // Jeton déclaré ailleurs que dans le thème (réglé par un élément) : laissé tel quel.
        if declared.contains(&format!("token-{id}")) && !theme.contains_key(id) {
            continue;
        }
        let fb = theme.get(id).unwrap_or(fb);
        let fb = resolve(fb, &|_| false, &|_| None);
        if revel_style::values::color::Rgba::parse(&fb).is_none() {
            continue;
        }
        let name = format!("couleur-{}", root_decls.len() + 1);
        names.insert(id.clone(), name.clone());
        root_decls.push(Decl {
            prop: format!("--{name}"),
            value: fb,
            important: false,
        });
        if let Some(dv) = theme_dark.get(id) {
            let dv = resolve(dv, &|_| false, &|_| None);
            if revel_style::values::color::Rgba::parse(&dv).is_some() {
                dark_decls.push(Decl {
                    prop: format!("--{name}"),
                    value: dv,
                    important: false,
                });
            }
        }
    }
    done.colors = root_decls.len();
    // Valeur réécrite : jetons → couleurs du site ; variables jamais déclarées → repli ; les
    // autres variables (réglées par un élément, lues par une règle) gardées.
    let fix = |v: &str| -> String {
        if !v.contains("var(") {
            return v.to_string();
        }
        resolve(
            v,
            // Jeton devenu couleur du site : remplacé même s'il était déclaré (thème).
            &|n| {
                (declared.contains(n)
                    && !n
                        .strip_prefix("token-")
                        .is_some_and(|id| names.contains_key(id)))
                    || names.values().any(|x| x == n)
            },
            &|n| {
                n.strip_prefix("token-")
                    .and_then(|id| names.get(id))
                    .map(|t| format!("var(--{t})"))
            },
        )
    };

    // 2. Feuille : valeurs réécrites, propriétés sans effet retirées.
    let sheet = site.stylesheet.as_mut().expect("feuille");
    let mut out: Vec<StyleItem> = Vec::new();
    for item in std::mem::take(&mut sheet.items) {
        let StyleItem::Rule(r) = item else {
            out.push(item);
            continue;
        };
        let decls: Vec<Decl> = r
            .decls
            .iter()
            .filter(|d| !DROPPED.contains(&d.prop.as_str()))
            // Jeton du thème devenu une couleur du site.
            .filter(|d| {
                !d.prop
                    .strip_prefix("--token-")
                    .is_some_and(|id| names.contains_key(id))
            })
            .map(|d| Decl {
                value: fix(&d.value),
                ..d.clone()
            })
            .filter(|d| !d.value.is_empty())
            .collect();
        if decls.is_empty() {
            done.removed += 1;
            continue;
        }
        out.push(StyleItem::Rule(StyleRule { decls, ..r }));
    }
    if !root_decls.is_empty() {
        let at = out.iter().position(
            |i| matches!(i, StyleItem::Rule(r) if r.selector == ":root" && r.media.is_none()),
        );
        match at {
            Some(i) => {
                if let StyleItem::Rule(r) = &mut out[i] {
                    r.decls.extend(root_decls);
                }
            }
            None => out.insert(
                0,
                StyleItem::Rule(StyleRule {
                    id: None,
                    selector: ":root".into(),
                    media: None,
                    decls: root_decls,
                }),
            ),
        }
    }
    if !dark_decls.is_empty() {
        let dark_media = "(prefers-color-scheme: dark)".to_string();
        let at = out.iter().position(|i| {
            matches!(i, StyleItem::Rule(r) if r.selector == ":root" && r.media.as_deref() == Some(dark_media.as_str()))
        });
        match at {
            Some(i) => {
                if let StyleItem::Rule(r) = &mut out[i] {
                    r.decls.extend(dark_decls);
                }
            }
            None => out.push(StyleItem::Rule(StyleRule {
                id: None,
                selector: ":root".into(),
                media: Some(dark_media),
                decls: dark_decls,
            })),
        }
    }
    sheet.items = out;

    // 3. Styles écrits sur les éléments : valeurs réécrites ; remplissage des images de fond
    // (enveloppe et image, sans classe) → classes partagées.
    let mut fills: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut fix_nodes = |nodes: &mut Vec<Node>| {
        walk_mut(nodes, &mut |n| {
            let Node::Element {
                tag,
                attrs,
                children,
                ..
            } = n
            else {
                return;
            };
            let wrapper = attr(attrs, "data-framer-background-image-wrapper").is_some();
            if wrapper {
                let classed = |a: &[Attr]| !classes(a).is_empty();
                for (el_attrs, base) in std::iter::once((attrs as &mut Vec<Attr>, "image-fill"))
                    .chain(children.iter_mut().filter_map(|c| match c {
                        Node::Element { tag, attrs, .. } if tag == "img" => {
                            Some((attrs, "image-fill__img"))
                        }
                        _ => None,
                    }))
                {
                    if classed(el_attrs) {
                        continue;
                    }
                    let Some(style) = attr(el_attrs, "style").map(parse_style) else {
                        continue;
                    };
                    let decls: Vec<(String, String)> = style
                        .into_iter()
                        .filter(|(k, _)| !DROPPED.contains(&k.as_str()))
                        // Valeur par défaut (milieu de l'image) : rien à écrire.
                        .filter(|(k, v)| {
                            !(k == "object-position"
                                && matches!(v.trim(), "center" | "center center" | "50% 50%"))
                        })
                        .map(|(k, v)| (k, fix(&v)))
                        .collect();
                    if decls.iter().any(|(k, _)| k.starts_with("--")) {
                        continue;
                    }
                    let key = decls
                        .iter()
                        .map(|(k, v)| format!("{k}:{v}"))
                        .collect::<Vec<_>>()
                        .join(";");
                    let class = match fills.iter().position(|(c, d)| {
                        c.starts_with(base)
                            && d.iter()
                                .map(|(k, v)| format!("{k}:{v}"))
                                .collect::<Vec<_>>()
                                .join(";")
                                == key
                    }) {
                        Some(i) => fills[i].0.clone(),
                        None => {
                            let n = fills
                                .iter()
                                .filter(|(c, _)| {
                                    c.starts_with(base) && !c[base.len()..].starts_with("__")
                                })
                                .count();
                            let c = if n == 0 {
                                base.to_string()
                            } else {
                                format!("{base}-{}", n + 1)
                            };
                            fills.push((c.clone(), decls));
                            c
                        }
                    };
                    el_attrs.retain(|a| a.name != "style");
                    el_attrs.insert(
                        0,
                        Attr {
                            name: "class".into(),
                            value: Some(class),
                            when: None,
                        },
                    );
                    done.inline += 1;
                }
                let _ = tag;
                return;
            }
            if let Some(a) = attrs.iter_mut().find(|a| a.name == "style")
                && let Some(v) = a.value.as_mut()
                && v.contains("var(")
            {
                let fixed = parse_style(v)
                    .into_iter()
                    .filter(|(k, _)| !DROPPED.contains(&k.as_str()))
                    .map(|(k, v)| format!("{k}:{}", fix(&v)))
                    .collect::<Vec<_>>()
                    .join(";");
                *v = fixed;
            }
        });
    };
    for p in &mut site.pages {
        fix_nodes(&mut p.body);
    }
    for l in &mut site.layouts {
        fix_nodes(&mut l.body);
    }
    for c in &mut site.components {
        fix_nodes(&mut c.body);
    }
    let sheet = site.stylesheet.as_mut().expect("feuille");
    for (class, decls) in fills {
        sheet.items.push(StyleItem::Rule(StyleRule {
            id: None,
            selector: format!(".{class}"),
            media: None,
            decls: decls
                .into_iter()
                .map(|(prop, value)| Decl {
                    prop,
                    value,
                    important: false,
                })
                .collect(),
        }));
    }
    if done.colors + done.inline > 0 {
        report.warnings.push(format!(
            "Framer : {} couleur(s) du site (Paramètres › Couleurs), {} remplissage(s) d'image en classes",
            done.colors, done.inline
        ));
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variables_resolues() {
        let keep = |n: &str| n == "couleur-1";
        let ctx: HashMap<String, String> =
            [("framer-font-size".to_string(), "24px".to_string())].into();
        let look = |n: &str| ctx.get(n).cloned();
        assert_eq!(
            resolve("var(--framer-font-size, 16px)", &keep, &look),
            "24px"
        );
        assert_eq!(
            resolve("var(--framer-aspect-ratio-supported, 705px)", &keep, &look),
            "705px"
        );
        assert_eq!(
            resolve("var(--x, var(--couleur-1, #fff))", &keep, &look),
            "var(--couleur-1)"
        );
        assert_eq!(
            parse_style("a:b;c:url(\"x;y\");d:rgb(1, 2, 3)"),
            vec![
                ("a".to_string(), "b".to_string()),
                ("c".into(), "url(\"x;y\")".into()),
                ("d".into(), "rgb(1, 2, 3)".into())
            ]
        );
    }
}
