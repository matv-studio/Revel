//! Calques (étape 2.4) : arbre de la page affichée — gabarits emboîtés, page, composants utilisés,
//! blocs logiques — avec sélection synchronisée avec le canevas, glisser-déposer, dupliquer,
//! envelopper, supprimer, monter et descendre.
//!
//! L'arbre montre ce que l'on voit sur la page, comme Webflow et Framer : pas de `html`, `head`,
//! `meta`, scripts ni doctype ; la page (`body`) à la racine ; le texte d'un élément affiché sur sa
//! ligne plutôt qu'en ligne à part ; des noms simples (classe rendue lisible ou nature de
//! l'élément), la balise en info-bulle.

use crate::editor::{Ed, Selection};
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::{Branch, Command, Owner, Parent, Target};
use revel_model::{Node, Site};
use std::collections::HashSet;

/// Nature d'une ligne de l'arbre.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Element {
        tag: String,
        class: String,
    },
    Text(String),
    Expr(String),
    If(String),
    Each {
        item: String,
        expr: String,
    },
    /// Branche « sinon » d'une condition, « si vide » d'une boucle.
    Branch(Branch),
    Use(String),
    Image(String),
    Html,
    /// Emplacement du contenu dans un gabarit (contenu Markdown pour un contenu de collection).
    Slot {
        markdown: bool,
    },
}

/// Ligne de l'arbre des calques.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Clé unique (chemin dans l'arbre affiché).
    pub key: String,
    /// Clés des lignes parentes.
    pub ancestors: Vec<String>,
    pub depth: usize,
    pub id: Option<String>,
    pub owner: Owner,
    /// Liste qui contient le nœud, et sa position.
    pub parent: Parent,
    pub index: usize,
    pub kind: Kind,
    /// A des lignes enfants.
    pub has_children: bool,
    /// Peut recevoir des enfants (glisser-déposer « dans ») : branche et nombre d'enfants.
    pub inside: Option<(Branch, usize)>,
    /// Texte de l'élément, montré sur sa ligne (texte et expressions qu'il contient).
    pub preview: Option<String>,
    /// Élément animé (panneau Interactions).
    pub animated: bool,
    /// Texte venu des données du site ou d'un contenu (liaison, comme Webflow).
    pub bound: bool,
    /// Affiché sous condition (`{% if %}` sans ligne propre) : la condition.
    pub condition: Option<String>,
    /// Élément qui ne contient que du texte (icône de texte, double-clic pour le modifier).
    pub textual: bool,
    /// Nom donné par l'utilisateur (renommer un calque).
    pub name: Option<String>,
    /// Calque masqué par l'utilisateur.
    pub hidden: bool,
    /// Calque dans un calque masqué (grisé aussi).
    pub dimmed: bool,
}

/// Affichage d'une ligne dans l'arbre simplifié.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Vis {
    Show,
    /// Montrée sans ses descendants (icône SVG).
    Leaf,
    /// Pas de ligne, enfants remontés d'un niveau (`html`, contenu de la page seul dans la page).
    Through,
    /// Masquée avec ses descendants (`head`, scripts, doctype…).
    Hide,
    /// Texte fondu dans la ligne de l'élément qui le contient.
    Merge,
}

/// Boucle ou condition qui n'affecte que des variables (au moins une) : rien d'affiché. Une
/// boucle vide (créée à remplir) reste visible.
fn data_only(a: &[Node], b: &[Node]) -> bool {
    only_sets(a) && only_sets(b) && a.iter().chain(b).any(|n| !matches!(n, Node::Text { .. }))
}

/// Contenu fait seulement d'affectations de variables (et de blancs), sans rien d'affiché.
fn only_sets(nodes: &[Node]) -> bool {
    nodes.iter().all(|n| match n {
        Node::Set { .. } | Node::SetBlock { .. } => true,
        Node::Text { text } => text.trim().is_empty(),
        Node::Each {
            children, empty, ..
        } => only_sets(children) && only_sets(empty),
        Node::If {
            then, otherwise, ..
        } => only_sets(then) && only_sets(otherwise),
        _ => false,
    })
}

/// Éléments réservés aux lecteurs d'écran ou au clavier (lien d'évitement, texte masqué) :
/// invisibles sur la page, absents des calques.
fn a11y_only(attrs: &[revel_model::Attr]) -> bool {
    const CLASSES: &[&str] = &[
        "skip-link",
        "skiplink",
        "skip-to-content",
        "sr-only",
        "visually-hidden",
        "visuallyhidden",
        "screen-reader-text",
    ];
    attrs
        .iter()
        .find(|a| a.name == "class")
        .and_then(|a| a.value.as_deref())
        .is_some_and(|v| v.split_whitespace().any(|c| CLASSES.contains(&c)))
}

/// Éléments sans rendu visible (ou techniques) : jamais dans les calques.
const HIDDEN: &[&str] = &[
    "head", "script", "style", "link", "meta", "noscript", "template", "base", "title", "br", "wbr",
];

fn class_of(attrs: &[revel_model::Attr]) -> String {
    attrs
        .iter()
        .find(|a| a.name == "class")
        .and_then(|a| a.value.as_deref())
        .filter(|v| !v.contains("{{") && !v.contains("{%"))
        .map(|v| v.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

fn short(s: &str, n: usize) -> String {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > n {
        format!("{}…", t.chars().take(n).collect::<String>())
    } else {
        t
    }
}

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Affectation de variable (`{% set %}`) de la page affichée : listée dans le panneau Variables,
/// pas dans les calques.
#[derive(Clone, Debug, PartialEq)]
pub struct VarRow {
    pub owner: Owner,
    pub parent: Parent,
    pub index: usize,
    pub name: String,
    /// Expression (`None` : bloc `{% set %}…{% endset %}`, contenu HTML capturé).
    pub expr: Option<String>,
    /// Dans une boucle ou une condition (libellé du bloc le plus proche).
    pub within: Option<Kind>,
}

struct Builder<'a> {
    doc: &'a Site,
    /// Affectations rencontrées (une fois chacune, même dans un composant utilisé plusieurs fois).
    vars: Vec<VarRow>,
    /// Bloc logique le plus proche (boucle, condition) du parcours en cours.
    within: Vec<Kind>,
    /// Corps à insérer à la place de chaque `Slot`, du gabarit extérieur vers la page.
    inner: Vec<(Owner, &'a [Node])>,
    markdown: bool,
    rows: Vec<Row>,
    /// Affichage de chaque ligne (même ordre que `rows`).
    vis: Vec<Vis>,
    /// Composants en cours de parcours (garde contre un composant qui s'utilise lui-même).
    stack: Vec<String>,
}

impl<'a> Builder<'a> {
    fn list(
        &mut self,
        nodes: &'a [Node],
        owner: &Owner,
        parent: Parent,
        key: &str,
        ancestors: &[String],
        slot_level: usize,
    ) {
        for (i, n) in nodes.iter().enumerate() {
            if matches!(n, Node::Text { text } if text.trim().is_empty())
                || matches!(n, Node::Html { html } if html.trim().is_empty())
            {
                continue;
            }
            if let Node::Set { name, .. } | Node::SetBlock { name, .. } = n {
                if !self.vars.iter().any(|v| v.parent == parent && v.index == i) {
                    self.vars.push(VarRow {
                        owner: owner.clone(),
                        parent: parent.clone(),
                        index: i,
                        name: name.clone(),
                        expr: match n {
                            Node::Set { expr, .. } => Some(expr.clone()),
                            _ => None,
                        },
                        within: self.within.last().cloned(),
                    });
                }
                continue;
            }
            let k = format!("{key}/{i}");
            let depth = ancestors.len();
            let id = revel_model::ids::node_id(n).map(str::to_string);
            let kind = match n {
                Node::Element { tag, attrs, .. } => Kind::Element {
                    tag: tag.clone(),
                    class: class_of(attrs),
                },
                Node::Text { text } => Kind::Text(short(text, 40)),
                Node::Out { expr } => Kind::Expr(short(expr, 40)),
                Node::Html { .. } => Kind::Html,
                Node::If { cond, .. } => Kind::If(short(cond, 40)),
                Node::Each { item, expr, .. } => Kind::Each {
                    item: item.clone(),
                    expr: short(expr, 32),
                },
                Node::Set { .. } | Node::SetBlock { .. } => unreachable!(),
                Node::Slot => Kind::Slot {
                    markdown: self.markdown && slot_level + 1 >= self.inner.len(),
                },
                // Nom donné, sinon fin de l'identifiant rendue lisible (`card-project` -> « Card
                // project »).
                Node::Use { component, .. } => Kind::Use(
                    self.doc
                        .component(component)
                        .and_then(|c| c.name.clone().filter(|n| !n.trim().is_empty()))
                        .unwrap_or_else(|| {
                            crate::names::humanize(
                                component.rsplit('/').next().unwrap_or(component),
                            )
                        }),
                ),
                Node::Image(img) => Kind::Image(short(&img.src, 32)),
            };
            let inside = match n {
                Node::Element { tag, children, .. }
                    if !VOID.contains(&tag.to_ascii_lowercase().as_str()) =>
                {
                    Some((Branch::Children, children.len()))
                }
                Node::Each { children, .. } => Some((Branch::Children, children.len())),
                Node::If { then, .. } => Some((Branch::Then, then.len())),
                // Composant à emplacement : son contenu propre (D-118).
                Node::Use {
                    component,
                    children,
                    ..
                } if self
                    .doc
                    .component(component)
                    .is_some_and(|c| revel_edit::make::slot_of(c).is_some()) =>
                {
                    Some((Branch::Children, children.len()))
                }
                _ => None,
            };
            // Texte dans un élément, éventuellement à travers des conditions sans ligne.
            let in_element = ancestors
                .iter()
                .rev()
                .filter_map(|a| self.rows.iter().rposition(|r| &r.key == a))
                .find(|&p| self.vis[p] != Vis::Through || !matches!(self.rows[p].kind, Kind::If(_)))
                .is_some_and(|p| matches!(self.rows[p].kind, Kind::Element { .. }));
            let vis = match n {
                Node::Element { tag, attrs, .. } => {
                    let t = tag.to_ascii_lowercase();
                    if t == "html" {
                        Vis::Through
                    } else if HIDDEN.contains(&t.as_str()) || a11y_only(attrs) {
                        Vis::Hide
                    } else if t == "svg" {
                        Vis::Leaf
                    } else {
                        Vis::Show
                    }
                }
                // Boucle ou condition qui ne fait que préparer des données (`{% set %}` seuls) :
                // rien à afficher.
                Node::Each {
                    children, empty, ..
                } if data_only(children, empty) => Vis::Hide,
                Node::If {
                    then, otherwise, ..
                } if data_only(then, otherwise) => Vis::Hide,
                // Condition sans « sinon » : pas de ligne, ses éléments marqués (visibilité
                // conditionnelle, comme Webflow).
                Node::If { otherwise, .. } if otherwise.is_empty() => Vis::Through,
                Node::Html { html } if html.trim_start().starts_with("<!") => Vis::Hide,
                Node::Text { .. } | Node::Out { .. } if in_element => Vis::Merge,
                _ => Vis::Show,
            };
            let at = self.rows.len();
            self.rows.push(Row {
                key: k.clone(),
                ancestors: ancestors.to_vec(),
                depth,
                id: id.clone(),
                owner: owner.clone(),
                parent: parent.clone(),
                index: i,
                kind,
                has_children: false,
                inside: inside.filter(|_| id.is_some()),
                preview: None,
                animated: matches!(n, Node::Element { attrs, .. } if attrs.iter().any(|a| a.name.starts_with("data-revel-")
                    && crate::interactions::FAMILIES.iter().any(|f| a.name == f.attr()))),
                bound: false,
                condition: None,
                textual: false,
                name: revel_model::layer_name(n).map(str::to_string),
                hidden: revel_model::is_hidden(n),
                dimmed: false,
            });
            self.vis.push(vis);
            // Image : nom du fichier sur sa ligne ; adresse venue des données : ligne liée.
            if let Node::Image(img) = n {
                match image_file(&img.src) {
                    Some(f) => self.rows[at].preview = Some(f),
                    None => self.rows[at].bound = true,
                }
            }
            let mut anc = ancestors.to_vec();
            anc.push(k.clone());
            let before = self.rows.len();
            let of = |b: Branch| Parent {
                owner: owner.clone(),
                node: id.clone(),
                branch: b,
            };
            let logic = matches!(n, Node::If { .. } | Node::Each { .. });
            if logic {
                self.within.push(self.rows[at].kind.clone());
            }
            match n {
                Node::Element { children, .. } => {
                    self.list(children, owner, of(Branch::Children), &k, &anc, slot_level);
                }
                Node::If {
                    then, otherwise, ..
                } => {
                    self.list(then, owner, of(Branch::Then), &k, &anc, slot_level);
                    if !otherwise.is_empty() {
                        self.branch(
                            Branch::Otherwise,
                            otherwise,
                            owner,
                            &id,
                            &k,
                            &anc,
                            slot_level,
                        );
                    }
                }
                Node::Each {
                    children, empty, ..
                } => {
                    self.list(children, owner, of(Branch::Children), &k, &anc, slot_level);
                    if !empty.is_empty() {
                        self.branch(Branch::Empty, empty, owner, &id, &k, &anc, slot_level);
                    }
                }
                // Utilisation de composant : une ligne, sans son contenu (modifié dans le canevas du
                // composant, ouvert par un double-clic sur la ligne) ; ses variables restent relevées
                // pour le panneau Variables.
                Node::Use {
                    component,
                    children,
                    ..
                } => {
                    if let Some(c) = self.doc.component(component)
                        && !self.stack.contains(component)
                    {
                        self.stack.push(component.clone());
                        let rows = std::mem::take(&mut self.rows);
                        let vis = std::mem::take(&mut self.vis);
                        let o = Owner::Component(c.id.clone());
                        self.list(&c.body, &o, Parent::root(o.clone()), &k, &anc, slot_level);
                        self.rows = rows;
                        self.vis = vis;
                        self.stack.pop();
                    }
                    // Contenu propre de l'utilisation (emplacement du composant, D-118) : calques
                    // de la page, sous l'utilisation.
                    self.list(children, owner, of(Branch::Children), &k, &anc, slot_level);
                }
                Node::Slot => {
                    if let Some((o, body)) = self.inner.get(slot_level + 1).cloned() {
                        self.list(body, &o, Parent::root(o.clone()), &k, &anc, slot_level + 1);
                    }
                }
                _ => {}
            }
            if logic {
                self.within.pop();
            }
            self.rows[at].has_children = self.rows.len() > before;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn branch(
        &mut self,
        b: Branch,
        nodes: &'a [Node],
        owner: &Owner,
        id: &Option<String>,
        key: &str,
        ancestors: &[String],
        slot_level: usize,
    ) {
        let k = format!("{key}/{b:?}");
        let parent = Parent {
            owner: owner.clone(),
            node: id.clone(),
            branch: b,
        };
        let at = self.rows.len();
        self.rows.push(Row {
            key: k.clone(),
            ancestors: ancestors.to_vec(),
            depth: ancestors.len(),
            id: None,
            owner: owner.clone(),
            parent: parent.clone(),
            index: 0,
            kind: Kind::Branch(b),
            has_children: false,
            inside: id.as_ref().map(|_| (b, nodes.len())),
            preview: None,
            animated: false,
            bound: false,
            condition: None,
            textual: false,
            name: None,
            hidden: false,
            dimmed: false,
        });
        self.vis.push(Vis::Show);
        let mut anc = ancestors.to_vec();
        anc.push(k.clone());
        let before = self.rows.len();
        self.list(nodes, owner, parent, &k, &anc, slot_level);
        self.rows[at].has_children = self.rows.len() > before;
    }
}

/// Toutes les lignes de l'arbre de la page (gabarits emboîtés, page, composants).
pub fn rows(doc: &Site, page: &revel_protocol::EditorPageInfo) -> Vec<Row> {
    tree(doc, page).0
}

/// Lignes d'un composant seul (canevas des composants).
pub fn component_rows(doc: &Site, id: &str) -> Vec<Row> {
    let Some(c) = doc.component(id) else {
        return Vec::new();
    };
    let owner = Owner::Component(c.id.clone());
    let mut b = Builder {
        doc,
        vars: Vec::new(),
        within: Vec::new(),
        inner: Vec::new(),
        markdown: false,
        rows: Vec::new(),
        vis: Vec::new(),
        stack: vec![c.id.clone()],
    };
    b.list(&c.body, &owner, Parent::root(owner.clone()), "", &[], 0);
    simplify(b.rows, b.vis)
}

/// Affectations de variables de la page (gabarits emboîtés, page, composants), dans l'ordre.
pub fn variables(doc: &Site, page: &revel_protocol::EditorPageInfo) -> Vec<VarRow> {
    tree(doc, page).1
}

fn tree(doc: &Site, page: &revel_protocol::EditorPageInfo) -> (Vec<Row>, Vec<VarRow>) {
    let mut chain: Vec<&revel_model::Layout> = Vec::new();
    let mut cur = page.layout.clone();
    while let Some(id) = cur {
        match doc.layout(&id) {
            Some(l) if !chain.iter().any(|c| c.id == l.id) => {
                chain.push(l);
                cur = l.parent.clone();
            }
            _ => break,
        }
    }
    // Du gabarit le plus extérieur vers la page.
    let mut inner: Vec<(Owner, &[Node])> = chain
        .iter()
        .rev()
        .map(|l| (Owner::Layout(l.id.clone()), l.body.as_slice()))
        .collect();
    let fixed = page
        .page
        .as_ref()
        .and_then(|id| doc.pages.iter().find(|p| &p.id == id));
    if let Some(p) = fixed {
        inner.push((Owner::Page(p.id.clone()), p.body.as_slice()));
    }
    let mut b = Builder {
        doc,
        vars: Vec::new(),
        within: Vec::new(),
        inner: inner.clone(),
        markdown: fixed.is_none(),
        rows: Vec::new(),
        vis: Vec::new(),
        stack: Vec::new(),
    };
    if let Some((o, body)) = inner.first().cloned() {
        b.list(body, &o, Parent::root(o.clone()), "", &[], 0);
    }
    (simplify(b.rows, b.vis), b.vars)
}

/// Arbre affiché : lignes masquées retirées, textes fondus dans leur élément, niveaux recalculés.
fn simplify(mut rows: Vec<Row>, mut vis: Vec<Vis>) -> Vec<Row> {
    use std::collections::HashMap;
    let pos: HashMap<String, usize> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| (r.key.clone(), i))
        .collect();
    let parent_of =
        |rows: &[Row], i: usize| rows[i].ancestors.last().and_then(|a| pos.get(a).copied());
    // Descendants d'une ligne masquée ou d'une icône : masqués (parcours dans l'ordre de l'arbre).
    for i in 0..rows.len() {
        if let Some(p) = parent_of(&rows, i)
            && matches!(vis[p], Vis::Hide | Vis::Leaf)
        {
            vis[i] = Vis::Hide;
        }
    }
    // Textes fondus : aperçu sur la ligne de l'élément.
    for i in 0..rows.len() {
        if vis[i] != Vis::Merge {
            continue;
        }
        let p = rows[i]
            .ancestors
            .iter()
            .rev()
            .filter_map(|a| pos.get(a).copied())
            .find(|&p| matches!(rows[p].kind, Kind::Element { .. }));
        let Some(p) = p else {
            continue;
        };
        // Texte écrit : sur la ligne ; expression : la ligne est marquée liée (son texte vient du
        // canevas, sans accolades).
        let part = match &rows[i].kind {
            Kind::Text(t) => t.clone(),
            Kind::Expr(_) => {
                rows[p].bound = true;
                continue;
            }
            _ => continue,
        };
        let prev = rows[p].preview.take().unwrap_or_default();
        rows[p].preview = Some(short(&format!("{prev} {part}"), 48));
    }
    let shown = |v: Vis| matches!(v, Vis::Show | Vis::Leaf);
    // Parent affiché le plus proche.
    let shown_parent = |rows: &[Row], vis: &[Vis], i: usize| {
        rows[i]
            .ancestors
            .iter()
            .rev()
            .filter_map(|a| pos.get(a).copied())
            .find(|&p| shown(vis[p]))
    };
    // Contenu de la page seul dans la page (`body` réduit à l'emplacement, site vierge) : ses
    // lignes directement sous la page.
    for i in 0..rows.len() {
        if !matches!(rows[i].kind, Kind::Slot { .. }) || !shown(vis[i]) {
            continue;
        }
        let p = shown_parent(&rows, &vis, i);
        let body = p.is_none_or(|p| {
            matches!(&rows[p].kind, Kind::Element { tag, .. } if tag.eq_ignore_ascii_case("body"))
        });
        let alone = (0..rows.len())
            .filter(|&j| j != i && shown(vis[j]) && shown_parent(&rows, &vis, j) == p)
            .count()
            == 0;
        if body && alone {
            vis[i] = Vis::Through;
        }
    }
    // Éléments d'une condition sans ligne : la condition les accompagne.
    for i in 0..rows.len() {
        if vis[i] != Vis::Through {
            continue;
        }
        let Kind::If(c) = rows[i].kind.clone() else {
            continue;
        };
        let key = rows[i].key.clone();
        for r in rows.iter_mut().skip(i + 1) {
            if r.ancestors.last() == Some(&key) && r.condition.is_none() {
                r.condition = Some(c.clone());
            }
        }
    }
    let keep: Vec<bool> = vis.iter().map(|v| shown(*v)).collect();
    let mut out: Vec<Row> = Vec::new();
    for (i, mut r) in rows.into_iter().enumerate() {
        if !keep[i] {
            continue;
        }
        r.ancestors.retain(|a| pos.get(a).is_some_and(|&p| keep[p]));
        r.depth = r.ancestors.len();
        r.has_children = false;
        if vis[i] == Vis::Leaf {
            r.inside = None;
        }
        out.push(r);
    }
    // Calques dans un calque masqué : grisés.
    let hidden_keys: HashSet<String> = out
        .iter()
        .filter(|r| r.hidden)
        .map(|r| r.key.clone())
        .collect();
    for r in &mut out {
        r.dimmed = r.hidden || r.ancestors.iter().any(|a| hidden_keys.contains(a));
    }
    let parents: HashSet<String> = out
        .iter()
        .filter_map(|r| r.ancestors.last().cloned())
        .collect();
    for r in &mut out {
        r.has_children = parents.contains(&r.key);
        r.textual = !r.has_children
            && (r.preview.is_some() || r.bound)
            && matches!(r.kind, Kind::Element { .. });
    }
    out
}

/// Commande de déplacement d'une ligne vers une autre (`zone` : avant, après, dans).
pub fn move_command(src: &Row, dst: &Row, zone: Zone) -> Option<Command> {
    let id = src.id.clone()?;
    if dst.key == src.key || dst.ancestors.contains(&src.key) {
        return None;
    }
    let (to, mut index) = match zone {
        Zone::Before => (dst.parent.clone(), dst.index),
        Zone::After => (dst.parent.clone(), dst.index + 1),
        Zone::Inside => {
            let (branch, n) = dst.inside?;
            let to = match &dst.id {
                Some(i) => Parent {
                    owner: dst.owner.clone(),
                    node: Some(i.clone()),
                    branch,
                },
                // Ligne « sinon » / « si vide » : sa liste.
                None => dst.parent.clone(),
            };
            (to, n)
        }
    };
    // Seulement à l'intérieur d'une même page, d'un même gabarit ou d'un même composant.
    if to.owner != src.owner {
        return None;
    }
    if to == src.parent && src.index < index {
        index -= 1;
    }
    if to == src.parent && index == src.index {
        return None;
    }
    Some(Command::Move {
        at: Target {
            owner: src.owner.clone(),
            node: id,
        },
        to,
        index,
    })
}

/// Monter (`-1`) ou descendre (`+1`) parmi les frères visibles.
pub fn step_command(all: &[Row], row: &Row, dir: i32) -> Option<Command> {
    let siblings: Vec<&Row> = all
        .iter()
        .filter(|r| r.parent == row.parent && r.ancestors == row.ancestors)
        .collect();
    let pos = siblings.iter().position(|r| r.key == row.key)?;
    let other = if dir < 0 {
        siblings.get(pos.checked_sub(1)?)?
    } else {
        siblings.get(pos + 1)?
    };
    move_command(row, other, if dir < 0 { Zone::Before } else { Zone::After })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Before,
    After,
    Inside,
}

/// Nom du fichier d'une image écrite (`/media/equipe.jpg` -> `equipe.jpg`) ; `None` : adresse
/// venue d'une expression.
fn image_file(src: &str) -> Option<String> {
    let s = src.trim();
    let literal = s.starts_with('/') || s.contains("://") || s.starts_with("data:");
    (literal && !s.contains("{{"))
        .then(|| {
            s.rsplit('/')
                .next()
                .unwrap_or(s)
                .split('?')
                .next()
                .unwrap_or_default()
                .to_string()
        })
        .filter(|f| !f.is_empty())
}

/// Nom lisible d'une condition : dernier mot de la première donnée testée (`env.staging` ->
/// « Staging », `not page.draft` -> « Draft »).
fn condition_name(cond: &str) -> String {
    let word = cond
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
        .find(|w| {
            !w.is_empty()
                && !matches!(*w, "not" | "and" | "or" | "in" | "is" | "true" | "false")
                && !w.chars().next().is_some_and(|c| c.is_ascii_digit())
        })
        .unwrap_or(cond);
    crate::names::humanize(word.rsplit('.').next().unwrap_or(word))
}

/// Nom d'une liste parcourue : dernier mot de sa source, lisible (`home.featured` -> « Featured »,
/// `collections.blog | reverse` -> « Blog »).
fn list_name(expr: &str) -> String {
    let src = expr.split('|').next().unwrap_or_default().trim();
    let last = src
        .rsplit('.')
        .next()
        .unwrap_or(src)
        .trim_matches(|c: char| !c.is_alphanumeric());
    crate::names::humanize(last)
}

/// Nom d'une liste de collection : libellé de la collection (« Actualités »), sinon le nom
/// tiré de l'expression.
fn list_label(expr: &str) -> String {
    // Expression raccourcie pour l'affichage : collection lue au début (`collections.<id>`).
    let id: Option<String> = expr.find("collections.").map(|i| {
        expr[i + 12..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect()
    });
    id.filter(|id| !id.is_empty())
        .and_then(|id| {
            use_context::<Ed>()?
                .col_labels
                .with(|m| m.get(&id).cloned())
        })
        .unwrap_or_else(|| list_name(expr))
}

pub fn label(ui: Ui, k: &Kind) -> String {
    match k {
        Kind::Element { tag, class } => crate::names::element_name(ui, tag, class),
        Kind::Text(t) => format!("« {t} »"),
        Kind::Expr(e) => format!("{{{{ {e} }}}}"),
        Kind::If(c) => ui.tx("layers-if", &[("cond", condition_name(c))]),
        Kind::Each { expr, .. } => ui.tx("layers-each", &[("list", list_label(expr))]),
        Kind::Branch(Branch::Otherwise) => ui.tx("layers-else", &[]),
        Kind::Branch(_) => ui.tx("layers-empty-list", &[]),
        Kind::Use(c) => c.rsplit('/').next().unwrap_or(c).to_string(),
        Kind::Image(_) => ui.tx("layers-image", &[]),
        Kind::Html => ui.tx("layers-html", &[]),
        Kind::Slot { markdown: true } => ui.tx("layers-markdown", &[]),
        Kind::Slot { markdown: false } => ui.tx("layers-slot", &[]),
    }
}

/// Utilisation de composant la plus proche qui contient la ligne (identifiant), pour retrouver le
/// texte rendu dans cette utilisation.
fn use_of(rows: &[Row], r: &Row) -> Option<String> {
    r.ancestors.iter().rev().find_map(|a| {
        rows.iter()
            .find(|x| &x.key == a && matches!(x.kind, Kind::Use(_)))
            .and_then(|x| x.id.clone())
    })
}

/// Info-bulle d'une ligne : nature et nom technique d'un élément.
fn tooltip(ui: Ui, k: &Kind) -> String {
    match k {
        Kind::Element { tag, class } => crate::names::tooltip(ui, tag, class),
        Kind::Each { item, expr } => ui.tx(
            "layers-each-help",
            &[("item", item.clone()), ("list", expr.clone())],
        ),
        Kind::If(c) => ui.tx("layers-if-help", &[("cond", c.clone())]),
        Kind::Image(src) => src.clone(),
        _ => label(ui, k),
    }
}

fn icon(k: &Kind, textual: bool, has_children: bool) -> AnyView {
    match k {
        // Bloc générique qui ne contient que du texte : icône de texte (comme Webflow).
        Kind::Element { tag, .. }
            if textual && matches!(tag.to_ascii_lowercase().as_str(), "div" | "span") =>
        {
            icons::text_left()
        }
        // Élément de texte en ligne qui contient d'autres éléments (image, blocs) : une boîte,
        // pas un texte (icône « T » trompeuse).
        Kind::Element { tag, .. }
            if !textual && has_children && tag.eq_ignore_ascii_case("span") =>
        {
            icons::box_()
        }
        Kind::Element { tag, .. } => crate::names::icon(tag),
        Kind::Text(_) => icons::text(),
        Kind::Expr(_) => icons::braces(),
        Kind::If(_) | Kind::Branch(_) => icons::branch(),
        Kind::Each { .. } => icons::repeat(),
        Kind::Use(_) => icons::component(),
        Kind::Image(_) => icons::image(),
        Kind::Html => icons::code(),
        Kind::Slot { .. } => icons::slot(),
    }
}

/// Le calque peut être renommé ou masqué (élément, utilisation de composant, image).
fn nameable(r: &Row) -> bool {
    r.id.is_some() && matches!(r.kind, Kind::Element { .. } | Kind::Use(_) | Kind::Image(_))
}

/// Nom affiché d'une ligne : nom donné, sinon nom automatique.
fn shown_name(ui: Ui, r: &Row) -> String {
    r.name.clone().unwrap_or_else(|| label(ui, &r.kind))
}

/// Donne un nom au calque `at` (vide : nom automatique).
pub fn rename_layer(ui: Ui, ed: Ed, at: Target, name: &str) {
    let cmd = ed.doc.with_untracked(|d| {
        d.as_ref()
            .and_then(|d| revel_edit::layer::rename(d, &at, name).ok().flatten())
    });
    if let Some(c) = cmd {
        ed.apply(ui, c, false);
    }
}

/// Masque ou affiche le calque `at`.
pub fn toggle_hidden(ui: Ui, ed: Ed, at: Target) {
    let cmd = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let hidden = revel_model::is_hidden(revel_edit::address::node(d, &at).ok()?);
        revel_edit::layer::set_hidden(d, &at, !hidden)
            .ok()
            .flatten()
    });
    if let Some(c) = cmd {
        ed.apply(ui, c, false);
    }
}

/// Champ du nom d'un calque, saisi sur place : Entrée ou sortie du champ appliquent, Échap
/// abandonne ; vide : nom automatique.
fn rename_input(
    ui: Ui,
    initial: String,
    commit: impl Fn(String) + Copy + 'static,
    cancel: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let value = RwSignal::new(initial);
    let node = NodeRef::<leptos::html::Input>::new();
    Effect::new(move |_| {
        if let Some(el) = node.get() {
            let _ = el.focus();
            el.select();
        }
    });
    let closed = StoredValue::new(false);
    let finish = move |ok: bool| {
        if closed.try_get_value().unwrap_or(true) {
            return;
        }
        closed.set_value(true);
        if ok {
            commit(value.get_untracked());
        } else {
            cancel();
        }
    };
    view! {
        <input class="layer-rename" node_ref=node aria-label=ui.t("layers-rename")
            prop:value=move || value.get()
            on:input=move |e| value.set(event_target_value(&e))
            on:click=|e| e.stop_propagation()
            on:dblclick=|e| e.stop_propagation()
            on:pointerdown=|e| e.stop_propagation()
            on:keydown=move |e: leptos::ev::KeyboardEvent| {
                e.stop_propagation();
                match e.key().as_str() {
                    "Enter" => { e.prevent_default(); finish(true) }
                    "Escape" => { e.prevent_default(); finish(false) }
                    _ => {}
                }
            }
            on:blur=move |_| finish(true)/>
    }
}

#[component]
pub fn Layers() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let opened = RwSignal::new(HashSet::<String>::new());
    let closed = RwSignal::new(HashSet::<String>::new());
    let drag = RwSignal::new(None::<Row>);
    let drop_at = RwSignal::new(None::<(String, Zone)>);
    let query = RwSignal::new(String::new());
    // Ligne en cours de renommage (clé).
    let renaming = RwSignal::new(None::<String>);
    // Libellés des collections (listes de collection nommées comme dans le panneau Pages).
    let col_folders = Memo::new(move |_| {
        ed.doc.with(|d| {
            d.as_ref()
                .map(|d| {
                    d.collections
                        .iter()
                        .map(|c| (c.id.clone(), c.folder.clone()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });
    Effect::new(move |_| {
        let cols = col_folders.get();
        let path = ed.path.get_value();
        leptos::task::spawn_local(async move {
            if let Ok(cms) = crate::api::cms_collections(&path).await {
                let map = cols
                    .into_iter()
                    .filter_map(|(id, folder)| {
                        let l = cms.iter().find(|c| c.folder == folder)?.label.clone()?;
                        Some((id, l))
                    })
                    .collect();
                let _ = ed.col_labels.try_set(map);
            }
        });
    });

    let all = Memo::new(move |_| {
        let url = ed.page.get();
        let comp = ed.comp.get();
        ed.doc.with(|d| {
            let Some(d) = d else { return Vec::new() };
            // Canevas des composants : les éléments du composant.
            if let Some(c) = comp {
                return component_rows(d, &c);
            }
            ed.page_ref(&url).map(|p| rows(d, &p)).unwrap_or_default()
        })
    });
    // Ouverture par défaut : toute la structure visible, sauf l'en-tête du document (`head`), les
    // composants et les niveaux profonds ; le reste s'ouvre à la demande ou à la sélection.
    let default_open = |r: &Row| {
        r.depth < 8
            && !matches!(&r.kind, Kind::Element { tag, .. } if tag.eq_ignore_ascii_case("head"))
            && !matches!(r.kind, Kind::Use(_))
    };
    let is_open = move |r: &Row, op: &HashSet<String>, cl: &HashSet<String>| {
        op.contains(&r.key) || (default_open(r) && !cl.contains(&r.key))
    };
    let visible = Memo::new(move |_| {
        let op = opened.get();
        let cl = closed.get();
        let q = query.get().trim().to_lowercase();
        all.with(|rows| {
            // Recherche : lignes dont le libellé correspond, avec leurs parents (ouverts).
            if !q.is_empty() {
                let hits: Vec<&Row> = rows
                    .iter()
                    .filter(|r| {
                        shown_name(ui, r).to_lowercase().contains(&q)
                            || label(ui, &r.kind).to_lowercase().contains(&q)
                            || tooltip(ui, &r.kind).to_lowercase().contains(&q)
                            || r.preview
                                .as_deref()
                                .is_some_and(|p| p.to_lowercase().contains(&q))
                    })
                    .collect();
                let keep: HashSet<&str> = hits
                    .iter()
                    .flat_map(|r| {
                        r.ancestors
                            .iter()
                            .map(String::as_str)
                            .chain(std::iter::once(r.key.as_str()))
                    })
                    .collect();
                return rows
                    .iter()
                    .filter(|r| keep.contains(r.key.as_str()))
                    .map(|r| (r.clone(), true))
                    .collect::<Vec<_>>();
            }
            let open: HashSet<&str> = rows
                .iter()
                .filter(|r| is_open(r, &op, &cl))
                .map(|r| r.key.as_str())
                .collect();
            rows.iter()
                .filter(|r| r.ancestors.iter().all(|a| open.contains(a.as_str())))
                .map(|r| (r.clone(), open.contains(r.key.as_str())))
                .collect::<Vec<_>>()
        })
    });
    // Sélection venue du canevas : ouvrir le chemin jusqu'à l'élément.
    Effect::new(move |_| {
        let Some(sel) = ed.selected.get() else {
            return;
        };
        let anc = all.with_untracked(|rows| {
            rows.iter()
                .find(|r| r.id.as_deref() == Some(sel.id.as_str()))
                .map(|r| r.ancestors.clone())
        });
        if let Some(anc) = anc {
            opened.update(|o| o.extend(anc.iter().cloned()));
            closed.update(|c| {
                for a in &anc {
                    c.remove(a);
                }
            });
        }
        // Ligne sélectionnée rendue visible dans l'arbre (après l'affichage des lignes ouvertes).
        request_animation_frame(|| {
            request_animation_frame(|| {
                if let Ok(Some(el)) = document().query_selector(".layers .layer.is-selected") {
                    let opts = web_sys::ScrollIntoViewOptions::new();
                    opts.set_block(web_sys::ScrollLogicalPosition::Center);
                    el.scroll_into_view_with_scroll_into_view_options(&opts);
                }
            });
        });
    });

    let select_row = move |r: &Row| {
        // Ligne sans identifiant (texte, expression) : son élément parent.
        let id = r.id.clone().or_else(|| r.parent.node.clone());
        let Some(id) = id else { return };
        let label = shown_name(ui, r);
        ed.selected.set(Some(Selection {
            id: id.clone(),
            tag: match &r.kind {
                Kind::Element { tag, .. } => tag.clone(),
                // Image du site : rendue en `<picture>` (réglages des médias : ajustement…).
                Kind::Image(_) => "img".into(),
                _ => String::new(),
            },
            label,
        }));
        ed.select_and_reveal(&id);
    };
    let selected_row = move || {
        let sel = ed.selected.get()?;
        all.with(|rows| {
            rows.iter()
                .find(|r| r.id.as_deref() == Some(sel.id.as_str()))
                .cloned()
        })
    };
    let target = move || {
        selected_row().and_then(|r| {
            Some(Target {
                owner: r.owner.clone(),
                node: r.id.clone()?,
            })
        })
    };
    let act = move |cmd: Option<Command>| {
        if let Some(c) = cmd {
            ed.apply(ui, c, false);
        }
    };

    // Parent de la ligne sélectionnée (élément qui la contient).
    let parent_row = move || {
        let r = selected_row()?;
        all.with(|rows| {
            r.ancestors
                .iter()
                .rev()
                .find_map(|a| rows.iter().find(|x| &x.key == a && x.id.is_some()).cloned())
        })
    };
    // Clavier : ↑ ↓ ligne précédente ou suivante, ← replier (ou parent), → déplier.
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        let key = e.key();
        if key == "F2" {
            if let Some(r) = selected_row().filter(nameable) {
                e.prevent_default();
                renaming.set(Some(r.key));
            }
            return;
        }
        if !matches!(
            key.as_str(),
            "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight"
        ) {
            return;
        }
        e.prevent_default();
        let rows: Vec<(Row, bool)> = visible.get_untracked();
        let cur = selected_row();
        let pos = cur
            .as_ref()
            .and_then(|c| rows.iter().position(|(r, _)| r.key == c.key));
        match key.as_str() {
            "ArrowUp" | "ArrowDown" => {
                let up = key == "ArrowUp";
                let start = pos.map(|p| p as i64).unwrap_or(-1);
                let mut i = start;
                loop {
                    i += if up { -1 } else { 1 };
                    if i < 0 || i as usize >= rows.len() {
                        break;
                    }
                    if rows[i as usize].0.id.is_some() {
                        select_row(&rows[i as usize].0);
                        break;
                    }
                }
            }
            "ArrowLeft" => {
                if let Some((r, open)) = pos.map(|p| rows[p].clone()) {
                    if open && r.has_children {
                        opened.update(|o| {
                            o.remove(&r.key);
                        });
                        closed.update(|c| {
                            c.insert(r.key.clone());
                        });
                    } else if let Some(p) = parent_row() {
                        select_row(&p);
                    }
                }
            }
            _ => {
                if let Some((r, open)) = pos.map(|p| rows[p].clone())
                    && !open
                    && r.has_children
                {
                    closed.update(|c| {
                        c.remove(&r.key);
                    });
                    opened.update(|o| {
                        o.insert(r.key.clone());
                    });
                }
            }
        }
    };
    let collapse_all = move |_| {
        let keys: HashSet<String> = all.with_untracked(|rows| {
            rows.iter()
                .filter(|r| r.has_children && r.depth > 0)
                .map(|r| r.key.clone())
                .collect()
        });
        opened.set(HashSet::new());
        closed.set(keys);
    };
    // Info-bulle : action et raccourci.
    let tip = move |label: &str, key: &str| format!("{} ({})", ui.tx(label, &[]), ui.tx(key, &[]));
    // Menu contextuel d'un calque (clic droit) : icône, action, raccourci.
    let open_menu = move |e: &leptos::ev::MouseEvent| {
        use crate::ctxmenu::{CtxItem, open};
        let has = target().is_some();
        let items = vec![
            CtxItem::new(ui.now("layers-parent", &[]), move || {
                if let Some(p) = parent_row() {
                    select_row(&p)
                }
            })
            .icon(icons::parent)
            .key(ui.now("key-parent", &[]))
            .disabled(parent_row().is_none()),
            CtxItem::new(ui.now("layers-up", &[]), move || {
                ed.step(ui, -1);
            })
            .icon(icons::arrow_up)
            .key(ui.now("key-up", &[])),
            CtxItem::new(ui.now("layers-down", &[]), move || {
                ed.step(ui, 1);
            })
            .icon(icons::arrow_down)
            .key(ui.now("key-down", &[])),
            CtxItem::new(ui.now("layers-wrap", &[]), move || {
                if let Some(at) = target() {
                    ed.wrap(ui, at)
                }
            })
            .icon(icons::wrap)
            .key(ui.now("key-wrap", &[]))
            .disabled(!has)
            .sep(),
            CtxItem::new(ui.now("layers-duplicate", &[]), move || {
                act(target().map(|at| Command::Duplicate { at }))
            })
            .icon(icons::copy)
            .key(ui.now("key-duplicate", &[]))
            .disabled(!has),
            CtxItem::new(ui.now("layers-rename", &[]), move || {
                if let Some(r) = selected_row().filter(nameable) {
                    renaming.set(Some(r.key));
                }
            })
            .icon(icons::edit)
            .key("F2".to_string())
            .disabled(!selected_row().is_some_and(|r| nameable(&r)))
            .sep(),
            CtxItem::new(
                ui.now(
                    if selected_row().is_some_and(|r| r.hidden) {
                        "layers-show"
                    } else {
                        "layers-hide"
                    },
                    &[],
                ),
                move || {
                    if let Some(at) = target() {
                        toggle_hidden(ui, ed, at)
                    }
                },
            )
            .icon(if selected_row().is_some_and(|r| r.hidden) {
                icons::eye
            } else {
                icons::eye_off
            })
            .key(ui.now("key-hide", &[]))
            .disabled(!selected_row().is_some_and(|r| nameable(&r))),
        ];
        let mut items = items;
        // Composants, comme dans le Navigator de Webflow : créer à partir du calque ; ouvrir ou
        // détacher une utilisation.
        let at = StoredValue::new(target());
        let is_use = at.with_value(|t| {
            t.as_ref().is_some_and(|t| {
                ed.doc.with_untracked(|d| {
                    d.as_ref()
                        .and_then(|d| revel_edit::address::node(d, t).ok())
                        .is_some_and(|n| matches!(n, revel_model::Node::Use { .. }))
                })
            })
        });
        if is_use {
            items.push(
                CtxItem::new(ui.now("component-edit", &[]), move || {
                    if let Some(t) = at.get_value() {
                        crate::components::open_component(ed, &t)
                    }
                })
                .icon(icons::edit)
                .sep(),
            );
            items.push(
                CtxItem::new(ui.now("component-detach", &[]), move || {
                    if let Some(t) = at.get_value() {
                        crate::components::detach(ui, ed, t)
                    }
                })
                .icon(icons::unlink),
            );
        } else {
            items.push(
                CtxItem::new(ui.now("component-make", &[]), move || {
                    crate::components::make_from_selection(ui, ed)
                })
                .icon(icons::component)
                .disabled(!has)
                .sep(),
            );
        }
        items.push(
            CtxItem::new(ui.now("layers-delete", &[]), move || {
                act(target().map(|at| Command::Remove { at }))
            })
            .icon(icons::trash)
            .key(ui.now("key-delete", &[]))
            .disabled(!has)
            .danger()
            .sep(),
        );
        open(ui, e, items);
    };
    view! {
        <aside class="layers" aria-label=ui.t("layers-title")>
            <label class="panel-search">
                {icons::search()}
                <input type="search" placeholder=ui.t("layers-search") aria-label=ui.t("layers-search")
                    prop:value=move || query.get() on:input=move |e| query.set(event_target_value(&e))/>
            </label>
            <div class="layers-tree" role="tree" tabindex="0" on:keydown=on_key>
                <For each=move || visible.get()
                    key=|(r, open)| (r.key.clone(), format!("{:?}{}{}{:?}{:?}{}{}{:?}{}{:?}{}{}", r.kind, r.has_children, open, r.id, r.preview, r.animated, r.bound, r.condition, r.textual, r.name, r.hidden, r.dimmed))
                    let:item>
                    {
                        let (r, open) = item;
                        let r = StoredValue::new(r);
                        let key = r.with_value(|r| r.key.clone());
                        let k_sel = key.clone();
                        let k_drop = key.clone();
                        let id = r.with_value(|r| r.id.clone());
                        let id_hover = id.clone();
                        let depth = r.with_value(|r| r.depth);
                        let has_children = r.with_value(|r| r.has_children);
                        let kind = r.with_value(|r| r.kind.clone());
                        let animated = r.with_value(|r| r.animated);
                        let bound = r.with_value(|r| r.bound);
                        let textual = r.with_value(|r| r.textual);
                        let condition = r.with_value(|r| r.condition.clone());
                        let is_conditional = condition.is_some();
                        let hidden = r.with_value(|r| r.hidden);
                        let dimmed = r.with_value(|r| r.dimmed);
                        let can_name = r.with_value(nameable);
                        let k_ren = key.clone();
                        let is_renaming = move || renaming.get().as_deref() == Some(k_ren.as_str());
                        // Texte affiché : texte écrit, sinon texte rendu dans la page (données du
                        // site, contenu), envoyé par le canevas — jamais l'expression.
                        let inst = r.with_value(|r| all.with_untracked(|rows| use_of(rows, r)));
                        let preview = move || {
                            r.with_value(|r| r.preview.clone()).or_else(|| {
                                let id = r.with_value(|r| r.id.clone())?;
                                if !bound {
                                    return None;
                                }
                                ed.previews.with(|m| {
                                    inst.as_ref()
                                        .and_then(|u| m.get(&format!("{u}/{id}")))
                                        .or_else(|| m.get(&id))
                                        .map(|t| short(t, 48))
                                })
                            })
                        };
                        // Nom technique (`section.hero`) : info-bulle, recherche, essais.
                        let tech = r.with_value(|r| match &r.kind {
                            Kind::Element { tag, class } => crate::names::technical(tag, class),
                            k => label(ui, k),
                        });
                        let title = r.with_value(|r| match &r.kind {
                            Kind::Use(_) => format!("{} · {}", label(ui, &r.kind), ui.tx("layers-badge-component", &[])),
                            k if r.name.is_some() => format!("{}\n{}", shown_name(ui, r), tooltip(ui, k)),
                            k => tooltip(ui, k),
                        });
                        let toggle = move |e: leptos::ev::MouseEvent| {
                            e.stop_propagation();
                            let k = r.with_value(|r| r.key.clone());
                            if open {
                                opened.update(|o| { o.remove(&k); });
                                closed.update(|c| { c.insert(k); });
                            } else {
                                closed.update(|c| { c.remove(&k); });
                                opened.update(|o| { o.insert(k); });
                            }
                        };
                        let id_multi = id.clone();
                        let in_multi = move || ed.multi.with(|m| m.iter().any(|x| Some(&x.id) == id_multi.as_ref()));
                        let is_selected = move || {
                            let sel = ed.selected.get().map(|s| s.id);
                            sel.is_some() && sel == id.clone() && all.with(|rows| {
                                rows.iter().find(|x| x.id == sel).map(|x| x.key == k_sel).unwrap_or(false)
                            })
                        };
                        let zone_class = move |z: Zone| {
                            let k = k_drop.clone();
                            move || drop_at.get().is_some_and(|(dk, dz)| dk == k && dz == z)
                        };
                        let draggable = r.with_value(|r| r.id.is_some()).to_string();
                        view! {
                            <div class="layer" role="treeitem" title=title data-tech=tech class:is-selected=is_selected class:is-multi=in_multi
                                class:is-component=matches!(kind, Kind::Use(_))
                                class:is-logic=matches!(kind, Kind::If(_) | Kind::Branch(_))
                                class:is-text=matches!(kind, Kind::Text(_) | Kind::Expr(_) | Kind::Html)
                                class:is-conditional=is_conditional
                                class:is-hidden-layer=dimmed
                                class:drop-before=zone_class(Zone::Before)
                                class:drop-after=zone_class(Zone::After)
                                class:drop-inside=zone_class(Zone::Inside)
                                style=format!("--depth:{depth}")
                                data-depth=depth
                                draggable=draggable
                                on:click=move |e: leptos::ev::MouseEvent| r.with_value(|r| {
                                    // Maj (ou Ctrl) + clic : ajouté à la sélection ou retiré.
                                    if (e.shift_key() || e.ctrl_key() || e.meta_key()) && ed.selected.get_untracked().is_some() {
                                        if let Some(id) = r.id.clone() {
                                            ed.toggle_select(Selection {
                                                id,
                                                tag: match &r.kind { Kind::Element { tag, .. } => tag.clone(), Kind::Image(_) => "img".into(), _ => String::new() },
                                                label: label(ui, &r.kind),
                                            });
                                        }
                                    } else {
                                        select_row(r)
                                    }
                                })
                                on:dblclick=move |_| {
                                    // Utilisation de composant : son canevas (comme un double-clic
                                    // sur la page).
                                    if matches!(r.with_value(|r| r.kind.clone()), Kind::Use(_)) {
                                        if let Some(t) = r.with_value(|r| Some(Target { owner: r.owner.clone(), node: r.id.clone()? })) {
                                            crate::components::open_component(ed, &t);
                                        }
                                        return;
                                    }
                                    // Texte : modifié sur place dans la page (comme un double-clic
                                    // sur la page).
                                    if textual && let Some(id) = r.with_value(|r| r.id.clone()) {
                                        ed.post(serde_json::json!({ "type": "edit", "id": id }));
                                    }
                                }
                                on:contextmenu=move |e: leptos::ev::MouseEvent| {
                                    r.with_value(|r| select_row(r));
                                    open_menu(&e);
                                }
                                on:mouseenter=move |_| ed.post(serde_json::json!({ "type": "hover", "id": id_hover.clone() }))
                                on:mouseleave=move |_| ed.post(serde_json::json!({ "type": "hover", "id": null }))
                                on:dragstart=move |e: leptos::ev::DragEvent| {
                                    if let Some(dt) = e.data_transfer() {
                                        let _ = dt.set_data("text/plain", &key);
                                        dt.set_effect_allowed("move");
                                    }
                                    drag.set(Some(r.get_value()));
                                }
                                on:dragend=move |_| { drag.set(None); drop_at.set(None); }
                                on:dragover=move |e: leptos::ev::DragEvent| {
                                    let Some(src) = drag.get_untracked() else { return };
                                    let dst = r.get_value();
                                    let rect = e
                                        .current_target()
                                        .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok())
                                        .map(|el| el.get_bounding_client_rect());
                                    let Some(rect) = rect else { return };
                                    let y = (f64::from(e.client_y()) - rect.top()) / rect.height().max(1.0);
                                    let zone = if dst.inside.is_some() && (0.3..0.7).contains(&y) {
                                        Zone::Inside
                                    } else if y < 0.5 {
                                        Zone::Before
                                    } else {
                                        Zone::After
                                    };
                                    if move_command(&src, &dst, zone).is_some() {
                                        e.prevent_default();
                                        drop_at.set(Some((dst.key.clone(), zone)));
                                    } else {
                                        drop_at.set(None);
                                    }
                                }
                                on:drop=move |e: leptos::ev::DragEvent| {
                                    e.prevent_default();
                                    let src = drag.get_untracked();
                                    let zone = drop_at.get_untracked().map(|(_, z)| z);
                                    drag.set(None);
                                    drop_at.set(None);
                                    if let (Some(src), Some(zone)) = (src, zone) {
                                        act(move_command(&src, &r.get_value(), zone));
                                    }
                                }>
                                <span class="layer-guides" aria-hidden="true"></span>
                                <button class="layer-toggle" tabindex="-1" aria-hidden="true"
                                    class:is-hidden=!has_children on:click=toggle>
                                    {if open { icons::chevron_down() } else { icons::chevron_right() }}
                                </button>
                                <span class="layer-icon">{icon(&kind, textual, has_children)}</span>
                                {let kind = kind.clone(); move || if is_renaming() {
                                    let at = StoredValue::new(r.with_value(|r| r.id.clone().map(|id| Target { owner: r.owner.clone(), node: id })));
                                    let initial = r.with_value(|r| shown_name(ui, r));
                                    rename_input(ui, initial,
                                        move |v: String| {
                                            renaming.set(None);
                                            if let Some(at) = at.get_value() {
                                                // Nom identique au nom automatique : pas de nom propre.
                                                let auto = r.with_value(|r| label(ui, &r.kind));
                                                let v = if v.trim() == auto { String::new() } else { v };
                                                rename_layer(ui, ed, at, &v);
                                            }
                                        },
                                        move || renaming.set(None)).into_any()
                                } else {
                                    let k = kind.clone();
                                    view! {
                                        <span class="layer-label" class:is-named=r.with_value(|r| r.name.is_some())
                                            // Double-clic sur le nom : renommer (sur l'icône ou le
                                            // texte de l'élément : action habituelle).
                                            on:dblclick=move |e: leptos::ev::MouseEvent| {
                                                if can_name {
                                                    e.stop_propagation();
                                                    r.with_value(|r| select_row(r));
                                                    renaming.set(Some(r.with_value(|r| r.key.clone())));
                                                }
                                            }>{
                                            // Liste de collection : nom suivi (libellés lus après coup).
                                            move || r.with_value(|r| r.name.clone()).unwrap_or_else(|| label(ui, &k))
                                        }</span>
                                    }.into_any()
                                }}
                                {move || preview().map(|p| view! {
                                    <span class="layer-preview" class:is-bound=bound
                                        title=move || textual.then(|| ui.now("layers-edit-text", &[]))>{p}</span>
                                })}
                                <span class="layer-marks">
                                    {bound.then(|| view! { <span class="layer-mark" title=ui.t("layers-bound")>{icons::database()}</span> })}
                                    {condition.map(|c| view! { <span class="layer-mark" title=ui.tx("layers-condition", &[("cond", c)])>{icons::branch()}</span> })}
                                    {animated.then(|| view! { <span class="layer-mark layer-anim" title=ui.t("layers-animated")>{icons::bolt()}</span> })}
                                    {hidden.then(|| view! {
                                        <button class="layer-mark layer-hidden" title=ui.t("layers-show") aria-label=ui.t("layers-show")
                                            on:click=move |e: leptos::ev::MouseEvent| {
                                                e.stop_propagation();
                                                if let Some(at) = r.with_value(|r| r.id.clone().map(|id| Target { owner: r.owner.clone(), node: id })) {
                                                    toggle_hidden(ui, ed, at);
                                                }
                                            }>{icons::eye_off()}</button>
                                    })}
                                </span>
                            </div>
                        }
                    }
                </For>
            </div>
            <footer class="panel-foot layers-foot">
                <button class="icon-btn icon-btn--small" title=ui.t("layers-collapse") aria-label=ui.t("layers-collapse")
                    on:click=collapse_all>{icons::collapse()}</button>
                <span class="toolbar-spacer"></span>
                <button class="icon-btn icon-btn--small" title=move || tip("layers-parent", "key-parent") aria-label=ui.t("layers-parent")
                    prop:disabled=move || parent_row().is_none()
                    on:click=move |_| { if let Some(p) = parent_row() { select_row(&p) } }>{icons::parent()}</button>
                <button class="icon-btn icon-btn--small" title=move || tip("layers-up", "key-up") aria-label=ui.t("layers-up")
                    prop:disabled=move || selected_row().is_none()
                    on:click=move |_| { ed.step(ui, -1); }>{icons::arrow_up()}</button>
                <button class="icon-btn icon-btn--small" title=move || tip("layers-down", "key-down") aria-label=ui.t("layers-down")
                    prop:disabled=move || selected_row().is_none()
                    on:click=move |_| { ed.step(ui, 1); }>{icons::arrow_down()}</button>
                <button class="icon-btn icon-btn--small" title=move || tip("layers-wrap", "key-wrap") aria-label=ui.t("layers-wrap")
                    prop:disabled=move || target().is_none()
                    on:click=move |_| { ed.wrap_selection(ui); }>{icons::wrap()}</button>
                <button class="icon-btn icon-btn--small" title=move || tip("layers-duplicate", "key-duplicate") aria-label=ui.t("layers-duplicate")
                    prop:disabled=move || target().is_none()
                    on:click=move |_| { ed.duplicate_selection(ui); }>{icons::copy()}</button>
                <button class="icon-btn icon-btn--small" title=move || tip("layers-delete", "key-delete") aria-label=ui.t("layers-delete")
                    prop:disabled=move || target().is_none()
                    on:click=move |_| { ed.remove_selection(ui); }>{icons::trash()}</button>
            </footer>
        </aside>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use revel_model::Attr;

    fn el(id: &str, tag: &str, class: &str, children: Vec<Node>) -> Node {
        Node::Element {
            id: Some(id.into()),
            tag: tag.into(),
            attrs: if class.is_empty() {
                vec![]
            } else {
                vec![Attr {
                    name: "class".into(),
                    value: Some(class.into()),
                    when: None,
                }]
            },
            children,
        }
    }

    fn text(t: &str) -> Node {
        Node::Text { text: t.into() }
    }

    #[test]
    fn arbre_sans_en_tete_ni_textes_a_part() {
        let mut site = Site::new("Essai", "fr");
        let layout = revel_edit::make::base_layout(&site);
        let lid = layout.id.clone();
        site.layouts.push(layout);
        let mut page: revel_model::Page = serde_json::from_value(serde_json::json!({
            "id": "index", "path": "/", "layout": lid
        }))
        .unwrap();
        page.body = vec![el(
            "s1",
            "section",
            "bloc bloc-hero",
            vec![
                el("t1", "h1", "", vec![text("Bonjour")]),
                text("seul"),
                el("b1", "br", "", vec![]),
            ],
        )];
        site.pages.push(page);
        let info: revel_protocol::EditorPageInfo = serde_json::from_value(serde_json::json!({
            "url": "/", "page": "index", "layout": lid
        }))
        .unwrap();
        let rows = rows(&site, &info);
        let shown: Vec<(usize, String, Option<String>)> = rows
            .iter()
            .map(|r| {
                let t = match &r.kind {
                    Kind::Element { tag, class } => crate::names::technical(tag, class),
                    k => format!("{k:?}"),
                };
                (r.depth, t, r.preview.clone())
            })
            .collect();
        // Ni doctype, `html`, `head`, `meta`, `title`, `link`, `br` ; `body` à la racine, contenu de
        // la page (seul dans `body`) directement dessous ; textes sur la ligne de leur élément.
        assert_eq!(
            shown,
            vec![
                (0, "body".to_string(), None),
                (
                    1,
                    "section.bloc.bloc-hero".to_string(),
                    Some("seul".to_string())
                ),
                (2, "h1".to_string(), Some("Bonjour".to_string())),
            ]
        );
        assert!(rows[0].has_children && rows[1].has_children && !rows[2].has_children);
        assert_eq!(
            rows[2].ancestors,
            vec![rows[0].key.clone(), rows[1].key.clone()]
        );
        // Le titre se déplace dans la section (même propriétaire : la page).
        let up = step_command(&rows, &rows[2], -1);
        assert!(up.is_none());
    }

    #[test]
    fn listes_conditions_et_textes_lies_lisibles() {
        let mut site = Site::new("Essai", "fr");
        let layout = revel_edit::make::base_layout(&site);
        let lid = layout.id.clone();
        site.layouts.push(layout);
        let mut page: revel_model::Page = serde_json::from_value(serde_json::json!({
            "id": "index", "path": "/", "layout": lid
        }))
        .unwrap();
        page.body = vec![
            el("a1", "a", "skip-link", vec![text("Aller au contenu")]),
            el(
                "d1",
                "div",
                "intro",
                vec![Node::Out {
                    expr: "home.agency_text | safe".into(),
                }],
            ),
            Node::If {
                id: Some("i1".into()),
                cond: "it".into(),
                then: vec![el("p1", "p", "", vec![text("Oui")])],
                otherwise: vec![],
            },
            Node::Each {
                id: Some("e1".into()),
                expr: "home.featured".into(),
                item: "slug".into(),
                children: vec![el("c1", "article", "carte", vec![])],
                empty: vec![],
            },
        ];
        site.pages.push(page);
        let info: revel_protocol::EditorPageInfo = serde_json::from_value(serde_json::json!({
            "url": "/", "page": "index", "layout": lid
        }))
        .unwrap();
        let rows = rows(&site, &info);
        // Lien d'évitement absent ; condition sans ligne (son paragraphe marqué) ; liste nommée
        // d'après sa source ; texte venu des données marqué lié, sans l'expression.
        assert!(!rows.iter().any(|r| r.id.as_deref() == Some("a1")));
        assert!(!rows.iter().any(|r| matches!(r.kind, Kind::If(_))));
        let p = rows.iter().find(|r| r.id.as_deref() == Some("p1")).unwrap();
        assert_eq!(p.condition.as_deref(), Some("it"));
        assert_eq!(p.preview.as_deref(), Some("Oui"));
        let d = rows.iter().find(|r| r.id.as_deref() == Some("d1")).unwrap();
        assert!(d.bound && d.textual && d.preview.is_none());
        assert_eq!(condition_name("env.staging"), "Staging");
        assert_eq!(condition_name("not page.draft and x"), "Draft");
        assert_eq!(image_file("/media/equipe.jpg"), Some("equipe.jpg".into()));
        assert_eq!(image_file("first.data.cover if first else ''"), None);
        assert_eq!(list_name("home.featured"), "Featured");
        assert_eq!(list_name("collections.blog | reverse"), "Blog");
    }
}
