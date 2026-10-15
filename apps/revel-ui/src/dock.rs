//! Espace de travail modulable (D-21), sur le modèle des panneaux d'After Effects : chaque panneau
//! (calques, styles…) se déplace par son onglet vers une colonne, un groupe (en onglet ou au-dessus,
//! au-dessous d'un autre) ou une fenêtre flottante ; colonnes et groupes se redimensionnent ; un
//! panneau se ferme et se rouvre depuis le menu « Panneaux » ; Ctrl+\ masque tous les panneaux.
//! La disposition est gardée d'une session à l'autre. Le canevas reste au centre.

use crate::{Ui, icons};
use leptos::ev;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

/// Panneaux de l'éditeur.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PanelId {
    Pages,
    Layers,
    Variables,
    Cms,
    Insert,
    Components,
    Assets,
    Styles,
    Settings,
    Interactions,
}

impl PanelId {
    pub const ALL: [PanelId; 10] = [
        PanelId::Pages,
        PanelId::Layers,
        PanelId::Variables,
        PanelId::Cms,
        PanelId::Insert,
        PanelId::Components,
        PanelId::Assets,
        PanelId::Styles,
        PanelId::Settings,
        PanelId::Interactions,
    ];

    /// Clé de traduction du nom.
    pub fn key(self) -> &'static str {
        match self {
            PanelId::Pages => "panel-pages",
            PanelId::Layers => "layers-title",
            PanelId::Variables => "panel-variables",
            PanelId::Cms => "panel-cms",
            PanelId::Insert => "insert-title",
            PanelId::Components => "panel-components",
            PanelId::Assets => "panel-assets",
            PanelId::Styles => "styles-title",
            PanelId::Settings => "inspector-settings",
            PanelId::Interactions => "panel-interactions",
        }
    }

    /// Nom technique (attribut `data-panel`, essais).
    pub fn name(self) -> &'static str {
        match self {
            PanelId::Pages => "pages",
            PanelId::Layers => "layers",
            PanelId::Variables => "variables",
            PanelId::Cms => "cms",
            PanelId::Insert => "insert",
            PanelId::Components => "components",
            PanelId::Assets => "assets",
            PanelId::Styles => "styles",
            PanelId::Settings => "settings",
            PanelId::Interactions => "interactions",
        }
    }

    /// Côté où le panneau revient quand on le rouvre.
    fn home(self) -> Side {
        match self {
            PanelId::Styles | PanelId::Settings | PanelId::Interactions => Side::Right,
            _ => Side::Left,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Side {
    Left,
    Right,
}

fn one() -> f64 {
    1.0
}

/// Groupe d'onglets.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Group {
    pub id: u32,
    pub tabs: Vec<PanelId>,
    pub active: PanelId,
    /// Part de la hauteur de la colonne (poids).
    #[serde(default = "one")]
    pub size: f64,
}

/// Fenêtre flottante (position et taille en pixels dans l'espace de travail).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Float {
    pub group: Group,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Disposition de l'espace de travail.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Layout {
    pub left: Vec<Group>,
    pub right: Vec<Group>,
    pub left_w: f64,
    pub right_w: f64,
    #[serde(default)]
    pub floats: Vec<Float>,
    /// Panneaux fermés.
    #[serde(default)]
    pub closed: Vec<PanelId>,
    /// Tous les panneaux masqués (Ctrl+\).
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub next: u32,
}

/// Emplacement de dépôt d'un onglet.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Drop {
    /// Dans un groupe, à la position `index` des onglets.
    Join { group: u32, index: usize },
    /// Nouveau groupe au-dessus (`after: false`) ou au-dessous d'un groupe.
    Split { group: u32, after: bool },
    /// Nouveau groupe en bas d'une colonne.
    Column { side: Side },
    /// Fenêtre flottante.
    Float { x: f64, y: f64 },
}

pub const MIN_COL: f64 = 200.0;
pub const MAX_COL: f64 = 520.0;

impl Default for Layout {
    /// Disposition par défaut : pages, composants et médias au-dessus des calques (comme Figma),
    /// CMS entre calques et ajouter ; styles, réglages et interactions à droite.
    fn default() -> Self {
        Self {
            left: vec![
                Group {
                    id: 1,
                    tabs: vec![PanelId::Pages, PanelId::Components, PanelId::Assets],
                    active: PanelId::Pages,
                    size: 0.32,
                },
                Group {
                    id: 2,
                    tabs: vec![PanelId::Layers, PanelId::Variables, PanelId::Insert],
                    active: PanelId::Layers,
                    size: 0.68,
                },
            ],
            right: vec![Group {
                id: 3,
                tabs: vec![PanelId::Styles, PanelId::Settings, PanelId::Interactions],
                active: PanelId::Styles,
                size: 1.0,
            }],
            left_w: 260.0,
            right_w: 284.0,
            floats: Vec::new(),
            // Contenus des collections : vue CMS et panneau Pages.
            closed: vec![PanelId::Cms],
            hidden: false,
            next: 4,
        }
    }
}

impl Layout {
    fn column_mut(&mut self, side: Side) -> &mut Vec<Group> {
        match side {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }

    /// Tous les groupes (colonnes et fenêtres).
    pub fn groups(&self) -> impl Iterator<Item = &Group> {
        self.left
            .iter()
            .chain(self.right.iter())
            .chain(self.floats.iter().map(|f| &f.group))
    }

    fn groups_mut(&mut self) -> impl Iterator<Item = &mut Group> {
        self.left
            .iter_mut()
            .chain(self.right.iter_mut())
            .chain(self.floats.iter_mut().map(|f| &mut f.group))
    }

    pub fn group(&self, id: u32) -> Option<&Group> {
        self.groups().find(|g| g.id == id)
    }

    /// Groupe qui contient un panneau.
    pub fn group_of(&self, p: PanelId) -> Option<u32> {
        self.groups().find(|g| g.tabs.contains(&p)).map(|g| g.id)
    }

    /// Colonne d'un groupe (`None` : fenêtre flottante).
    fn side_of(&self, id: u32) -> Option<Side> {
        if self.left.iter().any(|g| g.id == id) {
            Some(Side::Left)
        } else if self.right.iter().any(|g| g.id == id) {
            Some(Side::Right)
        } else {
            None
        }
    }

    fn new_group(&mut self, p: PanelId, size: f64) -> Group {
        self.next = self
            .next
            .max(self.groups().map(|g| g.id).max().unwrap_or(0) + 1);
        let id = self.next;
        self.next += 1;
        Group {
            id,
            tabs: vec![p],
            active: p,
            size,
        }
    }

    /// Retire un panneau de son groupe (groupes et fenêtres vides supprimés).
    pub fn take(&mut self, p: PanelId) {
        for g in self.groups_mut() {
            if let Some(i) = g.tabs.iter().position(|t| *t == p) {
                g.tabs.remove(i);
                if g.active == p
                    && let Some(n) = g.tabs.get(i.min(g.tabs.len().saturating_sub(1)))
                {
                    g.active = *n;
                }
            }
        }
        self.left.retain(|g| !g.tabs.is_empty());
        self.right.retain(|g| !g.tabs.is_empty());
        self.floats.retain(|f| !f.group.tabs.is_empty());
    }

    /// Place un panneau ; `false` si le dépôt ne change rien.
    pub fn put(&mut self, p: PanelId, d: Drop) -> bool {
        let src = self.group_of(p);
        // Seul onglet d'un groupe déposé sur ce même groupe : rien à faire.
        let alone = src
            .and_then(|id| self.group(id))
            .is_some_and(|g| g.tabs.len() == 1);
        match d {
            Drop::Join { group, .. } | Drop::Split { group, .. } if alone && Some(group) == src => {
                return false;
            }
            _ => {}
        }
        // Position voulue dans le groupe, corrigée si l'onglet vient du même groupe, avant.
        let d = match d {
            Drop::Join { group, index } if Some(group) == src => {
                let from = self
                    .group(group)
                    .and_then(|g| g.tabs.iter().position(|t| *t == p))
                    .unwrap_or(0);
                Drop::Join {
                    group,
                    index: if from < index { index - 1 } else { index },
                }
            }
            d => d,
        };
        self.take(p);
        self.closed.retain(|c| *c != p);
        match d {
            Drop::Join { group, index } => {
                if self.group(group).is_none() {
                    return self.put(p, Drop::Column { side: p.home() });
                }
                if let Some(g) = self.groups_mut().find(|g| g.id == group) {
                    g.tabs.insert(index.min(g.tabs.len()), p);
                    g.active = p;
                }
            }
            Drop::Split { group, after } => match self.side_of(group) {
                Some(side) => {
                    let col = self.column_mut(side);
                    let i = col.iter().position(|g| g.id == group).unwrap_or(0);
                    let half = col[i].size / 2.0;
                    col[i].size = half;
                    let g = self.new_group(p, half);
                    self.column_mut(side)
                        .insert(if after { i + 1 } else { i }, g);
                }
                // Fenêtre flottante : rejoint ses onglets.
                None => {
                    if self.group(group).is_none() {
                        return self.put(p, Drop::Column { side: p.home() });
                    }
                    if let Some(g) = self.groups_mut().find(|g| g.id == group) {
                        g.tabs.push(p);
                        g.active = p;
                    }
                }
            },
            Drop::Column { side } => {
                let size = {
                    let col = self.column_mut(side);
                    if col.is_empty() {
                        1.0
                    } else {
                        col.iter().map(|g| g.size).sum::<f64>() / col.len() as f64
                    }
                };
                let g = self.new_group(p, size);
                self.column_mut(side).push(g);
            }
            Drop::Float { x, y } => {
                let g = self.new_group(p, 1.0);
                self.floats.push(Float {
                    group: g,
                    x: x.max(0.0),
                    y: y.max(0.0),
                    w: 300.0,
                    h: 420.0,
                });
            }
        }
        true
    }

    pub fn activate(&mut self, p: PanelId) {
        for g in self.groups_mut() {
            if g.tabs.contains(&p) {
                g.active = p;
            }
        }
    }

    /// Ferme un panneau (il se rouvre depuis le menu « Panneaux »).
    pub fn close(&mut self, p: PanelId) {
        self.take(p);
        if !self.closed.contains(&p) {
            self.closed.push(p);
        }
    }

    /// Rouvre un panneau fermé (ou l'affiche s'il est ouvert) : dans le premier groupe de son côté.
    pub fn open(&mut self, p: PanelId) {
        self.hidden = false;
        if self.group_of(p).is_some() {
            self.activate(p);
            return;
        }
        // À côté de ses voisins de la disposition par défaut (Pages avec Composants et Médias,
        // Variables après Calques), sinon dans une colonne de son côté.
        if let Some((group, index)) = self.default_spot(p) {
            self.put(p, Drop::Join { group, index });
            return;
        }
        let side = p.home();
        let first = match side {
            Side::Left => self.left.last().map(|g| g.id),
            Side::Right => self.right.first().map(|g| g.id),
        };
        match first {
            Some(group) => {
                self.put(
                    p,
                    Drop::Join {
                        group,
                        index: usize::MAX,
                    },
                );
            }
            None => {
                self.put(p, Drop::Column { side });
            }
        }
    }

    /// Place d'un panneau d'après la disposition par défaut : après le voisin qui le précède
    /// (sinon avant celui qui le suit) dans le groupe où ce voisin se trouve.
    fn default_spot(&self, p: PanelId) -> Option<(u32, usize)> {
        let def = Layout::default();
        let tabs = &def.groups().find(|g| g.tabs.contains(&p))?.tabs;
        let at = tabs.iter().position(|t| *t == p)?;
        let find = |n: PanelId| {
            self.groups()
                .find_map(|g| Some((g.id, g.tabs.iter().position(|t| *t == n)?)))
        };
        tabs[..at]
            .iter()
            .rev()
            .find_map(|n| find(*n).map(|(g, i)| (g, i + 1)))
            .or_else(|| tabs[at + 1..].iter().find_map(|n| find(*n)))
    }

    /// Disposition relue (fichier d'une version précédente, modifiée à la main) : chaque panneau
    /// présent une fois, panneaux nouveaux ajoutés, largeurs bornées.
    pub fn normalize(mut self) -> Self {
        let mut seen = Vec::new();
        for g in self.groups_mut() {
            g.tabs.retain(|t| {
                if seen.contains(t) {
                    false
                } else {
                    seen.push(*t);
                    true
                }
            });
            if !g.tabs.contains(&g.active)
                && let Some(t) = g.tabs.first()
            {
                g.active = *t;
            }
            if !(g.size.is_finite() && g.size > 0.0) {
                g.size = 1.0;
            }
        }
        self.left.retain(|g| !g.tabs.is_empty());
        self.right.retain(|g| !g.tabs.is_empty());
        self.floats.retain(|f| !f.group.tabs.is_empty());
        self.closed.retain(|c| !seen.contains(c));
        // Panneaux nouveaux ajoutés sans changer l'onglet affiché des groupes.
        let actives: Vec<(u32, PanelId)> = self.groups().map(|g| (g.id, g.active)).collect();
        for p in PanelId::ALL {
            if !seen.contains(&p) && !self.closed.contains(&p) {
                self.open(p);
            }
        }
        for g in self.groups_mut() {
            if let Some((_, a)) = actives.iter().find(|(id, _)| *id == g.id)
                && g.tabs.contains(a)
            {
                g.active = *a;
            }
        }
        self.hidden = false;
        self.left_w = self.left_w.clamp(MIN_COL, MAX_COL);
        self.right_w = self.right_w.clamp(MIN_COL, MAX_COL);
        self
    }
}

// ------------------------------------------------------------------------------------------------
// Interface

const STORE: &str = "revel.workspace.v1";

fn load() -> Layout {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|s| s.get_item(STORE).ok().flatten())
        .and_then(|j| serde_json::from_str::<Layout>(&j).ok())
        .map(Layout::normalize)
        .unwrap_or_default()
}

fn save(l: &Layout) {
    if let (Some(s), Ok(j)) = (
        window().local_storage().ok().flatten(),
        serde_json::to_string(l),
    ) {
        let _ = s.set_item(STORE, &j);
    }
}

/// Rectangle (pixels, relatif à l'espace de travail).
#[derive(Clone, Copy, PartialEq, Debug)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Onglet en cours de déplacement.
#[derive(Clone, Copy, PartialEq, Debug)]
struct TabDrag {
    panel: PanelId,
    start: (f64, f64),
    pos: (f64, f64),
    active: bool,
}

/// État partagé de l'espace de travail.
#[derive(Clone, Copy)]
pub struct Dock {
    pub layout: RwSignal<Layout>,
    drag: RwSignal<Option<TabDrag>>,
    hint: RwSignal<Option<(Rect, Drop)>>,
    /// Menu de groupe ouvert.
    menu: RwSignal<Option<u32>>,
    root: NodeRef<leptos::html::Div>,
}

impl Dock {
    fn origin(self) -> (f64, f64) {
        self.root
            .get_untracked()
            .map(|r| {
                let b = r.get_bounding_client_rect();
                (b.left(), b.top())
            })
            .unwrap_or_default()
    }

    fn rel(self, r: &web_sys::DomRect) -> Rect {
        let (ox, oy) = self.origin();
        Rect {
            x: r.left() - ox,
            y: r.top() - oy,
            w: r.width(),
            h: r.height(),
        }
    }

    /// Dépôt possible sous le pointeur.
    fn target(self, x: f64, y: f64) -> Option<(Rect, Drop)> {
        let el = document().element_from_point(x as f32, y as f32)?;
        self.target_on(el, x, y)
    }

    /// Dépôt possible sous le pointeur, sans tenir compte d'un élément (fenêtre déplacée).
    fn target_except(self, x: f64, y: f64, skip: &web_sys::Element) -> Option<(Rect, Drop)> {
        let list = document().elements_from_point(x as f32, y as f32);
        let el = list
            .iter()
            .filter_map(|v| v.dyn_into::<web_sys::Element>().ok())
            .find(|e| !skip.contains(Some(e)))?;
        self.target_on(el, x, y)
    }

    fn target_on(self, el: web_sys::Element, x: f64, y: f64) -> Option<(Rect, Drop)> {
        if let Some(strip) = el.closest("[data-tabs]").ok().flatten() {
            let group: u32 = strip.get_attribute("data-tabs")?.parse().ok()?;
            let tabs = strip.query_selector_all(".dock-tab").ok()?;
            let mut index = 0;
            for i in 0..tabs.length() {
                if let Some(t) = tabs
                    .item(i)
                    .and_then(|n| n.dyn_into::<web_sys::Element>().ok())
                {
                    let r = t.get_bounding_client_rect();
                    if r.left() + r.width() / 2.0 < x {
                        index = i as usize + 1;
                    }
                }
            }
            return Some((
                self.rel(&strip.get_bounding_client_rect()),
                Drop::Join { group, index },
            ));
        }
        if let Some(body) = el.closest("[data-body]").ok().flatten() {
            let group: u32 = body.get_attribute("data-body")?.parse().ok()?;
            let r = self.rel(&body.get_bounding_client_rect());
            let (_, oy) = self.origin();
            let f = (y - oy - r.y) / r.h.max(1.0);
            return Some(if f < 0.25 {
                (
                    Rect { h: r.h / 2.0, ..r },
                    Drop::Split {
                        group,
                        after: false,
                    },
                )
            } else if f > 0.75 {
                (
                    Rect {
                        y: r.y + r.h / 2.0,
                        h: r.h / 2.0,
                        ..r
                    },
                    Drop::Split { group, after: true },
                )
            } else {
                (
                    r,
                    Drop::Join {
                        group,
                        index: usize::MAX,
                    },
                )
            });
        }
        if let Some(col) = el.closest("[data-col]").ok().flatten() {
            let side = if col.get_attribute("data-col")? == "left" {
                Side::Left
            } else {
                Side::Right
            };
            return Some((
                self.rel(&col.get_bounding_client_rect()),
                Drop::Column { side },
            ));
        }
        if el.closest(".canvas-area").ok().flatten().is_some() {
            let (ox, oy) = self.origin();
            let (fx, fy) = (x - ox - 40.0, y - oy - 14.0);
            return Some((
                Rect {
                    x: fx,
                    y: fy,
                    w: 300.0,
                    h: 420.0,
                },
                Drop::Float { x: fx, y: fy },
            ));
        }
        None
    }
}

fn capture(e: &ev::PointerEvent) {
    if let Some(t) = e
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
        let _ = t.set_pointer_capture(e.pointer_id());
    }
}

fn dragging_cursor(cursor: Option<&str>) {
    if let Some(root) = document().document_element() {
        match cursor {
            Some(c) => {
                let _ = root.set_attribute("data-dock-drag", c);
            }
            None => {
                let _ = root.remove_attribute("data-dock-drag");
            }
        }
    }
}

/// Menu contextuel d'un panneau (onglet, bandeau de son groupe) : détacher en fenêtre ou ancrer
/// une fenêtre, fermer.
fn tab_menu(dock: Dock, ui: Ui, panel: PanelId, e: &ev::MouseEvent) {
    use crate::ctxmenu::{CtxItem, open};
    let floating = dock
        .layout
        .with_untracked(|l| l.group_of(panel).is_some_and(|g| l.side_of(g).is_none()));
    let (ox, oy) = dock.origin();
    let (fx, fy) = (
        f64::from(e.client_x()) - ox - 40.0,
        f64::from(e.client_y()) - oy - 14.0,
    );
    let place = if floating {
        CtxItem::new(ui.now("dock-dock", &[]), move || {
            dock.layout.update(|l| {
                l.put(panel, Drop::Column { side: panel.home() });
            })
        })
    } else {
        CtxItem::new(ui.now("dock-float", &[]), move || {
            dock.layout.update(|l| {
                l.put(panel, Drop::Float { x: fx, y: fy });
            })
        })
    };
    let items = vec![
        place.icon(icons::external),
        CtxItem::new(ui.now("dock-close-short", &[]), move || {
            dock.layout.update(|l| l.close(panel))
        })
        .icon(icons::close),
    ];
    open(ui, e, items);
}

/// Onglet d'un panneau : clic = afficher ; glisser = déplacer.
#[component]
fn Tab(dock: Dock, group: u32, panel: PanelId) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let on = move || {
        dock.layout
            .with(|l| l.group(group).is_some_and(|g| g.active == panel))
    };
    let me = NodeRef::<leptos::html::Button>::new();
    Effect::new(move |_| {
        if on()
            && let Some(b) = me.get()
        {
            let opts = web_sys::ScrollIntoViewOptions::new();
            opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
            opts.set_inline(web_sys::ScrollLogicalPosition::Nearest);
            b.scroll_into_view_with_scroll_into_view_options(&opts);
        }
    });
    view! {
        <button class="dock-tab" node_ref=me class:is-on=on role="tab" aria-selected=move || on().to_string()
            data-panel=panel.name() title=ui.t(panel.key())
            on:pointerdown=move |e: ev::PointerEvent| {
                if e.button() != 0 { return; }
                // Seul onglet d'une fenêtre : le glisser déplace la fenêtre (voir `FloatView`).
                if dock.layout.with_untracked(|l| l.floats.iter().any(|f| f.group.id == group && f.group.tabs.len() == 1)) {
                    return;
                }
                capture(&e);
                let p = (f64::from(e.client_x()), f64::from(e.client_y()));
                dock.drag.set(Some(TabDrag { panel, start: p, pos: p, active: false }));
            }
            on:contextmenu=move |e: ev::MouseEvent| tab_menu(dock, ui, panel, &e)
            on:pointermove=move |e: ev::PointerEvent| {
                let Some(mut d) = dock.drag.get_untracked() else { return };
                let p = (f64::from(e.client_x()), f64::from(e.client_y()));
                if !d.active && (p.0 - d.start.0).abs() + (p.1 - d.start.1).abs() < 6.0 { return; }
                if !d.active { dragging_cursor(Some("grabbing")); }
                d.active = true;
                d.pos = p;
                dock.drag.set(Some(d));
                dock.hint.set(dock.target(p.0, p.1));
            }
            on:pointerup=move |_| {
                let d = dock.drag.get_untracked();
                let hint = dock.hint.get_untracked();
                dock.drag.set(None);
                dock.hint.set(None);
                dragging_cursor(None);
                match d {
                    Some(d) if d.active => {
                        if let Some((_, drop)) = hint {
                            dock.layout.update(|l| { l.put(d.panel, drop); });
                        }
                    }
                    Some(_) => dock.layout.update(|l| l.activate(panel)),
                    None => {}
                }
            }>
            <span class="dock-tab-label">{ui.t(panel.key())}</span>
        </button>
    }
}

/// Groupe d'onglets : barre d'onglets, menu, panneaux (tous montés, l'actif visible : l'état de
/// chaque panneau est gardé d'un onglet à l'autre).
#[component]
fn GroupView(
    dock: Dock,
    id: u32,
    tabs: Vec<PanelId>,
    /// Poignée de redimensionnement au-dessus (groupe qui n'est pas le premier de sa colonne).
    #[prop(optional)]
    side: Option<Side>,
    #[prop(optional)] floating: bool,
    panel: fn(PanelId) -> AnyView,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let active = move || dock.layout.with(|l| l.group(id).map(|g| g.active));
    let first = move || {
        side.is_none_or(|s| {
            dock.layout.with(|l| match s {
                Side::Left => l.left.first().map(|g| g.id) == Some(id),
                Side::Right => l.right.first().map(|g| g.id) == Some(id),
            })
        })
    };
    let size = move || {
        dock.layout
            .with(|l| l.group(id).map(|g| g.size).unwrap_or(1.0))
    };
    // Redimensionnement avec le groupe précédent de la colonne.
    let split = StoredValue::new(None::<(f64, f64, f64, f64, f64)>);
    let me = NodeRef::<leptos::html::Section>::new();
    let tabs2 = tabs.clone();
    let tabs3 = tabs.clone();
    view! {
        <section class="dock-group" class:is-floating=floating node_ref=me
            style:flex-grow=move || size().to_string() data-group=id>
            {move || (!floating && !first()).then(|| view! {
                <div class="dock-split" role="separator" aria-orientation="horizontal"
                    on:pointerdown=move |e: ev::PointerEvent| {
                        capture(&e);
                        let Some(el) = me.get_untracked() else { return };
                        let Some(prev) = el.previous_element_sibling() else { return };
                        let (hp, hc) = (prev.get_bounding_client_rect().height(), el.get_bounding_client_rect().height());
                        let (sp, sc) = dock.layout.with_untracked(|l| {
                            let col = match side { Some(Side::Left) => &l.left, _ => &l.right };
                            let i = col.iter().position(|g| g.id == id).unwrap_or(0);
                            (col.get(i.wrapping_sub(1)).map(|g| g.size).unwrap_or(1.0), col.get(i).map(|g| g.size).unwrap_or(1.0))
                        });
                        split.set_value(Some((f64::from(e.client_y()), hp, hc, sp, sc)));
                        dragging_cursor(Some("row-resize"));
                    }
                    on:pointermove=move |e: ev::PointerEvent| {
                        let Some((y0, hp, hc, sp, sc)) = split.get_value() else { return };
                        let dy = f64::from(e.client_y()) - y0;
                        let total = hp + hc;
                        let np = (hp + dy).clamp(72.0, total - 72.0);
                        let weight = sp + sc;
                        dock.layout.update(|l| {
                            let col = match side { Some(Side::Left) => &mut l.left, _ => &mut l.right };
                            if let Some(i) = col.iter().position(|g| g.id == id) && i > 0 {
                                col[i - 1].size = weight * np / total;
                                col[i].size = weight * (total - np) / total;
                            }
                        });
                    }
                    on:pointerup=move |_| { split.set_value(None); dragging_cursor(None); }></div>
            })}
            <header class="dock-head" on:contextmenu=move |e: ev::MouseEvent| {
                if let Some(p) = active() { tab_menu(dock, ui, p, &e) }
            }>
                <div class="dock-tabs" role="tablist" data-tabs=id
                    on:wheel=move |e: ev::WheelEvent| {
                        // Molette : défilement horizontal des onglets qui ne tiennent pas.
                        if let Some(t) = e.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                            t.set_scroll_left(t.scroll_left() + (e.delta_y() + e.delta_x()) as i32);
                        }
                    }>
                    {tabs.iter().map(|p| view! { <Tab dock group=id panel=*p/> }).collect_view()}
                </div>
                <div class="dock-menu-wrap">
                    <button class="icon-btn icon-btn--small dock-menu-btn" title=ui.t("dock-menu") aria-label=ui.t("dock-menu")
                        on:click=move |_| dock.menu.update(|m| *m = if *m == Some(id) { None } else { Some(id) })>
                        {icons::more()}
                    </button>
                    <Show when=move || dock.menu.get() == Some(id)>
                        <div class="dock-menu pop" role="menu">
                            {tabs3.iter().map(|p| {
                                let p = *p;
                                view! {
                                    <button class="menu-item menu-item--check" role="menuitemradio"
                                        aria-checked=move || (active() == Some(p)).to_string()
                                        on:click=move |_| { dock.menu.set(None); dock.layout.update(|l| l.activate(p)); }>
                                        <span class="menu-check">{move || (active() == Some(p)).then(icons::check)}</span>
                                        <span>{ui.t(p.key())}</span>
                                    </button>
                                }
                            }).collect_view()}
                            <hr class="menu-sep"/>
                            {move || active().map(|p| view! {
                                <button class="menu-item" role="menuitem" on:click=move |_| {
                                    dock.menu.set(None);
                                    dock.layout.update(|l| {
                                        let (x, y) = (120.0, 60.0);
                                        l.put(p, Drop::Float { x, y });
                                    });
                                }>{ui.t("dock-float")}</button>
                                <button class="menu-item" role="menuitem" on:click=move |_| {
                                    dock.menu.set(None);
                                    dock.layout.update(|l| l.close(p));
                                }>{move || ui.tx("dock-close", &[("panel", ui.tx(p.key(), &[]))])}</button>
                            })}
                        </div>
                    </Show>
                </div>
            </header>
            <div class="dock-body" data-body=id>
                {tabs2.into_iter().map(|p| view! {
                    <div class="dock-panel" data-panel=p.name() class:is-on=move || active() == Some(p)
                        role="tabpanel" aria-label=ui.t(p.key())>
                        {panel(p)}
                    </div>
                }).collect_view()}
            </div>
        </section>
    }
}

/// Colonne de groupes ; poignée de largeur côté canevas.
#[component]
fn Column(dock: Dock, side: Side, panel: fn(PanelId) -> AnyView) -> impl IntoView {
    let groups = move || {
        dock.layout.with(|l| {
            match side {
                Side::Left => &l.left,
                Side::Right => &l.right,
            }
            .iter()
            .map(|g| (g.id, g.tabs.clone()))
            .collect::<Vec<_>>()
        })
    };
    let width = move || {
        dock.layout.with(|l| {
            let (w, empty) = match side {
                Side::Left => (l.left_w, l.left.is_empty()),
                Side::Right => (l.right_w, l.right.is_empty()),
            };
            if l.hidden || empty { 0.0 } else { w }
        })
    };
    let resize = StoredValue::new(None::<(f64, f64)>);
    view! {
        <div class=move || format!("dock-col dock-col--{}", if side == Side::Left { "left" } else { "right" })
            class:is-empty=move || width() == 0.0
            data-col=if side == Side::Left { "left" } else { "right" }
            style:width=move || format!("{}px", width())>
            <For each=groups key=|(id, tabs)| (*id, tabs.clone()) let:g>
                <GroupView dock id=g.0 tabs=g.1 side=side panel/>
            </For>
            <div class="dock-resize" role="separator" aria-orientation="vertical"
                on:pointerdown=move |e: ev::PointerEvent| {
                    capture(&e);
                    resize.set_value(Some((f64::from(e.client_x()), width())));
                    dragging_cursor(Some("col-resize"));
                }
                on:pointermove=move |e: ev::PointerEvent| {
                    let Some((x0, w0)) = resize.get_value() else { return };
                    let dx = f64::from(e.client_x()) - x0;
                    let w = (if side == Side::Left { w0 + dx } else { w0 - dx }).clamp(MIN_COL, MAX_COL);
                    dock.layout.update(|l| match side { Side::Left => l.left_w = w, Side::Right => l.right_w = w });
                }
                on:pointerup=move |_| { resize.set_value(None); dragging_cursor(None); }></div>
        </div>
    }
}

/// Fenêtre flottante : barre d'onglets à glisser pour la déplacer, poignée de taille.
#[component]
fn FloatView(
    dock: Dock,
    id: u32,
    tabs: Vec<PanelId>,
    panel: fn(PanelId) -> AnyView,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let geom = move || {
        dock.layout.with(|l| {
            l.floats
                .iter()
                .find(|f| f.group.id == id)
                .map(|f| (f.x, f.y, f.w, f.h))
                .unwrap_or_default()
        })
    };
    let moving = StoredValue::new(None::<(f64, f64, f64, f64)>);
    let sizing = StoredValue::new(None::<(f64, f64, f64, f64)>);
    let me = NodeRef::<leptos::html::Div>::new();
    let update = move |f: &dyn Fn(&mut Float)| {
        dock.layout.update(|l| {
            if let Some(x) = l.floats.iter_mut().find(|x| x.group.id == id) {
                f(x);
            }
        })
    };
    // Fenêtre déplacée par tout son bandeau (poignée, barre d'onglets ; un onglet seul compris) ;
    // Maj maintenue : emplacement d'ancrage montré sous le pointeur, la fenêtre s'y range au
    // relâchement.
    let start_move = move |e: &ev::PointerEvent| {
        if e.button() != 0 {
            return;
        }
        let Some(t) = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        else {
            return;
        };
        let closest = |sel: &str| t.closest(sel).ok().flatten().is_some();
        let single = dock.layout.with_untracked(|l| {
            l.floats
                .iter()
                .any(|f| f.group.id == id && f.group.tabs.len() == 1)
        });
        if !closest(".dock-float-grip, .dock-head")
            || closest(".dock-menu-wrap")
            || (closest(".dock-tab") && !single)
        {
            return;
        }
        e.prevent_default();
        capture(e);
        let (x, y, _, _) = geom();
        moving.set_value(Some((
            f64::from(e.client_x()),
            f64::from(e.client_y()),
            x,
            y,
        )));
        dragging_cursor(Some("move"));
    };
    let move_to = move |e: &ev::PointerEvent| {
        let Some((x0, y0, fx, fy)) = moving.get_value() else {
            return;
        };
        let (cx, cy) = (f64::from(e.client_x()), f64::from(e.client_y()));
        let (nx, ny) = ((fx + cx - x0).max(0.0), (fy + cy - y0).max(0.0));
        update(&|f| {
            f.x = nx;
            f.y = ny;
        });
        let hint = if e.shift_key() {
            me.get_untracked()
                .and_then(|el| dock.target_except(cx, cy, &el))
                .filter(|(_, d)| !matches!(d, Drop::Float { .. }))
        } else {
            None
        };
        dock.hint.set(hint);
    };
    let end_move = move |e: &ev::PointerEvent| {
        if moving.get_value().is_none() {
            return;
        }
        moving.set_value(None);
        dragging_cursor(None);
        let hint = dock.hint.get_untracked();
        dock.hint.set(None);
        if let (true, Some((_, drop))) = (e.shift_key(), hint) {
            dock.layout.update(|l| {
                let Some(f) = l.floats.iter().find(|f| f.group.id == id).cloned() else {
                    return;
                };
                let mut tabs = f.group.tabs.into_iter();
                let Some(first) = tabs.next() else { return };
                l.put(first, drop);
                if let Some(g) = l.group_of(first) {
                    for t in tabs {
                        l.put(
                            t,
                            Drop::Join {
                                group: g,
                                index: usize::MAX,
                            },
                        );
                    }
                }
                l.activate(f.group.active);
            });
        }
    };
    view! {
        <div class="dock-float" node_ref=me class:is-hidden=move || dock.layout.with(|l| l.hidden)
            style=move || { let (x, y, w, h) = geom(); format!("left:{x}px;top:{y}px;width:{w}px;height:{h}px") }
            on:pointerdown=move |e: ev::PointerEvent| start_move(&e)
            on:pointermove=move |e: ev::PointerEvent| move_to(&e)
            on:pointerup=move |e: ev::PointerEvent| end_move(&e)
            on:pointercancel=move |_| { moving.set_value(None); dock.hint.set(None); dragging_cursor(None); }>
            <div class="dock-float-grip" title=ui.t("dock-move")></div>
            <GroupView dock id tabs floating=true panel/>
            <div class="dock-float-size"
                on:pointerdown=move |e: ev::PointerEvent| {
                    capture(&e);
                    let (_, _, w, h) = geom();
                    sizing.set_value(Some((f64::from(e.client_x()), f64::from(e.client_y()), w, h)));
                    dragging_cursor(Some("nwse-resize"));
                }
                on:pointermove=move |e: ev::PointerEvent| {
                    let Some((x0, y0, w0, h0)) = sizing.get_value() else { return };
                    let (nw, nh) = ((w0 + f64::from(e.client_x()) - x0).max(220.0), (h0 + f64::from(e.client_y()) - y0).max(160.0));
                    update(&|f| { f.w = nw; f.h = nh; });
                }
                on:pointerup=move |_| { sizing.set_value(None); dragging_cursor(None); }></div>
        </div>
    }
}

impl Dock {
    /// Espace de travail de l'éditeur (disposition relue), fourni comme contexte (barre d'outils
    /// et colonnes).
    pub fn provide() -> Self {
        let dock = Dock {
            layout: RwSignal::new(load()),
            drag: RwSignal::new(None),
            hint: RwSignal::new(None),
            menu: RwSignal::new(None),
            root: NodeRef::new(),
        };
        provide_context(dock);
        Effect::new(move |_| dock.layout.with(save));
        dock
    }
}

/// Espace de travail : colonnes de panneaux autour du canevas (`children`).
#[component]
pub fn Workspace(panel: fn(PanelId) -> AnyView, children: Children) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let dock = expect_context::<Dock>();
    // Menus fermés par un clic ailleurs ; Ctrl+\ masque ou affiche les panneaux.
    let on_down = window_event_listener(ev::pointerdown, move |e: web_sys::PointerEvent| {
        if dock.menu.get_untracked().is_none() {
            return;
        }
        let inside = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest(".dock-menu-wrap").ok().flatten())
            .is_some();
        if !inside {
            dock.menu.set(None);
        }
    });
    let on_key = window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if (e.ctrl_key() || e.meta_key()) && e.key() == "\\" {
            e.prevent_default();
            dock.layout.update(|l| l.hidden = !l.hidden);
        }
    });
    let on_blur = window_event_listener(ev::blur, move |_| dock.menu.set(None));
    on_cleanup(move || {
        on_down.remove();
        on_key.remove();
        on_blur.remove();
    });
    let floats = move || {
        dock.layout.with(|l| {
            l.floats
                .iter()
                .map(|f| (f.group.id, f.group.tabs.clone()))
                .collect::<Vec<_>>()
        })
    };
    view! {
        <div class="workspace" node_ref=dock.root class:is-dragging=move || dock.drag.with(|d| d.is_some_and(|d| d.active))>
            <Column dock side=Side::Left panel/>
            {children()}
            <Column dock side=Side::Right panel/>
            <For each=floats key=|(id, tabs)| (*id, tabs.clone()) let:f>
                <FloatView dock id=f.0 tabs=f.1 panel/>
            </For>
            {move || dock.hint.get().map(|(r, _)| view! {
                <div class="dock-hint" style=format!("left:{}px;top:{}px;width:{}px;height:{}px", r.x, r.y, r.w, r.h)></div>
            })}
            {move || dock.drag.get().filter(|d| d.active).map(|d| {
                let (ox, oy) = dock.origin();
                view! {
                    <div class="dock-ghost" style=format!("left:{}px;top:{}px", d.pos.0 - ox + 10.0, d.pos.1 - oy + 10.0)>
                        <span>{ui.t(d.panel.key())}</span>
                    </div>
                }
            })}
        </div>
    }
}

/// Menu « Panneaux » de la barre d'outils : panneaux ouverts ou fermés, masquer tout, disposition
/// par défaut.
#[component]
pub fn PanelsMenu() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let dock = expect_context::<Dock>();
    view! {
        <details class="bp-menu panels-menu">
            <summary class="icon-btn" title=ui.t("dock-panels") aria-label=ui.t("dock-panels")>{icons::layout()}</summary>
            <div class="bp-pop bp-pop--right panels-pop">
                {PanelId::ALL.into_iter().map(|p| {
                    let open = move || dock.layout.with(|l| l.group_of(p).is_some());
                    view! {
                        <button class="menu-item menu-item--check" class:is-on=open role="menuitemcheckbox"
                            aria-checked=move || open().to_string()
                            on:click=move |_| dock.layout.update(|l| if l.group_of(p).is_some() { l.close(p) } else { l.open(p) })>
                            <span class="menu-check">{move || open().then(icons::check)}</span>
                            <span>{ui.t(p.key())}</span>
                        </button>
                    }
                }).collect_view()}
                <hr class="menu-sep"/>
                <button class="menu-item" on:click=move |_| dock.layout.update(|l| l.hidden = !l.hidden)>
                    <span class="menu-check"></span>
                    <span>{move || ui.tx(if dock.layout.with(|l| l.hidden) { "dock-show-all" } else { "dock-hide-all" }, &[])}</span>
                    <kbd>"Ctrl+\\"</kbd>
                </button>
                <button class="menu-item" on:click=move |_| dock.layout.set(Layout::default())>
                    <span class="menu-check"></span>
                    <span>{ui.t("dock-reset")}</span>
                </button>
            </div>
        </details>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_once(l: &Layout) -> bool {
        PanelId::ALL.iter().all(|p| {
            l.groups().filter(|g| g.tabs.contains(p)).count() + usize::from(l.closed.contains(p))
                == 1
        })
    }

    #[test]
    fn deplacements() {
        let mut l = Layout::default();
        assert!(all_once(&l));
        // Styles rejoint le groupe des calques, en deuxième position.
        assert!(l.put(PanelId::Styles, Drop::Join { group: 2, index: 1 }));
        assert_eq!(l.group(2).unwrap().tabs[1], PanelId::Styles);
        assert_eq!(l.group(2).unwrap().active, PanelId::Styles);
        // Réordonner dans le même groupe.
        l.put(PanelId::Styles, Drop::Join { group: 2, index: 0 });
        assert_eq!(l.group(2).unwrap().tabs[0], PanelId::Styles);
        // Pages, resté seul dans son groupe, sous les calques : le groupe 1 disparaît.
        l.put(PanelId::Components, Drop::Join { group: 2, index: 9 });
        l.put(PanelId::Assets, Drop::Join { group: 2, index: 9 });
        assert_eq!(l.group(1).unwrap().tabs, vec![PanelId::Pages]);
        l.put(
            PanelId::Pages,
            Drop::Split {
                group: 2,
                after: true,
            },
        );
        assert!(l.group(1).is_none());
        assert_eq!(l.left.len(), 2);
        assert_eq!(l.left[1].tabs, vec![PanelId::Pages]);
        // Déposer un panneau seul sur son propre groupe : rien.
        let id = l.group_of(PanelId::Pages).unwrap();
        assert!(!l.put(
            PanelId::Pages,
            Drop::Split {
                group: id,
                after: false
            }
        ));
        // Fenêtre flottante, puis retour dans une colonne.
        l.put(PanelId::Settings, Drop::Float { x: 10.0, y: 20.0 });
        assert_eq!(l.floats.len(), 1);
        l.put(PanelId::Settings, Drop::Column { side: Side::Left });
        assert!(l.floats.is_empty());
        assert!(all_once(&l));
        // La colonne de droite peut se vider.
        l.put(PanelId::Interactions, Drop::Column { side: Side::Left });
        assert!(l.right.is_empty());
        assert!(all_once(&l));
    }

    #[test]
    fn fermer_rouvrir_et_relire() {
        let mut l = Layout::default();
        assert!(l.group_of(PanelId::Cms).is_none());
        l.close(PanelId::Insert);
        assert!(l.group_of(PanelId::Insert).is_none());
        assert!(all_once(&l));
        l.open(PanelId::Insert);
        l.open(PanelId::Cms);
        assert!(l.group_of(PanelId::Insert).is_some() && l.group_of(PanelId::Cms).is_some());
        assert!(all_once(&l));
        // Rouvert à sa place : Pages en tête du groupe des composants et médias.
        let mut r = Layout::default();
        r.close(PanelId::Pages);
        r.open(PanelId::Pages);
        assert_eq!(r.left[0].tabs[0], PanelId::Pages);
        assert_eq!(r.left[1].active, PanelId::Layers);
        // Disposition d'une version sans « Variables » : ajouté à côté des calques.
        let mut old = Layout::default();
        old.left[1].tabs = vec![PanelId::Layers, PanelId::Cms, PanelId::Insert];
        old.closed.clear();
        let n = old.normalize();
        assert_eq!(
            n.left[1].tabs,
            vec![
                PanelId::Layers,
                PanelId::Variables,
                PanelId::Cms,
                PanelId::Insert
            ]
        );
        assert_eq!(n.left[1].active, PanelId::Layers);
        // Disposition abîmée : doublons retirés, panneaux manquants ajoutés, largeurs bornées.
        let mut bad = Layout::default();
        bad.left[0].tabs.push(PanelId::Layers);
        bad.right[0].tabs.retain(|t| *t != PanelId::Settings);
        bad.left_w = 5000.0;
        let n = bad.normalize();
        assert!(all_once(&n));
        assert_eq!(n.left_w, MAX_COL);
        let json = serde_json::to_string(&n).unwrap();
        assert_eq!(serde_json::from_str::<Layout>(&json).unwrap(), n);
    }
}
