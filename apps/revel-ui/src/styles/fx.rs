//! Réglages sans CSS ajoutés après la recréation du site Alunites (D-73) : colonnes d'une grille
//! (largeurs une à une, colonnes automatiques), écart des lignes et des colonnes, place d'un
//! élément dans une grille ou une pile, mélange, filtres, filtres d'arrière-plan,
//! transformations et transitions.

use super::controls::*;
use super::ctx::Sx;
use crate::icons;
use leptos::prelude::*;
use revel_style::values::{
    Func, Tracks, Transition, format_funcs, format_tracks, format_transitions, length::parse_num,
    parse_funcs, parse_tracks, parse_transitions, span_of,
};

// ------------------------------------------------------------------------------------------------
// Listes de fonctions : filtres, transformations

/// Fonction proposée (filtre, transformation) : nom CSS, libellé, valeur ajoutée, unités.
#[derive(Clone, Copy)]
pub struct FnDef {
    pub name: &'static str,
    pub key: &'static str,
    pub default: &'static str,
    pub unit: &'static str,
    pub units: &'static [UnitOpt],
    pub step: f64,
}

const fn f(
    name: &'static str,
    key: &'static str,
    default: &'static str,
    unit: &'static str,
    units: &'static [UnitOpt],
    step: f64,
) -> FnDef {
    FnDef {
        name,
        key,
        default,
        unit,
        units,
        step,
    }
}

pub const FILTERS: &[FnDef] = &[
    f("blur", "fx-blur", "4px", "px", BLUR_UNITS, 1.0),
    f("brightness", "fx-brightness", "120%", "%", PCT_UNITS, 5.0),
    f("contrast", "fx-contrast", "120%", "%", PCT_UNITS, 5.0),
    f("grayscale", "fx-grayscale", "100%", "%", PCT_UNITS, 5.0),
    f("saturate", "fx-saturate", "150%", "%", PCT_UNITS, 5.0),
    f("sepia", "fx-sepia", "100%", "%", PCT_UNITS, 5.0),
    f("invert", "fx-invert", "100%", "%", PCT_UNITS, 5.0),
    f(
        "hue-rotate",
        "fx-hue-rotate",
        "90deg",
        "deg",
        DEG_UNITS,
        5.0,
    ),
];

pub const TRANSFORMS: &[FnDef] = &[
    f("translateX", "fx-move-x", "10px", "px", MOVE_UNITS, 1.0),
    f("translateY", "fx-move-y", "10px", "px", MOVE_UNITS, 1.0),
    f("rotate", "fx-rotate", "15deg", "deg", DEG_UNITS, 1.0),
    f("scale", "fx-scale", "1.1", "", &[], 0.05),
    f("scaleX", "fx-scale-x", "1.1", "", &[], 0.05),
    f("scaleY", "fx-scale-y", "1.1", "", &[], 0.05),
    f("skewX", "fx-skew-x", "10deg", "deg", DEG_UNITS, 1.0),
    f("skewY", "fx-skew-y", "10deg", "deg", DEG_UNITS, 1.0),
];

fn def_of(defs: &'static [FnDef], name: &str) -> Option<FnDef> {
    defs.iter()
        .find(|d| d.name.eq_ignore_ascii_case(name))
        .copied()
}

/// Liste de fonctions d'une propriété (`filter`, `transform`…) : une ligne par fonction, son
/// réglage en nombre ; forme non reconnue : texte.
#[component]
pub fn FuncList(
    sx: Sx,
    prop: &'static str,
    defs: &'static [FnDef],
    add_key: &'static str,
    none_key: &'static str,
) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get(prop));
    let list = Memo::new(move |_| pv.get().here.and_then(|v| parse_funcs(&v)));
    let raw = move || pv.get().here.is_some() && list.get().is_none();
    let write = move |l: Vec<Func>| sx.set(prop, (!l.is_empty()).then(|| format_funcs(&l)));
    let preview = move |l: Vec<Func>| sx.preview(&[(prop.to_string(), format_funcs(&l))]);
    let edit = move |i: usize, arg: String| {
        let mut l = list.get_untracked().unwrap_or_default();
        if let Some(x) = l.get_mut(i) {
            x.arg = arg;
        }
        l
    };
    let add = move |_| {
        let mut l = list.get_untracked().unwrap_or_default();
        let d = defs
            .iter()
            .find(|d| !l.iter().any(|x| x.name.eq_ignore_ascii_case(d.name)))
            .unwrap_or(&defs[0]);
        l.push(Func {
            name: d.name.into(),
            arg: d.default.into(),
        });
        write(l);
    };
    view! {
        <div class="fx-list">
            <div class="ctl-line ctl-line--end">
                <span class="muted shadow-count">{move || if list.get().is_none_or(|l| l.is_empty()) && !raw() {
                    let h = pv.get().hint();
                    if h.is_empty() || h == "none" { ui.tx(none_key, &[]) } else { ui.tx("fx-inherited", &[]) }
                } else { String::new() }}</span>
                <button class="icon-btn icon-btn--small" title=ui.t(add_key) aria-label=ui.t(add_key) on:click=add>
                    {icons::plus()}
                </button>
            </div>
            <Show when=raw>
                <input class="input input--mono" prop:value=move || pv.get().here.unwrap_or_default()
                    on:change=move |e| { let v = event_target_value(&e); sx.set(prop, (!v.trim().is_empty()).then_some(v)); }/>
            </Show>
            {move || list.get().unwrap_or_default().into_iter().enumerate().map(|(i, fun)| {
                let def = def_of(defs, &fun.name);
                let numeric = def.is_some() && parse_num(&fun.arg).is_some();
                let name = fun.name.clone();
                let value = if let (Some(d), true) = (def, numeric) {
                    view! {
                        <NumBox sx
                            value=Signal::derive(move || list.get().and_then(|l| l.get(i).map(|x| x.arg.clone())))
                            hint=Signal::derive(move || d.default.to_string())
                            start=Signal::derive(move || list.get().and_then(|l| l.get(i).map(|x| x.arg.clone())).unwrap_or_else(|| d.default.into()))
                            on_preview=Callback::new(move |v: String| preview(edit(i, v)))
                            on_commit=Callback::new(move |(v, _): (Option<String>, bool)| write(edit(i, v.unwrap_or_else(|| d.default.into()))))
                            unit=d.unit units=d.units step=d.step label=d.key/>
                    }.into_any()
                } else {
                    view! {
                        <input class="input input--mono" prop:value=fun.arg.clone()
                            on:change=move |e| write(edit(i, event_target_value(&e)))/>
                    }.into_any()
                };
                view! {
                    <div class="fx-row">
                        <div class="select">
                            <select class="input" aria-label=ui.t(add_key) on:change=move |e| {
                                let n = event_target_value(&e);
                                if let Some(d) = def_of(defs, &n) {
                                    let mut l = list.get_untracked().unwrap_or_default();
                                    if let Some(x) = l.get_mut(i) {
                                        x.name = d.name.into();
                                        x.arg = d.default.into();
                                    }
                                    write(l);
                                }
                            }>
                                {def.is_none().then(|| view! { <option value=name.clone() selected=true>{name.clone()}</option> })}
                                {defs.iter().map(|d| {
                                    let on = d.name.eq_ignore_ascii_case(&fun.name);
                                    view! { <option value=d.name selected=on>{ui.t(d.key)}</option> }
                                }).collect_view()}
                            </select>
                        </div>
                        {value}
                        <button class="icon-btn icon-btn--small" title=ui.t("fx-remove") aria-label=ui.t("fx-remove")
                            on:click=move |_| { let mut l = list.get_untracked().unwrap_or_default(); if i < l.len() { l.remove(i); } write(l); }>
                            {icons::trash()}
                        </button>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

// ------------------------------------------------------------------------------------------------
// Transitions

const TRANSITION_PROPS: &[(&str, &str)] = &[
    ("all", "tr-all"),
    ("opacity", "tr-opacity"),
    ("transform", "tr-transform"),
    ("color", "tr-color"),
    ("background-color", "tr-background"),
    ("border-color", "tr-border"),
    ("box-shadow", "tr-shadow"),
    ("filter", "tr-filter"),
    ("width", "tr-width"),
    ("height", "tr-height"),
];
const EASINGS: &[(&str, &str)] = &[
    ("ease", "ease-ease"),
    ("ease-in", "ease-in"),
    ("ease-out", "ease-out"),
    ("ease-in-out", "ease-in-out"),
    ("linear", "ease-linear"),
    ("cubic-bezier(0.16, 1, 0.3, 1)", "ease-expo-out"),
];

/// Transitions : propriété animée, durée, courbe, délai (une ligne par transition).
#[component]
pub fn Transitions(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get("transition"));
    let list = Memo::new(move |_| pv.get().here.and_then(|v| parse_transitions(&v)));
    let raw = move || pv.get().here.is_some() && list.get().is_none();
    let write = move |l: Vec<Transition>| {
        sx.set(
            "transition",
            (!l.is_empty()).then(|| format_transitions(&l)),
        )
    };
    let edit = move |i: usize, f: Box<dyn Fn(&mut Transition)>| {
        let mut l = list.get_untracked().unwrap_or_default();
        if let Some(t) = l.get_mut(i) {
            f(t);
        }
        l
    };
    let select = move |i: usize,
                       cur: String,
                       options: &'static [(&'static str, &'static str)],
                       label: &'static str,
                       put: fn(&mut Transition, String)| {
        let known = options.iter().any(|(v, _)| *v == cur);
        view! {
            <div class="select">
                <select class="input" aria-label=ui.t(label) title=ui.t(label) on:change=move |e| {
                    let v = event_target_value(&e);
                    write(edit(i, Box::new(move |t| put(t, v.clone()))));
                }>
                    {(!known).then(|| view! { <option value=cur.clone() selected=true>{cur.clone()}</option> })}
                    {options.iter().map(|(v, k)| view! { <option value=*v selected=*v == cur>{ui.t(k)}</option> }).collect_view()}
                </select>
            </div>
        }
    };
    let time = move |i: usize,
                     get: fn(&Transition) -> String,
                     put: fn(&mut Transition, String),
                     key: &'static str,
                     prefix: &'static str| {
        view! {
            <NumBox sx
                value=Signal::derive(move || list.get().and_then(|l| l.get(i).map(get)))
                hint=Signal::derive(|| "0ms".to_string())
                start=Signal::derive(move || list.get().and_then(|l| l.get(i).map(get)).unwrap_or_else(|| "0ms".into()))
                on_preview=Callback::new(|_: String| {})
                on_commit=Callback::new(move |(v, _): (Option<String>, bool)| write(edit(i, Box::new(move |t| put(t, v.clone().unwrap_or_else(|| "0ms".into()))))))
                unit="ms" units=TIME_UNITS step=50.0 prefix=Some(prefix) label=key/>
        }
    };
    view! {
        <div class="fx-list">
            <div class="ctl-line ctl-line--end">
                <span class="muted shadow-count">{move || if list.get().is_none_or(|l| l.is_empty()) && !raw() {
                    let h = pv.get().hint();
                    if h.is_empty() || h.starts_with("all 0s") || h == "none" { ui.tx("tr-none", &[]) } else { ui.tx("fx-inherited", &[]) }
                } else { String::new() }}</span>
                <button class="icon-btn icon-btn--small" title=ui.t("tr-add") aria-label=ui.t("tr-add")
                    on:click=move |_| { let mut l = list.get_untracked().unwrap_or_default(); l.push(Transition::default()); write(l); }>
                    {icons::plus()}
                </button>
            </div>
            <Show when=raw>
                <input class="input input--mono" prop:value=move || pv.get().here.unwrap_or_default()
                    on:change=move |e| { let v = event_target_value(&e); sx.set("transition", (!v.trim().is_empty()).then_some(v)); }/>
            </Show>
            {move || list.get().unwrap_or_default().into_iter().enumerate().map(|(i, t)| view! {
                <div class="shadow-card">
                    <div class="ctl-line">
                        {select(i, t.prop.clone(), TRANSITION_PROPS, "tr-property", |t, v| t.prop = v)}
                        <button class="icon-btn icon-btn--small" title=ui.t("tr-remove") aria-label=ui.t("tr-remove")
                            on:click=move |_| { let mut l = list.get_untracked().unwrap_or_default(); if i < l.len() { l.remove(i); } write(l); }>
                            {icons::trash()}
                        </button>
                    </div>
                    <div class="shadow-grid">
                        {time(i, |t| t.duration.clone(), |t, v| t.duration = v, "tr-duration", "⏱")}
                        {time(i, |t| t.delay.clone(), |t, v| t.delay = v, "tr-delay", "+")}
                    </div>
                    {select(i, t.easing.clone(), EASINGS, "tr-easing", |t, v| t.easing = v)}
                </div>
            }).collect_view()}
        </div>
    }
}

// ------------------------------------------------------------------------------------------------
// Grille

/// Mode de réglage des colonnes.
#[derive(Clone, Copy, PartialEq)]
enum TrackMode {
    Equal,
    Custom,
    Auto,
}

/// Colonnes d'une grille : égales (nombre), sur mesure (largeur de chaque colonne : `2fr`,
/// `240px`, `auto`) ou automatiques (autant que la place permet, largeur minimale).
#[component]
pub fn GridTracks(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get("grid-template-columns"));
    // Valeur réglée ici, sinon en vigueur (colonnes calculées en pixels : nombre seul).
    let tracks = Memo::new(move |_| {
        let p = pv.get();
        match &p.here {
            Some(v) => parse_tracks(v),
            None => p.inherited.as_deref().and_then(parse_tracks).or_else(|| {
                let n = revel_style::values::split_top(&p.computed, ' ').len() as u32;
                (n > 0 && p.computed != "none").then_some(Tracks::Equal(n))
            }),
        }
    });
    let raw = move || pv.get().here.is_some() && tracks.get().is_none();
    let mode = move || match tracks.get() {
        Some(Tracks::List(_)) => TrackMode::Custom,
        Some(Tracks::Auto { .. }) => TrackMode::Auto,
        _ => TrackMode::Equal,
    };
    let write = move |t: Tracks| sx.set("grid-template-columns", Some(format_tracks(&t)));
    let count = move || match tracks.get() {
        Some(Tracks::Equal(n)) => n as usize,
        Some(Tracks::List(l)) => l.len(),
        _ => 2,
    };
    let to_mode = move |m: TrackMode| {
        let n = count().max(1);
        write(match m {
            TrackMode::Equal => Tracks::Equal(n as u32),
            TrackMode::Custom => Tracks::List(vec!["1fr".into(); n]),
            TrackMode::Auto => Tracks::Auto {
                min: "240px".into(),
                fit: false,
            },
        });
    };
    let list = move || match tracks.get() {
        Some(Tracks::List(l)) => l,
        _ => Vec::new(),
    };
    let put = move |i: usize, v: Option<String>| {
        let mut l = list();
        match v {
            Some(v) if i < l.len() => l[i] = v,
            None if i < l.len() && l.len() > 1 => {
                l.remove(i);
            }
            _ => {}
        }
        write(Tracks::List(l));
    };
    let seg = move |m: TrackMode, key: &'static str, help: &'static str| {
        view! {
            <button role="radio" class:is-on=move || mode() == m aria-checked=move || (mode() == m).to_string()
                title=ui.t(help) on:click=move |_| if mode() != m { to_mode(m) }>
                <span>{ui.t(key)}</span>
            </button>
        }
    };
    view! {
        <div class="stack-tight">
            <div class="seg seg--small" role="radiogroup" aria-label=ui.t("styles-columns")>
                {seg(TrackMode::Equal, "grid-equal", "grid-equal-help")}
                {seg(TrackMode::Custom, "grid-custom", "grid-custom-help")}
                {seg(TrackMode::Auto, "grid-auto", "grid-auto-help")}
            </div>
            <Show when=raw>
                <input class="input input--mono" prop:value=move || pv.get().here.unwrap_or_default()
                    on:change=move |e| { let v = event_target_value(&e); sx.set("grid-template-columns", (!v.trim().is_empty()).then_some(v)); }/>
            </Show>
            {move || match (mode(), raw()) {
                (_, true) => ().into_any(),
                (TrackMode::Equal, _) => view! {
                    <NumBox sx
                        value=Signal::derive(move || pv.get().here.is_some().then(|| count().to_string()))
                        hint=Signal::derive(move || count().to_string())
                        start=Signal::derive(move || count().to_string())
                        on_preview=Callback::new(move |v: String| if let Ok(n) = v.parse::<u32>() {
                            sx.preview(&[("grid-template-columns".into(), format_tracks(&Tracks::Equal(n.max(1))))]);
                        })
                        on_commit=Callback::new(move |(v, co): (Option<String>, bool)| {
                            let v = v.map(|v| match v.parse::<f64>() {
                                Ok(n) => format_tracks(&Tracks::Equal(n.round().max(1.0) as u32)),
                                Err(_) => v,
                            });
                            sx.set_many(vec![("grid-template-columns".into(), v)], co);
                        })
                        unit="" label="styles-columns" prefix=Some("#")/>
                }.into_any(),
                (TrackMode::Custom, _) => view! {
                    <div class="grid-tracks">
                        {list().into_iter().enumerate().map(|(i, t)| view! {
                            <NumBox sx
                                value=Signal::derive(move || Some(t.clone()))
                                hint=Signal::derive(|| "1fr".to_string())
                                start=Signal::derive(move || list().get(i).cloned().unwrap_or_else(|| "1fr".into()))
                                on_preview=Callback::new(move |v: String| {
                                    let mut l = list();
                                    if i < l.len() { l[i] = v; }
                                    sx.preview(&[("grid-template-columns".into(), format_tracks(&Tracks::List(l)))]);
                                })
                                on_commit=Callback::new(move |(v, _): (Option<String>, bool)| put(i, v))
                                unit="fr" units=TRACK_UNITS step=1.0 label="grid-track"
                                prefix=Some(["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12"].get(i).copied().unwrap_or("·"))/>
                        }).collect_view()}
                        <button class="icon-btn icon-btn--small" title=ui.t("grid-track-add") aria-label=ui.t("grid-track-add")
                            on:click=move |_| { let mut l = list(); l.push("1fr".into()); write(Tracks::List(l)); }>
                            {icons::plus()}
                        </button>
                    </div>
                }.into_any(),
                (TrackMode::Auto, _) => {
                    let (min, fit) = match tracks.get() {
                        Some(Tracks::Auto { min, fit }) => (min, fit),
                        _ => ("240px".to_string(), false),
                    };
                    let min2 = min.clone();
                    view! {
                        <div class="ctl-line">
                            <NumBox sx
                                value=Signal::derive(move || Some(min.clone()))
                                hint=Signal::derive(|| "240px".to_string())
                                start=Signal::derive(move || min2.clone())
                                on_preview=Callback::new(move |v: String| sx.preview(&[("grid-template-columns".into(), format_tracks(&Tracks::Auto { min: v, fit }))]))
                                on_commit=Callback::new(move |(v, _): (Option<String>, bool)| write(Tracks::Auto { min: v.unwrap_or_else(|| "240px".into()), fit }))
                                unit="px" units=GAP_UNITS label="grid-auto-min" prefix=Some("↔")/>
                            <label class="row-switch" title=ui.t("grid-auto-fit-help")>
                                <span class="switch">
                                    <input type="checkbox" prop:checked=fit on:change=move |e| {
                                        let fit = event_target_checked(&e);
                                        if let Some(Tracks::Auto { min, .. }) = tracks.get_untracked() { write(Tracks::Auto { min, fit }) }
                                    }/>
                                    <span class="switch-track"></span>
                                </span>
                                <span>{ui.t("grid-auto-fit")}</span>
                            </label>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Cases occupées dans une grille (`grid-column: span N`).
#[component]
pub fn Span(
    sx: Sx,
    prop: &'static str,
    label: &'static str,
    prefix: &'static str,
) -> impl IntoView {
    let pv = Memo::new(move |_| sx.get(prop));
    let shown = move |v: &str| {
        span_of(v)
            .map(|n| n.to_string())
            .unwrap_or_else(|| v.to_string())
    };
    let to_css = |n: u32| {
        if n <= 1 {
            "auto".to_string()
        } else {
            format!("span {n}")
        }
    };
    view! {
        <NumBox sx
            value=Signal::derive(move || pv.get().here.map(|v| shown(&v)))
            hint=Signal::derive(move || shown(&pv.get().hint()))
            start=Signal::derive(move || pv.get().here.and_then(|v| span_of(&v)).unwrap_or(1).to_string())
            on_preview=Callback::new(move |v: String| if let Ok(n) = v.parse::<u32>() { sx.preview(&[(prop.into(), to_css(n))]) })
            on_commit=Callback::new(move |(v, co): (Option<String>, bool)| {
                let v = v.map(|v| match v.parse::<f64>() {
                    Ok(n) => to_css(n.round().max(1.0) as u32),
                    Err(_) => v,
                });
                sx.set_many(vec![(prop.into(), v)], co);
            })
            unit="" label prefix=Some(prefix)/>
    }
}
