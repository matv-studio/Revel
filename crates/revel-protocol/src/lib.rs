//! `revel-protocol` : messages échangés entre l'interface (WebAssembly) et le cœur de Revel.
//!
//! Types sérialisables uniquement, sans dépendance système : la même crate est compilée
//! pour l'interface (wasm32) et pour l'application, la ligne de commande et le futur serveur MCP.

use serde::{Deserialize, Serialize};

/// Site connu de l'application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentSite {
    pub path: String,
    pub name: String,
    /// Date d'ouverture (RFC 3339).
    pub opened_at: String,
}

/// Résumé d'un site ouvert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteSummary {
    pub path: String,
    pub name: String,
    pub base_url: Option<String>,
    pub pages: usize,
    pub layouts: usize,
    pub components: usize,
    pub collections: Vec<CollectionSummary>,
    /// Environnements de publication déclarés (`production`, `staging`…).
    pub environments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSummary {
    pub id: String,
    pub folder: String,
    pub entries: usize,
}

/// Résultat d'un import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportResult {
    pub site: SiteSummary,
    pub warnings: Vec<String>,
}

/// Étape de génération.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildPhase {
    Pages,
    Copies,
    Images,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum BuildEvent {
    Progress {
        phase: BuildPhase,
        done: usize,
        total: usize,
    },
    Finished(BuildSummary),
    Failed(Failure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildSummary {
    pub environment: String,
    /// Dossier produit.
    pub out_dir: String,
    pub generated: usize,
    pub copied: usize,
    pub images_produced: usize,
    pub images_cached: usize,
    pub images_failed: usize,
    pub seconds: f32,
    pub warnings: Vec<String>,
}

/// Hébergement configuré pour un site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostingProfile {
    pub id: String,
    pub name: String,
    /// Environnement généré pour cet hébergement (`production`, `staging`…).
    pub environment: String,
    pub target: HostingTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostingTarget {
    Sftp {
        host: String,
        port: u16,
        user: String,
        /// Dossier distant relatif au dossier personnel (ex. `www/`).
        remote_dir: String,
        /// Clé du serveur enregistrée (format `known_hosts`) ; `None` avant la première connexion.
        known_host_key: Option<String>,
    },
    /// GitHub Pages : le site généré est envoyé sur une branche du dépôt, servie par GitHub.
    GithubPages {
        /// Dépôt `proprietaire/nom`.
        repo: String,
        /// Branche publiée (`gh-pages` par défaut), réservée au site généré.
        branch: String,
        /// Domaine personnalisé (`www.exemple.fr`) ; `None` : adresse `*.github.io`.
        #[serde(default)]
        custom_domain: Option<String>,
    },
}

impl HostingTarget {
    /// Identifiant de capacités (`revel-deploy::providers`).
    pub fn provider_id(&self) -> &'static str {
        match self {
            Self::Sftp { .. } => "sftp-apache",
            Self::GithubPages { .. } => "github-pages",
        }
    }
}

/// Connexion d'un compte GitHub (autorisation par code sur github.com).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum GithubEvent {
    /// Code à saisir sur `verification_uri`.
    Code {
        user_code: String,
        verification_uri: String,
        expires_in: u64,
    },
    Connected {
        login: String,
    },
    Failed(Failure),
}

/// Échec d'une commande : clé de traduction (`error-…` dans `revel-i18n`) et paramètres.
///
/// Le cœur ne produit pas de phrases : l'interface, la ligne de commande ou le serveur MCP
/// affichent l'échec dans la langue de l'utilisateur. `detail` porte le message technique
/// d'origine (système, serveur distant), non traduit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failure {
    pub key: String,
    #[serde(default)]
    pub args: Vec<(String, String)>,
}

impl Failure {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            args: Vec::new(),
        }
    }

    pub fn arg(mut self, name: &str, value: impl Into<String>) -> Self {
        self.args.push((name.to_string(), value.into()));
        self
    }

    /// Échec sans traduction dédiée : message technique seul.
    pub fn detail(message: impl Into<String>) -> Self {
        Self::new("error-detail").arg("detail", message)
    }
}

/// Avertissement d'hébergement : clé de traduction et paramètres.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostWarning {
    pub blocking: bool,
    pub key: String,
    pub args: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum DeployEvent {
    /// Génération préalable. Variante à champ nommé : `BuildEvent` porte lui aussi une
    /// étiquette `event`, qui entrerait en conflit avec celle-ci si elle était aplatie.
    Building {
        build: BuildEvent,
    },
    Warnings {
        warnings: Vec<HostWarning>,
    },
    Connected {
        server_key: String,
    },
    /// Récupération de la version en ligne (dépôt Git) : objets reçus / attendus.
    Fetching {
        done: usize,
        total: usize,
    },
    /// Lecture des fichiers du site généré.
    Preparing {
        done: usize,
        total: usize,
    },
    Planned {
        upload: usize,
        delete: usize,
        bytes: u64,
        total: usize,
    },
    Uploaded {
        done: usize,
        total: usize,
    },
    Deleted {
        done: usize,
        total: usize,
    },
    Finished(DeploySummary),
    Failed(Failure),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploySummary {
    pub uploaded: usize,
    pub deleted: usize,
    pub total_files: usize,
    pub bytes: u64,
    pub dry_run: bool,
    /// Clé du serveur reçue (à confirmer par l'utilisateur à la première connexion).
    pub server_key: String,
    pub server_fingerprint: String,
    /// La clé vient d'être enregistrée (première connexion).
    pub key_recorded: bool,
    /// Adresse publique du site, si l'hébergeur la fournit.
    #[serde(default)]
    pub url: Option<String>,
    /// GitHub Pages vient d'être activé : la mise en ligne peut prendre quelques minutes.
    #[serde(default)]
    pub pages_enabled: bool,
}

/// Réglages de l'application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    /// Langue de l'interface (`fr`, `en`).
    pub lang: String,
    /// Thème : `system`, `dark` ou `light`.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Couleur d'accent (`#rrggbb`) ; `None` : interface neutre, sans accent.
    #[serde(default)]
    pub accent: Option<String>,
}

fn default_theme() -> String {
    "system".into()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            lang: "fr".into(),
            theme: default_theme(),
            accent: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evenements_json() {
        let e = DeployEvent::Planned {
            upload: 2,
            delete: 1,
            bytes: 10,
            total: 5,
        };
        let j = serde_json::to_string(&e).unwrap();
        assert_eq!(
            j,
            r#"{"event":"planned","upload":2,"delete":1,"bytes":10,"total":5}"#
        );
        assert_eq!(serde_json::from_str::<DeployEvent>(&j).unwrap(), e);
        let t = HostingTarget::Sftp {
            host: "h".into(),
            port: 22,
            user: "u".into(),
            remote_dir: "www/".into(),
            known_host_key: None,
        };
        assert!(
            serde_json::to_string(&t)
                .unwrap()
                .starts_with(r#"{"kind":"sftp""#)
        );
        let b = DeployEvent::Building {
            build: BuildEvent::Progress {
                phase: BuildPhase::Images,
                done: 1,
                total: 2,
            },
        };
        let j = serde_json::to_string(&b).unwrap();
        assert_eq!(
            j,
            r#"{"event":"building","build":{"event":"progress","phase":"images","done":1,"total":2}}"#
        );
        assert_eq!(serde_json::from_str::<DeployEvent>(&j).unwrap(), b);
        let f = DeployEvent::Failed(Failure::new("error-sftp-auth"));
        let j = serde_json::to_string(&f).unwrap();
        assert_eq!(j, r#"{"event":"failed","key":"error-sftp-auth","args":[]}"#);
        assert_eq!(serde_json::from_str::<DeployEvent>(&j).unwrap(), f);
    }
}
