//! Zone du canevas : cadre de la page, zoom et déplacement de la vue (comme Figma et Framer),
//! élément glissé depuis le panneau « Ajouter ».
//!
//! - molette sur la page : la page défile ; molette autour du cadre : la vue se déplace ;
//! - Alt+molette (ou Ctrl+molette, pincement) : zoom autour du pointeur ;
//! - espace maintenue + glisser, ou bouton du milieu : déplacement de la vue ;
//! - clic dans le vide autour du cadre : plus de sélection ;
//! - Ctrl+R : règles en pixels de la page ; guides de colonnes par-dessus la page.
//!
//! La molette est écoutée directement sur la zone, en mode actif (`passive: false`) : un
//! écouteur délégué à la fenêtre (Leptos) y serait passif, le navigateur ignorerait alors
//! `preventDefault` (https://developer.chrome.com/blog/scrolling-intervention-2/).
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

    // Changement de taille d'écran (point de rupture) : le cadre glisse vers sa nouvelle largeur.
    let resizing = RwSignal::new(false);
    let timer = StoredValue::new(None::<leptos::leptos_dom::helpers::TimeoutHandle>);
    Effect::new(move |prev: Option<String>| {
        let bp = ed.breakpoint.get();
        if prev.is_some_and(|p| p != bp) {
            resizing.set(true);
            if let Some(t) = timer.get_value() {
                t.clear();
            }
            timer.set_value(
                set_timeout_with_handle(
                    move || resizing.set(false),
                    std::time::Duration::from_millis(420),
                )
                .ok(),
            );
        }
        bp
    });

    // Molette : écouteur actif posé sur la zone elle-même.
    Effect::new(move |_| {
        let Some(a) = area.get() else {
            return;
        };
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::WheelEvent)>::new(
            move |e: web_sys::WheelEvent| {
                e.prevent_default();
                if e.alt_key() || e.ctrl_key() || e.meta_key() {
                    let at = local(e.client_x(), e.client_y());
                    // Alt+molette : certaines souris donnent un défilement horizontal.
                    let d = if e.delta_y() != 0.0 {
                        e.delta_y()
                    } else {
                        e.delta_x()
                    };
                    ed.zoom_by((-d * 0.002).exp(), at);
                } else if e.shift_key() && e.delta_x() == 0.0 {
                    ed.pan_by(-e.delta_y(), 0.0);
                } else {
                    ed.pan_by(-e.delta_x(), -e.delta_y());
                }
            },
        );
        let opts = web_sys::AddEventListenerOptions::new();
        opts.set_passive(false);
        let _ = a.add_event_listener_with_callback_and_add_event_listener_options(
            "wheel",
            cb.as_ref().unchecked_ref(),
            &opts,
        );
        cb.forget();
    });

    view! {
        <div class="canvas-area" node_ref=area
            class:is-panning=move || ed.panning.get()
            class:is-space=move || ed.space.get()
            class:has-rulers=move || ed.rulers.get()
            class:is-resizing=move || resizing.get()
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
                    <ColumnGuides/>
                </Show>
            </div>
            <Show when=move || ed.rulers.get()>
                <Rulers/>
            </Show>
            // Composant ouvert (modification isolée) : son nom, « Terminer ».
            {move || ed.isolated.get().map(|c| {
                let name = ed.doc.with(|d| d.as_ref().and_then(|d| d.component(&c).map(|c| c.label()))).unwrap_or(c);
                view! {
                    <div class="iso-bar" role="status">
                        <span class="iso-name">{crate::icons::component()}{name}</span>
                        <button class="btn btn--small iso-done" on:click=move |_| ed.close_component()>{ui.t("component-done")}</button>
                    </div>
                }
            })}
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

/// Guides de colonnes par-dessus la page (comme les grilles de Figma) : bandes des colonnes,
/// à la position et à l'échelle du cadre, sans capter le pointeur.
#[component]
fn ColumnGuides() -> impl IntoView {
    let ed = expect_context::<Ed>();
    let style = move || {
        let g = ed.geo();
        format!(
            "left:{}px;top:{}px;width:{}px;height:{}px;transform:scale({})",
            g.x, g.y, g.w, g.h, g.s
        )
    };
    let columns = move || {
        let gd = ed.guides.get();
        if !gd.on || gd.count == 0 {
            return Vec::new();
        }
        let w = ed.geo().w;
        let margin = f64::from(gd.margin);
        let mut inner = (w - 2.0 * margin).max(0.0);
        if gd.max > 0 {
            inner = inner.min(f64::from(gd.max));
        }
        let left = (w - inner) / 2.0;
        let n = f64::from(gd.count);
        let gutter = f64::from(gd.gutter);
        let col = ((inner - (n - 1.0) * gutter) / n).max(0.0);
        (0..gd.count)
            .map(|i| (left + f64::from(i) * (col + gutter), col))
            .collect::<Vec<_>>()
    };
    view! {
        <Show when=move || ed.guides.get().on>
            <div class="column-guides" style=style aria-hidden="true">
                {move || columns().into_iter().map(|(x, w)| view! {
                    <div class="column-guide" style=format!("left:{x}px;width:{w}px")></div>
                }).collect_view()}
            </div>
        </Show>
    }
}

/// Pas des graduations : le plus petit dont les étiquettes restent espacées d'au moins `min`
/// pixels de l'écran.
fn ruler_step(scale: f64, min: f64) -> f64 {
    [
        1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0,
    ]
    .into_iter()
    .find(|s| s * scale >= min)
    .unwrap_or(10000.0)
}

/// Règles en pixels de la page (Ctrl+R) : en haut la largeur, à gauche la hauteur (défilement
/// compris) ; la page est plus claire que ses abords.
#[component]
fn Rulers() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let top = NodeRef::<html::Canvas>::new();
    let left = NodeRef::<html::Canvas>::new();
    const T: f64 = 20.0;
    let color = |name: &str| {
        window()
            .get_computed_style(&document().document_element()?)
            .ok()
            .flatten()?
            .get_property_value(name)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    Effect::new(move |_| {
        let g = ed.geo();
        let (aw, ah) = ed.area.get();
        let (_, sy) = ed.live_scroll.get();
        let _ = ui.settings.with(|s| s.theme.clone());
        let (Some(tc), Some(lc)) = (top.get(), left.get()) else {
            return;
        };
        let dpr = window().device_pixel_ratio().max(1.0);
        let ink = color("--text-muted").unwrap_or_else(|| "#888".into());
        let page = color("--surface").unwrap_or_else(|| "#222".into());
        let out = color("--chrome").unwrap_or_else(|| "#111".into());
        let step = ruler_step(g.s, 56.0);
        let minor = if (step / 5.0) * g.s >= 6.0 {
            step / 5.0
        } else {
            step
        };
        // (toile, longueur, origine écran, origine page, partie visible de la page, vertical)
        for (cv, len, o, p0, (from, to), vertical) in [
            (tc, aw, g.x, 0.0, (0.0, g.w), false),
            (lc, ah - T, g.y - T, sy, (sy, sy + g.h), true),
        ] {
            let el: &web_sys::HtmlCanvasElement = &cv;
            let (w, h) = if vertical { (T, len) } else { (len, T) };
            el.set_width((w * dpr) as u32);
            el.set_height((h * dpr) as u32);
            let _ = el.set_attribute("style", &format!("width:{w}px;height:{h}px"));
            let Some(ctx) = el
                .get_context("2d")
                .ok()
                .flatten()
                .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
            else {
                continue;
            };
            let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
            if vertical {
                // Axe vertical dessiné comme l'horizontal, tourné.
                let _ = ctx.translate(T, 0.0);
                let _ = ctx.rotate(std::f64::consts::FRAC_PI_2);
            }
            ctx.set_fill_style_str(&out);
            ctx.fill_rect(0.0, 0.0, len, T);
            // Page visible (en pixels de l'écran).
            let a = o + (from - p0) * g.s;
            let b = o + (to - p0) * g.s;
            ctx.set_fill_style_str(&page);
            ctx.fill_rect(a.max(0.0), 0.0, (b.min(len) - a.max(0.0)).max(0.0), T);
            ctx.set_fill_style_str(&ink);
            ctx.set_stroke_style_str(&ink);
            ctx.set_font("10px system-ui, sans-serif");
            ctx.set_line_width(1.0);
            let first = ((p0 + (0.0 - o) / g.s) / minor).floor() * minor;
            let last = p0 + (len - o) / g.s;
            let mut v = first;
            ctx.begin_path();
            while v <= last {
                let x = (o + (v - p0) * g.s).round() + 0.5;
                let major = (v / step).round() * step == v;
                let tick = if major { T * 0.55 } else { T * 0.25 };
                // Graduation montée depuis le bord intérieur (côté page).
                let (y0, y1) = if vertical { (0.0, tick) } else { (T - tick, T) };
                ctx.move_to(x, y0);
                ctx.line_to(x, y1);
                if major {
                    let _ = ctx.fill_text(
                        &format!("{}", v as i64),
                        x + 3.0,
                        if vertical { T - 4.0 } else { 10.0 },
                    );
                }
                v += minor;
            }
            ctx.stroke();
        }
    });
    view! {
        <div class="canvas-rulers" aria-hidden="true">
            <canvas class="ruler ruler--top" node_ref=top></canvas>
            <canvas class="ruler ruler--left" node_ref=left></canvas>
            <div class="ruler-corner"></div>
        </div>
    }
}

/// Menu contextuel du canevas (clic droit sur un élément de la page), à la position donnée
/// dans la fenêtre.
pub fn open_canvas_menu(ui: Ui, ed: Ed, x: f64, y: f64) {
    use crate::ctxmenu::{CtxItem, open_at};
    use crate::icons;
    let sel = ed.selected.get_untracked().is_none();
    let clip = ui.clipboard.with_untracked(Option::is_none);
    let k = |key: &str| ui.now(key, &[]);
    let items = vec![
        CtxItem::new(k("editor-select-parent"), move || {
            ed.post(serde_json::json!({ "type": "nav", "dir": "parent" }))
        })
        .icon(icons::parent)
        .key(k("key-parent"))
        .disabled(sel),
        CtxItem::new(k("editor-zoom-selection"), move || ed.zoom_selection())
            .icon(icons::search)
            .key(k("key-zoom-sel"))
            .disabled(sel),
        CtxItem::new(k("editor-copy"), move || {
            ed.copy(ui, false);
        })
        .icon(icons::copy)
        .key(k("key-copy"))
        .disabled(sel)
        .sep(),
        CtxItem::new(k("editor-cut"), move || {
            ed.copy(ui, true);
        })
        .icon(icons::copy)
        .key(k("key-cut"))
        .disabled(sel),
        CtxItem::new(k("editor-paste"), move || {
            ed.paste(ui);
        })
        .icon(icons::copy)
        .key(k("key-paste"))
        .disabled(clip),
        CtxItem::new(k("layers-duplicate"), move || {
            if let Some(at) = ed.selected_target() {
                ed.apply(ui, revel_edit::Command::Duplicate { at }, false)
            }
        })
        .icon(icons::copy)
        .key(k("key-duplicate"))
        .disabled(sel),
        CtxItem::new(k("layers-wrap"), move || {
            if let Some(at) = ed.selected_target() {
                ed.wrap(ui, at)
            }
        })
        .icon(icons::wrap)
        .key(k("key-wrap"))
        .disabled(sel)
        .sep(),
        CtxItem::new(k("editor-unwrap"), move || {
            ed.unwrap(ui);
        })
        .icon(icons::reverse)
        .key(k("key-unwrap"))
        .disabled(sel),
        CtxItem::new(k("layers-up"), move || {
            ed.step(ui, -1);
        })
        .icon(icons::arrow_up)
        .key(k("key-up"))
        .disabled(sel),
        CtxItem::new(k("layers-down"), move || {
            ed.step(ui, 1);
        })
        .icon(icons::arrow_down)
        .key(k("key-down"))
        .disabled(sel),
        CtxItem::new(k("layers-delete"), move || {
            if let Some(at) = ed.selected_target() {
                ed.apply(ui, revel_edit::Command::Remove { at }, false)
            }
        })
        .icon(icons::trash)
        .key(k("key-delete"))
        .disabled(sel)
        .danger()
        .sep(),
    ];
    let mut items = items;
    // Composants : créer à partir de l'élément ; ouvrir ou détacher une utilisation.
    let target = ed.selected_target();
    let is_use = target.as_ref().is_some_and(|at| {
        ed.doc.with_untracked(|d| {
            d.as_ref()
                .and_then(|d| revel_edit::address::node(d, at).ok())
                .is_some_and(|n| matches!(n, revel_model::Node::Use { .. }))
        })
    });
    let at = StoredValue::new(target);
    let delete = items.pop();
    if is_use {
        items.push(
            CtxItem::new(k("component-edit"), move || {
                if let Some(at) = at.get_value() {
                    crate::components::open_component(ed, &at)
                }
            })
            .icon(icons::edit)
            .sep(),
        );
        items.push(
            CtxItem::new(k("component-detach"), move || {
                if let Some(at) = at.get_value() {
                    crate::components::detach(ui, ed, at)
                }
            })
            .icon(icons::unlink),
        );
    } else {
        items.push(
            CtxItem::new(k("component-make"), move || {
                crate::components::make_from_selection(ui, ed)
            })
            .icon(icons::component)
            .disabled(sel)
            .sep(),
        );
    }
    items.extend(delete);
    open_at(ui, x, y, items, true);
}
