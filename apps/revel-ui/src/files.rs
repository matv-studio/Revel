//! Fichiers du système choisis (sélecteur) ou glissés dans l'interface, lus en mémoire pour être
//! importés dans le site.

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// Contenu d'un fichier.
pub async fn read(f: &web_sys::File) -> Result<Vec<u8>, String> {
    let buf = wasm_bindgen_futures::JsFuture::from(f.array_buffer())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

/// Fichiers d'une liste (sélecteur, dépôt).
pub fn list(l: Option<web_sys::FileList>) -> Vec<web_sys::File> {
    let Some(l) = l else { return Vec::new() };
    (0..l.length()).filter_map(|i| l.item(i)).collect()
}

/// Le glisser vient du système (fichiers) et pas de l'interface.
pub fn has_files(e: &web_sys::DragEvent) -> bool {
    e.data_transfer().is_some_and(|dt| {
        let t = dt.types();
        (0..t.length()).any(|i| t.get(i).as_string().as_deref() == Some("Files"))
    })
}

/// Ouvre le sélecteur de fichiers du système (`accept` : types acceptés).
pub fn pick(accept: &str, multiple: bool, on: impl Fn(Vec<web_sys::File>) + 'static) {
    let Ok(el) = document().create_element("input") else {
        return;
    };
    let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() else {
        return;
    };
    input.set_type("file");
    input.set_accept(accept);
    input.set_multiple(multiple);
    let i2 = input.clone();
    let cb = Closure::<dyn Fn()>::new(move || on(list(i2.files())));
    input.set_onchange(Some(cb.as_ref().unchecked_ref()));
    cb.forget();
    input.click();
}

/// Types de fichiers acceptés pour les médias.
pub const MEDIA_ACCEPT: &str = "image/*,video/*,audio/*,.pdf,.doc,.docx,.xls,.xlsx,.ppt,.pptx,.odt,.ods,.odp,.csv,.woff,.woff2,.ttf,.otf,.lottie,.json,.zip";

/// Dossiers d'un dépôt de fichiers du système (noms) : entrées du glisser-déposer qui sont des
/// dossiers (`webkitGetAsEntry`, aussi lu par WebView2, WebKit et Chromium).
pub fn dropped_folders(dt: &web_sys::DataTransfer) -> Vec<String> {
    use js_sys::{Function, Reflect};
    let items = dt.items();
    (0..items.length())
        .filter_map(|i| items.get(i))
        .filter_map(|it| {
            let f = Reflect::get(&it, &"webkitGetAsEntry".into())
                .ok()?
                .dyn_into::<Function>()
                .ok()?;
            let entry = f.call0(&it).ok()?;
            let dir = Reflect::get(&entry, &"isDirectory".into())
                .ok()?
                .as_bool()?;
            dir.then(|| Reflect::get(&entry, &"name".into()).ok()?.as_string())
                .flatten()
        })
        .collect()
}

/// Sous Windows (WebView2) : fichiers déposés transmis à l'application, qui en lit les chemins
/// (`dropped_paths`, D-122). À appeler pendant l'événement `drop`. `false` ailleurs.
pub fn post_dropped(dt: &web_sys::DataTransfer) -> bool {
    use js_sys::{Function, Object, Reflect};
    let w = window();
    let Some(post) = Reflect::get(&w, &"chrome".into())
        .ok()
        .and_then(|c| Reflect::get(&c, &"webview".into()).ok())
        .filter(|v| !v.is_undefined())
        .and_then(|wv| {
            let f = Reflect::get(&wv, &"postMessageWithAdditionalObjects".into()).ok()?;
            Some((wv, f.dyn_into::<Function>().ok()?))
        })
    else {
        return false;
    };
    let Some(files) = dt.files() else {
        return false;
    };
    // Message objet (pas un texte) : le relais des commandes de l'application l'ignore.
    let msg = Object::new();
    let _ = Reflect::set(&msg, &"type".into(), &"revel-drop".into());
    post.1.call2(&post.0, &msg, &files).is_ok()
}
