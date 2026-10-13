//! `revel-model` : le document de site propre à Revel.
//!
//! Un site est un document sérialisé (JSON) dont le schéma est **versionné**.
//! Toute évolution du schéma ajoute une migration `N -> N+1` dans [`migrations`] :
//! un fichier ancien est toujours relu, puis réécrit dans la version courante.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod migrations;

/// Version courante du schéma du document de site.
pub const SCHEMA_VERSION: u32 = 1;

/// Document racine d'un site Revel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Site {
    /// Version du schéma ayant servi à écrire ce document.
    pub schema_version: u32,
    pub name: String,
    /// Langue par défaut du site publié (BCP 47, ex. `fr`).
    pub default_locale: String,
    /// URL publique du site, si connue (ex. `https://alunites.com`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default)]
    pub pages: Vec<Page>,
    /// Points de rupture, du plus large au plus étroit.
    #[serde(default = "Breakpoint::defaults")]
    pub breakpoints: Vec<Breakpoint>,
    /// Classes de styles partagées, indexées par nom.
    #[serde(default)]
    pub styles: BTreeMap<String, StyleClass>,
    /// Redirections permanentes (301) à reproduire chez l'hébergeur.
    #[serde(default)]
    pub redirects: Vec<Redirect>,
}

impl Site {
    pub fn new(name: impl Into<String>, default_locale: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: name.into(),
            default_locale: default_locale.into(),
            base_url: None,
            pages: Vec::new(),
            breakpoints: Breakpoint::defaults(),
            styles: BTreeMap::new(),
            redirects: Vec::new(),
        }
    }

    /// Lit un document JSON, quelle que soit sa version de schéma, et le migre.
    pub fn from_json(text: &str) -> Result<Self, ModelError> {
        let value: serde_json::Value = serde_json::from_str(text)?;
        let migrated = migrations::migrate(value)?;
        Ok(serde_json::from_value(migrated)?)
    }

    /// Écrit le document en JSON lisible (diffs Git lisibles).
    pub fn to_json(&self) -> Result<String, ModelError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    /// Identifiant stable (ne change pas si l'URL change).
    pub id: String,
    /// Chemin public, ex. `/agence/`.
    pub path: String,
    pub title: String,
    #[serde(default)]
    pub seo: Seo,
    /// Arbre d'éléments de la page.
    pub root: Element,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Seo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub noindex: bool,
}

/// Nœud de l'arbre d'éléments (équivalent d'un élément HTML éditable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: String,
    pub kind: ElementKind,
    /// Classes de styles appliquées (noms présents dans `Site::styles`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ElementKind {
    Section,
    Container,
    Heading {
        level: u8,
        text: String,
    },
    Text {
        text: String,
    },
    Link {
        href: String,
        text: String,
    },
    Image {
        src: String,
        alt: String,
    },
    /// HTML brut importé tel quel (reprise d'un site existant).
    RawHtml {
        html: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: String,
    /// Largeur maximale en pixels (`None` = base, sans media query).
    pub max_width: Option<u32>,
}

impl Breakpoint {
    pub fn defaults() -> Vec<Self> {
        [
            ("desktop", None),
            ("tablet", Some(991)),
            ("mobile", Some(767)),
        ]
        .into_iter()
        .map(|(id, max_width)| Self {
            id: id.into(),
            max_width,
        })
        .collect()
    }
}

/// Classe de styles : déclarations CSS par point de rupture.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StyleClass {
    /// Point de rupture -> (propriété CSS -> valeur).
    pub rules: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Redirect {
    pub from: String,
    pub to: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("JSON invalide : {0}")]
    Json(#[from] serde_json::Error),
    #[error("version de schéma {found} plus récente que celle prise en charge ({supported})")]
    TooNew { found: u32, supported: u32 },
    #[error("champ `schema_version` absent ou invalide")]
    MissingVersion,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_json() {
        let mut site = Site::new("Alunites", "fr");
        site.pages.push(Page {
            id: "accueil".into(),
            path: "/".into(),
            title: "Accueil".into(),
            seo: Seo::default(),
            root: Element {
                id: "root".into(),
                kind: ElementKind::Section,
                classes: vec![],
                children: vec![Element {
                    id: "h1".into(),
                    kind: ElementKind::Heading {
                        level: 1,
                        text: "Bonjour".into(),
                    },
                    classes: vec![],
                    children: vec![],
                }],
            },
        });
        let json = site.to_json().unwrap();
        assert_eq!(Site::from_json(&json).unwrap(), site);
    }
}
