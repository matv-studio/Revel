//! Calques : nom donné par l'utilisateur et calque masqué (élément, utilisation de composant,
//! image), comme dans Framer et Figma.
//!
//! Le nom et l'état masqué sont portés par le nœud : attributs `data-revel-name` et
//! `data-revel-hidden` d'un élément ou d'une utilisation de composant, champs `name` et `hidden`
//! d'une image. Le nom n'est jamais écrit dans les pages ; un calque masqué n'est pas écrit dans
//! le site publié.

use crate::{Command, EditError, Target, address};
use revel_model::{HIDDEN_ATTR, LAYER_NAME_ATTR, Node};

/// Le calque peut recevoir un nom ou être masqué.
pub fn supported(n: &Node) -> bool {
    matches!(n, Node::Element { .. } | Node::Use { .. } | Node::Image(_))
}

fn attr_command(
    n: &Node,
    at: &Target,
    name: &str,
    value: Option<Option<String>>,
) -> Option<Command> {
    let attrs = match n {
        Node::Element { attrs, .. } | Node::Use { attrs, .. } => attrs,
        _ => return None,
    };
    let has = attrs.iter().find(|a| a.name == name);
    match value {
        Some(v) => {
            if has.is_some_and(|a| a.value == v && a.when.is_none()) {
                return None;
            }
            Some(Command::SetAttr {
                at: at.clone(),
                name: name.to_string(),
                value: v,
            })
        }
        None => has.map(|_| Command::RemoveAttr {
            at: at.clone(),
            name: name.to_string(),
        }),
    }
}

/// Commande qui donne au calque `at` le nom `name` (vide : nom automatique) ; `None` si rien ne
/// change.
pub fn rename(
    site: &revel_model::Site,
    at: &Target,
    name: &str,
) -> Result<Option<Command>, EditError> {
    let n = address::node(site, at)?;
    if !supported(n) {
        return Err(EditError::NotAnElement(at.node.clone()));
    }
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let value = (!name.is_empty()).then_some(name);
    Ok(match n {
        Node::Image(img) => (img.name != value).then(|| {
            let mut image = img.clone();
            image.name = value;
            Command::SetImage {
                at: at.clone(),
                image,
            }
        }),
        _ => attr_command(n, at, LAYER_NAME_ATTR, value.map(Some)),
    })
}

/// Commande qui masque ou affiche le calque `at` ; `None` si rien ne change.
pub fn set_hidden(
    site: &revel_model::Site,
    at: &Target,
    hidden: bool,
) -> Result<Option<Command>, EditError> {
    let n = address::node(site, at)?;
    if !supported(n) {
        return Err(EditError::NotAnElement(at.node.clone()));
    }
    Ok(match n {
        Node::Image(img) => (img.hidden != hidden).then(|| {
            let mut image = img.clone();
            image.hidden = hidden;
            Command::SetImage {
                at: at.clone(),
                image,
            }
        }),
        _ if revel_model::is_hidden(n) == hidden => None,
        _ => attr_command(n, at, HIDDEN_ATTR, hidden.then_some(None)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Owner, apply};
    use revel_model::{ImageNode, Site};

    fn site() -> Site {
        let mut s = Site::new("Essai", "fr");
        let mut p: revel_model::Page =
            serde_json::from_value(serde_json::json!({ "id": "index", "path": "/" })).unwrap();
        p.body = vec![
            Node::Element {
                id: Some("h".into()),
                tag: "section".into(),
                attrs: vec![],
                children: vec![],
            },
            Node::Image(ImageNode {
                id: Some("i".into()),
                src: "\"/media/a.jpg\"".into(),
                ..Default::default()
            }),
        ];
        s.pages.push(p);
        s
    }

    fn at(id: &str) -> Target {
        Target {
            owner: Owner::Page("index".into()),
            node: id.into(),
        }
    }

    #[test]
    fn nom_et_masquage_des_calques() {
        let mut s = site();
        for id in ["h", "i"] {
            let c = rename(&s, &at(id), "  Bandeau   principal ")
                .unwrap()
                .unwrap();
            apply(&mut s, &c).unwrap();
            let n = address::node(&s, &at(id)).unwrap();
            assert_eq!(revel_model::layer_name(n), Some("Bandeau principal"));
            assert!(rename(&s, &at(id), "Bandeau principal").unwrap().is_none());
            let c = set_hidden(&s, &at(id), true).unwrap().unwrap();
            apply(&mut s, &c).unwrap();
            assert!(revel_model::is_hidden(address::node(&s, &at(id)).unwrap()));
            assert!(set_hidden(&s, &at(id), true).unwrap().is_none());
            // Nom vide : nom automatique ; affiché de nouveau.
            let c = rename(&s, &at(id), " ").unwrap().unwrap();
            apply(&mut s, &c).unwrap();
            let c = set_hidden(&s, &at(id), false).unwrap().unwrap();
            apply(&mut s, &c).unwrap();
            let n = address::node(&s, &at(id)).unwrap();
            assert!(revel_model::layer_name(n).is_none() && !revel_model::is_hidden(n));
        }
    }
}
