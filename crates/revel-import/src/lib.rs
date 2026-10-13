//! `revel-import` : import de sites existants vers un projet Revel.
//!
//! - [`eleventy`] : projet Eleventy (gabarits Nunjucks, contenus Markdown, données YAML/JSON).
//!   Les gabarits deviennent des pages, gabarits et composants Revel ; les contenus restent
//!   des fichiers (collections compatibles Pages CMS). La logique JavaScript du site
//!   (filtres, collections calculées) est décrite dans un **profil d'import** JSON.

use std::path::Path;

pub mod eleventy;

/// Rapport d'import : ce qui a été repris et ce qui reste à vérifier.
#[derive(Debug, Default)]
pub struct Report {
    pub pages: usize,
    pub layouts: usize,
    pub components: usize,
    pub collections: usize,
    pub warnings: Vec<String>,
}

/// Détection d'un projet Eleventy (fichier de configuration à la racine).
pub fn is_eleventy(dir: &Path) -> bool {
    [
        "eleventy.config.js",
        "eleventy.config.mjs",
        "eleventy.config.cjs",
        ".eleventy.js",
    ]
    .iter()
    .any(|f| dir.join(f).is_file())
}
