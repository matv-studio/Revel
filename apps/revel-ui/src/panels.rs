//! Panneaux de l'éditeur autres que calques, ajouter, styles et réglages : pages, CMS, composants,
//! médias, interactions. Les fonctions prévues aux étapes suivantes sont déjà en place, désactivées,
//! avec l'étape indiquée en info-bulle.

use crate::editor::Ed;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_model::Node;
use std::collections::BTreeMap;

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
fn Search(value: RwSignal<String>, key: &'static str) -> impl IntoView {
    let ui = expect_context::<Ui>();
    view! {
        <label class="panel-search">
            {icons::search()}
            <input type="search" placeholder=ui.t(key) aria-label=ui.t(key) prop:value=move || value.get()
                on:input=move |e| value.set(event_target_value(&e))/>
        </label>
    }
}

fn matches(q: &str, texts: &[&str]) -> bool {
    let q = q.trim().to_lowercase();
    q.is_empty() || texts.iter().any(|t| t.to_lowercase().contains(&q))
}

/// Ouvre une page dans le canevas.
fn open_page(ed: Ed, url: String) {
    if ed.page.get_untracked() != url {
        ed.page.set(url);
        ed.scroll.set((0.0, 0.0));
        ed.selected.set(None);
    }
}

/// Pages fixes et contenus de collections (dossiers repliables), avec recherche.
#[component]
pub fn Pages() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let q = RwSignal::new(String::new());
    let open = RwSignal::new(Vec::<String>::new());
    let fixed = move || {
        ed.pages.with(|ps| {
            ps.iter()
                .filter(|p| p.page.is_some())
                .filter(|p| q.with(|q| matches(q, &[&p.title, &p.url])))
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    let collections = move || {
        let mut m: BTreeMap<String, Vec<revel_protocol::EditorPageInfo>> = BTreeMap::new();
        ed.pages.with(|ps| {
            for p in ps.iter().filter(|p| p.collection.is_some()) {
                if q.with(|q| {
                    matches(
                        q,
                        &[
                            &p.title,
                            &p.url,
                            p.collection.as_deref().unwrap_or_default(),
                        ],
                    )
                }) {
                    m.entry(p.collection.clone().unwrap_or_default())
                        .or_default()
                        .push(p.clone());
                }
            }
        });
        m.into_iter().collect::<Vec<_>>()
    };
    let row = move |p: revel_protocol::EditorPageInfo, nested: bool| {
        let url = p.url.clone();
        let u2 = url.clone();
        let title = if p.title.is_empty() {
            p.url.clone()
        } else {
            p.title.clone()
        };
        view! {
            <button class="list-row" class:is-on=move || ed.page.get() == u2 class:is-nested=nested
                title=p.url.clone() on:click=move |_| open_page(ed, url.clone())>
                <span class="list-icon">{if nested { icons::text() } else { icons::page() }}</span>
                <span class="list-label">{title}</span>
                <span class="list-meta">{p.url.clone()}</span>
            </button>
        }
    };
    view! {
        <div class="panel-list">
            <Search value=q key="pages-search"/>
            <div class="list-scroll">
                {move || fixed().into_iter().map(|p| row(p, false)).collect_view()}
                {move || collections().into_iter().map(|(c, items)| {
                    let c2 = c.clone();
                    let c3 = c.clone();
                    let is_open = Memo::new(move |_| !q.with(|q| q.is_empty()) || open.with(|o| o.contains(&c2)));
                    let n = items.len();
                    view! {
                        <button class="list-row list-folder" on:click=move |_| open.update(|o| {
                            if let Some(i) = o.iter().position(|x| *x == c3) { o.remove(i); } else { o.push(c3.clone()); }
                        })>
                            <span class="list-chevron">{move || if is_open.get() { icons::chevron_down() } else { icons::chevron_right() }}</span>
                            <span class="list-icon">{icons::folder()}</span>
                            <span class="list-label">{c.clone()}</span>
                            <span class="list-count">{n}</span>
                        </button>
                        <Show when=move || is_open.get()>
                            {items.clone().into_iter().map(|p| row(p, true)).collect_view()}
                        </Show>
                    }
                }).collect_view()}
            </div>
            <footer class="panel-foot">
                <span class="panel-foot-text muted">{move || ui.tx("pages-count", &[("count", ed.pages.with(|p| p.len()).to_string())])}</span>
                <PlannedButton key="pages-new" step=3 icon=icons::plus/>
                <PlannedButton key="pages-folder-new" step=3 icon=icons::folder/>
            </footer>
        </div>
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

/// Composants du site et nombre d'utilisations.
#[component]
pub fn Components() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let q = RwSignal::new(String::new());
    let list = move || {
        ed.doc.with(|d| {
            let Some(d) = d.as_ref() else {
                return Vec::new();
            };
            let mut uses: BTreeMap<String, usize> = BTreeMap::new();
            for (_, _, body) in d.owners() {
                revel_model::ids::walk(body, &mut |n| {
                    if let Node::Use { component, .. } = n {
                        *uses.entry(component.clone()).or_default() += 1;
                    }
                });
            }
            d.components
                .iter()
                .filter(|c| q.with(|q| matches(q, &[&c.id])))
                .map(|c| (c.id.clone(), uses.get(&c.id).copied().unwrap_or(0)))
                .collect::<Vec<_>>()
        })
    };
    view! {
        <div class="panel-list">
            <Search value=q key="components-search"/>
            <div class="list-scroll">
                {move || list().into_iter().map(|(id, n)| view! {
                    <div class="list-row" title=move || ui.tx("components-uses", &[("count", n.to_string())])>
                        <span class="list-icon">{icons::component()}</span>
                        <span class="list-label">{id}</span>
                        <span class="list-count">{n}</span>
                    </div>
                }).collect_view()}
            </div>
            <footer class="panel-foot">
                <span class="panel-foot-text muted">{move || ui.tx("planned-edit", &[("step", "3".to_string())])}</span>
                <PlannedButton key="components-new" step=3 icon=icons::plus/>
            </footer>
        </div>
    }
}

/// Images du site : utilisées dans les pages (sources littérales) et dans le CSS (`url()`).
#[component]
pub fn Assets() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let q = RwSignal::new(String::new());
    let list = move || {
        let mut out: Vec<String> = Vec::new();
        ed.doc.with(|d| {
            let Some(d) = d.as_ref() else { return };
            for (_, _, body) in d.owners() {
                revel_model::ids::walk(body, &mut |n| {
                    let src = match n {
                        Node::Image(i) => crate::inspector::literal(&i.src),
                        Node::Element { tag, attrs, .. } if tag.eq_ignore_ascii_case("img") => {
                            attrs
                                .iter()
                                .find(|a| a.name == "src")
                                .and_then(|a| a.value.clone())
                                .filter(|v| !v.contains("{{"))
                        }
                        _ => None,
                    };
                    if let Some(s) = src
                        && !out.contains(&s)
                    {
                        out.push(s);
                    }
                });
            }
            for r in d.style_rules() {
                for decl in &r.decls {
                    let mut rest = decl.value.as_str();
                    while let Some(i) = rest.find("url(") {
                        let after = &rest[i + 4..];
                        let end = after.find(')').unwrap_or(after.len());
                        let u = after[..end]
                            .trim()
                            .trim_matches(|c| c == '"' || c == '\'')
                            .to_string();
                        if !u.starts_with("data:") && !u.is_empty() && !out.contains(&u) {
                            out.push(u);
                        }
                        rest = &after[end..];
                    }
                }
            }
        });
        out.retain(|u| q.with(|q| matches(q, &[u])));
        out
    };
    let base = move || ed.base.get().unwrap_or_default();
    let src_of = move |u: &str| {
        if u.starts_with("http") {
            u.to_string()
        } else {
            format!("{}{}", base(), u.trim_start_matches('/'))
        }
    };
    view! {
        <div class="panel-list">
            <Search value=q key="assets-search"/>
            <div class="list-scroll">
                {move || list().is_empty().then(|| view! { <p class="panel-empty muted">{ui.t("assets-none")}</p> })}
                <div class="asset-grid">
                    {move || list().into_iter().map(|u| {
                        let name = u.rsplit('/').next().unwrap_or(&u).to_string();
                        view! {
                            <figure class="asset" title=u.clone()>
                                <div class="asset-thumb"><img src=src_of(&u) alt="" loading="lazy"/></div>
                                <figcaption>{name}</figcaption>
                            </figure>
                        }
                    }).collect_view()}
                </div>
            </div>
            <footer class="panel-foot">
                <span class="panel-foot-text muted">{move || ui.tx("assets-count", &[("count", list().len().to_string())])}</span>
                <PlannedButton key="assets-import" step=3 icon=icons::upload/>
            </footer>
        </div>
    }
}
