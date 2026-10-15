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
fn edit_stamp(s: State<AppState>, path: String) -> Res<String> {
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
            app.manage(AppState { core });
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
