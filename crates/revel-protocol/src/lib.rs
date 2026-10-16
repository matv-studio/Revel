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

/// Modèle d'un nouveau site.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteTemplate {
    /// Une page d'accueil à remplir.
    #[default]
    Blank,
    /// Site vitrine : accueil, à propos, contact, menu.
    Showcase,
    /// Blog : accueil, liste des articles, collection d'articles modifiable dans le CMS.
    Blog,
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
    /// Langue du site (`fr`).
    #[serde(default)]
    pub locale: String,
    /// Redirections (ancienne adresse, nouvelle adresse).
    #[serde(default)]
    pub redirects: Vec<(String, String)>,
}

/// Fichier média du site : fichier des dossiers copiés tels quels à la publication (images,
/// vidéos, sons, documents, polices), hors code (CSS, scripts, pages).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaFile {
    /// Adresse dans le site publié (`/media/2024/01/photo.jpg`).
    pub url: String,
    pub kind: MediaKind,
    /// Taille en octets.
    pub size: u64,
    /// Date de modification (secondes depuis 1970).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<i64>,
}

/// Type d'un média, d'après l'extension de son fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Video,
    Audio,
    Document,
    Font,
    /// Animation Lottie (`.lottie`).
    Animation,
    Other,
}

impl MediaKind {
    /// Type d'un fichier ; `None` : code ou fichier technique (pas un média).
    pub fn of(name: &str) -> Option<MediaKind> {
        let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
        Some(match ext.as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "avif" | "svg" | "ico" | "bmp" | "tif"
            | "tiff" | "heic" => MediaKind::Image,
            "mp4" | "webm" | "mov" | "m4v" | "ogv" => MediaKind::Video,
            "mp3" | "wav" | "ogg" | "oga" | "m4a" | "aac" | "flac" | "opus" => MediaKind::Audio,
            "pdf" | "doc" | "docx" | "odt" | "rtf" | "xls" | "xlsx" | "ods" | "csv" | "ppt"
            | "pptx" | "odp" | "key" | "pages" | "numbers" | "epub" => MediaKind::Document,
            "woff" | "woff2" | "ttf" | "otf" | "eot" => MediaKind::Font,
            "lottie" => MediaKind::Animation,
            "zip" | "rar" | "7z" | "gz" | "dmg" | "exe" | "msi" | "apk" | "glb" | "gltf"
            | "usdz" | "vtt" | "srt" => MediaKind::Other,
            _ => return None,
        })
    }
}

/// Page HTML du site, proposée dans l'éditeur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorPageInfo {
    /// Adresse (`/agence/`).
    pub url: String,
    /// Titre de la page s'il est connu.
    #[serde(default)]
    pub title: String,
    /// Page fixe : identifiant ; `None` pour un contenu de collection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// Contenu : identifiant de sa collection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    /// Gabarit de la page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    /// Contenu : nom de son fichier, sans extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Page en brouillon (non publiée).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub draft: bool,
    /// Nom de la page dans Revel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Page fixe : référencement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo: Option<PageSeoInfo>,
}

/// Référencement d'une page fixe et endroit où il s'écrit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSeoInfo {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub noindex: bool,
    /// `seo` dans un fichier de données : chemin des clés (`pages`, `agence`, `seo`) ; vide :
    /// variable de la page.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_path: Vec<String>,
    /// `seo` calculé par une expression : non modifiable dans les paramètres de la page.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub computed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSummary {
    pub id: String,
    pub folder: String,
    pub entries: usize,
}

/// Collection de contenus, telle que l'affiche la vue CMS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CmsCollection {
    pub id: String,
    /// Nom affiché (configuration de Pages CMS), sinon `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Dossier des fichiers, relatif au site.
    pub folder: String,
    /// Champs : schéma de la collection (`.pages.yml` de Pages CMS), sinon ceux trouvés dans
    /// les contenus.
    pub fields: Vec<CmsField>,
    /// Nom de la collection dans `.pages.yml` quand son schéma en vient (les références
    /// désignent une collection par ce nom).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// Pages CMS garde les valeurs absentes du schéma en enregistrant un contenu
    /// (`settings.content.merge`) ; sinon il les retire.
    #[serde(default)]
    pub merge: bool,
    /// Tableau : champ principal, colonnes, tri (`view` de Pages CMS).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<CmsTableView>,
    /// Modèle du nom de fichier d'un nouveau contenu (`{fields.slug}.md`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    pub entries: Vec<CmsEntry>,
    /// Contenus traduits, par langue (fichiers du dossier de chaque autre langue du site, 3.8).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub locales: std::collections::BTreeMap<String, Vec<CmsEntry>>,
}

/// Présentation du tableau d'une collection.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CmsTableView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// Ordre décroissant.
    #[serde(default)]
    pub descending: bool,
}

/// Champ d'une collection (types et options de Pages CMS).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CmsField {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Type (`string`, `text`, `rich_text`, `image`, `file`, `date`, `boolean`, `number`,
    /// `select`, `reference`, `object`, `block`, `code`, `list`) ; `None` : déduit des valeurs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Aide affichée au survol du libellé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    /// Champ technique, caché par défaut dans la fiche.
    #[serde(default)]
    pub hidden: bool,
    /// Liste de valeurs de ce type.
    #[serde(default)]
    pub list: bool,
    /// Valeur d'un nouveau contenu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Options du type (format d'un texte riche, choix d'une liste, collection d'une référence,
    /// heure d'une date, motif…), telles qu'écrites dans `.pages.yml`.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub options: serde_json::Value,
    /// Motif (expression régulière) d'un texte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Champs d'un objet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<CmsField>,
    /// Composant de `.pages.yml` dont le champ reprend la définition (partagée).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
}

/// Contenu d'une collection : en-tête du fichier Markdown et page publiée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CmsEntry {
    /// Nom du fichier, sans extension.
    pub file: String,
    /// Valeurs de l'en-tête (objet).
    pub data: serde_json::Value,
    /// Texte (Markdown) après l'en-tête.
    #[serde(default)]
    pub body: String,
    /// Adresse de sa page ; `None` : pas de page (ou contenu non publié).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
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
    /// Avant de publier : changements enregistrés et versions synchronisées avec le dépôt du site
    /// (D-25), quand le site a des versions et un dépôt distant.
    #[serde(default = "yes")]
    pub sync_versions: bool,
}

fn yes() -> bool {
    true
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
    /// Éléments qui prennent la couleur d'accent : `minimal` (focus et contour du plan de
    /// travail), `standard` (aussi l'action principale et la sélection), `full` (aussi les onglets,
    /// choix et éléments actifs).
    #[serde(default = "default_accent_scope")]
    pub accent_scope: String,
}

fn default_theme() -> String {
    "system".into()
}

fn default_accent_scope() -> String {
    "standard".into()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            lang: "fr".into(),
            theme: default_theme(),
            accent: None,
            accent_scope: default_accent_scope(),
        }
    }
}

/// Texte à traduire automatiquement (D-36).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranslateItem {
    pub text: String,
    /// `plain`, `markdown` ou `html` ; absente : devinée d'après le texte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Langue de traduction automatique (réglages de l'application, D-36).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranslatePack {
    /// Code de la langue (`de`).
    pub code: String,
    pub state: PackState,
    /// Taille à télécharger et taille une fois installée, en octets.
    pub download: u64,
    pub disk: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackState {
    /// Livrée avec Revel, en service.
    Bundled,
    /// Livrée avec Revel, retirée par l'utilisateur (remise en service sans téléchargement).
    BundledOff,
    /// Téléchargée.
    Installed,
    /// À télécharger.
    Available,
}

/// Avancement du téléchargement d'une langue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TranslateEvent {
    Progress { done: u64, total: u64 },
    Preparing,
}

/// Police du site (3.9, D-24) : famille déclarée par des règles `@font-face` de la feuille de
/// style, fichiers servis par le site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontFamily {
    pub family: String,
    pub faces: Vec<FontFace>,
    /// Règles de style qui l'utilisent (`font-family`).
    pub uses: usize,
    /// Taille des fichiers, en octets.
    pub size: u64,
}

/// Graisse et style d'une police (plusieurs règles `@font-face` : jeux de caractères).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontFace {
    /// `400`, `700`, `100 900` (police variable).
    pub weight: String,
    /// `normal` ou `italic`.
    pub style: String,
    /// Adresses des fichiers.
    pub files: Vec<String>,
}

/// Police du catalogue de Google Fonts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleFont {
    pub family: String,
    pub category: String,
    /// `400`, `400i`, `700`…
    pub variants: Vec<String>,
}

/// Fichier de police importé (contenu en base64), graisse et style choisis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontUpload {
    pub name: String,
    pub data: String,
    pub weight: String,
    #[serde(default)]
    pub italic: bool,
}

/// Versions du site (3.10, D-25) : état du dépôt Git du dossier du site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionsStatus {
    pub branch: String,
    /// Fichiers changés depuis la dernière version.
    pub changes: usize,
    /// Dépôt distant (`origin`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// Versions à envoyer, versions en ligne à récupérer (selon la dernière synchronisation).
    pub ahead: usize,
    pub behind: usize,
    /// Compte GitHub connecté (nécessaire pour un dépôt GitHub).
    pub github: bool,
}

/// Version enregistrée du site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    pub message: String,
    pub author: String,
    /// Secondes depuis 1970.
    pub time: i64,
    /// Fichiers changés par cette version.
    pub files: usize,
    /// Fusion de versions faites ailleurs (Pages CMS).
    pub merge: bool,
}

/// Fichier changé : `added`, `modified`, `deleted`, `renamed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub change: String,
}

/// Ligne d'une comparaison : `+`, `-` ou ` `.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: char,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new: Option<u32>,
    pub text: String,
}

/// Résultat d'une synchronisation avec le dépôt distant.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncInfo {
    pub pulled: usize,
    pub pushed: usize,
    pub merged: bool,
    /// Fichiers modifiés des deux côtés : rien n'a été changé, choix à faire.
    #[serde(default)]
    pub conflicts: Vec<String>,
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

/// Données des réglages lisibles (3.12) : valeurs des chemins de données demandés (dans l'ordre),
/// leur libellé dans Pages CMS, contenus des collections demandées.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReadableData {
    pub values: Vec<Option<serde_json::Value>>,
    pub labels: Vec<Option<String>>,
    pub entries: Vec<PickEntry>,
}

/// Contenu d'une collection proposé dans une liste de contenus mis en avant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickEntry {
    /// Collection (`projets`).
    pub collection: String,
    /// Identifiant écrit dans la liste (`data.slug`).
    pub slug: String,
    pub title: String,
}
