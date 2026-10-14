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
        shift: bool,
    },
    Scroll {
        x: f64,
        y: f64,
    },
    Computed {
        values: std::collections::HashMap<String, String>,
    },
    /// Texte modifié sur place (étape 2.7).
    Text {
        slot: revel_edit::canvas::Slot,
        text: String,
    },
    #[serde(other)]
    Other,
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
    /// Heure de la dernière modification faite dans Revel (ms) : les fichiers changés juste après
    /// ne sont pas pris pour une modification extérieure.
    pub own_change: RwSignal<f64>,
    frame: NodeRef<html::Iframe>,
}

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
    fn src(self) -> Option<String> {
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
        });
    }

    pub fn undo(self, ui: Ui) {
        if !self
            .state
            .with_untracked(|s| s.as_ref().is_some_and(|s| s.can_undo))
        {
            return;
        }
        let path = self.path.get_value();
        spawn_local(async move {
            match api::edit_undo(&path).await {
                Ok(st) => self.after(ui, st),
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
        spawn_local(async move {
            match api::edit_redo(&path).await {
                Ok(st) => self.after(ui, st),
                Err(e) => ui.fail(e),
            }
        });
    }

    /// Applique une commande d'édition.
    pub fn apply(self, ui: Ui, cmd: Command, coalesce: bool) {
        let path = self.path.get_value();
        spawn_local(async move {
            match api::edit_apply(&path, &cmd, coalesce).await {
                Ok(st) => self.after(ui, st),
                Err(e) => ui.fail(e),
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
    fn frame_width(self) -> Option<f64> {
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

/// Propriétaire d'un nœud (page, gabarit, composant) : libellé pour la barre d'état.
fn owner_of(doc: &Site, id: &str) -> Option<(revel_model::ids::OwnerKind, String)> {
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
        selected: RwSignal::new(None),
        hovered: RwSignal::new(None),
        scripts: RwSignal::new(false),
        load: RwSignal::new(0),
        scroll: RwSignal::new((0.0, 0.0)),
        computed: RwSignal::new(Default::default()),
        own_change: RwSignal::new(0.0),
        frame: NodeRef::new(),
    };
    provide_context(ed);
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
            FromCanvas::Select { id, tag, label } => {
                if id.is_none() {
                    ed.computed.set(Default::default());
                }
                ed.selected.set(id.map(|id| Selection { id, tag, label }));
            }
            FromCanvas::Key { key, shift } => match key.to_lowercase().as_str() {
                "z" if shift => ed.redo(ui),
                "z" => ed.undo(ui),
                "y" => ed.redo(ui),
                _ => {}
            },
            FromCanvas::Scroll { x, y } => ed.scroll.set((x, y)),
            FromCanvas::Computed { values } => ed.computed.set(values),
            FromCanvas::Text { slot, text } => {
                if let Some(cmd) = ed.text_command(&slot, text) {
                    ed.apply(ui, cmd, false);
                }
            }
            FromCanvas::Other => {}
        }
    });
    // Raccourcis dans l'interface (hors champs de saisie).
    let on_key = window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if !(e.ctrl_key() || e.meta_key()) {
            return;
        }
        let in_field = document()
            .active_element()
            .map(|el| matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT"))
            .unwrap_or(false);
        if in_field {
            return;
        }
        match e.key().to_lowercase().as_str() {
            "z" if e.shift_key() => {
                e.prevent_default();
                ed.redo(ui);
            }
            "z" => {
                e.prevent_default();
                ed.undo(ui);
            }
            "y" => {
                e.prevent_default();
                ed.redo(ui);
            }
            _ => {}
        }
    });
    on_cleanup(move || {
        on_message.remove();
        on_key.remove();
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

    // Place disponible pour le cadre (zoom ajusté).
    let area = NodeRef::<html::Div>::new();
    let area_w = RwSignal::new(0.0f64);
    let area_h = RwSignal::new(0.0f64);
    let measure = move || {
        if let Some(a) = area.get_untracked() {
            area_w.set(f64::from(a.client_width()));
            area_h.set(f64::from(a.client_height()));
        }
    };
    let on_resize = window_event_listener(ev::resize, move |_| measure());
    on_cleanup(move || on_resize.remove());
    Effect::new(move |_| {
        if area.get().is_some() {
            measure();
        }
    });

    // Largeur du cadre et échelle.
    let gutter = 48.0;
    // Base (ordinateur) : toute la place, mais au moins la largeur qui dépasse le plus grand point de
    // rupture « jusqu'à » (sinon la page s'afficherait déjà dans sa version étroite).
    let desktop_min = move || {
        ed.doc.with(|d| {
            d.as_ref()
                .and_then(|d| d.breakpoints.iter().filter_map(|b| b.max_width).max())
                .map(|w| f64::from(w) + 1.0)
                .unwrap_or(320.0)
        })
    };
    let width = move || {
        ed.frame_width()
            .unwrap_or_else(|| (area_w.get() - gutter).max(desktop_min()))
    };
    let scale = move || match ed.zoom.get() {
        Some(z) => z,
        None => ((area_w.get() - gutter) / width()).clamp(0.1, 1.0),
    };
    let frame_style = move || {
        let s = scale();
        let w = width();
        let h = ((area_h.get() - gutter) / s).max(200.0);
        format!(
            "width:{w}px;height:{h}px;transform:scale({s});margin-left:{}px",
            ((area_w.get() - w * s) / 2.0).max(gutter / 2.0)
        )
    };

    view! {
        <section class="editor">
            <Toolbar/>
            <LeftPanel/>
            <crate::inspector::Inspector/>
            <div class="canvas-area" node_ref=area>
                <Show when=move || ed.base.get().is_some() fallback=move || view! {
                    <div class="canvas-wait muted">{move || if failed.get() { String::new() } else { ui.tx("editor-loading", &[]) }}</div>
                }>
                    <iframe class="canvas-frame" node_ref=ed.frame title=ui.t("editor-canvas")
                        style=frame_style src=move || ed.src().unwrap_or_default()></iframe>
                </Show>
            </div>
            <StatusBar/>
        </section>
    }
}

/// Panneau de gauche : calques ou éléments à ajouter.
#[component]
fn LeftPanel() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let insert = RwSignal::new(false);
    view! {
        <div class="left-panel">
            <div class="panel-tabs" role="tablist">
                <button role="tab" class:is-on=move || !insert.get() aria-selected=move || (!insert.get()).to_string()
                    on:click=move |_| insert.set(false)>{ui.t("layers-title")}</button>
                <button role="tab" class:is-on=move || insert.get() aria-selected=move || insert.get().to_string()
                    on:click=move |_| insert.set(true)>{ui.t("insert-title")}</button>
            </div>
            <Show when=move || insert.get() fallback=|| view! { <crate::layers::Layers/> }>
                <crate::insert::Insert/>
            </Show>
        </div>
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
    let zooms: [(Option<f64>, &'static str); 5] = [
        (None, "editor-zoom-fit"),
        (Some(0.5), "50 %"),
        (Some(0.75), "75 %"),
        (Some(1.0), "100 %"),
        (Some(1.25), "125 %"),
    ];
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
                            on:click=move |_| ed.breakpoint.set(id.clone())>
                            {bp_icon(&b)}
                            <span class="bp-width">{b.max_width.or(b.min_width).map(|w| w.to_string()).unwrap_or_default()}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
            <BreakpointMenu/>
            <div class="select editor-zoom">
                <select class="input" aria-label=ui.t("editor-zoom")
                    on:change=move |e| {
                        let v = event_target_value(&e);
                        ed.zoom.set(v.parse::<f64>().ok());
                    }>
                    {zooms.into_iter().map(|(z, key)| {
                        let label = if key.starts_with("editor-") { ui.tx(key, &[]) } else { key.to_string() };
                        view! { <option value=z.map(|z| z.to_string()).unwrap_or_default() selected=move || ed.zoom.get() == z>{label}</option> }
                    }).collect_view()}
                </select>
            </div>
            <label class="editor-scripts" title=ui.t("editor-scripts-help")>
                <span class="switch">
                    <input type="checkbox" prop:checked=move || ed.scripts.get()
                        on:change=move |e| set_scripts(event_target_checked(&e))/>
                    <span class="switch-track"></span>
                </span>
                <span class="muted">{ui.t("editor-scripts")}</span>
            </label>
            <div class="toolbar-spacer"></div>
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
    view! {
        <footer class="editor-status">
            <span class="strong">{move || ed.selected.get().map(|s| s.label).unwrap_or_else(|| ui.tx("editor-no-selection", &[]))}</span>
            <span class="muted">{owner}</span>
            <span class="toolbar-spacer"></span>
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
    let hook = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&hook, &JsValue::from_str("apply"), apply.as_ref());
    apply.forget();
    let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__revelTest"), &hook);
}
