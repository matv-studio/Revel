//! Import de pages et de composants, export de composants (D-206, maquettes `import-1` à
//! `import-3` validées le 24/11/2026).
//!
//! « Importer des pages… » (panneau Pages) : depuis un autre site Revel ou des fichiers HTML.
//! « Importer un composant… » (panneau Composants) : depuis un autre site Revel, un fichier de
//! composants (`.revel-component`) ou des fichiers HTML. « Exporter le composant… » (menu d'un
//! composant) écrit un fichier de composants. Le bilan (pages, composants, médias) est calculé
//! par le cœur à chaque choix ; l'import est une seule étape d'annulation.

use crate::editor::Ed;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{ImportChoice, ImportList, ImportPlan, ImportSource};

/// Ce qu'on importe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Pages,
    Components,
}

/// Source choisie dans la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    /// Autre site Revel.
    Site,
    /// Fichier de composants.
    File,
    /// Fichiers HTML.
    Html,
}

/// Extension d'un fichier de composants.
const COMPONENT_EXT: &str = "revel-component";

/// Ouvre la fenêtre d'import.
pub fn open(ed: Ed, mode: Mode) {
    ed.import.set(Some(mode));
}

/// Exporte le composant `id` dans un fichier choisi.
pub fn export(ui: Ui, ed: Ed, id: &str, label: &str) {
    let path = ed.path.get_value();
    let id = id.to_string();
    let name = {
        let s = revel_model::slugify(label);
        if s.is_empty() { id.clone() } else { s }
    };
    spawn_local(async move {
        let label = ui.now("component-file", &[]);
        let dest =
            match api::pick_save(&label, COMPONENT_EXT, &format!("{name}.{COMPONENT_EXT}")).await {
                Ok(Some(d)) => d,
                Ok(None) => return,
                Err(e) => return ui.fail(e),
            };
        if let Err(e) = api::export_components(&path, &[id], &dest).await {
            ui.fail(e);
        }
    });
}

fn has_ext(p: &str, exts: &[&str]) -> bool {
    let l = p.to_ascii_lowercase();
    exts.iter().any(|e| l.ends_with(&format!(".{e}")))
}

fn file_name(p: &str) -> String {
    p.rsplit(['/', '\\']).next().unwrap_or(p).to_string()
}

/// Fenêtre d'import (montée dans l'éditeur).
#[component]
pub fn ImportDialog() -> impl IntoView {
    let ed = expect_context::<Ed>();
    move || ed.import.get().map(|mode| view! { <ImportBox mode/> })
}

#[component]
fn ImportBox(mode: Mode) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let pages = mode == Mode::Pages;
    let tab = RwSignal::new(Tab::Site);
    let site = RwSignal::new(None::<String>);
    // Dossier choisi hors des sites récents (affiché dans la liste).
    let other = RwSignal::new(None::<String>);
    let files = RwSignal::new(Vec::<String>::new());
    let list = RwSignal::new(None::<ImportList>);
    let loading = RwSignal::new(false);
    let checked = RwSignal::new(Vec::<String>::new());
    let bring = RwSignal::new(true);
    let plan = RwSignal::new(None::<ImportPlan>);
    let busy = RwSignal::new(false);
    let over = RwSignal::new(false);
    let list_gen = StoredValue::new(0u32);
    let plan_gen = StoredValue::new(0u32);
    let me = ed.path.get_value();
    let close = move || ed.import.set(None);
    if ui.recents.with_untracked(Vec::is_empty) {
        ui.reload_recents();
    }
    let sites = Memo::new(move |_| {
        ui.recents.with(|r| {
            r.iter()
                .filter(|s| s.path != me)
                .map(|s| (s.path.clone(), s.name.clone()))
                .collect::<Vec<_>>()
        })
    });
    let source = Memo::new(move |_| match tab.get() {
        Tab::Site => site.get().map(|path| ImportSource::Site { path }),
        Tab::Html => (!files.with(Vec::is_empty)).then(|| ImportSource::Html {
            paths: files.get(),
            components: !pages,
        }),
        Tab::File => files
            .with(|f| f.first().cloned())
            .map(|path| ImportSource::Component { path }),
    });
    // Source changée : liste relue ; fichiers donnés cochés d'office, pages d'un site au choix.
    Effect::new(move |_| {
        let Some(src) = source.get() else {
            list.set(None);
            checked.set(Vec::new());
            return;
        };
        let n = list_gen.get_value() + 1;
        list_gen.set_value(n);
        loading.set(true);
        let path = ed.path.get_value();
        spawn_local(async move {
            let r = api::import_list(&path, &src).await;
            if list_gen.get_value() != n {
                return;
            }
            loading.set(false);
            match r {
                Ok(l) => {
                    let items = if pages { &l.pages } else { &l.components };
                    let all = !matches!(src, ImportSource::Site { .. }) || items.len() == 1;
                    checked.set(if all {
                        items.iter().map(|i| i.id.clone()).collect()
                    } else {
                        Vec::new()
                    });
                    list.set(Some(l));
                }
                Err(e) => {
                    list.set(None);
                    checked.set(Vec::new());
                    ui.fail(e);
                }
            }
        });
    });
    let choice = move || {
        let ids = checked.get();
        let b = bring.get();
        match (pages, tab.get()) {
            (true, Tab::Site) => ImportChoice {
                pages: ids,
                components: Vec::new(),
                uses: b,
                styles: b,
                media: b,
            },
            // Fichiers HTML : leurs styles toujours repris, leurs images et polices au choix.
            (true, _) => ImportChoice {
                pages: ids,
                components: Vec::new(),
                uses: true,
                styles: true,
                media: b,
            },
            (false, _) => ImportChoice {
                pages: Vec::new(),
                components: ids,
                uses: true,
                styles: b,
                media: b,
            },
        }
    };
    // Bilan recalculé à chaque choix.
    Effect::new(move |_| {
        let c = choice();
        let src = source.get();
        let ready = list.with(Option::is_some);
        let n = plan_gen.get_value() + 1;
        plan_gen.set_value(n);
        let Some(src) = src.filter(|_| ready) else {
            plan.set(None);
            return;
        };
        let path = ed.path.get_value();
        spawn_local(async move {
            let r = api::import_plan(&path, &src, &c).await;
            if plan_gen.get_value() == n {
                plan.set(r.ok());
            }
        });
    });
    let count = move || checked.with(Vec::len);
    let set_tab = move |t: Tab| {
        if tab.get_untracked() != t {
            files.set(Vec::new());
            list.set(None);
            plan.set(None);
            checked.set(Vec::new());
            tab.set(t);
        }
    };
    // Fichiers donnés (dépôt, sélecteur) : ceux du bon type.
    let take = move |paths: Vec<String>| {
        let t = tab.get_untracked();
        let ok: Vec<String> = paths
            .into_iter()
            .filter(|p| match t {
                Tab::File => has_ext(p, &[COMPONENT_EXT]),
                // Dossier (sans extension de fichier) ou page HTML.
                _ => {
                    has_ext(p, &["html", "htm"])
                        || !file_name(p).contains('.')
                        || std::path::Path::new(p).extension().is_none()
                }
            })
            .collect();
        if ok.is_empty() {
            ui.fail(revel_protocol::Failure::new(if t == Tab::File {
                "error-import-drop-component"
            } else {
                "error-import-drop-html"
            }));
            return;
        }
        if t == Tab::File {
            files.set(ok.into_iter().take(1).collect());
        } else {
            files.update(|f| {
                for p in ok {
                    if !f.contains(&p) {
                        f.push(p);
                    }
                }
            });
        }
    };
    let browse = move |_| {
        let t = tab.get_untracked();
        spawn_local(async move {
            let r = if t == Tab::File {
                api::pick_files(&ui.now("component-file", &[]), &[COMPONENT_EXT], false).await
            } else {
                api::pick_files("HTML", &["html", "htm"], true).await
            };
            match r {
                Ok(p) if !p.is_empty() => take(p),
                Ok(_) => {}
                Err(e) => ui.fail(e),
            }
        });
    };
    let on_drop = move |e: leptos::ev::DragEvent| {
        over.set(false);
        if !crate::files::has_files(&e) {
            return;
        }
        e.prevent_default();
        let Some(dt) = e.data_transfer() else { return };
        let mut names: Vec<String> = crate::files::list(dt.files())
            .iter()
            .map(web_sys::File::name)
            .collect();
        names.extend(crate::files::dropped_folders(&dt));
        names.sort();
        names.dedup();
        let posted = crate::files::post_dropped(&dt);
        spawn_local(async move {
            let mut paths = Vec::new();
            if posted {
                for _ in 0..20 {
                    if let Ok(p) = api::dropped_paths().await
                        && !p.is_empty()
                    {
                        paths = p;
                        break;
                    }
                    crate::sleep_ms(50).await;
                }
            }
            if paths.is_empty() {
                paths = api::find_dropped(&names).await.unwrap_or_default();
            }
            if paths.is_empty() {
                ui.fail(
                    revel_protocol::Failure::new("error-drop-folder-unknown")
                        .arg("name", names.join(", ")),
                );
            } else {
                take(paths);
            }
        });
    };
    let pick_site = move |e: leptos::ev::Event| {
        let v = event_target_value(&e);
        if v == "__other" {
            spawn_local(async move {
                match api::pick_folder().await {
                    Ok(Some(p)) => {
                        if !sites.with_untracked(|s| s.iter().any(|(x, _)| *x == p)) {
                            other.set(Some(p.clone()));
                        }
                        site.set(Some(p));
                    }
                    Ok(None) => site.notify(),
                    Err(e) => ui.fail(e),
                }
            });
        } else if !v.is_empty() {
            site.set(Some(v));
        }
    };
    let go = move |_| {
        let Some(src) = source.get_untracked() else {
            return;
        };
        let c = untrack(choice);
        if busy.get_untracked() || (c.pages.is_empty() && c.components.is_empty()) {
            return;
        }
        busy.set(true);
        let path = ed.path.get_value();
        spawn_local(async move {
            let r = api::import_apply(&path, &src, &c).await;
            busy.set(false);
            match r {
                Ok((_, done)) => {
                    ed.reopen_with(ui, Some(true));
                    if done.media > 0 {
                        ui.media_rev.update(|n| *n += 1);
                    }
                    if let Some((_, url)) = done.pages.first() {
                        crate::pagetree::open_page(ed, url.clone());
                    }
                    ed.import.set(None);
                }
                Err(e) => ui.fail(e),
            }
        });
    };
    let on_key = window_event_listener(leptos::ev::keydown, move |e| {
        if e.key() == "Escape" {
            close();
        }
    });
    on_cleanup(move || on_key.remove());
    let toggle = move |id: String, on: bool| {
        checked.update(|c| {
            c.retain(|x| *x != id);
            if on {
                c.push(id);
            }
        })
    };
    // Lignes : nom, puis adresse d'arrivée (fichiers HTML), adresse dans la source (site) ou
    // nombre de variables (composants).
    let rows = move || {
        let l = list.get()?;
        let items = if pages { l.pages } else { l.components };
        if items.is_empty() {
            return Some(
                view! { <p class="import-empty muted">{ui.t(if pages { "import-no-pages" } else { "import-no-components" })}</p> }
                    .into_any(),
            );
        }
        Some(
            view! {
                <div class="import-items" role="group">
                    {items.into_iter().map(|it| {
                        let id = StoredValue::new(it.id.clone());
                        let label = match (&it.file, tab.get_untracked()) {
                            (Some(f), Tab::Html) if pages => f.clone(),
                            _ => it.name.clone(),
                        };
                        let src_path = it.path.clone();
                        let props = it.props;
                        let detail = move || {
                            if !pages {
                                return ui.tx("import-variables", &[("count", props.to_string())]);
                            }
                            if tab.get() == Tab::Html {
                                let to = plan.with(|p| p.as_ref().and_then(|p| {
                                    id.with_value(|i| p.pages.iter().find(|(s, _)| s == i).map(|(_, d)| d.clone()))
                                }));
                                return format!("→ {}", to.unwrap_or_else(|| src_path.clone()));
                            }
                            src_path.clone()
                        };
                        view! {
                            <label class="import-item" data-id=it.id.clone() title=it.file.clone().unwrap_or_default()>
                                <input type="checkbox" class="check"
                                    prop:checked=move || id.with_value(|i| checked.with(|c| c.contains(i)))
                                    on:change=move |e| toggle(id.get_value(), event_target_checked(&e))/>
                                <span class="import-item-name">{label}</span>
                                <span class="import-item-detail mono">{detail}</span>
                            </label>
                        }
                    }).collect_view()}
                </div>
            }
            .into_any(),
        )
    };
    let summary = move || {
        let p = plan.get();
        let n = count();
        let mut parts = Vec::new();
        if pages {
            parts.push(ui.tx("import-count-pages", &[("count", n.to_string())]));
            let c = p.as_ref().map(|p| p.components.len()).unwrap_or(0);
            if c > 0 {
                parts.push(ui.tx("import-count-components", &[("count", c.to_string())]));
            }
        } else {
            parts.push(ui.tx("import-count-components", &[("count", n.to_string())]));
        }
        let m = p.as_ref().map(|p| p.media).unwrap_or(0);
        if m > 0 {
            parts.push(ui.tx("import-count-media", &[("count", m.to_string())]));
        }
        parts.join(" · ")
    };
    // Détail du bilan (info-bulle) : réutilisés, styles, classes renommées.
    let summary_tip = move || {
        plan.with(|p| {
            let p = p.as_ref()?;
            let mut v = vec![ui.tx(
                "import-plan-detail",
                &[
                    ("reused", p.reused.to_string()),
                    ("styles", p.styles.to_string()),
                    ("vars", p.vars.to_string()),
                ],
            )];
            for (a, b) in &p.renamed {
                v.push(format!(".{a} → .{b}"));
            }
            Some(v.join("\n"))
        })
    };
    let tabs: Vec<Tab> = if pages {
        vec![Tab::Site, Tab::Html]
    } else {
        vec![Tab::Site, Tab::File, Tab::Html]
    };
    let tab_key = |t: Tab| match t {
        Tab::Site => "import-from-site",
        Tab::File => "import-from-component-file",
        Tab::Html => "import-from-html",
    };
    view! {
        <div class="overlay" on:mousedown=move |e| if crate::dialog::backdrop_click(&e) { close() }>
            <div class="dialog import-box" role="dialog" aria-modal="true" aria-labelledby="import-box-title"
                data-mode=if pages { "pages" } else { "components" }>
                <h2 id="import-box-title" class="dialog-title">{ui.t(if pages { "import-pages-title" } else { "import-component-title" })}</h2>
                <div class="segmented import-tabs" role="tablist">
                    {tabs.into_iter().map(|t| view! {
                        <button type="button" role="tab" data-tab=tab_key(t) class:is-on=move || tab.get() == t
                            aria-selected=move || (tab.get() == t).to_string() on:click=move |_| set_tab(t)>
                            {ui.t(tab_key(t))}</button>
                    }).collect_view()}
                </div>
                {move || match tab.get() {
                    Tab::Site => view! {
                        <label class="field is-inline import-site">
                            <span class="label">{ui.t("import-site")}</span>
                            <select class="input" on:change=pick_site>
                                <option value="" disabled=true selected=move || site.get().is_none()>{ui.t("import-choose-site")}</option>
                                {move || sites.get().into_iter().map(|(p, n)| {
                                    let p2 = p.clone();
                                    view! { <option value=p.clone() title=p selected=move || site.get().as_deref() == Some(p2.as_str())>{n}</option> }
                                }).collect_view()}
                                {move || other.get().map(|p| {
                                    let p2 = p.clone();
                                    view! { <option value=p.clone() title=p.clone() selected=move || site.get().as_deref() == Some(p2.as_str())>{file_name(&p)}</option> }
                                })}
                                <option value="__other">{ui.t("import-other-folder")}</option>
                            </select>
                        </label>
                    }.into_any(),
                    t => view! {
                        <div class="import-drop" class:is-over=move || over.get()
                            on:dragover=move |e: leptos::ev::DragEvent| if crate::files::has_files(&e) { e.prevent_default(); over.set(true) }
                            on:dragleave=move |_| over.set(false)
                            on:drop=on_drop>
                            <span class="import-drop-icon">{icons::upload()}</span>
                            <span class="import-drop-text">{ui.t(if t == Tab::File { "import-drop-component" } else { "import-drop-html" })}</span>
                            <button type="button" class="btn btn--small import-browse" on:click=browse>{ui.t("import-browse")}</button>
                        </div>
                    }.into_any(),
                }}
                <Show when=move || loading.get()>
                    <div class="import-loading muted" role="status"><span class="meter-bar is-busy"></span></div>
                </Show>
                {rows}
                <label class="field is-inline import-bring">
                    <span class="label" title=move || ui.tx(match (pages, tab.get()) {
                        (true, Tab::Site) => "import-bring-pages-help",
                        (true, _) => "import-bring-files-help",
                        (false, _) => "import-bring-components-help",
                    }, &[])>
                        {move || ui.tx(match (pages, tab.get()) {
                            (true, Tab::Site) => "import-bring-pages",
                            (true, _) => "import-bring-files",
                            (false, _) => "import-bring-components",
                        }, &[])}
                    </span>
                    <span class="switch">
                        <input type="checkbox" class="import-bring-input" prop:checked=move || bring.get()
                            on:change=move |e| bring.set(event_target_checked(&e))/>
                        <span class="switch-track"></span>
                    </span>
                </label>
                <div class="dialog-actions import-actions">
                    <span class="import-summary muted" title=summary_tip>{summary}</span>
                    <button type="button" class="btn" on:click=move |_| close()>{ui.t("action-cancel")}</button>
                    <button type="button" class="btn btn--primary import-go"
                        prop:disabled=move || busy.get() || count() == 0 || plan.with(Option::is_none)
                        on:click=go>
                        {move || ui.tx(if pages { "import-pages-go" } else { "import-components-go" }, &[("count", count().to_string())])}
                    </button>
                </div>
            </div>
        </div>
    }
}
