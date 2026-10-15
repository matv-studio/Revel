//! `revel-project` : un site Revel sur disque.
//!
//! ```text
//! <projet>/
//!   site.revel.json        document de site (pages, gabarits, composants, collections…)
//!   <dossiers de contenus> fichiers Markdown + front matter (compatibles Pages CMS)
//!   <dossier de données>   fichiers YAML/JSON (variables globales)
//!   <médias, CSS, JS…>     copiés tels quels (passthrough)
//!   .revel/                cache local (images générées), jamais versionné
//! ```
//!
//! Les chemins du document sont relatifs au projet : un dépôt existant (Eleventy, Hugo…)
//! peut accueillir `site.revel.json` à sa racine sans déplacer ses fichiers.

pub mod bound;
pub mod entries;
pub mod pagesyml;
pub mod yamlpath;

use revel_model::Site;
use revel_render::{
    AssetHooks, BuildInput, DocKind, EditorOptions, EditorPage, Entry, ImageArgs, OutputFile,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const SITE_FILE: &str = "site.revel.json";
pub const CACHE_DIR: &str = ".revel";

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("{path} : {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("{path} : {msg}")]
    Parse { path: String, msg: String },
    #[error(transparent)]
    Model(#[from] revel_model::ModelError),
    #[error(transparent)]
    Render(#[from] revel_render::RenderError),
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> ProjectError + '_ {
    move |source| ProjectError::Io {
        path: path.display().to_string(),
        source,
    }
}

pub struct Project {
    pub dir: PathBuf,
    pub site: Site,
}

/// Résultat d'une génération.
#[derive(Debug, Default)]
pub struct BuildReport {
    /// Fichiers écrits (chemins relatifs à la sortie), générés et copiés.
    pub files: Vec<String>,
    pub generated: usize,
    pub copied: usize,
    pub images: revel_assets::Stats,
    pub warnings: Vec<String>,
}

/// Étape signalée pendant une génération.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Pages,
    Copies,
    Images,
}

/// Options de génération.
pub struct BuildOptions<'a> {
    /// Environnement (`production`, `staging`…).
    pub environment: String,
    /// Produire les variantes d'images (sinon `<img>` simple).
    pub images: bool,
    /// Progression : étape, fait, total.
    pub progress: Option<&'a (dyn Fn(Phase, usize, usize) + Sync)>,
}

impl Default for BuildOptions<'_> {
    fn default() -> Self {
        Self {
            environment: "production".into(),
            images: true,
            progress: None,
        }
    }
}

impl Project {
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, ProjectError> {
        let dir = dir.into();
        let path = dir.join(SITE_FILE);
        let text = std::fs::read_to_string(&path).map_err(io(&path))?;
        Ok(Self {
            site: Site::from_json(&text)?,
            dir,
        })
    }

    pub fn create(dir: impl Into<PathBuf>, site: Site) -> Result<Self, ProjectError> {
        let p = Self {
            dir: dir.into(),
            site,
        };
        p.save()?;
        Ok(p)
    }

    pub fn save(&self) -> Result<(), ProjectError> {
        std::fs::create_dir_all(&self.dir).map_err(io(&self.dir))?;
        let path = self.dir.join(SITE_FILE);
        let tmp = self.dir.join(format!("{SITE_FILE}.tmp"));
        std::fs::write(&tmp, self.site.to_json()?).map_err(io(&tmp))?;
        std::fs::rename(&tmp, &path).map_err(io(&path))
    }

    /// Lit les entrées de toutes les collections (une fois par dossier).
    pub fn load_entries(&self) -> Result<Vec<Entry>, ProjectError> {
        let mut folders: Vec<&str> = self
            .site
            .collections
            .iter()
            .filter(|c| !c.folder.is_empty())
            .map(|c| c.folder.as_str())
            .collect();
        folders.sort();
        folders.dedup();
        let mut out = Vec::new();
        for folder in folders {
            let dir = self.dir.join(folder);
            let Ok(rd) = std::fs::read_dir(&dir) else {
                continue;
            };
            let mut files: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "md" || e == "markdown"))
                .collect();
            files.sort();
            for f in files {
                let text = std::fs::read_to_string(&f).map_err(io(&f))?;
                let (data, body) = front_matter(&text).map_err(|msg| ProjectError::Parse {
                    path: f.display().to_string(),
                    msg,
                })?;
                out.push(Entry {
                    folder: folder.to_string(),
                    file_stem: f
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    data,
                    body,
                });
            }
        }
        Ok(out)
    }

    /// Variables issues du dossier de données.
    pub fn load_data(&self) -> Result<BTreeMap<String, Value>, ProjectError> {
        let mut out = BTreeMap::new();
        if let Some(d) = &self.site.data_dir {
            load_data_dir(&self.dir.join(d), &mut out)?;
        }
        Ok(out)
    }

    /// Génère le site dans `out_dir` (vidé au préalable).
    pub fn build(&self, out_dir: &Path, opts: &BuildOptions) -> Result<BuildReport, ProjectError> {
        let mut site = self.site.clone();
        for (k, v) in self.load_data()? {
            site.data.entry(k).or_insert(v);
        }
        let pipeline = opts.images.then(|| {
            Arc::new(revel_assets::Pipeline::new(
                &self.dir,
                &site.passthrough,
                &site.settings.image_widths,
            ))
        });
        let hooks: Arc<dyn AssetHooks> = match &pipeline {
            Some(p) => p.clone(),
            None => Arc::new(ProjectAssets::new(self)),
        };
        let progress = |phase, done, total| {
            if let Some(p) = opts.progress {
                p(phase, done, total);
            }
        };
        progress(Phase::Pages, 0, 1);
        let input = BuildInput {
            site: &site,
            entries: self.load_entries()?,
            environment: opts.environment.clone(),
            version: self.version_tag(&site),
            hooks,
            now: chrono::Utc::now(),
        };
        let out = revel_render::build(&input)?;
        progress(Phase::Pages, 1, 1);
        if out_dir.exists() {
            std::fs::remove_dir_all(out_dir).map_err(io(out_dir))?;
        }
        std::fs::create_dir_all(out_dir).map_err(io(out_dir))?;
        let mut report = BuildReport {
            warnings: out.warnings,
            ..Default::default()
        };
        let skip: Option<PathBuf> = site
            .stylesheet
            .as_ref()
            .and_then(|s| s.source.as_ref())
            .map(|src| self.dir.join(src));
        for (i, p) in site.passthrough.iter().enumerate() {
            progress(Phase::Copies, i, site.passthrough.len());
            let from = self.dir.join(&p.from);
            let to = out_dir.join(p.to.trim_start_matches('/'));
            report.copied += copy_tree(&from, &to, out_dir, &mut report.files, skip.as_deref())?;
        }
        // Feuille de style du modèle (D-18), écrite après les copies.
        if let Some(sheet) = &site.stylesheet {
            let rel_url = sheet.url.trim_start_matches('/');
            if rel_url.is_empty() || rel_url.split('/').any(|c| c == "..") {
                return Err(ProjectError::Parse {
                    path: SITE_FILE.into(),
                    msg: format!("adresse de feuille de style invalide : {}", sheet.url),
                });
            }
            let dest = out_dir.join(rel_url);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(io(parent))?;
            }
            std::fs::write(&dest, revel_style::render(sheet)).map_err(io(&dest))?;
            report.files.push(rel_url.to_string());
            report.generated += 1;
        }
        for OutputFile { path, bytes } in out.files {
            let dest = out_dir.join(&path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(io(parent))?;
            }
            std::fs::write(&dest, bytes).map_err(io(&dest))?;
            report.files.push(path);
            report.generated += 1;
        }
        if let Some(p) = &pipeline {
            let names: Vec<String> = p
                .pending()
                .iter()
                .map(|v| format!("img/{}", v.name))
                .collect();
            report.images = p.process(out_dir, &|d, t| progress(Phase::Images, d, t));
            report.warnings.extend(p.take_errors());
            report.files.extend(names);
        }
        report.files.sort();
        report.files.dedup();
        Ok(report)
    }

    /// Site complété des données du dossier de données (comme pour la génération).
    fn site_with_data(&self) -> Result<Site, ProjectError> {
        let mut site = self.site.clone();
        for (k, v) in self.load_data()? {
            site.data.entry(k).or_insert(v);
        }
        Ok(site)
    }

    /// Page rendue pour l'éditeur (canevas, étape 2.2) à partir du document en mémoire.
    pub fn render_editor(
        &self,
        url: &str,
        hooks: Arc<dyn AssetHooks>,
        environment: &str,
        opts: &EditorOptions,
    ) -> Result<Option<EditorPage>, ProjectError> {
        let site = self.site_with_data()?;
        let input = BuildInput {
            site: &site,
            entries: self.load_entries()?,
            environment: environment.to_string(),
            version: self.version_tag(&site),
            hooks,
            now: chrono::Utc::now(),
        };
        Ok(revel_render::render_editor(&input, url, opts)?)
    }

    /// Pages HTML du site (pages fixes et contenus de collection), pour l'éditeur.
    pub fn list_pages(&self) -> Result<Vec<revel_render::PageRef>, ProjectError> {
        let site = self.site_with_data()?;
        let input = BuildInput {
            site: &site,
            entries: self.load_entries()?,
            environment: "production".into(),
            version: String::new(),
            hooks: Arc::new(ProjectAssets::new(self)),
            now: chrono::Utc::now(),
        };
        Ok(revel_render::list_pages(&input)?)
    }

    /// Vérifie le canevas : chaque page HTML rendue pour l'éditeur, repères retirés, est identique
    /// à la page générée. Renvoie le nombre de pages et les écarts (adresse, extrait).
    pub fn check_canvas(&self, environment: &str) -> Result<(usize, Vec<String>), ProjectError> {
        let site = self.site_with_data()?;
        let input = BuildInput {
            site: &site,
            entries: self.load_entries()?,
            environment: environment.to_string(),
            version: self.version_tag(&site),
            hooks: Arc::new(ProjectAssets::new(self)),
            now: chrono::Utc::now(),
        };
        let out = revel_render::build(&input)?;
        let opts = EditorOptions { scripts: true };
        let mut pages = 0;
        let mut diffs = Vec::new();
        for f in &out.files {
            let url = format!("/{}", f.path.strip_suffix("index.html").unwrap_or(&f.path));
            if DocKind::of_path(&url) != DocKind::Html {
                continue;
            }
            pages += 1;
            let built = String::from_utf8_lossy(&f.bytes);
            match revel_render::render_editor(&input, &url, &opts)? {
                None => diffs.push(format!("{url} : page absente du canevas")),
                Some(p) => {
                    let stripped = revel_render::strip_editor_marks(&p.html);
                    if stripped != built {
                        let at = stripped
                            .bytes()
                            .zip(built.bytes())
                            .position(|(a, b)| a != b)
                            .unwrap_or(stripped.len().min(built.len()));
                        let from = at.saturating_sub(60);
                        let excerpt = |s: &str| {
                            let a = (from..=at)
                                .rev()
                                .find(|i| s.is_char_boundary(*i))
                                .unwrap_or(0);
                            let e = (at + 60).min(s.len());
                            let b = (e..=s.len())
                                .find(|i| s.is_char_boundary(*i))
                                .unwrap_or(s.len());
                            s[a..b].to_string()
                        };
                        diffs.push(format!(
                            "{url} : octet {at}\n  généré : {}\n  canevas : {}",
                            excerpt(&built),
                            excerpt(&stripped)
                        ));
                    }
                }
            }
        }
        Ok((pages, diffs))
    }

    /// Fichier source correspondant à une URL publique copiée telle quelle (`/media/a.jpg`).
    pub fn source_of_url(&self, url: &str) -> Option<PathBuf> {
        let url = url.split(['?', '#']).next().unwrap_or(url);
        let url = percent_decode(url);
        if url.split(['/', '\\']).any(|s| s == "..") {
            return None;
        }
        for p in &self.site.passthrough {
            let prefix = format!("/{}", p.to.trim_matches('/'));
            let prefix = if prefix == "/" {
                String::from("/")
            } else {
                format!("{prefix}/")
            };
            if let Some(rest) = url.strip_prefix(&prefix) {
                let candidate = self.dir.join(&p.from).join(rest);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        None
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Copie d'un site généré dont les adresses depuis la racine sont préfixées par `prefix`
/// (site servi dans un sous-dossier, ex. `proprietaire.github.io/depot/`). Les pages HTML et
/// les feuilles de style sont réécrites ; les autres fichiers sont liés (sans copie) quand le
/// système le permet. `dest` est remplacé.
pub fn rebase_dir(src: &Path, dest: &Path, prefix: &str) -> std::io::Result<usize> {
    if dest.exists() {
        std::fs::remove_dir_all(dest)?;
    }
    fn walk(src: &Path, dest: &Path, prefix: &str, n: &mut usize) -> std::io::Result<()> {
        std::fs::create_dir_all(dest)?;
        for e in std::fs::read_dir(src)? {
            let e = e?;
            let (from, to) = (e.path(), dest.join(e.file_name()));
            if e.file_type()?.is_dir() {
                walk(&from, &to, prefix, n)?;
                continue;
            }
            let ext = from
                .extension()
                .and_then(|x| x.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            match ext.as_str() {
                "html" | "htm" => {
                    let t = std::fs::read_to_string(&from)?;
                    std::fs::write(&to, revel_render::rebase::rebase_html(&t, prefix))?;
                    *n += 1;
                }
                "css" => {
                    let t = std::fs::read_to_string(&from)?;
                    std::fs::write(&to, revel_render::rebase::rebase_css(&t, prefix))?;
                    *n += 1;
                }
                _ => {
                    if std::fs::hard_link(&from, &to).is_err() {
                        std::fs::copy(&from, &to)?;
                    }
                }
            }
        }
        Ok(())
    }
    let mut n = 0;
    walk(src, dest, prefix, &mut n)?;
    Ok(n)
}

/// Extensions des fichiers dont dépend le paramètre de version (`?v=`) des CSS et JS.
const VERSIONED: &[&str] = &["css", "js", "mjs"];

impl Project {
    /// Paramètre de version des CSS/JS : empreinte de leur contenu. Deux générations sans
    /// changement de style ou de script produisent les mêmes pages, et la publication suivante
    /// n'envoie que ce qui a réellement changé.
    fn version_tag(&self, site: &Site) -> String {
        let mut files = Vec::new();
        for p in &site.passthrough {
            collect_files(&self.dir.join(&p.from), &mut files);
        }
        files.retain(|f| {
            f.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| VERSIONED.contains(&e.to_ascii_lowercase().as_str()))
        });
        // La feuille importée n'est plus copiée : c'est le CSS généré qui compte.
        let skip = site
            .stylesheet
            .as_ref()
            .and_then(|s| s.source.as_ref())
            .map(|src| self.dir.join(src));
        files.retain(|f| Some(f) != skip.as_ref());
        files.sort();
        let mut h = blake3::Hasher::new();
        if let Some(sheet) = &site.stylesheet {
            h.update(sheet.url.as_bytes());
            h.update(&[0]);
            h.update(revel_style::render(sheet).as_bytes());
        }
        for f in &files {
            let rel = f.strip_prefix(&self.dir).unwrap_or(f);
            h.update(rel.to_string_lossy().replace('\\', "/").as_bytes());
            h.update(&[0]);
            if let Ok(bytes) = std::fs::read(f) {
                h.update(&bytes);
            }
        }
        let d = h.finalize();
        let mut n = [0u8; 8];
        n.copy_from_slice(&d.as_bytes()[..8]);
        // 41 bits : 8 caractères en base 36, comme l'horodatage utilisé auparavant.
        to_base36(u128::from(u64::from_le_bytes(n) >> 23))
    }
}

/// Fichiers d'un dossier (récursif) ou fichier seul.
fn collect_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        out.push(path.to_path_buf());
    } else if let Ok(rd) = std::fs::read_dir(path) {
        for e in rd.flatten() {
            collect_files(&e.path(), out);
        }
    }
}

fn to_base36(mut n: u128) -> String {
    const D: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".into();
    }
    let mut s = Vec::new();
    while n > 0 {
        s.push(D[(n % 36) as usize]);
        n /= 36;
    }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

/// Copie récursive ; renvoie le nombre de fichiers copiés.
/// Copie un fichier ou un dossier ; `skip` : fichier à ne pas copier (feuille de style importée,
/// générée par Revel).
fn copy_tree(
    from: &Path,
    to: &Path,
    root: &Path,
    list: &mut Vec<String>,
    skip: Option<&Path>,
) -> Result<usize, ProjectError> {
    if skip == Some(from) {
        return Ok(0);
    }
    if from.is_file() {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(io(parent))?;
        }
        std::fs::copy(from, to).map_err(io(from))?;
        list.push(rel(to, root));
        return Ok(1);
    }
    let Ok(rd) = std::fs::read_dir(from) else {
        return Ok(0);
    };
    std::fs::create_dir_all(to).map_err(io(to))?;
    let mut n = 0;
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name();
        if name.to_string_lossy().starts_with(".DS_Store") {
            continue;
        }
        n += copy_tree(&p, &to.join(&name), root, list, skip)?;
    }
    Ok(n)
}

fn rel(p: &Path, root: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Sépare le front matter YAML (`---` … `---`) du corps.
pub fn front_matter(text: &str) -> Result<(Value, String), String> {
    let t = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some(rest) = t.strip_prefix("---") else {
        return Ok((Value::Object(Default::default()), t.to_string()));
    };
    let rest = rest
        .trim_start_matches(['\r', ' '])
        .strip_prefix('\n')
        .unwrap_or(rest);
    let (yaml, body) = match find_fence(rest) {
        Some((a, b)) => (&rest[..a], &rest[b..]),
        None => return Err("front matter non fermé (---)".into()),
    };
    let data = parse_yaml(yaml)?;
    Ok((data, body.trim_start_matches(['\r', '\n']).to_string()))
}

fn find_fence(s: &str) -> Option<(usize, usize)> {
    let mut pos = 0;
    for line in s.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return Some((pos, pos + line.len()));
        }
        pos += line.len();
    }
    None
}

pub fn parse_yaml(yaml: &str) -> Result<Value, String> {
    if yaml.trim().is_empty() {
        return Ok(Value::Object(Default::default()));
    }
    let v: Value = serde_saphyr::from_str(yaml).map_err(|e| e.to_string())?;
    Ok(if v.is_null() {
        Value::Object(Default::default())
    } else {
        v
    })
}

fn load_data_dir(dir: &Path, out: &mut BTreeMap<String, Value>) -> Result<(), ProjectError> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let name = p
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if p.is_dir() {
            let mut sub = BTreeMap::new();
            load_data_dir(&p, &mut sub)?;
            let key = p
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.insert(key, Value::Object(sub.into_iter().collect()));
            continue;
        }
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        let text = match ext {
            "yml" | "yaml" | "json" => std::fs::read_to_string(&p).map_err(io(&p))?,
            _ => continue,
        };
        let v = if ext == "json" {
            serde_json::from_str(&text).map_err(|e| ProjectError::Parse {
                path: p.display().to_string(),
                msg: e.to_string(),
            })?
        } else {
            parse_yaml(&text).map_err(|msg| ProjectError::Parse {
                path: p.display().to_string(),
                msg,
            })?
        };
        out.insert(name, v);
    }
    Ok(())
}

/// Médias du projet : vérifie l'existence des fichiers ; images en `<img>` simple
/// (les variantes responsives sont produites par `revel-assets`).
pub struct ProjectAssets {
    dir: PathBuf,
    passthrough: Vec<revel_model::Passthrough>,
}

impl ProjectAssets {
    pub fn new(p: &Project) -> Self {
        Self {
            dir: p.dir.clone(),
            passthrough: p.site.passthrough.clone(),
        }
    }

    fn exists(&self, url: &str) -> bool {
        let p = Project {
            dir: self.dir.clone(),
            site: {
                let mut s = Site::new("", "");
                s.passthrough = self.passthrough.clone();
                s
            },
        };
        p.source_of_url(url).is_some()
    }
}

impl AssetHooks for ProjectAssets {
    fn image(&self, a: &ImageArgs) -> Option<String> {
        if a.src.is_empty() || !self.exists(&a.src) {
            return None;
        }
        revel_render::PlainAssets.image(a)
    }

    fn media_exists(&self, src: &str) -> bool {
        self.exists(src)
    }

    fn share_image(&self, src: &str, base: &str) -> String {
        revel_render::PlainAssets.share_image(src, base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_et_corps() {
        let (d, b) = front_matter(
            "---\ntitle: « A »\ndate: '2026-01-02'\ntags:\n  - x\n---\n\nCorps *md*\n",
        )
        .unwrap();
        assert_eq!(d["title"], "« A »");
        assert_eq!(d["tags"][0], "x");
        assert_eq!(b, "Corps *md*\n");
        let (d, b) = front_matter("Sans en-tête").unwrap();
        assert!(d.as_object().unwrap().is_empty());
        assert_eq!(b, "Sans en-tête");
    }

    #[test]
    fn base36() {
        assert_eq!(to_base36(35), "z");
        assert_eq!(to_base36(36), "10");
    }

    #[test]
    fn version_selon_le_contenu() {
        let dir = std::env::temp_dir().join(format!("revel-version-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("assets/css")).unwrap();
        std::fs::write(dir.join("assets/css/site.css"), "a{}").unwrap();
        std::fs::write(dir.join("assets/photo.jpg"), "x").unwrap();
        let mut site = Site::new("Essai", "fr");
        site.passthrough.push(revel_model::Passthrough {
            from: "assets".into(),
            to: "assets".into(),
        });
        let p = Project::create(&dir, site.clone()).unwrap();
        let v1 = p.version_tag(&site);
        assert_eq!(v1, p.version_tag(&site), "deux générations identiques");
        assert_eq!(v1.len(), 8);
        std::fs::write(dir.join("assets/photo.jpg"), "y").unwrap();
        assert_eq!(
            v1,
            p.version_tag(&site),
            "une image ne change pas la version"
        );
        std::fs::write(dir.join("assets/css/site.css"), "b{}").unwrap();
        assert_ne!(
            v1,
            p.version_tag(&site),
            "une feuille de style modifiée change la version"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn decodage_url() {
        assert_eq!(percent_decode("/media/a%20b.jpg"), "/media/a b.jpg");
    }
}
