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
    // Menu contextuel d'un site : ouvrir, paramètres, afficher le dossier, retirer de la liste.
    let site_menu = move |e: &leptos::ev::MouseEvent, path: String| {
        use crate::ctxmenu::{CtxItem, open};
        let (a, b, c, d) = (path.clone(), path.clone(), path.clone(), path);
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
        ];
        open(ui, e, items);
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
    view! {
        <section class="page page--wide">
            <header class="page-head">
                <h1>{ui.t("home-title")}</h1>
                <div class="toolbar">
                    <button class="btn" title=ui.t("action-open-help") on:click=open disabled=move || busy.get()>{icons::folder()}{ui.t("action-open")}</button>
                    <button class="btn" title=ui.t("action-import-help") on:click=do_import disabled=move || busy.get()>{icons::import()}{ui.t("action-import")}</button>
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
                            <button class="btn btn--primary" on:click=move |_| ui.open(site.clone())>{ui.t("action-edit-site")}</button>
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
                                            on:contextmenu={ let p = r.path.clone(); move |e: leptos::ev::MouseEvent| site_menu(&e, p.clone()) }
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
                                        on:contextmenu={ let p = r.path.clone(); move |e: leptos::ev::MouseEvent| site_menu(&e, p.clone()) }
                                        on:keydown=move |e: leptos::ev::KeyboardEvent| if e.key() == "Enter" { open_path(p4.clone()) }>
                                        <div class="site-thumb"><span class="site-mono">{mono(&r.name)}</span></div>
                                        <div class="site-info">
                                            <span class="site-name">{r.name.clone()}</span>
                                            <span class="site-meta">{move || fmt_datetime(&ui.lang(), &opened)}</span>
                                        </div>
                                        <div class="site-actions">
                                            <button class="icon-btn icon-btn--small" title=ui.t("site-settings") aria-label=ui.t("site-settings")
                                                on:click=move |ev| { ev.stop_propagation(); open_settings(p3.clone()) }>{icons::settings()}</button>
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
            <Show when=move || creating.get()>
                <NewSite close=Callback::new(move |()| creating.set(false))/>
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

/// Création d'un site : nom, langue et emplacement. Le dossier est proposé (Documents/Revel/nom) ;
/// la fenêtre du système ne s'ouvre que sur « Changer… » (D-22).
#[component]
fn NewSite(close: Callback<()>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let name = RwSignal::new(String::new());
    let lang = RwSignal::new(ui.settings.with_untracked(|s| s.lang.clone()));
    let parent = RwSignal::new(String::new());
    let folder = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let template = RwSignal::new(revel_protocol::SiteTemplate::Blank);
    // Fond du site : sombre par défaut.
    let light = RwSignal::new(false);
    let input = NodeRef::<leptos::html::Input>::new();
    spawn_local(async move {
        if let Ok(d) = api::default_sites_dir().await {
            parent.set(d);
        }
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
                template.get_untracked(),
                light.get_untracked(),
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
    view! {
        <div class="overlay" on:mousedown=move |e| if e.target() == e.current_target() { close.run(()) }>
            <form class="dialog dialog--form" role="dialog" aria-modal="true" aria-labelledby="new-site-title"
                on:submit=move |e| { e.prevent_default(); submit() }>
                <h2 id="new-site-title">{ui.t("new-site-title")}</h2>
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
                    <span class="label">{ui.t("new-site-template")}</span>
                    <div class="template-choices" role="radiogroup" aria-label=ui.t("new-site-template")>
                        {[
                            (revel_protocol::SiteTemplate::Blank, "new-site-template-blank", "new-site-template-blank-help", icons::page as fn() -> AnyView),
                            (revel_protocol::SiteTemplate::Showcase, "new-site-template-showcase", "new-site-template-showcase-help", icons::layout as fn() -> AnyView),
                            (revel_protocol::SiteTemplate::Blog, "new-site-template-blog", "new-site-template-blog-help", icons::database as fn() -> AnyView),
                        ].into_iter().map(|(t, key, help, icon)| view! {
                            <button type="button" class="template-choice" role="radio" data-template=key
                                aria-checked=move || (template.get() == t).to_string()
                                class:is-on=move || template.get() == t
                                title=ui.t(help)
                                on:click=move |_| template.set(t)>
                                {icon()}<span>{ui.t(key)}</span>
                            </button>
                        }).collect_view()}
                    </div>
                </div>
                <div class="field">
                    <span class="label">{ui.t("new-site-background")}</span>
                    <div class="segmented site-bg" role="radiogroup" aria-label=ui.t("new-site-background")>
                        {[(false, "new-site-dark"), (true, "new-site-light")].into_iter().map(|(v, key)| view! {
                            <button type="button" role="radio" data-bg=key class:is-on=move || light.get() == v
                                aria-checked=move || (light.get() == v).to_string()
                                on:click=move |_| light.set(v)>
                                <span class="bg-swatch" class:bg-swatch--light=v></span>{ui.t(key)}
                            </button>
                        }).collect_view()}
                    </div>
                </div>
                <div class="field">
                    <span class="label">{ui.t("new-site-location")}</span>
                    <div class="path-field">
                        <span class="path-value" title=move || folder.get()>
                            {move || if folder.get().is_empty() { parent.get() } else { folder.get() }}
                        </span>
                        <button type="button" class="btn btn--small" on:click=change>{ui.t("new-site-change")}</button>
                    </div>
                </div>
                <div class="dialog-actions">
                    <button type="button" class="btn" on:click=move |_| close.run(())>{ui.t("action-cancel")}</button>
                    <button type="submit" class="btn btn--primary"
                        disabled=move || busy.get() || name.with(|n| n.trim().is_empty()) || parent.with(String::is_empty)>
                        {ui.t("new-site-create")}
                    </button>
                </div>
            </form>
        </div>
    }
}
