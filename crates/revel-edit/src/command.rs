//! Commandes d'édition. Chaque commande appliquée renvoie la commande inverse.

use revel_model::ids::{node_id, walk};
use revel_model::{Attr, ImageNode, Node, Site};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::EditError;
use crate::address::{self, Parent, Target};

/// Modification du document de site. Sérialisable : la même commande vient de l'interface, de la
/// ligne de commande ou du serveur MCP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    /// Donne une valeur à un attribut (`None` : attribut booléen), créé à la fin s'il n'existe pas.
    SetAttr {
        at: Target,
        name: String,
        #[serde(default)]
        value: Option<String>,
    },
    /// Retire un attribut (par son nom).
    RemoveAttr { at: Target, name: String },
    /// Insère un attribut à une position (inverse d'un retrait).
    InsertAttr {
        at: Target,
        index: usize,
        attr: Attr,
    },
    /// Retire l'attribut d'une position (inverse d'une insertion).
    RemoveAttrAt { at: Target, index: usize },
    /// Change la balise d'un élément (`div` -> `section`).
    SetTag { at: Target, tag: String },
    /// Remplace un nœud texte.
    SetText {
        parent: Parent,
        index: usize,
        text: String,
    },
    /// Remplace un nœud quelconque à une position (expression, HTML, affectation…).
    Replace {
        parent: Parent,
        index: usize,
        node: Node,
    },
    /// Remplace les paramètres d'une image (l'identifiant est conservé).
    SetImage { at: Target, image: ImageNode },
    /// Change l'expression d'une condition ou d'une boucle (`cond` ou `expr`/`item`).
    SetLogic {
        at: Target,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cond: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expr: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        item: Option<String>,
    },
    /// Insère un nœud (et ses enfants) à une position.
    Insert {
        parent: Parent,
        index: usize,
        node: Node,
    },
    /// Supprime un nœud structurel.
    Remove { at: Target },
    /// Supprime le nœud d'une position.
    RemoveAt { parent: Parent, index: usize },
    /// Déplace un nœud ; `index` est la position dans la liste d'arrivée, après le retrait.
    Move {
        at: Target,
        to: Parent,
        index: usize,
    },
    /// Variable d'une page (`None` : supprimée).
    SetPageProp {
        page: String,
        key: String,
        #[serde(default)]
        value: Option<Value>,
    },
    /// Suite de commandes appliquées ensemble (annulées ensemble).
    Batch { commands: Vec<Command> },
    /// Duplique un nœud (copie insérée après lui, nouveaux identifiants).
    Duplicate { at: Target },
    /// Enveloppe un nœud dans un nouvel élément.
    Wrap { at: Target, tag: String },
    /// Insère un élément du panneau « Ajouter » ; `text` : texte par défaut (traduit).
    InsertPreset {
        parent: Parent,
        index: usize,
        preset: crate::make::Preset,
        #[serde(default)]
        text: String,
    },
}

/// Remplace une commande « d'intention » (dupliquer, envelopper, ajouter) par les commandes
/// concrètes équivalentes, avec leurs nouveaux identifiants. C'est la forme concrète qui est
/// enregistrée dans l'historique : rétablir redonne exactement les mêmes identifiants.
pub fn expand(site: &Site, cmd: &Command) -> Result<Command, EditError> {
    Ok(match cmd {
        Command::Duplicate { at } => crate::make::duplicate(site, at)?.0,
        Command::Wrap { at, tag } => {
            if !valid_tag(tag) {
                return Err(EditError::InvalidTag(tag.clone()));
            }
            crate::make::wrap(site, at, tag)?.0
        }
        Command::InsertPreset {
            parent,
            index,
            preset,
            text,
        } => Command::Insert {
            parent: parent.clone(),
            index: *index,
            node: preset.node(site, text),
        },
        Command::Batch { commands } => {
            // Les intentions d'une suite sont développées au fur et à mesure (identifiants libres
            // compte tenu des commandes précédentes) : développement sur une copie.
            let mut scratch = site.clone();
            let mut out = Vec::with_capacity(commands.len());
            for c in commands {
                let e = expand(&scratch, c)?;
                apply(&mut scratch, &e)?;
                out.push(e);
            }
            Command::Batch { commands: out }
        }
        other => other.clone(),
    })
}

fn element_attrs<'a>(site: &'a mut Site, at: &Target) -> Result<&'a mut Vec<Attr>, EditError> {
    match address::node_mut(site, at)? {
        Node::Element { attrs, .. } => Ok(attrs),
        _ => Err(EditError::NotAnElement(at.node.clone())),
    }
}

/// Identifiants contenus dans un nœud (lui compris).
fn ids_of(n: &Node) -> Vec<String> {
    let mut v = Vec::new();
    walk(std::slice::from_ref(n), &mut |x| {
        if let Some(id) = node_id(x) {
            v.push(id.to_string());
        }
    });
    v
}

/// Vérifie qu'un nœud à insérer n'introduit aucun identifiant déjà présent dans le site.
fn check_new_ids(site: &Site, n: &Node) -> Result<(), EditError> {
    let taken = site.node_ids();
    let mut seen = std::collections::HashSet::new();
    for id in ids_of(n) {
        if taken.contains(&id) || !seen.insert(id.clone()) {
            return Err(EditError::DuplicateId(id));
        }
    }
    Ok(())
}

/// Applique une commande et renvoie son inverse. En cas d'échec, le site est inchangé.
pub fn apply(site: &mut Site, cmd: &Command) -> Result<Command, EditError> {
    match cmd {
        Command::SetAttr { at, name, value } => {
            let attrs = element_attrs(site, at)?;
            match attrs.iter_mut().position(|a| &a.name == name) {
                Some(i) => {
                    let old = std::mem::replace(&mut attrs[i].value, value.clone());
                    Ok(Command::SetAttr {
                        at: at.clone(),
                        name: name.clone(),
                        value: old,
                    })
                }
                None => {
                    attrs.push(Attr {
                        name: name.clone(),
                        value: value.clone(),
                        when: None,
                    });
                    Ok(Command::RemoveAttrAt {
                        at: at.clone(),
                        index: attrs.len() - 1,
                    })
                }
            }
        }
        Command::RemoveAttr { at, name } => {
            let attrs = element_attrs(site, at)?;
            let i = attrs
                .iter()
                .position(|a| &a.name == name)
                .ok_or_else(|| EditError::NotFound(format!("attribut {name}")))?;
            let attr = attrs.remove(i);
            Ok(Command::InsertAttr {
                at: at.clone(),
                index: i,
                attr,
            })
        }
        Command::InsertAttr { at, index, attr } => {
            let attrs = element_attrs(site, at)?;
            if *index > attrs.len() {
                return Err(EditError::Index(*index));
            }
            attrs.insert(*index, attr.clone());
            Ok(Command::RemoveAttrAt {
                at: at.clone(),
                index: *index,
            })
        }
        Command::RemoveAttrAt { at, index } => {
            let attrs = element_attrs(site, at)?;
            if *index >= attrs.len() {
                return Err(EditError::Index(*index));
            }
            let attr = attrs.remove(*index);
            Ok(Command::InsertAttr {
                at: at.clone(),
                index: *index,
                attr,
            })
        }
        Command::SetTag { at, tag } => {
            if !valid_tag(tag) {
                return Err(EditError::InvalidTag(tag.clone()));
            }
            match address::node_mut(site, at)? {
                Node::Element { tag: t, .. } => Ok(Command::SetTag {
                    at: at.clone(),
                    tag: std::mem::replace(t, tag.clone()),
                }),
                _ => Err(EditError::NotAnElement(at.node.clone())),
            }
        }
        Command::SetText {
            parent,
            index,
            text,
        } => {
            let list = address::list_mut(site, parent)?;
            match list.get_mut(*index) {
                Some(Node::Text { text: t }) => Ok(Command::SetText {
                    parent: parent.clone(),
                    index: *index,
                    text: std::mem::replace(t, text.clone()),
                }),
                Some(_) => Err(EditError::NotText(*index)),
                None => Err(EditError::Index(*index)),
            }
        }
        Command::Replace {
            parent,
            index,
            node,
        } => {
            let old = address::list(site, parent)?
                .get(*index)
                .cloned()
                .ok_or(EditError::Index(*index))?;
            // Les identifiants du nœud remplacé sont libérés avant la vérification.
            let freed: std::collections::HashSet<String> = ids_of(&old).into_iter().collect();
            let taken = site.node_ids();
            for id in ids_of(node) {
                if taken.contains(&id) && !freed.contains(&id) {
                    return Err(EditError::DuplicateId(id));
                }
            }
            let list = address::list_mut(site, parent)?;
            list[*index] = node.clone();
            Ok(Command::Replace {
                parent: parent.clone(),
                index: *index,
                node: old,
            })
        }
        Command::SetImage { at, image } => match address::node_mut(site, at)? {
            Node::Image(img) => {
                let mut new = image.clone();
                new.id = img.id.clone();
                let old = std::mem::replace(img, new);
                Ok(Command::SetImage {
                    at: at.clone(),
                    image: old,
                })
            }
            _ => Err(EditError::WrongKind(at.node.clone())),
        },
        Command::SetLogic {
            at,
            cond,
            expr,
            item,
        } => match address::node_mut(site, at)? {
            Node::If { cond: c, .. } => {
                let new = cond.clone().ok_or(EditError::WrongKind(at.node.clone()))?;
                Ok(Command::SetLogic {
                    at: at.clone(),
                    cond: Some(std::mem::replace(c, new)),
                    expr: None,
                    item: None,
                })
            }
            Node::Each {
                expr: e, item: i, ..
            } => {
                let old_e = expr.as_ref().map(|x| std::mem::replace(e, x.clone()));
                let old_i = item.as_ref().map(|x| std::mem::replace(i, x.clone()));
                Ok(Command::SetLogic {
                    at: at.clone(),
                    cond: None,
                    expr: old_e,
                    item: old_i,
                })
            }
            _ => Err(EditError::WrongKind(at.node.clone())),
        },
        Command::Insert {
            parent,
            index,
            node,
        } => {
            check_new_ids(site, node)?;
            let list = address::list_mut(site, parent)?;
            if *index > list.len() {
                return Err(EditError::Index(*index));
            }
            list.insert(*index, node.clone());
            Ok(Command::RemoveAt {
                parent: parent.clone(),
                index: *index,
            })
        }
        Command::Remove { at } => {
            let (parent, index) = address::locate(site, at)?;
            apply(site, &Command::RemoveAt { parent, index })
        }
        Command::RemoveAt { parent, index } => {
            let list = address::list_mut(site, parent)?;
            if *index >= list.len() {
                return Err(EditError::Index(*index));
            }
            let node = list.remove(*index);
            Ok(Command::Insert {
                parent: parent.clone(),
                index: *index,
                node,
            })
        }
        Command::Move { at, to, index } => {
            let (from, from_index) = address::locate(site, at)?;
            // Interdit : déplacer un nœud dans lui-même ou dans un de ses descendants.
            if let Some(dest) = &to.node {
                let moved = address::node(site, at)?;
                if ids_of(moved).iter().any(|i| i == dest) {
                    return Err(EditError::InvalidMove);
                }
            }
            address::list(site, to)?; // destination valide avant toute modification
            let node = address::list_mut(site, &from)?.remove(from_index);
            let dest = match address::list_mut(site, to) {
                Ok(d) if *index <= d.len() => d,
                Ok(_) | Err(_) => {
                    // Remise en place si la position d'arrivée est invalide.
                    address::list_mut(site, &from)?.insert(from_index, node);
                    return Err(EditError::Index(*index));
                }
            };
            dest.insert(*index, node);
            // Le nœud appartient désormais au propriétaire d'arrivée (page, gabarit, composant).
            Ok(Command::Move {
                at: Target {
                    owner: to.owner.clone(),
                    node: at.node.clone(),
                },
                to: from,
                index: from_index,
            })
        }
        Command::SetPageProp { page, key, value } => {
            let p = site
                .pages
                .iter_mut()
                .find(|p| &p.id == page)
                .ok_or_else(|| EditError::NotFound(format!("page {page}")))?;
            let old = match value {
                Some(v) => p.props.insert(key.clone(), v.clone()),
                None => p.props.remove(key),
            };
            Ok(Command::SetPageProp {
                page: page.clone(),
                key: key.clone(),
                value: old,
            })
        }
        Command::Duplicate { .. } | Command::Wrap { .. } | Command::InsertPreset { .. } => {
            let concrete = expand(site, cmd)?;
            apply(site, &concrete)
        }
        Command::Batch { commands } => {
            let mut inverses: Vec<Command> = Vec::with_capacity(commands.len());
            for c in commands {
                match apply(site, c) {
                    Ok(inv) => inverses.push(inv),
                    Err(e) => {
                        // Annulation de ce qui a déjà été appliqué : la suite est atomique.
                        for inv in inverses.iter().rev() {
                            let _ = apply(site, inv);
                        }
                        return Err(e);
                    }
                }
            }
            inverses.reverse();
            Ok(Command::Batch { commands: inverses })
        }
    }
}

/// Nom de balise HTML acceptable (lettres, chiffres, tirets ; commence par une lettre).
fn valid_tag(t: &str) -> bool {
    let mut c = t.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic())
        && c.all(|x| x.is_ascii_alphanumeric() || x == '-')
}

impl Command {
    /// Nœud principal visé (pour la sélection après une commande).
    pub fn target(&self) -> Option<&Target> {
        match self {
            Self::SetAttr { at, .. }
            | Self::RemoveAttr { at, .. }
            | Self::InsertAttr { at, .. }
            | Self::RemoveAttrAt { at, .. }
            | Self::SetTag { at, .. }
            | Self::SetImage { at, .. }
            | Self::SetLogic { at, .. }
            | Self::Remove { at }
            | Self::Move { at, .. }
            | Self::Duplicate { at }
            | Self::Wrap { at, .. } => Some(at),
            Self::Batch { commands } => commands.iter().find_map(Self::target),
            _ => None,
        }
    }

    /// Deux commandes successives peuvent-elles être fusionnées dans l'historique (saisie continue
    /// dans un champ) ? Même genre et même cible.
    pub fn same_field(&self, other: &Command) -> bool {
        match (self, other) {
            (Self::SetAttr { at: a, name: n, .. }, Self::SetAttr { at: b, name: m, .. }) => {
                a == b && n == m
            }
            (
                Self::SetText {
                    parent: a,
                    index: i,
                    ..
                },
                Self::SetText {
                    parent: b,
                    index: j,
                    ..
                },
            ) => a == b && i == j,
            (
                Self::SetPageProp {
                    page: a, key: k, ..
                },
                Self::SetPageProp {
                    page: b, key: l, ..
                },
            ) => a == b && k == l,
            (Self::SetLogic { at: a, .. }, Self::SetLogic { at: b, .. }) => a == b,
            _ => false,
        }
    }
}
