//! Éléments construits fermés (D-159) : un seul calque, réglés depuis leurs réglages ;
//! « Décomposer » montre leurs parties (calques ordinaires) pour les modifier une à une.

use crate::Ui;
use crate::editor::Ed;
use leptos::prelude::*;
use revel_edit::{Command, Target};

/// L'élément désigné est un élément construit fermé.
pub fn is_widget_at(ed: Ed, at: &Target) -> bool {
    ed.doc.with_untracked(|d| {
        d.as_ref()
            .and_then(|d| revel_edit::address::node(d, at).ok())
            .is_some_and(revel_model::is_widget)
    })
}

/// Décompose l'élément : ses parties deviennent des calques ordinaires (annulable).
pub fn decompose(ui: Ui, ed: Ed, at: Target) {
    ed.apply(
        ui,
        Command::RemoveAttr {
            at,
            name: revel_model::WIDGET_ATTR.into(),
        },
        false,
    );
}

/// Bouton « Décomposer » en tête des réglages d'un élément construit fermé.
pub fn decompose_button(ui: Ui, ed: Ed, at: &Target) -> AnyView {
    let at = StoredValue::new(at.clone());
    view! {
        <button type="button" class="btn btn--small widget-decompose" data-action="decompose"
            title=ui.t("widget-decompose-help")
            on:click=move |_| decompose(ui, ed, at.get_value())>
            {crate::icons::unlink()}<span>{ui.t("widget-decompose")}</span>
        </button>
    }
    .into_any()
}

/// Premier descendant (élément) qui porte l'attribut `name` : (identifiant, valeur).
fn find_attr(n: &revel_model::Node, name: &str) -> Option<(String, String)> {
    use revel_model::Node;
    match n {
        Node::Element {
            id,
            attrs,
            children,
            ..
        } => {
            if let (Some(id), Some(a)) = (id, attrs.iter().find(|a| a.name == name)) {
                return Some((id.clone(), a.value.clone().unwrap_or_default()));
            }
            children.iter().find_map(|c| find_attr(c, name))
        }
        Node::Each { children, .. } => children.iter().find_map(|c| find_attr(c, name)),
        _ => None,
    }
}

/// Interrupteur fermé : ce qu'il commande (thème, musique, sons) et le texte de son état.
pub fn switch_settings(ui: Ui, ed: Ed, at: &Target, node: &revel_model::Node) -> Option<AnyView> {
    use revel_model::Node;
    let Node::Element { attrs, .. } = node else {
        return None;
    };
    let cur = attrs
        .iter()
        .find(|a| a.name == "data-revel-switch")?
        .value
        .clone()
        .unwrap_or_default();
    let state = find_attr(node, "data-revel-switch-text");
    let owner = at.owner.clone();
    let at = StoredValue::new(at.clone());
    let state_at = StoredValue::new(state.as_ref().map(|(id, _)| Target {
        owner: owner.clone(),
        node: id.clone(),
    }));
    let texts = state.map(|(_, v)| v).unwrap_or_default();
    let (on, off) = texts
        .split_once('|')
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .unwrap_or((texts.clone(), String::new()));
    // Textes par défaut d'une commande (état allumé, éteint).
    let defaults = move |cmd: &str| -> (String, String) {
        let t = ui.now(
            if cmd == "theme" {
                "insert-text-theme-switch"
            } else {
                "insert-text-switch"
            },
            &[],
        );
        let mut p = t.split('|').skip(1);
        (
            p.next().unwrap_or_default().to_string(),
            p.next().unwrap_or_default().to_string(),
        )
    };
    let texts_s = StoredValue::new((on.clone(), off.clone()));
    let write_text = move |a: String, b: String| {
        if let Some(t) = state_at.get_value() {
            ed.apply(
                ui,
                Command::SetAttr {
                    at: t,
                    name: "data-revel-switch-text".into(),
                    value: Some(format!("{a}|{b}")),
                },
                false,
            );
        }
    };
    let choose = move |cmd: &'static str| {
        let prev = cur_of(ed, &at.get_value());
        let mut cmds = vec![Command::SetAttr {
            at: at.get_value(),
            name: "data-revel-switch".into(),
            value: Some(cmd.into()),
        }];
        // Textes de l'état encore ceux par défaut : ceux de la nouvelle commande.
        let (a, b) = texts_s.get_value();
        let was = defaults(if prev == "theme" { "theme" } else { "music" });
        let now = defaults(cmd);
        if (a.clone(), b.clone()) == was
            && was != now
            && let Some(t) = state_at.get_value()
        {
            cmds.push(Command::SetAttr {
                at: t,
                name: "data-revel-switch-text".into(),
                value: Some(format!("{}|{}", now.0, now.1)),
            });
        }
        ed.apply(ui, Command::Batch { commands: cmds }, false);
    };
    let field = move |key: &'static str, value: String, first: bool| {
        view! {
            <label class="field">
                <span class="label">{ui.t(key)}</span>
                <input class="input" prop:value=value on:change=move |e| {
                    let v = event_target_value(&e);
                    let (a, b) = texts_s.get_value();
                    if first { write_text(v, b) } else { write_text(a, v) }
                }/>
            </label>
        }
    };
    Some(
        view! {
            <div class="insp-group" data-role="switch">
                <p class="panel-title" title=ui.t("switch-role-help")>{ui.t("switch-cmd")}</p>
                <div class="seg seg--small" role="radiogroup" aria-label=ui.t("switch-cmd")>
                    {[("theme", "switch-cmd-theme"), ("music", "switch-music"), ("sounds", "switch-sounds")].into_iter().map(|(v, k)| {
                        let on = cur == v;
                        view! {
                            <button role="radio" data-cmd=v class:is-on=on aria-checked=on.to_string()
                                on:click=move |_| if !on { choose(v) }>
                                <span>{ui.t(k)}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
                {state_at.with_value(Option::is_some).then(|| view! {
                    <div class="gap-pair">
                        {field("switch-text-on", on.clone(), true)}
                        {field("switch-text-off", off.clone(), false)}
                    </div>
                })}
            </div>
        }
        .into_any(),
    )
}

/// Commande actuelle de l'interrupteur désigné.
fn cur_of(ed: Ed, at: &Target) -> String {
    ed.doc.with_untracked(|d| {
        d.as_ref()
            .and_then(|d| revel_edit::address::node(d, at).ok())
            .and_then(|n| match n {
                revel_model::Node::Element { attrs, .. } => attrs
                    .iter()
                    .find(|a| a.name == "data-revel-switch")
                    .and_then(|a| a.value.clone()),
                _ => None,
            })
            .unwrap_or_default()
    })
}

/// Images des parties d'un élément fermé (couverture d'un lecteur vidéo) : réglées depuis
/// l'élément, comme une image ordinaire.
pub fn images_settings(ui: Ui, ed: Ed, at: &Target, node: &revel_model::Node) -> Option<AnyView> {
    use revel_model::Node;
    fn collect(n: &Node, top: bool, out: &mut Vec<revel_model::ImageNode>) {
        match n {
            Node::Image(img) => out.push(img.clone()),
            Node::Element { children, .. } if top || revel_model::is_part(n) => {
                for c in children {
                    collect(c, false, out);
                }
            }
            _ => {}
        }
    }
    let mut imgs = Vec::new();
    collect(node, true, &mut imgs);
    if imgs.is_empty() {
        return None;
    }
    let apply = move |cmd: Command| ed.apply(ui, cmd, false);
    let one = imgs.len() == 1;
    Some(
        imgs.into_iter()
            .enumerate()
            .filter_map(|(i, img)| {
                let id = img.id.clone()?;
                let t = Target {
                    owner: at.owner.clone(),
                    node: id,
                };
                let title = if one {
                    ui.now("widget-image", &[])
                } else {
                    format!("{} {}", ui.now("widget-image", &[]), i + 1)
                };
                Some(view! {
                    <div class="insp-group" data-role="widget-image">
                        <p class="panel-title">{title}</p>
                        {crate::inspector::image(ui, ed, apply, t, img)}
                    </div>
                })
            })
            .collect_view()
            .into_any(),
    )
}

/// Filtres de liste fermés : chaque bouton (texte et valeur cherchée dans les étiquettes des
/// éléments filtrés), ajouter, retirer. Le premier bouton sans valeur montre tout.
pub fn filters_settings(ui: Ui, ed: Ed, at: &Target, node: &revel_model::Node) -> Option<AnyView> {
    use revel_model::{Attr, Node};
    let Node::Element { attrs, children, .. } = node else {
        return None;
    };
    let param = attrs
        .iter()
        .find(|a| a.name == "data-revel-filters")?
        .value
        .clone()
        .unwrap_or_default();
    // Boutons : (identifiant, texte, valeur).
    let buttons: Vec<(String, String, String)> = children
        .iter()
        .filter_map(|c| match c {
            Node::Element {
                id: Some(id),
                attrs,
                children,
                ..
            } => {
                let v = attrs.iter().find(|a| a.name == "data-revel-filter")?;
                let text = children
                    .iter()
                    .filter_map(|k| match k {
                        Node::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<String>();
                Some((id.clone(), text.trim().to_string(), v.value.clone().unwrap_or_default()))
            }
            _ => None,
        })
        .collect();
    let owner = at.owner.clone();
    let root = StoredValue::new(at.clone());
    let template = StoredValue::new(children.iter().rev().find_map(|c| match c {
        Node::Element { attrs, .. } if attrs.iter().any(|a| a.name == "data-revel-filter") => Some(c.clone()),
        _ => None,
    }));
    let count = children.len();
    let set_value = move |id: String, v: String| {
        ed.apply(
            ui,
            Command::SetAttr {
                at: Target { owner: root.get_value().owner, node: id },
                name: "data-revel-filter".into(),
                value: Some(v.trim().to_string()),
            },
            false,
        );
    };
    let set_text = move |id: String, v: String| {
        let at = Target { owner: root.get_value().owner, node: id };
        ed.apply(
            ui,
            Command::Replace {
                parent: revel_edit::Parent::of(&at, revel_edit::Branch::Children),
                index: 0,
                node: Node::Text { text: v },
            },
            false,
        );
    };
    let remove = move |id: String| {
        ed.apply(
            ui,
            Command::Remove {
                at: Target { owner: root.get_value().owner, node: id },
            },
            false,
        );
    };
    let add = move || {
        let Some(mut n) = template.get_value() else { return };
        let n_btn = buttons_len(ed, &root.get_value());
        let label = format!("{} {}", ui.now("filters-option", &[]), n_btn);
        if let Node::Element { id, attrs, children, .. } = &mut n {
            *id = None;
            for a in attrs.iter_mut() {
                if a.name == "data-revel-filter" {
                    *a = Attr { name: a.name.clone(), value: Some(format!("option-{n_btn}")), when: None };
                }
            }
            *children = vec![Node::Text { text: label }];
        }
        let at = root.get_value();
        let index = ed.doc.with_untracked(|d| match d.as_ref().and_then(|d| revel_edit::address::node(d, &at).ok()) {
            Some(Node::Element { children, .. }) => children.len(),
            _ => count,
        });
        ed.apply(
            ui,
            Command::Insert {
                parent: revel_edit::Parent::of(&at, revel_edit::Branch::Children),
                index,
                node: n,
            },
            false,
        );
    };
    let _ = owner;
    let set_param = move |v: String| {
        ed.apply(
            ui,
            Command::SetAttr {
                at: root.get_value(),
                name: "data-revel-filters".into(),
                value: Some(v.trim().to_string()),
            },
            false,
        );
    };
    Some(
        view! {
            <div class="insp-group" data-role="filters">
                <div class="panel-title-line">
                    <p class="panel-title" title=ui.t("filters-role-help")>{ui.t("filters-role")}</p>
                    <button type="button" class="icon-btn icon-btn--small" data-action="filter-add"
                        title=ui.t("filters-add") aria-label=ui.t("filters-add") on:click=move |_| add()>{crate::icons::plus()}</button>
                </div>
                {buttons.into_iter().enumerate().map(|(i, (id, text, value))| {
                    let (i1, i2, i3) = (id.clone(), id.clone(), id.clone());
                    let tag = value.clone();
                    view! {
                        <div class="filter-line" data-filter=tag>
                            <input class="input" aria-label=ui.t("filters-text") title=ui.t("filters-text") prop:value=text
                                on:change=move |e| set_text(i1.clone(), event_target_value(&e))/>
                            <input class="input input--mono" aria-label=ui.t("filters-value") title=ui.t("filters-value-help")
                                placeholder=ui.t("filters-all") prop:value=value
                                on:change=move |e| set_value(i2.clone(), event_target_value(&e))/>
                            <button type="button" class="icon-btn icon-btn--small" title=ui.t("filters-remove") aria-label=ui.t("filters-remove")
                                disabled=i == 0 on:click=move |_| remove(i3.clone())>{crate::icons::close()}</button>
                        </div>
                    }
                }).collect_view()}
                <label class="field">
                    <span class="label" title=ui.t("filters-param-help")>{ui.t("filters-param")}</span>
                    <input class="input input--mono" spellcheck="false" prop:value=param
                        on:change=move |e| set_param(event_target_value(&e))/>
                </label>
            </div>
        }
        .into_any(),
    )
}

/// Nombre de boutons d'un groupe de filtres.
fn buttons_len(ed: Ed, at: &Target) -> usize {
    ed.doc.with_untracked(|d| {
        match d.as_ref().and_then(|d| revel_edit::address::node(d, at).ok()) {
            Some(revel_model::Node::Element { children, .. }) => children
                .iter()
                .filter(|c| matches!(c, revel_model::Node::Element { attrs, .. } if attrs.iter().any(|a| a.name == "data-revel-filter")))
                .count(),
            _ => 0,
        }
    })
}
