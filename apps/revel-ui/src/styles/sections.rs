//! Sections du panneau de styles (D-20) : Disposition, Espacement, Taille, Position, Texte, Fond,
//! Bordure, Effets, Code CSS. Noms et choix : `docs/DESIGN.md`, section *Panneau de styles*.

use super::color::{ColorField, ColorInput};
use super::controls::*;
use super::ctx::Sx;
use crate::icons;
use leptos::ev;
use leptos::prelude::*;
use revel_edit::Command;
use revel_model::{Decl, StyleItem, StyleRule};
use revel_style::values::{
    Shadow, first_family, format_shadows, grid_columns, grid_count, length::parse_num,
    parse_shadows, split_top,
};

// ------------------------------------------------------------------------------------------------
// Disposition

const DISPLAY: &[Opt] = &[
    o("block", "display-block"),
    o("flex", "display-flex"),
    o("grid", "display-grid"),
    o("none", "display-none"),
];
const DISPLAY_MORE: &[Opt] = &[
    o("inline", "display-inline"),
    o("inline-block", "display-inline-block"),
    o("inline-flex", "display-inline-flex"),
    o("inline-grid", "display-inline-grid"),
    o("contents", "display-contents"),
];
const JUSTIFY_ROW: &[Opt] = &[
    oa("flex-start", "align-left", &["start", "left", "normal"]),
    o("center", "align-center"),
    oa("flex-end", "align-right", &["end", "right"]),
    o("space-between", "align-between"),
    o("space-around", "align-around"),
    o("space-evenly", "align-evenly"),
];
const JUSTIFY_COL: &[Opt] = &[
    oa("flex-start", "align-top", &["start", "normal"]),
    o("center", "align-center"),
    oa("flex-end", "align-bottom", &["end"]),
    o("space-between", "align-between"),
    o("space-around", "align-around"),
    o("space-evenly", "align-evenly"),
];
const ALIGN_ROW: &[Opt] = &[
    oa("flex-start", "align-top", &["start", "self-start"]),
    o("center", "align-center"),
    oa("flex-end", "align-bottom", &["end", "self-end"]),
    oa("stretch", "align-stretch", &["normal"]),
    o("baseline", "align-baseline"),
];
const ALIGN_COL: &[Opt] = &[
    oa("flex-start", "align-left", &["start", "self-start"]),
    o("center", "align-center"),
    oa("flex-end", "align-right", &["end", "self-end"]),
    oa("stretch", "align-stretch", &["normal"]),
    o("baseline", "align-baseline"),
];
const GRID_X: &[Opt] = &[
    o("start", "align-left"),
    o("center", "align-center"),
    o("end", "align-right"),
    oa("stretch", "align-stretch", &["normal"]),
];
const GRID_Y: &[Opt] = &[
    o("start", "align-top"),
    o("center", "align-center"),
    o("end", "align-bottom"),
    oa("stretch", "align-stretch", &["normal"]),
];
const FLEX_CHILD: &[Opt] = &[
    oa("0 1 auto", "flex-shrink", &["initial", "0 1 auto"]),
    oa(
        "1 1 auto",
        "flex-grow",
        &["auto", "1", "1 1 0%", "1 1 0px", "1 1 0"],
    ),
    oa("0 0 auto", "flex-none", &["none"]),
];

/// Position (0 début, 1 centre, 2 fin) sur un axe ; `None` : réparti ou étiré.
fn axis_pos(v: &str) -> Option<usize> {
    match v.trim() {
        "flex-start" | "start" | "left" | "self-start" | "normal" => Some(0),
        "center" => Some(1),
        "flex-end" | "end" | "right" | "self-end" => Some(2),
        _ => None,
    }
}

const POS_VALUES: [&str; 3] = ["flex-start", "center", "flex-end"];

/// Boîte d'alignement 3 × 3 d'une pile : un clic règle la répartition et l'alignement.
#[component]
fn AlignBox(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let column = Memo::new(move |_| sx.get("flex-direction").current().starts_with("column"));
    let jc = Memo::new(move |_| sx.get("justify-content").current());
    let ai = Memo::new(move |_| {
        let v = sx.get("align-items").current();
        // `normal` aligne les éléments d'une pile comme `stretch`.
        if v == "normal" {
            "stretch".to_string()
        } else {
            v
        }
    });
    let cell = move |c: usize, r: usize| {
        let (main, cross) = if column.get() { (r, c) } else { (c, r) };
        let j = axis_pos(&jc.get());
        let a = axis_pos(&ai.get());
        (j.is_none() || j == Some(main)) && (a.is_none() || a == Some(cross))
    };
    view! {
        <div class="align-box" role="group" aria-label=ui.t("styles-align")>
            {(0..3).flat_map(|r| (0..3).map(move |c| (c, r))).map(|(c, r)| view! {
                <button class="align-cell" class:is-on=move || cell(c, r)
                    title=move || ui.tx("align-cell", &[("col", (c + 1).to_string()), ("row", (r + 1).to_string())])
                    on:click=move |_| {
                        let (main, cross) = if column.get_untracked() { (r, c) } else { (c, r) };
                        sx.set_many(vec![
                            ("justify-content".into(), Some(POS_VALUES[main].into())),
                            ("align-items".into(), Some(POS_VALUES[cross].into())),
                        ], false);
                    }>
                    <i></i>
                </button>
            }).collect_view()}
        </div>
    }
}

/// Direction d'une pile : horizontale ou verticale, ordre inversé.
#[component]
fn Direction(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get("flex-direction"));
    let cur = move || pv.get().current();
    let reversed = move || cur().ends_with("-reverse");
    let column = move || cur().starts_with("column");
    let here = move || pv.get().here.is_some();
    let set = move |col: bool, rev: bool| {
        let v = format!(
            "{}{}",
            if col { "column" } else { "row" },
            if rev { "-reverse" } else { "" }
        );
        sx.set("flex-direction", Some(v));
    };
    view! {
        <div class="ctl-line">
            <div class="seg" role="radiogroup">
                <button role="radio" class:is-on=move || here() && !column() class:is-implied=move || !here() && !column()
                    title=ui.t("dir-horizontal") aria-label=ui.t("dir-horizontal")
                    on:click=move |_| set(false, reversed())>{icons::arrow_right()}<span>{ui.t("dir-horizontal")}</span></button>
                <button role="radio" class:is-on=move || here() && column() class:is-implied=move || !here() && column()
                    title=ui.t("dir-vertical") aria-label=ui.t("dir-vertical")
                    on:click=move |_| set(true, reversed())>{icons::arrow_down()}<span>{ui.t("dir-vertical")}</span></button>
            </div>
            <button class="tool-toggle" class:is-on=reversed aria-pressed=move || reversed().to_string()
                title=ui.t("dir-reverse") aria-label=ui.t("dir-reverse")
                on:click=move |_| set(column(), !reversed())>{icons::reverse()}</button>
        </div>
    }
}

/// Variantes « en ligne » de l'affichage, dans une liste compacte.
#[component]
fn DisplayMore(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get("display"));
    view! {
        <div class="select select--quiet">
            <select class="input" aria-label=ui.t("display-more") on:change=move |e| {
                let v = event_target_value(&e);
                if !v.is_empty() { sx.set("display", Some(v)); }
            }>
                <option value="" selected=move || !DISPLAY_MORE.iter().any(|o| matches(o, &pv.get().current()))>{ui.t("display-more")}</option>
                {DISPLAY_MORE.iter().map(|opt| {
                    let opt = *opt;
                    view! { <option value=opt.v selected=move || matches(&opt, &pv.get().current())>{ui.t(opt.key)}</option> }
                }).collect_view()}
            </select>
        </div>
    }
}

/// Nombre de colonnes d'une grille (`repeat(N, minmax(0, 1fr))`) ; autre forme : texte.
#[component]
fn GridColumns(sx: Sx) -> impl IntoView {
    let pv = Memo::new(move |_| sx.get("grid-template-columns"));
    let count = |v: String| grid_count(&v).map(|n| n.to_string()).unwrap_or(v);
    view! {
        <NumBox sx
            value=Signal::derive(move || pv.get().here.map(count))
            hint=Signal::derive(move || count(pv.get().hint()))
            start=Signal::derive(move || pv.get().here.and_then(|v| grid_count(&v)).unwrap_or(1).to_string())
            on_preview=Callback::new(move |v: String| {
                if let Ok(n) = v.parse::<u32>() {
                    sx.preview(&[("grid-template-columns".into(), grid_columns(n.max(1)))]);
                }
            })
            on_commit=Callback::new(move |(v, co): (Option<String>, bool)| {
                let v = v.map(|v| match v.parse::<f64>() {
                    Ok(n) => grid_columns(n.round().max(1.0) as u32),
                    Err(_) => v,
                });
                sx.set_many(vec![("grid-template-columns".into(), v)], co);
            })
            unit="" label="styles-columns"/>
    }
}

#[component]
pub fn LayoutSection(sx: Sx) -> impl IntoView {
    let display = Memo::new(move |_| sx.get("display").current());
    let is_flex = move || display.get().ends_with("flex");
    let is_grid = move || display.get().ends_with("grid");
    let column = Memo::new(move |_| sx.get("flex-direction").current().starts_with("column"));
    let in_stack = move || sx.computed("--rv-parent-display").ends_with("flex");
    view! {
        <Section sx key="styles-layout" open=true
            props=&["display", "flex-direction", "justify-content", "align-items", "gap", "flex-wrap", "grid-template-columns", "flex"]>
            <Row sx props=&["display"] key="styles-display" stack=true>
                <Seg sx prop="display" options=DISPLAY/>
                <DisplayMore sx/>
            </Row>
            <Show when=is_flex>
                <Row sx props=&["flex-direction"] key="styles-direction"><Direction sx/></Row>
                <div class="style-row style-row--align">
                    <Label sx props=&["justify-content", "align-items"] key="styles-align"/>
                    <div class="style-control align-ctl">
                        <AlignBox sx/>
                        <div class="align-lists">
                            <span class="mini-label">{sx.ui.t("align-horizontal")}</span>
                            {move || if column.get() {
                                view! { <Choice sx prop="align-items" options=opts(sx, ALIGN_COL)/> }.into_any()
                            } else {
                                view! { <Choice sx prop="justify-content" options=opts(sx, JUSTIFY_ROW)/> }.into_any()
                            }}
                            <span class="mini-label">{sx.ui.t("align-vertical")}</span>
                            {move || if column.get() {
                                view! { <Choice sx prop="justify-content" options=opts(sx, JUSTIFY_COL)/> }.into_any()
                            } else {
                                view! { <Choice sx prop="align-items" options=opts(sx, ALIGN_ROW)/> }.into_any()
                            }}
                        </div>
                    </div>
                </div>
                <Row sx props=&["gap"] key="styles-gap" scrub=Some(LabelScrub { prop: "gap", unit: "px", step: 1.0, zero: "0px" })>
                    <PropNum sx prop="gap" units=GAP_UNITS label="styles-gap"/>
                </Row>
                <Row sx props=&["flex-wrap"] key="styles-wrap">
                    <Switch sx prop="flex-wrap" on="wrap" off="nowrap"/>
                </Row>
            </Show>
            <Show when=is_grid>
                <Row sx props=&["grid-template-columns"] key="styles-columns"><GridColumns sx/></Row>
                <Row sx props=&["gap"] key="styles-gap" scrub=Some(LabelScrub { prop: "gap", unit: "px", step: 1.0, zero: "0px" })>
                    <PropNum sx prop="gap" units=GAP_UNITS label="styles-gap"/>
                </Row>
                <Row sx props=&["justify-items"] key="align-horizontal"><Choice sx prop="justify-items" options=opts(sx, GRID_X)/></Row>
                <Row sx props=&["align-items"] key="align-vertical"><Choice sx prop="align-items" options=opts(sx, GRID_Y)/></Row>
            </Show>
            <Show when=in_stack>
                <Row sx props=&["flex"] key="styles-flex-child"><Choice sx prop="flex" options=opts(sx, FLEX_CHILD)/></Row>
            </Show>
        </Section>
    }
}

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
                    }>
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
        <div class="bm-box" class:bm-box--outer=inner.is_some()>
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
    view! {
        <Section sx key="styles-spacing" open=true props=&[
            "margin-top", "margin-right", "margin-bottom", "margin-left",
            "padding-top", "padding-right", "padding-bottom", "padding-left",
        ]>
            <SidesBox sx group=&MARGIN key="styles-margin" inner=(&PADDING, "styles-padding")/>
            <p class="style-help">{sx.ui.t("styles-spacing-help")}</p>
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
                <Choice sx prop="position" options=opts(sx, POSITION)/>
            </Row>
            <p class="style-help">{help}</p>
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
                            push(d.value.clone(), first_family(&d.value));
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
            "text-align", "font-style", "text-decoration-line", "text-transform",
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

/// Image de fond : chemin (sans `url()`) ; un dégradé reste en texte.
#[component]
fn BgImage(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get("background-image"));
    let strip = |v: &str| -> String {
        let t = v.trim();
        match t.strip_prefix("url(").and_then(|r| r.strip_suffix(')')) {
            Some(inner) if !t.contains(',') => inner
                .trim()
                .trim_matches(|c| c == '"' || c == '\'')
                .to_string(),
            _ => t.to_string(),
        }
    };
    view! {
        <input class="input" spellcheck="false" placeholder=move || {
                let h = pv.get().hint();
                if h == "none" || h.is_empty() { ui.tx("bg-image-none", &[]) } else { strip(&h) }
            }
            prop:value=move || pv.get().here.map(|v| strip(&v)).unwrap_or_default()
            on:change=move |e| {
                let t = event_target_value(&e).trim().to_string();
                let v = if t.is_empty() {
                    None
                } else if t == "none" || t.contains('(') {
                    Some(t)
                } else {
                    Some(format!("url(\"{}\")", t.replace('"', "%22")))
                };
                sx.set("background-image", v);
            }/>
    }
}

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
    let has_image = move || {
        let v = sx.get("background-image").current();
        !v.is_empty() && v != "none"
    };
    view! {
        <Section sx key="styles-backgrounds" props=&["background-color", "background-image", "background-size", "background-position", "background-repeat"]>
            <Row sx props=&["background-color"] key="styles-background-color">
                <ColorField sx prop="background-color"/>
            </Row>
            <Row sx props=&["background-image"] key="styles-background-image">
                <BgImage sx/>
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
                aria-label=sx.ui.t("styles-opacity")
                on:input=move |e| { if let Ok(v) = event_target_value(&e).parse::<f64>() { sx.preview(&[("opacity".into(), to_css(v))]) } }
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
        <Section sx key="styles-effects" props=&["opacity", "box-shadow", "cursor"]>
            <Row sx props=&["opacity"] key="styles-opacity"><Opacity sx/></Row>
            <Row sx props=&["box-shadow"] key="styles-box-shadow" stack=true><Shadows sx/></Row>
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
                <p class="style-help">{ui.t("styles-code-help")}</p>
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
