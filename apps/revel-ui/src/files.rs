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
pub const MEDIA_ACCEPT: &str = "image/*,video/*,audio/*,.pdf,.doc,.docx,.xls,.xlsx,.ppt,.pptx,.odt,.ods,.odp,.csv,.woff,.woff2,.ttf,.otf,.lottie,.zip";
