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

use revel_model::Site;
use revel_render::{AssetHooks, BuildInput, Entry, ImageArgs, OutputFile};
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
            version: version_tag(),
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
        for (i, p) in site.passthrough.iter().enumerate() {
            progress(Phase::Copies, i, site.passthrough.len());
            let from = self.dir.join(&p.from);
            let to = out_dir.join(p.to.trim_start_matches('/'));
            report.copied += copy_tree(&from, &to, out_dir, &mut report.files)?;
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

    /// Fichier source correspondant à une URL publique copiée telle quelle (`/media/a.jpg`).
    pub fn source_of_url(&self, url: &str) -> Option<PathBuf> {
        let url = url.split(['?', '#']).next().unwrap_or(url);
        let url = percent_decode(url);
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

fn version_tag() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    to_base36(t)
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
fn copy_tree(
    from: &Path,
    to: &Path,
    root: &Path,
    list: &mut Vec<String>,
) -> Result<usize, ProjectError> {
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
        n += copy_tree(&p, &to.join(&name), root, list)?;
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
    fn decodage_url() {
        assert_eq!(percent_decode("/media/a%20b.jpg"), "/media/a b.jpg");
    }
}
