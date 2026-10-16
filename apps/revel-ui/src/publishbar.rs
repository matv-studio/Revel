//! Bouton « Publier » de la barre supérieure (5.1) : destinations du site, dernière publication,
//! pages modifiées depuis, publication sans quitter la vue ; point si des modifications ne sont
//! pas en ligne.

use crate::dialog::Ask;
use crate::publish::{self, Run};
use crate::{Route, Section, Tab, Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{ChangeKind, Failure, PublishDestination};
use std::collections::HashSet;
use wasm_bindgen::JsCast;

/// Relecture de l'état de publication (point des modifications non publiées).
const POLL: std::time::Duration = std::time::Duration::from_secs(8);

/// `https://alunites.com/` -> `alunites.com`.
fn host_of(url: &str) -> String {
    let s = url.trim();
    let s = s.split_once("://").map(|(_, r)| r).unwrap_or(s);
    s.trim_end_matches('/').to_string()
}

/// Date de la dernière publication : relative le jour même, sinon date et heure locales.
fn published_text(ui: Ui, at_ms: i64) -> String {
    let age = ((js_sys::Date::now() - at_ms as f64) / 1000.0).max(0.0);
    let n = |v: f64| (v as u64).to_string();
    if age < 60.0 {
        ui.tx("pub-at-now", &[])
    } else if age < 3600.0 {
        ui.tx("pub-at-min", &[("n", n(age / 60.0))])
    } else if age < 20.0 * 3600.0 {
        ui.tx("pub-at-hours", &[("n", n(age / 3600.0))])
    } else {
        ui.tx("pub-at-date", &[("date", date_text(ui, at_ms))])
    }
}

/// Date et heure locales courtes (« 16 oct., 10:02 »).
fn date_text(ui: Ui, at_ms: i64) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(at_ms as f64));
    let opts = js_sys::Object::new();
    for (k, v) in [
        ("day", "numeric"),
        ("month", "short"),
        ("hour", "2-digit"),
        ("minute", "2-digit"),
    ] {
        let _ = js_sys::Reflect::set(&opts, &k.into(), &v.into());
    }
    d.to_locale_string(&ui.lang(), &opts)
        .as_string()
        .unwrap_or_default()
}

/// Résumé des modifications : pages, ou styles et médias seulement.
fn changes_text(ui: Ui, d: &PublishDestination) -> String {
    let pages = d
        .changes
        .iter()
        .filter(|c| c.kind != ChangeKind::Files)
        .count();
    if pages == 0 {
        ui.tx("pub-files-changed", &[])
    } else {
        ui.tx("pub-changes", &[("n", pages.to_string())])
    }
}

#[component]
pub fn PublishButton() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let open = ui.publish_panel;
    // État lu pour un site (dossier, destinations) ; échec de la dernière lecture.
    let state = RwSignal::new(None::<(String, Vec<PublishDestination>)>);
    let error = RwSignal::new(None::<Failure>);
    // Destinations décochées (les autres sont publiées).
    let unchecked = RwSignal::new(HashSet::<String>::new());
    let expanded = RwSignal::new(None::<String>);
    let loading = StoredValue::new(false);
    let site_path = Memo::new(move |_| match ui.route.get() {
        Route::Site { site, .. } => Some(site.path),
        _ => None,
    });

    let refresh = move || {
        let Some(path) = site_path.get_untracked() else {
            return;
        };
        if loading.get_value() {
            return;
        }
        loading.set_value(true);
        spawn_local(async move {
            let r = api::publish_state(&path).await;
            if state.is_disposed() {
                return;
            }
            loading.set_value(false);
            match r {
                Ok(l) => {
                    state.set(Some((path, l)));
                    error.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    // Relue à l'ouverture d'un site, en changeant de vue, après une publication ou un changement
    // d'hébergement, puis régulièrement (sauf fenêtre cachée ou publication en cours).
    Effect::new(move |_| {
        ui.publish_rev.track();
        ui.route.track();
        refresh();
    });
    let timer = set_interval_with_handle(
        move || {
            let hidden = web_sys::window()
                .and_then(|w| w.document())
                .is_some_and(|d| d.hidden());
            if !hidden && !publish::running(ui) {
                refresh();
            }
        },
        POLL,
    )
    .ok();
    on_cleanup(move || {
        if let Some(t) = timer {
            t.clear();
        }
    });

    let list = move || {
        state.with(|s| match s {
            Some((p, l)) if Some(p) == site_path.get().as_ref() => Some(l.clone()),
            _ => None,
        })
    };
    let dirty = move || {
        list().is_some_and(|l| {
            l.iter()
                .any(|d| d.published_at.is_some() && !d.changes.is_empty())
        })
    };
    let runs = move || -> Vec<Run> {
        let here = site_path.get();
        ui.deploys.with(|d| {
            d.iter()
                .filter(|r| Some(&r.site) == here.as_ref())
                .cloned()
                .collect()
        })
    };
    let busy = move || publish::running(ui);
    let selected = move || -> Vec<String> {
        let off = unchecked.get();
        list()
            .unwrap_or_default()
            .into_iter()
            .filter(|d| d.ready && !off.contains(&d.profile))
            .map(|d| d.profile)
            .collect()
    };
    let toggle = move |_| {
        let now = !open.get_untracked();
        if now {
            refresh();
            // Publications réussies déjà vues : retirées (les échecs restent affichés).
            if !publish::running(ui) {
                ui.deploys
                    .update(|d| d.retain(|r| !matches!(r.done, Some(Ok(_)))));
            }
        }
        open.set(now);
    };
    let publish_now = move |_| {
        let Some(path) = site_path.get_untracked() else {
            return;
        };
        expanded.set(None);
        publish::start(ui, path, selected(), false, false);
    };
    // Publication arrêtée par les contrôles (5.4) : points à vérifier, « Publier quand même ».
    let stopped = move || {
        runs()
            .into_iter()
            .find(|r| r.stopped_by_checks())
            .map(|r| r.checks)
    };
    let anyway = move |_| {
        let Some(path) = site_path.get_untracked() else {
            return;
        };
        publish::start(ui, path, selected(), false, true);
    };
    let dismiss = move |_| {
        let here = site_path.get_untracked();
        ui.deploys
            .update(|d| d.retain(|r| Some(&r.site) != here.as_ref()));
    };
    let goto = move |url: String| {
        open.set(false);
        ui.goto_page.set(Some(url));
        ui.tab(Tab::Editor);
    };
    let manage = move |_| {
        open.set(false);
        ui.tab(Tab::Settings(Section::Publish));
    };
    let setup = move |_| {
        open.set(false);
        ui.setup.set(true);
    };

    let on_down =
        window_event_listener(leptos::ev::pointerdown, move |e: web_sys::PointerEvent| {
            let inside = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .and_then(|el| el.closest(".pub-wrap, .overlay").ok().flatten())
                .is_some();
            if !inside {
                open.set(false);
            }
        });
    let on_key = window_event_listener(leptos::ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" && open.get_untracked() {
            open.set(false);
        }
    });
    on_cleanup(move || {
        on_down.remove();
        on_key.remove();
        open.set(false);
    });

    view! {
        <div class="pub-wrap">
            <button class="btn btn--primary pub-btn" aria-haspopup="dialog" aria-expanded=move || open.get().to_string()
                class:is-busy=busy on:click=toggle>
                {ui.t("nav-publish")}
                <Show when=move || dirty() && !busy()>
                    <span class="pub-dot" title=ui.t("pub-dirty") aria-label=ui.t("pub-dirty")></span>
                </Show>
            </button>
            <Show when=move || open.get()>
                <div class="menu pub-pop" role="dialog" aria-label=ui.t("pub-panel")>
                    {move || match list() {
                        None => match error.get() {
                            Some(f) => view! {
                                <p class="pub-error">{ui.failure(&f)}</p>
                            }.into_any(),
                            None => view! { <div class="pub-loading meter-bar is-busy"><div class="meter-fill"></div></div> }.into_any(),
                        },
                        Some(l) if l.is_empty() => view! {
                            <div class="pub-empty">
                                <p class="pub-empty-title">{ui.t("pub-empty")}</p>
                                <button class="btn btn--primary" on:click=setup>{icons::upload()}{ui.t("pub-setup")}</button>
                            </div>
                        }.into_any(),
                        Some(l) => {
                            let multi = l.len() > 1;
                            view! {
                                <ul class="pub-dests">
                                    {l.into_iter().map(|d| view! {
                                        <Destination d=d multi=multi unchecked=unchecked expanded=expanded
                                            runs=Signal::derive(runs) open=open/>
                                    }).collect_view()}
                                </ul>
                                {move || stopped().map(|checks| view! {
                                    <div class="pub-checks">
                                        <p class="pub-checks-title">{ui.t("check-title")}</p>
                                        <publish::ChecksView checks=checks on_page=goto/>
                                    </div>
                                })}
                                <div class="pub-foot">
                                    {move || if stopped().is_some() {
                                        view! {
                                            <button class="btn pub-manage" on:click=dismiss>{ui.t("action-cancel")}</button>
                                            <button class="btn btn--primary pub-anyway" on:click=anyway disabled=move || busy() || selected().is_empty()>
                                                {icons::upload()}{ui.t("publish-anyway")}
                                            </button>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <button class="btn pub-manage" title=ui.t("pub-hosting-help") on:click=manage>
                                                {icons::settings()}{ui.t("pub-hosting")}
                                            </button>
                                            <button class="btn btn--primary" on:click=publish_now
                                                disabled=move || busy() || selected().is_empty()>
                                                {icons::upload()}{ui.t("publish-run")}
                                            </button>
                                        }.into_any()
                                    }}
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </Show>
        </div>
    }
}

/// Une destination : adresse, dernière publication, pages modifiées, avancement ou résultat.
#[component]
fn Destination(
    d: PublishDestination,
    multi: bool,
    unchecked: RwSignal<HashSet<String>>,
    expanded: RwSignal<Option<String>>,
    runs: Signal<Vec<Run>>,
    open: RwSignal<bool>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let id = StoredValue::new(d.profile.clone());
    let run = move || runs.with(|r| r.iter().find(|r| r.profile == id.get_value()).cloned());
    let running = move || run().is_some_and(|r| r.done.is_none());
    let ok = move || run().is_some_and(|r| matches!(r.done, Some(Ok(_))));
    let title = d
        .url
        .as_deref()
        .map(host_of)
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| d.name.clone());
    let env = if d.environment == "production" {
        "pub-public"
    } else {
        "pub-test"
    };
    let ready = d.ready;
    let published_at = d.published_at;
    let previous_at = d.previous_at.filter(|_| ready);
    // Adresse du site : donnée par la publication qui vient de réussir, sinon connue.
    let url = {
        let known = d.url.clone();
        move || {
            run()
                .and_then(|r| match r.done {
                    Some(Ok(s)) => s.url,
                    _ => None,
                })
                .or_else(|| known.clone())
        }
    };
    let changes = StoredValue::new(d.changes.clone());
    let summary = StoredValue::new(d.clone());
    let has_changes = !d.changes.is_empty();
    let is_open = move || expanded.get().as_deref() == Some(id.get_value().as_str());
    let meta = move || {
        let when = if ok() {
            ui.tx("pub-at-now", &[])
        } else {
            match published_at {
                Some(at) => published_text(ui, at),
                None => ui.tx("pub-never", &[]),
            }
        };
        format!("{} · {}", ui.tx(env, &[]), when)
    };
    let goto = move |url: String| {
        open.set(false);
        ui.goto_page.set(Some(url));
        ui.tab(Tab::Editor);
    };
    view! {
        <li class="pub-dest" class:is-off=!ready>
            <div class="pub-dest-row">
                {multi.then(|| view! {
                    <input type="checkbox" class="pub-check" aria-label=ui.t("pub-select") disabled=move || !ready || running()
                        prop:checked=move || ready && !unchecked.with(|u| u.contains(&id.get_value()))
                        on:change=move |e| {
                            let on = event_target_checked(&e);
                            unchecked.update(|u| { if on { u.remove(&id.get_value()); } else { u.insert(id.get_value()); } })
                        }/>
                })}
                <div class="pub-dest-text" title=d.name.clone()>
                    <span class="pub-dest-name">{title}</span>
                    <span class="pub-dest-meta">
                        {move || if ready {
                            meta().into_any()
                        } else {
                            view! {
                                {ui.tx(env, &[])}" · "
                                <button class="link-btn" title=ui.t("pub-hosting-help") on:click=move |_| {
                                    open.set(false);
                                    ui.tab(Tab::Settings(Section::Publish));
                                }>{ui.t("pub-needs-access")}</button>
                            }.into_any()
                        }}
                    </span>
                </div>
                <div class="pub-dest-side">
                    <Show when=ok>
                        <span class="chip chip--ok">{icons::check()}{ui.t("pub-live")}</span>
                    </Show>
                    <Show when=move || run().is_some_and(|r| r.done.is_none() && r.status.is_none())>
                        <span class="muted pub-waiting">{ui.t("pub-waiting")}</span>
                    </Show>
                    {previous_at.map(|at| {
                        let label = move || ui.tx("pub-rollback-help", &[("date", date_text(ui, at))]);
                        // Masqué juste après une publication (place de « En ligne »).
                        view! { <Show when=move || !ok()>
                            <button class="icon-btn pub-rollback" title=label aria-label=label
                                disabled=move || publish::running(ui)
                                on:click=move |_| {
                                    let Some(path) = ui.site_path() else { return };
                                    open.set(false);
                                    let id = id.get_value();
                                    ui.ask.set(Some(Ask::confirm(
                                        ui.now("pub-rollback-title", &[("date", date_text(ui, at))]),
                                        None,
                                        ui.now("pub-rollback-confirm", &[]),
                                        move || {
                                            ui.publish_panel.set(true);
                                            publish::start_rollback(ui, path.clone(), id.clone());
                                        },
                                    )));
                                }>{icons::undo()}</button>
                        </Show> }
                    })}
                    {move || url().map(|u| view! {
                        <button class="icon-btn" title=ui.t("publish-open-site") aria-label=ui.t("publish-open-site")
                            on:click=move |_| {
                                let u = u.clone();
                                spawn_local(async move {
                                    if let Err(e) = api::open_external(&u).await {
                                        ui.fail(e);
                                    }
                                });
                            }>{icons::external()}</button>
                    })}
                </div>
            </div>
            // Avancement de la publication en cours.
            {move || run().and_then(|r| r.status.filter(|_| r.done.is_none())).map(|s| view! {
                <p class="pub-status">{s}</p>
            })}
            // Échec : message, accès direct aux réglages de l'hébergement s'ils sont en cause.
            {move || run().and_then(|r| match r.done { Some(Err(f)) if f.key != publish::CHECKS_KEY => Some(f), _ => None }).map(|f| {
                let fixable = publish::FIXABLE.contains(&f.key.as_str());
                view! {
                    <div class="pub-fail" role="alert">
                        <span>{ui.failure(&f)}</span>
                        {fixable.then(|| view! {
                            <button class="btn btn--small" on:click=move |_| {
                                open.set(false);
                                ui.tab(Tab::Settings(Section::Publish));
                            }>{ui.t("publish-fix")}</button>
                        })}
                    </div>
                }
            })}
            {move || {
                let warnings = run().map(|r| r.warnings).unwrap_or_default();
                (!warnings.is_empty()).then(|| view! {
                    <ul class="pub-warnings">
                        {warnings.iter().map(|w| view! {
                            <li class:is-blocking=w.blocking>{publish::warning_text(&ui.lang(), w)}</li>
                        }).collect_view()}
                    </ul>
                })
            }}
            // Pages modifiées depuis la dernière publication (repliées).
            <Show when=move || has_changes && !running() && !ok()>
                <button class="pub-changes" aria-expanded=move || is_open().to_string()
                    on:click=move |_| {
                        let next = (!is_open()).then(|| id.get_value());
                        expanded.set(next);
                    }>
                    <span class="pub-chevron" class:is-open=is_open>{icons::chevron_right()}</span>
                    {move || summary.with_value(|d| changes_text(ui, d))}
                </button>
            </Show>
            <Show when=move || published_at.is_some() && !has_changes && !running() && !ok() && ready>
                <p class="pub-uptodate">{ui.t("pub-up-to-date")}</p>
            </Show>
            <Show when=is_open>
                <ul class="pub-change-list">
                    {changes.get_value().into_iter().map(|c| {
                        let (label, kind) = match c.kind {
                            ChangeKind::Added => (None, "pub-kind-added"),
                            ChangeKind::Changed => (None, "pub-kind-changed"),
                            ChangeKind::Removed => (None, "pub-kind-removed"),
                            ChangeKind::Files => (Some("pub-files"), ""),
                        };
                        let name = match label {
                            Some(k) => ui.tx(k, &[]),
                            None if c.title.trim().is_empty() => c.url.clone(),
                            None => c.title.clone(),
                        };
                        let link = matches!(c.kind, ChangeKind::Added | ChangeKind::Changed);
                        let url = c.url.clone();
                        view! {
                            <li>
                                {if link {
                                    let u = url.clone();
                                    view! {
                                        <button class="pub-change-link" title=ui.t("pub-page-open") on:click=move |_| goto(u.clone())>
                                            <span class="pub-change-name">{name}</span>
                                            <span class="pub-change-url">{url}</span>
                                        </button>
                                    }.into_any()
                                } else {
                                    view! {
                                        <span class="pub-change-plain">
                                            <span class="pub-change-name">{name}</span>
                                            <span class="pub-change-url">{url}</span>
                                        </span>
                                    }.into_any()
                                }}
                                {(!kind.is_empty()).then(|| view! {
                                    <span class=format!("pub-kind pub-kind--{}", kind.trim_start_matches("pub-kind-"))>{ui.t(kind)}</span>
                                })}
                            </li>
                        }
                    }).collect_view()}
                </ul>
            </Show>
        </li>
    }
}
