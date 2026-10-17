//! Texte mis en forme modifié d'un bloc (D-72) : un élément qui ne contient que du texte et des
//! éléments de mise en forme (`strong`, `em`, `u`, `a`, `span`, `br`…) se modifie comme un seul
//! texte, sur la page comme dans le panneau. Le contenu modifié revient en morceaux
//! ([`Piece`]) ; les éléments déjà présents gardent leur identifiant, leurs classes et leurs autres
//! attributs.

use std::collections::{HashMap, HashSet};

use revel_model::ids::{node_id, walk};
use revel_model::{Attr, Node, Site};
use serde::{Deserialize, Serialize};

use crate::EditError;
use crate::address::{self, Target};
use crate::command::Command;

/// Balises de mise en forme modifiables avec le texte.
pub const INLINE_TAGS: &[&str] = &[
    "strong", "b", "em", "i", "u", "a", "span", "br", "small", "sup", "sub", "mark", "s", "code",
];

/// Morceau d'un texte mis en forme : texte, ou élément de mise en forme (`id` : élément déjà
/// présent, repris tel quel avec ses attributs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Piece {
    Text {
        text: String,
    },
    Element {
        tag: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Classes d'un élément nouveau (un élément repris garde les siennes).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        class: Option<String>,
        /// Adresse et cible d'un lien (reprises aussi pour un lien existant : la barre de mise
        /// en forme les modifie).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        href: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<Piece>,
    },
}

/// Attribut sans expression ni condition.
fn plain_attr(a: &Attr) -> bool {
    a.when.is_none()
        && !a
            .value
            .as_deref()
            .is_some_and(|v| v.contains("{{") || v.contains("{%"))
}

/// Contenu fait seulement de texte et d'éléments de mise en forme sans expression.
fn inline_only(children: &[Node]) -> bool {
    children.iter().all(|c| match c {
        Node::Text { .. } => true,
        Node::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            INLINE_TAGS.contains(&tag.to_ascii_lowercase().as_str())
                && attrs.iter().all(plain_attr)
                && inline_only(children)
        }
        _ => false,
    })
}

/// Élément modifiable d'un bloc : du texte et au moins un élément de mise en forme.
pub fn is_rich_host(n: &Node) -> bool {
    match n {
        Node::Element { children, .. } => {
            children.iter().any(|c| matches!(c, Node::Element { .. }))
                && children.iter().any(|c| match c {
                    Node::Text { text } => !text.trim().is_empty(),
                    Node::Element { children, .. } => !children.is_empty(),
                    _ => false,
                })
                && inline_only(children)
        }
        _ => false,
    }
}

/// Identifiants des éléments du site modifiables d'un bloc (envoyés au canevas).
pub fn rich_hosts(site: &Site) -> Vec<String> {
    let mut out = Vec::new();
    for (_, _, body) in site.owners() {
        walk(body, &mut |n| {
            if is_rich_host(n)
                && let Some(id) = node_id(n)
            {
                out.push(id.to_string());
            }
        });
    }
    out
}

/// Texte de tous les nœuds texte (typographie de la source).
fn text_of(nodes: &[Node], out: &mut String) {
    for n in nodes {
        match n {
            Node::Text { text } => out.push_str(text),
            Node::Element { children, .. } => text_of(children, out),
            _ => {}
        }
    }
}

/// Éléments du contenu actuel, par identifiant : balise et attributs.
fn elements_of(nodes: &[Node], out: &mut HashMap<String, (String, Vec<Attr>)>) {
    for n in nodes {
        if let Node::Element {
            id,
            tag,
            attrs,
            children,
        } = n
        {
            if let Some(id) = id {
                out.insert(id.clone(), (tag.to_ascii_lowercase(), attrs.clone()));
            }
            elements_of(children, out);
        }
    }
}

struct Build<'a> {
    old: HashMap<String, (String, Vec<Attr>)>,
    used: HashSet<String>,
    taken: HashSet<String>,
    source: &'a str,
    typo: bool,
}

impl Build<'_> {
    fn nodes(&mut self, pieces: &[Piece]) -> Vec<Node> {
        let mut out: Vec<Node> = Vec::new();
        for p in pieces {
            match p {
                Piece::Text { text } => {
                    let t = if self.typo {
                        crate::canvas::untypo(text, self.source)
                    } else {
                        text.clone()
                    };
                    if t.is_empty() {
                        continue;
                    }
                    // Textes voisins réunis (comme dans la page).
                    if let Some(Node::Text { text: prev }) = out.last_mut() {
                        prev.push_str(&t);
                    } else {
                        out.push(Node::Text { text: t });
                    }
                }
                Piece::Element {
                    tag,
                    id,
                    class,
                    href,
                    target,
                    children,
                } => {
                    let tag = match tag.to_ascii_lowercase().as_str() {
                        "b" => "strong".to_string(),
                        "i" => "em".to_string(),
                        t => t.to_string(),
                    };
                    if !INLINE_TAGS.contains(&tag.as_str()) {
                        // Balise inconnue : son texte seul.
                        let inner = self.nodes(children);
                        for n in inner {
                            match (n, out.last_mut()) {
                                (Node::Text { text }, Some(Node::Text { text: prev })) => {
                                    prev.push_str(&text)
                                }
                                (n, _) => out.push(n),
                            }
                        }
                        continue;
                    }
                    let kids = if tag == "br" {
                        Vec::new()
                    } else {
                        self.nodes(children)
                    };
                    // Élément vide (texte effacé) : retiré, sauf un retour à la ligne.
                    if kids.is_empty() && tag != "br" {
                        continue;
                    }
                    // Élément repris : même balise, première occurrence (un élément copié dans le
                    // texte reçoit un nouvel identifiant).
                    let reused = id.as_ref().and_then(|i| {
                        let (t, attrs) = self.old.get(i)?;
                        (*t == tag && !self.used.contains(i)).then(|| (i.clone(), attrs.clone()))
                    });
                    let (nid, mut attrs) = match reused {
                        Some((i, attrs)) => {
                            self.used.insert(i.clone());
                            (i, attrs)
                        }
                        None => {
                            // Copie d'un élément existant : mêmes attributs, nouvel identifiant.
                            let attrs = id
                                .as_ref()
                                .and_then(|i| self.old.get(i))
                                .filter(|(t, _)| *t == tag)
                                .map(|(_, a)| a.clone())
                                .unwrap_or_else(|| {
                                    class
                                        .iter()
                                        .filter(|c| !c.trim().is_empty())
                                        .map(|c| Attr {
                                            name: "class".into(),
                                            value: Some(c.trim().to_string()),
                                            when: None,
                                        })
                                        .collect()
                                });
                            let fresh = revel_model::ids::fresh(&self.taken);
                            self.taken.insert(fresh.clone());
                            (fresh, attrs)
                        }
                    };
                    if tag == "a" {
                        set_attr(&mut attrs, "href", href.as_deref());
                        set_attr(&mut attrs, "target", target.as_deref());
                    }
                    out.push(Node::Element {
                        id: Some(nid),
                        tag,
                        attrs,
                        children: kids,
                    });
                }
            }
        }
        out
    }
}

/// Attribut posé (à sa place s'il existait), ou retiré (`None`).
fn set_attr(attrs: &mut Vec<Attr>, name: &str, value: Option<&str>) {
    match (attrs.iter().position(|a| a.name == name), value) {
        (Some(i), Some(v)) => attrs[i].value = Some(v.to_string()),
        (Some(i), None) => {
            attrs.remove(i);
        }
        (None, Some(v)) => attrs.push(Attr {
            name: name.into(),
            value: Some(v.into()),
            when: None,
        }),
        (None, None) => {}
    }
}

/// Commande qui remplace le contenu de l'élément `at` par `pieces` (`typo` : typographie
/// française de la page, retirée si la source ne l'utilisait pas). `None` : contenu inchangé.
pub fn set_inline(
    site: &Site,
    at: &Target,
    pieces: &[Piece],
    typo: bool,
) -> Result<Option<Command>, EditError> {
    let node = address::node(site, at)?;
    let Node::Element {
        id,
        tag,
        attrs,
        children,
    } = node
    else {
        return Err(EditError::NotAnElement(at.node.clone()));
    };
    if !inline_only(children) {
        return Err(EditError::WrongKind(at.node.clone()));
    }
    let mut source = String::new();
    text_of(children, &mut source);
    let mut old = HashMap::new();
    elements_of(children, &mut old);
    let mut b = Build {
        old,
        used: HashSet::new(),
        taken: site.node_ids(),
        source: &source,
        typo,
    };
    let mut kids = b.nodes(pieces);
    // Retour à la ligne final laissé par le navigateur : retiré.
    while matches!(kids.last(), Some(Node::Element { tag, .. }) if tag == "br") && kids.len() > 1 {
        kids.pop();
    }
    if kids == *children {
        return Ok(None);
    }
    let (parent, index) = address::locate(site, at)?;
    Ok(Some(Command::Replace {
        parent,
        index,
        node: Node::Element {
            id: id.clone(),
            tag: tag.clone(),
            attrs: attrs.clone(),
            children: kids,
        },
    }))
}
