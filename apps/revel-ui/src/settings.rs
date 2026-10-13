//! Réglages de l'application : langue, thème, couleur d'accent.

use crate::{Ui, api};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::AppSettings;

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
                    <span class="label">{ui.t("settings-accent")}</span>
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
                    <span></span>
                    <p class="muted help">{ui.t("settings-accent-help")}</p>
                </div>
            </div>
            <div class="section">
                <h2>{ui.t("settings-accounts")}</h2>
                <div class="form-grid">
                    <span class="label">"GitHub"</span>
                    <crate::account::GithubAccount/>
                    <span></span>
                    <p class="muted help">{ui.t("settings-github-help")}</p>
                </div>
            </div>
        </section>
    }
}
