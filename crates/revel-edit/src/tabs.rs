//! Onglets (`data-revel-tabs`) : le n-ième onglet (`data-revel-tab`) montre le n-ième panneau
//! (`data-revel-panel`) ; plusieurs séries de panneaux suivent les onglets dans l'ordre. Ce module
//! retrouve, dans le document, quel onglet montre quel panneau, pour que l'interface le dise et
//! permette d'aller de l'un à l'autre.

use crate::address::{Owner, Target};
use revel_model::ids::child_lists;
use revel_model::{Attr, Node, Site};

pub const BOX: &str = "data-revel-tabs";
pub const TAB: &str = "data-revel-tab";
pub const PANEL: &str = "data-revel-panel";

fn has(attrs: &[Attr], name: &str) -> bool {
    attrs.iter().any(|a| a.name == name)
}

/// Onglets et panneaux d'une boîte d'onglets, dans l'ordre du document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TabsMap {
    pub tabs: Vec<Target>,
    pub panels: Vec<Target>,
    /// Onglets ou panneaux répétés par une liste (leur nombre dépend des contenus).
    pub repeated: bool,
}

impl TabsMap {
    /// Panneaux montrés par l'onglet `i`.
    pub fn panels_of(&self, i: usize) -> Vec<Target> {
        let nb = self.tabs.len().max(1);
        self.panels
            .iter()
            .enumerate()
            .filter(|(k, _)| k % nb == i)
            .map(|(_, t)| t.clone())
            .collect()
    }
    /// Onglet qui montre le panneau `k`.
    pub fn tab_of(&self, k: usize) -> Option<Target> {
        let nb = self.tabs.len();
        (nb > 0).then(|| self.tabs[k % nb].clone())
    }
}

/// Le composant contient-il un élément portant `name` ?
fn component_has(site: &Site, cid: &str, name: &str) -> bool {
    site.components.iter().any(|c| {
        c.id == cid && {
            let mut found = false;
            revel_model::ids::walk(&c.body, &mut |n| {
                if let Node::Element { attrs, .. } = n
                    && has(attrs, name)
                {
                    found = true;
                }
            });
            found
        }
    })
}

/// Boîte d'onglets qui contient `at` (ou `at` lui-même).
pub fn tabs_box(site: &Site, at: &Target) -> Option<Target> {
    let mut cur = at.clone();
    for _ in 0..64 {
        if let Ok(Node::Element { attrs, .. }) = crate::address::node(site, &cur)
            && has(attrs, BOX)
        {
            return Some(cur);
        }
        let (parent, _) = crate::address::locate(site, &cur).ok()?;
        cur = Target {
            owner: at.owner.clone(),
            node: parent.node?,
        };
    }
    None
}

/// Onglets et panneaux de la boîte `box_at`.
pub fn map(site: &Site, box_at: &Target) -> TabsMap {
    let mut m = TabsMap::default();
    let Ok(Node::Element { children, .. }) = crate::address::node(site, box_at) else {
        return m;
    };
    fn walk(site: &Site, owner: &Owner, nodes: &[Node], m: &mut TabsMap, in_loop: bool) {
        for n in nodes {
            let t = |id: &Option<String>| {
                id.clone().map(|node| Target {
                    owner: owner.clone(),
                    node,
                })
            };
            match n {
                Node::Element { id, attrs, .. } => {
                    // Boîte d'onglets imbriquée : ses onglets sont les siens.
                    if has(attrs, BOX) {
                        continue;
                    }
                    if has(attrs, TAB)
                        && let Some(t) = t(id)
                    {
                        m.tabs.push(t);
                        m.repeated |= in_loop;
                    }
                    if has(attrs, PANEL)
                        && let Some(t) = t(id)
                    {
                        m.panels.push(t);
                        m.repeated |= in_loop;
                        continue;
                    }
                    for list in child_lists(n) {
                        walk(site, owner, list, m, in_loop);
                    }
                }
                Node::Use { id, component, .. } => {
                    if component_has(site, component, TAB)
                        && let Some(t) = t(id)
                    {
                        m.tabs.push(t);
                        m.repeated |= in_loop;
                    } else if component_has(site, component, PANEL)
                        && let Some(t) = t(id)
                    {
                        m.panels.push(t);
                        m.repeated |= in_loop;
                    }
                }
                Node::Each { .. } => {
                    for list in child_lists(n) {
                        walk(site, owner, list, m, true);
                    }
                }
                _ => {
                    for list in child_lists(n) {
                        walk(site, owner, list, m, in_loop);
                    }
                }
            }
        }
    }
    walk(site, &box_at.owner, children, &mut m, false);
    m
}

/// Onglet ou panneau qui est `at` ou le contient : (onglet ?, rang).
pub fn place_of(site: &Site, m: &TabsMap, at: &Target) -> Option<(bool, usize)> {
    let mut cur = at.clone();
    for _ in 0..64 {
        if let Some(i) = m.tabs.iter().position(|t| *t == cur) {
            return Some((true, i));
        }
        if let Some(k) = m.panels.iter().position(|t| *t == cur) {
            return Some((false, k));
        }
        let (parent, _) = crate::address::locate(site, &cur).ok()?;
        cur = Target {
            owner: at.owner.clone(),
            node: parent.node?,
        };
    }
    None
}
