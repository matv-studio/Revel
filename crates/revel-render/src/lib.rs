//! `revel-render` : génération du site statique à partir du modèle.
//!
//! - Les expressions sont évaluées par [minijinja](https://docs.rs/minijinja) (syntaxe Jinja).
//! - Les variables suivent la cascade : données globales < gabarits < collection < page.
//! - `{% set %}` met à jour la variable existante la plus proche (sinon la crée localement),
//!   comme Nunjucks quand une liste est complétée dans une boucle.
//! - Les images sont confiées à un [`AssetHooks`] (variantes générées par `revel-assets`).

use chrono::{DateTime, Utc};
use minijinja::Value;
use revel_model::{CollectionSource, Node, Site, SortKind};
use std::collections::BTreeMap;
use std::sync::Arc;

mod eval;
mod html;
mod typography;

pub use html::escape;

/// Entrée de collection lue sur disque (fichier Markdown avec front matter).
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Dossier d'origine relatif au projet (ex. `content/projets`).
    pub folder: String,
    /// Nom du fichier sans extension.
    pub file_stem: String,
    /// Front matter (objet JSON).
    pub data: serde_json::Value,
    /// Corps Markdown.
    pub body: String,
}

/// Paramètres d'image évalués.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageArgs {
    pub src: String,
    pub alt: String,
    pub sizes: String,
    pub class: String,
    pub ratio: String,
    pub loading: String,
    pub widths: Vec<u32>,
}

/// Accès aux médias pendant la génération.
pub trait AssetHooks: Send + Sync {
    /// HTML de l'image, ou `None` si le fichier n'existe pas (le composant
    /// `image-placeholder` est alors utilisé s'il existe).
    fn image(&self, args: &ImageArgs) -> Option<String>;
    fn media_exists(&self, src: &str) -> bool;
    /// URL absolue d'une image de partage (réseaux sociaux).
    fn share_image(&self, src: &str, base: &str) -> String;
}

/// Médias sans traitement : `<img>` simple, fichiers supposés présents.
#[derive(Debug, Default)]
pub struct PlainAssets;

impl AssetHooks for PlainAssets {
    fn image(&self, a: &ImageArgs) -> Option<String> {
        if a.src.is_empty() {
            return None;
        }
        let mut s = format!("<img src=\"{}\" alt=\"{}\"", escape(&a.src), escape(&a.alt));
        if !a.class.is_empty() {
            s.push_str(&format!(" class=\"{}\"", escape(&a.class)));
        }
        s.push_str(&format!(
            " loading=\"{}\" decoding=\"async\">",
            if a.loading.is_empty() {
                "lazy"
            } else {
                &a.loading
            }
        ));
        Some(s)
    }

    fn media_exists(&self, _src: &str) -> bool {
        true
    }

    fn share_image(&self, src: &str, base: &str) -> String {
        eval::absolute_url(src, base)
    }
}

pub struct BuildInput<'a> {
    pub site: &'a Site,
    pub entries: Vec<Entry>,
    /// Nom de l'environnement (`production`, `staging`…), clé de `settings.environments`.
    pub environment: String,
    /// Identifiant de version (paramètre de cache des CSS/JS).
    pub version: String,
    pub hooks: Arc<dyn AssetHooks>,
    /// Date de génération (fonction `now()`).
    pub now: DateTime<Utc>,
}

/// Fichier produit, chemin relatif à la racine publiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct BuildOutput {
    pub files: Vec<OutputFile>,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("chemin de page invalide : {0}")]
    InvalidPath(String),
    #[error("{context} : expression `{expr}` : {msg}")]
    Expr {
        context: String,
        expr: String,
        msg: String,
    },
    #[error("gabarit introuvable : {0}")]
    MissingLayout(String),
    #[error("{0}")]
    Markup(String),
}

/// Type de document produit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    Html,
    Xml,
    Text,
}

impl DocKind {
    pub fn of_path(path: &str) -> Self {
        let last = path.rsplit('/').next().unwrap_or("");
        if path.ends_with('/') || last.ends_with(".html") || last.ends_with(".htm") {
            Self::Html
        } else if [".xml", ".rss", ".atom", ".svg"]
            .iter()
            .any(|e| last.ends_with(e))
        {
            Self::Xml
        } else {
            Self::Text
        }
    }
}

/// `/agence/` -> `agence/index.html` ; `/` -> `index.html` ; `/sitemap.xml` -> `sitemap.xml`.
pub fn output_path(path: &str) -> Result<String, RenderError> {
    if !path.starts_with('/') || path.split('/').any(|s| s == "..") {
        return Err(RenderError::InvalidPath(path.into()));
    }
    let trimmed = &path[1..];
    Ok(if trimmed.is_empty() || trimmed.ends_with('/') {
        format!("{trimmed}index.html")
    } else {
        trimmed.to_string()
    })
}

/// Entrée de collection en cours de constitution : données, valeur, page éventuelle, nom de fichier.
type CollectedItem<'s> = (BTreeMap<String, Value>, Value, Option<Job<'s>>, String);

/// Page à produire (page fixe ou entrée de collection).
struct Job<'s> {
    url: String,
    layout: Option<String>,
    data: BTreeMap<String, Value>,
    body: Option<&'s [Node]>,
    /// HTML du corps Markdown (entrées de collection).
    content: Option<String>,
    file_slug: String,
}

/// Génère tout le site.
pub fn build(input: &BuildInput) -> Result<BuildOutput, RenderError> {
    let site = input.site;
    let b = eval::Builder::new(input);
    let mut warnings = Vec::new();

    // 1. Collections issues de dossiers.
    let mut collections: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut jobs: Vec<Job> = Vec::new();
    let base_globals = b.base_globals();
    for col in site
        .collections
        .iter()
        .filter(|c| c.source == CollectionSource::Folder)
    {
        let mut items: Vec<CollectedItem> = Vec::new();
        for e in input.entries.iter().filter(|e| e.folder == col.folder) {
            let mut data: BTreeMap<String, Value> = col
                .props
                .iter()
                .map(|(k, v)| (k.clone(), crate::eval::from_json(v)))
                .collect();
            if let serde_json::Value::Object(m) = &e.data {
                for (k, v) in m {
                    data.insert(k.clone(), crate::eval::from_json(v));
                }
            }
            let scope = eval::Scope::new(base_globals.clone(), data.clone());
            if let Some(f) = &col.filter
                && !b
                    .eval(&scope, f, &format!("filtre de {}", col.id))?
                    .is_true()
            {
                continue;
            }
            let url = match &col.path {
                Some(p) => Some(b.render_inline(&scope, p, &format!("chemin de {}", col.id))?),
                None => None,
            };
            let content = markdown(&e.body);
            let date = data.get("date").cloned().unwrap_or(Value::UNDEFINED);
            let mut item_map: BTreeMap<String, Value> = BTreeMap::new();
            item_map.insert(
                "url".into(),
                url.clone().map(Value::from).unwrap_or(Value::from(false)),
            );
            item_map.insert("data".into(), Value::from(data.clone()));
            item_map.insert("content".into(), Value::from_safe_string(content.clone()));
            item_map.insert("fileSlug".into(), Value::from(e.file_stem.clone()));
            item_map.insert("date".into(), date);
            let job = url.map(|url| Job {
                url,
                layout: data
                    .get("layout")
                    .and_then(|v| v.as_str().map(str::to_string))
                    .or_else(|| col.layout.clone()),
                data: data.clone(),
                body: None,
                content: Some(content),
                file_slug: e.file_stem.clone(),
            });
            items.push((data, Value::from(item_map), job, e.file_stem.clone()));
        }
        // Ordre de base (comme Eleventy) : date croissante, puis nom de fichier ;
        // le tri de la collection s'applique ensuite (tri stable).
        items.sort_by(|a, b| {
            let da =
                a.0.get("date")
                    .and_then(|v| eval::parse_date(&v.to_string()));
            let db =
                b.0.get("date")
                    .and_then(|v| eval::parse_date(&v.to_string()));
            match (da, db) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
            .then_with(|| a.3.cmp(&b.3))
        });
        if let Some(sort) = &col.sort {
            items.sort_by(|a, b| {
                let ord = compare(a.0.get(&sort.field), b.0.get(&sort.field), sort.kind);
                if sort.descending { ord.reverse() } else { ord }
            });
        }
        let mut list = Vec::with_capacity(items.len());
        for (_, item, job, _) in items {
            list.push(item);
            if let Some(j) = job {
                jobs.push(j);
            }
        }
        collections.insert(col.id.clone(), list);
    }

    // 2. Pages fixes.
    let partial_globals = b.globals(collections.clone());
    for p in site.pages.iter().filter(|p| !p.draft) {
        let mut data: BTreeMap<String, Value> = BTreeMap::new();
        for l in b.layout_chain(p.layout.as_deref())?.iter().rev() {
            for (k, v) in &l.props {
                data.insert(k.clone(), crate::eval::from_json(v));
            }
        }
        for (k, v) in &p.props {
            data.insert(k.clone(), crate::eval::from_json(v));
        }
        for (k, e) in &p.computed {
            let scope = eval::Scope::new(partial_globals.clone(), data.clone());
            let v = b.eval(&scope, e, &format!("variable calculée {k} de {}", p.path))?;
            data.insert(k.clone(), v);
        }
        jobs.push(Job {
            url: p.path.clone(),
            layout: p.layout.clone(),
            data,
            body: Some(&p.body),
            content: None,
            file_slug: p.id.rsplit('/').next().unwrap_or(&p.id).to_string(),
        });
    }

    // 3. Collections de pages (plan du site…).
    for col in site
        .collections
        .iter()
        .filter(|c| c.source == CollectionSource::Pages)
    {
        let mut list = Vec::new();
        let mut ordered: Vec<&Job> = jobs.iter().collect();
        ordered.sort_by(|a, b| a.url.cmp(&b.url));
        for j in ordered {
            let mut m: BTreeMap<String, Value> = BTreeMap::new();
            m.insert("url".into(), Value::from(j.url.clone()));
            m.insert("data".into(), Value::from(j.data.clone()));
            m.insert(
                "date".into(),
                j.data.get("date").cloned().unwrap_or(Value::UNDEFINED),
            );
            let item = Value::from(m.clone());
            if let Some(f) = &col.filter {
                let scope = eval::Scope::new(base_globals.clone(), m);
                if !b
                    .eval(&scope, f, &format!("filtre de {}", col.id))?
                    .is_true()
                {
                    continue;
                }
            }
            list.push(item);
        }
        collections.insert(col.id.clone(), list);
    }

    // 4. Rendu.
    let globals = b.globals(collections);
    let mut files = Vec::new();
    for j in &jobs {
        let kind = DocKind::of_path(&j.url);
        let html = b.render_job(&globals, j, kind, &mut warnings)?;
        let html = if kind == DocKind::Html && site.settings.typography.as_deref() == Some("fr") {
            typography::french(&html)
        } else {
            html
        };
        files.push(OutputFile {
            path: output_path(&j.url)?,
            bytes: html.into_bytes(),
        });
    }
    warnings.extend(b.take_warnings());
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(BuildOutput { files, warnings })
}

fn compare(a: Option<&Value>, b: Option<&Value>, kind: SortKind) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => match kind {
            SortKind::Number => {
                let x = f64::try_from(a.clone()).unwrap_or(0.0);
                let y = f64::try_from(b.clone()).unwrap_or(0.0);
                x.partial_cmp(&y).unwrap_or(Ordering::Equal)
            }
            SortKind::Date => {
                let x = eval::parse_date(&a.to_string());
                let y = eval::parse_date(&b.to_string());
                x.cmp(&y)
            }
            SortKind::Text => a.to_string().cmp(&b.to_string()),
        },
    }
}

/// Markdown -> HTML (CommonMark, tableaux, texte barré ; HTML brut autorisé).
pub fn markdown(src: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};
    if src.trim().is_empty() {
        return String::new();
    }
    let mut out = String::new();
    html::push_html(
        &mut out,
        Parser::new_ext(src, Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH),
    );
    out
}

#[cfg(test)]
mod tests;
