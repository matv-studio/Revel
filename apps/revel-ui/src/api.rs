//! Appels au cœur de Revel (commandes Tauri) depuis l'interface.

use js_sys::{Array, Function, Object, Reflect};
use revel_protocol::{
    AppSettings, BuildEvent, DeployEvent, Failure, GithubEvent, HostingProfile, ImportResult,
    RecentSite, SiteSummary,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn raw_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

/// Résultat d'un appel : l'échec est une clé de traduction (voir `Failure`).
pub type Res<T> = Result<T, Failure>;

/// Appel d'une commande ; échec explicite hors de l'application (interface ouverte dans un navigateur).
async fn tauri_invoke(cmd: &str, args: JsValue) -> Res<JsValue> {
    let present = Reflect::get(&web_window(), &"__TAURI__".into())
        .map(|v| !v.is_undefined())
        .unwrap_or(false);
    if !present {
        return Err(Failure::new("error-no-desktop"));
    }
    raw_invoke(cmd, args).await.map_err(failure)
}

/// Échec renvoyé par une commande : `Failure` sérialisé, sinon message brut.
fn failure(e: JsValue) -> Failure {
    serde_wasm_bindgen::from_value::<Failure>(e.clone())
        .unwrap_or_else(|_| Failure::detail(err_text(e)))
}

fn de<R: DeserializeOwned>(v: JsValue) -> Res<R> {
    serde_wasm_bindgen::from_value(v).map_err(|e| Failure::detail(e.to_string()))
}

fn to_js<T: Serialize>(v: &T) -> JsValue {
    v.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .unwrap_or(JsValue::NULL)
}

fn err_text(e: JsValue) -> String {
    e.as_string()
        .or_else(|| js_sys::JSON::stringify(&e).ok().and_then(|s| s.as_string()))
        .unwrap_or_else(|| "?".into())
}

async fn call<A: Serialize, R: DeserializeOwned>(cmd: &str, args: &A) -> Res<R> {
    de(tauri_invoke(cmd, to_js(args)).await?)
}

/// Canal Tauri : chaque message reçu est désérialisé et transmis à `f`.
fn channel<E: DeserializeOwned + 'static>(f: impl Fn(E) + 'static) -> Res<JsValue> {
    let tauri = Reflect::get(&web_window(), &"__TAURI__".into()).map_err(failure)?;
    if tauri.is_undefined() {
        return Err(Failure::new("error-no-desktop"));
    }
    let core = Reflect::get(&tauri, &"core".into()).map_err(failure)?;
    let ctor: Function = Reflect::get(&core, &"Channel".into())
        .map_err(failure)?
        .dyn_into()
        .map_err(failure)?;
    let ch = Reflect::construct(&ctor, &Array::new()).map_err(failure)?;
    let cb = Closure::<dyn Fn(JsValue)>::new(move |msg: JsValue| {
        if let Ok(e) = serde_wasm_bindgen::from_value::<E>(msg) {
            f(e);
        }
    });
    Reflect::set(&ch, &"onmessage".into(), cb.as_ref().unchecked_ref()).map_err(failure)?;
    cb.forget();
    Ok(ch)
}

fn web_window() -> JsValue {
    js_sys::global().into()
}

async fn call_with_channel<A: Serialize, R: DeserializeOwned, E: DeserializeOwned + 'static>(
    cmd: &str,
    args: &A,
    on_event: impl Fn(E) + 'static,
) -> Res<R> {
    let obj: Object = to_js(args).dyn_into().map_err(failure)?;
    Reflect::set(&obj, &"onEvent".into(), &channel(on_event)?).map_err(failure)?;
    de(tauri_invoke(cmd, obj.into()).await?)
}

#[derive(Serialize)]
struct NoArgs {}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PathArg<'a> {
    path: &'a str,
}

pub async fn settings() -> Res<AppSettings> {
    call("get_settings", &NoArgs {}).await
}

pub async fn set_settings(settings: &AppSettings) -> Res<()> {
    #[derive(Serialize)]
    struct A<'a> {
        settings: &'a AppSettings,
    }
    call("set_settings", &A { settings }).await
}

pub async fn recent_sites() -> Res<Vec<RecentSite>> {
    call("recent_sites", &NoArgs {}).await
}

pub async fn forget_site(path: &str) -> Res<()> {
    call("forget_site", &PathArg { path }).await
}

/// Site mis à la corbeille du système et retiré de la liste.
pub async fn delete_site(path: &str) -> Res<()> {
    call("delete_site", &PathArg { path }).await
}

/// Sélection d'un dossier (fenêtre du système). `None` si annulé.
pub async fn pick_folder() -> Res<Option<String>> {
    pick_folder_at(None).await
}

/// Sélection d'un dossier, en partant de `start` s'il est donné.
pub async fn pick_folder_at(start: Option<&str>) -> Res<Option<String>> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Opts<'a> {
        directory: bool,
        multiple: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        default_path: Option<&'a str>,
    }
    #[derive(Serialize)]
    struct A<'a> {
        options: Opts<'a>,
    }
    call(
        "plugin:dialog|open",
        &A {
            options: Opts {
                directory: true,
                multiple: false,
                default_path: start,
            },
        },
    )
    .await
}

/// Dossier proposé pour les nouveaux sites (`Documents/Revel`).
pub async fn default_sites_dir() -> Res<String> {
    call("default_sites_dir", &NoArgs {}).await
}

/// Dossier qui sera créé pour un nouveau site.
pub async fn site_folder(parent: &str, name: &str) -> Res<String> {
    #[derive(Serialize)]
    struct A<'a> {
        parent: &'a str,
        name: &'a str,
    }
    call("site_folder", &A { parent, name }).await
}

/// Crée un site dans un nouveau dossier de `parent`.
pub async fn create_site(
    parent: &str,
    name: &str,
    lang: &str,
    template: revel_protocol::SiteTemplate,
    light: bool,
) -> Res<SiteSummary> {
    #[derive(Serialize)]
    struct A<'a> {
        parent: &'a str,
        name: &'a str,
        lang: &'a str,
        template: revel_protocol::SiteTemplate,
        light: bool,
    }
    call(
        "create_site",
        &A {
            parent,
            name,
            lang,
            template,
            light,
        },
    )
    .await
}

/// Collections de contenus du site et leurs entrées.
pub async fn cms_collections(path: &str) -> Res<Vec<revel_protocol::CmsCollection>> {
    call("cms_collections", &PathArg { path }).await
}

pub async fn cms_folders(path: &str) -> Res<Vec<revel_protocol::CmsFolder>> {
    call("cms_folders", &PathArg { path }).await
}

/// Version de Revel, version du format des sites.
pub async fn app_info() -> Res<(String, u32)> {
    call("app_info", &serde_json::json!({})).await
}

/// Version plus récente publiée (6.6).
pub async fn update_check() -> Res<revel_protocol::UpdateCheck> {
    call("update_check", &serde_json::json!({})).await
}

/// Installe la version proposée et relance Revel.
pub async fn update_install() -> Res<()> {
    call("update_install", &serde_json::json!({})).await
}

pub async fn open_site(path: &str) -> Res<SiteSummary> {
    call("open_site", &PathArg { path }).await
}

/// Nature d'un dossier avant import (7.1, 7.2).
pub async fn probe_site(path: &str) -> Res<revel_protocol::ImportProbe> {
    call("probe_site", &PathArg { path }).await
}

/// Importe le site d'un dossier (Eleventy ou HTML statique).
pub async fn import_site(path: &str, overwrite: bool, lang: &str) -> Res<ImportResult> {
    #[derive(Serialize)]
    struct A<'a> {
        path: &'a str,
        overwrite: bool,
        lang: &'a str,
    }
    call(
        "import_site",
        &A {
            path,
            overwrite,
            lang,
        },
    )
    .await
}

/// Copie un dépôt Git dans un nouveau dossier de `parent`.
pub async fn git_clone(
    url: &str,
    parent: &str,
    on: impl Fn(revel_protocol::CloneEvent) + 'static,
) -> Res<revel_protocol::ClonedRepo> {
    call_with_channel(
        "git_clone",
        &serde_json::json!({ "url": url, "parent": parent }),
        on,
    )
    .await
}

pub async fn git_clone_cancel() -> Res<()> {
    call("git_clone_cancel", &NoArgs {}).await
}

/// Retire une copie de dépôt pas importée.
pub async fn discard_clone(path: &str) -> Res<()> {
    call("discard_clone", &PathArg { path }).await
}

/// Exporte le site dans un nouveau dossier de `parent` ; renvoie ce dossier.
pub async fn export(
    path: &str,
    environment: &str,
    images: bool,
    parent: &str,
    on: impl Fn(BuildEvent) + 'static,
) -> Res<String> {
    #[derive(Serialize)]
    struct A<'a> {
        path: &'a str,
        environment: &'a str,
        images: bool,
        parent: &'a str,
    }
    call_with_channel::<_, String, _>(
        "export_site",
        &A {
            path,
            environment,
            images,
            parent,
        },
        on,
    )
    .await
}

/// Aperçu du site (pages rendues à la demande), à partir de `page`.
pub async fn preview(path: &str, environment: &str, page: Option<&str>) -> Res<String> {
    #[derive(Serialize)]
    struct A<'a> {
        path: &'a str,
        environment: &'a str,
        page: Option<&'a str>,
    }
    call(
        "open_preview",
        &A {
            path,
            environment,
            page,
        },
    )
    .await
}

/// Adresse de l'aperçu du site, pour l'afficher dans l'espace de travail.
pub async fn preview_url(path: &str, environment: &str) -> Res<String> {
    #[derive(Serialize)]
    struct A<'a> {
        path: &'a str,
        environment: &'a str,
    }
    call("preview_url", &A { path, environment }).await
}

/// Montre un dossier dans l'explorateur du système.
pub async fn reveal(path: &str) -> Res<()> {
    call("reveal_path", &PathArg { path }).await
}

pub async fn profiles(site: &str) -> Res<Vec<(HostingProfile, bool)>> {
    #[derive(Serialize)]
    struct A<'a> {
        site: &'a str,
    }
    call("list_profiles", &A { site }).await
}

pub async fn save_profile(
    site: &str,
    profile: &HostingProfile,
    password: Option<String>,
) -> Res<HostingProfile> {
    #[derive(Serialize)]
    struct A<'a> {
        site: &'a str,
        profile: &'a HostingProfile,
        password: Option<String>,
    }
    call(
        "save_profile",
        &A {
            site,
            profile,
            password,
        },
    )
    .await
}

pub async fn delete_profile(site: &str, id: &str) -> Res<()> {
    #[derive(Serialize)]
    struct A<'a> {
        site: &'a str,
        id: &'a str,
    }
    call("delete_profile", &A { site, id }).await
}

/// Publication (ou vérification, `dry_run`) ; `skip_checks` : « Publier quand même » (5.4).
pub async fn deploy(
    site: &str,
    profile_id: &str,
    dry_run: bool,
    skip_checks: bool,
    on: impl Fn(DeployEvent) + 'static,
) -> Res<()> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A<'a> {
        site: &'a str,
        profile_id: &'a str,
        dry_run: bool,
        skip_checks: bool,
    }
    call_with_channel::<_, (), _>(
        "deploy",
        &A {
            site,
            profile_id,
            dry_run,
            skip_checks,
        },
        on,
    )
    .await
}

/// Connexion d'essai à un hébergement SFTP avant de l'enregistrer (5.3) ; `password` : saisi.
pub async fn hosting_probe(
    site: &str,
    profile: &HostingProfile,
    password: Option<String>,
) -> Res<revel_protocol::HostProbe> {
    #[derive(Serialize)]
    struct A<'a> {
        site: &'a str,
        profile: &'a HostingProfile,
        password: Option<String>,
    }
    call(
        "hosting_probe",
        &A {
            site,
            profile,
            password,
        },
    )
    .await
}

/// Destinations de publication du site : dernière publication, pages modifiées depuis (5.1).
pub async fn publish_state(path: &str) -> Res<Vec<revel_protocol::PublishDestination>> {
    call("publish_state", &PathArg { path }).await
}

/// Republie la version précédente d'un hébergement (5.5).
pub async fn rollback(site: &str, profile_id: &str, on: impl Fn(DeployEvent) + 'static) -> Res<()> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A<'a> {
        site: &'a str,
        profile_id: &'a str,
    }
    call_with_channel::<_, (), _>("rollback", &A { site, profile_id }, on).await
}

/// Vérifie qu'un domaine mène au site publié sur un hébergement (5.7).
pub async fn domain_check(
    site: &str,
    profile_id: &str,
    domain: &str,
) -> Res<revel_protocol::DomainStatus> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A<'a> {
        site: &'a str,
        profile_id: &'a str,
        domain: &'a str,
    }
    call(
        "domain_check",
        &A {
            site,
            profile_id,
            domain,
        },
    )
    .await
}

/// Compte GitHub connecté (identifiant).
pub async fn github_account() -> Res<Option<String>> {
    call("github_account", &NoArgs {}).await
}

/// Connexion à GitHub (autorisation par code) ; renvoie l'identifiant du compte.
pub async fn github_connect(on: impl Fn(GithubEvent) + 'static) -> Res<String> {
    call_with_channel("github_connect", &NoArgs {}, on).await
}

/// Versions du site (3.10) ; `None` : versions pas encore activées.
/// Réglages lisibles (3.12) : valeurs et libellés des chemins de données, contenus proposés.
pub async fn readable_data(
    path: &str,
    paths: &[Vec<String>],
    collections: &[String],
) -> Res<revel_protocol::ReadableData> {
    call(
        "readable_data",
        &serde_json::json!({ "path": path, "paths": paths, "collections": collections }),
    )
    .await
}

pub async fn versions_status(path: &str) -> Res<Option<revel_protocol::VersionsStatus>> {
    call("versions_status", &serde_json::json!({ "path": path })).await
}

pub async fn versions_init(path: &str, message: &str) -> Res<revel_protocol::VersionInfo> {
    call(
        "versions_init",
        &serde_json::json!({ "path": path, "message": message }),
    )
    .await
}

pub async fn versions_list(path: &str, limit: usize) -> Res<Vec<revel_protocol::VersionInfo>> {
    call(
        "versions_list",
        &serde_json::json!({ "path": path, "limit": limit }),
    )
    .await
}

pub async fn versions_save(path: &str, message: &str) -> Res<Option<revel_protocol::VersionInfo>> {
    call(
        "versions_save",
        &serde_json::json!({ "path": path, "message": message }),
    )
    .await
}

/// Fichiers changés : par une version (`of_version`), ou depuis une version (ou la dernière).
pub async fn versions_changes(
    path: &str,
    id: Option<&str>,
    of_version: bool,
) -> Res<Vec<revel_protocol::FileChange>> {
    call(
        "versions_changes",
        &serde_json::json!({ "path": path, "id": id, "ofVersion": of_version }),
    )
    .await
}

pub async fn versions_diff(
    path: &str,
    id: Option<&str>,
    of_version: bool,
    file: &str,
) -> Res<Option<Vec<revel_protocol::DiffLine>>> {
    call(
        "versions_diff",
        &serde_json::json!({ "path": path, "id": id, "ofVersion": of_version, "file": file }),
    )
    .await
}

pub async fn versions_restore(
    path: &str,
    id: &str,
    before: &str,
    message: &str,
) -> Res<Option<revel_protocol::VersionInfo>> {
    call(
        "versions_restore",
        &serde_json::json!({ "path": path, "id": id, "before": before, "message": message }),
    )
    .await
}

pub async fn versions_sync(
    path: &str,
    message: &str,
    merge: &str,
) -> Res<revel_protocol::SyncInfo> {
    call(
        "versions_sync",
        &serde_json::json!({ "path": path, "message": message, "merge": merge }),
    )
    .await
}

pub async fn versions_resolve(
    path: &str,
    keep_ours: bool,
    message: &str,
) -> Res<revel_protocol::SyncInfo> {
    call(
        "versions_resolve",
        &serde_json::json!({ "path": path, "keepOurs": keep_ours, "message": message }),
    )
    .await
}

pub async fn versions_set_remote(path: &str, url: &str) -> Res<()> {
    call(
        "versions_set_remote",
        &serde_json::json!({ "path": path, "url": url }),
    )
    .await
}

/// Polices du site.
pub async fn fonts(path: &str) -> Res<Vec<revel_protocol::FontFamily>> {
    call("fonts", &serde_json::json!({ "path": path })).await
}

/// Catalogue de Google Fonts.
pub async fn fonts_google() -> Res<Vec<revel_protocol::GoogleFont>> {
    call("fonts_google", &NoArgs {}).await
}

pub async fn fonts_add_google(
    path: &str,
    family: &str,
    variants: &[String],
) -> Res<revel_edit::EditState> {
    call(
        "fonts_add_google",
        &serde_json::json!({ "path": path, "family": family, "variants": variants }),
    )
    .await
}

pub async fn fonts_add_files(
    path: &str,
    family: &str,
    files: &[revel_protocol::FontUpload],
) -> Res<revel_edit::EditState> {
    call(
        "fonts_add_files",
        &serde_json::json!({ "path": path, "family": family, "files": files }),
    )
    .await
}

pub async fn fonts_remove(path: &str, family: &str) -> Res<revel_edit::EditState> {
    call(
        "fonts_remove",
        &serde_json::json!({ "path": path, "family": family }),
    )
    .await
}

/// Langues de traduction automatique (livrées, téléchargées, à télécharger).
pub async fn translate_packs() -> Res<Vec<revel_protocol::TranslatePack>> {
    call("translate_packs", &NoArgs {}).await
}

/// Télécharge une langue de traduction ; `on` reçoit l'avancement.
pub async fn translate_install(
    code: &str,
    on: impl Fn(revel_protocol::TranslateEvent) + 'static,
) -> Res<()> {
    call_with_channel(
        "translate_install",
        &serde_json::json!({ "code": code }),
        on,
    )
    .await
}

pub async fn translate_install_cancel() -> Res<()> {
    call("translate_install_cancel", &NoArgs {}).await
}

/// Retire une langue de traduction téléchargée.
pub async fn translate_remove(code: &str) -> Res<()> {
    call("translate_remove", &serde_json::json!({ "code": code })).await
}

pub async fn github_cancel() -> Res<()> {
    call("github_cancel", &NoArgs {}).await
}

pub async fn github_disconnect() -> Res<()> {
    call("github_disconnect", &NoArgs {}).await
}

/// Ouvre une adresse dans le navigateur.
pub async fn open_external(url: &str) -> Res<()> {
    #[derive(Serialize)]
    struct A<'a> {
        url: &'a str,
    }
    call("open_external", &A { url }).await
}

/// Copie un texte dans le presse-papiers (sans effet si le système le refuse).
pub fn copy_text(text: &str) {
    let nav = Reflect::get(&web_window(), &"navigator".into()).ok();
    let clip = nav.and_then(|n| Reflect::get(&n, &"clipboard".into()).ok());
    if let Some(c) = clip.filter(|c| !c.is_undefined())
        && let Ok(f) = Reflect::get(&c, &"writeText".into())
        && let Ok(f) = f.dyn_into::<Function>()
    {
        let _ = f.call1(&c, &JsValue::from_str(text));
    }
}

// ------------------------------------------------------------------ éditeur (étape 2)

/// Ouvre un site pour l'édition : document et état de l'historique.
pub async fn edit_open(path: &str) -> Res<revel_edit::Opened> {
    call("edit_open", &PathArg { path }).await
}

/// Applique une commande ; `coalesce` : saisie continue fusionnée dans l'historique.
pub async fn edit_apply(
    path: &str,
    command: &revel_edit::Command,
    coalesce: bool,
) -> Res<revel_edit::EditState> {
    #[derive(Serialize)]
    struct A<'a> {
        path: &'a str,
        command: &'a revel_edit::Command,
        coalesce: bool,
    }
    call(
        "edit_apply",
        &A {
            path,
            command,
            coalesce,
        },
    )
    .await
}

/// Termine une saisie continue (la commande suivante ouvre une nouvelle étape).
#[allow(dead_code)] // panneaux des étapes 2.4 à 2.7
pub async fn edit_seal(path: &str) -> Res<()> {
    call("edit_seal", &PathArg { path }).await
}

pub async fn edit_undo(path: &str) -> Res<revel_edit::EditState> {
    call("edit_undo", &PathArg { path }).await
}

pub async fn edit_redo(path: &str) -> Res<revel_edit::EditState> {
    call("edit_redo", &PathArg { path }).await
}

/// Importe un média dans le dossier des médias du site.
pub async fn edit_import_media(
    path: &str,
    name: &str,
    data: &[u8],
) -> Res<revel_protocol::MediaFile> {
    use base64::Engine;
    let data = base64::engine::general_purpose::STANDARD.encode(data);
    call(
        "edit_import_media",
        &serde_json::json!({ "path": path, "name": name, "data": data }),
    )
    .await
}

/// Remplace le contenu d'un média (même adresse).
pub async fn edit_replace_media(
    path: &str,
    url: &str,
    name: &str,
    data: &[u8],
) -> Res<revel_protocol::MediaFile> {
    use base64::Engine;
    let data = base64::engine::general_purpose::STANDARD.encode(data);
    call(
        "edit_replace_media",
        &serde_json::json!({ "path": path, "url": url, "name": name, "data": data }),
    )
    .await
}

/// Commande d'une opération faite par lots (traduction automatique) : les commandes d'un même
/// `group` envoyées à la suite forment une seule étape d'annulation.
pub async fn edit_apply_group(
    path: &str,
    command: &revel_edit::Command,
    group: &str,
) -> Res<revel_edit::EditState> {
    #[derive(Serialize)]
    struct A<'a> {
        path: &'a str,
        command: &'a revel_edit::Command,
        coalesce: bool,
        group: &'a str,
    }
    call(
        "edit_apply",
        &A {
            path,
            command,
            coalesce: false,
            group,
        },
    )
    .await
}

/// Traduction automatique possible d'une langue vers une autre (modèles livrés, D-36).
pub async fn translate_can(from: &str, to: &str) -> bool {
    call(
        "translate_can",
        &serde_json::json!({ "from": from, "to": to }),
    )
    .await
    .unwrap_or(false)
}

/// Traduction automatique hors ligne d'un lot de textes.
pub async fn translate(
    from: &str,
    to: &str,
    items: &[revel_protocol::TranslateItem],
    keep: &[String],
) -> Res<Vec<String>> {
    call(
        "translate",
        &serde_json::json!({ "from": from, "to": to, "items": items, "keep": keep }),
    )
    .await
}

/// Textes des fichiers de données à traduire dans une autre langue (vue Localisation).
pub async fn edit_locale_data(path: &str, code: &str) -> Res<Vec<revel_edit::locale::DataText>> {
    // Chemins mêlant clés et positions (énumération sans étiquette) : relus depuis le JSON, les
    // nombres JavaScript n'étant pas reconnus comme positions par `serde_wasm_bindgen`.
    let v = tauri_invoke(
        "edit_locale_data",
        to_js(&serde_json::json!({ "path": path, "code": code })),
    )
    .await?;
    let text = js_sys::JSON::stringify(&v)
        .ok()
        .and_then(|s| s.as_string())
        .unwrap_or_default();
    serde_json::from_str(&text).map_err(|e| Failure::detail(e.to_string()))
}

/// Médias du site ouvert dans l'éditeur.
pub async fn edit_media(path: &str) -> Res<Vec<revel_protocol::MediaFile>> {
    call("edit_media", &PathArg { path }).await
}

/// Pages HTML du site (pages fixes et contenus).
pub async fn edit_pages(path: &str) -> Res<Vec<revel_protocol::EditorPageInfo>> {
    call("edit_pages", &PathArg { path }).await
}

/// Affiche le site dans le canevas ; renvoie l'adresse de base du protocole.
pub async fn canvas_open(path: &str, scripts: bool) -> Res<String> {
    #[derive(Serialize)]
    struct A<'a> {
        path: &'a str,
        scripts: bool,
    }
    call("canvas_open", &A { path, scripts }).await
}

/// Empreinte du site ouvert (révision de la session, fichiers modifiés hors de Revel).
pub async fn edit_stamp(path: &str) -> Res<revel_edit::EditStamp> {
    call("edit_stamp", &PathArg { path }).await
}

/// Serveur des assistants IA : réglages et état.
pub async fn mcp_status() -> Res<revel_protocol::McpStatus> {
    call("mcp_status", &NoArgs {}).await
}

#[derive(Serialize)]
struct McpSetArgs {
    enabled: bool,
    port: u16,
}

/// Active ou désactive le serveur des assistants, change son port.
pub async fn mcp_set(enabled: bool, port: u16) -> Res<revel_protocol::McpStatus> {
    call("mcp_set", &McpSetArgs { enabled, port }).await
}

/// Nouvelle clé d'accès du serveur des assistants.
pub async fn mcp_new_token() -> Res<revel_protocol::McpStatus> {
    call("mcp_new_token", &NoArgs {}).await
}

/// Dernières modifications faites par les assistants.
pub async fn mcp_activity() -> Res<Vec<revel_protocol::McpActivity>> {
    call("mcp_activity", &NoArgs {}).await
}

/// Site à ouvrir dans l'éditeur au démarrage (argument `--open` ou variable `REVEL_OPEN`).
pub async fn startup_site() -> Res<Option<String>> {
    call("startup_site", &NoArgs {}).await
}
