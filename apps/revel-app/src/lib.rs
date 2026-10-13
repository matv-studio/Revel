//! Coque applicative : relie l'interface (dossier `ui/`) aux modules Rust
//! par des commandes Tauri. Aucune logique métier ici : elle vit dans les crates.

use serde::Serialize;

/// Textes et informations affichés par l'écran d'accueil de l'étape 0.
#[derive(Serialize)]
struct AppInfo {
    lang: String,
    title: String,
    tagline: String,
    version: String,
    stage: String,
    schema_version: u32,
    hosts: Vec<&'static str>,
}

#[tauri::command]
fn app_info(lang: String) -> AppInfo {
    let l = revel_i18n::Localizer::new(&lang);
    AppInfo {
        lang: l.lang().to_string(),
        title: l.t("app-title", &[]),
        tagline: l.t("app-tagline", &[]),
        version: l.t("app-version", &[("version", env!("CARGO_PKG_VERSION"))]),
        stage: l.t("app-stage", &[("stage", "0")]),
        schema_version: revel_model::SCHEMA_VERSION,
        hosts: revel_deploy::providers::all()
            .into_iter()
            .map(|c| c.id)
            .collect(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_info])
        .run(tauri::generate_context!())
        .expect("erreur au démarrage de Revel");
}
