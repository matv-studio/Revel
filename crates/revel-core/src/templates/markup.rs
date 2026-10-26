//! Forme écrite des modèles de site (D-128) : balisage Nunjucks de Revel, plus
//! - `<x-use c="carte" titre="Texte" x-image="item.data.image" variant="sombre">…</x-use>` :
//!   utilisation d'un composant (`x-` : expression, sinon texte ; `class`, `data-*`, `id`,
//!   `style`, `aria-*` posés sur la racine du composant ; enfants : contenu de l'emplacement) ;
//! - `{{ content | safe }}` : emplacement de la page dans un gabarit.
//!
//! [`print`] écrit des nœuds dans cette forme (relecture des outils, essais).

use revel_model::{Attr, Node};
use std::collections::BTreeMap;

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Nœuds d'un texte de modèle (sans identifiants) ; avertissements du balisage en erreur.
pub fn parse(src: &str) -> Result<Vec<Node>, String> {
    let parsed = revel_markup::parse(
        src,
        &revel_markup::Options {
            dialect: revel_markup::Dialect::Nunjucks,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    if let Some(w) = parsed.warnings.first() {
        return Err(w.clone());
    }
    let mut nodes = parsed.nodes;
    convert(&mut nodes);
    Ok(nodes)
}

/// Attribut posé sur la racine du composant plutôt que donné en propriété.
fn root_attr(name: &str) -> bool {
    matches!(name, "class" | "id" | "style" | "hidden")
        || name.starts_with("data-")
        || name.starts_with("aria-")
}

fn convert(nodes: &mut [Node]) {
    for n in nodes.iter_mut() {
        if let Node::Out { expr } = n
            && expr.split_whitespace().collect::<String>() == "content|safe"
        {
            *n = Node::Slot;
            continue;
        }
        if let Node::Element {
            tag,
            attrs,
            children,
            ..
        } = n
            && tag == "x-use"
        {
            let mut component = String::new();
            let mut variant = None;
            let mut args = BTreeMap::new();
            let mut root = Vec::new();
            for a in std::mem::take(attrs) {
                let v = a.value.clone().unwrap_or_default();
                match a.name.as_str() {
                    "c" => component = v,
                    "variant" => variant = Some(v),
                    name if name.starts_with("x-") => {
                        args.insert(name[2..].to_string(), v);
                    }
                    name if root_attr(name) => root.push(a),
                    name => {
                        args.insert(
                            name.to_string(),
                            serde_json::to_string(&v).unwrap_or_default(),
                        );
                    }
                }
            }
            let mut kids = std::mem::take(children);
            convert(&mut kids);
            kids.retain(|k| !matches!(k, Node::Text { text } if text.trim().is_empty()));
            *n = Node::Use {
                id: None,
                component,
                args,
                variant,
                attrs: root,
                children: kids,
            };
            continue;
        }
        for list in revel_model::ids::child_lists_mut(n) {
            convert(list);
        }
    }
}

fn esc_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;")
}

fn esc_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
}

fn attrs(out: &mut String, list: &[Attr]) {
    for a in list {
        if let Some(w) = &a.when {
            out.push_str(&format!("{{% if {w} %}}"));
        }
        out.push(' ');
        out.push_str(&a.name);
        if let Some(v) = &a.value {
            out.push_str(&format!("=\"{}\"", esc_attr(v)));
        }
        if a.when.is_some() {
            out.push_str("{% endif %}");
        }
    }
}

/// Nœuds écrits dans la forme des modèles.
pub fn print(nodes: &[Node]) -> String {
    let mut out = String::new();
    write(nodes, &mut out);
    out
}

fn write(nodes: &[Node], out: &mut String) {
    for n in nodes {
        match n {
            Node::Element {
                tag,
                attrs: a,
                children,
                ..
            } => {
                out.push('<');
                out.push_str(tag);
                attrs(out, a);
                out.push('>');
                if !VOID.contains(&tag.as_str()) {
                    write(children, out);
                    out.push_str(&format!("</{tag}>"));
                }
            }
            Node::Text { text } => out.push_str(&esc_text(text)),
            Node::Html { html } => out.push_str(html),
            Node::Out { expr } => out.push_str(&format!("{{{{ {expr} }}}}")),
            Node::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                out.push_str(&format!("{{% if {cond} %}}"));
                write(then, out);
                if !otherwise.is_empty() {
                    out.push_str("{% else %}");
                    write(otherwise, out);
                }
                out.push_str("{% endif %}");
            }
            Node::Each {
                expr,
                item,
                children,
                empty,
                ..
            } => {
                out.push_str(&format!("{{% for {item} in {expr} %}}"));
                write(children, out);
                if !empty.is_empty() {
                    out.push_str("{% else %}");
                    write(empty, out);
                }
                out.push_str("{% endfor %}");
            }
            Node::Set { name, expr } => out.push_str(&format!("{{% set {name} = {expr} %}}")),
            Node::SetBlock { name, children, .. } => {
                out.push_str(&format!("{{% set {name} %}}"));
                write(children, out);
                out.push_str("{% endset %}");
            }
            Node::Slot => out.push_str("{{ content | safe }}"),
            Node::Use {
                component,
                args,
                variant,
                attrs: a,
                children,
                ..
            } => {
                out.push_str(&format!("<x-use c=\"{}\"", esc_attr(component)));
                if let Some(v) = variant {
                    out.push_str(&format!(" variant=\"{}\"", esc_attr(v)));
                }
                for (k, v) in args {
                    match serde_json::from_str::<String>(v) {
                        Ok(text) => out.push_str(&format!(" {k}=\"{}\"", esc_attr(&text))),
                        Err(_) => out.push_str(&format!(" x-{k}=\"{}\"", esc_attr(v))),
                    }
                }
                attrs(out, a);
                out.push('>');
                write(children, out);
                out.push_str("</x-use>");
            }
            Node::Image(i) => {
                let parts = [
                    Some(i.src.clone()),
                    i.alt.clone(),
                    i.sizes.clone(),
                    i.class.clone(),
                    i.ratio.clone(),
                    i.loading.clone(),
                    i.widths.clone(),
                ];
                let last = parts.iter().rposition(Option::is_some).unwrap_or(0);
                let args: Vec<String> = parts[..=last]
                    .iter()
                    .map(|p| p.clone().unwrap_or_else(|| "\"\"".into()))
                    .collect();
                out.push_str(&format!("{{% image {} %}}", args.join(", ")));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composant_ecrit_et_relu() {
        let src = r#"<section class="s"><x-use c="carte" titre="Bonjour « vous »" x-image="item.data.image" variant="sombre" data-revel-anim="fade-up"><p>Contenu</p></x-use>{{ content | safe }}</section>"#;
        let nodes = parse(src).unwrap();
        let Node::Element { children, .. } = &nodes[0] else {
            panic!()
        };
        let Node::Use {
            component,
            args,
            variant,
            attrs,
            children: kids,
            ..
        } = &children[0]
        else {
            panic!("{children:?}")
        };
        assert_eq!(component, "carte");
        assert_eq!(args["titre"], "\"Bonjour « vous »\"");
        assert_eq!(args["image"], "item.data.image");
        assert_eq!(variant.as_deref(), Some("sombre"));
        assert_eq!(attrs[0].name, "data-revel-anim");
        assert_eq!(kids.len(), 1);
        assert!(matches!(children[1], Node::Slot));
        assert_eq!(parse(&print(&nodes)).unwrap(), nodes);
    }
}
