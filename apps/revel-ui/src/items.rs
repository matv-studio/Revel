//! Éléments d'un défilement, d'un carrousel ou d'un diaporama (D-123), réglés depuis l'onglet
//! Styles de l'élément qui les contient : vignette et nom de chaque élément ; remplacer son
//! image, monter, descendre, dupliquer, retirer ; « Ajouter une image » copie le dernier élément
//! avec l'image choisie. Clic sur un élément : il est choisi dans la page.

use crate::editor::{Ed, Selection};
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::canvas::{literal, quote};
use revel_edit::{Branch, Command, Parent, Target};
use revel_model::Node;

/// Élément de la suite : identifiant, balise, image (adresse), nom affiché (vide : « Élément n »).
#[derive(Clone, PartialEq)]
struct Item {
    id: String,
    tag: String,
    /// Adresse de l'image (vide : pas encore choisie).
    src: Option<String>,
    /// L'élément a une image (à remplacer).
    img: bool,
    name: String,
}

fn items_of(n: &Node) -> Vec<Item> {
    let Some(kids) = revel_model::ids::child_lists(n).into_iter().next() else {
        return Vec::new();
    };
    kids.iter()
        .filter_map(|k| {
            let id = revel_model::ids::node_id(k)?.to_string();
            let tag = match k {
                Node::Element { tag, .. } => tag.clone(),
                Node::Image(_) => "img".into(),
                Node::Use { .. } => String::new(),
                _ => return None,
            };
            let img = revel_edit::make::first_image(k);
            let src = img.and_then(|i| literal(&i.src)).filter(|s| !s.is_empty());
            let name = revel_model::layer_name(k)
                .map(str::to_string)
                .or_else(|| (tag == "a").then(|| first_text(k)).flatten())
                .or_else(|| {
                    img.and_then(|i| i.alt.as_deref().and_then(literal))
                        .filter(|a| !a.trim().is_empty())
                })
                .or_else(|| {
                    src.as_deref().map(|s| {
                        let f = s.rsplit('/').next().unwrap_or(s);
                        crate::names::humanize(f.rsplit_once('.').map_or(f, |(a, _)| a))
                    })
                })
                .unwrap_or_default();
            Some(Item {
                id,
                tag,
                src,
                img: img.is_some(),
                name,
            })
        })
        .collect()
}

#[component]
pub fn ItemsList(
    at: Target,
    /// Liens d'un menu : « Ajouter un lien » copie le dernier lien (sinon : image ajoutée).
    #[prop(optional)]
    links: bool,
    /// Nombre d'éléments fixe (image révélée : dessous et dessus) : ni ajout, ni retrait.
    #[prop(optional)]
    fixed: bool,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let at = StoredValue::new(at);
    let items = Memo::new(move |_| {
        let at = at.get_value();
        ed.doc.with(|d| {
            d.as_ref()
                .and_then(|d| revel_edit::address::node(d, &at).ok())
                .map(items_of)
                .unwrap_or_default()
        })
    });
    // Élément sans nom : « Élément n » d'après sa position.
    let named = Memo::new(move |_| {
        items
            .get()
            .into_iter()
            .enumerate()
            .map(|(i, mut it)| {
                if fixed {
                    // Image révélée : dessous, puis dessus.
                    it.name = ui.now(
                        if i == 0 {
                            "reveal-under"
                        } else {
                            "reveal-over"
                        },
                        &[],
                    );
                } else if it.name.is_empty() {
                    it.name = ui.now("items-item", &[("n", (i + 1).to_string())]);
                }
                it
            })
            .collect::<Vec<_>>()
    });
    let base = move || ed.base.get().unwrap_or_default();
    let target = move |id: &str| Target {
        owner: at.get_value().owner,
        node: id.to_string(),
    };
    let select = move |it: &Item| {
        ed.selected.set(Some(Selection {
            id: it.id.clone(),
            tag: it.tag.clone(),
            label: it.name.clone(),
        }));
        ed.select_and_reveal(&it.id);
    };
    let step = move |id: String, dir: i32| {
        let list = items.get_untracked();
        let Some(i) = list.iter().position(|x| x.id == id) else {
            return;
        };
        let j = i as i32 + dir;
        if j < 0 || j as usize >= list.len() {
            return;
        }
        // Position dans la liste des enfants (textes et conditions compris).
        let index = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let (_, k) = revel_edit::address::locate(d, &target(&list[j as usize].id)).ok()?;
            Some(k)
        });
        if let Some(index) = index {
            ed.apply(
                ui,
                Command::Move {
                    at: target(&id),
                    to: Parent::of(&at.get_value(), Branch::Children),
                    index,
                },
                false,
            );
        }
    };
    let replace = move |id: String| {
        crate::library::pick_image(ui, crate::library::media_shelf(ed), move |url| {
            let cmd = ed.doc.with_untracked(|d| {
                let d = d.as_ref()?;
                let n = revel_edit::address::node(d, &target(&id)).ok()?;
                let img = revel_edit::make::first_image(n)?;
                let mut i = img.clone();
                i.src = quote(&url);
                if let Some(a) = d.library.alt.get(&url) {
                    i.alt = Some(quote(a));
                }
                Some(Command::SetImage {
                    at: target(img.id.as_deref()?),
                    image: i,
                })
            });
            if let Some(c) = cmd {
                ed.apply(ui, c, false);
            }
        });
    };
    let add_link = move |_| {
        if let Some(last) = items.with_untracked(|l| l.last().map(|x| x.id.clone())) {
            ed.apply_keep(ui, Command::Duplicate { at: target(&last) });
        }
    };
    let add = move |_| {
        crate::library::pick_image(ui, crate::library::media_shelf(ed), move |url| {
            let cmd = ed.doc.with_untracked(|d| {
                let d = d.as_ref()?;
                let alt = d.library.alt.get(&url).cloned().unwrap_or_default();
                revel_edit::make::add_image_item(d, &at.get_value(), &url, &alt).ok()
            });
            if let Some((c, _)) = cmd {
                ed.apply_keep(ui, c);
            }
        });
    };
    view! {
        <div class="items-list" role="list" aria-label=ui.t(if links { "items-links" } else { "items-title" })>
            <div class="items-head">
                <span class="label">{ui.t(if links { "items-links" } else { "items-title" })}</span>
                <span class="muted items-count">{move || items.with(Vec::len)}</span>
            </div>
            <For each=move || named.get() key=|it| (it.id.clone(), it.src.clone(), it.name.clone()) let:it>
                {
                    let it = StoredValue::new(it);
                    let id = it.with_value(|i| i.id.clone());
                    let (i1, i2, i3, i4, i5) = (id.clone(), id.clone(), id.clone(), id.clone(), id.clone());
                    let has_img = it.with_value(|i| i.img);
                    let first = move || items.with(|l| l.first().is_some_and(|x| x.id == i4));
                    let last = move || items.with(|l| l.last().is_some_and(|x| x.id == i5));
                    let on = { let id = id.clone(); move || ed.selected.with(|s| s.as_ref().is_some_and(|s| s.id == id)) };
                    view! {
                        <div class="items-row" role="listitem" class:is-on=on data-item=id.clone()>
                            <button class="items-main" on:click=move |_| select(&it.get_value())
                                title=it.with_value(|i| i.name.clone())>
                                <span class="items-thumb">
                                    {move || match it.with_value(|i| i.src.clone()) {
                                        Some(src) => view! { <img alt="" src=crate::library::thumb_src(&base(), &src)/> }.into_any(),
                                        None if links => icons::link(),
                                        None => icons::box_(),
                                    }}
                                </span>
                                <span class="items-name">{it.with_value(|i| i.name.clone())}</span>
                            </button>
                            <span class="items-actions">
                                {has_img.then(|| view! {
                                    <button class="icon-btn icon-btn--small" title=ui.t("items-replace") aria-label=ui.t("items-replace")
                                        on:click=move |_| replace(i1.clone())>{icons::image()}</button>
                                })}
                                {(!fixed).then(|| view! {
                                    <button class="icon-btn icon-btn--small" title=ui.t("items-up") aria-label=ui.t("items-up")
                                        prop:disabled=first on:click=move |_| step(i2.clone(), -1)>{icons::arrow_up()}</button>
                                    <button class="icon-btn icon-btn--small" title=ui.t("items-down") aria-label=ui.t("items-down")
                                        prop:disabled=last on:click=move |_| step(i3.clone(), 1)>{icons::arrow_down()}</button>
                                    <button class="icon-btn icon-btn--small" title=ui.t("layers-duplicate") aria-label=ui.t("layers-duplicate")
                                        on:click={ let id = id.clone(); move |_| ed.apply_keep(ui, Command::Duplicate { at: target(&id) }) }>{icons::copy()}</button>
                                    <button class="icon-btn icon-btn--small" title=ui.t("action-remove") aria-label=ui.t("action-remove")
                                        on:click={ let id = id.clone(); move |_| ed.apply(ui, Command::Remove { at: target(&id) }, false) }>{icons::trash()}</button>
                                })}
                            </span>
                        </div>
                    }
                }
            </For>
            {if fixed {
                ().into_any()
            } else if links {
                view! { <button class="btn btn--small items-add" data-action="items-add-link" on:click=add_link>{icons::plus()}<span>{ui.t("items-add-link")}</span></button> }.into_any()
            } else {
                view! { <button class="btn btn--small items-add" data-action="items-add" on:click=add>{icons::plus()}<span>{ui.t("items-add-image")}</span></button> }.into_any()
            }}
        </div>
    }
}

/// Premier texte écrit d'un nœud (nom d'un onglet).
fn first_text(n: &Node) -> Option<String> {
    if let Node::Text { text } = n {
        let t = text.trim();
        return (!t.is_empty()).then(|| t.to_string());
    }
    revel_model::ids::child_lists(n)
        .into_iter()
        .flat_map(|l| l.iter())
        .find_map(first_text)
}

/// Onglets d'une boîte d'onglets (D-123) : nom de chaque onglet, retirer (avec ses panneaux),
/// « Ajouter un onglet » (copie du dernier et de ses panneaux). Onglets répétés par une liste
/// du CMS : rien à régler ici.
#[component]
pub fn TabsList(at: Target) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let at = StoredValue::new(at);
    let tabs = Memo::new(move |_| {
        let at = at.get_value();
        ed.doc.with(|d| {
            let d = d.as_ref()?;
            let m = revel_edit::tabs::map(d, &at);
            if m.repeated {
                return None;
            }
            Some(
                m.tabs
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        let n = revel_edit::address::node(d, t).ok();
                        let name = n
                            .and_then(|n| {
                                revel_model::layer_name(n)
                                    .map(str::to_string)
                                    .or_else(|| first_text(n))
                            })
                            .unwrap_or_else(|| ui.now("items-tab", &[("n", (i + 1).to_string())]));
                        let tag = match n {
                            Some(Node::Element { tag, .. }) => tag.clone(),
                            _ => String::new(),
                        };
                        (t.node.clone(), tag, name)
                    })
                    .collect::<Vec<_>>(),
            )
        })
    });
    let add = move |_| {
        let cmd = ed.doc.with_untracked(|d| {
            d.as_ref()
                .and_then(|d| revel_edit::tabs::add_tab(d, &at.get_value()))
        });
        if let Some(c) = cmd {
            ed.apply_keep(ui, c);
        }
    };
    let remove = move |i: usize| {
        let cmd = ed.doc.with_untracked(|d| {
            d.as_ref()
                .and_then(|d| revel_edit::tabs::remove_tab(d, &at.get_value(), i))
        });
        if let Some(c) = cmd {
            ed.apply(ui, c, false);
        }
    };
    move || {
        tabs.get().map(|list| {
            let n = list.len();
            view! {
                <div class="items-list tabs-list" role="list" aria-label=ui.t("items-tabs")>
                    <div class="items-head">
                        <span class="label">{ui.t("items-tabs")}</span>
                        <span class="muted items-count">{n}</span>
                    </div>
                    {list.into_iter().enumerate().map(|(i, (id, tag, name))| {
                        let id2 = id.clone();
                        let (sel_id, sel_tag, sel_name) = (id.clone(), tag.clone(), name.clone());
                        let on = move || ed.selected.with(|s| s.as_ref().is_some_and(|s| s.id == id2));
                        view! {
                            <div class="items-row" role="listitem" class:is-on=on data-item=id.clone()>
                                <button class="items-main" title=name.clone() on:click=move |_| {
                                    ed.selected.set(Some(Selection { id: sel_id.clone(), tag: sel_tag.clone(), label: sel_name.clone() }));
                                    ed.select_and_reveal(&sel_id);
                                }>
                                    <span class="items-thumb items-thumb--icon">{icons::tabs()}</span>
                                    <span class="items-name">{name.clone()}</span>
                                </button>
                                <span class="items-actions">
                                    <button class="icon-btn icon-btn--small" title=ui.t("action-remove") aria-label=ui.t("action-remove")
                                        prop:disabled=n < 2 on:click=move |_| remove(i)>{icons::trash()}</button>
                                </span>
                            </div>
                        }
                    }).collect_view()}
                    <button class="btn btn--small items-add" data-action="tabs-add" on:click=add>{icons::plus()}<span>{ui.t("items-add-tab")}</span></button>
                </div>
            }
        })
    }
}
