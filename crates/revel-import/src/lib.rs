//! `revel-import` : import de sites existants vers un projet Revel.
//!
//! - [`eleventy`] : projet Eleventy (gabarits Nunjucks, contenus Markdown, données YAML/JSON).
//!   Les gabarits deviennent des pages, gabarits et composants Revel ; les contenus restent
//!   des fichiers (collections compatibles Pages CMS). La logique JavaScript du site
//!   (filtres, collections calculées) est décrite dans un **profil d'import** JSON.
//! - [`html`] : site HTML statique (7.1) : pages, gabarit commun, feuille de style du site,
//!   fichiers copiés tels quels.
//!
//! [`detect`] reconnaît la nature d'un dossier (Eleventy, HTML statique, site à construire).

use std::path::Path;

pub mod eleventy;
pub mod html;

/// Rapport d'import : ce qui a été repris et ce qui reste à vérifier.
#[derive(Debug, Default)]
pub struct Report {
    pub pages: usize,
    pub layouts: usize,
    pub components: usize,
    pub collections: usize,
    /// Règles de style importées dans le modèle (D-18).
    pub style_rules: usize,
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

/// Outils dont le site se construit (dépendances de `package.json`), du plus précis au plus
/// général.
const BUILD_TOOLS: &[(&str, &str)] = &[
    ("@sveltejs/kit", "SvelteKit"),
    ("next", "Next.js"),
    ("nuxt", "Nuxt"),
    ("astro", "Astro"),
    ("gatsby", "Gatsby"),
    ("@remix-run/react", "Remix"),
    ("@react-router/dev", "React Router"),
    ("@angular/core", "Angular"),
    ("@solidjs/start", "SolidStart"),
    ("@docusaurus/core", "Docusaurus"),
    ("vitepress", "VitePress"),
    ("react-scripts", "Create React App"),
    ("svelte", "Svelte"),
    ("vue", "Vue"),
    ("react", "React"),
    ("solid-js", "Solid"),
    ("vite", "Vite"),
    ("parcel", "Parcel"),
    ("webpack", "webpack"),
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

/// Outil de construction du site, d'après `package.json` ou les fichiers de configuration de
/// générateurs connus (Hugo, Jekyll).
fn build_tool(dir: &Path) -> Option<String> {
    if let Ok(text) = std::fs::read_to_string(dir.join("package.json"))
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&text)
    {
        let has = |name: &str| {
            ["dependencies", "devDependencies"]
                .iter()
                .any(|k| v.get(k).and_then(|d| d.get(name)).is_some())
        };
        if let Some((_, tool)) = BUILD_TOOLS.iter().find(|(dep, _)| has(dep)) {
            return Some((*tool).to_string());
        }
    }
    if ["hugo.toml", "hugo.yaml", "hugo.json"]
        .iter()
        .any(|f| dir.join(f).is_file())
        || (dir.join("config.toml").is_file() && dir.join("layouts").is_dir())
    {
        return Some("Hugo".into());
    }
    if dir.join("_config.yml").is_file()
        && (dir.join("_layouts").is_dir() || dir.join("Gemfile").is_file())
    {
        return Some("Jekyll".into());
    }
    None
}
