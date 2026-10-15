//! Site ouvert : vues (éditeur, CMS, localisation, paramètres) et rubriques des paramètres.

use crate::panels::Planned;
use crate::{Route, Section, Tab, Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{BuildEvent, BuildPhase, BuildSummary, Failure, SiteSummary};

/// Vues d'un site. L'éditeur, une fois ouvert, reste en place (caché) pendant que les autres
/// vues sont affichées : on y revient sans recharger le canevas.
#[component]
pub fn SiteView(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let tab = Memo::new(move |_| match ui.route.get() {
        Route::Site { tab, .. } => tab,
        _ => Tab::Editor,
    });
    let editor_seen = RwSignal::new(false);
    Effect::new(move |_| {
        if tab.get() == Tab::Editor {
            editor_seen.set(true);
        }
    });
    let s_editor = site.clone();
    let site = StoredValue::new(site);
    view! {
        <div class="site-view">
            <Show when=move || editor_seen.get()>
                <div class="view-pane view-pane--editor" class:is-hidden=move || tab.get() != Tab::Editor>
                    <crate::editor::Editor site=s_editor.clone()/>
                </div>
            </Show>
            {move || match tab.get() {
                Tab::Editor => ().into_any(),
                Tab::Cms => view! { <div class="view-pane"><crate::cms::CmsView site=site.get_value()/></div> }.into_any(),
                Tab::Locales => view! { <div class="view-pane view-pane--scroll"><Locales site=site.get_value()/></div> }.into_any(),
                Tab::Settings(sec) => view! { <div class="view-pane"><SiteSettings site=site.get_value() section=sec/></div> }.into_any(),
            }}
        </div>
    }
}

/// Rubriques des paramètres : (rubrique, libellé, étape prévue si pas encore disponible).
const SECTIONS: &[(Section, &str, Option<u8>)] = &[
    (Section::General, "settings-general", None),
    (Section::Publish, "settings-publish", None),
    (Section::Export, "settings-export", None),
    (Section::Seo, "settings-seo", Some(3)),
    (Section::Redirects, "settings-redirects", None),
    (Section::Fonts, "settings-fonts", Some(3)),
    (Section::Code, "settings-code", Some(3)),
    (Section::Forms, "settings-forms", Some(5)),
    (Section::Backups, "settings-backups", Some(3)),
];

/// Paramètres du site : rubriques à gauche (comme les réglages d'un site Webflow), contenu à droite.
#[component]
fn SiteSettings(site: SiteSummary, section: Section) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let body = match section {
        Section::General => view! { <General site=site.clone()/> }.into_any(),
        Section::Publish => view! { <crate::publish::Publish site=site.clone()/> }.into_any(),
        Section::Export => view! { <Export site=site.clone()/> }.into_any(),
        Section::Redirects => view! { <Redirects site=site.clone()/> }.into_any(),
        other => {
            let (key, step) = SECTIONS
                .iter()
                .find(|(s, ..)| *s == other)
                .map(|(_, k, st)| (*k, st.unwrap_or(3)))
                .unwrap_or(("settings-general", 3));
            view! {
                <section class="page settings-page">
                    <header class="page-head"><h1>{ui.t(key)}</h1></header>
                    <Planned key=planned_key(other) step=step icon=icons::settings/>
                </section>
            }
            .into_any()
        }
    };
    view! {
        <div class="settings-layout">
            <nav class="settings-nav" aria-label=ui.t("view-settings")>
                {SECTIONS.iter().map(|(sec, key, step)| {
                    let sec = *sec;
                    let title = step.map(|n| ui.tx("planned-step", &[("step", n.to_string())]));
                    view! {
                        <button class="settings-nav-item" class:is-on=section == sec class:is-planned=step.is_some()
                            data-section=*key title=title
                            on:click=move |_| ui.tab(Tab::Settings(sec))>
                            <span>{ui.t(key)}</span>
                            {step.map(|n| view! { <span class="settings-nav-step">{n}</span> })}
                        </button>
                    }
                }).collect_view()}
            </nav>
            <div class="settings-content">{body}</div>
        </div>
    }
}

/// Texte d'une rubrique prévue.
fn planned_key(s: Section) -> &'static str {
    match s {
        Section::Seo => "settings-seo-planned",
        Section::Fonts => "settings-fonts-planned",
        Section::Code => "settings-code-planned",
        Section::Forms => "settings-forms-planned",
        Section::Backups => "settings-backups-planned",
        _ => "settings-general",
    }
}

/// Première lettre en capitale (identifiants de collection affichés comme noms).
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// Nom d'une langue dans la langue de l'interface (`fr` -> « Français »).
pub fn language_name(ui: Ui, code: &str) -> String {
    let base = code.split(['-', '_']).next().unwrap_or(code).to_lowercase();
    let key = format!("lang-{base}");
    let name = ui.tx(&key, &[]);
    if name == key { code.to_string() } else { name }
}

/// Paramètres généraux : identité, structure, environnements.
#[component]
fn General(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    // Résumé relu (le site a pu changer dans l'éditeur).
    let fresh = RwSignal::new(site.clone());
    let path = site.path.clone();
    spawn_local(async move {
        if let Ok(s) = api::open_site(&path).await {
            fresh.set(s);
        }
    });
    let n_cols = move || {
        fresh.with(|s| {
            s.collections
                .iter()
                .filter(|c| !c.folder.is_empty())
                .count()
        })
    };
    view! {
        <section class="page settings-page">
            <header class="page-head"><h1>{ui.t("settings-general")}</h1></header>
            <div class="section">
                <h2>{ui.t("settings-identity")}</h2>
                <div class="form-grid">
                    <span class="label">{ui.t("settings-site-name")}</span>
                    <span class="strong">{move || fresh.with(|s| s.name.clone())}</span>
                    <span class="label">{ui.t("settings-site-url")}</span>
                    <span class="path">{move || fresh.with(|s| s.base_url.clone()).unwrap_or_else(|| "—".into())}</span>
                    <span class="label">{ui.t("settings-site-lang")}</span>
                    <span>{move || language_name(ui, &fresh.with(|s| s.locale.clone()))}</span>
                    <span class="label">{ui.t("settings-site-folder")}</span>
                    <span class="muted path">{move || fresh.with(|s| s.path.clone())}</span>
                </div>
            </div>
            <div class="section">
                <h2>{ui.t("overview-structure")}</h2>
                <dl class="facts">
                    <div><dt>{ui.t("site-pages")}</dt><dd>{move || fresh.with(|s| s.pages)}</dd></div>
                    <div><dt>{ui.t("site-layouts")}</dt><dd>{move || fresh.with(|s| s.layouts)}</dd></div>
                    <div><dt>{ui.t("site-components")}</dt><dd>{move || fresh.with(|s| s.components)}</dd></div>
                    <div>
                        <dt>{ui.t("site-collections")}</dt>
                        <dd><button class="link-btn" title=ui.t("view-cms") on:click=move |_| ui.tab(Tab::Cms)>{n_cols}</button></dd>
                    </div>
                </dl>
            </div>
            <div class="section">
                <h2>{ui.t("overview-environments")}</h2>
                <div class="chips">{move || fresh.with(|s| s.environments.clone()).into_iter().map(|e| view! { <span class="chip">{e}</span> }).collect_view()}</div>
            </div>
            <p class="muted help">{move || ui.tx("planned-edit", &[("step", "3".to_string())])}</p>
        </section>
    }
}

/// Redirections du site (lecture ; modification prévue à l'étape 5 avec les hébergeurs).
#[component]
fn Redirects(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let list = site.redirects.clone();
    let empty = list.is_empty();
    view! {
        <section class="page settings-page">
            <header class="page-head">
                <h1>{ui.t("settings-redirects")}</h1>
                <div class="toolbar">
                    <button class="btn" disabled=true title=move || ui.tx("planned-step", &[("step", "5".to_string())])>
                        {icons::plus()}{ui.t("redirects-add")}
                    </button>
                </div>
            </header>
            {if empty {
                view! { <div class="empty"><p>{ui.t("redirects-none")}</p></div> }.into_any()
            } else {
                view! {
                    <div class="section section--flush">
                        <table class="table table--compact">
                            <thead><tr><th>{ui.t("redirects-from")}</th><th></th><th>{ui.t("redirects-to")}</th></tr></thead>
                            <tbody>
                                {list.into_iter().map(|(from, to)| view! {
                                    <tr><td class="path">{from}</td><td class="muted col-arrow">{icons::arrow_right()}</td><td class="path">{to}</td></tr>
                                }).collect_view()}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            }}
        </section>
    }
}

/// Localisation : langues du site (traduction prévue à l'étape 3).
#[component]
fn Locales(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let code = site.locale.clone();
    let code2 = code.clone();
    view! {
        <section class="page">
            <header class="page-head">
                <h1>{ui.t("view-locales")}</h1>
                <div class="toolbar">
                    <button class="btn" disabled=true title=move || ui.tx("planned-step", &[("step", "3".to_string())])>
                        {icons::plus()}{ui.t("locales-add")}
                    </button>
                </div>
            </header>
            <div class="section section--flush">
                <table class="table">
                    <thead><tr>
                        <th>{ui.t("locales-lang")}</th>
                        <th>{ui.t("locales-code")}</th>
                        <th>{ui.t("locales-prefix")}</th>
                        <th>{ui.t("locales-status")}</th>
                    </tr></thead>
                    <tbody>
                        <tr>
                            <td class="strong">{move || language_name(ui, &code)}</td>
                            <td class="muted path">{code2.clone()}</td>
                            <td class="muted path">"/"</td>
                            <td><span class="chip chip--ok">{ui.t("locales-primary")}</span></td>
                        </tr>
                    </tbody>
                </table>
            </div>
            <Planned key="locales-planned" step=3 icon=icons::globe/>
        </section>
    }
}

/// Export du site : fichiers finaux (pages, styles, images optimisées) copiés dans un nouveau
/// dossier choisi, pour héberger le site ailleurs ou l'archiver. L'aperçu et la publication n'en
/// ont pas besoin (D-26).
#[component]
pub fn Export(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let env = RwSignal::new(ui.preview_environment(&site));
    let images = RwSignal::new(true);
    let parent = RwSignal::new(String::new());
    let running = RwSignal::new(false);
    let progress = RwSignal::new(None::<(BuildPhase, usize, usize)>);
    let result = RwSignal::new(None::<Result<BuildSummary, Failure>>);
    let path = StoredValue::new(site.path.clone());
    // Dossier proposé : celui des sites (Documents/Revel).
    spawn_local(async move {
        if let Ok(d) = api::default_sites_dir().await
            && parent.get_untracked().is_empty()
        {
            parent.set(d);
        }
    });
    let change = move |_| {
        spawn_local(async move {
            let start = parent.get_untracked();
            match api::pick_folder_at(Some(start.as_str()).filter(|s| !s.is_empty())).await {
                Ok(Some(p)) => parent.set(p),
                Ok(None) => {}
                Err(e) => ui.fail(e),
            }
        })
    };
    // Fin de l'export. Les événements du canal peuvent arriver après la réponse de la commande :
    // l'état final est fixé par le premier signal de fin, la progression qui suit est ignorée.
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
            let r = api::export(
                &path.get_value(),
                &env.get_untracked(),
                images.get_untracked(),
                &parent.get_untracked(),
                move |e| match e {
                    BuildEvent::Progress { .. } if result.with_untracked(|r| r.is_some()) => {}
                    BuildEvent::Progress { phase, done, total } => {
                        progress.set(Some((phase, done, total)));
                        let f = match phase {
                            BuildPhase::Pages => 0.05,
                            BuildPhase::Copies => 0.1,
                            BuildPhase::Images if total > 0 => {
                                0.1 + 0.85 * done as f32 / total as f32
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
        <section class="page settings-page">
            <header class="page-head">
                <div>
                    <h1>{ui.t("export-title")}</h1>
                    <p class="muted page-lead">{ui.t("export-lead")}</p>
                </div>
            </header>
            <div class="section">
                <div class="form-grid">
                    <span class="label" title=ui.t("export-env-help")>{ui.t("build-env")}</span>
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
                    <span class="label" title=ui.t("export-images-help")>{ui.t("build-images")}</span>
                    <label class="switch">
                        <input type="checkbox" prop:checked=move || images.get() on:change=move |ev| images.set(event_target_checked(&ev))/>
                        <span class="switch-track"></span>
                    </label>
                    <span class="label">{ui.t("export-where")}</span>
                    <div class="path-field">
                        <span class="path-value" title=move || parent.get()>{move || parent.get()}</span>
                        <button type="button" class="btn btn--small" on:click=change>{ui.t("new-site-change")}</button>
                    </div>
                </div>
                <div class="toolbar toolbar--start">
                    <button class="btn btn--primary" on:click=run disabled=move || running.get() || parent.with(String::is_empty)>
                        {icons::upload()}
                        {move || if running.get() { ui.tx("export-running", &[]) } else { ui.tx("export-run", &[]) }}
                    </button>
                    <span class="muted">{phase_text}</span>
                </div>
            </div>
            {move || result.get().map(|r| match r {
                Ok(s) => {
                    let n = s.warnings.len();
                    let dir = s.out_dir.clone();
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
                                <button class="btn" on:click=move |_| {
                                    let d = dir.clone();
                                    spawn_local(async move { if let Err(e) = api::reveal(&d).await { ui.fail(e) } });
                                }>{icons::folder()}{ui.t("export-reveal")}</button>
                            </div>
                            <p class="muted path">{s.out_dir.clone()}</p>
                            <Show when=move || { n > 0 }>
                                <h3>{ui.t("build-warnings")}</h3>
                                <ul class="list-plain">{s.warnings.iter().map(|w| view! { <li>{w.clone()}</li> }).collect_view()}</ul>
                            </Show>
                        </div>
                    }.into_any()
                }
                Err(f) => view! { <div class="notice notice--error" role="alert">{ui.tx("export-failed", &[("message", ui.failure(&f))])}</div> }.into_any(),
            })}
        </section>
    }
}
