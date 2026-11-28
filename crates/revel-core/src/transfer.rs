//! Imports de pages et de composants, export de composants (D-206).
//!
//! Trois sources, lues comme un site puis fusionnées dans le site ouvert par
//! [`revel_edit::merge`] (une seule étape d'annulation) :
//! - un autre site Revel (dossier) ;
//! - des fichiers HTML ou un dossier : copiés avec les fichiers qu'ils lisent (images, feuilles de
//!   style, polices) dans un dossier temporaire, importés comme un site (7.1), remis dans la forme
//!   de l'interface ; en composants, chaque fichier est importé seul et son corps devient un
//!   composant ;
//! - un fichier de composants (`.revel-component`) : archive ZIP d'un petit site (composants,
//!   styles, médias), écrite par l'export. Son code (scripts, attributs `on…`) est retiré.
//!
//! Les médias lus par ce qui est importé sont copiés dans le dossier des médias du site ouvert
//! (fichier identique réutilisé, sinon nom numéroté).

use crate::{Core, CoreError, Result};
use revel_edit::merge::{self, Merge, Selection};
use revel_model::Site;
use revel_project::Project;
use revel_protocol::{ImportChoice, ImportItem, ImportList, ImportPlan, ImportSource};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{Read, Write};
use std::path::{Component as PathPart, Path, PathBuf};
use std::sync::Arc;

/// Extension d'un fichier de composants.
pub const COMPONENT_EXT: &str = "revel-component";
/// Version du format du fichier de composants.
const FORMAT: u32 = 1;
/// Taille maximale d'un fichier de composants, et des fichiers copiés pour un import HTML.
const SOURCE_MAX: u64 = 512 * 1024 * 1024;
/// Pages HTML lues au plus.
const HTML_MAX: usize = 400;

/// Source lue, gardée pour la fenêtre d'import.
pub(crate) struct Cached {
    key: ImportSource,
    stamp: u128,
    source: Arc<Source>,
}

pub(crate) struct Source {
    project: Project,
    name: String,
    /// Fichier HTML d'origine de chaque page ou composant (import de fichiers HTML).
    files: BTreeMap<String, String>,
    warnings: Vec<String>,
    _tmp: Option<tempfile::TempDir>,
}

/// Description d'un fichier de composants (`component.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Meta {
    format: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    components: Vec<String>,
    #[serde(default)]
    revel: String,
}

fn source_err(detail: impl std::fmt::Display) -> CoreError {
    CoreError::Template("error-import-source", detail.to_string())
}

fn file_err(detail: impl std::fmt::Display) -> CoreError {
    CoreError::Template("error-component-file", detail.to_string())
}

/// Date de modification (pour relire une source changée).
fn mtime(p: &Path) -> u128 {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn stamp(src: &ImportSource) -> u128 {
    match src {
        ImportSource::Site { path } => mtime(&Path::new(path).join(revel_project::SITE_FILE)),
        ImportSource::Component { path } => mtime(Path::new(path)),
        ImportSource::Html { paths, .. } => paths
            .iter()
            .map(|p| mtime(Path::new(p)))
            .fold(0u128, u128::wrapping_add),
    }
}

fn is_html(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"))
}

/// Chemin sans `.` ni `..` (lexical).
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            PathPart::ParentDir => {
                out.pop();
            }
            PathPart::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Adresses de fichiers dans un texte HTML ou CSS : valeurs entre guichets, `url(…)`, listes
/// `srcset`. Le tri se fait ensuite (fichier existant dans le dossier).
fn references(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut push = |v: &str| {
        for part in v.split(',') {
            let t = part.split_whitespace().next().unwrap_or("").trim();
            let t = t.trim_matches(['"', '\'']);
            if t.is_empty()
                || t.len() > 400
                || t.contains("://")
                || t.starts_with("//")
                || t.starts_with('#')
                || t.starts_with("data:")
                || t.starts_with("mailto:")
                || t.starts_with("tel:")
                || t.starts_with("javascript:")
                || t.contains(['<', '>', '{', '}'])
            {
                continue;
            }
            let t = t.split(['?', '#']).next().unwrap_or(t);
            if t.rsplit('/').next().is_some_and(|f| f.contains('.')) {
                out.push(percent_decode(t));
            }
        }
    };
    for quote in ['"', '\''] {
        let mut parts = text.split(quote);
        parts.next();
        while let Some(inner) = parts.next() {
            push(inner);
            parts.next();
        }
    }
    let mut rest = text;
    while let Some(i) = rest.find("url(") {
        let s = &rest[i + 4..];
        let end = s.find(')').unwrap_or(s.len());
        push(s[..end].trim());
        rest = &s[end..];
    }
    let mut rest = text;
    while let Some(i) = rest.find("@import") {
        let s = &rest[i + 7..];
        let end = s.find(';').unwrap_or(s.len());
        push(
            s[..end]
                .trim()
                .trim_start_matches("url(")
                .trim_end_matches(')'),
        );
        rest = &s[end..];
    }
    out
}

/// Copie dans `dest` les pages HTML `files` (relatives à `root`) et les fichiers qu'elles lisent,
/// de proche en proche (feuilles de style). Les autres pages liées ne sont pas reprises.
fn stage(root: &Path, files: &[PathBuf], dest: &Path) -> Result<()> {
    let root = normalize(root);
    let mut todo: Vec<PathBuf> = files.iter().map(|f| normalize(f)).collect();
    let mut done: BTreeSet<PathBuf> = BTreeSet::new();
    let mut total = 0u64;
    let pages: BTreeSet<PathBuf> = todo.iter().cloned().collect();
    while let Some(file) = todo.pop() {
        if !done.insert(file.clone()) {
            continue;
        }
        let Ok(rel) = file.strip_prefix(&root) else {
            continue;
        };
        let meta = match std::fs::metadata(&file) {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };
        total += meta.len();
        if total > SOURCE_MAX {
            return Err(source_err("> 512 MB"));
        }
        let out = dest.join(rel);
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::copy(&file, &out)?;
        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "html" | "htm" | "css" | "svg") || meta.len() > 16 << 20 {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let dir = file.parent().unwrap_or(&root).to_path_buf();
        for r in references(&text) {
            let target = if let Some(abs) = r.strip_prefix('/') {
                root.join(abs)
            } else {
                dir.join(&r)
            };
            let target = normalize(&target);
            if target.starts_with(&root)
                && !done.contains(&target)
                && (!is_html(&target) || pages.contains(&target))
                && target.is_file()
            {
                todo.push(target);
            }
        }
    }
    Ok(())
}

/// Pages HTML données : fichiers, ou toutes celles d'un dossier ; dossier commun.
fn html_files(paths: &[String]) -> Result<(PathBuf, Vec<PathBuf>)> {
    let mut files: Vec<PathBuf> = Vec::new();
    for p in paths {
        let p = Path::new(p);
        if p.is_dir() {
            files.extend(revel_import::html::scan(p).into_iter().map(|r| p.join(r)));
        } else if is_html(p) && p.is_file() {
            files.push(p.to_path_buf());
        }
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err(source_err("0 HTML"));
    }
    files.truncate(HTML_MAX);
    let files: Vec<PathBuf> = files.iter().map(|f| normalize(f)).collect();
    // Dossier commun : celui d'un dossier donné seul, sinon l'ancêtre commun des fichiers.
    let root = match paths {
        [one] if Path::new(one).is_dir() => normalize(Path::new(one)),
        _ => {
            let mut root = files[0].parent().unwrap_or(Path::new("/")).to_path_buf();
            for f in &files[1..] {
                while !f.starts_with(&root) {
                    if !root.pop() {
                        break;
                    }
                }
            }
            root
        }
    };
    Ok((root, files))
}

/// Nom d'une page : son nom, le titre de son référencement, sinon la fin de son adresse.
fn page_name(p: &revel_model::Page) -> String {
    if p.name.as_deref().is_some_and(|n| !n.trim().is_empty()) {
        return revel_edit::linkmap::page_label(p);
    }
    let title = p
        .props
        .get("seo")
        .and_then(|s| s.get("title"))
        .or_else(|| p.props.get("title"))
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|t| !t.is_empty() && !t.contains("{{"));
    match title {
        Some(t) => t.to_string(),
        None => revel_edit::linkmap::page_label(p),
    }
}

/// Page qu'on peut importer : adresse fixe d'une page HTML.
fn importable(p: &revel_model::Page) -> bool {
    !p.path.contains("{{")
        && !p.path.contains('*')
        && (p.path.ends_with('/') || p.path.ends_with(".html"))
}

/// Importe un dossier de pages HTML préparé (`dir`) comme un site, remis dans la forme de
/// l'interface.
fn html_project(dir: &Path, name: &str, lang: &str) -> Result<(Project, Vec<String>)> {
    let (site, report) = revel_import::html::import(
        dir,
        &revel_import::html::Options {
            name: name.to_string(),
            lang: lang.to_string(),
            merge_styles: true,
            ..Default::default()
        },
    )?;
    let mut p = Project::create(dir, site)?;
    let mut warnings = report.warnings;
    warnings.extend(crate::rebuild_for_ui(&mut p));
    Ok((p, warnings))
}

/// `/offre.html` → `/offre/` (pages de Revel), liens entre pages suivis.
fn folder_paths(site: &mut Site) {
    let mut map: HashMap<String, String> = HashMap::new();
    for p in site.pages.iter_mut() {
        if let Some(stem) = p.path.strip_suffix(".html").or(p.path.strip_suffix(".htm")) {
            let to = format!("{stem}/");
            if !map.values().any(|v| *v == to) {
                map.insert(p.path.clone(), to.clone());
                p.path = to;
            }
        }
    }
    if map.is_empty() {
        return;
    }
    let fix = |nodes: &mut Vec<revel_model::Node>| {
        fn walk(nodes: &mut [revel_model::Node], map: &HashMap<String, String>) {
            for n in nodes {
                if let revel_model::Node::Element { attrs, .. } = n {
                    for a in attrs.iter_mut().filter(|a| a.name == "href") {
                        let Some(v) = a.value.as_mut() else { continue };
                        let (base, rest) = match v.find(['#', '?']) {
                            Some(i) => (v[..i].to_string(), v[i..].to_string()),
                            None => (v.clone(), String::new()),
                        };
                        if let Some(to) = map.get(&base) {
                            *v = format!("{to}{rest}");
                        }
                    }
                }
                for list in revel_model::ids::child_lists_mut(n) {
                    walk(list, map);
                }
            }
        }
        walk(nodes, &map);
    };
    for p in site.pages.iter_mut() {
        fix(&mut p.body);
    }
    for c in site.components.iter_mut() {
        fix(&mut c.body);
    }
    for l in site.layouts.iter_mut() {
        fix(&mut l.body);
    }
}

/// Contenu commun des pages HTML (gabarit déduit à l'import, 7.1) remis dans chaque page : une
/// page importée garde tout le contenu de son `<body>` et prend le gabarit du site ouvert. Seul le
/// gabarit du document (`<html>`, `<head>`) est laissé.
fn unfold_layouts(site: &mut Site) {
    fn fill(nodes: &mut Vec<revel_model::Node>, own: &[revel_model::Node]) -> bool {
        let mut done = false;
        let mut out = Vec::with_capacity(nodes.len());
        for mut n in std::mem::take(nodes) {
            if matches!(n, revel_model::Node::Slot) && !done {
                out.extend(own.iter().cloned());
                done = true;
                continue;
            }
            if !done {
                for list in revel_model::ids::child_lists_mut(&mut n) {
                    if fill(list, own) {
                        done = true;
                        break;
                    }
                }
            }
            out.push(n);
        }
        *nodes = out;
        done
    }
    let document = |l: &revel_model::Layout| {
        let mut doc = false;
        revel_model::ids::walk(&l.body, &mut |n| {
            if let revel_model::Node::Element { tag, .. } = n
                && (tag == "html" || tag == "head")
            {
                doc = true;
            }
        });
        doc
    };
    let layouts = site.layouts.clone();
    for p in site.pages.iter_mut() {
        let mut depth = 0;
        while let Some(l) = p
            .layout
            .as_deref()
            .and_then(|id| layouts.iter().find(|l| l.id == id))
        {
            if document(l) || depth > 8 {
                break;
            }
            let mut body = l.body.clone();
            if !fill(&mut body, &p.body) {
                break;
            }
            p.body = body;
            p.layout = l.parent.clone();
            depth += 1;
        }
    }
}

/// Nom lisible d'un fichier (`offre-speciale.html` → `Offre speciale`).
fn pretty(stem: &str) -> String {
    let s = stem.replace(['-', '_'], " ");
    let mut c = s.trim().chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => "Composant".into(),
    }
}

/// Fichiers de même contenu.
fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(x), Ok(y)) if x.len() == y.len() => std::fs::read(a).ok() == std::fs::read(b).ok(),
        _ => false,
    }
}

/// Chemin d'une adresse sous le dossier copié tel quel qui la publie (`/media/a/b.jpg` →
/// `a/b.jpg`).
fn rel_in(site: &Site, url: &str) -> String {
    let url = percent_decode(url.split(['?', '#']).next().unwrap_or(url));
    let mut best: Option<String> = None;
    for p in &site.passthrough {
        let to = p.to.trim_matches('/');
        let prefix = if to.is_empty() {
            "/".to_string()
        } else {
            format!("/{to}/")
        };
        if let Some(rest) = url.strip_prefix(&prefix)
            && best.as_ref().is_none_or(|b| rest.len() < b.len())
        {
            best = Some(rest.to_string());
        }
    }
    best.unwrap_or_else(|| url.trim_start_matches('/').to_string())
}

/// Placement des médias d'une source dans un site : adresse d'arrivée, fichiers à copier.
struct Placer<'a> {
    source: &'a Project,
    target: &'a Project,
    dir: PathBuf,
    prefix: String,
    /// Adresse d'arrivée → fichier source (réservations de cette fusion).
    reserved: HashMap<String, PathBuf>,
    copies: Vec<(PathBuf, PathBuf)>,
}

impl<'a> Placer<'a> {
    fn new(source: &'a Project, target: &'a Project, dir: PathBuf, prefix: String) -> Self {
        Self {
            source,
            target,
            dir,
            prefix,
            reserved: HashMap::new(),
            copies: Vec::new(),
        }
    }

    fn place(&mut self, url: &str) -> Option<String> {
        revel_protocol::MediaKind::of(url)?;
        let from = self.source.source_of_url(url)?;
        if let Some(f) = self.target.source_of_url(url)
            && same_file(&f, &from)
        {
            return Some(url.to_string());
        }
        let mut rel = rel_in(&self.source.site, url);
        // `/media/a.jpg` d'un site sans dossier des médias déclaré : pas de `media/media/`.
        let head = format!("{}/", self.prefix.trim_matches('/'));
        if let Some(r) = rel.strip_prefix(&head) {
            rel = r.to_string();
        }
        if !rel
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != "..")
        {
            rel = from.file_name()?.to_string_lossy().to_string();
        }
        let (stem, ext) = match rel.rsplit_once('.') {
            Some((s, e)) => (s.to_string(), format!(".{e}")),
            None => (rel.clone(), String::new()),
        };
        let mut n = 1;
        loop {
            let name = if n == 1 {
                rel.clone()
            } else {
                format!("{stem}-{n}{ext}")
            };
            let to_url = format!("{}/{}", self.prefix.trim_end_matches('/'), name);
            let to_file = self.dir.join(&name);
            match self.reserved.get(&to_url) {
                Some(f) if same_file(f, &from) => return Some(to_url),
                Some(_) => {}
                None if to_file.exists() => {
                    if same_file(&to_file, &from) {
                        return Some(to_url);
                    }
                }
                None => {
                    self.reserved.insert(to_url.clone(), from.clone());
                    self.copies.push((from, to_file));
                    return Some(to_url);
                }
            }
            n += 1;
        }
    }
}

/// Dossier des médias d'un site, sans le créer (voir [`Core::media_dir`]).
fn media_target(p: &Project) -> (PathBuf, String) {
    p.site
        .passthrough
        .iter()
        .find(|x| x.to.trim_matches('/') == "media")
        .or_else(|| {
            p.site
                .passthrough
                .iter()
                .find(|x| p.dir.join(&x.from).is_dir() && !x.to.trim_matches('/').is_empty())
        })
        .map(|x| (p.dir.join(&x.from), format!("/{}", x.to.trim_matches('/'))))
        .unwrap_or_else(|| (p.dir.join("media"), "/media".into()))
}

fn plan_of(sel: &Selection, source: &Site, m: &Merge) -> ImportPlan {
    let ids: Vec<&String> = sel
        .pages
        .iter()
        .filter(|id| source.page(id).is_some())
        .collect();
    ImportPlan {
        pages: ids
            .into_iter()
            .zip(&m.pages)
            .map(|(id, p)| (id.clone(), p.path.clone()))
            .collect(),
        components: m.components.iter().map(|c| c.id.clone()).collect(),
        reused: m.reused,
        styles: m.styles.len(),
        vars: m.vars.len(),
        media: 0,
        renamed: m.renamed.clone(),
    }
}

fn selection(c: &ImportChoice) -> Selection {
    Selection {
        pages: c.pages.clone(),
        components: c.components.clone(),
        uses: c.uses,
        styles: c.styles,
        media: c.media,
    }
}

/// Site vide qui reçoit des composants (export, import de fichiers HTML en composants).
fn empty_site(name: &str, lang: &str) -> Site {
    let mut s = Site::new(name, lang);
    s.stylesheet = Some(revel_model::Stylesheet {
        url: revel_edit::style::DEFAULT_URL.into(),
        source: None,
        items: Vec::new(),
    });
    s.passthrough.push(revel_model::Passthrough {
        from: "media".into(),
        to: "media".into(),
    });
    s
}

/// Applique une fusion à un site hors de l'éditeur et copie ses médias.
fn merge_into(target: &mut Project, source: &Project, sel: &Selection) -> Result<Merge> {
    let (dir, prefix) = media_target(target);
    let snapshot = Project {
        dir: target.dir.clone(),
        site: target.site.clone(),
    };
    let mut placer = Placer::new(source, &snapshot, dir, prefix);
    let m = merge::merge(&target.site, &source.site, sel, &mut |u| placer.place(u));
    for (from, to) in &placer.copies {
        if let Some(p) = to.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::copy(from, to)?;
    }
    let cmds = merge::commands(&target.site, &m);
    revel_edit::apply(
        &mut target.site,
        &revel_edit::Command::Batch { commands: cmds },
    )
    .map_err(CoreError::Edit)?;
    Ok(m)
}

impl Core {
    /// Source lue (gardée tant qu'elle ne change pas).
    fn import_source_of(&self, src: &ImportSource, lang: &str) -> Result<Arc<Source>> {
        let st = stamp(src);
        {
            let kept = self.import_source.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(c) = kept.as_ref()
                && c.key == *src
                && c.stamp == st
            {
                return Ok(c.source.clone());
            }
        }
        let source = Arc::new(match src {
            ImportSource::Site { path } => {
                let dir = Path::new(path);
                if !dir.join(revel_project::SITE_FILE).is_file() {
                    return Err(CoreError::NotASite);
                }
                let project = Project::open(dir)?;
                Source {
                    name: project.site.name.clone(),
                    project,
                    files: BTreeMap::new(),
                    warnings: Vec::new(),
                    _tmp: None,
                }
            }
            ImportSource::Component { path } => read_component_file(Path::new(path))?,
            ImportSource::Html { paths, components } => {
                if *components {
                    html_components(paths, lang)?
                } else {
                    html_pages(paths, lang)?
                }
            }
        });
        *self.import_source.lock().unwrap_or_else(|e| e.into_inner()) = Some(Cached {
            key: src.clone(),
            stamp: st,
            source: source.clone(),
        });
        Ok(source)
    }

    fn site_lang(&self, site: &Path) -> String {
        self.with_project(site, |p| Ok(p.site.default_locale.clone()))
            .unwrap_or_else(|_| "en".into())
    }

    /// Pages et composants d'une source.
    pub fn import_list(&self, site: &Path, src: &ImportSource) -> Result<ImportList> {
        if let ImportSource::Site { path } = src
            && normalize(Path::new(path)) == normalize(site)
        {
            return Err(source_err(path));
        }
        let s = self.import_source_of(src, &self.site_lang(site))?;
        let mut pages: Vec<ImportItem> = s
            .project
            .site
            .pages
            .iter()
            .filter(|p| importable(p))
            .map(|p| ImportItem {
                id: p.id.clone(),
                name: page_name(p),
                path: p.path.clone(),
                props: 0,
                file: s.files.get(&p.id).cloned(),
            })
            .collect();
        if matches!(src, ImportSource::Html { .. }) {
            pages.sort_by(|a, b| a.file.cmp(&b.file));
        }
        let components = s
            .project
            .site
            .components
            .iter()
            .map(|c| ImportItem {
                id: c.id.clone(),
                name: c.label(),
                path: String::new(),
                props: c.props.len(),
                file: s.files.get(&c.id).cloned(),
            })
            .collect();
        Ok(ImportList {
            name: s.name.clone(),
            pages,
            components,
            warnings: s.warnings.clone(),
        })
    }

    /// Fusion préparée : bilan, fusion, fichiers à copier.
    fn import_prepare(
        &self,
        site: &Path,
        src: &ImportSource,
        choice: &ImportChoice,
    ) -> Result<(ImportPlan, Merge, Vec<(PathBuf, PathBuf)>)> {
        let source = self.import_source_of(src, &self.site_lang(site))?;
        let target = self.with_project(site, |p| {
            Ok(Project {
                dir: p.dir.clone(),
                site: p.site.clone(),
            })
        })?;
        let (dir, prefix) = media_target(&target);
        let sel = selection(choice);
        let mut placer = Placer::new(&source.project, &target, dir, prefix);
        let m = merge::merge(&target.site, &source.project.site, &sel, &mut |u| {
            placer.place(u)
        });
        let mut plan = plan_of(&sel, &source.project.site, &m);
        plan.media = placer.copies.len();
        Ok((plan, m, placer.copies))
    }

    /// Ce que l'import ajoutera (bilan affiché avant l'import).
    pub fn import_plan(
        &self,
        site: &Path,
        src: &ImportSource,
        choice: &ImportChoice,
    ) -> Result<ImportPlan> {
        Ok(self.import_prepare(site, src, choice)?.0)
    }

    /// Importe les pages et composants choisis dans le site ouvert (une étape d'annulation).
    pub fn import_apply(
        &self,
        site: &Path,
        src: &ImportSource,
        choice: &ImportChoice,
    ) -> Result<(revel_edit::EditState, ImportPlan)> {
        if choice.media {
            // Dossier des médias créé et déclaré s'il manque.
            self.media_dir(site)?;
        }
        let (plan, m, copies) = self.import_prepare(site, src, choice)?;
        // Tout est déjà là à l'identique (composant réimporté) : rien à ajouter.
        if m.is_empty() {
            return Ok((self.session_state(site)?, plan));
        }
        let mut written = Vec::new();
        for (from, to) in &copies {
            if let Some(p) = to.parent() {
                std::fs::create_dir_all(p)?;
            }
            if let Err(e) = std::fs::copy(from, to) {
                for f in &written {
                    let _ = std::fs::remove_file(f);
                }
                return Err(e.into());
            }
            written.push(to.clone());
        }
        let target = self.with_project(site, |p| Ok(p.site.clone()))?;
        let commands = merge::commands(&target, &m);
        match self.edit_apply(site, revel_edit::Command::Batch { commands }, false) {
            Ok(state) => Ok((state, plan)),
            Err(e) => {
                for f in &written {
                    let _ = std::fs::remove_file(f);
                }
                Err(e)
            }
        }
    }

    /// Écrit les composants `ids` du site ouvert, avec ce qu'ils utilisent (composants, styles,
    /// médias), dans un fichier de composants `dest`.
    pub fn export_components(&self, site: &Path, ids: &[String], dest: &Path) -> Result<PathBuf> {
        let source = self.with_project(site, |p| {
            Ok(Project {
                dir: p.dir.clone(),
                site: p.site.clone(),
            })
        })?;
        if ids.is_empty() || ids.iter().any(|i| source.site.component(i).is_none()) {
            return Err(source_err(ids.join(", ")));
        }
        let tmp = tempfile::tempdir()?;
        let names: Vec<String> = ids
            .iter()
            .filter_map(|i| source.site.component(i))
            .map(|c| c.label())
            .collect();
        let mut out = Project {
            dir: tmp.path().to_path_buf(),
            site: empty_site(&names.join(", "), &source.site.default_locale),
        };
        merge_into(
            &mut out,
            &source,
            &Selection::all(Vec::new(), ids.to_vec(), true),
        )?;
        let meta = Meta {
            format: FORMAT,
            name: names.join(", "),
            components: ids.to_vec(),
            revel: env!("CARGO_PKG_VERSION").into(),
        };
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            let zerr = |e: zip::result::ZipError| file_err(e);
            z.start_file("component.json", opts).map_err(zerr)?;
            z.write_all(&serde_json::to_vec_pretty(&meta).map_err(file_err)?)?;
            z.start_file(format!("site/{}", revel_project::SITE_FILE), opts)
                .map_err(zerr)?;
            z.write_all(out.site.to_json().map_err(file_err)?.as_bytes())?;
            let media = tmp.path().join("media");
            let mut stack = vec![media.clone()];
            while let Some(d) = stack.pop() {
                let Ok(rd) = std::fs::read_dir(&d) else {
                    continue;
                };
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if let Ok(rel) = p.strip_prefix(tmp.path()) {
                        let rel = rel.to_string_lossy().replace('\\', "/");
                        z.start_file(format!("site/{rel}"), opts).map_err(zerr)?;
                        z.write_all(&std::fs::read(&p)?)?;
                    }
                }
            }
            z.finish().map_err(zerr)?;
        }
        let dest = if dest
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(COMPONENT_EXT))
        {
            dest.to_path_buf()
        } else {
            let mut d = dest.as_os_str().to_owned();
            d.push(format!(".{COMPONENT_EXT}"));
            PathBuf::from(d)
        };
        std::fs::write(&dest, buf.into_inner())?;
        Ok(dest)
    }
}

/// Lit un fichier de composants dans un dossier temporaire.
fn read_component_file(path: &Path) -> Result<Source> {
    let size = std::fs::metadata(path)?.len();
    if size > SOURCE_MAX {
        return Err(file_err("> 512 MB"));
    }
    let bytes = std::fs::read(path)?;
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(file_err)?;
    let tmp = tempfile::tempdir()?;
    let mut meta: Option<Meta> = None;
    let mut doc: Option<Vec<u8>> = None;
    let mut total = 0u64;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(file_err)?;
        if f.is_dir() {
            continue;
        }
        let name = f.name().to_string();
        total += f.size();
        if total > SOURCE_MAX {
            return Err(file_err("> 512 MB"));
        }
        let mut b = Vec::new();
        f.by_ref().take(SOURCE_MAX).read_to_end(&mut b)?;
        if name == "component.json" {
            meta = Some(serde_json::from_slice(&b).map_err(file_err)?);
        } else if name == format!("site/{}", revel_project::SITE_FILE) {
            doc = Some(b);
        } else if let Some(rel) = name.strip_prefix("site/")
            && crate::sharing::safe_rel(rel)
            && !crate::sharing::is_code(rel)
        {
            let out = tmp.path().join(rel);
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::write(out, b)?;
        }
    }
    let meta = meta.ok_or_else(|| file_err("component.json"))?;
    if meta.format > FORMAT {
        return Err(file_err(format!("format {} ({})", meta.format, meta.revel)));
    }
    let mut doc: serde_json::Value =
        serde_json::from_slice(&doc.ok_or_else(|| file_err(revel_project::SITE_FILE))?)
            .map_err(file_err)?;
    let removed = crate::sharing::sanitize(&mut doc);
    let site = Site::from_json(&doc.to_string()).map_err(file_err)?;
    let name = if meta.name.trim().is_empty() {
        path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        meta.name.clone()
    };
    Ok(Source {
        project: Project {
            dir: tmp.path().to_path_buf(),
            site,
        },
        name,
        files: BTreeMap::new(),
        warnings: if removed > 0 {
            vec![format!("{removed} code")]
        } else {
            Vec::new()
        },
        _tmp: Some(tmp),
    })
}

/// Fichiers HTML importés comme pages.
fn html_pages(paths: &[String], lang: &str) -> Result<Source> {
    let (root, files) = html_files(paths)?;
    let tmp = tempfile::tempdir()?;
    stage(&root, &files, tmp.path())?;
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "HTML".into());
    let (mut project, warnings) =
        html_project(tmp.path(), &crate::import::pretty_name(&name), lang)?;
    // Fichier de chaque page (adresse d'après le chemin du fichier, 7.1).
    let mut files_of: BTreeMap<String, String> = BTreeMap::new();
    for f in &files {
        let Ok(rel) = f.strip_prefix(&root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        let url = revel_import::html::page_url(&rel);
        if let Some(p) = project.site.pages.iter().find(|p| p.path == url) {
            files_of.insert(p.id.clone(), rel);
        }
    }
    folder_paths(&mut project.site);
    unfold_layouts(&mut project.site);
    Ok(Source {
        project,
        name,
        files: files_of,
        warnings,
        _tmp: Some(tmp),
    })
}

/// Fichiers HTML importés comme composants : chaque fichier seul (pas de gabarit commun), son
/// corps devient un composant ; tous réunis dans un site, avec leurs styles et médias.
fn html_components(paths: &[String], lang: &str) -> Result<Source> {
    let (root, files) = html_files(paths)?;
    let tmp = tempfile::tempdir()?;
    let mut all = Project {
        dir: tmp.path().join("all"),
        site: empty_site("HTML", lang),
    };
    std::fs::create_dir_all(&all.dir)?;
    let mut files_of = BTreeMap::new();
    let mut warnings = Vec::new();
    for (i, f) in files.iter().enumerate() {
        let dir = tmp.path().join(format!("f{i}"));
        std::fs::create_dir_all(&dir)?;
        stage(&root, std::slice::from_ref(f), &dir)?;
        let stem = f
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let (mut one, w) = match html_project(&dir, &pretty(&stem), lang) {
            Ok(x) => x,
            Err(e) => {
                warnings.push(format!("{stem} : {e}"));
                continue;
            }
        };
        warnings.extend(w);
        let Some(page) = one.site.pages.first().cloned() else {
            continue;
        };
        let id = revel_edit::make::free_component_id(&all.site, &stem);
        let mut c = revel_model::Component::new(id.clone(), page.body);
        c.name = Some(pretty(&stem));
        one.site.components.retain(|x| x.id != id);
        one.site.components.push(c);
        merge_into(
            &mut all,
            &one,
            &Selection::all(Vec::new(), vec![id.clone()], true),
        )?;
        let rel = f
            .strip_prefix(&root)
            .map(|r| r.to_string_lossy().replace('\\', "/"))
            .unwrap_or(stem);
        files_of.insert(id, rel);
    }
    if all.site.components.is_empty() {
        return Err(source_err(warnings.join(" ; ")));
    }
    Ok(Source {
        name: root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "HTML".into()),
        project: all,
        files: files_of,
        warnings,
        _tmp: Some(tmp),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_des_pages() {
        let r = references(
            r#"<img src="img/a.png" srcset="img/a@2x.png 2x, /img/b.webp 3x"><a href="offre.html#x">
            <a href="https://x.com/a.png"><div style="background:url('fond.jpg')"></div>
            <style>@import "theme.css"; .a{background:url(data:image/png;base64,xx)}</style>"#,
        );
        for want in [
            "img/a.png",
            "img/a@2x.png",
            "/img/b.webp",
            "offre.html",
            "fond.jpg",
            "theme.css",
        ] {
            assert!(r.iter().any(|x| x == want), "{want} : {r:?}");
        }
        assert!(
            !r.iter()
                .any(|x| x.contains("x.com") || x.starts_with("data:"))
        );
    }

    #[test]
    fn adresse_sous_le_dossier_copie() {
        let mut s = Site::new("a", "fr");
        s.passthrough.push(revel_model::Passthrough {
            from: "media".into(),
            to: "media".into(),
        });
        assert_eq!(rel_in(&s, "/media/a/b%20c.jpg?x"), "a/b c.jpg");
        assert_eq!(rel_in(&s, "/img/x.png"), "img/x.png");
    }
}

/// Dossier des médias d'un site (essais).
#[cfg(test)]
pub(crate) fn tests_media(p: &Project) -> (PathBuf, String) {
    media_target(p)
}
