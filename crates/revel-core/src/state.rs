//! État persistant de l'application (sites récents, hébergements, réglages).
//! Aucun secret ici : les mots de passe sont dans le trousseau du système.

use revel_protocol::{AppSettings, HostingProfile, RecentSite};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FILE: &str = "revel-state.json";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(skip)]
    path: PathBuf,
    #[serde(default)]
    pub settings: AppSettings,
    #[serde(default)]
    pub recents: Vec<RecentSite>,
    /// Chemin du site -> hébergements.
    #[serde(default)]
    pub profiles: BTreeMap<String, Vec<HostingProfile>>,
    /// Identifiant du compte GitHub connecté (le jeton est dans le trousseau).
    #[serde(default)]
    pub github_login: Option<String>,
    /// Langues de traduction livrées avec l'application et retirées par l'utilisateur (D-36).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub translate_off: Vec<String>,
    /// Chemin du site -> hébergement -> dernière publication (5.1).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub published: BTreeMap<String, BTreeMap<String, PublishRecord>>,
}

/// Dernière publication sur un hébergement : date, adresse, empreinte de ce qui a été publié.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PublishRecord {
    /// Millisecondes depuis 1970.
    pub at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub snapshot: revel_project::Snapshot,
}

impl State {
    /// Dossier de l'état (vignettes des sites à côté).
    pub fn dir(&self) -> Option<&Path> {
        self.path.parent().filter(|d| !d.as_os_str().is_empty())
    }

    pub fn load(dir: &Path) -> Self {
        let path = dir.join(FILE);
        let mut s: State = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        s.path = path;
        s
    }

    pub fn save(&self) {
        if self.path.as_os_str().is_empty() {
            return;
        }
        if let Some(d) = self.path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Ok(t) = serde_json::to_string_pretty(self) {
            let tmp = self.path.with_extension("json.tmp");
            if std::fs::write(&tmp, t).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
    }
}
