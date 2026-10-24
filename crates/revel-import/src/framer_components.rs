//! Composants Framer utilisés plusieurs fois → composants Revel (D-112).
//!
//! Dans la page publiée, chaque utilisation d'un composant Framer est recopiée en entier
//! (élément racine `framer-<composant> … framer-v-<variante>`). Les utilisations d'une même
//! variante qui ne diffèrent que par leurs textes, images, liens et textes alternatifs deviennent
//! un composant Revel : ce qui change d'une utilisation à l'autre devient une propriété (nommée
//! d'après le calque de Framer), chaque utilisation garde ses valeurs. Le site produit reste le
//! même ; une modification du composant vaut pour toutes ses utilisations.

use crate::Report;
use revel_model::{Attr, Component, Node, PropDef, PropKind, Site};
use std::collections::{BTreeMap, HashMap};

/// Attributs qui peuvent changer d'une utilisation à l'autre (propriétés).
const VARYING: [&str; 5] = ["src", "href", "alt", "aria-label", "title"];
/// Attributs retirés quand l'image change (variantes de taille propres à chaque image).
const IMAGE_SIZES: [&str; 2] = ["srcset", "sizes"];

fn class_list(attrs: &[Attr]) -> Vec<&str> {
    attrs
        .iter()
        .find(|a| a.name == "class")
        .and_then(|a| a.value.as_deref())
        .unwrap_or("")
        .split_whitespace()
        .collect()
}

/// Clé d'une utilisation : (classe du composant, classe de la variante).
fn instance_key(n: &Node) -> Option<(String, String)> {
    let Node::Element { attrs, .. } = n else {
        return None;
    };
    let cls = class_list(attrs);
    let first = cls.first()?;
    let comp = first.strip_prefix("framer-")?;
    if comp.len() != 5 || !comp.chars().any(|c| c.is_ascii_uppercase()) {
        return None;
    }
    let variant = cls.iter().find(|c| c.starts_with("framer-v-"))?;
    Some((first.to_string(), variant.to_string()))
}

/// Même structure (balises, attributs autres que ceux qui peuvent changer, nombre d'enfants).
fn same_shape(a: &Node, b: &Node) -> bool {
    match (a, b) {
        (Node::Text { .. }, Node::Text { .. }) => true,
        (
            Node::Element {
                tag: t1,
                attrs: a1,
                children: c1,
                ..
            },
            Node::Element {
                tag: t2,
                attrs: a2,
                children: c2,
                ..
            },
        ) => {
            let fixed = |a: &[Attr]| -> Vec<(String, Option<String>)> {
                let mut v: Vec<(String, Option<String>)> = a
                    .iter()
                    .filter(|x| {
                        !VARYING.contains(&x.name.as_str())
                            && !IMAGE_SIZES.contains(&x.name.as_str())
                    })
                    .map(|x| (x.name.clone(), x.value.clone()))
                    .collect();
                v.sort();
                v
            };
            t1 == t2
                && fixed(a1) == fixed(a2)
                && c1.len() == c2.len()
                && c1.iter().zip(c2).all(|(x, y)| same_shape(x, y))
        }
        (x, y) => x == y,
    }
}

/// Nom de propriété d'après le calque (`data-framer-name`) le plus proche : `title`, `name`…
fn prop_name(layer: Option<&str>, kind: &str, taken: &mut Vec<String>) -> String {
    let base: String = layer
        .map(|l| {
            l.chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '_'
                    }
                })
                .collect::<String>()
                .trim_matches('_')
                .to_string()
        })
        .filter(|s| !s.is_empty() && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic()))
        .unwrap_or_else(|| kind.to_string());
    let base = if kind == "text" {
        base
    } else {
        format!("{base}_{kind}")
    };
    let mut name = base.clone();
    let mut n = 2;
    while taken.contains(&name) {
        name = format!("{base}_{n}");
        n += 1;
    }
    taken.push(name.clone());
    name
}

struct Plan {
    props: Vec<PropDef>,
    /// Pour chaque utilisation : propriété → valeur.
    values: Vec<BTreeMap<String, String>>,
}

/// Parcourt les utilisations en parallèle : le corps du composant (copie de la première) reçoit
/// les propriétés à la place de ce qui change.
fn diff(
    body: &mut Node,
    others: &[&Node],
    layer: Option<&str>,
    plan: &mut Plan,
    taken: &mut Vec<String>,
) {
    match body {
        Node::Text { text } => {
            let vals: Vec<String> = others
                .iter()
                .map(|o| match o {
                    Node::Text { text } => text.clone(),
                    _ => String::new(),
                })
                .collect();
            if vals.iter().any(|v| v.trim() != text.trim()) && !text.trim().is_empty() {
                let name = prop_name(layer, "text", taken);
                plan.props.push(PropDef {
                    name: name.clone(),
                    label: layer.map(str::to_string),
                    kind: PropKind::Text,
                    default: Some(serde_json::Value::String(text.trim().to_string())),
                });
                plan.values[0].insert(name.clone(), text.trim().to_string());
                for (i, v) in vals.iter().enumerate() {
                    plan.values[i + 1].insert(name.clone(), v.trim().to_string());
                }
                *body = Node::Out { expr: name };
            }
        }
        Node::Element {
            attrs, children, ..
        } => {
            let own = attrs
                .iter()
                .find(|a| a.name == "data-framer-name")
                .and_then(|a| a.value.clone());
            let layer = own.as_deref().or(layer);
            let mut drop_sizes = false;
            for name in VARYING {
                let mine = attrs
                    .iter()
                    .find(|a| a.name == name)
                    .and_then(|a| a.value.clone());
                let theirs: Vec<Option<String>> = others
                    .iter()
                    .map(|o| match o {
                        Node::Element { attrs, .. } => attrs
                            .iter()
                            .find(|a| a.name == name)
                            .and_then(|a| a.value.clone()),
                        _ => None,
                    })
                    .collect();
                if mine.is_none() || theirs.iter().all(|t| *t == mine) {
                    continue;
                }
                let kind = match name {
                    "src" => PropKind::Image,
                    "href" => PropKind::Link,
                    _ => PropKind::Text,
                };
                let suffix = match name {
                    "src" => "image",
                    "href" => "link",
                    _ => "alt",
                };
                let prop = prop_name(layer, suffix, taken);
                let mine = mine.unwrap_or_default();
                plan.props.push(PropDef {
                    name: prop.clone(),
                    label: layer.map(|l| format!("{l} ({suffix})")),
                    kind,
                    default: Some(serde_json::Value::String(mine.clone())),
                });
                plan.values[0].insert(prop.clone(), mine);
                for (i, t) in theirs.iter().enumerate() {
                    plan.values[i + 1].insert(prop.clone(), t.clone().unwrap_or_default());
                }
                if let Some(a) = attrs.iter_mut().find(|a| a.name == name) {
                    a.value = Some(format!("{{{{ {prop} }}}}"));
                }
                if name == "src" {
                    drop_sizes = true;
                }
            }
            if drop_sizes {
                attrs.retain(|a| !IMAGE_SIZES.contains(&a.name.as_str()));
            }
            for (i, c) in children.iter_mut().enumerate() {
                let kids: Vec<&Node> = others
                    .iter()
                    .filter_map(|o| match o {
                        Node::Element { children, .. } => children.get(i),
                        _ => None,
                    })
                    .collect();
                if kids.len() == others.len() {
                    diff(c, &kids, layer, plan, taken);
                }
            }
        }
        _ => {}
    }
}

/// Valeur d'une propriété écrite en expression (`"Titre"`).
fn literal(v: &str) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "\"\"".into())
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// Adresses des utilisations : (propriétaire, chemin d'index) hors des listes de collection.
fn find(nodes: &[Node], path: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, (String, String))>) {
    for (i, n) in nodes.iter().enumerate() {
        path.push(i);
        if let Some(k) = instance_key(n) {
            out.push((path.clone(), k));
        } else if let Node::Element { children, .. } = n {
            find(children, path, out);
        }
        path.pop();
    }
}

fn at<'a>(nodes: &'a mut [Node], path: &[usize]) -> Option<&'a mut Node> {
    let (first, rest) = path.split_first()?;
    let n = nodes.get_mut(*first)?;
    if rest.is_empty() {
        return Some(n);
    }
    match n {
        Node::Element { children, .. } => at(children, rest),
        _ => None,
    }
}

fn at_ref<'a>(nodes: &'a [Node], path: &[usize]) -> Option<&'a Node> {
    let (first, rest) = path.split_first()?;
    let n = nodes.get(*first)?;
    if rest.is_empty() {
        return Some(n);
    }
    match n {
        Node::Element { children, .. } => at_ref(children, rest),
        _ => None,
    }
}

/// Crée les composants ; `names` : classe du composant Framer → son nom (`displayName`).
pub fn make_components(
    site: &mut Site,
    names: &HashMap<String, String>,
    report: &mut Report,
) -> usize {
    // Utilisations par page (les gabarits gardent leurs copies).
    let mut groups: BTreeMap<(String, String), Vec<(usize, Vec<usize>)>> = BTreeMap::new();
    for (pi, p) in site.pages.iter().enumerate() {
        let mut found = Vec::new();
        find(&p.body, &mut Vec::new(), &mut found);
        for (path, key) in found {
            groups.entry(key).or_default().push((pi, path));
        }
    }
    let mut made = 0;
    let mut replace: Vec<(usize, Vec<usize>, Node)> = Vec::new();
    for ((comp, _variant), uses) in groups {
        if uses.len() < 2 {
            continue;
        }
        let nodes: Vec<&Node> = uses
            .iter()
            .filter_map(|(pi, path)| at_ref(&site.pages[*pi].body, path))
            .collect();
        if nodes.len() != uses.len() || !nodes[1..].iter().all(|n| same_shape(nodes[0], n)) {
            continue;
        }
        let mut body = nodes[0].clone();
        let mut plan = Plan {
            props: Vec::new(),
            values: vec![BTreeMap::new(); nodes.len()],
        };
        diff(&mut body, &nodes[1..], None, &mut plan, &mut Vec::new());
        let name = names
            .get(&comp)
            .cloned()
            .unwrap_or_else(|| comp.trim_start_matches("framer-").to_string());
        let mut id = slug(&name);
        if id.is_empty() {
            id = slug(&comp);
        }
        let base = id.clone();
        let mut k = 2;
        while site.components.iter().any(|c| c.id == id) {
            id = format!("{base}-{k}");
            k += 1;
        }
        let mut component = Component::new(id.clone(), vec![strip_ids(body)]);
        component.name = Some(name.clone());
        component.props = plan.props.clone();
        site.components.push(component);
        for (i, (pi, path)) in uses.iter().enumerate() {
            let args: BTreeMap<String, String> = plan.values[i]
                .iter()
                .filter(|(p, v)| {
                    plan.props
                        .iter()
                        .find(|d| &d.name == *p)
                        .and_then(|d| d.default.as_ref())
                        .and_then(|d| d.as_str())
                        != Some(v.as_str())
                })
                .map(|(p, v)| (p.clone(), literal(v)))
                .collect();
            replace.push((
                *pi,
                path.clone(),
                Node::Use {
                    id: None,
                    component: id.clone(),
                    args,
                    variant: None,
                    attrs: Vec::new(),
                },
            ));
        }
        made += 1;
        report.warnings.push(format!(
            "Framer : composant « {name} » ({} utilisations, {} propriété(s))",
            uses.len(),
            plan.props.len()
        ));
    }
    for (pi, path, node) in replace {
        if let Some(n) = at(&mut site.pages[pi].body, &path) {
            *n = node;
        }
    }
    report.components += made;
    made
}

/// Copie sans identifiants de nœuds (le composant reçoit les siens).
fn strip_ids(mut n: Node) -> Node {
    fn go(n: &mut Node) {
        if let Node::Element { id, children, .. } = n {
            *id = None;
            children.iter_mut().for_each(go);
        }
    }
    go(&mut n);
    n
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
                .map(|(k, v)| Attr {
                    name: (*k).into(),
                    value: Some((*v).into()),
                    when: None,
                })
                .collect(),
            children,
        }
    }
    fn stat(n: &str, label: &str, img: &str) -> Node {
        el(
            "div",
            &[("class", "framer-container")],
            vec![el(
                "div",
                &[
                    ("class", "framer-xMd78 framer-v-o4c472"),
                    ("data-framer-name", "Variant 1"),
                ],
                vec![
                    el(
                        "p",
                        &[("data-framer-name", "Number")],
                        vec![Node::Text { text: n.into() }],
                    ),
                    el(
                        "p",
                        &[("data-framer-name", "Label")],
                        vec![Node::Text { text: label.into() }],
                    ),
                    el("img", &[("src", img), ("srcset", "x 1x")], vec![]),
                ],
            )],
        )
    }

    #[test]
    fn utilisations_en_composant() {
        let mut site = Site::new("t", "fr");
        let mut page: revel_model::Page =
            serde_json::from_value(serde_json::json!({"id": "index", "path": "/", "body": []}))
                .unwrap();
        page.body = vec![el(
            "main",
            &[],
            vec![
                stat("4+", "Years", "/a.png"),
                stat("50+", "Projects", "/a.png"),
                stat("12", "Clients", "/b.png"),
            ],
        )];
        site.pages.push(page);
        let names: HashMap<String, String> =
            [("framer-xMd78".to_string(), "Stats".to_string())].into();
        let mut report = Report::default();
        assert_eq!(make_components(&mut site, &names, &mut report), 1);
        let c = &site.components[0];
        assert_eq!(c.id, "stats");
        let props: Vec<&str> = c.props.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(props, vec!["number", "label", "variant_1_image"]);
        let body = serde_json::to_string(&c.body).unwrap();
        assert!(
            body.contains("\"expr\":\"number\"")
                && body.contains("{{ variant_1_image }}")
                && !body.contains("srcset"),
            "{body}"
        );
        let page = serde_json::to_string(&site.pages[0].body).unwrap();
        assert_eq!(page.matches("\"component\":\"stats\"").count(), 3, "{page}");
        assert!(
            page.contains("\"number\":\"\\\"50+\\\"\"")
                && page.contains("\"variant_1_image\":\"\\\"/b.png\\\"\""),
            "{page}"
        );
    }
}
