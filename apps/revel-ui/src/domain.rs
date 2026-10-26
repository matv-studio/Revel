//! Nom de domaine d'un hébergement (5.7) : enregistrements à créer chez le gestionnaire du
//! domaine (prêts à copier pour GitHub Pages, chemin dans l'espace client pour les autres
//! hébergeurs), vérification par Revel (adresse, HTTPS, page d'accueil publiée), domaine
//! enregistré comme adresse du site.

use crate::assistants::CopyBtn;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{DomainStatus, Failure, HostingProfile, HostingTarget};

/// Adresses de GitHub Pages (domaine racine).
const GITHUB_A: &[&str] = &[
    "185.199.108.153",
    "185.199.109.153",
    "185.199.110.153",
    "185.199.111.153",
];
const GITHUB_AAAA: &[&str] = &[
    "2606:50c0:8000::153",
    "2606:50c0:8001::153",
    "2606:50c0:8002::153",
    "2606:50c0:8003::153",
];

/// `https://www.exemple.fr/x` -> `www.exemple.fr`.
fn host_of(url: &str) -> String {
    let s = url.trim();
    let s = s.split_once("://").map(|(_, r)| r).unwrap_or(s);
    s.split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

/// Domaine racine (`www.exemple.fr` -> `exemple.fr`) : enregistrements A sur `@`.
fn apex(domain: &str) -> &str {
    domain.strip_prefix("www.").unwrap_or(domain)
}

/// Fenêtre « Nom de domaine », ouverte par `ui.domain_for` (identifiant de l'hébergement).
#[component]
pub fn DomainDialog() -> impl IntoView {
    let ui = expect_context::<Ui>();
    view! {
        {move || ui.domain_for.get().map(|id| view! { <Domain id=id/> })}
    }
}

#[derive(Clone)]
enum Check {
    Idle,
    Running,
    Done(DomainStatus),
    Failed(Failure),
}

#[component]
fn Domain(id: String) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let Some(path) = ui.site_path() else {
        return ().into_any();
    };
    let base_url = match ui.route.get_untracked() {
        crate::Route::Site { site, .. } => site.base_url,
        _ => None,
    };
    let path = StoredValue::new(path);
    let profile = RwSignal::new(None::<HostingProfile>);
    let domain = RwSignal::new(String::new());
    let check = RwSignal::new(Check::Idle);
    let saving = RwSignal::new(false);
    let close = move || ui.domain_for.set(None);
    {
        let id = id.clone();
        spawn_local(async move {
            match api::profiles(&path.get_value()).await {
                Ok(l) => {
                    let Some((p, _)) = l.into_iter().find(|(p, _)| p.id == id) else {
                        ui.domain_for.set(None);
                        return;
                    };
                    // Domaine connu : domaine personnalisé, adresse du site publié (hors
                    // github.io), adresse publique du site.
                    let known = match &p.target {
                        HostingTarget::GithubPages { custom_domain, .. } => custom_domain.clone(),
                        _ => None,
                    }
                    .or_else(|| p.site_url.as_deref().map(host_of))
                    .or_else(|| {
                        (p.environment == "production")
                            .then(|| base_url.as_deref().map(host_of))
                            .flatten()
                    })
                    .filter(|d| !d.ends_with(".github.io") && !d.is_empty())
                    .unwrap_or_default();
                    if !domain.is_disposed() {
                        domain.set(known);
                        profile.set(Some(p));
                    }
                }
                Err(e) => ui.fail(e),
            }
        });
    }
    let github_owner = move || {
        profile.with(|p| match p.as_ref().map(|p| &p.target) {
            Some(HostingTarget::GithubPages { repo, .. }) => {
                Some(repo.split('/').next().unwrap_or_default().to_string())
            }
            _ => None,
        })
    };
    let preset = move || profile.with(|p| p.as_ref().and_then(|p| p.preset.clone()));
    let valid = move || {
        domain.with(|d| {
            let d = host_of(d);
            d.contains('.') && !d.starts_with('.') && !d.ends_with(".github.io")
        })
    };
    let verify = move || {
        if !valid() {
            return;
        }
        check.set(Check::Running);
        let id = id.clone();
        let d = host_of(&domain.get_untracked());
        spawn_local(async move {
            let r = api::domain_check(&path.get_value(), &id, &d).await;
            if check.is_disposed() {
                return;
            }
            check.set(match r {
                Ok(s) => Check::Done(s),
                Err(e) => Check::Failed(e),
            });
        });
    };
    let verify = StoredValue::new(verify);
    let save = move || {
        let Some(mut p) = profile.get_untracked() else {
            return;
        };
        let d = host_of(&domain.get_untracked());
        if let HostingTarget::GithubPages { custom_domain, .. } = &mut p.target {
            *custom_domain = Some(d.clone());
        }
        p.site_url = Some(format!("https://{d}/"));
        saving.set(true);
        spawn_local(async move {
            match api::save_profile(&path.get_value(), &p, None).await {
                Ok(_) => {
                    ui.publish_rev.update(|n| *n += 1);
                    ui.domain_for.set(None);
                }
                Err(e) => {
                    saving.set(false);
                    ui.fail(e);
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

    // Une ligne d'enregistrement : type, nom, valeur, copie.
    let record = move |kind: &'static str, name: String, value: String| {
        let v = value.clone();
        view! {
            <tr>
                <td class="dns-type">{kind}</td>
                <td class="dns-name">{name}</td>
                <td class="dns-value"><code>{value}</code></td>
                <td class="dns-copy"><CopyBtn text=Signal::derive(move || v.clone()) label="action-copy"/></td>
            </tr>
        }
    };
    let name_of = move |www: bool| {
        domain.with(|d| {
            let d = host_of(d);
            let a = apex(&d).to_string();
            if www { format!("www.{a}") } else { a }
        })
    };
    let ok_line = move |ok: bool, text: String, tip: Option<String>| {
        view! {
            <li class="domain-line" class:is-ok=ok class:is-ko=!ok title=tip.unwrap_or_default()>
                {if ok { icons::check() } else { icons::warning() }}<span>{text}</span>
            </li>
        }
    };

    view! {
        <div class="overlay" on:input=move |e| crate::dialog::mark_dirty(&e) on:mousedown=move |e| { if crate::dialog::backdrop_click(&e) { close() } }>
            <form class="dialog domain" role="dialog" aria-modal="true" aria-labelledby="domain-title"
                on:submit=move |e| { e.prevent_default(); verify.with_value(|f| f()) }>
                <div class="setup-head">
                    <h2 id="domain-title">{ui.t("domain-title")}</h2>
                    <button type="button" class="icon-btn" title=ui.t("action-close") aria-label=ui.t("action-close")
                        on:click=move |_| close()>{icons::close()}</button>
                </div>
                <label class="field">
                    <span class="label">{ui.t("domain-name")}</span>
                    <input class="input domain-input" type="text" autocomplete="off" spellcheck="false"
                        placeholder="exemple.fr" prop:value=move || domain.get()
                        on:input=move |ev| { domain.set(event_target_value(&ev)); check.set(Check::Idle) }/>
                </label>
                {move || github_owner().map(|owner| {
                    let target = format!("{owner}.github.io");
                    view! {
                        <div class="field">
                            <span class="label" title=ui.t("domain-records-help")>{ui.t("domain-records")}</span>
                            <table class="dns-table">
                                <thead><tr><th>{ui.t("domain-type")}</th><th>{ui.t("domain-host")}</th><th>{ui.t("domain-value")}</th><th></th></tr></thead>
                                <tbody>
                                    {GITHUB_A.iter().map(|ip| record("A", name_of(false), ip.to_string())).collect_view()}
                                    {record("CNAME", name_of(true), target)}
                                </tbody>
                            </table>
                            <details class="setup-advanced">
                                <summary>{ui.t("domain-ipv6")}</summary>
                                <table class="dns-table">
                                    <tbody>
                                        {GITHUB_AAAA.iter().map(|ip| record("AAAA", name_of(false), ip.to_string())).collect_view()}
                                    </tbody>
                                </table>
                            </details>
                        </div>
                    }
                })}
                {move || (github_owner().is_none() && profile.with(Option::is_some)).then(|| {
                    let keys: Vec<&'static str> = match preset().as_deref() {
                        Some("ovh") => vec!["host-ovh-domain-1", "host-ovh-domain-2"],
                        Some("o2switch") => vec!["host-o2switch-domain-1", "host-o2switch-domain-2"],
                        Some("infomaniak") => vec!["host-infomaniak-domain-1", "host-infomaniak-domain-2"],
                        Some("hostinger") => vec!["host-hostinger-domain-1", "host-hostinger-domain-2"],
                        Some("ionos") => vec!["host-ionos-domain-1", "host-ionos-domain-2"],
                        _ => vec!["domain-other"],
                    };
                    view! {
                        <div class="setup-steps">
                            {if keys.len() == 1 {
                                view! { <p class="setup-step">{ui.t(keys[0])}</p> }.into_any()
                            } else {
                                view! { <ol>{keys.into_iter().map(|k| view! { <li>{ui.t(k)}</li> }).collect_view()}</ol> }.into_any()
                            }}
                        </div>
                    }
                })}
                {move || match check.get() {
                    Check::Idle => ().into_any(),
                    Check::Running => view! { <p class="setup-state" aria-live="polite">{ui.t("domain-checking")}</p> }.into_any(),
                    Check::Failed(f) => view! { <div class="notice notice--error" role="alert"><span>{ui.failure(&f)}</span></div> }.into_any(),
                    Check::Done(s) => {
                        let addrs = s.addresses.join(", ");
                        let pointing = if s.addresses.is_empty() {
                            ok_line(false, ui.tx("domain-none", &[("domain", s.domain.clone())]), None)
                        } else if !s.expected.is_empty() && !s.addresses.iter().all(|a| s.expected.contains(a)) {
                            ok_line(false, ui.tx("domain-elsewhere", &[("domain", s.domain.clone()), ("addresses", addrs)]), None)
                        } else {
                            ok_line(true, ui.tx("domain-points", &[("domain", s.domain.clone()), ("addresses", addrs)]), None)
                        };
                        let resolved = !s.addresses.is_empty();
                        let https = resolved.then(|| if s.https {
                            ok_line(true, ui.tx("domain-https", &[]), None)
                        } else {
                            ok_line(false, ui.tx("domain-no-https", &[]), s.https_error.clone())
                        });
                        let served = s.serves_site.map(|ok| if ok {
                            ok_line(true, ui.tx("domain-serves", &[]), None)
                        } else {
                            ok_line(false, ui.tx("domain-other-page", &[]), None)
                        });
                        view! { <ul class="domain-lines" aria-live="polite">{pointing}{https}{served}</ul> }.into_any()
                    }
                }}
                <div class="dialog-actions">
                    <button type="submit" class="btn" title=ui.t("domain-check-help")
                        disabled=move || !valid() || matches!(check.get(), Check::Running)>{ui.t("domain-check")}</button>
                    <button type="button" class="btn btn--primary" disabled=move || !valid() || saving.get() || profile.with(Option::is_none)
                        on:click=move |_| save()>{ui.t("action-save")}</button>
                </div>
            </form>
        </div>
    }
    .into_any()
}
