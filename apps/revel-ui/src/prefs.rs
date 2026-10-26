//! Réglages de Revel pour l'éditeur (D-125) : liens entre variantes, familles de la recherche
//! rapide, couleurs fonctionnelles de l'interface, raccourcis clavier.

use crate::keys::{ACTIONS, Combo};
use crate::settings::update;
use crate::{Ui, icons};
use leptos::prelude::*;

/// Familles de la recherche rapide (Ctrl+K), dans l'ordre de la palette.
pub const PALETTE_GROUPS: &[&str] = &[
    "palette-actions",
    "palette-view",
    "palette-add",
    "palette-pages",
    "palette-screens",
    "palette-layers",
    "palette-goto",
];

/// Éditeur : liens entre variantes (forme, bout), recherche rapide.
#[component]
pub fn EditorPrefs() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let seg = move |field: &'static str, value: &'static str, key: &'static str| {
        let get = move || {
            ui.settings.with(|s| match field {
                "links" => s.variant_links == value,
                _ => s.variant_link_end == value,
            })
        };
        view! {
            <button role="radio" data-value=value class:is-on=get aria-checked=move || get().to_string()
                on:click=move |_| update(ui, &|s| match field {
                    "links" => s.variant_links = value.to_string(),
                    _ => s.variant_link_end = value.to_string(),
                })>{ui.t(key)}</button>
        }
    };
    view! {
        <section class="section settings-card is-titled" data-section="editor">
            <h2>{ui.t("prefs-variants")}</h2>
            <div class="form-grid">
                <span class="label" title=ui.t("prefs-links-help")>{ui.t("prefs-links")}</span>
                <div class="segmented prefs-links" role="radiogroup">
                    {seg("links", "curve", "prefs-links-curve")}
                    {seg("links", "square", "prefs-links-square")}
                    {seg("links", "straight", "prefs-links-straight")}
                </div>
                <span class="label">{ui.t("prefs-link-end")}</span>
                <div class="segmented prefs-link-end" role="radiogroup">
                    {seg("end", "arrow", "prefs-link-end-arrow")}
                    {seg("end", "dot", "prefs-link-end-dot")}
                </div>
            </div>
        </section>
        <section class="section settings-card is-titled" data-section="palette">
            <h2 title=ui.t("prefs-palette-help")>{ui.t("prefs-palette")}</h2>
            <div class="form-grid">
                {PALETTE_GROUPS.iter().map(|g| {
                    let g: &'static str = g;
                    let on = move || ui.settings.with(|s| !s.palette_hidden.iter().any(|x| x == g));
                    view! {
                        <span class="label">{ui.t(g)}</span>
                        <label class="switch" data-group=g>
                            <input type="checkbox" prop:checked=on aria-label=ui.t(g) on:change=move |e| {
                                let show = event_target_checked(&e);
                                update(ui, &|s| {
                                    s.palette_hidden.retain(|x| x != g);
                                    if !show {
                                        s.palette_hidden.push(g.to_string());
                                    }
                                });
                            }/>
                            <span class="switch-track"></span>
                        </label>
                    }
                }).collect_view()}
            </div>
        </section>
    }
}

/// Couleurs fonctionnelles (code couleur de l'interface, jamais remplacé par l'accent) :
/// composants, valeur réglée ici, valeur héritée, traduction automatique.
const COLORS: &[(&str, &str, &str)] = &[
    ("comp", "--comp", "prefs-color-comp"),
    ("set", "--set-text", "prefs-color-set"),
    ("inherit", "--inherit-text", "prefs-color-inherit"),
    ("auto", "--auto-text", "prefs-color-auto"),
];

/// Valeur d'une variable CSS de l'interface en `#rrggbb` (couleur du thème en vigueur).
fn css_hex(var: &str) -> String {
    let v = window()
        .get_computed_style(&document().document_element().expect("racine"))
        .ok()
        .flatten()
        .and_then(|cs| cs.get_property_value(var).ok())
        .unwrap_or_default();
    revel_style::values::Rgba::parse(v.trim())
        .map(|c| c.hex6())
        .unwrap_or_else(|| "#808080".into())
}

#[component]
pub fn UiColors() -> impl IntoView {
    let ui = expect_context::<Ui>();
    view! {
        <section class="section settings-card ui-colors" data-section="ui-colors">
            <h2 title=ui.t("prefs-colors-help")>{ui.t("prefs-colors")}</h2>
            <div class="form-grid">
                {COLORS.iter().map(|(key, var, label)| {
                    let (key, var): (&'static str, &'static str) = (key, var);
                    let custom = move || ui.settings.with(|s| s.ui_colors.get(key).cloned());
                    // Couleur montrée : celle choisie, sinon celle du thème (relue après un changement).
                    let shown = move || {
                        ui.settings.track();
                        custom().unwrap_or_else(|| css_hex(var))
                    };
                    view! {
                        <span class="label">{ui.t(label)}</span>
                        <div class="ui-color-row" data-color=key>
                            <input type="color" class="color-swatch-input" aria-label=ui.t(label) prop:value=shown
                                on:change=move |e| { let v = event_target_value(&e); update(ui, &|s| { s.ui_colors.insert(key.to_string(), v.clone()); }) }/>
                            <span class="ui-color-value">{shown}</span>
                            <Show when=move || custom().is_some()>
                                <button type="button" class="icon-btn icon-btn--small" title=ui.t("prefs-color-reset") aria-label=ui.t("prefs-color-reset")
                                    on:click=move |_| update(ui, &|s| { s.ui_colors.remove(key); })>{icons::reset()}</button>
                            </Show>
                        </div>
                    }
                }).collect_view()}
            </div>
        </section>
    }
}

/// Raccourcis : une ligne par action, ses touches ; « Modifier » enregistre la prochaine
/// combinaison (Échap annule), retirer, revenir aux touches par défaut ; conflit signalé.
#[component]
pub fn Shortcuts() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let recording = RwSignal::new(None::<&'static str>);
    view! {
        <section class="section settings-card shortcuts-card" data-section="shortcuts">
            <div class="shortcut-list" role="list">
                {ACTIONS.iter().map(|a| {
                    let id = a.id;
                    let custom = move || ui.settings.with(|s| s.shortcuts.contains_key(id));
                    let shown = move || {
                        ui.settings.track();
                        crate::keys::bindings(ui, id).iter().map(|b| crate::keys::display_combo(ui, b)).collect::<Vec<_>>()
                    };
                    let conflict = move || {
                        ui.settings.track();
                        let c = crate::keys::conflicts(ui, id);
                        (!c.is_empty()).then(|| c.iter().map(|l| ui.now(l, &[])).collect::<Vec<_>>().join(", "))
                    };
                    view! {
                        <div class="shortcut-row" role="listitem" data-action=id class:is-custom=custom>
                            <span class="shortcut-label">{ui.t(a.label)}</span>
                            <span class="shortcut-keys">
                                {move || if recording.get() == Some(id) {
                                    view! {
                                        <input class="input key-capture" readonly placeholder=ui.t("prefs-key-press")
                                            aria-label=ui.t("prefs-key-press")
                                            on:keydown=move |e: leptos::ev::KeyboardEvent| {
                                                e.prevent_default();
                                                e.stop_propagation();
                                                let k = crate::editor::Key::of(&e);
                                                if k.key == "Escape" && !k.ctrl && !k.shift && !k.alt {
                                                    recording.set(None);
                                                    return;
                                                }
                                                if let Some(c) = Combo::of(&k) {
                                                    let w = c.write();
                                                    update(ui, &|s| { s.shortcuts.insert(id.to_string(), w.clone()); });
                                                    recording.set(None);
                                                }
                                            }
                                            on:blur=move |_| recording.set(None)
                                            node_ref={
                                                let r = NodeRef::<leptos::html::Input>::new();
                                                Effect::new(move |_| if let Some(i) = r.get() { let _ = i.focus(); });
                                                r
                                            }/>
                                    }.into_any()
                                } else {
                                    let list = shown();
                                    if list.is_empty() {
                                        view! { <span class="muted shortcut-none">{ui.t("prefs-key-none")}</span> }.into_any()
                                    } else {
                                        list.into_iter().map(|k| view! { <kbd>{k}</kbd> }).collect_view().into_any()
                                    }
                                }}
                                {move || conflict().map(|c| view! {
                                    <span class="shortcut-conflict" title=ui.tx("prefs-key-conflict", &[("names", c.clone())])
                                        aria-label=ui.tx("prefs-key-conflict", &[("names", c)])>{icons::warning()}</span>
                                })}
                            </span>
                            <span class="shortcut-actions">
                                <button type="button" class="btn btn--small" data-action="record" on:click=move |_| recording.set(Some(id))>{ui.t("prefs-key-change")}</button>
                                <button type="button" class="icon-btn icon-btn--small" title=ui.t("prefs-key-clear") aria-label=ui.t("prefs-key-clear")
                                    on:click=move |_| update(ui, &|s| { s.shortcuts.insert(id.to_string(), String::new()); })>{icons::close()}</button>
                                <Show when=custom>
                                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("prefs-key-reset") aria-label=ui.t("prefs-key-reset")
                                        on:click=move |_| update(ui, &|s| { s.shortcuts.remove(id); })>{icons::reset()}</button>
                                </Show>
                            </span>
                        </div>
                    }
                }).collect_view()}
            </div>
            <div class="shortcut-foot">
                <button type="button" class="btn btn--small" data-action="keys-reset-all"
                    prop:disabled=move || ui.settings.with(|s| s.shortcuts.is_empty())
                    on:click=move |_| update(ui, &|s| s.shortcuts.clear())>{icons::reset()}<span>{ui.t("prefs-key-reset-all")}</span></button>
            </div>
        </section>
    }
}
