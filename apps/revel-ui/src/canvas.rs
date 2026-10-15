//! Zone du canevas : cadre de la page, zoom et déplacement de la vue (comme Figma et Framer),
//! élément glissé depuis le panneau « Ajouter ».
//!
//! - molette sur la page : la page défile ; molette autour du cadre : la vue se déplace ;
//! - Ctrl+molette (ou pincement) : zoom autour du pointeur ;
//! - espace maintenue + glisser, ou bouton du milieu : déplacement de la vue ;
//! - clic dans le vide autour du cadre : plus de sélection.
//!
//! Le cadre garde sa hauteur quand le zoom change (la page n'est pas remise en page) : elle
//! correspond à la place disponible au zoom ajusté.

use crate::Ui;
use crate::editor::{Ed, Selection};
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn CanvasArea(failed: RwSignal<bool>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let area = NodeRef::<html::Div>::new();

    // Taille de la zone (fenêtre, colonnes de l'espace de travail, panneaux masqués).
    let measure = move || {
        if let Some(a) = area.get_untracked() {
            ed.area
                .set((f64::from(a.client_width()), f64::from(a.client_height())));
        }
    };
    let on_resize = window_event_listener(ev::resize, move |_| measure());
    Effect::new(move |_| {
        if let Some(a) = area.get() {
            let cb = wasm_bindgen::closure::Closure::<dyn Fn()>::new(measure);
            if let Ok(ro) = web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref()) {
                ro.observe(&a);
            }
            cb.forget();
            measure();
        }
    });

    // Déplacement de la vue à la souris (espace maintenue ou bouton du milieu).
    let last = StoredValue::new((0.0f64, 0.0f64));
    let start_pan = move |e: &web_sys::PointerEvent| {
        if let Some(t) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = t.set_pointer_capture(e.pointer_id());
        }
        last.set_value((f64::from(e.client_x()), f64::from(e.client_y())));
        ed.panning.set(true);
    };
    let move_pan = move |e: &web_sys::PointerEvent| {
        if !ed.panning.get_untracked() {
            return;
        }
        let (x, y) = (f64::from(e.client_x()), f64::from(e.client_y()));
        let (lx, ly) = last.get_value();
        last.set_value((x, y));
        ed.pan_by(x - lx, y - ly);
    };
    let end_pan = move |_| ed.panning.set(false);
    on_cleanup(move || {
        on_resize.remove();
        ed.space.set(false);
        ed.panning.set(false);
    });

    // Point de la zone (pixels de l'écran) depuis un événement de la fenêtre.
    let local = move |cx: i32, cy: i32| -> Option<(f64, f64)> {
        let r = area.get_untracked()?.get_bounding_client_rect();
        Some((f64::from(cx) - r.left(), f64::from(cy) - r.top()))
    };

    let frame_style = move || {
        let g = ed.geo();
        // Position en `left`/`top`, échelle seule en `transform` (à 100 %, aucune transformation :
        // les pilotes WebDriver calculent mal les clics dans un cadre transformé).
        format!(
            "left:{}px;top:{}px;width:{}px;height:{}px;transform:scale({})",
            g.x, g.y, g.w, g.h, g.s
        )
    };
    let shield = move || ed.space.get() || ed.panning.get() || ed.drag.get().is_some();

    view! {
        <div class="canvas-area" node_ref=area
            class:is-panning=move || ed.panning.get()
            class:is-space=move || ed.space.get()
            on:wheel=move |e: ev::WheelEvent| {
                e.prevent_default();
                if e.ctrl_key() || e.meta_key() {
                    let at = local(e.client_x(), e.client_y());
                    ed.zoom_by((-e.delta_y() * 0.002).exp(), at);
                } else if e.shift_key() && e.delta_x() == 0.0 {
                    ed.pan_by(-e.delta_y(), 0.0);
                } else {
                    ed.pan_by(-e.delta_x(), -e.delta_y());
                }
            }
            on:pointerdown=move |e: ev::PointerEvent| {
                if e.button() == 1 {
                    e.prevent_default();
                    start_pan(&e);
                } else if e.button() == 0 && !ed.space.get_untracked() {
                    // Clic dans le vide autour du cadre : plus de sélection.
                    let on_bg = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                        .is_some_and(|el| el.class_list().contains("canvas-area") || el.class_list().contains("canvas-world"));
                    if on_bg && ed.selected.get_untracked().is_some() {
                        ed.selected.set(None::<Selection>);
                        ed.post(serde_json::json!({ "type": "select", "id": null }));
                    }
                }
            }
            on:pointermove=move |e: ev::PointerEvent| move_pan(&e)
            on:pointerup=end_pan
            on:pointercancel=end_pan>
            <div class="canvas-world">
                <Show when=move || ed.base.get().is_some() fallback=move || view! {
                    <div class="canvas-wait muted">{move || if failed.get() { String::new() } else { ui.tx("editor-loading", &[]) }}</div>
                }>
                    <iframe class="canvas-frame" node_ref=ed.frame title=ui.t("editor-canvas")
                        style=frame_style src=move || ed.src().unwrap_or_default()></iframe>
                </Show>
            </div>
            // Écran transparent au-dessus du cadre pendant un déplacement de la vue ou un glisser
            // depuis le panneau « Ajouter » : le pointeur reste dans l'interface.
            <Show when=shield>
                <div class="canvas-shield"
                    on:pointerdown=move |e: ev::PointerEvent| {
                        if ed.space.get_untracked() || e.button() == 1 {
                            e.prevent_default();
                            start_pan(&e);
                        }
                    }></div>
            </Show>
            <CanvasMenu/>
            {move || ed.drag.get().map(|d| {
                let (x, y) = ed.drag_pos.get();
                view! {
                    <div class="drag-ghost" class:is-off=move || ed.drop.get().is_none()
                        style=format!("left:{}px;top:{}px", x + 12.0, y + 14.0)>{d.label}</div>
                }
            })}
        </div>
    }
}

/// Menu contextuel du canevas (clic droit sur un élément de la page).
#[component]
fn CanvasMenu() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let close = move || ed.menu.set(None);
    let on_down = window_event_listener(ev::pointerdown, move |e: web_sys::PointerEvent| {
        if ed.menu.get_untracked().is_none() {
            return;
        }
        let inside = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest(".canvas-menu").ok().flatten())
            .is_some();
        if !inside {
            ed.menu.set(None);
        }
    });
    let on_key = window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" && ed.menu.get_untracked().is_some() {
            ed.menu.set(None);
        }
    });
    let on_blur = window_event_listener(ev::blur, move |_| {
        // Le canevas prend le focus au clic droit : le menu reste ouvert.
        let to_canvas = document()
            .active_element()
            .is_some_and(|el| el.tag_name() == "IFRAME");
        if !to_canvas {
            ed.menu.set(None);
        }
    });
    on_cleanup(move || {
        on_down.remove();
        on_key.remove();
        on_blur.remove();
    });
    let has_sel = move || ed.selected.get().is_some();
    let item = move |label: &'static str,
                     key: &'static str,
                     icon: fn() -> AnyView,
                     enabled: Signal<bool>,
                     run: Box<dyn Fn() + Send + Sync>| {
        view! {
            <button class="menu-item context-item" role="menuitem" prop:disabled=move || !enabled.get()
                on:click=move |_| { close(); run() }>
                <span class="menu-icon">{icon()}</span>
                <span class="menu-label">{ui.t(label)}</span>
                {(!key.is_empty()).then(|| view! { <kbd>{ui.t(key)}</kbd> })}
            </button>
        }
    };
    let sel = Signal::derive(has_sel);
    let clip = Signal::derive(move || ui.clipboard.with(Option::is_some));
    move || {
        ed.menu.get().map(|(x, y)| {
            // Menu gardé dans la fenêtre.
            let (w, h) = (240.0, 360.0);
            let vw = window().inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1e4);
            let vh = window().inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(1e4);
            let (x, y) = (x.min(vw - w - 8.0).max(8.0), y.min(vh - h - 8.0).max(8.0));
            view! {
                <div class="context-menu canvas-menu pop" role="menu" style=format!("position:fixed;left:{x}px;top:{y}px")>
                    {item("editor-select-parent", "key-parent", crate::icons::parent, sel, Box::new(move || ed.post(serde_json::json!({ "type": "nav", "dir": "parent" }))))}
                    {item("editor-zoom-selection", "key-zoom-sel", crate::icons::search, sel, Box::new(move || ed.zoom_selection()))}
                    <hr class="menu-sep"/>
                    {item("editor-copy", "key-copy", crate::icons::copy, sel, Box::new(move || { ed.copy(ui, false); }))}
                    {item("editor-cut", "key-cut", crate::icons::copy, sel, Box::new(move || { ed.copy(ui, true); }))}
                    {item("editor-paste", "key-paste", crate::icons::copy, clip, Box::new(move || { ed.paste(ui); }))}
                    {item("layers-duplicate", "key-duplicate", crate::icons::copy, sel, Box::new(move || {
                        if let Some(at) = ed.selected_target() { ed.apply(ui, revel_edit::Command::Duplicate { at }, false) }
                    }))}
                    <hr class="menu-sep"/>
                    {item("layers-wrap", "key-wrap", crate::icons::wrap, sel, Box::new(move || {
                        if let Some(at) = ed.selected_target() { ed.wrap(ui, at) }
                    }))}
                    {item("editor-unwrap", "key-unwrap", crate::icons::reverse, sel, Box::new(move || { ed.unwrap(ui); }))}
                    {item("layers-up", "", crate::icons::arrow_up, sel, Box::new(move || { ed.step(ui, -1); }))}
                    {item("layers-down", "", crate::icons::arrow_down, sel, Box::new(move || { ed.step(ui, 1); }))}
                    <hr class="menu-sep"/>
                    {item("layers-delete", "key-delete", crate::icons::trash, sel, Box::new(move || {
                        if let Some(at) = ed.selected_target() { ed.apply(ui, revel_edit::Command::Remove { at }, false) }
                    }))}
                </div>
            }
        })
    }
}
