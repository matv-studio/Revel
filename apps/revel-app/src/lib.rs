//! Coque applicative : relie l'interface Leptos (`apps/revel-ui`) aux services de
//! `revel-core` par des commandes Tauri. Aucune logique métier ici.

use revel_core::{Core, CoreError};
use revel_protocol::{
    AppSettings, BuildEvent, DeployEvent, Failure, GithubEvent, HostingProfile, ImportResult,
    RecentSite, SiteSummary,
};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::{Manager, State, WebviewUrl, WebviewWindowBuilder};

struct AppState {
    core: Arc<Core>,
    /// Dossier des réglages de l'application.
    config_dir: PathBuf,
    /// Serveur des assistants IA (MCP en HTTP local), démarré si activé dans les réglages.
    mcp: std::sync::Mutex<Mcp>,
    /// Dernières modifications faites par les assistants.
    activity: revel_mcp::activity::ActivityLog,
}

#[derive(Default)]
struct Mcp {
    running: Option<revel_mcp::http::Running>,
    /// Échec du dernier démarrage (port occupé…).
    error: Option<String>,
}

/// Identifiant de l'application (`tauri.conf.json`) : dossier des réglages, lu aussi par le relais.
const IDENTIFIER: &str = "app.revel-studio.desktop";

mod drops;

/// Dossiers déposés (hors Windows, ou chemin non transmis) retrouvés par leur nom (D-122).
#[tauri::command]
fn find_dropped(app: tauri::AppHandle, s: State<AppState>, names: Vec<String>) -> Vec<String> {
    let recent: Vec<String> = s.core.recent_sites().into_iter().map(|r| r.path).collect();
    let sites = PathBuf::from(default_sites_dir(app.clone(), s));
    let places = drops::places(&recent, Some(sites), app.path().home_dir().ok());
    drops::find_named(&names, &places)
}

/// Démarre le serveur des assistants sur le cœur de l'application.
async fn start_mcp(
    core: &Arc<Core>,
    activity: &revel_mcp::activity::ActivityLog,
    c: &revel_mcp::config::McpConfig,
) -> Mcp {
    let inv: Arc<dyn revel_mcp::Invoke> = Arc::new(revel_mcp::activity::Logged {
        inner: Arc::new(revel_mcp::direct::CoreInvoke(core.clone())),
        log: activity.clone(),
    });
    match revel_mcp::http::start(inv, c.token.clone(), c.port).await {
        Ok(r) => Mcp {
            running: Some(r),
            error: None,
        },
        Err(e) => Mcp {
            running: None,
            error: Some(e.to_string()),
        },
    }
}

fn mcp_status_of(s: &AppState) -> revel_protocol::McpStatus {
    let c = revel_mcp::config::McpConfig::load(&s.config_dir);
    let m = s.mcp.lock().unwrap_or_else(|e| e.into_inner());
    revel_protocol::McpStatus {
        enabled: c.enabled,
        port: c.port,
        url: c.url(),
        token: c.token,
        running: m.running.is_some(),
        error: m.error.clone(),
        program: program_path().map(|p| p.display().to_string()),
    }
}

/// Réglages et état du serveur des assistants IA.
#[tauri::command]
fn mcp_status(s: State<AppState>) -> revel_protocol::McpStatus {
    mcp_status_of(&s)
}

/// Active ou désactive le serveur des assistants, change son port ; redémarré au besoin.
#[tauri::command]
async fn mcp_set(
    s: State<'_, AppState>,
    enabled: bool,
    port: u16,
) -> Res<revel_protocol::McpStatus> {
    let mut c = revel_mcp::config::McpConfig::load(&s.config_dir);
    c.enabled = enabled;
    c.port = if port == 0 {
        revel_mcp::config::DEFAULT_PORT
    } else {
        port
    };
    c.save(&s.config_dir).map_err(sys)?;
    restart_mcp(&s, &c).await;
    Ok(mcp_status_of(&s))
}

/// Dernières modifications faites par les assistants, la plus récente d'abord.
#[tauri::command]
fn mcp_activity(s: State<AppState>) -> Vec<revel_protocol::McpActivity> {
    s.activity.list()
}

/// Nouvelle clé d'accès (l'ancienne ne fonctionne plus) ; serveur redémarré s'il tourne.
#[tauri::command]
async fn mcp_new_token(s: State<'_, AppState>) -> Res<revel_protocol::McpStatus> {
    let mut c = revel_mcp::config::McpConfig::load(&s.config_dir);
    c.token = revel_mcp::config::new_token();
    c.save(&s.config_dir).map_err(sys)?;
    restart_mcp(&s, &c).await;
    Ok(mcp_status_of(&s))
}

async fn restart_mcp(s: &AppState, c: &revel_mcp::config::McpConfig) {
    // Ancien serveur arrêté (port libéré) avant le nouveau.
    let old = std::mem::take(&mut *s.mcp.lock().unwrap_or_else(|e| e.into_inner())).running;
    if let Some(r) = old {
        r.shutdown().await;
    }
    if c.enabled {
        let m = start_mcp(&s.core, &s.activity, c).await;
        *s.mcp.lock().unwrap_or_else(|e| e.into_inner()) = m;
    }
}

/// Programme de Revel à lancer par les assistants : l'AppImage elle-même sous Linux (le programme
/// en cours d'exécution vit dans un montage temporaire), sinon l'exécutable.
fn program_path() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
}

/// Relais stdio des assistants (`revel --mcp`) : code de sortie du processus.
pub fn mcp_relay() -> i32 {
    let Some(dir) = revel_mcp::config::app_config_dir(IDENTIFIER) else {
        eprintln!("revel --mcp : dossier des réglages introuvable");
        return 2;
    };
    match tauri::async_runtime::block_on(revel_mcp::relay::run(dir, program_path())) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("revel --mcp : {e}");
            1
        }
    }
}

/// Les échecs sont transmis à l'interface sous forme de clé de traduction (`Failure`).
type Res<T> = Result<T, Failure>;

fn err(e: CoreError) -> Failure {
    e.failure()
}

/// Échec technique hors du cœur (fenêtre, tâche interrompue…).
fn sys(e: impl std::fmt::Display) -> Failure {
    Failure::detail(e.to_string())
}

/// Mises à jour actives : clé publique de vérification présente dans la configuration (ajoutée
/// à la construction des installateurs officiels, 6.6).
fn updates_enabled(app: &tauri::AppHandle) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("pubkey"))
        .and_then(|k| k.as_str())
        .is_some_and(|k| !k.trim().is_empty())
}

/// Version plus récente publiée, s'il y en a une, et si c'est une préversion (6.6). Préversions
/// demandées : la plus récente version publiée du dépôt, préversion comprise, est cherchée d'abord ;
/// sans réponse de GitHub, l'adresse inscrite dans l'application sert seule (D-86).
async fn find_update(
    app: &tauri::AppHandle,
) -> Result<Option<(tauri_plugin_updater::Update, bool)>, Failure> {
    use tauri_plugin_updater::UpdaterExt;
    let wanted = app.state::<AppState>().core.settings().prereleases;
    let endpoint = app
        .config()
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("endpoints"))
        .and_then(|e| e.get(0))
        .and_then(|e| e.as_str())
        .map(str::to_string);
    let feed = match (wanted, endpoint) {
        (true, Some(endpoint)) => tauri::async_runtime::spawn_blocking(move || {
            revel_core::updates::prerelease_feed_online(&endpoint)
        })
        .await
        .ok()
        .flatten(),
        _ => None,
    };
    let updater = match &feed {
        Some(f) => {
            let url = tauri::Url::parse(&f.url).map_err(sys)?;
            app.updater_builder()
                .endpoints(vec![url])
                .map_err(sys)?
                .build()
                .map_err(sys)?
        }
        None => app.updater().map_err(sys)?,
    };
    // Aucune version publiée hors préversion (adresse « latest » sans réponse) : rien à proposer,
    // pas une panne.
    let found = match updater.check().await {
        Err(tauri_plugin_updater::Error::ReleaseNotFound) => None,
        r => r.map_err(|e| Failure::new("error-update").arg("detail", e.to_string()))?,
    };
    Ok(found.map(|u| {
        let pre = feed
            .as_ref()
            .is_some_and(|f| f.prerelease && f.version == u.version);
        (u, pre)
    }))
}

/// Version plus récente publiée, s'il y en a une (6.6).
#[tauri::command]
async fn update_check(app: tauri::AppHandle) -> Res<revel_protocol::UpdateCheck> {
    if !updates_enabled(&app) {
        return Ok(revel_protocol::UpdateCheck::default());
    }
    let found = find_update(&app).await?;
    Ok(revel_protocol::UpdateCheck {
        enabled: true,
        available: found.map(|(u, prerelease)| revel_protocol::UpdateInfo {
            version: u.version.clone(),
            notes: u.body.clone(),
            prerelease,
        }),
    })
}

/// Télécharge la version proposée, vérifie sa signature, l'installe et relance Revel (6.6).
#[tauri::command]
async fn update_install(app: tauri::AppHandle) -> Res<()> {
    if !updates_enabled(&app) {
        return Ok(());
    }
    let Some((update, _)) = find_update(&app).await? else {
        return Ok(());
    };
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(sys)?;
    app.restart()
}

#[tauri::command]
fn get_settings(s: State<AppState>) -> AppSettings {
    s.core.settings()
}

#[tauri::command]
fn set_settings(s: State<AppState>, settings: AppSettings) {
    s.core.set_settings(settings);
}

#[tauri::command]
fn recent_sites(s: State<AppState>) -> Vec<RecentSite> {
    s.core.recent_sites()
}

#[tauri::command]
fn forget_site(s: State<AppState>, path: String) {
    s.core.forget_site(&path);
}

/// Site supprimé depuis l'accueil : dossier mis à la corbeille du système.
#[tauri::command]
fn delete_site(s: State<AppState>, path: String) -> Res<()> {
    s.core.delete_site(&path).map_err(err)
}

#[tauri::command]
fn has_site(path: String) -> bool {
    PathBuf::from(path)
        .join(revel_project_site_file())
        .is_file()
}

fn revel_project_site_file() -> &'static str {
    "site.revel.json"
}

#[tauri::command]
fn open_site(s: State<AppState>, path: String) -> Res<SiteSummary> {
    s.core.open_site(&PathBuf::from(path)).map_err(err)
}

/// Dossier proposé pour les nouveaux sites : celui choisi dans les réglages, sinon `Projects` à
/// côté de l'application quand il est modifiable (Windows : installation dans « Programmes »,
/// dossier ouvert aux utilisateurs par l'installateur ; Linux : à côté de l'AppImage), sinon
/// `Documents/Revel` (dossier personnel à défaut). Jamais dans l'application sur macOS (paquet
/// signé, non modifiable).
#[tauri::command]
fn default_sites_dir(app: tauri::AppHandle, s: State<AppState>) -> String {
    sites_dir(&app, &s.core)
}

fn sites_dir(app: &tauri::AppHandle, core: &revel_core::Core) -> String {
    if let Some(d) = core
        .settings()
        .projects_dir
        .filter(|d| !d.trim().is_empty())
    {
        return d;
    }
    if let Some(d) = install_projects_dir() {
        return d.display().to_string();
    }
    app.path()
        .document_dir()
        .or_else(|_| app.path().home_dir())
        .map(|d| d.join("Revel").display().to_string())
        .unwrap_or_default()
}

/// Dossier `Projects` à côté de l'application, s'il est utilisable.
fn install_projects_dir() -> Option<PathBuf> {
    // Version de développement (`target/debug`) et macOS : jamais.
    if cfg!(debug_assertions) || cfg!(target_os = "macos") {
        return None;
    }
    let base = if cfg!(target_os = "linux") {
        // Paquets `.deb`/`.rpm` dans `/usr` : non modifiable ; AppImage : son dossier.
        PathBuf::from(std::env::var_os("APPIMAGE")?)
            .parent()?
            .to_path_buf()
    } else {
        std::env::current_exe().ok()?.parent()?.to_path_buf()
    };
    let dir = base.join("Projects");
    revel_core::usable_dir(&dir).then_some(dir)
}

/// Dossier qui sera créé pour un nouveau site nommé `name` dans `parent`.
#[tauri::command]
fn site_folder(parent: String, name: String) -> String {
    revel_core::site_folder(&PathBuf::from(parent), &name)
        .display()
        .to_string()
}

/// Crée un site d'après un modèle (D-128, `blank` : vide) dans un nouveau dossier de `parent`
/// (voir `site_folder`) ; `scheme` : `light`, `dark` ou `auto`. Polices du modèle téléchargées
/// au passage : hors du fil de l'interface.
#[tauri::command]
async fn create_site(
    s: State<'_, AppState>,
    parent: String,
    name: String,
    lang: String,
    template: Option<String>,
    scheme: Option<String>,
) -> Res<SiteSummary> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.create_site_template(
            &PathBuf::from(parent),
            &name,
            &lang,
            template.as_deref().unwrap_or("blank"),
            scheme.as_deref().unwrap_or("auto"),
        )
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Modèles proposés à la création d'un site (D-128).
#[tauri::command]
fn site_templates(s: State<AppState>, lang: String) -> Vec<revel_protocol::TemplateInfo> {
    s.core.site_templates(&lang)
}

/// Collections de contenus et leurs entrées (vue CMS).
#[tauri::command]
async fn cms_collections(
    s: State<'_, AppState>,
    path: String,
) -> Res<Vec<revel_protocol::CmsCollection>> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.cms(&PathBuf::from(path)))
        .await
        .map_err(sys)?
        .map_err(err)
}

/// Dossiers de contenus qu'aucune collection ne lit encore (création d'une collection).
#[tauri::command]
async fn cms_folders(s: State<'_, AppState>, path: String) -> Res<Vec<revel_protocol::CmsFolder>> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.cms_folders(&PathBuf::from(path)))
        .await
        .map_err(sys)?
        .map_err(err)
}

#[tauri::command]
fn new_site(s: State<AppState>, path: String, name: String, lang: String) -> Res<SiteSummary> {
    s.core
        .new_site(&PathBuf::from(path), &name, &lang)
        .map_err(err)
}

#[tauri::command]
async fn import_eleventy(
    s: State<'_, AppState>,
    path: String,
    overwrite: bool,
) -> Res<ImportResult> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.import_eleventy(&PathBuf::from(path), overwrite)
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Nature d'un dossier avant import (7.1, 7.2).
#[tauri::command]
fn probe_site(s: State<AppState>, path: String) -> Res<revel_protocol::ImportProbe> {
    s.core.probe_site(&PathBuf::from(path)).map_err(err)
}

/// Importe un site (Eleventy ou HTML statique) dans son dossier ; export Webflow : contenus du
/// CMS (fichiers CSV ou API).
#[tauri::command]
async fn import_site(
    s: State<'_, AppState>,
    path: String,
    options: revel_protocol::ImportOptions,
    on_event: Channel<revel_protocol::ImportEvent>,
) -> Res<ImportResult> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.import_site(&PathBuf::from(path), &options, |e| {
            let _ = on_event.send(e);
        })
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Lit le site publié à une adresse et l'importe dans un nouveau dossier de `parent` (7.4).
#[tauri::command]
async fn import_url(
    s: State<'_, AppState>,
    url: String,
    parent: String,
    lang: String,
    on_event: Channel<revel_protocol::ImportEvent>,
) -> Res<ImportResult> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.import_url(&url, &PathBuf::from(parent), &lang, |e| {
            let _ = on_event.send(e);
        })
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Ce que la construction d'un projet par Revel utilisera et téléchargera (D-88).
#[tauri::command]
async fn project_build_plan(
    s: State<'_, AppState>,
    path: String,
) -> Res<revel_protocol::BuildPlan> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.build_plan(&PathBuf::from(path)))
        .await
        .map_err(sys)?
        .map_err(err)
}

/// Construit un projet puis importe le site produit (D-88).
#[tauri::command]
async fn project_build_import(
    s: State<'_, AppState>,
    path: String,
    parent: String,
    lang: String,
    on_event: Channel<revel_protocol::BuildProgress>,
) -> Res<ImportResult> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.build_import(&PathBuf::from(path), &PathBuf::from(parent), &lang, |e| {
            let _ = on_event.send(e);
        })
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Adresses du site publié d'un projet, vérifiées en ligne.
#[tauri::command]
async fn published_sites(
    s: State<'_, AppState>,
    path: String,
    repo: Option<String>,
) -> Res<Vec<String>> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.published_sites(&PathBuf::from(path), repo.as_deref())
    })
    .await
    .map_err(sys)
}

/// Outils de construction téléchargés (Réglages).
#[tauri::command]
async fn build_tools(s: State<'_, AppState>) -> Res<revel_protocol::BuildTools> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.build_tools())
        .await
        .map_err(sys)
}

#[tauri::command]
async fn build_tool_remove(s: State<'_, AppState>, id: String) -> Res<()> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.build_tool_remove(&id))
        .await
        .map_err(sys)?
        .map_err(err)
}

#[tauri::command]
async fn build_cache_clear(s: State<'_, AppState>) -> Res<()> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.build_cache_clear())
        .await
        .map_err(sys)?
        .map_err(err)
}

/// Interrompt l'import en cours.
#[tauri::command]
fn import_cancel(s: State<AppState>) {
    s.core.import_cancel();
}

/// Examine un fichier d'export WordPress (7.6).
#[tauri::command]
fn probe_wordpress(s: State<AppState>, file: String) -> Res<revel_protocol::WxrProbe> {
    s.core.probe_wordpress(&PathBuf::from(file)).map_err(err)
}

/// Importe un site WordPress depuis son fichier d'export dans un nouveau dossier de `parent`.
#[tauri::command]
async fn import_wordpress(
    s: State<'_, AppState>,
    file: String,
    parent: String,
    lang: String,
    media: bool,
    on_event: Channel<revel_protocol::ImportEvent>,
) -> Res<ImportResult> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.import_wordpress(
            &PathBuf::from(file),
            &PathBuf::from(parent),
            &lang,
            media,
            |e| {
                let _ = on_event.send(e);
            },
        )
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Un jeton Figma est gardé dans le trousseau.
#[tauri::command]
fn figma_has_token(s: State<AppState>) -> bool {
    s.core.figma_has_token()
}

#[tauri::command]
fn figma_forget_token(s: State<AppState>) -> Res<()> {
    s.core.figma_forget_token().map_err(err)
}

/// Lit les pages et cadres d'un fichier Figma (7.5).
#[tauri::command]
async fn figma_read(
    s: State<'_, AppState>,
    url: String,
    token: Option<String>,
) -> Res<revel_protocol::FigmaFile> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.figma_read(&url, token.as_deref()))
        .await
        .map_err(sys)?
        .map_err(err)
}

/// Importe des cadres d'un fichier Figma dans un nouveau dossier de `parent`.
#[tauri::command]
async fn import_figma(
    s: State<'_, AppState>,
    url: String,
    frames: Vec<String>,
    parent: String,
    lang: String,
    on_event: Channel<revel_protocol::ImportEvent>,
) -> Res<ImportResult> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.import_figma(&url, &frames, &PathBuf::from(parent), &lang, |e| {
            let _ = on_event.send(e);
        })
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Copie un dépôt Git dans un nouveau dossier de `parent` (7.2).
#[tauri::command]
async fn git_clone(
    s: State<'_, AppState>,
    url: String,
    parent: String,
    on_event: Channel<revel_protocol::CloneEvent>,
) -> Res<revel_protocol::ClonedRepo> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.git_clone(&url, &PathBuf::from(parent), |e| {
            let _ = on_event.send(e);
        })
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

#[tauri::command]
fn git_clone_cancel(s: State<AppState>) {
    s.core.git_clone_cancel();
}

/// Retire une copie de dépôt pas importée.
#[tauri::command]
fn discard_clone(s: State<AppState>, path: String) -> Res<()> {
    s.core.discard_clone(&PathBuf::from(path)).map_err(err)
}

#[tauri::command]
async fn build_site(
    s: State<'_, AppState>,
    path: String,
    environment: String,
    images: bool,
    on_event: Channel<BuildEvent>,
) -> Res<()> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.build(&PathBuf::from(path), &environment, images, &|e| {
            let _ = on_event.send(e);
        })
    })
    .await
    .map_err(sys)?
    .map(|_| ())
    .map_err(err)
}

/// Aperçu du site dans une fenêtre : pages rendues à la demande depuis le document en cours
/// (protocole `revel-preview`, rien à générer avant), à partir de `page` (`/agence/`). Commande
/// asynchrone : une fenêtre créée dans une commande synchrone bloque sous Windows.
#[tauri::command]
async fn open_preview(
    app: tauri::AppHandle,
    s: State<'_, AppState>,
    path: String,
    environment: String,
    page: Option<String>,
) -> Res<String> {
    let base = s
        .core
        .preview_open(&PathBuf::from(&path), &environment)
        .map_err(err)?;
    let page = page.unwrap_or_default();
    let url = format!("{base}{}", page.trim_start_matches('/'));
    let parsed: tauri::Url = url.parse().map_err(sys)?;
    if let Some(w) = app.get_webview_window("preview") {
        w.navigate(parsed).map_err(sys)?;
        let _ = w.set_focus();
    } else {
        // Windows : `http://revel-preview.localhost/` ; ailleurs : `revel-preview://localhost/`.
        let target = if parsed.scheme() == "http" || parsed.scheme() == "https" {
            WebviewUrl::External(parsed)
        } else {
            WebviewUrl::CustomProtocol(parsed)
        };
        let opener = app.clone();
        WebviewWindowBuilder::new(&app, "preview", target)
            .title("Revel — aperçu")
            .inner_size(1280.0, 860.0)
            // Lien vers un autre site en nouvel onglet : navigateur du système (les liens ouverts
            // dans la fenêtre même s'y chargent).
            .on_new_window(move |url, _| {
                if matches!(url.scheme(), "http" | "https") {
                    use tauri_plugin_opener::OpenerExt;
                    let _ = opener.opener().open_url(url.as_str(), None::<&str>);
                }
                tauri::webview::NewWindowResponse::Deny
            })
            .build()
            .map_err(sys)?;
    }
    Ok(url)
}

/// Adresse de l'aperçu du site, affiché dans l'espace de travail (cadre de l'interface) ; rien à
/// générer avant.
#[tauri::command]
async fn preview_url(s: State<'_, AppState>, path: String, environment: String) -> Res<String> {
    s.core
        .preview_open(&PathBuf::from(&path), &environment)
        .map_err(err)
}

/// Exporte le site (génération, puis copie dans un nouveau dossier de `parent`).
#[tauri::command]
async fn export_site(
    s: State<'_, AppState>,
    path: String,
    environment: String,
    images: bool,
    parent: String,
    on_event: Channel<BuildEvent>,
) -> Res<String> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.export(
            &PathBuf::from(path),
            &environment,
            images,
            &PathBuf::from(parent),
            &|e| {
                let _ = on_event.send(e);
            },
        )
    })
    .await
    .map_err(sys)?
    .map(|s| s.out_dir)
    .map_err(err)
}

/// Montre un dossier ou un fichier dans l'explorateur du système (dossier exporté).
#[tauri::command]
fn reveal_path(app: tauri::AppHandle, path: String) -> Res<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().reveal_item_in_dir(&path).map_err(sys)
}

#[tauri::command]
fn list_profiles(s: State<AppState>, site: String) -> Vec<(HostingProfile, bool)> {
    s.core
        .profiles(&site)
        .into_iter()
        .map(|p| {
            let has = s.core.has_secret(&p.target);
            (p, has)
        })
        .collect()
}

#[tauri::command]
fn save_profile(
    s: State<AppState>,
    site: String,
    profile: HostingProfile,
    password: Option<String>,
) -> Res<HostingProfile> {
    s.core.save_profile(&site, profile, password).map_err(err)
}

#[tauri::command]
fn delete_profile(s: State<AppState>, site: String, id: String) -> Res<()> {
    s.core.delete_profile(&site, &id).map_err(err)
}

#[tauri::command]
async fn deploy(
    s: State<'_, AppState>,
    site: String,
    profile_id: String,
    dry_run: bool,
    skip_checks: Option<bool>,
    on_event: Channel<DeployEvent>,
) -> Res<()> {
    let core = s.core.clone();
    let send = move |e: DeployEvent| {
        let _ = on_event.send(e);
    };
    let checks = !skip_checks.unwrap_or(false);
    core.deploy(
        &PathBuf::from(site),
        &profile_id,
        dry_run,
        false,
        checks,
        &send,
    )
    .await
    .map(|_| ())
    .map_err(err)
}

/// Vérifie qu'un domaine mène au site publié sur un hébergement (5.7).
#[tauri::command]
async fn domain_check(
    s: State<'_, AppState>,
    site: String,
    profile_id: String,
    domain: String,
) -> Res<revel_protocol::DomainStatus> {
    let core = s.core.clone();
    core.domain_check(&PathBuf::from(site), &profile_id, &domain)
        .await
        .map_err(err)
}

/// Vérifie des adresses d'autres sites : vidéos, réseaux sociaux, liens (plan du site, D-107).
#[tauri::command]
async fn links_check(
    s: State<'_, AppState>,
    site: String,
    urls: Vec<String>,
    mode: String,
) -> Res<Vec<Option<revel_protocol::LinkCheck>>> {
    let core = s.core.clone();
    core.links_check(
        &PathBuf::from(site),
        urls,
        revel_core::links::CheckMode::parse(&mode),
    )
    .await
    .map_err(err)
}

/// Remet en ligne la version précédente d'un hébergement (5.5).
#[tauri::command]
async fn rollback(
    s: State<'_, AppState>,
    site: String,
    profile_id: String,
    on_event: Channel<DeployEvent>,
) -> Res<()> {
    let core = s.core.clone();
    let send = move |e: DeployEvent| {
        let _ = on_event.send(e);
    };
    core.rollback(&PathBuf::from(site), &profile_id, &send)
        .await
        .map(|_| ())
        .map_err(err)
}

/// Connexion d'essai à un hébergement SFTP avant de l'enregistrer (5.3).
#[tauri::command]
async fn hosting_probe(
    s: State<'_, AppState>,
    site: String,
    profile: HostingProfile,
    password: Option<String>,
) -> Res<revel_protocol::HostProbe> {
    let core = s.core.clone();
    core.hosting_probe(&PathBuf::from(site), &profile, password)
        .await
        .map_err(err)
}

/// Destinations de publication du site : dernière publication, pages modifiées depuis (5.1).
#[tauri::command]
async fn publish_state(
    s: State<'_, AppState>,
    path: String,
) -> Res<Vec<revel_protocol::PublishDestination>> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.publish_state(&PathBuf::from(path)))
        .await
        .map_err(sys)?
        .map_err(err)
}

/// Ouvre un site dans l'éditeur : document complet et état de l'historique.
#[tauri::command]
fn edit_open(s: State<AppState>, path: String) -> Res<revel_edit::Opened> {
    s.core.edit_open(&PathBuf::from(path)).map_err(err)
}

/// Applique une commande d'édition (enregistrée aussitôt).
#[tauri::command]
fn edit_apply(
    s: State<AppState>,
    path: String,
    command: revel_edit::Command,
    coalesce: bool,
    group: Option<String>,
) -> Res<revel_edit::EditState> {
    let path = PathBuf::from(path);
    match group {
        // Lots d'une même opération (traduction automatique) : une seule étape d'annulation.
        Some(g) => s.core.edit_apply_grouped(&path, command, &g),
        None => s.core.edit_apply(&path, command, coalesce),
    }
    .map_err(err)
}

#[tauri::command]
fn edit_seal(s: State<AppState>, path: String) -> Res<()> {
    s.core.edit_seal(&PathBuf::from(path)).map_err(err)
}

#[tauri::command]
fn edit_undo(s: State<AppState>, path: String) -> Res<revel_edit::EditState> {
    s.core.edit_undo(&PathBuf::from(path)).map_err(err)
}

#[tauri::command]
fn edit_redo(s: State<AppState>, path: String) -> Res<revel_edit::EditState> {
    s.core.edit_redo(&PathBuf::from(path)).map_err(err)
}

/// Compte GitHub connecté (identifiant), sans requête réseau.
#[tauri::command]
fn github_account(s: State<AppState>) -> Option<String> {
    s.core.github_account()
}

/// Connexion à GitHub : le code à saisir sur github.com est envoyé par le canal.
#[tauri::command]
async fn github_connect(s: State<'_, AppState>, on_event: Channel<GithubEvent>) -> Res<String> {
    let core = s.core.clone();
    core.github_connect(&move |e| {
        let _ = on_event.send(e);
    })
    .await
    .map_err(err)
}

#[tauri::command]
fn github_cancel(s: State<AppState>) {
    s.core.github_cancel();
}

#[tauri::command]
fn github_disconnect(s: State<AppState>) -> Res<()> {
    s.core.github_disconnect().map_err(err)
}

/// Ouvre une adresse web dans le navigateur par défaut (HTTPS uniquement).
#[tauri::command]
fn open_external(app: tauri::AppHandle, url: String) -> Res<()> {
    use tauri_plugin_opener::OpenerExt;
    if !url.starts_with("https://") {
        return Err(Failure::detail(format!("adresse refusée : {url}")));
    }
    app.opener().open_url(url, None::<&str>).map_err(sys)
}

/// Affiche un site ouvert pour l'édition dans le canevas ; renvoie l'adresse de base du protocole.
#[tauri::command]
fn canvas_open(s: State<AppState>, path: String, scripts: bool) -> Res<String> {
    s.core
        .canvas_open(
            std::path::Path::new(&path),
            revel_core::canvas::CanvasOptions {
                scripts,
                ..Default::default()
            },
        )
        .map_err(err)
}

/// Pages HTML d'un site ouvert pour l'édition.
#[tauri::command]
fn edit_pages(s: State<AppState>, path: String) -> Res<Vec<revel_protocol::EditorPageInfo>> {
    s.core.edit_pages(std::path::Path::new(&path)).map_err(err)
}

/// Textes des fichiers de données à traduire dans une autre langue (vue Localisation).
#[tauri::command]
fn edit_locale_data(
    s: State<AppState>,
    path: String,
    code: String,
) -> Res<Vec<revel_edit::locale::DataText>> {
    s.core
        .edit_locale_data(std::path::Path::new(&path), &code)
        .map_err(err)
}

/// Traduction automatique possible d'une langue vers une autre (modèles livrés, D-36).
#[tauri::command]
fn translate_can(s: State<AppState>, from: String, to: String) -> bool {
    s.core.translate_can(&from, &to)
}

/// Traduction automatique hors ligne d'un lot de textes (plusieurs secondes : hors du fil
/// principal).
#[tauri::command]
async fn translate(
    s: State<'_, AppState>,
    from: String,
    to: String,
    items: Vec<revel_protocol::TranslateItem>,
    keep: Vec<String>,
) -> Res<Vec<String>> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.translate(&from, &to, &items, &keep))
        .await
        .map_err(sys)?
        .map_err(err)
}

/// Réglages lisibles (3.12) : valeurs et libellés des chemins de données, contenus proposés.
#[tauri::command]
fn readable_data(
    s: State<AppState>,
    path: String,
    paths: Vec<Vec<String>>,
    collections: Vec<String>,
) -> Res<revel_protocol::ReadableData> {
    s.core
        .readable_data(&PathBuf::from(path), &paths, &collections)
        .map_err(err)
}

/// Versions du site (3.10) : état ; `None` : versions pas encore activées.
#[tauri::command]
fn versions_status(
    s: State<AppState>,
    path: String,
) -> Res<Option<revel_protocol::VersionsStatus>> {
    s.core.versions_status(&PathBuf::from(path)).map_err(err)
}

#[tauri::command]
async fn versions_init(
    s: State<'_, AppState>,
    path: String,
    message: String,
) -> Res<revel_protocol::VersionInfo> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.versions_init(&PathBuf::from(path), &message))
        .await
        .map_err(sys)?
        .map_err(err)
}

#[tauri::command]
fn versions_list(
    s: State<AppState>,
    path: String,
    limit: usize,
) -> Res<Vec<revel_protocol::VersionInfo>> {
    s.core
        .versions_list(&PathBuf::from(path), limit)
        .map_err(err)
}

#[tauri::command]
async fn versions_save(
    s: State<'_, AppState>,
    path: String,
    message: String,
) -> Res<Option<revel_protocol::VersionInfo>> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.versions_save(&PathBuf::from(path), &message))
        .await
        .map_err(sys)?
        .map_err(err)
}

#[tauri::command]
fn versions_changes(
    s: State<AppState>,
    path: String,
    id: Option<String>,
    of_version: bool,
) -> Res<Vec<revel_protocol::FileChange>> {
    s.core
        .versions_changes(&PathBuf::from(path), id.as_deref(), of_version)
        .map_err(err)
}

#[tauri::command]
fn versions_diff(
    s: State<AppState>,
    path: String,
    id: Option<String>,
    of_version: bool,
    file: String,
) -> Res<Option<Vec<revel_protocol::DiffLine>>> {
    let p = PathBuf::from(path);
    match (id, of_version) {
        (Some(i), true) => s.core.versions_version_diff(&p, &i, &file),
        (id, _) => s.core.versions_diff(&p, id.as_deref(), &file),
    }
    .map_err(err)
}

#[tauri::command]
async fn versions_restore(
    s: State<'_, AppState>,
    path: String,
    id: String,
    before: String,
    message: String,
) -> Res<Option<revel_protocol::VersionInfo>> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.versions_restore(&PathBuf::from(path), &id, &before, &message)
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Le dépôt commun a une version nouvelle (synchronisation automatique, D-108).
#[tauri::command]
async fn versions_incoming(s: State<'_, AppState>, path: String) -> Res<bool> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.versions_incoming(&PathBuf::from(path)))
        .await
        .map_err(sys)?
        .map_err(err)
}

/// Sites Revel des dépôts GitHub du compte connecté et leur copie sur cet ordinateur (D-127).
#[tauri::command]
async fn github_sites(
    app: tauri::AppHandle,
    s: State<'_, AppState>,
) -> Res<Vec<revel_protocol::GithubSite>> {
    let core = s.core.clone();
    let dir = sites_dir(&app, &core);
    tauri::async_runtime::spawn_blocking(move || {
        let d = PathBuf::from(&dir);
        core.github_sites((!dir.is_empty()).then_some(d.as_path()))
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Met à jour toutes les copies de sites GitHub de cet ordinateur (D-127).
#[tauri::command]
async fn github_sync_all(
    app: tauri::AppHandle,
    s: State<'_, AppState>,
    merge: String,
) -> Res<revel_protocol::GithubSyncReport> {
    let core = s.core.clone();
    let dir = sites_dir(&app, &core);
    tauri::async_runtime::spawn_blocking(move || {
        let d = PathBuf::from(&dir);
        core.github_sync_all((!dir.is_empty()).then_some(d.as_path()), &merge)
    })
    .await
    .map_err(sys)
}

/// Reçoit les versions du dépôt commun sans en enregistrer (D-108).
#[tauri::command]
async fn versions_receive(
    s: State<'_, AppState>,
    path: String,
    merge: String,
) -> Res<revel_protocol::ReceiveInfo> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.versions_receive(&PathBuf::from(path), &merge)
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

#[tauri::command]
async fn versions_sync(
    s: State<'_, AppState>,
    path: String,
    message: String,
    merge: String,
) -> Res<revel_protocol::SyncInfo> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.versions_sync(&PathBuf::from(path), &message, &merge)
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

#[tauri::command]
async fn versions_resolve(
    s: State<'_, AppState>,
    path: String,
    keep_ours: bool,
    message: String,
) -> Res<revel_protocol::SyncInfo> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.versions_resolve(&PathBuf::from(path), keep_ours, &message)
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Endroits modifiés des deux côtés choisis un par un, puis envoi (D-89).
#[tauri::command]
async fn versions_resolve_items(
    s: State<'_, AppState>,
    path: String,
    keep_ours: bool,
    items: Vec<(String, bool)>,
    message: String,
) -> Res<revel_protocol::SyncInfo> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.versions_resolve_items(&PathBuf::from(path), keep_ours, &items, &message)
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Dépôt central dans un dossier partagé (NAS, réseau local, D-89).
#[tauri::command]
async fn versions_set_shared(s: State<'_, AppState>, path: String, folder: String) -> Res<String> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.versions_set_shared(&PathBuf::from(path), &PathBuf::from(folder))
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

#[tauri::command]
fn versions_set_remote(s: State<AppState>, path: String, url: String) -> Res<()> {
    s.core
        .versions_set_remote(&PathBuf::from(path), &url)
        .map_err(err)
}

/// Polices du site (règles `@font-face`, 3.9).
#[tauri::command]
fn fonts(s: State<AppState>, path: String) -> Res<Vec<revel_protocol::FontFamily>> {
    s.core.fonts(&PathBuf::from(path)).map_err(err)
}

/// Catalogue de Google Fonts (gardé pour le travail sans connexion).
#[tauri::command]
async fn fonts_google(s: State<'_, AppState>) -> Res<Vec<revel_protocol::GoogleFont>> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.fonts_google())
        .await
        .map_err(sys)?
        .map_err(err)
}

/// Ajoute une police de Google Fonts (fichiers téléchargés dans le site).
#[tauri::command]
async fn fonts_add_google(
    s: State<'_, AppState>,
    path: String,
    family: String,
    variants: Vec<String>,
) -> Res<revel_edit::EditState> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.fonts_add_google(&PathBuf::from(path), &family, &variants)
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Ajoute des fichiers de police importés.
#[tauri::command]
async fn fonts_add_files(
    s: State<'_, AppState>,
    path: String,
    family: String,
    files: Vec<revel_protocol::FontUpload>,
) -> Res<revel_edit::EditState> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.fonts_add_files(&PathBuf::from(path), &family, &files)
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

/// Retire une police du site (annulable).
#[tauri::command]
fn fonts_remove(s: State<AppState>, path: String, family: String) -> Res<revel_edit::EditState> {
    s.core
        .fonts_remove(&PathBuf::from(path), &family)
        .map_err(err)
}

/// Langues de traduction automatique : livrées, téléchargées, à télécharger.
#[tauri::command]
fn translate_packs(s: State<AppState>) -> Vec<revel_protocol::TranslatePack> {
    s.core.translate_packs()
}

/// Télécharge une langue de traduction (avancement transmis par `on_event`).
#[tauri::command]
async fn translate_install(
    s: State<'_, AppState>,
    code: String,
    on_event: Channel<revel_protocol::TranslateEvent>,
) -> Res<()> {
    let core = s.core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.translate_install(&code, |e| {
            let _ = on_event.send(e);
        })
    })
    .await
    .map_err(sys)?
    .map_err(err)
}

#[tauri::command]
fn translate_install_cancel(s: State<AppState>) {
    s.core.translate_install_cancel();
}

/// Retire une langue de traduction téléchargée.
#[tauri::command]
fn translate_remove(s: State<AppState>, code: String) -> Res<()> {
    s.core.translate_remove(&code).map_err(err)
}

/// Médias d'un site ouvert pour l'édition (dossiers copiés tels quels).
#[tauri::command]
fn edit_media(s: State<AppState>, path: String) -> Res<Vec<revel_protocol::MediaFile>> {
    s.core.edit_media(std::path::Path::new(&path)).map_err(err)
}

/// Images en ligne d'un site ouvert (adresses, D-128).
#[tauri::command]
fn media_online(s: State<AppState>, path: String) -> Res<Vec<String>> {
    s.core.media_online(std::path::Path::new(&path)).map_err(err)
}

/// Copie les images en ligne dans les médias du site (une étape d'annulation).
#[tauri::command]
async fn media_localize(s: State<'_, AppState>, path: String) -> Res<serde_json::Value> {
    let core = s.core.clone();
    let (state, count) = tauri::async_runtime::spawn_blocking(move || {
        core.media_localize(std::path::Path::new(&path))
    })
    .await
    .map_err(sys)?
    .map_err(err)?;
    Ok(serde_json::json!({ "state": state, "count": count }))
}

/// Importe un média (contenu en base64) dans le dossier des médias du site.
#[tauri::command]
fn edit_import_media(
    s: State<AppState>,
    path: String,
    name: String,
    data: String,
) -> Res<revel_protocol::MediaFile> {
    let bytes = decode_b64(&data)?;
    s.core
        .edit_import_media(std::path::Path::new(&path), &name, &bytes)
        .map_err(err)
}

/// Remplace le contenu d'un média (même adresse).
#[tauri::command]
fn edit_replace_media(
    s: State<AppState>,
    path: String,
    url: String,
    name: String,
    data: String,
) -> Res<revel_protocol::MediaFile> {
    let bytes = decode_b64(&data)?;
    s.core
        .edit_replace_media(std::path::Path::new(&path), &url, &name, &bytes)
        .map_err(err)
}

fn decode_b64(data: &str) -> Res<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| Failure::detail(format!("base64 : {e}")))
}

/// Empreinte des fichiers du site (rechargement après une modification hors de Revel).
#[tauri::command]
fn edit_stamp(s: State<AppState>, path: String) -> Res<revel_edit::EditStamp> {
    s.core.edit_stamp(std::path::Path::new(&path)).map_err(err)
}

/// Réponse des protocoles `revel-canvas` (canevas de l'éditeur) et `revel-preview` (aperçu),
/// sans port réseau.
fn canvas_protocol(
    app: &tauri::AppHandle,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
    preview: bool,
) {
    let core = app.state::<AppState>().core.clone();
    // Adresse avec sa requête (canevas des composants : `?component=…`).
    let path = request
        .uri()
        .path_and_query()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());
    let range = request
        .headers()
        .get(tauri::http::header::RANGE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    std::thread::spawn(move || {
        let r = if preview {
            core.preview_respond(&path)
        } else {
            core.canvas_respond(&path)
        };
        let mut builder = tauri::http::Response::builder()
            .header(tauri::http::header::CONTENT_TYPE, r.content_type)
            .header(tauri::http::header::CACHE_CONTROL, "no-store")
            .header(tauri::http::header::ACCEPT_RANGES, "bytes");
        let mut status = r.status;
        let mut body = r.body;
        // Vidéos et sons : lecture par morceaux demandée par le lecteur du navigateur.
        if status == 200
            && let Some((start, end)) = range.as_deref().and_then(|h| byte_range(h, body.len()))
        {
            builder = builder.header(
                tauri::http::header::CONTENT_RANGE,
                format!("bytes {start}-{end}/{}", body.len()),
            );
            body = body[start..=end].to_vec();
            status = 206;
        }
        let response = builder
            .status(status)
            .body(body)
            .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()));
        responder.respond(response);
    });
}

/// Morceau demandé par un en-tête `Range: bytes=début-fin` (un seul morceau ; `bytes=-n` : les
/// n derniers octets), borné à la taille ; `None` si l'en-tête ne se lit pas ou sort du fichier.
fn byte_range(header: &str, len: usize) -> Option<(usize, usize)> {
    let spec = header.trim().strip_prefix("bytes=")?;
    if spec.contains(',') || len == 0 {
        return None;
    }
    let (a, b) = spec.split_once('-')?;
    let (a, b) = (a.trim(), b.trim());
    let (start, end) = if a.is_empty() {
        let n: usize = b.parse().ok()?;
        (len.saturating_sub(n), len - 1)
    } else {
        let start: usize = a.parse().ok()?;
        let end = if b.is_empty() {
            len - 1
        } else {
            b.parse::<usize>().ok()?.min(len - 1)
        };
        (start, end)
    };
    (start <= end && start < len).then_some((start, end))
}

/// Site à ouvrir dans l'éditeur au démarrage : argument `--open DOSSIER` (ou `--open=DOSSIER`),
/// sinon variable d'environnement `REVEL_OPEN` (essais de l'application, diagnostic).
#[tauri::command]
fn startup_site() -> Option<String> {
    open_arg(std::env::args().skip(1))
        .or_else(|| std::env::var("REVEL_OPEN").ok())
        .filter(|p| !p.is_empty())
}

/// Valeur de l'argument `--open` dans une ligne de commande.
fn open_arg(mut args: impl Iterator<Item = String>) -> Option<String> {
    while let Some(a) = args.next() {
        if a == "--open" {
            return args.next();
        }
        if let Some(p) = a.strip_prefix("--open=") {
            return Some(p.to_string());
        }
    }
    None
}

/// Informations de version (écran « À propos »).
#[tauri::command]
fn app_info() -> (String, u32) {
    (
        env!("CARGO_PKG_VERSION").to_string(),
        revel_model::SCHEMA_VERSION,
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .register_asynchronous_uri_scheme_protocol("revel-canvas", |ctx, request, responder| {
            canvas_protocol(ctx.app_handle(), request, responder, false);
        })
        .register_asynchronous_uri_scheme_protocol("revel-preview", |ctx, request, responder| {
            canvas_protocol(ctx.app_handle(), request, responder, true);
        })
        .setup(|app| {
            let dir = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("revel"));
            let secrets: Box<dyn revel_core::SecretStore> = Box::new(revel_core::secrets::Keyring);
            let core = Arc::new(Core::new(&dir, secrets));
            // Modèles de traduction placés dans les ressources de l'application : aucun n'est livré
            // depuis la 0.5.1 (D-79), le dossier n'est lu que s'il existe.
            if let Ok(models) = app
                .path()
                .resolve("models", tauri::path::BaseDirectory::Resource)
            {
                core.set_models_dir(&models);
            }
            // Serveur des assistants IA (étape 4), désactivé par défaut.
            let mcp_config = revel_mcp::config::McpConfig::load(&dir);
            let activity = revel_mcp::activity::ActivityLog::default();
            let mcp = if mcp_config.enabled {
                tauri::async_runtime::block_on(start_mcp(&core, &activity, &mcp_config))
            } else {
                Mcp::default()
            };
            app.manage(AppState {
                core,
                config_dir: dir,
                mcp: std::sync::Mutex::new(mcp),
                activity,
            });
            // Dossiers déposés sur la fenêtre (D-122).
            app.manage(drops::Dropped::default());
            drops::watch(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            update_check,
            update_install,
            get_settings,
            set_settings,
            recent_sites,
            forget_site,
            delete_site,
            has_site,
            open_site,
            new_site,
            default_sites_dir,
            site_folder,
            create_site,
            cms_collections,
            cms_folders,
            import_eleventy,
            probe_site,
            import_site,
            import_url,
            import_cancel,
            project_build_plan,
            project_build_import,
            published_sites,
            build_tools,
            build_tool_remove,
            build_cache_clear,
            probe_wordpress,
            import_wordpress,
            figma_has_token,
            figma_forget_token,
            figma_read,
            import_figma,
            git_clone,
            git_clone_cancel,
            discard_clone,
            build_site,
            export_site,
            reveal_path,
            open_preview,
            preview_url,
            list_profiles,
            save_profile,
            delete_profile,
            deploy,
            publish_state,
            hosting_probe,
            rollback,
            domain_check,
            github_account,
            github_connect,
            github_cancel,
            github_disconnect,
            open_external,
            edit_open,
            edit_apply,
            edit_seal,
            edit_undo,
            edit_redo,
            edit_pages,
            edit_stamp,
            edit_media,
            edit_locale_data,
            translate_can,
            translate,
            translate_packs,
            fonts,
            readable_data,
            mcp_status,
            mcp_set,
            mcp_new_token,
            mcp_activity,
            versions_status,
            versions_init,
            versions_list,
            versions_save,
            versions_changes,
            versions_diff,
            versions_restore,
            versions_sync,
            versions_incoming,
            versions_receive,
            github_sites,
            site_templates,
            github_sync_all,
            links_check,
            versions_resolve,
            versions_resolve_items,
            versions_set_shared,
            versions_set_remote,
            fonts_google,
            fonts_add_google,
            fonts_add_files,
            fonts_remove,
            translate_install,
            translate_install_cancel,
            translate_remove,
            edit_import_media,
            media_online,
            media_localize,
            edit_replace_media,
            canvas_open,
            startup_site,
            drops::dropped_paths,
            find_dropped
        ])
        .run(tauri::generate_context!())
        .expect("erreur au démarrage de Revel");
}

#[cfg(test)]
mod tests {
    use super::{byte_range, open_arg};

    /// Lecture des vidéos dans l'aperçu : morceaux demandés par le lecteur.
    #[test]
    fn morceaux_demandes() {
        assert_eq!(byte_range("bytes=0-", 10), Some((0, 9)));
        assert_eq!(byte_range("bytes=2-5", 10), Some((2, 5)));
        assert_eq!(byte_range("bytes=8-99", 10), Some((8, 9)));
        assert_eq!(byte_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(byte_range("bytes=10-", 10), None);
        assert_eq!(byte_range("bytes=0-1,4-5", 10), None);
        assert_eq!(byte_range("items=0-1", 10), None);
    }

    fn args(list: &[&str]) -> impl Iterator<Item = String> {
        list.iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Images, vidéos et polices du site affichées par l'interface (vignettes des médias, des
    /// champs du CMS, des fonds) : servies par le protocole du canevas, que la politique de
    /// sécurité doit autoriser (sous Windows, `http://revel-canvas.localhost`).
    #[test]
    fn politique_de_securite_autorise_le_canevas() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let csp = conf["app"]["security"]["csp"].as_str().unwrap();
        for directive in ["img-src", "media-src", "font-src", "frame-src"] {
            let d = csp
                .split(';')
                .map(str::trim)
                .find(|d| d.starts_with(&format!("{directive} ")))
                .unwrap_or_else(|| panic!("{directive} absent"));
            assert!(
                d.contains(" revel-canvas:") && d.contains("http://revel-canvas.localhost"),
                "{d}"
            );
        }
    }

    #[test]
    fn argument_open() {
        assert_eq!(open_arg(args(&["--open", "/site"])), Some("/site".into()));
        assert_eq!(
            open_arg(args(&["-x", "--open=C:\\site"])),
            Some("C:\\site".into())
        );
        assert_eq!(open_arg(args(&["--open"])), None);
        assert_eq!(open_arg(args(&[])), None);
    }
}
