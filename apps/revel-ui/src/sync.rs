//! Synchronisation automatique du travail à plusieurs (D-108), visible en haut à droite quand le
//! site a un dépôt commun (dossier partagé ou dépôt en ligne) :
//!
//! - toutes les minutes, Revel regarde si le dépôt commun a une version nouvelle (sans rien
//!   télécharger) ;
//! - une version nouvelle est reçue à la prochaine pause d'édition (quelques secondes sans saisie,
//!   aucun texte en cours d'édition), sans enregistrer de version : les changements en cours
//!   restent en cours et Ctrl+Z n'annule que les siens ;
//! - les versions enregistrées partent aussitôt (Paramètres › Versions, publication) ; une version
//!   restée en attente (hors ligne) part à la vérification suivante ;
//! - même endroit modifié des deux côtés : la synchronisation se met en pause et le dit ;
//!   « Vérifier » ouvre Paramètres › Versions, qui montre les endroits à choisir.

use crate::{Route, Section, Tab, Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

/// Pause d'édition avant de recevoir une version (ms).
const IDLE_MS: f64 = 3000.0;

/// Intervalle des vérifications (ms) : une minute ; `#syncevery=2` (secondes) pour les essais.
fn every_ms() -> f64 {
    let hash = web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .unwrap_or_default();
    hash.trim_start_matches('#')
        .split('&')
        .find_map(|kv| kv.strip_prefix("syncevery="))
        .and_then(|v| v.parse::<f64>().ok())
        .map_or(60_000.0, |s| s.max(1.0) * 1000.0)
}

/// État de la synchronisation du site ouvert.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SyncState {
    /// Le site a un dépôt commun.
    pub team: bool,
    /// Réception en cours.
    pub busy: bool,
    /// Dernière vérification (ms depuis 1970).
    pub checked: Option<f64>,
    /// Pas de réponse du dépôt commun à la dernière vérification.
    pub offline: bool,
    /// Dernières versions reçues : nombre, auteurs, heure.
    pub received: Option<(usize, Vec<String>, f64)>,
    /// Versions faites des deux côtés et changements en cours : réunies au prochain
    /// enregistrement d'une version.
    pub waiting: bool,
    /// Conflit : synchronisation en pause (nombre d'endroits).
    pub paused: Option<usize>,
    /// « Vérifier » demandé : Paramètres › Versions lance la synchronisation à l'ouverture.
    pub verify: bool,
}

/// Un champ de texte a le focus (une saisie n'est jamais interrompue) ; une case à cocher, un
/// bouton ou une liste n'en sont pas.
fn typing() -> bool {
    let Some(el) = document().active_element() else {
        return false;
    };
    if let Some(i) = el.dyn_ref::<web_sys::HtmlInputElement>() {
        return matches!(
            i.type_().as_str(),
            "" | "text" | "search" | "email" | "url" | "tel" | "number" | "password"
        );
    }
    el.tag_name().eq_ignore_ascii_case("TEXTAREA")
        || el
            .dyn_ref::<web_sys::HtmlElement>()
            .is_some_and(|h| h.is_content_editable())
}

/// Heure (14:32).
fn hm(ms: f64) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms));
    format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
}

#[component]
pub fn SyncButton() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let path = Memo::new(move |_| match ui.route.get() {
        Route::Site { site, .. } => Some(site.path),
        _ => None,
    });
    let open = RwSignal::new(false);
    let flash = RwSignal::new(None::<String>);
    let last_input = StoredValue::new(js_sys::Date::now());
    let editing = StoredValue::new(false);
    let pending = StoredValue::new(false);
    let checking = StoredValue::new(false);
    let next_check = StoredValue::new(0.0f64);
    // Activité : clavier et pointeur de la fenêtre, saisie dans le canevas (ses messages).
    let bump = move || last_input.set_value(js_sys::Date::now());
    let on_key = window_event_listener(leptos::ev::keydown, move |_| bump());
    let on_down =
        window_event_listener(leptos::ev::pointerdown, move |e: web_sys::PointerEvent| {
            bump();
            let inside = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .and_then(|el| el.closest(".sync").ok().flatten())
                .is_some();
            if !inside {
                open.set(false);
            }
        });
    let on_msg = window_event_listener(leptos::ev::message, move |e: web_sys::MessageEvent| {
        let d = e.data();
        let get = |k: &str| js_sys::Reflect::get(&d, &k.into()).ok();
        if get("revel").and_then(|v| v.as_f64()) != Some(1.0) {
            return;
        }
        match get("type").and_then(|v| v.as_string()).as_deref() {
            Some("editing") => {
                editing.set_value(get("slot").is_some_and(|v| !v.is_null() && !v.is_undefined()));
                bump();
            }
            Some("text" | "texts" | "key" | "moved" | "drop") => bump(),
            _ => {}
        }
    });
    // Dépôt commun du site ouvert.
    let refresh = move || {
        let Some(p) = path.get_untracked() else {
            return;
        };
        spawn_local(async move {
            let team = matches!(api::versions_status(&p).await, Ok(Some(s)) if s.remote.is_some());
            let _ = ui.sync.try_update(|s| s.team = team);
        });
    };
    Effect::new(move |_| {
        path.track();
        ui.sync.set(SyncState::default());
        pending.set_value(false);
        next_check.set_value(js_sys::Date::now() + 1500.0);
        refresh();
    });
    let check = move |p: String| {
        if checking.get_value() {
            return;
        }
        checking.set_value(true);
        spawn_local(async move {
            // Dépôt commun retiré entre-temps (travail seul) : plus de vérification.
            let st = api::versions_status(&p).await.ok().flatten();
            if st.as_ref().is_none_or(|s| s.remote.is_none()) {
                let _ = ui.sync.try_update(|s| s.team = false);
                let _ = checking.try_set_value(false);
                return;
            }
            let r = api::versions_incoming(&p).await;
            let now = js_sys::Date::now();
            match r {
                Ok(incoming) => {
                    // Versions enregistrées pas encore envoyées (hors ligne) : envoyées aussi.
                    let ahead = st.is_some_and(|s| s.ahead > 0);
                    if incoming || ahead {
                        let _ = pending.try_set_value(true);
                    }
                    let _ = ui.sync.try_update(|s| {
                        s.offline = false;
                        s.checked = Some(now);
                    });
                }
                Err(_) => {
                    let _ = ui.sync.try_update(|s| {
                        s.offline = true;
                        s.checked = Some(now);
                    });
                }
            }
            let _ = checking.try_set_value(false);
        });
    };
    let receive = move |p: String| {
        pending.set_value(false);
        ui.sync.update(|s| s.busy = true);
        let merge = ui.now("versions-merge-message", &[]);
        spawn_local(async move {
            let r = api::versions_receive(&p, &merge).await;
            let now = js_sys::Date::now();
            match r {
                Ok(r) if !r.conflicts.is_empty() => {
                    let n = r.items.len().max(r.conflicts.len());
                    let _ = ui.sync.try_update(|s| {
                        s.busy = false;
                        s.offline = false;
                        s.paused = Some(n);
                    });
                    let _ = open.try_set(true);
                }
                Ok(r) => {
                    let _ = ui.sync.try_update(|s| {
                        s.busy = false;
                        s.offline = false;
                        s.waiting = r.waiting;
                        if r.pulled > 0 {
                            s.received = Some((r.pulled, r.authors.clone(), now));
                        }
                    });
                    if r.pulled > 0 {
                        let who = r.authors.join(", ");
                        let text = ui.now(
                            "sync-received",
                            &[("count", r.pulled.to_string()), ("authors", who)],
                        );
                        let _ = flash.try_set(Some(text.clone()));
                        set_timeout(
                            move || {
                                if flash.try_get_untracked().flatten().as_deref() == Some(&text) {
                                    let _ = flash.try_set(None);
                                }
                            },
                            std::time::Duration::from_secs(6),
                        );
                        // Nom et langue du site relus (barre supérieure).
                        if let Ok(o) = api::edit_open(&p).await {
                            ui.site_identity(&o.site);
                        }
                    }
                }
                Err(_) => {
                    let _ = ui.sync.try_update(|s| {
                        s.busy = false;
                        s.offline = true;
                    });
                }
            }
            let _ = next_check.try_set_value(js_sys::Date::now() + every_ms());
        });
    };
    let every = every_ms();
    let next_team = StoredValue::new(0.0f64);
    let tick = set_interval_with_handle(
        move || {
            let Some(p) = path.get_untracked() else {
                return;
            };
            let st = ui.sync.get_untracked();
            // Dépôt commun réglé depuis (Paramètres › Versions) : vu en quelques secondes.
            if !st.team {
                let now = js_sys::Date::now();
                if now >= next_team.get_value() {
                    next_team.set_value(now + 4000.0);
                    refresh();
                }
                return;
            }
            if st.busy || st.paused.is_some() || !ui.settings.with_untracked(|s| s.auto_sync) {
                return;
            }
            let now = js_sys::Date::now();
            if pending.get_value() {
                if now - last_input.get_value() >= IDLE_MS && !editing.get_value() && !typing() {
                    receive(p);
                }
            } else if now >= next_check.get_value() {
                next_check.set_value(now + every);
                check(p);
            }
        },
        std::time::Duration::from_millis(500),
    )
    .ok();
    on_cleanup(move || {
        on_key.remove();
        on_down.remove();
        on_msg.remove();
        if let Some(t) = tick {
            t.clear();
        }
    });
    // Synchroniser maintenant : réception (et envoi) sans attendre de pause.
    let now = move || {
        let Some(p) = path.get_untracked() else {
            return;
        };
        ui.sync.update(|s| s.paused = None);
        receive(p);
    };
    let verify = move || {
        open.set(false);
        ui.sync.update(|s| s.verify = true);
        ui.tab(Tab::Settings(Section::Versions));
    };
    let state = move || {
        let s = ui.sync.get();
        let auto = ui.settings.with(|x| x.auto_sync);
        if s.paused.is_some() {
            "paused"
        } else if s.busy {
            "busy"
        } else if !auto {
            "off"
        } else if s.offline {
            "offline"
        } else if s.waiting {
            "waiting"
        } else {
            "ok"
        }
    };
    let title = move || {
        let s = ui.sync.get();
        match state() {
            "paused" => ui.tx(
                "sync-paused",
                &[("count", s.paused.unwrap_or(0).to_string())],
            ),
            "busy" => ui.tx("sync-busy", &[]),
            "off" => ui.tx("sync-off", &[]),
            "offline" => ui.tx("sync-offline", &[]),
            "waiting" => ui.tx("sync-waiting", &[]),
            _ => match s.checked {
                Some(t) => ui.tx("sync-ok", &[("time", hm(t))]),
                None => ui.tx("sync-title", &[]),
            },
        }
    };
    view! {
        <Show when=move || ui.sync.with(|s| s.team)>
            <div class="sync">
                <button class=move || format!("icon-btn sync-btn sync-btn--{}", state())
                    title=title aria-label=ui.t("sync-title") aria-haspopup="dialog" aria-expanded=move || open.get().to_string()
                    on:click=move |_| open.update(|o| *o = !*o)>
                    {icons::sync()}
                    {move || ui.sync.with(|s| s.paused.is_some()).then(|| view! { <span class="sync-dot"></span> })}
                </button>
                {move || flash.get().map(|t| view! { <div class="sync-flash" role="status">{t}</div> })}
                <Show when=move || open.get()>
                    <div class="sync-pop" role="dialog" aria-label=ui.t("sync-title")>
                        <strong class="sync-pop-title">{ui.t("sync-title")}</strong>
                        <p class=move || format!("sync-pop-state sync-pop-state--{}", state())>{title}</p>
                        {move || ui.sync.with(|s| s.received.clone()).filter(|_| state() != "paused").map(|(n, who, t)| view! {
                            <p class="muted small">{ui.tx("sync-last-received", &[("count", n.to_string()), ("authors", who.join(", ")), ("time", hm(t))])}</p>
                        })}
                        {move || if state() == "paused" {
                            view! {
                                <div class="sync-pop-actions">
                                    <button class="btn btn--small btn--primary sync-verify" on:click=move |_| verify()>{ui.t("sync-verify")}</button>
                                    <button class="btn btn--small sync-later" on:click=move |_| open.set(false)>{ui.t("sync-later")}</button>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="sync-pop-actions">
                                    <button class="btn btn--small sync-now" disabled=move || ui.sync.with(|s| s.busy)
                                        on:click=move |_| now()>{icons::sync()}{ui.t("sync-now")}</button>
                                    <button class="btn btn--small" on:click=move |_| { open.set(false); ui.tab(Tab::Settings(Section::Versions)); }>
                                        {ui.t("settings-versions")}
                                    </button>
                                </div>
                            }.into_any()
                        }}
                    </div>
                </Show>
            </div>
        </Show>
    }
}
