//! `revel-cms` : contenus en fichiers (Markdown/YAML) compatibles Pages CMS.
//!
//! Interface posée à l'étape 0 ; implémentation prévue : Étape 3.

/// Entrée de contenu : front matter YAML + corps Markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub collection: String,
    pub slug: String,
    pub front_matter: String,
    pub body: String,
}

/// Source de contenus (dossier local, dépôt Git…).
pub trait ContentSource {
    type Error: std::error::Error;
    fn collections(&self) -> Result<Vec<String>, Self::Error>;
    fn entries(&self, collection: &str) -> Result<Vec<Entry>, Self::Error>;
}

/// Nom du fichier de configuration lu par Pages CMS, à la racine du dépôt.
pub const PAGES_CMS_CONFIG: &str = ".pages.yml";
