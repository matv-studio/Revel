//! Identifiants stables des nœuds.
//!
//! Chaque nœud structurel (élément, image, condition, boucle, composant utilisé, bloc) porte un
//! identifiant unique dans le site, utilisé par l'éditeur et les commandes. Les textes, expressions
//! et affectations sont adressés par leur parent et leur position.
//!
//! - Identifiant attribué à la lecture ou à l'import : **déterministe**, dérivé du propriétaire
//!   (page, gabarit, composant) et de la position du nœud ; un même document donne les mêmes
//!   identifiants.
//! - Identifiant d'un nœud créé dans l'éditeur : aléatoire ([`fresh`]).

use crate::{Node, Site};
use std::collections::HashSet;

/// Propriétaire d'un arbre de nœuds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerKind {
    Page,
    Layout,
    Component,
}

impl OwnerKind {
    fn tag(self) -> &'static str {
        match self {
            Self::Page => "p",
            Self::Layout => "l",
            Self::Component => "c",
        }
    }
}

/// Identifiant du nœud (s'il est structurel).
pub fn node_id(n: &Node) -> Option<&str> {
    match n {
        Node::Element { id, .. }
        | Node::If { id, .. }
        | Node::Each { id, .. }
        | Node::SetBlock { id, .. }
        | Node::Use { id, .. } => id.as_deref(),
        Node::Image(img) => img.id.as_deref(),
        _ => None,
    }
}

/// Emplacement de l'identifiant (nœuds structurels seulement).
pub fn node_id_mut(n: &mut Node) -> Option<&mut Option<String>> {
    match n {
        Node::Element { id, .. }
        | Node::If { id, .. }
        | Node::Each { id, .. }
        | Node::SetBlock { id, .. }
        | Node::Use { id, .. } => Some(id),
        Node::Image(img) => Some(&mut img.id),
        _ => None,
    }
}

/// Listes d'enfants d'un nœud, dans l'ordre (enfants, `then`, `otherwise`, `empty`).
pub fn child_lists(n: &Node) -> Vec<&Vec<Node>> {
    match n {
        Node::Element { children, .. } | Node::SetBlock { children, .. } => vec![children],
        Node::If {
            then, otherwise, ..
        } => vec![then, otherwise],
        Node::Each {
            children, empty, ..
        } => vec![children, empty],
        _ => vec![],
    }
}

pub fn child_lists_mut(n: &mut Node) -> Vec<&mut Vec<Node>> {
    match n {
        Node::Element { children, .. } | Node::SetBlock { children, .. } => vec![children],
        Node::If {
            then, otherwise, ..
        } => vec![then, otherwise],
        Node::Each {
            children, empty, ..
        } => vec![children, empty],
        _ => vec![],
    }
}

/// Parcourt un arbre (préordre).
pub fn walk<'a>(nodes: &'a [Node], f: &mut dyn FnMut(&'a Node)) {
    for n in nodes {
        f(n);
        for list in child_lists(n) {
            walk(list, f);
        }
    }
}

/// Empreinte FNV-1a 64 bits.
fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// 8 caractères en base 36 (minuscules et chiffres), commençant par une lettre.
fn encode(mut n: u64) -> String {
    const D: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut s = vec![D[10 + (n % 26) as usize]];
    n /= 26;
    for _ in 0..7 {
        s.push(D[(n % 36) as usize]);
        n /= 36;
    }
    String::from_utf8(s).unwrap_or_default()
}

/// Identifiant aléatoire, absent de `taken`.
pub fn fresh(taken: &HashSet<String>) -> String {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    loop {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
        // Horloge indisponible en WebAssembly (`SystemTime::now` y échoue) : compteur seul.
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(d) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            h.write_u128(d.as_nanos());
        }
        let id = encode(h.finish());
        if !taken.contains(&id) {
            return id;
        }
    }
}

impl Site {
    /// Identifiants de tous les nœuds et règles de style du site.
    pub fn node_ids(&self) -> HashSet<String> {
        let mut set = HashSet::new();
        for r in self.style_rules() {
            if let Some(id) = &r.id {
                set.insert(id.clone());
            }
        }
        for (_, _, body) in self.owners() {
            walk(body, &mut |n| {
                if let Some(id) = node_id(n) {
                    set.insert(id.to_string());
                }
            });
        }
        set
    }

    /// Arbres du site : (type de propriétaire, identifiant, corps).
    pub fn owners(&self) -> Vec<(OwnerKind, &str, &Vec<Node>)> {
        let mut v: Vec<(OwnerKind, &str, &Vec<Node>)> = Vec::new();
        v.extend(
            self.pages
                .iter()
                .map(|p| (OwnerKind::Page, p.id.as_str(), &p.body)),
        );
        v.extend(
            self.layouts
                .iter()
                .map(|l| (OwnerKind::Layout, l.id.as_str(), &l.body)),
        );
        v.extend(
            self.components
                .iter()
                .map(|c| (OwnerKind::Component, c.id.as_str(), &c.body)),
        );
        v
    }

    /// Attribue un identifiant déterministe à chaque nœud structurel qui n'en a pas, et remplace
    /// les identifiants en double (le premier rencontré est conservé). Les identifiants existants
    /// ne changent pas ; les nouveaux évitent tous les identifiants déjà présents.
    pub fn ensure_ids(&mut self) {
        let explicit = self.node_ids();
        let mut claimed: HashSet<String> = HashSet::new();
        fn rec(
            nodes: &mut [Node],
            prefix: &str,
            explicit: &HashSet<String>,
            claimed: &mut HashSet<String>,
        ) {
            for (i, n) in nodes.iter_mut().enumerate() {
                let path = format!("{prefix}/{i}");
                if let Some(slot) = node_id_mut(n) {
                    let keep = slot
                        .as_ref()
                        .is_some_and(|id| !id.is_empty() && !claimed.contains(id));
                    if keep {
                        if let Some(id) = slot.as_ref() {
                            claimed.insert(id.clone());
                        }
                    } else {
                        let mut salt = 0u32;
                        let id = loop {
                            let id = encode(fnv(&format!("{path}#{salt}")));
                            if !claimed.contains(&id) && !explicit.contains(&id) {
                                break id;
                            }
                            salt += 1;
                        };
                        claimed.insert(id.clone());
                        *slot = Some(id);
                    }
                }
                for (j, list) in child_lists_mut(n).into_iter().enumerate() {
                    rec(list, &format!("{path}.{j}"), explicit, claimed);
                }
            }
        }
        for p in &mut self.pages {
            let prefix = format!("{}:{}", OwnerKind::Page.tag(), p.id);
            rec(&mut p.body, &prefix, &explicit, &mut claimed);
        }
        for l in &mut self.layouts {
            let prefix = format!("{}:{}", OwnerKind::Layout.tag(), l.id);
            rec(&mut l.body, &prefix, &explicit, &mut claimed);
        }
        for c in &mut self.components {
            let prefix = format!("{}:{}", OwnerKind::Component.tag(), c.id);
            rec(&mut c.body, &prefix, &explicit, &mut claimed);
        }
        if let Some(sheet) = &mut self.stylesheet {
            for (i, item) in sheet.items.iter_mut().enumerate() {
                let crate::StyleItem::Rule(r) = item else {
                    continue;
                };
                let keep =
                    r.id.as_ref()
                        .is_some_and(|id| !id.is_empty() && !claimed.contains(id));
                if keep {
                    if let Some(id) = &r.id {
                        claimed.insert(id.clone());
                    }
                    continue;
                }
                let mut salt = 0u32;
                let id = loop {
                    let id = encode(fnv(&format!("s:{i}#{salt}")));
                    if !claimed.contains(&id) && !explicit.contains(&id) {
                        break id;
                    }
                    salt += 1;
                };
                claimed.insert(id.clone());
                r.id = Some(id);
            }
        }
    }

    /// Règles de la feuille de style, dans l'ordre.
    pub fn style_rules(&self) -> impl Iterator<Item = &crate::StyleRule> {
        self.stylesheet.iter().flat_map(|s| {
            s.items.iter().filter_map(|i| match i {
                crate::StyleItem::Rule(r) => Some(r),
                _ => None,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::*;
    use crate::{Component, Page};

    fn site() -> Site {
        let mut s = Site::new("x", "fr");
        s.pages.push(Page {
            id: "accueil".into(),
            path: "/".into(),
            layout: None,
            props: Default::default(),
            computed: Default::default(),
            body: vec![
                el("main", &[], vec![el("h1", &[], vec![text("A")]), out("x")]),
                Node::Each {
                    id: None,
                    expr: "items".into(),
                    item: "i".into(),
                    children: vec![el("li", &[], vec![])],
                    empty: vec![],
                },
            ],
            draft: false,
            name: None,
        });
        s.components.push(Component {
            id: "carte".into(),
            body: vec![el("div", &[], vec![])],
        });
        s
    }

    #[test]
    fn identifiants_deterministes_et_uniques() {
        let mut a = site();
        a.ensure_ids();
        let mut b = site();
        b.ensure_ids();
        assert_eq!(a, b, "même document, mêmes identifiants");
        let ids = a.node_ids();
        assert_eq!(ids.len(), 5); // main, h1, each, li, div
        assert!(
            ids.iter()
                .all(|i| i.len() == 8 && i.as_bytes()[0].is_ascii_lowercase())
        );
        // Identifiant existant conservé ; doublon remplacé.
        let mut c = site();
        if let Node::Element { id, .. } = &mut c.pages[0].body[0] {
            *id = Some("principal".into());
        }
        c.components[0].body[0] = el("div", &[], vec![]);
        if let Node::Element { id, .. } = &mut c.components[0].body[0] {
            *id = Some("principal".into());
        }
        c.ensure_ids();
        assert_eq!(node_id(&c.pages[0].body[0]), Some("principal"));
        assert_ne!(node_id(&c.components[0].body[0]), Some("principal"));
        assert_eq!(c.node_ids().len(), 5);
    }

    #[test]
    fn identifiant_aleatoire_libre() {
        let mut taken = HashSet::new();
        for _ in 0..1000 {
            let id = fresh(&taken);
            assert!(taken.insert(id));
        }
    }

    #[test]
    fn lecture_attribue_les_identifiants() {
        let mut s = site();
        s.schema_version = 1;
        let json = serde_json::to_string(&s).unwrap();
        let read = Site::from_json(&json).unwrap();
        assert_eq!(read.schema_version, crate::SCHEMA_VERSION);
        assert_eq!(read.node_ids().len(), 5);
    }
}
