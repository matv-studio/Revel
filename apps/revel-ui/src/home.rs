//! Liste des sites : ouvrir, importer, créer.

use crate::{Ui, api, fmt_datetime, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{GithubSite, GithubSyncReport, ImportResult};

#[component]
pub fn Home() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let open_path = move |path: String| ui.open_path(path, crate::Tab::Editor);
    let open = move |_| {
        spawn_local(async move {
            match api::pick_folder().await {
                Ok(Some(p)) => open_path(p),
                Ok(None) => {}
                Err(e) => ui.fail(e),
            }
        })
    };
    // Import d'un site (étape 7) : boîte Dossier / Dépôt Git.
    let importing = RwSignal::new(false);
    let do_import = move |_| importing.set(true);
    // Nouveau site : boîte de création (nom, langue, emplacement).
    let creating = RwSignal::new(false);
    let create = move |_| creating.set(true);
    // Paramètres d'un site.
    let open_settings =
        move |path: String| ui.open_path(path, crate::Tab::Settings(crate::Section::General));
    let forget = move |path: String| {
        spawn_local(async move {
            let _ = api::forget_site(&path).await;
            ui.reload_recents();
        });
    };
    // Suppression d'un site : confirmée, dossier mis à la corbeille du système.
    let delete = move |path: String, name: String| {
        let p = StoredValue::new(path);
        ui.ask.set(Some(
            crate::dialog::Ask::confirm(
                ui.now("site-delete-title", &[("name", name.clone())]),
                Some(ui.now("site-delete-body", &[])),
                ui.now("site-delete-confirm", &[]),
                move || {
                    let path = p.get_value();
                    spawn_local(async move {
                        match api::delete_site(&path).await {
                            Ok(()) => ui.reload_recents(),
                            Err(e) => ui.fail(e),
                        }
                    });
                },
            )
            .danger(),
        ));
    };
    // Menu contextuel d'un site : ouvrir, paramètres, afficher le dossier, retirer de la liste,
    // supprimer.
    let site_menu = move |e: &leptos::ev::MouseEvent, path: String, name: String| {
        use crate::ctxmenu::{CtxItem, open};
        let (a, b, c, d, f) = (path.clone(), path.clone(), path.clone(), path.clone(), path);
        let items = vec![
            CtxItem::new(ui.now("action-edit-site", &[]), move || {
                open_path(a.clone())
            })
            .icon(icons::edit),
            CtxItem::new(ui.now("site-settings", &[]), move || {
                open_settings(b.clone())
            })
            .icon(icons::settings),
            CtxItem::new(ui.now("export-reveal", &[]), move || {
                let p = c.clone();
                leptos::task::spawn_local(async move {
                    if let Err(e) = api::reveal(&p).await {
                        ui.fail(e);
                    }
                });
            })
            .icon(icons::folder),
            CtxItem::new(ui.now("action-forget", &[]), move || forget(d.clone()))
                .icon(icons::close)
                .sep(),
            CtxItem::new(ui.now("site-delete", &[]), move || {
                delete(f.clone(), name.clone())
            })
            .icon(icons::trash)
            .danger(),
        ];
        open(ui, e, items);
    };
    // Dossiers du système déposés sur l'accueil (D-122) : site Revel ajouté (ouvert s'il est
    // seul), autre site proposé à l'import.
    let drop_over = RwSignal::new(false);
    let import_from = RwSignal::new(None::<String>);
    let add_paths = move |paths: Vec<String>| {
        spawn_local(async move {
            let mut sites = Vec::new();
            let mut other = None;
            for p in paths {
                if api::has_site(&p).await.unwrap_or(false) {
                    sites.push(p);
                } else if other.is_none() {
                    other = Some(p);
                }
            }
            match sites.as_slice() {
                [one] if other.is_none() => open_path(one.clone()),
                _ => {
                    for p in &sites {
                        if let Err(e) = api::open_site(p).await {
                            ui.fail(e);
                        }
                    }
                    ui.reload_recents();
                    if let Some(p) = other {
                        import_from.set(Some(p));
                        importing.set(true);
                    }
                }
            }
        });
    };
    let on_over = window_event_listener(leptos::ev::dragover, move |e: web_sys::DragEvent| {
        if !crate::files::has_files(&e) || importing.get_untracked() || creating.get_untracked() {
            return;
        }
        e.prevent_default();
        if let Some(dt) = e.data_transfer() {
            dt.set_drop_effect("copy");
        }
        drop_over.set(true);
    });
    let on_leave = window_event_listener(leptos::ev::dragleave, move |e: web_sys::DragEvent| {
        // Sortie de la fenêtre : plus d'élément d'arrivée.
        if e.related_target().is_none() {
            drop_over.set(false);
        }
    });
    let on_drop = window_event_listener(leptos::ev::drop, move |e: web_sys::DragEvent| {
        drop_over.set(false);
        if !crate::files::has_files(&e) || importing.get_untracked() || creating.get_untracked() {
            return;
        }
        e.prevent_default();
        let Some(dt) = e.data_transfer() else { return };
        let names = crate::files::dropped_folders(&dt);
        if names.is_empty() {
            ui.fail(revel_protocol::Failure::new("error-drop-folder-only"));
            return;
        }
        let posted = crate::files::post_dropped(&dt);
        spawn_local(async move {
            let mut paths = Vec::new();
            if posted {
                // Chemins reçus par l'application juste après le message.
                for _ in 0..20 {
                    if let Ok(p) = api::dropped_paths().await
                        && !p.is_empty()
                    {
                        paths = p;
                        break;
                    }
                    crate::sleep_ms(50).await;
                }
                // Seulement les dossiers déposés (pas les fichiers à côté).
                paths.retain(|p: &String| {
                    let name = p.rsplit(['/', '\\']).next().unwrap_or_default();
                    names.iter().any(|n| n == name)
                });
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
                add_paths(paths);
            }
        });
    });
    on_cleanup(move || {
        on_over.remove();
        on_leave.remove();
        on_drop.remove();
    });
    // Sites du compte GitHub (D-127) : ceux qui ne sont pas encore sur cet ordinateur sont
    // montrés à côté des autres, prêts à télécharger ; « Synchroniser » met à jour les copies.
    let gh_busy = RwSignal::new(false);
    let gh_note = RwSignal::new(None::<GithubSyncReport>);
    // Dépôt en cours de téléchargement et avancement (0 à 1).
    let downloading = RwSignal::new(None::<(String, f32)>);
    let load_gh = move || {
        spawn_local(async move {
            match api::github_sites().await {
                Ok(l) => ui.gh_sites.set(Some(l)),
                Err(e) => ui.fail(e),
            }
        })
    };
    Effect::new(move |_| {
        if ui.github.get().is_some() && ui.gh_sites.get_untracked().is_none() {
            load_gh();
        }
    });
    let sync_all = move |_| {
        if gh_busy.get_untracked() {
            return;
        }
        gh_busy.set(true);
        gh_note.set(None);
        spawn_local(async move {
            let merge = ui.now("versions-merge-message", &[]);
            match api::github_sync_all(&merge).await {
                Ok(r) => gh_note.set(Some(r)),
                Err(e) => ui.fail(e),
            }
            if let Ok(l) = api::github_sites().await {
                ui.gh_sites.set(Some(l));
            }
            ui.reload_recents();
            gh_busy.set(false);
        });
    };
    // Téléchargement d'un site dans le dossier des sites ; ouvert ensuite si `open`.
    let download = move |site: GithubSite, open: bool| {
        if downloading.get_untracked().is_some() {
            return;
        }
        downloading.set(Some((site.repo.clone(), 0.0)));
        spawn_local(async move {
            let repo = site.repo.clone();
            let parent = api::default_sites_dir().await.unwrap_or_default();
            let r = api::git_clone(&site.clone_url, &parent, move |e| {
                if e.total > 0 {
                    downloading.set(Some((repo.clone(), e.received as f32 / e.total as f32)));
                }
            })
            .await;
            match r {
                Ok(c) => match api::open_site(&c.path).await {
                    Ok(summary) => {
                        ui.gh_sites.update(|l| {
                            if let Some(g) = l
                                .as_mut()
                                .and_then(|l| l.iter_mut().find(|g| g.repo == site.repo))
                            {
                                g.path = Some(c.path.clone());
                            }
                        });
                        ui.reload_recents();
                        if open {
                            ui.open(summary);
                        }
                    }
                    Err(e) => ui.fail(e),
                },
                Err(e) => ui.fail(e),
            }
            downloading.set(None);
        });
    };
    let remote_menu = move |e: &leptos::ev::MouseEvent, site: GithubSite| {
        use crate::ctxmenu::{CtxItem, open};
        let (a, url) = (site.clone(), site.html_url.clone());
        let items = vec![
            CtxItem::new(ui.now("gh-site-download", &[]), move || download(a.clone(), false))
                .icon(icons::cloud_down),
            CtxItem::new(ui.now("gh-site-github", &[]), move || {
                let u = url.clone();
                spawn_local(async move {
                    let _ = api::open_external(&u).await;
                });
            })
            .icon(icons::external),
        ];
        open(ui, e, items);
    };
    // Dépôt GitHub d'un site de cet ordinateur (badge de la vignette).
    let repo_of = move |path: &str| {
        ui.gh_sites.with(|l| {
            l.as_ref()
                .and_then(|l| l.iter().find(|g| g.path.as_deref() == Some(path)))
                .map(|g| g.repo.clone())
        })
    };
    let query = RwSignal::new(String::new());
    // Vue en grille ou en liste (préférence gardée dans ce navigateur).
    let grid = RwSignal::new(
        window()
            .local_storage()
            .ok()
            .flatten()
            .and_then(|s| s.get_item("revel.home.view").ok().flatten())
            .is_none_or(|v| v != "list"),
    );
    Effect::new(move |_| {
        let v = if grid.get() { "grid" } else { "list" };
        if let Some(s) = window().local_storage().ok().flatten() {
            let _ = s.set_item("revel.home.view", v);
        }
    });
    // Tri : ouverts récemment (ordre de la liste) ou par nom.
    let by_name = RwSignal::new(
        window()
            .local_storage()
            .ok()
            .flatten()
            .and_then(|s| s.get_item("revel.home.sort").ok().flatten())
            .is_some_and(|v| v == "name"),
    );
    Effect::new(move |_| {
        let v = if by_name.get() { "name" } else { "recent" };
        if let Some(s) = window().local_storage().ok().flatten() {
            let _ = s.set_item("revel.home.sort", v);
        }
    });
    let sites = move || {
        let q = query.get().trim().to_lowercase();
        let mut list = ui
            .recents
            .get()
            .into_iter()
            .filter(|r| {
                q.is_empty()
                    || r.name.to_lowercase().contains(&q)
                    || r.path.to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>();
        if by_name.get() {
            list.sort_by_key(|r| r.name.to_lowercase());
        }
        list
    };
    let remote = move || {
        let q = query.get().trim().to_lowercase();
        let mut list = ui
            .gh_sites
            .get()
            .unwrap_or_default()
            .into_iter()
            .filter(|g| g.path.is_none())
            .filter(|g| {
                q.is_empty()
                    || g.name.to_lowercase().contains(&q)
                    || g.repo.to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>();
        if by_name.get() {
            list.sort_by_key(|g| g.name.to_lowercase());
        }
        list
    };
    let has_any = move || !ui.recents.get().is_empty() || !remote().is_empty();
    let note = move || {
        gh_note.get().map(|r| {
            let mut parts = Vec::new();
            if !r.updated.is_empty() {
                parts.push(ui.now("gh-sync-updated", &[("n", r.updated.len().to_string())]));
            }
            if !r.sent.is_empty() {
                parts.push(ui.now("gh-sync-sent", &[("n", r.sent.len().to_string())]));
            }
            if !r.waiting.is_empty() {
                parts.push(ui.now("gh-sync-waiting", &[("n", r.waiting.len().to_string())]));
            }
            if !r.failed.is_empty() {
                parts.push(ui.now("gh-sync-failed", &[("n", r.failed.len().to_string())]));
            }
            if parts.is_empty() {
                parts.push(ui.now("gh-sync-none", &[]));
            }
            let detail = r
                .updated
                .iter()
                .chain(&r.sent)
                .chain(&r.waiting)
                .cloned()
                .chain(r.failed.iter().map(|(n, m)| format!("{n} : {m}")))
                .collect::<Vec<_>>()
                .join("\n");
            (parts.join(" · "), detail, !r.waiting.is_empty() || !r.failed.is_empty())
        })
    };
    view! {
        <section class="page page--wide">
            <header class="page-head">
                <h1>{ui.t("home-title")}</h1>
                <div class="toolbar">
                    <Show when=move || ui.github.get().is_some()>
                        <button class="btn gh-sync" class:is-busy=move || gh_busy.get() disabled=move || gh_busy.get()
                            title=ui.t("gh-sync-help") on:click=sync_all>{icons::sync()}{ui.t("gh-sync")}</button>
                    </Show>
                    <button class="btn" title=ui.t("action-open-help") on:click=open>{icons::folder()}{ui.t("action-open")}</button>
                    <button class="btn" title=ui.t("action-import-help") on:click=do_import>{icons::import()}{ui.t("action-import")}</button>
                    <button class="btn btn--primary" on:click=create>{icons::plus()}{ui.t("action-new")}</button>
                </div>
            </header>
            {move || note().map(|(text, detail, warn)| view! {
                <div class="gh-note" class:is-warn=warn role="status" title=detail>
                    {icons::cloud()}<span>{text}</span>
                    <button class="icon-btn icon-btn--small" aria-label=ui.t("action-close") on:click=move |_| gh_note.set(None)>{icons::close()}</button>
                </div>
            })}
            <Show when=move || !has_any()>
                <div class="empty">
                    <p>{ui.t("home-empty")}</p>
                    <div class="toolbar">
                        <button class="btn btn--primary" on:click=create>{icons::plus()}{ui.t("action-new")}</button>
                    </div>
                </div>
            </Show>
            <Show when=has_any>
                <div class="home-tools">
                    <label class="panel-search home-search">
                        {icons::search()}
                        <input type="search" placeholder=ui.t("home-search") aria-label=ui.t("home-search")
                            prop:value=move || query.get() on:input=move |e| query.set(event_target_value(&e))/>
                    </label>
                    <div class="select home-sort" title=ui.t("home-sort")>
                        <select class="input" aria-label=ui.t("home-sort") on:change=move |e| by_name.set(event_target_value(&e) == "name")>
                            <option value="recent" selected=move || !by_name.get()>{ui.t("home-sort-recent")}</option>
                            <option value="name" selected=move || by_name.get()>{ui.t("home-sort-name")}</option>
                        </select>
                    </div>
                    <div class="segmented" role="radiogroup" aria-label=ui.t("home-view")>
                        <button role="radio" class:is-on=move || grid.get() aria-checked=move || grid.get().to_string()
                            title=ui.t("home-view-grid") aria-label=ui.t("home-view-grid") on:click=move |_| grid.set(true)>{icons::grid()}</button>
                        <button role="radio" class:is-on=move || !grid.get() aria-checked=move || (!grid.get()).to_string()
                            title=ui.t("home-view-list") aria-label=ui.t("home-view-list") on:click=move |_| grid.set(false)>{icons::list()}</button>
                    </div>
                </div>
                <Show when=move || grid.get() fallback=move || view! {
                    <table class="table">
                        <thead><tr>
                            <th>{ui.t("home-col-name")}</th>
                            <th>{ui.t("home-col-folder")}</th>
                            <th class="col-date">{ui.t("home-col-opened")}</th>
                            <th class="col-actions"></th>
                        </tr></thead>
                        <tbody>
                            <For each=sites key=|r| r.path.clone() let:r>
                                {
                                    let (p, p2, p3) = (r.path.clone(), r.path.clone(), r.path.clone());
                                    let opened = r.opened_at.clone();
                                    view! {
                                        <tr class="row-link site-open" data-site=r.path.clone() title=ui.t("action-edit-site")
                                            on:contextmenu={ let (p, n) = (r.path.clone(), r.name.clone()); move |e: leptos::ev::MouseEvent| site_menu(&e, p.clone(), n.clone()) }
                                            on:click=move |_| open_path(p.clone())>
                                            <td class="strong">
                                                <span class="site-dot" style=format!("--hue:{}", hue(&r.name))></span>
                                                {r.name.clone()}
                                            </td>
                                            <td class="muted path">{r.path.clone()}</td>
                                            <td class="muted col-date">{move || fmt_datetime(&ui.lang(), &opened)}</td>
                                            <td class="col-actions">
                                                <div class="row-actions">
                                                    <button class="icon-btn" title=ui.t("site-settings") aria-label=ui.t("site-settings")
                                                        on:click=move |ev| { ev.stop_propagation(); open_settings(p3.clone()) }>{icons::settings()}</button>
                                                    <button class="icon-btn" title=ui.t("action-forget") aria-label=ui.t("action-forget")
                                                        on:click=move |ev| { ev.stop_propagation(); forget(p2.clone()) }>{icons::close()}</button>
                                                    <button class="icon-btn site-delete" title=ui.t("site-delete") aria-label=ui.t("site-delete")
                                                        on:click={ let (p, n) = (r.path.clone(), r.name.clone()); move |ev: leptos::ev::MouseEvent| { ev.stop_propagation(); delete(p.clone(), n.clone()) } }>{icons::trash()}</button>
                                                </div>
                                            </td>
                                        </tr>
                                    }
                                }
                            </For>
                            <For each=remote key=|g| g.repo.clone() let:g>
                                {
                                    let (a, b, m) = (g.clone(), g.clone(), g.clone());
                                    let repo = g.repo.clone();
                                    let busy = move || downloading.with(|d| d.as_ref().map(|d| d.0 == repo).unwrap_or(false));
                                    view! {
                                        <tr class="row-link site-remote" data-repo=g.repo.clone() title=ui.t("gh-site-download-open")
                                            on:contextmenu=move |e: leptos::ev::MouseEvent| remote_menu(&e, m.clone())
                                            on:click=move |_| download(a.clone(), true)>
                                            <td class="strong">
                                                <span class="site-dot" style=format!("--hue:{}", hue(&g.name))></span>
                                                {g.name.clone()}
                                            </td>
                                            <td class="muted path">{icons::cloud()}" "{g.repo.clone()}</td>
                                            <td class="muted col-date">{g.pushed_at.clone().map(|d| fmt_datetime(&ui.lang(), &d)).unwrap_or_default()}</td>
                                            <td class="col-actions">
                                                <div class="row-actions">
                                                    <button class="icon-btn gh-download" class:is-busy=busy title=ui.t("gh-site-download") aria-label=ui.t("gh-site-download")
                                                        on:click=move |ev| { ev.stop_propagation(); download(b.clone(), false) }>{icons::cloud_down()}</button>
                                                </div>
                                            </td>
                                        </tr>
                                    }
                                }
                            </For>
                        </tbody>
                    </table>
                }>
                    <div class="site-grid">
                        <For each=sites key=|r| r.path.clone() let:r>
                            {
                                let (p, p2, p3, p4) = (r.path.clone(), r.path.clone(), r.path.clone(), r.path.clone());
                                let opened = r.opened_at.clone();
                                view! {
                                    <article class="site-card site-open" data-site=r.path.clone() title=r.path.clone()
                                        tabindex="0" style=format!("--hue:{}", hue(&r.name))
                                        on:click=move |_| open_path(p.clone())
                                        on:contextmenu={ let (p, n) = (r.path.clone(), r.name.clone()); move |e: leptos::ev::MouseEvent| site_menu(&e, p.clone(), n.clone()) }
                                        on:keydown=move |e: leptos::ev::KeyboardEvent| if e.key() == "Enter" { open_path(p4.clone()) }>
                                        <div class="site-thumb"><span class="site-mono">{mono(&r.name)}</span>
                                            {let p = r.path.clone(); move || repo_of(&p).map(|repo| view! { <span class="site-badge" title=repo>{icons::cloud()}</span> })}
                                        </div>
                                        <div class="site-info">
                                            <span class="site-name">{r.name.clone()}</span>
                                            <span class="site-meta">{move || fmt_datetime(&ui.lang(), &opened)}</span>
                                        </div>
                                        <div class="site-actions">
                                            <button class="icon-btn icon-btn--small" title=ui.t("site-settings") aria-label=ui.t("site-settings")
                                                on:click=move |ev| { ev.stop_propagation(); open_settings(p3.clone()) }>{icons::settings()}</button>
                                            <button class="icon-btn icon-btn--small" title=ui.t("action-forget") aria-label=ui.t("action-forget")
                                                on:click=move |ev| { ev.stop_propagation(); forget(p2.clone()) }>{icons::close()}</button>
                                            <button class="icon-btn icon-btn--small site-delete" title=ui.t("site-delete") aria-label=ui.t("site-delete")
                                                on:click={ let (p, n) = (r.path.clone(), r.name.clone()); move |ev: leptos::ev::MouseEvent| { ev.stop_propagation(); delete(p.clone(), n.clone()) } }>{icons::trash()}</button>
                                        </div>
                                    </article>
                                }
                            }
                        </For>
                        <For each=remote key=|g| g.repo.clone() let:g>
                            {
                                let (a, b, c, m) = (g.clone(), g.clone(), g.clone(), g.clone());
                                let repo = StoredValue::new(g.repo.clone());
                                let progress = move || repo.with_value(|r| downloading.with(|d| d.as_ref().filter(|d| d.0 == *r).map(|d| d.1)));
                                let size = if g.size_kb >= 1024 { format!("{:.0} Mo", g.size_kb as f64 / 1024.0) } else { format!("{} Ko", g.size_kb) };
                                view! {
                                    <article class="site-card site-card--remote" data-repo=g.repo.clone() title=ui.t("gh-site-download-open")
                                        tabindex="0" style=format!("--hue:{}", hue(&g.name))
                                        class:is-busy=move || progress().is_some()
                                        on:click=move |_| download(a.clone(), true)
                                        on:contextmenu=move |e: leptos::ev::MouseEvent| remote_menu(&e, m.clone())
                                        on:keydown=move |e: leptos::ev::KeyboardEvent| if e.key() == "Enter" { download(c.clone(), true) }>
                                        <div class="site-thumb"><span class="site-mono">{mono(&g.name)}</span>
                                            <span class="site-badge site-badge--remote">{icons::cloud()}</span>
                                            {move || progress().map(|p| view! { <span class="site-progress" style=format!("--p:{:.3}", p)></span> })}
                                        </div>
                                        <div class="site-info">
                                            <span class="site-name">{g.name.clone()}</span>
                                            <span class="site-meta">{g.repo.clone()}" · "{size}</span>
                                        </div>
                                        <div class="site-actions">
                                            <button class="icon-btn icon-btn--small gh-download" title=ui.t("gh-site-download") aria-label=ui.t("gh-site-download")
                                                on:click=move |ev| { ev.stop_propagation(); download(b.clone(), false) }>{icons::cloud_down()}</button>
                                        </div>
                                    </article>
                                }
                            }
                        </For>
                    </div>
                </Show>
            </Show>
            <Show when=move || creating.get()>
                <NewSite close=Callback::new(move |()| creating.set(false))/>
            </Show>
            <Show when=move || importing.get()>
                <crate::importer::ImportSite close=Callback::new(move |()| { importing.set(false); import_from.set(None); })
                    done=Callback::new(move |_: ImportResult| {}) start=import_from.get_untracked().unwrap_or_default()/>
            </Show>
            <Show when=move || drop_over.get()>
                <div class="drop-veil" aria-hidden="true">
                    <div class="drop-veil-card">{icons::folder_plus()}<span>{ui.t("home-drop")}</span></div>
                </div>
            </Show>
        </section>
    }
}

/// Monogramme d'un site : initiales des deux premiers mots.
fn mono(name: &str) -> String {
    let w: Vec<&str> = name.split_whitespace().collect();
    match w.as_slice() {
        [] => "?".into(),
        [one] => one.chars().take(2).collect::<String>(),
        [a, b, ..] => format!(
            "{}{}",
            a.chars().next().unwrap_or(' '),
            b.chars().next().unwrap_or(' ')
        ),
    }
    .to_uppercase()
}

/// Teinte stable dérivée du nom (couleur de la vignette).
fn hue(name: &str) -> u32 {
    name.bytes()
        .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)))
        % 360
}

/// Catégories de la galerie des modèles, dans l'ordre (D-128).
const TEMPLATE_CATS: [&str; 11] = [
    "business", "creative", "commerce", "editorial", "food", "events", "travel", "services",
    "education", "community", "personal",
];

/// Clé de traduction d'une catégorie de modèles.
fn cat_key(c: &str) -> &'static str {
    match c {
        "business" => "template-cat-business",
        "creative" => "template-cat-creative",
        "commerce" => "template-cat-commerce",
        "editorial" => "template-cat-editorial",
        "food" => "template-cat-food",
        "events" => "template-cat-events",
        "travel" => "template-cat-travel",
        "services" => "template-cat-services",
        "education" => "template-cat-education",
        "community" => "template-cat-community",
        _ => "template-cat-personal",
    }
}

/// Création d'un site : galerie des modèles (catégories, aperçu, couleurs), nom, langue,
/// couleurs du site et emplacement. Le dossier est proposé (Projects de Revel, sinon
/// Documents/Revel) ; la fenêtre du système ne s'ouvre que sur « Changer… » (D-22).
#[component]
fn NewSite(close: Callback<()>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let name = RwSignal::new(String::new());
    let lang = RwSignal::new(ui.settings.with_untracked(|s| s.lang.clone()));
    let parent = RwSignal::new(String::new());
    let folder = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let template = RwSignal::new("blank".to_string());
    // Couleurs du site : claires et sombres selon l'appareil du visiteur par défaut.
    let scheme = RwSignal::new("auto".to_string());
    let cat = RwSignal::new(String::new());
    let templates = RwSignal::new(Vec::<revel_protocol::TemplateInfo>::new());
    let input = NodeRef::<leptos::html::Input>::new();
    spawn_local(async move {
        if let Ok(d) = api::default_sites_dir().await {
            parent.set(d);
        }
    });
    // Modèles dans la langue choisie pour le site.
    Effect::new(move |_| {
        let l = lang.get();
        spawn_local(async move {
            if let Ok(list) = api::site_templates(&l).await {
                templates.set(list);
            }
        });
    });
    // Dossier qui sera créé (nom du site, suffixe si déjà pris).
    Effect::new(move |_| {
        let (p, n) = (parent.get(), name.get());
        if p.is_empty() {
            return;
        }
        spawn_local(async move {
            if let Ok(f) = api::site_folder(&p, &n).await {
                folder.set(f);
            }
        });
    });
    Effect::new(move |_| {
        if let Some(i) = input.get() {
            request_animation_frame(move || {
                let _ = i.focus();
            });
        }
    });
    let change = move |_| {
        spawn_local(async move {
            let start = parent.get_untracked();
            match api::pick_folder_at(Some(&start).filter(|s| !s.is_empty()).map(|s| s.as_str()))
                .await
            {
                Ok(Some(p)) => parent.set(p),
                Ok(None) => {}
                Err(e) => ui.fail(e),
            }
        })
    };
    let submit = move || {
        let n = name.get_untracked().trim().to_string();
        let p = parent.get_untracked();
        if n.is_empty() || p.is_empty() || busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match api::create_site(
                &p,
                &n,
                &lang.get_untracked(),
                &template.get_untracked(),
                &scheme.get_untracked(),
            )
            .await
            {
                Ok(s) => {
                    close.run(());
                    ui.open(s);
                }
                Err(e) => {
                    busy.set(false);
                    ui.fail(e)
                }
            }
        })
    };
    let on_key = window_event_listener(leptos::ev::keydown, move |e| {
        if e.key() == "Escape" {
            close.run(());
        }
    });
    on_cleanup(move || on_key.remove());
    let cats = move || {
        let list = templates.get();
        TEMPLATE_CATS
            .iter()
            .filter(|c| list.iter().any(|t| t.category == **c))
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
    };
    let shown = move || {
        let c = cat.get();
        templates
            .get()
            .into_iter()
            .filter(|t| c.is_empty() || t.category == c)
            .collect::<Vec<_>>()
    };
    let chosen = move || {
        let id = template.get();
        templates.with(|l| l.iter().find(|t| t.id == id).cloned())
    };
    view! {
        <div class="overlay" on:input=move |e| crate::dialog::mark_dirty(&e) on:mousedown=move |e| if crate::dialog::backdrop_click(&e) { close.run(()) }>
            <form class="dialog dialog--form new-site" role="dialog" aria-modal="true" aria-labelledby="new-site-title"
                on:submit=move |e| { e.prevent_default(); submit() }>
                <div class="new-site__gallery">
                    <div class="new-site__head">
                        <h2 id="new-site-title">{ui.t("new-site-title")}</h2>
                        <div class="new-site__cats" role="radiogroup" aria-label=ui.t("new-site-template")>
                            <button type="button" role="radio" class="cat-chip" class:is-on=move || cat.get().is_empty()
                                aria-checked=move || cat.get().is_empty().to_string()
                                on:click=move |_| cat.set(String::new())>{ui.t("template-cat-all")}</button>
                            <For each=cats key=|c| c.clone() let:c>
                                {
                                    let (c1, c2, c3) = (c.clone(), c.clone(), c.clone());
                                    view! {
                                        <button type="button" role="radio" class="cat-chip" data-cat=c.clone()
                                            class:is-on=move || cat.get() == c1 aria-checked=move || (cat.get() == c2).to_string()
                                            on:click=move |_| cat.set(c3.clone())>{ui.t(cat_key(&c))}</button>
                                    }
                                }
                            </For>
                        </div>
                    </div>
                    <div class="template-grid" role="radiogroup" aria-label=ui.t("new-site-template")>
                        <Show when=move || cat.get().is_empty()>
                            <button type="button" class="template-card template-card--blank" role="radio" data-template="blank"
                                class:is-on=move || template.get() == "blank" aria-checked=move || (template.get() == "blank").to_string()
                                title=ui.t("new-site-template-blank-help")
                                on:click=move |_| template.set("blank".into())>
                                <span class="template-card__thumb">{icons::page()}</span>
                                <span class="template-card__name">{ui.t("new-site-template-blank")}</span>
                            </button>
                        </Show>
                        <For each=shown key=|t| t.id.clone() let:t>
                            {
                                let (id, id2, id3) = (t.id.clone(), t.id.clone(), t.id.clone());
                                let swatch = t.palette.iter().map(|c| format!("<i style=\"background:{c}\"></i>")).collect::<String>();
                                let bg = t.palette.first().cloned().unwrap_or_default();
                                view! {
                                    <button type="button" class="template-card" role="radio" data-template=t.id.clone()
                                        class:is-on=move || template.get() == id aria-checked=move || (template.get() == id2).to_string()
                                        title=t.description.clone()
                                        on:click=move |_| template.set(id3.clone())>
                                        <span class="template-card__thumb" style=format!("background:{bg}")>
                                            {t.preview.clone().map(|src| view! { <img src=src alt="" loading="lazy" onerror="this.remove()"/> })}
                                            <span class="template-card__palette" inner_html=swatch></span>
                                        </span>
                                        <span class="template-card__name">{t.name.clone()}</span>
                                    </button>
                                }
                            }
                        </For>
                    </div>
                </div>
                <div class="new-site__side">
                    <div class="new-site__chosen">
                        {move || match chosen() {
                            Some(t) => view! {
                                <p class="new-site__chosen-name">{t.name.clone()}</p>
                                <p class="new-site__chosen-text">{t.description.clone()}</p>
                                <p class="muted new-site__chosen-pages">{ui.now("template-pages", &[("n", t.pages.to_string())])}</p>
                            }.into_any(),
                            None => view! {
                                <p class="new-site__chosen-name">{ui.t("new-site-template-blank")}</p>
                                <p class="new-site__chosen-text">{ui.t("new-site-template-blank-help")}</p>
                            }.into_any(),
                        }}
                    </div>
                    <label class="field">
                        <span class="label">{ui.t("new-site-name")}</span>
                        <input class="input" type="text" node_ref=input spellcheck="false" autocomplete="off"
                            placeholder=ui.t("new-site-placeholder")
                            prop:value=move || name.get() on:input=move |e| name.set(event_target_value(&e))/>
                    </label>
                    <label class="field">
                        <span class="label">{ui.t("new-site-lang")}</span>
                        <div class="select">
                            <select class="input" on:change=move |e| lang.set(event_target_value(&e))>
                                <option value="fr" lang="fr" selected=move || lang.get() == "fr">"Français"</option>
                                <option value="en" lang="en" selected=move || lang.get() == "en">"English"</option>
                            </select>
                        </div>
                    </label>
                    <div class="field">
                        <span class="label" title=ui.t("new-site-scheme-help")>{ui.t("new-site-background")}</span>
                        <div class="segmented site-bg" role="radiogroup" aria-label=ui.t("new-site-background")>
                            {[("auto", "new-site-auto"), ("light", "new-site-light"), ("dark", "new-site-dark")].into_iter().map(|(v, key)| view! {
                                <button type="button" role="radio" data-bg=v class:is-on=move || scheme.get() == v
                                    aria-checked=move || (scheme.get() == v).to_string()
                                    on:click=move |_| scheme.set(v.to_string())>
                                    <span class="bg-swatch" class:bg-swatch--light=v == "light" class:bg-swatch--auto=v == "auto"></span>{ui.t(key)}
                                </button>
                            }).collect_view()}
                        </div>
                    </div>
                    <div class="field">
                        <span class="label">{ui.t("new-site-location")}</span>
                        <div class="path-field">
                            <span class="path-value" title=move || folder.get()>
                                {move || crate::ltr(&if folder.get().is_empty() { parent.get() } else { folder.get() })}
                            </span>
                            <button type="button" class="btn btn--small" on:click=change>{ui.t("new-site-change")}</button>
                        </div>
                    </div>
                    <div class="dialog-actions">
                        <button type="button" class="btn" on:click=move |_| close.run(())>{ui.t("action-cancel")}</button>
                        <button type="submit" class="btn btn--primary"
                            disabled=move || busy.get() || name.with(|n| n.trim().is_empty()) || parent.with(String::is_empty)>
                            {move || if busy.get() { ui.t("new-site-creating") } else { ui.t("new-site-create") }}
                        </button>
                    </div>
                </div>
            </form>
        </div>
    }
}
