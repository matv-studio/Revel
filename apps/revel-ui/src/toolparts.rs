//! Parties répétées d'un outil fermé (D-176) : onglets, volets d'un accordéon, liens d'une barre
//! de navigation. Chaque ligne se renomme sur place (le texte de la partie), monte, descend, se
//! retire ; « Ajouter » copie la dernière. Le champ « Texte » générique n'est pas montré pour un
//! outil : ses textes se règlent ici, avec leur nom.

use crate::editor::{Ed, Selection};
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::canvas::Slot;
use revel_edit::{Branch, Command, Owner, Parent, Target};
use revel_model::{Node, Site};

/// Partie d'un outil : élément de la ligne, élément qui porte son nom, valeur du second champ.
#[derive(Clone, PartialEq)]
pub struct Part {
    pub id: String,
    pub tag: String,
    /// Élément dont le premier texte est le nom de la ligne.
    pub label: String,
    pub name: String,
    pub extra: Option<String>,
}

/// Premier texte écrit dans un élément (ou ses descendants) : emplacement et valeur.
pub fn first_text(owner: &Owner, n: &Node) -> Option<(Slot, String)> {
    let Node::Element { id, children, .. } = n else {
        return None;
    };
    for (index, c) in children.iter().enumerate() {
        if let Node::Text { text } = c
            && !text.trim().is_empty()
        {
            return Some((
                Slot::Text {
                    parent: Parent {
                        owner: owner.clone(),
                        node: id.clone(),
                        branch: Branch::Children,
                    },
                    index,
                },
                text.trim().to_string(),
            ));
        }
    }
    children.iter().find_map(|c| first_text(owner, c))
}

/// Texte d'un élément du document, par son identifiant.
pub fn text_of(d: &Site, owner: &Owner, id: &str) -> Option<(Slot, String)> {
    let at = Target {
        owner: owner.clone(),
        node: id.to_string(),
    };
    first_text(owner, revel_edit::address::node(d, &at).ok()?)
}

/// Texte de l'élément `id` remplacé (dans la langue en cours).
pub fn rename(ui: Ui, ed: Ed, owner: &Owner, id: &str, text: String) {
    let slot = ed
        .doc
        .with_untracked(|d| text_of(d.as_ref()?, owner, id).map(|(s, _)| s));
    if let Some(cmd) = slot.and_then(|s| ed.text_command(&s, text)) {
        ed.apply(ui, cmd, false);
    }
}

/// Ligne « nom : texte » d'un outil : le texte d'une de ses parties, réglé au panneau.
pub fn text_row(
    ui: Ui,
    ed: Ed,
    owner: Owner,
    id: String,
    key: &'static str,
    help: Option<&'static str>,
) -> AnyView {
    let owner = StoredValue::new(owner);
    let id = StoredValue::new(id);
    let value = move || {
        ed.doc.with(|d| {
            d.as_ref()
                .and_then(|d| text_of(d, &owner.get_value(), &id.get_value()))
                .map(|(_, t)| t)
                .unwrap_or_default()
        })
    };
    view! {
        <div class="style-row" data-row=key>
            <span class="style-label" title=help.map(|h| ui.t(h))><span class="style-label-text">{ui.t(key)}</span></span>
            <div class="style-control">
                <input class="input" type="text" prop:value=value aria-label=ui.t(key)
                    on:change=move |e| rename(ui, ed, &owner.get_value(), &id.get_value(), event_target_value(&e))/>
            </div>
        </div>
    }
    .into_any()
}

/// Interrupteur d'un réglage d'outil.
pub fn switch_row(ui: Ui, key: &'static str, on: bool, set: impl Fn(bool) + 'static) -> AnyView {
    view! {
        <label class="view-line" data-row=key title=ui.tx(&format!("{key}-help"), &[])>
            <span>{ui.t(key)}</span>
            <span class="switch">
                <input type="checkbox" prop:checked=on on:change=move |e| set(event_target_checked(&e))/>
                <span class="switch-track"></span>
            </span>
        </label>
    }
    .into_any()
}

/// Choix d'un réglage d'outil dans une liste.
pub fn select_row(
    ui: Ui,
    key: &'static str,
    options: Vec<(String, String)>,
    current: String,
    set: impl Fn(String) + 'static,
) -> AnyView {
    view! {
        <div class="style-row" data-row=key>
            <span class="style-label" title=ui.tx(&format!("{key}-help"), &[])><span class="style-label-text">{ui.t(key)}</span></span>
            <div class="style-control">
                <div class="select">
                    <select class="input" aria-label=ui.t(key) on:change=move |e| set(event_target_value(&e))>
                        {options.into_iter().map(|(v, l)| {
                            let on = v == current;
                            view! { <option value=v selected=on>{l}</option> }
                        }).collect_view()}
                    </select>
                </div>
            </div>
        </div>
    }
    .into_any()
}

/// Liste des parties d'un outil. `extra` : second champ de chaque ligne (attribut de l'élément
/// qui porte le nom, clé de son nom affiché). Sans `on_add`, `on_remove` ni `on_move` : copie de
/// la dernière ligne, retrait de la ligne, déplacement parmi les lignes.
#[component]
pub fn PartsList(
    owner: Owner,
    title: &'static str,
    add_key: &'static str,
    parts: Signal<Vec<Part>>,
    icon: fn() -> AnyView,
    #[prop(optional)] extra: Option<(&'static str, &'static str)>,
    #[prop(optional)] on_add: Option<Callback<()>>,
    #[prop(optional)] on_remove: Option<Callback<usize>>,
    #[prop(optional)] on_move: Option<Callback<(usize, i32)>>,
    #[prop(default = 1)] min: usize,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let owner = StoredValue::new(owner);
    let target = move |id: &str| Target {
        owner: owner.get_value(),
        node: id.to_string(),
    };
    let add = move |_| match on_add {
        Some(f) => f.run(()),
        None => {
            if let Some(last) = parts.with_untracked(|l| l.last().map(|p| p.id.clone())) {
                ed.apply_keep(ui, Command::Duplicate { at: target(&last) });
            }
        }
    };
    let remove = move |i: usize| match on_remove {
        Some(f) => f.run(i),
        None => {
            if let Some(id) = parts.with_untracked(|l| l.get(i).map(|p| p.id.clone())) {
                ed.apply(ui, Command::Remove { at: target(&id) }, false);
            }
        }
    };
    let step = move |i: usize, dir: i32| match on_move {
        Some(f) => f.run((i, dir)),
        None => {
            let list = parts.get_untracked();
            let j = i as i32 + dir;
            if j < 0 || j as usize >= list.len() {
                return;
            }
            // Position parmi les enfants (textes compris) de la ligne voisine.
            let cmd = ed.doc.with_untracked(|d| {
                let d = d.as_ref()?;
                let (to, index) =
                    revel_edit::address::locate(d, &target(&list[j as usize].id)).ok()?;
                Some(Command::Move {
                    at: target(&list[i].id),
                    to,
                    index,
                })
            });
            if let Some(c) = cmd {
                ed.apply(ui, c, false);
            }
        }
    };
    view! {
        <div class="items-list parts-list" role="list" aria-label=ui.t(title) data-list=title>
            <div class="items-head">
                <span class="label">{ui.t(title)}</span>
                <span class="muted items-count">{move || parts.with(Vec::len)}</span>
            </div>
            <For each={move || { let l: Vec<(usize, Part)> = parts.get().into_iter().enumerate().collect(); l }}
                key=|(i, p)| (*i, p.id.clone(), p.name.clone(), p.extra.clone()) let:row>
                {
                    let (i, p) = row;
                    let n = move || parts.with(Vec::len);
                    let (id, label) = (StoredValue::new(p.id.clone()), StoredValue::new(p.label.clone()));
                    let (sel_tag, sel_name) = (p.tag.clone(), p.name.clone());
                    let on = move || ed.selected.with(|s| s.as_ref().is_some_and(|s| id.with_value(|i| s.id == *i)));
                    view! {
                        <div class="items-row parts-row" role="listitem" class:is-on=on data-item=p.id.clone()>
                            <button class="items-thumb items-thumb--icon parts-pick" title=ui.t("parts-select") aria-label=ui.t("parts-select")
                                on:click=move |_| {
                                    ed.selected.set(Some(Selection { id: id.get_value(), tag: sel_tag.clone(), label: sel_name.clone() }));
                                    ed.select_and_reveal(&id.get_value());
                                }>{icon()}</button>
                            <input class="parts-name" type="text" prop:value=p.name.clone() aria-label=ui.t("parts-name")
                                on:change=move |e| rename(ui, ed, &owner.get_value(), &label.get_value(), event_target_value(&e))/>
                            {extra.map(|(attr, key)| {
                                let value = p.extra.clone().unwrap_or_default();
                                view! {
                                    <input class="parts-extra mono" type="text" prop:value=value placeholder=ui.t(key)
                                        title=ui.t(key) aria-label=ui.t(key)
                                        on:change=move |e| {
                                            let v = event_target_value(&e).trim().to_string();
                                            ed.apply(ui, Command::SetAttr { at: target(&label.get_value()), name: attr.into(), value: Some(v) }, false);
                                        }/>
                                }
                            })}
                            <span class="items-actions">
                                <button class="icon-btn icon-btn--small" title=ui.t("items-up") aria-label=ui.t("items-up")
                                    prop:disabled={i == 0} on:click=move |_| step(i, -1)>{icons::arrow_up()}</button>
                                <button class="icon-btn icon-btn--small" title=ui.t("items-down") aria-label=ui.t("items-down")
                                    prop:disabled={move || n() <= i + 1} on:click=move |_| step(i, 1)>{icons::arrow_down()}</button>
                                <button class="icon-btn icon-btn--small" title=ui.t("action-remove") aria-label=ui.t("action-remove")
                                    prop:disabled={move || n() <= min} on:click=move |_| remove(i)>{icons::trash()}</button>
                            </span>
                        </div>
                    }
                }
            </For>
            <button class="btn btn--small items-add" data-action=add_key on:click=add>{icons::plus()}<span>{ui.t(add_key)}</span></button>
        </div>
    }
}
