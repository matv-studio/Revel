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
}

impl State {
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
