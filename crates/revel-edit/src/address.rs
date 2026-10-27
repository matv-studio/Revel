//! Adresses dans le document : propriétaire d'un arbre, nœud, emplacement d'insertion.

use revel_model::ids::{child_lists, child_lists_mut, node_id};
use revel_model::{Node, Site};
use serde::{Deserialize, Serialize};

use crate::EditError;

/// Arbre de nœuds modifiable : page, gabarit ou composant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Owner {
    Page(String),
    Layout(String),
    Component(String),
}

/// Liste d'enfants d'un nœud.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Branch {
    /// Enfants d'un élément, d'un bloc ou d'une boucle ; corps de la page à la racine.
    #[default]
    Children,
    /// Condition vraie.
    Then,
    /// Condition fausse.
    Otherwise,
    /// Boucle sur une liste vide.
    Empty,
}

/// Nœud structurel désigné par son identifiant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Target {
    pub owner: Owner,
    pub node: String,
}

/// Liste d'enfants : corps du propriétaire (`node: None`) ou branche d'un nœud.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Parent {
    pub owner: Owner,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(default)]
    pub branch: Branch,
}

impl Parent {
    pub fn root(owner: Owner) -> Self {
        Self {
            owner,
            node: None,
            branch: Branch::Children,
        }
    }

    pub fn of(target: &Target, branch: Branch) -> Self {
        Self {
            owner: target.owner.clone(),
            node: Some(target.node.clone()),
            branch,
        }
    }
}

impl Owner {
    pub fn describe(&self) -> String {
        match self {
            Self::Page(id) => format!("page {id}"),
            Self::Layout(id) => format!("gabarit {id}"),
            Self::Component(id) => format!("composant {id}"),
        }
    }
}

pub(crate) fn body<'a>(site: &'a Site, owner: &Owner) -> Result<&'a Vec<Node>, EditError> {
    let b = match owner {
        Owner::Page(id) => site.pages.iter().find(|p| &p.id == id).map(|p| &p.body),
        Owner::Layout(id) => site.layouts.iter().find(|l| &l.id == id).map(|l| &l.body),
        Owner::Component(id) => site
            .components
            .iter()
            .find(|c| &c.id == id)
            .map(|c| &c.body),
    };
    b.ok_or_else(|| EditError::NotFound(owner.describe()))
}

pub(crate) fn body_mut<'a>(
    site: &'a mut Site,
    owner: &Owner,
) -> Result<&'a mut Vec<Node>, EditError> {
    let b = match owner {
        Owner::Page(id) => site
            .pages
            .iter_mut()
            .find(|p| &p.id == id)
            .map(|p| &mut p.body),
        Owner::Layout(id) => site
            .layouts
            .iter_mut()
            .find(|l| &l.id == id)
            .map(|l| &mut l.body),
        Owner::Component(id) => site
            .components
            .iter_mut()
            .find(|c| &c.id == id)
            .map(|c| &mut c.body),
    };
    b.ok_or_else(|| EditError::NotFound(owner.describe()))
}

pub(crate) fn find<'a>(nodes: &'a [Node], id: &str) -> Option<&'a Node> {
    for n in nodes {
        if node_id(n) == Some(id) {
            return Some(n);
        }
        for list in child_lists(n) {
            if let Some(f) = find(list, id) {
                return Some(f);
            }
        }
    }
    None
}

pub(crate) fn find_mut<'a>(nodes: &'a mut [Node], id: &str) -> Option<&'a mut Node> {
    for n in nodes.iter_mut() {
        if node_id(n) == Some(id) {
            return Some(n);
        }
        for list in child_lists_mut(n) {
            if let Some(f) = find_mut(list, id) {
                return Some(f);
            }
        }
    }
    None
}

/// Branches disponibles d'un nœud, dans l'ordre de `child_lists`.
pub(crate) fn branches(n: &Node) -> &'static [Branch] {
    match n {
        Node::Element { .. } | Node::SetBlock { .. } | Node::Use { .. } => &[Branch::Children],
        Node::If { .. } => &[Branch::Then, Branch::Otherwise],
        Node::Each { .. } => &[Branch::Children, Branch::Empty],
        _ => &[],
    }
}

/// Parent et position d'un nœud dans un arbre.
fn locate_in(
    nodes: &[Node],
    id: &str,
    parent: Option<&Node>,
    branch: Branch,
) -> Option<(Option<String>, Branch, usize)> {
    for (i, n) in nodes.iter().enumerate() {
        if node_id(n) == Some(id) {
            return Some((parent.and_then(node_id).map(str::to_string), branch, i));
        }
        for (list, b) in child_lists(n).into_iter().zip(branches(n)) {
            if let Some(r) = locate_in(list, id, Some(n), *b) {
                return Some(r);
            }
        }
    }
    None
}

/// Emplacement d'un nœud : liste qui le contient et position.
pub fn locate(site: &Site, at: &Target) -> Result<(Parent, usize), EditError> {
    let b = body(site, &at.owner)?;
    let (node, branch, index) =
        locate_in(b, &at.node, None, Branch::Children).ok_or_else(|| {
            EditError::NotFound(format!("nœud {} ({})", at.node, at.owner.describe()))
        })?;
    Ok((
        Parent {
            owner: at.owner.clone(),
            node,
            branch,
        },
        index,
    ))
}

pub fn node<'a>(site: &'a Site, at: &Target) -> Result<&'a Node, EditError> {
    find(body(site, &at.owner)?, &at.node)
        .ok_or_else(|| EditError::NotFound(format!("nœud {} ({})", at.node, at.owner.describe())))
}

pub(crate) fn node_mut<'a>(site: &'a mut Site, at: &Target) -> Result<&'a mut Node, EditError> {
    let what = format!("nœud {} ({})", at.node, at.owner.describe());
    find_mut(body_mut(site, &at.owner)?, &at.node).ok_or(EditError::NotFound(what))
}

/// Première classe écrite de l'élément ancêtre le plus proche qui en porte une (règles « balises
/// dans une classe » du panneau Styles, D-151).
pub fn class_ancestor(site: &Site, at: &Target) -> Option<String> {
    let mut cur = at.clone();
    for _ in 0..64 {
        let (parent, _) = locate(site, &cur).ok()?;
        let id = parent.node?;
        cur = Target {
            owner: cur.owner.clone(),
            node: id,
        };
        if let Ok(Node::Element { attrs, .. }) = node(site, &cur)
            && let Some(c) = attrs
                .iter()
                .find(|a| a.name == "class" && a.when.is_none())
                .and_then(|a| a.value.as_deref())
                .filter(|v| !v.contains("{{") && !v.contains("{%"))
                .and_then(|v| v.split_whitespace().next())
        {
            return Some(c.to_string());
        }
    }
    None
}

/// Liste d'enfants désignée par `parent`.
pub fn list<'a>(site: &'a Site, parent: &Parent) -> Result<&'a Vec<Node>, EditError> {
    let b = body(site, &parent.owner)?;
    let Some(id) = &parent.node else {
        return if parent.branch == Branch::Children {
            Ok(b)
        } else {
            Err(EditError::NoBranch)
        };
    };
    let n = find(b, id).ok_or_else(|| EditError::NotFound(format!("nœud {id}")))?;
    let pos = branches(n)
        .iter()
        .position(|x| *x == parent.branch)
        .ok_or(EditError::NoBranch)?;
    Ok(child_lists(n).swap_remove(pos))
}

pub(crate) fn list_mut<'a>(
    site: &'a mut Site,
    parent: &Parent,
) -> Result<&'a mut Vec<Node>, EditError> {
    let b = body_mut(site, &parent.owner)?;
    let Some(id) = &parent.node else {
        return if parent.branch == Branch::Children {
            Ok(b)
        } else {
            Err(EditError::NoBranch)
        };
    };
    let n = find_mut(b, id).ok_or_else(|| EditError::NotFound(format!("nœud {id}")))?;
    let pos = branches(n)
        .iter()
        .position(|x| *x == parent.branch)
        .ok_or(EditError::NoBranch)?;
    Ok(child_lists_mut(n).swap_remove(pos))
}
