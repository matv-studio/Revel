//! Réglages du serveur des assistants (`mcp.json` dans le dossier des réglages de Revel) : activé,
//! port local, jeton. Le relais stdio (`revel --mcp`) les lit pour joindre l'application ouverte,
//! sans jeton à recopier dans la configuration de l'assistant.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Port proposé (libre de tout service courant).
pub const DEFAULT_PORT: u16 = 47_520;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpConfig {
    /// Serveur HTTP local démarré avec l'application (désactivé par défaut).
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    /// Jeton exigé par le serveur (`Authorization: Bearer …`).
    #[serde(default)]
    pub token: String,
    /// Adresse publique (tunnel) pour les assistants en ligne, autorisés par OAuth (4.4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_url: Option<String>,
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_PORT,
            token: new_token(),
            public_url: None,
        }
    }
}

/// Jeton aléatoire (32 octets, hexadécimal).
pub fn new_token() -> String {
    let mut b = [0u8; 32];
    if getrandom::fill(&mut b).is_err() {
        // Sans source d'aléa : horloge et adresse mémoire, jamais un jeton vide.
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        for (i, x) in b.iter_mut().enumerate() {
            *x = ((t >> ((i % 16) * 8)) as u8) ^ (i as u8).wrapping_mul(151);
        }
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Fichier des réglages.
pub fn path(config_dir: &Path) -> PathBuf {
    config_dir.join("mcp.json")
}

impl McpConfig {
    /// Réglages lus (créés avec un jeton neuf s'ils n'existent pas).
    pub fn load(config_dir: &Path) -> Self {
        let p = path(config_dir);
        match std::fs::read_to_string(&p)
            .ok()
            .and_then(|t| serde_json::from_str::<McpConfig>(&t).ok())
        {
            Some(mut c) => {
                if c.token.len() < 32 {
                    c.token = new_token();
                    let _ = c.save(config_dir);
                }
                c
            }
            None => {
                let c = Self::default();
                let _ = c.save(config_dir);
                c
            }
        }
    }

    /// Enregistre les réglages (lisibles par le seul utilisateur sous Unix).
    pub fn save(&self, config_dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(config_dir)?;
        let p = path(config_dir);
        let tmp = p.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self).unwrap_or_default())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        std::fs::rename(&tmp, &p)
    }

    /// Adresse du serveur MCP (HTTP en flux).
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/mcp", self.port)
    }
}

/// Dossier des réglages de l'application (même calcul que Tauri : dossier de configuration de
/// l'utilisateur et identifiant de l'application), pour le relais lancé sans l'application.
pub fn app_config_dir(identifier: &str) -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }?;
    Some(base.join(identifier))
}
