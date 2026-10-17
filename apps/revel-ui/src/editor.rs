//! Éditeur visuel (étape 2) : canevas, points de rupture, zoom, sélection, annuler/rétablir.
//!
//! Le document du site est tenu en copie locale : chaque modification passe par une commande
//! envoyée au cœur (D-13) ; la commande effectivement appliquée (`EditState::applied`) est rejouée
//! sur la copie. Le canevas est un cadre d'une autre origine (protocole `revel-canvas`) qui
//! dialogue par `postMessage` (voir `crates/revel-core/src/canvas.js`).

use crate::{Ui, api, icons};
use leptos::ev;
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

/// Champ d'une propriété dans le panneau de styles (après son affichage) : mis en saisie.
fn focus_style_field(prop: String) {
    use wasm_bindgen::JsCast;
    request_animation_frame(move || {
        request_animation_frame(move || {
            let find = |p: &str| {
                document()
                    .query_selector(&format!(
                        ".dock-panel[data-panel=\"styles\"] [data-prop=\"{p}\"]"
                    ))
                    .ok()
                    .flatten()
            };
            let Some(cell) = find(&prop).or_else(|| {
                matches!(prop.as_str(), "column-gap" | "row-gap")
                    .then(|| find("gap"))
                    .flatten()
            }) else {
                return;
            };
            let opts = web_sys::ScrollIntoViewOptions::new();
            opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
            cell.scroll_into_view_with_scroll_into_view_options(&opts);
            if let Ok(Some(b)) = cell.query_selector(".bm-value")
                && let Ok(b) = b.dyn_into::<web_sys::HtmlElement>()
            {
                b.click();
            } else if let Ok(Some(i)) = cell.query_selector("input")
                && let Ok(i) = i.dyn_into::<web_sys::HtmlInputElement>()
            {
                let _ = i.focus();
                i.select();
            }
        })
    });
}

/// Message reçu du canevas.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum FromCanvas {
    Ready {
        path: String,
        #[serde(default)]
        vars: Vec<revel_edit::canvas::VarValue>,
        /// Révision du document rendu.
        #[serde(default)]
        rev: Option<u64>,
    },
    /// Page mise à jour sur place après une modification.
    Refreshed {
        #[serde(default)]
        vars: Vec<revel_edit::canvas::VarValue>,
        #[serde(default)]
        rev: Option<u64>,
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
    /// Maj+clic : élément ajouté à la sélection (ou retiré).
    Toggle {
        id: String,
        #[serde(default)]
        tag: String,
        #[serde(default)]
        label: String,
    },
    /// Poignées de taille : largeur et/ou hauteur (valeurs CSS) ; `reset` : poignée
    /// double-cliquée (`e`, `s`, `se`), valeur retirée.
    Resized {
        id: String,
        #[serde(default)]
        width: Option<String>,
        #[serde(default)]
        height: Option<String>,
        #[serde(default)]
        reset: Option<String>,
    },
    /// Espacements réglés sur le canevas (propriété -> valeur, `None` : retirée).
    Spacing {
        id: String,
        values: std::collections::BTreeMap<String, Option<String>>,
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
    /// Défilement en cours (une fois par image) : règles.
    Scrolled {
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
        /// Sélection à montrer (calques) : vue déplacée si besoin, zoom inchangé.
        #[serde(default)]
        reveal: bool,
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
    /// Textes modifiables de l'élément sélectionné (champ « Texte » du panneau de styles).
    Texts {
        id: Option<String>,
        #[serde(default)]
        items: Vec<TextItem>,
    },
    /// Texte de la page mis en forme sur place (gras, italique, lien, span) : HTML à écrire en
    /// texte et éléments.
    TextHtml {
        slot: revel_edit::canvas::Slot,
        html: String,
    },
    /// Élément de texte mis en forme modifié d'un bloc (D-72) : `id` de l'élément, `slot` d'un
    /// de ses textes (propriétaire), contenu en HTML (éléments repérés par `data-rv`).
    RichText {
        slot: revel_edit::canvas::Slot,
        id: String,
        html: String,
    },
    /// Canevas des composants : hauteur du composant dans ce cadre.
    Stage {
        h: f64,
    },
    /// Double-clic dans une utilisation de composant : canevas du composant, élément pointé
    /// sélectionné.
    OpenComponent {
        component: String,
        #[serde(default)]
        node: Option<String>,
        #[serde(default)]
        tag: String,
    },
    /// Clic sur une poignée d'espacement sans la glisser : champ du panneau de styles.
    SpacingFocus {
        prop: String,
    },
    /// Textes rendus des éléments liés (calques : texte venu des données, sans l'expression).
    Previews {
        #[serde(default)]
        map: std::collections::HashMap<String, String>,
    },
    #[serde(other)]
    Other,
}

/// Texte modifiable de l'élément sélectionné : emplacement, texte affiché (HTML pour un texte
/// riche).
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct TextItem {
    pub slot: revel_edit::canvas::Slot,
    pub text: String,
    #[serde(default)]
    pub rich: bool,
    /// Texte mis en forme modifié d'un bloc : identifiant de l'élément (`text` : son HTML).
    #[serde(default)]
    pub host: Option<String>,
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
    /// Nœud à insérer à la place de l'élément prédéfini (média, composant).
    pub node: Option<revel_model::Node>,
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
/// Vue côte à côte : écart entre deux écrans (pixels de la page) et place de leur nom au-dessus
/// (pixels de l'écran).
pub const SIDE_GAP: f64 = 96.0;
pub const SIDE_LABEL: f64 = 24.0;
/// Vue côte à côte : hauteur des cadres par défaut (pixels de la page, écran d'ordinateur).
pub const SIDE_HEIGHT: f64 = 900.0;

/// Cadre d'un écran dans le canevas : point de rupture, largeur et décalage depuis le premier
/// cadre (pixels de la page).
#[derive(Clone, Debug, PartialEq)]
pub struct FrameBox {
    /// Clé du cadre : point de rupture, ou `@<variante>` dans le canevas des composants
    /// (`@` : variante principale).
    pub bp: String,
    pub w: f64,
    pub ox: f64,
    /// Décalage vertical (variantes d'un composant large, les unes sous les autres).
    pub oy: f64,
}

/// Disposition des cadres : un seul (l'écran choisi) ou tous les écrans côte à côte, du plus
/// large au plus étroit ; largeur totale, hauteur, zoom ajusté, zoom, origine dans la zone.
#[derive(Clone, Debug, PartialEq)]
pub struct World {
    pub frames: Vec<FrameBox>,
    pub total: f64,
    pub h: f64,
    /// Hauteur normale des cadres (sans allongement) : référence des unités `vh` de la page.
    pub base_h: f64,
    pub fit: f64,
    pub s: f64,
    pub x: f64,
    pub y: f64,
}

/// Guides de colonnes affichés sur la page (grille web : colonnes, gouttière, marges, largeur
/// maximale du contenu).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, Deserialize)]
pub struct Guides {
    pub on: bool,
    pub count: u32,
    pub gutter: u32,
    pub margin: u32,
    /// Largeur maximale du contenu (0 : toute la largeur).
    pub max: u32,
}

impl Default for Guides {
    fn default() -> Self {
        Self {
            on: false,
            count: 12,
            gutter: 24,
            margin: 24,
            max: 0,
        }
    }
}

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
    /// Échelle du zoom ajusté (affichée dans la liste des zooms).
    pub fit: RwSignal<f64>,
    pub selected: RwSignal<Option<Selection>>,
    /// Sélection multiple (Maj+clic) : tous les éléments sélectionnés, l'élément principal
    /// (`selected`) compris ; vide pour une sélection simple.
    pub multi: RwSignal<Vec<Selection>>,
    /// Cible du panneau de styles (classe ou combinaison, état), partagée avec les poignées de
    /// taille du canevas.
    pub style_sel: RwSignal<String>,
    pub style_state: RwSignal<String>,
    pub hovered: RwSignal<Option<String>>,
    /// Textes modifiables de l'élément sélectionné (identifiant, textes), envoyés par le canevas.
    pub texts: RwSignal<(Option<String>, Vec<TextItem>)>,
    /// Texte rendu des éléments liés à des données (identifiant, ou « utilisation/identifiant »
    /// dans un composant), envoyé par le canevas à chaque chargement.
    pub previews: RwSignal<std::collections::HashMap<String, String>>,
    /// Libellés des collections (`.pages.yml`, « Actualités »), par identifiant de collection.
    pub col_labels: RwSignal<std::collections::HashMap<String, String>>,
    /// Numéro de chargement du canevas (incrémenté pour le recharger).
    pub load: RwSignal<u64>,
    /// Défilement du canevas, rétabli après un rechargement.
    pub scroll: RwSignal<(f64, f64)>,
    /// Défilement suivi en continu (règles).
    pub live_scroll: RwSignal<(f64, f64)>,
    /// Règles en pixels (Ctrl+R).
    pub rulers: RwSignal<bool>,
    /// Valeurs des affectations `{% set %}` de la page affichée (relevées au rendu du canevas).
    pub vars: RwSignal<Vec<revel_edit::canvas::VarValue>>,
    /// Guides de colonnes.
    pub guides: RwSignal<Guides>,
    /// Valeurs effectives des propriétés de style de l'élément sélectionné (canevas).
    pub computed: RwSignal<std::collections::HashMap<String, String>>,
    /// Nombre d'éléments de la page qui portent la classe réglée (sélecteur, nombre).
    pub count: RwSignal<Option<(String, u32)>>,
    /// Modifications envoyées au cœur et pas encore confirmées.
    pub pending: RwSignal<u32>,
    /// Heure (ms) du dernier enregistrement confirmé.
    pub saved_at: RwSignal<Option<f64>>,
    /// Dernière modification faite par un assistant (« Claude · Texte modifié »), affichée
    /// quelques secondes dans la barre d'état.
    pub assistant: RwSignal<Option<String>>,
    /// Composant ouvert (modification isolée) : celui qui contient l'élément sélectionné.
    pub isolated: RwSignal<Option<String>>,
    /// Canevas des composants (comme Framer) : composant ouvert seul, ses variantes en cadres.
    pub comp: RwSignal<Option<String>>,
    /// Variante modifiée dans le canevas des composants (`""` : principale).
    pub variant: RwSignal<String>,
    /// Hauteur mesurée du composant dans chaque cadre du canevas des composants.
    pub comp_hs: RwSignal<std::collections::HashMap<String, f64>>,
    /// Langue de travail (celle de l'interface du site ouvert, `Ui::locale`).
    pub locale: RwSignal<Option<String>>,
    /// Tous les écrans côte à côte (le cadre de l'écran choisi est actif).
    pub side: RwSignal<bool>,
    /// Canevas libre (déplacer la vue, zoomer, régler la largeur du cadre) ; sinon vue
    /// verrouillée : page calée sur la zone, la molette la fait défiler (comme Webflow).
    pub free: RwSignal<bool>,
    /// Hauteur des cadres réglée à la poignée du bas (pixels de la page) ; `None` : la place
    /// disponible (un écran : zone du canevas ; côte à côte : 900 px).
    pub frame_h: RwSignal<Option<f64>>,
    /// Largeur de la page choisie en glissant le bord du cadre (vue simple) ; `None` : celle de
    /// l'écran affiché.
    pub frame_w: RwSignal<Option<f64>>,
    /// Vue côte à côte : écrans masqués (boutons des écrans de la barre d'outils).
    pub hidden_bps: RwSignal<Vec<String>>,
    /// Palette de commandes ouverte (Ctrl+K).
    pub palette: RwSignal<bool>,
}

/// Paliers de zoom.
pub const ZOOMS: [f64; 9] = [0.25, 0.33, 0.5, 0.67, 0.75, 1.0, 1.25, 1.5, 2.0];

/// Commande qui ne touche que la feuille de style (le canevas recharge alors la feuille seule).
/// Commandes qui ne touchent qu'aux attributs d'éléments (et aux styles) : appliquées en direct
/// dans le canevas, sans rechargement (classes, animations, liens…). Éléments concernés.
fn attr_targets(c: &Command, out: &mut Vec<revel_edit::Target>) -> bool {
    match c {
        Command::SetAttr { at, .. }
        | Command::RemoveAttr { at, .. }
        | Command::InsertAttr { at, .. }
        | Command::RemoveAttrAt { at, .. } => {
            if !out.contains(at) {
                out.push(at.clone());
            }
            true
        }
        Command::Batch { commands } => {
            !commands.is_empty()
                && commands
                    .iter()
                    .all(|c| style_only(c) || attr_targets(c, out))
        }
        _ => false,
    }
}

/// Attributs d'un élément tels qu'écrits (sans condition ni expression) ; `None` si l'un d'eux
/// dépend du rendu (expression, condition) : la page doit alors être rechargée.
fn literal_attrs(
    d: &revel_model::Site,
    at: &revel_edit::Target,
) -> Option<Vec<(String, Option<String>)>> {
    match revel_edit::address::node(d, at).ok()? {
        // Utilisation de composant : attributs posés sur l'élément racine (une classe s'ajoute à
        // celles du composant : page rechargée).
        revel_model::Node::Use { attrs, .. } if attrs.iter().any(|a| a.name == "class") => None,
        revel_model::Node::Element { attrs, .. } | revel_model::Node::Use { attrs, .. } => attrs
            .iter()
            .map(|a| {
                let dynamic = a.when.is_some()
                    || a.value
                        .as_deref()
                        .is_some_and(|v| v.contains("{{") || v.contains("{%"));
                (!dynamic).then(|| (a.name.clone(), a.value.clone()))
            })
            .collect(),
        _ => None,
    }
}

/// Documents qui s'affichent autrement dans le canevas (réglages sans effet sur la page ignorés :
/// transition entre les pages, référencement, formulaires, redirections, rangement des panneaux).
fn looks_different(a: &revel_model::Site, b: &revel_model::Site) -> bool {
    let plain = |s: &revel_model::Site| {
        let mut s = s.clone();
        s.settings.page_transition = None;
        s.settings.seo = Default::default();
        s.settings.forms = Default::default();
        s.redirects.clear();
        s.library = Default::default();
        s
    };
    plain(a) != plain(b)
}

pub fn style_only(c: &Command) -> bool {
    match c {
        Command::SetDecl { .. }
        | Command::InsertDecl { .. }
        | Command::RemoveDeclAt { .. }
        | Command::SetSelector { .. }
        | Command::InsertStyle { .. }
        | Command::RemoveStyleAt { .. }
        | Command::RemoveRule { .. }
        | Command::AddRule { .. }
        | Command::ResizeBreakpoint { .. } => true,
        Command::Batch { commands } => !commands.is_empty() && commands.iter().all(style_only),
        _ => false,
    }
}

impl Ed {
    /// Message au canevas : à tous les cadres (sélection, survol, feuille de style…), ou au seul
    /// cadre actif pour ce qui concerne l'écran choisi (aperçu d'un geste, valeurs calculées,
    /// mesures, glisser, défilement).
    pub fn post(self, msg: serde_json::Value) {
        let only_active = matches!(
            msg.get("type").and_then(|t| t.as_str()),
            Some(
                "preview"
                    | "count"
                    | "nav"
                    | "measure"
                    | "drag"
                    | "watch"
                    | "scroll"
                    | "spacing"
                    | "animate"
            )
        );
        if only_active {
            if let Some(f) = self.frame_el() {
                post_to(&f, msg);
            }
        } else {
            for (_, f) in self.frames_el() {
                post_to(&f, msg.clone());
            }
        }
    }

    /// Cadre actif du canevas (l'écran choisi).
    pub fn frame_el(self) -> Option<web_sys::HtmlIFrameElement> {
        use wasm_bindgen::JsCast;
        document()
            .query_selector(".canvas-area iframe.canvas-frame.is-active")
            .ok()
            .flatten()?
            .dyn_into()
            .ok()
    }

    /// Cadres du canevas et leur écran (vue côte à côte : un par écran).
    pub fn frames_el(self) -> Vec<(String, web_sys::HtmlIFrameElement)> {
        use wasm_bindgen::JsCast;
        let Ok(list) = document().query_selector_all(".canvas-area iframe.canvas-frame") else {
            return Vec::new();
        };
        (0..list.length())
            .filter_map(|i| {
                let f: web_sys::HtmlIFrameElement = list.item(i)?.dyn_into().ok()?;
                let key = f
                    .get_attribute("data-key")
                    .or_else(|| f.get_attribute("data-bp"))
                    .unwrap_or_default();
                Some((key, f))
            })
            .collect()
    }

    /// La vue se déplace et se zoome : canevas libre, ou canevas des composants (ses variantes
    /// s'étendent au-delà de la zone).
    pub fn navigable(self) -> bool {
        self.free.get() || self.comp.get().is_some()
    }

    /// Canevas libre ou vue verrouillée. Verrouillée : zoom ajusté, cadre à sa largeur et sa
    /// hauteur normales, un seul écran.
    pub fn set_free(self, on: bool) {
        if self.free.get_untracked() == on {
            return;
        }
        self.free.set(on);
        crate::pref_set("free", if on { "1" } else { "0" });
        if !on {
            if self.side.get_untracked() {
                self.set_side(false);
            }
            self.zoom.set(None);
            self.frame_w.set(None);
            self.frame_h.set(None);
        }
    }

    /// Tous les écrans côte à côte, ou l'écran choisi seul (zoom ajusté à la nouvelle vue).
    pub fn set_side(self, on: bool) {
        if self.side.get_untracked() == on {
            return;
        }
        // Plusieurs écrans : la vue se déplace (canevas libre).
        if on {
            self.set_free(true);
        }
        self.side.set(on);
        self.zoom.set(None);
        self.frame_h.set(None);
        crate::pref_set("side", if on { "1" } else { "0" });
    }

    /// Vue côte à côte : écran affiché ou masqué (au moins un reste affiché ; l'écran choisi
    /// masqué cède la place au premier affiché).
    pub fn toggle_bp(self, id: &str) {
        let all: Vec<String> = self.doc.with_untracked(|d| {
            d.as_ref()
                .map(|d| d.breakpoints.iter().map(|b| b.id.clone()).collect())
                .unwrap_or_default()
        });
        let mut hidden = self.hidden_bps.get_untracked();
        if let Some(i) = hidden.iter().position(|h| h == id) {
            hidden.remove(i);
        } else {
            let shown: Vec<&String> = all.iter().filter(|b| !hidden.contains(b)).collect();
            if shown.len() <= 1 {
                return;
            }
            hidden.push(id.to_string());
            if self.breakpoint.get_untracked() == id
                && let Some(next) = all.iter().find(|b| !hidden.contains(b))
            {
                self.breakpoint.set(next.clone());
            }
        }
        hidden.retain(|h| all.contains(h));
        if let Ok(j) = serde_json::to_string(&hidden) {
            crate::pref_set("side_hidden", &j);
        }
        self.hidden_bps.set(hidden);
    }

    /// Hauteur normale des cadres transmise au canevas quand ils sont allongés (unités `vh`
    /// inchangées), retirée sinon.
    pub fn post_viewport(self) {
        let w = untrack(|| self.world());
        let h = if self.frame_h.get_untracked().is_some() && (w.h - w.base_h).abs() > 0.5 {
            w.base_h
        } else {
            0.0
        };
        self.post(serde_json::json!({ "type": "viewport", "h": h }));
    }

    /// Autre langue affichée (langue de travail, si le site l'a encore).
    pub fn lang(self) -> Option<String> {
        let code = self.locale.get()?;
        self.doc
            .with(|d| d.as_ref()?.locale(&code).map(|l| l.code.clone()))
    }

    /// Adresse d'une page dans la langue affichée (`/agence/` -> `/en/agence/`), sauf pour une
    /// page qui n'existe que dans la langue principale (404).
    pub fn localized(self, page: &str) -> String {
        match self.lang() {
            Some(code) if page.starts_with('/') && revel_model::localizable_path(page) => {
                format!("/{code}{page}")
            }
            _ => page.to_string(),
        }
    }

    /// Adresse du canevas pour la page affichée.
    pub fn src(self) -> Option<String> {
        let base = self.base.get()?;
        let page = self.localized(&self.page.get());
        Some(format!(
            "{}{}?rv={}",
            base,
            page.trim_start_matches('/'),
            self.load.get()
        ))
    }

    /// Page modifiée : mise à jour sur place dans chaque cadre du canevas, sans rechargement ni
    /// animation rejouée (le canevas recharge lui-même la page s'il ne peut pas la mettre à jour).
    pub fn refresh(self) {
        self.post(serde_json::json!({ "type": "refresh" }));
    }

    /// Rendu antérieur à la dernière modification connue de l'éditeur.
    fn stale(self, rev: Option<u64>) -> bool {
        let now = self
            .state
            .with_untracked(|s| s.as_ref().map(|s| s.revision));
        matches!((rev, now), (Some(r), Some(n)) if r < n)
    }

    /// Après une modification, une annulation ou un rétablissement : copie locale et canevas.
    pub fn after(self, ui: Ui, st: EditState) {
        if let Some(cmd) = &st.applied {
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
                self.refresh();
                self.state.set(Some(st));
                return;
            }
            // Contenu de collection écrit par le cœur (vue CMS, design propre, annulation) :
            // document, pages et canevas relus.
            if writes_entries(cmd) {
                self.reopen(ui);
                self.refresh();
                self.state.set(Some(st));
                return;
            }
            // Schéma des collections (`.pages.yml`) : sans effet sur les pages.
            if matches!(
                cmd,
                Command::SetCmsField { .. }
                    | Command::AddCmsField { .. }
                    | Command::RemoveCmsField { .. }
                    | Command::MoveCmsField { .. }
                    | Command::SetCmsCollection { .. }
                    | Command::WriteCmsConfig { .. }
            ) {
                self.state.set(Some(st));
                return;
            }
            // Élément ajouté (copie, enveloppe, insertion) : sélectionné. Mise en forme d'un
            // texte (le texte remplacé par des morceaux, gras, retour à la ligne) : sélection
            // gardée.
            let text_format = matches!(cmd, Command::Batch { commands }
                if matches!(commands.first(), Some(Command::Replace { .. })));
            if !text_format
                && let Some(n) = inserted(cmd)
                && let Some(id) = revel_model::ids::node_id(n)
            {
                self.selected.set(Some(Selection {
                    id: id.to_string(),
                    tag: node_tag(n),
                    label: node_label(ui, n),
                }));
            }
            // Attributs avant la modification (application en direct dans le canevas).
            let mut targets = Vec::new();
            let attr_cmd = attr_targets(cmd, &mut targets);
            let before: Option<Vec<_>> = attr_cmd
                .then(|| {
                    self.doc.with_untracked(|d| {
                        let d = d.as_ref()?;
                        targets.iter().map(|t| literal_attrs(d, t)).collect()
                    })
                })
                .flatten();
            let mut ok = true;
            self.doc.update(|d| {
                if let Some(d) = d {
                    ok = revel_edit::apply(d, cmd).is_ok();
                }
            });
            // Différences d'attributs à poser dans le canevas : `None` si une valeur dépend du
            // rendu.
            let live: Option<Vec<serde_json::Value>> = before.and_then(|before| {
                self.doc.with_untracked(|d| {
                    let d = d.as_ref()?;
                    targets
                        .iter()
                        .zip(before)
                        .map(|(t, b)| {
                            let a = literal_attrs(d, t)?;
                            let mut set = serde_json::Map::new();
                            for (n, v) in &a {
                                if !b.iter().any(|(m, w)| m == n && w == v) {
                                    set.insert(
                                        n.clone(),
                                        v.clone().map(Into::into).unwrap_or_else(|| "".into()),
                                    );
                                }
                            }
                            let removed: Vec<String> = b
                                .iter()
                                .filter(|(n, _)| !a.iter().any(|(m, _)| m == n))
                                .map(|(n, _)| n.clone())
                                .collect();
                            Some(serde_json::json!({ "id": t.node, "set": set, "remove": removed }))
                        })
                        .collect()
                })
            });
            if matches!(cmd, Command::SetSiteInfo { .. })
                && let Some(d) = self.doc.read_untracked().as_ref()
            {
                ui.site_identity(d);
            }
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
            if matches!(
                cmd,
                Command::SetBreakpoints { .. }
                    | Command::SetShelf { .. }
                    | Command::SetRedirects { .. }
                    | Command::SetSeoSettings { .. }
                    | Command::SetFormSettings { .. }
                    | Command::SetPageTransition { .. }
            ) {
                // Points de rupture, rangement : réglages de l'éditeur, la page ne change pas.
            } else if style_only(cmd) {
                if let Some(url) = self
                    .doc
                    .with_untracked(|d| d.as_ref()?.stylesheet.as_ref().map(|s| s.url.clone()))
                {
                    self.post(serde_json::json!({ "type": "stylesheet", "url": url }));
                }
            } else if let Some(changes) = live.filter(|_| ok) {
                // Classes, animations, liens… : posés en direct, sans recharger la page (ni rejouer
                // ses animations).
                self.post(serde_json::json!({ "type": "attrs", "changes": changes }));
                if let Some(url) = self
                    .doc
                    .with_untracked(|d| d.as_ref()?.stylesheet.as_ref().map(|s| s.url.clone()))
                {
                    self.post(serde_json::json!({ "type": "stylesheet", "url": url }));
                }
            } else {
                self.refresh();
            }
        }
        self.state.set(Some(st));
    }

    /// Document, historique et liste des pages relus (fichiers modifiés hors de l'éditeur : vue
    /// CMS, Pages CMS, éditeur de texte).
    fn reopen(self, ui: Ui) {
        self.reopen_with(ui, None);
    }

    /// Relecture suivie du rechargement du canevas : toujours (`Some(true)` : fichiers du site
    /// changés), ou seulement si la page peut s'afficher autrement (`Some(false)` : document
    /// modifié par une autre vue ; une transition entre les pages ou des redirections ne
    /// rechargent pas la page).
    fn reopen_with(self, ui: Ui, canvas: Option<bool>) {
        let path = self.path.get_value();
        spawn_local(async move {
            match api::edit_open(&path).await {
                Ok(o) => {
                    let reload = match canvas {
                        Some(true) => true,
                        Some(false) => self.doc.with_untracked(|d| {
                            d.as_ref().is_none_or(|d| looks_different(d, &o.site))
                        }),
                        None => false,
                    };
                    ui.site_identity(&o.site);
                    self.doc.set(Some(o.site));
                    self.state.set(Some(o.state));
                    if reload {
                        self.refresh();
                    }
                }
                Err(e) => ui.fail(e),
            }
            if let Ok(p) = api::edit_pages(&path).await {
                // Page demandée (nouvelle page, adresse changée), sinon la page affichée si elle
                // existe encore, sinon l'accueil.
                let want = ui
                    .goto_page
                    .get_untracked()
                    .filter(|u| p.iter().any(|x| &x.url == u));
                if let Some(u) = want {
                    ui.goto_page.set(None);
                    self.page.set(u);
                    self.scroll.set((0.0, 0.0));
                    self.selected.set(None);
                } else if !p.iter().any(|x| x.url == self.page.get_untracked())
                    && let Some(first) = p.iter().find(|x| x.url == "/").or(p.first())
                {
                    self.page.set(first.url.clone());
                }
                self.pages.set(p);
            }
        });
    }

    /// Raccourci clavier de l'éditeur (interface ou canevas) ; `true` s'il a été traité.
    pub fn shortcut(self, ui: Ui, k: &Key) -> bool {
        let key = k.key.to_lowercase();
        let digit = k.digit();
        match (key.as_str(), k.ctrl) {
            ("z", true) if k.shift => self.redo(ui),
            ("z", true) => self.undo(ui),
            ("y", true) => self.redo(ui),
            // Palette de commandes (le canevas transmet Ctrl+K).
            ("k", true) if !k.shift => self.palette.set(true),
            ("d", true) => return self.duplicate_selection(ui),
            // Envelopper dans un conteneur (Ctrl+G, comme Figma et Framer) ; Ctrl+Maj+G : retirer
            // l'enveloppe (les enfants prennent sa place).
            ("g", true) if !k.shift => return self.wrap_selection(ui),
            ("g", true) => return self.unwrap(ui),
            // Règles en pixels (Ctrl+R, comme Figma ; le rechargement de la fenêtre est neutralisé).
            ("r", true) if !k.shift => {
                let on = !self.rulers.get_untracked();
                self.rulers.set(on);
                crate::pref_set("rulers", if on { "1" } else { "0" });
            }
            // Copier, couper, coller des éléments.
            ("c", true) if !k.shift => return self.copy(ui, false),
            ("x", true) if !k.shift => return self.copy(ui, true),
            ("v", true) if !k.shift => return self.paste(ui),
            ("delete" | "backspace", false) => return self.remove_selection(ui),
            ("escape", false) => {
                self.selected.set(None);
                self.multi.set(Vec::new());
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
    pub(crate) fn selected_node(self) -> Option<(revel_edit::Target, revel_model::Node)> {
        let at = self.selected_target()?;
        let n = self
            .doc
            .with_untracked(|d| revel_edit::address::node(d.as_ref()?, &at).ok().cloned())?;
        Some((at, n))
    }

    /// Copie les éléments sélectionnés (et les retire avec `cut`).
    pub fn copy(self, ui: Ui, cut: bool) -> bool {
        let targets = self.targets();
        let nodes: Vec<revel_model::Node> = self.doc.with_untracked(|d| {
            let Some(d) = d.as_ref() else {
                return Vec::new();
            };
            targets
                .iter()
                .filter_map(|at| revel_edit::address::node(d, at).ok().cloned())
                .collect()
        });
        if nodes.is_empty() {
            return false;
        }
        ui.clipboard.set(nodes);
        if cut {
            self.remove_selection(ui);
        }
        true
    }

    /// Colle l'élément copié (nouveaux identifiants) : à côté d'un élément de même balise
    /// sélectionné, sinon dans l'élément sélectionné (ou après lui), sinon à la fin de la page.
    pub fn paste(self, ui: Ui) -> bool {
        let mut nodes = ui.clipboard.get_untracked();
        let Some(node) = nodes.first().cloned() else {
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
                for n in &mut nodes {
                    revel_edit::make::refresh_ids(n, &mut taken);
                }
            }
        });
        let mut commands: Vec<Command> = nodes
            .into_iter()
            .enumerate()
            .map(|(i, node)| Command::Insert {
                parent: parent.clone(),
                index: index + i,
                node,
            })
            .collect();
        let cmd = if commands.len() == 1 {
            commands.remove(0)
        } else {
            Command::Batch { commands }
        };
        self.apply(ui, cmd, false);
        true
    }

    /// Ajoute l'élément à la sélection ou l'en retire (Maj+clic) ; le dernier ajouté devient
    /// l'élément principal (panneaux).
    pub fn toggle_select(self, s: Selection) {
        let Some(primary) = self.selected.get_untracked() else {
            self.post(serde_json::json!({ "type": "select", "id": s.id }));
            self.selected.set(Some(s));
            return;
        };
        let mut all = self.multi.get_untracked();
        if all.is_empty() {
            all.push(primary);
        }
        match all.iter().position(|x| x.id == s.id) {
            Some(i) => {
                all.remove(i);
            }
            None => all.push(s),
        }
        let next = all.last().cloned();
        if all.len() <= 1 {
            all.clear();
        }
        self.multi.set(all);
        self.post(
            serde_json::json!({ "type": "select", "id": next.as_ref().map(|n| n.id.clone()) }),
        );
        self.selected.set(next);
        self.post_multi();
    }

    /// Autres éléments de la sélection multiple, transmis au canevas.
    pub fn post_multi(self) {
        let primary = self.selected.get_untracked().map(|s| s.id);
        let ids: Vec<String> = self
            .multi
            .get_untracked()
            .into_iter()
            .map(|s| s.id)
            .filter(|i| Some(i) != primary.as_ref())
            .collect();
        self.post(serde_json::json!({ "type": "multi", "ids": ids }));
    }

    /// Éléments sélectionnés (sélection multiple, sinon l'élément sélectionné), dans l'ordre du
    /// document, sans ceux contenus dans un autre élément sélectionné.
    pub fn targets(self) -> Vec<revel_edit::Target> {
        let ids: Vec<String> = {
            let m = self.multi.get_untracked();
            if m.is_empty() {
                self.selected
                    .get_untracked()
                    .into_iter()
                    .map(|s| s.id)
                    .collect()
            } else {
                m.into_iter().map(|s| s.id).collect()
            }
        };
        self.doc.with_untracked(|d| {
            let Some(d) = d.as_ref() else {
                return Vec::new();
            };
            let mut order: Vec<String> = Vec::new();
            for (_, _, body) in d.owners() {
                revel_model::ids::walk(body, &mut |n| {
                    if let Some(i) = revel_model::ids::node_id(n)
                        && ids.iter().any(|x| x == i)
                    {
                        order.push(i.to_string());
                    }
                });
            }
            let inside = |outer: &str, id: &str| {
                target_of(d, outer)
                    .and_then(|t| revel_edit::address::node(d, &t).ok())
                    .is_some_and(|n| {
                        let mut found = false;
                        for list in revel_model::ids::child_lists(n) {
                            revel_model::ids::walk(list, &mut |c| {
                                found |= revel_model::ids::node_id(c) == Some(id);
                            });
                        }
                        found
                    })
            };
            order
                .iter()
                .filter(|id| !order.iter().any(|o| o != *id && inside(o, id)))
                .filter_map(|id| target_of(d, id))
                .collect()
        })
    }

    /// Supprime les éléments sélectionnés (une seule étape d'annulation).
    pub fn remove_selection(self, ui: Ui) -> bool {
        let mut commands: Vec<Command> = self
            .targets()
            .into_iter()
            .map(|at| Command::Remove { at })
            .collect();
        match commands.len() {
            0 => false,
            1 => {
                self.apply(ui, commands.remove(0), false);
                true
            }
            _ => {
                self.multi.set(Vec::new());
                self.apply(ui, Command::Batch { commands }, false);
                true
            }
        }
    }

    /// Duplique les éléments sélectionnés (une seule étape d'annulation).
    pub fn duplicate_selection(self, ui: Ui) -> bool {
        let mut commands: Vec<Command> = self
            .targets()
            .into_iter()
            .map(|at| Command::Duplicate { at })
            .collect();
        match commands.len() {
            0 => false,
            1 => {
                self.apply(ui, commands.remove(0), false);
                true
            }
            _ => {
                self.apply(ui, Command::Batch { commands }, false);
                true
            }
        }
    }

    /// Enveloppe la sélection : un élément, ou plusieurs éléments voisins (même parent), placés
    /// ensemble dans un nouveau conteneur à la place du premier.
    pub fn wrap_selection(self, ui: Ui) -> bool {
        let targets = self.targets();
        match targets.len() {
            0 => false,
            1 => {
                self.wrap(ui, targets[0].clone());
                true
            }
            _ => {
                let plan = self.doc.with_untracked(|d| {
                    let d = d.as_ref()?;
                    let mut places = Vec::new();
                    for t in &targets {
                        places.push(revel_edit::address::locate(d, t).ok()?);
                    }
                    let parent = places[0].0.clone();
                    if places.iter().any(|(p, _)| *p != parent) {
                        return None;
                    }
                    let first = places.iter().map(|(_, i)| *i).min()?;
                    let mut wrapper = revel_model::Node::Element {
                        id: None,
                        tag: "div".into(),
                        attrs: Vec::new(),
                        children: Vec::new(),
                    };
                    let mut taken = d.node_ids();
                    revel_edit::make::refresh_ids(&mut wrapper, &mut taken);
                    let wid = revel_model::ids::node_id(&wrapper)?.to_string();
                    Some((parent, first, wrapper, wid))
                });
                let Some((parent, first, wrapper, wid)) = plan else {
                    ui.fail(revel_protocol::Failure::new("editor-wrap-siblings"));
                    return true;
                };
                let into = revel_edit::Parent {
                    owner: parent.owner.clone(),
                    node: Some(wid.clone()),
                    branch: Default::default(),
                };
                let mut commands = vec![Command::Insert {
                    parent,
                    index: first,
                    node: wrapper,
                }];
                for (i, at) in targets.into_iter().enumerate() {
                    commands.push(Command::Move {
                        at,
                        to: into.clone(),
                        index: i,
                    });
                }
                self.multi.set(Vec::new());
                self.apply(ui, Command::Batch { commands }, false);
                true
            }
        }
    }

    /// Taille réglée aux poignées du canevas (ou retirée par un double-clic) : écrite dans la
    /// classe réglée du panneau de styles, sinon la première classe de l'élément, au point de
    /// rupture affiché ; un élément sans classe en reçoit une (comme Webflow).
    pub fn resize(
        self,
        ui: Ui,
        id: &str,
        width: Option<String>,
        height: Option<String>,
        reset: Option<String>,
    ) {
        let release = move || self.post(serde_json::json!({ "type": "preview", "css": "" }));
        let Some(at) = self.selected_target().filter(|t| t.node == id) else {
            return release();
        };
        let Some(node) = self
            .doc
            .with_untracked(|d| revel_edit::address::node(d.as_ref()?, &at).ok().cloned())
        else {
            return release();
        };
        let list: Vec<(String, Option<String>)> = match reset.as_deref() {
            Some("e") => vec![("width".into(), None)],
            Some("s") => vec![("height".into(), None)],
            Some(_) => vec![("width".into(), None), ("height".into(), None)],
            None => [("width", width), ("height", height)]
                .into_iter()
                .filter_map(|(p, v)| v.map(|v| (p.to_string(), Some(v))))
                .collect(),
        };
        self.set_props(ui, at, node, list);
    }

    /// Propriétés de l'élément `at` écrites dans sa classe (point de rupture et état affichés) ;
    /// élément sans classe : classe créée d'après sa nature. Taille et espacements réglés sur le
    /// canevas.
    pub fn set_props(
        self,
        ui: Ui,
        at: revel_edit::Target,
        node: revel_model::Node,
        list: Vec<(String, Option<String>)>,
    ) {
        let release = move || self.post(serde_json::json!({ "type": "preview", "css": "" }));
        let removing = list.iter().all(|(_, v)| v.is_none());
        let bp = self.breakpoint.get_untracked();
        let media = self.doc.with_untracked(|d| {
            d.as_ref()?
                .breakpoints
                .iter()
                .find(|b| b.id == bp)
                .and_then(|b| b.media())
        });
        let classes = crate::styles::classes_of(&node);
        let cands = crate::styles::candidates(&classes);
        let cur = self.style_sel.get_untracked();
        // Choix « toutes les balises » (élément sans classe) gardé.
        let tag = classes
            .is_empty()
            .then(|| crate::styles::tag_selector(&node))
            .flatten()
            .filter(|(t, _)| *t == cur);
        let (sel, state) = if cands.contains(&cur) || tag.is_some() {
            (cur, self.style_state.get_untracked())
        } else if let Some(c) = cands.last() {
            (c.clone(), String::new())
        } else {
            // Élément sans classe : classe créée d'après sa nature, réglée avec la taille.
            if removing || list.is_empty() {
                return release();
            }
            let Some(name) = self
                .doc
                .with_untracked(|d| d.as_ref().map(|d| crate::styles::new_class(d, &node)))
            else {
                return release();
            };
            let Some(set_class) =
                crate::styles::class_command(at, &node, std::slice::from_ref(&name))
            else {
                return release();
            };
            let decls = list
                .into_iter()
                .filter_map(|(prop, value)| {
                    Some(revel_model::Decl {
                        prop,
                        value: value?,
                        important: false,
                    })
                })
                .collect();
            // État choisi (survol…) gardé : la règle le porte.
            let state = self.style_state.get_untracked();
            self.style_sel.set(format!(".{name}"));
            self.apply(
                ui,
                Command::Batch {
                    commands: vec![
                        set_class,
                        Command::AddRule {
                            selector: format!(".{name}{state}"),
                            media,
                            decls,
                        },
                    ],
                },
                false,
            );
            return;
        };
        let full = format!("{sel}{state}");
        let rule = self
            .doc
            .with_untracked(|d| crate::styles::find_rule(d.as_ref()?, &full, &media));
        match crate::styles::style_command(rule, full, media, list) {
            Some(cmd) => self.apply(ui, cmd, false),
            None => release(),
        }
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
            crate::layers::step_command(&rows, row, dir).or_else(|| {
                // Pas d'autre calque de ce côté, mais un texte (texte lié au contenu, par
                // exemple) : l'élément passe de l'autre côté du texte.
                let at = self.selected_target()?;
                let (parent, idx) = revel_edit::address::locate(d, &at).ok()?;
                let len = revel_edit::address::list(d, &parent).ok()?.len();
                let index = if dir < 0 {
                    idx.checked_sub(1)?
                } else if idx + 1 < len {
                    idx + 1
                } else {
                    return None;
                };
                Some(Command::Move {
                    at,
                    to: parent,
                    index,
                })
            })
        });
        match cmd {
            Some(c) => {
                self.apply(ui, c, false);
                true
            }
            None => false,
        }
    }

    /// Disposition des cadres (lecture suivie : à appeler hors réaction avec `untrack`).
    pub fn world(self) -> World {
        let (aw, ah) = self.area.get();
        let comp = self.comp.get();
        let side = self.side.get() && comp.is_none();
        let active = self.breakpoint.get();
        if let Some(c) = comp {
            return self.comp_world(&c, aw);
        }
        let frames: Vec<FrameBox> = self.doc.with(|d| {
            let bps = d
                .as_ref()
                .map(|d| d.breakpoints.clone())
                .unwrap_or_default();
            let max_max = bps.iter().filter_map(|b| b.max_width).max();
            let min_min = bps.iter().filter_map(|b| b.min_width).min();
            let desktop_min = max_max.map(|w| f64::from(w) + 1.0).unwrap_or(320.0);
            if !side {
                let w = self.frame_w.get().unwrap_or_else(|| {
                    bps.iter()
                        .find(|b| b.id == active)
                        .and_then(|b| b.max_width.or(b.min_width))
                        .map(f64::from)
                        .unwrap_or_else(|| (aw - GUTTER).max(desktop_min))
                });
                return vec![FrameBox {
                    bp: active.clone(),
                    w,
                    ox: 0.0,
                    oy: 0.0,
                }];
            }
            // Ordinateur (base) : 1280 px, entre les largeurs des autres écrans.
            let hi = min_min.map(|w| f64::from(w) - 1.0).unwrap_or(f64::INFINITY);
            let base_w = 1280f64.min(hi).max(desktop_min);
            let hidden = self.hidden_bps.get();
            let mut list: Vec<_> = bps
                .into_iter()
                .filter(|b| b.id == active || !hidden.contains(&b.id))
                .collect();
            list.sort_by_key(|b| match (b.min_width, b.max_width) {
                (Some(w), _) => (0, -i64::from(w)),
                (None, None) => (1, 0),
                (None, Some(w)) => (2, -i64::from(w)),
            });
            let mut ox = 0.0;
            list.into_iter()
                .map(|b| {
                    let w = b.min_width.or(b.max_width).map(f64::from).unwrap_or(base_w);
                    let f = FrameBox {
                        bp: b.id,
                        w,
                        ox,
                        oy: 0.0,
                    };
                    ox += w + SIDE_GAP;
                    f
                })
                .collect()
        });
        let total = frames.last().map(|f| f.ox + f.w).unwrap_or(320.0);
        let label = if side { SIDE_LABEL } else { 0.0 };
        let fit = ((aw - GUTTER) / total).clamp(0.05, 1.0);
        let base_h = if side {
            SIDE_HEIGHT
        } else {
            ((ah - GUTTER - label) / fit).max(200.0)
        };
        let h = self.frame_h.get().unwrap_or(base_h);
        match self.zoom.get() {
            None => World {
                frames,
                total,
                h,
                base_h,
                fit,
                s: fit,
                x: ((aw - total * fit) / 2.0).max(GUTTER / 2.0),
                y: GUTTER / 2.0 + label,
            },
            Some(z) => {
                let (x, y) = self.pan.get();
                World {
                    frames,
                    total,
                    h,
                    base_h,
                    fit,
                    s: z,
                    x,
                    y,
                }
            }
        }
    }

    /// Géométrie du cadre actif (lecture suivie : à appeler hors réaction avec `untrack`).
    pub fn geo(self) -> Geo {
        let w = self.world();
        let active = self.active_key();
        let (fw, ox, oy) = w
            .frames
            .iter()
            .find(|f| f.bp == active)
            .or(w.frames.first())
            .map(|f| (f.w, f.ox, f.oy))
            .unwrap_or((320.0, 0.0, 0.0));
        Geo {
            w: fw,
            h: self.frame_height(&w),
            fit: w.fit,
            s: w.s,
            x: w.x + ox * w.s,
            y: w.y + oy * w.s,
        }
    }

    /// Clé du cadre actif : écran choisi, ou `@<variante>` dans le canevas des composants.
    pub fn active_key(self) -> String {
        if self.comp.get().is_some() {
            format!("@{}", self.variant.get())
        } else {
            self.breakpoint.get()
        }
    }

    /// Cadre choisi par sa clé (clic dans un cadre ou sur son nom).
    pub fn activate_key(self, key: &str) {
        match key.strip_prefix('@') {
            Some(v) => {
                if self.variant.get_untracked() != v {
                    self.variant.set(v.to_string());
                }
            }
            None => {
                if self.breakpoint.get_untracked() != key {
                    self.breakpoint.set(key.to_string());
                }
            }
        }
    }

    /// Hauteur d'un cadre (canevas des composants : hauteur du composant).
    pub fn frame_height(self, w: &World) -> f64 {
        if self.comp.get().is_some() {
            self.comp_frame_h()
        } else {
            w.h
        }
    }

    /// Hauteur des cadres du canevas des composants : la plus grande hauteur mesurée.
    pub fn comp_frame_h(self) -> f64 {
        self.comp_hs
            .with(|m| m.values().copied().fold(0.0f64, f64::max))
            .clamp(120.0, 4000.0)
    }

    /// Variantes affichées du composant `id` : clé du cadre, nom, variante.
    pub fn comp_frames(self, id: &str) -> Vec<(String, Option<revel_model::Variant>)> {
        self.doc.with(|d| {
            let mut v = vec![("@".to_string(), None)];
            if let Some(c) = d.as_ref().and_then(|d| d.component(id)) {
                v.extend(
                    c.variants
                        .iter()
                        .map(|x| (format!("@{}", x.id), Some(x.clone()))),
                );
            }
            v
        })
    }

    /// Disposition du canevas des composants : un cadre par variante, à la largeur de l'écran
    /// choisi ; côte à côte pour un composant étroit, les uns sous les autres sinon.
    fn comp_world(self, id: &str, aw: f64) -> World {
        let bp_w = self.doc.with(|d| {
            let id = self.breakpoint.get();
            d.as_ref()?
                .breakpoints
                .iter()
                .find(|b| b.id == id)
                .and_then(|b| b.max_width.or(b.min_width))
                .map(f64::from)
        });
        let fw = bp_w.unwrap_or(1280.0);
        let fh = self.comp_frame_h();
        let keys = self.comp_frames(id);
        let stacked = fw > 700.0;
        let mut frames = Vec::new();
        for (i, (k, _)) in keys.iter().enumerate() {
            let i = i as f64;
            frames.push(FrameBox {
                bp: k.clone(),
                w: fw,
                ox: if stacked { 0.0 } else { i * (fw + SIDE_GAP) },
                oy: if stacked { i * (fh + SIDE_GAP) } else { 0.0 },
            });
        }
        let n = frames.len().max(1) as f64;
        let total = if stacked {
            fw
        } else {
            n * fw + (n - 1.0) * SIDE_GAP
        };
        let h = if stacked {
            n * fh + (n - 1.0) * SIDE_GAP
        } else {
            fh
        };
        let fit = ((aw - GUTTER) / total).clamp(0.05, 1.0);
        match self.zoom.get() {
            None => World {
                frames,
                total,
                h,
                base_h: fh,
                fit,
                s: fit,
                x: ((aw - total * fit) / 2.0).max(GUTTER / 2.0),
                y: GUTTER / 2.0 + SIDE_LABEL,
            },
            Some(z) => {
                let (x, y) = self.pan.get();
                World {
                    frames,
                    total,
                    h,
                    base_h: fh,
                    fit,
                    s: z,
                    x,
                    y,
                }
            }
        }
    }

    /// Ouvre le canevas d'un composant (variante principale) ; `node` : élément sélectionné.
    pub fn open_comp(self, id: &str, node: Option<(String, String)>) {
        self.comp_hs.set(Default::default());
        self.variant.set(String::new());
        self.zoom.set(None);
        self.comp.set(Some(id.to_string()));
        match node {
            Some((nid, tag)) => self.selected.set(Some(Selection {
                id: nid,
                tag: tag.clone(),
                label: tag,
            })),
            None => self.selected.set(None),
        }
    }

    /// Retour à la page depuis le canevas des composants.
    pub fn close_comp(self) {
        if self.comp.get_untracked().is_none() {
            return;
        }
        self.comp.set(None);
        self.variant.set(String::new());
        self.zoom.set(None);
        self.selected.set(None);
    }

    /// Origine (dans la zone) du cadre d'un écran.
    pub fn frame_origin(self, bp: &str) -> (f64, f64, f64) {
        let w = untrack(|| self.world());
        let (ox, oy) = w
            .frames
            .iter()
            .find(|f| f.bp == bp)
            .map(|f| (f.ox, f.oy))
            .unwrap_or((0.0, 0.0));
        (w.x + ox * w.s, w.y + oy * w.s, w.s)
    }

    /// Origine des cadres gardée telle qu'une partie reste visible.
    fn clamp_pan(self, w: &World, s: f64, x: f64, y: f64) -> (f64, f64) {
        let (aw, ah) = self.area.get_untracked();
        let keep = 80.0;
        (
            x.clamp(keep - w.total * s, (aw - keep).max(keep - w.total * s)),
            y.clamp(keep - w.h * s, (ah - keep).max(keep - w.h * s)),
        )
    }

    /// Zoom en vigueur (ajusté compris).
    pub fn scale(self) -> f64 {
        untrack(|| self.geo().s)
    }

    /// Zoom `z` autour du point (`cx`, `cy`) de la zone du canevas.
    pub fn zoom_at(self, z: f64, cx: f64, cy: f64) {
        self.set_free(true);
        let w = untrack(|| self.world());
        let z = ((z * 100.0).round() / 100.0).clamp(0.05, 4.0);
        let k = z / w.s;
        let (x, y) = self.clamp_pan(&w, z, cx - (cx - w.x) * k, cy - (cy - w.y) * k);
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
        // Zoom choisi : canevas libre.
        self.set_free(true);
        let w = untrack(|| self.world());
        let (aw, _) = self.area.get_untracked();
        let z = ((z * 100.0).round() / 100.0).clamp(0.05, 4.0);
        let x = if w.total * z <= aw - GUTTER {
            (aw - w.total * z) / 2.0
        } else {
            GUTTER / 2.0
        };
        let y = if w.y >= 0.0 { w.y } else { w.y * z / w.s };
        let (x, y) = self.clamp_pan(&w, z, x, y);
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
        self.set_free(true);
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
            .clamp(0.05, 4.0);
        let z = (z * 100.0).round() / 100.0;
        let w = untrack(|| self.world());
        let active = untrack(|| self.active_key());
        let (ox, oy) = w
            .frames
            .iter()
            .find(|f| f.bp == active)
            .map(|f| (f.ox, f.oy))
            .unwrap_or((0.0, 0.0));
        let x = aw / 2.0 - (ox + r.x + r.width / 2.0) * z;
        let y = ah / 2.0 - (oy + r.y + r.height / 2.0) * z;
        let (x, y) = self.clamp_pan(&w, z, x, y);
        self.pan.set((x, y));
        self.zoom.set(Some(z));
    }

    /// Sélection choisie hors du canevas (calques, chemin, palette) : sélectionnée dans tous les
    /// écrans, montrée dans l'écran actif (Ordinateur par défaut) ; la vue ne se déplace que si
    /// elle n'y est pas visible.
    pub fn select_and_reveal(self, id: &str) {
        self.post(serde_json::json!({ "type": "select", "id": id, "reveal": true }));
        // Partie de l'écran actif visible dans la zone : la page y défile d'abord (la vue ne bouge
        // que si l'élément reste hors de la zone).
        let (aw, ah) = self.area.get_untracked();
        let w = untrack(|| self.world());
        let active = untrack(|| self.active_key());
        let fh = untrack(|| self.frame_height(&w));
        let band = w.frames.iter().find(|f| f.bp == active).and_then(|f| {
            let (s, m) = (w.s, GUTTER / 2.0);
            let fy = w.y + f.oy * s;
            let top = ((m - fy) / s).max(0.0);
            let bottom = ((ah - m - fy) / s).min(fh);
            let left = ((m - w.x) / s - f.ox).max(0.0);
            let right = ((aw - m - w.x) / s - f.ox).min(f.w);
            (bottom - top >= 80.0 && right - left >= 80.0).then(
                || serde_json::json!({ "top": top, "bottom": bottom, "left": left, "right": right }),
            )
        });
        self.post(serde_json::json!({ "type": "measure", "id": id, "reveal": true, "band": band }));
    }

    /// Vue déplacée (sans zoom) pour montrer un rectangle de l'écran actif, s'il sort de la zone.
    fn reveal_rect(self, r: Rect) {
        let (aw, ah) = self.area.get_untracked();
        let w = untrack(|| self.world());
        let active = untrack(|| self.active_key());
        let (ox, oy) = w
            .frames
            .iter()
            .find(|f| f.bp == active)
            .map(|f| (f.ox, f.oy))
            .unwrap_or((0.0, 0.0));
        let s = w.s;
        let m = GUTTER / 2.0;
        let (left, top) = (w.x + (ox + r.x) * s, w.y + (oy + r.y) * s);
        let (right, bottom) = (left + r.width * s, top + r.height * s);
        let mut x = w.x;
        let mut y = w.y;
        if left < m || right > aw - m {
            x = if r.width * s > aw - 2.0 * m {
                m - (ox + r.x) * s
            } else {
                aw / 2.0 - (ox + r.x + r.width / 2.0) * s
            };
        }
        if top < m || bottom > ah - m {
            y = if r.height * s > ah - 2.0 * m {
                m - (oy + r.y) * s
            } else {
                ah / 2.0 - (oy + r.y + r.height / 2.0) * s
            };
        }
        if (x, y) == (w.x, w.y) || !untrack(|| self.navigable()) {
            return;
        }
        let (x, y) = self.clamp_pan(&w, s, x, y);
        self.pan.set((x, y));
        if self.zoom.get_untracked().is_none() {
            self.zoom.set(Some(s));
        }
    }

    /// Zoom libre : cadre recentré horizontalement (changement de point de rupture).
    pub fn recenter(self) {
        if self.zoom.get_untracked().is_some() && !self.side.get_untracked() {
            let w = untrack(|| self.world());
            let (aw, _) = self.area.get_untracked();
            self.pan
                .set((((aw - w.total * w.s) / 2.0).max(GUTTER / 2.0), w.y));
        }
    }

    /// Déplacement de la vue (pixels de l'écran) ; rien en vue verrouillée.
    pub fn pan_by(self, dx: f64, dy: f64) {
        if !untrack(|| self.navigable()) {
            return;
        }
        let w = untrack(|| self.world());
        let (x, y) = self.clamp_pan(&w, w.s, w.x + dx, w.y + dy);
        self.pan.set((x, y));
        if self.zoom.get_untracked().is_none() {
            self.zoom.set(Some(w.s));
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
    /// Texte de la page mis en forme sur place : le nœud texte remplacé par du texte et des
    /// éléments (`strong`, `em`, `u`, `a`, `span`, `br`) ; dans une autre langue, texte seul.
    pub fn text_html_command(self, slot: &revel_edit::canvas::Slot, html: &str) -> Option<Command> {
        use revel_edit::canvas::Slot;
        let tpl: web_sys::HtmlTemplateElement =
            wasm_bindgen::JsCast::dyn_into(document().create_element("template").ok()?).ok()?;
        tpl.set_inner_html(html);
        let content = tpl.content();
        if self.lang_untracked().is_some() {
            let text = content.text_content().unwrap_or_default();
            return self.text_command(slot, text);
        }
        let Slot::Text { parent, index } = slot else {
            return None;
        };
        self.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let old = match revel_edit::address::list(d, parent).ok()?.get(*index)? {
                revel_model::Node::Text { text } => text.clone(),
                _ => return None,
            };
            let typo = d.settings.typography.as_deref() == Some("fr");
            let mut taken = d.node_ids();
            fn convert(
                n: &web_sys::Node,
                old: &str,
                typo: bool,
                taken: &mut std::collections::HashSet<String>,
            ) -> Option<revel_model::Node> {
                use wasm_bindgen::JsCast;
                if n.node_type() == web_sys::Node::TEXT_NODE {
                    let t = n.text_content().unwrap_or_default();
                    let t = if typo {
                        revel_edit::canvas::untypo(&t, old)
                    } else {
                        t
                    };
                    return (!t.is_empty()).then_some(revel_model::Node::Text { text: t });
                }
                let el = n.dyn_ref::<web_sys::Element>()?;
                let tag = el.tag_name().to_ascii_lowercase();
                if !matches!(tag.as_str(), "strong" | "em" | "u" | "a" | "span" | "br") {
                    return None;
                }
                let attrs = ["href", "target", "class"]
                    .iter()
                    .filter_map(|a| {
                        el.get_attribute(a).map(|v| revel_model::Attr {
                            name: a.to_string(),
                            value: Some(v),
                            when: None,
                        })
                    })
                    .collect();
                let kids = el.child_nodes();
                let children = (0..kids.length())
                    .filter_map(|i| convert(&kids.item(i)?, old, typo, taken))
                    .collect();
                let id = revel_model::ids::fresh(taken);
                taken.insert(id.clone());
                Some(revel_model::Node::Element {
                    id: Some(id),
                    tag,
                    attrs,
                    children,
                })
            }
            let kids = content.child_nodes();
            let nodes: Vec<revel_model::Node> = (0..kids.length())
                .filter_map(|i| convert(&kids.item(i)?, &old, typo, &mut taken))
                .collect();
            let mut it = nodes.into_iter();
            let first = it.next()?;
            let mut commands = vec![Command::Replace {
                parent: parent.clone(),
                index: *index,
                node: first,
            }];
            for (i, node) in it.enumerate() {
                commands.push(Command::Insert {
                    parent: parent.clone(),
                    index: index + 1 + i,
                    node,
                });
            }
            Some(Command::Batch { commands })
        })
    }

    /// Élément de texte mis en forme modifié d'un bloc (D-72), sur la page ou dans le panneau :
    /// contenu remplacé, éléments existants repris par leur repère `data-rv`. Renvoie aussi le
    /// span ajouté par la barre de mise en forme (un seul), à sélectionner. `None` : rien de
    /// changé.
    pub fn rich_text_command(
        self,
        slot: &revel_edit::canvas::Slot,
        id: &str,
        html: &str,
    ) -> Option<(Command, Option<String>)> {
        use revel_edit::canvas::Slot;
        use revel_edit::inline::Piece;
        use wasm_bindgen::JsCast;
        let Slot::Text { parent, .. } = slot else {
            return None;
        };
        let tpl: web_sys::HtmlTemplateElement = document()
            .create_element("template")
            .ok()?
            .dyn_into()
            .ok()?;
        tpl.set_inner_html(html);
        fn pieces(n: &web_sys::Node) -> Vec<Piece> {
            let kids = n.child_nodes();
            (0..kids.length())
                .filter_map(|i| kids.item(i))
                .filter_map(|c| {
                    if c.node_type() == web_sys::Node::TEXT_NODE {
                        return Some(Piece::Text {
                            text: c.text_content().unwrap_or_default(),
                        });
                    }
                    let el = c.dyn_ref::<web_sys::Element>()?;
                    Some(Piece::Element {
                        tag: el.tag_name().to_ascii_lowercase(),
                        id: el.get_attribute("data-rv"),
                        class: el.get_attribute("class"),
                        href: el.get_attribute("href"),
                        target: el.get_attribute("target"),
                        children: pieces(&c),
                    })
                })
                .collect()
        }
        let pieces = pieces(&tpl.content());
        let at = revel_edit::Target {
            owner: parent.owner.clone(),
            node: id.to_string(),
        };
        self.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let typo = d.settings.typography.as_deref() == Some("fr");
            let cmd = revel_edit::inline::set_inline(d, &at, &pieces, typo).ok()??;
            // Span nouveau : identifiant absent du document.
            let taken = d.node_ids();
            let mut spans = Vec::new();
            if let Command::Replace { node, .. } = &cmd {
                revel_model::ids::walk(std::slice::from_ref(node), &mut |n| {
                    if let revel_model::Node::Element {
                        id: Some(i), tag, ..
                    } = n
                        && tag == "span"
                        && !taken.contains(i)
                    {
                        spans.push(i.clone());
                    }
                });
            }
            let span = (spans.len() == 1).then(|| spans.remove(0));
            Some((cmd, span))
        })
    }

    /// Éléments de texte mis en forme modifiables d'un bloc, envoyés au canevas (aucun dans une
    /// autre langue : les textes s'y traduisent un par un).
    pub fn post_rich_hosts(self) {
        let ids = if self.lang_untracked().is_some() {
            Vec::new()
        } else {
            self.doc.with_untracked(|d| {
                d.as_ref()
                    .map(revel_edit::inline::rich_hosts)
                    .unwrap_or_default()
            })
        };
        self.post(serde_json::json!({ "type": "rich", "ids": ids }));
    }

    pub fn text_command(self, slot: &revel_edit::canvas::Slot, text: String) -> Option<Command> {
        use revel_edit::canvas::Slot;
        use revel_edit::locale::TextPlace;
        // Autre langue : texte écrit et propriété de composant traduits (les valeurs liées
        // s'écrivent dans les fichiers de la langue).
        if let Some(locale) = self.lang_untracked() {
            let place = match slot {
                Slot::Text { parent, index } => Some(TextPlace::Text {
                    parent: parent.clone(),
                    index: *index,
                }),
                Slot::Expr {
                    binding:
                        Some(revel_edit::canvas::Binding {
                            source: revel_edit::canvas::Source::Prop { owner, node, name },
                            path,
                        }),
                    transformed: false,
                    html: false,
                    ..
                } if path.is_empty() => Some(TextPlace::Arg {
                    at: revel_edit::Target {
                        owner: owner.clone(),
                        node: node.clone(),
                    },
                    name: name.clone(),
                }),
                _ => None,
            };
            if let Some(place) = place {
                return Some(Command::Translate {
                    locale,
                    place,
                    text,
                });
            }
        }
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
            // Texte d'une propriété de composant : dans le composant ouvert, sa valeur par défaut ;
            // sinon la valeur de cette utilisation.
            Slot::Expr {
                binding:
                    Some(revel_edit::canvas::Binding {
                        source: revel_edit::canvas::Source::Prop { owner, node, name },
                        path,
                    }),
                transformed: false,
                html: false,
                ..
            } if path.is_empty() => {
                let at = revel_edit::Target {
                    owner: owner.clone(),
                    node: node.clone(),
                };
                let component = self.doc.with_untracked(|d| {
                    match revel_edit::address::node(d.as_ref()?, &at).ok()? {
                        revel_model::Node::Use { component, .. } => Some(component.clone()),
                        _ => None,
                    }
                })?;
                if self.isolated.get_untracked().as_deref() == Some(component.as_str()) {
                    let mut props = self.doc.with_untracked(|d| {
                        d.as_ref()?.component(&component).map(|c| c.props.clone())
                    })?;
                    let p = props.iter_mut().find(|p| &p.name == name)?;
                    p.default = Some(serde_json::Value::String(text));
                    Some(Command::SetComponentProps {
                        id: component,
                        props,
                    })
                } else {
                    Some(Command::SetUseArg {
                        at,
                        name: name.clone(),
                        value: Some(revel_edit::canvas::quote(&text)),
                    })
                }
            }
            Slot::Expr {
                binding: Some(binding),
                transformed: false,
                ..
            } => Some(Command::SetBound {
                binding: binding.clone(),
                value: text,
            }),
            _ => None,
        }
    }

    /// Traduction, dans la langue `code`, d'un texte de la langue principale (`None` : pas
    /// encore traduit).
    pub fn translation(self, code: &str, source: &str) -> Option<String> {
        let key = revel_edit::locale::normalize(source);
        self.doc
            .with_untracked(|d| d.as_ref()?.locale(code)?.texts.get(&key).cloned())
    }

    /// Autre langue affichée, hors suivi réactif.
    pub fn lang_untracked(self) -> Option<String> {
        let code = self.locale.get_untracked()?;
        self.doc
            .with_untracked(|d| d.as_ref()?.locale(&code).map(|l| l.code.clone()))
    }

    /// Composant ouvert et noms des composants, transmis au canevas.
    pub fn post_isolation(self) {
        let names: serde_json::Map<String, serde_json::Value> = self.doc.with_untracked(|d| {
            d.as_ref()
                .map(|d| {
                    d.components
                        .iter()
                        .map(|c| {
                            (
                                c.id.clone(),
                                serde_json::Value::String(crate::names::component_name(c)),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default()
        });
        self.post(serde_json::json!({
            "type": "isolate",
            "component": self.isolated.get_untracked(),
            "names": names,
        }));
    }

    /// Ferme le composant ouvert (plus de sélection).
    pub fn close_component(self) {
        self.selected.set(None);
        self.post(serde_json::json!({ "type": "select", "id": null }));
    }

    /// Adresse du nœud sélectionné (son propriétaire est retrouvé dans le document).
    pub fn selected_target(self) -> Option<revel_edit::Target> {
        let sel = self.selected.get()?;
        self.doc.with(|d| target_of(d.as_ref()?, &sel.id))
    }

    /// Écran dont la plage contient la largeur `w` (comme Webflow : glisser le bord du cadre
    /// passe d'un écran à l'autre) : plus petit « jusqu'à » qui la contient, sinon plus grand « à
    /// partir de », sinon l'écran de base.
    pub fn bp_for_width(self, w: f64) -> Option<String> {
        self.doc.with_untracked(|d| {
            let bps = &d.as_ref()?.breakpoints;
            bps.iter()
                .filter(|b| b.max_width.is_some_and(|m| f64::from(m) >= w))
                .min_by_key(|b| b.max_width)
                .or_else(|| {
                    bps.iter()
                        .filter(|b| b.min_width.is_some_and(|m| f64::from(m) <= w))
                        .max_by_key(|b| b.min_width)
                })
                .or_else(|| {
                    bps.iter()
                        .find(|b| b.max_width.is_none() && b.min_width.is_none())
                })
                .map(|b| b.id.clone())
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

/// Commande (ou suite) après laquelle document et liste des pages sont relus : contenus de
/// collection, collections, fichiers de données, pages (adresse, brouillon, nom, référencement).
fn writes_entries(c: &Command) -> bool {
    match c {
        Command::Batch { commands } => commands.iter().any(writes_entries),
        Command::SetEntryField { .. }
        | Command::SetEntryBody { .. }
        | Command::CreateEntry { .. }
        | Command::WriteEntry { .. }
        | Command::CreateCmsCollection { .. }
        | Command::RemoveCmsCollection { .. }
        | Command::SetDataValue { .. }
        | Command::WriteDataFile { .. }
        | Command::AddPage { .. }
        | Command::RemovePage { .. }
        | Command::SetPagePath { .. }
        | Command::SetPageDraft { .. }
        | Command::SetPageName { .. }
        | Command::SetPageProp { .. }
        | Command::SetCollectionPages { .. } => true,
        _ => false,
    }
}

/// Balise d'un nœud pour la sélection : celle de l'élément, `img` pour une image du site
/// (réglages propres aux médias : ajustement, proportions).
pub(crate) fn node_tag(n: &revel_model::Node) -> String {
    match n {
        revel_model::Node::Element { tag, .. } => tag.clone(),
        revel_model::Node::Image(_) => "img".into(),
        _ => String::new(),
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

/// Libellé d'un nœud : nom simple (classe lisible ou nature de l'élément).
pub fn node_label(ui: Ui, n: &revel_model::Node) -> String {
    let (tag, class) = node_tag_class(n);
    if tag.is_empty() {
        return String::new();
    }
    crate::names::element_name(ui, &tag, &class)
}

/// Balise et classes (hors expressions) d'un nœud.
fn node_tag_class(n: &revel_model::Node) -> (String, String) {
    match n {
        revel_model::Node::Element { tag, attrs, .. } => {
            let class = attrs
                .iter()
                .find(|a| a.name == "class")
                .and_then(|a| a.value.as_deref())
                .filter(|c| !c.contains("{{") && !c.contains("{%"))
                .unwrap_or_default();
            (tag.clone(), class.to_string())
        }
        revel_model::Node::Image(_) => ("img".into(), String::new()),
        _ => (String::new(), String::new()),
    }
}

/// Élément du fil d'Ariane : identifiant, balise, classes.
type Crumb = (String, String, String);

/// Chemin jusqu'au nœud `id` : éléments de la racine au nœud (sans `html`, absent des calques).
fn path_to(nodes: &[revel_model::Node], id: &str, stack: &mut Vec<Crumb>) -> bool {
    for n in nodes {
        let nid = revel_model::ids::node_id(n);
        let (tag, class) = node_tag_class(n);
        let pushed = match nid {
            Some(i) if !tag.is_empty() && !tag.eq_ignore_ascii_case("html") => {
                stack.push((i.to_string(), tag, class));
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

/// Message à un cadre du canevas.
fn post_to(frame: &web_sys::HtmlIFrameElement, msg: serde_json::Value) {
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

/// Adresse d'un nœud (son propriétaire est retrouvé dans le document).
pub fn target_of(doc: &Site, id: &str) -> Option<revel_edit::Target> {
    let (kind, owner) = owner_of(doc, id)?;
    let owner = match kind {
        revel_model::ids::OwnerKind::Page => revel_edit::Owner::Page(owner),
        revel_model::ids::OwnerKind::Layout => revel_edit::Owner::Layout(owner),
        revel_model::ids::OwnerKind::Component => revel_edit::Owner::Component(owner),
    };
    Some(revel_edit::Target {
        owner,
        node: id.to_string(),
    })
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
        fit: RwSignal::new(1.0),
        selected: RwSignal::new(None),
        multi: RwSignal::new(Vec::new()),
        style_sel: RwSignal::new(String::new()),
        style_state: RwSignal::new(String::new()),
        hovered: RwSignal::new(None),
        texts: RwSignal::new((None, Vec::new())),
        previews: RwSignal::new(std::collections::HashMap::new()),
        col_labels: RwSignal::new(std::collections::HashMap::new()),
        load: RwSignal::new(0),
        scroll: RwSignal::new((0.0, 0.0)),
        live_scroll: RwSignal::new((0.0, 0.0)),
        vars: RwSignal::new(Vec::new()),
        rulers: RwSignal::new(crate::pref_get("rulers").as_deref() == Some("1")),
        guides: RwSignal::new(
            crate::pref_get("guides")
                .and_then(|g| serde_json::from_str(&g).ok())
                .unwrap_or_default(),
        ),
        computed: RwSignal::new(Default::default()),
        count: RwSignal::new(None),
        pending: RwSignal::new(0),
        saved_at: RwSignal::new(None),
        assistant: RwSignal::new(None),
        isolated: RwSignal::new(None),
        comp: RwSignal::new(None),
        variant: RwSignal::new(String::new()),
        comp_hs: RwSignal::new(std::collections::HashMap::new()),
        locale: ui.locale,
        side: RwSignal::new(crate::pref_get("side").as_deref() == Some("1")),
        free: RwSignal::new(crate::pref_get("free").as_deref() != Some("0")),
        frame_h: RwSignal::new(None),
        frame_w: RwSignal::new(None),
        hidden_bps: RwSignal::new(
            crate::pref_get("side_hidden")
                .and_then(|v| serde_json::from_str(&v).ok())
                .unwrap_or_default(),
        ),
        palette: RwSignal::new(false),
    };
    provide_context(ed);
    let dock = crate::dock::Dock::provide();
    let failed = RwSignal::new(false);
    // Modification isolée d'un composant : ouverte quand la sélection est dans un composant,
    // fermée quand elle le quitte ; le canevas estompe le reste de la page.
    Effect::new(move |_| {
        let iso = ed
            .selected
            .with(|s| {
                let s = s.as_ref()?;
                ed.doc.with(|d| match owner_of(d.as_ref()?, &s.id)? {
                    (revel_model::ids::OwnerKind::Component, c) => Some(c),
                    _ => None,
                })
            })
            // Canevas des composants : toujours ouvert sur son composant.
            .or_else(|| ed.comp.get());
        if ed.isolated.get_untracked() != iso {
            ed.isolated.set(iso);
        }
    });
    Effect::new(move |_| {
        ed.isolated.track();
        ed.post_isolation();
    });
    // Sélection multiple : abandonnée quand l'élément principal n'en fait plus partie (clic
    // simple, calques, fil d'Ariane, suppression).
    Effect::new(move |_| {
        let sel = ed.selected.get();
        let keep = ed.multi.with_untracked(|m| {
            m.is_empty() || sel.as_ref().is_some_and(|s| m.iter().any(|x| x.id == s.id))
        });
        if !keep {
            ed.multi.set(Vec::new());
            ed.post_multi();
        }
    });
    // Vue côte à côte : écran choisi signalé à chaque cadre (poignées de taille dans le seul
    // cadre actif), valeurs calculées et sélection relues dans le nouveau cadre actif.
    Effect::new(move |prev: Option<(String, bool)>| {
        let bp = ed.active_key();
        let side = ed.side.get() || ed.comp.get().is_some();
        if side && prev.is_some() {
            let bp2 = bp.clone();
            request_animation_frame(move || {
                for (b, f) in ed.frames_el() {
                    post_to(&f, serde_json::json!({ "type": "active", "on": b == bp2 }));
                }
                ed.post(serde_json::json!({ "type": "watch", "props": crate::styles::watched() }));
                if let Some(s) = ed.selected.get_untracked() {
                    ed.post(serde_json::json!({ "type": "select", "id": s.id }));
                    ed.post_multi();
                }
            });
        }
        (bp, side)
    });
    // Cadres allongés : hauteur normale transmise au canevas (unités `vh` inchangées).
    Effect::new(move |_| {
        let w = ed.world();
        let _ = (w.h, w.base_h, ed.frame_h.get());
        ed.post_viewport();
    });
    // Zoom de l'éditeur transmis au canevas (contours, poignées, étiquettes à taille constante).
    Effect::new(move |_| {
        let s = ed.geo().s;
        ed.post(serde_json::json!({ "type": "scale", "s": s }));
    });
    // Adresse du canevas partagée (vignettes des médias dans les autres vues).
    {
        let site_path = site.path.clone();
        Effect::new(move |_| {
            if let Some(b) = ed.base.get() {
                ui.canvas_base.set(Some((site_path.clone(), b)));
            }
        });
        on_cleanup(move || ui.canvas_base.set(None));
    }

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

    // Page affichée, connue de la barre supérieure (l'aperçu s'ouvre sur elle).
    Effect::new(move |_| ui.current_page.set(Some(ed.localized(&ed.page.get()))));
    on_cleanup(move || ui.current_page.set(None));

    // Page demandée par une autre vue (vue CMS) : affichée dès que la liste des pages est connue.
    Effect::new(move |_| {
        let Some(mut url) = ui.goto_page.get() else {
            return;
        };
        // Page d'une autre langue (`/en/projets/wattr/`) : langue de travail choisie.
        let lang = ed.doc.with(|d| {
            let d = d.as_ref()?;
            d.locale_of_url(&url)
                .map(|(l, rest)| (l.code.clone(), rest.to_string()))
        });
        if let Some((code, rest)) = lang {
            if ui.locale.get_untracked().as_deref() != Some(code.as_str()) {
                ui.locale.set(Some(code));
            }
            url = rest;
        }
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
        // Cadre qui écrit : l'actif (écran choisi) ou un autre écran de la vue côte à côte.
        let src = e.source();
        let from = ed
            .frames_el()
            .into_iter()
            .find(|(_, f)| match (&src, f.content_window()) {
                (Some(s), Some(w)) => js_sys::Object::is(s, w.as_ref()),
                _ => false,
            });
        let Some((from_bp, from_frame)) = from else {
            return;
        };
        let active = from_frame.class_list().contains("is-active");
        let data = e.data();
        let tagged = js_sys::Reflect::get(&data, &JsValue::from_str("revel"))
            .ok()
            .and_then(|v| v.as_f64())
            == Some(1.0);
        if !tagged {
            return;
        }
        let Ok(msg) = serde_wasm_bindgen::from_value::<FromCanvas>(data) else {
            return;
        };
        if !active {
            match &msg {
                // Clic, Maj+clic, clic droit ou texte modifié dans un autre écran : il devient
                // l'écran choisi.
                FromCanvas::Select { id: Some(_), .. }
                | FromCanvas::Toggle { .. }
                | FromCanvas::Context { .. }
                | FromCanvas::Text { .. }
                | FromCanvas::TextHtml { .. }
                | FromCanvas::RichText { .. } => ed.activate_key(&from_bp),
                FromCanvas::Ready { .. }
                | FromCanvas::Refreshed { .. }
                | FromCanvas::Stage { .. }
                | FromCanvas::OpenComponent { .. }
                | FromCanvas::Key { .. }
                | FromCanvas::Keyup { .. }
                | FromCanvas::Pan { .. }
                | FromCanvas::Zoom { .. }
                | FromCanvas::Moved { .. } => {}
                _ => return,
            }
        }
        match msg {
            FromCanvas::Ready { path, vars, rev } => {
                // Navigation impossible dans le canevas : la page est celle demandée.
                let _ = path;
                // Page rendue avant la dernière modification : mise à jour sur place.
                if ed.stale(rev) {
                    post_to(&from_frame, serde_json::json!({ "type": "refresh" }));
                }
                post_to(
                    &from_frame,
                    serde_json::json!({ "type": "active", "on": active }),
                );
                post_to(&from_frame, crate::names::canvas_labels(ui));
                ed.post_rich_hosts();
                if active {
                    ed.vars.set(vars);
                    ed.live_scroll.set((0.0, 0.0));
                    ed.post(
                        serde_json::json!({ "type": "watch", "props": crate::styles::watched() }),
                    );
                    let (x, y) = ed.scroll.get_untracked();
                    if x != 0.0 || y != 0.0 {
                        ed.post(serde_json::json!({ "type": "scroll", "x": x, "y": y }));
                    }
                }
                ed.post_isolation();
                ed.post_viewport();
                ed.post(serde_json::json!({ "type": "scale", "s": ed.scale() }));
                if let Some(s) = ed.selected.get_untracked() {
                    ed.post(serde_json::json!({ "type": "select", "id": s.id }));
                    ed.post_multi();
                }
            }
            FromCanvas::Refreshed { vars, rev } => {
                if ed.stale(rev) {
                    post_to(&from_frame, serde_json::json!({ "type": "refresh" }));
                }
                if active {
                    ed.vars.set(vars);
                }
                ed.post_rich_hosts();
                ed.post_isolation();
                if let Some(s) = ed.selected.get_untracked() {
                    post_to(
                        &from_frame,
                        serde_json::json!({ "type": "select", "id": s.id }),
                    );
                    ed.post_multi();
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
                let r = from_frame.get_bounding_client_rect();
                let s = ed.scale();
                crate::canvas::open_canvas_menu(ui, ed, r.left() + x * s, r.top() + y * s);
            }
            FromCanvas::Measure { rect, reveal } => {
                if let Some(r) = rect {
                    if reveal {
                        ed.reveal_rect(r);
                    } else {
                        ed.fit_rect(r);
                    }
                }
            }
            FromCanvas::Select { id, tag, label } => {
                if id.is_none() {
                    ed.computed.set(Default::default());
                }
                ed.multi.set(Vec::new());
                ed.selected.set(id.map(|id| Selection { id, tag, label }));
            }
            FromCanvas::Toggle { id, tag, label } => ed.toggle_select(Selection { id, tag, label }),
            FromCanvas::Resized {
                id,
                width,
                height,
                reset,
            } => ed.resize(ui, &id, width, height, reset),
            FromCanvas::Spacing { id, values } => {
                if let Some(at) = ed.selected_target().filter(|t| t.node == id)
                    && let Some(node) = ed.doc.with_untracked(|d| {
                        revel_edit::address::node(d.as_ref()?, &at).ok().cloned()
                    })
                {
                    ed.set_props(ui, at, node, values.into_iter().collect());
                }
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
            FromCanvas::Scrolled { x, y } => ed.live_scroll.set((x, y)),
            FromCanvas::Computed { values } => ed.computed.set(values),
            FromCanvas::Count { selector, n } => ed.count.set(Some((selector, n))),
            // Vue verrouillée : pas de zoom au pincement ni à la molette (bouton « Canevas libre »).
            FromCanvas::Zoom { .. } if !untrack(|| ed.navigable()) => {}
            FromCanvas::Zoom { dy, x, y } => {
                // Position du pointeur dans le cadre -> dans la zone du canevas.
                let at = x.zip(y).map(|(x, y)| {
                    let (fx, fy, s) = ed.frame_origin(&from_bp);
                    (fx + x * s, fy + y * s)
                });
                ed.zoom_by((-dy * 0.002).exp(), at);
            }
            FromCanvas::Text { slot, text } => {
                if let Some(cmd) = ed.text_command(&slot, text) {
                    ed.apply(ui, cmd, false);
                }
            }
            FromCanvas::TextHtml { slot, html } => {
                if let Some(cmd) = ed.text_html_command(&slot, &html) {
                    ed.apply(ui, cmd, false);
                }
            }
            FromCanvas::RichText { slot, id, html } => {
                match ed.rich_text_command(&slot, &id, &html) {
                    Some((cmd, span)) => {
                        // Span créé par la barre de mise en forme : sélectionné, prêt à styler.
                        if let Some(span) = span {
                            ed.selected.set(Some(Selection {
                                id: span,
                                tag: "span".into(),
                                label: crate::names::type_name(ui, "span"),
                            }));
                        }
                        ed.apply(ui, cmd, false)
                    }
                    // Rien de changé (ou contenu refusé) : la page reprend son texte.
                    None => ed.refresh(),
                }
            }
            FromCanvas::Texts { id, items } => ed.texts.set((id, items)),
            FromCanvas::SpacingFocus { prop } => {
                dock.layout.update(|l| l.open(crate::dock::PanelId::Styles));
                focus_style_field(prop);
            }
            FromCanvas::Stage { h } => {
                let key = from_bp.clone();
                if ed
                    .comp_hs
                    .with_untracked(|m| m.get(&key).is_none_or(|v| (v - h).abs() > 1.0))
                {
                    ed.comp_hs.update(|m| {
                        m.insert(key, h);
                    });
                }
            }
            FromCanvas::OpenComponent {
                component,
                node,
                tag,
            } => ed.open_comp(&component, node.map(|n| (n, tag))),
            FromCanvas::Previews { map } => {
                if ed.previews.with_untracked(|m| *m != map) {
                    ed.previews.set(map);
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
        let is_field = |el: &web_sys::Element| {
            matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                || el.has_attribute("contenteditable")
        };
        // Touche partie d'un champ, même s'il vient de disparaître (champ d'espacement validé
        // par Entrée) : pas un raccourci de l'éditeur (Entrée sélectionnerait l'enfant).
        let in_field = document().active_element().is_some_and(|el| is_field(&el))
            || e.target()
                .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok())
                .is_some_and(|el| is_field(&el));
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

    // Document modifié ailleurs : page et document relus. Par une autre vue ou un assistant :
    // révision de la session différente de celle de l'éditeur (aucune modification de l'éditeur
    // en cours d'envoi ; stable sur deux relevés, ou dès le retour dans l'éditeur). Hors de Revel
    // (Pages CMS, éditeur de texte) : même révision, fichiers différents.
    let stamp = StoredValue::new(None::<revel_edit::EditStamp>);
    // Données ou contenus changés depuis la dernière relecture (la page affichée en dépend).
    let content_seen = StoredValue::new(false);
    let act_seen = StoredValue::new(js_sys::Date::now());
    let path_poll = StoredValue::new(site.path.clone());
    let check = move |shown: bool| {
        let path = path_poll.get_value();
        spawn_local(async move {
            let Ok(now) = api::edit_stamp(&path).await else {
                return;
            };
            let before = stamp.get_value();
            stamp.set_value(Some(now.clone()));
            let Some(before) = before else {
                return;
            };
            let same = before.revision == now.revision;
            let mine = ed.state.with_untracked(|s| s.as_ref().map(|s| s.revision));
            let elsewhere = (shown || same)
                && mine.is_some_and(|r| r != now.revision)
                && ed.pending.get_untracked() == 0;
            let outside = same && before.files != now.files;
            if before.content != now.content {
                content_seen.set_value(true);
            }
            if elsewhere || outside {
                // Données ou contenus changés : la page change ; document seul : rechargée
                // seulement si elle peut s'afficher autrement.
                let content = content_seen.get_value();
                content_seen.set_value(false);
                ed.reopen_with(ui, Some(outside || content));
            } else if mine == Some(now.revision) {
                // Modifications de l'éditeur lui-même : déjà affichées.
                content_seen.set_value(false);
            }
            // Modification d'un assistant : signalée dans la barre d'état.
            if elsewhere && let Ok(list) = api::mcp_activity().await {
                let since = act_seen.get_value();
                if let Some(a) = list.first().filter(|a| a.at as f64 > since) {
                    act_seen.set_value(a.at as f64);
                    ed.assistant
                        .set(Some(crate::assistants::activity_label(ui, a)));
                    let at = a.at;
                    set_timeout(
                        move || {
                            if !ed.assistant.is_disposed() && act_seen.get_value() == at as f64 {
                                ed.assistant.set(None);
                            }
                        },
                        std::time::Duration::from_millis(6000),
                    );
                }
            }
        });
    };
    if let Ok(h) =
        set_interval_with_handle(move || check(false), std::time::Duration::from_millis(1000))
    {
        on_cleanup(move || h.clear());
    }
    let shown = Memo::new(move |_| {
        matches!(
            ui.route.get(),
            crate::Route::Site {
                tab: crate::Tab::Editor,
                ..
            }
        )
    });
    Effect::new(move |prev: Option<bool>| {
        let now = shown.get();
        if prev == Some(false) && now {
            check(true);
        }
        now
    });

    // Échelle du zoom ajusté (affichée dans la liste des zooms).
    Effect::new(move |_| ed.fit.set(ed.geo().fit));

    view! {
        <section class="editor">
            <crate::palette::Palette/>
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
        PanelId::Pages => view! { <crate::pagetree::Pages/> }.into_any(),
        PanelId::Layers => view! { <crate::layers::Layers/> }.into_any(),
        PanelId::Variables => view! { <crate::variables::Variables/> }.into_any(),
        PanelId::Cms => view! { <crate::panels::Cms/> }.into_any(),
        PanelId::Insert => view! { <crate::insert::Insert/> }.into_any(),
        PanelId::Components => view! { <crate::library::Components/> }.into_any(),
        PanelId::Assets => view! { <crate::library::Media/> }.into_any(),
        PanelId::Styles => {
            view! { <div class="panel-pad"><crate::styles::Styles/></div> }.into_any()
        }
        PanelId::Settings => ().into_any(),
        PanelId::Interactions => view! { <crate::interactions::Interactions/> }.into_any(),
    }
}

/// Langue affichée dans le canevas (sites à plusieurs langues) : les textes modifiés sont écrits
/// dans cette langue ; la structure et les styles restent communs à toutes les langues.
#[component]
fn LocaleSwitch() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let langs = Memo::new(move |_| {
        ed.doc.with(|d| {
            let d = d.as_ref()?;
            (!d.locales.is_empty()).then(|| {
                let mut v = vec![(String::new(), revel_model::language_name(&d.default_locale))];
                v.extend(d.locales.iter().map(|l| (l.code.clone(), l.label())));
                v
            })
        })
    });
    move || {
        langs.get().map(|list| {
            view! {
                <div class="select editor-locale" class:is-other=move || ed.lang().is_some()
                    title=move || ui.tx(if ed.lang().is_some() { "editor-locale-other-help" } else { "editor-locale-help" }, &[])>
                    {icons::globe()}
                    <select class="input" aria-label=ui.t("editor-locale")
                        on:change=move |e| {
                            let v = event_target_value(&e);
                            ui.locale.set((!v.is_empty()).then_some(v));
                        }>
                        {list.into_iter().map(|(code, label)| {
                            let c2 = code.clone();
                            view! {
                                <option value=code selected=move || ed.lang().unwrap_or_default() == c2>{label}</option>
                            }
                        }).collect_view()}
                    </select>
                </div>
            }
        })
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
    let help_open = RwSignal::new(false);
    view! {
        <crate::help::Help open=help_open/>
        <div class="editor-toolbar">
            <crate::pagetree::PageContext/>
            <LocaleSwitch/>
            <div class="segmented" role="group" aria-label=ui.t("editor-breakpoints")>
                {move || breakpoints().into_iter().map(|b| {
                    let id = b.id.clone();
                    let on = b.id.clone();
                    let title = bp_label(&b);
                    let (shown, cur) = (b.id.clone(), b.id.clone());
                    let title2 = title.clone();
                    // Un écran : choisi. Côte à côte : affiché ou masqué (l'écran choisi marqué).
                    view! {
                        <button
                            class:is-on=move || if ed.side.get() { !ed.hidden_bps.with(|h| h.contains(&shown)) } else { ed.breakpoint.get() == on }
                            class:is-current=move || ed.side.get() && ed.breakpoint.get() == cur
                            title=move || if ed.side.get() { ui.tx("editor-bp-toggle", &[("name", title2.clone())]) } else { title2.clone() }
                            aria-label=title.clone()
                            on:click=move |_| if ed.side.get_untracked() { ed.toggle_bp(&id) } else { ed.frame_w.set(None); ed.breakpoint.set(id.clone()); ed.recenter(); }>
                            {bp_icon(&b)}
                            <span class="bp-width">{b.max_width.or(b.min_width).map(|w| w.to_string()).unwrap_or_default()}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
            <button class="tool-toggle free-toggle" class:is-on=move || ed.free.get() aria-pressed=move || ed.free.get().to_string()
                title=ui.t("editor-free-help") aria-label=ui.t("editor-free")
                on:click=move |_| ed.set_free(!ed.free.get_untracked())>{icons::expand()}</button>
            <button class="tool-toggle side-toggle" class:is-on=move || ed.side.get() aria-pressed=move || ed.side.get().to_string()
                title=ui.t("editor-side-help") aria-label=ui.t("editor-side")
                on:click=move |_| ed.set_side(!ed.side.get_untracked())>{icons::screens()}</button>
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
            <ViewMenu/>
            <div class="toolbar-spacer"></div>
            <crate::dock::PanelsMenu/>
            <details class="bp-menu shortcuts">
                <summary class="icon-btn" title=ui.t("help-menu") aria-label=ui.t("help-menu")>"?"</summary>
                <div class="bp-pop bp-pop--right">
                    // Aide intégrée (6.8) : guide « Premiers pas ».
                    <button class="btn btn--small help-open" on:click=move |e: leptos::ev::MouseEvent| {
                        help_open.set(true);
                        if let Some(d) = e.target()
                            .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok())
                            .and_then(|t| t.closest("details").ok().flatten())
                        {
                            let _ = d.remove_attribute("open");
                        }
                    }>{icons::info()}{ui.t("help-title")}</button>
                    <p class="panel-title">{ui.t("editor-shortcuts")}</p>
                    {[
                        ("key-palette", "palette"),
                        ("key-undo", "editor-undo-short"),
                        ("key-redo", "editor-redo-short"),
                        ("key-duplicate", "layers-duplicate-short"),
                        ("key-delete", "layers-delete-short"),
                        ("key-escape", "editor-deselect"),
                        ("key-multi", "editor-multi-add"),
                        ("key-resize", "editor-resize"),
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
                        ("key-rulers", "editor-rulers"),
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

/// Affichage du canevas : règles en pixels, guides de colonnes (nombre, gouttière, marges,
/// largeur maximale du contenu). Préférences de l'utilisateur, gardées d'une session à l'autre.
#[component]
fn ViewMenu() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let set_guides = move |f: &dyn Fn(&mut Guides)| {
        ed.guides.update(|g| f(g));
        if let Ok(j) = serde_json::to_string(&ed.guides.get_untracked()) {
            crate::pref_set("guides", &j);
        }
    };
    let number = move |label: &'static str,
                       help: Option<&'static str>,
                       get: fn(&Guides) -> u32,
                       put: fn(&mut Guides, u32),
                       min: u32,
                       max: u32| {
        view! {
            <label class="view-line" title=move || help.map(|h| ui.tx(h, &[])).unwrap_or_default()>
                <span>{ui.t(label)}</span>
                <input class="input input--mono view-num" type="number" min=min max=max
                    prop:value=move || { let v = get(&ed.guides.get()); if v == 0 && min == 0 { String::new() } else { v.to_string() } }
                    placeholder=move || if min == 0 { "—".to_string() } else { String::new() }
                    on:change=move |e| {
                        let v = event_target_value(&e).trim().parse::<u32>().unwrap_or(0).clamp(min, max);
                        set_guides(&|g| put(g, v));
                    }/>
            </label>
        }
    };
    view! {
        <details class="bp-menu view-menu pop-menu">
            <summary class="tool-toggle" class:is-on=move || ed.rulers.get() || ed.guides.get().on
                title=ui.t("editor-view") aria-label=ui.t("editor-view")>{icons::ruler()}</summary>
            <div class="bp-pop">
                <label class="view-line" title=move || ui.tx("key-rulers", &[])>
                    <span>{ui.t("editor-rulers")}</span>
                    <span class="switch">
                        <input type="checkbox" prop:checked=move || ed.rulers.get()
                            on:change=move |e| {
                                let on = event_target_checked(&e);
                                ed.rulers.set(on);
                                crate::pref_set("rulers", if on { "1" } else { "0" });
                            }/>
                        <span class="switch-track"></span>
                    </span>
                </label>
                <label class="view-line">
                    <span>{ui.t("editor-guides")}</span>
                    <span class="switch">
                        <input type="checkbox" prop:checked=move || ed.guides.get().on
                            on:change=move |e| { let on = event_target_checked(&e); set_guides(&|g| g.on = on); }/>
                        <span class="switch-track"></span>
                    </span>
                </label>
                <div class="view-guides" class:is-off=move || !ed.guides.get().on>
                    {number("editor-guides-count", None, |g| g.count, |g, v| g.count = v, 1, 24)}
                    {number("editor-guides-gutter", None, |g| g.gutter, |g, v| g.gutter = v, 0, 200)}
                    {number("editor-guides-margin", None, |g| g.margin, |g, v| g.margin = v, 0, 400)}
                    {number("editor-guides-max", Some("editor-guides-max-help"), |g| g.max, |g, v| g.max = v, 0, 4000)}
                </div>
            </div>
        </details>
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
        <details class="bp-menu bp-manage">
            <summary class="icon-btn" title=ui.t("editor-bp-manage") aria-label=ui.t("editor-bp-manage")>{icons::plus()}</summary>
            <div class="bp-pop">
                <p class="panel-title">{ui.t("editor-breakpoints")}</p>
                {move || sorted(list()).into_iter().filter(|b| b.id != "desktop").map(|b| {
                    let (id, id2) = (b.id.clone(), b.id.clone());
                    let (kind, help, w) = match (b.max_width, b.min_width) {
                        (Some(w), _) => ("editor-bp-kind-max", "editor-bp-kind-max-help", w),
                        (_, Some(w)) => ("editor-bp-kind-min", "editor-bp-kind-min-help", w),
                        _ => ("", "", 0),
                    };
                    view! {
                        <div class="bp-line bp-edit" data-bp=b.id.clone()>
                            <span class="bp-kind" title=ui.t(help)>{ui.t(kind)}</span>
                            <label class="bp-num" title=ui.t("editor-bp-width-help")>
                                <input class="input input--mono" type="number" min="100" step="1" prop:value=w.to_string()
                                    on:keydown=move |e: leptos::ev::KeyboardEvent| {
                                        if e.key() == "Enter" && let Some(t) = e.target().and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlInputElement>(t).ok()) { let _ = t.blur(); }
                                    }
                                    on:change=move |e| {
                                        let input = event_target::<web_sys::HtmlInputElement>(&e);
                                        match input.value().trim().parse::<u32>() {
                                            Ok(v) if v != w => ed.apply(ui, Command::ResizeBreakpoint { id: id2.clone(), width: v }, false),
                                            _ => input.set_value(&w.to_string()),
                                        }
                                    }/>
                                <span class="muted">"px"</span>
                            </label>
                            <button class="icon-btn icon-btn--small" title=ui.t("editor-bp-remove") aria-label=ui.t("editor-bp-remove")
                                on:click=move |_| remove(id.clone())>{icons::trash()}</button>
                        </div>
                    }
                }).collect_view()}
                <div class="bp-add">
                    <div class="select" title=move || ui.tx(if kind_max.get() { "editor-bp-kind-max-help" } else { "editor-bp-kind-min-help" }, &[])>
                        <select class="input" on:change=move |e| kind_max.set(event_target_value(&e) == "max")>
                            <option value="max" title=ui.t("editor-bp-kind-max-help") selected=move || kind_max.get()>{ui.t("editor-bp-kind-max")}</option>
                            <option value="min" title=ui.t("editor-bp-kind-min-help") selected=move || !kind_max.get()>{ui.t("editor-bp-kind-min")}</option>
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
            let name = match kind {
                revel_model::ids::OwnerKind::Component => d
                    .as_ref()?
                    .component(&id)
                    .map(crate::names::component_name)
                    .unwrap_or(id),
                _ => id,
            };
            Some(ui.tx(key, &[("name", name)]))
        })
    };
    let width = move || {
        if ed.side.get() {
            ed.frame_width()
                .map(|w| format!("{w} px"))
                .unwrap_or_default()
        } else {
            format!("{} px", ed.geo().w.round())
        }
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
    let pick = move |id: String, tag: String, label: String| {
        ed.selected.set(Some(Selection {
            id: id.clone(),
            tag,
            label,
        }));
        ed.select_and_reveal(&id);
    };
    view! {
        <footer class="editor-status">
            <nav class="node-path" aria-label=ui.t("editor-path")>
                {move || (ed.multi.with(Vec::len) > 1).then(|| view! {
                    <span class="chip chip--multi">{ui.tx("editor-multi", &[("count", ed.multi.with(Vec::len).to_string())])}</span>
                })}
                {move || match crumbs() {
                    None => view! { <span class="muted">{ui.t("editor-no-selection")}</span> }.into_any(),
                    Some(list) => {
                        let n = list.len();
                        let skip = n.saturating_sub(6);
                        view! {
                            {(skip > 0).then(|| view! { <span class="node-sep">"…"</span> })}
                            {list.into_iter().enumerate().skip(skip).map(|(i, (id, tag, class))| {
                                let last = i + 1 == n;
                                let label = crate::names::element_name(ui, &tag, &class);
                                let title = crate::names::tooltip(ui, &tag, &class);
                                let tech = crate::names::technical(&tag, &class);
                                let l2 = label.clone();
                                view! {
                                    {(i > skip || skip > 0).then(|| view! { <span class="node-sep">"›"</span> })}
                                    <button class="node-crumb" class:is-on=last title=title data-tech=tech
                                        on:click=move |_| pick(id.clone(), tag.clone(), l2.clone())>{label}</button>
                                }
                            }).collect_view()}
                        }.into_any()
                    }
                }}
            </nav>
            <span class="muted">{owner}</span>
            <span class="toolbar-spacer"></span>
            {move || ed.assistant.get().map(|t| view! {
                <span class="assistant-note" aria-live="polite">{icons::sparkle()}{t}</span>
            })}
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
