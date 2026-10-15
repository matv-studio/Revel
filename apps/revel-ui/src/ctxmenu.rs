//! Menus contextuels de l'interface (clic droit) : chaque zone propose ses propres actions
//! (onglet d'un panneau : fermer, détacher ; ligne d'un tableau : ouvrir, supprimer…). Le menu du
//! navigateur est désactivé partout ailleurs que dans les champs de saisie (`main.rs`).
//!
//! Une ligne : icône, libellé, raccourci (à droite, sans le répéter dans le libellé).

use crate::Ui;
use leptos::ev;
use leptos::prelude::*;
use std::sync::Arc;
use wasm_bindgen::JsCast;

/// Ligne d'un menu contextuel.
#[derive(Clone)]
pub struct CtxItem {
    pub label: String,
    pub icon: Option<fn() -> AnyView>,
    /// Raccourci affiché à droite (`Ctrl+D`).
    pub key: Option<String>,
    pub danger: bool,
    pub disabled: bool,
    /// Trait de séparation au-dessus.
    pub sep: bool,
    pub run: Arc<dyn Fn() + Send + Sync>,
}

impl CtxItem {
    pub fn new(label: String, run: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            label,
            icon: None,
            key: None,
            danger: false,
            disabled: false,
            sep: false,
            run: Arc::new(run),
        }
    }
    pub fn icon(mut self, f: fn() -> AnyView) -> Self {
        self.icon = Some(f);
        self
    }
    pub fn key(mut self, k: String) -> Self {
        self.key = Some(k);
        self
    }
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    pub fn sep(mut self) -> Self {
        self.sep = true;
        self
    }
}

/// Menu ouvert : position dans la fenêtre, lignes.
#[derive(Clone)]
pub struct CtxMenu {
    pub x: f64,
    pub y: f64,
    pub items: Vec<CtxItem>,
    /// Heure d'ouverture (ms) : le focus passé au canevas juste avant ne le ferme pas.
    pub at: f64,
    /// Ouvert depuis le canevas (qui prend le focus au clic droit).
    pub canvas: bool,
}

/// Ouvre un menu à une position de la fenêtre (`canvas` : clic droit dans le canevas).
pub fn open_at(ui: Ui, x: f64, y: f64, items: Vec<CtxItem>, canvas: bool) {
    if items.is_empty() {
        return;
    }
    ui.context.set(Some(CtxMenu {
        x,
        y,
        items,
        at: js_sys::Date::now(),
        canvas,
    }));
}

/// Ouvre un menu à la position du clic droit.
pub fn open(ui: Ui, e: &web_sys::MouseEvent, items: Vec<CtxItem>) {
    e.prevent_default();
    e.stop_propagation();
    open_at(
        ui,
        f64::from(e.client_x()),
        f64::from(e.client_y()),
        items,
        false,
    );
}

/// Couche du menu contextuel (une seule pour toute l'interface) : fermée par un clic ailleurs,
/// Échap, la perte du focus, un défilement ou un changement de taille de la fenêtre.
#[component]
pub fn ContextLayer() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let close = move || ui.context.set(None);
    let on_down = window_event_listener(ev::pointerdown, move |e: web_sys::PointerEvent| {
        if ui.context.with_untracked(Option::is_none) {
            return;
        }
        let inside = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest(".ctx-menu").ok().flatten())
            .is_some();
        if !inside {
            close();
        }
    });
    let on_key = window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" && ui.context.with_untracked(Option::is_some) {
            e.stop_propagation();
            close();
        }
    });
    let on_blur = window_event_listener(ev::blur, move |_| {
        // Clic droit dans le canevas : il prend le focus au moment où le menu s'ouvre.
        let just = ui.context.with_untracked(|m| {
            m.as_ref()
                .is_some_and(|m| m.canvas && js_sys::Date::now() - m.at < 400.0)
        });
        if !just {
            close();
        }
    });
    let on_resize = window_event_listener(ev::resize, move |_| close());
    on_cleanup(move || {
        on_down.remove();
        on_key.remove();
        on_blur.remove();
        on_resize.remove();
    });
    move || {
        ui.context.get().map(|m| {
            // Menu gardé dans la fenêtre.
            let h = m.items.len() as f64 * 30.0 + m.items.iter().filter(|i| i.sep).count() as f64 * 9.0 + 8.0;
            let w = 240.0;
            let vw = window().inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1e4);
            let vh = window().inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(1e4);
            let x = if m.x + w > vw - 8.0 { (m.x - w).max(8.0) } else { m.x };
            let y = if m.y + h > vh - 8.0 { (m.y - h).max(8.0) } else { m.y };
            view! {
                <div class="ctx-menu context-menu pop" role="menu" style=format!("position:fixed;left:{x}px;top:{y}px")
                    on:contextmenu=move |e: ev::MouseEvent| e.prevent_default()>
                    {m.items.into_iter().map(|it| {
                        let run = it.run.clone();
                        view! {
                            {it.sep.then(|| view! { <hr class="menu-sep"/> })}
                            <button class="menu-item context-item" class:is-danger=it.danger role="menuitem"
                                disabled=it.disabled
                                on:click=move |_| { close(); run() }>
                                <span class="menu-icon">{it.icon.map(|f| f())}</span>
                                <span class="menu-label">{it.label}</span>
                                {it.key.map(|k| view! { <kbd>{k}</kbd> })}
                            </button>
                        }
                    }).collect_view()}
                </div>
            }
        })
    }
}
