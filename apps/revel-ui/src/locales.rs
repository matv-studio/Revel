//! Vue Localisation (3.8) : langues du site (ajout, nom, publication, retrait), avancement de la
//! traduction et textes à traduire, comme la localisation de Webflow.
//!
//! - **Pages** : textes écrits dans les pages, gabarits et composants (`Locale::texts`,
//!   commande `SetTranslation`) ;
//! - **Données** : textes des fichiers de données, écrits dans le dossier de la langue
//!   (`_data/en/…`, copié au besoin par le cœur) ;
//! - **Contenus** : contenus des collections, traduits dans la vue CMS (dossier de la langue).
//!
//! Traduction automatique hors ligne (D-36, [`crate::autotr`]) : textes pas encore traduits,
//! marqués « à relire » jusqu'à ce qu'ils soient relus (validés ou modifiés).

use crate::autotr::{self, Unit};
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
pub(crate) const COMMON: &[&str] = &[
    "en", "fr", "de", "es", "it", "pt", "nl", "ca", "pl", "sv", "da", "ja", "zh", "ar",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Pages,
    Data,
    Entries,
}

/// Données de la vue : document, textes des fichiers de données par langue, contenus, langues
/// traduisibles automatiquement.
#[derive(Clone, Copy)]
struct LocSession {
    doc: RwSignal<Option<revel_model::Site>>,
    data: RwSignal<BTreeMap<String, Vec<DataText>>>,
    cms: RwSignal<Option<Vec<CmsCollection>>>,
    /// Traduction automatique possible vers chaque langue (modèles livrés).
    can: RwSignal<BTreeMap<String, bool>>,
    apply: Callback<Command>,
}

fn session(ui: Ui, path: String) -> LocSession {
    let doc = RwSignal::new(None::<revel_model::Site>);
    let data = RwSignal::new(BTreeMap::<String, Vec<DataText>>::new());
    let cms = RwSignal::new(None::<Vec<CmsCollection>>);
    let can = RwSignal::new(BTreeMap::<String, bool>::new());
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
            let primary = o.site.default_locale.clone();
            doc.set(Some(o.site));
            let mut all = BTreeMap::new();
            let mut ok = BTreeMap::new();
            for c in codes {
                if let Ok(list) = api::edit_locale_data(&p, &c).await {
                    all.insert(c.clone(), list);
                }
                // Relue à chaque fois : une langue peut avoir été téléchargée entre-temps.
                ok.insert(c.clone(), api::translate_can(&primary, &c).await);
            }
            if !can.is_disposed() {
                can.set(ok);
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
    // Traduction automatique en cours : textes relus après chaque lot écrit.
    Effect::new(move |prev: Option<Option<usize>>| {
        let now = ui.auto.with(|j| j.as_ref().map(|j| j.done));
        if prev.is_some_and(|p| p != now) {
            reload();
        }
        now
    });
    Effect::new(move |prev: Option<u32>| {
        let n = ui.undo_rev.get();
        if prev.is_some_and(|p| p != n) {
            reload();
        }
        n
    });
    let apply = Callback::new(move |cmd: Command| {
        let p = path.get_value();
        spawn_local(async move {
            if let Err(e) = api::edit_apply(&p, &cmd, false).await {
                ui.fail(e);
            }
            ui.doc_rev.update(|n| *n += 1);
            if !doc.is_disposed() {
                reload();
            }
        });
    });
    LocSession {
        doc,
        data,
        cms,
        can,
        apply,
    }
}

/// Valeur traduite d'un texte de données : différente du texte d'origine.
fn data_done(t: &DataText, kept: &std::collections::BTreeSet<String>) -> bool {
    t.text.as_ref().is_some_and(|x| x != &t.source)
        || kept.contains(&revel_edit::locale::data_key(&t.path))
}

/// Titre d'un contenu (champ titre habituel, sinon nom du fichier).
fn entry_title(c: &CmsCollection, e: &revel_protocol::CmsEntry) -> String {
    crate::cms::primary_of(c)
        .and_then(|k| e.data.get(&k).and_then(|v| v.as_str()).map(str::to_string))
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| e.file.clone())
}

/// Avancement d'une langue : (traduits, dont traductions automatiques à relire, total).
fn progress(
    texts: &[SiteText],
    l: &revel_model::Locale,
    data: Option<&Vec<DataText>>,
    cms: Option<&Vec<CmsCollection>>,
) -> (usize, usize, usize) {
    use revel_edit::locale::{data_key, entry_key, text_key};
    let mut done = 0;
    let mut auto = 0;
    for t in texts.iter().filter(|t| l.texts.contains_key(&t.source)) {
        done += 1;
        auto += usize::from(l.machine.contains(&text_key(&t.source)));
    }
    let mut total = texts.len();
    if let Some(d) = data {
        total += d.len();
        for t in d.iter().filter(|t| data_done(t, &l.kept)) {
            done += 1;
            auto += usize::from(l.machine.contains(&data_key(&t.path)));
        }
    }
    for c in cms.into_iter().flatten() {
        total += c.entries.len();
        if let Some(own) = c.locales.get(&l.code) {
            for e in c
                .entries
                .iter()
                .filter(|e| own.iter().any(|o| o.file == e.file))
            {
                done += 1;
                auto += usize::from(l.machine.contains(&entry_key(&c.folder, &e.file)));
            }
        }
    }
    (done, auto, total)
}

/// Textes pas encore traduits d'une langue, par partie : pages, données, contenus.
fn untranslated(
    texts: &[SiteText],
    l: &revel_model::Locale,
    data: Option<&Vec<DataText>>,
    cms: Option<&Vec<CmsCollection>>,
) -> [Vec<Unit>; 3] {
    let pages = texts
        .iter()
        .filter(|t| !l.texts.contains_key(&t.source))
        .map(|t| Unit::Text {
            source: t.source.clone(),
        })
        .collect();
    let data = data
        .into_iter()
        .flatten()
        .filter(|t| !data_done(t, &l.kept))
        .map(|t| Unit::Data {
            path: t.path.clone(),
            text: t.source.clone(),
        })
        .collect();
    let mut entries = Vec::new();
    for c in cms.into_iter().flatten() {
        let own = c.locales.get(&l.code);
        for e in &c.entries {
            if own.is_some_and(|o| o.iter().any(|o| o.file == e.file)) {
                continue;
            }
            entries.push(Unit::Entry {
                folder: c.folder.clone(),
                file: e.file.clone(),
                data: e.data.clone(),
                body: e.body.clone(),
                fields: c.fields.clone(),
            });
        }
    }
    [pages, data, entries]
}

/// Durée restante lisible.
fn eta(ui: Ui, secs: Option<u64>) -> String {
    match secs {
        None => ui.now("locales-auto-eta-wait", &[]),
        Some(s) if s < 60 => ui.now("locales-auto-eta-less", &[]),
        Some(s) => ui.now("locales-auto-eta-min", &[("n", s.div_ceil(60).to_string())]),
    }
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
    let review_only = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    // Fenêtre « Traduire automatiquement » : langue, parties choisies, mots gardés.
    let dlg = RwSignal::new(None::<String>);
    let dlg_parts = RwSignal::new([true; 3]);
    let dlg_keep = RwSignal::new(String::new());
    let site_path = StoredValue::new(site.path.clone());
    // Textes pas encore traduits de chaque partie, pour une langue.
    let todo = move |code: &str| -> Option<[Vec<Unit>; 3]> {
        s.doc.with(|d| {
            let l = d.as_ref()?.locale(code)?.clone();
            Some(s.data.with(|data| {
                s.cms
                    .with(|cms| texts.with(|tx| untranslated(tx, &l, data.get(code), cms.as_ref())))
            }))
        })
    };
    // Traduction automatique en cours sur ce site.
    let job = Memo::new(move |_| ui.auto.get().filter(|j| j.path == site_path.get_value()));
    let open_auto = move |code: String| {
        let keep = s.doc.with_untracked(|d| {
            d.as_ref()
                .map(|d| d.settings.no_translate.join(", "))
                .unwrap_or_default()
        });
        dlg_keep.set(keep);
        dlg_parts.set([true; 3]);
        dlg.set(Some(code));
    };
    let esc = window_event_listener(leptos::ev::keydown, move |e| {
        if e.key() == "Escape" && dlg.with_untracked(Option::is_some) {
            e.prevent_default();
            dlg.set(None);
        }
    });
    on_cleanup(move || esc.remove());
    let start_auto = move || {
        let Some(code) = dlg.get_untracked() else {
            return;
        };
        dlg.set(None);
        let Some(parts) = todo(&code) else {
            return;
        };
        let on = dlg_parts.get_untracked();
        let units: Vec<Unit> = parts
            .into_iter()
            .zip(on)
            .filter(|(_, on)| *on)
            .flat_map(|(u, _)| u)
            .collect();
        let terms: Vec<String> = dlg_keep
            .get_untracked()
            .split([',', ';', '\n'])
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        let (name, old, primary) = s.doc.with_untracked(|d| {
            d.as_ref()
                .map(|d| {
                    (
                        d.name.clone(),
                        d.settings.no_translate.clone(),
                        d.default_locale.clone(),
                    )
                })
                .unwrap_or_default()
        });
        if terms != old {
            s.apply.run(Command::SetNoTranslate {
                terms: terms.clone(),
            });
        }
        let mut keep = vec![name];
        keep.extend(terms);
        autotr::start(ui, site_path.get_value(), primary, code, units, keep);
    };

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
                                // Langue relue dans le document (marques « à relire » à jour).
                                let l1 = s.doc.with(|d| d.as_ref().and_then(|d| d.locale(&l1.code)).cloned()).unwrap_or_else(|| l1.clone());
                                let (d, a, t) = s.data.with(|data| s.cms.with(|cms| {
                                    texts.with(|tx| progress(tx, &l1, data.get(&l1.code), cms.as_ref()))
                                }));
                                let pct = |n: usize| if t == 0 { 0.0 } else { n as f64 * 100.0 / t as f64 };
                                let shown = if t == 0 { 100 } else if d > 0 { (d * 100 / t).max(1) } else { 0 };
                                (d, a, t, shown, pct(d - a), pct(a))
                            });
                            view! {
                                <tr data-locale=l.code.clone() class="row-link" class:is-on=move || sel.get().as_deref() == Some(code.as_str())
                                    on:click=move |_| ui.locale.set(Some(code2.clone()))>
                                    <td class="strong">{l.label()}</td>
                                    <td class="muted path">{l.code.clone()}</td>
                                    <td class="muted path">{format!("/{}/", l.code)}</td>
                                    <td>
                                        <div class="meter" title=move || {
                                            let (d, a, t, ..) = prog.get();
                                            if a > 0 {
                                                ui.tx("locales-progress-auto-help", &[("done", d.to_string()), ("total", t.to_string()), ("auto", a.to_string())])
                                            } else {
                                                ui.tx("locales-progress-help", &[("done", d.to_string()), ("total", t.to_string())])
                                            }
                                        }>
                                            <span class="meter-bar">
                                                <span class="meter-fill" style:width=move || format!("{:.2}%", if prog.get().2 == 0 { 100.0 } else { prog.get().4 })></span>
                                                <span class="meter-fill meter-fill--auto" style:width=move || format!("{:.2}%", prog.get().5)></span>
                                            </span>
                                            <span class="meter-num">{move || format!("{} %", prog.get().3)}</span>
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
                <p class="muted help" title=ui.t("locales-no-url")>{crate::icons::warning()}{ui.t("locales-no-url-short")}</p>
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
                                on:click=move |_| { review_only.set(false); todo_only.update(|t| *t = !*t) }>{icons::filter()}</button>
                            <button class="tool-toggle tool-toggle--auto locales-review" class:is-on=move || review_only.get()
                                aria-pressed=move || review_only.get().to_string()
                                title=ui.t("locales-review-only-help") aria-label=ui.t("locales-review-only")
                                on:click=move |_| { todo_only.set(false); review_only.update(|t| *t = !*t) }>{icons::sparkle()}</button>
                            {
                                let code = StoredValue::new(code.clone());
                                let state = Memo::new(move |_| {
                                    let c = code.get_value();
                                    let can = s.can.with(|m| m.get(&c).copied());
                                    let left = todo(&c).map(|p| p.iter().map(Vec::len).sum::<usize>()).unwrap_or(0);
                                    (can, left)
                                });
                                let from = move || s.doc.with(|d| d.as_ref().map(|d| revel_model::language_name(&d.default_locale)).unwrap_or_default());
                                move || (job.get().is_none()).then(|| view! {
                                    <button class="btn btn--auto locales-auto"
                                        disabled=move || { let (can, left) = state.get(); can != Some(true) || left == 0 }
                                        title=move || {
                                            let (can, left) = state.get();
                                            if can == Some(false) {
                                                ui.tx("locales-auto-none", &[("from", from()), ("to", revel_model::language_name(&code.get_value()))])
                                            } else if left == 0 {
                                                ui.tx("locales-auto-nothing", &[])
                                            } else {
                                                ui.tx("locales-auto-help", &[])
                                            }
                                        }
                                        on:click=move |_| open_auto(code.get_value())>
                                        {icons::sparkle()}{ui.t("locales-auto")}
                                    </button>
                                })
                            }
                        </div>
                    </div>
                    {move || job.get().map(|j| {
                        let pct = if j.chars_total == 0 { 0.0 } else { j.chars_done as f64 * 100.0 / j.chars_total as f64 };
                        view! {
                            <div class="auto-run" role="status">
                                <span class="auto-run-icon">{icons::sparkle()}</span>
                                <span class="auto-run-label">{ui.tx("locales-auto-running", &[("from", s.doc.with_untracked(|d| d.as_ref().map(|d| revel_model::language_name(&d.default_locale)).unwrap_or_default())), ("to", revel_model::language_name(&j.code))])}</span>
                                <span class="meter-bar auto-run-bar"><span class="meter-fill meter-fill--auto" style:width=format!("{pct:.2}%")></span></span>
                                <span class="auto-run-count">{format!("{} / {}", j.done, j.total)}</span>
                                <span class="auto-run-eta muted">{eta(ui, j.remaining())}</span>
                                <button class="btn btn--small auto-run-stop" disabled=j.stop title=ui.t("locales-auto-stop-help")
                                    on:click=move |_| ui.auto.update(|j| if let Some(j) = j { j.stop = true })>
                                    {if j.stop { ui.now("locales-auto-stopping", &[]) } else { ui.now("locales-auto-stop", &[]) }}
                                </button>
                            </div>
                        }
                    })}
                    {
                        let code = StoredValue::new(code);
                        move || match part.get() {
                            Part::Pages => view! { <PageTexts s=s code=code.get_value() texts=texts todo_only=todo_only review_only=review_only query=query/> }.into_any(),
                            Part::Data => view! { <DataTexts s=s code=code.get_value() todo_only=todo_only review_only=review_only query=query/> }.into_any(),
                            Part::Entries => view! { <EntryTexts s=s code=code.get_value() todo_only=todo_only review_only=review_only query=query/> }.into_any(),
                        }
                    }
                </div>
            })}
            {move || dlg.get().map(|code| {
                let parts = todo(&code).unwrap_or_default();
                let counts: [usize; 3] = [parts[0].len(), parts[1].len(), parts[2].len()];
                let rows = [
                    (0usize, "locales-part-pages", ui.now("locales-auto-texts", &[("n", counts[0].to_string())])),
                    (1, "locales-part-data", ui.now("locales-auto-texts", &[("n", counts[1].to_string())])),
                    (2, "locales-part-entries", ui.now("locales-auto-entries", &[("n", counts[2].to_string())])),
                ];
                let none = move || { let on = dlg_parts.get(); !(0..3).any(|i| on[i] && counts[i] > 0) };
                view! {
                    <div class="overlay" on:input=move |e| crate::dialog::mark_dirty(&e) on:mousedown=move |e| if crate::dialog::backdrop_click(&e) { dlg.set(None) }>
                        <form class="dialog auto-dialog" role="dialog" aria-modal="true" aria-labelledby="auto-title"
                            on:submit=move |e| { e.prevent_default(); if !none() { start_auto() } }>
                            <h2 id="auto-title">{ui.t("locales-auto-title")}</h2>
                            <p class="auto-pair">
                                <span>{move || s.doc.with(|d| d.as_ref().map(|d| revel_model::language_name(&d.default_locale)).unwrap_or_default())}</span>
                                {icons::arrow_right()}
                                <span class="strong">{revel_model::language_name(&code)}</span>
                            </p>
                            <div class="auto-parts">
                                {rows.into_iter().map(|(i, key, count)| view! {
                                    <label class="view-line auto-part" class:is-empty={counts[i] == 0}>
                                        <span>{ui.t(key)}</span>
                                        <span class="muted auto-part-count">{count}</span>
                                        <span class="switch">
                                            <input type="checkbox" disabled={counts[i] == 0}
                                                prop:checked={move || counts[i] > 0 && dlg_parts.get()[i]}
                                                on:change=move |e| { let v = event_target_checked(&e); dlg_parts.update(|p| p[i] = v) }/>
                                            <span class="switch-track"></span>
                                        </span>
                                    </label>
                                }).collect_view()}
                            </div>
                            <label class="field" title=ui.t("locales-auto-keep-help")>
                                <span class="label">{ui.t("locales-auto-keep")}</span>
                                <input class="input" type="text" spellcheck="false" autocomplete="off"
                                    prop:value=move || dlg_keep.get() on:input=move |e| dlg_keep.set(event_target_value(&e))/>
                            </label>
                            <div class="dialog-actions">
                                <button type="button" class="btn" on:click=move |_| dlg.set(None)>{ui.t("action-cancel")}</button>
                                <button type="submit" class="btn btn--primary" disabled=none>{icons::sparkle()}{ui.t("locales-auto-start")}</button>
                            </div>
                        </form>
                    </div>
                }
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
/// Pas encore traduit : coche à droite du champ pour garder le texte d'origine comme traduction
/// (noms propres, mots identiques dans les deux langues).
fn translation_input(
    ui: Ui,
    source: &str,
    value: Option<String>,
    on_set: impl Fn(String) + 'static,
    on_keep: Option<std::rc::Rc<dyn Fn()>>,
) -> AnyView {
    let long = source.len() > 70 || source.contains('\n');
    let placeholder = source.to_string();
    let todo = value.is_none() && !source.trim().is_empty();
    let value = value.unwrap_or_default();
    let on_set = std::rc::Rc::new(on_set);
    let keep = todo.then(|| {
        let (f, src) = (on_set.clone(), source.to_string());
        view! {
            <button type="button" class="icon-btn icon-btn--small loc-keep" title=ui.t("locales-keep-source")
                aria-label=ui.t("locales-keep-source")
                on:click=move |e| {
                    e.stop_propagation();
                    match &on_keep { Some(k) => k(), None => f(src.clone()) }
                }>{icons::check()}</button>
        }
    });
    let set = move |v: String| on_set(v);
    let field = if long {
        view! {
            <textarea class="input loc-input" class:has-keep=todo rows="3" placeholder=placeholder aria-label=ui.t("locales-translation")
                prop:value=value on:change=move |e| set(event_target_value(&e))></textarea>
        }
        .into_any()
    } else {
        view! {
            <input class="input loc-input" class:has-keep=todo placeholder=placeholder aria-label=ui.t("locales-translation")
                prop:value=value on:change=move |e| set(event_target_value(&e))/>
        }
        .into_any()
    };
    view! { {field}{keep} }.into_any()
}

/// Textes écrits dans les pages, gabarits et composants.
#[component]
fn PageTexts(
    s: LocSession,
    code: String,
    texts: Memo<Vec<SiteText>>,
    todo_only: RwSignal<bool>,
    review_only: RwSignal<bool>,
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
                    .map(crate::names::component_name)
                    .unwrap_or_else(|| id.clone()),
            }
        })
    };
    let rows = move || {
        let q = query.get().trim().to_lowercase();
        let (tr, machine) = s.doc.with(|d| {
            d.as_ref()
                .and_then(|d| d.locale(&code.get_value()))
                .map(|l| (l.texts.clone(), l.machine.clone()))
                .unwrap_or_default()
        });
        let auto = |t: &SiteText| machine.contains(&revel_edit::locale::text_key(&t.source));
        texts
            .get()
            .into_iter()
            .filter(|t| !todo_only.get() || !tr.contains_key(&t.source))
            .filter(|t| !review_only.get() || auto(t))
            .filter(|t| {
                q.is_empty()
                    || t.source.to_lowercase().contains(&q)
                    || tr
                        .get(&t.source)
                        .is_some_and(|x| x.to_lowercase().contains(&q))
            })
            .map(|t| (tr.get(&t.source).cloned(), auto(&t), t))
            .collect::<Vec<_>>()
    };
    let review_all = move |keys: Vec<String>| {
        s.apply.run(Command::MarkMachine {
            locale: code.get_value(),
            keys,
            on: false,
        })
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
                    let auto_keys: Vec<String> = list.iter().filter(|r| r.1).map(|r| revel_edit::locale::text_key(&r.2.source)).collect();
                    let rows = list.into_iter().map(|(value, auto, t)| {
                        let places = t.places.iter().map(names).collect::<Vec<_>>().join(" · ");
                        let source = t.source.clone();
                        let key = revel_edit::locale::text_key(&t.source);
                        let done = value.is_some();
                        view! {
                            <tr class:is-done=done class:is-auto=auto>
                                <td class="loc-source" title=places>{t.source.clone()}</td>
                                <td><div class="loc-field">
                                    {translation_input(ui, &t.source, value, move |v| {
                                        s.apply.run(Command::SetTranslation {
                                            locale: code.get_value(),
                                            source: source.clone(),
                                            text: Some(v),
                                            machine: false,
                                        })
                                    }, None)}
                                    {auto.then(|| review_button(ui, move || review_all(vec![key.clone()])))}
                                </div></td>
                            </tr>
                        }
                    }).collect_view();
                    view! {
                        {review_bar(ui, review_only, auto_keys, review_all)}
                        {rows}
                    }.into_any()
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
    review_only: RwSignal<bool>,
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
    let machine = move || {
        s.doc.with(|d| {
            d.as_ref()
                .and_then(|d| d.locale(&code.get_value()))
                .map(|l| l.machine.clone())
                .unwrap_or_default()
        })
    };
    let review_all = move |keys: Vec<String>| {
        s.apply.run(Command::MarkMachine {
            locale: code.get_value(),
            keys,
            on: false,
        })
    };
    // Textes gardés tels quels dans cette langue (coche « garder le texte d'origine »).
    let kept = move || {
        s.doc.with(|d| {
            d.as_ref()
                .and_then(|d| d.locales.iter().find(|l| l.code == code.get_value()))
                .map(|l| l.kept.clone())
                .unwrap_or_default()
        })
    };
    let rows = move || {
        let q = query.get().trim().to_lowercase();
        let machine = machine();
        let kept = kept();
        s.data.with(|d| {
            d.get(&code.get_value())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|t| !todo_only.get() || !data_done(t, &kept))
                .filter(|t| {
                    !review_only.get() || machine.contains(&revel_edit::locale::data_key(&t.path))
                })
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
                    let machine = machine();
                    let auto_keys: Vec<String> = list
                        .iter()
                        .map(|t| revel_edit::locale::data_key(&t.path))
                        .filter(|k| machine.contains(k))
                        .collect();
                    let kept = kept();
                    let rows = list.into_iter().map(|t| {
                        let done = data_done(&t, &kept);
                        let path = t.path.clone();
                        let key = revel_edit::locale::data_key(&t.path);
                        let auto = done && machine.contains(&key);
                        let value = t.text.clone().filter(|_| done);
                        view! {
                            <tr class:is-done=done class:is-auto=auto>
                                <td class="loc-source">
                                    <span class="loc-key muted path">{label(&t.path)}</span>
                                    <span>{t.source.clone()}</span>
                                </td>
                                <td><div class="loc-field">{
                                    // Écriture de la traduction (clé absente du fichier de la langue :
                                    // créée, chemins sans liste).
                                    let write = {
                                        let path = path.clone();
                                        move |v: String| {
                                            let binding = Binding {
                                                source: Source::LocaleData { locale: code.get_value() },
                                                path: path.clone(),
                                            };
                                            let keys_only = path.iter().all(|p| matches!(p, Seg::Key(_)));
                                            if keys_only {
                                                Command::SetDataValue { binding, value: Some(serde_json::Value::String(v)) }
                                            } else {
                                                Command::SetBound { binding, value: v }
                                            }
                                        }
                                    };
                                    let write2 = write.clone();
                                    let (src, k2) = (t.source.clone(), key.clone());
                                    // Garder le texte d'origine : écrit tel quel et marqué gardé (une étape).
                                    let keep: std::rc::Rc<dyn Fn()> = std::rc::Rc::new(move || {
                                        s.apply.run(Command::Batch { commands: vec![
                                            write2(src.clone()),
                                            Command::MarkKept { locale: code.get_value(), keys: vec![k2.clone()], on: true },
                                        ] })
                                    });
                                    translation_input(ui, &t.source, value, move |v| s.apply.run(write(v)), Some(keep))
                                }
                                {auto.then(|| review_button(ui, move || review_all(vec![key.clone()])))}
                                </div></td>
                            </tr>
                        }
                    }).collect_view();
                    view! {
                        {review_bar(ui, review_only, auto_keys, review_all)}
                        {rows}
                    }.into_any()
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
    review_only: RwSignal<bool>,
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
                    let machine = s.doc.with(|d| {
                        d.as_ref()
                            .and_then(|d| d.locale(&code.get_value()))
                            .map(|l| l.machine.clone())
                            .unwrap_or_default()
                    });
                    let mut rows = Vec::new();
                    let mut auto_keys = Vec::new();
                    for c in &cols {
                        let own = c.locales.get(&code.get_value()).cloned().unwrap_or_default();
                        for e in &c.entries {
                            let done = own.iter().any(|o| o.file == e.file);
                            let key = revel_edit::locale::entry_key(&c.folder, &e.file);
                            let auto = done && machine.contains(&key);
                            if review_only.get() && !auto {
                                continue;
                            }
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
                            if auto {
                                auto_keys.push(key.clone());
                            }
                            rows.push((c.id.clone(), crate::cms::col_label(c), e.file.clone(), title, done, auto, key));
                        }
                    }
                    if rows.is_empty() {
                        return view! { <tr><td colspan="3" class="muted loc-empty">{ui.t("locales-none")}</td></tr> }.into_any();
                    }
                    let review_all = move |keys: Vec<String>| {
                        s.apply.run(Command::MarkMachine { locale: code.get_value(), keys, on: false })
                    };
                    let rows = rows.into_iter().map(|(col, label, file, title, done, auto, key)| {
                        let (c2, f2) = (col.clone(), file.clone());
                        view! {
                            <tr class="row-link" class:is-done=done class:is-auto=auto data-entry=file.clone() on:click=move |_| open(c2.clone(), f2.clone())>
                                <td class="strong">{title}</td>
                                <td class="muted">{label}</td>
                                <td class="loc-state">
                                    {if auto {
                                        view! {
                                            <span class="loc-state-auto" on:click=|e| e.stop_propagation()>
                                                <span class="chip chip--auto" title=ui.t("locales-review-help")>{icons::sparkle()}{ui.t("locales-review")}</span>
                                                {review_button(ui, move || review_all(vec![key.clone()]))}
                                            </span>
                                        }.into_any()
                                    } else if done {
                                        view! { <span class="chip chip--ok">{ui.t("locales-entry-done")}</span> }.into_any()
                                    } else {
                                        view! { <span class="chip chip--todo">{ui.t("cms-untranslated")}</span> }.into_any()
                                    }}
                                </td>
                            </tr>
                        }
                    }).collect_view();
                    view! {
                        {review_bar(ui, review_only, auto_keys, review_all)}
                        {rows}
                    }.into_any()
                }}
            </tbody>
        </table>
    }
}

/// Bouton « Marquer comme relu » d'une traduction automatique.
fn review_button(ui: Ui, on: impl Fn() + 'static) -> AnyView {
    view! {
        <button type="button" class="icon-btn icon-btn--small loc-review" title=ui.t("locales-review-ok")
            aria-label=ui.t("locales-review-ok") on:click=move |e| { e.stop_propagation(); on() }>
            {icons::check()}
        </button>
    }
    .into_any()
}

/// Ligne « Tout marquer comme relu » en tête de liste (filtre « À relire »).
fn review_bar(
    ui: Ui,
    review_only: RwSignal<bool>,
    keys: Vec<String>,
    on: impl Fn(Vec<String>) + 'static,
) -> Option<AnyView> {
    (review_only.get_untracked() && !keys.is_empty()).then(|| {
        let n = keys.len();
        view! {
            <tr class="loc-review-bar">
                <td colspan="3">
                    <span class="chip chip--auto">{icons::sparkle()}{ui.t("locales-review")}</span>
                    <span class="muted">{n}</span>
                    <button type="button" class="btn btn--small" on:click=move |_| on(keys.clone())>
                        {icons::check()}{ui.t("locales-review-all")}
                    </button>
                </td>
            </tr>
        }
        .into_any()
    })
}
