//! Paramètres › Versions (3.10, D-25) : versions enregistrées du site (dépôt Git du dossier),
//! comparaison fichier par fichier, restauration, synchronisation avec le dépôt GitHub du site
//! (récupérer les enregistrements de Pages CMS, envoyer ceux de Revel).

use crate::dialog::Ask;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{DiffLine, FileChange, SiteSummary, VersionInfo, VersionsStatus};

/// Versions affichées (les plus récentes).
const LIMIT: usize = 200;
/// Lignes d'une comparaison affichées au plus.
const MAX_LINES: usize = 3000;

/// Date d'une version : relative si récente, sinon date et heure locales.
fn when(ui: Ui, secs: i64) -> String {
    let now = js_sys::Date::now() / 1000.0;
    let age = (now - secs as f64).max(0.0);
    if age < 60.0 {
        ui.now("time-now", &[])
    } else if age < 3600.0 {
        ui.now("time-min", &[("n", ((age / 60.0) as u64).to_string())])
    } else if age < 6.0 * 3600.0 {
        ui.now("time-hours", &[("n", ((age / 3600.0) as u64).to_string())])
    } else {
        let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(secs as f64 * 1000.0));
        let opts = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&opts, &"dateStyle".into(), &"medium".into());
        let _ = js_sys::Reflect::set(&opts, &"timeStyle".into(), &"short".into());
        d.to_locale_string(&ui.lang(), &opts)
            .as_string()
            .unwrap_or_default()
    }
}

/// Message par défaut d'une version (« Version du 15 oct. 2026, 21:58 »).
fn default_message(ui: Ui) -> String {
    let d = js_sys::Date::new_0();
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"dateStyle".into(), &"medium".into());
    let _ = js_sys::Reflect::set(&opts, &"timeStyle".into(), &"short".into());
    let date = d
        .to_locale_string(&ui.lang(), &opts)
        .as_string()
        .unwrap_or_default();
    ui.now("versions-default-message", &[("date", date)])
}

/// Initiale d'un auteur (pastille).
fn initial(name: &str) -> String {
    name.chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_else(|| "?".into())
}

/// Marque d'un changement : lettre et classe de couleur.
fn change_mark(c: &str) -> (&'static str, &'static str) {
    match c {
        "added" => ("A", "is-added"),
        "deleted" => ("S", "is-deleted"),
        "renamed" => ("R", "is-modified"),
        _ => ("M", "is-modified"),
    }
}

/// Sélection : changements en cours (`None`) ou une version.
#[derive(Clone, PartialEq)]
enum Sel {
    Current,
    Version(VersionInfo),
}

#[component]
pub fn Versions(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let path = StoredValue::new(site.path.clone());
    // `None` : lecture ; `Some(None)` : versions pas encore activées.
    let status = RwSignal::new(None::<Option<VersionsStatus>>);
    let list = RwSignal::new(Vec::<VersionInfo>::new());
    let sel = RwSignal::new(Sel::Current);
    let files = RwSignal::new(None::<Vec<FileChange>>);
    let file = RwSignal::new(None::<String>);
    let diff = RwSignal::new(None::<Option<Vec<DiffLine>>>);
    let busy = RwSignal::new(None::<&'static str>);
    let conflicts = RwSignal::new(Vec::<String>::new());
    let saving = RwSignal::new(None::<String>);

    let load_files = move || {
        let Some(p) = path.try_get_value() else {
            return;
        };
        let s = sel.get_untracked();
        files.set(None);
        file.set(None);
        diff.set(None);
        spawn_local(async move {
            let r = match &s {
                Sel::Current => api::versions_changes(&p, None, false).await,
                Sel::Version(v) => api::versions_changes(&p, Some(&v.id), true).await,
            };
            match r {
                Ok(l) => {
                    let _ = files.try_set(Some(l));
                }
                Err(e) => ui.fail(e),
            }
        });
    };
    let reload = move || {
        let Some(p) = path.try_get_value() else {
            return;
        };
        spawn_local(async move {
            match api::versions_status(&p).await {
                Ok(st) => {
                    let repo = st.is_some();
                    let changes = st.as_ref().map(|s| s.changes).unwrap_or(0);
                    if status.try_set(Some(st)).is_some() {
                        return;
                    }
                    if repo {
                        if let Ok(l) = api::versions_list(&p, LIMIT).await {
                            let _ = list.try_set(l.clone());
                            // Rien en cours : dernière version choisie.
                            if changes == 0
                                && sel.get_untracked() == Sel::Current
                                && let Some(first) = l.first()
                            {
                                sel.set(Sel::Version(first.clone()));
                            }
                        }
                        load_files();
                    }
                }
                Err(e) => ui.fail(e),
            }
        });
    };
    reload();
    let show_file = move |f: String| {
        let Some(p) = path.try_get_value() else {
            return;
        };
        file.set(Some(f.clone()));
        diff.set(None);
        let s = sel.get_untracked();
        spawn_local(async move {
            let r = match &s {
                Sel::Current => api::versions_diff(&p, None, false, &f).await,
                Sel::Version(v) => api::versions_diff(&p, Some(&v.id), true, &f).await,
            };
            match r {
                Ok(d) => {
                    let _ = diff.try_set(Some(d));
                }
                Err(e) => ui.fail(e),
            }
        });
    };
    // Premier fichier affiché d'emblée.
    Effect::new(move |_| {
        let first = files.with(|f| f.as_ref().and_then(|l| l.first().map(|x| x.path.clone())));
        if let Some(f) = first
            && file.with_untracked(Option::is_none)
        {
            show_file(f);
        }
    });
    let run = move |what: &'static str,
                    fut: std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Option<revel_protocol::SyncInfo>, revel_protocol::Failure>,
                >,
        >,
    >| {
        busy.set(Some(what));
        ui.activity.set(Some(0.3));
        spawn_local(async move {
            let r = fut.await;
            let _ = busy.try_set(None);
            ui.activity.set(None);
            match r {
                Ok(Some(info)) => {
                    let _ = conflicts.try_set(info.conflicts);
                }
                Ok(None) => {}
                Err(e) => ui.fail(e),
            }
            // Version restaurée ou reçue : nom et langue du site relus (barre supérieure).
            if matches!(what, "restore" | "sync")
                && let Some(p) = path.try_get_value()
                && let Ok(o) = api::edit_open(&p).await
            {
                ui.site_identity(&o.site);
            }
            let _ = sel.try_set(Sel::Current);
            reload();
        });
    };
    let init = move || {
        let p = path.get_value();
        let m = ui.now("versions-first", &[]);
        run(
            "init",
            Box::pin(async move { api::versions_init(&p, &m).await.map(|_| None) }),
        );
    };
    let save = move |message: String| {
        let p = path.get_value();
        let m = if message.trim().is_empty() {
            default_message(ui)
        } else {
            message.trim().to_string()
        };
        run(
            "save",
            Box::pin(async move { api::versions_save(&p, &m).await.map(|_| None) }),
        );
    };
    let sync = move || {
        let p = path.get_value();
        let (m, f) = (default_message(ui), ui.now("versions-merge-message", &[]));
        run(
            "sync",
            Box::pin(async move { api::versions_sync(&p, &m, &f).await.map(Some) }),
        );
    };
    let resolve = move |ours: bool| {
        let p = path.get_value();
        let m = ui.now(
            if ours {
                "versions-resolved-ours"
            } else {
                "versions-resolved-theirs"
            },
            &[],
        );
        run(
            "sync",
            Box::pin(async move { api::versions_resolve(&p, ours, &m).await.map(Some) }),
        );
    };
    let restore = move |v: VersionInfo| {
        ui.ask.set(Some(
            Ask::confirm(
                ui.now("versions-restore-title", &[("name", v.message.clone())]),
                None,
                ui.now("versions-restore", &[]),
                move || {
                    let p = path.get_value();
                    let (b, m) = (
                        ui.now("versions-before-restore", &[]),
                        ui.now("versions-restored", &[("name", v.message.clone())]),
                    );
                    let id = v.id.clone();
                    run(
                        "restore",
                        Box::pin(async move {
                            api::versions_restore(&p, &id, &b, &m).await.map(|_| None)
                        }),
                    );
                },
            )
            .danger(),
        ));
    };
    let set_remote = move |url: String| {
        let p = path.get_value();
        spawn_local(async move {
            if let Err(e) = api::versions_set_remote(&p, url.trim()).await {
                ui.fail(e);
            }
            reload();
        });
    };
    let esc = window_event_listener(leptos::ev::keydown, move |e| {
        if e.key() == "Escape" && saving.with_untracked(Option::is_some) {
            e.prevent_default();
            saving.set(None);
        }
    });
    on_cleanup(move || esc.remove());
    let st = move || status.get().flatten();
    let github_remote = move || {
        st().and_then(|s| s.remote)
            .is_some_and(|r| r.contains("github.com"))
    };
    let no_account = move || github_remote() && !st().is_some_and(|s| s.github);

    view! {
        <section class="page settings-page versions-page">
            <header class="page-head">
                <h1 title=ui.t("versions-help")>{ui.t("settings-versions")}</h1>
                {move || st().map(|s| {
                    let changes = s.changes;
                    view! {
                        <div class="toolbar">
                            <span class="chip" class:chip--todo={changes > 0} title=ui.t("versions-changes-help")>
                                {if changes > 0 { ui.now("versions-changes", &[("n", changes.to_string())]) } else { ui.now("versions-clean", &[]) }}
                            </span>
                            {s.remote.is_some().then(|| view! {
                                <button class="btn versions-sync" disabled=move || busy.get().is_some() || no_account()
                                    title=move || if no_account() { ui.tx("versions-sync-account", &[]) } else { ui.tx("versions-sync-help", &[]) }
                                    on:click=move |_| sync()>
                                    {icons::reset()}{ui.t("versions-sync")}
                                    {(s.ahead > 0).then(|| view! { <span class="sync-count" title=ui.t("versions-ahead")>{format!("↑{}", s.ahead)}</span> })}
                                    {(s.behind > 0).then(|| view! { <span class="sync-count" title=ui.t("versions-behind")>{format!("↓{}", s.behind)}</span> })}
                                </button>
                            })}
                            <button class="btn btn--primary versions-save" disabled=move || busy.get().is_some() || changes == 0
                                on:click=move |_| saving.set(Some(String::new()))>{icons::check()}{ui.t("versions-save")}</button>
                        </div>
                    }
                })}
            </header>
            {move || match status.get() {
                None => ().into_any(),
                Some(None) => view! {
                    <div class="section versions-empty">
                        <button class="btn btn--primary" title=ui.t("versions-init-help") disabled=move || busy.get().is_some()
                            on:click=move |_| init()>{icons::branch()}{ui.t("versions-init")}</button>
                    </div>
                }.into_any(),
                Some(Some(s)) => view! {
                    <div class="section versions-remote">
                        <label class="label" for="versions-remote" title=ui.t("versions-remote-help")>{ui.t("versions-remote")}</label>
                        <input id="versions-remote" class="input input--mono" spellcheck="false" placeholder="https://github.com/compte/site.git"
                            prop:value=s.remote.clone().unwrap_or_default() on:change=move |e| set_remote(event_target_value(&e))/>
                        <span class="chip" title=ui.t("versions-branch")>{icons::branch()}{s.branch.clone()}</span>
                        {move || no_account().then(|| view! {
                            <span class="seo-warn" title=ui.t("versions-sync-account")>{icons::warning()}</span>
                        })}
                    </div>
                    {move || (!conflicts.get().is_empty()).then(|| view! {
                        <div class="notice notice--warning versions-conflict" role="alert">
                            <div class="versions-conflict-text">
                                <strong>{ui.t("versions-conflict")}</strong>
                                <span class="path">{conflicts.get().join(" · ")}</span>
                            </div>
                            <button class="btn btn--small" disabled=move || busy.get().is_some() on:click=move |_| resolve(true)>{ui.t("versions-keep-ours")}</button>
                            <button class="btn btn--small" disabled=move || busy.get().is_some() on:click=move |_| resolve(false)>{ui.t("versions-keep-theirs")}</button>
                        </div>
                    })}
                    <div class="versions-grid">
                        <ul class="ver-list" role="listbox" aria-label=ui.t("settings-versions")>
                            {move || st().is_some_and(|s| s.changes > 0).then(|| view! {
                                <li role="option" class="ver-item ver-item--current" class:is-on=move || sel.get() == Sel::Current
                                    on:click=move |_| { sel.set(Sel::Current); load_files() }>
                                    <span class="ver-dot"></span>
                                    <span class="ver-main">
                                        <span class="ver-msg">{ui.t("versions-current")}</span>
                                        <span class="ver-meta muted">{move || ui.tx("versions-changes", &[("n", st().map(|s| s.changes).unwrap_or(0).to_string())])}</span>
                                    </span>
                                </li>
                            })}
                            {move || list.get().into_iter().enumerate().map(|(i, v)| {
                                let v2 = v.clone();
                                let id = v.id.clone();
                                view! {
                                    <li role="option" class="ver-item" data-version=v.id.clone() class:is-merge=v.merge
                                        class:is-on=move || matches!(sel.get(), Sel::Version(ref x) if x.id == id)
                                        on:click=move |_| { sel.set(Sel::Version(v2.clone())); load_files() }>
                                        <span class="ver-avatar" title=v.author.clone()>{initial(&v.author)}</span>
                                        <span class="ver-main">
                                            <span class="ver-msg" title=v.message.clone()>{v.message.clone()}</span>
                                            <span class="ver-meta muted">
                                                {when(ui, v.time)}" · "{v.author.clone()}
                                                {(i == 0).then(|| view! { <span class="chip chip--ok ver-latest">{ui.t("versions-latest")}</span> })}
                                            </span>
                                        </span>
                                        <span class="ver-files muted" title=ui.t("versions-files")>{v.files}</span>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                        <div class="ver-detail">
                            {move || {
                                let s = sel.get();
                                let (title, meta, version) = match &s {
                                    Sel::Current => (ui.now("versions-current", &[]), ui.now("versions-current-help", &[]), None),
                                    Sel::Version(v) => (v.message.clone(), format!("{} · {} · {}", v.author, when(ui, v.time), &v.id[..v.id.len().min(8)]), Some(v.clone())),
                                };
                                let latest = version.as_ref().is_some_and(|v| list.with_untracked(|l| l.first().is_some_and(|f| f.id == v.id)));
                                view! {
                                    <header class="ver-detail-head">
                                        <div class="ver-detail-title">
                                            <h2>{title}</h2>
                                            <span class="muted ver-meta">{meta}</span>
                                        </div>
                                        {version.filter(|_| !latest).map(|v| view! {
                                            <button class="btn btn--small btn--danger-ghost" title=ui.t("versions-restore-help")
                                                disabled=move || busy.get().is_some()
                                                on:click=move |_| restore(v.clone())>{icons::undo()}{ui.t("versions-restore")}</button>
                                        })}
                                    </header>
                                }
                            }}
                            <ul class="ver-files-list">
                                {move || match files.get() {
                                    None => view! { <li class="muted ver-wait"><span class="meter-bar is-busy"><span class="meter-fill"></span></span></li> }.into_any(),
                                    Some(l) if l.is_empty() => view! { <li class="muted ver-wait">{ui.t("versions-no-files")}</li> }.into_any(),
                                    Some(l) => l.into_iter().map(|f| {
                                        let (mark, class) = change_mark(&f.change);
                                        let p = f.path.clone();
                                        let p2 = f.path.clone();
                                        view! {
                                            <li class="ver-file" class:is-on=move || file.get().as_deref() == Some(p2.as_str())
                                                on:click=move |_| show_file(p.clone())>
                                                <span class=format!("ver-mark {class}") title=ui.t(match f.change.as_str() { "added" => "versions-added", "deleted" => "versions-deleted", _ => "versions-modified" })>{mark}</span>
                                                <span class="path ver-path" title=f.path.clone()>{f.path.clone()}</span>
                                            </li>
                                        }
                                    }).collect_view().into_any(),
                                }}
                            </ul>
                            {move || file.get().map(|f| view! {
                                <div class="ver-diff" aria-label=f.clone()>
                                    <div class="ver-diff-head">
                                        <span class="path ver-path">{f.clone()}</span>
                                        {move || diff.with(|d| d.as_ref().and_then(|d| d.as_ref()).map(|l| {
                                            let n = |k: char| l.iter().filter(|x| x.kind == k).count();
                                            view! {
                                                <span class="diff-stat">
                                                    <span class="is-add">{format!("+{}", n('+'))}</span>
                                                    <span class="is-del">{format!("−{}", n('-'))}</span>
                                                </span>
                                            }
                                        }))}
                                    </div>
                                    {move || match diff.get() {
                                        None => view! { <div class="ver-wait"><span class="meter-bar is-busy"><span class="meter-fill"></span></span></div> }.into_any(),
                                        Some(None) => view! { <div class="muted ver-wait">{ui.t("versions-binary")}</div> }.into_any(),
                                        Some(Some(lines)) => {
                                            let more = lines.len().saturating_sub(MAX_LINES);
                                            view! {
                                                <table class="diff">
                                                    <tbody>
                                                        {lines.into_iter().take(MAX_LINES).map(|l| view! {
                                                            <tr class:is-add={l.kind == '+'} class:is-del={l.kind == '-'}>
                                                                <td class="diff-no">{l.old.map(|n| n.to_string()).unwrap_or_default()}</td>
                                                                <td class="diff-no">{l.new.map(|n| n.to_string()).unwrap_or_default()}</td>
                                                                <td class="diff-sign">{l.kind.to_string()}</td>
                                                                <td class="diff-text">{l.text}</td>
                                                            </tr>
                                                        }).collect_view()}
                                                    </tbody>
                                                </table>
                                                {(more > 0).then(|| view! { <div class="muted ver-wait">{ui.now("versions-more-lines", &[("n", more.to_string())])}</div> })}
                                            }.into_any()
                                        }
                                    }}
                                </div>
                            })}
                        </div>
                    </div>
                }.into_any(),
            }}
            {move || saving.get().map(|_| {
                let msg = RwSignal::new(String::new());
                view! {
                    <div class="overlay" on:mousedown=move |e| if e.target() == e.current_target() { saving.set(None) }>
                        <form class="dialog" role="dialog" aria-modal="true" aria-labelledby="ver-save-title"
                            on:submit=move |e| { e.prevent_default(); saving.set(None); save(msg.get_untracked()) }>
                            <h2 id="ver-save-title">{ui.t("versions-save")}</h2>
                            <label class="field">
                                <span class="label">{ui.t("versions-message")}</span>
                                <input class="input" autofocus=true placeholder=default_message(ui)
                                    prop:value=move || msg.get() on:input=move |e| msg.set(event_target_value(&e))/>
                            </label>
                            <div class="dialog-actions">
                                <button type="button" class="btn" on:click=move |_| saving.set(None)>{ui.t("action-cancel")}</button>
                                <button type="submit" class="btn btn--primary">{ui.t("versions-save")}</button>
                            </div>
                        </form>
                    </div>
                }
            })}
        </section>
    }
}
