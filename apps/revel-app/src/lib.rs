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
    /// Application (fenêtre signalée quand un assistant en ligne demande l'accès).
    handle: tauri::AppHandle,
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
    /// Accès à distance (adresse publique réglée) : autorisations OAuth.
    oauth: Option<Arc<revel_mcp::oauth::OAuth>>,
}

/// Identifiant de l'application (`tauri.conf.json`) : dossier des réglages, lu aussi par le relais.
const IDENTIFIER: &str = "app.revel-studio.desktop";

/// Autorisations de l'accès à distance pour l'adresse publique réglée (reprises si l'adresse n'a
/// pas changé) ; une nouvelle demande fait signaler la fenêtre de Revel.
fn oauth_for(
    handle: &tauri::AppHandle,
    config_dir: &std::path::Path,
    c: &revel_mcp::config::McpConfig,
    previous: Option<Arc<revel_mcp::oauth::OAuth>>,
) -> Option<Arc<revel_mcp::oauth::OAuth>> {
    let base = c.public_url.as_deref()?;
    if let Some(p) = previous.filter(|p| p.base() == base) {
        return Some(p);
    }
    let h = handle.clone();
    let notify: Box<dyn Fn() + Send + Sync> = Box::new(move || {
        if let Some(w) = h.get_webview_window("main") {
            let _ = w.request_user_attention(Some(tauri::UserAttentionType::Informational));
        }
    });
    Some(Arc::new(revel_mcp::oauth::OAuth::new(
        config_dir,
        base,
        Some(notify),
    )))
}

/// Démarre le serveur des assistants sur le cœur de l'application.
async fn start_mcp(
    core: &Arc<Core>,
    activity: &revel_mcp::activity::ActivityLog,
    c: &revel_mcp::config::McpConfig,
    oauth: Option<Arc<revel_mcp::oauth::OAuth>>,
) -> Mcp {
    let inv: Arc<dyn revel_mcp::Invoke> = Arc::new(revel_mcp::activity::Logged {
        inner: Arc::new(revel_mcp::direct::CoreInvoke(core.clone())),
        log: activity.clone(),
    });
    match revel_mcp::http::start_with(inv, c.token.clone(), c.port, oauth.clone()).await {
        Ok(r) => Mcp {
            running: Some(r),
            error: None,
            oauth,
        },
        Err(e) => Mcp {
            running: None,
            error: Some(e.to_string()),
            oauth,
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
        program: std::env::current_exe()
            .ok()
            .map(|p| p.display().to_string()),
        public_url: c.public_url,
    }
}

fn oauth_of(s: &AppState) -> Option<Arc<revel_mcp::oauth::OAuth>> {
    s.mcp
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .oauth
        .clone()
}

/// Adresse publique (tunnel) des assistants en ligne ; vide : accès à distance coupé.
#[tauri::command]
async fn mcp_set_public(
    s: State<'_, AppState>,
    url: Option<String>,
) -> Res<revel_protocol::McpStatus> {
    let mut c = revel_mcp::config::McpConfig::load(&s.config_dir);
    c.public_url = match url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
        None => None,
        Some(u) => Some(
            revel_mcp::oauth::normalize_base(u)
                .ok_or_else(|| Failure::new("error-mcp-public-url").arg("detail", u))?,
        ),
    };
    c.save(&s.config_dir).map_err(sys)?;
    restart_mcp(&s, &c).await;
    Ok(mcp_status_of(&s))
}

/// Demandes d'accès des assistants en ligne, en attente de décision.
#[tauri::command]
fn mcp_pending(s: State<AppState>) -> Vec<revel_protocol::McpAuthRequest> {
    oauth_of(&s).map(|o| o.pending()).unwrap_or_default()
}

/// Accorde ou refuse une demande d'accès.
#[tauri::command]
fn mcp_decide(s: State<AppState>, id: String, allow: bool) -> bool {
    oauth_of(&s).is_some_and(|o| o.decide(&id, allow))
}

/// Assistants en ligne autorisés.
#[tauri::command]
fn mcp_connections(s: State<AppState>) -> Vec<revel_protocol::McpConnection> {
    oauth_of(&s).map(|o| o.connections()).unwrap_or_default()
}

/// Retire l'accès d'un assistant en ligne.
#[tauri::command]
fn mcp_revoke(s: State<AppState>, id: String) {
    if let Some(o) = oauth_of(&s) {
        o.revoke(&id);
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
    let old = std::mem::take(&mut *s.mcp.lock().unwrap_or_else(|e| e.into_inner()));
    if let Some(r) = old.running {
        r.shutdown().await;
    }
    let oauth = oauth_for(&s.handle, &s.config_dir, c, old.oauth);
    let m = if c.enabled {
        start_mcp(&s.core, &s.activity, c, oauth).await
    } else {
        Mcp {
            oauth,
            ..Default::default()
        }
    };
    *s.mcp.lock().unwrap_or_else(|e| e.into_inner()) = m;
}

/// Relais stdio des assistants (`revel --mcp`) : code de sortie du processus.
pub fn mcp_relay() -> i32 {
    let Some(dir) = revel_mcp::config::app_config_dir(IDENTIFIER) else {
        eprintln!("revel --mcp : dossier des réglages introuvable");
        return 2;
    };
    let exe = std::env::current_exe().ok();
    match tauri::async_runtime::block_on(revel_mcp::relay::run(dir, exe)) {
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

/// Dossier proposé pour les nouveaux sites : `Documents/Revel` (dossier personnel à défaut).
#[tauri::command]
fn default_sites_dir(app: tauri::AppHandle) -> String {
    app.path()
        .document_dir()
        .or_else(|_| app.path().home_dir())
        .map(|d| d.join("Revel").display().to_string())
        .unwrap_or_default()
}

/// Dossier qui sera créé pour un nouveau site nommé `name` dans `parent`.
#[tauri::command]
fn site_folder(parent: String, name: String) -> String {
    revel_core::site_folder(&PathBuf::from(parent), &name)
        .display()
        .to_string()
}

/// Crée un site dans un nouveau dossier de `parent` (voir `site_folder`).
#[tauri::command]
fn create_site(
    s: State<AppState>,
    parent: String,
    name: String,
    lang: String,
    template: Option<revel_protocol::SiteTemplate>,
) -> Res<SiteSummary> {
    s.core
        .create_site_from(
            &PathBuf::from(parent),
            &name,
            &lang,
            template.unwrap_or_default(),
        )
        .map_err(err)
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
        WebviewWindowBuilder::new(&app, "preview", target)
            .title("Revel — aperçu")
            .inner_size(1280.0, 860.0)
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
    on_event: Channel<DeployEvent>,
) -> Res<()> {
    let core = s.core.clone();
    let send = move |e: DeployEvent| {
        let _ = on_event.send(e);
    };
    core.deploy(&PathBuf::from(site), &profile_id, dry_run, false, &send)
        .await
        .map(|_| ())
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
    let path = request.uri().path().to_string();
    std::thread::spawn(move || {
        let r = if preview {
            core.preview_respond(&path)
        } else {
            core.canvas_respond(&path)
        };
        let response = tauri::http::Response::builder()
            .status(r.status)
            .header(tauri::http::header::CONTENT_TYPE, r.content_type)
            .header(tauri::http::header::CACHE_CONTROL, "no-store")
            .body(r.body)
            .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()));
        responder.respond(response);
    });
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
            // Modèles de traduction livrés dans les ressources de l'installateur (D-36).
            if let Ok(models) = app
                .path()
                .resolve("models", tauri::path::BaseDirectory::Resource)
            {
                core.set_models_dir(&models);
            }
            // Serveur des assistants IA (étape 4), désactivé par défaut.
            let mcp_config = revel_mcp::config::McpConfig::load(&dir);
            let activity = revel_mcp::activity::ActivityLog::default();
            let oauth = oauth_for(app.handle(), &dir, &mcp_config, None);
            let mcp = if mcp_config.enabled {
                tauri::async_runtime::block_on(start_mcp(&core, &activity, &mcp_config, oauth))
            } else {
                Mcp {
                    oauth,
                    ..Default::default()
                }
            };
            app.manage(AppState {
                core,
                handle: app.handle().clone(),
                config_dir: dir,
                mcp: std::sync::Mutex::new(mcp),
                activity,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            get_settings,
            set_settings,
            recent_sites,
            forget_site,
            has_site,
            open_site,
            new_site,
            default_sites_dir,
            site_folder,
            create_site,
            cms_collections,
            import_eleventy,
            build_site,
            export_site,
            reveal_path,
            open_preview,
            preview_url,
            list_profiles,
            save_profile,
            delete_profile,
            deploy,
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
            mcp_set_public,
            mcp_pending,
            mcp_decide,
            mcp_connections,
            mcp_revoke,
            versions_status,
            versions_init,
            versions_list,
            versions_save,
            versions_changes,
            versions_diff,
            versions_restore,
            versions_sync,
            versions_resolve,
            versions_set_remote,
            fonts_google,
            fonts_add_google,
            fonts_add_files,
            fonts_remove,
            translate_install,
            translate_install_cancel,
            translate_remove,
            edit_import_media,
            edit_replace_media,
            canvas_open,
            startup_site
        ])
        .run(tauri::generate_context!())
        .expect("erreur au démarrage de Revel");
}

#[cfg(test)]
mod tests {
    use super::open_arg;

    fn args(list: &[&str]) -> impl Iterator<Item = String> {
        list.iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .into_iter()
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
