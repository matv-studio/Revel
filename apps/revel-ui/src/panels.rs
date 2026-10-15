//! Éléments communs des panneaux (recherche, fonctions prévues) et panneau CMS (collections et
//! contenus, fermé par défaut). Les fonctions prévues aux étapes suivantes sont déjà en place,
//! désactivées, avec l'étape indiquée en info-bulle.

use crate::editor::Ed;
use crate::{Ui, icons};
use leptos::prelude::*;

/// Bouton d'une fonction prévue (désactivé, étape en info-bulle).
#[component]
pub fn PlannedButton(key: &'static str, step: u8, icon: fn() -> AnyView) -> impl IntoView {
    let ui = expect_context::<Ui>();
    view! {
        <button class="icon-btn icon-btn--small is-planned" disabled=true
            title=move || format!("{} — {}", ui.tx(key, &[]), ui.tx("planned-step", &[("step", step.to_string())]))
            aria-label=ui.t(key)>{icon()}</button>
    }
}

/// Panneau entièrement prévu pour une étape suivante.
#[component]
pub fn Planned(key: &'static str, step: u8, icon: fn() -> AnyView) -> impl IntoView {
    let ui = expect_context::<Ui>();
    view! {
        <div class="planned">
            <span class="planned-icon">{icon()}</span>
            <p>{ui.t(key)}</p>
            <span class="chip">{move || ui.tx("planned-step", &[("step", step.to_string())])}</span>
        </div>
    }
}

/// Champ de recherche d'un panneau.
#[component]
pub(crate) fn Search(value: RwSignal<String>, key: &'static str) -> impl IntoView {
    let ui = expect_context::<Ui>();
    view! {
        <label class="panel-search">
            {icons::search()}
            <input type="search" placeholder=ui.t(key) aria-label=ui.t(key) prop:value=move || value.get()
                on:input=move |e| value.set(event_target_value(&e))/>
        </label>
    }
}

/// Ouvre une page dans le canevas.
fn open_page(ed: Ed, url: String) {
    if ed.page.get_untracked() != url {
        ed.page.set(url);
        ed.scroll.set((0.0, 0.0));
        ed.selected.set(None);
    }
}

/// Collections du site et leurs entrées (lecture ; édition à l'étape 3).
#[component]
pub fn Cms() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let open = RwSignal::new(None::<String>);
    let list = move || {
        ed.doc.with(|d| {
            d.as_ref()
                .map(|d| {
                    d.collections
                        .iter()
                        .map(|c| (c.id.clone(), c.folder.clone()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    };
    view! {
        <div class="panel-list">
            <div class="list-scroll">
                {move || {
                    let l = list();
                    if l.is_empty() {
                        return view! { <p class="panel-empty muted">{ui.t("cms-none")}</p> }.into_any();
                    }
                    l.into_iter().map(|(id, folder)| {
                        let entries = ed.pages.with(|ps| ps.iter().filter(|p| p.collection.as_deref() == Some(id.as_str())).cloned().collect::<Vec<_>>());
                        let n = entries.len();
                        let (i2, i3) = (id.clone(), id.clone());
                        view! {
                            <button class="list-row list-folder" title=folder.clone()
                                on:click=move |_| open.update(|o| *o = if o.as_deref() == Some(i2.as_str()) { None } else { Some(i2.clone()) })>
                                <span class="list-chevron">{move || if open.get().as_deref() == Some(i3.as_str()) { icons::chevron_down() } else { icons::chevron_right() }}</span>
                                <span class="list-icon">{icons::database()}</span>
                                <span class="list-label">{id.clone()}</span>
                                <span class="list-count">{n}</span>
                            </button>
                            <Show when={let id = id.clone(); move || open.get().as_deref() == Some(id.as_str())}>
                                {entries.clone().into_iter().map(|p| {
                                    let url = p.url.clone();
                                    let u2 = url.clone();
                                    view! {
                                        <button class="list-row is-nested" class:is-on=move || ed.page.get() == u2 title=p.url.clone()
                                            on:click=move |_| open_page(ed, url.clone())>
                                            <span class="list-icon">{icons::text()}</span>
                                            <span class="list-label">{if p.title.is_empty() { p.url.clone() } else { p.title.clone() }}</span>
                                        </button>
                                    }
                                }).collect_view()}
                            </Show>
                        }
                    }).collect_view().into_any()
                }}
            </div>
            <footer class="panel-foot">
                <span class="panel-foot-text muted">{move || ui.tx("planned-edit", &[("step", "3".to_string())])}</span>
                <PlannedButton key="cms-entry-new" step=3 icon=icons::plus/>
                <PlannedButton key="cms-collection-new" step=3 icon=icons::database/>
            </footer>
        </div>
    }
}
