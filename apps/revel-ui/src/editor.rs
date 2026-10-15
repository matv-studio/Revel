//! Éditeur visuel (étape 2) : canevas, points de rupture, zoom, sélection, annuler/rétablir.
//!
//! Le document du site est tenu en copie locale : chaque modification passe par une commande
//! envoyée au cœur (D-13) ; la commande effectivement appliquée (`EditState::applied`) est rejouée
//! sur la copie. Le canevas est un cadre d'une autre origine (protocole `revel-canvas`) qui
//! dialogue par `postMessage` (voir `crates/revel-core/src/canvas.js`).

use crate::{Ui, api, icons};
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::{Command, EditState};
use revel_model::Site;
use revel_protocol::{EditorPageInfo, SiteSummary};
use serde::Deserialize;
use wasm_bindgen::JsValue;

/// Élément sélectionné dans le canevas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub id: String,
    pub tag: String,
    pub label: String,
}

/// Message reçu du canevas.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum FromCanvas {
    Ready {
        path: String,
    },
    Hover {
        id: Option<String>,
    },
    Select {
        id: Option<String>,
        #[serde(default)]
        tag: String,
        #[serde(default)]
        label: String,
    },
    Key {
        key: String,
        #[serde(default)]
        code: String,
        #[serde(default)]
        shift: bool,
        #[serde(default)]
        alt: bool,
        #[serde(default)]
        ctrl: bool,
        #[serde(default)]
        meta: bool,
    },
    /// Touche relâchée (espace : fin du déplacement de la vue).
    Keyup {
        key: String,
    },
    Scroll {
        x: f64,
        y: f64,
    },
    Computed {
        values: std::collections::HashMap<String, String>,
    },
    /// Molette avec Ctrl (zoom) : déplacement vertical, position du pointeur dans le cadre.
    Zoom {
        dy: f64,
        #[serde(default)]
        x: Option<f64>,
        #[serde(default)]
        y: Option<f64>,
    },
    /// Déplacement de la vue (bouton du milieu dans le canevas), en pixels de l'écran.
    Pan {
        dx: f64,
        dy: f64,
    },
    /// Emplacement de dépôt sous le pointeur pendant un glisser (`None` : aucun).
    Drop {
        target: Option<DropTarget>,
    },
    /// Élément de la page glissé à un autre endroit (`copy` : Alt, copie).
    Moved {
        id: String,
        target: DropTarget,
        #[serde(default)]
        copy: bool,
    },
    /// Clic droit dans la page : position dans le cadre.
    Context {
        x: f64,
        y: f64,
    },
    /// Position de la sélection dans le cadre (zoom sur la sélection).
    Measure {
        rect: Option<Rect>,
    },
    /// Nombre d'éléments de la page qui portent un sélecteur.
    Count {
        selector: String,
        n: u32,
    },
    /// Texte modifié sur place (étape 2.7).
    Text {
        slot: revel_edit::canvas::Slot,
        text: String,
    },
    #[serde(other)]
    Other,
}

/// Emplacement de dépôt : avant ou après l'élément `at`, ou dedans (à la fin).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DropTarget {
    #[serde(rename = "ref")]
    pub at: String,
    #[serde(rename = "where")]
    pub place: Place,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    Before,
    After,
    Inside,
}

/// Rectangle dans le cadre du canevas (pixels CSS de la page).
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Élément glissé depuis le panneau « Ajouter ».
#[derive(Clone, Debug, PartialEq)]
pub struct DragItem {
    pub preset: revel_edit::make::Preset,
    /// Texte par défaut (clé de traduction, vide si aucun).
    pub text_key: &'static str,
    pub label: String,
    /// Balise produite et nature (bloc ou en ligne) : règles de dépôt du canevas.
    pub tag: &'static str,
    pub block: bool,
}

/// Touche (interface ou canevas).
#[derive(Clone, Debug, Default)]
pub struct Key {
    pub key: String,
    /// Touche physique (`Digit1`) : raccourcis à chiffres indépendants de la disposition du clavier.
    pub code: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Key {
    pub fn of(e: &web_sys::KeyboardEvent) -> Self {
        Self {
            key: e.key(),
            code: e.code(),
            ctrl: e.ctrl_key() || e.meta_key(),
            shift: e.shift_key(),
            alt: e.alt_key(),
        }
    }
    /// Chiffre de la rangée supérieure ou du pavé numérique (`Digit0`, `Numpad0`).
    fn digit(&self) -> Option<u8> {
        let c = self
            .code
            .strip_prefix("Digit")
            .or_else(|| self.code.strip_prefix("Numpad"))?;
        c.parse().ok()
    }
}

/// Géométrie du canevas : cadre (largeur, hauteur en pixels CSS), zoom ajusté, échelle et position
/// du cadre dans la zone (pixels de l'écran).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geo {
    pub w: f64,
    pub h: f64,
    pub fit: f64,
    pub s: f64,
    pub x: f64,
    pub y: f64,
}

/// Marge autour du cadre (zoom ajusté).
pub const GUTTER: f64 = 48.0;

/// État de l'éditeur, partagé par ses panneaux.
#[derive(Clone, Copy)]
pub struct Ed {
    pub path: StoredValue<String>,
    /// Copie locale du document.
    pub doc: RwSignal<Option<Site>>,
    pub state: RwSignal<Option<EditState>>,
    /// Adresse de base du canevas (`revel-canvas://localhost/`).
    pub base: RwSignal<Option<String>>,
    pub pages: RwSignal<Vec<EditorPageInfo>>,
    /// Page affichée (`/agence/`).
    pub page: RwSignal<String>,
    /// Point de rupture affiché (identifiant).
    pub breakpoint: RwSignal<String>,
    /// Zoom ; `None` : ajusté à la place disponible.
    pub zoom: RwSignal<Option<f64>>,
    /// Position du cadre dans la zone du canevas (zoom libre).
    pub pan: RwSignal<(f64, f64)>,
    /// Taille de la zone du canevas.
    pub area: RwSignal<(f64, f64)>,
    /// Espace maintenue : la vue se déplace à la souris.
    pub space: RwSignal<bool>,
    /// Déplacement de la vue en cours (espace ou bouton du milieu).
    pub panning: RwSignal<bool>,
    /// Glisser depuis le panneau « Ajouter » : élément et position du pointeur.
    pub drag: RwSignal<Option<DragItem>>,
    pub drag_pos: RwSignal<(f64, f64)>,
    /// Emplacement de dépôt proposé par le canevas.
    pub drop: RwSignal<Option<DropTarget>>,
    /// Menu contextuel du canevas ouvert (position dans la fenêtre).
    pub menu: RwSignal<Option<(f64, f64)>>,
    /// Échelle du zoom ajusté (affichée dans la liste des zooms).
    pub fit: RwSignal<f64>,
    pub selected: RwSignal<Option<Selection>>,
    pub hovered: RwSignal<Option<String>>,
    /// Exécuter les scripts du site dans le canevas.
    pub scripts: RwSignal<bool>,
    /// Numéro de chargement du canevas (incrémenté pour le recharger).
    pub load: RwSignal<u64>,
    /// Défilement du canevas, rétabli après un rechargement.
    pub scroll: RwSignal<(f64, f64)>,
    /// Valeurs effectives des propriétés de style de l'élément sélectionné (canevas).
    pub computed: RwSignal<std::collections::HashMap<String, String>>,
    /// Nombre d'éléments de la page qui portent la classe réglée (sélecteur, nombre).
    pub count: RwSignal<Option<(String, u32)>>,
    /// Modifications envoyées au cœur et pas encore confirmées.
    pub pending: RwSignal<u32>,
    /// Heure (ms) du dernier enregistrement confirmé.
    pub saved_at: RwSignal<Option<f64>>,
    /// Heure de la dernière modification faite dans Revel (ms) : les fichiers changés juste après
    /// ne sont pas pris pour une modification extérieure.
    pub own_change: RwSignal<f64>,
    pub frame: NodeRef<html::Iframe>,
}

/// Paliers de zoom.
pub const ZOOMS: [f64; 9] = [0.25, 0.33, 0.5, 0.67, 0.75, 1.0, 1.25, 1.5, 2.0];

/// Commande qui ne touche que la feuille de style (le canevas recharge alors la feuille seule).
pub fn style_only(c: &Command) -> bool {
    match c {
        Command::SetDecl { .. }
        | Command::InsertDecl { .. }
        | Command::RemoveDeclAt { .. }
        | Command::SetSelector { .. }
        | Command::InsertStyle { .. }
        | Command::RemoveStyleAt { .. }
        | Command::RemoveRule { .. }
        | Command::AddRule { .. } => true,
        Command::Batch { commands } => !commands.is_empty() && commands.iter().all(style_only),
        _ => false,
    }
}

impl Ed {
    /// Message au canevas.
    pub fn post(self, msg: serde_json::Value) {
        let Some(frame) = self.frame.get_untracked() else {
            return;
        };
        let Some(win) = frame.content_window() else {
            return;
        };
        let mut msg = msg;
        if let Some(o) = msg.as_object_mut() {
            o.insert("revel".into(), 1.into());
        }
        if let Ok(v) = js_sys::JSON::parse(&msg.to_string()) {
            let _ = win.post_message(&v, "*");
        }
    }

    /// Adresse du canevas pour la page affichée.
    pub fn src(self) -> Option<String> {
        let base = self.base.get()?;
        let page = self.page.get();
        Some(format!(
            "{}{}?rv={}",
            base,
            page.trim_start_matches('/'),
            self.load.get()
        ))
    }

    /// Après une modification, une annulation ou un rétablissement : copie locale et canevas.
    pub fn after(self, ui: Ui, st: EditState) {
        if let Some(cmd) = &st.applied {
            self.own_change.set(js_sys::Date::now());
            // Valeur liée : écrite dans un fichier par le cœur ; variable de page ou de gabarit :
            // document relu.
            if let Command::SetBound { binding, .. } = cmd {
                if matches!(
                    binding.source,
                    revel_edit::canvas::Source::Page { .. }
                        | revel_edit::canvas::Source::Layout { .. }
                ) {
                    self.reopen(ui);
                }
                self.load.update(|n| *n += 1);
                self.state.set(Some(st));
                return;
            }
            // Contenu de collection écrit par le cœur (vue CMS, annulation) : document, pages et
            // canevas relus.
            if matches!(
                cmd,
                Command::SetEntryField { .. }
                    | Command::SetEntryBody { .. }
                    | Command::CreateEntry { .. }
                    | Command::WriteEntry { .. }
            ) {
                self.reopen(ui);
                self.load.update(|n| *n += 1);
                self.state.set(Some(st));
                return;
            }
            // Élément ajouté (copie, enveloppe, insertion) : sélectionné.
            if let Some(n) = inserted(cmd)
                && let Some(id) = revel_model::ids::node_id(n)
            {
                self.selected.set(Some(Selection {
                    id: id.to_string(),
                    tag: match n {
                        revel_model::Node::Element { tag, .. } => tag.clone(),
                        _ => String::new(),
                    },
                    label: node_label(n),
                }));
            }
            let mut ok = true;
            self.doc.update(|d| {
                if let Some(d) = d {
                    ok = revel_edit::apply(d, cmd).is_ok();
                }
            });
            if !ok {
                // Copie locale désynchronisée : relue depuis le cœur.
                self.reopen(ui);
            }
            // Élément supprimé : plus de sélection.
            let gone = self.selected.with_untracked(|s| {
                s.as_ref().is_some_and(|s| {
                    self.doc.with_untracked(|d| {
                        d.as_ref().is_some_and(|d| !d.node_ids().contains(&s.id))
                    })
                })
            });
            if gone {
                self.selected.set(None);
            }
            if matches!(cmd, Command::SetBreakpoints { .. }) {
                // Points de rupture : réglage de l'éditeur, la page ne change pas.
            } else if style_only(cmd) {
                if let Some(url) = self
                    .doc
                    .with_untracked(|d| d.as_ref()?.stylesheet.as_ref().map(|s| s.url.clone()))
                {
                    self.post(serde_json::json!({ "type": "stylesheet", "url": url }));
                }
            } else {
                self.load.update(|n| *n += 1);
            }
        }
        self.state.set(Some(st));
    }

    /// Document, historique et liste des pages relus (fichiers modifiés hors de l'éditeur : vue
    /// CMS, Pages CMS, éditeur de texte).
    fn reopen(self, ui: Ui) {
        let path = self.path.get_value();
        spawn_local(async move {
            match api::edit_open(&path).await {
                Ok(o) => {
                    self.doc.set(Some(o.site));
                    self.state.set(Some(o.state));
                }
                Err(e) => ui.fail(e),
            }
            if let Ok(p) = api::edit_pages(&path).await {
                if !p.iter().any(|x| x.url == self.page.get_untracked())
                    && let Some(first) = p.first()
                {
                    self.page.set(first.url.clone());
                }
                self.pages.set(p);
            }
        });
    }

    /// Raccourci clavier de l'éditeur (interface ou canevas) ; `true` s'il a été traité.
    pub fn shortcut(self, ui: Ui, k: &Key) -> bool {
        let target = || self.selected_target();
        let key = k.key.to_lowercase();
        let digit = k.digit();
        match (key.as_str(), k.ctrl) {
            ("z", true) if k.shift => self.redo(ui),
            ("z", true) => self.undo(ui),
            ("y", true) => self.redo(ui),
            ("d", true) => match target() {
                Some(at) => self.apply(ui, Command::Duplicate { at }, false),
                None => return false,
            },
            // Envelopper dans un conteneur (Ctrl+G, comme Figma et Framer) ; Ctrl+Maj+G : retirer
            // l'enveloppe (les enfants prennent sa place).
            ("g", true) if !k.shift => match target() {
                Some(at) => self.wrap(ui, at),
                None => return false,
            },
            ("g", true) => return self.unwrap(ui),
            // Copier, couper, coller des éléments.
            ("c", true) if !k.shift => return self.copy(ui, false),
            ("x", true) if !k.shift => return self.copy(ui, true),
            ("v", true) if !k.shift => return self.paste(ui),
            ("delete" | "backspace", false) => match target() {
                Some(at) => self.apply(ui, Command::Remove { at }, false),
                None => return false,
            },
            ("escape", false) => {
                self.selected.set(None);
                self.post(serde_json::json!({ "type": "select", "id": null }));
            }
            // Parcours de la sélection (comme Figma) : Entrée enfant, Maj+Entrée parent, Tab frères.
            ("enter", false) if self.selected.get_untracked().is_some() => {
                let dir = if k.shift { "parent" } else { "child" };
                self.post(serde_json::json!({ "type": "nav", "dir": dir }));
            }
            ("tab", false) if self.selected.get_untracked().is_some() => {
                let dir = if k.shift { "prev" } else { "next" };
                self.post(serde_json::json!({ "type": "nav", "dir": dir }));
            }
            // Ordre parmi les frères : Alt+flèches.
            ("arrowup" | "arrowdown", false) if k.alt => {
                return self.step(ui, if key == "arrowup" { -1 } else { 1 });
            }
            // Zoom (comme Figma) : Maj+0 100 %, Maj+1 ajusté, Maj+2 sélection ; aussi avec Ctrl.
            _ if (k.ctrl || k.shift) && digit == Some(0) => self.zoom_to(Some(1.0)),
            _ if (k.ctrl || k.shift) && digit == Some(1) => self.zoom_to(None),
            _ if (k.ctrl || k.shift) && digit == Some(2) => self.zoom_selection(),
            ("=" | "+", true) => self.zoom_step(1),
            ("-", true) => self.zoom_step(-1),
            _ if k.ctrl && matches!(k.code.as_str(), "NumpadAdd" | "Equal") => self.zoom_step(1),
            _ if k.ctrl && matches!(k.code.as_str(), "NumpadSubtract" | "Minus") => {
                self.zoom_step(-1)
            }
            _ => return false,
        }
        true
    }

    /// Enveloppe un élément dans un conteneur : `span` dans un texte (paragraphe, titre, lien…,
    /// où un `div` casserait la page), `div` ailleurs.
    pub fn wrap(self, ui: Ui, at: revel_edit::Target) {
        const PHRASING: &[&str] = &[
            "p",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "span",
            "a",
            "button",
            "label",
            "strong",
            "em",
            "b",
            "i",
            "u",
            "small",
            "code",
            "figcaption",
            "summary",
            "legend",
            "dt",
            "q",
            "cite",
        ];
        let inline = self.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let (p, _) = revel_edit::address::locate(d, &at).ok()?;
            let parent = revel_edit::Target {
                owner: p.owner,
                node: p.node?,
            };
            match revel_edit::address::node(d, &parent).ok()? {
                revel_model::Node::Element { tag, .. } => {
                    Some(PHRASING.contains(&tag.to_ascii_lowercase().as_str()))
                }
                _ => None,
            }
        });
        let tag = if inline == Some(true) { "span" } else { "div" };
        self.apply(
            ui,
            Command::Wrap {
                at,
                tag: tag.into(),
            },
            false,
        );
    }

    /// Nœud sélectionné (copie).
    fn selected_node(self) -> Option<(revel_edit::Target, revel_model::Node)> {
        let at = self.selected_target()?;
        let n = self
            .doc
            .with_untracked(|d| revel_edit::address::node(d.as_ref()?, &at).ok().cloned())?;
        Some((at, n))
    }

    /// Copie l'élément sélectionné (et le retire avec `cut`).
    pub fn copy(self, ui: Ui, cut: bool) -> bool {
        let Some((at, n)) = self.selected_node() else {
            return false;
        };
        ui.clipboard.set(Some(n));
        if cut {
            self.apply(ui, Command::Remove { at }, false);
        }
        true
    }

    /// Colle l'élément copié (nouveaux identifiants) : à côté d'un élément de même balise
    /// sélectionné, sinon dans l'élément sélectionné (ou après lui), sinon à la fin de la page.
    pub fn paste(self, ui: Ui) -> bool {
        let Some(mut node) = ui.clipboard.get_untracked() else {
            return false;
        };
        let tag = |n: &revel_model::Node| match n {
            revel_model::Node::Element { tag, .. } => Some(tag.clone()),
            revel_model::Node::Image(_) => Some("img".into()),
            _ => None,
        };
        let sibling = self
            .selected_node()
            .filter(|(_, sel)| tag(sel).is_some() && tag(sel) == tag(&node))
            .and_then(|(at, _)| {
                self.doc.with_untracked(|d| {
                    let (p, i) = revel_edit::address::locate(d.as_ref()?, &at).ok()?;
                    Some((p, i + 1))
                })
            });
        let Some((parent, index)) = sibling.or_else(|| crate::insert::insertion_point(self)) else {
            return false;
        };
        self.doc.with_untracked(|d| {
            if let Some(d) = d {
                let mut taken = d.node_ids();
                revel_edit::make::refresh_ids(&mut node, &mut taken);
            }
        });
        self.apply(
            ui,
            Command::Insert {
                parent,
                index,
                node,
            },
            false,
        );
        true
    }

    /// Retire l'enveloppe de l'élément sélectionné : ses enfants prennent sa place.
    pub fn unwrap(self, ui: Ui) -> bool {
        let Some((at, node)) = self.selected_node() else {
            return false;
        };
        let revel_model::Node::Element { children, .. } = node else {
            return false;
        };
        let Some((parent, index)) = self
            .doc
            .with_untracked(|d| revel_edit::address::locate(d.as_ref()?, &at).ok())
        else {
            return false;
        };
        let mut commands = vec![Command::Remove { at }];
        for (i, child) in children.into_iter().enumerate() {
            commands.push(Command::Insert {
                parent: parent.clone(),
                index: index + i,
                node: child,
            });
        }
        self.apply(ui, Command::Batch { commands }, false);
        true
    }

    /// Monte (`-1`) ou descend (`+1`) l'élément sélectionné parmi ses frères.
    pub fn step(self, ui: Ui, dir: i32) -> bool {
        let Some(sel) = self.selected.get_untracked() else {
            return false;
        };
        let cmd = self.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let url = self.page.get_untracked();
            let page = self
                .pages
                .with_untracked(|ps| ps.iter().find(|p| p.url == url).cloned())?;
            let rows = crate::layers::rows(d, &page);
            let row = rows
                .iter()
                .find(|r| r.id.as_deref() == Some(sel.id.as_str()))?;
            crate::layers::step_command(&rows, row, dir)
        });
        match cmd {
            Some(c) => {
                self.apply(ui, c, false);
                true
            }
            None => false,
        }
    }

    /// Géométrie du canevas (lecture suivie : à appeler hors réaction avec `untrack`).
    pub fn geo(self) -> Geo {
        let (aw, ah) = self.area.get();
        let desktop_min = self.doc.with(|d| {
            d.as_ref()
                .and_then(|d| d.breakpoints.iter().filter_map(|b| b.max_width).max())
                .map(|w| f64::from(w) + 1.0)
                .unwrap_or(320.0)
        });
        let w = self
            .frame_width()
            .unwrap_or_else(|| (aw - GUTTER).max(desktop_min));
        let fit = ((aw - GUTTER) / w).clamp(0.1, 1.0);
        let h = ((ah - GUTTER) / fit).max(200.0);
        match self.zoom.get() {
            None => Geo {
                w,
                h,
                fit,
                s: fit,
                x: ((aw - w * fit) / 2.0).max(GUTTER / 2.0),
                y: GUTTER / 2.0,
            },
            Some(z) => {
                let (x, y) = self.pan.get();
                Geo {
                    w,
                    h,
                    fit,
                    s: z,
                    x,
                    y,
                }
            }
        }
    }

    /// Position du cadre gardée en partie visible.
    fn clamp_pan(self, g: &Geo, x: f64, y: f64) -> (f64, f64) {
        let (aw, ah) = self.area.get_untracked();
        let keep = 80.0;
        (
            x.clamp(keep - g.w * g.s, (aw - keep).max(keep - g.w * g.s)),
            y.clamp(keep - g.h * g.s, (ah - keep).max(keep - g.h * g.s)),
        )
    }

    /// Zoom en vigueur (ajusté compris).
    pub fn scale(self) -> f64 {
        untrack(|| self.geo().s)
    }

    /// Zoom `z` autour du point (`cx`, `cy`) de la zone du canevas.
    pub fn zoom_at(self, z: f64, cx: f64, cy: f64) {
        let g = untrack(|| self.geo());
        let z = ((z * 100.0).round() / 100.0).clamp(0.1, 4.0);
        let k = z / g.s;
        let ng = Geo { s: z, ..g };
        let (x, y) = self.clamp_pan(&ng, cx - (cx - g.x) * k, cy - (cy - g.y) * k);
        self.pan.set((x, y));
        self.zoom.set(Some(z));
    }

    /// Zoom choisi (liste, raccourci) : cadre centré s'il tient dans la largeur, sinon calé à
    /// gauche ; le haut visible du cadre reste en place.
    pub fn zoom_to(self, z: Option<f64>) {
        let Some(z) = z else {
            self.zoom.set(None);
            return;
        };
        let g = untrack(|| self.geo());
        let (aw, _) = self.area.get_untracked();
        let z = ((z * 100.0).round() / 100.0).clamp(0.1, 4.0);
        let x = if g.w * z <= aw - GUTTER {
            (aw - g.w * z) / 2.0
        } else {
            GUTTER / 2.0
        };
        let y = if g.y >= 0.0 { g.y } else { g.y * z / g.s };
        let (x, y) = self.clamp_pan(&Geo { s: z, ..g }, x, y);
        self.pan.set((x, y));
        self.zoom.set(Some(z));
    }

    /// Zoom multiplié (molette avec Ctrl) autour d'un point de la zone (centre à défaut).
    pub fn zoom_by(self, f: f64, at: Option<(f64, f64)>) {
        let (aw, ah) = self.area.get_untracked();
        let (cx, cy) = at.unwrap_or((aw / 2.0, ah / 2.0));
        self.zoom_at(self.scale() * f, cx, cy);
    }

    /// Palier de zoom suivant ou précédent.
    pub fn zoom_step(self, dir: i32) {
        let cur = self.scale();
        let next = if dir > 0 {
            ZOOMS.iter().copied().find(|z| *z > cur + 0.001)
        } else {
            ZOOMS.iter().rev().copied().find(|z| *z < cur - 0.001)
        };
        if let Some(z) = next {
            self.zoom_to(Some(z));
        }
    }

    /// Zoom sur la sélection : le canevas la montre puis donne sa position (message `measure`).
    pub fn zoom_selection(self) {
        if let Some(s) = self.selected.get_untracked() {
            self.post(serde_json::json!({ "type": "measure", "id": s.id }));
        }
    }

    /// Cadre sur la sélection mesurée.
    fn fit_rect(self, r: Rect) {
        let (aw, ah) = self.area.get_untracked();
        if r.width <= 0.0 || r.height <= 0.0 {
            return;
        }
        let z = ((aw - 2.0 * GUTTER) / r.width)
            .min((ah - 2.0 * GUTTER) / r.height)
            .clamp(0.1, 4.0);
        let z = (z * 100.0).round() / 100.0;
        let g = Geo {
            s: z,
            ..untrack(|| self.geo())
        };
        let x = aw / 2.0 - (r.x + r.width / 2.0) * z;
        let y = ah / 2.0 - (r.y + r.height / 2.0) * z;
        let (x, y) = self.clamp_pan(&g, x, y);
        self.pan.set((x, y));
        self.zoom.set(Some(z));
    }

    /// Zoom libre : cadre recentré horizontalement (changement de point de rupture).
    pub fn recenter(self) {
        if self.zoom.get_untracked().is_some() {
            let g = untrack(|| self.geo());
            let (aw, _) = self.area.get_untracked();
            self.pan
                .set((((aw - g.w * g.s) / 2.0).max(GUTTER / 2.0), g.y));
        }
    }

    /// Déplacement de la vue (pixels de l'écran).
    pub fn pan_by(self, dx: f64, dy: f64) {
        let g = untrack(|| self.geo());
        let (x, y) = self.clamp_pan(&g, g.x + dx, g.y + dy);
        self.pan.set((x, y));
        if self.zoom.get_untracked().is_none() {
            self.zoom.set(Some(g.s));
        }
    }

    pub fn undo(self, ui: Ui) {
        if !self
            .state
            .with_untracked(|s| s.as_ref().is_some_and(|s| s.can_undo))
        {
            return;
        }
        let path = self.path.get_value();
        self.pending.update(|n| *n += 1);
        spawn_local(async move {
            let r = api::edit_undo(&path).await;
            self.pending.update(|n| *n = n.saturating_sub(1));
            match r {
                Ok(st) => {
                    self.saved_at.set(Some(js_sys::Date::now()));
                    self.after(ui, st)
                }
                Err(e) => ui.fail(e),
            }
        });
    }

    pub fn redo(self, ui: Ui) {
        if !self
            .state
            .with_untracked(|s| s.as_ref().is_some_and(|s| s.can_redo))
        {
            return;
        }
        let path = self.path.get_value();
        self.pending.update(|n| *n += 1);
        spawn_local(async move {
            let r = api::edit_redo(&path).await;
            self.pending.update(|n| *n = n.saturating_sub(1));
            match r {
                Ok(st) => {
                    self.saved_at.set(Some(js_sys::Date::now()));
                    self.after(ui, st)
                }
                Err(e) => ui.fail(e),
            }
        });
    }

    /// Applique une commande d'édition.
    pub fn apply(self, ui: Ui, cmd: Command, coalesce: bool) {
        let path = self.path.get_value();
        self.pending.update(|n| *n += 1);
        spawn_local(async move {
            let r = api::edit_apply(&path, &cmd, coalesce).await;
            self.pending.update(|n| *n = n.saturating_sub(1));
            match r {
                Ok(st) => {
                    self.saved_at.set(Some(js_sys::Date::now()));
                    self.after(ui, st)
                }
                Err(e) => {
                    // Aperçu d'un geste refusé : retiré du canevas.
                    self.post(serde_json::json!({ "type": "preview", "css": "" }));
                    ui.fail(e)
                }
            }
        });
    }

    /// Commande d'un texte modifié dans le canevas : texte de la page (`SetText`, sans la
    /// typographie de la page si la source ne l'utilisait pas) ou valeur liée à sa source
    /// (`SetBound`, même traitement par le cœur).
    fn text_command(self, slot: &revel_edit::canvas::Slot, text: String) -> Option<Command> {
        use revel_edit::canvas::Slot;
        match slot {
            Slot::Text { parent, index } => self.doc.with_untracked(|d| {
                let d = d.as_ref()?;
                let old = match revel_edit::address::list(d, parent).ok()?.get(*index)? {
                    revel_model::Node::Text { text } => text.clone(),
                    _ => return None,
                };
                let new = if d.settings.typography.as_deref() == Some("fr") {
                    revel_edit::canvas::untypo(&text, &old)
                } else {
                    text
                };
                (new != old).then(|| Command::SetText {
                    parent: parent.clone(),
                    index: *index,
                    text: new,
                })
            }),
            Slot::Expr {
                binding: Some(binding),
                transformed: false,
                html: false,
                ..
            } => Some(Command::SetBound {
                binding: binding.clone(),
                value: text,
            }),
            _ => None,
        }
    }

    /// Adresse du nœud sélectionné (son propriétaire est retrouvé dans le document).
    pub fn selected_target(self) -> Option<revel_edit::Target> {
        let sel = self.selected.get()?;
        self.doc.with(|d| {
            let (kind, id) = owner_of(d.as_ref()?, &sel.id)?;
            let owner = match kind {
                revel_model::ids::OwnerKind::Page => revel_edit::Owner::Page(id),
                revel_model::ids::OwnerKind::Layout => revel_edit::Owner::Layout(id),
                revel_model::ids::OwnerKind::Component => revel_edit::Owner::Component(id),
            };
            Some(revel_edit::Target {
                owner,
                node: sel.id.clone(),
            })
        })
    }

    /// Largeur du cadre pour le point de rupture affiché ; `None` : toute la place.
    pub fn frame_width(self) -> Option<f64> {
        let id = self.breakpoint.get();
        self.doc.with(|d| {
            let bp = d.as_ref()?.breakpoints.iter().find(|b| b.id == id)?;
            bp.max_width.or(bp.min_width).map(f64::from)
        })
    }
}

/// Nœud inséré par une commande (première insertion d'une suite).
fn inserted(c: &Command) -> Option<&revel_model::Node> {
    match c {
        Command::Insert { node, .. } => Some(node),
        Command::Batch { commands } => commands.iter().find_map(inserted),
        _ => None,
    }
}

/// Libellé d'un nœud (`div.hero`).
pub fn node_label(n: &revel_model::Node) -> String {
    match n {
        revel_model::Node::Element { tag, attrs, .. } => {
            let class = attrs
                .iter()
                .find(|a| a.name == "class")
                .and_then(|a| a.value.as_deref())
                .and_then(|v| v.split_whitespace().next())
                .filter(|c| !c.contains("{{"));
            match class {
                Some(c) => format!("{tag}.{c}"),
                None => tag.clone(),
            }
        }
        revel_model::Node::Image(_) => "img".into(),
        _ => String::new(),
    }
}

/// Chemin jusqu'au nœud `id` : éléments (identifiant, libellé) de la racine au nœud.
fn path_to(nodes: &[revel_model::Node], id: &str, stack: &mut Vec<(String, String)>) -> bool {
    for n in nodes {
        let nid = revel_model::ids::node_id(n);
        let pushed = match (nid, n) {
            (Some(i), revel_model::Node::Element { .. } | revel_model::Node::Image(_)) => {
                stack.push((i.to_string(), node_label(n)));
                true
            }
            _ => false,
        };
        if nid == Some(id) {
            return true;
        }
        for list in revel_model::ids::child_lists(n) {
            if path_to(list, id, stack) {
                return true;
            }
        }
        if pushed {
            stack.pop();
        }
    }
    false
}

/// Propriétaire d'un nœud (page, gabarit, composant) : libellé pour la barre d'état.
pub fn owner_of(doc: &Site, id: &str) -> Option<(revel_model::ids::OwnerKind, String)> {
    for (kind, owner, body) in doc.owners() {
        let mut found = false;
        revel_model::ids::walk(body, &mut |n| {
            if !found && revel_model::ids::node_id(n) == Some(id) {
                found = true;
            }
        });
        if found {
            return Some((kind, owner.to_string()));
        }
    }
    None
}

#[component]
pub fn Editor(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = Ed {
        path: StoredValue::new(site.path.clone()),
        doc: RwSignal::new(None),
        state: RwSignal::new(None),
        base: RwSignal::new(None),
        pages: RwSignal::new(Vec::new()),
        page: RwSignal::new("/".into()),
        breakpoint: RwSignal::new("desktop".into()),
        zoom: RwSignal::new(None),
        pan: RwSignal::new((0.0, 0.0)),
        area: RwSignal::new((0.0, 0.0)),
        space: RwSignal::new(false),
        panning: RwSignal::new(false),
        drag: RwSignal::new(None),
        drag_pos: RwSignal::new((0.0, 0.0)),
        drop: RwSignal::new(None),
        menu: RwSignal::new(None),
        fit: RwSignal::new(1.0),
        selected: RwSignal::new(None),
        hovered: RwSignal::new(None),
        scripts: RwSignal::new(false),
        load: RwSignal::new(0),
        scroll: RwSignal::new((0.0, 0.0)),
        computed: RwSignal::new(Default::default()),
        count: RwSignal::new(None),
        own_change: RwSignal::new(0.0),
        pending: RwSignal::new(0),
        saved_at: RwSignal::new(None),
        frame: NodeRef::new(),
    };
    provide_context(ed);
    let dock = crate::dock::Dock::provide();
    let failed = RwSignal::new(false);

    // Ouverture : document, pages, canevas.
    let path = site.path.clone();
    spawn_local(async move {
        let opened = api::edit_open(&path).await;
        let pages = api::edit_pages(&path).await;
        let base = api::canvas_open(&path, false).await;
        match (opened, pages, base) {
            (Ok(o), Ok(p), Ok(b)) => {
                if !p.iter().any(|x| x.url == "/")
                    && let Some(first) = p.first()
                {
                    ed.page.set(first.url.clone());
                }
                ed.doc.set(Some(o.site));
                ed.state.set(Some(o.state));
                ed.pages.set(p);
                ed.base.set(Some(b));
            }
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                failed.set(true);
                ui.fail(e);
            }
        }
    });

    // Page demandée par une autre vue (vue CMS) : affichée dès que la liste des pages est connue.
    Effect::new(move |_| {
        let Some(url) = ui.goto_page.get() else {
            return;
        };
        if ed.pages.with(|p| p.iter().any(|x| x.url == url)) {
            ui.goto_page.set(None);
            if ed.page.get_untracked() != url {
                ed.page.set(url);
                ed.scroll.set((0.0, 0.0));
                ed.selected.set(None);
            }
        }
    });

    // Messages du canevas.
    let on_message = window_event_listener(ev::message, move |e: web_sys::MessageEvent| {
        let Some(frame) = ed.frame.get_untracked() else {
            return;
        };
        let from_canvas = match (e.source(), frame.content_window()) {
            (Some(src), Some(win)) => js_sys::Object::is(&src, win.as_ref()),
            _ => false,
        };
        let data = e.data();
        let tagged = js_sys::Reflect::get(&data, &JsValue::from_str("revel"))
            .ok()
            .and_then(|v| v.as_f64())
            == Some(1.0);
        if !from_canvas || !tagged {
            return;
        }
        let Ok(msg) = serde_wasm_bindgen::from_value::<FromCanvas>(data) else {
            return;
        };
        match msg {
            FromCanvas::Ready { path } => {
                // Navigation impossible dans le canevas : la page est celle demandée.
                let _ = path;
                ed.post(serde_json::json!({ "type": "watch", "props": crate::styles::watched() }));
                let (x, y) = ed.scroll.get_untracked();
                if x != 0.0 || y != 0.0 {
                    ed.post(serde_json::json!({ "type": "scroll", "x": x, "y": y }));
                }
                if let Some(s) = ed.selected.get_untracked() {
                    ed.post(serde_json::json!({ "type": "select", "id": s.id }));
                }
            }
            FromCanvas::Hover { id } => ed.hovered.set(id),
            FromCanvas::Keyup { key } => {
                if key == " " {
                    ed.space.set(false);
                }
            }
            FromCanvas::Pan { dx, dy } => ed.pan_by(dx, dy),
            FromCanvas::Drop { target } => ed.drop.set(target),
            FromCanvas::Moved { id, target, copy } => {
                ed.drop.set(None);
                if let Some(cmd) = crate::insert::move_command(ed, &id, &target, copy) {
                    ed.apply(ui, cmd, false);
                }
            }
            FromCanvas::Context { x, y } => {
                // Position dans le cadre -> dans la fenêtre.
                if let Some(f) = ed.frame.get_untracked() {
                    let r = f.get_bounding_client_rect();
                    let s = ed.scale();
                    ed.menu.set(Some((r.left() + x * s, r.top() + y * s)));
                }
            }
            FromCanvas::Measure { rect } => {
                if let Some(r) = rect {
                    ed.fit_rect(r);
                }
            }
            FromCanvas::Select { id, tag, label } => {
                if id.is_none() {
                    ed.computed.set(Default::default());
                }
                ed.selected.set(id.map(|id| Selection { id, tag, label }));
            }
            FromCanvas::Key {
                key,
                code,
                shift,
                alt,
                ctrl,
                meta,
            } => {
                let k = Key {
                    key,
                    code,
                    ctrl: ctrl || meta,
                    shift,
                    alt,
                };
                // Ctrl+\ dans le canevas : panneaux masqués ou affichés.
                if k.key == "\\" && k.ctrl {
                    dock.layout.update(|l| l.hidden = !l.hidden);
                } else if k.key == " " {
                    ed.space.set(true);
                } else {
                    ed.shortcut(ui, &k);
                }
            }
            FromCanvas::Scroll { x, y } => ed.scroll.set((x, y)),
            FromCanvas::Computed { values } => ed.computed.set(values),
            FromCanvas::Count { selector, n } => ed.count.set(Some((selector, n))),
            FromCanvas::Zoom { dy, x, y } => {
                // Position du pointeur dans le cadre -> dans la zone du canevas.
                let at = x.zip(y).map(|(x, y)| {
                    let g = untrack(|| ed.geo());
                    (g.x + x * g.s, g.y + y * g.s)
                });
                ed.zoom_by((-dy * 0.002).exp(), at);
            }
            FromCanvas::Text { slot, text } => {
                if let Some(cmd) = ed.text_command(&slot, text) {
                    ed.apply(ui, cmd, false);
                }
            }
            FromCanvas::Other => {}
        }
    });
    // Raccourcis dans l'interface, calques compris (hors champs de saisie), quand l'éditeur est
    // la vue affichée (il reste en place, caché, pendant les autres vues).
    let on_key = window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if !matches!(
            ui.route.get_untracked(),
            crate::Route::Site {
                tab: crate::Tab::Editor,
                ..
            }
        ) {
            return;
        }
        let in_field = document()
            .active_element()
            .map(|el| {
                matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                    || el.has_attribute("contenteditable")
            })
            .unwrap_or(false);
        // Un menu ouvert (panneau de styles) se ferme d'abord avec Échap.
        let menu_open = document().query_selector(".pop").ok().flatten().is_some();
        if in_field || (menu_open && e.key() == "Escape") {
            return;
        }
        // Espace maintenue : déplacement de la vue.
        if e.key() == " " && !e.ctrl_key() && !e.meta_key() {
            let on_control = document()
                .active_element()
                .is_some_and(|el| matches!(el.tag_name().as_str(), "BUTTON" | "SUMMARY"));
            if !on_control {
                e.prevent_default();
                ed.space.set(true);
                return;
            }
        }
        // Tab et Entrée gardent leur rôle dans l'interface (navigation au clavier) hors du canevas.
        let focus_in_ui = document()
            .active_element()
            .is_some_and(|el| el.tag_name() != "BODY" && el.tag_name() != "IFRAME");
        if focus_in_ui && matches!(e.key().as_str(), "Tab" | "Enter") {
            return;
        }
        if ed.shortcut(ui, &Key::of(&e)) {
            e.prevent_default();
        }
    });
    let on_keyup = window_event_listener(ev::keyup, move |e: web_sys::KeyboardEvent| {
        if e.key() == " " {
            ed.space.set(false);
        }
    });
    let on_blur = window_event_listener(ev::blur, move |_| {
        // Fenêtre quittée pendant que l'espace était maintenue (sauf vers le canevas).
        let to_canvas = document()
            .active_element()
            .is_some_and(|el| el.tag_name() == "IFRAME");
        if !to_canvas {
            ed.space.set(false);
        }
    });
    on_cleanup(move || {
        on_message.remove();
        on_key.remove();
        on_keyup.remove();
        on_blur.remove();
    });
    install_test_hook(ed, ui);

    // Fichiers modifiés hors de Revel (Pages CMS, éditeur de texte…) : page et document relus.
    let stamp = StoredValue::new(None::<String>);
    let path_poll = site.path.clone();
    if let Ok(h) = set_interval_with_handle(
        move || {
            let path = path_poll.clone();
            spawn_local(async move {
                let Ok(now) = api::edit_stamp(&path).await else {
                    return;
                };
                let before = stamp.get_value();
                stamp.set_value(Some(now.clone()));
                let recent = js_sys::Date::now() - ed.own_change.get_untracked() < 2500.0;
                if before.is_some_and(|b| b != now) && !recent {
                    ed.reopen(ui);
                    ed.load.update(|n| *n += 1);
                }
            });
        },
        std::time::Duration::from_millis(2000),
    ) {
        on_cleanup(move || h.clear());
    }

    // Échelle du zoom ajusté (affichée dans la liste des zooms).
    Effect::new(move |_| ed.fit.set(ed.geo().fit));

    view! {
        <section class="editor">
            <Toolbar/>
            <crate::dock::Workspace panel=panel_view>
                <crate::canvas::CanvasArea failed=failed/>
            </crate::dock::Workspace>
            <StatusBar/>
        </section>
    }
}

/// Contenu d'un panneau de l'espace de travail.
fn panel_view(p: crate::dock::PanelId) -> AnyView {
    use crate::dock::PanelId;
    match p {
        PanelId::Pages => view! { <crate::panels::Pages/> }.into_any(),
        PanelId::Layers => view! { <crate::layers::Layers/> }.into_any(),
        PanelId::Cms => view! { <crate::panels::Cms/> }.into_any(),
        PanelId::Insert => view! { <crate::insert::Insert/> }.into_any(),
        PanelId::Components => view! { <crate::panels::Components/> }.into_any(),
        PanelId::Assets => view! { <crate::panels::Assets/> }.into_any(),
        PanelId::Styles => {
            view! { <div class="panel-pad"><crate::styles::Styles/></div> }.into_any()
        }
        PanelId::Settings => view! { <crate::inspector::ElementSettings/> }.into_any(),
        PanelId::Interactions => view! {
            <crate::panels::Planned key="panel-interactions-planned" step=6 icon=icons::bolt/>
        }
        .into_any(),
    }
}

#[component]
fn Toolbar() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let breakpoints = move || {
        ed.doc.with(|d| {
            d.as_ref()
                .map(|d| d.breakpoints.clone())
                .unwrap_or_default()
        })
    };
    let bp_label = move |b: &revel_model::Breakpoint| match (b.max_width, b.min_width) {
        (Some(w), _) => ui.tx("editor-bp-max", &[("width", w.to_string())]),
        (_, Some(w)) => ui.tx("editor-bp-min", &[("width", w.to_string())]),
        _ => ui.tx("editor-bp-desktop", &[]),
    };
    let bp_icon = |b: &revel_model::Breakpoint| match b.max_width {
        Some(w) if w <= 600 => icons::mobile(),
        Some(_) => icons::tablet(),
        None => icons::desktop(),
    };
    let can_undo = move || ed.state.with(|s| s.as_ref().is_some_and(|s| s.can_undo));
    let can_redo = move || ed.state.with(|s| s.as_ref().is_some_and(|s| s.can_redo));
    let pct = |z: f64| format!("{} %", (z * 100.0).round());
    let set_scripts = move |on: bool| {
        ed.scripts.set(on);
        let path = ed.path.get_value();
        spawn_local(async move {
            match api::canvas_open(&path, on).await {
                Ok(b) => {
                    ed.base.set(Some(b));
                    ed.load.update(|n| *n += 1);
                }
                Err(e) => ui.fail(e),
            }
        });
    };
    view! {
        <div class="editor-toolbar">
            <div class="select editor-page">
                <select class="input" aria-label=ui.t("editor-page")
                    prop:value=move || ed.page.get()
                    on:change=move |e| {
                        ed.page.set(event_target_value(&e));
                        ed.scroll.set((0.0, 0.0));
                        ed.selected.set(None);
                    }>
                    <optgroup label=ui.t("editor-pages-fixed")>
                        {move || ed.pages.get().into_iter().filter(|p| p.page.is_some()).map(|p| {
                            let label = if p.title.is_empty() { p.url.clone() } else { format!("{} — {}", p.title, p.url) };
                            view! { <option value=p.url.clone()>{label}</option> }
                        }).collect_view()}
                    </optgroup>
                    {move || {
                        let pages = ed.pages.get();
                        let mut cols: Vec<String> = pages.iter().filter_map(|p| p.collection.clone()).collect();
                        cols.dedup();
                        cols.into_iter().map(|c| {
                            let items: Vec<_> = pages.iter().filter(|p| p.collection.as_deref() == Some(c.as_str())).cloned().collect();
                            view! {
                                <optgroup label=c.clone()>
                                    {items.into_iter().map(|p| {
                                        let label = if p.title.is_empty() { p.url.clone() } else { format!("{} — {}", p.title, p.url) };
                                        view! { <option value=p.url.clone()>{label}</option> }
                                    }).collect_view()}
                                </optgroup>
                            }
                        }).collect_view()
                    }}
                </select>
            </div>
            <div class="segmented" role="group" aria-label=ui.t("editor-breakpoints")>
                {move || breakpoints().into_iter().map(|b| {
                    let id = b.id.clone();
                    let on = b.id.clone();
                    let title = bp_label(&b);
                    view! {
                        <button class:is-on=move || ed.breakpoint.get() == on title=title.clone() aria-label=title.clone()
                            on:click=move |_| { ed.breakpoint.set(id.clone()); ed.recenter(); }>
                            {bp_icon(&b)}
                            <span class="bp-width">{b.max_width.or(b.min_width).map(|w| w.to_string()).unwrap_or_default()}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
            <BreakpointMenu/>
            <div class="select editor-zoom" title=ui.t("editor-zoom-help")>
                <select class="input" aria-label=ui.t("editor-zoom")
                    on:change=move |e| {
                        let v = event_target_value(&e);
                        ed.zoom_to(v.parse::<f64>().ok());
                    }>
                    <option value="" selected=move || ed.zoom.get().is_none()>
                        {move || ui.tx("editor-zoom-fit-pct", &[("pct", pct(ed.fit.get()))])}
                    </option>
                    {ZOOMS.into_iter().map(|z| view! {
                        <option value=z.to_string() selected=move || ed.zoom.get() == Some(z)>{pct(z)}</option>
                    }).collect_view()}
                    {move || ed.zoom.get().filter(|z| !ZOOMS.contains(z)).map(|z| view! {
                        <option value=z.to_string() selected=true>{pct(z)}</option>
                    })}
                </select>
            </div>
            <button class="tool-toggle" class:is-on=move || ed.scripts.get() aria-pressed=move || ed.scripts.get().to_string()
                title=ui.t("editor-scripts-help") aria-label=ui.t("editor-scripts")
                on:click=move |_| set_scripts(!ed.scripts.get_untracked())>{icons::code()}</button>
            <div class="toolbar-spacer"></div>
            <crate::dock::PanelsMenu/>
            <details class="bp-menu shortcuts">
                <summary class="icon-btn" title=ui.t("editor-shortcuts") aria-label=ui.t("editor-shortcuts")>"?"</summary>
                <div class="bp-pop bp-pop--right">
                    <p class="panel-title">{ui.t("editor-shortcuts")}</p>
                    {[
                        ("key-undo", "editor-undo-short"),
                        ("key-redo", "editor-redo-short"),
                        ("key-duplicate", "layers-duplicate-short"),
                        ("key-delete", "layers-delete-short"),
                        ("key-escape", "editor-deselect"),
                        ("key-dblclick", "editor-edit-text"),
                        ("key-enter", "editor-edit-validate"),
                        ("key-child", "editor-select-child"),
                        ("key-parent", "editor-select-parent"),
                        ("key-siblings", "editor-select-sibling"),
                        ("key-move-drag", "editor-move"),
                        ("key-copy-drag", "editor-move-copy"),
                        ("key-order", "editor-order"),
                        ("key-wrap", "layers-wrap"),
                        ("key-unwrap", "editor-unwrap"),
                        ("key-copy", "editor-copy"),
                        ("key-paste", "editor-paste"),
                        ("key-zoom-fit", "editor-zoom-fit-short"),
                        ("key-zoom-100", "editor-zoom-100"),
                        ("key-zoom-sel", "editor-zoom-selection"),
                        ("key-zoom-wheel", "editor-zoom"),
                        ("key-pan", "editor-pan"),
                        ("key-panels", "dock-hide-all"),
                        ("key-alt-click", "styles-reset"),
                        ("key-drag-label", "editor-scrub"),
                    ].into_iter().map(|(k, label)| view! {
                        <div class="bp-line"><span>{ui.t(label)}</span><kbd>{ui.t(k)}</kbd></div>
                    }).collect_view()}
                </div>
            </details>
            <button class="icon-btn btn-undo" title=ui.t("editor-undo") aria-label=ui.t("editor-undo")
                prop:disabled=move || !can_undo() on:click=move |_| ed.undo(ui)>{icons::undo()}</button>
            <button class="icon-btn btn-redo" title=ui.t("editor-redo") aria-label=ui.t("editor-redo")
                prop:disabled=move || !can_redo() on:click=move |_| ed.redo(ui)>{icons::redo()}</button>
        </div>
    }
}

/// Points de rupture du site (étape 2.8) : ajout d'une largeur, retrait d'un point inutilisé.
#[component]
fn BreakpointMenu() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let kind_max = RwSignal::new(true);
    let width = RwSignal::new(String::new());
    let list = move || {
        ed.doc.with(|d| {
            d.as_ref()
                .map(|d| d.breakpoints.clone())
                .unwrap_or_default()
        })
    };
    // Ordre : base, largeurs maximales décroissantes, largeurs minimales croissantes.
    let sorted = |mut v: Vec<revel_model::Breakpoint>| {
        v.sort_by_key(|b| match (b.max_width, b.min_width) {
            (None, None) => (0, 0i64),
            (Some(w), _) => (1, -i64::from(w)),
            (_, Some(w)) => (2, i64::from(w)),
        });
        v
    };
    let add = move || {
        let Ok(w) = width.get_untracked().trim().parse::<u32>() else {
            return;
        };
        let max = kind_max.get_untracked();
        let mut v = list();
        v.push(revel_model::Breakpoint {
            id: format!("{}-{w}", if max { "max" } else { "min" }),
            max_width: max.then_some(w),
            min_width: (!max).then_some(w),
        });
        width.set(String::new());
        ed.apply(
            ui,
            Command::SetBreakpoints {
                breakpoints: sorted(v),
            },
            false,
        );
    };
    let remove = move |id: String| {
        let (used, v) = ed.doc.with_untracked(|d| {
            let Some(d) = d.as_ref() else {
                return (false, Vec::new());
            };
            let media = d
                .breakpoints
                .iter()
                .find(|b| b.id == id)
                .and_then(|b| b.media());
            let used = media.is_some()
                && d.style_rules()
                    .any(|r| r.media.as_deref() == media.as_deref());
            let v = d
                .breakpoints
                .iter()
                .filter(|b| b.id != id)
                .cloned()
                .collect();
            (used, v)
        });
        if used {
            ui.fail(revel_protocol::Failure::new("editor-bp-in-use"));
            return;
        }
        if ed.breakpoint.get_untracked() == id {
            ed.breakpoint.set("desktop".into());
        }
        ed.apply(ui, Command::SetBreakpoints { breakpoints: v }, false);
    };
    view! {
        <details class="bp-menu">
            <summary class="icon-btn" title=ui.t("editor-bp-manage") aria-label=ui.t("editor-bp-manage")>{icons::plus()}</summary>
            <div class="bp-pop">
                <p class="panel-title">{ui.t("editor-breakpoints")}</p>
                {move || list().into_iter().filter(|b| b.id != "desktop").map(|b| {
                    let id = b.id.clone();
                    let label = match (b.max_width, b.min_width) {
                        (Some(w), _) => ui.tx("editor-bp-max", &[("width", w.to_string())]),
                        (_, Some(w)) => ui.tx("editor-bp-min", &[("width", w.to_string())]),
                        _ => String::new(),
                    };
                    view! {
                        <div class="bp-line">
                            <span>{label}</span>
                            <button class="icon-btn" title=ui.t("editor-bp-remove") aria-label=ui.t("editor-bp-remove")
                                on:click=move |_| remove(id.clone())>{icons::trash()}</button>
                        </div>
                    }
                }).collect_view()}
                <div class="bp-add">
                    <div class="select">
                        <select class="input" on:change=move |e| kind_max.set(event_target_value(&e) == "max")>
                            <option value="max" selected=move || kind_max.get()>{ui.t("editor-bp-kind-max")}</option>
                            <option value="min" selected=move || !kind_max.get()>{ui.t("editor-bp-kind-min")}</option>
                        </select>
                    </div>
                    <input class="input input--mono" inputmode="numeric" placeholder="px" prop:value=move || width.get()
                        on:input=move |e| width.set(event_target_value(&e))
                        on:keydown=move |e: leptos::ev::KeyboardEvent| if e.key() == "Enter" { add() }/>
                    <button class="btn" on:click=move |_| add()>{ui.t("editor-bp-add")}</button>
                </div>
            </div>
        </details>
    }
}

#[component]
fn StatusBar() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let owner = move || {
        let sel = ed.selected.get()?;
        ed.doc.with(|d| {
            let (kind, id) = owner_of(d.as_ref()?, &sel.id)?;
            let key = match kind {
                revel_model::ids::OwnerKind::Page => "editor-owner-page",
                revel_model::ids::OwnerKind::Layout => "editor-owner-layout",
                revel_model::ids::OwnerKind::Component => "editor-owner-component",
            };
            Some(ui.tx(key, &[("name", id)]))
        })
    };
    let width = move || {
        ed.frame_width()
            .map(|w| format!("{w} px"))
            .unwrap_or_default()
    };
    // Fil d'Ariane (comme Webflow) : éléments parents de la sélection, cliquables.
    let crumbs = move || {
        let sel = ed.selected.get()?;
        ed.doc.with(|d| {
            let d = d.as_ref()?;
            d.owners().into_iter().find_map(|(_, _, body)| {
                let mut stack = Vec::new();
                path_to(body, &sel.id, &mut stack).then_some(stack)
            })
        })
    };
    let pick = move |id: String, label: String| {
        ed.selected.set(Some(Selection {
            id: id.clone(),
            tag: label.split('.').next().unwrap_or_default().to_string(),
            label,
        }));
        ed.post(serde_json::json!({ "type": "select", "id": id, "reveal": true }));
    };
    view! {
        <footer class="editor-status">
            <nav class="node-path" aria-label=ui.t("editor-path")>
                {move || match crumbs() {
                    None => view! { <span class="muted">{ui.t("editor-no-selection")}</span> }.into_any(),
                    Some(list) => {
                        let n = list.len();
                        let skip = n.saturating_sub(6);
                        view! {
                            {(skip > 0).then(|| view! { <span class="node-sep">"…"</span> })}
                            {list.into_iter().enumerate().skip(skip).map(|(i, (id, label))| {
                                let last = i + 1 == n;
                                let l2 = label.clone();
                                view! {
                                    {(i > skip || skip > 0).then(|| view! { <span class="node-sep">"›"</span> })}
                                    <button class="node-crumb" class:is-on=last title=label.clone()
                                        on:click=move |_| pick(id.clone(), l2.clone())>{label.clone()}</button>
                                }
                            }).collect_view()}
                        }.into_any()
                    }
                }}
            </nav>
            <span class="muted">{owner}</span>
            <span class="toolbar-spacer"></span>
            <span class="muted save-state" aria-live="polite">{move || {
                if ed.pending.get() > 0 {
                    ui.tx("editor-saving", &[])
                } else if let Some(t) = ed.saved_at.get() {
                    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(t));
                    let time = format!("{:02}:{:02}", d.get_hours(), d.get_minutes());
                    ui.tx("editor-saved", &[("time", time)])
                } else {
                    ui.tx("editor-saved-auto", &[])
                }
            }}</span>
            <span class="muted">{width}</span>
        </footer>
    }
}

/// Essais de l'interface dans un navigateur (API Tauri simulée, `dev/mock-tauri.js`) :
/// `window.__revelTest.apply(commande JSON, fusion)` applique une commande comme le ferait un
/// panneau. Jamais installé dans l'application (pas de simulation).
fn install_test_hook(ed: Ed, ui: Ui) {
    use wasm_bindgen::closure::Closure;
    let win = window();
    let mocked = js_sys::Reflect::get(&win, &JsValue::from_str("__TAURI__"))
        .ok()
        .and_then(|t| js_sys::Reflect::get(&t, &JsValue::from_str("__mock")).ok())
        .and_then(|m| m.as_bool())
        == Some(true);
    if !mocked {
        return;
    }
    let apply = Closure::<dyn Fn(String, bool)>::new(move |json: String, coalesce: bool| {
        match serde_json::from_str::<Command>(&json) {
            Ok(cmd) => ed.apply(ui, cmd, coalesce),
            Err(e) => ui.fail(revel_protocol::Failure::detail(e.to_string())),
        }
    });
    // Langue de l'interface (captures et contrôle des libellés en anglais).
    let lang = Closure::<dyn Fn(String)>::new(move |l: String| ui.settings.update(|s| s.lang = l));
    let hook = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&hook, &JsValue::from_str("apply"), apply.as_ref());
    let _ = js_sys::Reflect::set(&hook, &JsValue::from_str("lang"), lang.as_ref());
    apply.forget();
    lang.forget();
    let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__revelTest"), &hook);
}
