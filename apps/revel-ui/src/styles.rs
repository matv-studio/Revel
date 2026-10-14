//! Panneau de styles (étapes 2.6 et 2.8) : styles détenus par Revel (D-18), modèle Webflow.
//!
//! On règle un **sélecteur** (classe de l'élément, ou combinaison de ses classes), éventuellement un
//! **état** (`:hover`…), au **point de rupture** affiché. La règle correspondante est modifiée
//! (`SetDecl`) ; si elle n'existe pas, la première valeur la crée à sa place dans la cascade
//! (`AddRule`). Les valeurs effectives de l'élément (calculées dans le canevas) servent d'indication.

use crate::editor::Ed;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::Command;
use revel_model::{Decl, Node, StyleItem, StyleRule};

/// Champ d'une propriété : texte libre ou liste de valeurs.
#[derive(Clone, Copy)]
enum Input {
    Text,
    Choice(&'static [&'static str]),
}

/// Propriété proposée : nom CSS, clé de traduction du libellé, saisie.
type Prop = (&'static str, &'static str, Input);

const DISPLAY: &[&str] = &[
    "block",
    "flex",
    "grid",
    "inline-block",
    "inline",
    "inline-flex",
    "none",
];
const POSITION: &[&str] = &["static", "relative", "absolute", "fixed", "sticky"];
const FLEX_DIR: &[&str] = &["row", "column", "row-reverse", "column-reverse"];
const JUSTIFY: &[&str] = &[
    "flex-start",
    "center",
    "flex-end",
    "space-between",
    "space-around",
    "space-evenly",
];
const ALIGN: &[&str] = &["stretch", "flex-start", "center", "flex-end", "baseline"];
const TEXT_ALIGN: &[&str] = &["left", "center", "right", "justify", "start", "end"];
const TRANSFORM: &[&str] = &["none", "uppercase", "lowercase", "capitalize"];
const OVERFLOW: &[&str] = &["visible", "hidden", "auto", "scroll", "clip"];
const BORDER_STYLE: &[&str] = &["none", "solid", "dashed", "dotted", "double"];
const WEIGHT: &[&str] = &[
    "100", "200", "300", "400", "500", "600", "700", "800", "900",
];

/// Sections du panneau.
const SECTIONS: &[(&str, &[Prop])] = &[
    (
        "styles-layout",
        &[
            ("display", "styles-display", Input::Choice(DISPLAY)),
            (
                "flex-direction",
                "styles-direction",
                Input::Choice(FLEX_DIR),
            ),
            ("justify-content", "styles-justify", Input::Choice(JUSTIFY)),
            ("align-items", "styles-align", Input::Choice(ALIGN)),
            ("gap", "styles-gap", Input::Text),
            ("grid-template-columns", "styles-columns", Input::Text),
        ],
    ),
    (
        "styles-size",
        &[
            ("width", "styles-width", Input::Text),
            ("height", "styles-height", Input::Text),
            ("min-width", "styles-min-width", Input::Text),
            ("max-width", "styles-max-width", Input::Text),
            ("min-height", "styles-min-height", Input::Text),
            ("max-height", "styles-max-height", Input::Text),
            ("overflow", "styles-overflow", Input::Choice(OVERFLOW)),
        ],
    ),
    (
        "styles-position",
        &[
            ("position", "styles-position-type", Input::Choice(POSITION)),
            ("top", "styles-top", Input::Text),
            ("right", "styles-right", Input::Text),
            ("bottom", "styles-bottom", Input::Text),
            ("left", "styles-left", Input::Text),
            ("z-index", "styles-z-index", Input::Text),
        ],
    ),
    (
        "styles-typography",
        &[
            ("font-family", "styles-font-family", Input::Text),
            ("font-size", "styles-font-size", Input::Text),
            ("font-weight", "styles-font-weight", Input::Choice(WEIGHT)),
            ("line-height", "styles-line-height", Input::Text),
            ("letter-spacing", "styles-letter-spacing", Input::Text),
            ("color", "styles-color", Input::Text),
            ("text-align", "styles-text-align", Input::Choice(TEXT_ALIGN)),
            (
                "text-transform",
                "styles-text-transform",
                Input::Choice(TRANSFORM),
            ),
            ("text-decoration", "styles-text-decoration", Input::Text),
        ],
    ),
    (
        "styles-backgrounds",
        &[
            ("background-color", "styles-background-color", Input::Text),
            ("background-image", "styles-background-image", Input::Text),
            ("background-size", "styles-background-size", Input::Text),
            (
                "background-position",
                "styles-background-position",
                Input::Text,
            ),
        ],
    ),
    (
        "styles-borders",
        &[
            ("border-width", "styles-border-width", Input::Text),
            (
                "border-style",
                "styles-border-style",
                Input::Choice(BORDER_STYLE),
            ),
            ("border-color", "styles-border-color", Input::Text),
            ("border-radius", "styles-border-radius", Input::Text),
        ],
    ),
    (
        "styles-effects",
        &[
            ("opacity", "styles-opacity", Input::Text),
            ("box-shadow", "styles-box-shadow", Input::Text),
            ("transform", "styles-transform", Input::Text),
            ("transition", "styles-transition", Input::Text),
            ("filter", "styles-filter", Input::Text),
        ],
    ),
];

const MARGIN: [&str; 4] = ["margin-top", "margin-right", "margin-bottom", "margin-left"];
const PADDING: [&str; 4] = [
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
];

/// Propriétés dont le canevas transmet la valeur effective.
pub fn watched() -> Vec<String> {
    let mut v: Vec<String> = SECTIONS
        .iter()
        .flat_map(|(_, props)| props.iter().map(|(p, _, _)| p.to_string()))
        .collect();
    v.extend(MARGIN.iter().chain(PADDING.iter()).map(|p| p.to_string()));
    v
}

/// États proposés (pseudo-classes ajoutées au sélecteur).
const STATES: &[(&str, &str)] = &[
    ("", "styles-state-none"),
    (":hover", "styles-state-hover"),
    (":focus-visible", "styles-state-focus"),
    (":active", "styles-state-active"),
];

/// Classes littérales de l'élément (pas d'expression).
fn classes_of(n: &Node) -> Vec<String> {
    match n {
        Node::Element { attrs, .. } => attrs
            .iter()
            .find(|a| a.name == "class")
            .and_then(|a| a.value.as_deref())
            .filter(|v| !v.contains("{{") && !v.contains("{%"))
            .map(|v| v.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Sélecteurs proposés : chaque classe, puis la combinaison de toutes (classe combinée).
fn candidates(classes: &[String]) -> Vec<String> {
    let mut v: Vec<String> = classes.iter().map(|c| format!(".{c}")).collect();
    if classes.len() > 1 {
        v.push(classes.iter().map(|c| format!(".{c}")).collect());
    }
    v
}

#[component]
pub fn Styles() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let selector = RwSignal::new(String::new());
    let state = RwSignal::new(String::new());

    let node = Memo::new(move |_| {
        let t = ed.selected_target()?;
        ed.doc
            .with(|d| revel_edit::address::node(d.as_ref()?, &t).ok().cloned())
    });
    let classes = Memo::new(move |_| node.get().map(|n| classes_of(&n)).unwrap_or_default());
    // Sélecteur par défaut : la première classe (gardé tant qu'il reste proposé).
    Effect::new(move |_| {
        let c = candidates(&classes.get());
        if !c.contains(&selector.get_untracked()) {
            selector.set(c.first().cloned().unwrap_or_default());
        }
    });
    let media = Memo::new(move |_| {
        let id = ed.breakpoint.get();
        ed.doc.with(|d| {
            d.as_ref()?
                .breakpoints
                .iter()
                .find(|b| b.id == id)
                .and_then(|b| b.media())
        })
    });
    let full_selector = move || format!("{}{}", selector.get(), state.get());
    // Règle du sélecteur au point de rupture affiché.
    let rule = Memo::new(move |_| {
        let sel = full_selector();
        let m = media.get();
        ed.doc.with(|d| {
            d.as_ref()?
                .stylesheet
                .as_ref()?
                .items
                .iter()
                .rev()
                .find_map(|i| match i {
                    StyleItem::Rule(r) if r.selector == sel && r.media == m => Some(r.clone()),
                    _ => None,
                })
        })
    });
    let has_sheet = move || {
        ed.doc
            .with(|d| d.as_ref().is_some_and(|d| d.stylesheet.is_some()))
    };

    // Écrit une propriété (valeur vide : retirée de la règle).
    let set = move |prop: String, value: String| {
        let v = value.trim().to_string();
        let cmd = match rule.get_untracked() {
            Some(r) => {
                let current = r.decls.iter().rev().find(|d| d.prop == prop);
                if v.is_empty() && current.is_none() {
                    return;
                }
                if current.is_some_and(|d| d.value == v && !d.important) {
                    return;
                }
                Command::SetDecl {
                    rule: r.id.clone().unwrap_or_default(),
                    prop,
                    value: (!v.is_empty()).then_some(v),
                    important: false,
                }
            }
            None if v.is_empty() => return,
            None => Command::AddRule {
                selector: full_selector(),
                media: media.get_untracked(),
                decls: vec![Decl {
                    prop,
                    value: v,
                    important: false,
                }],
            },
        };
        ed.apply(ui, cmd, false);
    };
    let declared = move |prop: &str| -> Option<String> {
        rule.with(|r| {
            r.as_ref()?
                .decls
                .iter()
                .rev()
                .find(|d| d.prop == prop)
                .map(|d| d.value.clone())
        })
    };
    let computed = move |prop: &str| {
        ed.computed
            .with(|c| c.get(prop).cloned().unwrap_or_default())
    };

    let bp_label = move || {
        let id = ed.breakpoint.get();
        ed.doc.with(|d| {
            let b = d.as_ref()?.breakpoints.iter().find(|b| b.id == id)?.clone();
            Some(match (b.max_width, b.min_width) {
                (Some(w), _) => ui.tx("editor-bp-max", &[("width", w.to_string())]),
                (_, Some(w)) => ui.tx("editor-bp-min", &[("width", w.to_string())]),
                _ => ui.tx("editor-bp-desktop", &[]),
            })
        })
    };

    // Ajout d'une classe à l'élément (s'il n'en a pas, ou pour une nouvelle classe).
    let add_class = move |name: String| {
        let name = name.trim().trim_start_matches('.').to_string();
        if name.is_empty() || name.contains(char::is_whitespace) {
            return;
        }
        let Some(at) = ed.selected_target() else {
            return;
        };
        let mut list = classes.get_untracked();
        if !list.contains(&name) {
            list.push(name.clone());
        }
        selector.set(format!(".{name}"));
        ed.apply(
            ui,
            Command::SetAttr {
                at,
                name: "class".into(),
                value: Some(list.join(" ")),
            },
            false,
        );
    };

    let field = move |prop: &'static str, key: &'static str, input: Input| {
        let value = move || declared(prop).unwrap_or_default();
        let is_set = move || declared(prop).is_some();
        let control = match input {
            Input::Text => view! {
                <input class="input input--mono" prop:value=value placeholder=move || computed(prop)
                    on:change=move |e| set(prop.to_string(), event_target_value(&e))
                    on:keydown=move |e: leptos::ev::KeyboardEvent| if e.key() == "Enter" { set(prop.to_string(), event_target_value(&e)) }/>
            }.into_any(),
            Input::Choice(options) => view! {
                <div class="select">
                    <select class="input input--mono" on:change=move |e| set(prop.to_string(), event_target_value(&e))>
                        <option value="" selected=move || !is_set()>{move || {
                            let c = computed(prop);
                            if c.is_empty() { "—".to_string() } else { format!("— ({c})") }
                        }}</option>
                        {options.iter().map(|o| view! {
                            <option value=*o selected=move || declared(prop).as_deref() == Some(*o)>{*o}</option>
                        }).collect_view()}
                    </select>
                </div>
            }.into_any(),
        };
        view! {
            <div class="style-row" class:is-set=is_set>
                <span class="style-label" title=prop>{ui.t(key)}</span>
                {control}
                <button class="icon-btn style-reset" title=ui.t("styles-reset") aria-label=ui.t("styles-reset")
                    class:is-hidden=move || !is_set()
                    on:click=move |_| set(prop.to_string(), String::new())>{icons::close()}</button>
            </div>
        }
    };
    let spacing = move |props: &'static [&'static str; 4], key: &'static str| {
        // Nom de la propriété CSS en info-bulle, comme pour les autres lignes (« margin », « padding »).
        let prop_name = props[0].split('-').next().unwrap_or_default();
        view! {
            <div class="style-row style-row--box">
                <span class="style-label" title=prop_name>{ui.t(key)}</span>
                <div class="box-sides">
                    {props.iter().map(|prop| {
                        let prop: &'static str = prop;
                        view! {
                            <input class="input input--mono" class:is-set=move || declared(prop).is_some()
                                title=prop aria-label=prop
                                prop:value=move || declared(prop).unwrap_or_default()
                                placeholder=move || computed(prop)
                                on:change=move |e| set(prop.to_string(), event_target_value(&e))/>
                        }
                    }).collect_view()}
                </div>
            </div>
        }
    };

    view! {
        <div class="styles">
            {move || match node.get() {
                None => view! { <p class="muted inspector-empty">{ui.t("inspector-none")}</p> }.into_any(),
                Some(n) if !matches!(n, Node::Element { .. }) => view! { <p class="muted inspector-empty">{ui.t("styles-elements-only")}</p> }.into_any(),
                Some(_) if !has_sheet() => view! { <p class="muted inspector-empty">{ui.t("error-edit-no-stylesheet")}</p> }.into_any(),
                Some(_) => view! {
                    <div class="stack">
                        <div class="style-target">
                            <label class="field">
                                <span class="label">{ui.t("styles-selector")}</span>
                                <div class="select">
                                    <select class="input input--mono" on:change=move |e| selector.set(event_target_value(&e))>
                                        {move || candidates(&classes.get()).into_iter().map(|c| {
                                            let (c2, v) = (c.clone(), c.clone());
                                            view! { <option value=v selected=move || selector.get() == c2>{c}</option> }
                                        }).collect_view()}
                                    </select>
                                </div>
                            </label>
                            <label class="field">
                                <span class="label">{ui.t("styles-state")}</span>
                                <div class="select">
                                    <select class="input" on:change=move |e| state.set(event_target_value(&e))>
                                        {STATES.iter().map(|(s, k)| view! {
                                            <option value=*s selected=move || state.get() == *s>{ui.t(k)}</option>
                                        }).collect_view()}
                                    </select>
                                </div>
                            </label>
                            <input class="input input--mono" placeholder=ui.t("styles-class-new")
                                on:change=move |e| add_class(event_target_value(&e))/>
                            <p class="style-where muted">
                                {move || {
                                    let sel = full_selector();
                                    if sel.is_empty() {
                                        ui.tx("styles-no-class", &[])
                                    } else {
                                        ui.tx("styles-where", &[("selector", sel), ("breakpoint", bp_label().unwrap_or_default())])
                                    }
                                }}
                                {move || (rule.get().is_none() && !selector.get().is_empty()).then(|| view! { <br/><span>{ui.t("styles-new-rule")}</span> })}
                            </p>
                        </div>
                        <Show when=move || !selector.get().is_empty()>
                            <details class="style-section" open>
                                <summary>{ui.t("styles-spacing")}</summary>
                                {spacing(&MARGIN, "styles-margin")}
                                {spacing(&PADDING, "styles-padding")}
                            </details>
                            {SECTIONS.iter().map(|(title, props)| view! {
                                <details class="style-section" open=*title == "styles-layout" || *title == "styles-typography">
                                    <summary>{ui.t(title)}</summary>
                                    {props.iter().map(|(p, k, i)| field(p, k, *i)).collect_view()}
                                </details>
                            }).collect_view()}
                            <RuleDecls rule=rule set=set/>
                        </Show>
                    </div>
                }.into_any(),
            }}
        </div>
    }
}

/// Toutes les déclarations de la règle (raccourcis, propriétés personnalisées…), modifiables.
#[component]
fn RuleDecls(
    rule: Memo<Option<StyleRule>>,
    set: impl Fn(String, String) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
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
    view! {
        <details class="style-section" open>
            <summary>{ui.t("styles-declarations")}</summary>
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
                            on:click=move |_| set(prop_rm.clone(), String::new())>{icons::trash()}</button>
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
        </details>
    }
}
