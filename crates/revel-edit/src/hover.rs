//! Média au survol : une vidéo ou une image (`data-revel-hover-media="<nom>"`) montre le média de
//! l'élément survolé (ou atteint au clavier) parmi ceux qui la désignent
//! (`data-revel-media-for="<nom>"`, adresse `data-revel-media`, image d'attente
//! `data-revel-media-poster`). Outil distinct des onglets : les éléments survolés sont le plus
//! souvent ceux d'une liste de collection, dont chaque contenu donne son média (champ « Vidéo »),
//! ou des éléments choisis un à un, chacun avec son adresse. Le script du site
//! (`revel-render/src/actions.js`) met le premier au chargement, la classe `is-active` sur
//! l'élément montré et `is-ready` sur le média lancé.

use crate::address::{Branch, Owner, Target};
use crate::command::Command;
use revel_model::ids::{OwnerKind, child_lists};
use revel_model::{Attr, Node, Site};

/// Média qui change au survol (vidéo ou image) : nom qui le relie aux éléments survolés.
pub const DISPLAY: &str = "data-revel-hover-media";
/// Élément survolé : nom du média qu'il change.
pub const FOR: &str = "data-revel-media-for";
/// Élément survolé : adresse du média montré.
pub const SRC: &str = "data-revel-media";
/// Élément survolé : image d'attente de la vidéo (poster).
pub const POSTER: &str = "data-revel-media-poster";

/// Média au survol du site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Display {
    pub at: Target,
    /// Nom (vide : relié à aucun élément).
    pub name: String,
    pub tag: String,
}

fn owner(kind: OwnerKind, id: &str) -> Owner {
    match kind {
        OwnerKind::Page => Owner::Page(id.to_string()),
        OwnerKind::Layout => Owner::Layout(id.to_string()),
        OwnerKind::Component => Owner::Component(id.to_string()),
    }
}

fn value<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_deref().unwrap_or(""))
}

/// Éléments du site qui portent l'attribut `name` : (adresse, valeur, balise).
fn with_attr(site: &Site, name: &str) -> Vec<(Target, String, String)> {
    let mut out = Vec::new();
    for (kind, id, body) in site.owners() {
        revel_model::ids::walk(body, &mut |n| {
            if let Node::Element {
                id: Some(nid),
                tag,
                attrs,
                ..
            } = n
                && let Some(v) = value(attrs, name)
            {
                out.push((
                    Target {
                        owner: owner(kind, id),
                        node: nid.clone(),
                    },
                    v.trim().to_string(),
                    tag.to_ascii_lowercase(),
                ));
            }
        });
    }
    out
}

/// Médias au survol du site, dans l'ordre des pages, gabarits et composants.
pub fn displays(site: &Site) -> Vec<Display> {
    with_attr(site, DISPLAY)
        .into_iter()
        .map(|(at, name, tag)| Display { at, name, tag })
        .collect()
}

/// Média au survol nommé `name`.
pub fn display_named(site: &Site, name: &str) -> Option<Display> {
    (!name.is_empty())
        .then(|| displays(site).into_iter().find(|d| d.name == name))
        .flatten()
}

/// Éléments survolés qui changent le média `name`.
pub fn triggers(site: &Site, name: &str) -> Vec<Target> {
    if name.is_empty() {
        return Vec::new();
    }
    with_attr(site, FOR)
        .into_iter()
        .filter(|(_, v, _)| v == name)
        .map(|(t, _, _)| t)
        .collect()
}

/// Nom lisible tiré d'un texte (`Hero video` -> `hero-video`).
pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "media".into() } else { out }
}

/// Nom libre pour un média au survol, tiré de `base` (`hero-video`, sinon `hero-video-2`…).
pub fn free_name(site: &Site, base: &str) -> String {
    let base = slug(base);
    let taken: Vec<String> = displays(site).into_iter().map(|d| d.name).collect();
    if !taken.contains(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|n| !taken.contains(n))
        .unwrap_or(base)
}

/// Nom proposé pour le média `at` : sa première classe, sinon `media`.
pub fn name_for(site: &Site, at: &Target) -> String {
    let base = crate::address::node(site, at)
        .ok()
        .and_then(|n| match n {
            Node::Element { attrs, .. } => value(attrs, "class")
                .and_then(|c| c.split_whitespace().next())
                .filter(|c| !c.contains('{'))
                .map(|c| c.replace("__", "-")),
            _ => None,
        })
        .unwrap_or_else(|| "media".into());
    free_name(site, &base)
}

/// Boucle (liste de collection) qui contient `at` : son adresse et le nom de son élément.
pub fn enclosing_each(site: &Site, at: &Target) -> Option<(Target, String)> {
    let mut cur = at.clone();
    for _ in 0..64 {
        let (parent, _) = crate::address::locate(site, &cur).ok()?;
        let id = parent.node?;
        let t = Target {
            owner: at.owner.clone(),
            node: id,
        };
        if let Ok(Node::Each { item, .. }) = crate::address::node(site, &t)
            && parent.branch == Branch::Children
        {
            return Some((t, item.trim().to_string()));
        }
        cur = t;
    }
    None
}

/// Élément survolé dans le modèle d'une liste : son premier lien, sinon son premier élément.
pub fn list_trigger(site: &Site, each: &Target) -> Option<Target> {
    let Ok(Node::Each { children, .. }) = crate::address::node(site, each) else {
        return None;
    };
    fn first_link(nodes: &[Node]) -> Option<String> {
        for n in nodes {
            if let Node::Element { id, tag, .. } = n
                && tag.eq_ignore_ascii_case("a")
                && let Some(id) = id
            {
                return Some(id.clone());
            }
            // Pas dans une liste imbriquée (ses liens sont ceux d'un autre contenu).
            if matches!(n, Node::Each { .. }) {
                continue;
            }
            for list in child_lists(n) {
                if let Some(id) = first_link(list) {
                    return Some(id);
                }
            }
        }
        None
    }
    let id = first_link(children).or_else(|| {
        children.iter().find_map(|n| match n {
            Node::Element { id, .. } => id.clone(),
            _ => None,
        })
    })?;
    Some(Target {
        owner: each.owner.clone(),
        node: id,
    })
}

/// Valeur d'un attribut d'un élément (présent sans valeur : `Some("")`).
pub fn attr_of(site: &Site, at: &Target, name: &str) -> Option<String> {
    match crate::address::node(site, at).ok()? {
        Node::Element { attrs, .. } => value(attrs, name).map(str::to_string),
        _ => None,
    }
}

/// Champ lu par une valeur `{{ <item>.data.<champ> }}` (élément d'une liste nommé `item`).
pub fn field_of(v: &str, item: &str) -> Option<String> {
    let e = v.trim().strip_prefix("{{")?.strip_suffix("}}")?.trim();
    let f = e.strip_prefix(&format!("{item}.data."))?;
    (!f.is_empty()
        && f.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-'))
    .then(|| f.to_string())
}

/// Valeur d'un attribut relié au champ `field` de l'élément `item` d'une liste.
pub fn field_value(item: &str, field: &str) -> String {
    format!("{{{{ {item}.data.{field} }}}}")
}

fn set(at: &Target, name: &str, v: &str) -> Command {
    Command::SetAttr {
        at: at.clone(),
        name: name.into(),
        value: Some(v.to_string()),
    }
}

fn remove_if(site: &Site, at: &Target, name: &str, out: &mut Vec<Command>) {
    if attr_of(site, at, name).is_some() {
        out.push(Command::RemoveAttr {
            at: at.clone(),
            name: name.into(),
        });
    }
}

/// Retire d'un élément ce qui le rend survolé (nom, adresse, image d'attente).
pub fn unlink_trigger(site: &Site, at: &Target, out: &mut Vec<Command>) {
    for n in [FOR, SRC, POSTER] {
        remove_if(site, at, n, out);
    }
}

/// Nom du média `display` (attribué s'il n'en a pas) et commande qui l'écrit au besoin.
fn ensure_name(site: &Site, display: &Target, out: &mut Vec<Command>) -> Option<String> {
    let cur = attr_of(site, display, DISPLAY)?;
    if !cur.is_empty() {
        return Some(cur);
    }
    let name = name_for(site, display);
    out.push(set(display, DISPLAY, &name));
    Some(name)
}

/// Relie l'élément `trigger` au média `display` : valeurs de l'adresse (`src`) et de l'image
/// d'attente (`poster`) telles qu'écrites dans l'attribut (`{{ it.data.video }}` dans une liste,
/// une adresse sinon ; `None` : inchangée, ou vide si absente ; image d'attente vide : retirée).
/// `exclusive` : les éléments qui changeaient ce média jusque-là ne le changent plus.
pub fn connect(
    site: &Site,
    display: &Target,
    trigger: &Target,
    src: Option<&str>,
    poster: Option<&str>,
    exclusive: bool,
) -> Option<Command> {
    let mut out = Vec::new();
    let name = ensure_name(site, display, &mut out)?;
    if exclusive {
        for t in triggers(site, &name) {
            if t != *trigger {
                unlink_trigger(site, &t, &mut out);
            }
        }
    }
    out.push(set(trigger, FOR, &name));
    match src {
        Some(v) => out.push(set(trigger, SRC, v)),
        None if attr_of(site, trigger, SRC).is_none() => out.push(set(trigger, SRC, "")),
        None => {}
    }
    match poster {
        Some(v) if !v.trim().is_empty() => out.push(set(trigger, POSTER, v)),
        Some(_) => remove_if(site, trigger, POSTER, &mut out),
        None => {}
    }
    Some(Command::Batch { commands: out })
}

/// Relie le média `display` aux éléments de la liste de collection `each` : chaque contenu
/// montre la valeur de son champ `src_field` (et `poster_field` en image d'attente). Les
/// éléments survolés qui le changeaient jusque-là ne le changent plus.
pub fn connect_list(
    site: &Site,
    display: &Target,
    each: &Target,
    src_field: &str,
    poster_field: Option<&str>,
) -> Option<Command> {
    let Ok(Node::Each { item, .. }) = crate::address::node(site, each) else {
        return None;
    };
    let item = item.trim().to_string();
    let trigger = list_trigger(site, each)?;
    let src = field_value(&item, src_field);
    let poster = poster_field
        .filter(|f| !f.is_empty())
        .map(|f| field_value(&item, f))
        .unwrap_or_default();
    connect(site, display, &trigger, Some(&src), Some(&poster), true)
}

/// Média au survol retiré : le média redevient ordinaire, les éléments survolés aussi.
pub fn disconnect(site: &Site, display: &Target) -> Option<Command> {
    let name = attr_of(site, display, DISPLAY)?;
    let mut out = Vec::new();
    for t in triggers(site, &name) {
        unlink_trigger(site, &t, &mut out);
    }
    out.push(Command::RemoveAttr {
        at: display.clone(),
        name: DISPLAY.into(),
    });
    Some(Command::Batch { commands: out })
}

/// Liste de collection dont les éléments changent le média `name` (le premier élément survolé
/// qui est dans une liste) : adresse de la boucle et nom de son élément.
pub fn driving_list(site: &Site, name: &str) -> Option<(Target, String)> {
    triggers(site, name)
        .iter()
        .find_map(|t| enclosing_each(site, t))
}
