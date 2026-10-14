//! Panneau de styles (étapes 2.6, 2.8 ; D-18, D-20) : styles détenus par Revel, modèle Webflow,
//! contrôles visuels et noms simples.
//!
//! On règle une **classe** de l'élément (ou leur combinaison), éventuellement un **état**
//! (survol…), au **point de rupture** affiché. La règle correspondante est modifiée ; si elle
//! n'existe pas, la première valeur la crée à sa place dans la cascade. Chaque valeur indique son
//! origine (réglée ici, héritée d'un écran plus large ou de l'état de base) ; la valeur effective
//! de l'élément (calculée dans le canevas) sert d'indication.

mod color;
mod controls;
mod ctx;
mod sections;

pub use ctx::watched;

use crate::editor::Ed;
use crate::{Ui, icons};
use ctx::{Sx, bp_name};
use leptos::ev;
use leptos::prelude::*;
use revel_edit::Command;
use revel_model::Node;
use sections::*;
use wasm_bindgen::JsCast;

/// États proposés (pseudo-classes ajoutées au sélecteur).
const STATES: &[(&str, &str)] = &[
    ("", "styles-state-none"),
    (":hover", "styles-state-hover"),
    (":active", "styles-state-active"),
    (":focus-visible", "styles-state-focus"),
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
    let sx = Sx::new(ui, ed);
    let selector = sx.selector;
    let state = sx.state;

    let node = Memo::new(move |_| {
        let t = ed.selected_target()?;
        ed.doc
            .with(|d| revel_edit::address::node(d.as_ref()?, &t).ok().cloned())
    });
    let classes = Memo::new(move |_| node.get().map(|n| classes_of(&n)).unwrap_or_default());
    // Classe par défaut : la première (gardée tant qu'elle reste proposée).
    Effect::new(move |_| {
        let c = candidates(&classes.get());
        if !c.contains(&selector.get_untracked()) {
            selector.set(c.first().cloned().unwrap_or_default());
        }
    });
    // Nombre d'éléments de la page qui portent la classe (demandé au canevas).
    Effect::new(move |_| {
        let sel = selector.get();
        let _ = ed.load.get();
        ed.count.set(None);
        if !sel.is_empty() {
            ed.post(serde_json::json!({ "type": "count", "selector": sel }));
        }
    });
    // Mémorisé : le panneau n'est pas reconstruit à chaque modification du document.
    let has_sheet = Memo::new(move |_| {
        ed.doc
            .with(|d| d.as_ref().is_some_and(|d| d.stylesheet.is_some()))
    });
    let bp_label = move || {
        let id = ed.breakpoint.get();
        ed.doc.with(|d| {
            let b = d.as_ref()?.breakpoints.iter().find(|b| b.id == id)?.clone();
            Some(bp_name(ui, &b))
        })
    };

    // Ce qui est affiché : rien de sélectionné (0), pas un élément (1), pas de feuille de style (2),
    // panneau (3). Le panneau reste en place d'un élément à l'autre (sections ouvertes gardées).
    let kind = Memo::new(move |_| match node.get() {
        None => 0u8,
        Some(n) if !matches!(n, Node::Element { .. }) => 1,
        Some(_) if !has_sheet.get() => 2,
        Some(_) => 3,
    });

    // Menus : fermés par un clic ailleurs ou par Échap.
    let on_down = window_event_listener(ev::pointerdown, move |e: web_sys::PointerEvent| {
        if sx.menu.get_untracked().is_none() {
            return;
        }
        let inside = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest(".pop, .pop-trigger").ok().flatten())
            .is_some();
        if !inside {
            sx.menu.set(None);
        }
    });
    let on_key = window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" && sx.menu.get_untracked().is_some() {
            sx.menu.set(None);
        }
    });
    on_cleanup(move || {
        on_down.remove();
        on_key.remove();
        sx.clear_preview();
    });

    // Ajout d'une classe à l'élément.
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

    view! {
        <div class="styles">
            {move || match kind.get() {
                0 => view! { <p class="muted inspector-empty">{ui.t("inspector-none")}</p> }.into_any(),
                1 => view! { <p class="muted inspector-empty">{ui.t("styles-elements-only")}</p> }.into_any(),
                2 => view! { <p class="muted inspector-empty">{ui.t("error-edit-no-stylesheet")}</p> }.into_any(),
                _ => view! {
                    <div class="style-target">
                        <div class="target-line">
                            <span class="label">{ui.t("styles-selector")}</span>
                            <div class="select">
                                <select class="input input--mono" aria-label=ui.t("styles-selector")
                                    on:change=move |e| selector.set(event_target_value(&e))>
                                    {move || candidates(&classes.get()).into_iter().map(|c| {
                                        let (c2, v) = (c.clone(), c.clone());
                                        view! { <option value=v selected=move || selector.get() == c2>{c}</option> }
                                    }).collect_view()}
                                    {move || classes.get().is_empty().then(|| view! { <option value="" selected=true>{ui.t("styles-no-class-short")}</option> })}
                                </select>
                            </div>
                        </div>
                        <div class="target-line">
                            <span class="label">{ui.t("styles-state")}</span>
                            <div class="select">
                                <select class="input" aria-label=ui.t("styles-state") on:change=move |e| state.set(event_target_value(&e))>
                                    {STATES.iter().map(|(s, k)| view! {
                                        <option value=*s selected=move || state.get() == *s>{ui.t(k)}</option>
                                    }).collect_view()}
                                </select>
                            </div>
                        </div>
                        <div class="target-line">
                            <span class="label">{icons::plus()}</span>
                            <input class="input" placeholder=ui.t("styles-class-new") aria-label=ui.t("styles-class-new")
                                on:change=move |e| {
                                    add_class(event_target_value(&e));
                                    if let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) {
                                        t.set_value("");
                                    }
                                }/>
                        </div>
                        <p class="style-where muted">
                            {move || {
                                let sel = sx.full_selector();
                                if sel.is_empty() {
                                    ui.tx("styles-no-class", &[])
                                } else {
                                    let mut t = ui.tx("styles-where", &[("selector", sel), ("breakpoint", bp_label().unwrap_or_default())]);
                                    if let Some((s, n)) = ed.count.get() && s == selector.get() {
                                        t.push_str(" · ");
                                        t.push_str(&ui.tx("styles-count", &[("count", n.to_string())]));
                                    }
                                    t
                                }
                            }}
                        </p>
                        <p class="style-legend muted">
                            <span><i class="dot dot--here"></i>{ui.t("styles-legend-here")}</span>
                            <span><i class="dot dot--inherited"></i>{ui.t("styles-legend-inherited")}</span>
                        </p>
                        {move || (sx.rule.get().is_none() && !selector.get().is_empty()).then(|| view! {
                            <p class="style-where muted">{ui.t("styles-new-rule")}</p>
                        })}
                    </div>
                    <Show when=move || !selector.get().is_empty()>
                        <LayoutSection sx/>
                        <SpacingSection sx/>
                        <SizeSection sx/>
                        <PositionSection sx/>
                        <TextSection sx/>
                        <BackgroundSection sx/>
                        <BorderSection sx/>
                        <EffectsSection sx/>
                        <CodeSection sx/>
                    </Show>
                }.into_any(),
            }}
        </div>
    }
}
