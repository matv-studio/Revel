//! Première publication (audit M11, maquette `tools-5-publish-assistant` validée, D-203) : un site
//! sans hébergement se publie gratuitement sur GitHub Pages en trois étapes — compte GitHub, nom
//! du site (adresse montrée), mise en ligne (Revel crée le dépôt, envoie le site et active
//! GitHub Pages). Un hébergement payant (SFTP) reste possible dans Paramètres › Publication.

use crate::{Ui, api, icons, publish};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{Failure, HostingProfile, HostingTarget};

/// Fenêtre ouverte par `ui.first_publish`.
#[component]
pub fn FirstPublishDialog() -> impl IntoView {
    let ui = expect_context::<Ui>();
    view! {
        <Show when=move || ui.first_publish.get()>
            <FirstPublish/>
        </Show>
    }
}

/// Nom de dépôt accepté par GitHub (lettres, chiffres, `-`, `_`, `.`).
fn valid_name(n: &str) -> bool {
    !n.is_empty()
        && n != "."
        && n != ".."
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

#[component]
fn FirstPublish() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let Some(site) = (match ui.route.get_untracked() {
        crate::Route::Site { site, .. } => Some(site),
        _ => None,
    }) else {
        ui.first_publish.set(false);
        return ().into_any();
    };
    let path = StoredValue::new(site.path.clone());
    let env = if site.environments.iter().any(|e| e == "production") || site.environments.is_empty()
    {
        "production".to_string()
    } else {
        site.environments[0].clone()
    };
    let env = StoredValue::new(env);
    let name = RwSignal::new(crate::setup::slug(&site.name));
    let busy = RwSignal::new(false);
    let failed: RwSignal<Option<Failure>> = RwSignal::new(None);
    let close = move || {
        if !busy.get_untracked() {
            ui.first_publish.set(false)
        }
    };
    let signed = move || ui.github.get();
    let ready = move || signed().is_some() && name.with(|n| valid_name(n.trim())) && !busy.get();
    let address = move || {
        let owner = signed().unwrap_or_else(|| "…".into()).to_lowercase();
        format!("https://{owner}.github.io/{}/", name.get().trim())
    };
    let go = move || {
        let Some(owner) = ui.github.get_untracked() else {
            return;
        };
        if !ready() {
            return;
        }
        busy.set(true);
        failed.set(None);
        let profile = HostingProfile {
            id: String::new(),
            name: "GitHub Pages".into(),
            environment: env.get_value(),
            target: HostingTarget::GithubPages {
                repo: format!("{owner}/{}", name.get_untracked().trim()),
                branch: "gh-pages".into(),
                custom_domain: None,
            },
            sync_versions: true,
            site_url: None,
            preset: Some("github".into()),
        };
        spawn_local(async move {
            match api::save_profile(&path.get_value(), &profile, None).await {
                Ok(saved) => {
                    busy.set(false);
                    ui.first_publish.set(false);
                    ui.publish_rev.update(|n| *n += 1);
                    ui.publish_panel.set(true);
                    publish::start(ui, path.get_value(), vec![saved.id], false, false);
                }
                Err(e) => {
                    busy.set(false);
                    failed.set(Some(e));
                }
            }
        });
    };
    let on_key = window_event_listener(leptos::ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" {
            close();
        }
    });
    on_cleanup(move || on_key.remove());
    let step_class = move |n: u8| {
        let done = match n {
            1 => signed().is_some(),
            _ => false,
        };
        let current = match n {
            1 => signed().is_none(),
            2 => signed().is_some(),
            _ => false,
        };
        format!(
            "fp-step{}{}",
            if done { " is-done" } else { "" },
            if current { " is-current" } else { "" }
        )
    };
    view! {
        <div class="overlay" on:mousedown=move |e| if crate::dialog::backdrop_click(&e) { close() }>
            <form class="dialog fp" role="dialog" aria-modal="true" aria-labelledby="fp-title"
                on:submit=move |e| { e.prevent_default(); go() }>
                <h2 id="fp-title" class="fp-title">
                    {ui.t("fp-title")}
                    <span class="info-tip" title=ui.t("fp-title-help") aria-label=ui.t("fp-title-help") role="img">{icons::info()}</span>
                </h2>
                <ol class="fp-steps">
                    <li class=move || step_class(1) data-step="sign">
                        <span class="fp-n">{move || if signed().is_some() { icons::check().into_any() } else { view! { "1" }.into_any() }}</span>
                        <div class="fp-body">
                            <b>{ui.t("fp-step-sign")}</b>
                            {move || match signed() {
                                Some(login) => view! { <span class="fp-note">{ui.now("fp-signed", &[("name", login)])}</span> }.into_any(),
                                None => view! { <crate::account::GithubAccount/> }.into_any(),
                            }}
                        </div>
                    </li>
                    <li class=move || step_class(2) data-step="name">
                        <span class="fp-n">"2"</span>
                        <div class="fp-body">
                            <b>{ui.t("fp-step-name")}</b>
                            <input class="input fp-name" spellcheck="false" aria-label=ui.t("fp-step-name")
                                class:is-invalid=move || !name.with(|n| valid_name(n.trim()))
                                disabled=move || signed().is_none()
                                prop:value=move || name.get()
                                on:input=move |e| name.set(event_target_value(&e))/>
                            <span class="fp-url">{address}</span>
                        </div>
                    </li>
                    <li class="fp-step" data-step="online">
                        <span class="fp-n">"3"</span>
                        <div class="fp-body">
                            <b>{ui.t("fp-step-online")}</b>
                            <span class="fp-note">{ui.t("fp-step-online-help")}</span>
                        </div>
                    </li>
                </ol>
                {move || failed.get().map(|f| view! {
                    <div class="notice notice--error" role="alert"><span>{ui.failure(&f)}</span></div>
                })}
                <div class="dialog-actions">
                    <button type="button" class="btn" disabled=move || busy.get() on:click=move |_| close()>{ui.t("action-cancel")}</button>
                    <button type="submit" class="btn btn--primary" disabled=move || !ready()>{ui.t("fp-continue")}</button>
                </div>
            </form>
        </div>
    }
    .into_any()
}
