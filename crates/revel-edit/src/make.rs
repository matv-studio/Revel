//! Commandes composées (dupliquer, envelopper) et nouveaux nœuds pour l'éditeur.

use revel_model::ids::{child_lists_mut, fresh, node_id_mut};
use revel_model::{Attr, ImageNode, Node, Site};
use std::collections::HashSet;

use crate::EditError;
use crate::address::{self, Branch, Parent, Target};
use crate::command::Command;

/// Donne de nouveaux identifiants à tous les nœuds structurels d'un sous-arbre.
pub fn refresh_ids(n: &mut Node, taken: &mut HashSet<String>) {
    if let Some(slot) = node_id_mut(n) {
        let id = fresh(taken);
        taken.insert(id.clone());
        *slot = Some(id);
    }
    for list in child_lists_mut(n) {
        for c in list.iter_mut() {
            refresh_ids(c, taken);
        }
    }
}

/// Copie d'un nœud insérée juste après lui. Renvoie la commande et l'identifiant de la copie.
pub fn duplicate(site: &Site, at: &Target) -> Result<(Command, String), EditError> {
    let (parent, index) = address::locate(site, at)?;
    let mut copy = address::node(site, at)?.clone();
    let mut taken = site.node_ids();
    refresh_ids(&mut copy, &mut taken);
    let id = revel_model::ids::node_id(&copy)
        .unwrap_or_default()
        .to_string();
    Ok((
        Command::Insert {
            parent,
            index: index + 1,
            node: copy,
        },
        id,
    ))
}

/// Enveloppe un nœud dans un nouvel élément (`div` par défaut). Renvoie la commande et
/// l'identifiant de l'enveloppe.
pub fn wrap(site: &Site, at: &Target, tag: &str) -> Result<(Command, String), EditError> {
    let (parent, index) = address::locate(site, at)?;
    let taken = site.node_ids();
    let id = fresh(&taken);
    let wrapper = Node::Element {
        id: Some(id.clone()),
        tag: tag.to_string(),
        attrs: Vec::new(),
        children: Vec::new(),
    };
    let wrapper_target = Target {
        owner: at.owner.clone(),
        node: id.clone(),
    };
    Ok((
        Command::Batch {
            commands: vec![
                Command::Insert {
                    parent,
                    index,
                    node: wrapper,
                },
                Command::Move {
                    at: at.clone(),
                    to: Parent::of(&wrapper_target, Branch::Children),
                    index: 0,
                },
            ],
        },
        id,
    ))
}

/// Éléments proposés par le panneau « Ajouter ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    Section,
    Container,
    Heading,
    Paragraph,
    Link,
    Button,
    Image,
    List,
}

impl Preset {
    pub const ALL: [Preset; 8] = [
        Preset::Section,
        Preset::Container,
        Preset::Heading,
        Preset::Paragraph,
        Preset::Link,
        Preset::Button,
        Preset::Image,
        Preset::List,
    ];

    /// Nouveau nœud (identifiants libres dans `site`). Les textes par défaut sont fournis par
    /// l'appelant (traduits dans la langue de l'interface).
    pub fn node(self, site: &Site, text: &str) -> Node {
        let mut taken = site.node_ids();
        let mut id = || {
            let i = fresh(&taken);
            taken.insert(i.clone());
            Some(i)
        };
        let el =
            |id: Option<String>, tag: &str, attrs: Vec<Attr>, children: Vec<Node>| Node::Element {
                id,
                tag: tag.into(),
                attrs,
                children,
            };
        let attr = |n: &str, v: &str| Attr {
            name: n.into(),
            value: Some(v.into()),
            when: None,
        };
        let txt = |t: &str| Node::Text { text: t.into() };
        match self {
            Preset::Section => el(id(), "section", vec![], vec![]),
            Preset::Container => el(id(), "div", vec![], vec![]),
            Preset::Heading => el(id(), "h2", vec![], vec![txt(text)]),
            Preset::Paragraph => el(id(), "p", vec![], vec![txt(text)]),
            Preset::Link => el(id(), "a", vec![attr("href", "#")], vec![txt(text)]),
            Preset::Button => el(
                id(),
                "a",
                vec![attr("href", "#"), attr("class", "button")],
                vec![txt(text)],
            ),
            Preset::Image => Node::Image(ImageNode {
                id: id(),
                src: "\"\"".into(),
                alt: Some(format!("\"{}\"", text.replace('"', "'"))),
                ..Default::default()
            }),
            Preset::List => {
                let ul = id();
                let li1 = id();
                let li2 = id();
                el(
                    ul,
                    "ul",
                    vec![],
                    vec![
                        el(li1, "li", vec![], vec![txt(text)]),
                        el(li2, "li", vec![], vec![txt(text)]),
                    ],
                )
            }
        }
    }
}

/// Identifiant libre pour le design propre d'un contenu : `<gabarit>--<fichier>` (suffixe
/// `-2`, `-3`… si pris).
pub fn own_layout_id(site: &Site, template: &str, file: &str) -> String {
    let base = format!("{template}--{file}");
    let mut id = base.clone();
    let mut n = 2;
    while site.layouts.iter().any(|l| l.id == id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Copie du gabarit `from` sous l'identifiant `id` (même parent, mêmes variables), avec de
/// nouveaux identifiants de nœuds.
pub fn layout_copy(site: &Site, from: &str, id: &str) -> Result<revel_model::Layout, EditError> {
    let src = site
        .layout(from)
        .ok_or_else(|| EditError::NotFound(from.to_string()))?;
    let mut taken = site.node_ids();
    let mut body = src.body.clone();
    for n in body.iter_mut() {
        refresh_ids(n, &mut taken);
    }
    Ok(revel_model::Layout {
        id: id.to_string(),
        parent: src.parent.clone(),
        props: src.props.clone(),
        body,
    })
}
