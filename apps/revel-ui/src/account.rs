//! Compte GitHub : connexion par code (device flow), état, déconnexion.
//!
//! Le même composant sert dans *Réglages > Comptes* et dans le formulaire d'un hébergement
//! GitHub Pages (connexion sans quitter l'écran en cours).

use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{Failure, GithubEvent};

#[derive(Clone, PartialEq)]
enum Step {
    Idle,
    /// Demande du code en cours.
    Starting,
    /// Code à saisir sur github.com.
    Code {
        code: String,
        uri: String,
    },
    Failed(Failure),
}

#[component]
pub fn GithubAccount() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let step = RwSignal::new(Step::Idle);

    let connect = move |_| {
        step.set(Step::Starting);
        spawn_local(async move {
            let r = api::github_connect(move |e| match e {
                GithubEvent::Code {
                    user_code,
                    verification_uri,
                    ..
                } => step.set(Step::Code {
                    code: user_code,
                    uri: verification_uri,
                }),
                GithubEvent::Connected { login } => {
                    ui.github.set(Some(login));
                    step.set(Step::Idle);
                }
                GithubEvent::Failed(f) => step.set(Step::Failed(f)),
            })
            .await;
            match r {
                Ok(login) => {
                    ui.github.set(Some(login));
                    step.set(Step::Idle);
                }
                // Annulation : retour à l'état initial, sans message.
                Err(f) if f.key == "error-github-cancelled" => step.set(Step::Idle),
                Err(f) => step.set(Step::Failed(f)),
            }
        });
    };
    let cancel = move |_| {
        spawn_local(async move {
            let _ = api::github_cancel().await;
        });
        step.set(Step::Idle);
    };
    let disconnect = move |_| {
        spawn_local(async move {
            match api::github_disconnect().await {
                Ok(()) => {
                    ui.github.set(None);
                    ui.gh_sites.set(None);
                }
                Err(e) => ui.fail(e),
            }
        });
    };

    // Un seul élément racine : le composant occupe une seule cellule d'une grille de formulaire.
    let body = move || {
        match (ui.github.get(), step.get()) {
        (Some(login), _) => view! {
            <div class="account">
                <span class="account-id">{icons::link()}<span>{move || ui.tx("github-connected", &[("login", login.clone())])}</span></span>
                <button class="btn" on:click=disconnect>{ui.t("github-disconnect")}</button>
            </div>
        }
        .into_any(),
        (None, Step::Code { code, uri }) => {
            let (c1, c2, u) = (code.clone(), code.clone(), uri.clone());
            view! {
                <div class="device-code" aria-live="polite">
                    <p>{ui.t("github-code-help")}</p>
                    <code class="code code--large">{c1}</code>
                    <div class="toolbar toolbar--start">
                        <button class="btn btn--primary" on:click=move |_| {
                            api::copy_text(&c2);
                            let u = u.clone();
                            spawn_local(async move {
                                if let Err(e) = api::open_external(&u).await {
                                    ui.fail(e);
                                }
                            });
                        }>{icons::external()}{ui.t("github-open")}</button>
                        <button class="btn" on:click=move |_| api::copy_text(&code)>{icons::copy()}{ui.t("github-copy")}</button>
                        <button class="btn" on:click=cancel>{ui.t("action-cancel")}</button>
                    </div>
                    <p class="muted help">{ui.t("github-waiting")}</p>
                </div>
            }
            .into_any()
        }
        (None, s) => {
            let starting = s == Step::Starting;
            let failure = match s {
                Step::Failed(f) => Some(f),
                _ => None,
            };
            view! {
                <div class="account">
                    <span class="muted">{ui.t("github-signed-out")}</span>
                    <button class="btn" on:click=connect disabled=starting>{icons::link()}{ui.t("github-connect")}</button>
                </div>
                {failure.map(|f| view! { <div class="notice notice--error" role="alert">{ui.failure(&f)}</div> })}
            }
            .into_any()
        }
    }
    };
    view! { <div class="stack">{body}</div> }
}
