//! Réglages de l'application : langue, thème, couleur d'accent et son étendue, langues de
//! traduction automatique (D-36).

use crate::dialog::Ask;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{AppSettings, PackState, TranslateEvent, TranslatePack};

/// Couleurs proposées (une seule est utilisée à la fois).
const PRESETS: &[&str] = &["#3D7EFF", "#7C5CFF", "#1F9D6B", "#E8890C", "#E5487A"];

#[component]
pub fn Settings() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let update = move |f: &dyn Fn(&mut AppSettings)| {
        ui.settings.update(|s| f(s));
        let s = ui.settings.get_untracked();
        spawn_local(async move {
            if let Err(e) = api::set_settings(&s).await {
                ui.fail(e);
            }
        });
    };
    let theme_btn = move |value: &'static str, key: &'static str| {
        view! {
            <button role="radio" class:is-on=move || ui.settings.with(|s| s.theme == value)
                aria-checked=move || ui.settings.with(|s| (s.theme == value).to_string())
                on:click=move |_| update(&|s| s.theme = value.to_string())>{ui.t(key)}</button>
        }
    };
    // Noms des langues dans leur propre langue, quelle que soit celle de l'interface.
    let lang_btn = move |value: &'static str, label: &'static str| {
        view! {
            <button role="radio" lang=value class:is-on=move || ui.settings.with(|s| s.lang == value)
                aria-checked=move || ui.settings.with(|s| (s.lang == value).to_string())
                on:click=move |_| update(&|s| s.lang = value.to_string())>{label}</button>
        }
    };
    let accent = move || ui.settings.with(|s| s.accent.clone());
    view! {
        <section class="page page--narrow">
            <header class="page-head">
                <h1>{ui.t("nav-settings")}</h1>
            </header>
            <div class="section">
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
                            on:click=move |_| update(&|s| s.accent = None)></button>
                        {PRESETS.iter().map(|c| {
                            let c: &'static str = c;
                            view! {
                                <button role="radio" class="swatch" class:is-on=move || accent().as_deref() == Some(c)
                                    style:background=c aria-label=c title=c
                                    on:click=move |_| update(&|s| s.accent = Some(c.to_string()))></button>
                            }
                        }).collect_view()}
                        <label class="swatch swatch--custom" title=ui.t("accent-custom")>
                            <input type="color" aria-label=ui.t("accent-custom")
                                prop:value=move || accent().unwrap_or_else(|| "#808080".into())
                                on:change=move |ev| { let v = event_target_value(&ev); update(&|s| s.accent = Some(v.clone())) }/>
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
                                    on:click=move |_| update(&|s| s.accent_scope = value.to_string())>{ui.t(key)}</button>
                            }).collect_view()}
                    </div>
                </div>
            </div>
            <TranslatePacks/>
            <div class="section">
                <h2>{ui.t("settings-accounts")}</h2>
                <div class="form-grid">
                    <span class="label" title=ui.t("settings-github-help")>"GitHub"</span>
                    <crate::account::GithubAccount/>
                </div>
            </div>
        </section>
    }
}

/// Taille lisible (Mo, Go).
fn size(ui: Ui, bytes: u64) -> String {
    let mb = bytes as f64 / 1_000_000.0;
    if mb >= 1000.0 {
        ui.now("size-gb", &[("n", format!("{:.1}", mb / 1000.0))])
    } else {
        ui.now("size-mb", &[("n", format!("{mb:.0}"))])
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
    view! {
        <div class="section">
            <h2 title=ui.t("settings-translate-help")>{ui.t("settings-translate")}</h2>
            <ul class="packs">
                {move || packs.get().map(|mut list| {
                    list.sort_by_key(|p| match p.state {
                        PackState::Bundled | PackState::Installed => 0,
                        PackState::BundledOff => 1,
                        PackState::Available => 2,
                    });
                    list.into_iter().map(row).collect_view()
                })}
            </ul>
        </div>
    }
}
