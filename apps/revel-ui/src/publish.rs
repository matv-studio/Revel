//! Hébergements et publication.

use crate::dialog::Ask;
use crate::{Ui, api, icons, tr};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{
    BuildEvent, DeployEvent, DeploySummary, Failure, HostWarning, HostingProfile, HostingTarget,
    SiteSummary,
};

/// Type d'hébergement.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Kind {
    #[default]
    Sftp,
    Github,
}

#[derive(Clone, Default, PartialEq)]
struct Form {
    kind: Kind,
    id: String,
    name: String,
    environment: String,
    // SFTP
    host: String,
    port: String,
    user: String,
    remote_dir: String,
    password: String,
    known_host_key: Option<String>,
    has_secret: bool,
    // GitHub Pages
    repo: String,
    branch: String,
    domain: String,
    /// Versions synchronisées avant de publier (D-25).
    sync_versions: bool,
}

impl Form {
    fn new(kind: Kind, name: String, environment: String) -> Self {
        Self {
            kind,
            name,
            environment,
            port: "22".into(),
            remote_dir: "www/".into(),
            branch: "gh-pages".into(),
            sync_versions: true,
            ..Default::default()
        }
    }

    fn from_profile(p: &HostingProfile, has_secret: bool) -> Self {
        let base = Self {
            id: p.id.clone(),
            name: p.name.clone(),
            environment: p.environment.clone(),
            has_secret,
            sync_versions: p.sync_versions,
            ..Default::default()
        };
        match &p.target {
            HostingTarget::Sftp {
                host,
                port,
                user,
                remote_dir,
                known_host_key,
            } => Self {
                kind: Kind::Sftp,
                host: host.clone(),
                port: port.to_string(),
                user: user.clone(),
                remote_dir: remote_dir.clone(),
                known_host_key: known_host_key.clone(),
                ..base
            },
            HostingTarget::GithubPages {
                repo,
                branch,
                custom_domain,
            } => Self {
                kind: Kind::Github,
                repo: repo.clone(),
                branch: branch.clone(),
                domain: custom_domain.clone().unwrap_or_default(),
                ..base
            },
        }
    }

    fn to_profile(&self) -> HostingProfile {
        let target = match self.kind {
            Kind::Sftp => HostingTarget::Sftp {
                host: self.host.trim().to_string(),
                port: self.port.trim().parse().unwrap_or(22),
                user: self.user.trim().to_string(),
                remote_dir: self.remote_dir.trim().to_string(),
                known_host_key: self.known_host_key.clone(),
            },
            Kind::Github => HostingTarget::GithubPages {
                repo: normalize_repo(&self.repo),
                branch: self.branch.trim().to_string(),
                custom_domain: Some(self.domain.trim().to_string()).filter(|d| !d.is_empty()),
            },
        };
        HostingProfile {
            id: self.id.clone(),
            name: self.name.trim().to_string(),
            environment: self.environment.clone(),
            target,
            sync_versions: self.sync_versions,
        }
    }

    /// Champs indispensables remplis (le mot de passe peut déjà être dans le trousseau ;
    /// GitHub Pages demande un compte GitHub connecté).
    fn complete(&self, github: bool) -> bool {
        !self.name.trim().is_empty()
            && match self.kind {
                Kind::Sftp => {
                    !self.host.trim().is_empty()
                        && !self.user.trim().is_empty()
                        && !self.remote_dir.trim().is_empty()
                        && self.port.trim().parse::<u16>().is_ok_and(|p| p > 0)
                        && (self.has_secret || !self.password.is_empty())
                }
                Kind::Github => {
                    github
                        && normalize_repo(&self.repo)
                            .split('/')
                            .filter(|p| !p.is_empty())
                            .count()
                            == 2
                        && !self.branch.trim().is_empty()
                }
            }
    }
}

/// `https://github.com/a/b(.git)` ou `a/b` -> `a/b`.
fn normalize_repo(s: &str) -> String {
    let s = s.trim().trim_end_matches('/').trim_end_matches(".git");
    s.strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("github.com/"))
        .unwrap_or(s)
        .to_string()
}

#[derive(Clone, Default)]
struct Run {
    profile: String,
    dry_run: bool,
    status: Option<String>,
    warnings: Vec<HostWarning>,
    done: Option<Result<DeploySummary, Failure>>,
}

#[component]
pub fn Publish(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let list = RwSignal::new(None::<Vec<(HostingProfile, bool)>>);
    let editing = RwSignal::new(None::<Form>);
    let run = RwSignal::new(None::<Run>);
    let path = StoredValue::new(site.path.clone());
    let envs = StoredValue::new(site.environments.clone());
    let busy = move || run.get().is_some_and(|r| r.done.is_none());
    let empty = move || list.with(|l| l.as_ref().is_some_and(|l| l.is_empty()));
    let adding = move || editing.with(|f| f.as_ref().is_some_and(|f| f.id.is_empty()));

    let reload = Callback::new(move |()| {
        spawn_local(async move {
            match api::profiles(&path.get_value()).await {
                Ok(l) => list.set(Some(l)),
                Err(e) => ui.fail(e),
            }
        })
    });
    reload.run(());

    let add = move |_| {
        editing.set(Some(Form::new(
            Kind::Sftp,
            ui.now("publish-default-name", &[]),
            envs.with_value(|e| e.first().cloned())
                .unwrap_or_else(|| "production".into()),
        )))
    };
    let save = Callback::new(move |()| {
        let Some(f) = editing.get_untracked() else {
            return;
        };
        spawn_local(async move {
            let pw = (!f.password.is_empty()).then(|| f.password.clone());
            match api::save_profile(&path.get_value(), &f.to_profile(), pw).await {
                Ok(_) => {
                    editing.set(None);
                    reload.run(());
                }
                Err(e) => ui.fail(e),
            }
        })
    });
    let remove = Callback::new(move |(id, name): (String, String)| {
        ui.ask.set(Some(
            Ask::confirm(
                ui.now("publish-delete-title", &[("name", name)]),
                Some(ui.now("publish-delete-body", &[])),
                ui.now("action-delete", &[]),
                move || {
                    let id = id.clone();
                    spawn_local(async move {
                        if let Err(e) = api::delete_profile(&path.get_value(), &id).await {
                            ui.fail(e);
                        }
                        if run.with_untracked(|r| r.as_ref().is_some_and(|r| r.profile == id)) {
                            run.set(None);
                        }
                        reload.run(());
                    });
                },
            )
            .danger(),
        ));
    });
    let start = Callback::new(move |(id, dry_run): (String, bool)| {
        run.set(Some(Run {
            profile: id.clone(),
            dry_run,
            status: Some(ui.now("publish-building", &[])),
            ..Default::default()
        }));
        ui.activity.set(Some(0.02));
        spawn_local(async move {
            let upd = move |f: &dyn Fn(&mut Run)| {
                run.update(|r| {
                    if let Some(r) = r {
                        f(r)
                    }
                })
            };
            // Fin : fixée par le premier signal (événement du canal ou réponse de la commande) ;
            // les messages de progression arrivés ensuite sont ignorés.
            let end = move |result: Result<DeploySummary, Failure>| {
                upd(&|r| {
                    r.done = Some(result.clone());
                    r.status = None;
                });
                ui.activity.set(None);
                reload.run(());
            };
            let ended = move || run.with_untracked(|r| r.as_ref().is_none_or(|r| r.done.is_some()));
            // Versions (D-25) : changements enregistrés et synchronisés avec le dépôt du site avant
            // de publier (récupère d'abord ce qui a été fait ailleurs, Pages CMS).
            let sync = !dry_run
                && list.with_untracked(|l| {
                    l.as_ref()
                        .and_then(|l| l.iter().find(|(p, _)| p.id == id))
                        .is_some_and(|(p, _)| p.sync_versions)
                });
            if sync
                && let Ok(Some(st)) = api::versions_status(&path.get_value()).await
                && st.remote.is_some()
            {
                let t = ui.now("publish-syncing", &[]);
                upd(&|r| r.status = Some(t.clone()));
                let message = ui.now("publish-sync-message", &[]);
                let merge = ui.now("versions-merge-message", &[]);
                match api::versions_sync(&path.get_value(), &message, &merge).await {
                    Ok(info) if !info.conflicts.is_empty() => {
                        end(Err(Failure::new("error-versions-conflict")));
                        return;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        end(Err(e));
                        return;
                    }
                }
            }
            let r = api::deploy(&path.get_value(), &id, dry_run, move |e| match e {
                DeployEvent::Finished(s) => end(Ok(s)),
                DeployEvent::Failed(f) => end(Err(f)),
                _ if ended() => {}
                DeployEvent::Building {
                    build: BuildEvent::Progress { done, total, .. },
                } => {
                    if total > 1 {
                        ui.activity.set(Some(0.4 * done as f32 / total as f32));
                    }
                }
                DeployEvent::Building { .. } => {}
                DeployEvent::Warnings { warnings } => {
                    let t = ui.now("publish-connecting", &[]);
                    upd(&|r| {
                        r.warnings = warnings.clone();
                        r.status = Some(t.clone());
                    })
                }
                DeployEvent::Connected { .. } => {
                    ui.activity.set(Some(0.45));
                    let t = ui.now("publish-comparing", &[]);
                    upd(&|r| r.status = Some(t.clone()))
                }
                DeployEvent::Fetching { done, total } => {
                    ui.activity
                        .set(Some(0.4 + 0.05 * done as f32 / total.max(1) as f32));
                    let t = ui.now("publish-fetching", &[]);
                    upd(&|r| r.status = Some(t.clone()))
                }
                DeployEvent::Preparing { done, total } => {
                    ui.activity
                        .set(Some(0.45 + 0.05 * done as f32 / total.max(1) as f32));
                    let t = ui.now(
                        "publish-preparing",
                        &[
                            ("done", crate::num(done as f64, 0)),
                            ("total", crate::num(total as f64, 0)),
                        ],
                    );
                    upd(&|r| r.status = Some(t.clone()))
                }
                DeployEvent::Planned {
                    upload,
                    delete,
                    bytes,
                    total,
                } => {
                    let n = |v: usize| v.to_string();
                    let t = ui.now(
                        "publish-planned",
                        &[
                            ("upload", n(upload)),
                            ("delete", n(delete)),
                            ("total", n(total)),
                            ("size", crate::num(bytes as f64 / 1_048_576.0, 1)),
                        ],
                    );
                    upd(&|r| r.status = Some(t.clone()));
                }
                DeployEvent::Uploaded { done, total } => {
                    ui.activity
                        .set(Some(0.5 + 0.5 * done as f32 / total.max(1) as f32));
                    let t = ui.now(
                        "publish-progress",
                        &[
                            ("done", crate::num(done as f64, 0)),
                            ("total", crate::num(total as f64, 0)),
                        ],
                    );
                    upd(&|r| r.status = Some(t.clone()));
                }
                DeployEvent::Deleted { .. } => {}
            })
            .await;
            if let Err(e) = r
                && !ended()
            {
                end(Err(e));
            }
        })
    });

    view! {
        <section class="page settings-page">
            <header class="page-head">
                <h1>{ui.t("settings-publish")}</h1>
                <Show when=move || !empty()>
                    <div class="toolbar">
                        <button class="btn" on:click=add disabled=move || editing.get().is_some()>{icons::plus()}{ui.t("publish-add")}</button>
                    </div>
                </Show>
            </header>
            <Show when=move || empty() && editing.get().is_none()>
                <div class="empty">
                    <p>{ui.t("publish-empty")}</p>
                    <div class="toolbar">
                        <button class="btn btn--primary" on:click=add>{icons::plus()}{ui.t("publish-add")}</button>
                    </div>
                </div>
            </Show>
            <Show when=adding>
                <div class="section">
                    <HostForm editing=editing envs=envs.get_value() save=save/>
                </div>
            </Show>
            <div class="stack">
                <For each=move || list.get().unwrap_or_default() key=|(p, s)| format!("{}|{}|{:?}|{}|{}", p.id, s, p.target, p.name, p.environment) let:item>
                    {
                        let (p, has_secret) = item;
                        let pe = p.clone();
                        let id = StoredValue::new(p.id.clone());
                        let name = StoredValue::new(p.name.clone());
                        let facts = StoredValue::new(target_facts(&p.target));
                        let is_editing = move || editing.with(|f| f.as_ref().is_some_and(|f| f.id == id.get_value()));
                        view! {
                            <article class="section host">
                                <Show when=is_editing fallback=move || {
                                    let pe = pe.clone();
                                    let pe_fix = pe.clone();
                                    let on_edit = Callback::new(move |()| editing.set(Some(Form::from_profile(&pe_fix, has_secret))));
                                    view! {
                                        <div class="host-head">
                                            <div class="host-id">
                                                <h2>{name.get_value()}</h2>
                                                <span class="chip" title=ui.t("publish-env")>{p.environment.clone()}</span>
                                            </div>
                                            <div class="toolbar">
                                                <button class="btn" title=ui.t("publish-check-help")
                                                    on:click=move |_| start.run((id.get_value(), true)) disabled=busy>{ui.t("publish-check")}</button>
                                                <button class="btn btn--primary" on:click=move |_| start.run((id.get_value(), false)) disabled=busy>{icons::upload()}{ui.t("publish-run")}</button>
                                                <button class="icon-btn" title=ui.t("action-edit") aria-label=ui.t("action-edit") disabled=busy
                                                    on:click=move |_| editing.set(Some(Form::from_profile(&pe, has_secret)))>{icons::edit()}</button>
                                                <button class="icon-btn" title=ui.t("action-delete") aria-label=ui.t("action-delete") disabled=busy
                                                    on:click=move |_| remove.run((id.get_value(), name.get_value()))>{icons::trash()}</button>
                                            </div>
                                        </div>
                                        <dl class="facts facts--inline">
                                            {facts.get_value().into_iter().map(|(k, v)| view! {
                                                <div><dt>{ui.t(k)}</dt><dd class="path">{v}</dd></div>
                                            }).collect_view()}
                                        </dl>
                                        {move || run.get().filter(|r| r.profile == id.get_value()).map(|r| view! { <RunView run=r on_edit=on_edit/> })}
                                    }
                                }>
                                    <HostForm editing=editing envs=envs.get_value() save=save/>
                                </Show>
                            </article>
                        }
                    }
                </For>
            </div>
        </section>
    }
}

/// Informations affichées sur la fiche d'un hébergement : (clé du libellé, valeur).
fn target_facts(t: &HostingTarget) -> Vec<(&'static str, String)> {
    match t {
        HostingTarget::Sftp {
            host,
            port,
            user,
            remote_dir,
            ..
        } => vec![
            ("publish-host", format!("{user}@{host}:{port}")),
            ("publish-remote-dir", remote_dir.clone()),
        ],
        HostingTarget::GithubPages {
            repo,
            branch,
            custom_domain,
        } => {
            let mut v = vec![
                ("publish-repo", repo.clone()),
                ("publish-branch", branch.clone()),
            ];
            if let Some(d) = custom_domain {
                v.push(("publish-domain", d.clone()));
            }
            v
        }
    }
}

/// Texte d'un avertissement d'hébergement ; tailles (octets) converties en Mo pour les
/// clés `…-too-big`.
fn warning_text(lang: &str, w: &HostWarning) -> String {
    let bytes = w.key.ends_with("-too-big");
    let args: Vec<(&str, String)> = w
        .args
        .iter()
        .map(|(k, v)| {
            let v = match v.parse::<u64>() {
                Ok(n) if bytes => crate::num(n as f64 / 1_048_576.0, 1),
                _ => v.clone(),
            };
            (k.as_str(), v)
        })
        .collect();
    tr(lang, &w.key, &args)
}

/// Échecs dus aux réglages de l'hébergement (adresse, identifiants, dossier).
const FIXABLE: &[&str] = &[
    "error-sftp-auth",
    "error-sftp-connect",
    "error-sftp-remote-dir",
    "error-missing-secret",
    "error-github-signed-out",
    "error-github-repo",
    "error-github-no-push",
];

/// Résultat d'une vérification ou d'une publication.
#[component]
fn RunView(run: Run, on_edit: Callback<()>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let n = run.warnings.len();
    view! {
        <div class="host-run" aria-live="polite">
            <Show when=move || { n > 0 }>
                <h3>{ui.t("publish-warnings")}</h3>
                <ul class="list-plain">
                    {run.warnings.iter().map(|w| {
                        let txt = warning_text(&ui.lang(), w);
                        view! { <li class:is-blocking=w.blocking>{txt}</li> }
                    }).collect_view()}
                </ul>
            </Show>
            {run.status.clone().map(|s| view! { <p class="muted">{s}</p> })}
            {run.done.clone().map(|d| match d {
                Ok(s) => {
                    let size = crate::num(s.bytes as f64 / 1_048_576.0, 1);
                    let msg = if s.dry_run {
                        ui.tx("publish-dry-done", &[
                            ("upload", crate::num(s.uploaded as f64, 0)),
                            ("delete", crate::num(s.deleted as f64, 0)),
                            ("size", size),
                        ])
                    } else {
                        ui.tx("publish-done", &[
                            ("uploaded", crate::num(s.uploaded as f64, 0)),
                            ("deleted", crate::num(s.deleted as f64, 0)),
                        ])
                    };
                    let fp = s.server_fingerprint.clone();
                    // Lien vers le site après une vraie publication.
                    let url = s.url.clone().filter(|_| !s.dry_run);
                    let pages_enabled = s.pages_enabled;
                    view! {
                        <div class="section-head">
                            <p class="status status--ok">{icons::check()}<span>{msg}</span></p>
                            {url.map(|u| view! {
                                <button class="btn" on:click=move |_| {
                                    let u = u.clone();
                                    spawn_local(async move {
                                        if let Err(e) = api::open_external(&u).await {
                                            ui.fail(e);
                                        }
                                    });
                                }>{icons::external()}{ui.t("publish-open-site")}</button>
                            })}
                        </div>
                        <Show when=move || pages_enabled>
                            <p class="muted help">{ui.t("publish-pages-enabled")}</p>
                        </Show>
                        <Show when=move || s.key_recorded>
                            <p class="muted help">{ui.t("publish-key-recorded")}</p>
                            <code class="code">{fp.clone()}</code>
                        </Show>
                    }.into_any()
                }
                Err(f) => {
                    let key = if run.dry_run { "publish-check-failed" } else { "publish-failed" };
                    // Échecs corrigés en modifiant l'hébergement : accès direct au formulaire.
                    let fixable = FIXABLE.contains(&f.key.as_str());
                    view! {
                        <div class="notice notice--error" role="alert">
                            <span>{ui.tx(key, &[("message", ui.failure(&f))])}</span>
                            <Show when=move || fixable>
                                <button class="btn" on:click=move |_| on_edit.run(())>{ui.t("publish-fix")}</button>
                            </Show>
                        </div>
                    }.into_any()
                }
            })}
        </div>
    }
}

/// Saisie d'un hébergement SFTP (nouveau ou existant).
#[component]
fn HostForm(
    editing: RwSignal<Option<Form>>,
    envs: Vec<String>,
    save: Callback<()>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let is_new = editing.with_untracked(|f| f.as_ref().is_some_and(|f| f.id.is_empty()));
    let has_secret = editing.with_untracked(|f| f.as_ref().is_some_and(|f| f.has_secret));
    let kind = move || editing.with(|f| f.as_ref().map(|f| f.kind).unwrap_or_default());
    let complete = move || {
        let github = ui.github.with(|g| g.is_some());
        editing.with(|f| f.as_ref().is_some_and(|f| f.complete(github)))
    };
    let field = move |key: &'static str,
                      kind: &'static str,
                      get: fn(&Form) -> String,
                      set: fn(&mut Form, String),
                      help: Option<&'static str>| {
        view! {
            <label class="field">
                <span class="label">{ui.t(key)}</span>
                <input class="input" type=kind autocomplete="off" spellcheck="false"
                    prop:value=move || editing.with(|f| f.as_ref().map(get).unwrap_or_default())
                    on:input=move |ev| { let v = event_target_value(&ev); editing.update(|f| if let Some(f) = f { set(f, v) }) }/>
                {help.map(|h| view! { <span class="help">{ui.t(h)}</span> })}
            </label>
        }
    };
    let kind_btn = move |k: Kind, label: &'static str| {
        view! {
            <button type="button" role="radio" class:is-on=move || kind() == k
                aria-checked=move || (kind() == k).to_string()
                on:click=move |_| {
                    // Nom proposé par défaut : suit le type tant qu'il n'a pas été modifié.
                    let default_name = |k: Kind| ui.now(match k {
                        Kind::Sftp => "publish-default-name",
                        Kind::Github => "publish-kind-github",
                    }, &[]);
                    editing.update(|f| if let Some(f) = f {
                        if f.name == default_name(f.kind) {
                            f.name = default_name(k);
                        }
                        f.kind = k;
                    })
                }>{ui.t(label)}</button>
        }
    };
    let title = move || match (is_new, kind()) {
        (false, _) => ui.tx("publish-form-edit", &[]),
        (true, Kind::Sftp) => ui.tx("publish-form-new", &[]),
        (true, Kind::Github) => ui.tx("publish-form-new-github", &[]),
    };
    view! {
        <form class="stack" on:submit=move |e| { e.prevent_default(); if complete() { save.run(()) } }>
            <div class="section-head">
                <h2 title=move || match kind() {
                    Kind::Sftp => ui.tx("publish-form-help", &[]),
                    Kind::Github => ui.tx("publish-form-help-github", &[]),
                }>{title}</h2>
                <Show when=move || is_new>
                    <div class="segmented" role="radiogroup" aria-label=ui.t("publish-kind")>
                        {kind_btn(Kind::Sftp, "publish-kind-sftp")}
                        {kind_btn(Kind::Github, "publish-kind-github")}
                    </div>
                </Show>
            </div>
            <Show when=move || kind() == Kind::Github>
                <div class="subsection">
                    <span class="label">{ui.t("github-account")}</span>
                    <crate::account::GithubAccount/>
                </div>
            </Show>
            <div class="form-cols">
                {field("publish-name", "text", |f| f.name.clone(), |f, v| f.name = v, None)}
                <label class="field">
                    <span class="label">{ui.t("publish-env")}</span>
                    <span class="select">
                        <select class="input"
                            prop:value=move || editing.with(|f| f.as_ref().map(|f| f.environment.clone()).unwrap_or_default())
                            on:change=move |ev| { let v = event_target_value(&ev); editing.update(|f| if let Some(f) = f { f.environment = v }) }>
                            {envs.into_iter().map(|e| { let v = e.clone(); view! { <option value=v>{e}</option> } }).collect_view()}
                        </select>
                    </span>
                </label>
            </div>
            <Show when=move || kind() == Kind::Sftp>
                <div class="form-cols">
                    {field("publish-host", "text", |f| f.host.clone(), |f, v| f.host = v, Some("publish-host-help"))}
                    {field("publish-port", "number", |f| f.port.clone(), |f, v| f.port = v, None)}
                    {field("publish-user", "text", |f| f.user.clone(), |f, v| f.user = v, None)}
                    {field("publish-password", "password", |f| f.password.clone(), |f, v| f.password = v,
                        Some(if has_secret { "publish-password-saved" } else { "publish-password-help" }))}
                    {field("publish-remote-dir", "text", |f| f.remote_dir.clone(), |f, v| f.remote_dir = v, Some("publish-remote-dir-help"))}
                </div>
            </Show>
            <Show when=move || kind() == Kind::Github>
                <div class="form-cols">
                    {field("publish-repo", "text", |f| f.repo.clone(), |f, v| f.repo = v, Some("publish-repo-help"))}
                    {field("publish-branch", "text", |f| f.branch.clone(), |f, v| f.branch = v, Some("publish-branch-help"))}
                    {field("publish-domain", "text", |f| f.domain.clone(), |f, v| f.domain = v, Some("publish-domain-help"))}
                </div>
            </Show>
            <label class="view-line publish-sync" title=ui.t("publish-sync-versions-help")>
                <span>{ui.t("publish-sync-versions")}</span>
                <span class="switch">
                    <input type="checkbox" prop:checked=move || editing.with(|f| f.as_ref().is_some_and(|f| f.sync_versions))
                        on:change=move |e| { let on = event_target_checked(&e); editing.update(|f| if let Some(f) = f { f.sync_versions = on }) }/>
                    <span class="switch-track"></span>
                </span>
            </label>
            <div class="toolbar toolbar--start">
                <button type="submit" class="btn btn--primary" disabled=move || !complete()>{ui.t("action-save")}</button>
                <button type="button" class="btn" on:click=move |_| editing.set(None)>{ui.t("action-cancel")}</button>
            </div>
        </form>
    }
}
