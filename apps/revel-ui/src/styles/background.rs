//! Image et dégradé du fond (calques de `background-image`), comme « Backgrounds › Image &
//! gradient » de Webflow : images du site, dégradés linéaires et radiaux réglés sans écrire de
//! CSS (angle au cadran, forme, position, arrêts de couleur glissés sur une barre).
//!
//! Un calque qu'on ne sait pas montrer sans le dénaturer (coin « to top right », tailles en
//! pixels…) reste tel quel : il se lit et se retire, et se règle dans « Déclarations ».

use super::color::{ColorInput, resolve_color};
use super::controls::{NumBox, Pop};
use super::ctx::{Sx, menu_id};
use crate::icons;
use leptos::ev;
use leptos::prelude::*;
use revel_style::values::{Rgba, fmt_num, split_top};
use wasm_bindgen::JsCast;

/// Arrêt de couleur : couleur écrite telle quelle, position en pourcentage.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Stop {
    pub color: String,
    pub pos: Option<f64>,
}

/// Calque du fond.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Layer {
    /// Adresse de l'image.
    Image(String),
    /// Angle en degrés (0 : vers le haut, sens des aiguilles d'une montre).
    Linear { angle: f64, stops: Vec<Stop> },
    /// Cercle ou ellipse, centre en pourcentages.
    Radial {
        circle: bool,
        x: f64,
        y: f64,
        stops: Vec<Stop>,
    },
    /// Calque gardé tel quel.
    Raw(String),
}

/// Pourcentage (`22%`, `0`).
fn percent(t: &str) -> Option<f64> {
    if t == "0" {
        return Some(0.0);
    }
    t.strip_suffix('%')?.trim().parse().ok()
}

/// Arrêts d'un dégradé (`None` : forme non reconnue).
fn stops(parts: &[String]) -> Option<Vec<Stop>> {
    let mut out = Vec::new();
    for p in parts {
        let toks = split_top(p, ' ');
        let (color, pos) = match toks.as_slice() {
            [c] => (c.clone(), None),
            [c, p] => (c.clone(), Some(percent(p)?)),
            _ => return None,
        };
        // Couleur : reconnue, ou variable du site.
        if Rgba::parse(&color).is_none() && !color.starts_with("var(") {
            return None;
        }
        out.push(Stop { color, pos });
    }
    (out.len() >= 2).then_some(out)
}

/// Mot-clé de position (`left`, `center`…) ou pourcentage.
fn position(t: &str) -> Option<f64> {
    match t {
        "left" | "top" => Some(0.0),
        "center" => Some(50.0),
        "right" | "bottom" => Some(100.0),
        t => percent(t),
    }
}

/// Lit un calque.
pub(crate) fn parse_layer(t: &str) -> Layer {
    let t = t.trim();
    let raw = || Layer::Raw(t.to_string());
    let call = |name: &str| {
        t.strip_prefix(name)
            .and_then(|r| r.strip_prefix('('))
            .and_then(|r| r.strip_suffix(')'))
    };
    if let Some(inner) = call("url") {
        let u = inner.trim().trim_matches(|c| c == '"' || c == '\'');
        return Layer::Image(u.replace("%22", "\""));
    }
    if let Some(inner) = call("linear-gradient") {
        let parts = split_top(inner, ',');
        let first = parts.first().map(String::as_str).unwrap_or_default();
        let (angle, rest) = if let Some(d) = first.strip_suffix("deg") {
            match d.trim().parse::<f64>() {
                Ok(a) => (a, &parts[1..]),
                Err(_) => return raw(),
            }
        } else if let Some(side) = first.strip_prefix("to ") {
            let a = match side.trim() {
                "top" => 0.0,
                "right" => 90.0,
                "bottom" => 180.0,
                "left" => 270.0,
                // Coins : l'angle dépend des proportions de la boîte.
                _ => return raw(),
            };
            (a, &parts[1..])
        } else {
            (180.0, &parts[..])
        };
        return match stops(rest) {
            Some(stops) => Layer::Linear { angle, stops },
            None => raw(),
        };
    }
    if let Some(inner) = call("radial-gradient") {
        let parts = split_top(inner, ',');
        let first = parts.first().map(String::as_str).unwrap_or_default();
        let toks = split_top(first, ' ');
        let shape_like = toks
            .first()
            .is_some_and(|w| matches!(w.as_str(), "circle" | "ellipse" | "at"));
        let (circle, x, y, rest) = if shape_like {
            let (shape, at) = match toks.iter().position(|w| w == "at") {
                Some(i) => (&toks[..i], &toks[i + 1..]),
                None => (&toks[..], &toks[..0]),
            };
            let circle = match shape {
                [] => false,
                [s] if s == "circle" => true,
                [s] if s == "ellipse" => false,
                _ => return raw(),
            };
            let (x, y) = match at {
                [] => (50.0, 50.0),
                [x, y] => match (position(x), position(y)) {
                    (Some(x), Some(y)) => (x, y),
                    _ => return raw(),
                },
                [c] if c == "center" => (50.0, 50.0),
                _ => return raw(),
            };
            (circle, x, y, &parts[1..])
        } else {
            (false, 50.0, 50.0, &parts[..])
        };
        return match stops(rest) {
            Some(stops) => Layer::Radial {
                circle,
                x,
                y,
                stops,
            },
            None => raw(),
        };
    }
    raw()
}

/// Calques d'une valeur de `background-image` (`none` : aucun).
pub(crate) fn parse_layers(v: &str) -> Vec<Layer> {
    let v = v.trim();
    if v.is_empty() || v == "none" {
        return Vec::new();
    }
    split_top(v, ',').iter().map(|t| parse_layer(t)).collect()
}

fn stops_css(stops: &[Stop], color: &dyn Fn(&str) -> String) -> String {
    stops
        .iter()
        .map(|s| match s.pos {
            Some(p) => format!("{} {}%", color(&s.color), fmt_num(p)),
            None => color(&s.color),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Écrit un calque ; `color` transforme les couleurs (variables résolues pour l'aperçu).
fn layer_css_with(l: &Layer, color: &dyn Fn(&str) -> String) -> String {
    match l {
        Layer::Image(u) => format!("url(\"{}\")", u.replace('"', "%22")),
        Layer::Linear { angle, stops } => {
            format!(
                "linear-gradient({}deg, {})",
                fmt_num(*angle),
                stops_css(stops, color)
            )
        }
        Layer::Radial {
            circle,
            x,
            y,
            stops,
        } => format!(
            "radial-gradient({} at {}% {}%, {})",
            if *circle { "circle" } else { "ellipse" },
            fmt_num(*x),
            fmt_num(*y),
            stops_css(stops, color)
        ),
        Layer::Raw(t) => t.clone(),
    }
}

pub(crate) fn layer_css(l: &Layer) -> String {
    layer_css_with(l, &|c| c.to_string())
}

/// Valeur de `background-image` (`None` : aucun calque).
pub(crate) fn layers_css(ls: &[Layer]) -> Option<String> {
    (!ls.is_empty()).then(|| ls.iter().map(layer_css).collect::<Vec<_>>().join(", "))
}

/// Position effective de chaque arrêt (répartis comme le fait le navigateur quand elle manque).
fn stop_positions(stops: &[Stop]) -> Vec<f64> {
    let n = stops.len();
    let mut out: Vec<Option<f64>> = stops.iter().map(|s| s.pos).collect();
    if let Some(f) = out.first_mut() {
        f.get_or_insert(0.0);
    }
    if let Some(l) = out.last_mut() {
        l.get_or_insert(100.0);
    }
    let mut i = 0;
    while i < n {
        if out[i].is_none() {
            let a = i - 1;
            let b = (i..n).find(|&j| out[j].is_some()).unwrap_or(n - 1);
            let (pa, pb) = (out[a].unwrap_or(0.0), out[b].unwrap_or(100.0));
            for (k, slot) in out.iter_mut().enumerate().take(b).skip(i) {
                *slot = Some(pa + (pb - pa) * (k - a) as f64 / (b - a) as f64);
            }
            i = b;
        }
        i += 1;
    }
    out.into_iter().map(|p| p.unwrap_or(0.0)).collect()
}

fn stops_of(l: &Layer) -> Option<&Vec<Stop>> {
    match l {
        Layer::Linear { stops, .. } | Layer::Radial { stops, .. } => Some(stops),
        _ => None,
    }
}

fn stops_mut(l: &mut Layer) -> Option<&mut Vec<Stop>> {
    match l {
        Layer::Linear { stops, .. } | Layer::Radial { stops, .. } => Some(stops),
        _ => None,
    }
}

/// Nom du fichier d'une adresse.
fn file_name(u: &str) -> String {
    u.rsplit('/').next().unwrap_or(u).to_string()
}

/// Calques du fond réglés dans le panneau de styles.
#[component]
pub fn BgLayers(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get("background-image"));
    let layers = Memo::new(move |_| parse_layers(&pv.get().current()));
    let is_here = move || pv.with(|p| p.here.is_some());
    // Calque ouvert, arrêt choisi dans son dégradé.
    let open = RwSignal::new(None::<usize>);
    let stop = RwSignal::new(0usize);
    let add_id = menu_id();
    let resolve = move |c: &str| -> String {
        resolve_color(sx, c)
            .map(|r| r.to_css())
            .unwrap_or_else(|| c.to_string())
    };
    let commit = move |ls: Vec<Layer>| sx.set("background-image", layers_css(&ls));
    let preview = move |ls: Vec<Layer>| {
        sx.preview(&[(
            "background-image".into(),
            ls.iter().map(layer_css).collect::<Vec<_>>().join(", "),
        )])
    };
    // Modification du calque `i` (aperçu ou enregistrement).
    let edit = move |i: usize, f: &dyn Fn(&mut Layer), save: bool| {
        let mut ls = layers.get_untracked();
        if let Some(l) = ls.get_mut(i) {
            f(l);
        }
        if save { commit(ls) } else { preview(ls) }
    };
    let add = move |l: Layer| {
        sx.menu.set(None);
        let mut ls = layers.get_untracked();
        ls.insert(0, l);
        open.set(Some(0));
        stop.set(0);
        commit(ls);
    };
    let add_image = move || {
        sx.menu.set(None);
        let ed = sx.ed;
        crate::library::pick_image(ui, crate::library::media_shelf(ed), move |url| {
            add(Layer::Image(url))
        });
    };
    let remove = move |i: usize| {
        let mut ls = layers.get_untracked();
        if i < ls.len() {
            ls.remove(i);
        }
        open.set(None);
        commit(ls);
    };
    let base = crate::library::canvas_base(ui, &ui.site_path().unwrap_or_default());
    let thumb = move |l: &Layer| -> String {
        match l {
            Layer::Image(u) => {
                let src = base
                    .get()
                    .map(|b| crate::library::thumb_src(&b, u))
                    .unwrap_or_else(|| u.clone());
                format!("background-image: url(\"{}\")", src.replace('"', "%22"))
            }
            l => format!("background-image: {}", layer_css_with(l, &resolve)),
        }
    };
    let label = move |l: &Layer| -> String {
        match l {
            Layer::Image(u) => file_name(u),
            Layer::Linear { angle, .. } => {
                format!("{} · {}°", ui.now("bg-linear", &[]), fmt_num(*angle))
            }
            Layer::Radial { .. } => ui.now("bg-radial", &[]),
            Layer::Raw(t) => t.clone(),
        }
    };
    view! {
        <div class="bg-layers" class:is-here=is_here>
            <div class="bg-layers-head">
                <button class="icon-btn icon-btn--small pop-trigger" title=ui.t("bg-add") aria-label=ui.t("bg-add")
                    on:click=move |_| sx.toggle_menu(add_id)>{icons::plus()}</button>
                <Pop sx id=add_id narrow=true>
                    <ul class="menu-list" role="menu">
                        <li><button role="menuitem" class="menu-item" on:click=move |_| add_image()>{icons::image()}{ui.t("bg-image")}</button></li>
                        <li><button role="menuitem" class="menu-item" on:click=move |_| add(Layer::Linear {
                            angle: 180.0,
                            stops: vec![Stop { color: "#000000".into(), pos: Some(0.0) }, Stop { color: "#ffffff".into(), pos: Some(100.0) }],
                        })>{icons::gradient_linear()}{ui.t("bg-linear")}</button></li>
                        <li><button role="menuitem" class="menu-item" on:click=move |_| add(Layer::Radial {
                            circle: true, x: 50.0, y: 50.0,
                            stops: vec![Stop { color: "#ffffff".into(), pos: Some(0.0) }, Stop { color: "#000000".into(), pos: Some(100.0) }],
                        })>{icons::gradient_radial()}{ui.t("bg-radial")}</button></li>
                    </ul>
                </Pop>
            </div>
            {move || {
                let ls = layers.get();
                (ls.is_empty()).then(|| view! { <p class="bg-empty muted">{ui.t("bg-none")}</p> })
            }}
            <ul class="bg-list">
                {move || layers.get().into_iter().enumerate().map(|(i, l)| {
                    let raw = matches!(l, Layer::Raw(_));
                    let style = thumb(&l);
                    let text = label(&l);
                    let title = layer_css(&l);
                    view! {
                        <li class="bg-layer" class:is-open=move || open.get() == Some(i)>
                            <div class="bg-layer-row">
                                <button class="bg-layer-main" title=title disabled=raw
                                    on:click=move |_| { stop.set(0); open.update(|o| *o = if *o == Some(i) { None } else { Some(i) }) }>
                                    <span class="bg-thumb" style=style></span>
                                    <span class="bg-layer-name">{text}</span>
                                </button>
                                <button class="icon-btn icon-btn--small" title=ui.t("bg-remove") aria-label=ui.t("bg-remove")
                                    on:click=move |_| remove(i)>{icons::trash()}</button>
                            </div>
                            <Show when=move || open.get() == Some(i)>
                                <LayerEditor sx i layers stop edit/>
                            </Show>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </div>
    }
}

/// Réglages du calque ouvert.
#[component]
fn LayerEditor(
    sx: Sx,
    i: usize,
    layers: Memo<Vec<Layer>>,
    stop: RwSignal<usize>,
    edit: impl Fn(usize, &dyn Fn(&mut Layer), bool) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let ui = sx.ui;
    let layer = Memo::new(move |_| layers.with(|ls| ls.get(i).cloned()));
    move || match layer.get() {
        Some(Layer::Image(u)) => {
            let ed = sx.ed;
            view! {
                <div class="bg-editor">
                    <div class="bg-image-pick">
                        <span class="bg-image-name path" title=u.clone()>{file_name(&u)}</span>
                        <button class="btn btn--small" on:click=move |_| {
                            crate::library::pick_image(ui, crate::library::media_shelf(ed), move |url| {
                                edit(i, &|l| *l = Layer::Image(url.clone()), true)
                            })
                        }>{ui.t("bg-image-change")}</button>
                    </div>
                </div>
            }
            .into_any()
        }
        Some(l @ (Layer::Linear { .. } | Layer::Radial { .. })) => {
            let linear = matches!(l, Layer::Linear { .. });
            view! {
                <div class="bg-editor">
                    {if linear {
                        view! { <AngleRow sx i layer edit/> }.into_any()
                    } else {
                        view! { <RadialRows sx i layer edit/> }.into_any()
                    }}
                    <StopsBar sx i layer stop edit/>
                </div>
            }
            .into_any()
        }
        _ => ().into_any(),
    }
}

/// Angle d'un dégradé linéaire : cadran glissé et valeur en degrés.
#[component]
fn AngleRow(
    sx: Sx,
    i: usize,
    layer: Memo<Option<Layer>>,
    edit: impl Fn(usize, &dyn Fn(&mut Layer), bool) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let ui = sx.ui;
    let angle = move || match layer.get() {
        Some(Layer::Linear { angle, .. }) => angle,
        _ => 180.0,
    };
    let set = move |a: f64, save: bool| {
        let a = a.rem_euclid(360.0);
        edit(
            i,
            &move |l| {
                if let Layer::Linear { angle, .. } = l {
                    *angle = a;
                }
            },
            save,
        )
    };
    let dragging = StoredValue::new(None::<f64>);
    let at = move |e: &ev::PointerEvent, snap: bool| -> Option<f64> {
        let el = e.current_target()?.dyn_into::<web_sys::Element>().ok()?;
        let r = el.get_bounding_client_rect();
        let dx = f64::from(e.client_x()) - (r.left() + r.width() / 2.0);
        let dy = f64::from(e.client_y()) - (r.top() + r.height() / 2.0);
        let a = dx.atan2(-dy).to_degrees().rem_euclid(360.0);
        Some(if snap {
            (a / 15.0).round() * 15.0
        } else {
            a.round()
        })
    };
    view! {
        <div class="bg-row">
            <span class="bg-row-label">{ui.t("bg-angle")}</span>
            <div class="bg-row-control">
                <div class="dial" role="slider" tabindex="0" aria-label=ui.t("bg-angle")
                    aria-valuemin="0" aria-valuemax="360" aria-valuenow=move || fmt_num(angle())
                    title=ui.t("bg-angle-help")
                    on:pointerdown=move |e: ev::PointerEvent| {
                        if let Some(t) = e.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                            let _ = t.set_pointer_capture(e.pointer_id());
                        }
                        if let Some(a) = at(&e, e.shift_key()) {
                            dragging.set_value(Some(a));
                            set(a, false);
                        }
                    }
                    on:pointermove=move |e: ev::PointerEvent| {
                        if dragging.get_value().is_some() && let Some(a) = at(&e, e.shift_key()) {
                            dragging.set_value(Some(a));
                            set(a, false);
                        }
                    }
                    on:pointerup=move |_| {
                        if let Some(a) = dragging.get_value() {
                            dragging.set_value(None);
                            set(a, true);
                        }
                    }
                    on:keydown=move |e: ev::KeyboardEvent| {
                        let d = match e.key().as_str() { "ArrowRight" | "ArrowUp" => 1.0, "ArrowLeft" | "ArrowDown" => -1.0, _ => return };
                        e.prevent_default();
                        set(angle() + d * if e.shift_key() { 15.0 } else { 1.0 }, true);
                    }>
                    <i class="dial-hand" style:transform=move || format!("rotate({}deg)", fmt_num(angle()))></i>
                </div>
                <NumBox sx value=Signal::derive(move || Some(fmt_num(angle()))) hint=Signal::derive(move || fmt_num(angle()))
                    start=Signal::derive(move || fmt_num(angle()))
                    on_preview=Callback::new(move |v: String| { if let Ok(a) = v.parse::<f64>() { set(a, false) } })
                    on_commit=Callback::new(move |(v, _): (Option<String>, bool)| { if let Some(a) = v.and_then(|v| v.parse::<f64>().ok()) { set(a, true) } })
                    unit="" suffix=Some("°") label="bg-angle"/>
            </div>
        </div>
    }
}

/// Forme et centre d'un dégradé radial.
#[component]
fn RadialRows(
    sx: Sx,
    i: usize,
    layer: Memo<Option<Layer>>,
    edit: impl Fn(usize, &dyn Fn(&mut Layer), bool) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let ui = sx.ui;
    let get = move || match layer.get() {
        Some(Layer::Radial { circle, x, y, .. }) => (circle, x, y),
        _ => (true, 50.0, 50.0),
    };
    let set_shape = move |c: bool| {
        edit(
            i,
            &move |l| {
                if let Layer::Radial { circle, .. } = l {
                    *circle = c;
                }
            },
            true,
        )
    };
    let set_at = move |nx: Option<f64>, ny: Option<f64>| {
        edit(
            i,
            &move |l| {
                if let Layer::Radial { x, y, .. } = l {
                    if let Some(v) = nx {
                        *x = v.clamp(0.0, 100.0);
                    }
                    if let Some(v) = ny {
                        *y = v.clamp(0.0, 100.0);
                    }
                }
            },
            true,
        )
    };
    let num = move |v: Option<String>| v.and_then(|v| v.trim_end_matches('%').parse::<f64>().ok());
    view! {
        <div class="bg-row">
            <span class="bg-row-label">{ui.t("bg-shape")}</span>
            <div class="bg-row-control">
                <div class="seg seg--small" role="radiogroup">
                    <button role="radio" class:is-on=move || get().0 aria-checked=move || get().0.to_string()
                        on:click=move |_| set_shape(true)><span>{ui.t("bg-circle")}</span></button>
                    <button role="radio" class:is-on=move || !get().0 aria-checked=move || (!get().0).to_string()
                        on:click=move |_| set_shape(false)><span>{ui.t("bg-ellipse")}</span></button>
                </div>
            </div>
        </div>
        <div class="bg-row">
            <span class="bg-row-label">{ui.t("bg-center")}</span>
            <div class="bg-row-control bg-center">
                <div class="align-box align-box--small" role="group">
                    {(0..3).flat_map(|r| (0..3).map(move |c| (c, r))).map(|(c, r)| {
                        let (cx, cy) = (c as f64 * 50.0, r as f64 * 50.0);
                        view! {
                            <button class="align-cell" class:is-on=move || { let (_, x, y) = get(); x == cx && y == cy }
                                on:click=move |_| set_at(Some(cx), Some(cy))><i></i></button>
                        }
                    }).collect_view()}
                </div>
                <div class="bg-xy">
                    <NumBox sx value=Signal::derive(move || Some(fmt_num(get().1))) hint=Signal::derive(move || fmt_num(get().1))
                        start=Signal::derive(move || fmt_num(get().1)) prefix=Some("X") suffix=Some("%") unit=""
                        on_preview=Callback::new(move |_: String| {})
                        on_commit=Callback::new(move |(v, _): (Option<String>, bool)| set_at(num(v), None)) label="bg-center-x"/>
                    <NumBox sx value=Signal::derive(move || Some(fmt_num(get().2))) hint=Signal::derive(move || fmt_num(get().2))
                        start=Signal::derive(move || fmt_num(get().2)) prefix=Some("Y") suffix=Some("%") unit=""
                        on_preview=Callback::new(move |_: String| {})
                        on_commit=Callback::new(move |(v, _): (Option<String>, bool)| set_at(None, num(v))) label="bg-center-y"/>
                </div>
            </div>
        </div>
    }
}

/// Barre des arrêts de couleur : glisser un arrêt le déplace, un clic sur la barre en ajoute un ;
/// l'arrêt choisi se règle dessous (couleur, position, retrait).
#[component]
fn StopsBar(
    sx: Sx,
    i: usize,
    layer: Memo<Option<Layer>>,
    stop: RwSignal<usize>,
    edit: impl Fn(usize, &dyn Fn(&mut Layer), bool) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let ui = sx.ui;
    let list = Memo::new(move |_| {
        layer
            .get()
            .and_then(|l| stops_of(&l).cloned())
            .unwrap_or_default()
    });
    let positions = Memo::new(move |_| stop_positions(&list.get()));
    let resolve = move |c: &str| -> String {
        resolve_color(sx, c)
            .map(|r| r.to_css())
            .unwrap_or_else(|| c.to_string())
    };
    let bar_css = move || {
        let s = list.get();
        let p = positions.get();
        let parts: Vec<String> = s
            .iter()
            .zip(p.iter())
            .map(|(s, p)| format!("{} {}%", resolve(&s.color), fmt_num(*p)))
            .collect();
        format!(
            "background-image: linear-gradient(90deg, {})",
            parts.join(", ")
        )
    };
    // Position (0-100) du pointeur sur la barre.
    let bar = NodeRef::<leptos::html::Div>::new();
    let pos_at = move |x: i32| -> Option<f64> {
        let r = bar.get_untracked()?.get_bounding_client_rect();
        (r.width() > 0.0).then(|| {
            (((f64::from(x) - r.left()) / r.width()) * 100.0)
                .clamp(0.0, 100.0)
                .round()
        })
    };
    let move_stop = move |k: usize, p: f64, save: bool| {
        edit(
            i,
            &move |l| {
                if let Some(s) = stops_mut(l)
                    && let Some(st) = s.get_mut(k)
                {
                    st.pos = Some(p);
                }
            },
            save,
        )
    };
    // Ajout : couleur de l'arrêt le plus proche, à l'endroit cliqué ; liste gardée en ordre.
    let add_at = move |p: f64| {
        let pos = positions.get_untracked();
        let s = list.get_untracked();
        let k = pos.iter().position(|q| *q > p).unwrap_or(s.len());
        let near = pos
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - p).abs().total_cmp(&(b.1 - p).abs()))
            .map(|(j, _)| j)
            .unwrap_or(0);
        let color = s.get(near).map(|x| x.color.clone()).unwrap_or_default();
        edit(
            i,
            &move |l| {
                if let Some(st) = stops_mut(l) {
                    st.insert(
                        k.min(st.len()),
                        Stop {
                            color: color.clone(),
                            pos: Some(p),
                        },
                    );
                }
            },
            true,
        );
        stop.set(k);
    };
    let drag = StoredValue::new(None::<(usize, f64, bool)>);
    let cur = Memo::new(move |_| {
        let s = list.get();
        let k = stop.get().min(s.len().saturating_sub(1));
        s.get(k).cloned().map(|st| (k, st))
    });
    view! {
        <div class="bg-row bg-row--stack">
            <span class="bg-row-label">{ui.t("bg-stops")}</span>
            <div class="stops">
                <div class="stops-bar" node_ref=bar title=ui.t("bg-stops-help")
                    on:pointerdown=move |e: ev::PointerEvent| {
                        if e.target() == e.current_target() && let Some(p) = pos_at(e.client_x()) {
                            add_at(p);
                        }
                    }>
                    <div class="stops-fill" style=bar_css></div>
                    {move || positions.get().into_iter().enumerate().map(|(k, p)| {
                        let color = list.with(|s| s.get(k).map(|x| resolve(&x.color)).unwrap_or_default());
                        view! {
                            <button class="stop" class:is-on=move || stop.get() == k
                                style=format!("left: {}%; --stop: {color}", fmt_num(p))
                                aria-label=ui.t("bg-stop")
                                on:pointerdown=move |e: ev::PointerEvent| {
                                    e.stop_propagation();
                                    stop.set(k);
                                    if let Some(t) = e.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                                        let _ = t.set_pointer_capture(e.pointer_id());
                                    }
                                    drag.set_value(Some((k, p, false)));
                                }
                                on:pointermove=move |e: ev::PointerEvent| {
                                    if let Some((k, _, _)) = drag.get_value() && let Some(q) = pos_at(e.client_x()) {
                                        drag.set_value(Some((k, q, true)));
                                        move_stop(k, q, false);
                                    }
                                }
                                on:pointerup=move |_| {
                                    if let Some((k, q, moved)) = drag.get_value() {
                                        drag.set_value(None);
                                        if moved { move_stop(k, q, true) }
                                    }
                                }></button>
                        }
                    }).collect_view()}
                </div>
                {move || cur.get().map(|(k, st)| {
                    let c1 = st.color.clone();
                    let c2 = st.color.clone();
                    let c3 = resolve(&st.color);
                    let p = positions.with_untracked(|p| p.get(k).copied().unwrap_or(0.0));
                    let set_color = move |v: Option<String>, save: bool| {
                        let Some(v) = v else { return };
                        edit(i, &move |l| {
                            if let Some(s) = stops_mut(l) && let Some(st) = s.get_mut(k) {
                                st.color = v.clone();
                            }
                        }, save)
                    };
                    let can_remove = list.with_untracked(|s| s.len() > 2);
                    view! {
                        <div class="stop-edit">
                            <ColorInput sx value=Signal::derive(move || Some(c1.clone())) hint=Signal::derive(move || c2.clone())
                                resolved=Signal::derive(move || c3.clone())
                                on_preview=Callback::new(move |v: String| set_color(Some(v), false))
                                on_commit=Callback::new(move |v: Option<String>| set_color(v, true))
                                label="bg-stop-color"/>
                            <NumBox sx value=Signal::derive(move || Some(fmt_num(p))) hint=Signal::derive(move || fmt_num(p))
                                start=Signal::derive(move || fmt_num(p)) suffix=Some("%") unit=""
                                on_preview=Callback::new(move |_: String| {})
                                on_commit=Callback::new(move |(v, _): (Option<String>, bool)| {
                                    if let Some(q) = v.and_then(|v| v.trim_end_matches('%').parse::<f64>().ok()) {
                                        move_stop(k, q.clamp(0.0, 100.0), true)
                                    }
                                }) label="bg-stop-position"/>
                            <button class="icon-btn icon-btn--small" disabled=!can_remove title=ui.t("bg-stop-remove") aria-label=ui.t("bg-stop-remove")
                                on:click=move |_| {
                                    edit(i, &move |l| { if let Some(s) = stops_mut(l) && s.len() > 2 { s.remove(k); } }, true);
                                    stop.set(k.saturating_sub(1));
                                }>{icons::trash()}</button>
                        </div>
                    }
                })}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calques_lus_et_reecrits() {
        let v = "linear-gradient(128deg, #4f1cb8 0%, #7c34c8 22%, #fe9d44 100%), url(\"/media/a.jpg\")";
        let ls = parse_layers(v);
        assert_eq!(ls.len(), 2);
        assert!(matches!(&ls[0], Layer::Linear { angle, stops } if *angle == 128.0 && stops.len() == 3));
        assert_eq!(ls[1], Layer::Image("/media/a.jpg".into()));
        assert_eq!(layers_css(&ls).unwrap(), v);
        let r = "radial-gradient(circle at 30% 30%, rgba(124, 52, 200, 0.35), transparent 60%)";
        let l = parse_layer(r);
        assert!(matches!(&l, Layer::Radial { circle: true, x, y, stops } if *x == 30.0 && *y == 30.0 && stops[0].pos.is_none()));
        assert_eq!(layer_css(&l), r);
        // Sens écrit en mots : côté reconnu, coin gardé tel quel.
        assert!(matches!(parse_layer("linear-gradient(to right, red, blue)"), Layer::Linear { angle, .. } if angle == 90.0));
        assert!(matches!(parse_layer("linear-gradient(to top right, red, blue)"), Layer::Raw(_)));
        assert!(matches!(parse_layer("linear-gradient(90deg, red 10px, blue)"), Layer::Raw(_)));
        assert!(matches!(parse_layer("linear-gradient(var(--aube))"), Layer::Raw(_)));
        assert!(matches!(parse_layer("radial-gradient(var(--a), var(--b))"), Layer::Radial { circle: false, .. }));
        assert!(parse_layers("none").is_empty());
        assert_eq!(layers_css(&[]), None);
    }

    #[test]
    fn positions_des_arrets() {
        let s = |p: Option<f64>| Stop {
            color: "red".into(),
            pos: p,
        };
        assert_eq!(stop_positions(&[s(None), s(None)]), vec![0.0, 100.0]);
        assert_eq!(
            stop_positions(&[s(None), s(None), s(Some(60.0))]),
            vec![0.0, 30.0, 60.0]
        );
        assert_eq!(
            stop_positions(&[s(Some(10.0)), s(None), s(None), s(None)]),
            vec![10.0, 40.0, 70.0, 100.0]
        );
    }
}
