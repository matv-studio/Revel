//! Contexte du panneau de styles : cible (classe, état, point de rupture), origine des valeurs,
//! écriture des déclarations et aperçu pendant un geste.

use crate::Ui;
use crate::editor::Ed;
use leptos::prelude::*;
use revel_model::{Breakpoint, Decl, StyleItem, StyleRule};
use revel_style::values;

/// Propriétés dont le canevas renvoie la valeur effective pour l'élément sélectionné.
const WATCHED: &[&str] = &[
    "display",
    "flex-direction",
    "flex-wrap",
    "justify-content",
    "align-items",
    "justify-items",
    "gap",
    "row-gap",
    "column-gap",
    "grid-template-columns",
    "grid-template-rows",
    "grid-auto-flow",
    "align-content",
    "grid-column",
    "grid-row",
    "justify-self",
    "align-self",
    "order",
    "flex",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "width",
    "height",
    "min-width",
    "max-width",
    "min-height",
    "max-height",
    "overflow",
    "object-fit",
    "aspect-ratio",
    "position",
    "top",
    "right",
    "bottom",
    "left",
    "z-index",
    "font-family",
    "font-size",
    "font-weight",
    "line-height",
    "letter-spacing",
    "color",
    "text-align",
    "text-transform",
    "font-style",
    "text-decoration-line",
    "background-color",
    "background-image",
    "background-size",
    "background-position",
    "background-repeat",
    "border-width",
    "border-style",
    "border-color",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
    "border-top-style",
    "border-right-style",
    "border-bottom-style",
    "border-left-style",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    "border-radius",
    "border-top-left-radius",
    "border-top-right-radius",
    "border-bottom-right-radius",
    "border-bottom-left-radius",
    "opacity",
    "box-shadow",
    "cursor",
    "mix-blend-mode",
    "filter",
    "backdrop-filter",
    "transform",
    "transition",
    "white-space",
    "text-wrap",
    "text-overflow",
];

/// Propriétés suivies par le canevas.
pub fn watched() -> Vec<String> {
    WATCHED.iter().map(|p| p.to_string()).collect()
}

/// Origine d'une valeur.
#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    /// Réglée sur la classe, l'état et le point de rupture affichés.
    Here,
    /// Héritée d'un écran plus large ou de l'état de base (libellé de la source).
    Inherited(String),
    /// Autre source (autre classe, parent, navigateur) : valeur effective seulement.
    None,
}

/// Valeur d'une propriété pour le panneau.
#[derive(Debug, Clone, PartialEq)]
pub struct PropValue {
    /// Valeur réglée ici (`None` si absente ou portée par un raccourci illisible).
    pub here: Option<String>,
    /// Valeur héritée (écran plus large, état de base).
    pub inherited: Option<String>,
    pub origin: Origin,
    /// Valeur effective dans le canevas.
    pub computed: String,
    /// La valeur réglée ici peut être retirée.
    pub resettable: bool,
}

impl PropValue {
    /// Indication affichée quand rien n'est réglé ici : valeur héritée, sinon effective.
    pub fn hint(&self) -> String {
        self.inherited
            .clone()
            .unwrap_or_else(|| self.computed.clone())
    }
    /// Valeur en vigueur.
    pub fn current(&self) -> String {
        self.here.clone().unwrap_or_else(|| self.hint())
    }
    pub fn is_here(&self) -> bool {
        self.origin == Origin::Here
    }
}

/// Nom d'un point de rupture (« Ordinateur », « Jusqu'à 900 px »).
pub fn bp_name(ui: Ui, b: &Breakpoint) -> String {
    match (b.max_width, b.min_width) {
        (Some(w), _) => ui.tx("editor-bp-max", &[("width", w.to_string())]),
        (_, Some(w)) => ui.tx("editor-bp-min", &[("width", w.to_string())]),
        _ => ui.tx("editor-bp-desktop", &[]),
    }
}

/// Points de rupture dont les styles s'appliquent aussi à `cur`, du plus proche au plus lointain
/// (base en dernier) : écrans plus larges pour un « jusqu'à », plus étroits pour un « à partir de ».
fn wider<'a>(bps: &'a [Breakpoint], cur: &Breakpoint) -> Vec<&'a Breakpoint> {
    let base = bps
        .iter()
        .find(|b| b.max_width.is_none() && b.min_width.is_none());
    let mut v: Vec<&Breakpoint> = match (cur.max_width, cur.min_width) {
        (Some(w), _) => {
            let mut v: Vec<_> = bps
                .iter()
                .filter(|b| b.max_width.is_some_and(|x| x > w))
                .collect();
            v.sort_by_key(|b| b.max_width);
            v
        }
        (None, Some(w)) => {
            let mut v: Vec<_> = bps
                .iter()
                .filter(|b| b.min_width.is_some_and(|x| x < w))
                .collect();
            v.sort_by_key(|b| std::cmp::Reverse(b.min_width));
            v
        }
        _ => return Vec::new(),
    };
    v.extend(base);
    v
}

pub(crate) use revel_edit::style::{find_rule, style_command};

/// Contexte partagé par les contrôles du panneau.
#[derive(Clone, Copy)]
pub struct Sx {
    pub ui: Ui,
    pub ed: Ed,
    /// Classe (ou combinaison de classes) réglée.
    pub selector: RwSignal<String>,
    /// État (`:hover`…), vide pour l'état de base.
    pub state: RwSignal<String>,
    pub media: Memo<Option<String>>,
    /// Règle modifiée (dernière règle du sélecteur et du point de rupture).
    pub rule: Memo<Option<StyleRule>>,
    /// Sources héritées, de la plus proche à la plus lointaine.
    pub sources: Memo<Vec<(Vec<Decl>, String)>>,
    /// Menu ou panneau flottant ouvert (identifiant).
    pub menu: RwSignal<Option<u32>>,
    /// Balise de l'élément sélectionné.
    pub tag: Memo<String>,
    /// Variante du composant ouvert dans le canevas des composants : (classe de l'élément
    /// racine, suffixe de la variante, identifiant de l'élément racine, nom de la variante).
    pub scope: Memo<Option<(String, String, String, String)>>,
}

/// Sélecteur complet : classe et état ; dans une variante, préfixé par la racine du composant
/// dans cette variante (`.carte:hover .titre`, `.carte.is-sombre`).
fn scoped(
    scope: &Option<(String, String, String, String)>,
    root_selected: bool,
    sel: &str,
    state: &str,
) -> String {
    match scope {
        None => format!("{sel}{state}"),
        Some((root, suffix, _, _)) if root_selected => {
            // Racine : la variante s'ajoute à sa classe (`.carte.is-sombre`, `.carte:hover`).
            let rest = sel.strip_prefix(root.as_str()).unwrap_or(sel);
            format!("{root}{suffix}{rest}{state}")
        }
        Some((root, suffix, _, _)) => format!("{root}{suffix} {sel}{state}"),
    }
}

impl Sx {
    pub fn new(ui: Ui, ed: Ed) -> Self {
        // Cible partagée avec l'éditeur (poignées de taille du canevas).
        let selector = ed.style_sel;
        let state = ed.style_state;
        let scope = Memo::new(move |_| {
            let id = ed.comp.get()?;
            let v = ed.variant.get();
            if v.is_empty() {
                return None;
            }
            ed.doc.with(|d| {
                let c = d.as_ref()?.component(&id)?;
                let var = c.variant(&v)?;
                let root_id = match c.root()? {
                    revel_model::Node::Element { id: Some(r), .. } => r.clone(),
                    _ => return None,
                };
                Some((
                    format!(".{}", c.root_class()?),
                    var.suffix(),
                    root_id,
                    crate::components::variant_label(ui, Some(var)),
                ))
            })
        });
        let root_selected = move || {
            scope.with(|sc| {
                sc.as_ref().is_some_and(|(_, _, r, _)| {
                    ed.selected.with(|s| s.as_ref().is_some_and(|s| &s.id == r))
                })
            })
        };
        let media = Memo::new(move |_| {
            let id = ed.breakpoint.get();
            ed.doc.with(|d| {
                d.as_ref()?
                    .breakpoints
                    .iter()
                    .find(|b| b.id == id)
                    .and_then(|b| b.media())
            })
        });
        let rule = Memo::new(move |_| {
            let sel = scoped(&scope.get(), root_selected(), &selector.get(), &state.get());
            let m = media.get();
            ed.doc.with(|d| find_rule(d.as_ref()?, &sel, &m))
        });
        let sources = Memo::new(move |_| {
            let sel = selector.get();
            let st = state.get();
            let bp = ed.breakpoint.get();
            ed.doc
                .with(|d| {
                    let d = d.as_ref()?;
                    let items = &d.stylesheet.as_ref()?.items;
                    let cur = d.breakpoints.iter().find(|b| b.id == bp)?;
                    let wide = wider(&d.breakpoints, cur);
                    let sc = scope.get();
                    let rs = root_selected();
                    let full = scoped(&sc, rs, &sel, &st);
                    let mut chain: Vec<(String, &Breakpoint)> =
                        wide.iter().map(|b| (full.clone(), *b)).collect();
                    // Variante : valeurs de la variante principale héritées (même état).
                    if sc.is_some() {
                        let base = format!("{sel}{st}");
                        chain.push((base.clone(), cur));
                        chain.extend(wide.iter().map(|b| (base.clone(), *b)));
                    }
                    if !st.is_empty() {
                        chain.push((sel.clone(), cur));
                        chain.extend(wide.iter().map(|b| (sel.clone(), *b)));
                    }
                    Some(
                        chain
                            .into_iter()
                            .filter_map(|(s, b)| {
                                let media = b.media();
                                let decls: Vec<Decl> = items
                                    .iter()
                                    .filter_map(|i| match i {
                                        StyleItem::Rule(r)
                                            if r.selector == s && r.media == media =>
                                        {
                                            Some(r.decls.clone())
                                        }
                                        _ => None,
                                    })
                                    .flatten()
                                    .collect();
                                (!decls.is_empty())
                                    .then(|| (decls, format!("{s} · {}", bp_name(ui, b))))
                            })
                            .collect(),
                    )
                })
                .unwrap_or_default()
        });
        let tag = Memo::new(move |_| ed.selected.get().map(|s| s.tag).unwrap_or_default());
        Self {
            ui,
            ed,
            selector,
            state,
            media,
            rule,
            sources,
            menu: RwSignal::new(None),
            tag,
            scope,
        }
    }

    /// L'élément sélectionné est la racine du composant ouvert dans une variante.
    fn root_selected(self) -> bool {
        self.scope.with(|sc| {
            sc.as_ref().is_some_and(|(_, _, r, _)| {
                self.ed
                    .selected
                    .with(|s| s.as_ref().is_some_and(|s| &s.id == r))
            })
        })
    }

    /// Sélecteur complet (classe et état ; dans une variante, préfixé par la racine).
    pub fn full_selector(self) -> String {
        scoped(
            &self.scope.get(),
            self.root_selected(),
            &self.selector.get(),
            &self.state.get(),
        )
    }

    /// Sélecteur complet, lecture non suivie (gestionnaires d'événements).
    fn full_selector_now(self) -> String {
        untrack(|| self.full_selector())
    }

    /// Valeur effective dans le canevas.
    pub fn computed(self, prop: &str) -> String {
        self.ed
            .computed
            .with(|c| c.get(prop).cloned().unwrap_or_default())
    }

    /// Valeur d'une propriété et son origine (réactif).
    pub fn get(self, prop: &str) -> PropValue {
        let here = self
            .rule
            .with(|r| r.as_ref().and_then(|r| values::read(&r.decls, prop)));
        let inherited = self.sources.with(|s| {
            s.iter().find_map(|(decls, label)| {
                values::read(decls, prop).map(|f| (f.value, label.clone()))
            })
        });
        let computed = self.computed(prop);
        let resettable = here.is_some()
            && self.rule.with(|r| {
                r.as_ref()
                    .is_some_and(|r| values::edits(&r.decls, prop, None).is_some())
            });
        match (here, inherited) {
            (Some(f), inh) => PropValue {
                here: f.value,
                inherited: inh.and_then(|(v, _)| v),
                origin: Origin::Here,
                computed,
                resettable,
            },
            (None, Some((v, label))) => PropValue {
                here: None,
                inherited: v,
                origin: Origin::Inherited(label),
                computed,
                resettable: false,
            },
            (None, None) => PropValue {
                here: None,
                inherited: None,
                origin: Origin::None,
                computed,
                resettable: false,
            },
        }
    }

    /// Texte de l'origine d'une valeur (menu du libellé).
    pub fn origin_text(self, origin: &Origin) -> String {
        match origin {
            Origin::Here => {
                let bp = self.ed.breakpoint.get();
                let name = self
                    .ed
                    .doc
                    .with(|d| {
                        d.as_ref()?
                            .breakpoints
                            .iter()
                            .find(|b| b.id == bp)
                            .map(|b| bp_name(self.ui, b))
                    })
                    .unwrap_or_default();
                self.ui.tx(
                    "styles-origin-here",
                    &[("source", format!("{} · {name}", self.full_selector()))],
                )
            }
            Origin::Inherited(label) => self
                .ui
                .tx("styles-origin-inherited", &[("source", label.clone())]),
            Origin::None => self.ui.tx("styles-origin-none", &[]),
        }
    }

    /// Règle une propriété (`None` ou vide : retirée).
    pub fn set(self, prop: &str, value: Option<String>) {
        self.set_many(vec![(prop.to_string(), value)], false);
    }

    /// Règle plusieurs propriétés en une seule étape d'annulation. `coalesce` : fusionnée avec la
    /// précédente si elle porte sur le même champ (flèches du clavier).
    pub fn set_many(self, list: Vec<(String, Option<String>)>, coalesce: bool) {
        // Même modification reçue deux fois avant que le document ne change (Entrée puis
        // `change` d'un même champ) : une seule commande, sinon la règle serait créée deux fois.
        let key = format!(
            "{}|{:?}|{list:?}|{:?}",
            self.full_selector_now(),
            self.media.get_untracked(),
            self.rule.get_untracked()
        );
        let now = js_sys::Date::now();
        if LAST_SET.with(|l| {
            l.borrow()
                .as_ref()
                .is_some_and(|(k, t)| *k == key && now - t < 600.0)
        }) {
            return;
        }
        LAST_SET.with(|l| *l.borrow_mut() = Some((key, now)));
        // Élément sans classe (ni balise choisie) : classe créée d'après sa nature, comme les
        // poignées du canevas.
        if self.selector.get_untracked().is_empty() {
            if let Some((at, node)) = self.ed.selected_node() {
                self.ed.set_props(self.ui, at, node, list);
            }
            return;
        }
        let cmd = style_command(
            self.rule.get_untracked(),
            self.full_selector_now(),
            self.media.get_untracked(),
            list,
        );
        match cmd {
            Some(cmd) => self.ed.apply(self.ui, cmd, coalesce),
            None => self.clear_preview(),
        }
    }

    /// Aperçu dans le canevas pendant un geste, sans écrire le fichier.
    pub fn preview(self, list: &[(String, String)]) {
        let sel = self.full_selector_now();
        let body: String = list
            .iter()
            .filter(|(_, v)| !v.trim().is_empty())
            .map(|(p, v)| format!("{p}: {v} !important; "))
            .collect();
        self.ed.post(serde_json::json!({
            "type": "preview",
            "css": format!("{sel} {{ {body}}}"),
        }));
    }

    pub fn clear_preview(self) {
        self.ed
            .post(serde_json::json!({ "type": "preview", "css": "" }));
    }

    /// Ouvre ou ferme un menu.
    pub fn toggle_menu(self, id: u32) {
        self.menu
            .update(|m| *m = if *m == Some(id) { None } else { Some(id) });
    }
}

thread_local! {
    /// Dernière modification envoyée (clé, heure) : doublons rapprochés ignorés.
    static LAST_SET: std::cell::RefCell<Option<(String, f64)>> = const { std::cell::RefCell::new(None) };
}

/// Identifiant unique d'un menu.
pub fn menu_id() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
