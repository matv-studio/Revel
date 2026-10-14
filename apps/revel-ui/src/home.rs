//! Liste des sites : ouvrir, importer, créer.

use crate::dialog::Ask;
use crate::{Ui, api, fmt_datetime, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::ImportResult;

#[component]
pub fn Home() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let busy = RwSignal::new(false);
    let import = RwSignal::new(None::<ImportResult>);
    let open_path = move |path: String| {
        spawn_local(async move {
            match api::open_site(&path).await {
                Ok(s) => ui.open(s),
                Err(e) => ui.fail(e),
            }
        })
    };
    let open = move |_| {
        spawn_local(async move {
            match api::pick_folder().await {
                Ok(Some(p)) => open_path(p),
                Ok(None) => {}
                Err(e) => ui.fail(e),
            }
        })
    };
    let run_import = move |p: String, overwrite: bool| {
        spawn_local(async move {
            busy.set(true);
            ui.activity.set(Some(0.3));
            match api::import_eleventy(&p, overwrite).await {
                Ok(r) => {
                    import.set(Some(r));
                    ui.reload_recents();
                }
                Err(e) => ui.fail(e),
            }
            ui.activity.set(None);
            busy.set(false);
        })
    };
    let do_import = move |_| {
        spawn_local(async move {
            let p = match api::pick_folder().await {
                Ok(Some(p)) => p,
                Ok(None) => return,
                Err(e) => return ui.fail(e),
            };
            if let Ok(true) = api::has_site(&p).await {
                ui.ask.set(Some(
                    Ask::confirm(
                        ui.now("import-overwrite-title", &[]),
                        Some(ui.now("import-overwrite", &[])),
                        ui.now("action-replace", &[]),
                        move || run_import(p.clone(), true),
                    )
                    .danger(),
                ));
            } else {
                run_import(p, false);
            }
        })
    };
    let create = move |_| {
        ui.ask.set(Some(Ask::input(
            ui.now("new-site-title", &[]),
            ui.now("new-site-name", &[]),
            String::new(),
            ui.now("new-site-choose", &[]),
            move |name| {
                spawn_local(async move {
                    let p = match api::pick_folder().await {
                        Ok(Some(p)) => p,
                        Ok(None) => return,
                        Err(e) => return ui.fail(e),
                    };
                    let lang = ui.settings.with_untracked(|s| s.lang.clone());
                    match api::new_site(&p, &name, &lang).await {
                        Ok(s) => ui.open(s),
                        Err(e) => ui.fail(e),
                    }
                })
            },
        )))
    };
    // Ouvrir directement dans l'éditeur.
    let open_editor = move |path: String| {
        spawn_local(async move {
            match api::open_site(&path).await {
                Ok(s) => {
                    ui.route.set(crate::Route::Site {
                        site: s,
                        tab: crate::Tab::Editor,
                    });
                    ui.reload_recents();
                }
                Err(e) => ui.fail(e),
            }
        })
    };
    let forget = move |path: String| {
        spawn_local(async move {
            let _ = api::forget_site(&path).await;
            ui.reload_recents();
        });
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
    let sites = move || {
        let q = query.get().trim().to_lowercase();
        ui.recents
            .get()
            .into_iter()
            .filter(|r| {
                q.is_empty()
                    || r.name.to_lowercase().contains(&q)
                    || r.path.to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>()
    };
    // Monogramme d'un site : initiales des deux premiers mots.
    let mono = |name: &str| -> String {
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
    };
    view! {
        <section class="page page--wide">
            <header class="page-head">
                <h1>{ui.t("home-title")}</h1>
                <div class="toolbar">
                    <button class="btn" on:click=open disabled=move || busy.get()>{icons::folder()}{ui.t("action-open")}</button>
                    <button class="btn" on:click=do_import disabled=move || busy.get()>{icons::import()}{ui.t("action-import")}</button>
                    <button class="btn btn--primary" on:click=create disabled=move || busy.get()>{icons::plus()}{ui.t("action-new")}</button>
                </div>
            </header>
            {move || import.get().map(|r| {
                let site = r.site.clone();
                let n = r.warnings.len();
                view! {
                    <div class="section">
                        <div class="section-head">
                            <h2>{ui.t("import-done")}</h2>
                            <button class="btn btn--primary" on:click=move |_| ui.open(site.clone())>{ui.t("action-open")}</button>
                        </div>
                        <p class="strong">{r.site.name.clone()}</p>
                        <Show when=move || { n > 0 }>
                            <h3>{ui.t("import-warnings")}</h3>
                            <ul class="list-plain">{r.warnings.iter().map(|w| view! { <li>{w.clone()}</li> }).collect_view()}</ul>
                        </Show>
                    </div>
                }
            })}
            <Show when=move || ui.recents.get().is_empty() && import.get().is_none()>
                <div class="empty">
                    <p>{ui.t("home-empty")}</p>
                    <div class="toolbar">
                        <button class="btn btn--primary" on:click=create>{icons::plus()}{ui.t("action-new")}</button>
                    </div>
                </div>
            </Show>
            <Show when=move || !ui.recents.get().is_empty()>
                <div class="home-tools">
                    <label class="panel-search home-search">
                        {icons::search()}
                        <input type="search" placeholder=ui.t("home-search") aria-label=ui.t("home-search")
                            prop:value=move || query.get() on:input=move |e| query.set(event_target_value(&e))/>
                    </label>
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
                                        <tr class="row-link site-open" data-site=r.path.clone() on:click=move |_| open_path(p.clone())>
                                            <td class="strong">{r.name.clone()}</td>
                                            <td class="muted path">{r.path.clone()}</td>
                                            <td class="muted col-date">{move || fmt_datetime(&ui.lang(), &opened)}</td>
                                            <td class="col-actions">
                                                <div class="row-actions">
                                                    <button class="icon-btn" title=ui.t("action-edit-site") aria-label=ui.t("action-edit-site")
                                                        on:click=move |ev| { ev.stop_propagation(); open_editor(p3.clone()) }>{icons::edit()}</button>
                                                    <button class="icon-btn" title=ui.t("action-forget") aria-label=ui.t("action-forget")
                                                        on:click=move |ev| { ev.stop_propagation(); forget(p2.clone()) }>{icons::close()}</button>
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
                                let (p, p2, p3) = (r.path.clone(), r.path.clone(), r.path.clone());
                                let opened = r.opened_at.clone();
                                view! {
                                    <article class="site-card site-open" data-site=r.path.clone() title=r.path.clone()
                                        tabindex="0" on:click=move |_| open_path(p.clone())>
                                        <div class="site-thumb"><span class="site-mono">{mono(&r.name)}</span></div>
                                        <div class="site-info">
                                            <span class="site-name">{r.name.clone()}</span>
                                            <span class="site-meta">{move || fmt_datetime(&ui.lang(), &opened)}</span>
                                        </div>
                                        <div class="site-actions">
                                            <button class="icon-btn icon-btn--small" title=ui.t("action-edit-site") aria-label=ui.t("action-edit-site")
                                                on:click=move |ev| { ev.stop_propagation(); open_editor(p3.clone()) }>{icons::edit()}</button>
                                            <button class="icon-btn icon-btn--small" title=ui.t("action-forget") aria-label=ui.t("action-forget")
                                                on:click=move |ev| { ev.stop_propagation(); forget(p2.clone()) }>{icons::close()}</button>
                                        </div>
                                    </article>
                                }
                            }
                        </For>
                    </div>
                </Show>
            </Show>
        </section>
    }
}
