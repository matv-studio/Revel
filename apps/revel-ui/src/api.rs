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
pub async fn create_site(parent: &str, name: &str, lang: &str) -> Res<SiteSummary> {
    #[derive(Serialize)]
    struct A<'a> {
        parent: &'a str,
        name: &'a str,
        lang: &'a str,
    }
    call("create_site", &A { parent, name, lang }).await
}

/// Collections de contenus du site et leurs entrées.
pub async fn cms_collections(path: &str) -> Res<Vec<revel_protocol::CmsCollection>> {
    call("cms_collections", &PathArg { path }).await
}

pub async fn open_site(path: &str) -> Res<SiteSummary> {
    call("open_site", &PathArg { path }).await
}

pub async fn import_eleventy(path: &str, overwrite: bool) -> Res<ImportResult> {
    #[derive(Serialize)]
    struct A<'a> {
        path: &'a str,
        overwrite: bool,
    }
    call("import_eleventy", &A { path, overwrite }).await
}

pub async fn has_site(path: &str) -> Res<bool> {
    call("has_site", &PathArg { path }).await
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

pub async fn deploy(
    site: &str,
    profile_id: &str,
    dry_run: bool,
    on: impl Fn(DeployEvent) + 'static,
) -> Res<()> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A<'a> {
        site: &'a str,
        profile_id: &'a str,
        dry_run: bool,
    }
    call_with_channel::<_, (), _>(
        "deploy",
        &A {
            site,
            profile_id,
            dry_run,
        },
        on,
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

/// Empreinte des fichiers du site (modifications hors de Revel).
pub async fn edit_stamp(path: &str) -> Res<String> {
    call("edit_stamp", &PathArg { path }).await
}

/// Site à ouvrir dans l'éditeur au démarrage (argument `--open` ou variable `REVEL_OPEN`).
pub async fn startup_site() -> Res<Option<String>> {
    call("startup_site", &NoArgs {}).await
}
