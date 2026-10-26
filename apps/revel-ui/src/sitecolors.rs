//! Couleurs du site et mode sombre (D-115) : Paramètres › Général › Couleurs.
//!
//! Les couleurs du site sont des variables CSS déclarées dans `:root` (`--texte: #0a0a0a`), que
//! les styles utilisent (`color: var(--texte)`). Chacune peut avoir une valeur pour le mode sombre,
//! écrite dans `@media (prefers-color-scheme: dark) { :root { … } }` : le site suit alors le
//! thème de l'appareil du visiteur. Ajouter, régler en clair et en sombre, retirer.

use crate::site::DocSession;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::Command;
use revel_edit::style::{find_rule, style_command};
use revel_model::{Site, StyleItem};
use revel_style::values::color::Rgba;

/// Requête média du mode sombre.
pub const DARK: &str = "(prefers-color-scheme: dark)";

/// Couleurs du site : (variable, valeur claire, valeur sombre).
fn colors(d: &Site) -> Vec<(String, String, Option<String>)> {
    let light = find_rule(d, ":root", &None);
    let dark = find_rule(d, ":root", &Some(DARK.into()));
    let Some(light) = light else {
        return Vec::new();
    };
    light
        .decls
        .iter()
        .filter(|x| x.prop.starts_with("--") && Rgba::parse(&x.value).is_some())
        .map(|x| {
            let dv = dark
                .as_ref()
                .and_then(|r| r.decls.iter().rev().find(|y| y.prop == x.prop))
                .map(|y| y.value.clone());
            (x.prop.clone(), x.value.clone(), dv)
        })
        .collect()
}

/// Nombre d'utilisations d'une variable dans les styles.
fn uses(d: &Site, var: &str) -> usize {
    let needle = format!("var({var}");
    d.style_rules()
        .flat_map(|r| r.decls.iter())
        .filter(|x| x.value.contains(&needle))
        .count()
}

/// Nom lisible d'une variable (`--texte-doux` → « texte doux »).
fn label(var: &str) -> String {
    var.trim_start_matches("--").replace(['-', '_'], " ")
}

fn hex(v: &str) -> String {
    Rgba::parse(v)
        .map(|c| c.hex6())
        .unwrap_or_else(|| "#000000".into())
}

#[component]
pub fn SiteColors(s: DocSession) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let list = Memo::new(move |_| s.doc.with(|d| d.as_ref().map(colors).unwrap_or_default()));
    let has_dark = Memo::new(move |_| {
        s.doc.with(|d| {
            d.as_ref()
                .is_some_and(|d| find_rule(d, ":root", &Some(DARK.into())).is_some())
        })
    });
    // Valeur d'une variable dans le clair (`None`) ou le sombre.
    let set = move |var: String, media: Option<String>, value: Option<String>| {
        let cmd = s.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            style_command(
                find_rule(d, ":root", &media),
                ":root".into(),
                media.clone(),
                vec![(var, value)],
            )
        });
        if let Some(c) = cmd {
            s.apply.run(c);
        }
    };
    let add = move |_| {
        let n = list.with_untracked(Vec::len) + 1;
        ui.ask.set(Some(crate::dialog::Ask::input(
            ui.now("color-site-add", &[]),
            ui.now("color-site-name", &[]),
            ui.now("color-site-default", &[("n", n.to_string())]),
            ui.now("color-site-create", &[]),
            move |name: String| {
                let base = revel_model::slugify(&name);
                let base = if base.is_empty() {
                    "couleur".to_string()
                } else {
                    base
                };
                let taken: Vec<String> = s.doc.with_untracked(|d| {
                    d.as_ref()
                        .map(|d| {
                            d.style_rules()
                                .flat_map(|r| r.decls.iter().map(|x| x.prop.clone()))
                                .filter(|p| p.starts_with("--"))
                                .collect()
                        })
                        .unwrap_or_default()
                });
                let var = (1..)
                    .map(|i| {
                        if i == 1 {
                            format!("--{base}")
                        } else {
                            format!("--{base}-{i}")
                        }
                    })
                    .find(|v| !taken.contains(v))
                    .unwrap_or_else(|| format!("--{base}"));
                set(var, None, Some("#808080".into()));
            },
        )));
    };
    let toggle_dark = move |on: bool| {
        if on {
            // Mode sombre : chaque couleur part de sa valeur claire.
            let decls: Vec<(String, Option<String>)> = list.with_untracked(|l| {
                l.iter()
                    .map(|(v, c, _)| (v.clone(), Some(c.clone())))
                    .collect()
            });
            let cmd = s.doc.with_untracked(|d| {
                let d = d.as_ref()?;
                style_command(
                    find_rule(d, ":root", &Some(DARK.into())),
                    ":root".into(),
                    Some(DARK.into()),
                    decls,
                )
            });
            if let Some(c) = cmd {
                s.apply.run(c);
            }
        } else {
            ui.ask.set(Some(
                crate::dialog::Ask::confirm(
                    ui.now("colors-dark-off-title", &[]),
                    None,
                    ui.now("action-remove", &[]),
                    move || {
                        let ids: Vec<String> = s.doc.with_untracked(|d| {
                            d.as_ref()
                                .and_then(|d| d.stylesheet.as_ref())
                                .map(|sh| {
                                    sh.items
                                        .iter()
                                        .filter_map(|i| match i {
                                            StyleItem::Rule(r)
                                                if r.selector == ":root"
                                                    && r.media.as_deref() == Some(DARK) =>
                                            {
                                                r.id.clone()
                                            }
                                            _ => None,
                                        })
                                        .collect()
                                })
                                .unwrap_or_default()
                        });
                        let commands: Vec<Command> = ids
                            .into_iter()
                            .map(|rule| Command::RemoveRule { rule })
                            .collect();
                        if !commands.is_empty() {
                            s.apply.run(Command::Batch { commands });
                        }
                    },
                )
                .danger(),
            ));
        }
    };
    let remove = move |var: String| {
        let n = s
            .doc
            .with_untracked(|d| d.as_ref().map(|d| uses(d, &var)).unwrap_or(0));
        let v2 = var.clone();
        ui.ask.set(Some(
            crate::dialog::Ask::confirm(
                ui.now("colors-remove-title", &[("name", label(&var))]),
                (n > 0).then(|| ui.now("colors-remove-used", &[("count", n.to_string())])),
                ui.now("action-remove", &[]),
                move || {
                    let cmds: Vec<Command> = s.doc.with_untracked(|d| {
                        let Some(d) = d.as_ref() else {
                            return Vec::new();
                        };
                        [None, Some(DARK.to_string())]
                            .into_iter()
                            .filter_map(|m| {
                                let rule = find_rule(d, ":root", &m)?;
                                // Dernière déclaration de la règle : règle retirée (pas de `:root {}` vide).
                                if rule.decls.iter().all(|x| x.prop == v2) {
                                    return rule
                                        .id
                                        .clone()
                                        .map(|rule| Command::RemoveRule { rule });
                                }
                                style_command(
                                    Some(rule),
                                    ":root".into(),
                                    m.clone(),
                                    vec![(v2.clone(), None)],
                                )
                            })
                            .collect()
                    });
                    if !cmds.is_empty() {
                        s.apply.run(Command::Batch { commands: cmds });
                    }
                },
            )
            .danger(),
        ));
    };
    let swatch = move |var: String,
                       media: Option<String>,
                       value: String,
                       placeholder: Option<String>| {
        let v1 = var.clone();
        let m1 = media.clone();
        let v2 = var.clone();
        let m2 = media.clone();
        let shown = if value.is_empty() {
            placeholder.clone().unwrap_or_default()
        } else {
            value.clone()
        };
        // Transparence gardée : la pastille montre la couleur sur un damier, le choix natif ne règle
        // que la teinte (l'opacité reste celle de la valeur), la glissière règle l'opacité.
        let cur = Rgba::parse(&shown);
        let alpha = cur.map(|c| c.a).unwrap_or(1.0);
        let (v3, m3) = (var.clone(), media.clone());
        let shown2 = shown.clone();
        view! {
            <span class="color-cell">
                <label class="color-cell__swatch" style=format!("--swatch:{}", if shown.is_empty() { "transparent".to_string() } else { shown.clone() })
                    title=ui.t("colors-pick")>
                    <input type="color" class="color-swatch-input" prop:value=hex(&shown)
                        aria-label=label(&var)
                        on:change=move |e| {
                            let picked = Rgba::parse(&event_target_value(&e));
                            let out = match (picked, Rgba::parse(&shown2)) {
                                (Some(mut p), Some(old)) if old.a < 1.0 => { p.a = old.a; p.to_css() }
                                _ => event_target_value(&e),
                            };
                            set(v1.clone(), m1.clone(), Some(out))
                        }/>
                </label>
                <input type="range" class="range range--plain color-cell__alpha" min="0" max="100" step="1"
                    title=ui.t("colors-opacity") aria-label=ui.t("colors-opacity")
                    prop:value=(alpha * 100.0).round().to_string()
                    on:change=move |e| {
                        let Ok(a) = event_target_value(&e).parse::<f64>() else { return };
                        let Some(mut c) = cur else { return };
                        c.a = (a / 100.0).clamp(0.0, 1.0);
                        set(v3.clone(), m3.clone(), Some(if c.a >= 1.0 { c.hex6() } else { c.to_css() }))
                    }/>
                <input class="input input--mono color-value" prop:value=value spellcheck="false"
                    placeholder=placeholder.unwrap_or_default()
                    on:change=move |e| {
                        let v = event_target_value(&e).trim().to_string();
                        let ok = v.is_empty() || Rgba::parse(&v).is_some();
                        if ok { set(v2.clone(), m2.clone(), (!v.is_empty()).then_some(v)) }
                    }/>
            </span>
        }
    };
    view! {
        <section class="section settings-card site-colors" data-section="colors">
            <div class="colors-head">
                <span class="label">{ui.t("colors-name")}</span>
                <span class="label">{ui.t("colors-light")}</span>
                {move || has_dark.get().then(|| view! { <span class="label">{ui.t("colors-dark")}</span> })}
                <span></span>
            </div>
            {move || list.get().into_iter().map(|(var, light, dark)| {
                let dark_on = has_dark.get();
                let rm = var.clone();
                view! {
                    <div class="colors-row" data-var=var.clone() class:has-dark=dark_on>
                        <span class="colors-label" title=var.clone()>{label(&var)}</span>
                        {swatch(var.clone(), None, light.clone(), None)}
                        {dark_on.then(|| swatch(var.clone(), Some(DARK.into()), dark.clone().unwrap_or_default(), Some(light.clone())))}
                        <button class="icon-btn icon-btn--small" title=ui.t("action-remove") aria-label=ui.t("action-remove")
                            on:click=move |_| remove(rm.clone())>{icons::trash()}</button>
                    </div>
                }
            }).collect_view()}
            {move || list.with(Vec::is_empty).then(|| view! { <p class="muted small">{ui.t("colors-none")}</p> })}
            <div class="colors-foot">
                <button class="btn btn--small" data-action="add-color" on:click=add>{icons::plus()}<span>{ui.t("color-site-add")}</span></button>
                <label class="row-switch" title=ui.t("colors-dark-help")>
                    <span class="switch">
                        <input type="checkbox" id="site-dark-colors" prop:checked=move || has_dark.get()
                            on:change=move |e| toggle_dark(event_target_checked(&e))/>
                        <span class="switch-track"></span>
                    </span>
                    <span>{ui.t("colors-dark-mode")}</span>
                </label>
            </div>
        </section>
    }
}
