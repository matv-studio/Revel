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
use std::collections::BTreeMap;

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
}

impl Library {
    pub fn is_empty(&self) -> bool {
        self.media.is_empty()
            && self.components.is_empty()
            && self.pages.is_empty()
            && self.alt.is_empty()
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
}

impl Shelf {
    pub fn is_empty(&self) -> bool {
        self.folders.is_empty() && self.items.is_empty()
    }
}

impl Site {
    pub fn new(name: impl Into<String>, default_locale: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: name.into(),
            default_locale: default_locale.into(),
            base_url: None,
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
}

/// Plan du site (`sitemap.xml`) et `robots.txt` produits à la génération quand aucune page du site
/// n'a déjà ces adresses.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SeoSettings {
    /// Plan du site désactivé (produit par défaut).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_sitemap: bool,
    /// Texte de `robots.txt` ; `None` : tout autoriser et indiquer le plan du site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub robots: Option<String>,
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
    pub body: Vec<Node>,
}

/// Sous-arbre réutilisable. Il est rendu avec les variables de l'endroit où il est utilisé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub body: Vec<Node>,
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
            name: None,
        });
        site.layouts.push(Layout {
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
