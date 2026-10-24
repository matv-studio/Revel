//! Styles › Disposition, repris de Webflow (retours du 17/10/2026) : affichage Bloc, Flex,
//! Grille, Masqué et une flèche pour les autres affichages ; flex : direction, retour à la ligne,
//! matrice d'alignement avec ses listes X et Y, écarts des colonnes et des lignes ; grille :
//! nombre de colonnes et de lignes (largeurs une à une sur demande), sens de remplissage,
//! alignement, écarts, autres alignements. Enfant d'une grille ou d'un flex : section à part
//! (position dans la grille, taille dans le flex, alignement propre, ordre).

use super::controls::*;
use super::ctx::{Sx, menu_id};
use super::fx::GridTracks;
use crate::icons;
use leptos::prelude::*;
use revel_style::values::{Tracks, format_tracks, parse_tracks, split_top};

// ------------------------------------------------------------------------------------------------
// Affichage

const DISPLAY: &[Opt] = &[
    o("block", "display-block"),
    o("flex", "display-flex"),
    o("grid", "display-grid"),
    o("none", "display-none"),
];
/// Autre affichage du menu de la flèche : valeur, libellé, icône.
type MoreDisplay = (&'static str, &'static str, fn() -> AnyView);
const DISPLAY_MORE: &[MoreDisplay] = &[
    (
        "inline-block",
        "display-inline-block",
        icons::display_inline_block,
    ),
    (
        "inline-flex",
        "display-inline-flex",
        icons::display_inline_flex,
    ),
    (
        "inline-grid",
        "display-inline-grid",
        icons::display_inline_grid,
    ),
    ("inline", "display-inline", icons::display_inline),
    ("contents", "display-contents", icons::display_contents),
];

/// Flèche collée à droite des affichages : menu des affichages « en ligne » et « contenu seul ».
#[component]
fn DisplayMore(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let id = menu_id();
    let pv = Memo::new(move |_| sx.get("display"));
    let active = move || {
        let v = pv.with(|p| p.current());
        DISPLAY_MORE
            .iter()
            .find(|(d, _, _)| *d == v.trim())
            .copied()
    };
    view! {
        <div class="display-more">
            <button class="seg-more pop-trigger"
                class:is-on=move || active().is_some() && pv.with(|p| p.here.is_some())
                class:is-implied=move || active().is_some() && pv.with(|p| p.here.is_none())
                aria-haspopup="menu" aria-expanded=move || (sx.menu.get() == Some(id)).to_string()
                title=move || match active() {
                    Some((_, key, _)) => ui.tx(key, &[]),
                    None => ui.tx("display-more", &[]),
                }
                aria-label=ui.t("display-more")
                on:click=move |_| sx.menu.update(|m| *m = if *m == Some(id) { None } else { Some(id) })>
                {icons::chevron_down()}
            </button>
            <Pop sx id narrow=true>
                <div class="pop-menu" role="menu">
                    {DISPLAY_MORE.iter().map(|(v, key, icon)| {
                        let (v, key, icon) = (*v, *key, *icon);
                        view! {
                            <button class="menu-item" role="menuitemradio"
                                class:is-on=move || active().is_some_and(|a| a.0 == v)
                                aria-checked=move || active().is_some_and(|a| a.0 == v).to_string()
                                on:click=move |_| { sx.set("display", Some(v.to_string())); sx.menu.set(None); }>
                                {icon()}<span class="menu-label">{ui.t(key)}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </Pop>
        </div>
    }
}

// ------------------------------------------------------------------------------------------------
// Alignement : matrice 3 × 3 et listes X / Y

/// Position (0 début, 1 centre, 2 fin) sur un axe ; `None` : réparti ou étiré.
fn axis_pos(v: &str) -> Option<usize> {
    match v.trim() {
        "flex-start" | "start" | "left" | "self-start" | "normal" | "top" => Some(0),
        "center" => Some(1),
        "flex-end" | "end" | "right" | "self-end" | "bottom" => Some(2),
        _ => None,
    }
}

/// Matrice d'alignement : `x` et `y` sont les propriétés réglées par colonne et par ligne de la
/// matrice (selon la direction d'un flex), `values` les valeurs écrites (début, centre, fin).
#[component]
fn AlignMatrix(
    sx: Sx,
    #[prop(into)] x: Signal<&'static str>,
    #[prop(into)] y: Signal<&'static str>,
    values: [&'static str; 3],
) -> impl IntoView {
    let ui = sx.ui;
    // `normal` : étiré pour l'alignement des éléments (`align-items`, `justify-items`), au début
    // pour la répartition (`justify-content`).
    let value = move |prop: &str| {
        let v = sx.get(prop).current();
        if v.trim() == "normal" && prop.ends_with("-items") {
            "stretch".to_string()
        } else {
            v
        }
    };
    let xv = Memo::new(move |_| value(x.get()));
    let yv = Memo::new(move |_| value(y.get()));
    let on = move |c: usize, r: usize| {
        let (a, b) = (axis_pos(&xv.get()), axis_pos(&yv.get()));
        (a.is_none() || a == Some(c)) && (b.is_none() || b == Some(r))
    };
    // Valeur étirée ou répartie sur un axe : barres dans la matrice (comme Webflow).
    let spread_x = move || axis_pos(&xv.get()).is_none();
    let spread_y = move || axis_pos(&yv.get()).is_none();
    view! {
        <div class="align-box" class:spread-x=spread_x class:spread-y=spread_y role="group" aria-label=ui.t("styles-align")>
            {(0..3).flat_map(|r| (0..3).map(move |c| (c, r))).map(|(c, r)| view! {
                <button class="align-cell" class:is-on=move || on(c, r)
                    title=move || ui.tx("align-cell", &[("col", (c + 1).to_string()), ("row", (r + 1).to_string())])
                    on:click=move |_| sx.set_many(vec![
                        (x.get_untracked().into(), Some(values[c].into())),
                        (y.get_untracked().into(), Some(values[r].into())),
                    ], false)>
                    <i></i>
                </button>
            }).collect_view()}
        </div>
    }
}

/// Ligne « Aligner » : matrice à gauche, listes X et Y à droite.
#[component]
fn AlignRow(
    sx: Sx,
    #[prop(into)] x: Signal<&'static str>,
    #[prop(into)] y: Signal<&'static str>,
    #[prop(into)] x_opts: Signal<&'static [Opt]>,
    #[prop(into)] y_opts: Signal<&'static [Opt]>,
    values: [&'static str; 3],
    props: &'static [&'static str],
) -> impl IntoView {
    let ui = sx.ui;
    view! {
        <div class="style-row style-row--align">
            <Label sx props key="styles-align"/>
            <div class="style-control align-ctl">
                <AlignMatrix sx x y values/>
                <div class="align-xy">
                    <div class="xy-line" title=ui.t("align-horizontal")>
                        <span class="xy">"X"</span>
                        {move || { let p = x.get(); let o = x_opts.get(); view! { <Choice sx prop=p options=opts(sx, o)/> } }}
                    </div>
                    <div class="xy-line" title=ui.t("align-vertical")>
                        <span class="xy">"Y"</span>
                        {move || { let p = y.get(); let o = y_opts.get(); view! { <Choice sx prop=p options=opts(sx, o)/> } }}
                    </div>
                </div>
            </div>
        </div>
    }
}

const FLEX_X: &[Opt] = &[
    oa("flex-start", "align-left", &["start", "left", "normal"]),
    o("center", "align-center"),
    oa("flex-end", "align-right", &["end", "right"]),
    o("space-between", "align-between"),
    o("space-around", "align-around"),
    o("space-evenly", "align-evenly"),
];
const FLEX_Y_CROSS: &[Opt] = &[
    oa("flex-start", "align-top", &["start", "self-start"]),
    o("center", "align-center"),
    oa("flex-end", "align-bottom", &["end", "self-end"]),
    oa("stretch", "align-stretch", &["normal"]),
    o("baseline", "align-baseline"),
];
const FLEX_X_CROSS: &[Opt] = &[
    oa("flex-start", "align-left", &["start", "self-start"]),
    o("center", "align-center"),
    oa("flex-end", "align-right", &["end", "self-end"]),
    oa("stretch", "align-stretch", &["normal"]),
    o("baseline", "align-baseline"),
];
const FLEX_Y: &[Opt] = &[
    oa("flex-start", "align-top", &["start", "normal"]),
    o("center", "align-center"),
    oa("flex-end", "align-bottom", &["end"]),
    o("space-between", "align-between"),
    o("space-around", "align-around"),
    o("space-evenly", "align-evenly"),
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
    o("baseline", "align-baseline"),
];

// ------------------------------------------------------------------------------------------------
// Écarts : colonnes et lignes, liés par un cadenas

#[component]
fn GapPair(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let row = Memo::new(move |_| sx.get("row-gap"));
    let col = Memo::new(move |_| sx.get("column-gap"));
    let split = RwSignal::new(false);
    let apart = move || split.get() || row.get().current() != col.get().current();
    view! {
        <div class="pair-ctl">
            <div class="pair-fields">
                {move || if apart() {
                    view! {
                        <PropNum sx prop="column-gap" units=GAP_UNITS label="styles-column-gap"/>
                        <PropNum sx prop="row-gap" units=GAP_UNITS label="styles-row-gap"/>
                    }.into_any()
                } else {
                    view! {
                        <PropNum sx prop="gap" units=GAP_UNITS label="styles-column-gap"/>
                        <PropNum sx prop="gap" units=GAP_UNITS label="styles-row-gap"/>
                    }.into_any()
                }}
                <span class="pair-label">{ui.t("gap-columns")}</span>
                <span class="pair-label">{ui.t("gap-rows")}</span>
            </div>
            <button class="tool-toggle" class:is-on=move || !apart() aria-pressed=move || (!apart()).to_string()
                title=ui.t("gap-link") aria-label=ui.t("gap-link")
                on:click=move |_| {
                    if apart() {
                        let v = col.get_untracked().here.or_else(|| sx.get("gap").here);
                        split.set(false);
                        sx.set_many(vec![("row-gap".into(), None), ("column-gap".into(), None), ("gap".into(), v)], false);
                    } else {
                        split.set(true);
                    }
                }>
                {move || if apart() { icons::unlink() } else { icons::link() }}
            </button>
        </div>
    }
}

// ------------------------------------------------------------------------------------------------
// Flex

/// Direction d'un flex : horizontale ou verticale, retour à la ligne, ordre inversé.
#[component]
fn FlexDirection(sx: Sx) -> impl IntoView {
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
                    on:click=move |_| set(false, reversed())>{icons::arrow_right()}</button>
                <button role="radio" class:is-on=move || here() && column() class:is-implied=move || !here() && column()
                    title=ui.t("dir-vertical") aria-label=ui.t("dir-vertical")
                    on:click=move |_| set(true, reversed())>{icons::arrow_down()}</button>
            </div>
            <Toggle sx prop="flex-wrap" on="wrap" off="nowrap" key="styles-wrap" icon=icons::wrap_lines/>
            <button class="tool-toggle" class:is-on=reversed aria-pressed=move || reversed().to_string()
                title=ui.t("dir-reverse") aria-label=ui.t("dir-reverse")
                on:click=move |_| set(column(), !reversed())>{icons::reverse()}</button>
        </div>
    }
}

// ------------------------------------------------------------------------------------------------
// Grille

/// Nombre de pistes d'une valeur `grid-template-*` (`None` : automatique).
fn track_count(v: &str) -> Option<usize> {
    let v = v.trim();
    if v.is_empty() || v == "none" || v == "auto" {
        return None;
    }
    if let Some(t) = parse_tracks(v) {
        return match t {
            Tracks::Equal(n) => Some(n as usize),
            Tracks::List(l) => Some(l.len()),
            Tracks::Auto { .. } => None,
        };
    }
    if let Some(inner) = v.strip_prefix("repeat(")
        && let Some((n, _)) = inner.split_once(',')
    {
        return n.trim().parse().ok();
    }
    Some(split_top(v, ' ').len())
}

/// Nombre de colonnes ou de lignes : champ numérique avec flèches ; vide : automatique.
#[component]
fn TrackCount(sx: Sx, prop: &'static str, label: &'static str) -> impl IntoView {
    let pv = Memo::new(move |_| sx.get(prop));
    let count = move || {
        let p = pv.get();
        track_count(&p.current())
    };
    // Colonnes : largeurs égales (ou liste allongée, raccourcie) ; lignes : hauteur du contenu.
    let to_css = move |n: usize| -> String {
        let n = n.max(1);
        if prop == "grid-template-rows" {
            return format!("repeat({n}, auto)");
        }
        match pv.get_untracked().here.as_deref().and_then(parse_tracks) {
            Some(Tracks::List(mut l)) => {
                l.resize(n, "1fr".into());
                format_tracks(&Tracks::List(l))
            }
            _ => format_tracks(&Tracks::Equal(n as u32)),
        }
    };
    let step = move |d: i64| {
        let n = count().unwrap_or(1) as i64 + d;
        if n >= 1 {
            sx.set(prop, Some(to_css(n as usize)));
        }
    };
    view! {
        <div class="stepper">
            <NumBox sx
                value=Signal::derive(move || pv.get().here.is_some().then(|| count().map(|n| n.to_string()).unwrap_or_else(|| "auto".into())))
                hint=Signal::derive(move || count().map(|n| n.to_string()).unwrap_or_else(|| "auto".into()))
                start=Signal::derive(move || count().unwrap_or(1).to_string())
                on_preview=Callback::new(move |v: String| if let Ok(n) = v.parse::<usize>() { sx.preview(&[(prop.into(), to_css(n))]) })
                on_commit=Callback::new(move |(v, co): (Option<String>, bool)| {
                    let v = v.map(|v| match v.parse::<f64>() {
                        Ok(n) => to_css(n.round().max(1.0) as usize),
                        Err(_) => v,
                    });
                    sx.set_many(vec![(prop.into(), v)], co);
                })
                unit="" label/>
            <div class="stepper-arrows">
                <button tabindex="-1" aria-label="+1" on:click=move |_| step(1)>{icons::chevron_up()}</button>
                <button tabindex="-1" aria-label="-1" on:click=move |_| step(-1)>{icons::chevron_down()}</button>
            </div>
        </div>
    }
}

/// Colonnes et lignes d'une grille ; la clé ouvre les largeurs une à une (colonnes sur mesure ou
/// automatiques).
#[component]
fn GridSize(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let edit = RwSignal::new(false);
    // Colonnes sur mesure ou automatiques : réglage détaillé ouvert d'office.
    let custom = move || {
        sx.get("grid-template-columns")
            .here
            .as_deref()
            .and_then(parse_tracks)
            .is_some_and(|t| !matches!(t, Tracks::Equal(_)))
    };
    let open = move || edit.get() || custom();
    view! {
        <div class="style-row style-row--align" data-prop="grid-template-columns">
            <Label sx props=&["grid-template-columns", "grid-template-rows"] key="styles-grid"/>
            <div class="style-control">
                <div class="pair-ctl">
                    <div class="pair-fields">
                        <TrackCount sx prop="grid-template-columns" label="grid-columns-count"/>
                        <TrackCount sx prop="grid-template-rows" label="grid-rows-count"/>
                        <span class="pair-label">{ui.t("grid-columns-count")}</span>
                        <span class="pair-label">{ui.t("grid-rows-count")}</span>
                    </div>
                    <button class="tool-toggle" class:is-on=open aria-pressed=move || open().to_string()
                        title=ui.t("grid-edit") aria-label=ui.t("grid-edit")
                        on:click=move |_| edit.update(|e| *e = !*e)>{icons::wrench()}</button>
                </div>
                <Show when=open>
                    <GridTracks sx/>
                </Show>
            </div>
        </div>
    }
}

const GRID_FLOW: &[Opt] = &[
    Opt {
        v: "row",
        key: "grid-flow-row",
        icon: Some(icons::arrow_right),
        also: &["row dense"],
    },
    Opt {
        v: "column",
        key: "grid-flow-column",
        icon: Some(icons::arrow_down),
        also: &["column dense"],
    },
];

/// Sens de remplissage d'une grille et remplissage des vides.
#[component]
fn GridFlow(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get("grid-auto-flow"));
    let dense = move || pv.with(|p| p.current().contains("dense"));
    let set_dense = move |on: bool| {
        let base = if pv.with(|p| p.current().starts_with("column")) {
            "column"
        } else {
            "row"
        };
        sx.set(
            "grid-auto-flow",
            Some(if on {
                format!("{base} dense")
            } else {
                base.into()
            }),
        );
    };
    // Maçonnerie (D-110) : attribut de l'élément sélectionné, lu par le script du site.
    let masonry = Memo::new(move |_| {
        sx.ed.selected.with(|_| ());
        sx.ed.doc.with(|_| ());
        sx.ed.selected_node().is_some_and(|(_, n)| {
            matches!(n, revel_model::Node::Element { attrs, .. }
                if attrs.iter().any(|a| a.name == "data-revel-masonry"))
        })
    });
    let set_masonry = move |on: bool| {
        if let Some((at, _)) = sx.ed.selected_node() {
            sx.ed.apply(
                ui,
                revel_edit::Command::SetAttr {
                    at,
                    name: "data-revel-masonry".into(),
                    value: on.then(String::new),
                },
                false,
            );
        }
    };
    view! {
        <div class="ctl-line">
            <Seg sx prop="grid-auto-flow" options=GRID_FLOW/>
            <button class="tool-toggle" class:is-on=dense aria-pressed=move || dense().to_string()
                title=ui.t("grid-dense") aria-label=ui.t("grid-dense")
                on:click=move |_| set_dense(!dense())>{icons::grid()}</button>
            <button class="tool-toggle" data-tool="masonry" class:is-on=move || masonry.get() aria-pressed=move || masonry.get().to_string()
                title=ui.t("grid-masonry-help") aria-label=ui.t("grid-masonry")
                on:click=move |_| set_masonry(!masonry.get_untracked())>{icons::masonry()}</button>
        </div>
    }
}

/// Répartition des colonnes et des lignes dans la grille (`justify-content`, `align-content`).
const CONTENT_X: &[Opt] = &[
    Opt {
        v: "start",
        key: "align-left",
        icon: Some(icons::align_left),
        also: &["normal", "flex-start"],
    },
    Opt {
        v: "center",
        key: "align-center",
        icon: Some(icons::align_center_h),
        also: &[],
    },
    Opt {
        v: "end",
        key: "align-right",
        icon: Some(icons::align_right),
        also: &["flex-end"],
    },
    Opt {
        v: "stretch",
        key: "align-stretch",
        icon: Some(icons::align_stretch_h),
        also: &[],
    },
    Opt {
        v: "space-between",
        key: "align-between",
        icon: Some(icons::space_between_h),
        also: &[],
    },
    Opt {
        v: "space-around",
        key: "align-around",
        icon: Some(icons::space_around_h),
        also: &[],
    },
];
const CONTENT_Y: &[Opt] = &[
    Opt {
        v: "start",
        key: "align-top",
        icon: Some(icons::align_top),
        also: &["normal", "flex-start"],
    },
    Opt {
        v: "center",
        key: "align-center",
        icon: Some(icons::align_middle),
        also: &[],
    },
    Opt {
        v: "end",
        key: "align-bottom",
        icon: Some(icons::align_bottom),
        also: &["flex-end"],
    },
    Opt {
        v: "stretch",
        key: "align-stretch",
        icon: Some(icons::align_stretch_v),
        also: &[],
    },
    Opt {
        v: "space-between",
        key: "align-between",
        icon: Some(icons::space_between_v),
        also: &[],
    },
    Opt {
        v: "space-around",
        key: "align-around",
        icon: Some(icons::space_around_v),
        also: &[],
    },
];

#[component]
pub fn LayoutSection(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let display = Memo::new(move |_| sx.get("display").current());
    let is_flex = move || display.get().ends_with("flex");
    let is_grid = move || display.get().ends_with("grid");
    let column = Memo::new(move |_| sx.get("flex-direction").current().starts_with("column"));
    // Flex horizontal : X répartit (justify-content), Y aligne (align-items) ; vertical : l'inverse.
    let fx = Signal::derive(move || {
        if column.get() {
            "align-items"
        } else {
            "justify-content"
        }
    });
    let fy = Signal::derive(move || {
        if column.get() {
            "justify-content"
        } else {
            "align-items"
        }
    });
    let fxo = Signal::derive(move || if column.get() { FLEX_X_CROSS } else { FLEX_X });
    let fyo = Signal::derive(move || if column.get() { FLEX_Y } else { FLEX_Y_CROSS });
    view! {
        <Section sx key="styles-layout" open=true
            props=&["display", "flex-direction", "flex-wrap", "justify-content", "align-items", "align-content",
                "justify-items", "gap", "row-gap", "column-gap", "grid-template-columns", "grid-template-rows", "grid-auto-flow"]>
            <Row sx props=&["display"] key="styles-display" stack=true>
                <div class="display-ctl">
                    <Seg sx prop="display" options=DISPLAY/>
                    <DisplayMore sx/>
                </div>
            </Row>
            <Show when=is_flex>
                <Row sx props=&["flex-direction", "flex-wrap"] key="styles-direction"><FlexDirection sx/></Row>
                <AlignRow sx x=fx y=fy x_opts=fxo y_opts=fyo values=["flex-start", "center", "flex-end"]
                    props=&["justify-content", "align-items"]/>
                <Row sx props=&["gap", "row-gap", "column-gap"] key="styles-gap" scrub=Some(LabelScrub { prop: "gap", unit: "px", step: 1.0, zero: "0px" })>
                    <GapPair sx/>
                </Row>
            </Show>
            <Show when=is_grid>
                <GridSize sx/>
                <Row sx props=&["grid-auto-flow"] key="styles-direction"><GridFlow sx/></Row>
                <AlignRow sx x="justify-items" y="align-items"
                    x_opts=Signal::derive(|| GRID_X) y_opts=Signal::derive(|| GRID_Y)
                    values=["start", "center", "end"] props=&["justify-items", "align-items"]/>
                <Row sx props=&["gap", "row-gap", "column-gap"] key="styles-gap" scrub=Some(LabelScrub { prop: "gap", unit: "px", step: 1.0, zero: "0px" })>
                    <GapPair sx/>
                </Row>
                <details class="more-options">
                    <summary>{ui.t("align-more")}</summary>
                    <Row sx props=&["justify-content"] key="align-content-columns"><Seg sx prop="justify-content" options=CONTENT_X/></Row>
                    <Row sx props=&["align-content"] key="align-content-rows"><Seg sx prop="align-content" options=CONTENT_Y/></Row>
                </details>
            </Show>
        </Section>
    }
}

// ------------------------------------------------------------------------------------------------
// Enfant d'une grille ou d'un flex

const SELF_Y: &[Opt] = &[
    Opt {
        v: "auto",
        key: "self-auto",
        icon: Some(icons::close),
        also: &["normal"],
    },
    Opt {
        v: "flex-start",
        key: "align-top",
        icon: Some(icons::align_top),
        also: &["start", "self-start"],
    },
    Opt {
        v: "center",
        key: "align-center",
        icon: Some(icons::align_middle),
        also: &[],
    },
    Opt {
        v: "flex-end",
        key: "align-bottom",
        icon: Some(icons::align_bottom),
        also: &["end", "self-end"],
    },
    Opt {
        v: "stretch",
        key: "align-stretch",
        icon: Some(icons::align_stretch_v),
        also: &[],
    },
    Opt {
        v: "baseline",
        key: "align-baseline",
        icon: Some(icons::align_baseline),
        also: &[],
    },
];
const SELF_X: &[Opt] = &[
    Opt {
        v: "auto",
        key: "self-auto",
        icon: Some(icons::close),
        also: &["normal"],
    },
    Opt {
        v: "start",
        key: "align-left",
        icon: Some(icons::align_left),
        also: &["flex-start", "self-start"],
    },
    Opt {
        v: "center",
        key: "align-center",
        icon: Some(icons::align_center_h),
        also: &[],
    },
    Opt {
        v: "end",
        key: "align-right",
        icon: Some(icons::align_right),
        also: &["flex-end", "self-end"],
    },
    Opt {
        v: "stretch",
        key: "align-stretch",
        icon: Some(icons::align_stretch_h),
        also: &[],
    },
];
const FLEX_SIZE: &[Opt] = &[
    oa("0 1 auto", "flex-shrink", &["initial", "0 1 auto"]),
    oa(
        "1 1 auto",
        "flex-grow",
        &["auto", "1", "1 1 0%", "1 1 0px", "1 1 0"],
    ),
    oa("0 0 auto", "flex-none", &["none"]),
];
/// Ordre : aucun, premier, dernier (valeurs de Webflow).
const ORDER: &[Opt] = &[
    Opt {
        v: "0",
        key: "order-auto",
        icon: Some(icons::close),
        also: &["initial"],
    },
    oi("-9999", "order-first", icons::order_first),
    oi("9999", "order-last", icons::order_last),
];

/// Ordre d'un enfant : aucun, premier, dernier ; « … » : un nombre.
#[component]
fn OrderField(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get("order"));
    let custom = RwSignal::new(false);
    let other = move || {
        pv.with(|p| {
            p.here
                .as_deref()
                .is_some_and(|v| !matches!(v.trim(), "0" | "-9999" | "9999" | "initial"))
        })
    };
    let show = move || custom.get() || other();
    view! {
        <div class="stack-tight">
            <div class="ctl-line">
                <Seg sx prop="order" options=ORDER/>
                <button class="tool-toggle" class:is-on=show aria-pressed=move || show().to_string()
                    title=ui.t("order-custom") aria-label=ui.t("order-custom")
                    on:click=move |_| custom.update(|c| *c = !*c)>{icons::more()}</button>
            </div>
            <Show when=show>
                <PropNum sx prop="order" unit="" label="styles-order"/>
            </Show>
        </div>
    }
}

/// Début et fin d'un élément dans la grille (`grid-column: 2 / 4`) : deux champs.
#[component]
fn LineRange(sx: Sx, prop: &'static str, label: &'static str) -> impl IntoView {
    let pv = Memo::new(move |_| sx.get(prop));
    let part = move |i: usize| {
        let v = pv.get().current();
        let mut it = v.split('/').map(|p| p.trim().to_string());
        let (a, b) = (it.next().unwrap_or_default(), it.next().unwrap_or_default());
        let p = if i == 0 { a } else { b };
        if p.is_empty() || p == "auto" {
            None
        } else {
            Some(p)
        }
    };
    let write = move |i: usize, v: Option<String>| {
        let mut a = part(0).unwrap_or_else(|| "auto".into());
        let mut b = part(1).unwrap_or_else(|| "auto".into());
        let v = v.unwrap_or_else(|| "auto".into());
        if i == 0 {
            a = v
        } else {
            b = v
        }
        sx.set(
            prop,
            Some(if b == "auto" { a } else { format!("{a} / {b}") }),
        );
    };
    view! {
        {(0..2).map(|i| view! {
            <NumBox sx
                value=Signal::derive(move || pv.get().here.and(part(i)))
                hint=Signal::derive(move || part(i).unwrap_or_else(|| "auto".into()))
                start=Signal::derive(move || part(i).unwrap_or_else(|| "1".into()))
                on_preview=Callback::new(move |_v: String| {})
                on_commit=Callback::new(move |(v, _): (Option<String>, bool)| write(i, v))
                unit="" label/>
        }).collect_view()}
    }
}

#[component]
fn GridPlacement(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    let col = Memo::new(move |_| sx.get("grid-column"));
    let row = Memo::new(move |_| sx.get("grid-row"));
    // Placé à la main : une ligne de début écrite (`3`, `2 / 4`) ; sinon automatique (`span N`).
    let manual = move || {
        let starts = |v: String| {
            let first = v.split('/').next().unwrap_or_default().trim().to_string();
            !first.is_empty() && first != "auto" && !first.starts_with("span")
        };
        starts(col.get().current()) || starts(row.get().current())
    };
    view! {
        <div class="style-row">
            <Label sx props=&["grid-column", "grid-row"] key="child-position"/>
            <div class="style-control">
                <div class="seg" role="radiogroup">
                    <button role="radio" class:is-on=move || !manual() aria-checked=move || (!manual()).to_string()
                        title=ui.t("child-auto-help") on:click=move |_| if manual() {
                            sx.set_many(vec![("grid-column".into(), None), ("grid-row".into(), None)], false)
                        }><span>{ui.t("child-auto")}</span></button>
                    <button role="radio" class:is-on=manual aria-checked=move || manual().to_string()
                        title=ui.t("child-manual-help") on:click=move |_| if !manual() {
                            sx.set_many(vec![("grid-column".into(), Some("1 / 2".into())), ("grid-row".into(), Some("1 / 2".into()))], false)
                        }><span>{ui.t("child-manual")}</span></button>
                </div>
            </div>
        </div>
        <Show when=manual fallback=move || view! {
            <div class="style-row" data-prop="grid-column">
                <Label sx props=&["grid-column", "grid-row"] key="styles-grid-span"/>
                <div class="style-control">
                    <div class="pair-fields">
                        <super::fx::Span sx prop="grid-column" label="grid-span-columns" prefix="↔"/>
                        <super::fx::Span sx prop="grid-row" label="grid-span-rows" prefix="↕"/>
                        <span class="pair-label">{ui.t("gap-columns")}</span>
                        <span class="pair-label">{ui.t("gap-rows")}</span>
                    </div>
                </div>
            </div>
        }>
            <div class="style-row" data-prop="grid-column">
                <span class="style-label-spacer"></span>
                <div class="style-control">
                    <div class="pair-fields pair-fields--four">
                        <LineRange sx prop="grid-column" label="grid-column-line"/>
                        <LineRange sx prop="grid-row" label="grid-row-line"/>
                        <span class="pair-label pair-label--two" title=ui.t("grid-column-start-end")>{ui.t("gap-columns")}</span>
                        <span class="pair-label pair-label--two" title=ui.t("grid-row-start-end")>{ui.t("gap-rows")}</span>
                    </div>
                </div>
            </div>
        </Show>
    }
}

/// Section « Enfant de la grille » ou « Enfant flex » : seulement quand l'élément est dans une
/// grille ou un flex.
#[component]
pub fn ChildSection(sx: Sx) -> impl IntoView {
    let ui = sx.ui;
    // Mémos : la section n'est redessinée que si le parent change de nature.
    let in_grid = Memo::new(move |_| sx.computed("--rv-parent-display").ends_with("grid"));
    let in_flex = Memo::new(move |_| sx.computed("--rv-parent-display").ends_with("flex"));
    let edit_parent = move |_| {
        sx.ed
            .post(serde_json::json!({ "type": "nav", "dir": "parent" }))
    };
    move || {
        (in_grid.get() || in_flex.get()).then(|| {
        let grid = in_grid.get_untracked();
        view! {
            <Section sx key=if grid { "styles-grid-child" } else { "styles-flex-child" } open=true
                props=&["grid-column", "grid-row", "justify-self", "align-self", "order", "flex"]>
                <button class="btn btn--small parent-edit" on:click=edit_parent>
                    {icons::parent()}{ui.t(if grid { "grid-parent-edit" } else { "flex-parent-edit" })}
                </button>
                {if grid {
                    view! {
                        <GridPlacement sx/>
                        <Row sx props=&["align-self"] key="styles-align-self"><Seg sx prop="align-self" options=SELF_Y/></Row>
                        <Row sx props=&["justify-self"] key="styles-justify-self"><Seg sx prop="justify-self" options=SELF_X/></Row>
                    }.into_any()
                } else {
                    view! {
                        <Row sx props=&["flex"] key="flex-sizing"><Choice sx prop="flex" options=opts(sx, FLEX_SIZE)/></Row>
                        <Row sx props=&["align-self"] key="styles-align-self"><Seg sx prop="align-self" options=SELF_Y/></Row>
                    }.into_any()
                }}
                <Row sx props=&["order"] key="styles-order" scrub=Some(LabelScrub { prop: "order", unit: "", step: 1.0, zero: "0" })>
                    <OrderField sx/>
                </Row>
            </Section>
        }
    })
    }
}
