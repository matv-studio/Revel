//! `revel-model` : le document de site propre à Revel.
//!
//! Un site est un document sérialisé (JSON) dont le schéma est **versionné**.
//! Toute évolution du schéma ajoute une migration `N -> N+1` dans [`migrations`] :
//! un fichier ancien est toujours relu, puis réécrit dans la version courante.
//!
//! ## Contenu
//! - [`Page`] : une URL publiée (HTML ou fichier texte : `sitemap.xml`, `.htaccess`…).
//! - [`Layout`] : gabarit partagé ; la page s'insère à l'emplacement [`Node::Slot`].
//! - [`Component`] : sous-arbre réutilisable (en-tête, pied, carte…).
//! - [`Collection`] : contenus structurés (fichiers Markdown + front matter, compatibles Pages CMS),
//!   avec un gabarit de page par entrée.
//! - [`Node`] : arbre d'éléments. Les parties dynamiques utilisent des **expressions**
//!   (syntaxe Jinja : `item.data.title`, `items | limit(3)`, `a if b else c`).
//!
//! Les expressions sont stockées en texte et évaluées par `revel-render`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub mod ids;
pub mod migrations;

/// Version courante du schéma du document de site.
pub const SCHEMA_VERSION: u32 = 3;

/// Document racine d'un site Revel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Site {
    /// Version du schéma ayant servi à écrire ce document.
    pub schema_version: u32,
    pub name: String,
    /// Langue par défaut du site publié (BCP 47, ex. `fr`).
    pub default_locale: String,
    /// URL publique du site (ex. `https://alunites.com`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Autres langues du site (3.8, D-23) : adresses préfixées (`/en/…`), textes des pages
    /// traduits ici, données et contenus dans un dossier par langue (`_data/en/`, `content/en/…`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locales: Vec<Locale>,
    #[serde(default)]
    pub settings: Settings,
    /// Données globales accessibles dans les expressions par leur nom (ex. `site.title`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub data: BTreeMap<String, Value>,
    /// Dossier de fichiers de données (YAML/JSON), relatif au projet. Chaque fichier devient
    /// une variable globale portant son nom (`pages/accueil.yml` -> `pages.accueil`).
    /// Ces fichiers restent modifiables par un CMS (Pages CMS).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<String>,
    #[serde(default)]
    pub pages: Vec<Page>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layouts: Vec<Layout>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collections: Vec<Collection>,
    /// Dossiers copiés tels quels dans le site publié : (source dans le projet, destination).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub passthrough: Vec<Passthrough>,
    /// Points de rupture, du plus large au plus étroit.
    #[serde(default = "Breakpoint::defaults")]
    pub breakpoints: Vec<Breakpoint>,
    /// Feuille de style du site, détenue par Revel (D-18) ; `None` : aucune (les CSS éventuels
    /// sont copiés tels quels).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stylesheet: Option<Stylesheet>,
    /// Redirections permanentes (301) déclarées dans Revel.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub redirects: Vec<Redirect>,
    /// Rangement de l'éditeur (dossiers des médias et des composants) ; sans effet sur le site
    /// publié ni sur l'emplacement des fichiers.
    #[serde(default, skip_serializing_if = "Library::is_empty")]
    pub library: Library,
}

/// Rangement des médias et des composants dans l'éditeur.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Library {
    #[serde(default, skip_serializing_if = "Shelf::is_empty")]
    pub media: Shelf,
    #[serde(default, skip_serializing_if = "Shelf::is_empty")]
    pub components: Shelf,
    /// Dossiers de pages créés vides (chemins d'adresse `offres/2026`) ; une page est dans un
    /// dossier par son adresse.
    #[serde(default, skip_serializing_if = "Shelf::is_empty")]
    pub pages: Shelf,
    /// Texte alternatif par défaut d'un média (adresse -> texte), repris quand l'image est placée.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub alt: BTreeMap<String, String>,
    /// Plan du site (D-103) : position choisie de chaque nœud déplacé (clé du nœud -> `[x, y]`) ;
    /// les autres sont rangés d'après les adresses.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub map: BTreeMap<String, [i32; 2]>,
}

impl Library {
    pub fn is_empty(&self) -> bool {
        self.media.is_empty()
            && self.components.is_empty()
            && self.pages.is_empty()
            && self.alt.is_empty()
            && self.map.is_empty()
    }
}

/// Dossiers (chemins `logos/clients`, dans l'ordre de création) et dossier de chaque élément
/// rangé (adresse d'un média, identifiant d'un composant). Un élément absent est à la racine.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Shelf {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub items: BTreeMap<String, String>,
    /// Ordre d'affichage choisi dans l'éditeur (glisser dans le panneau) : clés des éléments et
    /// dossiers, les autres à la suite dans l'ordre habituel. Sans effet sur le site publié.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<String>,
}

impl Shelf {
    pub fn is_empty(&self) -> bool {
        self.folders.is_empty() && self.items.is_empty() && self.order.is_empty()
    }

    /// Position d'affichage d'une clé (`None` : pas d'ordre choisi).
    pub fn rank(&self, key: &str) -> Option<usize> {
        self.order.iter().position(|k| k == key)
    }

    /// Range `keys` (éléments voisins, dans leur nouvel ordre) : leurs places précédentes retirées,
    /// la suite ajoutée à la fin.
    pub fn reorder(&mut self, keys: &[String]) {
        self.order.retain(|k| !keys.contains(k));
        self.order.extend(keys.iter().cloned());
    }
}

impl Site {
    pub fn new(name: impl Into<String>, default_locale: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: name.into(),
            default_locale: default_locale.into(),
            base_url: None,
            locales: Vec::new(),
            settings: Settings::default(),
            data: BTreeMap::new(),
            data_dir: None,
            pages: Vec::new(),
            layouts: Vec::new(),
            components: Vec::new(),
            collections: Vec::new(),
            passthrough: Vec::new(),
            breakpoints: Breakpoint::defaults(),
            stylesheet: None,
            redirects: Vec::new(),
            library: Library::default(),
        }
    }

    /// Lit un document JSON, quelle que soit sa version de schéma, et le migre.
    /// Chaque nœud structurel reçoit un identifiant s'il n'en a pas (voir [`ids`]).
    pub fn from_json(text: &str) -> Result<Self, ModelError> {
        let value: Value = serde_json::from_str(text)?;
        let migrated = migrations::migrate(value)?;
        let mut site: Self = serde_json::from_value(migrated)?;
        site.ensure_ids();
        Ok(site)
    }

    /// Écrit le document en JSON lisible (diffs Git lisibles).
    pub fn to_json(&self) -> Result<String, ModelError> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }

    pub fn layout(&self, id: &str) -> Option<&Layout> {
        self.layouts.iter().find(|l| l.id == id)
    }

    pub fn component(&self, id: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.id == id)
    }

    pub fn collection(&self, id: &str) -> Option<&Collection> {
        self.collections.iter().find(|c| c.id == id)
    }

    pub fn page(&self, id: &str) -> Option<&Page> {
        self.pages.iter().find(|p| p.id == id)
    }

    /// Autre langue du site.
    pub fn locale(&self, code: &str) -> Option<&Locale> {
        self.locales.iter().find(|l| l.code == code)
    }

    /// Langue d'une adresse : `/en/agence/` -> `("en", "/agence/")` ; langue principale : `None`.
    pub fn locale_of_url<'u>(&self, url: &'u str) -> Option<(&Locale, &'u str)> {
        self.locales.iter().find_map(|l| {
            let rest = url.strip_prefix('/')?.strip_prefix(l.code.as_str())?;
            match rest {
                "" => Some((l, "/")),
                r if r.starts_with('/') => Some((l, r)),
                _ => None,
            }
        })
    }
}

/// Autre langue du site (3.8).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Locale {
    /// Code de langue BCP 47 (`en`, `de`, `pt-BR`) ; préfixe des adresses (`/en/`).
    pub code: String,
    /// Nom affiché (sélecteur de langue) ; par défaut, le nom de la langue dans cette langue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Langue en préparation : visible dans l'éditeur et l'aperçu, pas publiée.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub draft: bool,
    /// Textes écrits dans les pages, gabarits et composants (texte de la langue principale,
    /// espaces regroupés -> traduction).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub texts: BTreeMap<String, String>,
    /// Traductions automatiques à relire (D-36) : `text:<texte d'origine>` (textes des pages),
    /// `data:<chemin>` (fichiers de données), `entry:<dossier>/<fichier>` (contenus).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub machine: BTreeSet<String>,
}

impl Locale {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            ..Default::default()
        }
    }

    /// Nom affiché.
    pub fn label(&self) -> String {
        self.name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| language_name(&self.code))
    }
}

/// Page qui existe dans chaque langue du site : page HTML (`/agence/`, `/a.html`), sauf la page
/// d'erreur 404 (plan du site, flux, `robots.txt` : langue principale seulement).
pub fn localizable_path(path: &str) -> bool {
    let last = path.rsplit('/').next().unwrap_or("");
    (path.ends_with('/') || last.ends_with(".html") || last.ends_with(".htm"))
        && !path.ends_with("404.html")
        && !path.ends_with("/404/")
}

/// Dossier des contenus d'une collection dans une autre langue (D-23) : le code de langue avant
/// le dossier (`src/content/projets` -> `src/content/en/projets`).
pub fn locale_folder(folder: &str, code: &str) -> String {
    let f = folder.trim_end_matches('/');
    match f.rsplit_once('/') {
        Some((parent, name)) => format!("{parent}/{code}/{name}"),
        None => format!("{code}/{f}"),
    }
}

/// Code de langue accepté : `en`, `fr-CA`, `pt-BR`, `zh-Hant` (lettres minuscules, puis
/// sous-étiquettes de 2 à 8 lettres ou chiffres).
pub fn valid_locale_code(code: &str) -> bool {
    let mut parts = code.split('-');
    parts
        .next()
        .is_some_and(|p| (2..=3).contains(&p.len()) && p.chars().all(|c| c.is_ascii_lowercase()))
        && parts.all(|p| (2..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// Nom d'une langue dans cette langue (`en` -> `English`) ; sinon le code en majuscules.
pub fn language_name(code: &str) -> String {
    let base = code.split('-').next().unwrap_or(code).to_ascii_lowercase();
    let name = match base.as_str() {
        "fr" => "Français",
        "en" => "English",
        "de" => "Deutsch",
        "es" => "Español",
        "it" => "Italiano",
        "pt" => "Português",
        "nl" => "Nederlands",
        "ca" => "Català",
        "eu" => "Euskara",
        "br" => "Brezhoneg",
        "oc" => "Occitan",
        "co" => "Corsu",
        "pl" => "Polski",
        "cs" => "Čeština",
        "sk" => "Slovenčina",
        "sl" => "Slovenščina",
        "hr" => "Hrvatski",
        "hu" => "Magyar",
        "ro" => "Română",
        "bg" => "Български",
        "el" => "Ελληνικά",
        "ru" => "Русский",
        "uk" => "Українська",
        "sv" => "Svenska",
        "da" => "Dansk",
        "nb" | "no" => "Norsk",
        "fi" => "Suomi",
        "et" => "Eesti",
        "lv" => "Latviešu",
        "lt" => "Lietuvių",
        "ga" => "Gaeilge",
        "tr" => "Türkçe",
        "ar" => "العربية",
        "he" => "עברית",
        "fa" => "فارسی",
        "hi" => "हिन्दी",
        "zh" => "中文",
        "ja" => "日本語",
        "ko" => "한국어",
        "vi" => "Tiếng Việt",
        "th" => "ไทย",
        "id" => "Bahasa Indonesia",
        "ms" => "Bahasa Melayu",
        _ => return code.to_ascii_uppercase(),
    };
    name.to_string()
}

/// Réglages de génération.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Typographie appliquée au texte visible des pages HTML (`fr` : espaces insécables,
    /// apostrophe typographique). `None` : aucune transformation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typography: Option<String>,
    /// Images : largeurs générées par défaut.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub image_widths: Vec<u32>,
    /// Environnements de publication (ex. `production`, `staging`) et leurs variables,
    /// exposées dans les expressions sous `env`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub environments: BTreeMap<String, Value>,
    /// Filtres personnalisés, définis par une expression (sans code) :
    /// `value | nom(arg)` évalue `expr` avec `params[0] = value`, `params[1] = arg`…
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub filters: BTreeMap<String, CustomFilter>,
    /// Référencement du site : plan du site et `robots.txt` produits par Revel.
    #[serde(default, skip_serializing_if = "SeoSettings::is_default")]
    pub seo: SeoSettings,
    /// Mots gardés tels quels par la traduction automatique (noms de marque, de produit), en plus
    /// du nom du site.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub no_translate: Vec<String>,
    /// Code ajouté à toutes les pages HTML du site (3.9).
    #[serde(default, skip_serializing_if = "CustomCode::is_empty")]
    pub code: CustomCode,
    /// Formulaires du site reçus par l'hébergement (5.8).
    #[serde(default, skip_serializing_if = "FormSettings::is_default")]
    pub forms: FormSettings,
    /// Transition entre les pages du site (6.4) : `fade` (fondu), `slide` (glissement) ; `None` :
    /// aucune.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_transition: Option<String>,
    /// Défilement doux (D-110) : la molette fait glisser la page, ou le conteneur qui défile sous
    /// le pointeur, au lieu de sauter d'un cran (comme Lenis). Coupé pour les visiteurs qui
    /// demandent moins de mouvement.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub smooth_scroll: bool,
    /// Flux RSS des collections (abonnement aux actualités), produits à la génération.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feeds: Vec<FeedSettings>,
    /// Noms affichés des informations du site (chemin `address.city` -> « Ville ») donnés dans
    /// Paramètres › Informations du site.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub site_labels: BTreeMap<String, String>,
}

/// Flux RSS d'une collection : derniers contenus publiés (titre, adresse, date, résumé).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FeedSettings {
    /// Collection (identifiant).
    pub collection: String,
    /// Adresse du flux (`/feed.xml`).
    pub path: String,
    /// Titre du flux ; sinon le nom du site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Description du flux ; sinon celle du site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Page de la liste (adresse) ; sinon l'accueil.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// Nombre de contenus (20 par défaut).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// Formulaires du site (5.8) : les messages des `<form data-revel-form>` sont transmis par
/// courriel par un script PHP produit à la génération, sans service extérieur.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormSettings {
    /// Adresse qui reçoit les messages ; vide : formulaires non reçus.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub recipient: String,
    /// Objet des courriels (par défaut : « Nouveau message depuis <site> »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Message affiché après l'envoi (par défaut selon la langue du site).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<String>,
}

impl FormSettings {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Code ajouté tel quel aux pages HTML (statistiques, vérification d'un service, scripts) :
/// `head` avant `</head>`, `body` avant `</body>`. Celui du site d'abord, puis celui de la page.
/// Il n'est pas exécuté dans le canevas de l'éditeur (comme Webflow), seulement dans l'aperçu et
/// le site publié.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomCode {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub head: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
}

impl CustomCode {
    pub fn is_empty(&self) -> bool {
        self.head.trim().is_empty() && self.body.trim().is_empty()
    }
}

/// Plan du site (`sitemap.xml`) et `robots.txt` produits à la génération quand aucune page du site
/// n'a déjà ces adresses.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SeoSettings {
    /// Plan du site désactivé (produit par défaut).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_sitemap: bool,
    /// Plan du site : date de mise à jour de chaque page (`lastmod`), tirée de sa date de
    /// publication (champ `date` d'un contenu, réglage de la page) quand elle en a une.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sitemap_lastmod: bool,
    /// Texte de `robots.txt` ; `None` : tout autoriser et indiquer le plan du site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub robots: Option<String>,
    /// Nom affiché du site : ajouté au titre des pages (« Page | Nom »), nom au partage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site_title: Option<String>,
    /// Description des pages qui n'en ont pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Image de partage des pages qui n'en ont pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_image: Option<String>,
    /// Couleur de la barre du navigateur (téléphones).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_color: Option<String>,
    /// Icône du site (SVG ou PNG).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    /// Icône PNG de repli (192 px).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon_png: Option<String>,
    /// Icône des écrans d'accueil (Apple, 180 px).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub touch_icon: Option<String>,
    /// Fiche de l'organisation (données structurées schema.org de chaque page).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<Organization>,
}

/// Organisation qui publie le site (`schema.org/Organization`) ; champs vides : omis.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub alternate_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub logo: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub email: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub telephone: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub founding_date: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub street: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub postcode: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub city: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub country: String,
    /// Pages de l'organisation ailleurs (réseaux sociaux).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub same_as: Vec<String>,
}

impl SeoSettings {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Adresse d'une page acceptée par Revel : commence par `/`, segments en minuscules, chiffres,
/// `-`, `_`, `.` ; une page HTML finit par `/` (dossier) ou `.html`.
pub fn valid_page_path(path: &str) -> bool {
    let Some(rest) = path.strip_prefix('/') else {
        return false;
    };
    let segs: Vec<&str> = rest.split('/').collect();
    segs.iter().enumerate().all(|(i, s)| {
        let last = i + 1 == segs.len();
        (last && s.is_empty())
            || (!s.is_empty()
                && *s != "."
                && *s != ".."
                && s.chars().all(|c| {
                    c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.')
                }))
    }) && !path.contains("//")
}

/// Fragment d'adresse tiré d'un texte (`Nos offres d'été` -> `nos-offres-d-ete`).
pub fn slugify(text: &str) -> String {
    let mut out = String::new();
    for c in text
        .chars()
        .flat_map(|c| fold_accent(c).chars().collect::<Vec<_>>())
    {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

fn fold_accent(c: char) -> String {
    let base = match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "a",
        'ç' | 'Ç' => "c",
        'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => "e",
        'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => "i",
        'ñ' | 'Ñ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => "o",
        'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => "u",
        'ý' | 'ÿ' | 'Ý' => "y",
        'œ' | 'Œ' => "oe",
        'æ' | 'Æ' => "ae",
        'ß' => "ss",
        _ => return c.to_string(),
    };
    base.to_string()
}

/// Filtre défini par une expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomFilter {
    /// Noms des paramètres ; le premier reçoit la valeur filtrée.
    pub params: Vec<String>,
    pub expr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Une URL publiée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    /// Identifiant stable (ne change pas si l'URL change).
    pub id: String,
    /// Chemin public : `/agence/` (HTML, écrit dans `agence/index.html`) ou `/sitemap.xml`.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    /// Variables de la page (titre, SEO, classes du `body`…), accessibles par leur nom.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: BTreeMap<String, Value>,
    /// Variables calculées : nom -> expression (ex. `seo` -> `pages.agence.seo`),
    /// évaluées avec les données globales et les collections.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub computed: BTreeMap<String, String>,
    /// Contenu de la page.
    #[serde(default)]
    pub body: Vec<Node>,
    /// Brouillon : non publié.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub draft: bool,
    /// Nom de la page dans Revel (sans effet sur le site publié) ; `None` : titre ou adresse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Code ajouté à cette page, après celui du site (3.9).
    #[serde(default, skip_serializing_if = "CustomCode::is_empty")]
    pub code: CustomCode,
}

/// Gabarit partagé par plusieurs pages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub id: String,
    /// Gabarit parent (les gabarits s'emboîtent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Variables par défaut du gabarit.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: BTreeMap<String, Value>,
    /// Propriétés du gabarit : valeurs choisies par chaque page qui l'utilise (réglages de la
    /// page), lues dans le gabarit comme des variables, comme les propriétés d'un composant le
    /// sont à chaque utilisation. La valeur par défaut vaut pour les pages qui n'en donnent pas.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<PropDef>,
    pub body: Vec<Node>,
}

impl Layout {
    pub fn field(&self, name: &str) -> Option<&PropDef> {
        self.fields.iter().find(|p| p.name == name)
    }
}

/// Sous-arbre réutilisable. Il est rendu avec les variables de l'endroit où il est utilisé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    /// Nom affiché (panneau Composants) ; sinon la fin de l'identifiant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Propriétés : valeurs choisies à chaque utilisation (texte, image, lien, visibilité),
    /// lues dans le composant comme des variables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<PropDef>,
    /// Variantes (comme Framer) : états du composant au survol, à l'appui, ou choisis à chaque
    /// utilisation. Leurs styles sont des règles de la feuille du site préfixées par la classe de
    /// l'élément racine du composant (`.carte:hover .titre`, `.carte.is-sombre .titre`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<Variant>,
    pub body: Vec<Node>,
}

impl Component {
    pub fn new(id: impl Into<String>, body: Vec<Node>) -> Self {
        Self {
            id: id.into(),
            name: None,
            props: Vec::new(),
            variants: Vec::new(),
            body,
        }
    }

    /// Élément racine (premier élément du contenu) : porte la classe des variantes.
    pub fn root(&self) -> Option<&Node> {
        self.body.iter().find(|n| matches!(n, Node::Element { .. }))
    }

    /// Classe principale de l'élément racine (première classe écrite, sans expression).
    pub fn root_class(&self) -> Option<String> {
        let Some(Node::Element { attrs, .. }) = self.root() else {
            return None;
        };
        attrs
            .iter()
            .find(|a| a.name == "class" && a.when.is_none())
            .and_then(|a| a.value.as_deref())
            .filter(|v| !v.contains("{{") && !v.contains("{%"))
            .and_then(|v| v.split_whitespace().next())
            .map(str::to_string)
    }

    pub fn variant(&self, id: &str) -> Option<&Variant> {
        self.variants.iter().find(|v| v.id == id)
    }

    /// Contenu pour une utilisation : l'élément racine reçoit la classe de la variante choisie
    /// (`is-<id>`, variante personnalisée) et les attributs de l'utilisation (une classe s'ajoute,
    /// un autre attribut remplace) ; `None` : rien à poser, contenu tel quel.
    pub fn patched_body(&self, variant: Option<&str>, attrs: &[Attr]) -> Option<Vec<Node>> {
        let class = variant
            .and_then(|v| self.variant(v))
            .filter(|v| v.kind == VariantKind::Custom)
            .map(|v| format!("is-{}", v.id));
        if class.is_none() && attrs.is_empty() {
            return None;
        }
        let mut body = self.body.clone();
        let Some(Node::Element { attrs: list, .. }) =
            body.iter_mut().find(|n| matches!(n, Node::Element { .. }))
        else {
            return None;
        };
        let mut set = |name: &str, value: Option<String>| match list
            .iter_mut()
            .find(|a| a.name == name && a.when.is_none())
        {
            Some(a) if name == "class" => {
                let cur = a.value.take().unwrap_or_default();
                let add = value.unwrap_or_default();
                a.value = Some(format!("{} {add}", cur.trim()).trim().to_string());
            }
            Some(a) => a.value = value,
            None => list.push(Attr {
                name: name.to_string(),
                value,
                when: None,
            }),
        };
        if let Some(cl) = class {
            set("class", Some(cl));
        }
        for a in attrs {
            set(&a.name, a.value.clone());
        }
        Some(body)
    }

    /// Nom affiché : le nom donné, sinon la fin de l'identifiant (`partials/card` -> `card`).
    pub fn label(&self) -> String {
        self.name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| self.id.rsplit('/').next().unwrap_or(&self.id).to_string())
    }

    pub fn prop(&self, name: &str) -> Option<&PropDef> {
        self.props.iter().find(|p| p.name == name)
    }
}

/// Variante d'un composant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant {
    /// Identifiant (lettres minuscules, chiffres, tirets) : classe `is-<id>` d'une variante
    /// personnalisée ; `survol`, `appui` pour les gestes.
    pub id: String,
    /// Nom affiché ; sinon d'après le type ou l'identifiant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub kind: VariantKind,
}

/// Nature d'une variante.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariantKind {
    /// Choisie à chaque utilisation (classe `is-<id>` sur l'élément racine).
    #[default]
    Custom,
    /// Pointeur sur le composant (`:hover`).
    Hover,
    /// Appui ou clic en cours (`:active`).
    Pressed,
}

impl Variant {
    /// Suffixe du sélecteur de l'élément racine dans cette variante (`.is-sombre`, `:hover`).
    pub fn suffix(&self) -> String {
        match self.kind {
            VariantKind::Custom => format!(".is-{}", self.id),
            VariantKind::Hover => ":hover".into(),
            VariantKind::Pressed => ":active".into(),
        }
    }
}

/// Propriété d'un composant (comme les propriétés de composant de Webflow et Framer).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropDef {
    /// Nom dans les expressions du composant (`titre`) : lettres, chiffres, `_`.
    pub name: String,
    /// Libellé affiché ; sinon le nom.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub kind: PropKind,
    /// Valeur par défaut (texte, adresse, booléen), prise quand une utilisation n'en donne pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
}

impl PropDef {
    pub fn label(&self) -> &str {
        self.label
            .as_deref()
            .filter(|l| !l.trim().is_empty())
            .unwrap_or(&self.name)
    }
}

/// Type d'une propriété de composant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropKind {
    /// Texte d'un élément.
    Text,
    /// Adresse d'une image.
    Image,
    /// Adresse d'un lien.
    Link,
    /// Élément affiché ou masqué.
    Switch,
    /// Liste parcourue dans le composant (valeurs d'un champ, adresses de contenus reliés) :
    /// chaque utilisation la relie à un champ liste de son contenu.
    List,
}

/// Identifiant de variante valide : lettres minuscules, chiffres, tirets (classe `is-<id>`).
pub fn valid_variant_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 40
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.starts_with('-')
}

/// Nom de propriété valide (identifiant d'expression, pas un mot réservé).
pub fn valid_prop_name(name: &str) -> bool {
    let mut cs = name.chars();
    cs.next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && cs.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !matches!(
            name,
            "true"
                | "false"
                | "none"
                | "and"
                | "or"
                | "not"
                | "in"
                | "is"
                | "if"
                | "else"
                | "loop"
                | "page"
                | "site"
                | "collections"
                | "super"
                | "self"
        )
}

/// Collection de contenus structurés.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    /// Origine des entrées.
    #[serde(default, skip_serializing_if = "CollectionSource::is_folder")]
    pub source: CollectionSource,
    /// Dossier des fichiers Markdown, relatif au projet (ex. `content/projets`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub folder: String,
    /// Chemin public de chaque entrée, avec expressions entre `{{ }}`
    /// (ex. `/project/{{ slug }}/`). `None` : pas de page par entrée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Gabarit des pages d'entrée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    /// Variables communes à toutes les entrées.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: BTreeMap<String, Value>,
    /// Condition d'inclusion dans la collection publiée (ex. `published != false`),
    /// évaluée sur les données de l'entrée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// Tri des entrées.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<Sort>,
    /// Champs (schéma pour l'éditeur et pour Pages CMS).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
}

/// Origine des entrées d'une collection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionSource {
    /// Fichiers Markdown du dossier `folder`.
    #[default]
    Folder,
    /// Toutes les pages publiées du site (pour un plan du site, par exemple).
    Pages,
}

impl CollectionSource {
    pub fn is_folder(&self) -> bool {
        *self == Self::Folder
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sort {
    /// Champ de tri (ex. `date`, `order`).
    pub field: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub descending: bool,
    /// Type de comparaison.
    #[serde(default)]
    pub kind: SortKind,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortKind {
    #[default]
    Text,
    Number,
    Date,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub kind: FieldKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldKind {
    String,
    Text,
    RichText,
    Image,
    Date,
    Boolean,
    Number,
    List,
    Object,
    /// Référence à une entrée d'une autre collection (par son `slug`).
    Reference {
        collection: String,
    },
    /// Liste de références.
    References {
        collection: String,
    },
}

/// Copie de fichiers statiques.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Passthrough {
    /// Source relative au projet (dossier ou fichier).
    pub from: String,
    /// Destination relative à la racine publiée (`""` = racine).
    pub to: String,
}

/// Nœud de l'arbre d'éléments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "node", rename_all = "snake_case")]
pub enum Node {
    /// Élément HTML. Les valeurs d'attributs peuvent contenir des expressions `{{ … }}`.
    Element {
        /// Identifiant stable pour l'éditeur et les commandes (facultatif).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        tag: String,
        /// Attributs dans l'ordre d'origine.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        attrs: Vec<Attr>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<Node>,
    },
    /// Texte littéral (échappé à la génération).
    Text { text: String },
    /// HTML littéral (non échappé) : commentaires, doctype, balisage non reconnu.
    Html { html: String },
    /// Valeur d'une expression. Échappée sauf si la valeur est marquée sûre (`| safe`).
    Out { expr: String },
    /// Affichage conditionnel.
    If {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        cond: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        then: Vec<Node>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        otherwise: Vec<Node>,
    },
    /// Répétition sur une liste. `loop.index`, `loop.first`, `loop.last` sont disponibles.
    Each {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        expr: String,
        /// Nom de la variable d'élément (ou `cle, valeur` pour un dictionnaire).
        item: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<Node>,
        /// Contenu affiché si la liste est vide.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        empty: Vec<Node>,
    },
    /// Affectation d'une variable (visible dans la suite du bloc courant).
    Set { name: String, expr: String },
    /// Affectation par bloc : la variable reçoit le rendu des enfants.
    SetBlock {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        name: String,
        children: Vec<Node>,
    },
    /// Emplacement du contenu de la page dans un gabarit.
    Slot,
    /// Utilisation d'un composant.
    Use {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        component: String,
        /// Valeurs des propriétés du composant pour cette utilisation (expressions évaluées là
        /// où le composant est utilisé : `"Titre"`, `p.data.title`, `true`).
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        args: BTreeMap<String, String>,
        /// Variante personnalisée choisie (identifiant) ; aucune : variante principale.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        variant: Option<String>,
        /// Attributs posés sur l'élément racine du composant pour cette utilisation (animations
        /// `data-revel-*`).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        attrs: Vec<Attr>,
        /// Contenu propre à cette utilisation, placé dans l'emplacement du composant (élément
        /// `data-revel-slot`, D-118) ; vide : contenu par défaut de l'emplacement.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<Node>,
    },
    /// Image responsive (variantes AVIF/WebP/JPEG générées).
    Image(ImageNode),
}

/// Attribut HTML. `value: None` = attribut booléen (`defer`, `hidden`…).
/// La valeur peut contenir des expressions `{{ … }}` (et des `{% if %}` simples).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attr {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Condition : l'attribut n'est écrit que si l'expression est vraie.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

/// Paramètres d'une image ; chaque champ est une expression.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImageNode {
    /// Identifiant stable pour l'éditeur et les commandes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub src: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratio: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loading: Option<String>,
    /// Largeurs imposées, séparées par des virgules (ex. `"160,320"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widths: Option<String>,
    /// N'affiche rien si la source est vide (au lieu d'un visuel de remplacement).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Nom du calque donné par l'utilisateur (panneau Calques).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Calque masqué : absent du site publié, invisible dans le canevas.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
}

/// Attribut du nom de calque donné par l'utilisateur (élément, utilisation de composant) ; jamais
/// écrit dans les pages.
pub const LAYER_NAME_ATTR: &str = "data-revel-name";
/// Attribut de l'emplacement d'un composant (D-118) : élément du composant dont le contenu est,
/// pour chaque utilisation, celui qu'elle donne (sinon le contenu de l'élément, par défaut).
pub const SLOT_ATTR: &str = "data-revel-slot";
/// Attribut d'un calque masqué : l'élément n'est pas écrit dans le site publié, le canevas et
/// l'aperçu le cachent.
pub const HIDDEN_ATTR: &str = "data-revel-hidden";

/// `{{ content | safe }}` (ou `{{ content }}`) d'un gabarit -> emplacement de la page : la page
/// apparaît dans les calques à cet endroit.
pub fn content_to_slot(nodes: &mut [Node]) {
    for n in nodes {
        match n {
            Node::Out { expr } => {
                let e: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
                if e == "content|safe" || e == "content" {
                    *n = Node::Slot;
                }
            }
            Node::Element { children, .. } | Node::SetBlock { children, .. } => {
                content_to_slot(children)
            }
            Node::If {
                then, otherwise, ..
            } => {
                content_to_slot(then);
                content_to_slot(otherwise);
            }
            Node::Each {
                children, empty, ..
            } => {
                content_to_slot(children);
                content_to_slot(empty);
            }
            _ => {}
        }
    }
}

/// Nom de calque donné par l'utilisateur.
pub fn layer_name(n: &Node) -> Option<&str> {
    match n {
        Node::Element { attrs, .. } | Node::Use { attrs, .. } => attrs
            .iter()
            .find(|a| a.name == LAYER_NAME_ATTR)
            .and_then(|a| a.value.as_deref()),
        Node::Image(img) => img.name.as_deref(),
        _ => None,
    }
    .map(str::trim)
    .filter(|v| !v.is_empty())
}

/// Calque masqué par l'utilisateur.
pub fn is_hidden(n: &Node) -> bool {
    match n {
        Node::Element { attrs, .. } | Node::Use { attrs, .. } => attrs
            .iter()
            .any(|a| a.name == HIDDEN_ATTR && a.when.is_none()),
        Node::Image(img) => img.hidden,
        _ => false,
    }
}

/// Point de rupture : largeur d'écran à partir de laquelle des styles différents s'appliquent.
/// Cascade comme Webflow : la base (sans largeur) s'applique partout ; un point `max_width`
/// modifie les écrans plus étroits ; un point `min_width` les écrans plus larges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: String,
    /// Largeur maximale en pixels (`@media (max-width: …)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_width: Option<u32>,
    /// Largeur minimale en pixels (`@media (min-width: …)`), pour les grands écrans.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_width: Option<u32>,
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
            min_width: None,
        })
        .collect()
    }

    /// Requête média de ce point de rupture (`None` pour la base).
    pub fn media(&self) -> Option<String> {
        match (self.max_width, self.min_width) {
            (Some(w), _) => Some(format!("(max-width: {w}px)")),
            (None, Some(w)) => Some(format!("(min-width: {w}px)")),
            (None, None) => None,
        }
    }
}

/// Feuille de style du site (D-18). Les éléments sont dans l'ordre de la cascade CSS.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stylesheet {
    /// Adresse publiée du CSS généré (ex. `/assets/css/site.css`).
    pub url: String,
    /// Fichier d'origine importé, relatif au projet : il n'est plus copié tel quel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default)]
    pub items: Vec<StyleItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "item", rename_all = "snake_case")]
pub enum StyleItem {
    Rule(StyleRule),
    /// Commentaire, sans délimiteurs ; `media` : à l'intérieur d'un bloc `@media`.
    Comment {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        media: Option<String>,
    },
    /// Bloc conservé tel quel : `@font-face`, `@keyframes`, `@import`, `@supports`…
    Raw {
        css: String,
    },
}

/// Règle : sélecteur, requête média facultative, déclarations dans l'ordre.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StyleRule {
    /// Identifiant stable pour l'éditeur et les commandes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Sélecteur tel qu'écrit (espaces normalisés), ex. `.button:hover`.
    pub selector: String,
    /// Requête média sans `@media`, ex. `(max-width: 860px)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    #[serde(default)]
    pub decls: Vec<Decl>,
}

/// Déclaration CSS ; la valeur est conservée telle qu'écrite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decl {
    pub prop: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub important: bool,
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

/// Raccourcis de construction (tests, importateurs).
pub mod build {
    use super::*;

    pub fn el(tag: &str, attrs: &[(&str, &str)], children: Vec<Node>) -> Node {
        Node::Element {
            id: None,
            tag: tag.into(),
            attrs: attrs
                .iter()
                .map(|(n, v)| Attr {
                    name: (*n).into(),
                    value: Some((*v).into()),
                    when: None,
                })
                .collect(),
            children,
        }
    }

    pub fn text(t: &str) -> Node {
        Node::Text { text: t.into() }
    }

    pub fn out(expr: &str) -> Node {
        Node::Out { expr: expr.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::build::*;
    use super::*;

    #[test]
    fn aller_retour_json() {
        let mut site = Site::new("Alunites", "fr");
        site.pages.push(Page {
            id: "accueil".into(),
            path: "/".into(),
            layout: Some("base".into()),
            props: [("title".to_string(), Value::from("Accueil"))].into(),
            body: vec![
                el("h1", &[("class", "titre")], vec![out("title")]),
                Node::Each {
                    id: None,
                    expr: "collections.projets | limit(3)".into(),
                    item: "item".into(),
                    children: vec![el("a", &[("href", "{{ item.url }}")], vec![text("Voir")])],
                    empty: vec![],
                },
            ],
            computed: Default::default(),
            draft: false,
            code: Default::default(),
            name: None,
        });
        site.layouts.push(Layout {
            fields: Vec::new(),
            id: "base".into(),
            parent: None,
            props: BTreeMap::new(),
            body: vec![el("main", &[], vec![Node::Slot])],
        });
        site.ensure_ids();
        let json = site.to_json().unwrap();
        assert_eq!(Site::from_json(&json).unwrap(), site);
        assert!(site.layout("base").is_some());
    }
}
