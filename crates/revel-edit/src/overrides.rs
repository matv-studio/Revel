//! Valeur d'un contenu du CMS remplacée sur une utilisation de composant (D-179).
//!
//! Un composant relié à un contenu (propriété « Contenu ») lit ses textes dans ce contenu
//! (`{{ projet.data.title }}`) ; chaque utilisation choisit le contenu. Pour remplacer un de ces
//! textes sur une seule utilisation, le composant reçoit une propriété texte sans valeur par
//! défaut, lue avant le champ : `{{ titre or projet.data.title }}`. L'utilisation qui la remplit
//! montre sa valeur ; les autres gardent celle du contenu.

use crate::EditError;
use crate::address::{Owner, Parent, Target};
use crate::command::Command;
use revel_model::{Component, Node, PropDef, PropKind, Site};
use std::collections::BTreeSet;

/// `P or E` : propriété `P` lue avant l'expression `E`.
pub fn split(expr: &str) -> Option<(&str, &str)> {
    let (p, e) = expr.trim().split_once(" or ")?;
    let p = p.trim();
    (revel_model::valid_prop_name(p) && !e.trim().is_empty()).then_some((p, e.trim()))
}

/// Propriétés d'un composant qui remplacent un champ du contenu sur une utilisation (masquées de
/// la liste des propriétés : elles se règlent dans « Champs CMS »).
pub fn override_props(c: &Component) -> BTreeSet<String> {
    let texts: BTreeSet<&str> = c
        .props
        .iter()
        .filter(|p| p.kind == PropKind::Text && p.default.is_none())
        .map(|p| p.name.as_str())
        .collect();
    let mut out = BTreeSet::new();
    revel_model::ids::walk(&c.body, &mut |n| {
        if let Node::Out { expr } = n
            && let Some((p, e)) = split(expr)
            && texts.contains(p)
            && reads_entry(c, e)
        {
            out.insert(p.to_string());
        }
    });
    out
}

/// L'expression lit un champ d'un contenu choisi par une propriété « Contenu » du composant
/// (`projet.data.title`, `projet.url`).
pub fn reads_entry(c: &Component, expr: &str) -> bool {
    let head = expr.split('|').next().unwrap_or("").trim();
    c.props
        .iter()
        .filter(|p| p.kind == PropKind::Entry)
        .any(|p| {
            head.strip_prefix(p.name.as_str())
                .is_some_and(|rest| rest.starts_with('.'))
        })
}

/// Libellé d'un champ lu (`projet.data.date_texte` : « Date texte »).
pub fn field_label(expr: &str) -> String {
    let head = split(expr).map(|(_, e)| e).unwrap_or(expr);
    let head = head.split('|').next().unwrap_or("").trim();
    let last = head.rsplit('.').next().unwrap_or(head);
    let t = last.replace(['_', '-'], " ");
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => t,
    }
}

/// Commande qui remplace (ou rend au contenu, `value` vide) le texte lu dans le contenu à
/// l'emplacement `parent`/`index` du composant, pour l'utilisation `at`.
pub fn set_override(
    site: &Site,
    parent: &Parent,
    index: usize,
    at: &Target,
    value: Option<&str>,
) -> Result<Command, EditError> {
    let Owner::Component(cid) = &parent.owner else {
        return Err(EditError::WrongKind(at.node.clone()));
    };
    let c = site
        .components
        .iter()
        .find(|c| &c.id == cid)
        .ok_or_else(|| EditError::NotFound(cid.clone()))?;
    let list = crate::address::list(site, parent)?;
    let Some(Node::Out { expr }) = list.get(index) else {
        return Err(EditError::WrongKind(at.node.clone()));
    };
    let value = value.filter(|v| !v.trim().is_empty());
    if let Some((p, _)) = split(expr)
        && override_props(c).contains(p)
    {
        return Ok(Command::SetUseArg {
            at: at.clone(),
            name: p.to_string(),
            value: value.map(crate::canvas::quote),
        });
    }
    if !reads_entry(c, expr) {
        return Err(EditError::WrongKind(at.node.clone()));
    }
    let Some(value) = value else {
        // Rien de remplacé : rien à faire.
        return Ok(Command::Batch {
            commands: Vec::new(),
        });
    };
    let base = {
        let head = expr.split('|').next().unwrap_or("").trim();
        let last = head.rsplit('.').next().unwrap_or("texte");
        let mut b: String = last
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        if b.is_empty() || b.starts_with(|c: char| c.is_ascii_digit()) {
            b.insert_str(0, "texte_");
        }
        b
    };
    let name = crate::make::free_prop_name(c, &base);
    let over = format!("{name} or {expr}");
    let mut props = c.props.clone();
    props.push(PropDef {
        options: Vec::new(),
        name: name.clone(),
        label: Some(field_label(expr)),
        kind: PropKind::Text,
        default: None,
        collection: None,
    });
    Ok(Command::Batch {
        commands: [Command::SetComponentProps {
            id: cid.clone(),
            props,
        }]
        .into_iter()
        // Toutes les sorties du même champ dans le composant (numéro répété…) : un seul
        // remplacement.
        .chain(
            outs_of(&c.body, expr, &Parent::root(parent.owner.clone()))
                .into_iter()
                .map(|(parent, index)| Command::Replace {
                    parent,
                    index,
                    node: Node::Out { expr: over.clone() },
                }),
        )
        .chain([Command::SetUseArg {
            at: at.clone(),
            name,
            value: Some(crate::canvas::quote(value)),
        }])
        .collect(),
    })
}

/// Emplacements des sorties `{{ expr }}` d'un arbre.
fn outs_of(nodes: &[Node], expr: &str, parent: &Parent) -> Vec<(Parent, usize)> {
    let mut found = Vec::new();
    for (i, n) in nodes.iter().enumerate() {
        if let Node::Out { expr: e } = n
            && e == expr
        {
            found.push((parent.clone(), i));
        }
        let id = revel_model::ids::node_id(n).map(str::to_string);
        for (list, b) in revel_model::ids::child_lists(n)
            .into_iter()
            .zip(crate::address::branches(n))
        {
            let p = Parent {
                owner: parent.owner.clone(),
                node: id.clone(),
                branch: *b,
            };
            found.extend(outs_of(list, expr, &p));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::Branch;
    use revel_model::build::*;

    fn site() -> Site {
        let mut s = Site::new("t", "fr");
        let mut c = Component::new(
            "carte",
            vec![el(
                "article",
                &[("class", "carte")],
                vec![
                    el("h3", &[], vec![out("projet.data.title")]),
                    el("p", &[], vec![out("projet.data.title")]),
                ],
            )],
        );
        c.props.push(PropDef {
            options: Vec::new(),
            name: "projet".into(),
            label: None,
            kind: PropKind::Entry,
            default: None,
            collection: Some("projets".into()),
        });
        s.components.push(c);
        let mut p: revel_model::Page =
            serde_json::from_value(serde_json::json!({"id": "index", "path": "/"})).unwrap();
        for _ in 0..2 {
            p.body.push(Node::Use {
                id: None,
                component: "carte".into(),
                args: [(
                    "projet".to_string(),
                    "collections.projets | by_slug(\"a\")".to_string(),
                )]
                .into(),
                variant: None,
                attrs: Vec::new(),
                children: Vec::new(),
            });
        }
        s.pages.push(p);
        s.ensure_ids();
        s
    }

    /// Remplacé sur une utilisation : propriété ajoutée, lue avant le champ, valeur sur cette
    /// utilisation seulement ; rendu au contenu ; annulable.
    #[test]
    fn champ_remplace_sur_une_utilisation() {
        let mut s = site();
        let h3 = match &s.components[0].body[0] {
            Node::Element { children, .. } => children[0].clone(),
            _ => unreachable!(),
        };
        let Node::Element { id: Some(h3), .. } = h3 else {
            unreachable!()
        };
        let parent = Parent::of(
            &Target {
                owner: Owner::Component("carte".into()),
                node: h3,
            },
            Branch::Children,
        );
        let uses: Vec<String> = s.pages[0]
            .body
            .iter()
            .filter_map(|n| match n {
                Node::Use { id, .. } => id.clone(),
                _ => None,
            })
            .collect();
        let at = Target {
            owner: Owner::Page("index".into()),
            node: uses[0].clone(),
        };
        let cmd = set_override(&s, &parent, 0, &at, Some("Titre \"libre\"")).unwrap();
        crate::apply(&mut s, &cmd).unwrap();
        let c = &s.components[0];
        assert_eq!(c.props[1].name, "title");
        assert_eq!(override_props(c).into_iter().collect::<Vec<_>>(), ["title"]);
        let arg = |s: &Site, i: usize| match &s.pages[0].body[i] {
            Node::Use { args, .. } => args.get("title").cloned(),
            _ => None,
        };
        assert_eq!(arg(&s, 0).as_deref(), Some("\"Titre \\\"libre\\\"\""));
        assert_eq!(arg(&s, 1), None);
        // Même champ ailleurs dans le composant : remplacé aussi.
        match &s.components[0].body[0] {
            Node::Element { children, .. } => assert!(children.iter().all(|n| matches!(n,
                Node::Element { children, .. } if matches!(&children[0], Node::Out { expr } if expr == "title or projet.data.title")))),
            _ => unreachable!(),
        }
        assert!(
            crate::editable::audit(&s).is_empty(),
            "{:?}",
            crate::editable::audit(&s)
        );
        // Second remplacement : la même propriété.
        let cmd = set_override(&s, &parent, 0, &at, Some("Autre")).unwrap();
        assert!(matches!(cmd, Command::SetUseArg { ref name, .. } if name == "title"));
        // Rendu au contenu.
        let cmd = set_override(&s, &parent, 0, &at, None).unwrap();
        crate::apply(&mut s, &cmd).unwrap();
        assert_eq!(arg(&s, 0), None);
        assert_eq!(field_label("title or projet.data.date_texte"), "Date texte");
    }
}
