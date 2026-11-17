//! Recadrage sans perte d'une image (D-180) : zoom et point de cadrage écrits dans
//! `data-revel-crop` ; le fichier n'est pas touché, le site produit reçoit le style
//! correspondant (`revel_model::Crop::style`). Ouvert par un double-clic sur l'image dans le
//! canevas, sur sa vignette dans le panneau Styles ou sur celle d'un élément d'un outil.

use crate::editor::Ed;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::{Command, Target};
use revel_model::{CROP_ATTR, Crop, Node};

/// Identifiant de l'élément `img` de `n` (lui-même, sinon le premier de ses descendants).
pub fn img_in(n: &Node) -> Option<String> {
    match n {
        Node::Element { id, tag, .. } if tag.eq_ignore_ascii_case("img") => id.clone(),
        _ => revel_model::ids::child_lists(n)
            .into_iter()
            .flatten()
            .find_map(img_in),
    }
}

/// Adresse et cadrage écrits de l'image `at`.
fn image_of(ed: Ed, at: &Target) -> Option<(String, Option<Crop>)> {
    ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let Ok(Node::Element { attrs, .. }) = revel_edit::address::node(d, at) else {
            return None;
        };
        let get = |n: &str| {
            attrs
                .iter()
                .find(|a| a.name == n && a.when.is_none())
                .and_then(|a| a.value.clone())
        };
        let src = get("src").or_else(|| get("data-src")).unwrap_or_default();
        Some((src, get(CROP_ATTR).as_deref().and_then(Crop::parse)))
    })
}

/// Cadrage de l'image `at` (sans valeur : retiré).
pub fn set_crop(ui: Ui, ed: Ed, at: &Target, c: Option<Crop>) {
    let had = image_of(ed, at).is_some_and(|(_, c)| c.is_some());
    let cmd = match c {
        Some(c)
            if !(c.zoom <= 1.0001 && (c.x - 50.0).abs() < 0.01 && (c.y - 50.0).abs() < 0.01) =>
        {
            Command::SetAttr {
                at: at.clone(),
                name: CROP_ATTR.into(),
                value: Some(c.value()),
            }
        }
        _ if had => Command::RemoveAttr {
            at: at.clone(),
            name: CROP_ATTR.into(),
        },
        _ => return,
    };
    ed.apply(ui, cmd, false);
}

/// Cadrage écrit de `at`, sinon le cadrage neutre.
pub fn crop_of(ed: Ed, at: &Target) -> Crop {
    image_of(ed, at).and_then(|(_, c)| c).unwrap_or(Crop {
        x: 50.0,
        y: 50.0,
        zoom: 1.0,
        radius: 0.0,
    })
}

fn px(v: &str) -> Option<f64> {
    v.trim()
        .trim_end_matches("px")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

#[component]
pub fn CropDialog() -> impl IntoView {
    let ed = expect_context::<Ed>();
    move || ed.crop.get().map(|at| view! { <CropBox at/> })
}

#[component]
fn CropBox(at: Target) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let at = StoredValue::new(at);
    let (src, _) = at.with_value(|a| image_of(ed, a)).unwrap_or_default();
    let start = at.with_value(|a| crop_of(ed, a));
    let base = ed.base.get_untracked().unwrap_or_default();
    let url = crate::library::thumb_src(&base, &src);
    let (x, y, z) = (
        RwSignal::new(start.x),
        RwSignal::new(start.y),
        RwSignal::new(start.zoom),
    );
    // Boîte de l'image dans la page (taille calculée), arrondi de ses coins.
    let (bw, bh, radius) = ed.computed.with_untracked(|c| {
        (
            c.get("width")
                .and_then(|v| px(v))
                .filter(|v| *v > 1.0)
                .unwrap_or(400.0),
            c.get("height")
                .and_then(|v| px(v))
                .filter(|v| *v > 1.0)
                .unwrap_or(300.0),
            c.get("border-top-left-radius")
                .and_then(|v| px(v))
                .unwrap_or(start.radius),
        )
    });
    // Cadre de l'aperçu : la boîte ramenée à 560 × 380 au plus.
    let k = (560.0 / bw).min(380.0 / bh);
    let (fw, fh) = (bw * k, bh * k);
    let crop = move || Crop {
        x: x.get(),
        y: y.get(),
        zoom: z.get(),
        radius,
    };
    let close = move || ed.crop.set(None);
    let apply = move || {
        let c = crop();
        at.with_value(|a| set_crop(ui, ed, a, Some(c)));
        close();
    };
    let drag = StoredValue::new(None::<(f64, f64, f64, f64)>);
    let on_down = move |e: leptos::ev::PointerEvent| {
        e.prevent_default();
        use wasm_bindgen::JsCast;
        if let Some(t) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = t.set_pointer_capture(e.pointer_id());
        }
        drag.set_value(Some((
            e.client_x() as f64,
            e.client_y() as f64,
            x.get_untracked(),
            y.get_untracked(),
        )));
    };
    let on_move = move |e: leptos::ev::PointerEvent| {
        if let Some((sx, sy, x0, y0)) = drag.get_value() {
            let zz = z.get_untracked();
            let dx = (e.client_x() as f64 - sx) / fw * 100.0 / zz;
            let dy = (e.client_y() as f64 - sy) / fh * 100.0 / zz;
            x.set((x0 - dx).clamp(0.0, 100.0));
            y.set((y0 - dy).clamp(0.0, 100.0));
        }
    };
    let on_up = move |_| drag.set_value(None);
    let on_wheel = move |e: leptos::ev::WheelEvent| {
        e.prevent_default();
        z.update(|v| *v = (*v * (1.0 - e.delta_y() * 0.0015)).clamp(1.0, 4.0));
    };
    let on_key = move |e: leptos::ev::KeyboardEvent| match e.key().as_str() {
        "Escape" => {
            e.stop_propagation();
            close()
        }
        "Enter" => {
            e.prevent_default();
            apply()
        }
        _ => {}
    };
    let pct = |v: f64| format!("{}", v.round() as i64);
    view! {
        <div class="overlay crop-overlay" on:mousedown=move |e| { if e.target() == e.current_target() { close() } } on:keydown=on_key>
            <div class="dialog crop-dialog" role="dialog" aria-modal="true" aria-labelledby="crop-title" tabindex="-1"
                node_ref={ let r = NodeRef::<leptos::html::Div>::new(); Effect::new(move |_| { if let Some(el) = r.get() { let _ = el.focus(); } }); r }>
                <h2 id="crop-title" class="dialog-title">{icons::crop()}<span>{ui.t("crop-title")}</span></h2>
                <div class="crop-stage">
                    <div class="crop-frame" title=ui.t("crop-frame-help")
                        style=format!("width:{fw:.0}px;height:{fh:.0}px;border-radius:{:.1}px", radius * k)
                        on:pointerdown=on_down on:pointermove=on_move on:pointerup=on_up on:pointercancel=on_up on:wheel=on_wheel>
                        <img src=url.clone() alt="" draggable="false"
                            style=move || format!("{};width:100%;height:100%;display:block", crop().style())/>
                        <span class="crop-grid" aria-hidden="true"></span>
                    </div>
                </div>
                <div class="crop-controls">
                    <label class="crop-row" title=ui.t("crop-zoom-help")>
                        <span class="label">{ui.t("crop-zoom")}</span>
                        <input class="range" type="range" min="100" max="400" step="1" data-role="crop-zoom"
                            prop:value=move || pct(z.get() * 100.0)
                            on:input=move |e| { if let Ok(v) = event_target_value(&e).parse::<f64>() { z.set((v / 100.0).clamp(1.0, 4.0)) } }/>
                        <span class="crop-value">{move || format!("{} %", pct(z.get() * 100.0))}</span>
                    </label>
                    <div class="crop-row" title=ui.t("crop-position-help")>
                        <span class="label">{ui.t("crop-position")}</span>
                        <span class="crop-xy">
                            {[("X", x), ("Y", y)].into_iter().map(|(n, s)| view! {
                                <label class="crop-num"><span class="muted">{n}</span>
                                    <input class="input input--mono" type="number" min="0" max="100" step="1" data-role=format!("crop-{}", n.to_lowercase())
                                        prop:value=move || pct(s.get())
                                        on:change=move |e| { if let Ok(v) = event_target_value(&e).parse::<f64>() { s.set(v.clamp(0.0, 100.0)) } }/>
                                    <span class="muted">"%"</span></label>
                            }).collect_view()}
                        </span>
                    </div>
                </div>
                <div class="dialog-actions">
                    <button type="button" class="btn crop-reset" data-action="crop-reset" title=ui.t("crop-reset-help")
                        on:click=move |_| { x.set(50.0); y.set(50.0); z.set(1.0); }>{ui.t("crop-reset")}</button>
                    <span class="crop-spacer"></span>
                    <button type="button" class="btn" on:click=move |_| close()>{ui.t("action-cancel")}</button>
                    <button type="button" class="btn btn--primary" data-action="crop-apply" on:click=move |_| apply()>{ui.t("crop-apply")}</button>
                </div>
            </div>
        </div>
    }
}
