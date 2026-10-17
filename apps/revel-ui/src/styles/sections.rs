//! Sections du panneau de styles (D-20) : Disposition, Espacement, Taille, Position, Texte, Fond,
//! Bordure, Effets, Code CSS. Noms et choix : `docs/DESIGN.md`, section *Panneau de styles*.

use super::color::{ColorField, ColorInput};
use super::controls::*;
use super::ctx::Sx;
use super::fx::{FILTERS, FuncList, TRANSFORMS, Transitions};
use crate::icons;
use leptos::ev;
use leptos::prelude::*;
use revel_edit::Command;
use revel_model::{Decl, StyleItem, StyleRule};
use revel_style::values::{
    Shadow, first_family, format_shadows, length::parse_num, parse_shadows, split_top,
};
use wasm_bindgen::JsCast;

// ------------------------------------------------------------------------------------------------
// Espacement : boîte des marges

static MARGIN: [&str; 4] = ["margin-top", "margin-right", "margin-bottom", "margin-left"];
static PADDING: [&str; 4] = [
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
];
static INSET: [&str; 4] = ["top", "right", "bottom", "left"];

/// Valeur courte d'un côté (`16` pour `16px`, `2rem`, `auto`).
fn short(v: &str) -> String {
    match parse_num(v) {
        Some(n) if n.unit == "px" || n.value == 0.0 => revel_style::values::fmt_num(n.value),
        _ => v.to_string(),
    }
}

/// Un côté de la boîte : glisser (vers la droite ou le haut augmente ; Alt : côté opposé aussi,
/// Maj : les quatre côtés), clic : saisie, Alt+clic : retiré.
#[component]
fn Side(sx: Sx, group: &'static [&'static str; 4], side: usize) -> impl IntoView {
    let ui = sx.ui;
    let prop = group[side];
    let pv = Memo::new(move |_| sx.get(prop));
    let editing = RwSignal::new(false);
    let drag = Scrub::new();
    let targets = move |alt: bool, shift: bool| -> Vec<&'static str> {
        if shift {
            group.to_vec()
        } else if alt {
            vec![prop, group[(side + 2) % 4]]
        } else {
            vec![prop]
        }
    };
    let input = NodeRef::<leptos::html::Input>::new();
    Effect::new(move |_| {
        if editing.get()
            && let Some(i) = input.get()
        {
            let _ = i.focus();
            i.select();
        }
    });
    let commit = move |t: String| {
        editing.set(false);
        let cur = pv.get_untracked();
        let base = cur
            .here
            .clone()
            .unwrap_or_else(|| start_value(&cur, "px", "0px"));
        let v = revel_style::values::eval(&t, Some(&base), "px");
        if v != cur.here {
            sx.set(prop, v);
        }
    };
    view! {
        <div class=format!("bm-side bm-side--{side}") data-prop=prop>
            <Show when=move || editing.get() fallback=move || view! {
                <button class="bm-value" class:is-here=move || pv.get().is_here()
                    class:is-inherited=move || matches!(pv.get().origin, super::ctx::Origin::Inherited(_))
                    title=move || format!("{prop} — {}", ui.tx("styles-side-help", &[]))
                    on:pointerdown=move |e: ev::PointerEvent| drag.down(&e, start_value(&pv.get_untracked(), "px", "0px"))
                    on:pointermove=move |e: ev::PointerEvent| {
                        if let Some((s, px)) = drag.moved(&e, true)
                            && let Some(v) = dragged(&s, px, 1.0, "px", 1.0)
                        {
                            let list: Vec<(String, String)> = targets(e.alt_key(), e.shift_key()).into_iter().map(|p| (p.to_string(), v.clone())).collect();
                            sx.preview(&list);
                            drag_last(v);
                        }
                    }
                    on:pointerup=move |e: ev::PointerEvent| match drag.up() {
                        Some(true) => if let Some(v) = take_last() {
                            sx.set_many(targets(e.alt_key(), e.shift_key()).into_iter().map(|p| (p.to_string(), Some(v.clone()))).collect(), false);
                        },
                        Some(false) => if e.alt_key() { sx.set(prop, None) } else { editing.set(true) },
                        None => {}
                    }
                    // Clavier (Entrée, Espace) ou poignée du canevas : saisie.
                    on:click=move |e: ev::MouseEvent| if e.detail() == 0 { editing.set(true) }>
                    {move || { let p = pv.get(); short(&p.here.clone().unwrap_or_else(|| p.hint())) }}
                </button>
            }>
                <input class="bm-input" node_ref=input spellcheck="false"
                    prop:value=move || pv.get_untracked().here.map(|v| short(&v)).unwrap_or_default()
                    on:blur=move |e| { if editing.get_untracked() { commit(event_target_value(&e)) } }
                    on:keydown=move |e: ev::KeyboardEvent| match e.key().as_str() {
                        "Enter" => commit(event_target_value(&e)),
                        "Escape" => editing.set(false),
                        _ => {}
                    }/>
            </Show>
        </div>
    }
}

/// Boîte : marge extérieure autour de la marge intérieure (ou décalages d'une position).
#[component]
fn SidesBox(
    sx: Sx,
    group: &'static [&'static str; 4],
    key: &'static str,
    #[prop(optional)] inner: Option<(&'static [&'static str; 4], &'static str)>,
) -> impl IntoView {
    view! {
        <div class="bm-box" class:bm-box--outer=inner.is_some() title=sx.ui.t("styles-spacing-help")>
            <div class="bm-caption"><Label sx props=group.as_slice() key/></div>
            <Side sx group side=0/>
            <Side sx group side=3/>
            <div class="bm-center">
                {inner.map(|(g, k)| view! {
                    <div class="bm-box bm-box--inner">
                        <div class="bm-caption"><Label sx props=g.as_slice() key=k/></div>
                        <Side sx group=g side=0/>
                        <Side sx group=g side=3/>
                        <div class="bm-center"><div class="bm-core"></div></div>
                        <Side sx group=g side=1/>
                        <Side sx group=g side=2/>
                    </div>
                })}
                {inner.is_none().then(|| view! { <div class="bm-core"></div> })}
            </div>
            <Side sx group side=1/>
            <Side sx group side=2/>
        </div>
    }
}

#[component]
pub fn SpacingSection(sx: Sx) -> impl IntoView {
    on_cleanup(move || {
        sx.ed
            .post(serde_json::json!({ "type": "spacing", "on": false }))
    });
    view! {
        <Section sx key="styles-spacing" open=true props=&[
            "margin-top", "margin-right", "margin-bottom", "margin-left",
            "padding-top", "padding-right", "padding-bottom", "padding-left",
        ]>
            // Survolée : marges et espacements de la sélection montrés dans le canevas.
            <div class="spacing-zone"
                on:mouseenter=move |_| sx.ed.post(serde_json::json!({ "type": "spacing", "on": true }))
                on:mouseleave=move |_| sx.ed.post(serde_json::json!({ "type": "spacing", "on": false }))>
                <SidesBox sx group=&MARGIN key="styles-margin-short" inner=(&PADDING, "styles-padding-short")/>
            </div>
        </Section>
    }
}

// ------------------------------------------------------------------------------------------------
// Taille

const OVERFLOW: &[Opt] = &[
    o("visible", "overflow-visible"),
    o("hidden", "overflow-hidden"),
    o("scroll", "overflow-scroll"),
    o("auto", "overflow-auto"),
];
const FIT: &[Opt] = &[
    o("cover", "fit-cover"),
    o("contain", "fit-contain"),
    o("fill", "fit-fill"),
    o("none", "fit-none"),
    o("scale-down", "fit-scale-down"),
];
const RATIOS: &[Opt] = &[
    o("auto", "ratio-auto"),
    oa("1 / 1", "ratio-square", &["1"]),
    oa("16 / 9", "ratio-wide", &["16/9"]),
    oa("4 / 3", "ratio-classic", &["4/3"]),
    oa("3 / 2", "ratio-landscape", &["3/2"]),
    oa("2 / 3", "ratio-portrait", &["2/3"]),
    oa("9 / 16", "ratio-tall", &["9/16"]),
];

#[component]
pub fn SizeSection(sx: Sx) -> impl IntoView {
    let media = move || {
        matches!(
            sx.tag.get().as_str(),
            "img" | "video" | "picture" | "iframe" | "svg" | "canvas"
        )
    };
    view! {
        <Section sx key="styles-size" props=&["width", "height", "min-width", "min-height", "max-width", "max-height", "overflow", "object-fit", "aspect-ratio"]>
            <div class="style-grid">
                <Cell sx props=&["width"] key="styles-width" units=SIZE_UNITS/>
                <Cell sx props=&["height"] key="styles-height" units=SIZE_UNITS/>
                <Cell sx props=&["min-width"] key="styles-min-width" units=SIZE_UNITS/>
                <Cell sx props=&["min-height"] key="styles-min-height" units=SIZE_UNITS/>
                <Cell sx props=&["max-width"] key="styles-max-width" units=MAX_UNITS/>
                <Cell sx props=&["max-height"] key="styles-max-height" units=MAX_UNITS/>
            </div>
            <Row sx props=&["overflow"] key="styles-overflow" stack=true>
                <Seg sx prop="overflow" options=OVERFLOW/>
            </Row>
            <Row sx props=&["aspect-ratio"] key="styles-ratio">
                <Choice sx prop="aspect-ratio" options=opts(sx, RATIOS)/>
            </Row>
            <Show when=media>
                <Row sx props=&["object-fit"] key="styles-fit">
                    <Choice sx prop="object-fit" options=opts(sx, FIT)/>
                </Row>
            </Show>
        </Section>
    }
}

// ------------------------------------------------------------------------------------------------
// Position

const POSITION: &[Opt] = &[
    o("static", "position-static"),
    o("relative", "position-relative"),
    o("absolute", "position-absolute"),
    o("fixed", "position-fixed"),
    o("sticky", "position-sticky"),
];

/// Préréglages de décalage : (clé, haut, droite, bas, gauche).
const PRESETS: &[(&str, [Option<&str>; 4])] = &[
    ("preset-full", [Some("0"), Some("0"), Some("0"), Some("0")]),
    ("preset-top", [Some("0"), Some("0"), None, Some("0")]),
    ("preset-bottom", [None, Some("0"), Some("0"), Some("0")]),
    ("preset-left", [Some("0"), None, Some("0"), Some("0")]),
    ("preset-right", [Some("0"), Some("0"), Some("0"), None]),
    ("preset-top-left", [Some("0"), None, None, Some("0")]),
    ("preset-top-right", [Some("0"), Some("0"), None, None]),
    ("preset-bottom-left", [None, None, Some("0"), Some("0")]),
    ("preset-bottom-right", [None, Some("0"), Some("0"), None]),
];

#[component]
pub fn PositionSection(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let pos = Memo::new(move |_| sx.get("position").current());
    let placed = move || !matches!(pos.get().as_str(), "" | "static");
    let help = move || match pos.get().as_str() {
        "relative" => ui.tx("position-relative-help", &[]),
        "absolute" => ui.tx("position-absolute-help", &[]),
        "fixed" => ui.tx("position-fixed-help", &[]),
        "sticky" => ui.tx("position-sticky-help", &[]),
        _ => ui.tx("position-static-help", &[]),
    };
    view! {
        <Section sx key="styles-position" props=&["position", "top", "right", "bottom", "left", "z-index"]>
            <Row sx props=&["position"] key="styles-position-type">
                <div title=help><Choice sx prop="position" options=opts(sx, POSITION)/></div>
            </Row>
            <Show when=placed>
                <SidesBox sx group=&INSET key="styles-offsets"/>
                <Row sx props=&["top", "right", "bottom", "left"] key="styles-preset">
                    <div class="select">
                        <select class="input" on:change=move |e| {
                            let k = event_target_value(&e);
                            if let Some((_, v)) = PRESETS.iter().find(|(key, _)| *key == k) {
                                sx.set_many(INSET.iter().zip(v.iter()).map(|(p, x)| (p.to_string(), x.map(String::from))).collect(), false);
                            }
                        }>
                            <option value="" selected=true>{ui.t("preset-choose")}</option>
                            {PRESETS.iter().map(|(k, _)| view! { <option value=*k>{ui.t(k)}</option> }).collect_view()}
                        </select>
                    </div>
                </Row>
            </Show>
            <Row sx props=&["z-index"] key="styles-z-index" scrub=Some(LabelScrub { prop: "z-index", unit: "", step: 1.0, zero: "0" })>
                <PropNum sx prop="z-index" unit="" label="styles-z-index"/>
            </Row>
        </Section>
    }
}

// ------------------------------------------------------------------------------------------------
// Texte

const WEIGHTS: &[Opt] = &[
    o("100", "weight-100"),
    o("200", "weight-200"),
    o("300", "weight-300"),
    oa("400", "weight-400", &["normal"]),
    o("500", "weight-500"),
    o("600", "weight-600"),
    oa("700", "weight-700", &["bold"]),
    o("800", "weight-800"),
    o("900", "weight-900"),
];
const TEXT_ALIGN: &[Opt] = &[
    Opt {
        v: "left",
        key: "text-left",
        icon: Some(icons::text_left),
        also: &["start"],
    },
    oi("center", "text-center", icons::text_center),
    Opt {
        v: "right",
        key: "text-right",
        icon: Some(icons::text_right),
        also: &["end"],
    },
    oi("justify", "text-justify", icons::text_justify),
];
fn g_none() -> AnyView {
    view! { <span class="glyph">"–"</span> }.into_any()
}
fn g_upper() -> AnyView {
    view! { <span class="glyph">"AA"</span> }.into_any()
}
fn g_lower() -> AnyView {
    view! { <span class="glyph">"aa"</span> }.into_any()
}
fn g_cap() -> AnyView {
    view! { <span class="glyph">"Aa"</span> }.into_any()
}
const CASE: &[Opt] = &[
    oi("none", "case-none", g_none),
    oi("uppercase", "case-upper", g_upper),
    oi("lowercase", "case-lower", g_lower),
    oi("capitalize", "case-capitalize", g_cap),
];

/// Retour à la ligne : automatique, jamais (une ligne), retours écrits gardés.
const WHITE_SPACE: &[Opt] = &[
    o("normal", "ws-normal"),
    o("nowrap", "ws-nowrap"),
    o("pre-line", "ws-pre-line"),
    o("pre-wrap", "ws-pre-wrap"),
];
/// Coupure des lignes : lignes équilibrées (titres), sans mot seul en fin (paragraphes).
const TEXT_WRAP: &[Opt] = &[
    oa("wrap", "tw-wrap", &["auto"]),
    o("balance", "tw-balance"),
    o("pretty", "tw-pretty"),
];

/// Texte coupé sur une ligne avec des points de suspension (« Truncate » de Webflow) : trois
/// déclarations posées ou retirées ensemble.
#[component]
fn Truncate(sx: Sx) -> impl IntoView {
    let pv = Memo::new(move |_| sx.get("text-overflow"));
    let on = move || pv.with(|p| p.current() == "ellipsis");
    let ui = expect_context::<crate::Ui>();
    view! {
        <label class="switch" title=ui.t("styles-truncate-help")>
            <input type="checkbox" aria-label=ui.t("styles-truncate") prop:checked=on on:change=move |e| {
                let v = event_target_checked(&e);
                sx.set_many(vec![
                    ("text-overflow".into(), v.then(|| "ellipsis".into())),
                    ("overflow".into(), v.then(|| "hidden".into())),
                    ("white-space".into(), v.then(|| "nowrap".into())),
                ], false);
            }/>
            <span class="switch-track"></span>
        </label>
    }
}

/// Valeur d'une variable CSS du site (`--font`), déclarée sur `:root` ou `html`.
pub fn css_var(sheet: &revel_model::Stylesheet, name: &str) -> Option<String> {
    sheet.items.iter().find_map(|item| match item {
        StyleItem::Rule(r)
            if r.media.is_none()
                && (r.selector.contains(":root") || r.selector.trim() == "html") =>
        {
            r.decls
                .iter()
                .find(|d| d.prop == name)
                .map(|d| d.value.clone())
        }
        _ => None,
    })
}

/// Nom d'une police : sa première famille ; une variable (`var(--font)`) par la police qu'elle
/// contient, sinon par son nom lisible.
pub fn font_label(sheet: &revel_model::Stylesheet, value: &str) -> String {
    let v = value.trim();
    if let Some(inner) = v.strip_prefix("var(").and_then(|r| r.strip_suffix(')')) {
        let name = inner.split(',').next().unwrap_or_default().trim();
        return css_var(sheet, name)
            .filter(|x| !x.trim_start().starts_with("var("))
            .map(|x| first_family(&x))
            .filter(|x| !x.is_empty())
            .unwrap_or_else(|| crate::names::humanize(name));
    }
    first_family(v)
}

/// Polices proposées : piles de polices du site (par première famille), polices déclarées par
/// `@font-face`, polices du système.
fn font_options(sx: Sx) -> Signal<Vec<Item>> {
    let ui = sx.ui;
    Signal::derive(move || {
        let mut out: Vec<Item> = Vec::new();
        let mut push = |v: String, l: String| {
            if !out.iter().any(|(_, x, _)| *x == l) {
                out.push((v, l, &[]));
            }
        };
        sx.ed.doc.with(|d| {
            let Some(sheet) = d.as_ref().and_then(|d| d.stylesheet.as_ref()) else {
                return;
            };
            for item in &sheet.items {
                match item {
                    StyleItem::Rule(r) => {
                        for d in r.decls.iter().filter(|d| d.prop == "font-family") {
                            push(d.value.clone(), font_label(sheet, &d.value));
                        }
                    }
                    StyleItem::Raw { css } if css.trim_start().starts_with("@font-face") => {
                        if let Some(i) = css.find("font-family") {
                            let rest = &css[i + 11..];
                            let v = rest
                                .trim_start_matches([' ', ':'])
                                .split([';', '}'])
                                .next()
                                .unwrap_or_default();
                            let f = first_family(v.trim());
                            if !f.is_empty() {
                                push(format!("\"{f}\""), f);
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
        push(
            "system-ui, -apple-system, \"Segoe UI\", sans-serif".into(),
            ui.tx("font-system", &[]),
        );
        push(
            "Georgia, \"Times New Roman\", serif".into(),
            ui.tx("font-serif", &[]),
        );
        push(
            "ui-monospace, Menlo, Consolas, monospace".into(),
            ui.tx("font-mono", &[]),
        );
        out
    })
}

/// Soulignement ou barré, cumulables (`text-decoration-line`).
#[component]
fn LineToggle(
    sx: Sx,
    line: &'static str,
    key: &'static str,
    icon: fn() -> AnyView,
) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get("text-decoration-line"));
    let tokens = move || -> Vec<String> {
        split_top(&pv.get().current(), ' ')
            .into_iter()
            .filter(|t| t != "none")
            .collect()
    };
    let active = move || tokens().iter().any(|t| t == line);
    view! {
        <button class="tool-toggle" class:is-on=move || active() && pv.get().here.is_some()
            class:is-implied=move || active() && pv.get().here.is_none()
            aria-pressed=move || active().to_string() title=ui.t(key) aria-label=ui.t(key)
            on:click=move |_| {
                let mut t = tokens();
                if active() { t.retain(|x| x != line) } else { t.push(line.to_string()) }
                sx.set("text-decoration-line", Some(if t.is_empty() { "none".into() } else { t.join(" ") }));
            }>{icon()}</button>
    }
}

#[component]
pub fn TextSection(sx: Sx) -> impl IntoView {
    view! {
        <Section sx key="styles-typography" open=true props=&[
            "font-family", "font-weight", "font-size", "line-height", "letter-spacing", "color",
            "text-align", "font-style", "text-decoration-line", "text-transform", "white-space", "text-wrap",
            "text-overflow",
        ]>
            <Row sx props=&["font-family"] key="styles-font-family">
                <Choice sx prop="font-family" options=font_options(sx)/>
            </Row>
            <Row sx props=&["font-weight"] key="styles-font-weight">
                <Choice sx prop="font-weight" options=opts(sx, WEIGHTS)/>
            </Row>
            <Row sx props=&["font-size"] key="styles-font-size" scrub=Some(LabelScrub { prop: "font-size", unit: "px", step: 1.0, zero: "16px" })>
                <PropNum sx prop="font-size" units=FONT_UNITS zero="16px" label="styles-font-size"/>
            </Row>
            <Row sx props=&["line-height"] key="styles-line-height" scrub=Some(LabelScrub { prop: "line-height", unit: "", step: 0.1, zero: "1.2" })>
                <PropNum sx prop="line-height" unit="" units=LINE_UNITS step=0.1 zero="1.2" label="styles-line-height"/>
            </Row>
            <Row sx props=&["letter-spacing"] key="styles-letter-spacing" scrub=Some(LabelScrub { prop: "letter-spacing", unit: "px", step: 0.1, zero: "0px" })>
                <PropNum sx prop="letter-spacing" units=LETTER_UNITS step=0.1 zero="0px" label="styles-letter-spacing"/>
            </Row>
            <Row sx props=&["color"] key="styles-color">
                <ColorField sx prop="color" with_contrast=true/>
            </Row>
            <Row sx props=&["text-align"] key="styles-text-align">
                <Seg sx prop="text-align" options=TEXT_ALIGN/>
            </Row>
            <Row sx props=&["font-style", "text-decoration-line"] key="styles-text-style">
                <div class="ctl-line">
                    <Toggle sx prop="font-style" on="italic" off="normal" key="text-italic" icon=icons::italic/>
                    <LineToggle sx line="underline" key="text-underline" icon=icons::underline/>
                    <LineToggle sx line="line-through" key="text-strike" icon=icons::strike/>
                </div>
            </Row>
            <Row sx props=&["text-transform"] key="styles-text-transform">
                <Seg sx prop="text-transform" options=CASE/>
            </Row>
            <Row sx props=&["white-space"] key="styles-white-space">
                <Choice sx prop="white-space" options=opts(sx, WHITE_SPACE)/>
            </Row>
            <Row sx props=&["text-wrap"] key="styles-text-wrap">
                <Choice sx prop="text-wrap" options=opts(sx, TEXT_WRAP)/>
            </Row>
            <Row sx props=&["text-overflow", "overflow", "white-space"] key="styles-truncate">
                <Truncate sx/>
            </Row>
        </Section>
    }
}

// ------------------------------------------------------------------------------------------------
// Fond

const BG_SIZE: &[Opt] = &[
    o("cover", "bg-cover"),
    o("contain", "bg-contain"),
    oa("auto", "bg-auto", &["auto auto"]),
];

/// Position 3 × 3 d'une image de fond.
#[component]
fn BgPosition(sx: Sx) -> impl IntoView {
    let pv = Memo::new(move |_| sx.get("background-position"));
    const X: [&str; 3] = ["left", "center", "right"];
    const Y: [&str; 3] = ["top", "center", "bottom"];
    let at = move || {
        let v = pv.get().current();
        let t = split_top(&v, ' ');
        let pos = |s: &str, axis: [&str; 3]| -> Option<usize> {
            match s {
                "0%" | "0px" | "0" => Some(0),
                "50%" => Some(1),
                "100%" => Some(2),
                s => axis.iter().position(|a| *a == s),
            }
        };
        match t.as_slice() {
            [x, y] => (pos(x, X), pos(y, Y)),
            [c] if c == "center" => (Some(1), Some(1)),
            _ => (None, None),
        }
    };
    view! {
        <div class="align-box align-box--small" role="group">
            {(0..3).flat_map(|r| (0..3).map(move |c| (c, r))).map(|(c, r)| view! {
                <button class="align-cell" class:is-on=move || at() == (Some(c), Some(r))
                    title=format!("{} {}", X[c], Y[r])
                    on:click=move |_| sx.set("background-position", Some(format!("{} {}", X[c], Y[r]))) ><i></i></button>
            }).collect_view()}
        </div>
    }
}

#[component]
pub fn BackgroundSection(sx: Sx) -> impl IntoView {
    // Taille, position et répétition : pour un calque image (pas pour un dégradé seul).
    let has_image = move || {
        let v = sx.get("background-image").current();
        super::background::parse_layers(&v)
            .iter()
            .any(|l| matches!(l, super::background::Layer::Image(_)))
    };
    view! {
        <Section sx key="styles-backgrounds" props=&["background-color", "background-image", "background-size", "background-position", "background-repeat"]>
            <Row sx props=&["background-color"] key="styles-background-color">
                <ColorField sx prop="background-color"/>
            </Row>
            <Row sx props=&["background-image"] key="styles-background-image" stack=true>
                <super::background::BgLayers sx/>
            </Row>
            <Show when=has_image>
                <Row sx props=&["background-size"] key="styles-background-size" stack=true>
                    <Seg sx prop="background-size" options=BG_SIZE/>
                </Row>
                <Row sx props=&["background-position"] key="styles-background-position">
                    <BgPosition sx/>
                </Row>
                <Row sx props=&["background-repeat"] key="styles-background-repeat">
                    <Switch sx prop="background-repeat" on="repeat" off="no-repeat"/>
                </Row>
            </Show>
        </Section>
    }
}

// ------------------------------------------------------------------------------------------------
// Bordure

static BW: [&str; 5] = [
    "border-width",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
];
static BS: [&str; 5] = [
    "border-style",
    "border-top-style",
    "border-right-style",
    "border-bottom-style",
    "border-left-style",
];
static BC: [&str; 5] = [
    "border-color",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
];
const BORDER_STYLE: &[Opt] = &[
    o("none", "border-none"),
    o("solid", "border-solid"),
    o("dashed", "border-dashed"),
    o("dotted", "border-dotted"),
];
/// Icône d'un bouton.
type Icon = fn() -> AnyView;
const SIDE_KEYS: [(&str, Icon); 5] = [
    ("side-all", icons::side_all),
    ("side-top", icons::side_top),
    ("side-right", icons::side_right),
    ("side-bottom", icons::side_bottom),
    ("side-left", icons::side_left),
];
static CORNERS: [&str; 4] = [
    "border-top-left-radius",
    "border-top-right-radius",
    "border-bottom-left-radius",
    "border-bottom-right-radius",
];
const CORNER_KEYS: [&str; 4] = [
    "corner-top-left",
    "corner-top-right",
    "corner-bottom-left",
    "corner-bottom-right",
];

#[component]
pub fn BorderSection(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let side = RwSignal::new(0usize);
    // Angles indépendants : ouverts d'office si les angles diffèrent.
    let corners_differ = move || {
        let v: Vec<String> = CORNERS.iter().map(|c| sx.get(c).current()).collect();
        v.iter().any(|x| *x != v[0])
    };
    let corners = RwSignal::new(false);
    view! {
        <Section sx key="styles-borders" props=&["border-radius", "border-width", "border-style", "border-color",
            "border-top-width", "border-right-width", "border-bottom-width", "border-left-width"]>
            <Row sx props=&["border-radius"] key="styles-border-radius" scrub=Some(LabelScrub { prop: "border-radius", unit: "px", step: 1.0, zero: "0px" })>
                <div class="ctl-line">
                    <PropNum sx prop="border-radius" units=GAP_UNITS label="styles-border-radius"/>
                    <button class="tool-toggle" class:is-on=move || corners.get() || corners_differ()
                        aria-pressed=move || (corners.get() || corners_differ()).to_string()
                        title=ui.t("corner-independent") aria-label=ui.t("corner-independent")
                        on:click=move |_| corners.update(|c| *c = !*c)>{icons::corners()}</button>
                </div>
            </Row>
            <Show when=move || corners.get() || corners_differ()>
                <div class="style-grid">
                    {(0..4).map(|i| view! {
                        <Cell sx props=&CORNERS[i..=i] key=CORNER_KEYS[i] units=GAP_UNITS/>
                    }).collect_view()}
                </div>
            </Show>
            <Row sx props=&["border-width"] key="styles-border-sides" stack=true>
                <div class="seg" role="radiogroup">
                    {SIDE_KEYS.iter().enumerate().map(|(i, &(k, icon))| view! {
                        <button role="radio" class:is-on=move || side.get() == i aria-checked=move || (side.get() == i).to_string()
                            title=ui.t(k) aria-label=ui.t(k) on:click=move |_| side.set(i)>{icon()}</button>
                    }).collect_view()}
                </div>
            </Row>
            {move || {
                let i = side.get();
                view! {
                    <Row sx props=&BW[i..=i] key="styles-border-width" scrub=Some(LabelScrub { prop: BW[i], unit: "px", step: 1.0, zero: "0px" })>
                        <PropNum sx prop=BW[i] units=GAP_UNITS label="styles-border-width"/>
                    </Row>
                    <Row sx props=&BS[i..=i] key="styles-border-style" stack=true>
                        <Seg sx prop=BS[i] options=BORDER_STYLE/>
                    </Row>
                    <Row sx props=&BC[i..=i] key="styles-border-color">
                        <ColorField sx prop=BC[i]/>
                    </Row>
                }
            }}
        </Section>
    }
}

// ------------------------------------------------------------------------------------------------
// Effets

const CURSORS: &[Opt] = &[
    o("auto", "cursor-auto"),
    o("default", "cursor-default"),
    o("pointer", "cursor-pointer"),
    o("text", "cursor-text"),
    o("move", "cursor-move"),
    o("grab", "cursor-grab"),
    o("not-allowed", "cursor-not-allowed"),
    o("help", "cursor-help"),
    o("none", "cursor-none"),
];

/// Mélange avec ce qui est dessous (comme les modes de fusion d'un logiciel de dessin).
const BLEND: &[Opt] = &[
    o("normal", "blend-normal"),
    o("multiply", "blend-multiply"),
    o("screen", "blend-screen"),
    o("overlay", "blend-overlay"),
    o("darken", "blend-darken"),
    o("lighten", "blend-lighten"),
    o("color-dodge", "blend-color-dodge"),
    o("color-burn", "blend-color-burn"),
    o("hard-light", "blend-hard-light"),
    o("soft-light", "blend-soft-light"),
    o("difference", "blend-difference"),
    o("exclusion", "blend-exclusion"),
    o("hue", "blend-hue"),
    o("saturation", "blend-saturation"),
    o("color", "blend-color"),
    o("luminosity", "blend-luminosity"),
];

/// Opacité : curseur et pourcentage ; écrite en décimal (`0.6`).
#[component]
fn Opacity(sx: Sx) -> impl IntoView {
    let pv = Memo::new(move |_| sx.get("opacity"));
    let pct = move || {
        let p = pv.get();
        parse_num(&p.current())
            .map(|n| {
                if n.unit == "%" {
                    n.value
                } else {
                    n.value * 100.0
                }
            })
            .unwrap_or(100.0)
    };
    let to_css = |pct: f64| revel_style::values::fmt_num((pct.clamp(0.0, 100.0)) / 100.0);
    view! {
        <div class="ctl-line">
            <input type="range" class="range" min="0" max="100" step="1" prop:value=move || pct().round().to_string()
                class:is-here=move || pv.with(|p| p.origin == super::ctx::Origin::Here)
                class:is-inherited=move || pv.with(|p| matches!(p.origin, super::ctx::Origin::Inherited(_)))
                style=move || format!("--fill: {}%", pct().round())
                aria-label=sx.ui.t("styles-opacity")
                on:input=move |e| {
                    if let Ok(v) = event_target_value(&e).parse::<f64>() {
                        sx.preview(&[("opacity".into(), to_css(v))]);
                        if let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) {
                            let _ = t.style().set_property("--fill", &format!("{v}%"));
                        }
                    }
                }
                on:change=move |e| { if let Ok(v) = event_target_value(&e).parse::<f64>() { sx.set("opacity", Some(to_css(v))) } }/>
            <NumBox sx
                value=Signal::derive(move || pv.get().here.map(|_| revel_style::values::fmt_num(pct().round())))
                hint=Signal::derive(move || revel_style::values::fmt_num(pct().round()))
                start=Signal::derive(move || revel_style::values::fmt_num(pct().round()))
                on_preview=Callback::new(move |v: String| if let Ok(x) = v.parse::<f64>() { sx.preview(&[("opacity".into(), to_css(x))]) })
                on_commit=Callback::new(move |(v, co): (Option<String>, bool)| {
                    let v = v.and_then(|v| v.trim_end_matches('%').parse::<f64>().ok()).map(to_css);
                    sx.set_many(vec![("opacity".into(), v)], co);
                })
                unit="" suffix=Some("%") label="styles-opacity"/>
        </div>
    }
}

/// Ombres : liste de calques (portée ou intérieure, X, Y, flou, étendue, couleur).
#[component]
fn Shadows(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get("box-shadow"));
    let list = Memo::new(move |_| pv.get().here.and_then(|v| parse_shadows(&v)));
    let raw = move || pv.get().here.is_some() && list.get().is_none();
    let write =
        move |l: Vec<Shadow>| sx.set("box-shadow", (!l.is_empty()).then(|| format_shadows(&l)));
    let preview = move |l: Vec<Shadow>| sx.preview(&[("box-shadow".into(), format_shadows(&l))]);
    let edit = move |i: usize, f: Box<dyn Fn(&mut Shadow)>| {
        let mut l = list.get_untracked().unwrap_or_default();
        if let Some(s) = l.get_mut(i) {
            f(s);
        }
        l
    };
    view! {
        <div class="shadows">
            <div class="ctl-line ctl-line--end">
                <span class="muted shadow-count">{move || if list.get().is_none_or(|l| l.is_empty()) && !raw() {
                    let h = pv.get().hint();
                    if h.is_empty() || h == "none" { ui.tx("shadow-none", &[]) } else { ui.tx("shadow-inherited", &[]) }
                } else { String::new() }}</span>
                <button class="icon-btn icon-btn--small" title=ui.t("shadow-add") aria-label=ui.t("shadow-add")
                    on:click=move |_| { let mut l = list.get_untracked().unwrap_or_default(); l.push(Shadow::default()); write(l); }>
                    {icons::plus()}
                </button>
            </div>
            <Show when=raw>
                <input class="input input--mono" prop:value=move || pv.get().here.unwrap_or_default()
                    on:change=move |e| { let v = event_target_value(&e); sx.set("box-shadow", (!v.trim().is_empty()).then_some(v)); }/>
            </Show>
            {move || list.get().unwrap_or_default().into_iter().enumerate().map(|(i, s)| {
                let field = move |get: fn(&Shadow) -> String, put: fn(&mut Shadow, String), key: &'static str, prefix: &'static str| view! {
                    <NumBox sx
                        value=Signal::derive(move || list.get().and_then(|l| l.get(i).map(get)))
                        hint=Signal::derive(|| "0".to_string())
                        start=Signal::derive(move || list.get().and_then(|l| l.get(i).map(get)).unwrap_or_else(|| "0px".into()))
                        on_preview=Callback::new(move |v: String| preview(edit(i, Box::new(move |s| put(s, v.clone())))))
                        on_commit=Callback::new(move |(v, _): (Option<String>, bool)| write(edit(i, Box::new(move |s| put(s, v.clone().unwrap_or_else(|| "0px".into()))))))
                        prefix=Some(prefix) label=key/>
                };
                view! {
                    <div class="shadow-card">
                        <div class="ctl-line">
                            <div class="select">
                                <select class="input" aria-label=ui.t("shadow-type") on:change=move |e| {
                                    let inset = event_target_value(&e) == "inset";
                                    write(edit(i, Box::new(move |s| s.inset = inset)));
                                }>
                                    <option value="drop" selected=!s.inset>{ui.t("shadow-drop")}</option>
                                    <option value="inset" selected=s.inset>{ui.t("shadow-inner")}</option>
                                </select>
                            </div>
                            <button class="icon-btn icon-btn--small" title=ui.t("shadow-remove") aria-label=ui.t("shadow-remove")
                                on:click=move |_| { let mut l = list.get_untracked().unwrap_or_default(); if i < l.len() { l.remove(i); } write(l); }>
                                {icons::trash()}
                            </button>
                        </div>
                        <div class="shadow-grid">
                            {field(|s| s.x.clone(), |s, v| s.x = v, "shadow-x", "X")}
                            {field(|s| s.y.clone(), |s, v| s.y = v, "shadow-y", "Y")}
                            {field(|s| s.blur.clone(), |s, v| s.blur = v, "shadow-blur", "⌀")}
                            {field(|s| s.spread.clone(), |s, v| s.spread = v, "shadow-spread", "±")}
                        </div>
                        <ColorInput sx
                            value=Signal::derive(move || list.get().and_then(|l| l.get(i).map(|s| s.color.clone())).filter(|c| !c.is_empty()))
                            hint=Signal::derive(|| "currentcolor".to_string())
                            resolved=Signal::derive(move || list.get().and_then(|l| l.get(i).map(|s| s.color.clone())).unwrap_or_default())
                            on_preview=Callback::new(move |v: String| preview(edit(i, Box::new(move |s| s.color = v.clone()))))
                            on_commit=Callback::new(move |v: Option<String>| write(edit(i, Box::new(move |s| s.color = v.clone().unwrap_or_default()))))
                            label="shadow-color"/>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

#[component]
pub fn EffectsSection(sx: Sx) -> impl IntoView {
    view! {
        <Section sx key="styles-effects" props=&["opacity", "box-shadow", "cursor", "mix-blend-mode", "filter", "backdrop-filter", "transform", "transition"]>
            <Row sx props=&["mix-blend-mode"] key="styles-blend"><Choice sx prop="mix-blend-mode" options=opts(sx, BLEND)/></Row>
            <Row sx props=&["opacity"] key="styles-opacity"><Opacity sx/></Row>
            <Row sx props=&["box-shadow"] key="styles-box-shadow" stack=true><Shadows sx/></Row>
            <Row sx props=&["transform"] key="styles-transform" stack=true>
                <FuncList sx prop="transform" defs=TRANSFORMS add_key="fx-transform-add" none_key="fx-transform-none"/>
            </Row>
            <Row sx props=&["transition"] key="styles-transition" stack=true><Transitions sx/></Row>
            <Row sx props=&["filter"] key="styles-filter" stack=true>
                <FuncList sx prop="filter" defs=FILTERS add_key="fx-filter-add" none_key="fx-filter-none"/>
            </Row>
            <Row sx props=&["backdrop-filter"] key="styles-backdrop" stack=true>
                <FuncList sx prop="backdrop-filter" defs=FILTERS add_key="fx-backdrop-add" none_key="fx-filter-none"/>
            </Row>
            <Row sx props=&["cursor"] key="styles-cursor"><Choice sx prop="cursor" options=opts(sx, CURSORS)/></Row>
        </Section>
    }
}

// ------------------------------------------------------------------------------------------------
// Code CSS

/// Toutes les déclarations de la règle (raccourcis, propriétés personnalisées…), modifiables.
#[component]
pub fn CodeSection(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let ed = sx.ed;
    let rule: Memo<Option<StyleRule>> = sx.rule;
    let set = move |p: String, v: String| sx.set(&p, Some(v));
    let toggle_important = move |d: Decl| {
        let Some(r) = rule.get_untracked() else {
            return;
        };
        ed.apply(
            ui,
            Command::SetDecl {
                rule: r.id.clone().unwrap_or_default(),
                prop: d.prop,
                value: Some(d.value),
                important: !d.important,
            },
            false,
        );
    };
    let remove = move |p: String| {
        let Some(r) = rule.get_untracked() else {
            return;
        };
        ed.apply(
            ui,
            Command::SetDecl {
                rule: r.id.clone().unwrap_or_default(),
                prop: p,
                value: None,
                important: false,
            },
            false,
        );
    };
    view! {
        <details class="style-section">
            <summary><span>{ui.t("styles-declarations")}</span></summary>
            <div class="style-section-body">
                {move || rule.get().map(|r| r.decls).unwrap_or_default().into_iter().map(|d| {
                    let prop = d.prop.clone();
                    let prop_rm = d.prop.clone();
                    let d2 = d.clone();
                    view! {
                        <div class="decl-row">
                            <span class="attr-name" title=d.prop.clone()>{d.prop.clone()}</span>
                            <input class="input input--mono" prop:value=d.value.clone()
                                on:change=move |e| set(prop.clone(), event_target_value(&e))/>
                            <button class="chip chip--toggle" class:is-on=d.important title=ui.t("styles-important")
                                on:click=move |_| toggle_important(d2.clone())>"!"</button>
                            <button class="icon-btn" title=ui.t("styles-reset") aria-label=ui.t("styles-reset")
                                on:click=move |_| remove(prop_rm.clone())>{icons::trash()}</button>
                        </div>
                    }
                }).collect_view()}
                <input class="input input--mono" placeholder=ui.t("styles-decl-add")
                    on:change=move |e| {
                        let v = event_target_value(&e);
                        if let Some((p, val)) = v.split_once(':') {
                            set(p.trim().to_string(), val.trim().trim_end_matches(';').to_string());
                        }
                    }/>
            </div>
        </details>
    }
}
