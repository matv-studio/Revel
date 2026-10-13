//! Appels au cœur de Revel (commandes Tauri) depuis l'interface.

use js_sys::{Array, Function, Object, Reflect};
use revel_protocol::{
    AppSettings, BuildEvent, DeployEvent, Failure, HostingProfile, ImportResult, RecentSite,
    SiteSummary,
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
    #[derive(Serialize)]
    struct Opts {
        directory: bool,
        multiple: bool,
    }
    #[derive(Serialize)]
    struct A {
        options: Opts,
    }
    call(
        "plugin:dialog|open",
        &A {
            options: Opts {
                directory: true,
                multiple: false,
            },
        },
    )
    .await
}

pub async fn open_site(path: &str) -> Res<SiteSummary> {
    call("open_site", &PathArg { path }).await
}

pub async fn new_site(path: &str, name: &str, lang: &str) -> Res<SiteSummary> {
    #[derive(Serialize)]
    struct A<'a> {
        path: &'a str,
        name: &'a str,
        lang: &'a str,
    }
    call("new_site", &A { path, name, lang }).await
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

pub async fn build(
    path: &str,
    environment: &str,
    images: bool,
    on: impl Fn(BuildEvent) + 'static,
) -> Res<()> {
    #[derive(Serialize)]
    struct A<'a> {
        path: &'a str,
        environment: &'a str,
        images: bool,
    }
    call_with_channel::<_, (), _>(
        "build_site",
        &A {
            path,
            environment,
            images,
        },
        on,
    )
    .await
}

pub async fn preview(path: &str, environment: &str) -> Res<String> {
    #[derive(Serialize)]
    struct A<'a> {
        path: &'a str,
        environment: &'a str,
    }
    call("open_preview", &A { path, environment }).await
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
