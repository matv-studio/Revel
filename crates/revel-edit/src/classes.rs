//! Classes du site (D-113) : relevé (règles de style, éléments qui les portent), renommage
//! partout, retrait.
//!
//! Une classe est un nom (`carte`, `about-title`) posé sur des éléments (`class="…"`) et utilisé
//! par les règles de la feuille du site (`.carte:hover .titre`). Le gestionnaire de classes de
//! l'éditeur liste chaque classe avec ses règles et ses utilisations, la renomme partout (règles,
//! éléments des pages, gabarits et composants, cibles des interactions) et retire celles qui ne
//! servent plus. Les classes d'état posées par les scripts du site (`is-active`, `rv-…`) ne sont
//! pas listées : elles ne s'écrivent pas sur les éléments.

use crate::EditError;
use crate::address::{Owner, Target};
use crate::command::Command;
use revel_model::ids::OwnerKind;
use revel_model::{Node, Site, StyleItem};
use std::collections::BTreeMap;

/// Élément qui porte une classe.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClassUse {
    pub at: Target,
    /// Balise (`div`, `a`) ou composant utilisé.
    pub tag: String,
    /// Nom du calque, s'il en a un.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Classe du site.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClassInfo {
    pub name: String,
    /// Règles de style qui la citent (identifiants).
    pub rules: Vec<String>,
    /// Éléments qui la portent.
    pub uses: Vec<ClassUse>,
}

/// Nom de classe valide : lettres, chiffres, `-`, `_`, sans commencer par un chiffre.
pub fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 80
        && !s.starts_with(|c: char| c.is_ascii_digit())
        && !s.starts_with("--")
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Classe d'état posée par un script (non listée).
pub fn is_state(s: &str) -> bool {
    s.starts_with("is-") || s.starts_with("rv-") || s.starts_with("w--")
}

fn ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii()
}

/// Positions des classes d'un sélecteur ou d'un bloc CSS : (début du nom, fin), après chaque `.`
/// suivi d'un nom (pas les nombres : `0.5s`).
fn class_spans(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let b: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    while i < b.len() {
        let (pos, c) = b[i];
        // `.` précédé d'un chiffre : nombre décimal (`0.5`), pas une classe.
        let after_digit = i > 0 && b[i - 1].1.is_ascii_digit();
        if c == '.'
            && !after_digit
            && b.get(i + 1)
                .is_some_and(|(_, n)| n.is_ascii_alphabetic() || *n == '_' || *n == '-')
        {
            let start = pos + 1;
            let mut j = i + 1;
            while j < b.len() && ident(b[j].1) {
                j += 1;
            }
            let end = b.get(j).map_or(text.len(), |(p, _)| *p);
            out.push((start, end));
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

/// Classes citées par un sélecteur.
pub fn selector_classes(sel: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for (a, b) in class_spans(sel) {
        let c = sel[a..b].to_string();
        if !v.contains(&c) {
            v.push(c);
        }
    }
    v
}

/// Classes écrites dans un attribut `class` (les parties calculées `{{ … }}` sont ignorées).
fn attr_classes(v: &str) -> Vec<&str> {
    if v.contains("{{") || v.contains("{%") {
        return v.split_whitespace().filter(|c| valid_name(c)).collect();
    }
    v.split_whitespace().collect()
}

/// Expression écrite d'une liste de classes (`"carte grande"`), pour la classe d'une image.
fn quoted(v: &str) -> String {
    format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Classes d'un élément, d'une utilisation de composant ou d'une image (classe écrite).
fn node_classes(n: &Node) -> Option<(String, String)> {
    match n {
        Node::Element { attrs, tag, .. } => attrs
            .iter()
            .find(|a| a.name == "class")
            .and_then(|a| a.value.clone())
            .map(|v| (v, tag.clone())),
        Node::Use {
            attrs, component, ..
        } => attrs
            .iter()
            .find(|a| a.name == "class")
            .and_then(|a| a.value.clone())
            .map(|v| (v, component.clone())),
        Node::Image(img) => img
            .class
            .as_deref()
            .and_then(crate::canvas::literal)
            .map(|v| (v, "img".to_string())),
        _ => None,
    }
}

fn owner_of(kind: &OwnerKind, id: &str) -> Owner {
    match kind {
        OwnerKind::Page => Owner::Page(id.to_string()),
        OwnerKind::Layout => Owner::Layout(id.to_string()),
        OwnerKind::Component => Owner::Component(id.to_string()),
    }
}

/// Relevé des classes du site, par nom.
pub fn inventory(site: &Site) -> Vec<ClassInfo> {
    let mut map: BTreeMap<String, ClassInfo> = BTreeMap::new();
    fn entry<'m>(
        map: &'m mut BTreeMap<String, ClassInfo>,
        name: &str,
    ) -> Option<&'m mut ClassInfo> {
        if is_state(name) || !valid_name(name) {
            return None;
        }
        Some(map.entry(name.to_string()).or_insert_with(|| ClassInfo {
            name: name.to_string(),
            rules: Vec::new(),
            uses: Vec::new(),
        }))
    }
    for r in site.style_rules() {
        for c in selector_classes(&r.selector) {
            if let Some(e) = entry(&mut map, &c)
                && let Some(id) = &r.id
                && !e.rules.contains(id)
            {
                e.rules.push(id.clone());
            }
        }
    }
    for (kind, id, body) in site.owners() {
        let owner = owner_of(&kind, id);
        revel_model::ids::walk(body, &mut |n| {
            let Some(node) = revel_model::ids::node_id(n) else {
                return;
            };
            let Some((v, tag)) = node_classes(n) else {
                return;
            };
            for c in attr_classes(&v) {
                if let Some(e) = entry(&mut map, c) {
                    e.uses.push(ClassUse {
                        at: Target {
                            owner: owner.clone(),
                            node: node.to_string(),
                        },
                        tag: tag.clone(),
                        name: revel_model::layer_name(n).map(str::to_string),
                    });
                }
            }
        });
    }
    map.into_values().collect()
}

/// `text` avec la classe `from` remplacée par `to`, mot entier (`.carte` mais pas `.carte-titre`) :
/// `dot`, dans du CSS ou une cible (`.carte`) ; sinon dans une liste de classes (`carte grande`).
fn replace_token(text: &str, from: &str, to: &str, dot: bool) -> String {
    if dot {
        let mut out = String::new();
        let mut last = 0;
        for (a, b) in class_spans(text) {
            if &text[a..b] == from {
                out.push_str(&text[last..a]);
                out.push_str(to);
                last = b;
            }
        }
        out.push_str(&text[last..]);
        out
    } else {
        let mut out = String::new();
        let mut word = String::new();
        let flush = |w: &mut String, out: &mut String| {
            out.push_str(if w == from { to } else { w.as_str() });
            w.clear();
        };
        for c in text.chars() {
            if c.is_whitespace() {
                flush(&mut word, &mut out);
                out.push(c);
            } else {
                word.push(c);
            }
        }
        flush(&mut word, &mut out);
        out
    }
}

/// Renomme une classe partout ; renvoie le nombre de règles et d'éléments changés.
pub(crate) fn rename(site: &mut Site, from: &str, to: &str) -> Result<(usize, usize), EditError> {
    if !valid_name(to) || is_state(to) {
        return Err(EditError::InvalidClass(to.to_string()));
    }
    if from == to {
        return Err(EditError::InvalidClass(to.to_string()));
    }
    if inventory(site).iter().any(|c| c.name == to) {
        return Err(EditError::ClassTaken(to.to_string()));
    }
    let mut rules = 0;
    if let Some(sheet) = site.stylesheet.as_mut() {
        for item in &mut sheet.items {
            match item {
                StyleItem::Rule(r) => {
                    let n = replace_token(&r.selector, from, to, true);
                    if n != r.selector {
                        r.selector = n;
                        rules += 1;
                    }
                }
                StyleItem::Raw { css } => {
                    let n = replace_token(css, from, to, true);
                    if n != *css {
                        *css = n;
                        rules += 1;
                    }
                }
                StyleItem::Comment { .. } => {}
            }
        }
    }
    let mut nodes = 0;
    fn walk(
        nodes: &mut [Node],
        f: &mut dyn FnMut(&mut Vec<revel_model::Attr>),
        g: &mut dyn FnMut(&mut revel_model::ImageNode),
    ) {
        for n in nodes {
            match n {
                Node::Element { attrs, .. } | Node::Use { attrs, .. } => f(attrs),
                Node::Image(img) => g(img),
                _ => {}
            }
            for list in revel_model::ids::child_lists_mut(n) {
                walk(list, f, g);
            }
        }
    }
    let mut fix = |attrs: &mut Vec<revel_model::Attr>| {
        for a in attrs.iter_mut() {
            let Some(v) = &a.value else { continue };
            let n = match a.name.as_str() {
                "class" => replace_token(v, from, to, false),
                // Cibles des interactions (`.carte`, `closest:.carte`).
                "data-revel-on" => replace_token(v, from, to, true),
                _ => continue,
            };
            if n != *v {
                a.value = Some(n);
                if a.name == "class" {
                    nodes += 1;
                }
            }
        }
    };
    // Image : classe écrite entre guillemets (`"carte__image"`).
    let mut images = 0;
    let mut fix_image = |img: &mut revel_model::ImageNode| {
        let Some(v) = img.class.as_deref().and_then(crate::canvas::literal) else {
            return;
        };
        let n = replace_token(&v, from, to, false);
        if n != v {
            img.class = Some(quoted(&n));
            images += 1;
        }
    };
    for p in &mut site.pages {
        walk(&mut p.body, &mut fix, &mut fix_image);
    }
    for l in &mut site.layouts {
        walk(&mut l.body, &mut fix, &mut fix_image);
    }
    for c in &mut site.components {
        walk(&mut c.body, &mut fix, &mut fix_image);
    }
    Ok((rules, nodes + images))
}

/// Commandes qui retirent une classe : des éléments qui la portent, et ses règles (une règle à
/// plusieurs sélecteurs garde les autres).
pub fn remove_commands(site: &Site, name: &str) -> Result<Vec<Command>, EditError> {
    let info = inventory(site)
        .into_iter()
        .find(|c| c.name == name)
        .ok_or_else(|| EditError::NotFound(format!("classe « {name} »")))?;
    let mut out = Vec::new();
    for u in &info.uses {
        let node = crate::address::node(site, &u.at)?;
        if let Node::Image(img) = node {
            let v = img
                .class
                .as_deref()
                .and_then(crate::canvas::literal)
                .unwrap_or_default();
            let rest: Vec<&str> = v.split_whitespace().filter(|c| *c != name).collect();
            let mut image = img.clone();
            image.class = (!rest.is_empty()).then(|| quoted(&rest.join(" ")));
            out.push(Command::SetImage {
                at: u.at.clone(),
                image,
            });
            continue;
        }
        let (Node::Element { attrs, .. } | Node::Use { attrs, .. }) = node else {
            continue;
        };
        let v = attrs
            .iter()
            .find(|a| a.name == "class")
            .and_then(|a| a.value.clone())
            .unwrap_or_default();
        let rest: Vec<&str> = v.split_whitespace().filter(|c| *c != name).collect();
        out.push(if rest.is_empty() {
            Command::RemoveAttr {
                at: u.at.clone(),
                name: "class".into(),
            }
        } else {
            Command::SetAttr {
                at: u.at.clone(),
                name: "class".into(),
                value: Some(rest.join(" ")),
            }
        });
    }
    for r in site.style_rules() {
        let Some(id) = &r.id else { continue };
        if !info.rules.contains(id) {
            continue;
        }
        let keep: Vec<&str> = r
            .selector
            .split(',')
            .map(str::trim)
            .filter(|s| !selector_classes(s).iter().any(|c| c == name))
            .collect();
        out.push(if keep.is_empty() {
            Command::RemoveRule { rule: id.clone() }
        } else {
            Command::SetSelector {
                rule: id.clone(),
                selector: keep.join(", "),
            }
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use revel_model::{Attr, StyleRule, Stylesheet};

    fn el(id: &str, class: &str, on: Option<&str>) -> Node {
        let mut attrs = vec![Attr {
            name: "class".into(),
            value: Some(class.into()),
            when: None,
        }];
        if let Some(on) = on {
            attrs.push(Attr {
                name: "data-revel-on".into(),
                value: Some(on.into()),
                when: None,
            });
        }
        Node::Element {
            id: Some(id.into()),
            tag: "div".into(),
            attrs,
            children: vec![],
        }
    }

    fn site() -> Site {
        let mut s = Site::new("t", "fr");
        let mut p: revel_model::Page =
            serde_json::from_value(serde_json::json!({"id": "index", "path": "/", "body": []}))
                .unwrap();
        p.body = vec![
            el("a1", "carte carte-grande", None),
            el(
                "a2",
                "carte-titre is-active",
                Some("click class:is-open closest:.carte"),
            ),
            Node::Image(revel_model::ImageNode {
                id: Some("i1".into()),
                src: "\"/media/a.jpg\"".into(),
                class: Some("\"carte-image\"".into()),
                ..Default::default()
            }),
        ];
        s.pages.push(p);
        s.stylesheet = Some(Stylesheet {
            url: "/s.css".into(),
            source: None,
            items: vec![
                StyleItem::Rule(StyleRule {
                    id: Some("r1".into()),
                    selector: ".carte:hover .carte-titre".into(),
                    ..Default::default()
                }),
                StyleItem::Rule(StyleRule {
                    id: Some("r2".into()),
                    selector: ".carte, .vieille".into(),
                    ..Default::default()
                }),
                StyleItem::Rule(StyleRule {
                    id: Some("r3".into()),
                    selector: ".vieille.is-active".into(),
                    ..Default::default()
                }),
            ],
        });
        s
    }

    #[test]
    fn releve() {
        let inv = inventory(&site());
        let names: Vec<&str> = inv.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "carte",
                "carte-grande",
                "carte-image",
                "carte-titre",
                "vieille"
            ]
        );
        assert_eq!(inv[2].uses.len(), 1);
        assert_eq!(inv[2].uses[0].tag, "img");
        let carte = &inv[0];
        assert_eq!(carte.rules, vec!["r1", "r2"]);
        assert_eq!(carte.uses.len(), 1);
        assert!(inv[4].uses.is_empty() && inv[4].rules == vec!["r2", "r3"]);
    }

    #[test]
    fn renommer_partout_et_revenir() {
        let mut s = site();
        let before = s.clone();
        let inv = crate::command::apply(
            &mut s,
            &Command::RenameClass {
                from: "carte".into(),
                to: "projet".into(),
            },
        )
        .unwrap();
        let json = serde_json::to_string(&s).unwrap();
        assert!(
            json.contains("\"projet carte-grande\"") && json.contains("closest:.projet"),
            "{json}"
        );
        assert!(
            json.contains(".projet:hover .carte-titre") && json.contains(".projet, .vieille"),
            "{json}"
        );
        assert!(!json.contains(".carte:") && json.contains("carte-titre"));
        crate::command::apply(&mut s, &inv).unwrap();
        assert_eq!(s, before);
        // Classe d'une image renommée et retirée.
        let inv = crate::command::apply(
            &mut s,
            &Command::RenameClass {
                from: "carte-image".into(),
                to: "visuel".into(),
            },
        )
        .unwrap();
        assert!(
            serde_json::to_string(&s.pages)
                .unwrap()
                .contains(r#""class":"\"visuel\"""#)
        );
        crate::command::apply(&mut s, &inv).unwrap();
        assert_eq!(s, before);
        // Nom déjà pris ou invalide : refusé.
        for to in ["carte-titre", "2col", "is-x", "a b"] {
            assert!(
                crate::command::apply(
                    &mut s,
                    &Command::RenameClass {
                        from: "carte".into(),
                        to: to.into()
                    }
                )
                .is_err(),
                "{to}"
            );
        }
    }

    #[test]
    fn retirer() {
        let mut s = site();
        let cmds = remove_commands(&s, "vieille").unwrap();
        assert_eq!(cmds.len(), 2);
        crate::command::apply(&mut s, &Command::Batch { commands: cmds }).unwrap();
        let json = serde_json::to_string(&s.stylesheet).unwrap();
        assert!(
            json.contains("\"selector\":\".carte\"") && !json.contains("vieille"),
            "{json}"
        );
        let cmds = remove_commands(&s, "carte-grande").unwrap();
        crate::command::apply(&mut s, &Command::Batch { commands: cmds }).unwrap();
        assert!(
            serde_json::to_string(&s.pages)
                .unwrap()
                .contains("\"value\":\"carte\"")
        );
        let cmds = remove_commands(&s, "carte-image").unwrap();
        crate::command::apply(&mut s, &Command::Batch { commands: cmds }).unwrap();
        assert!(
            !serde_json::to_string(&s.pages)
                .unwrap()
                .contains("carte-image")
        );
    }
}
