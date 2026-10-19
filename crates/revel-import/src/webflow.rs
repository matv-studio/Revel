//! Contenus du CMS de Webflow (7.3), en plus de l'export du code importé comme un dossier
//! (7.1) : chaque collection devient une collection Revel (fichiers Markdown dans
//! `content/<collection>`, décrite dans `.pages.yml`), lue
//!
//! - dans les **fichiers CSV** exportés depuis Webflow (panneau CMS › collection › Export),
//!   types des champs déduits des valeurs ;
//! - ou par l'**API de Webflow** (jeton d'accès du site, jamais gardé), types des champs exacts,
//!   brouillons compris.
//!
//! Images des contenus téléchargées dans le site (`images/cms/`). La page modèle d'une
//! collection exportée par Webflow (`detail_<collection>.html`, sans contenu) devient le
//! gabarit de ses pages : éléments liés aux champs (`w-dyn-bind-empty`) reliés aux champs de la
//! collection dans l'ordre, à vérifier.

use crate::Report;
use crate::crawl::{Fetch, get_json};
use revel_model::{Attr, Layout, Node, Site};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use url::Url;

/// Type d'un champ du CMS de Webflow.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    PlainText,
    RichText,
    Image,
    MultiImage,
    Link,
    Number,
    Date,
    Switch,
    Option,
    File,
    /// Référence à une autre collection (identifiant Webflow ou nom de la collection).
    Reference(String),
    MultiReference(String),
}

#[derive(Debug, Clone)]
pub struct Field {
    /// Nom dans l'en-tête des contenus (`post_body`).
    pub key: String,
    pub label: String,
    pub kind: Kind,
}

#[derive(Debug, Clone, Default)]
pub struct Item {
    /// Identifiant Webflow (références).
    pub id: String,
    pub slug: String,
    pub draft: bool,
    /// Valeurs par champ (`key`), images en adresses.
    pub values: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct Collection {
    pub id: String,
    /// Adresse des pages de la collection dans Webflow (`blog-posts`).
    pub slug: String,
    pub label: String,
    pub fields: Vec<Field>,
    pub items: Vec<Item>,
}

/// `Post Body`, `post-body` → `post_body`.
fn key_of(name: &str) -> String {
    let s = revel_model::slugify(name).replace('-', "_");
    if s.is_empty() { "champ".into() } else { s }
}

// -------------------------------------------------------------------------------------------------
// CSV

/// Lignes d'un texte CSV (virgules, guillemets doublés, retours à la ligne entre guillemets).
pub fn csv_rows(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    cell.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                cell.push(c);
            }
            continue;
        }
        match c {
            '"' if cell.is_empty() => quoted = true,
            ',' => row.push(std::mem::take(&mut cell)),
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut cell));
                if !(row.len() == 1 && row[0].is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            c => cell.push(c),
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows
}

/// Colonnes propres à Webflow (pas des champs).
const SYSTEM: &[&str] = &[
    "collection id",
    "locale id",
    "item id",
    "archived",
    "draft",
    "created on",
    "updated on",
    "published on",
];

/// Le fichier CSV vient du CMS de Webflow (colonnes `Item ID` et `Slug`).
pub fn is_webflow_csv(text: &str) -> bool {
    let first = text.lines().next().unwrap_or("").to_ascii_lowercase();
    first.contains("item id") && first.contains("slug")
}

/// Nom de la collection d'après le nom du fichier exporté (`Site - Blog Posts - 64f….csv`).
fn csv_label(file: &str) -> String {
    let stem = file.trim_end_matches(".csv").trim_end_matches(".CSV");
    let parts: Vec<&str> = stem.split(" - ").collect();
    match parts.len() {
        n if n >= 3 => parts[n - 2].trim().to_string(),
        2 => parts[1].trim().to_string(),
        _ => stem.trim().to_string(),
    }
}

fn is_image_url(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    let path = l.split(['?', '#']).next().unwrap_or("");
    (l.starts_with("http") || l.starts_with("//"))
        && [".jpg", ".jpeg", ".png", ".webp", ".avif", ".gif", ".svg"]
            .iter()
            .any(|e| path.ends_with(e))
}

/// Date au format écrit par Webflow (`Mon Jan 15 2024 00:00:00 GMT+0000 (…)`) ou ISO →
/// `AAAA-MM-JJ` (avec l'heure si elle n'est pas minuit).
fn date_of(s: &str) -> Option<String> {
    let t = s.trim();
    if t.len() >= 10 && t.as_bytes()[4] == b'-' && t.as_bytes()[7] == b'-' {
        let d = &t[..10];
        let time = t.get(11..19).filter(|x| *x != "00:00:00");
        return Some(match time {
            Some(h) => format!("{d}T{h}"),
            None => d.to_string(),
        });
    }
    let p: Vec<&str> = t.split_whitespace().collect();
    if p.len() >= 5 && p[4].contains(':') {
        const M: [&str; 12] = [
            "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
        ];
        let m = M
            .iter()
            .position(|m| p[1].to_ascii_lowercase().starts_with(m))?
            + 1;
        let d: u32 = p[2].parse().ok()?;
        let y: u32 = p[3].parse().ok()?;
        let date = format!("{y:04}-{m:02}-{d:02}");
        return Some(if p[4] == "00:00:00" {
            date
        } else {
            format!("{date}T{}", p[4])
        });
    }
    None
}

/// Type d'une colonne d'après ses valeurs.
fn infer(values: &[&str]) -> Kind {
    let vals: Vec<&str> = values
        .iter()
        .copied()
        .filter(|v| !v.trim().is_empty())
        .collect();
    if vals.is_empty() {
        return Kind::PlainText;
    }
    let all = |f: &dyn Fn(&str) -> bool| vals.iter().all(|v| f(v));
    if all(&|v| v == "true" || v == "false") {
        return Kind::Switch;
    }
    if all(&|v| date_of(v).is_some()) {
        return Kind::Date;
    }
    if all(&|v| v.trim().parse::<f64>().is_ok()) {
        return Kind::Number;
    }
    if all(&is_image_url) {
        return Kind::Image;
    }
    if all(&|v| {
        v.split(';')
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .all(is_image_url)
    }) {
        return Kind::MultiImage;
    }
    if vals.iter().any(|v| {
        let t = v.trim_start();
        t.starts_with('<') && t.contains("</")
    }) {
        return Kind::RichText;
    }
    if all(&|v| v.starts_with("http://") || v.starts_with("https://") || v.starts_with("mailto:")) {
        return Kind::Link;
    }
    Kind::PlainText
}

/// Collection lue dans un fichier CSV du CMS de Webflow (`file` : nom du fichier).
pub fn from_csv(file: &str, text: &str) -> Option<Collection> {
    let rows = csv_rows(text);
    let header = rows.first()?;
    let lower: Vec<String> = header
        .iter()
        .map(|h| h.trim().to_ascii_lowercase())
        .collect();
    let col = |n: &str| lower.iter().position(|h| h == n);
    let slug_i = col("slug")?;
    let id_i = col("item id");
    let draft_i = col("draft");
    let archived_i = col("archived");
    let label = csv_label(file);
    let mut fields = Vec::new();
    let mut idx = Vec::new();
    for (i, h) in header.iter().enumerate() {
        if SYSTEM.contains(&lower[i].as_str()) || i == slug_i || h.trim().is_empty() {
            continue;
        }
        let values: Vec<&str> = rows[1..]
            .iter()
            .map(|r| r.get(i).map(String::as_str).unwrap_or(""))
            .collect();
        let kind = if lower[i] == "name" {
            Kind::PlainText
        } else {
            infer(&values)
        };
        let mut key = key_of(h);
        while fields.iter().any(|f: &Field| f.key == key) || key == "slug" {
            key.push('_');
        }
        fields.push(Field {
            key,
            label: h.trim().to_string(),
            kind,
        });
        idx.push(i);
    }
    let mut items = Vec::new();
    for r in &rows[1..] {
        let get = |i: usize| r.get(i).map(|s| s.trim()).unwrap_or("");
        if archived_i.is_some_and(|a| get(a) == "true") {
            continue;
        }
        let slug = get(slug_i).to_string();
        if slug.is_empty() {
            continue;
        }
        let mut values = Map::new();
        for (f, &i) in fields.iter().zip(&idx) {
            let raw = r.get(i).map(String::as_str).unwrap_or("");
            if raw.trim().is_empty() {
                continue;
            }
            let v = match f.kind {
                Kind::Switch => Value::Bool(raw.trim() == "true"),
                Kind::Number => {
                    serde_json::from_str(raw.trim()).unwrap_or(Value::String(raw.into()))
                }
                Kind::Date => Value::String(date_of(raw).unwrap_or_else(|| raw.into())),
                Kind::MultiImage => Value::Array(
                    raw.split(';')
                        .map(str::trim)
                        .filter(|x| !x.is_empty())
                        .map(|x| Value::String(x.into()))
                        .collect(),
                ),
                _ => Value::String(raw.to_string()),
            };
            values.insert(f.key.clone(), v);
        }
        items.push(Item {
            id: id_i.map(|i| get(i).to_string()).unwrap_or_default(),
            slug,
            draft: draft_i.is_some_and(|d| get(d) == "true"),
            values,
        });
    }
    Some(Collection {
        id: String::new(),
        slug: revel_model::slugify(&label),
        label,
        fields,
        items,
    })
}

/// Références entre collections lues dans des fichiers CSV : colonne dont les valeurs sont des
/// noms (ou adresses) de contenus d'une autre collection.
pub fn link_csv_references(cols: &mut [Collection]) {
    let names: Vec<(String, HashMap<String, String>)> = cols
        .iter()
        .map(|c| {
            let mut m = HashMap::new();
            for it in &c.items {
                m.insert(it.slug.clone(), it.slug.clone());
                if let Some(n) = it.values.get("name").and_then(Value::as_str) {
                    m.insert(n.to_string(), it.slug.clone());
                }
            }
            (c.slug.clone(), m)
        })
        .collect();
    for c in cols.iter_mut() {
        for f in c
            .fields
            .iter_mut()
            .filter(|f| f.kind == Kind::PlainText && f.key != "name")
        {
            let vals: Vec<&str> = c
                .items
                .iter()
                .filter_map(|i| i.values.get(&f.key).and_then(Value::as_str))
                .collect();
            if vals.is_empty() {
                continue;
            }
            for (target, map) in &names {
                if *target == c.slug {
                    continue;
                }
                let parts = |v: &str| -> Vec<String> {
                    v.split([',', ';'])
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect()
                };
                if vals
                    .iter()
                    .all(|v| parts(v).iter().all(|p| map.contains_key(p)))
                {
                    let multi = vals.iter().any(|v| parts(v).len() > 1);
                    f.kind = if multi {
                        Kind::MultiReference(target.clone())
                    } else {
                        Kind::Reference(target.clone())
                    };
                    let key = f.key.clone();
                    for it in c.items.iter_mut() {
                        if let Some(v) = it
                            .values
                            .get(&key)
                            .and_then(Value::as_str)
                            .map(str::to_string)
                        {
                            let slugs: Vec<Value> = parts(&v)
                                .iter()
                                .filter_map(|p| map.get(p).cloned().map(Value::String))
                                .collect();
                            it.values.insert(
                                key.clone(),
                                if multi {
                                    Value::Array(slugs)
                                } else {
                                    slugs.into_iter().next().unwrap_or(Value::Null)
                                },
                            );
                        }
                    }
                    break;
                }
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// API

/// Erreur de l'API de Webflow.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("jeton de l'API de Webflow refusé")]
    Token,
    #[error("API de Webflow : {0}")]
    Other(String),
}

const API: &str = "https://api.webflow.com/v2";

/// Collections du site `site_id` (attribut `data-wf-site` des pages exportées) lues par l'API
/// de Webflow (jeton du site, droit `cms:read`), contenus en brouillon compris.
pub fn from_api(
    fetch: &dyn Fetch,
    site_id: &str,
    token: &str,
) -> Result<Vec<Collection>, ApiError> {
    let auth = format!("Bearer {}", token.trim());
    let headers = [
        ("Authorization", auth.as_str()),
        ("accept", "application/json"),
    ];
    let get = |url: &str| {
        get_json(fetch, url, &headers).map_err(|(status, msg)| match status {
            401 | 403 => ApiError::Token,
            _ => ApiError::Other(format!("{status} {msg}")),
        })
    };
    let list = get(&format!("{API}/sites/{site_id}/collections"))?;
    let mut out = Vec::new();
    for c in list["collections"].as_array().into_iter().flatten() {
        let Some(id) = c["id"].as_str() else { continue };
        let detail = get(&format!("{API}/collections/{id}"))?;
        let mut fields = Vec::new();
        let mut options: HashMap<String, HashMap<String, String>> = HashMap::new();
        for f in detail["fields"].as_array().into_iter().flatten() {
            let slug = f["slug"].as_str().unwrap_or("");
            if slug.is_empty() || slug == "slug" {
                continue;
            }
            let target = f["validations"]["collectionId"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let kind = match f["type"].as_str().unwrap_or("") {
                "RichText" => Kind::RichText,
                "Image" => Kind::Image,
                "MultiImage" => Kind::MultiImage,
                "Link" | "VideoLink" | "Email" | "Phone" | "Color" => Kind::Link,
                "Number" => Kind::Number,
                "DateTime" => Kind::Date,
                "Switch" => Kind::Switch,
                "Option" => {
                    let m: HashMap<String, String> = f["validations"]["options"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|o| {
                            Some((
                                o["id"].as_str()?.to_string(),
                                o["name"].as_str()?.to_string(),
                            ))
                        })
                        .collect();
                    options.insert(slug.to_string(), m);
                    Kind::Option
                }
                "File" => Kind::File,
                "Reference" => Kind::Reference(target),
                "MultiReference" => Kind::MultiReference(target),
                _ => Kind::PlainText,
            };
            fields.push((
                slug.to_string(),
                Field {
                    key: key_of(slug),
                    label: f["displayName"].as_str().unwrap_or(slug).to_string(),
                    kind,
                },
            ));
        }
        let mut items = Vec::new();
        let mut offset = 0;
        loop {
            let page = get(&format!(
                "{API}/collections/{id}/items?limit=100&offset={offset}"
            ))?;
            let batch = page["items"].as_array().cloned().unwrap_or_default();
            for it in &batch {
                if it["isArchived"].as_bool() == Some(true) {
                    continue;
                }
                let data = &it["fieldData"];
                let slug = data["slug"].as_str().unwrap_or("").to_string();
                if slug.is_empty() {
                    continue;
                }
                let mut values = Map::new();
                for (s, f) in &fields {
                    let v = &data[s.as_str()];
                    if v.is_null() {
                        continue;
                    }
                    let v = match &f.kind {
                        Kind::Image | Kind::File => match v["url"].as_str() {
                            Some(u) => Value::String(u.into()),
                            None => continue,
                        },
                        Kind::MultiImage => Value::Array(
                            v.as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(|x| x["url"].as_str().map(|u| Value::String(u.into())))
                                .collect(),
                        ),
                        Kind::Option => match v.as_str().and_then(|o| options.get(s)?.get(o)) {
                            Some(name) => Value::String(name.clone()),
                            None => continue,
                        },
                        Kind::Date => {
                            Value::String(v.as_str().and_then(date_of).unwrap_or_default())
                        }
                        _ => v.clone(),
                    };
                    values.insert(f.key.clone(), v);
                }
                items.push(Item {
                    id: it["id"].as_str().unwrap_or("").to_string(),
                    slug,
                    draft: it["isDraft"].as_bool() == Some(true),
                    values,
                });
            }
            let total = page["pagination"]["total"].as_u64().unwrap_or(0) as usize;
            offset += batch.len();
            if batch.is_empty() || offset >= total {
                break;
            }
        }
        out.push(Collection {
            id: id.to_string(),
            slug: c["slug"].as_str().map(str::to_string).unwrap_or_else(|| {
                revel_model::slugify(c["displayName"].as_str().unwrap_or("collection"))
            }),
            label: c["displayName"]
                .as_str()
                .unwrap_or("Collection")
                .to_string(),
            fields: fields.into_iter().map(|(_, f)| f).collect(),
            items,
        });
    }
    // Références : identifiants Webflow → adresse (slug) du contenu visé, collection par nom.
    let ids: HashMap<String, (String, String)> = out
        .iter()
        .flat_map(|c| {
            c.items
                .iter()
                .map(move |i| (i.id.clone(), (c.slug.clone(), i.slug.clone())))
        })
        .collect();
    let by_id: HashMap<String, String> =
        out.iter().map(|c| (c.id.clone(), c.slug.clone())).collect();
    for c in out.iter_mut() {
        for f in c.fields.iter_mut() {
            match &mut f.kind {
                Kind::Reference(t) | Kind::MultiReference(t) => {
                    if let Some(s) = by_id.get(t.as_str()) {
                        *t = s.clone();
                    }
                }
                _ => {}
            }
        }
        let refs: Vec<String> = c
            .fields
            .iter()
            .filter(|f| matches!(f.kind, Kind::Reference(_) | Kind::MultiReference(_)))
            .map(|f| f.key.clone())
            .collect();
        for it in c.items.iter_mut() {
            for k in &refs {
                let slug = |v: &Value| {
                    v.as_str()
                        .and_then(|id| ids.get(id))
                        .map(|(_, s)| Value::String(s.clone()))
                };
                if let Some(v) = it.values.get(k).cloned() {
                    let nv = match v {
                        Value::Array(a) => Value::Array(a.iter().filter_map(slug).collect()),
                        other => slug(&other).unwrap_or(Value::Null),
                    };
                    it.values.insert(k.clone(), nv);
                }
            }
        }
    }
    Ok(out)
}

/// Identifiant du site Webflow (`<html data-wf-site="…">`) d'un export.
pub fn site_id(site: &Site) -> Option<String> {
    site.layouts.iter().chain(std::iter::empty()).find_map(|l| {
        l.body.iter().find_map(|n| match n {
            Node::Element { tag, attrs, .. } if tag == "html" => attrs
                .iter()
                .find(|a| a.name == "data-wf-site")
                .and_then(|a| a.value.clone()),
            _ => None,
        })
    })
}

// -------------------------------------------------------------------------------------------------
// Écriture dans le site

/// Écrit les collections dans le site importé `dest` : contenus, `.pages.yml`, collections du
/// site, gabarits des pages modèles (`detail_<collection>.html`), images téléchargées.
pub fn apply(
    site: &mut Site,
    dest: &Path,
    mut cols: Vec<Collection>,
    fetch: &dyn Fetch,
    cancel: Option<&AtomicBool>,
    progress: &mut dyn FnMut(usize),
    report: &mut Report,
) -> Result<(), String> {
    // Images : téléchargées une fois, `images/cms/<fichier>`.
    let mut wanted: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    let re = regex::Regex::new(r#"src="(https?://[^"]+)""#).expect("regex");
    for c in &cols {
        for f in c
            .fields
            .iter()
            .filter(|f| matches!(f.kind, Kind::Image | Kind::MultiImage | Kind::File))
        {
            for it in &c.items {
                let mut add = |v: &Value| {
                    if let Some(u) = v.as_str()
                        && (u.starts_with("http") || u.starts_with("//"))
                        && seen.insert(u.to_string())
                    {
                        wanted.push(u.to_string());
                    }
                };
                match it.values.get(&f.key) {
                    Some(Value::Array(a)) => a.iter().for_each(&mut add),
                    Some(v) => add(v),
                    None => {}
                }
            }
        }
        // Images des textes riches.
        for f in c.fields.iter().filter(|f| f.kind == Kind::RichText) {
            for it in &c.items {
                if let Some(t) = it.values.get(&f.key).and_then(Value::as_str) {
                    for m in re.captures_iter(t) {
                        if is_image_url(&m[1]) && seen.insert(m[1].to_string()) {
                            wanted.push(m[1].to_string());
                        }
                    }
                }
            }
        }
    }
    let jobs: Vec<(Url, String)> = wanted
        .iter()
        .filter_map(|u| {
            let full = if u.starts_with("//") {
                format!("https:{u}")
            } else {
                u.clone()
            };
            let x = Url::parse(&full).ok()?;
            let name = x.path_segments()?.next_back()?.to_string();
            let name = percent_encoding::percent_decode_str(&name)
                .decode_utf8_lossy()
                .to_string();
            (!name.is_empty()).then(|| (x, format!("images/cms/{name}")))
        })
        .collect();
    let ok = crate::crawl::download(fetch, &jobs, dest, 6, cancel, progress);
    let mut mapped: HashMap<String, String> = HashMap::new();
    let mut failed = 0;
    for (((_, rel), done), orig) in jobs.iter().zip(&ok).zip(&wanted) {
        if *done {
            mapped.insert(orig.clone(), format!("/{rel}"));
        } else {
            failed += 1;
        }
    }
    report.files += mapped.len();
    if failed > 0 {
        report.warnings.push(format!(
            "CMS : {failed} image(s) non téléchargée(s), adresses de Webflow gardées"
        ));
    }
    let local = |v: &Value| -> Value {
        match v {
            Value::String(s) => Value::String(mapped.get(s).cloned().unwrap_or_else(|| s.clone())),
            Value::Array(a) => Value::Array(
                a.iter()
                    .map(|x| match x {
                        Value::String(s) => {
                            Value::String(mapped.get(s).cloned().unwrap_or_else(|| s.clone()))
                        }
                        o => o.clone(),
                    })
                    .collect(),
            ),
            o => o.clone(),
        }
    };

    let mut yml = revel_project::pagesyml::read(dest)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let created = yml.trim().is_empty();
    let mut used_ids: HashSet<String> = site.collections.iter().map(|c| c.id.clone()).collect();
    for c in cols.iter_mut() {
        let mut id = revel_model::slugify(&c.label).replace('-', "_");
        if id.is_empty() {
            id = "collection".into();
        }
        while !used_ids.insert(id.clone()) {
            id.push('_');
        }
        let folder = format!("content/{}", id.replace('_', "-"));
        let dir = dest.join(&folder);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        // Premier texte riche : texte du fichier (champ `body`), les autres dans l'en-tête.
        let body_key = c
            .fields
            .iter()
            .find(|f| f.kind == Kind::RichText)
            .map(|f| f.key.clone());
        let mut files = HashSet::new();
        for it in &c.items {
            let mut data = Map::new();
            let mut slug = revel_model::slugify(&it.slug);
            if slug.is_empty() {
                slug = "contenu".into();
            }
            while !files.insert(slug.clone()) {
                slug.push_str("-2");
            }
            if let Some(n) = it.values.get("name") {
                data.insert("name".into(), n.clone());
            }
            data.insert("slug".into(), Value::String(slug.clone()));
            // Titre de la page du contenu (référencement), comme la page modèle de Webflow.
            if let Some(n) = it.values.get("name").and_then(Value::as_str) {
                data.insert("seo".into(), json!({ "title": n }));
            }
            let mut body = String::new();
            for f in &c.fields {
                if f.key == "name" {
                    continue;
                }
                let Some(v) = it.values.get(&f.key) else {
                    continue;
                };
                if v.is_null() {
                    continue;
                }
                let v = match f.kind {
                    Kind::Image | Kind::MultiImage | Kind::File => local(v),
                    Kind::RichText => Value::String(rich_local(v.as_str().unwrap_or(""), &mapped)),
                    _ => v.clone(),
                };
                if Some(&f.key) == body_key.as_ref() {
                    body = html_body(v.as_str().unwrap_or(""));
                } else {
                    data.insert(f.key.clone(), v);
                }
            }
            if it.draft {
                data.insert("draft".into(), Value::Bool(true));
            }
            let text = revel_project::entries::entry_text(&Value::Object(data), &body);
            std::fs::write(dir.join(format!("{slug}.md")), text).map_err(|e| e.to_string())?;
            report.entries += 1;
        }
        // Description pour Pages CMS et la vue CMS.
        let mut defs = vec![
            json!({ "name": "name", "label": c.fields.iter().find(|f| f.key == "name").map(|f| f.label.clone()).unwrap_or_else(|| "Name".into()), "type": "string", "required": true }),
            json!({ "name": "slug", "label": "Slug", "type": "string", "required": true }),
        ];
        for f in c.fields.iter().filter(|f| f.key != "name") {
            let name = if Some(&f.key) == body_key.as_ref() {
                "body".to_string()
            } else {
                f.key.clone()
            };
            let mut d = json!({ "name": name, "label": f.label });
            match &f.kind {
                Kind::PlainText | Kind::Link => {
                    let long = c.items.iter().any(|i| {
                        i.values
                            .get(&f.key)
                            .and_then(Value::as_str)
                            .is_some_and(|s| s.len() > 90 || s.contains('\n'))
                    });
                    d["type"] = json!(if long { "text" } else { "string" });
                }
                Kind::RichText => {
                    d["type"] = json!("rich-text");
                    d["options"] = json!({ "format": "html" });
                }
                Kind::Image => d["type"] = json!("image"),
                Kind::MultiImage => {
                    d["type"] = json!("image");
                    d["list"] = json!(true);
                }
                Kind::File => d["type"] = json!("file"),
                Kind::Number => d["type"] = json!("number"),
                Kind::Date => d["type"] = json!("date"),
                Kind::Switch => d["type"] = json!("boolean"),
                Kind::Option => {
                    let mut vals: Vec<String> = c
                        .items
                        .iter()
                        .filter_map(|i| {
                            i.values
                                .get(&f.key)
                                .and_then(Value::as_str)
                                .map(str::to_string)
                        })
                        .collect();
                    vals.sort();
                    vals.dedup();
                    d["type"] = json!("select");
                    d["options"] = json!({ "values": vals });
                }
                Kind::Reference(t) | Kind::MultiReference(t) => {
                    d["type"] = json!("reference");
                    let target = revel_model::slugify(t).replace('-', "_");
                    let mut o = json!({ "collection": target, "value": "{fields.slug}", "label": "{fields.name}", "search": "name" });
                    if matches!(f.kind, Kind::MultiReference(_)) {
                        o["multiple"] = json!(true);
                    }
                    d["options"] = o;
                }
            }
            defs.push(d);
        }
        defs.push(json!({ "name": "draft", "label": "Draft", "type": "boolean" }));
        let def = json!({
            "name": id,
            "label": c.label,
            "type": "collection",
            "path": folder,
            "filename": "{fields.slug}.md",
            "view": { "primary": "name", "fields": ["name", "slug"] },
            "fields": defs,
        });
        yml = revel_project::pagesyml::add_collection(&yml, &def)?;
        report.collections += 1;

        // Gabarit : page modèle exportée (`detail_<collection>.html`).
        let detail = site.pages.iter().position(|p| {
            let name = p.path.trim_start_matches('/').trim_end_matches(".html");
            name.strip_prefix("detail_")
                .is_some_and(|s| same_name(s, &c.slug) || same_name(s, &c.label))
        });
        let mut col = revel_model::Collection {
            id: id.clone(),
            source: Default::default(),
            folder: folder.clone(),
            path: None,
            layout: None,
            props: Default::default(),
            filter: None,
            sort: None,
            fields: Vec::new(),
        };
        if let Some(i) = detail {
            let page = site.pages.remove(i);
            let url_slug = page
                .path
                .trim_start_matches("/detail_")
                .trim_end_matches(".html")
                .to_string();
            let mut body = page.body;
            let bound = bind_template(&mut body, &c.fields, body_key.as_deref());
            let layout_id = format!("cms-{}", id.replace('_', "-"));
            site.layouts.push(Layout {
                fields: Vec::new(),
                id: layout_id.clone(),
                parent: page.layout.clone(),
                props: Default::default(),
                body,
            });
            col.path = Some(format!("/{url_slug}/{{{{ slug }}}}/"));
            col.layout = Some(layout_id);
            report.warnings.push(format!(
                "CMS « {} » : {bound} élément(s) de la page modèle relié(s) aux champs, à vérifier",
                c.label
            ));
            report.layouts += 1;
            report.pages = report.pages.saturating_sub(1);
        } else {
            report.warnings.push(format!(
                "CMS « {} » : pas de page modèle dans l'export, contenus sans page",
                c.label
            ));
        }
        site.collections.push(col);
    }
    if created && !yml.trim().is_empty() {
        yml = revel_project::pagesyml::set_top(
            &yml,
            "settings",
            &json!({ "content": { "merge": true } }),
        )?;
    }
    revel_project::pagesyml::write(dest, Some(&yml))?;
    // Listes du CMS des pages : exportées vides par Webflow.
    let mut lists = 0;
    for p in &site.pages {
        revel_model::ids::walk(&p.body, &mut |n| {
            if let Node::Element { attrs, .. } = n
                && attrs.iter().any(|a| {
                    a.name == "class"
                        && a.value
                            .as_deref()
                            .is_some_and(|v| v.split_whitespace().any(|c| c == "w-dyn-list"))
                })
            {
                lists += 1;
            }
        });
    }
    if lists > 0 {
        report.warnings.push(format!(
            "{lists} liste(s) du CMS exportée(s) vide(s) par Webflow : à relier à une collection (panneau Données)"
        ));
    }
    Ok(())
}

/// `blog-posts` et `Blog Posts`, `post` et `posts` : même collection.
fn same_name(a: &str, b: &str) -> bool {
    let n = |s: &str| {
        let s = revel_model::slugify(s);
        s.trim_end_matches('s').to_string()
    };
    n(a) == n(b)
}

/// Adresses des images d'un texte riche remplacées par leurs copies locales.
fn rich_local(html: &str, mapped: &HashMap<String, String>) -> String {
    let mut s = html.to_string();
    for (from, to) in mapped {
        if s.contains(from.as_str()) {
            s = s.replace(from.as_str(), to);
        }
    }
    s
}

/// Texte riche écrit comme texte du fichier : lu comme du HTML (pas de ligne vide ni de retrait).
fn html_body(html: &str) -> String {
    html.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn class_has(attrs: &[Attr], c: &str) -> bool {
    attrs.iter().any(|a| {
        a.name == "class"
            && a.value
                .as_deref()
                .is_some_and(|v| v.split_whitespace().any(|x| x == c))
    })
}

/// Éléments de la page modèle liés aux champs (classe `w-dyn-bind-empty` laissée par l'export de
/// Webflow ; sinon titre, texte riche et image vides) reliés aux champs dans l'ordre. Renvoie le
/// nombre d'éléments reliés.
fn bind_template(nodes: &mut [Node], fields: &[Field], body_key: Option<&str>) -> usize {
    let mut marked = false;
    revel_model::ids::walk(nodes, &mut |n| {
        if let Node::Element { attrs, .. } = n
            && class_has(attrs, "w-dyn-bind-empty")
        {
            marked = true;
        }
    });
    let mut texts: Vec<&Field> = fields
        .iter()
        .filter(|f| {
            matches!(
                f.kind,
                Kind::PlainText | Kind::Date | Kind::Number | Kind::Option | Kind::Link
            )
        })
        .collect();
    // Le nom d'abord.
    texts.sort_by_key(|f| f.key != "name");
    let mut images = fields.iter().filter(|f| f.kind == Kind::Image);
    let mut riches = fields.iter().filter(|f| f.kind == Kind::RichText);
    let mut texts = texts.into_iter();
    let mut links = fields.iter().filter(|f| f.kind == Kind::Link);
    let mut count = 0;
    /// Élément candidat : balise, attributs, enfants ; vrai s'il a été relié.
    type Bind<'a> = dyn FnMut(&str, &mut Vec<Attr>, &mut Vec<Node>) -> bool + 'a;
    fn visit(nodes: &mut [Node], marked: bool, f: &mut Bind) {
        for n in nodes.iter_mut() {
            if let Node::Element {
                tag,
                attrs,
                children,
                ..
            } = n
            {
                let candidate = if marked {
                    class_has(attrs, "w-dyn-bind-empty")
                } else {
                    let empty = children
                        .iter()
                        .all(|c| matches!(c, Node::Text { text } if text.trim().is_empty()));
                    (empty
                        && (matches!(tag.as_str(), "h1" | "h2") || class_has(attrs, "w-richtext")))
                        || (tag == "img"
                            && attrs.iter().any(|a| {
                                a.name == "src"
                                    && a.value
                                        .as_deref()
                                        .is_some_and(|v| v.contains("placeholder"))
                            }))
                };
                if candidate && f(tag, attrs, children) {
                    for a in attrs.iter_mut().filter(|a| a.name == "class") {
                        if let Some(v) = &a.value {
                            a.value = Some(
                                v.split_whitespace()
                                    .filter(|c| *c != "w-dyn-bind-empty")
                                    .collect::<Vec<_>>()
                                    .join(" "),
                            );
                        }
                    }
                    continue;
                }
                visit(children, marked, f);
            }
        }
    }
    let out = |e: String| Node::Out { expr: e };
    visit(nodes, marked, &mut |tag, attrs, children| {
        if tag == "img" {
            let Some(f) = images.next() else { return false };
            attrs.retain(|a| a.name != "srcset" && a.name != "sizes");
            match attrs.iter_mut().find(|a| a.name == "src") {
                Some(a) => a.value = Some(format!("{{{{ {} }}}}", f.key)),
                None => attrs.push(Attr {
                    name: "src".into(),
                    value: Some(format!("{{{{ {} }}}}", f.key)),
                    when: None,
                }),
            }
            count += 1;
            return true;
        }
        if class_has(attrs, "w-richtext") {
            let Some(f) = riches.next() else { return false };
            *children = vec![out(if Some(f.key.as_str()) == body_key {
                "content | safe".into()
            } else {
                format!("{} | safe", f.key)
            })];
            count += 1;
            return true;
        }
        if tag == "a" && children.is_empty() {
            let Some(f) = links.next() else { return false };
            if let Some(a) = attrs.iter_mut().find(|a| a.name == "href") {
                a.value = Some(format!("{{{{ {} }}}}", f.key));
            }
            count += 1;
            return true;
        }
        let Some(f) = texts.next() else { return false };
        let e = match f.kind {
            Kind::Date => format!("{} | date(\"%d/%m/%Y\")", f.key),
            _ => f.key.clone(),
        };
        *children = vec![out(e)];
        count += 1;
        true
    });
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSV: &str = "Name,Slug,Collection ID,Locale ID,Item ID,Archived,Draft,Created On,Updated On,Published On,Post Body,Main Image,Featured,Category\n\
\"Premier article\",premier,c1,l1,i1,false,false,\"Mon Jan 15 2024 10:00:00 GMT+0000 (Coordinated Universal Time)\",x,x,\"<p>Texte, avec virgule.</p>\n<h2>Titre</h2>\",https://cdn.prod.website-files.com/abc/65_bol.jpg,true,Céramique\n\
Second,second,c1,l1,i2,false,true,x,x,x,<p>B</p>,,false,Objets\n\
Archivé,vieux,c1,l1,i3,true,false,x,x,x,<p>C</p>,,false,Objets\n";

    const CATS: &str = "Name,Slug,Collection ID,Locale ID,Item ID,Archived,Draft,Created On,Updated On,Published On\nCéramique,ceramique,c2,l1,k1,false,false,x,x,x\nObjets,objets,c2,l1,k2,false,false,x,x,x\n";

    #[test]
    fn lecture_csv() {
        assert!(is_webflow_csv(CSV));
        let c = from_csv("Atelier - Blog Posts - 65f0.csv", CSV).unwrap();
        assert_eq!(c.label, "Blog Posts");
        assert_eq!(c.slug, "blog-posts");
        let kinds: Vec<(&str, &Kind)> =
            c.fields.iter().map(|f| (f.key.as_str(), &f.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                ("name", &Kind::PlainText),
                ("post_body", &Kind::RichText),
                ("main_image", &Kind::Image),
                ("featured", &Kind::Switch),
                ("category", &Kind::PlainText),
            ]
        );
        assert_eq!(c.items.len(), 2, "contenu archivé écarté");
        assert!(c.items[1].draft);
        assert_eq!(
            c.items[0].values["post_body"],
            "<p>Texte, avec virgule.</p>\n<h2>Titre</h2>"
        );
        assert_eq!(
            date_of("Mon Jan 15 2024 00:00:00 GMT+0000 (UTC)").as_deref(),
            Some("2024-01-15")
        );
        assert_eq!(
            date_of("2024-03-02T08:30:00.000Z").as_deref(),
            Some("2024-03-02T08:30:00")
        );
        let mut cols = vec![c, from_csv("Atelier - Categories - 1.csv", CATS).unwrap()];
        link_csv_references(&mut cols);
        assert_eq!(cols[0].fields[4].kind, Kind::Reference("categories".into()));
        assert_eq!(cols[0].items[0].values["category"], "ceramique");
    }

    #[test]
    fn page_modele_reliee() {
        let fields = vec![
            Field {
                key: "name".into(),
                label: "Name".into(),
                kind: Kind::PlainText,
            },
            Field {
                key: "post_body".into(),
                label: "Body".into(),
                kind: Kind::RichText,
            },
            Field {
                key: "main_image".into(),
                label: "Image".into(),
                kind: Kind::Image,
            },
            Field {
                key: "date".into(),
                label: "Date".into(),
                kind: Kind::Date,
            },
        ];
        let mut nodes = crate::html::fragment(
            r#"<div class="wrap"><h1 class="heading w-dyn-bind-empty"></h1><div class="meta w-dyn-bind-empty"></div><img src="https://d3e54v103j8qbb.cloudfront.net/plugins/Basic/assets/placeholder.60f9b1840c.svg" loading="lazy" alt="" class="w-dyn-bind-empty"><div class="w-richtext w-dyn-bind-empty"></div></div>"#,
        );
        assert_eq!(bind_template(&mut nodes, &fields, Some("post_body")), 4);
        let html = crate::html::to_html(&nodes);
        assert!(
            html.contains(r#"<h1 class="heading">{{ name }}</h1>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<div class="meta">{{ date | date("%d/%m/%Y") }}</div>"#),
            "{html}"
        );
        assert!(html.contains(r#"src="{{ main_image }}""#), "{html}");
        assert!(
            html.contains(r#"<div class="w-richtext">{{ content | safe }}</div>"#),
            "{html}"
        );
    }
}
