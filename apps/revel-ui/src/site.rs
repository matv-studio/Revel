//! Site ouvert : vue d'ensemble et génération.

use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{BuildEvent, BuildPhase, BuildSummary, Failure, SiteSummary};

/// Première lettre en capitale (identifiants de collection affichés comme noms).
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

#[component]
pub fn Overview(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    // Collections de contenus (dossiers d'entrées) ; les collections calculées (plan du site…)
    // ne sont pas des contenus à éditer.
    let cols: Vec<_> = site
        .collections
        .iter()
        .filter(|c| !c.folder.is_empty())
        .cloned()
        .collect();
    let n_cols = cols.len();
    let s_edit = site.clone();
    let s_build = site.clone();
    view! {
        <section class="page">
            <header class="page-head">
                <div>
                    <h1>{site.name.clone()}</h1>
                    <p class="muted path">{site.path.clone()}</p>
                </div>
                <div class="toolbar">
                    <button class="btn" on:click=move |_| ui.route.set(crate::Route::Site { site: s_build.clone(), tab: crate::Tab::Build })>
                        {icons::build()}{ui.t("nav-build")}
                    </button>
                    <button class="btn btn--primary" on:click=move |_| ui.route.set(crate::Route::Site { site: s_edit.clone(), tab: crate::Tab::Editor })>
                        {icons::edit()}{ui.t("action-edit-site")}
                    </button>
                </div>
            </header>
            <div class="section">
                <h2>{ui.t("overview-structure")}</h2>
                <dl class="facts">
                    <div><dt>{ui.t("site-pages")}</dt><dd>{site.pages}</dd></div>
                    <div><dt>{ui.t("site-layouts")}</dt><dd>{site.layouts}</dd></div>
                    <div><dt>{ui.t("site-components")}</dt><dd>{site.components}</dd></div>
                    <div><dt>{ui.t("site-collections")}</dt><dd>{n_cols}</dd></div>
                </dl>
            </div>
            <div class="section">
                <h2>{ui.t("overview-content")}</h2>
                <table class="table table--compact">
                    <tbody>
                        {cols.into_iter().map(|c| {
                            let n = c.entries;
                            view! {
                                <tr>
                                    <td class="strong">{capitalize(&c.id)}</td>
                                    <td class="muted path">{c.folder.clone()}</td>
                                    <td class="muted col-count">{move || ui.tx("site-entries", &[("count", crate::num(n as f64, 0))])}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
            <div class="section">
                <h2>{ui.t("overview-environments")}</h2>
                <div class="chips">{site.environments.iter().map(|e| view! { <span class="chip">{e.clone()}</span> }).collect_view()}</div>
            </div>
        </section>
    }
}

#[component]
pub fn Build(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let env = RwSignal::new(ui.preview_environment(&site));
    // L'aperçu de la barre supérieure suit l'environnement choisi ici.
    Effect::new(move |_| ui.environment.set(Some(env.get())));
    let images = RwSignal::new(true);
    let running = RwSignal::new(false);
    let progress = RwSignal::new(None::<(BuildPhase, usize, usize)>);
    let result = RwSignal::new(None::<Result<BuildSummary, Failure>>);
    let path = StoredValue::new(site.path.clone());
    // Fin de la génération. Les événements du canal peuvent arriver après la réponse de la
    // commande : l'état final est fixé par le premier signal de fin, et les messages de
    // progression qui suivent sont ignorés.
    let finish = move |r: Result<BuildSummary, Failure>| {
        result.set(Some(r));
        progress.set(None);
        running.set(false);
        ui.activity.set(None);
    };
    let run = move |_| {
        running.set(true);
        result.set(None);
        ui.activity.set(Some(0.02));
        spawn_local(async move {
            let r = api::build(
                &path.get_value(),
                &env.get_untracked(),
                images.get_untracked(),
                move |e| match e {
                    BuildEvent::Progress { .. } if result.with_untracked(|r| r.is_some()) => {}
                    BuildEvent::Progress { phase, done, total } => {
                        progress.set(Some((phase, done, total)));
                        let f = match phase {
                            BuildPhase::Pages => 0.05,
                            BuildPhase::Copies => 0.1,
                            BuildPhase::Images if total > 0 => {
                                0.1 + 0.9 * done as f32 / total as f32
                            }
                            BuildPhase::Images => 0.1,
                        };
                        ui.activity.set(Some(f));
                    }
                    BuildEvent::Finished(s) => finish(Ok(s)),
                    BuildEvent::Failed(f) => finish(Err(f)),
                },
            )
            .await;
            if let Err(e) = r
                && result.with_untracked(|r| r.is_none())
            {
                finish(Err(e));
            }
        });
    };
    // Aperçu de l'environnement qui vient d'être généré.
    let preview = move |environment: String| {
        spawn_local(async move {
            if let Err(e) = api::preview(&path.get_value(), &environment).await {
                ui.fail(e);
            }
        })
    };
    let envs = site.environments.clone();
    let phase_text = move || {
        progress.get().map(|(phase, done, total)| match phase {
            BuildPhase::Pages => ui.now("build-phase-pages", &[]),
            BuildPhase::Copies => ui.now("build-phase-copies", &[]),
            BuildPhase::Images => ui.now(
                "build-phase-images",
                &[
                    ("done", crate::num(done as f64, 0)),
                    ("total", crate::num(total as f64, 0)),
                ],
            ),
        })
    };
    view! {
        <section class="page">
            <header class="page-head">
                <h1>{ui.t("build-title")}</h1>
            </header>
            <div class="section">
                <div class="form-grid">
                    <span class="label">{ui.t("build-env")}</span>
                    <div class="segmented" role="radiogroup">
                        {envs.into_iter().map(|e| {
                            let v = e.clone();
                            let v2 = e.clone();
                            view! {
                                <button role="radio" class:is-on=move || env.get() == v aria-checked=move || (env.get() == v2).to_string()
                                    on:click=move |_| env.set(e.clone())>{e.clone()}</button>
                            }
                        }).collect_view()}
                    </div>
                    <span class="label">{ui.t("build-images")}</span>
                    <label class="switch">
                        <input type="checkbox" prop:checked=move || images.get() on:change=move |ev| images.set(event_target_checked(&ev))/>
                        <span class="switch-track"></span>
                    </label>
                </div>
                <div class="toolbar toolbar--start">
                    <button class="btn btn--primary" on:click=run disabled=move || running.get()>
                        {icons::build()}
                        {move || if running.get() { ui.tx("build-running", &[]) } else { ui.tx("build-run", &[]) }}
                    </button>
                    <span class="muted">{phase_text}</span>
                </div>
            </div>
            {move || result.get().map(|r| match r {
                Ok(s) => {
                    let n = s.warnings.len();
                    let built = s.environment.clone();
                    let msg = ui.tx("build-done", &[
                        ("generated", crate::num(s.generated as f64, 0)),
                        ("copied", crate::num(s.copied as f64, 0)),
                        ("produced", crate::num(s.images_produced as f64, 0)),
                        ("cached", crate::num(s.images_cached as f64, 0)),
                        ("seconds", crate::num(s.seconds as f64, 1)),
                    ]);
                    view! {
                        <div class="section" aria-live="polite">
                            <div class="section-head">
                                <p class="status status--ok">{icons::check()}<span>{msg}</span></p>
                                <button class="btn" on:click=move |_| preview(built.clone())>{icons::play()}{ui.t("preview-open")}</button>
                            </div>
                            <Show when=move || { n > 0 }>
                                <h3>{ui.t("build-warnings")}</h3>
                                <ul class="list-plain">{s.warnings.iter().map(|w| view! { <li>{w.clone()}</li> }).collect_view()}</ul>
                            </Show>
                        </div>
                    }.into_any()
                }
                Err(f) => view! { <div class="notice notice--error" role="alert">{ui.tx("build-failed", &[("message", ui.failure(&f))])}</div> }.into_any(),
            })}
        </section>
    }
}
