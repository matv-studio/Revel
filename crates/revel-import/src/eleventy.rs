//! Import d'un projet Eleventy.
//!
//! Repris automatiquement :
//! - pages (`*.njk` du dossier d'entrée) avec leur front matter (`permalink`, `layout`, variables) ;
//! - gabarits (`_includes/layouts/**`) et composants (autres fichiers de `_includes`) ;
//! - collections déclarées par `addCollection("nom", api => api.getFilteredByGlob("…"))`,
//!   leurs filtres `published` et tris courants, leurs `*.11tydata.js` (`layout`, `permalink`) ;
//! - copies `addPassthroughCopy({ "a": "b" })` et dossier `_data`.
//!
//! Le reste (filtres JavaScript, collections calculées, variables d'environnement) est décrit
//! dans le profil `revel-import.json` ; chaque élément non traduit produit un avertissement.

use crate::Report;
use regex::Regex;
use revel_markup::{Dialect, Mode, Options};
use revel_model::{
    Collection, CollectionSource, Component, CustomFilter, Layout, Node, Page, Passthrough, Site,
    Sort, SortKind,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const PROFILE_FILE: &str = "revel-import.json";

/// Description de la logique JavaScript d'un site Eleventy.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Nom du site.
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub default_locale: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    /// Dossier d'entrée Eleventy (`dir.input`), si non détectable.
    #[serde(default)]
    pub input: Option<String>,
    /// Typographie (`fr`).
    #[serde(default)]
    pub typography: Option<String>,
    /// Filtres renommés : nom Eleventy -> nom Revel (intégré ou personnalisé).
    #[serde(default)]
    pub rename_filters: BTreeMap<String, String>,
    /// Filtres personnalisés (expressions).
    #[serde(default)]
    pub filters: BTreeMap<String, CustomFilter>,
    /// Collections ajoutées ou complétées.
    #[serde(default)]
    pub collections: BTreeMap<String, CollectionPatch>,
    /// Variables `env` par environnement.
    #[serde(default)]
    pub environments: BTreeMap<String, Value>,
    /// Copies supplémentaires (ex. fichiers venant de `node_modules`, recopiés dans le projet).
    #[serde(default)]
    pub passthrough: Vec<Passthrough>,
    /// Copies détectées à ignorer (sources absentes du dépôt).
    #[serde(default)]
    pub ignore_passthrough: Vec<String>,
    /// Composants supplémentaires écrits en HTML/Jinja (ex. `image-placeholder`).
    #[serde(default)]
    pub components: BTreeMap<String, String>,
    /// Feuille de style importée dans le modèle (D-18), relative au projet ; par défaut, la
    /// première feuille locale liée par les gabarits (`<link rel="stylesheet">`).
    #[serde(default)]
    pub stylesheet: Option<String>,
    /// `false` : les feuilles de style restent des fichiers copiés tels quels.
    #[serde(default)]
    pub import_styles: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionPatch {
    #[serde(default)]
    pub source: Option<CollectionSource>,
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub layout: Option<String>,
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(default)]
    pub sort: Option<Sort>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("{path} : {msg}")]
    File { path: String, msg: String },
}

fn ferr(p: &Path, msg: impl ToString) -> ImportError {
    ImportError::File {
        path: p.display().to_string(),
        msg: msg.to_string(),
    }
}

/// Lit le profil `revel-import.json` s'il existe.
pub fn load_profile(root: &Path) -> Result<Profile, ImportError> {
    let p = root.join(PROFILE_FILE);
    if !p.is_file() {
        return Ok(Profile::default());
    }
    let text = std::fs::read_to_string(&p).map_err(|e| ferr(&p, e))?;
    serde_json::from_str(&text).map_err(|e| ferr(&p, e))
}

/// Importe le projet Eleventy situé dans `root`.
pub fn import(root: &Path, profile: &Profile) -> Result<(Site, Report), ImportError> {
    let mut report = Report::default();
    let config_path = [
        "eleventy.config.js",
        "eleventy.config.mjs",
        "eleventy.config.cjs",
        ".eleventy.js",
    ]
    .iter()
    .map(|f| root.join(f))
    .find(|p| p.is_file());
    let config = match &config_path {
        Some(p) => std::fs::read_to_string(p).map_err(|e| ferr(p, e))?,
        None => String::new(),
    };
    let input = profile
        .input
        .clone()
        .or_else(|| detect_input(&config, root))
        .unwrap_or_else(|| ".".into());
    let input_dir = root.join(&input);
    let rel = |p: &Path| -> String {
        p.strip_prefix(root)
            .unwrap_or(p)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/")
    };

    let mut site = Site::new(
        profile.name.clone().unwrap_or_else(|| {
            root.file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "site".into())
        }),
        profile
            .default_locale
            .clone()
            .unwrap_or_else(|| "fr".into()),
    );
    site.base_url = profile.base_url.clone();
    site.settings.typography = profile.typography.clone();
    site.settings.filters = profile.filters.clone();
    site.settings.environments = profile.environments.clone();
    let data_dir = input_dir.join("_data");
    if data_dir.is_dir() {
        site.data_dir = Some(rel(&data_dir));
    }

    let renames: Vec<(String, String)> = profile
        .rename_filters
        .iter()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect();
    let conv = Converter { renames };

    // Passthrough.
    for (from, to) in detect_passthrough(&config) {
        if profile.ignore_passthrough.contains(&from) {
            continue;
        }
        if !root.join(&from).exists() {
            report.warnings.push(format!(
                "copie « {from} » -> « {to} » : source absente du projet (à recopier dans le dépôt ou à ignorer dans le profil)"
            ));
            continue;
        }
        site.passthrough.push(Passthrough {
            from,
            to: to.trim_matches('/').to_string(),
        });
    }
    site.passthrough.extend(profile.passthrough.iter().cloned());

    // Gabarits et composants.
    let includes = input_dir.join("_includes");
    for f in walk(&includes, &["njk", "html", "liquid"]) {
        let id = revel_markup::component_id(&rel_to(&f, &includes));
        let text = std::fs::read_to_string(&f).map_err(|e| ferr(&f, e))?;
        let (fm, body) = split_front_matter(&text, &f)?;
        let mut nodes = conv.parse(body, Mode::Html, &f, &mut report)?;
        if id.starts_with("layouts/") {
            content_to_slot(&mut nodes);
            let mut props = fm.clone();
            let parent = props
                .remove("layout")
                .and_then(|v| v.as_str().map(revel_markup::component_id));
            site.layouts.push(Layout {
                id,
                parent,
                props,
                body: nodes,
            });
            report.layouts += 1;
        } else {
            site.components.push(Component::new(id, nodes));
            report.components += 1;
        }
    }
    for (id, src) in &profile.components {
        let nodes = conv.parse(src, Mode::Html, Path::new(id), &mut report)?;
        site.components.push(Component::new(id.clone(), nodes));
        report.components += 1;
    }

    // Collections déclarées dans la configuration.
    for c in detect_collections(&config) {
        let folder = c
            .glob
            .rsplit_once('/')
            .map(|(d, _)| d.to_string())
            .unwrap_or_default();
        let mut col = Collection {
            id: c.name.clone(),
            source: CollectionSource::Folder,
            folder: folder.clone(),
            path: None,
            layout: None,
            props: BTreeMap::new(),
            filter: c.published.then(|| "published != false".to_string()),
            sort: c.sort,
            fields: Vec::new(),
        };
        // Données de dossier *.11tydata.js.
        if let Some(dd) = walk(&root.join(&folder), &["js", "json"])
            .into_iter()
            .find(|p| p.to_string_lossy().contains(".11tydata."))
        {
            let js = std::fs::read_to_string(&dd).map_err(|e| ferr(&dd, e))?;
            let (layout, path) = parse_dir_data(&js);
            col.layout = layout.map(|l| revel_markup::component_id(&l));
            col.path = path;
        }
        site.collections.push(col);
        report.collections += 1;
    }
    for name in detect_unhandled_collections(&config, &site) {
        if !profile.collections.contains_key(&name) {
            report.warnings.push(format!(
                "collection « {name} » calculée en JavaScript : à décrire dans le profil"
            ));
        }
    }
    for (name, patch) in &profile.collections {
        let col = match site.collections.iter_mut().find(|c| &c.id == name) {
            Some(c) => c,
            None => {
                site.collections.push(Collection {
                    id: name.clone(),
                    source: CollectionSource::Folder,
                    folder: String::new(),
                    path: None,
                    layout: None,
                    props: BTreeMap::new(),
                    filter: None,
                    sort: None,
                    fields: Vec::new(),
                });
                report.collections += 1;
                site.collections.last_mut().expect("ajoutée")
            }
        };
        if let Some(v) = patch.source {
            col.source = v;
        }
        if let Some(v) = &patch.folder {
            col.folder = v.clone();
        }
        if let Some(v) = &patch.path {
            col.path = Some(v.clone());
        }
        if let Some(v) = &patch.layout {
            col.layout = Some(v.clone());
        }
        if let Some(v) = &patch.filter {
            col.filter = Some(v.clone());
        }
        if let Some(v) = &patch.sort {
            col.sort = Some(v.clone());
        }
    }

    // Pages.
    for f in walk(&input_dir, &["njk", "html"]) {
        let r = rel_to(&f, &input_dir);
        if r.starts_with("_includes/") || r.starts_with("_data/") || r.starts_with("node_modules/")
        {
            continue;
        }
        let text = std::fs::read_to_string(&f).map_err(|e| ferr(&f, e))?;
        let (mut fm, body) = split_front_matter(&text, &f)?;
        let permalink = fm.remove("permalink");
        let path = match permalink {
            Some(Value::String(p)) => p,
            Some(Value::Bool(false)) => continue,
            _ => default_permalink(&r),
        };
        let layout = fm
            .remove("layout")
            .and_then(|v| v.as_str().map(revel_markup::component_id));
        let mode = match revel_render::DocKind::of_path(&path) {
            revel_render::DocKind::Html => Mode::Html,
            revel_render::DocKind::Xml => Mode::Xml,
            revel_render::DocKind::Text => Mode::Text,
        };
        let body = conv.parse(body, mode, &f, &mut report)?;
        // Données de modèle `<nom>.11tydata.js` : variables calculées.
        let stem = f
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut computed = BTreeMap::new();
        for ext in ["js", "mjs", "cjs", "json"] {
            let dd = f.with_file_name(format!("{stem}.11tydata.{ext}"));
            if dd.is_file() {
                let js = std::fs::read_to_string(&dd).map_err(|e| ferr(&dd, e))?;
                computed.extend(parse_computed(&js));
            }
        }
        site.pages.push(Page {
            id: r
                .trim_end_matches(".njk")
                .trim_end_matches(".html")
                .to_string(),
            path,
            layout,
            props: fm,
            computed,
            body,
            draft: false,
            name: None,
        });
        report.pages += 1;
    }
    site.pages.sort_by(|a, b| a.path.cmp(&b.path));

    // Filtres utilisés mais inconnus.
    let known = known_filters(&site);
    let used = used_filters(&site);
    for f in used {
        if !known.contains(&f.as_str()) && !site.settings.filters.contains_key(&f) {
            report.warnings.push(format!(
                "filtre « {f} » sans équivalent : à définir dans le profil"
            ));
        }
    }
    // Feuille de style (D-18) : importée dans le modèle, générée par Revel.
    if profile.import_styles != Some(false) {
        import_stylesheet(root, profile, &mut site, &mut report);
    }
    // Identifiants stables des nœuds et des règles (déterministes : un même import donne les mêmes).
    site.ensure_ids();
    Ok((site, report))
}

/// Adresses des feuilles de style liées par les gabarits, composants et pages
/// (`<link rel="stylesheet" href="/…">`), sans paramètre de version.
fn linked_stylesheets(site: &Site) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, _, body) in site.owners() {
        revel_model::ids::walk(body, &mut |n| {
            if let Node::Element { tag, attrs, .. } = n
                && tag.eq_ignore_ascii_case("link")
            {
                let get = |name: &str| {
                    attrs
                        .iter()
                        .find(|a| a.name.eq_ignore_ascii_case(name))
                        .and_then(|a| a.value.clone())
                };
                if get("rel").is_some_and(|r| {
                    r.split_whitespace()
                        .any(|t| t.eq_ignore_ascii_case("stylesheet"))
                }) && let Some(href) = get("href")
                {
                    let url = href.split(['?', '#', '{']).next().unwrap_or("").to_string();
                    if url.starts_with('/') && !url.starts_with("//") && !out.contains(&url) {
                        out.push(url);
                    }
                }
            }
        });
    }
    out
}

/// Fichier source d'une adresse publiée, d'après les copies (`passthrough`).
fn source_for_url(site: &Site, root: &Path, url: &str) -> Option<String> {
    let path = url.trim_start_matches('/');
    for p in &site.passthrough {
        let to = p.to.trim_matches('/');
        let rest = if to.is_empty() {
            Some(path)
        } else {
            path.strip_prefix(to).and_then(|r| r.strip_prefix('/'))
        };
        if let Some(rest) = rest {
            let candidate = format!("{}/{}", p.from.trim_end_matches('/'), rest);
            if root.join(&candidate).is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn import_stylesheet(root: &Path, profile: &Profile, site: &mut Site, report: &mut Report) {
    let links = linked_stylesheets(site);
    let (url, source) = match &profile.stylesheet {
        Some(src) => {
            // Adresse publiée : celle du lien qui correspond à ce fichier, sinon dérivée des copies.
            let url = links
                .iter()
                .find(|u| source_for_url(site, root, u).as_deref() == Some(src.as_str()))
                .cloned()
                .unwrap_or_else(|| {
                    site.passthrough
                        .iter()
                        .find_map(|p| {
                            src.strip_prefix(p.from.trim_end_matches('/')).map(|r| {
                                format!("/{}{}", p.to.trim_matches('/'), r).replace("//", "/")
                            })
                        })
                        .unwrap_or_else(|| "/styles.css".into())
                });
            (url, src.clone())
        }
        None => {
            let Some((url, src)) = links
                .iter()
                .find_map(|u| source_for_url(site, root, u).map(|s| (u.clone(), s)))
            else {
                return;
            };
            (url, src)
        }
    };
    let path = root.join(&source);
    let css = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            report
                .warnings
                .push(format!("feuille de style « {source} » illisible : {e}"));
            return;
        }
    };
    let items = match revel_style::parse(&css) {
        Ok(i) => i,
        Err(e) => {
            report.warnings.push(format!(
                "feuille de style « {source} » non importée ({e}) : elle reste copiée telle quelle"
            ));
            return;
        }
    };
    for other in links.iter().filter(|u| **u != url) {
        report.warnings.push(format!(
            "feuille de style « {other} » : copiée telle quelle (une seule feuille est importée dans le modèle)"
        ));
    }
    site.breakpoints = revel_style::breakpoints_from(&items);
    report.style_rules = items
        .iter()
        .filter(|i| matches!(i, revel_model::StyleItem::Rule(_)))
        .count();
    site.stylesheet = Some(revel_model::Stylesheet {
        url,
        source: Some(source),
        items,
    });
}

struct Converter {
    renames: Vec<(String, String)>,
}

impl Converter {
    fn parse(
        &self,
        src: &str,
        mode: Mode,
        f: &Path,
        report: &mut Report,
    ) -> Result<Vec<Node>, ImportError> {
        let parsed = revel_markup::parse(
            src,
            &Options {
                dialect: Dialect::Nunjucks,
                mode,
            },
        )
        .map_err(|e| ferr(f, e))?;
        for w in parsed.warnings {
            report.warnings.push(format!("{} : {w}", f.display()));
        }
        let mut nodes = parsed.nodes;
        let pairs: Vec<(&str, &str)> = self
            .renames
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        map_exprs(&mut nodes, &|e: &str| {
            revel_markup::expr::rename_filters(e, &pairs)
        });
        Ok(nodes)
    }
}

/// Applique `f` à toutes les expressions d'un arbre.
pub fn map_exprs(nodes: &mut [Node], f: &dyn Fn(&str) -> String) {
    for n in nodes {
        match n {
            Node::Element {
                attrs, children, ..
            } => {
                for a in attrs.iter_mut() {
                    if let Some(v) = &a.value
                        && (v.contains("{{") || v.contains("{%"))
                    {
                        a.value = Some(revel_markup::expr::map_template(v, f));
                    }
                    if let Some(w) = &a.when {
                        a.when = Some(f(w));
                    }
                }
                map_exprs(children, f);
            }
            Node::Out { expr } => *expr = f(expr),
            Node::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                *cond = f(cond);
                map_exprs(then, f);
                map_exprs(otherwise, f);
            }
            Node::Each {
                expr,
                children,
                empty,
                ..
            } => {
                *expr = f(expr);
                map_exprs(children, f);
                map_exprs(empty, f);
            }
            Node::Set { expr, .. } => *expr = f(expr),
            Node::SetBlock { children, .. } => map_exprs(children, f),
            Node::Image(img) => {
                img.src = f(&img.src);
                for o in [
                    &mut img.alt,
                    &mut img.sizes,
                    &mut img.class,
                    &mut img.ratio,
                    &mut img.loading,
                    &mut img.widths,
                ]
                .into_iter()
                .flatten()
                {
                    *o = f(o);
                }
            }
            Node::Text { .. } | Node::Html { .. } | Node::Slot | Node::Use { .. } => {}
        }
    }
}

/// `{{ content | safe }}` -> emplacement de la page.
fn content_to_slot(nodes: &mut [Node]) {
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

fn walk(dir: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name()
                    .is_some_and(|n| n == "node_modules" || n.to_string_lossy().starts_with('.'))
                {
                    continue;
                }
                stack.push(p);
            } else if p
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| exts.contains(&x))
            {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn rel_to(p: &Path, base: &Path) -> String {
    p.strip_prefix(base)
        .unwrap_or(p)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn split_front_matter<'a>(
    text: &'a str,
    f: &Path,
) -> Result<(BTreeMap<String, Value>, &'a str), ImportError> {
    let Some(rest) = text.strip_prefix("---") else {
        return Ok((BTreeMap::new(), text));
    };
    let rest = rest
        .strip_prefix('\n')
        .or_else(|| rest.strip_prefix("\r\n"))
        .unwrap_or(rest);
    let mut pos = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            let yaml = &rest[..pos];
            let body = &rest[pos + line.len()..];
            let v: Value = if yaml.trim().is_empty() {
                Value::Object(Default::default())
            } else {
                serde_saphyr::from_str(yaml).map_err(|e| ferr(f, e))?
            };
            let map = match v {
                Value::Object(m) => m.into_iter().collect(),
                _ => BTreeMap::new(),
            };
            return Ok((map, body));
        }
        pos += line.len();
    }
    Err(ferr(f, "front matter non fermé"))
}

/// `agence.njk` -> `/agence/` ; `index.njk` -> `/` ; `a/index.njk` -> `/a/`.
fn default_permalink(rel: &str) -> String {
    let stem = rel.rsplit_once('.').map(|(a, _)| a).unwrap_or(rel);
    let stem = stem
        .strip_suffix("index")
        .unwrap_or(stem)
        .trim_end_matches('/');
    if stem.is_empty() {
        "/".into()
    } else {
        format!("/{stem}/")
    }
}

fn detect_input(config: &str, root: &Path) -> Option<String> {
    let re = Regex::new(r#"input:\s*["']([^"']+)["']"#).expect("regex");
    if let Some(c) = re.captures(config) {
        return Some(c[1].to_string());
    }
    // `input: SRC` avec `const SRC = "src"`.
    let re_var = Regex::new(r#"input:\s*([A-Za-z_]\w*)"#).expect("regex");
    if let Some(c) = re_var.captures(config) {
        let re_const = Regex::new(&format!(
            r#"(?:const|let|var)\s+{}\s*=\s*["']([^"']+)["']"#,
            &c[1]
        ))
        .expect("regex");
        if let Some(v) = re_const.captures(config) {
            return Some(v[1].to_string());
        }
    }
    root.join("src/_includes").is_dir().then(|| "src".into())
}

fn detect_passthrough(config: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let obj = Regex::new(r"addPassthroughCopy\(\s*\{([^}]*)\}\s*\)").expect("regex");
    let pair = Regex::new(r#"["']([^"']+)["']\s*:\s*["']([^"']*)["']"#).expect("regex");
    for c in obj.captures_iter(config) {
        for p in pair.captures_iter(&c[1]) {
            out.push((p[1].to_string(), p[2].to_string()));
        }
    }
    let single = Regex::new(r#"addPassthroughCopy\(\s*["']([^"']+)["']\s*\)"#).expect("regex");
    for c in single.captures_iter(config) {
        out.push((c[1].to_string(), c[1].to_string()));
    }
    out
}

struct DetectedCollection {
    name: String,
    glob: String,
    published: bool,
    sort: Option<Sort>,
}

fn detect_collections(config: &str) -> Vec<DetectedCollection> {
    let re = Regex::new(
        r#"addCollection\(\s*["'](\w+)["']\s*,\s*\(?\s*\w+\s*\)?\s*=>\s*\w+\.getFilteredByGlob\(\s*["']([^"']+)["']\s*\)([^;]*)"#,
    )
    .expect("regex");
    re.captures_iter(config)
        .map(|c| {
            let rest = &c[3];
            let sort = if rest.contains("byDateDesc") {
                Some(Sort {
                    field: "date".into(),
                    descending: true,
                    kind: SortKind::Date,
                })
            } else if rest.contains(".order") {
                Some(Sort {
                    field: "order".into(),
                    descending: false,
                    kind: SortKind::Number,
                })
            } else {
                None
            };
            DetectedCollection {
                name: c[1].to_string(),
                glob: c[2].to_string(),
                published: rest.contains("filter(published)"),
                sort,
            }
        })
        .collect()
}

fn detect_unhandled_collections(config: &str, site: &Site) -> Vec<String> {
    let re = Regex::new(r#"addCollection\(\s*["'](\w+)["']"#).expect("regex");
    re.captures_iter(config)
        .map(|c| c[1].to_string())
        .filter(|n| site.collection(n).is_none())
        .collect()
}

/// `*.11tydata.js` : `layout: "…"` et `permalink` sous forme de gabarit littéral.
fn parse_dir_data(js: &str) -> (Option<String>, Option<String>) {
    let layout = Regex::new(r#"layout:\s*["']([^"']+)["']"#)
        .expect("regex")
        .captures(js)
        .map(|c| c[1].to_string());
    if Regex::new(r"permalink:\s*false")
        .expect("regex")
        .is_match(js)
    {
        return (layout, None);
    }
    let path = Regex::new(r"`([^`]*)`")
        .expect("regex")
        .captures(js)
        .map(|c| {
            Regex::new(r"\$\{\s*\w+\.(\w+)\s*\}")
                .expect("regex")
                .replace_all(&c[1], "{{ $1 }}")
                .into_owned()
        });
    (layout, path)
}

/// `eleventyComputed: { seo: (d) => d.pages["agence"].seo }` -> `seo` = `pages["agence"].seo`.
fn parse_computed(js: &str) -> BTreeMap<String, String> {
    let re = Regex::new(r#"(\w+)\s*:\s*\(\s*(\w+)\s*\)\s*=>\s*([^,}\n]+)"#).expect("regex");
    let mut out = BTreeMap::new();
    for c in re.captures_iter(js) {
        let (key, param, body) = (&c[1], &c[2], c[3].trim());
        if key == "permalink" {
            continue;
        }
        let strip = Regex::new(&format!(r"\b{param}\.")).expect("regex");
        let expr = strip.replace_all(body, "");
        out.insert(key.to_string(), revel_markup::expr::from_nunjucks(&expr));
    }
    out
}

/// Filtres disponibles sans définition : minijinja + Revel.
fn known_filters(_site: &Site) -> Vec<&'static str> {
    vec![
        // minijinja
        "safe",
        "escape",
        "e",
        "lower",
        "upper",
        "title",
        "capitalize",
        "replace",
        "length",
        "count",
        "dictsort",
        "items",
        "reverse",
        "trim",
        "join",
        "split",
        "lines",
        "default",
        "d",
        "round",
        "abs",
        "int",
        "float",
        "attr",
        "first",
        "last",
        "min",
        "max",
        "sort",
        "list",
        "string",
        "bool",
        "batch",
        "slice",
        "sum",
        "indent",
        "select",
        "reject",
        "selectattr",
        "rejectattr",
        "map",
        "groupby",
        "unique",
        "chain",
        "zip",
        "pprint",
        "format",
        "tojson",
        "urlencode",
        // Revel
        "limit",
        "without",
        "by_slug",
        "by_slugs",
        "where_contains",
        "find_by",
        "date",
        "iso_date",
        "utc_string",
        "strip_tags",
        "truncate",
        "json",
        "absolute_url",
        "regex_replace",
        "regex_capture",
        "media_exists",
        "share_image",
    ]
}

fn used_filters(site: &Site) -> Vec<String> {
    let re = Regex::new(r#"\|\s*([A-Za-z_]\w*)"#).expect("regex");
    let mut found: Vec<String> = Vec::new();
    let mut collect = |e: &str| -> String {
        let stripped = strip_strings(e);
        for c in re.captures_iter(&stripped) {
            found.push(c[1].to_string());
        }
        e.to_string()
    };
    let mut all: Vec<Vec<Node>> = Vec::new();
    all.extend(site.pages.iter().map(|p| p.body.clone()));
    all.extend(site.layouts.iter().map(|l| l.body.clone()));
    all.extend(site.components.iter().map(|c| c.body.clone()));
    let cell = std::cell::RefCell::new(&mut collect);
    for mut nodes in all {
        map_exprs(&mut nodes, &|e| (cell.borrow_mut())(e));
    }
    found.sort();
    found.dedup();
    found
}

fn strip_strings(e: &str) -> String {
    let mut out = String::new();
    let mut quote: Option<char> = None;
    for c in e.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detections() {
        let cfg = r#"
const SRC = "src";
eleventyConfig.addPassthroughCopy({ "src/media": "media", "src/static": "/" });
eleventyConfig.addCollection("projets", (api) =>
    api.getFilteredByGlob("src/content/projets/*.md").filter(published).sort(byDateDesc));
eleventyConfig.addCollection("equipe", (api) =>
    api.getFilteredByGlob("src/content/equipe/*.md").filter(published).sort((a, b) => (a.data.order || 0) - (b.data.order || 0)));
eleventyConfig.addCollection("sitemap", (api) => api.getAll().filter((p) => p.url));
return { dir: { input: SRC } };
"#;
        assert_eq!(
            detect_input(cfg, Path::new("/nope")).as_deref(),
            Some("src")
        );
        assert_eq!(
            detect_passthrough(cfg),
            vec![
                ("src/media".into(), "media".into()),
                ("src/static".into(), "/".into())
            ]
        );
        let cols = detect_collections(cfg);
        assert_eq!(cols.len(), 2);
        assert!(cols[0].published);
        assert_eq!(cols[0].sort.as_ref().unwrap().field, "date");
        assert_eq!(cols[1].sort.as_ref().unwrap().kind, SortKind::Number);
        let js = "export default {\n  layout: \"layouts/project.njk\",\n  eleventyComputed: { permalink: (d) => (d.published === false ? false : `/project/${d.slug}/`) } };";
        assert_eq!(
            parse_dir_data(js),
            (
                Some("layouts/project.njk".into()),
                Some("/project/{{ slug }}/".into())
            )
        );
        assert_eq!(
            parse_dir_data("export default { permalink: false };"),
            (None, None)
        );
        assert_eq!(default_permalink("agence.njk"), "/agence/");
        let c = parse_computed(
            r#"export default { eleventyComputed: { seo: (d) => d.pages["mentions-legales"].seo } };"#,
        );
        assert_eq!(
            c.get("seo").map(String::as_str),
            Some(r#"pages["mentions-legales"].seo"#)
        );
        assert_eq!(default_permalink("index.njk"), "/");
    }
}
