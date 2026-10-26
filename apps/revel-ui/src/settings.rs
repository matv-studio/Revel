//! Réglages de l'application, sur une seule page : général (langue, thème, couleur d'accent et son
//! étendue, dossier des projets), comptes, langues de traduction automatique (D-36), assistants IA
//! (D-41).

use crate::dialog::Ask;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{AppSettings, PackState, TranslateEvent, TranslatePack};

/// Couleurs proposées (une seule est utilisée à la fois).
const PRESETS: &[&str] = &["#3D7EFF", "#7C5CFF", "#1F9D6B", "#E8890C", "#E5487A"];

/// Parties des réglages de Revel : (clé, titre, présentation).
const PARTS: &[(&str, &str, &str)] = &[
    ("general", "settings-general", "settings-app-general-lead"),
    ("accounts", "settings-profile", "settings-accounts-lead"),
    ("editor", "prefs-editor", "prefs-editor-help"),
    ("colors", "prefs-colors", "prefs-colors-help"),
    ("shortcuts", "prefs-shortcuts", "prefs-shortcuts-help"),
    ("translate", "settings-translate", "settings-translate-help"),
    ("updates", "update-title", "update-help"),
    (
        "build-tools",
        "settings-build-tools",
        "settings-build-tools-help",
    ),
    ("assistants", "settings-mcp", "settings-mcp-help"),
];

/// Réglages de l'application sur une seule page, comme les paramètres d'un site (D-104) : rail
/// des parties à gauche (la partie à l'écran raccordée à la page), chaque partie avec son titre
/// et sa présentation à gauche, ses réglages à droite.
#[component]
pub fn Settings() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let content = NodeRef::<leptos::html::Div>::new();
    let current = RwSignal::new(PARTS[0].0.to_string());
    let shown = RwSignal::new(PARTS.iter().map(|p| p.0.to_string()).collect::<Vec<_>>());
    let spy = move || crate::site::scroll_spy(content, current, shown);
    let part = move |key: &'static str, body: AnyView| {
        let (_, title, lead) = PARTS
            .iter()
            .find(|p| p.0 == key)
            .copied()
            .unwrap_or(PARTS[0]);
        crate::site::settings_part(ui, key, title, lead, body)
    };
    view! {
        <div class="settings-layout app-settings">
            <nav class="settings-nav" aria-label=ui.t("nav-settings")>
                {PARTS.iter().map(|(key, title, _)| {
                    let key: &'static str = key;
                    let on = move || current.with(|c| c == key);
                    view! {
                        <div class="settings-nav-group is-single" class:is-on=on>
                            <button class="settings-nav-item" class:is-on=on data-section=key
                                aria-current=move || on().then_some("location")
                                on:click=move |_| { current.set(key.to_string()); crate::site::scroll_to_part(key) }>
                                <span>{ui.t(title)}</span>
                            </button>
                        </div>
                    }
                }).collect_view()}
            </nav>
            <div class="settings-content" node_ref=content on:scroll=move |_| spy()>
                {part("general", view! { <General/> }.into_any())}
                {part("accounts", view! {
                    <section class="section settings-card" data-section="accounts">
                        <h2>{ui.t("settings-accounts")}</h2>
                        <div class="form-grid">
                            // Profil (D-106) : photo et nom, montrés en haut à droite ; le nom signe les
                            // versions (travail à plusieurs, D-89). Sans eux : ceux du compte GitHub.
                            <span class="label" title=ui.t("profile-picture-help")>{ui.t("profile-picture")}</span>
                            {crate::profile::picture_controls(ui)}
                            <label class="label" for="settings-author" title=ui.t("settings-author-help")>{ui.t("settings-author")}</label>
                            {crate::profile::name_input(ui, "settings-author")}
                            <span class="label" title=ui.t("settings-github-help")>"GitHub"</span>
                            <crate::account::GithubAccount/>
                        </div>
                    </section>
                }.into_any())}
                {part("editor", view! { <crate::prefs::EditorPrefs/> }.into_any())}
                {part("colors", view! { <crate::prefs::UiColors/> }.into_any())}
                {part("shortcuts", view! { <crate::prefs::Shortcuts/> }.into_any())}
                {part("translate", view! { <TranslatePacks/> }.into_any())}
                {part("updates", view! { <Updates/> }.into_any())}
                {part("build-tools", view! { <BuildToolsCard/> }.into_any())}
                {part("assistants", view! {
                    <section class="settings-card settings-card--bare" data-section="assistants">
                        <crate::assistants::Assistants/>
                    </section>
                }.into_any())}
            </div>
        </div>
    }
}

/// Mises à jour de Revel (6.6) : version installée, préversions (D-86), recherche, installation et
/// relance.
#[component]
fn Updates() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let version = RwSignal::new(String::new());
    // `None` : pas encore cherché ; `Some(Err)` : échec.
    let checked = RwSignal::new(None::<Result<revel_protocol::UpdateCheck, String>>);
    let busy = RwSignal::new(false);
    spawn_local(async move {
        if let Ok((v, _)) = api::app_info().await {
            let _ = version.try_set(v);
        }
    });
    let run_check = move || async move {
        let r = api::update_check().await;
        if let Ok(c) = &r {
            ui.update.set(c.available.clone());
        }
        let _ = checked.try_set(Some(r.map_err(|e| ui.failure(&e))));
        let _ = busy.try_set(false);
    };
    let check = move || {
        busy.set(true);
        spawn_local(run_check());
    };
    // Préversions (D-86) : réglage enregistré, puis nouvelle recherche selon lui.
    let set_prereleases = move |on: bool| {
        ui.settings.update(|s| s.prereleases = on);
        let s = ui.settings.get_untracked();
        busy.set(true);
        spawn_local(async move {
            if let Err(e) = api::set_settings(&s).await {
                ui.fail(e);
            }
            run_check().await;
        });
    };
    let install = move || {
        busy.set(true);
        spawn_local(async move {
            if let Err(e) = api::update_install().await {
                ui.fail(e);
            }
            let _ = busy.try_set(false);
        });
    };
    let available = move |u: revel_protocol::UpdateInfo| {
        let key = if u.prerelease {
            "update-available-pre"
        } else {
            "update-available"
        };
        (true, ui.tx(key, &[("version", u.version)]))
    };
    let status = move || match checked.get() {
        None => ui.update.get().map(available),
        Some(Ok(c)) if !c.enabled => Some((false, ui.tx("update-disabled", &[]))),
        Some(Ok(c)) => Some(match c.available {
            Some(u) => available(u),
            None => (false, ui.tx("update-none", &[])),
        }),
        Some(Err(e)) => Some((false, e)),
    };
    let notes = move || ui.update.get().and_then(|u| u.notes).unwrap_or_default();
    view! {
        <section class="section settings-card" data-section="updates">
            <h2 title=ui.t("update-help")>{ui.t("update-title")}</h2>
            <div class="form-grid">
                <span class="label">{ui.t("update-version")}</span>
                <span class="mono">{move || version.get()}</span>
                <span class="label" title=ui.t("update-prereleases-help")>{ui.t("update-prereleases")}</span>
                <label class="switch">
                    <input type="checkbox" class="update-prereleases"
                        prop:checked=move || ui.settings.with(|s| s.prereleases)
                        prop:disabled=move || busy.get()
                        aria-label=ui.t("update-prereleases")
                        on:change=move |e| set_prereleases(event_target_checked(&e))/>
                    <span class="switch-track"></span>
                </label>
            </div>
            <div class="ctl-line update-actions">
                <button class="btn" disabled=move || busy.get() on:click=move |_| check()>{ui.t("update-check")}</button>
                {move || ui.update.get().map(|_| view! {
                    <button class="btn btn--primary" disabled=move || busy.get() title=notes on:click=move |_| install()>
                        {icons::download()}{ui.t("update-install")}
                    </button>
                })}
            </div>
            {move || status().map(|(on, t)| view! { <p class="update-status" class:is-on=on>{t}</p> })}
        </section>
    }
}

/// Réglages enregistrés (interface mise à jour aussitôt, fichier ensuite).
pub(crate) fn update(ui: Ui, f: &dyn Fn(&mut AppSettings)) {
    ui.settings.update(|s| f(s));
    let s = ui.settings.get_untracked();
    spawn_local(async move {
        if let Err(e) = api::set_settings(&s).await {
            ui.fail(e);
        }
    });
}

/// Général : langue, thème, couleur d'accent et son étendue, dossier des nouveaux sites.
#[component]
fn General() -> impl IntoView {
    let ui = expect_context::<Ui>();
    // Noms des langues dans leur propre langue, quelle que soit celle de l'interface.
    let lang_btn = move |value: &'static str, label: &'static str| {
        view! {
            <button role="radio" lang=value class:is-on=move || ui.settings.with(|s| s.lang == value)
                aria-checked=move || ui.settings.with(|s| (s.lang == value).to_string())
                on:click=move |_| update(ui, &|s| s.lang = value.to_string())>{label}</button>
        }
    };
    let theme_btn = move |value: &'static str, key: &'static str| {
        view! {
            <button role="radio" class:is-on=move || ui.settings.with(|s| s.theme == value)
                aria-checked=move || ui.settings.with(|s| (s.theme == value).to_string())
                on:click=move |_| update(ui, &|s| s.theme = value.to_string())>{ui.t(key)}</button>
        }
    };
    let accent = move || ui.settings.with(|s| s.accent.clone());
    // Dossier des projets en vigueur (choisi, sinon celui proposé par l'application).
    let dir = RwSignal::new(String::new());
    let reload = move || {
        spawn_local(async move {
            if let Ok(d) = api::default_sites_dir().await
                && !dir.is_disposed()
            {
                dir.set(d);
            }
        })
    };
    reload();
    let custom = move || ui.settings.with(|s| s.projects_dir.is_some());
    let change = move |_| {
        let start = dir.get_untracked();
        spawn_local(async move {
            match api::pick_folder_at(Some(&start)).await {
                Ok(Some(d)) => {
                    update(ui, &|s| s.projects_dir = Some(d.clone()));
                    // Après l'enregistrement des réglages.
                    set_timeout(reload, std::time::Duration::from_millis(150));
                }
                Ok(None) => {}
                Err(e) => ui.fail(e),
            }
        });
    };
    let reset = move |_| {
        update(ui, &|s| s.projects_dir = None);
        set_timeout(reload, std::time::Duration::from_millis(150));
    };
    view! {
        <section class="section settings-card" data-section="general">
            <h2>{ui.t("settings-general")}</h2>
            <div class="form-grid">
                <span class="label">{ui.t("settings-lang")}</span>
                    <div class="segmented" role="radiogroup">
                        {lang_btn("fr", "Français")}
                        {lang_btn("en", "English")}
                    </div>
                    <span class="label">{ui.t("settings-theme")}</span>
                    <div class="segmented" role="radiogroup">
                        {theme_btn("system", "theme-system")}
                        {theme_btn("dark", "theme-dark")}
                        {theme_btn("light", "theme-light")}
                    </div>
                    <span class="label" title=ui.t("settings-accent-help")>{ui.t("settings-accent")}</span>
                    <div class="swatches" role="radiogroup">
                        <button role="radio" class="swatch swatch--none" class:is-on=move || accent().is_none()
                            title=ui.t("accent-none") aria-label=ui.t("accent-none")
                            on:click=move |_| update(ui, &|s| s.accent = None)></button>
                        {PRESETS.iter().map(|c| {
                            let c: &'static str = c;
                            view! {
                                <button role="radio" class="swatch" class:is-on=move || accent().as_deref() == Some(c)
                                    style:background=c aria-label=c title=c
                                    on:click=move |_| update(ui, &|s| s.accent = Some(c.to_string()))></button>
                            }
                        }).collect_view()}
                        <label class="swatch swatch--custom" title=ui.t("accent-custom")>
                            <input type="color" aria-label=ui.t("accent-custom")
                                prop:value=move || accent().unwrap_or_else(|| "#808080".into())
                                on:change=move |ev| { let v = event_target_value(&ev); update(ui, &|s| s.accent = Some(v.clone())) }/>
                        </label>
                    </div>
                    <span class="label" title=ui.t("settings-accent-scope-help")>{ui.t("settings-accent-scope")}</span>
                    <div class="segmented accent-scope" role="radiogroup" class:is-disabled=move || accent().is_none()
                        title=move || accent().is_none().then(|| ui.tx("settings-accent-scope-none", &[]))>
                        {[("minimal", "accent-scope-minimal", "accent-scope-minimal-help"),
                          ("standard", "accent-scope-standard", "accent-scope-standard-help"),
                          ("full", "accent-scope-full", "accent-scope-full-help")]
                            .into_iter().map(|(value, key, help)| view! {
                                <button role="radio" data-scope=value
                                    class:is-on=move || ui.settings.with(|s| s.accent_scope == value)
                                    aria-checked=move || ui.settings.with(|s| (s.accent_scope == value).to_string())
                                    prop:disabled=move || accent().is_none()
                                    title=ui.t(help)
                                    on:click=move |_| update(ui, &|s| s.accent_scope = value.to_string())>{ui.t(key)}</button>
                            }).collect_view()}
                    </div>
                    <span class="label" title=ui.t("settings-projects-help")>{ui.t("settings-projects")}</span>
                    <div class="path-field projects-dir">
                        <span class="path-value" title=move || dir.get()>{move || crate::ltr(&dir.get())}</span>
                        <Show when=custom>
                            <button type="button" class="icon-btn icon-btn--small" title=ui.t("settings-projects-reset")
                                aria-label=ui.t("settings-projects-reset") on:click=reset>{icons::reset()}</button>
                        </Show>
                        <button type="button" class="btn btn--small" on:click=change>{ui.t("new-site-change")}</button>
                    </div>
                </div>
        </section>
    }
}

/// Taille lisible (Mo, Go).
pub(crate) fn size(ui: Ui, bytes: u64) -> String {
    let mb = bytes as f64 / 1_000_000.0;
    if mb >= 1000.0 {
        ui.now("size-gb", &[("n", format!("{:.1}", mb / 1000.0))])
    } else if mb >= 1.0 {
        ui.now("size-mb", &[("n", format!("{mb:.0}"))])
    } else {
        ui.now(
            "size-kb",
            &[("n", format!("{:.0}", (bytes as f64 / 1000.0).max(1.0)))],
        )
    }
}

/// Langues de traduction automatique : livrées, téléchargées (retirables), à télécharger. Le
/// téléchargement continue quand on quitte les réglages.
#[component]
fn TranslatePacks() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let packs = RwSignal::new(None::<Vec<TranslatePack>>);
    let reload = move || {
        spawn_local(async move {
            match api::translate_packs().await {
                Ok(p) => {
                    if !packs.is_disposed() {
                        packs.set(Some(p))
                    }
                }
                Err(e) => ui.fail(e),
            }
        })
    };
    reload();
    // Téléchargement terminé (ou arrêté) : liste relue.
    Effect::new(move |prev: Option<bool>| {
        let busy = ui.pack_job.with(Option::is_some);
        if prev == Some(true) && !busy {
            reload();
        }
        busy
    });
    let install = move |code: String| {
        if ui.pack_job.with_untracked(Option::is_some) {
            return;
        }
        ui.pack_job.set(Some((code.clone(), Some(0.0))));
        ui.activity.set(Some(0.02));
        spawn_local(async move {
            let r = api::translate_install(&code, move |e| {
                let f = match e {
                    TranslateEvent::Progress { done, total } => {
                        Some(done as f32 / total.max(1) as f32)
                    }
                    TranslateEvent::Preparing => None,
                };
                ui.pack_job.update(|j| {
                    if let Some(j) = j {
                        j.1 = f;
                    }
                });
                ui.activity.set(Some(f.unwrap_or(0.98).clamp(0.02, 0.98)));
            })
            .await;
            ui.pack_job.set(None);
            ui.activity.set(None);
            if let Err(e) = r
                && e.key != "error-translate-cancelled"
            {
                ui.fail(e);
            }
        });
    };
    let remove = move |code: String| {
        let name = revel_model::language_name(&code);
        ui.ask.set(Some(
            Ask::confirm(
                ui.now("settings-translate-remove-title", &[("name", name)]),
                None,
                ui.now("action-remove", &[]),
                move || {
                    let code = code.clone();
                    spawn_local(async move {
                        if let Err(e) = api::translate_remove(&code).await {
                            ui.fail(e);
                        }
                        reload();
                    })
                },
            )
            .danger(),
        ));
    };
    let row = move |p: TranslatePack| {
        let code = StoredValue::new(p.code.clone());
        let job = move || {
            ui.pack_job
                .with(|j| j.as_ref().filter(|j| j.0 == code.get_value()).map(|j| j.1))
        };
        let state = p.state;
        view! {
            <li class="pack" data-pack=p.code.clone() class:is-on=matches!(state, PackState::Bundled | PackState::Installed)>
                <span class="pack-name">{revel_model::language_name(&p.code)}</span>
                <span class="muted path pack-code">{p.code.clone()}</span>
                <span class="pack-side">
                    {move || match (state, job()) {
                        (_, Some(f)) => view! {
                            <span class="meter-bar pack-bar" class:is-busy=f.is_none()>
                                <span class="meter-fill" style:width=format!("{:.1}%", f.unwrap_or(1.0) * 100.0)></span>
                            </span>
                            <span class="pack-num">{match f {
                                Some(f) => format!("{:.0} %", f * 100.0),
                                None => ui.now("settings-translate-preparing", &[]),
                            }}</span>
                            <button class="icon-btn icon-btn--small" title=ui.t("action-cancel") aria-label=ui.t("action-cancel")
                                disabled=f.is_none()
                                on:click=move |_| { spawn_local(async { let _ = api::translate_install_cancel().await; }); }>{icons::close()}</button>
                        }.into_any(),
                        (PackState::Bundled | PackState::Installed, None) => view! {
                            <span class="muted pack-size">{size(ui, p.disk)}</span>
                            <button class="icon-btn icon-btn--small pack-remove" title=ui.t("settings-translate-remove") aria-label=ui.t("settings-translate-remove")
                                on:click=move |_| remove(code.get_value())>{icons::trash()}</button>
                        }.into_any(),
                        (PackState::BundledOff, None) => view! {
                            <span class="muted pack-size">{size(ui, p.disk)}</span>
                            <button class="btn btn--small pack-get" title=ui.t("settings-translate-add-help")
                                disabled=move || ui.pack_job.with(Option::is_some)
                                on:click=move |_| install(code.get_value())>{icons::plus()}{ui.t("settings-translate-add")}</button>
                        }.into_any(),
                        (PackState::Available, None) => view! {
                            <span class="muted pack-size">{size(ui, p.disk)}</span>
                            <button class="btn btn--small pack-get"
                                title=ui.tx("settings-translate-get-help", &[("size", size(ui, p.download))])
                                disabled=move || ui.pack_job.with(Option::is_some)
                                on:click=move |_| install(code.get_value())>{icons::download()}{ui.t("settings-translate-get")}</button>
                        }.into_any(),
                    }}
                </span>
            </li>
        }
    };
    // Langues installées seulement ; les autres sous « Plus de langues » (celle en cours de
    // téléchargement reste affichée).
    let more = RwSignal::new(false);
    let job_code = Memo::new(move |_| ui.pack_job.with(|j| j.as_ref().map(|j| j.0.clone())));
    let installed =
        |p: &TranslatePack| matches!(p.state, PackState::Bundled | PackState::Installed);
    let others = move || {
        packs.with(|l| {
            l.as_ref()
                .map_or(0, |l| l.iter().filter(|p| !installed(p)).count())
        })
    };
    // Aucune langue installée (aucune n'est livrée avec Revel, D-79) : dit sans ouvrir la liste.
    let none = move || {
        packs.with(|l| l.as_ref().is_some_and(|l| !l.iter().any(installed)))
            && job_code.with(Option::is_none)
    };
    view! {
        <section class="section settings-card" data-section="translate">
            <h2 title=ui.t("settings-translate-help")>{ui.t("settings-translate")}</h2>
            <Show when=none>
                <p class="muted packs-none">{ui.t("settings-translate-none")}</p>
            </Show>
            <ul class="packs">
                {move || packs.get().map(|mut list| {
                    list.sort_by_key(|p| match p.state {
                        PackState::Bundled | PackState::Installed => 0,
                        PackState::BundledOff => 1,
                        PackState::Available => 2,
                    });
                    let all = more.get();
                    let job = job_code.get();
                    list.into_iter()
                        .filter(|p| all || installed(p) || job.as_deref() == Some(p.code.as_str()))
                        .map(row)
                        .collect_view()
                })}
            </ul>
            {move || (others() > 0).then(|| view! {
                <button class="btn btn--small packs-more" aria-expanded=move || more.get().to_string()
                    on:click=move |_| more.update(|m| *m = !*m)>
                    <span class="packs-more-icon" class:is-open=move || more.get()>{icons::chevron_down()}</span>
                    {move || if more.get() {
                        ui.now("settings-translate-less", &[])
                    } else {
                        ui.now("settings-translate-more", &[("count", others().to_string())])
                    }}
                </button>
            })}
        </section>
    }
}

/// Outils de construction téléchargés par Revel (D-88) : version, taille, retrait ; paquets en
/// cache, vidés à la demande.
#[component]
fn BuildToolsCard() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let state = RwSignal::new(None::<revel_protocol::BuildTools>);
    let busy = RwSignal::new(false);
    let reload = move || {
        spawn_local(async move {
            if let Ok(t) = api::build_tools().await
                && !state.is_disposed()
            {
                state.set(Some(t));
            }
            let _ = busy.try_set(false);
        });
    };
    reload();
    let remove = move |id: String| {
        busy.set(true);
        spawn_local(async move {
            if let Err(e) = api::build_tool_remove(&id).await {
                ui.fail(e);
            }
            reload();
        });
    };
    let clear = move || {
        busy.set(true);
        spawn_local(async move {
            if let Err(e) = api::build_cache_clear().await {
                ui.fail(e);
            }
            reload();
        });
    };
    view! {
        <section class="section settings-card" data-section="build-tools">
            <h2 title=ui.t("settings-build-tools-help")>{ui.t("settings-build-tools")}</h2>
            {move || state.get().map(|t| {
                let none = t.tools.is_empty();
                view! {
                    {none.then(|| view! { <p class="muted packs-none">{ui.t("build-tools-none")}</p> })}
                    <ul class="packs build-tools">
                        {t.tools.into_iter().map(|tool| {
                            let id = tool.id.clone();
                            view! {
                                <li class="pack is-on" data-tool=tool.id.clone()>
                                    <span class="pack-name">{tool.name.clone()}</span>
                                    <span class="muted path pack-code">{tool.version.clone()}</span>
                                    <span class="pack-side">
                                        <span class="muted pack-size">{size(ui, tool.size)}</span>
                                        <button class="icon-btn icon-btn--small pack-remove build-tool-remove"
                                            title=ui.t("build-tools-remove") aria-label=ui.t("build-tools-remove")
                                            disabled=move || busy.get()
                                            on:click=move |_| remove(id.clone())>{icons::trash()}</button>
                                    </span>
                                </li>
                            }
                        }).collect_view()}
                        {(t.cache > 0).then(|| view! {
                            <li class="pack build-cache">
                                <span class="pack-name">{ui.t("build-tools-cache")}</span>
                                <span class="pack-side">
                                    <span class="muted pack-size">{size(ui, t.cache)}</span>
                                    <button class="btn btn--small build-cache-clear" disabled=move || busy.get()
                                        on:click=move |_| clear()>{ui.t("build-tools-clear")}</button>
                                </span>
                            </li>
                        })}
                    </ul>
                }
            })}
        </section>
    }
}
