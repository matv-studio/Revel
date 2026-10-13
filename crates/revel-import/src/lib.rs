//! `revel-import` : import d'un site existant (cobaye Alunites : Eleventy/Nunjucks).
//!
//! Interface posée à l'étape 0 ; implémentation prévue : Étape 1.

use revel_model::Site;
use std::path::Path;

/// Rapport d'import : ce qui a été repris et ce qui reste à traiter à la main.
#[derive(Debug, Default)]
pub struct Report {
    pub pages: usize,
    pub redirects: usize,
    pub warnings: Vec<String>,
}

/// Importateur d'un type de site.
pub trait Importer {
    type Error: std::error::Error;
    /// Reconnaît-il ce dossier ?
    fn detect(&self, dir: &Path) -> bool;
    fn import(&self, dir: &Path) -> Result<(Site, Report), Self::Error>;
}

/// Détection d'un projet Eleventy (fichier de configuration à la racine).
pub fn is_eleventy(dir: &Path) -> bool {
    ["eleventy.config.js", "eleventy.config.mjs", ".eleventy.js"]
        .iter()
        .any(|f| dir.join(f).is_file())
}
