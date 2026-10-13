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
    view! {
        <section class="page">
            <header class="page-head">
                <h1>{ui.t("home-title")}</h1>
                <div class="toolbar">
                    <button class="btn btn--primary" on:click=open disabled=move || busy.get()>{icons::folder()}{ui.t("action-open")}</button>
                    <button class="btn" on:click=do_import disabled=move || busy.get()>{icons::import()}{ui.t("action-import")}</button>
                    <button class="btn" on:click=create disabled=move || busy.get()>{icons::plus()}{ui.t("action-new")}</button>
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
                </div>
            </Show>
            <Show when=move || !ui.recents.get().is_empty()>
                <table class="table">
                    <thead><tr>
                        <th>{ui.t("home-col-name")}</th>
                        <th>{ui.t("home-col-folder")}</th>
                        <th class="col-date">{ui.t("home-col-opened")}</th>
                        <th class="col-actions"></th>
                    </tr></thead>
                    <tbody>
                        <For each=move || ui.recents.get() key=|r| r.path.clone() let:r>
                            {
                                let p = r.path.clone();
                                let p2 = r.path.clone();
                                let opened = r.opened_at.clone();
                                view! {
                                    <tr class="row-link" on:click=move |_| open_path(p.clone())>
                                        <td class="strong">{r.name.clone()}</td>
                                        <td class="muted path">{r.path.clone()}</td>
                                        <td class="muted col-date">{move || fmt_datetime(&ui.lang(), &opened)}</td>
                                        <td class="col-actions">
                                            <button class="icon-btn" title=ui.t("action-forget") aria-label=ui.t("action-forget")
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    let p = p2.clone();
                                                    spawn_local(async move {
                                                        let _ = api::forget_site(&p).await;
                                                        ui.reload_recents();
                                                    });
                                                }>{icons::close()}</button>
                                        </td>
                                    </tr>
                                }
                            }
                        </For>
                    </tbody>
                </table>
            </Show>
        </section>
    }
}
