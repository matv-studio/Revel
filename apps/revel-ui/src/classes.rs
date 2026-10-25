//! Panneau Classes (D-113) : toutes les classes du site, avec le nombre d'éléments qui les
//! portent et de styles écrits pour elles ; renommer une classe partout, la retirer, retirer
//! d'un coup celles qui ne servent plus ; aller à un élément qui la porte.

use crate::editor::Ed;
use crate::panels::Search;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::classes::{ClassInfo, ClassUse, inventory, remove_commands};
use revel_edit::{Command, Owner};

/// Nom proposé à partir d'une saisie libre (« Titre de carte » → `titre-de-carte`).
pub fn suggest_name(raw: &str) -> String {
    let mut out = String::new();
    for c in raw.trim().chars() {
        let c = c.to_lowercase().next().unwrap_or(c);
        let c = match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-')
        .trim_start_matches('.')
        .to_string()
}

/// Libellé de l'endroit d'un élément : page, gabarit ou composant.
fn place(ui: Ui, ed: Ed, owner: &Owner) -> String {
    match owner {
        Owner::Page(id) => ed
            .pages
            .with_untracked(|ps| {
                ps.iter()
                    .find(|p| p.page.as_deref() == Some(id.as_str()))
                    .map(|p| {
                        if p.title.is_empty() {
                            p.url.clone()
                        } else {
                            p.title.clone()
                        }
                    })
            })
            .unwrap_or_else(|| id.clone()),
        Owner::Layout(id) => ui.now(
            "classes-in-layout",
            &[("name", id.rsplit('/').next().unwrap_or(id).to_string())],
        ),
        Owner::Component(id) => ed
            .doc
            .with_untracked(|d| {
                d.as_ref()
                    .and_then(|d| d.components.iter().find(|c| c.id == *id))
                    .map(crate::names::component_name)
            })
            .unwrap_or_else(|| id.clone()),
    }
}

/// Va à un élément : sa page (ou le canevas de son composant), puis le sélectionne quand il est
/// affiché.
fn go(ui: Ui, ed: Ed, u: &ClassUse) {
    let node = u.at.node.clone();
    let url = match &u.at.owner {
        Owner::Component(c) => {
            ed.open_comp(c, Some((node, u.tag.clone())));
            return;
        }
        Owner::Page(id) => ed.pages.with_untracked(|ps| {
            ps.iter()
                .find(|p| p.page.as_deref() == Some(id.as_str()))
                .map(|p| p.url.clone())
        }),
        Owner::Layout(_) => None,
    };
    // Élément sélectionné quand le canevas montre sa page (tout de suite si elle est affichée).
    let reload = ed.comp.get_untracked().is_some()
        || url.as_ref().is_some_and(|u| ed.page.get_untracked() != *u);
    ui.goto_node.set(Some(node));
    ed.close_comp();
    if let Some(url) = url {
        crate::pagetree::open_page(ed, url);
    }
    if !reload {
        crate::editor::take_goto_node(ui, ed);
    }
}

pub(crate) fn rename(ui: Ui, ed: Ed, from: String) {
    ui.ask.set(Some(crate::dialog::Ask::input(
        ui.now("classes-rename-title", &[("name", from.clone())]),
        ui.now("classes-rename-label", &[]),
        from.clone(),
        ui.now("classes-rename", &[]),
        move |v| {
            let to = suggest_name(&v);
            if !to.is_empty() && to != from {
                ed.apply(
                    ui,
                    Command::RenameClass {
                        from: from.clone(),
                        to,
                    },
                    false,
                );
            }
        },
    )));
}

fn remove(ui: Ui, ed: Ed, info: &ClassInfo) {
    let name = info.name.clone();
    let body = if info.uses.is_empty() {
        ui.now(
            "classes-remove-unused-body",
            &[("count", info.rules.len().to_string())],
        )
    } else {
        ui.now(
            "classes-remove-body",
            &[
                ("uses", info.uses.len().to_string()),
                ("count", info.rules.len().to_string()),
            ],
        )
    };
    ui.ask.set(Some(
        crate::dialog::Ask::confirm(
            ui.now("classes-remove-title", &[("name", name.clone())]),
            Some(body),
            ui.now("action-remove", &[]),
            move || {
                let cmds = ed.doc.with_untracked(|d| {
                    d.as_ref()
                        .map(|d| remove_commands(d, &name).unwrap_or_default())
                        .unwrap_or_default()
                });
                if !cmds.is_empty() {
                    ed.apply(ui, Command::Batch { commands: cmds }, false);
                }
            },
        )
        .danger(),
    ));
}

fn remove_unused(ui: Ui, ed: Ed, names: Vec<String>) {
    let n = names.len();
    ui.ask.set(Some(
        crate::dialog::Ask::confirm(
            ui.now("classes-clean-title", &[("count", n.to_string())]),
            Some(
                names
                    .iter()
                    .map(|x| format!(".{x}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            ui.now("action-remove", &[]),
            move || {
                let cmds: Vec<Command> = ed.doc.with_untracked(|d| {
                    d.as_ref()
                        .map(|d| {
                            names
                                .iter()
                                .flat_map(|x| remove_commands(d, x).unwrap_or_default())
                                .collect()
                        })
                        .unwrap_or_default()
                });
                if !cmds.is_empty() {
                    ed.apply(ui, Command::Batch { commands: cmds }, false);
                }
            },
        )
        .danger(),
    ));
}

/// Panneau Classes.
#[component]
pub fn Classes() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let q = RwSignal::new(String::new());
    let unused_only = RwSignal::new(false);
    let open = RwSignal::new(None::<String>);
    let all = Memo::new(move |_| {
        ed.doc
            .with(|d| d.as_ref().map(inventory).unwrap_or_default())
    });
    let unused = Memo::new(move |_| {
        all.with(|a| {
            a.iter()
                .filter(|c| c.uses.is_empty())
                .map(|c| c.name.clone())
                .collect::<Vec<_>>()
        })
    });
    // Classe de l'élément sélectionné : marquée dans la liste.
    let selected_classes = Memo::new(move |_| {
        let Some(at) = ed.selected_target() else {
            return Vec::new();
        };
        ed.doc.with(|d| {
            d.as_ref()
                .and_then(|d| revel_edit::address::node(d, &at).ok())
                .and_then(|n| match n {
                    revel_model::Node::Element { attrs, .. }
                    | revel_model::Node::Use { attrs, .. } => attrs
                        .iter()
                        .find(|a| a.name == "class")
                        .and_then(|a| a.value.clone()),
                    _ => None,
                })
                .map(|v| v.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default()
        })
    });
    let shown = move || {
        let ql = q.get().trim().trim_start_matches('.').to_lowercase();
        let only = unused_only.get();
        all.with(|a| {
            a.iter()
                .filter(|c| c.name.to_lowercase().contains(&ql) && (!only || c.uses.is_empty()))
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    let row = move |info: ClassInfo| {
        let name = info.name.clone();
        let key = StoredValue::new(name.clone());
        let stored = StoredValue::new(info.clone());
        let is_open = move || open.with(|o| key.with_value(|k| o.as_deref() == Some(k.as_str())));
        let toggle = move |_| {
            let k = key.get_value();
            open.update(|o| {
                *o = if o.as_deref() == Some(k.as_str()) {
                    None
                } else {
                    Some(k)
                }
            });
        };
        let menu = move |e: leptos::ev::MouseEvent| {
            use crate::ctxmenu::{CtxItem, open as open_menu};
            let items = vec![
                CtxItem::new(ui.now("classes-rename", &[]), move || {
                    rename(ui, ed, key.get_value())
                })
                .icon(icons::text),
                CtxItem::new(ui.now("classes-remove", &[]), move || {
                    remove(ui, ed, &stored.get_value())
                })
                .icon(icons::trash)
                .danger()
                .sep(),
            ];
            open_menu(ui, &e, items);
        };
        let n_uses = info.uses.len();
        let n_rules = info.rules.len();
        let title = ui.now(
            "classes-row-title",
            &[("uses", n_uses.to_string()), ("count", n_rules.to_string())],
        );
        let uses = info.uses.clone();
        view! {
            <div class="list-row class-row" data-class=name.clone() title=title
                class:is-open=is_open class:is-unused=n_uses == 0
                class:is-selected=move || selected_classes.with(|s| key.with_value(|k| s.contains(k)))
                on:click=toggle on:contextmenu=menu>
                <span class="list-chevron">{move || if is_open() { icons::chevron_down() } else { icons::chevron_right() }}</span>
                <span class="list-label"><span class="class-chip">{name.clone()}</span></span>
                {(n_uses == 0).then(|| view! { <span class="class-badge" title=ui.t("classes-unused-note")>{ui.t("classes-unused")}</span> })}
                <span class="list-count">{n_uses}</span>
            </div>
            {move || is_open().then(|| {
                let uses = uses.clone();
                view! {
                    <div class="class-detail">
                        {uses.iter().map(|u| {
                            let u2 = u.clone();
                            let label = u.name.clone().unwrap_or_else(|| u.tag.clone());
                            let where_ = place(ui, ed, &u.at.owner);
                            view! {
                                <button class="list-row class-use" style="--depth:1" on:click=move |_| go(ui, ed, &u2)
                                    title=ui.t("classes-go")>
                                    <span class="list-label">{label}</span>
                                    <span class="list-count muted">{where_}</span>
                                </button>
                            }
                        }).collect_view()}
                        <div class="class-actions">
                            <button class="btn btn--small" data-action="rename"
                                on:click=move |_| rename(ui, ed, key.get_value())>{ui.t("classes-rename")}</button>
                            <button class="btn btn--small btn--danger-ghost" data-action="remove"
                                on:click=move |_| remove(ui, ed, &stored.get_value())>{ui.t("classes-remove")}</button>
                        </div>
                    </div>
                }
            })}
        }
    };
    view! {
        <div class="panel-list class-manager">
            <Search value=q key="classes-search"/>
            <div class="list-scroll">
                {move || shown().into_iter().map(row).collect_view()}
                {move || {
                    let empty = all.with(Vec::is_empty);
                    shown().is_empty().then(|| view! {
                        <p class="panel-empty muted">{ui.t(if empty { "classes-none" } else { "classes-no-match" })}</p>
                    })
                }}
            </div>
            <footer class="panel-foot">
                <span class="panel-foot-text muted">{move || ui.tx("classes-count", &[("count", all.with(Vec::len).to_string())])}</span>
                <button class="icon-btn icon-btn--small" data-action="unused-filter"
                    class:is-on=move || unused_only.get() aria-pressed=move || unused_only.get().to_string()
                    title=ui.t("classes-unused-filter") aria-label=ui.t("classes-unused-filter")
                    on:click=move |_| unused_only.update(|v| *v = !*v)>{icons::filter()}</button>
                <button class="icon-btn icon-btn--small" data-action="clean"
                    title=move || ui.tx("classes-clean", &[("count", unused.with(Vec::len).to_string())])
                    aria-label=move || ui.tx("classes-clean", &[("count", unused.with(Vec::len).to_string())])
                    disabled=move || unused.with(Vec::is_empty)
                    on:click=move |_| remove_unused(ui, ed, unused.get_untracked())>{icons::trash()}</button>
            </footer>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::suggest_name;

    #[test]
    fn noms_proposes() {
        assert_eq!(suggest_name("Titre de carte"), "titre-de-carte");
        assert_eq!(suggest_name(".Bouton Principal "), "bouton-principal");
        assert_eq!(suggest_name("Élément_ok"), "element_ok");
    }
}
