//! Ajout d'un hébergement (5.2, 5.3, D-202) : hébergeur choisi dans une liste (ou « Autre »),
//! chemin des accès dans son espace client, trois champs ; connexion testée (clé du serveur
//! enregistrée sans rien afficher), puis dossier du site choisi parmi ceux trouvés sur le serveur
//! (aucun d'office) ; enregistrement et publication seulement après ce choix.

use crate::{Ui, api, icons, publish};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::hosting::{HostPreset, PRESETS, parse_access, preset, preset_for_host};
use revel_protocol::{Failure, HostProbe, HostingProfile, HostingTarget};

#[derive(Clone, Copy, PartialEq)]
enum Choice {
    None,
    Preset(&'static HostPreset),
    Other,
}

impl Choice {
    fn preset(self) -> Option<&'static HostPreset> {
        match self {
            Self::Preset(p) => Some(p),
            _ => None,
        }
    }
    fn github(self) -> bool {
        self.preset().is_some_and(|p| !p.sftp)
    }
}

#[derive(Clone, Default)]
struct Fields {
    host: String,
    port: String,
    user: String,
    password: String,
    remote_dir: String,
    environment: String,
    repo: String,
}

#[derive(Clone)]
enum Phase {
    Edit,
    Connecting,
    Connected(HostProbe),
    Failed(Failure),
    Saving,
}

/// Nom d'un dépôt d'après le nom du site (`Atelier Lumen` -> `atelier-lumen`).
pub(crate) fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().to_lowercase().chars() {
        let c = match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "site".into() } else { out }
}

/// Fenêtre « Mettre en ligne », ouverte par `ui.setup`.
#[component]
pub fn SetupDialog() -> impl IntoView {
    let ui = expect_context::<Ui>();
    view! {
        <Show when=move || ui.setup.get()>
            <Setup/>
        </Show>
    }
}

#[component]
fn Setup() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let site = match ui.route.get_untracked() {
        crate::Route::Site { site, .. } => Some(site),
        _ => None,
    };
    let Some(site) = site else {
        ui.setup.set(false);
        return ().into_any();
    };
    let path = StoredValue::new(site.path.clone());
    let site_name = StoredValue::new(site.name.clone());
    let envs = site.environments.clone();
    let public_env = if envs.iter().any(|e| e == "production") || envs.is_empty() {
        "production".to_string()
    } else {
        envs[0].clone()
    };
    let test_env = envs.iter().find(|e| **e != public_env).cloned();
    let choice = RwSignal::new(Choice::None);
    let f = RwSignal::new(Fields {
        port: "22".into(),
        environment: public_env.clone(),
        ..Default::default()
    });
    let phase = RwSignal::new(Phase::Edit);
    // « Autre dossier… » choisi : chemin tapé.
    let other = RwSignal::new(false);
    let close = move || ui.setup.set(false);

    let pick = move |c: Choice| {
        choice.set(c);
        phase.set(Phase::Edit);
        f.update(|f| {
            // Dossier : aucun d'office, choisi après la connexion (D-202).
            f.remote_dir.clear();
            if let Some(p) = c.preset().filter(|p| p.sftp) {
                f.port = p.port.to_string();
            } else {
                f.port = "22".into();
            }
            if c.github() && f.repo.is_empty() {
                let owner = ui.github.get_untracked().unwrap_or_default();
                f.repo = format!("{owner}/{}", slug(&site_name.get_value()));
            }
        });
    };
    // Message de l'hébergeur collé dans un champ (sans champ dédié, D-202) : accès reconnus,
    // hébergeur déduit de l'adresse du serveur.
    let on_paste = move |text: String| {
        let a = parse_access(&text);
        if a.is_empty() {
            return;
        }
        // Hébergeur déduit de l'adresse du serveur (ou du port de Hostinger, qui donne une IP).
        let found = a.host.as_deref().and_then(preset_for_host).or_else(|| {
            (a.port == Some(65002))
                .then(|| preset("hostinger"))
                .flatten()
        });
        match found {
            Some(p) if choice.get_untracked().preset() != Some(p) => pick(Choice::Preset(p)),
            None if choice.get_untracked() == Choice::None => pick(Choice::Other),
            _ => {}
        }
        f.update(|f| {
            if let Some(h) = a.host {
                f.host = h;
            }
            if let Some(p) = a.port {
                f.port = p.to_string();
            }
            if let Some(u) = a.user {
                f.user = u;
            }
            if let Some(p) = a.password {
                f.password = p;
            }
        });
        phase.set(Phase::Edit);
    };

    let profile = move |known_host_key: Option<String>| -> HostingProfile {
        let c = choice.get_untracked();
        let v = f.get_untracked();
        let name = match c.preset() {
            Some(p) => p.name.to_string(),
            None => v.host.trim().to_string(),
        };
        let target = if c.github() {
            HostingTarget::GithubPages {
                repo: v.repo.trim().trim_matches('/').to_string(),
                branch: "gh-pages".into(),
                custom_domain: None,
            }
        } else {
            HostingTarget::Sftp {
                host: v.host.trim().to_string(),
                port: v.port.trim().parse().unwrap_or(22),
                user: v.user.trim().to_string(),
                remote_dir: v.remote_dir.trim().to_string(),
                known_host_key,
            }
        };
        HostingProfile {
            id: String::new(),
            name,
            environment: v.environment.clone(),
            target,
            sync_versions: true,
            site_url: None,
            preset: c.preset().map(|p| p.id.to_string()),
        }
    };
    let ready = move || {
        let c = choice.get();
        f.with(|v| {
            if c.github() {
                ui.github.with(|g| g.is_some())
                    && v.repo
                        .trim()
                        .trim_matches('/')
                        .split('/')
                        .filter(|p| !p.is_empty())
                        .count()
                        == 2
            } else {
                c != Choice::None
                    && !v.host.trim().is_empty()
                    && !v.user.trim().is_empty()
                    && !v.password.is_empty()
                    && v.port.trim().parse::<u16>().is_ok_and(|p| p > 0)
            }
        })
    };
    // Enregistrer et publier : après le choix du dossier (SFTP).
    let chosen = move || choice.get().github() || f.with(|v| !v.remote_dir.trim().is_empty());
    let busy = move || matches!(phase.get(), Phase::Connecting | Phase::Saving);
    let connected = move || matches!(phase.get(), Phase::Connected(_));

    let connect = move || {
        if !ready() || busy() {
            return;
        }
        phase.set(Phase::Connecting);
        let p = profile(None);
        let pw = f.with_untracked(|v| v.password.clone());
        spawn_local(async move {
            match api::hosting_probe(&path.get_value(), &p, Some(pw)).await {
                Ok(probe) => {
                    // Dossier choisi par l'utilisateur dans la liste (D-202) : rien d'office ; un
                    // choix fait à une connexion précédente reste s'il est encore là.
                    let cur = f.with_untracked(|v| v.remote_dir.trim().to_string());
                    if !other.get_untracked() && !probe.folders.contains(&cur) {
                        f.update(|v| v.remote_dir.clear());
                    }
                    phase.set(Phase::Connected(probe));
                }
                Err(e) => phase.set(Phase::Failed(e)),
            }
        });
    };
    // Enregistrement (et publication si `go`).
    let save = move |go: bool| {
        let key = match phase.get_untracked() {
            Phase::Connected(p) => Some(p.server_key),
            _ => None,
        };
        let github = choice.get_untracked().github();
        if !github && key.is_none() {
            return;
        }
        let p = profile(key);
        let pw = (!github).then(|| f.with_untracked(|v| v.password.clone()));
        phase.set(Phase::Saving);
        spawn_local(async move {
            match api::save_profile(&path.get_value(), &p, pw).await {
                Ok(saved) => {
                    ui.setup.set(false);
                    ui.publish_rev.update(|n| *n += 1);
                    if go {
                        ui.publish_panel.set(true);
                        publish::start(ui, path.get_value(), vec![saved.id], false, false);
                    }
                }
                Err(e) => phase.set(Phase::Failed(e)),
            }
        });
    };

    let on_key = window_event_listener(leptos::ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" && !busy() {
            close();
        }
    });
    on_cleanup(move || on_key.remove());

    let field = move |key: &'static str,
                      kind: &'static str,
                      cls: &'static str,
                      get: fn(&Fields) -> String,
                      set: fn(&mut Fields, String),
                      placeholder: Signal<String>| {
        view! {
            <label class="field">
                <span class="label">{ui.t(key)}</span>
                <input class=format!("input {cls}") type=kind autocomplete="off" spellcheck="false"
                    placeholder=move || placeholder.get()
                    prop:value=move || f.with(get)
                    on:input=move |ev| {
                        let v = event_target_value(&ev);
                        // Message entier collé dans un champ : accès reconnus.
                        if v.contains('\n') || (kind != "password" && v.len() > 120) {
                            on_paste(v);
                            return;
                        }
                        f.update(|f| set(f, v));
                        if matches!(phase.get_untracked(), Phase::Connected(_) | Phase::Failed(_)) {
                            phase.set(Phase::Edit);
                        }
                    }/>
            </label>
        }
    };
    let console = move || {
        choice.get().preset().map(|p| {
            if ui.lang() == "fr" {
                p.console_fr
            } else {
                p.console
            }
        })
    };
    let open_console = move |_| {
        if let Some(u) = console() {
            spawn_local(async move {
                if let Err(e) = api::open_external(u).await {
                    ui.fail(e);
                }
            });
        }
    };
    let host_placeholder = Signal::derive(move || {
        choice
            .get()
            .preset()
            .map(|p| p.host_example.to_string())
            .unwrap_or_else(|| "sftp.exemple.com".into())
    });
    let none = Signal::derive(String::new);

    view! {
        <div class="overlay" on:input=move |e| crate::dialog::mark_dirty(&e) on:mousedown=move |e| {
            if crate::dialog::backdrop_click(&e) && !busy() { close() }
        }>
            <form class="dialog setup" role="dialog" aria-modal="true" aria-labelledby="setup-title"
                on:submit=move |e| {
                    e.prevent_default();
                    if connected() || choice.get_untracked().github() { if chosen() { save(true) } } else { connect() }
                }>
                <div class="setup-head">
                    <h2 id="setup-title">{ui.t("setup-title")}</h2>
                    <button type="button" class="icon-btn" title=ui.t("action-close") aria-label=ui.t("action-close")
                        on:click=move |_| close() disabled=busy>{icons::close()}</button>
                </div>
                <div class="field">
                    <span class="label" id="setup-host-label">{ui.t("setup-host")}</span>
                    <div class="setup-choice">
                        <span class="select">
                            <select class="input setup-preset" aria-labelledby="setup-host-label"
                                prop:value=move || choice.get().preset().map(|p| p.id).unwrap_or("")
                                on:change=move |ev| {
                                    let v = event_target_value(&ev);
                                    pick(preset(&v).map(Choice::Preset).unwrap_or(Choice::None));
                                }>
                                <option value="" disabled selected=move || choice.get().preset().is_none()>{ui.t("setup-host-pick")}</option>
                                {PRESETS.iter().map(|p| view! { <option value=p.id>{p.name}</option> }).collect_view()}
                            </select>
                        </span>
                        <button type="button" class="btn setup-other" class:is-on=move || choice.get() == Choice::Other
                            aria-pressed=move || (choice.get() == Choice::Other).to_string()
                            title=ui.t("setup-other-help")
                            on:click=move |_| pick(Choice::Other)>{ui.t("setup-other")}</button>
                    </div>
                </div>
                // Où trouver les accès chez l'hébergeur choisi.
                {move || choice.get().preset().filter(|p| p.sftp).map(|p| view! {
                    <div class="setup-steps">
                        <ol>
                            {p.steps.iter().map(|k| view! { <li>{ui.t(k)}</li> }).collect_view()}
                        </ol>
                        <button type="button" class="btn btn--small setup-console" on:click=open_console>
                            {icons::external()}{ui.t("setup-console")}
                        </button>
                    </div>
                })}
                <Show when=move || choice.get().github()>
                    <div class="subsection">
                        <span class="label">{ui.t("github-account")}</span>
                        <crate::account::GithubAccount/>
                    </div>
                    {field("publish-repo", "text", "setup-repo", |f| f.repo.clone(), |f, v| f.repo = v, none)}
                </Show>
                <Show when=move || { let c = choice.get(); c != Choice::None && !c.github() }>
                    {field("publish-host", "text", "setup-server", |f| f.host.clone(), |f, v| f.host = v, host_placeholder)}
                    <div class="form-cols">
                        {field("publish-user", "text", "setup-user", |f| f.user.clone(), |f, v| f.user = v, none)}
                        {field("publish-password", "password", "setup-password", |f| f.password.clone(), |f, v| f.password = v, none)}
                    </div>
                    <details class="setup-advanced">
                        <summary>{ui.t("setup-advanced")}</summary>
                        <div class="form-cols">
                            {field("publish-port", "number", "setup-port", |f| f.port.clone(), |f, v| f.port = v, none)}
                        </div>
                    </details>
                </Show>
                {move || match phase.get() {
                    Phase::Connecting => view! {
                        <p class="setup-state" aria-live="polite">{ui.t("setup-connecting")}</p>
                    }.into_any(),
                    Phase::Connected(p) => view! {
                        <p class="setup-state status status--ok" aria-live="polite" title=p.fingerprint.clone()>
                            {icons::check()}<span>{ui.t("setup-connected")}</span>
                        </p>
                    }.into_any(),
                    Phase::Failed(e) => view! {
                        <div class="notice notice--error setup-error" role="alert"><span>{ui.failure(&e)}</span></div>
                    }.into_any(),
                    _ => ().into_any(),
                }}
                // Dossier du site : choisi parmi ceux trouvés sur le serveur, après la connexion.
                {move || match phase.get() {
                    Phase::Connected(p) => Some(folder_choice(ui, f, other, p)),
                    _ => None,
                }}
                {test_env.clone().map(|test| {
                    let public = public_env.clone();
                    let (p1, p2, t1, t2) = (public.clone(), public.clone(), test.clone(), test.clone());
                    view! {
                        <Show when=move || choice.get() != Choice::None>
                            <div class="segmented setup-env" role="radiogroup" aria-label=ui.t("publish-env") title=ui.t("setup-env-help")>
                                {
                                    let (pa, pb) = (p1.clone(), p2.clone());
                                    view! {
                                        <button type="button" role="radio" class:is-on=move || f.with(|v| v.environment == pa)
                                            aria-checked=move || f.with(|v| v.environment == pb).to_string()
                                            on:click={ let p = p1.clone(); move |_| f.update(|v| v.environment = p.clone()) }>{ui.t("pub-public")}</button>
                                    }
                                }
                                {
                                    let (ta, tb) = (t1.clone(), t2.clone());
                                    view! {
                                        <button type="button" role="radio" class:is-on=move || f.with(|v| v.environment == ta)
                                            aria-checked=move || f.with(|v| v.environment == tb).to_string()
                                            on:click={ let t = t1.clone(); move |_| f.update(|v| v.environment = t.clone()) }>{ui.t("pub-test")}</button>
                                    }
                                }
                            </div>
                        </Show>
                    }
                })}
                <div class="dialog-actions">
                    <Show when=move || connected() || choice.get().github()>
                        <button type="button" class="btn" disabled=move || busy() || !ready() || !chosen()
                            on:click=move |_| save(false)>{ui.t("action-save")}</button>
                    </Show>
                    <button type="submit" class="btn btn--primary"
                        disabled=move || busy() || !ready() || (connected() && !chosen())>
                        {move || if connected() || choice.get().github() {
                            view! { {icons::upload()}{ui.t("publish-run")} }.into_any()
                        } else {
                            ui.t("setup-connect")().into_any()
                        }}
                    </button>
                </div>
            </form>
        </div>
    }
    .into_any()
}

/// Dossier du site, choisi parmi ceux trouvés sur le serveur (D-202) : aucun coché d'office ; le
/// dossier proposé est seulement signalé (domaine du site, ou dossier habituel de l'hébergeur) ;
/// « Autre dossier… » pour un sous-dossier ou un dossier à créer.
fn folder_choice(ui: Ui, f: RwSignal<Fields>, other: RwSignal<bool>, p: HostProbe) -> AnyView {
    let tag = |folder: &str| -> Option<String> {
        if p.suggested.as_deref() != Some(folder) {
            return None;
        }
        let matches = p.domain.as_deref().is_some_and(|d| {
            folder.split('/').any(|part| part == d)
                || (d.split('.').count() >= 3
                    && folder.trim_end_matches('/').rsplit('/').next() == d.split('.').next())
        });
        Some(if matches {
            ui.now(
                "setup-folder-matches",
                &[("domain", p.domain.clone().unwrap_or_default())],
            )
        } else {
            ui.now("setup-folder-usual", &[])
        })
    };
    let rows: Vec<(String, Option<String>)> =
        p.folders.iter().map(|d| (d.clone(), tag(d))).collect();
    view! {
        <div class="field setup-folder-field">
            <span class="label" id="setup-folder-label" title=ui.t("setup-folder-help")>{ui.t("setup-folder")}</span>
            <div class="setup-folders" role="radiogroup" aria-labelledby="setup-folder-label">
                {rows.into_iter().map(|(d, t)| {
                    let (d1, d2, d3, d4) = (d.clone(), d.clone(), d.clone(), d.clone());
                    let on = move || !other.get() && f.with(|v| v.remote_dir == d1);
                    view! {
                        <button type="button" role="radio" class="setup-folder-row" data-folder=d4
                            class:is-on=on aria-checked=move || (!other.get() && f.with(|v| v.remote_dir == d2)).to_string()
                            on:click=move |_| { other.set(false); f.update(|v| v.remote_dir = d3.clone()) }>
                            <span class="setup-radio"></span>{icons::folder()}<span class="setup-folder-name">{d}</span>
                            {t.map(|t| view! { <span class="setup-folder-tag">{t}</span> })}
                        </button>
                    }
                }).collect_view()}
                <button type="button" role="radio" class="setup-folder-row setup-folder-row--other" data-folder=""
                    class:is-on=move || other.get() aria-checked=move || other.get().to_string()
                    on:click=move |_| { if !other.get_untracked() { other.set(true); f.update(|v| v.remote_dir.clear()) } }>
                    <span class="setup-radio"></span>{icons::edit()}<span class="setup-folder-name">{ui.t("setup-folder-other")}</span>
                </button>
            </div>
            <Show when=move || other.get()>
                <input class="input input--mono setup-folder" spellcheck="false" aria-labelledby="setup-folder-label"
                    placeholder="dossier/du/site/" prop:value=move || f.with(|v| v.remote_dir.clone())
                    on:input=move |ev| { let v = event_target_value(&ev); f.update(|f| f.remote_dir = v) }/>
            </Show>
        </div>
    }
    .into_any()
}
