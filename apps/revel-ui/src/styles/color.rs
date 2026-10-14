//! Couleurs (D-20) : pastille et valeur dans le panneau ; sélecteur à plan saturation-luminosité,
//! curseurs de teinte et d'opacité, pipette, modèles Hex / RVB / TSL / TSV, couleurs du site,
//! contraste du texte.

use super::controls::Pop;
use super::ctx::{Sx, menu_id};
use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_model::StyleItem;
use revel_style::values::{Hsva, Rgba, contrast, fmt_num, is_var, split_top};
use std::cell::Cell;
use wasm_bindgen::{JsCast, JsValue};

#[derive(Clone, Copy, PartialEq)]
enum Model {
    Hex,
    Rgb,
    Hsl,
    Hsb,
}

thread_local! {
    /// Modèle choisi en dernier (gardé pendant la session).
    static MODEL: Cell<Model> = const { Cell::new(Model::Hex) };
}

/// Texte d'une valeur : une variable s'affiche par son nom.
fn display(v: &str) -> String {
    if is_var(v) {
        v[4..v.len() - 1].trim().to_string()
    } else {
        v.to_string()
    }
}

/// Valeur saisie : `--nom` -> `var(--nom)`, `3d7eff` -> `#3d7eff`.
fn from_input(t: &str) -> Option<String> {
    let t = t.trim();
    if t.is_empty() {
        None
    } else if t.starts_with("--") {
        Some(format!("var({t})"))
    } else if t.chars().all(|c| c.is_ascii_hexdigit()) && matches!(t.len(), 3 | 4 | 6 | 8) {
        Some(format!("#{t}"))
    } else {
        Some(t.to_string())
    }
}

/// Fond d'une pastille : la couleur sur un damier (transparence visible).
fn swatch_style(c: Option<Rgba>) -> String {
    match c {
        Some(c) => format!("--swatch: {}", c.to_css()),
        None => "--swatch: transparent".into(),
    }
}

/// Couleurs du site : variables (`:root`) puis couleurs les plus utilisées.
fn site_colors(sx: Sx) -> (Vec<(String, Rgba)>, Vec<Rgba>) {
    sx.ed.doc.with_untracked(|d| {
        let Some(sheet) = d.as_ref().and_then(|d| d.stylesheet.as_ref()) else {
            return (Vec::new(), Vec::new());
        };
        let mut vars: Vec<(String, Rgba)> = Vec::new();
        let mut used: Vec<(String, Rgba, usize)> = Vec::new();
        for item in &sheet.items {
            let StyleItem::Rule(r) = item else { continue };
            for d in &r.decls {
                if d.prop.starts_with("--") {
                    if (r.selector.contains(":root") || r.selector == "html")
                        && let Some(c) = Rgba::parse(&d.value)
                        && !vars.iter().any(|(n, _)| *n == d.prop)
                    {
                        vars.push((d.prop.clone(), c));
                    }
                    continue;
                }
                let colorish = d.prop.contains("color")
                    || matches!(
                        d.prop.as_str(),
                        "background" | "border" | "fill" | "stroke" | "outline" | "box-shadow"
                    )
                    || d.prop.starts_with("border-");
                if !colorish {
                    continue;
                }
                for layer in split_top(&d.value, ',') {
                    for t in split_top(&layer, ' ') {
                        if let Some(c) = Rgba::parse(&t).filter(|c| c.a >= 0.3) {
                            let key = c.to_css();
                            match used.iter_mut().find(|(k, _, _)| *k == key) {
                                Some(e) => e.2 += 1,
                                None => used.push((key, c, 1)),
                            }
                        }
                    }
                }
            }
        }
        used.sort_by_key(|e| std::cmp::Reverse(e.2));
        (vars, used.into_iter().take(18).map(|(_, c, _)| c).collect())
    })
}

/// Zone glissée du sélecteur.
#[derive(Clone, Copy, PartialEq)]
enum Area {
    Plane,
    Hue,
    Alpha,
}

/// Position relative (0-1) du pointeur dans l'élément.
fn rel(e: &ev::PointerEvent) -> Option<(f64, f64)> {
    let el = e.current_target()?.dyn_into::<web_sys::Element>().ok()?;
    let r = el.get_bounding_client_rect();
    if r.width() <= 0.0 || r.height() <= 0.0 {
        return None;
    }
    Some((
        ((f64::from(e.client_x()) - r.left()) / r.width()).clamp(0.0, 1.0),
        ((f64::from(e.client_y()) - r.top()) / r.height()).clamp(0.0, 1.0),
    ))
}

/// Champ d'une composante (validé à la sortie ou par Entrée).
#[component]
fn Channel(
    #[prop(into)] value: Signal<String>,
    on: Callback<String>,
    label: &'static str,
    #[prop(optional)] wide: bool,
) -> impl IntoView {
    view! {
        <label class="cp-channel" class:cp-channel--wide=wide>
            <input class="nbox-input" type="text" spellcheck="false" prop:value=value
                on:change=move |e| on.run(event_target_value(&e))
                on:keydown=move |e: ev::KeyboardEvent| { if e.key() == "Enter" { on.run(event_target_value(&e)) } }/>
            <span class="cp-channel-label">{label}</span>
        </label>
    }
}

/// Sélecteur de couleur. `start` : couleur au moment de l'ouverture.
#[component]
fn Picker(
    sx: Sx,
    start: Rgba,
    on_preview: Callback<String>,
    on_commit: Callback<Option<String>>,
    with_contrast: bool,
) -> impl IntoView {
    let ui = sx.ui;
    let hsva = RwSignal::new(start.to_hsva());
    let rgba = move || Rgba::from_hsva(hsva.get());
    let model = RwSignal::new(MODEL.with(Cell::get));
    let area = StoredValue::new(None::<Area>);
    let (vars, used) = site_colors(sx);

    let update = move |a: Area, (x, y): (f64, f64)| {
        hsva.update(|c| match a {
            Area::Plane => {
                c.s = x;
                c.v = 1.0 - y;
            }
            Area::Hue => c.h = x * 360.0,
            Area::Alpha => c.a = (x * 100.0).round() / 100.0,
        });
        on_preview.run(rgba().to_css());
    };
    let down = move |a: Area, e: ev::PointerEvent| {
        if e.button() != 0 {
            return;
        }
        if let Some(t) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = t.set_pointer_capture(e.pointer_id());
        }
        area.set_value(Some(a));
        if let Some(p) = rel(&e) {
            update(a, p);
        }
    };
    let moved = move |a: Area, e: ev::PointerEvent| {
        if area.get_value() == Some(a)
            && let Some(p) = rel(&e)
        {
            update(a, p);
        }
    };
    let up = move |_| {
        if area.get_value().is_some() {
            area.set_value(None);
            on_commit.run(Some(rgba().to_css()));
        }
    };
    // Clavier sur les curseurs : flèches (Maj : pas de 10).
    let key = move |a: Area, e: ev::KeyboardEvent| {
        let big = e.shift_key();
        let (dx, dy) = match e.key().as_str() {
            "ArrowRight" => (1.0, 0.0),
            "ArrowLeft" => (-1.0, 0.0),
            "ArrowUp" => (0.0, -1.0),
            "ArrowDown" => (0.0, 1.0),
            _ => return,
        };
        e.prevent_default();
        let m = if big { 10.0 } else { 1.0 };
        hsva.update(|c| match a {
            Area::Plane => {
                c.s = (c.s + dx * m / 100.0).clamp(0.0, 1.0);
                c.v = (c.v - dy * m / 100.0).clamp(0.0, 1.0);
            }
            Area::Hue => c.h = (c.h + (dx - dy) * m).clamp(0.0, 360.0),
            Area::Alpha => c.a = (c.a + (dx - dy) * m / 100.0).clamp(0.0, 1.0),
        });
        on_commit.run(Some(rgba().to_css()));
    };
    let set_rgba = move |c: Rgba| {
        let h = hsva.get_untracked().h;
        let mut n = c.to_hsva();
        if n.s == 0.0 || n.v == 0.0 {
            n.h = h;
        }
        hsva.set(n);
        on_commit.run(Some(c.to_css()));
    };
    let channel = move |f: fn(&mut Hsva, &mut Rgba, f64), v: String| {
        let Ok(x) = v
            .trim()
            .trim_end_matches(['%', '°'])
            .replace(',', ".")
            .parse::<f64>()
        else {
            return;
        };
        let old = hsva.get_untracked();
        let mut h = old;
        let mut c = Rgba::from_hsva(h);
        f(&mut h, &mut c, x);
        if h != old {
            hsva.set(h);
            on_commit.run(Some(Rgba::from_hsva(h).to_css()));
        } else {
            set_rgba(c);
        }
    };
    let eyedropper =
        js_sys::Reflect::has(&window(), &JsValue::from_str("EyeDropper")).unwrap_or(false);
    let pick = move |_| {
        // Appel synchrone (geste de l'utilisateur), résultat attendu ensuite.
        let promise = (|| -> Option<js_sys::Promise> {
            let ctor = js_sys::Reflect::get(&window(), &"EyeDropper".into())
                .ok()?
                .dyn_into::<js_sys::Function>()
                .ok()?;
            let inst = js_sys::Reflect::construct(&ctor, &js_sys::Array::new()).ok()?;
            let open = js_sys::Reflect::get(&inst, &"open".into())
                .ok()?
                .dyn_into::<js_sys::Function>()
                .ok()?;
            open.call0(&inst).ok()?.dyn_into::<js_sys::Promise>().ok()
        })();
        if let Some(p) = promise {
            spawn_local(async move {
                if let Ok(r) = wasm_bindgen_futures::JsFuture::from(p).await
                    && let Some(hex) = js_sys::Reflect::get(&r, &"sRGBHex".into())
                        .ok()
                        .and_then(|v| v.as_string())
                    && let Some(mut c) = Rgba::parse(&hex)
                {
                    c.a = hsva.get_untracked().a;
                    set_rgba(c);
                }
            });
        }
    };
    let contrast_line = move || {
        let bg = Rgba::parse(&sx.computed("--rv-background"))?;
        let r = contrast(rgba(), bg);
        let level = if r >= 7.0 {
            "styles-contrast-aaa"
        } else if r >= 4.5 {
            "styles-contrast-aa"
        } else if r >= 3.0 {
            "styles-contrast-large"
        } else {
            "styles-contrast-low"
        };
        Some((
            ui.tx(
                "styles-contrast",
                &[("ratio", crate::num(r, 1)), ("level", ui.tx(level, &[]))],
            ),
            r >= 4.5,
        ))
    };

    view! {
        <div class="cp">
            <div class="cp-plane" role="slider" tabindex="0" aria-label=ui.t("color-plane")
                style:background-color=move || Rgba::from_hsva(Hsva { h: hsva.get().h, s: 1.0, v: 1.0, a: 1.0 }).hex6()
                on:pointerdown=move |e| down(Area::Plane, e) on:pointermove=move |e| moved(Area::Plane, e)
                on:pointerup=up on:keydown=move |e| key(Area::Plane, e)>
                <i class="cp-thumb" style:left=move || format!("{}%", hsva.get().s * 100.0)
                    style:top=move || format!("{}%", (1.0 - hsva.get().v) * 100.0)
                    style:background-color=move || rgba().hex6()></i>
            </div>
            <div class="cp-row">
                {eyedropper.then(|| view! {
                    <button class="icon-btn cp-eye" title=ui.t("color-eyedropper") aria-label=ui.t("color-eyedropper") on:click=pick>
                        {crate::icons::eyedropper()}
                    </button>
                })}
                <div class="cp-sliders">
                    <div class="cp-hue" role="slider" tabindex="0" aria-label=ui.t("color-hue")
                        aria-valuemin="0" aria-valuemax="360" aria-valuenow=move || fmt_num(hsva.get().h)
                        on:pointerdown=move |e| down(Area::Hue, e) on:pointermove=move |e| moved(Area::Hue, e)
                        on:pointerup=up on:keydown=move |e| key(Area::Hue, e)>
                        <i class="cp-knob" style:left=move || format!("{}%", hsva.get().h / 3.6)
                            style:background-color=move || Rgba::from_hsva(Hsva { s: 1.0, v: 1.0, a: 1.0, ..hsva.get() }).hex6()></i>
                    </div>
                    <div class="cp-alpha" role="slider" tabindex="0" aria-label=ui.t("color-opacity")
                        aria-valuemin="0" aria-valuemax="100" aria-valuenow=move || fmt_num(hsva.get().a * 100.0)
                        style=move || format!("--cp-color: {}", rgba().hex6())
                        on:pointerdown=move |e| down(Area::Alpha, e) on:pointermove=move |e| moved(Area::Alpha, e)
                        on:pointerup=up on:keydown=move |e| key(Area::Alpha, e)>
                        <i class="cp-knob" style:left=move || format!("{}%", hsva.get().a * 100.0)></i>
                    </div>
                </div>
            </div>
            <div class="cp-fields">
                <div class="select cp-model">
                    <select class="input" aria-label=ui.t("color-model") on:change=move |e| {
                        let m = match event_target_value(&e).as_str() { "rgb" => Model::Rgb, "hsl" => Model::Hsl, "hsb" => Model::Hsb, _ => Model::Hex };
                        MODEL.with(|c| c.set(m));
                        model.set(m);
                    }>
                        <option value="hex" selected=move || model.get() == Model::Hex>"Hex"</option>
                        <option value="rgb" selected=move || model.get() == Model::Rgb>{ui.t("color-rgb")}</option>
                        <option value="hsl" selected=move || model.get() == Model::Hsl>{ui.t("color-hsl")}</option>
                        <option value="hsb" selected=move || model.get() == Model::Hsb>{ui.t("color-hsb")}</option>
                    </select>
                </div>
                {move || match model.get() {
                    Model::Hex => view! {
                        <Channel wide=true label="#" value=Signal::derive(move || rgba().hex6()[1..].to_string())
                            on=Callback::new(move |v: String| {
                                if let Some(mut c) = Rgba::parse(&format!("#{}", v.trim().trim_start_matches('#'))) {
                                    if c.a >= 1.0 { c.a = hsva.get_untracked().a; }
                                    set_rgba(c);
                                }
                            })/>
                    }.into_any(),
                    Model::Rgb => view! {
                        <Channel label="R" value=Signal::derive(move || fmt_num(rgba().r.round()))
                            on=Callback::new(move |v| channel(|_, c, x| c.r = x.clamp(0.0, 255.0), v))/>
                        <Channel label="G" value=Signal::derive(move || fmt_num(rgba().g.round()))
                            on=Callback::new(move |v| channel(|_, c, x| c.g = x.clamp(0.0, 255.0), v))/>
                        <Channel label="B" value=Signal::derive(move || fmt_num(rgba().b.round()))
                            on=Callback::new(move |v| channel(|_, c, x| c.b = x.clamp(0.0, 255.0), v))/>
                    }.into_any(),
                    Model::Hsl => {
                        let hsl = move || rgba().to_hsl();
                        view! {
                            <Channel label="H" value=Signal::derive(move || fmt_num(hsva.get().h.round()))
                                on=Callback::new(move |v| channel(|h, _, x| h.h = x.rem_euclid(360.0), v))/>
                            <Channel label="S" value=Signal::derive(move || fmt_num((hsl().1 * 100.0).round()))
                                on=Callback::new(move |v| channel(|h, _, x| {
                                    let c = Rgba::from_hsva(*h);
                                    let (hh, _, l) = c.to_hsl();
                                    *h = Hsva { a: h.a, ..Rgba::from_hsl(hh, x / 100.0, l).to_hsva() };
                                }, v))/>
                            <Channel label="L" value=Signal::derive(move || fmt_num((hsl().2 * 100.0).round()))
                                on=Callback::new(move |v| channel(|h, _, x| {
                                    let c = Rgba::from_hsva(*h);
                                    let (hh, s, _) = c.to_hsl();
                                    *h = Hsva { a: h.a, ..Rgba::from_hsl(hh, s, x / 100.0).to_hsva() };
                                }, v))/>
                        }.into_any()
                    }
                    Model::Hsb => view! {
                        <Channel label="H" value=Signal::derive(move || fmt_num(hsva.get().h.round()))
                            on=Callback::new(move |v| channel(|h, _, x| h.h = x.rem_euclid(360.0), v))/>
                        <Channel label="S" value=Signal::derive(move || fmt_num((hsva.get().s * 100.0).round()))
                            on=Callback::new(move |v| channel(|h, _, x| h.s = (x / 100.0).clamp(0.0, 1.0), v))/>
                        <Channel label="B" value=Signal::derive(move || fmt_num((hsva.get().v * 100.0).round()))
                            on=Callback::new(move |v| channel(|h, _, x| h.v = (x / 100.0).clamp(0.0, 1.0), v))/>
                    }.into_any(),
                }}
                <Channel label="%" value=Signal::derive(move || fmt_num((hsva.get().a * 100.0).round()))
                    on=Callback::new(move |v| channel(|h, _, x| h.a = (x / 100.0).clamp(0.0, 1.0), v))/>
            </div>
            {with_contrast.then(|| view! {
                {move || contrast_line().map(|(t, ok)| view! { <p class="cp-contrast" class:is-low=!ok>{t}</p> })}
            })}
            {(!vars.is_empty() || !used.is_empty()).then(|| view! {
                <div class="cp-site">
                    <p class="cp-title">{ui.t("color-site")}</p>
                    <div class="cp-swatches">
                        {vars.into_iter().map(|(name, c)| {
                            let n2 = name.clone();
                            view! {
                                <button class="cp-swatch is-var" style=swatch_style(Some(c)) title=name.clone() aria-label=name
                                    on:click=move |_| {
                                        let h = hsva.get_untracked().h;
                                        let mut n = c.to_hsva();
                                        if n.s == 0.0 { n.h = h; }
                                        hsva.set(n);
                                        on_commit.run(Some(format!("var({n2})")));
                                    }></button>
                            }
                        }).collect_view()}
                        {used.into_iter().map(|c| view! {
                            <button class="cp-swatch" style=swatch_style(Some(c)) title=c.to_css() aria-label=c.to_css()
                                on:click=move |_| set_rgba(c)></button>
                        }).collect_view()}
                    </div>
                </div>
            })}
        </div>
    }
}

/// Couleur : pastille (ouvre le sélecteur) et valeur modifiable (`#3d7eff`, `rgb()`, `--variable`).
#[component]
pub fn ColorInput(
    sx: Sx,
    /// Valeur réglée ici.
    #[prop(into)]
    value: Signal<Option<String>>,
    /// Indication (valeur héritée ou effective).
    #[prop(into)]
    hint: Signal<String>,
    /// Couleur effective (variables résolues), pour la pastille et l'ouverture du sélecteur.
    #[prop(into)]
    resolved: Signal<String>,
    on_preview: Callback<String>,
    on_commit: Callback<Option<String>>,
    #[prop(optional)] with_contrast: bool,
    #[prop(optional)] label: &'static str,
) -> impl IntoView {
    let ui = sx.ui;
    let id = menu_id();
    let color = move || {
        value
            .get()
            .and_then(|v| Rgba::parse(&v))
            .or_else(|| Rgba::parse(&resolved.get()))
            .or_else(|| Rgba::parse(&hint.get()))
    };
    let commit = move |v: Option<String>| {
        if v != value.get_untracked() {
            on_commit.run(v);
        } else {
            sx.clear_preview();
        }
    };
    view! {
        <div class="color-input" class:is-here=move || value.get().is_some()>
            <button class="color-swatch pop-trigger" style=move || swatch_style(color())
                title=ui.t("color-open") aria-label=ui.t("color-open")
                on:click=move |_| sx.toggle_menu(id)></button>
            <input class="nbox-input" type="text" spellcheck="false"
                aria-label=move || if label.is_empty() { String::new() } else { ui.now(label, &[]) }
                prop:value=move || value.get().map(|v| display(&v)).unwrap_or_default()
                placeholder=move || {
                    // Couleur effective : écrite en hexadécimal ; transparente : « Aucune ».
                    let h = hint.get();
                    match Rgba::parse(&h) {
                        Some(c) if c.a == 0.0 => ui.tx("color-none", &[]),
                        Some(c) if !is_var(&h) => c.to_css(),
                        _ => display(&h),
                    }
                }
                on:change=move |e| commit(from_input(&event_target_value(&e)))
                on:keydown=move |e: ev::KeyboardEvent| { if e.key() == "Enter" { commit(from_input(&event_target_value(&e))) } }/>
            <span class="color-alpha" class:is-hidden=move || color().is_none_or(|c| c.alpha() >= 1.0 || c.a == 0.0)>
                {move || color().map(|c| format!("{}%", fmt_num((c.a * 100.0).round()))).unwrap_or_default()}
            </span>
            <Pop sx id wide=true>
                {move || {
                    let start = untrack(color).unwrap_or(Rgba::BLACK);
                    view! { <Picker sx start on_preview on_commit=Callback::new(commit) with_contrast/> }
                }}
            </Pop>
        </div>
    }
}

/// Couleur d'une propriété.
#[component]
pub fn ColorField(
    sx: Sx,
    prop: &'static str,
    #[prop(optional)] with_contrast: bool,
) -> impl IntoView {
    let pv = Memo::new(move |_| sx.get(prop));
    view! {
        <ColorInput sx
            value=Signal::derive(move || pv.get().here)
            hint=Signal::derive(move || pv.get().hint())
            resolved=Signal::derive(move || pv.get().computed)
            on_preview=Callback::new(move |v: String| sx.preview(&[(prop.to_string(), v)]))
            on_commit=Callback::new(move |v: Option<String>| sx.set(prop, v))
            with_contrast/>
    }
}
