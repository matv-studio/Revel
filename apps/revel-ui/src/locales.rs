//! Vue Localisation (3.8) : langues du site (ajout, nom, publication, retrait), avancement de la
//! traduction et textes à traduire, comme la localisation de Webflow.
//!
//! - **Pages** : textes écrits dans les pages, gabarits et composants (`Locale::texts`,
//!   commande `SetTranslation`) ;
//! - **Données** : textes des fichiers de données, écrits dans le dossier de la langue
//!   (`_data/en/…`, copié au besoin par le cœur) ;
//! - **Contenus** : contenus des collections, traduits dans la vue CMS (dossier de la langue).

use crate::dialog::Ask;
use crate::{Tab, Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::Command;
use revel_edit::canvas::{Binding, Seg, Source};
use revel_edit::locale::{DataText, SiteText};
use revel_protocol::{CmsCollection, SiteSummary};
use std::collections::BTreeMap;

/// Langues proposées à l'ajout (les autres : code saisi).
const COMMON: &[&str] = &[
    "en", "fr", "de", "es", "it", "pt", "nl", "ca", "pl", "sv", "da", "ja", "zh", "ar",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Pages,
    Data,
    Entries,
}

/// Données de la vue : document, textes des fichiers de données par langue, contenus.
#[derive(Clone, Copy)]
struct LocSession {
    doc: RwSignal<Option<revel_model::Site>>,
    data: RwSignal<BTreeMap<String, Vec<DataText>>>,
    cms: RwSignal<Option<Vec<CmsCollection>>>,
    apply: Callback<Command>,
}

fn session(ui: Ui, path: String) -> LocSession {
    let doc = RwSignal::new(None::<revel_model::Site>);
    let data = RwSignal::new(BTreeMap::<String, Vec<DataText>>::new());
    let cms = RwSignal::new(None::<Vec<CmsCollection>>);
    let path = StoredValue::new(path);
    let reload = move || {
        let p = path.get_value();
        spawn_local(async move {
            let Ok(o) = api::edit_open(&p).await else {
                return;
            };
            if doc.is_disposed() {
                return;
            }
            let codes: Vec<String> = o.site.locales.iter().map(|l| l.code.clone()).collect();
            doc.set(Some(o.site));
            let mut all = BTreeMap::new();
            for c in codes {
                if let Ok(list) = api::edit_locale_data(&p, &c).await {
                    all.insert(c, list);
                }
            }
            if let Ok(c) = api::cms_collections(&p).await
                && !cms.is_disposed()
            {
                cms.set(Some(c));
            }
            if !data.is_disposed() {
                data.set(all);
            }
        });
    };
    reload();
    let apply = Callback::new(move |cmd: Command| {
        let p = path.get_value();
        spawn_local(async move {
            if let Err(e) = api::edit_apply(&p, &cmd, false).await {
                ui.fail(e);
            }
            if !doc.is_disposed() {
                reload();
            }
        });
    });
    LocSession {
        doc,
        data,
        cms,
        apply,
    }
}

/// Valeur traduite d'un texte de données : différente du texte d'origine.
fn data_done(t: &DataText) -> bool {
    t.text.as_ref().is_some_and(|x| x != &t.source)
}

/// Titre d'un contenu (champ titre habituel, sinon nom du fichier).
fn entry_title(c: &CmsCollection, e: &revel_protocol::CmsEntry) -> String {
    crate::cms::primary_of(c)
        .and_then(|k| e.data.get(&k).and_then(|v| v.as_str()).map(str::to_string))
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| e.file.clone())
}

/// Avancement d'une langue : (traduits, total).
fn progress(
    texts: &[SiteText],
    l: &revel_model::Locale,
    data: Option<&Vec<DataText>>,
    cms: Option<&Vec<CmsCollection>>,
) -> (usize, usize) {
    let mut done = texts
        .iter()
        .filter(|t| l.texts.contains_key(&t.source))
        .count();
    let mut total = texts.len();
    if let Some(d) = data {
        total += d.len();
        done += d.iter().filter(|t| data_done(t)).count();
    }
    for c in cms.into_iter().flatten() {
        total += c.entries.len();
        if let Some(own) = c.locales.get(&l.code) {
            done += c
                .entries
                .iter()
                .filter(|e| own.iter().any(|o| o.file == e.file))
                .count();
        }
    }
    (done, total)
}

#[component]
pub fn Locales(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let s = session(ui, site.path.clone());
    let primary = site.locale.clone();
    let locales = Memo::new(move |_| {
        s.doc
            .with(|d| d.as_ref().map(|d| d.locales.clone()).unwrap_or_default())
    });
    let texts = Memo::new(move |_| {
        s.doc.with(|d| {
            d.as_ref()
                .map(revel_edit::locale::texts)
                .unwrap_or_default()
        })
    });
    let no_url = move || {
        s.doc.with(|d| {
            d.as_ref()
                .is_some_and(|d| d.base_url.as_deref().unwrap_or_default().is_empty())
        })
    };
    // Langue des textes affichés : langue de travail, sinon la première autre langue.
    let sel = Memo::new(move |_| {
        let list = locales.get();
        ui.locale
            .get()
            .filter(|c| list.iter().any(|l| &l.code == c))
            .or_else(|| list.first().map(|l| l.code.clone()))
    });
    let part = RwSignal::new(Part::Pages);
    let todo_only = RwSignal::new(false);
    let query = RwSignal::new(String::new());

    let add = move |code: String| {
        let code = code.trim().to_string();
        if code.is_empty() {
            return;
        }
        ui.locale.set(Some(code.clone()));
        s.apply.run(Command::AddLocale {
            locale: revel_model::Locale::new(code),
            index: None,
        });
    };
    let add_other = move || {
        ui.ask.set(Some(Ask::input(
            ui.now("locales-add", &[]),
            ui.now("locales-code-input", &[]),
            String::new(),
            ui.now("action-add", &[]),
            move |v| add(v.trim().to_string()),
        )));
    };
    let rename = move |l: revel_model::Locale| {
        ui.ask.set(Some(Ask::input(
            ui.now("locales-rename", &[]),
            ui.now("locales-name", &[]),
            l.label(),
            ui.now("action-rename", &[]),
            move |v| {
                s.apply.run(Command::SetLocale {
                    code: l.code.clone(),
                    name: Some(v),
                    draft: l.draft,
                })
            },
        )));
    };
    let remove = move |l: revel_model::Locale| {
        ui.ask.set(Some(
            Ask::confirm(
                ui.now("locales-remove-title", &[("name", l.label())]),
                Some(ui.now("locales-remove-body", &[])),
                ui.now("action-remove", &[]),
                move || {
                    if ui.locale.get_untracked().as_deref() == Some(l.code.as_str()) {
                        ui.locale.set(None);
                    }
                    s.apply.run(Command::RemoveLocale {
                        code: l.code.clone(),
                    })
                },
            )
            .danger(),
        ));
    };
    let primary2 = primary.clone();
    let choices = move || {
        let taken: Vec<String> = locales.with(|l| l.iter().map(|l| l.code.clone()).collect());
        COMMON
            .iter()
            .filter(|c| **c != primary2.as_str() && !taken.iter().any(|t| t == *c))
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
    };

    view! {
        <section class="page locales-page">
            <header class="page-head">
                <h1>{ui.t("view-locales")}</h1>
                <div class="toolbar">
                    <details class="bp-menu locales-add">
                        <summary class="btn btn--primary" title=ui.t("locales-add-help")>{icons::plus()}{ui.t("locales-add")}</summary>
                        <div class="menu locales-menu" role="menu">
                            {move || choices().into_iter().map(|c| {
                                let c2 = c.clone();
                                view! {
                                    <button class="menu-item" role="menuitem" data-code=c.clone()
                                        on:click=move |e: leptos::ev::MouseEvent| {
                                            close_details(&e);
                                            add(c2.clone())
                                        }>
                                        <span class="menu-label">{revel_model::language_name(&c)}</span>
                                        <span class="muted path">{c.clone()}</span>
                                    </button>
                                }
                            }).collect_view()}
                            <button class="menu-item menu-sep" role="menuitem"
                                on:click=move |e: leptos::ev::MouseEvent| { close_details(&e); add_other() }>
                                {icons::edit()}<span class="menu-label">{ui.t("locales-other")}</span>
                            </button>
                        </div>
                    </details>
                </div>
            </header>
            <div class="section section--flush">
                <table class="table locales-table">
                    <thead><tr>
                        <th>{ui.t("locales-lang")}</th>
                        <th>{ui.t("locales-code")}</th>
                        <th>{ui.t("locales-prefix")}</th>
                        <th>{ui.t("locales-progress")}</th>
                        <th title=ui.t("locales-published-help")>{ui.t("locales-published")}</th>
                        <th class="col-actions"></th>
                    </tr></thead>
                    <tbody>
                        <tr>
                            <td class="strong">{revel_model::language_name(&primary)}</td>
                            <td class="muted path">{primary.clone()}</td>
                            <td class="muted path">"/"</td>
                            <td colspan="3"><span class="chip chip--ok">{ui.t("locales-primary")}</span></td>
                        </tr>
                        {move || locales.get().into_iter().map(|l| {
                            let (l1, l2, l3) = (l.clone(), l.clone(), l.clone());
                            let code = l.code.clone();
                            let code2 = l.code.clone();
                            let prog = Memo::new(move |_| {
                                let (d, t) = s.data.with(|data| s.cms.with(|cms| {
                                    texts.with(|tx| progress(tx, &l1, data.get(&l1.code), cms.as_ref()))
                                }));
                                (d, t, if t == 0 { 100 } else if d > 0 { (d * 100 / t).max(1) } else { 0 })
                            });
                            view! {
                                <tr data-locale=l.code.clone() class="row-link" class:is-on=move || sel.get().as_deref() == Some(code.as_str())
                                    on:click=move |_| ui.locale.set(Some(code2.clone()))>
                                    <td class="strong">{l.label()}</td>
                                    <td class="muted path">{l.code.clone()}</td>
                                    <td class="muted path">{format!("/{}/", l.code)}</td>
                                    <td>
                                        <div class="meter" title=move || { let (d, t, _) = prog.get(); ui.tx("locales-progress-help", &[("done", d.to_string()), ("total", t.to_string())]) }>
                                            <span class="meter-bar"><span class="meter-fill" style:width=move || format!("{}%", prog.get().2)></span></span>
                                            <span class="meter-num">{move || format!("{} %", prog.get().2)}</span>
                                        </div>
                                    </td>
                                    <td on:click=|e| e.stop_propagation()>
                                        <span class="switch" title=ui.t("locales-published-help")>
                                            <input type="checkbox" prop:checked=!l.draft aria-label=ui.t("locales-published")
                                                on:change=move |e| {
                                                    let on = event_target_checked(&e);
                                                    s.apply.run(Command::SetLocale { code: l2.code.clone(), name: l2.name.clone(), draft: !on });
                                                }/>
                                            <span class="switch-track"></span>
                                        </span>
                                    </td>
                                    <td class="col-actions" on:click=|e| e.stop_propagation()>
                                        <button class="icon-btn icon-btn--small" title=ui.t("locales-rename") aria-label=ui.t("locales-rename")
                                            on:click=move |_| rename(l3.clone())>{icons::edit()}</button>
                                        <button class="icon-btn icon-btn--small" title=ui.t("locales-remove") aria-label=ui.t("locales-remove")
                                            on:click={ let l = l.clone(); move |_| remove(l.clone()) }>{icons::trash()}</button>
                                    </td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
            {move || (no_url() && !locales.with(Vec::is_empty)).then(|| view! {
                <p class="muted help">{ui.t("locales-no-url")}</p>
            })}
            {move || sel.get().map(|code| view! {
                <div class="section locales-texts">
                    <div class="section-head">
                        <div class="segmented" role="tablist">
                            {[(Part::Pages, "locales-part-pages", "locales-part-pages-help"),
                              (Part::Data, "locales-part-data", "locales-part-data-help"),
                              (Part::Entries, "locales-part-entries", "locales-part-entries-help")]
                                .into_iter().map(|(p, key, help)| view! {
                                    <button role="tab" class:is-on=move || part.get() == p title=ui.t(help)
                                        aria-selected=move || (part.get() == p).to_string()
                                        on:click=move |_| part.set(p)>{ui.t(key)}</button>
                                }).collect_view()}
                        </div>
                        <div class="toolbar">
                            <label class="panel-search">
                                {icons::search()}
                                <input type="search" placeholder=ui.t("locales-search") aria-label=ui.t("locales-search")
                                    prop:value=move || query.get() on:input=move |e| query.set(event_target_value(&e))/>
                            </label>
                            <button class="tool-toggle locales-todo" class:is-on=move || todo_only.get()
                                aria-pressed=move || todo_only.get().to_string()
                                title=ui.t("locales-todo-help") aria-label=ui.t("locales-todo")
                                on:click=move |_| todo_only.update(|t| *t = !*t)>{icons::filter()}</button>
                        </div>
                    </div>
                    {
                        let code = StoredValue::new(code);
                        move || match part.get() {
                            Part::Pages => view! { <PageTexts s=s code=code.get_value() texts=texts todo_only=todo_only query=query/> }.into_any(),
                            Part::Data => view! { <DataTexts s=s code=code.get_value() todo_only=todo_only query=query/> }.into_any(),
                            Part::Entries => view! { <EntryTexts s=s code=code.get_value() todo_only=todo_only query=query/> }.into_any(),
                        }
                    }
                </div>
            })}
        </section>
    }
}

/// Ferme le menu `<details>` qui contient l'élément cliqué.
fn close_details(e: &leptos::ev::MouseEvent) {
    use wasm_bindgen::JsCast;
    if let Some(d) = e
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| el.closest("details").ok().flatten())
    {
        let _ = d.remove_attribute("open");
    }
}

/// Champ de traduction : une ligne, plusieurs pour un texte long ; écrit à la sortie du champ.
fn translation_input(
    ui: Ui,
    source: &str,
    value: Option<String>,
    on_set: impl Fn(String) + 'static,
) -> AnyView {
    let long = source.len() > 70 || source.contains('\n');
    let placeholder = source.to_string();
    let value = value.unwrap_or_default();
    if long {
        view! {
            <textarea class="input loc-input" rows="3" placeholder=placeholder aria-label=ui.t("locales-translation")
                prop:value=value on:change=move |e| on_set(event_target_value(&e))></textarea>
        }
        .into_any()
    } else {
        view! {
            <input class="input loc-input" placeholder=placeholder aria-label=ui.t("locales-translation")
                prop:value=value on:change=move |e| on_set(event_target_value(&e))/>
        }
        .into_any()
    }
}

/// Textes écrits dans les pages, gabarits et composants.
#[component]
fn PageTexts(
    s: LocSession,
    code: String,
    texts: Memo<Vec<SiteText>>,
    todo_only: RwSignal<bool>,
    query: RwSignal<String>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let code = StoredValue::new(code);
    let names = move |o: &revel_edit::Owner| -> String {
        s.doc.with_untracked(|d| {
            let d = d.as_ref();
            match o {
                revel_edit::Owner::Page(id) => d
                    .and_then(|d| d.page(id))
                    .map(|p| p.name.clone().unwrap_or_else(|| p.path.clone()))
                    .unwrap_or_else(|| id.clone()),
                revel_edit::Owner::Layout(id) => {
                    ui.now("locales-place-layout", &[("name", id.clone())])
                }
                revel_edit::Owner::Component(id) => d
                    .and_then(|d| d.component(id))
                    .map(|c| c.label())
                    .unwrap_or_else(|| id.clone()),
            }
        })
    };
    let rows = move || {
        let q = query.get().trim().to_lowercase();
        let tr = s.doc.with(|d| {
            d.as_ref()
                .and_then(|d| d.locale(&code.get_value()))
                .map(|l| l.texts.clone())
                .unwrap_or_default()
        });
        texts
            .get()
            .into_iter()
            .filter(|t| !todo_only.get() || !tr.contains_key(&t.source))
            .filter(|t| {
                q.is_empty()
                    || t.source.to_lowercase().contains(&q)
                    || tr
                        .get(&t.source)
                        .is_some_and(|x| x.to_lowercase().contains(&q))
            })
            .map(|t| (tr.get(&t.source).cloned(), t))
            .collect::<Vec<_>>()
    };
    view! {
        <table class="table loc-table">
            <thead><tr>
                <th>{move || revel_model::language_name(&s.doc.with(|d| d.as_ref().map(|d| d.default_locale.clone()).unwrap_or_default()))}</th>
                <th>{move || revel_model::language_name(&code.get_value())}</th>
            </tr></thead>
            <tbody>
                {move || {
                    let list = rows();
                    if list.is_empty() {
                        return view! { <tr><td colspan="2" class="muted loc-empty">{ui.t("locales-none")}</td></tr> }.into_any();
                    }
                    list.into_iter().map(|(value, t)| {
                        let places = t.places.iter().map(names).collect::<Vec<_>>().join(" · ");
                        let source = t.source.clone();
                        let done = value.is_some();
                        view! {
                            <tr class:is-done=done>
                                <td class="loc-source" title=places>{t.source.clone()}</td>
                                <td>{translation_input(ui, &t.source, value, move |v| {
                                    s.apply.run(Command::SetTranslation {
                                        locale: code.get_value(),
                                        source: source.clone(),
                                        text: Some(v),
                                    })
                                })}</td>
                            </tr>
                        }
                    }).collect_view().into_any()
                }}
            </tbody>
        </table>
    }
}

/// Textes des fichiers de données, écrits dans le dossier de la langue.
#[component]
fn DataTexts(
    s: LocSession,
    code: String,
    todo_only: RwSignal<bool>,
    query: RwSignal<String>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let code = StoredValue::new(code);
    let label = |path: &[Seg]| {
        path.iter()
            .map(|p| match p {
                Seg::Key(k) => k.clone(),
                Seg::Index(i) => (i + 1).to_string(),
            })
            .collect::<Vec<_>>()
            .join(" › ")
    };
    let rows = move || {
        let q = query.get().trim().to_lowercase();
        s.data.with(|d| {
            d.get(&code.get_value())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|t| !todo_only.get() || !data_done(t))
                .filter(|t| {
                    q.is_empty()
                        || t.source.to_lowercase().contains(&q)
                        || t.text
                            .as_ref()
                            .is_some_and(|x| x.to_lowercase().contains(&q))
                })
                .collect::<Vec<_>>()
        })
    };
    view! {
        <table class="table loc-table">
            <tbody>
                {move || {
                    let list = rows();
                    if list.is_empty() {
                        return view! { <tr><td colspan="2" class="muted loc-empty">{ui.t("locales-none")}</td></tr> }.into_any();
                    }
                    list.into_iter().map(|t| {
                        let done = data_done(&t);
                        let path = t.path.clone();
                        let value = t.text.clone().filter(|_| done);
                        view! {
                            <tr class:is-done=done>
                                <td class="loc-source">
                                    <span class="loc-key muted path">{label(&t.path)}</span>
                                    <span>{t.source.clone()}</span>
                                </td>
                                <td>{translation_input(ui, &t.source, value, move |v| {
                                    let binding = Binding {
                                        source: Source::LocaleData { locale: code.get_value() },
                                        path: path.clone(),
                                    };
                                    // Clé absente du fichier de la langue : créée (chemins sans liste).
                                    let keys_only = path.iter().all(|p| matches!(p, Seg::Key(_)));
                                    s.apply.run(if keys_only {
                                        Command::SetDataValue { binding, value: Some(serde_json::Value::String(v)) }
                                    } else {
                                        Command::SetBound { binding, value: v }
                                    })
                                })}</td>
                            </tr>
                        }
                    }).collect_view().into_any()
                }}
            </tbody>
        </table>
    }
}

/// Contenus des collections : traduits ou non ; traduits dans la vue CMS.
#[component]
fn EntryTexts(
    s: LocSession,
    code: String,
    todo_only: RwSignal<bool>,
    query: RwSignal<String>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let code = StoredValue::new(code);
    let open = move |col: String, file: String| {
        ui.locale.set(Some(code.get_value()));
        ui.goto_entry.set(Some((col, file)));
        ui.tab(Tab::Cms);
    };
    view! {
        <table class="table loc-table">
            <tbody>
                {move || {
                    let q = query.get().trim().to_lowercase();
                    let cols = s.cms.get().unwrap_or_default();
                    let mut rows = Vec::new();
                    for c in &cols {
                        let own = c.locales.get(&code.get_value()).cloned().unwrap_or_default();
                        for e in &c.entries {
                            let done = own.iter().any(|o| o.file == e.file);
                            let title = own
                                .iter()
                                .find(|o| o.file == e.file)
                                .map(|o| entry_title(c, o))
                                .filter(|t| t != &e.file)
                                .unwrap_or_else(|| entry_title(c, e));
                            if (todo_only.get() && done)
                                || (!q.is_empty() && !title.to_lowercase().contains(&q))
                            {
                                continue;
                            }
                            rows.push((c.id.clone(), crate::cms::col_label(c), e.file.clone(), title, done));
                        }
                    }
                    if rows.is_empty() {
                        return view! { <tr><td colspan="3" class="muted loc-empty">{ui.t("locales-none")}</td></tr> }.into_any();
                    }
                    rows.into_iter().map(|(col, label, file, title, done)| {
                        let (c2, f2) = (col.clone(), file.clone());
                        view! {
                            <tr class="row-link" class:is-done=done data-entry=file.clone() on:click=move |_| open(c2.clone(), f2.clone())>
                                <td class="strong">{title}</td>
                                <td class="muted">{label}</td>
                                <td class="loc-state">
                                    {if done {
                                        view! { <span class="chip chip--ok">{ui.t("locales-entry-done")}</span> }.into_any()
                                    } else {
                                        view! { <span class="chip chip--todo">{ui.t("cms-untranslated")}</span> }.into_any()
                                    }}
                                </td>
                            </tr>
                        }
                    }).collect_view().into_any()
                }}
            </tbody>
        </table>
    }
}
