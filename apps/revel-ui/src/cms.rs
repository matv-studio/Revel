//! Vue CMS : collections de contenus et leurs entrées, en tableau (lecture ; création et
//! modification des contenus prévues à l'étape 3).

use crate::panels::PlannedButton;
use crate::site::capitalize;
use crate::{Tab, Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{CmsCollection, CmsEntry, CmsField, SiteSummary};
use serde_json::Value;

/// Variables techniques des en-têtes, gardées hors du tableau (visibles dans la fiche).
const HIDDEN: &[&str] = &[
    "layout",
    "permalink",
    "eleventyExcludeFromCollections",
    "eleventyComputed",
    "eleventyNavigation",
];
/// Noms habituels du champ titre.
const TITLES: &[&str] = &["title", "titre", "name", "nom"];
/// Colonnes affichées en plus du titre.
const MAX_COLUMNS: usize = 5;

/// Libellé d'un champ : celui du schéma, sinon le nom mis en forme (`date_debut` -> « Date debut »).
fn field_label(f: &CmsField) -> String {
    f.label
        .clone()
        .unwrap_or_else(|| capitalize(&f.name.replace(['_', '-'], " ")))
}

/// Valeur d'un champ en texte court (cellule du tableau).
fn cell(ui: Ui, v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::Bool(b)) => if *b { "✓" } else { "—" }.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => {
            // Date ISO : date locale ; chemin de fichier : son nom.
            if s.len() >= 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
                let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(s));
                if !d.get_time().is_nan() {
                    let opts = js_sys::Object::new();
                    let _ = js_sys::Reflect::set(&opts, &"dateStyle".into(), &"medium".into());
                    return d
                        .to_locale_date_string(&ui.lang(), &opts)
                        .as_string()
                        .unwrap_or_else(|| s.clone());
                }
            }
            s.clone()
        }
        Some(Value::Array(a)) => {
            if a.iter().all(|x| x.is_string()) && a.len() <= 3 {
                a.iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                ui.tx("cms-items", &[("count", a.len().to_string())])
            }
        }
        Some(Value::Object(m)) => ui.tx("cms-fields", &[("count", m.len().to_string())]),
    }
}

/// Valeur complète (fiche) : texte, ou JSON lisible pour une liste ou un objet.
fn full(ui: Ui, v: Option<&Value>) -> String {
    match v {
        Some(Value::Array(_) | Value::Object(_)) => {
            serde_json::to_string_pretty(v.unwrap_or(&Value::Null)).unwrap_or_default()
        }
        other => cell(ui, other),
    }
}

/// Titre d'une entrée : champ titre, sinon nom du fichier.
fn entry_title(e: &CmsEntry, title: &Option<String>) -> String {
    title
        .as_ref()
        .and_then(|t| e.data.get(t))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| e.file.clone())
}

/// Contenu non publié (`draft: true` ou `published: false`).
fn is_draft(e: &CmsEntry) -> bool {
    e.data.get("draft").and_then(Value::as_bool) == Some(true)
        || e.data.get("published").and_then(Value::as_bool) == Some(false)
}

#[component]
pub fn CmsView(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let cols = RwSignal::new(None::<Vec<CmsCollection>>);
    let current = RwSignal::new(None::<String>);
    let open = RwSignal::new(None::<String>);
    let query = RwSignal::new(String::new());
    let sort = RwSignal::new(None::<(String, bool)>);
    let path = site.path.clone();
    spawn_local(async move {
        match api::cms_collections(&path).await {
            Ok(c) => {
                if current.get_untracked().is_none() {
                    current.set(c.first().map(|c| c.id.clone()));
                }
                cols.set(Some(c));
            }
            Err(e) => {
                cols.set(Some(Vec::new()));
                ui.fail(e)
            }
        }
    });
    Effect::new(move |_| {
        let _ = current.get();
        open.set(None);
        query.set(String::new());
        sort.set(None);
    });
    let col = Memo::new(move |_| {
        let id = current.get()?;
        cols.with(|c| c.as_ref()?.iter().find(|c| c.id == id).cloned())
    });
    // Champ titre et colonnes affichées.
    let layout = Memo::new(move |_| {
        let c = col.get()?;
        let title = c
            .fields
            .iter()
            .find(|f| TITLES.contains(&f.name.to_lowercase().as_str()))
            .map(|f| f.name.clone());
        let columns: Vec<CmsField> = c
            .fields
            .iter()
            .filter(|f| Some(&f.name) != title.as_ref() && !HIDDEN.contains(&f.name.as_str()))
            .take(MAX_COLUMNS)
            .cloned()
            .collect();
        Some((title, columns))
    });
    let rows = Memo::new(move |_| {
        let Some(c) = col.get() else {
            return Vec::new();
        };
        let title = layout.get().and_then(|l| l.0);
        let q = query.get().trim().to_lowercase();
        let mut list: Vec<CmsEntry> = c
            .entries
            .into_iter()
            .filter(|e| {
                q.is_empty()
                    || e.file.to_lowercase().contains(&q)
                    || e.data.to_string().to_lowercase().contains(&q)
            })
            .collect();
        if let Some((field, desc)) = sort.get() {
            let key = |e: &CmsEntry| -> String {
                if Some(&field) == title.as_ref() {
                    entry_title(e, &title).to_lowercase()
                } else {
                    match e.data.get(&field) {
                        Some(Value::Number(n)) => format!("{:020.6}", n.as_f64().unwrap_or(0.0)),
                        Some(Value::String(s)) => s.to_lowercase(),
                        Some(v) => v.to_string(),
                        None => String::new(),
                    }
                }
            };
            list.sort_by_key(key);
            if desc {
                list.reverse();
            }
        }
        list
    });
    let sort_by = move |field: String| {
        sort.update(|s| {
            *s = match s.take() {
                Some((f, false)) if f == field => Some((f, true)),
                Some((f, true)) if f == field => None,
                _ => Some((field, false)),
            }
        })
    };
    let sort_mark = move |field: &str| {
        sort.with(|s| match s {
            Some((f, false)) if f == field => "↑",
            Some((f, true)) if f == field => "↓",
            _ => "",
        })
    };
    let goto = move |url: String| {
        ui.goto_page.set(Some(url));
        ui.tab(Tab::Editor);
    };

    view! {
        <div class="cms" class:has-item=move || open.get().is_some()>
            <aside class="cms-cols">
                <p class="cms-cols-title">{ui.t("cms-collections")}</p>
                <div class="cms-cols-list">
                    {move || cols.get().map(|list| {
                        if list.is_empty() {
                            return view! { <p class="panel-empty muted">{ui.t("cms-none")}</p> }.into_any();
                        }
                        list.into_iter().map(|c| {
                            let (id, id2) = (c.id.clone(), c.id.clone());
                            view! {
                                <button class="cms-col" class:is-on=move || current.get().as_deref() == Some(id.as_str())
                                    data-collection=c.id.clone() title=c.folder.clone()
                                    on:click=move |_| current.set(Some(id2.clone()))>
                                    <span class="list-icon">{icons::database()}</span>
                                    <span class="list-label">{capitalize(&c.id)}</span>
                                    <span class="list-count">{c.entries.len()}</span>
                                </button>
                            }
                        }).collect_view().into_any()
                    })}
                </div>
                <footer class="panel-foot">
                    <span class="panel-foot-text muted"></span>
                    <PlannedButton key="cms-collection-new" step=3 icon=icons::plus/>
                </footer>
            </aside>
            <section class="cms-main">
                {move || col.get().map(|c| view! {
                    <header class="cms-head">
                        <div class="cms-title">
                            <h1>{capitalize(&c.id)}</h1>
                            <span class="muted path" title=c.folder.clone()>{c.folder.clone()}</span>
                        </div>
                        <label class="panel-search cms-search">
                            {icons::search()}
                            <input type="search" placeholder=ui.t("cms-search") aria-label=ui.t("cms-search")
                                prop:value=move || query.get() on:input=move |e| query.set(event_target_value(&e))/>
                        </label>
                        <button class="btn" disabled=true title=move || ui.tx("planned-step", &[("step", "3".to_string())])>
                            {icons::database()}{ui.t("cms-fields-edit")}
                        </button>
                        <button class="btn btn--primary" disabled=true title=move || ui.tx("planned-step", &[("step", "3".to_string())])>
                            {icons::plus()}{ui.t("cms-entry-new")}
                        </button>
                    </header>
                })}
                <div class="cms-table-wrap">
                    {move || match (cols.get().is_none(), layout.get()) {
                        (true, _) => view! { <p class="muted cms-wait">{ui.t("editor-loading")}</p> }.into_any(),
                        (false, None) => ().into_any(),
                        (false, Some((title, columns))) => {
                            let tkey = title.clone().unwrap_or_default();
                            let tkey2 = tkey.clone();
                            view! {
                                <table class="table cms-table">
                                    <thead><tr>
                                        <th class="cms-th-title">
                                            <button class="th-sort" on:click=move |_| sort_by(tkey.clone())>
                                                {title.as_ref().and_then(|t| col.get().and_then(|c| c.fields.into_iter().find(|f| &f.name == t))).map(|f| field_label(&f)).unwrap_or_else(|| ui.tx("cms-file", &[]))}
                                                <span class="th-mark">{move || sort_mark(&tkey2)}</span>
                                            </button>
                                        </th>
                                        {columns.iter().map(|f| {
                                            let (n1, n2) = (f.name.clone(), f.name.clone());
                                            view! {
                                                <th><button class="th-sort" on:click=move |_| sort_by(n1.clone())>
                                                    {field_label(f)}<span class="th-mark">{move || sort_mark(&n2)}</span>
                                                </button></th>
                                            }
                                        }).collect_view()}
                                        <th class="col-actions"></th>
                                    </tr></thead>
                                    <tbody>
                                        {move || {
                                            let title = title.clone();
                                            let columns = columns.clone();
                                            let list = rows.get();
                                            if list.is_empty() {
                                                return view! {
                                                    <tr><td class="muted cms-empty" colspan=(columns.len() + 2).to_string()>{ui.t("cms-no-entries")}</td></tr>
                                                }.into_any();
                                            }
                                            list.into_iter().map(|e| {
                                                let file = e.file.clone();
                                                let f2 = e.file.clone();
                                                let url = e.url.clone();
                                                let draft = is_draft(&e);
                                                view! {
                                                    <tr class="row-link" class:is-on=move || open.get().as_deref() == Some(f2.as_str())
                                                        data-entry=e.file.clone()
                                                        on:click=move |_| open.set(Some(file.clone()))>
                                                        <td class="strong cms-td-title"><div class="cms-title-cell">
                                                            <span class="cms-cell">{entry_title(&e, &title)}</span>
                                                            {draft.then(|| view! { <span class="chip chip--draft">{ui.t("cms-draft")}</span> })}
                                                        </div></td>
                                                        {columns.iter().map(|f| view! {
                                                            <td class="muted"><span class="cms-cell">{cell(ui, e.data.get(&f.name))}</span></td>
                                                        }).collect_view()}
                                                        <td class="col-actions">
                                                            {url.map(|u| view! {
                                                                <button class="icon-btn icon-btn--small" title=ui.t("cms-open-page") aria-label=ui.t("cms-open-page")
                                                                    on:click=move |ev| { ev.stop_propagation(); goto(u.clone()) }>{icons::edit()}</button>
                                                            })}
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect_view().into_any()
                                        }}
                                    </tbody>
                                </table>
                            }.into_any()
                        }
                    }}
                </div>
                <footer class="cms-foot muted">
                    {move || col.get().map(|c| {
                        let shown = rows.with(|r| r.len());
                        let total = c.entries.len();
                        if shown == total {
                            ui.tx("site-entries", &[("count", crate::num(total as f64, 0))])
                        } else {
                            ui.tx("cms-shown", &[("shown", shown.to_string()), ("total", total.to_string())])
                        }
                    })}
                </footer>
            </section>
            {move || {
                let file = open.get()?;
                let c = col.get()?;
                let e = c.entries.iter().find(|e| e.file == file)?.clone();
                let title = layout.get().and_then(|l| l.0);
                let url = e.url.clone();
                let fields = c.fields.clone();
                Some(view! {
                    <aside class="cms-item" aria-label=ui.t("cms-entry")>
                        <header class="cms-item-head">
                            <h2>{entry_title(&e, &title)}</h2>
                            <button class="icon-btn" title=ui.t("action-close") aria-label=ui.t("action-close")
                                on:click=move |_| open.set(None)>{icons::close()}</button>
                        </header>
                        <div class="cms-item-body">
                            <div class="cms-item-field">
                                <span class="label">{ui.t("cms-file")}</span>
                                <span class="path">{format!("{}/{}.md", c.folder, e.file)}</span>
                            </div>
                            {fields.into_iter().map(|f| {
                                // Liste de textes : pastilles.
                                if let Some(Value::Array(a)) = e.data.get(&f.name)
                                    && !a.is_empty()
                                    && a.iter().all(Value::is_string)
                                {
                                    let items: Vec<String> = a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect();
                                    return view! {
                                        <div class="cms-item-field">
                                            <span class="label" title=f.name.clone()>{field_label(&f)}</span>
                                            <span class="cms-value chips">{items.into_iter().map(|t| view! { <span class="chip">{t}</span> }).collect_view()}</span>
                                        </div>
                                    }.into_any();
                                }
                                let v = full(ui, e.data.get(&f.name));
                                let long = v.len() > 60 || v.contains('\n');
                                let empty = v.is_empty();
                                view! {
                                    <div class="cms-item-field" class:is-long=long>
                                        <span class="label" title=f.name.clone()>{field_label(&f)}</span>
                                        <span class="cms-value" class:muted=empty>{if empty { "—".to_string() } else { v }}</span>
                                    </div>
                                }.into_any()
                            }).collect_view()}
                            <div class="cms-item-field">
                                <span class="label">{ui.t("cms-body")}</span>
                                <span class="muted">{ui.tx("cms-body-len", &[("count", e.body_len.to_string())])}</span>
                            </div>
                        </div>
                        <footer class="cms-item-foot">
                            <button class="btn" disabled=true title=move || ui.tx("planned-step", &[("step", "3".to_string())])>{icons::edit()}{ui.t("action-edit")}</button>
                            {url.map(|u| view! {
                                <button class="btn btn--primary" on:click=move |_| goto(u.clone())>{ui.t("cms-open-page")}</button>
                            })}
                        </footer>
                    </aside>
                })
            }}
        </div>
    }
}
