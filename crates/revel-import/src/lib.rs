//! `revel-import` : import de sites existants vers un projet Revel.
//!
//! - [`eleventy`] : projet Eleventy (gabarits Nunjucks, contenus Markdown, données YAML/JSON).
//!   Les gabarits deviennent des pages, gabarits et composants Revel ; les contenus restent
//!   des fichiers (collections compatibles Pages CMS). La logique JavaScript du site
//!   (filtres, collections calculées) est décrite dans un **profil d'import** JSON.
//! - [`html`] : site HTML statique (7.1) : pages, gabarit commun, feuille de style du site,
//!   fichiers copiés tels quels.
//!
//! - [`crawl`] : site publié lu à son adresse (7.4), écrit comme un dossier statique.
//!
//! [`detect`] reconnaît la nature d'un dossier (Eleventy, HTML statique, site à construire).

#![cfg_attr(test, recursion_limit = "512")]

use std::path::Path;

pub mod adapt;
pub mod crawl;
pub mod eleventy;
pub mod figma;
pub mod framer;
pub mod framer_click;
pub mod framer_components;
pub mod framer_jsx;
pub mod framer_names;
pub mod framer_site;
pub mod html;
pub mod webflow;
pub mod wordpress;

/// Rapport d'import : ce qui a été repris et ce qui reste à vérifier.
#[derive(Debug, Default)]
pub struct Report {
    pub pages: usize,
    pub layouts: usize,
    pub components: usize,
    pub collections: usize,
    /// Règles de style importées dans le modèle (D-18).
    pub style_rules: usize,
    /// Outil d'origine reconnu (Framer, Webflow, WordPress).
    pub tool: Option<String>,
    /// Animations reprises en animations Revel (7.7).
    pub animations: usize,
    /// Formulaires repris en formulaires Revel.
    pub forms: usize,
    /// Entrées de collections écrites.
    pub entries: usize,
    /// Fichiers téléchargés (médias).
    pub files: usize,
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

/// Nature d'un dossier à importer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiteKind {
    /// Projet Eleventy (gabarits Nunjucks).
    Eleventy,
    /// Pages HTML à la racine.
    Static { pages: usize },
    /// Site à construire par son outil (D-80) : nom de l'outil, dossiers produits présents
    /// (relatifs, contenant un `index.html`).
    NeedsBuild { tool: String, outputs: Vec<String> },
    /// Rien d'importable à la racine ; dossiers de pages présents (`docs`, `dist`…).
    Unknown { outputs: Vec<String> },
}

/// Dossiers où un générateur ou un hébergement place souvent les pages produites.
const OUTPUT_DIRS: &[&str] = &[
    "dist", "build", "out", "_site", "public", "docs", "www", "site", "html",
];

/// Reconnaît la nature du dossier `dir`.
pub fn detect(dir: &Path) -> SiteKind {
    if is_eleventy(dir) {
        return SiteKind::Eleventy;
    }
    let outputs: Vec<String> = OUTPUT_DIRS
        .iter()
        .filter(|d| dir.join(d).join("index.html").is_file())
        .map(|d| d.to_string())
        .collect();
    if let Some(tool) = build_tool(dir) {
        return SiteKind::NeedsBuild { tool, outputs };
    }
    if html::is_static(dir) {
        return SiteKind::Static {
            pages: html::scan(dir).len(),
        };
    }
    SiteKind::Unknown { outputs }
}

/// Outil de construction du site (dépendances de `package.json`, Hugo, Jekyll).
fn build_tool(dir: &Path) -> Option<String> {
    revel_build::recipe::tool_name(dir)
}
