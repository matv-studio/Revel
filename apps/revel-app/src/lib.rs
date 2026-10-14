//! Coque applicative : relie l'interface Leptos (`apps/revel-ui`) aux services de
//! `revel-core` par des commandes Tauri. Aucune logique métier ici.

use revel_core::{Core, CoreError, preview::Preview};
use revel_protocol::{
    AppSettings, BuildEvent, DeployEvent, Failure, GithubEvent, HostingProfile, ImportResult,
    RecentSite, SiteSummary,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{Manager, State, WebviewUrl, WebviewWindowBuilder};

struct AppState {
    core: Arc<Core>,
    preview: Mutex<Option<Preview>>,
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

/// Démarre (ou redémarre) l'aperçu du dernier site généré et l'ouvre dans une fenêtre.
#[tauri::command]
async fn open_preview(
    app: tauri::AppHandle,
    s: State<'_, AppState>,
    path: String,
    environment: String,
) -> Res<String> {
    let dir = revel_core::out_dir(&PathBuf::from(&path), &environment);
    if !dir.join("index.html").is_file() {
        return Err(CoreError::NotBuilt.failure());
    }
    let p = revel_core::preview::start(dir)
        .await
        .map_err(|e| CoreError::Io(e).failure())?;
    let url = p.url.clone();
    if let Ok(mut slot) = s.preview.lock() {
        *slot = Some(p);
    }
    let parsed: tauri::Url = url.parse().map_err(sys)?;
    if let Some(w) = app.get_webview_window("preview") {
        w.navigate(parsed).map_err(sys)?;
        let _ = w.set_focus();
    } else {
        WebviewWindowBuilder::new(&app, "preview", WebviewUrl::External(parsed))
            .title("Revel — aperçu")
            .inner_size(1280.0, 860.0)
            .build()
            .map_err(sys)?;
    }
    Ok(url)
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
) -> Res<revel_edit::EditState> {
    s.core
        .edit_apply(&PathBuf::from(path), command, coalesce)
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

/// Empreinte des fichiers du site (rechargement après une modification hors de Revel).
#[tauri::command]
fn edit_stamp(s: State<AppState>, path: String) -> Res<String> {
    s.core.edit_stamp(std::path::Path::new(&path)).map_err(err)
}

/// Réponse du protocole `revel-canvas` (canevas de l'éditeur, sans port réseau).
fn canvas_protocol(
    app: &tauri::AppHandle,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let core = app.state::<AppState>().core.clone();
    let path = request.uri().path().to_string();
    std::thread::spawn(move || {
        let r = core.canvas_respond(&path);
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
            canvas_protocol(ctx.app_handle(), request, responder);
        })
        .setup(|app| {
            let dir = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("revel"));
            let secrets: Box<dyn revel_core::SecretStore> = Box::new(revel_core::secrets::Keyring);
            app.manage(AppState {
                core: Arc::new(Core::new(&dir, secrets)),
                preview: Mutex::new(None),
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
            import_eleventy,
            build_site,
            open_preview,
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
