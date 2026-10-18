//! Paramètres › Versions (3.10, D-25) : versions enregistrées du site (dépôt Git du dossier),
//! comparaison fichier par fichier, restauration, synchronisation avec le dépôt GitHub du site
//! (récupérer les enregistrements de Pages CMS, envoyer ceux de Revel). Travail à plusieurs
//! (D-89) : seul, dossier partagé du réseau local ou dépôt en ligne, différences montrées à
//! l'activation ; endroits modifiés des deux côtés choisis un par un.

use crate::dialog::Ask;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{
    DiffLine, FileChange, SiteSummary, SyncConflictItem, VersionInfo, VersionsStatus,
};
use std::collections::HashMap;

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

/// Dépôt distant dans un dossier (NAS, réseau local) plutôt qu'en ligne.
fn is_folder(remote: &str) -> bool {
    let b = remote.as_bytes();
    remote.starts_with('/')
        || remote.starts_with("\\\\")
        || remote.starts_with("file://")
        || (b.len() > 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/'))
}

/// Façon de travailler : `solo`, `shared` (dossier partagé) ou `online` (dépôt en ligne).
fn mode_of(remote: Option<&str>) -> &'static str {
    match remote {
        None => "solo",
        Some(r) if is_folder(r) => "shared",
        Some(_) => "online",
    }
}

/// Lignes de la comparaison des deux façons de travailler à plusieurs : libellé et valeurs.
const COMPARE: &[&str] = &["where", "accounts", "remote", "browser", "backup", "provider"];

/// Endroit modifié des deux côtés : où (page, composant…) et élément.
fn item_place(ui: Ui, i: &SyncConflictItem) -> String {
    let key = match i.area.as_str() {
        "page" => "versions-area-page",
        "layout" => "versions-area-layout",
        "component" => "versions-area-component",
        "collection" => "versions-area-collection",
        "styles" => "versions-area-styles",
        "settings" => "versions-area-settings",
        "data" => "versions-area-data",
        _ => "versions-area-site",
    };
    ui.now(key, &[("name", i.name.clone())])
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
    // Endroits modifiés des deux côtés (D-89) et choix faits (`fichier#chemin` → ma version).
    let items = RwSignal::new(Vec::<SyncConflictItem>::new());
    let picks = RwSignal::new(HashMap::<String, bool>::new());
    let others_ours = RwSignal::new(None::<bool>);
    let saving = RwSignal::new(None::<String>);
    // Travail à plusieurs : façon choisie dans la fenêtre d'activation.
    let team = RwSignal::new(None::<&'static str>);
    let copied = RwSignal::new(false);

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
                    let _ = items.try_set(info.items);
                    let _ = picks.try_set(HashMap::new());
                    let _ = others_ours.try_set(None);
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
    // Choix appliqués : endroits un par un, autres fichiers entiers.
    let apply = move || {
        let p = path.get_value();
        let chosen: Vec<(String, bool)> = picks.get_untracked().into_iter().collect();
        let keep = others_ours.get_untracked().unwrap_or(true);
        let m = ui.now("versions-resolved-items", &[]);
        run(
            "sync",
            Box::pin(async move {
                api::versions_resolve_items(&p, keep, &chosen, &m)
                    .await
                    .map(Some)
            }),
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
    // Travail à plusieurs : dossier partagé (dépôt central créé puis premier envoi) ou dépôt en
    // ligne ; « Seul » retire le dépôt commun (versions gardées sur cet ordinateur).
    let set_shared = move |folder: String| {
        let p = path.get_value();
        team.set(None);
        busy.set(Some("sync"));
        spawn_local(async move {
            match api::versions_set_shared(&p, &folder).await {
                Ok(_) => {
                    let _ = busy.try_set(None);
                    sync();
                }
                Err(e) => {
                    let _ = busy.try_set(None);
                    ui.fail(e);
                    reload();
                }
            }
        });
    };
    let set_online = move |url: String| {
        team.set(None);
        set_remote(url);
    };
    let go_solo = move || {
        ui.ask.set(Some(
            Ask::confirm(
                ui.now("versions-team-off-title", &[]),
                None,
                ui.now("versions-team-off", &[]),
                move || set_remote(String::new()),
            )
            .danger(),
        ));
    };
    let esc = window_event_listener(leptos::ev::keydown, move |e| {
        if e.key() == "Escape" && saving.with_untracked(Option::is_some) {
            e.prevent_default();
            saving.set(None);
        } else if e.key() == "Escape" && team.with_untracked(Option::is_some) {
            e.prevent_default();
            team.set(None);
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
                Some(Some(s)) => {
                    let mode = mode_of(s.remote.as_deref());
                    let remote = s.remote.clone().unwrap_or_default();
                    let remote2 = remote.clone();
                    view! {
                    <div class="section versions-team">
                        <span class="label" title=ui.t("versions-team-help")>{ui.t("versions-team")}</span>
                        <div class="segmented versions-mode" role="radiogroup" aria-label=ui.t("versions-team")>
                            {[("solo", "versions-mode-solo", "versions-mode-solo-help"),
                              ("shared", "versions-mode-shared", "versions-mode-shared-help"),
                              ("online", "versions-mode-online", "versions-mode-online-help")]
                                .into_iter().map(|(m, key, help)| view! {
                                    <button role="radio" data-mode=m class:is-on=mode == m aria-checked=(mode == m).to_string()
                                        title=ui.t(help) disabled=move || busy.get().is_some()
                                        on:click=move |_| if m != mode { if m == "solo" { go_solo() } else { team.set(Some(m)) } }>{ui.t(key)}</button>
                                }).collect_view()}
                        </div>
                        {(mode != "solo").then(|| view! {
                            <span class="path-value versions-remote-value" title=remote.clone()>{remote.clone()}</span>
                            <button type="button" class="icon-btn icon-btn--small versions-remote-copy"
                                title=move || ui.tx(if copied.get() { "versions-team-copied" } else { "versions-team-copy" }, &[])
                                aria-label=ui.t("versions-team-copy")
                                on:click=move |_| {
                                    api::copy_text(&remote2);
                                    copied.set(true);
                                    set_timeout(move || { let _ = copied.try_set(false); }, std::time::Duration::from_millis(1600));
                                }>{move || if copied.get() { icons::check().into_any() } else { icons::copy().into_any() }}</button>
                            <button type="button" class="btn btn--small versions-remote-change" disabled=move || busy.get().is_some()
                                on:click=move |_| team.set(Some(mode))>{ui.t("new-site-change")}</button>
                        })}
                        <span class="chip" title=ui.t("versions-branch")>{icons::branch()}{s.branch.clone()}</span>
                        {move || no_account().then(|| view! {
                            <span class="seo-warn" title=ui.t("versions-sync-account")>{icons::warning()}</span>
                        })}
                    </div>
                    {move || (!items.get().is_empty()).then(|| {
                        let n = items.with(Vec::len);
                        // Fichiers en conflit hors des endroits choisis un par un.
                        let others: Vec<String> = conflicts.get().into_iter()
                            .filter(|f| !items.with(|l| l.iter().any(|i| &i.file == f)))
                            .collect();
                        let has_others = !others.is_empty();
                        let ready = move || items.with(|l| picks.with(|p| l.iter().all(|i| p.contains_key(&format!("{}#{}", i.file, i.id)))))
                            && (!has_others || others_ours.get().is_some());
                        let pick_all = move |ours: bool| {
                            picks.set(items.with_untracked(|l| l.iter().map(|i| (format!("{}#{}", i.file, i.id), ours)).collect()));
                            if has_others { others_ours.set(Some(ours)); }
                        };
                        view! {
                            <div class="section versions-items" role="alert">
                                <header class="versions-items-head">
                                    <strong title=ui.t("versions-items-help")>{ui.now("versions-items", &[("n", n.to_string())])}</strong>
                                    <span class="spacer"></span>
                                    <button class="btn btn--small versions-pick-all-ours" disabled=move || busy.get().is_some()
                                        on:click=move |_| pick_all(true)>{ui.t("versions-pick-all-ours")}</button>
                                    <button class="btn btn--small versions-pick-all-theirs" disabled=move || busy.get().is_some()
                                        on:click=move |_| pick_all(false)>{ui.t("versions-pick-all-theirs")}</button>
                                    <button class="btn btn--small btn--primary versions-apply" disabled=move || busy.get().is_some() || !ready()
                                        on:click=move |_| apply()>{icons::check()}{ui.t("versions-apply")}</button>
                                </header>
                                <ul class="versions-item-list">
                                    {items.get().into_iter().map(|i| {
                                        let key = format!("{}#{}", i.file, i.id);
                                        let (k1, k2, k3, k4) = (key.clone(), key.clone(), key.clone(), key.clone());
                                        let removed = ui.now("versions-removed", &[]);
                                        let ours = i.ours.clone().unwrap_or_else(|| removed.clone());
                                        let theirs = i.theirs.clone().unwrap_or(removed);
                                        view! {
                                            <li class="versions-item" data-item=key.clone()>
                                                <span class="versions-item-place" title=i.id.clone()>
                                                    <span class="versions-item-area">{item_place(ui, &i)}</span>
                                                    {(!i.detail.is_empty()).then(|| view! { <span class="path versions-item-detail">{i.detail.clone()}</span> })}
                                                </span>
                                                <button class="versions-pick versions-pick--ours" role="radio"
                                                    class:is-on=move || picks.with(|p| p.get(&k1) == Some(&true))
                                                    aria-checked=move || picks.with(|p| (p.get(&k2) == Some(&true)).to_string())
                                                    on:click=move |_| picks.update(|p| { p.insert(k3.clone(), true); })>
                                                    <span class="versions-pick-label">{ui.t("versions-pick-ours")}</span>
                                                    <span class="versions-pick-value" class:is-removed=i.ours.is_none() title=ours.clone()>{ours.clone()}</span>
                                                </button>
                                                <button class="versions-pick versions-pick--theirs" role="radio"
                                                    class:is-on={let k = key.clone(); move || picks.with(|p| p.get(&k) == Some(&false))}
                                                    aria-checked={let k = key.clone(); move || picks.with(|p| (p.get(&k) == Some(&false)).to_string())}
                                                    on:click=move |_| picks.update(|p| { p.insert(k4.clone(), false); })>
                                                    <span class="versions-pick-label">{ui.t("versions-pick-theirs")}</span>
                                                    <span class="versions-pick-value" class:is-removed=i.theirs.is_none() title=theirs.clone()>{theirs.clone()}</span>
                                                </button>
                                            </li>
                                        }
                                    }).collect_view()}
                                    {has_others.then(|| view! {
                                        <li class="versions-item versions-item--files">
                                            <span class="versions-item-place">
                                                <span class="versions-item-area">{ui.t("versions-other-files")}</span>
                                                <span class="path versions-item-detail" title=others.join("\n")>{others.join(" · ")}</span>
                                            </span>
                                            <button class="versions-pick versions-pick--ours" role="radio" class:is-on=move || others_ours.get() == Some(true)
                                                on:click=move |_| others_ours.set(Some(true))>
                                                <span class="versions-pick-label">{ui.t("versions-pick-ours")}</span>
                                            </button>
                                            <button class="versions-pick versions-pick--theirs" role="radio" class:is-on=move || others_ours.get() == Some(false)
                                                on:click=move |_| others_ours.set(Some(false))>
                                                <span class="versions-pick-label">{ui.t("versions-pick-theirs")}</span>
                                            </button>
                                        </li>
                                    })}
                                </ul>
                            </div>
                        }
                    })}
                    {move || (!conflicts.get().is_empty() && items.with(Vec::is_empty)).then(|| view! {
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
                }.into_any()
                }
            }}
            {move || team.get().map(|target| {
                let remote = status.get_untracked().flatten().and_then(|s| s.remote);
                team_dialog(ui, team, target, remote, set_shared, set_online)
            })}
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

/// Fenêtre d'activation du travail à plusieurs : les deux façons comparées (la choisie mise en
/// avant), puis le dossier partagé ou l'adresse du dépôt en ligne.
fn team_dialog(
    ui: Ui,
    team: RwSignal<Option<&'static str>>,
    target: &'static str,
    remote: Option<String>,
    set_shared: impl Fn(String) + Copy + 'static,
    set_online: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    let current = mode_of(remote.as_deref());
    let choice = RwSignal::new(target);
    let initial = |m: &str| {
        if current == m {
            remote.clone().unwrap_or_default()
        } else {
            String::new()
        }
    };
    let (shared0, online0) = (initial("shared"), initial("online"));
    let folder = RwSignal::new(shared0.clone());
    let url = RwSignal::new(online0.clone());
    let pick = move |_| {
        let start = folder.get_untracked();
        spawn_local(async move {
            let start = (!start.trim().is_empty()).then_some(start);
            match api::pick_folder_at(start.as_deref()).await {
                Ok(Some(d)) => {
                    let _ = folder.try_set(d);
                }
                Ok(None) => {}
                Err(e) => ui.fail(e),
            }
        });
    };
    let (shared0, online0) = (StoredValue::new(shared0), StoredValue::new(online0));
    let ready = move || match choice.get() {
        "shared" => folder.with(|f| !f.trim().is_empty() && shared0.with_value(|s| f.trim() != s)),
        _ => url.with(|u| !u.trim().is_empty() && online0.with_value(|s| u.trim() != s)),
    };
    let activate = move || {
        if !ready() {
            return;
        }
        match choice.get_untracked() {
            "shared" => set_shared(folder.get_untracked().trim().to_string()),
            _ => set_online(url.get_untracked().trim().to_string()),
        }
    };
    let col = move |m: &'static str| {
        view! {
            <button type="button" role="radio" class="team-col" data-mode=m
                class:is-on=move || choice.get() == m aria-checked=move || (choice.get() == m).to_string()
                title=ui.t(if m == "shared" { "versions-mode-shared-help" } else { "versions-mode-online-help" })
                on:click=move |_| choice.set(m)>
                {ui.t(if m == "shared" { "versions-mode-shared" } else { "versions-mode-online" })}
            </button>
        }
    };
    view! {
        <div class="overlay" on:mousedown=move |e| if e.target() == e.current_target() { team.set(None) }>
            <form class="dialog dialog--wide team-dialog" role="dialog" aria-modal="true" aria-labelledby="team-title"
                on:submit=move |e| { e.prevent_default(); activate() }>
                <h2 id="team-title" title=ui.t("versions-team-help")>{ui.t("versions-team-title")}</h2>
                <table class="team-compare" role="radiogroup" aria-labelledby="team-title">
                    <thead>
                        <tr><th></th><th>{col("shared")}</th><th>{col("online")}</th></tr>
                    </thead>
                    <tbody>
                        {COMPARE.iter().map(|row| {
                            let row: &'static str = row;
                            let k = move |suffix: &str| {
                                let key = format!("versions-compare-{row}{suffix}");
                                move || ui.tx(&key, &[])
                            };
                            view! {
                                <tr data-row=row>
                                    <th scope="row" title=k("-help")>{k("")}</th>
                                    <td class:is-on=move || choice.get() == "shared">{k("-shared")}</td>
                                    <td class:is-on=move || choice.get() == "online">{k("-online")}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
                {move || if choice.get() == "shared" {
                    view! {
                        <label class="field team-field">
                            <span class="label" title=ui.t("versions-shared-folder-help")>{ui.t("versions-shared-folder")}</span>
                            <span class="team-folder-row">
                                <input class="input input--mono team-folder" spellcheck="false" autocomplete="off"
                                    placeholder=r"\\NAS\Equipe\Sites" title=ui.t("versions-shared-folder-help")
                                    prop:value=move || folder.get() on:input=move |e| folder.set(event_target_value(&e))/>
                                <button type="button" class="btn btn--small team-folder-pick" on:click=pick>{ui.t("versions-shared-choose")}</button>
                            </span>
                        </label>
                    }.into_any()
                } else {
                    view! {
                        <label class="field team-field">
                            <span class="label" title=ui.t("versions-online-url-help")>{ui.t("versions-online-url")}</span>
                            <input class="input input--mono team-url" spellcheck="false" autocomplete="off"
                                placeholder="https://github.com/compte/site.git" title=ui.t("versions-online-url-help")
                                prop:value=move || url.get() on:input=move |e| url.set(event_target_value(&e))/>
                        </label>
                    }.into_any()
                }}
                <div class="dialog-actions">
                    <button type="button" class="btn" on:click=move |_| team.set(None)>{ui.t("action-cancel")}</button>
                    <button type="submit" class="btn btn--primary team-on" disabled=move || !ready()>{ui.t("versions-team-on")}</button>
                </div>
            </form>
        </div>
    }
}
