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

mod build;
pub mod canvas;
mod checks;
pub mod devserver;
pub mod domain;
mod edit;
mod fonts;
mod ghsites;
mod github;
mod import;
pub mod links;
mod localize;
pub mod pagescms;
pub mod preview;
mod publish;
mod readable;
mod rollback;
pub mod secrets;
pub mod starter;
mod state;
pub mod templates;
mod translate;
pub mod updates;
mod versions;

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
    #[error("ce dossier ne contient pas de site Revel")]
    NotASite,
    #[error("mise à la corbeille impossible : {0}")]
    Trash(String),
    #[error("pas de traduction automatique de « {0} » vers « {1} »")]
    NoTranslation(String, String),
    #[error("traduction automatique : {0}")]
    Translate(String),
    #[error("fichier de traduction téléchargé altéré : {0}")]
    TranslateIntegrity(String),
    #[error("téléchargement annulé")]
    TranslateCancelled,
    #[error("un téléchargement de langue est déjà en cours")]
    TranslateBusy,
    /// Police refusée ou introuvable.
    #[error("police : {0}")]
    Font(String),
    /// Import ou remplacement d'un média refusé.
    #[error("{0}")]
    Media(String),
    /// Points à vérifier avant de publier (5.4).
    #[error("{0} point(s) à vérifier avant de publier")]
    Checks(usize),
    /// Pas de publication précédente gardée (5.5).
    #[error("aucune publication précédente")]
    NoPrevious,
    /// Site à construire par son outil (D-80).
    #[error("site à construire avec {0} : importer le dossier produit par sa construction")]
    NeedsBuild(String),
    #[error("aucune page HTML ni projet Eleventy dans ce dossier")]
    NothingToImport,
    #[error("adresse de dépôt non reconnue")]
    BadRepoUrl,
    #[error("copie du dépôt annulée")]
    CloneCancelled,
    #[error("ce dossier n'est pas une copie de dépôt en cours d'import")]
    NotAClone,
    #[error("adresse de site non reconnue")]
    BadSiteUrl,
    #[error("site injoignable : {0}")]
    SiteUnreachable(String),
    #[error("l'adresse ne mène pas à une page HTML")]
    NotHtml,
    #[error("import annulé")]
    ImportCancelled,
    #[error("{0}")]
    NotWxr(String),
    #[error("adresse de fichier Figma non reconnue")]
    FigmaUrl,
    #[error("jeton Figma absent ou refusé")]
    FigmaToken,
    #[error("fichier Figma introuvable ou inaccessible avec ce jeton")]
    FigmaNotFound,
    #[error("Figma : {0}")]
    Figma(String),
    #[error("jeton de l'API de Webflow refusé")]
    WebflowToken,
    /// Construction d'un site par Revel (D-88).
    #[error("{0}")]
    Build(#[from] revel_build::BuildError),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;

/// Échec d'une construction : clé de traduction, étape, fin du journal (D-88).
fn build_failure(e: &revel_build::BuildError) -> Failure {
    use revel_build::{BuildError as B, Missing};
    let with_log = |f: Failure, log: &str| f.arg("log", log.to_string());
    match e {
        B::Download(d) => Failure::new("error-build-download").arg("detail", d.clone()),
        B::Integrity(d) => Failure::new("error-build-integrity").arg("detail", d.clone()),
        B::Archive(d) => Failure::new("error-build-archive").arg("detail", d.clone()),
        B::Platform => Failure::new("error-build-platform"),
        B::Missing(Missing::Ruby) => Failure::new("error-build-needs-ruby"),
        B::Missing(Missing::Go) => Failure::new("error-build-needs-go"),
        B::Missing(Missing::BuildScript) => Failure::new("error-build-no-script"),
        B::Failed { step, code, log } => with_log(
            Failure::new("error-build-step")
                .arg("step", step.id().to_string())
                .arg(
                    "code",
                    code.map(|c| c.to_string()).unwrap_or_else(|| "-".into()),
                ),
            log,
        ),
        B::Timeout { step, log } => with_log(
            Failure::new("error-build-timeout").arg("step", step.id().to_string()),
            log,
        ),
        B::NoOutput { log } => with_log(Failure::new("error-build-no-output"), log),
        B::Cancelled => Failure::new("error-import-cancelled"),
        B::Io(e) => Failure::new("error-io").arg("detail", e.to_string()),
    }
}

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
                S::Timeout(host) => Failure::new("error-sftp-timeout").arg("host", host.clone()),
                S::Auth => Failure::new("error-sftp-auth"),
                S::Sftp(detail) => Failure::new("error-sftp").arg("detail", detail.clone()),
                S::Io(e) => Failure::new("error-io").arg("detail", e.to_string()),
                S::RemoteDir(dir) => Failure::new("error-sftp-remote-dir").arg("dir", dir.clone()),
                S::FileName(name) => Failure::new("error-sftp-file-name").arg("name", name.clone()),
            },
            Self::Checks(n) => Failure::new("error-publish-checks").arg("count", n.to_string()),
            Self::NoPrevious => Failure::new("error-no-previous"),
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
            Self::NotASite => Failure::new("error-not-a-site"),
            Self::Trash(detail) => Failure::new("error-trash").arg("detail", detail.clone()),
            Self::NoTranslation(from, to) => Failure::new("error-translate-pair")
                .arg("from", revel_model::language_name(from))
                .arg("to", revel_model::language_name(to)),
            Self::Translate(detail) => {
                Failure::new("error-translate").arg("detail", detail.clone())
            }
            Self::TranslateIntegrity(detail) => {
                Failure::new("error-translate-integrity").arg("detail", detail.clone())
            }
            Self::TranslateCancelled => Failure::new("error-translate-cancelled"),
            Self::TranslateBusy => Failure::new("error-translate-busy"),
            Self::Font(detail) => Failure::new("error-font").arg("detail", detail.clone()),
            Self::Media(detail) => Failure::new("error-media").arg("detail", detail.clone()),
            Self::NeedsBuild(tool) => {
                Failure::new("error-import-needs-build").arg("tool", tool.clone())
            }
            Self::NothingToImport => Failure::new("error-import-nothing"),
            Self::Build(e) => build_failure(e),
            Self::BadRepoUrl => Failure::new("error-git-url"),
            Self::CloneCancelled => Failure::new("error-git-clone-cancelled"),
            Self::NotAClone => Failure::new("error-not-a-clone"),
            Self::BadSiteUrl => Failure::new("error-import-url"),
            Self::SiteUnreachable(d) => {
                Failure::new("error-import-unreachable").arg("detail", d.clone())
            }
            Self::NotHtml => Failure::new("error-import-not-html"),
            Self::ImportCancelled => Failure::new("error-import-cancelled"),
            Self::NotWxr(d) => Failure::new("error-import-wxr").arg("detail", d.clone()),
            Self::FigmaUrl => Failure::new("error-figma-url"),
            Self::FigmaToken => Failure::new("error-figma-token"),
            Self::FigmaNotFound => Failure::new("error-figma-not-found"),
            Self::Figma(d) => Failure::new("error-figma").arg("detail", d.clone()),
            Self::WebflowToken => Failure::new("error-webflow-token"),
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
    /// Aperçu (protocole `revel-preview`).
    preview: Mutex<Option<canvas::Canvas>>,
    /// Traduction automatique hors ligne (D-36) : moteur (recréé quand les langues changent),
    /// modèles livrés avec l'application, langues téléchargées, téléchargement en cours.
    translator: Mutex<Option<std::sync::Arc<revel_translate::Engine>>>,
    bundled_models: Mutex<Option<PathBuf>>,
    user_models: PathBuf,
    /// Dossier des réglages de l'application (catalogues gardés pour le travail sans connexion).
    config_dir: PathBuf,
    translate_busy: AtomicBool,
    translate_cancel: AtomicBool,
    /// Dernier site ouvert pour l'édition : site des assistants (MCP) quand ils n'en nomment pas.
    last_site: Mutex<Option<PathBuf>>,
    /// Empreintes de publication par (site, environnement), avec l'état du site qu'elles décrivent.
    snapshots:
        Mutex<std::collections::HashMap<(String, String), (String, revel_project::Snapshot)>>,
    /// Copies de dépôts faites pour un import et pas encore importées (7.2).
    clones: Mutex<Vec<PathBuf>>,
    clone_cancel: AtomicBool,
    /// Interruption de l'import en cours (site lu en ligne, médias, Figma).
    import_cancel: AtomicBool,
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
            preview: Mutex::new(None),
            translator: Mutex::new(None),
            bundled_models: Mutex::new(None),
            user_models: config_dir.join("models"),
            config_dir: config_dir.to_path_buf(),
            translate_busy: AtomicBool::new(false),
            translate_cancel: AtomicBool::new(false),
            last_site: Mutex::new(None),
            snapshots: Mutex::new(Default::default()),
            clones: Mutex::new(Vec::new()),
            clone_cancel: AtomicBool::new(false),
            import_cancel: AtomicBool::new(false),
        }
    }

    /// Dossier des réglages de l'application.
    pub(crate) fn config_dir(&self) -> &Path {
        &self.config_dir
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

    /// Supprime un site depuis l'accueil : son dossier va à la corbeille du système (récupérable
    /// depuis elle), il quitte la liste des sites et sa session d'édition est fermée. Seul un
    /// dossier qui contient un site Revel est accepté.
    pub fn delete_site(&self, path: &str) -> Result<()> {
        let dir = Path::new(path);
        if !dir.join(revel_project::SITE_FILE).is_file() {
            return Err(CoreError::NotASite);
        }
        self.edit_close(dir);
        #[cfg(target_os = "macos")]
        let ctx = {
            // Sans passer par le Finder (pas d'autorisation d'automatisation demandée).
            use trash::macos::{DeleteMethod, TrashContextExtMacos};
            let mut c = trash::TrashContext::default();
            c.set_delete_method(DeleteMethod::NsFileManager);
            c
        };
        #[cfg(not(target_os = "macos"))]
        let ctx = trash::TrashContext::default();
        ctx.delete(dir)
            .map_err(|e| CoreError::Trash(e.to_string()))?;
        self.forget_site(path);
        Ok(())
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

    /// Crée un site vide dans un nouveau dossier de `parent`, nommé d'après le site
    /// (voir [`site_folder`]).
    pub fn create_site(&self, parent: &Path, name: &str, lang: &str) -> Result<SiteSummary> {
        self.new_site(&site_folder(parent, name), name, lang)
    }

    /// Crée un site d'après un modèle dans un nouveau dossier de `parent`.
    pub fn create_site_from(
        &self,
        parent: &Path,
        name: &str,
        lang: &str,
        template: revel_protocol::SiteTemplate,
        light: bool,
    ) -> Result<SiteSummary> {
        self.new_site_from(&site_folder(parent, name), name, lang, template, light)
    }

    /// Modèles proposés à la création d'un site (D-128), textes dans la langue `lang`.
    pub fn site_templates(&self, lang: &str) -> Vec<revel_protocol::TemplateInfo> {
        templates::catalogue(lang)
            .into_iter()
            .map(|i| revel_protocol::TemplateInfo {
                id: i.id,
                name: i.name,
                category: i.category,
                description: i.description,
                preview: i.preview,
                palette: i.palette,
                features: i.features,
                pages: i.pages,
            })
            .collect()
    }

    /// Crée un site d'après un modèle (`blank` : vide) dans un nouveau dossier de `parent` ;
    /// `scheme` : `light`, `dark` ou `auto` (couleurs selon l'appareil du visiteur).
    pub fn create_site_template(
        &self,
        parent: &Path,
        name: &str,
        lang: &str,
        template: &str,
        scheme: &str,
    ) -> Result<SiteSummary> {
        self.new_site_template(&site_folder(parent, name), name, lang, template, scheme)
    }

    /// Crée un site d'après un modèle dans le dossier `path` : document, contenus d'exemple,
    /// dossier des médias ; polices du modèle copiées depuis Google Fonts quand la connexion le
    /// permet (sinon les polices de repli de la feuille de style).
    pub fn new_site_template(
        &self,
        path: &Path,
        name: &str,
        lang: &str,
        template: &str,
        scheme: &str,
    ) -> Result<SiteSummary> {
        if path.join(revel_project::SITE_FILE).exists() {
            return Err(CoreError::AlreadyImported);
        }
        let scheme = templates::Scheme::parse(scheme);
        let (site, files, fonts) = if template == "blank" || template.is_empty() {
            (starter::blank(name, lang, scheme), Vec::new(), Vec::new())
        } else {
            let b = templates::build(template, name, lang, scheme).map_err(CoreError::Other)?;
            (b.site, b.files, b.fonts)
        };
        for (rel, text) in files {
            let f = path.join(&rel);
            if let Some(dir) = f.parent() {
                std::fs::create_dir_all(dir)?;
            }
            if !f.exists() {
                std::fs::write(&f, text)?;
            }
        }
        let p = revel_project::Project::create(path, site)?;
        let _ = std::fs::create_dir_all(path.join("media"));
        if !fonts.is_empty() && !cfg!(test) && self.edit_open(path).is_ok() {
            for f in &fonts {
                let _ = self.fonts_add_google(path, &f.family, &f.variants);
            }
            // Ajouts faits hors de l'historique de l'utilisateur.
            self.edit_close(path);
        }
        let summary = summarize(&revel_project::Project::open(path).unwrap_or(p))?;
        self.touch_recent(&summary);
        Ok(summary)
    }

    /// Crée un site vide.
    pub fn new_site(&self, path: &Path, name: &str, lang: &str) -> Result<SiteSummary> {
        self.new_site_from(path, name, lang, revel_protocol::SiteTemplate::Blank, false)
    }

    /// Crée un site d'après un modèle (vide, vitrine, blog), fond sombre ou clair (`light`).
    pub fn new_site_from(
        &self,
        path: &Path,
        name: &str,
        lang: &str,
        template: revel_protocol::SiteTemplate,
        light: bool,
    ) -> Result<SiteSummary> {
        if path.join(revel_project::SITE_FILE).exists() {
            return Err(CoreError::AlreadyImported);
        }
        // Contenus d'exemple et description des collections, écrits avant l'ouverture du projet.
        for (rel, text) in starter::template_files(template, lang) {
            let f = path.join(rel);
            if let Some(dir) = f.parent() {
                std::fs::create_dir_all(dir)?;
            }
            if !f.exists() {
                std::fs::write(&f, text)?;
            }
        }
        let p = revel_project::Project::create(
            path,
            starter::starter_from(name, lang, template, light),
        )?;
        // Dossier des médias (copié tel quel à la publication).
        let _ = std::fs::create_dir_all(path.join("media"));
        let summary = summarize(&p)?;
        self.touch_recent(&summary);
        Ok(summary)
    }

    /// Importe un projet Eleventy (écrit `site.revel.json` à sa racine), reconstruit dans la forme
    /// de l'interface.
    pub fn import_eleventy(&self, path: &Path, overwrite: bool) -> Result<ImportResult> {
        if !revel_import::is_eleventy(path) {
            return Err(CoreError::NotEleventy);
        }
        if path.join(revel_project::SITE_FILE).exists() && !overwrite {
            return Err(CoreError::AlreadyImported);
        }
        let profile = revel_import::eleventy::load_profile(path)?;
        let (site, mut report) = revel_import::eleventy::import(path, &profile)?;
        let mut p = revel_project::Project::create(path, site)?;
        report.warnings.extend(rebuild_for_ui(&mut p));
        let summary = summarize(&p)?;
        self.touch_recent(&summary);
        Ok(ImportResult {
            site: summary,
            stats: revel_protocol::ImportStats {
                tool: Some("Eleventy".into()),
                pages: report.pages,
                layouts: report.layouts,
                style_rules: report.style_rules,
                collections: report.collections,
                ..Default::default()
            },
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

    /// Exporte le site : génération (`<site>/.revel/out/<env>`), puis copie des fichiers dans un
    /// nouveau dossier de `parent` nommé d'après le site et l'environnement
    /// (`alunites-production`, suffixe si le dossier existe). Le résumé indique ce dossier.
    pub fn export(
        &self,
        path: &Path,
        environment: &str,
        images: bool,
        parent: &Path,
        on_event: &(dyn Fn(BuildEvent) + Sync),
    ) -> Result<BuildSummary> {
        // Progression et échec transmis ; la fin est annoncée après la copie.
        let inner = |e: BuildEvent| {
            if !matches!(e, BuildEvent::Finished(_)) {
                on_event(e)
            }
        };
        let mut summary = self.build(path, environment, images, &inner)?;
        let name = revel_project::Project::open(path)?.site.name;
        let dest = site_folder(parent, &format!("{name} {environment}"));
        if let Err(e) = copy_tree(&out_dir(path, environment), &dest) {
            let e = CoreError::Io(e);
            on_event(BuildEvent::Failed(e.failure()));
            return Err(e);
        }
        summary.out_dir = dest.display().to_string();
        on_event(BuildEvent::Finished(summary.clone()));
        Ok(summary)
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
        checks: bool,
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
        // Empreinte de ce qui est publié (pages modifiées depuis, 5.1), prise sur le site généré.
        let snapshot = if dry_run {
            None
        } else {
            tokio::task::block_in_place(|| {
                revel_project::Project::open(site)?
                    .publish_snapshot(&profile.environment)
                    .map_err(CoreError::from)
            })
            .ok()
        };
        // Contrôles (5.4) : une publication s'arrête s'il y a des points à vérifier (sauf
        // « Publier quand même ») ; une vérification les signale et continue.
        if checks {
            let found = tokio::task::block_in_place(|| checks::check_site(&out));
            if !found.is_empty() {
                let n = found.len();
                on_event(DeployEvent::Checks { checks: found });
                if !dry_run {
                    let e = CoreError::Checks(n);
                    on_event(DeployEvent::Failed(e.failure()));
                    return Err(e);
                }
            }
        }
        self.upload(site, &profile, &out, dry_run, full_sync, snapshot, on_event)
            .await
    }

    /// Envoi d'un site généré (dossier `out`) sur un hébergement ; publication réussie gardée
    /// (pages modifiées depuis, version pour revenir en arrière).
    #[allow(clippy::too_many_arguments)]
    async fn upload(
        &self,
        site: &Path,
        profile: &HostingProfile,
        out: &Path,
        dry_run: bool,
        full_sync: bool,
        snapshot: Option<revel_project::Snapshot>,
        on_event: &(dyn Fn(DeployEvent) + Send + Sync),
    ) -> Result<DeploySummary> {
        let site_key = site.display().to_string();
        let profile_id = profile.id.as_str();
        let warnings = self.host_warnings(out, &profile.target)?;
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
                        out,
                        &profile.environment,
                        repo,
                        branch,
                        custom_domain.as_deref(),
                        warnings,
                        dry_run,
                        on_event,
                    )
                    .await;
                match &r {
                    Err(e) => on_event(DeployEvent::Failed(e.failure())),
                    Ok(summary) => {
                        if let Some(snap) = snapshot {
                            self.keep_version(site, profile, out, &snap);
                            self.record_publish(site, profile_id, summary.url.clone(), snap);
                        }
                    }
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
                let report = revel_deploy::sftp::deploy(out, &target, &opts, &|e| {
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
                    url: self.destination_url(site, profile, None),
                    pages_enabled: false,
                };
                if let Some(snap) = snapshot {
                    self.keep_version(site, profile, out, &snap);
                    self.record_publish(site, profile_id, None, snap);
                }
                on_event(DeployEvent::Finished(s.clone()));
                Ok(s)
            }
        }
    }
}

/// Copie récursive d'un dossier (le dossier d'arrivée est créé).
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let target = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_tree(&e.path(), &target)?;
        } else {
            std::fs::copy(e.path(), &target)?;
        }
    }
    Ok(())
}

/// Dossier utilisable pour y créer des sites : créé s'il manque, et modifiable (un fichier
/// d'essai y est écrit puis retiré). Sert à choisir le dossier `Projects` à côté de l'application
/// (installation dans « Programmes » : modifiable seulement si l'installateur l'a ouvert aux
/// utilisateurs).
pub fn usable_dir(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(format!(".revel-essai-{}", std::process::id()));
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// Dossier d'un nouveau site dans `parent` : nom du site en minuscules, sans accents ni espaces
/// (`Atelier Lumen` -> `atelier-lumen`), suivi de `-2`, `-3`… si un dossier non vide porte déjà ce
/// nom.
pub fn site_folder(parent: &Path, name: &str) -> PathBuf {
    let mut slug = String::new();
    for c in name.trim().to_lowercase().chars() {
        let c = match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            'œ' => {
                slug.push('o');
                'e'
            }
            'æ' => {
                slug.push('a');
                'e'
            }
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if c != '-' || !slug.ends_with('-') {
            slug.push(c);
        }
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "site" } else { slug };
    let busy = |p: &Path| {
        std::fs::read_dir(p)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    };
    let mut dir = parent.join(slug);
    let mut n = 2;
    while busy(&dir) {
        dir = parent.join(format!("{slug}-{n}"));
        n += 1;
    }
    dir
}

impl Core {
    /// Collections de contenus du site, avec leurs entrées (vue CMS, lecture).
    /// Dossiers de contenus (fichiers Markdown) qu'aucune collection ne lit encore : contenus
    /// repris d'un autre site, proposés à la création d'une collection.
    pub fn cms_folders(&self, path: &Path) -> Result<Vec<revel_protocol::CmsFolder>> {
        let p = revel_project::Project::open(path)?;
        let used: std::collections::HashSet<String> = p
            .site
            .collections
            .iter()
            .map(|c| c.folder.trim_matches('/').to_string())
            .collect();
        let mut out = Vec::new();
        fn scan(
            root: &Path,
            dir: &Path,
            depth: usize,
            used: &std::collections::HashSet<String>,
            out: &mut Vec<revel_protocol::CmsFolder>,
        ) {
            let Ok(rd) = std::fs::read_dir(dir) else {
                return;
            };
            let mut count = 0;
            let mut subs = Vec::new();
            for e in rd.flatten() {
                let path = e.path();
                let name = e.file_name().to_string_lossy().into_owned();
                if path.is_dir() {
                    // Dossiers techniques et générés : jamais des contenus.
                    if !name.starts_with('.')
                        && !matches!(
                            name.as_str(),
                            "node_modules"
                                | "_site"
                                | "dist"
                                | "public"
                                | "assets"
                                | "media"
                                | "target"
                                | "_data"
                                | "_includes"
                                | "_layouts"
                        )
                    {
                        subs.push(path);
                    }
                } else if path
                    .extension()
                    .is_some_and(|x| x == "md" || x == "markdown")
                    && !name.eq_ignore_ascii_case("readme.md")
                {
                    count += 1;
                }
            }
            if let Ok(rel) = dir.strip_prefix(root) {
                let rel = rel.to_string_lossy().replace('\\', "/");
                if count > 0 && !rel.is_empty() && !used.contains(&rel) {
                    out.push(revel_protocol::CmsFolder { folder: rel, count });
                }
            }
            if depth < 4 {
                subs.sort();
                for s in subs {
                    scan(root, &s, depth + 1, used, out);
                }
            }
        }
        scan(&p.dir, &p.dir, 0, &used, &mut out);
        Ok(out)
    }

    pub fn cms(&self, path: &Path) -> Result<Vec<revel_protocol::CmsCollection>> {
        let p = revel_project::Project::open(path)?;
        let entries = p.load_entries()?;
        let locales = p.load_locales()?;
        let urls: std::collections::HashMap<(String, String), String> = p
            .list_pages()?
            .into_iter()
            .filter_map(|r| Some(((r.collection?, r.file?), r.url)))
            .collect();
        let config = pagescms::load(&p.dir);
        let pages_cms = &config.collections;
        // Un dossier lu par plusieurs collections d'Eleventy (`offres`, `toutesOffres`) : une
        // seule collection affichée, celle qui porte le nom donné par Pages CMS, sinon la
        // première.
        let mut kept: Vec<&revel_model::Collection> = Vec::new();
        for c in p.site.collections.iter().filter(|c| !c.folder.is_empty()) {
            let named = |x: &revel_model::Collection| {
                pages_cms
                    .iter()
                    .any(|pc| pc.name == x.id && pc.path == pagescms::norm_path(&x.folder))
            };
            match kept.iter().position(|k| k.folder == c.folder) {
                Some(i) if named(c) && !named(kept[i]) => kept[i] = c,
                Some(_) => {}
                None => kept.push(c),
            }
        }
        Ok(kept
            .into_iter()
            .map(|c| {
                let list: Vec<revel_protocol::CmsEntry> = entries
                    .iter()
                    .filter(|e| e.folder == c.folder)
                    .map(|e| revel_protocol::CmsEntry {
                        file: e.file_stem.clone(),
                        data: e.data.clone(),
                        body: e.body.clone(),
                        url: urls.get(&(c.id.clone(), e.file_stem.clone())).cloned(),
                    })
                    .collect();
                // Schéma : `.pages.yml` (Pages CMS), sinon celui du document, sinon les clés
                // trouvées dans les en-têtes, dans l'ordre d'apparition.
                let pages = pages_cms
                    .iter()
                    .find(|x| x.path == pagescms::norm_path(&c.folder));
                let fields = if let Some(pc) = pages {
                    pc.fields.clone()
                } else if c.fields.is_empty() {
                    let mut names: Vec<String> = Vec::new();
                    for e in &list {
                        if let serde_json::Value::Object(m) = &e.data {
                            for k in m.keys() {
                                if !names.contains(k) {
                                    names.push(k.clone());
                                }
                            }
                        }
                    }
                    names
                        .into_iter()
                        .map(|name| revel_protocol::CmsField {
                            name,
                            ..Default::default()
                        })
                        .collect()
                } else {
                    c.fields
                        .iter()
                        .map(|f| revel_protocol::CmsField {
                            name: f.name.clone(),
                            label: f.label.clone(),
                            kind: serde_json::to_value(&f.kind).ok().and_then(|v| {
                                v.get("type").and_then(|t| t.as_str()).map(str::to_string)
                            }),
                            ..Default::default()
                        })
                        .collect()
                };
                revel_protocol::CmsCollection {
                    id: c.id.clone(),
                    label: pages.and_then(|x| x.label.clone()),
                    folder: c.folder.clone(),
                    fields,
                    schema: pages.map(|x| x.name.clone()),
                    merge: config.merge,
                    view: pages.and_then(|x| x.view.clone()),
                    filename: pages.and_then(|x| x.filename.clone()),
                    locales: locales
                        .iter()
                        .map(|l| {
                            let folder = revel_model::locale_folder(&c.folder, &l.code);
                            let list = l
                                .entries
                                .iter()
                                .filter(|e| e.folder == folder)
                                .map(|e| revel_protocol::CmsEntry {
                                    file: e.file_stem.clone(),
                                    data: e.data.clone(),
                                    body: e.body.clone(),
                                    url: urls
                                        .get(&(c.id.clone(), e.file_stem.clone()))
                                        .map(|u| format!("/{}{u}", l.code)),
                                })
                                .collect();
                            (l.code.clone(), list)
                        })
                        .collect(),
                    entries: list,
                }
            })
            .collect())
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
        locale: p.site.default_locale.clone(),
        redirects: p
            .site
            .redirects
            .iter()
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect(),
    })
}

/// Site importé reconstruit dans la forme de l'interface (calques, listes de collection,
/// propriétés, réglages ; même rendu) : ce que l'interface ne pouvait pas modifier le devient.
/// Rend une ligne du bilan (points restants), ou l'erreur si la reconstruction échoue (site
/// importé gardé tel quel).
pub fn rebuild_for_ui(p: &mut revel_project::Project) -> Option<String> {
    let before = revel_edit::editable::audit(&p.site).len();
    if before == 0 {
        return None;
    }
    match p.rebuilt() {
        Ok(r) => {
            let after = revel_edit::editable::audit(&r.site).len();
            if let Err(e) = p.save_rebuilt(r) {
                return Some(format!(
                    "reconstruction dans la forme de l'interface non enregistrée : {e}"
                ));
            }
            (after > 0).then(|| {
                format!(
                    "structure de l'interface : {before} points convertis en {}, {after} restent écrits dans le code (liste : revel audit)",
                    before - after
                )
            })
        }
        Err(e) => Some(format!(
            "reconstruction dans la forme de l'interface impossible : {e}"
        )),
    }
}

#[cfg(test)]
mod tests;
