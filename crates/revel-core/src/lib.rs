//! `revel-core` : services de l'application, indépendants de l'interface.
//!
//! L'application de bureau, la ligne de commande et le futur serveur MCP (D-13) appellent
//! les mêmes fonctions : ouvrir, importer, générer, prévisualiser, publier. Les événements
//! de progression sont des types de `revel-protocol`.

use revel_protocol::{
    AppSettings, BuildEvent, BuildPhase, BuildSummary, CollectionSummary, DeployEvent,
    DeploySummary, Failure, HostWarning, HostingProfile, HostingTarget, ImportResult, RecentSite,
    SiteSummary,
};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

pub mod canvas;
mod edit;
mod github;
pub mod preview;
pub mod secrets;
mod state;

pub use secrets::SecretStore;
pub use state::State;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{0}")]
    Project(#[from] revel_project::ProjectError),
    #[error("{0}")]
    Import(#[from] revel_import::eleventy::ImportError),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Sftp(#[from] revel_deploy::sftp::SftpError),
    #[error("ce dossier contient déjà un site Revel (site.revel.json)")]
    AlreadyImported,
    #[error("ce dossier n'est pas un projet Eleventy")]
    NotEleventy,
    #[error("hébergement introuvable : {0}")]
    UnknownProfile(String),
    #[error("mot de passe absent pour {0} : à saisir dans les réglages de l'hébergement")]
    MissingSecret(String),
    #[error("trousseau du système : {0}")]
    Secret(String),
    #[error("publication bloquée : {0}")]
    Blocked(String),
    #[error("site non généré : générer le site avant l'aperçu")]
    NotBuilt,
    #[error("{0}")]
    Git(#[from] revel_git::GitError),
    #[error("aucun compte GitHub connecté")]
    GithubSignedOut,
    #[error("dépôt GitHub introuvable ou inaccessible : {0}")]
    GithubRepoNotFound(String),
    #[error("pas de droit d'écriture sur le dépôt {0}")]
    GithubNoPush(String),
    #[error("connexion GitHub annulée")]
    GithubCancelled,
    #[error("{0}")]
    Edit(revel_edit::EditError),
    #[error("site non ouvert dans l'éditeur")]
    NotOpen,
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;

impl CoreError {
    /// Échec à afficher dans la langue de l'utilisateur (clés `error-…` de `revel-i18n`).
    pub fn failure(&self) -> Failure {
        use revel_deploy::sftp::SftpError as S;
        match self {
            Self::Project(e) => Failure::new("error-project").arg("detail", e.to_string()),
            Self::Import(e) => Failure::new("error-import").arg("detail", e.to_string()),
            Self::Io(e) => Failure::new("error-io").arg("detail", e.to_string()),
            Self::Sftp(e) => match e {
                S::Connect(host, detail) => Failure::new("error-sftp-connect")
                    .arg("host", host.clone())
                    .arg("detail", detail.clone()),
                S::HostKeyMismatch(received) => {
                    Failure::new("error-sftp-host-key").arg("received", received.clone())
                }
                S::Auth => Failure::new("error-sftp-auth"),
                S::Sftp(detail) => Failure::new("error-sftp").arg("detail", detail.clone()),
                S::Io(e) => Failure::new("error-io").arg("detail", e.to_string()),
                S::RemoteDir(dir) => Failure::new("error-sftp-remote-dir").arg("dir", dir.clone()),
                S::FileName(name) => Failure::new("error-sftp-file-name").arg("name", name.clone()),
            },
            Self::AlreadyImported => Failure::new("error-already-imported"),
            Self::NotEleventy => Failure::new("error-not-eleventy"),
            Self::UnknownProfile(id) => Failure::new("error-unknown-profile").arg("id", id.clone()),
            Self::MissingSecret(_) => Failure::new("error-missing-secret"),
            Self::Secret(detail) => Failure::new("error-keyring").arg("detail", detail.clone()),
            Self::Blocked(_) => Failure::new("error-blocked"),
            Self::NotBuilt => Failure::new("error-not-built"),
            Self::Git(e) => git_failure(e),
            Self::GithubSignedOut => Failure::new("error-github-signed-out"),
            Self::GithubRepoNotFound(repo) => {
                Failure::new("error-github-repo").arg("repo", repo.clone())
            }
            Self::GithubNoPush(repo) => {
                Failure::new("error-github-no-push").arg("repo", repo.clone())
            }
            Self::GithubCancelled => Failure::new("error-github-cancelled"),
            Self::Edit(e) => Failure::new(e.key()).arg("detail", e.to_string()),
            Self::NotOpen => Failure::new("error-edit-not-open"),
            Self::Other(detail) => Failure::detail(detail.clone()),
        }
    }
}

fn git_failure(e: &revel_git::GitError) -> Failure {
    use revel_git::GitError as G;
    if e.is_auth() {
        return Failure::new("error-github-signed-out");
    }
    match e {
        G::CodeExpired => Failure::new("error-github-code-expired"),
        G::Denied => Failure::new("error-github-denied"),
        G::AppConfig(d) => Failure::new("error-github-app").arg("detail", d.clone()),
        G::Rejected(d) => Failure::new("error-git-rejected").arg("detail", d.clone()),
        G::Http(d) => Failure::new("error-network").arg("detail", d.clone()),
        G::Api(status, d) => Failure::new("error-github-api")
            .arg("status", status.to_string())
            .arg("detail", d.chars().take(300).collect::<String>()),
        G::Git(g) => Failure::new("error-git").arg("detail", g.message().to_string()),
        G::Io(io) => Failure::new("error-io").arg("detail", io.to_string()),
        other => Failure::detail(other.to_string()),
    }
}

/// Point d'entrée des services.
pub struct Core {
    state: Mutex<State>,
    secrets: Box<dyn SecretStore>,
    /// Demande d'interruption de la connexion GitHub en cours.
    github_cancel: AtomicBool,
    /// Sites ouverts dans l'éditeur (clé : chemin du site).
    sessions: Mutex<std::collections::HashMap<String, edit::Session>>,
    canvas: Mutex<Option<canvas::Canvas>>,
}

/// Dossier de sortie d'un environnement : `<site>/.revel/out/<env>`.
pub fn out_dir(site: &Path, environment: &str) -> PathBuf {
    site.join(revel_project::CACHE_DIR)
        .join("out")
        .join(sanitize(environment))
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Identifiant du secret propre à un hébergement dans le trousseau (mot de passe SFTP).
/// GitHub Pages utilise le jeton du compte GitHub connecté (`github:token`).
pub fn secret_key(target: &HostingTarget) -> Option<String> {
    match target {
        HostingTarget::Sftp {
            host, port, user, ..
        } => Some(format!("sftp:{user}@{host}:{port}")),
        HostingTarget::GithubPages { .. } => None,
    }
}

impl Core {
    /// `config_dir` : dossier de réglages de l'application (ex. `%APPDATA%/Revel`).
    pub fn new(config_dir: &Path, secrets: Box<dyn SecretStore>) -> Self {
        Self {
            state: Mutex::new(State::load(config_dir)),
            secrets,
            github_cancel: AtomicBool::new(false),
            sessions: Mutex::new(std::collections::HashMap::new()),
            canvas: Mutex::new(None),
        }
    }

    fn with_state<T>(&self, f: impl FnOnce(&mut State) -> T) -> T {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let r = f(&mut s);
        s.save();
        r
    }

    pub fn settings(&self) -> AppSettings {
        self.with_state(|s| s.settings.clone())
    }

    pub fn set_settings(&self, settings: AppSettings) {
        self.with_state(|s| s.settings = settings);
    }

    pub fn recent_sites(&self) -> Vec<RecentSite> {
        self.with_state(|s| {
            s.recents
                .retain(|r| Path::new(&r.path).join(revel_project::SITE_FILE).is_file());
            s.recents.clone()
        })
    }

    pub fn forget_site(&self, path: &str) {
        self.with_state(|s| s.recents.retain(|r| r.path != path));
    }

    fn touch_recent(&self, summary: &SiteSummary) {
        self.with_state(|s| {
            s.recents.retain(|r| r.path != summary.path);
            s.recents.insert(
                0,
                RecentSite {
                    path: summary.path.clone(),
                    name: summary.name.clone(),
                    opened_at: now(),
                },
            );
            s.recents.truncate(20);
        });
    }

    /// Ouvre un site existant.
    pub fn open_site(&self, path: &Path) -> Result<SiteSummary> {
        let p = revel_project::Project::open(path)?;
        let summary = summarize(&p)?;
        self.touch_recent(&summary);
        Ok(summary)
    }

    /// Crée un site vide.
    pub fn new_site(&self, path: &Path, name: &str, lang: &str) -> Result<SiteSummary> {
        if path.join(revel_project::SITE_FILE).exists() {
            return Err(CoreError::AlreadyImported);
        }
        let p = revel_project::Project::create(path, revel_model::Site::new(name, lang))?;
        let summary = summarize(&p)?;
        self.touch_recent(&summary);
        Ok(summary)
    }

    /// Importe un projet Eleventy (écrit `site.revel.json` à sa racine).
    pub fn import_eleventy(&self, path: &Path, overwrite: bool) -> Result<ImportResult> {
        if !revel_import::is_eleventy(path) {
            return Err(CoreError::NotEleventy);
        }
        if path.join(revel_project::SITE_FILE).exists() && !overwrite {
            return Err(CoreError::AlreadyImported);
        }
        let profile = revel_import::eleventy::load_profile(path)?;
        let (site, report) = revel_import::eleventy::import(path, &profile)?;
        let p = revel_project::Project::create(path, site)?;
        let summary = summarize(&p)?;
        self.touch_recent(&summary);
        Ok(ImportResult {
            site: summary,
            warnings: report.warnings,
        })
    }

    /// Génère le site pour un environnement dans `<site>/.revel/out/<env>`.
    pub fn build(
        &self,
        path: &Path,
        environment: &str,
        images: bool,
        on_event: &(dyn Fn(BuildEvent) + Sync),
    ) -> Result<BuildSummary> {
        let started = std::time::Instant::now();
        let p = revel_project::Project::open(path)?;
        let out = out_dir(path, environment);
        let progress = |phase: revel_project::Phase, done: usize, total: usize| {
            on_event(BuildEvent::Progress {
                phase: match phase {
                    revel_project::Phase::Pages => BuildPhase::Pages,
                    revel_project::Phase::Copies => BuildPhase::Copies,
                    revel_project::Phase::Images => BuildPhase::Images,
                },
                done,
                total,
            });
        };
        let r = p.build(
            &out,
            &revel_project::BuildOptions {
                environment: environment.to_string(),
                images,
                progress: Some(&progress),
            },
        );
        match r {
            Ok(report) => {
                let summary = BuildSummary {
                    environment: environment.to_string(),
                    out_dir: out.display().to_string(),
                    generated: report.generated,
                    copied: report.copied,
                    images_produced: report.images.produced,
                    images_cached: report.images.from_cache,
                    images_failed: report.images.failed,
                    seconds: started.elapsed().as_secs_f32(),
                    warnings: report.warnings,
                };
                on_event(BuildEvent::Finished(summary.clone()));
                Ok(summary)
            }
            Err(e) => {
                let e = CoreError::from(e);
                on_event(BuildEvent::Failed(e.failure()));
                Err(e)
            }
        }
    }

    pub fn profiles(&self, site: &str) -> Vec<HostingProfile> {
        self.with_state(|s| s.profiles.get(site).cloned().unwrap_or_default())
    }

    /// Ajoute ou remplace un hébergement ; `password` : `Some` pour l'enregistrer dans le trousseau.
    pub fn save_profile(
        &self,
        site: &str,
        mut profile: HostingProfile,
        password: Option<String>,
    ) -> Result<HostingProfile> {
        if profile.id.is_empty() {
            profile.id = format!("h{}", chrono::Utc::now().timestamp_millis());
        }
        if let (Some(pw), Some(key)) = (
            password.filter(|p| !p.is_empty()),
            secret_key(&profile.target),
        ) {
            self.secrets.set(&key, &pw).map_err(CoreError::Secret)?;
        }
        self.with_state(|s| {
            let list = s.profiles.entry(site.to_string()).or_default();
            list.retain(|p| p.id != profile.id);
            list.push(profile.clone());
        });
        Ok(profile)
    }

    pub fn delete_profile(&self, site: &str, id: &str) -> Result<()> {
        let removed = self.with_state(|s| {
            let list = s.profiles.entry(site.to_string()).or_default();
            let found = list.iter().find(|p| p.id == id).cloned();
            list.retain(|p| p.id != id);
            found
        });
        if let Some(key) = removed.and_then(|p| secret_key(&p.target)) {
            let _ = self.secrets.delete(&key);
        }
        Ok(())
    }

    /// L'hébergement dispose de ses accès : mot de passe SFTP enregistré, ou compte GitHub connecté.
    pub fn has_secret(&self, target: &HostingTarget) -> bool {
        let key = secret_key(target).unwrap_or_else(|| github::TOKEN_KEY.to_string());
        self.secrets.get(&key).ok().flatten().is_some()
    }

    /// Avertissements d'un hébergement pour le site généré.
    pub fn host_warnings(&self, out: &Path, target: &HostingTarget) -> Result<Vec<HostWarning>> {
        let needs = revel_deploy::needs_from_dir(out)?;
        let caps = revel_deploy::providers::all()
            .into_iter()
            .find(|c| c.id == target.provider_id())
            .ok_or_else(|| {
                CoreError::Other(format!("hébergeur inconnu : {}", target.provider_id()))
            })?;
        Ok(revel_deploy::check(&caps, &needs)
            .into_iter()
            .map(|w| HostWarning {
                blocking: w.severity == revel_deploy::Severity::Blocking,
                key: w.key.to_string(),
                args: w
                    .args
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
            })
            .collect())
    }

    /// Génère puis publie le site sur un hébergement.
    pub async fn deploy(
        &self,
        site: &Path,
        profile_id: &str,
        dry_run: bool,
        full_sync: bool,
        on_event: &(dyn Fn(DeployEvent) + Send + Sync),
    ) -> Result<DeploySummary> {
        let site_key = site.display().to_string();
        let profile = self
            .profiles(&site_key)
            .into_iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| CoreError::UnknownProfile(profile_id.to_string()))?;
        let out = out_dir(site, &profile.environment);
        // Génération (calcul intensif) sans bloquer les autres tâches du moteur asynchrone.
        tokio::task::block_in_place(|| {
            self.build(site, &profile.environment, true, &|e| {
                on_event(DeployEvent::Building { build: e })
            })
        })?;
        let warnings = self.host_warnings(&out, &profile.target)?;
        if let Some(b) = warnings.iter().find(|w| w.blocking) {
            on_event(DeployEvent::Warnings {
                warnings: warnings.clone(),
            });
            return Err(CoreError::Blocked(b.key.clone()));
        }
        match &profile.target {
            HostingTarget::GithubPages {
                repo,
                branch,
                custom_domain,
            } => {
                let r = self
                    .deploy_github_pages(
                        site,
                        &out,
                        &profile.environment,
                        repo,
                        branch,
                        custom_domain.as_deref(),
                        warnings,
                        dry_run,
                        on_event,
                    )
                    .await;
                if let Err(e) = &r {
                    on_event(DeployEvent::Failed(e.failure()));
                }
                r
            }
            HostingTarget::Sftp {
                host,
                port,
                user,
                remote_dir,
                known_host_key,
            } => {
                on_event(DeployEvent::Warnings { warnings });
                let key = secret_key(&profile.target).unwrap_or_default();
                let password = self
                    .secrets
                    .get(&key)
                    .map_err(CoreError::Secret)?
                    .ok_or_else(|| CoreError::MissingSecret(key.clone()))?;
                let target = revel_deploy::sftp::SftpTarget {
                    host: host.clone(),
                    port: *port,
                    user: user.clone(),
                    password,
                    remote_dir: remote_dir.clone(),
                    known_host_key: known_host_key.clone(),
                };
                let opts = revel_deploy::sftp::Options {
                    full_sync,
                    dry_run,
                    parallel: 4,
                };
                let report = revel_deploy::sftp::deploy(&out, &target, &opts, &|e| {
                    use revel_deploy::sftp::Event;
                    on_event(match e {
                        Event::Connected { server_key } => DeployEvent::Connected { server_key },
                        Event::Planned {
                            upload,
                            delete,
                            bytes,
                            total,
                        } => DeployEvent::Planned {
                            upload,
                            delete,
                            bytes,
                            total,
                        },
                        Event::Uploaded { done, total, .. } => {
                            DeployEvent::Uploaded { done, total }
                        }
                        Event::Deleted { done, total } => DeployEvent::Deleted { done, total },
                        Event::Finished => return,
                    })
                })
                .await;
                let report = match report {
                    Ok(r) => r,
                    Err(e) => {
                        let e = CoreError::from(e);
                        on_event(DeployEvent::Failed(e.failure()));
                        return Err(e);
                    }
                };
                // Première connexion : la clé reçue est enregistrée pour les suivantes.
                let key_recorded = known_host_key.is_none() && !report.server_key.is_empty();
                if key_recorded {
                    let mut p = profile.clone();
                    if let HostingTarget::Sftp { known_host_key, .. } = &mut p.target {
                        *known_host_key = Some(report.server_key.clone());
                    }
                    self.save_profile(&site_key, p, None)?;
                }
                let s = DeploySummary {
                    uploaded: report.plan.upload.len(),
                    deleted: report.plan.delete.len(),
                    total_files: report.total_files,
                    bytes: report.upload_bytes,
                    dry_run,
                    server_key: report.server_key,
                    server_fingerprint: report.server_fingerprint,
                    key_recorded,
                    url: None,
                    pages_enabled: false,
                };
                on_event(DeployEvent::Finished(s.clone()));
                Ok(s)
            }
        }
    }
}

fn summarize(p: &revel_project::Project) -> Result<SiteSummary> {
    let entries = p.load_entries()?;
    Ok(SiteSummary {
        path: p.dir.display().to_string(),
        name: p.site.name.clone(),
        base_url: p.site.base_url.clone(),
        pages: p.site.pages.len(),
        layouts: p.site.layouts.len(),
        components: p.site.components.len(),
        collections: p
            .site
            .collections
            .iter()
            .map(|c| CollectionSummary {
                id: c.id.clone(),
                folder: c.folder.clone(),
                entries: entries
                    .iter()
                    .filter(|e| e.folder == c.folder && !c.folder.is_empty())
                    .count(),
            })
            .collect(),
        environments: {
            let mut e: Vec<String> = p.site.settings.environments.keys().cloned().collect();
            if !e.contains(&"production".to_string()) {
                e.insert(0, "production".into());
            }
            e
        },
    })
}

#[cfg(test)]
mod tests;
