//! Plan du site (D-103) : vue nodale de toutes les pages et de ce qui les relie.
//!
//! - nœuds : pages, collections (une page par contenu ; contenus dépliables), menus du site et
//!   parties communes qui portent des liens (composants, gabarits), adresses sans page, autres
//!   sites, plateformes de vidéos et réseaux sociaux (D-107 : adresses vérifiées, remplacées
//!   partout où elles servent, champs et texte des contenus du CMS compris) ;
//! - liens : liens écrits, listes de contenus, liens des menus, redirections ;
//! - plan infini : glisser le fond (ou la molette) pour se déplacer, Alt ou Ctrl+molette (pincer)
//!   pour zoomer, ajuster (F), minicarte ;
//! - les nœuds se déplacent (positions gardées dans le site, annulables) ; « Ranger » rétablit
//!   l'ordre des adresses ;
//! - glisser le bout d'un lien sélectionné sur une autre page change sa destination ; glisser le
//!   point d'une page ou d'un menu sur une page y ajoute un lien ;
//! - panneau du nœud : nom, adresse, brouillon, ouvrir dans l'éditeur, dupliquer, supprimer,
//!   créer une page manquante, rediriger une adresse ; liens entrants et sortants.

use crate::dialog::Ask;
use crate::site::{DocSession, doc_session};
use crate::{Section, Tab, Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::linkmap::{
    self as lm, EntryRef, EntryText, LinkMap, LinkVia, MapLink, MapNode, NodeKind, PlatformKind,
};
use revel_edit::{Command, Owner, Parent};
use revel_model::Node;
use revel_protocol::{CmsCollection, SiteSummary};
use revel_protocol::{LinkCheck, LinkState};
use std::collections::{BTreeMap, HashMap, HashSet};
use wasm_bindgen::JsCast;

/// Largeur d'un nœud ; hauteur d'un nœud et d'un contenu.
const NODE_W: f64 = 220.0;
const NODE_H: f64 = 60.0;
const ENTRY_H: f64 = 32.0;
/// Hauteur d'une collection (nom, adresse, ligne « n contenus »).
const COLLECTION_H: f64 = 84.0;
/// Écarts entre colonnes et entre nœuds d'une colonne.
const COL_GAP: f64 = 110.0;
const ROW_GAP: f64 = 20.0;
/// Contenus d'une collection : alignés sous la collection (même largeur, centrés), écart
/// entre eux.
const ENTRY_GAP: f64 = 6.0;
const ZOOM_MIN: f64 = 0.15;
const ZOOM_MAX: f64 = 2.5;

fn node_h(kind: NodeKind) -> f64 {
    match kind {
        NodeKind::Entry | NodeKind::External => ENTRY_H,
        NodeKind::Collection => COLLECTION_H,
        _ => NODE_H,
    }
}

/// Sorte d'un trait (plusieurs liens entre deux nœuds : un seul trait).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EdgeKind {
    Link,
    List,
    Menu,
    Redirect,
    Broken,
    External,
}

impl EdgeKind {
    fn class(self) -> &'static str {
        match self {
            Self::Link => "link",
            Self::List => "list",
            Self::Menu => "menu",
            Self::Redirect => "redirect",
            Self::Broken => "broken",
            Self::External => "external",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
struct Edge {
    from: String,
    to: String,
    /// Positions des liens dans le plan.
    links: Vec<usize>,
    kind: EdgeKind,
}

fn edges_of(map: &LinkMap) -> Vec<Edge> {
    let mut out: Vec<Edge> = Vec::new();
    let mut at: HashMap<(String, String), usize> = HashMap::new();
    for (i, l) in map.links.iter().enumerate() {
        let k = (l.from.clone(), l.to.clone());
        match at.get(&k) {
            Some(&j) => out[j].links.push(i),
            None => {
                at.insert(k, out.len());
                out.push(Edge {
                    from: l.from.clone(),
                    to: l.to.clone(),
                    links: vec![i],
                    kind: EdgeKind::Link,
                });
            }
        }
    }
    for e in &mut out {
        let to = map.node(&e.to).map(|n| n.kind);
        let vias: Vec<&LinkVia> = e.links.iter().map(|&i| &map.links[i].via).collect();
        e.kind = if vias.iter().all(|v| matches!(v, LinkVia::Redirect { .. })) {
            EdgeKind::Redirect
        } else if to == Some(NodeKind::Missing) {
            EdgeKind::Broken
        } else if matches!(to, Some(NodeKind::External | NodeKind::Media)) {
            EdgeKind::External
        } else if vias.iter().all(|v| matches!(v, LinkVia::List { .. })) {
            EdgeKind::List
        } else if vias.iter().all(|v| matches!(v, LinkVia::Menu { .. })) {
            EdgeKind::Menu
        } else {
            EdgeKind::Link
        };
    }
    out
}

/// Contenus d'une collection repliée : leurs liens vont (et viennent) de la collection.
fn collapse(mut map: LinkMap, open: &HashSet<String>) -> LinkMap {
    let parent: HashMap<String, String> = map
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Entry)
        .filter_map(|n| Some((n.key.clone(), n.parent.clone()?)))
        .filter(|(_, p)| !open.contains(p))
        .collect();
    if parent.is_empty() {
        return map;
    }
    map.nodes.retain(|n| !parent.contains_key(&n.key));
    for l in &mut map.links {
        if let Some(p) = parent.get(&l.to) {
            l.to = p.clone();
        }
        if let Some(p) = parent.get(&l.from) {
            l.from = p.clone();
        }
    }
    map.links.retain(|l| l.from != l.to);
    map
}

/// Adresse montrée : valeurs `{{ slug }}` écrites `‹slug›` (chaque contenu y met la sienne).
fn readable_path(path: &str) -> String {
    let mut out = String::new();
    let mut rest = path;
    while let Some(i) = rest.find("{{") {
        out.push_str(&rest[..i]);
        let Some(j) = rest[i..].find("}}") else {
            out.push_str(&rest[i..]);
            return out;
        };
        let inner = rest[i + 2..i + j].trim();
        let name = inner
            .rsplit('.')
            .next()
            .unwrap_or(inner)
            .split('|')
            .next()
            .unwrap_or("")
            .trim();
        out.push('‹');
        out.push_str(name);
        out.push('›');
        rest = &rest[i + j + 2..];
    }
    out.push_str(rest);
    out
}

/// Partie littérale d'une adresse (avant la première valeur `{{ }}`).
fn literal_prefix(path: &str) -> &str {
    path.split("{{").next().unwrap_or(path)
}

/// Positions rangées d'après les adresses : parties communes (menus, composants, gabarits) à
/// gauche ; pages en arbre (dossier parent à gauche de ses pages ; une collection sous la page
/// qui la liste) ; adresses sans page et autres sites à droite.
fn arrange(map: &LinkMap) -> HashMap<String, (f64, f64)> {
    let mut pos = HashMap::new();
    let is_tree =
        |k: NodeKind| matches!(k, NodeKind::Page | NodeKind::Collection | NodeKind::Entry);
    // Parties communes.
    let shared: Vec<&MapNode> = map
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                n.kind,
                NodeKind::Menu | NodeKind::Component | NodeKind::Layout
            )
        })
        .collect();
    let mut y = 0.0;
    for n in &shared {
        pos.insert(n.key.clone(), (0.0, y));
        y += NODE_H + ROW_GAP;
    }
    let x0 = if shared.is_empty() {
        0.0
    } else {
        NODE_W + COL_GAP * 1.6
    };
    // Arbre des pages.
    let pages: Vec<&MapNode> = map
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Page)
        .collect();
    let path_of = |n: &MapNode| n.path.clone().unwrap_or_default();
    let root = pages
        .iter()
        .find(|n| n.path.as_deref() == Some("/"))
        .map(|n| n.key.clone());
    let page_parent = |path: &str, me: &str| -> Option<String> {
        // Page dont l'adresse est le plus long début de celle-ci.
        let lit = literal_prefix(path);
        pages
            .iter()
            .filter(|p| p.key != me)
            .filter(|p| {
                let pp = path_of(p);
                pp != "/" && !pp.contains("{{") && lit.starts_with(&pp) && lit != pp
            })
            .max_by_key(|p| path_of(p).len())
            .map(|p| p.key.clone())
            .or_else(|| root.clone().filter(|r| r != me))
    };
    let mut parent: HashMap<String, String> = HashMap::new();
    for n in map.nodes.iter().filter(|n| is_tree(n.kind)) {
        let p = match n.kind {
            NodeKind::Entry => n.parent.clone(),
            NodeKind::Collection => {
                // Page qui la liste ; plusieurs : une autre que l'accueil (la page de la rubrique).
                let mut listers: Vec<&str> = map
                    .links
                    .iter()
                    .filter(|l| l.to == n.key && matches!(l.via, LinkVia::List { .. }))
                    .map(|l| l.from.as_str())
                    .filter(|f| f.starts_with("page:"))
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect();
                listers.sort_by_key(|k| map.node(k).and_then(|p| p.path.clone()));
                let inner: Vec<&str> = listers
                    .iter()
                    .copied()
                    .filter(|k| Some(k.to_string()) != root)
                    .collect();
                match (inner.first(), listers.first()) {
                    (Some(one), _) | (None, Some(one)) => Some(one.to_string()),
                    _ => page_parent(&path_of(n), &n.key),
                }
            }
            _ => page_parent(&path_of(n), &n.key),
        };
        if let Some(p) = p.filter(|p| p != &n.key) {
            parent.insert(n.key.clone(), p);
        }
    }
    // Cycles (impossibles en principe) : rompus.
    let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (c, p) in &parent {
        children.entry(p.clone()).or_default().push(c.clone());
    }
    let order = |k: &String| {
        map.node(k)
            .map(|n| {
                (
                    match n.kind {
                        NodeKind::Page => 0,
                        NodeKind::Collection => 1,
                        _ => 2,
                    },
                    n.path.clone().unwrap_or_default(),
                    n.label.clone(),
                )
            })
            .unwrap_or_default()
    };
    for list in children.values_mut() {
        list.sort_by_key(order);
    }
    let mut roots: Vec<String> = map
        .nodes
        .iter()
        .filter(|n| is_tree(n.kind) && !parent.contains_key(&n.key))
        .map(|n| n.key.clone())
        .collect();
    roots.sort_by_key(|k| (Some(k) != root.as_ref(), order(k)));
    let mut cursor = 0.0;
    let mut seen: HashSet<String> = HashSet::new();
    let mut max_x: f64 = x0;
    #[allow(clippy::too_many_arguments)]
    fn place(
        key: &str,
        depth: usize,
        x0: f64,
        map: &LinkMap,
        children: &BTreeMap<String, Vec<String>>,
        cursor: &mut f64,
        seen: &mut HashSet<String>,
        pos: &mut HashMap<String, (f64, f64)>,
        max_x: &mut f64,
    ) -> f64 {
        if !seen.insert(key.to_string()) {
            return *cursor;
        }
        let h = map.node(key).map(|n| node_h(n.kind)).unwrap_or(NODE_H);
        let entry = map.node(key).is_some_and(|n| n.kind == NodeKind::Entry);
        let x = x0 + depth as f64 * (NODE_W + COL_GAP);
        *max_x = max_x.max(x);
        let kids = children.get(key).cloned().unwrap_or_default();
        // Contenus : en colonne serrée à droite de leur collection.
        let all_entries = !kids.is_empty()
            && kids
                .iter()
                .all(|k| map.node(k).is_some_and(|n| n.kind == NodeKind::Entry));
        if kids.is_empty() || entry {
            let y = *cursor;
            pos.insert(key.to_string(), (x, y));
            *cursor += h + ROW_GAP;
            return y + h / 2.0;
        }
        if all_entries {
            // Contenus : en liste serrée sous leur collection, légèrement en retrait.
            let top = *cursor;
            pos.insert(key.to_string(), (x, top));
            *cursor = top + h + ENTRY_GAP;
            for k in &kids {
                seen.insert(k.clone());
                pos.insert(k.clone(), (x, *cursor));
                *cursor += ENTRY_H + ENTRY_GAP;
            }
            *cursor += ROW_GAP - ENTRY_GAP;
            return top + h / 2.0;
        }
        let mut centers = Vec::new();
        for k in &kids {
            centers.push(place(
                k,
                depth + 1,
                x0,
                map,
                children,
                cursor,
                seen,
                pos,
                max_x,
            ));
        }
        let mid = (centers[0] + centers[centers.len() - 1]) / 2.0;
        let y = mid - h / 2.0;
        pos.insert(key.to_string(), (x, y));
        mid
    }
    for r in &roots {
        place(
            r,
            0,
            x0,
            map,
            &children,
            &mut cursor,
            &mut seen,
            &mut pos,
            &mut max_x,
        );
        cursor += ROW_GAP;
    }
    // Pages restées hors de l'arbre (cycle) : à la suite.
    for n in map.nodes.iter().filter(|n| is_tree(n.kind)) {
        if !pos.contains_key(&n.key) {
            pos.insert(n.key.clone(), (x0, cursor));
            cursor += node_h(n.kind) + ROW_GAP;
        }
    }
    // Adresses sans page et autres sites.
    let xr = max_x + NODE_W + COL_GAP * 1.6;
    let mut yr = 0.0;
    for kind in [NodeKind::Missing, NodeKind::Media, NodeKind::External] {
        for n in map.nodes.iter().filter(|n| n.kind == kind) {
            pos.insert(n.key.clone(), (xr, yr));
            yr += node_h(n.kind) + ROW_GAP;
        }
    }
    pos
}

/// Point de départ et d'arrivée d'un trait, et sa courbe.
/// Sens d'un trait : vers la droite (côté droit → côté gauche), vers la gauche (côté gauche →
/// côté droit), ou dans la même colonne (côté droit → côté droit, en arc).
fn route(a: (f64, f64, f64), b: (f64, f64, f64)) -> (f64, f64, f64, f64, f64, f64) {
    let (ya, yb) = (a.1 + a.2 / 2.0, b.1 + b.2 / 2.0);
    if b.0 >= a.0 + NODE_W + 24.0 {
        let (x1, x2) = (a.0 + NODE_W, b.0);
        let d = ((x2 - x1) * 0.5).max(40.0);
        (x1, ya, x2, yb, d, -d)
    } else if b.0 + NODE_W + 24.0 <= a.0 {
        let (x1, x2) = (a.0, b.0 + NODE_W);
        let d = ((x1 - x2) * 0.5).max(40.0);
        (x1, ya, x2, yb, -d, d)
    } else {
        let bulge = 40.0 + (yb - ya).abs() * 0.2;
        (a.0 + NODE_W, ya, b.0 + NODE_W, yb, bulge, bulge)
    }
}

/// Courbe d'un trait entre deux nœuds (position et hauteur).
fn curve(a: (f64, f64, f64), b: (f64, f64, f64)) -> String {
    let (x1, y1, x2, y2, c1, c2) = route(a, b);
    format!(
        "M{x1:.1},{y1:.1} C{:.1},{y1:.1} {:.1},{y2:.1} {x2:.1},{y2:.1}",
        x1 + c1,
        x2 + c2
    )
}

/// Trait en cours de tirage : du nœud au pointeur.
fn ghost_curve(a: (f64, f64, f64), (gx, gy): (f64, f64)) -> String {
    let left = gx < a.0;
    let (x1, y1) = (if left { a.0 } else { a.0 + NODE_W }, a.1 + a.2 / 2.0);
    let d = ((gx - x1).abs() * 0.5).max(30.0) * if left { -1.0 } else { 1.0 };
    format!(
        "M{x1:.1},{y1:.1} C{:.1},{y1:.1} {:.1},{gy:.1} {gx:.1},{gy:.1}",
        x1 + d,
        gx - d
    )
}

fn closest(e: &web_sys::Event, sel: &str) -> Option<web_sys::Element> {
    e.target()?
        .dyn_into::<web_sys::Element>()
        .ok()?
        .closest(sel)
        .ok()
        .flatten()
}

/// Élément sélectionné du plan.
#[derive(Clone, PartialEq, Debug)]
enum Picked {
    Node(String),
    Edge(String, String),
}

/// Geste en cours sur le plan.
#[derive(Clone, Debug)]
enum Gesture {
    Pan {
        sx: f64,
        sy: f64,
        t0: (f64, f64),
    },
    Move {
        key: String,
        sx: f64,
        sy: f64,
        p0: (f64, f64),
        moved: bool,
    },
    /// Bout d'un lien déplacé (changer sa destination).
    Retarget {
        from: String,
        to: String,
    },
    /// Point d'une page ou d'un menu tiré vers une page (ajouter un lien).
    Connect {
        from: String,
    },
    Minimap,
}

/// Titre d'un contenu du CMS (champ du titre, sinon nom du fichier).
fn entry_title(e: &revel_protocol::CmsEntry) -> String {
    ["title", "titre", "name", "nom", "label"]
        .iter()
        .find_map(|k| e.data.get(*k).and_then(|v| v.as_str()))
        .unwrap_or(&e.file)
        .to_string()
}

/// Adresses d'autres sites du plan (autres sites, vidéos, réseaux), une fois chacune.
fn external_urls(m: &LinkMap) -> Vec<String> {
    let ext: HashSet<&str> = m
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::External | NodeKind::Media))
        .map(|n| n.key.as_str())
        .collect();
    let mut out: Vec<String> = Vec::new();
    for l in &m.links {
        if ext.contains(l.to.as_str()) && !out.contains(&l.href) {
            out.push(l.href.clone());
        }
    }
    out
}

/// Gravité d'un état : 0 en ligne, 1 non vérifiable, 2 à revoir (déplacée, privée), 3 en panne.
fn severity(s: LinkState) -> u8 {
    match s {
        LinkState::Ok => 0,
        LinkState::Blocked => 1,
        LinkState::Redirected | LinkState::Restricted => 2,
        LinkState::Missing | LinkState::Failing | LinkState::Unreachable => 3,
    }
}

/// Classe et texte d'un état.
fn state_key(s: LinkState) -> (&'static str, &'static str) {
    match s {
        LinkState::Ok => ("ok", "map-url-ok"),
        LinkState::Redirected => ("warn", "map-url-redirected"),
        LinkState::Missing => ("bad", "map-url-missing"),
        LinkState::Restricted => ("warn", "map-url-restricted"),
        LinkState::Blocked => ("unknown", "map-url-blocked"),
        LinkState::Failing => ("bad", "map-url-failing"),
        LinkState::Unreachable => ("bad", "map-url-unreachable"),
    }
}

/// Adresse affichée : sans protocole, `www.` ni paramètres, caractères encodés rendus.
fn short_url(url: &str) -> String {
    let u = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let u = u.strip_prefix("www.").unwrap_or(u);
    let u = u.split(['?', '#']).next().unwrap_or(u);
    js_sys::decode_uri_component(u)
        .ok()
        .and_then(|s| s.as_string())
        .unwrap_or_else(|| u.to_string())
}

/// Outils des adresses d'autres sites passés au panneau (D-107).
#[derive(Clone, Copy)]
struct ExtTools {
    checks: RwSignal<HashMap<String, LinkCheck>>,
    checking: RwSignal<bool>,
    run_check: Callback<(Vec<String>, &'static str)>,
    replace: Callback<(String, String)>,
}

/// Nœud qui peut recevoir un lien écrit (adresse connue).
fn linkable(n: &MapNode) -> bool {
    matches!(n.kind, NodeKind::Page | NodeKind::Entry)
        && n.path.as_deref().is_some_and(|p| !p.contains("{{"))
}

#[component]
pub fn SiteMap(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let path = StoredValue::new(site.path.clone());
    let s: DocSession = doc_session(ui, site.path.clone());
    let info = crate::siteinfo::load(site.path.clone(), s.doc.into());
    let cms = RwSignal::new(Vec::<CmsCollection>::new());
    {
        let p = site.path.clone();
        spawn_local(async move {
            if let Ok(c) = api::cms_collections(&p).await {
                let _ = cms.try_set(c);
            }
        });
    }
    // Collections dépliées (contenus montrés).
    let open = RwSignal::new(HashSet::<String>::new());
    let show_shared = RwSignal::new(true);
    let show_external = RwSignal::new(false);
    let show_media = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    // Contenus des collections (adresse, titre), et identifiant de leur collection dans le CMS.
    let cms_id = Memo::new(move |_| {
        let folders: Vec<(String, String)> = s.doc.with(|d| {
            d.as_ref()
                .map(|d| {
                    d.collections
                        .iter()
                        .map(|c| (c.folder.clone(), c.id.clone()))
                        .collect()
                })
                .unwrap_or_default()
        });
        cms.with(|cs| {
            cs.iter()
                .filter_map(|c| {
                    let site_id = folders.iter().find(|(f, _)| *f == c.folder)?.1.clone();
                    Some((site_id, c.id.clone()))
                })
                .collect::<HashMap<String, String>>()
        })
    });
    let entries = Memo::new(move |_| {
        let folders: Vec<(String, String)> = s.doc.with(|d| {
            d.as_ref()
                .map(|d| {
                    d.collections
                        .iter()
                        .filter(|c| c.path.is_some())
                        .map(|c| (c.folder.clone(), c.id.clone()))
                        .collect()
                })
                .unwrap_or_default()
        });
        cms.with(|cs| {
            let mut out = Vec::new();
            for c in cs {
                let Some((_, id)) = folders.iter().find(|(f, _)| *f == c.folder) else {
                    continue;
                };
                for e in &c.entries {
                    let Some(url) = &e.url else { continue };
                    let title = entry_title(e);
                    out.push(EntryRef {
                        collection: id.clone(),
                        file: e.file.clone(),
                        url: url.clone(),
                        title,
                    });
                }
            }
            out
        })
    });
    // Nom affiché d'une collection (CMS), sinon son identifiant.
    let col_label = Memo::new(move |_| {
        let ids = cms_id.get();
        cms.with(|cs| {
            ids.iter()
                .filter_map(|(site_id, cid)| {
                    let c = cs.iter().find(|c| &c.id == cid)?;
                    Some((
                        site_id.clone(),
                        c.label
                            .clone()
                            .unwrap_or_else(|| crate::site::capitalize(&c.id)),
                    ))
                })
                .collect::<HashMap<String, String>>()
        })
    });
    // Contenus de toutes les collections : adresses d'autres sites écrites dans leurs champs et
    // leur texte (D-107).
    let texts = Memo::new(move |_| {
        let folders: Vec<(String, String)> = s.doc.with(|d| {
            d.as_ref()
                .map(|d| {
                    d.collections
                        .iter()
                        .map(|c| (c.folder.clone(), c.id.clone()))
                        .collect()
                })
                .unwrap_or_default()
        });
        cms.with(|cs| {
            let mut out = Vec::new();
            for c in cs {
                let id = folders
                    .iter()
                    .find(|(f, _)| *f == c.folder)
                    .map_or(c.id.clone(), |f| f.1.clone());
                for e in &c.entries {
                    out.push(EntryText {
                        collection: id.clone(),
                        folder: c.folder.clone(),
                        file: e.file.clone(),
                        title: entry_title(e),
                        data: e.data.clone(),
                        body: e.body.clone(),
                    });
                }
            }
            out
        })
    });
    let full = Memo::new(move |_| {
        let info = info.get();
        let entries = entries.get();
        let texts = texts.get();
        s.doc.with(|d| {
            d.as_ref()
                .map(|d| lm::build_with(d, info.as_ref(), &entries, &texts))
                .unwrap_or_default()
        })
    });
    // Plan affiché : contenus repliés, filtres.
    let map = Memo::new(move |_| {
        let mut m = collapse(full.get(), &open.get());
        let labels = col_label.get();
        for n in &mut m.nodes {
            if n.kind == NodeKind::Collection
                && let Some(l) = labels.get(n.key.trim_start_matches("col:"))
            {
                n.label = l.clone();
            }
        }
        let shared = show_shared.get();
        let ext = show_external.get();
        let media = show_media.get();
        m.nodes.retain(|n| match n.kind {
            NodeKind::Menu | NodeKind::Component | NodeKind::Layout => shared,
            NodeKind::External => ext,
            NodeKind::Media => media,
            _ => true,
        });
        let keep: HashSet<String> = m.nodes.iter().map(|n| n.key.clone()).collect();
        m.links
            .retain(|l| keep.contains(&l.from) && keep.contains(&l.to));
        m
    });
    let orphans = Memo::new(move |_| full.with(|m| m.orphans()));
    // Adresses d'autres sites vérifiées (D-107) : résultats gardés relus à l'ouverture, adresses
    // vérifiées quand leur groupe est montré (plus d'un jour : de nouveau).
    let ext_urls = Memo::new(move |_| full.with(external_urls));
    let checks = RwSignal::new(HashMap::<String, LinkCheck>::new());
    let checking = RwSignal::new(false);
    let run_check = Callback::new(move |(urls, mode): (Vec<String>, &'static str)| {
        if urls.is_empty() {
            return;
        }
        let p = path.get_value();
        let busy = mode != "cached";
        if busy {
            checking.set(true);
        }
        spawn_local(async move {
            match api::links_check(&p, &urls, mode).await {
                Ok(r) => {
                    let _ = checks.try_update(|c| {
                        for x in r.into_iter().flatten() {
                            c.insert(x.url.clone(), x);
                        }
                    });
                }
                Err(e) if busy => ui.fail(e),
                Err(_) => {}
            }
            if busy {
                let _ = checking.try_set(false);
            }
        });
    });
    Effect::new(move |prev: Option<(usize, bool)>| {
        let urls = ext_urls.get();
        let shown = show_media.get() || show_external.get();
        let now = (urls.len(), shown);
        if prev != Some(now) {
            run_check.run((urls, if shown { "stale" } else { "cached" }));
        }
        now
    });
    // Adresses à revoir (introuvables, en erreur, privées, déplacées).
    let to_review = Memo::new(move |_| {
        let c = checks.get();
        ext_urls.with(|u| {
            u.iter()
                .filter(|x| c.get(*x).is_some_and(|c| severity(c.state) >= 2))
                .count()
        })
    });
    let edges = Memo::new(move |_| map.with(edges_of));
    let auto = Memo::new(move |_| map.with(arrange));
    // Positions choisies (gardées dans le site) ; en cours d'écriture ; pendant un glisser.
    let saved = Memo::new(move |_| {
        s.doc.with(|d| {
            d.as_ref()
                .map(|d| d.library.map.clone())
                .unwrap_or_default()
        })
    });
    let pending = RwSignal::new(HashMap::<String, (f64, f64)>::new());
    Effect::new(move |_| {
        let sv = saved.get();
        pending.update(|p| {
            p.retain(|k, v| sv.get(k).is_none_or(|s| (s[0] as f64, s[1] as f64) != *v))
        });
    });
    let dragging = RwSignal::new(None::<(String, (f64, f64))>);
    let pos = Memo::new(move |_| {
        let mut p = auto.get();
        saved.with(|sv| {
            for (k, v) in sv {
                if p.contains_key(k) {
                    p.insert(k.clone(), (v[0] as f64, v[1] as f64));
                }
            }
        });
        pending.with(|pd| {
            for (k, v) in pd {
                if p.contains_key(k) {
                    p.insert(k.clone(), *v);
                }
            }
        });
        if let Some((k, v)) = dragging.get() {
            p.insert(k, v);
        }
        // Contenus sans position choisie : sous leur collection, où qu'elle soit (ils la suivent
        // quand on la déplace).
        let placed = |k: &str| {
            saved.with(|sv| sv.contains_key(k))
                || pending.with(|pd| pd.contains_key(k))
                || dragging.with(|d| d.as_ref().is_some_and(|(dk, _)| dk == k))
        };
        auto.with(|a| {
            map.with(|m| {
                for n in m.nodes.iter().filter(|n| n.kind == NodeKind::Entry) {
                    let Some(parent) = n.parent.as_deref() else {
                        continue;
                    };
                    if placed(&n.key) || !placed(parent) {
                        continue;
                    }
                    if let (Some(&(ex, ey)), Some(&(cx, cy)), Some(&(px, py))) =
                        (a.get(&n.key), a.get(parent), p.get(parent))
                    {
                        p.insert(n.key.clone(), (px + ex - cx, py + ey - cy));
                    }
                }
            })
        });
        p
    });
    let rect_of = move |key: &str| -> Option<(f64, f64, f64)> {
        let (x, y) = pos.with(|p| p.get(key).copied())?;
        let h = map.with(|m| m.node(key).map(|n| node_h(n.kind)))?;
        Some((x, y, h))
    };
    let bounds = Memo::new(move |_| {
        let p = pos.get();
        map.with(|m| {
            let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            for n in &m.nodes {
                if let Some(&(x, y)) = p.get(&n.key) {
                    b.0 = b.0.min(x);
                    b.1 = b.1.min(y);
                    b.2 = b.2.max(x + NODE_W);
                    b.3 = b.3.max(y + node_h(n.kind));
                }
            }
            if b.0 > b.2 { (0.0, 0.0, 1.0, 1.0) } else { b }
        })
    });

    // Vue : déplacement et zoom.
    let view_t = RwSignal::new((40.0f64, 40.0f64, 1.0f64));
    let stage = NodeRef::<leptos::html::Div>::new();
    let stage_rect = move || {
        stage
            .get_untracked()
            .map(|el| el.get_bounding_client_rect())
    };
    let fit = move || {
        let Some(r) = stage_rect() else { return };
        let (x0, y0, x1, y1) = bounds.get_untracked();
        let pad = 48.0;
        let (w, h) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
        let k = ((r.width() - 2.0 * pad) / w)
            .min((r.height() - 2.0 * pad) / h)
            .clamp(ZOOM_MIN, 1.2);
        let tx = (r.width() - w * k) / 2.0 - x0 * k;
        let ty = (r.height() - h * k) / 2.0 - y0 * k;
        view_t.set((tx, ty, k));
    };
    let zoom_at = move |cx: f64, cy: f64, factor: f64| {
        let (tx, ty, k) = view_t.get_untracked();
        let nk = (k * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        let (wx, wy) = ((cx - tx) / k, (cy - ty) / k);
        view_t.set((cx - wx * nk, cy - wy * nk, nk));
    };
    let zoom_center = move |factor: f64| {
        if let Some(r) = stage_rect() {
            zoom_at(r.width() / 2.0, r.height() / 2.0, factor);
        }
    };
    // Plan ajusté à l'ouverture (une fois les nœuds connus).
    let fitted = StoredValue::new(false);
    Effect::new(move |_| {
        let n = map.with(|m| m.nodes.len());
        if n > 0 && !fitted.get_value() {
            fitted.set_value(true);
            request_animation_frame(fit);
        }
    });

    let picked = RwSignal::new(None::<Picked>);
    let hovered = RwSignal::new(None::<String>);
    let gesture = StoredValue::new(None::<Gesture>);
    // Pointeur dans le plan pendant un tirage de lien ; nœud visé.
    let ghost = RwSignal::new(None::<(f64, f64)>);
    let drop_on = RwSignal::new(None::<String>);
    let notice = RwSignal::new(None::<String>);
    let say = move |t: String| {
        notice.set(Some(t.clone()));
        set_timeout(
            move || {
                if notice.try_get_untracked().flatten().as_deref() == Some(t.as_str()) {
                    let _ = notice.try_set(None);
                }
            },
            std::time::Duration::from_secs(6),
        );
    };

    // Nœuds liés au nœud sélectionné (ou survolé) : les autres s'effacent.
    let focus = Memo::new(move |_| {
        let key = match picked.get() {
            Some(Picked::Node(k)) => Some(k),
            Some(Picked::Edge(a, b)) => {
                return Some(HashSet::from([a, b]));
            }
            None => hovered.get(),
        }?;
        let mut set = HashSet::from([key.clone()]);
        map.with(|m| {
            for l in &m.links {
                if l.from == key {
                    set.insert(l.to.clone());
                }
                if l.to == key {
                    set.insert(l.from.clone());
                }
            }
        });
        Some(set)
    });
    let matches = Memo::new(move |_| {
        let q = query.get().trim().to_lowercase();
        if q.is_empty() {
            return None;
        }
        Some(map.with(|m| {
            m.nodes
                .iter()
                .filter(|n| {
                    n.label.to_lowercase().contains(&q)
                        || n.path
                            .as_deref()
                            .is_some_and(|p| p.to_lowercase().contains(&q))
                })
                .map(|n| n.key.clone())
                .collect::<HashSet<_>>()
        }))
    });
    let center_on = move |key: &str| {
        let Some((x, y, h)) = rect_of(key) else {
            return;
        };
        let Some(r) = stage_rect() else { return };
        let (_, _, k) = view_t.get_untracked();
        view_t.set((
            r.width() / 2.0 - (x + NODE_W / 2.0) * k,
            r.height() / 2.0 - (y + h / 2.0) * k,
            k,
        ));
    };

    // Entrée dans la recherche : premier nœud trouvé sélectionné et montré.
    let search_enter = move |e: leptos::ev::KeyboardEvent| {
        if e.key() != "Enter" {
            return;
        }
        let first = matches.get_untracked().and_then(|m| {
            map.with_untracked(|mm| {
                mm.nodes
                    .iter()
                    .find(|n| m.contains(&n.key))
                    .map(|n| n.key.clone())
            })
        });
        if let Some(k) = first {
            picked.set(Some(Picked::Node(k.clone())));
            center_on(&k);
        }
    };
    let world_at = move |cx: f64, cy: f64| -> Option<(f64, f64)> {
        let r = stage_rect()?;
        let (tx, ty, k) = view_t.get_untracked();
        Some(((cx - r.left() - tx) / k, (cy - r.top() - ty) / k))
    };
    let node_at = move |wx: f64, wy: f64| -> Option<String> {
        let p = pos.get_untracked();
        map.with_untracked(|m| {
            m.nodes
                .iter()
                .rev()
                .find(|n| {
                    p.get(&n.key).is_some_and(|&(x, y)| {
                        wx >= x && wx <= x + NODE_W && wy >= y && wy <= y + node_h(n.kind)
                    })
                })
                .map(|n| n.key.clone())
        })
    };

    // Écritures.
    let apply = move |cmd: Command| s.apply.run(cmd);
    let save_pos = move |key: String, p: (f64, f64)| {
        pending.update(|m| {
            m.insert(key.clone(), p);
        });
        apply(Command::SetMapPositions {
            positions: BTreeMap::from([(key, Some([p.0.round() as i32, p.1.round() as i32]))]),
        });
    };
    let rearrange = move || {
        let keys: Vec<String> = saved.with_untracked(|sv| sv.keys().cloned().collect());
        if keys.is_empty() {
            fit();
            return;
        }
        pending.set(HashMap::new());
        apply(Command::SetMapPositions {
            positions: keys.into_iter().map(|k| (k, None)).collect(),
        });
        set_timeout(fit, std::time::Duration::from_millis(250));
    };
    let link_of = move |i: usize| map.with_untracked(|m| m.links.get(i).cloned());
    // Destination des liens d'un trait changée.
    let retarget = move |from: String, to: String, new_to: String| {
        let Some(addr) = full.with_untracked(|m| lm::address_of(m, &new_to)) else {
            return;
        };
        let links: Vec<MapLink> = edges.with_untracked(|es| {
            es.iter()
                .find(|e| e.from == from && e.to == to)
                .map(|e| e.links.iter().filter_map(|&i| link_of(i)).collect())
                .unwrap_or_default()
        });
        let refs: Vec<&MapLink> = links.iter().filter(|l| l.retargetable()).collect();
        let n = refs.len();
        let cmd = s.doc.with_untracked(|d| {
            d.as_ref()
                .and_then(|d| lm::retarget(d, info.get_untracked().as_ref(), &refs, &addr))
        });
        if let Some(cmd) = cmd {
            apply(cmd);
            picked.set(Some(Picked::Edge(from, new_to)));
            say(ui.now("map-retargeted", &[("count", n.to_string()), ("to", addr)]));
        }
    };
    // Adresse d'un autre site remplacée partout où elle sert (D-107).
    let replace = Callback::new(move |(old, new): (String, String)| {
        let new = new.trim().to_string();
        if new.is_empty() || new == old {
            return;
        }
        let links: Vec<MapLink> =
            full.with_untracked(|m| m.links.iter().filter(|l| l.href == old).cloned().collect());
        let refs: Vec<&MapLink> = links.iter().collect();
        let cmd = s.doc.with_untracked(|d| {
            d.as_ref().and_then(|d| {
                lm::replace_url(
                    d,
                    info.get_untracked().as_ref(),
                    &texts.get_untracked(),
                    &refs,
                    &old,
                    &new,
                )
            })
        });
        let Some(cmd) = cmd else { return };
        let cms_touched = links
            .iter()
            .any(|l| matches!(l.via, LinkVia::Field { .. } | LinkVia::Body { .. }));
        apply(cmd);
        if cms_touched {
            let p = path.get_value();
            set_timeout(
                move || {
                    spawn_local(async move {
                        if let Ok(c) = api::cms_collections(&p).await {
                            let _ = cms.try_set(c);
                        }
                    })
                },
                std::time::Duration::from_millis(300),
            );
        }
        say(ui.now(
            "map-url-replaced",
            &[("count", links.len().to_string()), ("url", new.clone())],
        ));
        run_check.run((vec![new], "all"));
    });
    let ext = ExtTools {
        checks,
        checking,
        run_check,
        replace,
    };
    let last_added = RwSignal::new(None::<(String, String)>);
    // Lien ajouté : au menu (point d'un menu), à la fin de la page (point d'une page).
    let connect = move |from: String, to: String| {
        let Some((addr, label)) =
            full.with_untracked(|m| Some((lm::address_of(m, &to)?, m.node(&to)?.label.clone())))
        else {
            return;
        };
        if let Some(menu) = from.strip_prefix("menu:") {
            let Some(mut links) = info.with_untracked(|i| {
                i.as_ref().map(|i| {
                    revel_edit::siteinfo::menus(i)
                        .into_iter()
                        .find(|(n, _)| n == menu)
                        .map(|(_, l)| l)
                        .unwrap_or_default()
                })
            }) else {
                return;
            };
            links.push(revel_edit::siteinfo::MenuLink {
                label,
                url: addr.clone(),
            });
            let cmd = s.doc.with_untracked(|d| {
                d.as_ref().map(|d| {
                    revel_edit::siteinfo::set_value(
                        d,
                        &[revel_edit::siteinfo::MENUS.to_string(), menu.to_string()],
                        Some(revel_edit::siteinfo::menu_value(&links)),
                    )
                })
            });
            if let Some(cmd) = cmd {
                apply(cmd);
                say(ui.now("map-linked-menu", &[("to", addr)]));
            }
            return;
        }
        let Some(page) = from.strip_prefix("page:").map(str::to_string) else {
            return;
        };
        let cmd = s.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let p = d.pages.iter().find(|p| p.id == page)?;
            let mut taken = d.node_ids();
            let a_id = revel_model::ids::fresh(&taken);
            taken.insert(a_id.clone());
            let p_id = revel_model::ids::fresh(&taken);
            let node = Node::Element {
                id: Some(p_id),
                tag: "p".into(),
                attrs: vec![],
                children: vec![Node::Element {
                    id: Some(a_id.clone()),
                    tag: "a".into(),
                    attrs: vec![revel_model::Attr {
                        name: "href".into(),
                        value: Some(addr.clone()),
                        when: None,
                    }],
                    children: vec![Node::Text {
                        text: label.clone(),
                    }],
                }],
            };
            Some((
                Command::Insert {
                    parent: Parent::root(Owner::Page(page.clone())),
                    index: p.body.len(),
                    node,
                },
                p.path.clone(),
                a_id,
            ))
        });
        if let Some((cmd, page_path, a_id)) = cmd {
            apply(cmd);
            say(ui.now("map-linked-page", &[("to", addr)]));
            last_added.set(Some((page_path, a_id)));
        }
    };
    let open_in_editor = move |page_path: String, node: Option<String>| {
        ui.goto_node.set(node);
        ui.goto_page.set(Some(page_path));
        ui.tab(Tab::Editor);
    };

    let minimap = NodeRef::<leptos::html::Div>::new();
    // Minicarte : la vue centrée sur le point visé.
    let minimap_go = move |cx: f64, cy: f64| {
        let (Some(mm), Some(r)) = (minimap.get_untracked(), stage_rect()) else {
            return;
        };
        let mr = mm.get_bounding_client_rect();
        let (x0, y0, x1, y1) = bounds.get_untracked();
        let sc = (mr.width() / (x1 - x0).max(1.0)).min(mr.height() / (y1 - y0).max(1.0));
        let wx = x0 + (cx - mr.left()) / sc;
        let wy = y0 + (cy - mr.top()) / sc;
        let (_, _, k) = view_t.get_untracked();
        view_t.set((r.width() / 2.0 - wx * k, r.height() / 2.0 - wy * k, k));
    };
    // Gestes.
    let on_down = move |e: web_sys::PointerEvent| {
        if e.button() != 0 && e.button() != 1 {
            return;
        }
        let Some(el) = stage.get_untracked() else {
            return;
        };
        let ev: &web_sys::Event = e.as_ref();
        if closest(ev, ".map-panel, .map-toolbar, .map-zoom").is_some() {
            return;
        }
        let (cx, cy) = (f64::from(e.client_x()), f64::from(e.client_y()));
        let g = if e.button() == 1 {
            let (tx, ty, _) = view_t.get_untracked();
            Some(Gesture::Pan {
                sx: cx,
                sy: cy,
                t0: (tx, ty),
            })
        } else if closest(ev, ".map-minimap").is_some() {
            Some(Gesture::Minimap)
        } else if let Some(h) = closest(ev, "[data-handle]") {
            let v = h.get_attribute("data-handle").unwrap_or_default();
            v.split_once('|').map(|(a, b)| Gesture::Retarget {
                from: a.to_string(),
                to: b.to_string(),
            })
        } else if let Some(p) = closest(ev, ".map-port") {
            p.closest("[data-node]")
                .ok()
                .flatten()
                .and_then(|n| n.get_attribute("data-node"))
                .map(|from| Gesture::Connect { from })
        } else if closest(ev, ".map-expand, .map-node button").is_some() {
            None
        } else if let Some(n) = closest(ev, "[data-node]") {
            let key = n.get_attribute("data-node").unwrap_or_default();
            pos.with_untracked(|p| p.get(&key).copied())
                .map(|p0| Gesture::Move {
                    key,
                    sx: cx,
                    sy: cy,
                    p0,
                    moved: false,
                })
        } else if let Some(h) = closest(ev, "[data-edge]") {
            let v = h.get_attribute("data-edge").unwrap_or_default();
            if let Some((a, b)) = v.split_once('|') {
                picked.set(Some(Picked::Edge(a.to_string(), b.to_string())));
            }
            None
        } else {
            let (tx, ty, _) = view_t.get_untracked();
            Some(Gesture::Pan {
                sx: cx,
                sy: cy,
                t0: (tx, ty),
            })
        };
        if let Some(g) = g {
            let _ = el.set_pointer_capture(e.pointer_id());
            if matches!(g, Gesture::Minimap) {
                minimap_go(cx, cy);
            }
            gesture.set_value(Some(g));
            e.prevent_default();
            let _ = el.focus();
        }
    };
    let on_move = move |e: web_sys::PointerEvent| {
        let (cx, cy) = (f64::from(e.client_x()), f64::from(e.client_y()));
        let Some(g) = gesture.get_value() else {
            return;
        };
        match g {
            Gesture::Pan { sx, sy, t0 } => {
                let (_, _, k) = view_t.get_untracked();
                view_t.set((t0.0 + cx - sx, t0.1 + cy - sy, k));
            }
            Gesture::Move {
                key,
                sx,
                sy,
                p0,
                moved,
            } => {
                let (_, _, k) = view_t.get_untracked();
                let (dx, dy) = ((cx - sx) / k, (cy - sy) / k);
                if moved || dx.abs() + dy.abs() > 3.0 / k {
                    // Pas de 8 px (Maj : libre).
                    let snap = |v: f64| {
                        if e.shift_key() {
                            v
                        } else {
                            (v / 8.0).round() * 8.0
                        }
                    };
                    dragging.set(Some((key.clone(), (snap(p0.0 + dx), snap(p0.1 + dy)))));
                    gesture.set_value(Some(Gesture::Move {
                        key,
                        sx,
                        sy,
                        p0,
                        moved: true,
                    }));
                }
            }
            Gesture::Retarget { from, .. } | Gesture::Connect { from } => {
                if let Some((wx, wy)) = world_at(cx, cy) {
                    ghost.set(Some((wx, wy)));
                    let target = node_at(wx, wy).filter(|k| {
                        *k != from && map.with_untracked(|m| m.node(k).is_some_and(linkable))
                    });
                    drop_on.set(target);
                }
            }
            Gesture::Minimap => minimap_go(cx, cy),
        }
    };
    let on_up = move |e: web_sys::PointerEvent| {
        let Some(g) = gesture.get_value() else { return };
        gesture.set_value(None);
        if let Some(el) = stage.get_untracked() {
            let _ = el.release_pointer_capture(e.pointer_id());
        }
        match g {
            Gesture::Pan { sx, sy, .. } => {
                // Clic sur le fond : rien de sélectionné.
                let (cx, cy) = (f64::from(e.client_x()), f64::from(e.client_y()));
                if (cx - sx).abs() + (cy - sy).abs() < 4.0 {
                    picked.set(None);
                }
            }
            Gesture::Move { key, moved, .. } => {
                if moved {
                    if let Some((_, p)) = dragging.get_untracked() {
                        save_pos(key.clone(), p);
                    }
                    dragging.set(None);
                }
                picked.set(Some(Picked::Node(key)));
            }
            Gesture::Retarget { from, to } => {
                if let Some(t) = drop_on.get_untracked()
                    && t != to
                {
                    retarget(from, to, t);
                }
                ghost.set(None);
                drop_on.set(None);
            }
            Gesture::Connect { from } => {
                if let Some(t) = drop_on.get_untracked() {
                    connect(from, t);
                }
                ghost.set(None);
                drop_on.set(None);
            }
            Gesture::Minimap => {}
        }
    };
    let on_wheel = move |e: web_sys::WheelEvent| {
        let ev: &web_sys::Event = e.as_ref();
        if closest(ev, ".map-panel").is_some() {
            return;
        }
        e.prevent_default();
        let Some(r) = stage_rect() else { return };
        // Lignes ou pages : en pixels.
        let unit = match e.delta_mode() {
            1 => 16.0,
            2 => r.height(),
            _ => 1.0,
        };
        let (dx, dy) = (e.delta_x() * unit, e.delta_y() * unit);
        // Zoom : Alt, Ctrl ou Cmd + molette, comme le canevas d'édition.
        if e.alt_key() || e.ctrl_key() || e.meta_key() {
            // Avec Alt, certains systèmes envoient le défilement en horizontal.
            let dy = if dy == 0.0 { dx } else { dy };
            let f = (-dy * 0.0025).exp();
            zoom_at(
                f64::from(e.client_x()) - r.left(),
                f64::from(e.client_y()) - r.top(),
                f,
            );
        } else {
            let (tx, ty, k) = view_t.get_untracked();
            let (dx, dy) = if e.shift_key() && dx == 0.0 {
                (dy, 0.0)
            } else {
                (dx, dy)
            };
            view_t.set((tx - dx, ty - dy, k));
        }
    };
    let on_dbl = move |e: web_sys::MouseEvent| {
        let ev: &web_sys::Event = e.as_ref();
        if closest(ev, ".map-panel, .map-toolbar").is_some() {
            return;
        }
        if closest(ev, ".map-minimap, .map-zoom, .map-expand").is_some() {
            return;
        }
        // Nœud sous le pointeur (le plan a capturé le pointeur : la cible est le plan).
        let key = world_at(f64::from(e.client_x()), f64::from(e.client_y()))
            .and_then(|(wx, wy)| node_at(wx, wy));
        let Some(key) = key else {
            // Double-clic sur le fond : tout le plan.
            fit();
            return;
        };
        let node = map.with_untracked(|m| m.node(&key).cloned());
        match node {
            Some(n) if linkable(&n) => open_in_editor(n.path.unwrap_or_default(), None),
            Some(n) if n.kind == NodeKind::Collection => open.update(|o| {
                if !o.remove(&n.key) {
                    o.insert(n.key.clone());
                }
            }),
            _ => {}
        }
    };
    let undo = move |redo: bool| {
        let p = path.get_value();
        spawn_local(async move {
            let r = if redo {
                api::edit_redo(&p).await
            } else {
                api::edit_undo(&p).await
            };
            if let Err(e) = r {
                ui.fail(e);
            }
            if let Ok(o) = api::edit_open(&p).await
                && !s.doc.is_disposed()
            {
                pending.set(HashMap::new());
                s.doc.set(Some(o.site));
            }
            // Boutons Annuler et Rétablir de la barre du haut mis à jour.
            ui.doc_rev.update(|n| *n += 1);
        });
    };
    // Annulation par les boutons de la barre du haut : plan relu.
    Effect::new(move |prev: Option<u32>| {
        let n = ui.undo_rev.get();
        if prev.is_some_and(|p| p != n) {
            let p = path.get_value();
            spawn_local(async move {
                if let Ok(o) = api::edit_open(&p).await
                    && !s.doc.is_disposed()
                {
                    pending.set(HashMap::new());
                    s.doc.set(Some(o.site));
                }
            });
        }
        n
    });
    let on_key = move |e: web_sys::KeyboardEvent| {
        let ev: &web_sys::Event = e.as_ref();
        if closest(ev, "input, textarea, select, [contenteditable]").is_some() {
            return;
        }
        let ctrl = e.ctrl_key() || e.meta_key();
        let k = e.key();
        let done = match k.as_str() {
            "z" | "Z" if ctrl => {
                undo(e.shift_key());
                true
            }
            "y" if ctrl => {
                undo(true);
                true
            }
            "+" | "=" => {
                zoom_center(1.2);
                true
            }
            "-" | "_" => {
                zoom_center(1.0 / 1.2);
                true
            }
            "0" if !ctrl => {
                zoom_center(1.0 / view_t.get_untracked().2);
                true
            }
            "f" | "F" | "1" if !ctrl => {
                fit();
                true
            }
            "Escape" => {
                picked.set(None);
                true
            }
            "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" => {
                let (tx, ty, kk) = view_t.get_untracked();
                let step = if e.shift_key() { 240.0 } else { 60.0 };
                let (dx, dy) = match k.as_str() {
                    "ArrowLeft" => (step, 0.0),
                    "ArrowRight" => (-step, 0.0),
                    "ArrowUp" => (0.0, step),
                    _ => (0.0, -step),
                };
                view_t.set((tx + dx, ty + dy, kk));
                true
            }
            "Enter" => {
                if let Some(Picked::Node(key)) = picked.get_untracked()
                    && let Some(n) = map.with_untracked(|m| m.node(&key).cloned())
                    && linkable(&n)
                {
                    open_in_editor(n.path.unwrap_or_default(), None);
                }
                true
            }
            _ => false,
        };
        if done {
            e.prevent_default();
        }
    };
    let on_context = move |e: web_sys::MouseEvent| {
        let ev: &web_sys::Event = e.as_ref();
        let Some(n) = closest(ev, "[data-node]") else {
            return;
        };
        let key = n.get_attribute("data-node").unwrap_or_default();
        picked.set(Some(Picked::Node(key.clone())));
        let items = node_actions(ui, s, map, open, &key, open_in_editor);
        crate::ctxmenu::open(ui, &e, items);
    };

    // Vue.
    let world_style = move || {
        let (tx, ty, k) = view_t.get();
        format!("transform:translate({tx:.1}px,{ty:.1}px) scale({k:.4})")
    };
    let svg_box = move || {
        let (x0, y0, x1, y1) = bounds.get();
        (x0 - 400.0, y0 - 400.0, x1 - x0 + 800.0, y1 - y0 + 800.0)
    };
    let edge_views = move || {
        let f = focus.get();
        let sel = picked.get();
        edges
            .get()
            .into_iter()
            .filter_map(|e| {
                let a = rect_of(&e.from)?;
                let b = rect_of(&e.to)?;
                let d = curve(a, b);
                let lit = f.as_ref().is_some_and(|f| f.contains(&e.from) && f.contains(&e.to))
                    && !matches!(&sel, Some(Picked::Edge(..)));
                let on = sel == Some(Picked::Edge(e.from.clone(), e.to.clone()));
                let dim = f.is_some() && !lit && !on;
                let w = 1.4 + (e.links.len().min(5) as f64 - 1.0) * 0.45;
                let id = format!("{}|{}", e.from, e.to);
                let shared = ["comp:", "layout:", "menu:"]
                    .iter()
                    .any(|p| e.from.starts_with(p));
                let class = format!(
                    "map-edge map-edge--{}{}{}{}{}",
                    e.kind.class(),
                    if shared { " is-shared" } else { "" },
                    if lit { " is-lit" } else { "" },
                    if on { " is-on" } else { "" },
                    if dim { " is-dim" } else { "" }
                );
                let marker = format!("url(#map-arrow-{})", e.kind.class());
                Some(view! {
                    <g class=class>
                        <path class="map-edge-line" d=d.clone() stroke-width=format!("{w:.2}") marker-end=marker/>
                        <path class="map-edge-hit" d=d data-edge=id.clone()/>
                    </g>
                })
            })
            .collect_view()
    };
    // Bout du lien sélectionné (à tirer sur une autre page) ; trait en cours de tirage.
    let handle_view = move || {
        let Some(Picked::Edge(from, to)) = picked.get() else {
            return ().into_any();
        };
        let movable = edges.with(|es| {
            es.iter()
                .find(|e| e.from == from && e.to == to)
                .is_some_and(|e| {
                    e.links
                        .iter()
                        .any(|&i| map.with(|m| m.links[i].retargetable()))
                })
        });
        let (Some(a), Some(b)) = (rect_of(&from), rect_of(&to)) else {
            return ().into_any();
        };
        if !movable || ghost.with(Option::is_some) {
            return ().into_any();
        }
        // Bout du trait, juste avant la flèche.
        let (_, _, x, y, _, c2) = route(a, b);
        let x = x + c2.signum() * 4.0;
        view! {
            <circle class="map-handle" cx=format!("{x:.1}") cy=format!("{y:.1}") r="7"
                data-handle=format!("{from}|{to}")></circle>
        }
        .into_any()
    };
    let ghost_view = move || {
        let (gx, gy) = ghost.get()?;
        let from = match gesture.get_value()? {
            Gesture::Retarget { from, .. } | Gesture::Connect { from } => from,
            _ => return None,
        };
        let a = rect_of(&from)?;
        let d = ghost_curve(a, (gx, gy));
        Some(view! { <path class="map-ghost" d=d marker-end="url(#map-arrow-link)"/> })
    };
    let node_views = move || {
        let f = focus.get();
        let m = matches.get();
        let sel = picked.get();
        let orph = orphans.get();
        let drop = drop_on.get();
        let p = pos.get();
        map.get()
            .nodes
            .into_iter()
            .filter_map(|n| {
                let (x, y) = *p.get(&n.key)?;
                let key = n.key.clone();
                let on = sel == Some(Picked::Node(key.clone()));
                let dim = f.as_ref().is_some_and(|f| !f.contains(&key))
                    || m.as_ref().is_some_and(|m| !m.contains(&key));
                let hit = m.as_ref().is_some_and(|m| m.contains(&key));
                let orphan = orph.contains(&key);
                let kind = n.kind;
                let icon = match kind {
                    NodeKind::Page => icons::page(),
                    NodeKind::Collection => icons::database(),
                    NodeKind::Entry => icons::file(),
                    NodeKind::Layout => icons::template(),
                    NodeKind::Component => icons::component(),
                    NodeKind::Menu => icons::menu(),
                    NodeKind::Missing => icons::warning(),
                    NodeKind::External => icons::globe(),
                    NodeKind::Media => match lm::platform(&n.key).map(|p| p.2) {
                        Some(PlatformKind::Video) => icons::video(),
                        Some(PlatformKind::Audio) => icons::audio(),
                        _ => icons::share(),
                    },
                };
                let kind_class = match kind {
                    NodeKind::Page => "page",
                    NodeKind::Collection => "collection",
                    NodeKind::Entry => "entry",
                    NodeKind::Layout => "layout",
                    NodeKind::Component => "component",
                    NodeKind::Menu => "menu",
                    NodeKind::Missing => "missing",
                    NodeKind::External => "external",
                    NodeKind::Media => "media",
                };
                // Adresses d'un autre site ou d'une plateforme : état le plus grave vérifié.
                let urls: Vec<String> = matches!(kind, NodeKind::External | NodeKind::Media)
                    .then(|| full.with(|fm| {
                        let mut v: Vec<String> = fm.links.iter().filter(|l| l.to == key).map(|l| l.href.clone()).collect();
                        v.sort();
                        v.dedup();
                        v
                    }))
                    .unwrap_or_default();
                let status = (!urls.is_empty()).then(|| {
                    let c = checks.get();
                    let worst = urls.iter().filter_map(|u| c.get(u)).map(|c| c.state).max_by_key(|s| severity(*s));
                    let n_bad = urls.iter().filter(|u| c.get(*u).is_some_and(|c| severity(c.state) >= 2)).count();
                    match worst {
                        Some(w) if severity(w) >= 2 => {
                            let (cls, _) = state_key(w);
                            view! { <span class=format!("map-status-dot map-status-dot--{cls}") title=ui.now("map-urls-review", &[("count", n_bad.to_string())])></span> }.into_any()
                        }
                        Some(w) => {
                            let (cls, label) = state_key(w);
                            view! { <span class=format!("map-status-dot map-status-dot--{cls}") title=ui.now(label, &[])></span> }.into_any()
                        }
                        None => ().into_any(),
                    }
                });
                let sub = match kind {
                    NodeKind::Page | NodeKind::Missing | NodeKind::Collection => {
                        readable_path(&n.path.clone().unwrap_or_default())
                    }
                    NodeKind::Menu => ui.now("map-menu-links", &[("count", n.count.unwrap_or(0).to_string())]),
                    NodeKind::Component | NodeKind::Layout => ui.now(
                        "map-shared-pages",
                        &[("count", n.count.unwrap_or(0).to_string())],
                    ),
                    NodeKind::Media => ui.now("map-urls-count", &[("count", urls.len().to_string())]),
                    _ => String::new(),
                };
                let port = matches!(kind, NodeKind::Page | NodeKind::Menu)
                    && n.path.as_deref().is_none_or(|p| !p.contains("{{"));
                let expand = (kind == NodeKind::Collection).then(|| {
                    let k2 = key.clone();
                    let opened = open.with(|o| o.contains(&key));
                    let count = n.count.unwrap_or(0);
                    view! {
                        <button class="map-expand" class:is-open=opened disabled={count == 0}
                            title=ui.t("map-entries-help")
                            on:click=move |_| open.update(|o| { if !o.remove(&k2) { o.insert(k2.clone()); } })>
                            {icons::chevron_right()}
                            {ui.now("map-entries", &[("count", count.to_string())])}
                        </button>
                    }
                });
                let label = n.label.clone();
                let draft = n.draft;
                Some(view! {
                    <div class=format!("map-node map-node--{kind_class}") data-node=key.clone()
                        class:is-on=on class:is-dim=dim class:is-hit=hit class:is-draft=draft
                        class:is-orphan=orphan class:is-drop={drop.as_deref() == Some(key.as_str())}
                        style=format!("transform:translate({x:.1}px,{y:.1}px)")
                        on:pointerenter={let k = key.clone(); move |_| hovered.set(Some(k.clone()))}
                        on:pointerleave=move |_| hovered.set(None)>
                        <div class="map-node-head">
                            <span class="map-node-icon">{icon}</span>
                            <span class="map-node-name" title=label.clone()>{label.clone()}</span>
                            {draft.then(|| view! { <span class="map-badge" title=ui.t("map-draft-help")>{ui.t("map-draft")}</span> })}
                            {orphan.then(|| view! { <span class="map-badge map-badge--warn" title=ui.t("map-orphan-help")>{icons::unlink()}</span> })}
                            {status}
                        </div>
                        {(!sub.is_empty()).then(|| view! { <div class="map-node-sub" title=sub.clone()>{crate::ltr(&sub)}</div> })}
                        {expand}
                        {port.then(|| view! { <span class="map-port" title=ui.t("map-port-help")></span> })}
                    </div>
                })
            })
            .collect_view()
    };
    let minimap_view = move || {
        let (x0, y0, x1, y1) = bounds.get();
        let (w, h) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
        let p = pos.get();
        let rects = map.with(|m| {
            m.nodes
                .iter()
                .filter_map(|n| {
                    let (x, y) = p.get(&n.key)?;
                    Some((x - x0, y - y0, node_h(n.kind), n.kind))
                })
                .collect::<Vec<_>>()
        });
        let (tx, ty, k) = view_t.get();
        let (sw, sh) = stage_rect()
            .map(|r| (r.width(), r.height()))
            .unwrap_or((800.0, 600.0));
        let vx = -tx / k - x0;
        let vy = -ty / k - y0;
        view! {
            <svg viewBox=format!("0 0 {w:.0} {h:.0}") preserveAspectRatio="xMidYMid meet">
                {rects.into_iter().map(|(x, y, hh, kind)| view! {
                    <rect class=format!("map-mini-node{}", if kind == NodeKind::Missing { " is-missing" } else { "" })
                        x=format!("{x:.0}") y=format!("{y:.0}") width=format!("{NODE_W:.0}") height=format!("{hh:.0}") rx="6"/>
                }).collect_view()}
                <rect class="map-mini-view" x=format!("{vx:.0}") y=format!("{vy:.0}")
                    width=format!("{:.0}", sw / k) height=format!("{:.0}", sh / k)/>
            </svg>
        }
    };
    // Groupe montré (autres sites, médias) : plan ajusté pour que ses nœuds soient visibles.
    let show_group = move |g: RwSignal<bool>| {
        g.update(|v| *v = !*v);
        if g.get_untracked() {
            set_timeout(fit, std::time::Duration::from_millis(80));
        }
    };
    let counts = move || {
        map.with(|m| {
            let pages = m.nodes.iter().filter(|n| n.kind == NodeKind::Page).count();
            let broken = m
                .nodes
                .iter()
                .filter(|n| n.kind == NodeKind::Missing)
                .count();
            (pages, broken, orphans.with(|o| o.len()))
        })
    };

    view! {
        <div class="map" class:has-panel=move || picked.with(Option::is_some)>
            <div class="map-toolbar">
                <label class="map-search">
                    {icons::search()}
                    <input class="input" type="search" placeholder=ui.t("map-search") aria-label=ui.t("map-search")
                        prop:value=move || query.get()
                        on:input=move |e| query.set(event_target_value(&e))
                        on:keydown=move |e: leptos::ev::KeyboardEvent| search_enter(e)/>
                </label>
                <div class="segmented map-filters" role="group">
                    <button class:is-on=move || show_shared.get() aria-pressed=move || show_shared.get().to_string()
                        title=ui.t("map-shared-help") on:click=move |_| show_shared.update(|v| *v = !*v)>{icons::component()}{ui.t("map-shared")}</button>
                    <button class:is-on=move || show_external.get() aria-pressed=move || show_external.get().to_string()
                        title=ui.t("map-external-help") on:click=move |_| show_group(show_external)>{icons::globe()}{ui.t("map-external")}</button>
                    <button class:is-on=move || show_media.get() aria-pressed=move || show_media.get().to_string()
                        title=ui.t("map-media-help") on:click=move |_| show_group(show_media)>{icons::video()}{ui.t("map-media")}</button>
                </div>
                <span class="map-counts muted small">
                    {move || {
                        let (p, b, o) = counts();
                        view! {
                            <span>{ui.now("map-count-pages", &[("count", p.to_string())])}</span>
                            {(b > 0).then(|| view! { <span class="map-count-warn" title=ui.t("map-broken-help")>{icons::warning()}{ui.now("map-count-broken", &[("count", b.to_string())])}</span> })}
                            {(o > 0).then(|| view! { <span class="map-count-warn" title=ui.t("map-orphan-help")>{icons::unlink()}{ui.now("map-count-orphans", &[("count", o.to_string())])}</span> })}
                            {move || { let r = to_review.get(); (r > 0).then(|| view! {
                                <button class="map-count-warn map-count-urls" title=ui.t("map-urls-review-help")
                                    on:click=move |_| { show_media.set(true); show_external.set(true); set_timeout(fit, std::time::Duration::from_millis(80)); }>
                                    {icons::warning()}{ui.now("map-urls-review", &[("count", r.to_string())])}
                                </button>
                            }) }}
                            {move || checking.get().then(|| view! { <span class="map-checking">{ui.t("map-urls-checking")}</span> })}
                        }
                    }}
                </span>
                <span class="toolbar-spacer"></span>
                {move || notice.get().map(|t| view! {
                    <span class="map-notice" role="status">
                        {t}
                        {last_added.get().map(|(p, id)| view! {
                            <button class="link-btn" on:click=move |_| open_in_editor(p.clone(), Some(id.clone()))>{ui.t("map-show-in-editor")}</button>
                        })}
                    </span>
                })}
                <button class="btn btn--small" title=ui.t("map-arrange-help") on:click=move |_| rearrange()>{icons::grid()}{ui.t("map-arrange")}</button>
            </div>
            <div class="map-stage" node_ref=stage tabindex="0" aria-label=ui.t("view-map")
                class:is-linking=move || ghost.with(Option::is_some)
                on:pointerdown=on_down on:pointermove=on_move on:pointerup=on_up on:pointercancel=on_up
                on:wheel=on_wheel on:dblclick=on_dbl on:keydown=on_key on:contextmenu=on_context>
                <div class="map-world" style=world_style>
                    {move || {
                        let (bx, by, bw, bh) = svg_box();
                        view! {
                            <svg class="map-edges" style=format!("left:{bx:.0}px;top:{by:.0}px")
                                width=format!("{bw:.0}") height=format!("{bh:.0}")
                                viewBox=format!("{bx:.0} {by:.0} {bw:.0} {bh:.0}")>
                                <defs>
                                    {["link", "list", "menu", "redirect", "broken", "external"].into_iter().map(|k| view! {
                                        <marker id=format!("map-arrow-{k}") class=format!("map-arrow map-arrow--{k}")
                                            viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7"
                                            markerUnits="userSpaceOnUse" orient="auto">
                                            <path d="M0,1 L9,5 L0,9 z"/>
                                        </marker>
                                    }).collect_view()}
                                </defs>
                                {edge_views}
                                {ghost_view}
                                {handle_view}
                            </svg>
                        }
                    }}
                    {node_views}
                </div>
                <div class="map-minimap" node_ref=minimap title=ui.t("map-minimap")>{minimap_view}</div>
                <div class="map-zoom">
                    <button class="icon-btn icon-btn--small" title=ui.t("map-zoom-out") aria-label=ui.t("map-zoom-out")
                        on:click=move |_| zoom_center(1.0 / 1.2)>"−"</button>
                    <button class="map-zoom-value" title=ui.t("map-zoom-reset") on:click=move |_| zoom_center(1.0 / view_t.get_untracked().2)>
                        {move || format!("{:.0} %", view_t.get().2 * 100.0)}
                    </button>
                    <button class="icon-btn icon-btn--small" title=ui.t("map-zoom-in") aria-label=ui.t("map-zoom-in")
                        on:click=move |_| zoom_center(1.2)>"+"</button>
                    <button class="icon-btn icon-btn--small" title=ui.t("map-fit") aria-label=ui.t("map-fit")
                        on:click=move |_| fit()>{icons::expand()}</button>
                </div>
                <div class="map-legend muted small">
                    <span><i class="map-key map-key--link"></i>{ui.t("map-key-link")}</span>
                    <span><i class="map-key map-key--list"></i>{ui.t("map-key-list")}</span>
                    <span><i class="map-key map-key--menu"></i>{ui.t("map-key-menu")}</span>
                    <span><i class="map-key map-key--redirect"></i>{ui.t("map-key-redirect")}</span>
                    <span><i class="map-key map-key--broken"></i>{ui.t("map-key-broken")}</span>
                </div>
            </div>
            {move || picked.get().map(|p| view! {
                <aside class="map-panel" aria-label=ui.t("map-panel")>
                    {panel(ui, s, map, full, edges, open, cms_id, p, picked, retarget, open_in_editor, ext)}
                </aside>
            })}
        </div>
    }
}

/// Actions d'un nœud (menu contextuel et panneau).
fn node_actions(
    ui: Ui,
    s: DocSession,
    map: Memo<LinkMap>,
    open: RwSignal<HashSet<String>>,
    key: &str,
    open_in_editor: impl Fn(String, Option<String>) + Copy + Send + Sync + 'static,
) -> Vec<crate::ctxmenu::CtxItem> {
    use crate::ctxmenu::CtxItem;
    let Some(n) = map.with_untracked(|m| m.node(key).cloned()) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    match n.kind {
        NodeKind::Page => {
            let id = key.trim_start_matches("page:").to_string();
            let p = n.path.clone().unwrap_or_default();
            if linkable(&n) {
                let p2 = p.clone();
                items.push(
                    CtxItem::new(ui.now("map-open", &[]), move || {
                        open_in_editor(p2.clone(), None)
                    })
                    .icon(icons::edit),
                );
            }
            let id1 = id.clone();
            let folder = crate::pageops::folder_of(&format!("{p}x/"));
            items.push(
                CtxItem::new(ui.now("map-new-child", &[]), move || {
                    new_page(ui, s, &folder)
                })
                .icon(icons::plus)
                .sep(),
            );
            items.push(
                CtxItem::new(ui.now("pages-duplicate", &[]), move || {
                    duplicate(ui, s, &id1)
                })
                .icon(icons::copy),
            );
            let id2 = id.clone();
            items.push(
                CtxItem::new(ui.now("pages-delete", &[]), move || delete(ui, s, &id2))
                    .icon(icons::trash)
                    .danger()
                    .disabled(n.path.as_deref() == Some("/"))
                    .sep(),
            );
        }
        NodeKind::Entry => {
            let p = n.path.clone().unwrap_or_default();
            items.push(
                CtxItem::new(ui.now("map-open", &[]), move || {
                    open_in_editor(p.clone(), None)
                })
                .icon(icons::edit),
            );
        }
        NodeKind::Collection => {
            let k = key.to_string();
            let opened = open.with_untracked(|o| o.contains(key));
            items.push(
                CtxItem::new(
                    ui.now(
                        if opened {
                            "map-entries-hide"
                        } else {
                            "map-entries-show"
                        },
                        &[],
                    ),
                    move || {
                        open.update(|o| {
                            if !o.remove(&k) {
                                o.insert(k.clone());
                            }
                        })
                    },
                )
                .icon(icons::list),
            );
            items.push(
                CtxItem::new(ui.now("map-open-cms", &[]), move || ui.tab(Tab::Cms))
                    .icon(icons::database),
            );
        }
        NodeKind::Menu => {
            items.push(
                CtxItem::new(ui.now("map-edit-menus", &[]), move || {
                    ui.tab(Tab::Settings(Section::Menus))
                })
                .icon(icons::menu),
            );
        }
        NodeKind::Missing => {
            let p = n.path.clone().unwrap_or_default();
            items.push(
                CtxItem::new(ui.now("map-create-page", &[]), move || create_at(s, &p))
                    .icon(icons::plus),
            );
        }
        NodeKind::External => {
            let host = n.label.clone();
            items.push(
                CtxItem::new(ui.now("map-open-site", &[]), move || {
                    let url = format!("https://{host}");
                    spawn_local(async move {
                        let _ = api::open_external(&url).await;
                    });
                })
                .icon(icons::external),
            );
        }
        NodeKind::Component | NodeKind::Layout | NodeKind::Media => {}
    }
    items
}

/// Nouvelle page dans le dossier `folder` (nom demandé).
fn new_page(ui: Ui, s: DocSession, folder: &str) {
    let folder = folder.to_string();
    ui.ask.set(Some(Ask::input(
        ui.now("pages-new", &[]),
        ui.now("pages-name", &[]),
        String::new(),
        ui.now("action-create", &[]),
        move |name| {
            let name = name.trim().to_string();
            if name.is_empty() {
                return;
            }
            let cmd = s.doc.with_untracked(|d| {
                d.as_ref().map(|d| Command::AddPage {
                    page: revel_edit::make::new_page(d, &name, &folder),
                    index: None,
                })
            });
            if let Some(cmd) = cmd {
                s.apply.run(cmd);
            }
        },
    )));
}

/// Page créée à une adresse sans page (nom tiré de l'adresse).
fn create_at(s: DocSession, path: &str) {
    let folder = crate::pageops::folder_of(path);
    let slug = crate::pageops::slug_of(path);
    let name = crate::site::capitalize(&slug.replace('-', " "));
    let cmd = s.doc.with_untracked(|d| {
        d.as_ref().map(|d| {
            let mut page = revel_edit::make::new_page(d, &name, &folder);
            if revel_model::valid_page_path(path) && !d.pages.iter().any(|p| p.path == path) {
                page.path = path.to_string();
            }
            Command::AddPage { page, index: None }
        })
    });
    if let Some(cmd) = cmd {
        s.apply.run(cmd);
    }
}

fn duplicate(ui: Ui, s: DocSession, id: &str) {
    let word = ui.now("pages-copy-word", &[]);
    let cmd = s.doc.with_untracked(|d| {
        d.as_ref()
            .and_then(|d| revel_edit::make::duplicate_page(d, id, &word).ok())
            .map(|page| Command::AddPage { page, index: None })
    });
    if let Some(cmd) = cmd {
        s.apply.run(cmd);
    }
}

fn delete(ui: Ui, s: DocSession, id: &str) {
    let Some((path, name)) = s.doc.with_untracked(|d| {
        d.as_ref()?.pages.iter().find(|p| p.id == id).map(|p| {
            (
                p.path.clone(),
                p.name.clone().unwrap_or_else(|| p.path.clone()),
            )
        })
    }) else {
        return;
    };
    if path == "/" {
        return;
    }
    let id = id.to_string();
    ui.ask.set(Some(
        Ask::confirm(
            ui.now("pages-delete-title", &[("name", name)]),
            Some(ui.now("pages-delete-body", &[])),
            ui.now("pages-delete", &[]),
            move || s.apply.run(Command::RemovePage { id: id.clone() }),
        )
        .danger(),
    ));
}

/// Nouvelle adresse d'une page (redirection de l'ancienne si la page est publiée) ; `false` si
/// refusée.
fn set_path(s: DocSession, id: &str, path: &str) -> bool {
    let Some(Some((old, draft, ok))) = s.doc.with_untracked(|d| {
        d.as_ref().map(|d| {
            let p = d.pages.iter().find(|p| p.id == id)?;
            Some((
                p.path.clone(),
                p.draft,
                revel_model::valid_page_path(path) && !d.pages.iter().any(|x| x.path == path),
            ))
        })
    }) else {
        return false;
    };
    if old == path {
        return true;
    }
    if !ok || old == "/" {
        return false;
    }
    let mut commands = vec![Command::SetPagePath {
        id: id.to_string(),
        path: path.to_string(),
    }];
    if !draft {
        let redirects = s.doc.with_untracked(|d| {
            d.as_ref()
                .map(|d| crate::pageops::redirects_after(d, &[(old.clone(), path.to_string())]))
                .unwrap_or_default()
        });
        commands.push(Command::SetRedirects { redirects });
    }
    s.apply.run(Command::Batch { commands });
    true
}

/// Panneau de l'élément sélectionné : réglages et liens.
#[allow(clippy::too_many_arguments)]
fn panel(
    ui: Ui,
    s: DocSession,
    map: Memo<LinkMap>,
    full: Memo<LinkMap>,
    edges: Memo<Vec<Edge>>,
    open: RwSignal<HashSet<String>>,
    cms_id: Memo<HashMap<String, String>>,
    p: Picked,
    picked: RwSignal<Option<Picked>>,
    retarget: impl Fn(String, String, String) + Copy + Send + Sync + 'static,
    open_in_editor: impl Fn(String, Option<String>) + Copy + Send + Sync + 'static,
    ext: ExtTools,
) -> AnyView {
    let name_of = move |k: &str| {
        map.with_untracked(|m| m.node(k).map(|n| n.label.clone()))
            .unwrap_or_else(|| k.to_string())
    };
    let close = view! {
        <button class="icon-btn icon-btn--small map-panel-close" title=ui.t("action-close") aria-label=ui.t("action-close")
            on:click=move |_| picked.set(None)>{icons::close()}</button>
    };
    // Ligne d'un lien : texte, adresse ; « Montrer » (lien écrit dans une page).
    // Ligne d'un nœud : le lien (trait) sélectionné ; ligne d'un trait : le nœud d'où part le lien.
    let link_row = move |l: MapLink, other: String, pick: Picked| {
        let show = shown_at(s, &l.via);
        let kind = via_label(ui, &l.via);
        // Texte du lien, sinon son adresse, sinon sa sorte (liste, menu).
        let text = [l.text.clone(), l.href.clone(), kind.clone()]
            .into_iter()
            .find(|t| !t.trim().is_empty())
            .unwrap_or_default();
        view! {
            <li class="map-link">
                <button class="map-link-main" title=l.href.clone()
                    on:click=move |_| picked.set(Some(pick.clone()))>
                    <span class="map-link-text">{text}</span>
                    <span class="map-link-to muted">{name_of(&other)}</span>
                </button>
                {l.redirected.then(|| view! { <span class="map-badge" title=ui.t("map-redirected-help")>"301"</span> })}
                {show.map(|(page, node)| view! {
                    <button class="icon-btn icon-btn--small" title=ui.t("map-show-in-editor") aria-label=ui.t("map-show-in-editor")
                        on:click=move |_| open_in_editor(page.clone(), Some(node.clone()))>{icons::external()}</button>
                })}
            </li>
        }
    };
    match p {
        Picked::Edge(from, to) => {
            let links: Vec<MapLink> = edges.with_untracked(|es| {
                es.iter()
                    .find(|e| e.from == from && e.to == to)
                    .map(|e| {
                        e.links
                            .iter()
                            .filter_map(|&i| map.with_untracked(|m| m.links.get(i).cloned()))
                            .collect()
                    })
                    .unwrap_or_default()
            });
            let movable = links.iter().any(MapLink::retargetable);
            let targets: Vec<(String, String)> = map.with_untracked(|m| {
                let mut v: Vec<(String, String)> = m
                    .nodes
                    .iter()
                    .filter(|n| linkable(n))
                    .map(|n| {
                        (
                            n.key.clone(),
                            format!("{} · {}", n.label, n.path.clone().unwrap_or_default()),
                        )
                    })
                    .collect();
                v.sort_by(|a, b| a.1.cmp(&b.1));
                v
            });
            let (f2, t2) = (from.clone(), to.clone());
            view! {
                <header class="map-panel-head">
                    <span class="map-panel-kind">{ui.t("map-edge")}</span>
                    {close}
                </header>
                <div class="map-panel-route">
                    <button class="link-btn" on:click={let f = from.clone(); move |_| picked.set(Some(Picked::Node(f.clone())))}>{name_of(&from)}</button>
                    {icons::arrow_right()}
                    <button class="link-btn" on:click={let t = to.clone(); move |_| picked.set(Some(Picked::Node(t.clone())))}>{name_of(&to)}</button>
                </div>
                {movable.then(|| view! {
                    <label class="field">
                        <span class="label" title=ui.t("map-retarget-help")>{ui.t("map-retarget")}</span>
                        <div class="select">
                            <select class="input" on:change=move |e| {
                                let v = event_target_value(&e);
                                if !v.is_empty() && v != t2 { retarget(f2.clone(), t2.clone(), v) }
                            }>
                                <option value="" selected=true>{ui.t("map-retarget-choose")}</option>
                                {targets.into_iter().map(|(k, l)| view! { <option value=k>{l}</option> }).collect_view()}
                            </select>
                        </div>
                    </label>
                })}
                <h3 class="map-panel-h">{ui.now("map-links-count", &[("count", links.len().to_string())])}</h3>
                <ul class="map-links">
                    {links.into_iter().map(|l| { let (o, f) = (l.from.clone(), l.from.clone()); link_row(l, o, Picked::Node(f)) }).collect_view()}
                </ul>
            }
            .into_any()
        }
        Picked::Node(key) => {
            let Some(n) = map.with_untracked(|m| m.node(&key).cloned()) else {
                return ().into_any();
            };
            let outgoing: Vec<MapLink> =
                map.with_untracked(|m| m.links.iter().filter(|l| l.from == key).cloned().collect());
            let incoming: Vec<MapLink> =
                map.with_untracked(|m| m.links.iter().filter(|l| l.to == key).cloned().collect());
            let kind_label = ui.now(
                match n.kind {
                    NodeKind::Page => "map-kind-page",
                    NodeKind::Collection => "map-kind-collection",
                    NodeKind::Entry => "map-kind-entry",
                    NodeKind::Layout => "map-kind-layout",
                    NodeKind::Component => "map-kind-component",
                    NodeKind::Menu => "map-kind-menu",
                    NodeKind::Missing => "map-kind-missing",
                    NodeKind::External => "map-kind-external",
                    NodeKind::Media => match lm::platform(&key).map(|p| p.2) {
                        Some(PlatformKind::Social) => "map-kind-social",
                        _ => "map-kind-media",
                    },
                },
                &[],
            );
            let settings = match n.kind {
                NodeKind::Page => {
                    let id = key.trim_start_matches("page:").to_string();
                    let page = s.doc.with_untracked(|d| d.as_ref()?.pages.iter().find(|p| p.id == id).cloned());
                    page.map(|pg| {
                        let (id1, id2, id3) = (id.clone(), id.clone(), id.clone());
                        let home = pg.path == "/";
                        let path_err = RwSignal::new(false);
                        let p_open = pg.path.clone();
                        let templ = pg.path.contains("{{");
                        view! {
                            <div class="form-grid map-form">
                                <label class="label" for="map-page-name">{ui.t("map-page-name")}</label>
                                <input id="map-page-name" class="input" prop:value=pg.name.clone().unwrap_or_default()
                                    placeholder=n.label.clone()
                                    on:change=move |e| {
                                        let v = event_target_value(&e).trim().to_string();
                                        s.apply.run(Command::SetPageName { id: id1.clone(), name: (!v.is_empty()).then_some(v) });
                                    }/>
                                <label class="label" for="map-page-path" title=ui.t("map-page-path-help")>{ui.t("map-page-path")}</label>
                                <div class="map-path-field">
                                    <input id="map-page-path" class="input input--mono" spellcheck="false" prop:value=pg.path.clone()
                                        disabled={home || templ} class:is-invalid=move || path_err.get()
                                        on:change=move |e| {
                                            let mut v = event_target_value(&e).trim().to_string();
                                            if !v.starts_with('/') { v.insert(0, '/') }
                                            if !v.ends_with('/') && !v.ends_with(".html") { v.push('/') }
                                            path_err.set(!set_path(s, &id2, &v));
                                        }/>
                                    {move || path_err.get().then(|| view! { <span class="field-error">{ui.t("map-page-path-taken")}</span> })}
                                </div>
                                <span class="label" title=ui.t("map-draft-help")>{ui.t("map-draft")}</span>
                                <label class="switch">
                                    <input type="checkbox" aria-label=ui.t("map-draft") prop:checked=pg.draft disabled=home
                                        on:change=move |e| s.apply.run(Command::SetPageDraft { id: id3.clone(), draft: event_target_checked(&e) })/>
                                    <span class="switch-track"></span>
                                </label>
                            </div>
                            <div class="map-actions">
                                {(!templ).then(|| view! {
                                    <button class="btn btn--primary btn--small" on:click=move |_| open_in_editor(p_open.clone(), None)>{icons::edit()}{ui.t("map-open")}</button>
                                })}
                                <button class="btn btn--small" on:click={let p = pg.path.clone(); move |_| new_page(ui, s, &crate::pageops::folder_of(&format!("{p}x/")))}>{icons::plus()}{ui.t("map-new-child")}</button>
                                <button class="btn btn--small" on:click={let i = id.clone(); move |_| duplicate(ui, s, &i)}>{icons::copy()}{ui.t("pages-duplicate")}</button>
                                <button class="btn btn--small btn--danger-ghost" disabled=home on:click={let i = id.clone(); move |_| { delete(ui, s, &i); }}>{icons::trash()}{ui.t("pages-delete")}</button>
                            </div>
                        }
                        .into_any()
                    })
                }
                NodeKind::Entry => {
                    let p = n.path.clone().unwrap_or_default();
                    let (col, file) = key
                        .trim_start_matches("entry:")
                        .split_once('/')
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                        .unwrap_or_default();
                    let cid = cms_id.with_untracked(|m| m.get(&col).cloned());
                    Some(view! {
                        <p class="muted small path">{crate::ltr(&p)}</p>
                        <div class="map-actions">
                            <button class="btn btn--primary btn--small" on:click=move |_| open_in_editor(p.clone(), None)>{icons::edit()}{ui.t("map-open")}</button>
                            {cid.map(|cid| view! {
                                <button class="btn btn--small" on:click=move |_| { ui.goto_entry.set(Some((cid.clone(), file.clone()))); ui.tab(Tab::Cms); }>{icons::database()}{ui.t("map-open-entry")}</button>
                            })}
                        </div>
                    }
                    .into_any())
                }
                NodeKind::Collection => {
                    let k = key.clone();
                    let opened = open.with_untracked(|o| o.contains(&key));
                    let first = full.with_untracked(|m| {
                        m.nodes
                            .iter()
                            .find(|e| e.parent.as_deref() == Some(key.as_str()))
                            .and_then(|e| e.path.clone())
                    });
                    Some(view! {
                        <p class="muted small path">{crate::ltr(&n.path.clone().unwrap_or_default())}</p>
                        <p class="small">{ui.now("map-entries", &[("count", n.count.unwrap_or(0).to_string())])}</p>
                        <div class="map-actions">
                            {first.map(|f| view! {
                                <button class="btn btn--primary btn--small" title=ui.t("map-open-template-help")
                                    on:click=move |_| open_in_editor(f.clone(), None)>{icons::edit()}{ui.t("map-open-template")}</button>
                            })}
                            <button class="btn btn--small" on:click=move |_| open.update(|o| { if !o.remove(&k) { o.insert(k.clone()); } })>
                                {icons::list()}{ui.t(if opened { "map-entries-hide" } else { "map-entries-show" })}
                            </button>
                            <button class="btn btn--small" on:click=move |_| ui.tab(Tab::Cms)>{icons::database()}{ui.t("map-open-cms")}</button>
                        </div>
                    }
                    .into_any())
                }
                NodeKind::Menu => Some(view! {
                    <div class="map-actions">
                        <button class="btn btn--small" on:click=move |_| ui.tab(Tab::Settings(Section::Menus))>{icons::menu()}{ui.t("map-edit-menus")}</button>
                    </div>
                    <p class="muted small">{ui.t("map-menu-help")}</p>
                }
                .into_any()),
                NodeKind::Component | NodeKind::Layout => Some(view! {
                    <p class="muted small">{ui.now("map-shared-pages", &[("count", n.count.unwrap_or(0).to_string())])}</p>
                    <p class="muted small">{ui.t("map-shared-note")}</p>
                }
                .into_any()),
                NodeKind::Missing => {
                    let p = n.path.clone().unwrap_or_default();
                    let p2 = p.clone();
                    let pages: Vec<(String, String)> = map.with_untracked(|m| {
                        m.nodes
                            .iter()
                            .filter(|x| linkable(x))
                            .map(|x| (x.path.clone().unwrap_or_default(), x.label.clone()))
                            .collect()
                    });
                    let redirected = s.doc.with_untracked(|d| {
                        d.as_ref().is_some_and(|d| {
                            d.redirects.iter().any(|r| r.from.trim_end_matches('/') == p.trim_end_matches('/'))
                        })
                    });
                    Some(view! {
                        <p class="small">{ui.t(if redirected { "map-missing-redirected" } else { "map-missing-help" })}</p>
                        <div class="map-actions">
                            <button class="btn btn--primary btn--small" on:click=move |_| create_at(s, &p)>{icons::plus()}{ui.t("map-create-page")}</button>
                        </div>
                        {(!redirected).then(|| view! {
                            <label class="field">
                                <span class="label" title=ui.t("map-redirect-help")>{ui.t("map-redirect")}</span>
                                <div class="select">
                                    <select class="input" on:change=move |e| {
                                        let to = event_target_value(&e);
                                        if to.is_empty() { return; }
                                        let mut list = s.doc.with_untracked(|d| d.as_ref().map(|d| d.redirects.clone()).unwrap_or_default());
                                        list.push(revel_model::Redirect { from: p2.clone(), to });
                                        s.apply.run(Command::SetRedirects { redirects: list });
                                    }>
                                        <option value="" selected=true>{ui.t("map-retarget-choose")}</option>
                                        {pages.into_iter().map(|(path, label)| view! { <option value=path.clone()>{format!("{label} · {path}")}</option> }).collect_view()}
                                    </select>
                                </div>
                            </label>
                        })}
                    }
                    .into_any())
                }
                NodeKind::External | NodeKind::Media => {
                    let site = (n.kind == NodeKind::External).then(|| n.label.clone());
                    Some(addresses(ui, s, full, key.clone(), site, ext, open_in_editor).into_any())
                }
            };
            // Autre site, plateforme : les adresses tiennent lieu de liens entrants.
            let external = matches!(n.kind, NodeKind::External | NodeKind::Media);
            let n_out = outgoing.len();
            let n_in = incoming.len();
            view! {
                <header class="map-panel-head">
                    <span class="map-panel-kind">{kind_label}</span>
                    {close}
                </header>
                <h2 class="map-panel-title">{n.label.clone()}</h2>
                {settings}
                {(!external).then(|| view! {
                    <h3 class="map-panel-h">{ui.now("map-links-in", &[("count", n_in.to_string())])}</h3>
                    {(n_in == 0).then(|| view! { <p class="muted small">{ui.t(if n.kind == NodeKind::Page { "map-orphan-help" } else { "map-none" })}</p> })}
                    <ul class="map-links">{incoming.into_iter().map(|l| { let o = l.from.clone(); let e = Picked::Edge(l.from.clone(), l.to.clone()); link_row(l, o, e) }).collect_view()}</ul>
                    <h3 class="map-panel-h">{ui.now("map-links-out", &[("count", n_out.to_string())])}</h3>
                    {(n_out == 0).then(|| view! { <p class="muted small">{ui.t("map-none")}</p> })}
                    <ul class="map-links">{outgoing.into_iter().map(|l| { let o = l.to.clone(); let e = Picked::Edge(l.from.clone(), l.to.clone()); link_row(l, o, e) }).collect_view()}</ul>
                })}
            }
            .into_any()
        }
    }
}

/// Sorte d'un lien (vide pour un lien écrit dans une page).
fn via_label(ui: Ui, via: &LinkVia) -> String {
    match via {
        LinkVia::List { .. } => ui.now("map-via-list", &[]),
        LinkVia::Menu { .. } => ui.now("map-via-menu", &[]),
        LinkVia::MenuItem { .. } => ui.now("map-via-menu-item", &[]),
        LinkVia::Redirect { .. } => ui.now("map-via-redirect", &[]),
        LinkVia::Arg { .. } => ui.now("map-via-arg", &[]),
        LinkVia::Src { .. } => ui.now("map-via-src", &[]),
        LinkVia::Field { pointer, .. } => ui.now(
            "map-via-field",
            &[(
                "field",
                pointer
                    .trim_start_matches('/')
                    .split('/')
                    .next()
                    .unwrap_or("")
                    .to_string(),
            )],
        ),
        LinkVia::Body { .. } => ui.now("map-via-body", &[]),
        LinkVia::Info { .. } => ui.now("map-via-info", &[]),
        LinkVia::Attr { .. } => String::new(),
    }
}

/// Page et élément où montrer un lien dans l'éditeur (lien ou média écrit dans une page).
fn shown_at(s: DocSession, via: &LinkVia) -> Option<(String, String)> {
    match via {
        LinkVia::Attr { at } | LinkVia::Arg { at, .. } | LinkVia::Src { at, .. } => match &at.owner
        {
            Owner::Page(pid) => s.doc.with_untracked(|d| {
                d.as_ref()?
                    .pages
                    .iter()
                    .find(|p| p.id == *pid && !p.path.contains("{{"))
                    .map(|p| (p.path.clone(), at.node.clone()))
            }),
            _ => None,
        },
        _ => None,
    }
}

/// Adresses d'un autre site ou d'une plateforme (D-107) : état vérifié, endroits où chacune
/// sert, ouvrir, remplacer partout où elle sert.
fn addresses(
    ui: Ui,
    s: DocSession,
    full: Memo<LinkMap>,
    key: String,
    site: Option<String>,
    ext: ExtTools,
    open_in_editor: impl Fn(String, Option<String>) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    // Liens par adresse, dans l'ordre du plan.
    let groups: Vec<(String, Vec<MapLink>)> = full.with_untracked(|m| {
        let mut g: Vec<(String, Vec<MapLink>)> = Vec::new();
        for l in m.links.iter().filter(|l| l.to == key) {
            match g.iter_mut().find(|(u, _)| *u == l.href) {
                Some((_, v)) => v.push(l.clone()),
                None => g.push((l.href.clone(), vec![l.clone()])),
            }
        }
        g
    });
    // Adresses à revoir d'abord.
    let mut groups = groups;
    ext.checks.with_untracked(|c| {
        groups.sort_by_key(|(u, _)| std::cmp::Reverse(c.get(u).map_or(1, |c| severity(c.state))))
    });
    let urls: Vec<String> = groups.iter().map(|g| g.0.clone()).collect();
    // Nom d'un endroit : nœud du plan, contenu du CMS, information du site.
    let place = move |l: &MapLink| -> String {
        if l.from == "info:site" {
            return ui.now("map-source-info", &[("name", l.text.clone())]);
        }
        if l.from.starts_with("cms:") {
            return ui.now("map-source-cms", &[("name", l.text.clone())]);
        }
        full.with_untracked(|m| m.node(&l.from).map(|n| n.label.clone()))
            .unwrap_or_else(|| l.from.clone())
    };
    let n = urls.len();
    let rows = groups
        .into_iter()
        .map(|(url, links)| {
            let u = StoredValue::new(url.clone());
            let state = move || ext.checks.with(|c| u.with_value(|u| c.get(u).cloned()));
            let chip = move || match state() {
                Some(c) => {
                    let (cls, label) = state_key(c.state);
                    let when = crate::assistants::when((c.checked.max(0) as u64) * 1000, &ui.lang());
                    let detail = match (&c.error, c.status) {
                        (Some(e), _) => format!("{e} · {when}"),
                        (None, Some(st)) => format!("HTTP {st} · {when}"),
                        (None, None) => when,
                    };
                    view! { <span class=format!("map-status map-status--{cls}") title=detail>{ui.now(label, &[])}</span> }.into_any()
                }
                None => view! { <span class="map-status map-status--none">{ui.t("map-url-unchecked")}</span> }.into_any(),
            };
            let (u_fin, u_open, u_rep) = (url.clone(), url.clone(), url.clone());
            let final_row = move || {
                state()
                    .filter(|c| c.state == LinkState::Redirected)
                    .and_then(|c| c.final_url)
                    .map(|f| {
                        let (old, f2) = (u_fin.clone(), f.clone());
                        view! {
                            <div class="map-url-final small">
                                <span class="muted">{ui.t("map-url-final")}</span>
                                <span class="map-url-text" title=f.clone()>{crate::ltr(&f)}</span>
                                <button class="link-btn" on:click=move |_| ext.replace.run((old.clone(), f2.clone()))>{ui.t("map-url-use-final")}</button>
                            </div>
                        }
                    })
            };
            let places = links
                .iter()
                .map(|l| {
                    let name = place(l);
                    let via = via_label(ui, &l.via);
                    let show = shown_at(s, &l.via);
                    view! {
                        <li>
                            <span class="map-url-place" title=name.clone()>{name.clone()}</span>
                            {(!via.is_empty()).then(|| view! { <span class="map-url-via muted">{via}</span> })}
                            {show.map(|(page, node)| view! {
                                <button class="icon-btn icon-btn--small" title=ui.t("map-show-in-editor") aria-label=ui.t("map-show-in-editor")
                                    on:click=move |_| open_in_editor(page.clone(), Some(node.clone()))>{icons::edit()}</button>
                            })}
                        </li>
                    }
                })
                .collect_view();
            let uses = links.len();
            view! {
                <li class="map-url" data-url=url.clone()>
                    <div class="map-url-head">
                        {chip}
                        <span class="map-url-text" title=url.clone()>{crate::ltr(&short_url(&url))}</span>
                        <button class="icon-btn icon-btn--small" title=ui.t("map-url-open") aria-label=ui.t("map-url-open")
                            on:click=move |_| { let u = u_open.clone(); spawn_local(async move { let _ = api::open_external(&u).await; }); }>
                            {icons::external()}
                        </button>
                        <button class="icon-btn icon-btn--small map-url-replace" title=ui.t("map-url-replace-help") aria-label=ui.t("map-url-replace")
                            on:click=move |_| {
                                let old = u_rep.clone();
                                ui.ask.set(Some(Ask::input(
                                    ui.now("map-url-replace-title", &[]),
                                    ui.now("map-url-new", &[]),
                                    old.clone(),
                                    ui.now("map-url-replace", &[]),
                                    move |v| ext.replace.run((old.clone(), v)),
                                )));
                            }>
                            {icons::edit()}
                        </button>
                    </div>
                    {final_row}
                    <div class="map-url-uses muted small">{ui.now("map-url-uses", &[("count", uses.to_string())])}</div>
                    <ul class="map-url-places small">{places}</ul>
                </li>
            }
        })
        .collect_view();
    view! {
        <div class="map-actions">
            <button class="btn btn--small map-urls-check" disabled=move || ext.checking.get()
                title=ui.t("map-urls-check-help")
                on:click=move |_| ext.run_check.run((urls.clone(), "all"))>
                {icons::reset()}{ui.t("map-urls-check")}
            </button>
            {site.map(|host| view! {
                <button class="btn btn--small" on:click=move |_| {
                    let url = format!("https://{host}");
                    spawn_local(async move { let _ = api::open_external(&url).await; });
                }>{icons::external()}{ui.t("map-open-site")}</button>
            })}
        </div>
        <h3 class="map-panel-h">{ui.now("map-urls", &[("count", n.to_string())])}</h3>
        <ul class="map-urls">{rows}</ul>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(key: &str, kind: NodeKind, path: Option<&str>) -> MapNode {
        MapNode {
            key: key.into(),
            kind,
            label: key.into(),
            path: path.map(Into::into),
            draft: false,
            parent: None,
            count: None,
        }
    }

    #[test]
    fn rangement_par_adresses() {
        let map = LinkMap {
            nodes: vec![
                node("page:index", NodeKind::Page, Some("/")),
                node("page:agence", NodeKind::Page, Some("/agence/")),
                node("page:equipe", NodeKind::Page, Some("/agence/equipe/")),
                node("page:projets", NodeKind::Page, Some("/projets/")),
                node(
                    "col:projets",
                    NodeKind::Collection,
                    Some("/project/{{ slug }}/"),
                ),
                node("menu:principal", NodeKind::Menu, None),
                node("missing:/x/", NodeKind::Missing, Some("/x/")),
            ],
            links: vec![MapLink {
                from: "page:projets".into(),
                to: "col:projets".into(),
                href: String::new(),
                text: String::new(),
                via: LinkVia::List {
                    at: revel_edit::Target {
                        owner: Owner::Page("projets".into()),
                        node: "e".into(),
                    },
                },
                redirected: false,
            }],
        };
        let p = arrange(&map);
        let x = |k: &str| p[k].0;
        // Menus à gauche ; accueil, puis ses pages, puis leurs pages.
        assert!(x("menu:principal") < x("page:index"));
        assert!(x("page:index") < x("page:agence"));
        assert!(x("page:agence") < x("page:equipe"));
        // La collection sous la page qui la liste.
        assert_eq!(x("col:projets"), x("page:equipe"));
        // Adresses sans page à droite de tout.
        assert!(
            map.nodes
                .iter()
                .all(|n| n.key == "missing:/x/" || x(&n.key) < x("missing:/x/"))
        );
        // Pas de chevauchement dans une colonne.
        let mut cols: HashMap<i64, Vec<f64>> = HashMap::new();
        for (k, (x, y)) in &p {
            let _ = k;
            cols.entry(*x as i64).or_default().push(*y);
        }
        for ys in cols.values_mut() {
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for w in ys.windows(2) {
                assert!(w[1] - w[0] >= ENTRY_H, "{ys:?}");
            }
        }
    }

    #[test]
    fn contenus_sous_la_collection() {
        let mut m = LinkMap::default();
        m.nodes.push(node("page:index", NodeKind::Page, Some("/")));
        m.nodes
            .push(node("col:p", NodeKind::Collection, Some("/p/{{ slug }}/")));
        for f in ["a", "b"] {
            let mut e = node(&format!("entry:p/{f}"), NodeKind::Entry, Some("/p/x/"));
            e.parent = Some("col:p".into());
            m.nodes.push(e);
        }
        let p = arrange(&m);
        let (cx, cy) = p["col:p"];
        let (ax, ay) = p["entry:p/a"];
        let (bx, by) = p["entry:p/b"];
        assert_eq!(ax, cx);
        assert!(ay >= cy + COLLECTION_H, "{ay} sous {cy}");
        assert_eq!(bx, ax);
        assert!(ay >= cy + NODE_H && by >= ay + ENTRY_H, "{cy} {ay} {by}");
    }

    #[test]
    fn contenus_replies() {
        let mut m = LinkMap::default();
        m.nodes
            .push(node("col:p", NodeKind::Collection, Some("/p/{{ slug }}/")));
        let mut e = node("entry:p/a", NodeKind::Entry, Some("/p/a/"));
        e.parent = Some("col:p".into());
        m.nodes.push(e);
        m.nodes.push(node("page:index", NodeKind::Page, Some("/")));
        m.links.push(MapLink {
            from: "page:index".into(),
            to: "entry:p/a".into(),
            href: "/p/a/".into(),
            text: "A".into(),
            via: LinkVia::Redirect { index: 0 },
            redirected: false,
        });
        let c = collapse(m.clone(), &HashSet::new());
        assert!(c.node("entry:p/a").is_none());
        assert_eq!(c.links[0].to, "col:p");
        let o = collapse(m, &HashSet::from(["col:p".to_string()]));
        assert!(o.node("entry:p/a").is_some());
    }
}
