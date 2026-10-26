//! Modèles partagés (D-132) : un site enregistré comme modèle, en fichier léger ou dans un
//! catalogue GitHub, puis repris à la création d'un site.
//!
//! Un modèle est une archive (fichier `.revel-template`, archive ZIP) ou un dossier du
//! catalogue de la même forme :
//! - `template.json` : description ([`PackMeta`]) et liste des fichiers ;
//! - `preview.<ext>` : vignette (facultative) ;
//! - `site/…` : fichiers du site, à leur place (`site/site.revel.json`, `site/content/…`,
//!   `site/media/…`).
//!
//! Fichier de modèle (léger) : textes, icônes (SVG) et polices compris ; images du site
//! remplacées par des images d'attente en ligne (placehold.co, aux dimensions de l'image) ;
//! vidéos, sons et documents retirés (adresses vidées). Catalogue : tous les fichiers du site.
//! Jamais de code : scripts, programmes et fichiers de construction (`package.json`) laissés
//! de côté ; à l'ouverture, code personnalisé, éléments `<script>`, attributs `on…` et liens
//! `javascript:` retirés du document.

use crate::{Core, CoreError, Result};
use revel_protocol::{TemplateInfo, TemplatePublished, TemplateSaved, TemplateShare};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Component, Path};

/// Extension des fichiers de modèle.
pub const EXT: &str = "revel-template";
/// Catalogue proposé par défaut (dépôt GitHub public).
pub const DEFAULT_CATALOG: &str = "matv-studio/revel-templates";
const FORMAT: u32 = 1;
/// Taille au-delà de laquelle GitHub refuse un fichier.
const GITHUB_MAX: u64 = 100 * 1024 * 1024;
/// Taille totale lue au plus dans un fichier de modèle (archive piégée).
const PACK_MAX: u64 = 2 * 1024 * 1024 * 1024;

/// Description d'un modèle (`template.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackMeta {
    pub format: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub lang: String,
    #[serde(default)]
    pub pages: usize,
    #[serde(default)]
    pub palette: Vec<String>,
    /// Vignette : chemin dans le modèle (`preview.jpg`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    /// Version de Revel qui a enregistré le modèle.
    #[serde(default)]
    pub revel: String,
    /// Date d'enregistrement (`AAAA-MM-JJ`).
    #[serde(default)]
    pub created: String,
    /// Tous les fichiers du site (catalogue) ; sinon fichier léger.
    #[serde(default)]
    pub full: bool,
    /// Images remplacées : chemin dans le site d'origine → image d'attente.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placeholders: Vec<Placeholder>,
    /// Vidéos, sons et documents retirés (chemins dans le site d'origine).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<String>,
    /// Fichiers du modèle (hors `template.json`) et leur taille.
    #[serde(default)]
    pub files: Vec<PackFile>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Placeholder {
    pub file: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackFile {
    pub path: String,
    pub size: u64,
}

/// Modèle en mémoire : description et fichiers (chemin dans le modèle → contenu).
pub struct Pack {
    pub meta: PackMeta,
    pub files: BTreeMap<String, Vec<u8>>,
}

/// Nature d'un fichier du site, selon son extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Text,
    Font,
    Image,
    /// Vidéos, sons, documents : gardés dans le catalogue, retirés du fichier léger.
    Heavy,
    /// Scripts, programmes, fichiers de construction : jamais repris.
    Code,
}

fn kind(rel: &str) -> Kind {
    let name = rel.rsplit('/').next().unwrap_or(rel).to_ascii_lowercase();
    if matches!(
        name.as_str(),
        "package.json"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | ".npmrc"
            | "netlify.toml"
            | "vercel.json"
    ) {
        return Kind::Code;
    }
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    match ext {
        "md" | "markdown" | "json" | "yml" | "yaml" | "txt" | "csv" | "svg" | "njk" | "html"
        | "htm" | "liquid" | "css" | "xml" | "webmanifest" | "ics" | "vtt" | "srt" => Kind::Text,
        "woff2" | "woff" | "ttf" | "otf" => Kind::Font,
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "avif" | "bmp" | "tif" | "tiff" | "ico" => {
            Kind::Image
        }
        "mp4" | "webm" | "mov" | "m4v" | "ogv" | "mp3" | "wav" | "ogg" | "oga" | "m4a" | "aac"
        | "flac" | "pdf" | "lottie" | "glb" | "gltf" => Kind::Heavy,
        _ => Kind::Code,
    }
}

/// Dossiers jamais repris : travail de Revel, historique, constructions, dépendances.
fn skipped_dir(name: &str) -> bool {
    name.starts_with('.') || matches!(name, "node_modules" | "_site" | "dist" | "vendor")
}

/// Fichiers du site à reprendre (chemins relatifs, `/`), hors `site.revel.json`.
fn site_files(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = rd.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().to_string();
            let p = e.path();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                if !skipped_dir(&name) {
                    walk(&p, root, out);
                }
            } else if ft.is_file()
                && (!name.starts_with('.') || name == ".pages.yml")
                && let Ok(rel) = p.strip_prefix(root)
            {
                let rel = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("/");
                if rel != revel_project::SITE_FILE {
                    out.push(rel);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out
}

/// Identifiant tiré d'un nom : minuscules, chiffres et tirets.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        let c = match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' | 'À' | 'Â' | 'Ä' | 'Á' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' | 'Î' | 'Ï' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' | 'Ô' | 'Ö' => 'o',
            'ù' | 'û' | 'ü' | 'ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let s = out.trim_end_matches('-').to_string();
    if s.is_empty() { "modele".into() } else { s }
}

/// Image d'attente en ligne aux proportions de l'image (2 400 px au plus de côté).
pub fn placeholder_url(dims: Option<(u32, u32)>) -> String {
    let (w, h) = dims
        .filter(|(w, h)| *w > 0 && *h > 0)
        .unwrap_or((1600, 1000));
    let scale = (2400.0 / f64::from(w.max(h))).min(1.0);
    let w = ((f64::from(w) * scale).round() as u32).clamp(10, 4000);
    let h = ((f64::from(h) * scale).round() as u32).clamp(10, 4000);
    format!("https://placehold.co/{w}x{h}.webp")
}

/// Adresse encodée d'un chemin (`%20` pour les espaces…).
fn encode_path(p: &str) -> String {
    let mut out = String::new();
    for b in p.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Remplace dans `text` chaque fichier retiré (adresse `/media/…`, chemin `media/…`, forme
/// encodée) par sa nouvelle valeur.
fn replace_refs(text: &str, map: &[(String, String)]) -> String {
    let mut out = text.to_string();
    for (rel, to) in map {
        let enc = encode_path(rel);
        for from in [format!("/{rel}"), format!("/{enc}"), rel.clone(), enc] {
            if out.contains(&from) {
                out = out.replace(&from, to);
            }
        }
    }
    out
}

/// Premières couleurs de la feuille de style (variables de `:root`), pour la vignette.
fn palette(site: &revel_model::Site) -> Vec<String> {
    let Some(css) = site.stylesheet.as_ref().map(revel_style::render) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    for decl in css.split([';', '{', '}']) {
        let Some((name, value)) = decl.split_once(':') else {
            continue;
        };
        if !name.trim().starts_with("--") {
            continue;
        }
        let v = value.trim();
        let is_color = (v.starts_with('#') && matches!(v.len(), 4 | 7 | 9))
            || v.starts_with("rgb(")
            || v.starts_with("hsl(")
            || v.starts_with("oklch(");
        if is_color && !out.iter().any(|c| c.eq_ignore_ascii_case(v)) {
            out.push(v.to_string());
        }
        if out.len() == 4 {
            break;
        }
    }
    out
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Retire du document ce qui n'est pas des données : code personnalisé, `<script>`, attributs
/// `on…`, liens `javascript:`. Nombre d'éléments retirés.
fn sanitize(doc: &mut Value) -> usize {
    fn walk(v: &mut Value, n: &mut usize) {
        match v {
            Value::Array(items) => {
                let before = items.len();
                items.retain(|it| {
                    let node = it.get("node").and_then(Value::as_str);
                    let script = node == Some("element")
                        && it
                            .get("tag")
                            .and_then(Value::as_str)
                            .is_some_and(|t| t.eq_ignore_ascii_case("script"));
                    let raw = node == Some("html")
                        && it
                            .get("html")
                            .and_then(Value::as_str)
                            .is_some_and(|h| h.to_ascii_lowercase().contains("<script"));
                    !(script || raw)
                });
                *n += before - items.len();
                for it in items.iter_mut() {
                    walk(it, n);
                }
            }
            Value::Object(m) => {
                if m.get("node").and_then(Value::as_str) == Some("element")
                    && let Some(Value::Array(attrs)) = m.get_mut("attrs")
                {
                    let before = attrs.len();
                    attrs.retain(|a| {
                        let name = a.get("name").and_then(Value::as_str).unwrap_or("");
                        let value = a.get("value").and_then(Value::as_str).unwrap_or("");
                        !name.to_ascii_lowercase().starts_with("on")
                            && !value.trim().to_ascii_lowercase().starts_with("javascript:")
                    });
                    *n += before - attrs.len();
                }
                for (_, x) in m.iter_mut() {
                    walk(x, n);
                }
            }
            _ => {}
        }
    }
    let mut n = 0;
    if let Some(code) = doc.pointer_mut("/settings/code")
        && code.as_object().is_some_and(|o| {
            o.values()
                .any(|v| v.as_str().is_some_and(|s| !s.trim().is_empty()))
        })
    {
        *code = json!({});
        n += 1;
    }
    walk(doc, &mut n);
    // Copies telles quelles : seulement des chemins du site.
    if let Some(Value::Array(list)) = doc.get_mut("passthrough") {
        list.retain(|p| p.get("from").and_then(Value::as_str).is_some_and(safe_rel));
    }
    n
}

/// Chemin relatif sans remontée ni racine.
fn safe_rel(p: &str) -> bool {
    !p.is_empty()
        && !p.contains('\\')
        && !p.contains(':')
        && Path::new(p)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

impl Pack {
    /// Taille totale des fichiers.
    pub fn size(&self) -> u64 {
        self.files.values().map(|b| b.len() as u64).sum()
    }

    /// Description pour la galerie.
    pub fn info(&self, id: String, preview: Option<String>) -> TemplateInfo {
        let m = &self.meta;
        TemplateInfo {
            id,
            name: m.name.clone(),
            category: if m.category.is_empty() {
                "personal".into()
            } else {
                m.category.clone()
            },
            description: m.description.clone(),
            preview,
            palette: m.palette.clone(),
            features: Vec::new(),
            pages: m.pages,
            author: m.author.clone(),
            license: m.license.clone(),
            lang: m.lang.clone(),
            size: self.size(),
            placeholders: m.placeholders.len(),
        }
    }

    /// Archive ZIP (`template.json` d'abord).
    pub fn to_zip(&self) -> Result<Vec<u8>> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            let stored = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            let meta = serde_json::to_vec_pretty(&self.meta)
                .map_err(|e| CoreError::Other(e.to_string()))?;
            z.start_file("template.json", opts).map_err(zip_err)?;
            z.write_all(&meta)?;
            for (p, b) in &self.files {
                // Formats déjà compressés : rangés tels quels.
                let packed = matches!(kind(p), Kind::Font | Kind::Image | Kind::Heavy);
                z.start_file(p.as_str(), if packed { stored } else { opts })
                    .map_err(zip_err)?;
                z.write_all(b)?;
            }
            z.finish().map_err(zip_err)?;
        }
        Ok(buf.into_inner())
    }

    /// Lit une archive ZIP de modèle (chemins vérifiés, taille bornée).
    pub fn from_zip(bytes: &[u8]) -> Result<Pack> {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes))
            .map_err(|_| template_err("error-template-file", ""))?;
        let mut meta: Option<PackMeta> = None;
        let mut files = BTreeMap::new();
        let mut total = 0u64;
        for i in 0..z.len() {
            let mut f = z.by_index(i).map_err(zip_err)?;
            if f.is_dir() {
                continue;
            }
            let name = f.name().to_string();
            total += f.size();
            if total > PACK_MAX {
                return Err(template_err("error-template-file", "trop lourd"));
            }
            let mut b = Vec::with_capacity(f.size().min(64 << 20) as usize);
            f.by_ref().take(PACK_MAX).read_to_end(&mut b)?;
            if name == "template.json" {
                meta = Some(
                    serde_json::from_slice(&b)
                        .map_err(|e| template_err("error-template-file", &e.to_string()))?,
                );
            } else if safe_rel(&name) {
                files.insert(name, b);
            }
        }
        let meta = meta.ok_or_else(|| template_err("error-template-file", "template.json"))?;
        if meta.format > FORMAT {
            return Err(template_err("error-template-version", &meta.revel));
        }
        Ok(Pack { meta, files })
    }
}

fn zip_err(e: zip::result::ZipError) -> CoreError {
    template_err("error-template-file", &e.to_string())
}

fn template_err(key: &'static str, detail: &str) -> CoreError {
    CoreError::Template(key, detail.to_string())
}

/// Adresse brute d'un fichier d'un dépôt GitHub public (branche par défaut).
fn raw_url(repo: &str, path: &str) -> String {
    let base = RAW_OVERRIDE
        .with(|o| o.borrow().clone())
        .unwrap_or_else(|| "https://raw.githubusercontent.com".into());
    format!("{base}/{repo}/HEAD/{}", encode_path(path))
}

thread_local! {
    /// Adresse des fichiers bruts remplacée (essais : serveur local).
    static RAW_OVERRIDE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Remplace l'adresse des fichiers bruts de GitHub pour le fil courant (essais).
#[doc(hidden)]
pub fn override_raw(base: Option<String>) {
    RAW_OVERRIDE.with(|o| *o.borrow_mut() = base);
}

/// `propriétaire/nom` valide.
fn check_repo(repo: &str) -> Result<String> {
    let r = revel_git::github::repo_of_url(repo)
        .or_else(|| {
            let t = repo.trim().trim_matches('/');
            let (o, n) = t.split_once('/')?;
            let ok = |s: &str| {
                !s.is_empty()
                    && !s.starts_with('.')
                    && s.chars()
                        .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
            };
            (ok(o) && ok(n) && !n.contains('/')).then(|| t.to_string())
        })
        .ok_or_else(|| template_err("error-template-catalog", repo))?;
    Ok(r)
}

/// Index d'un catalogue (`catalog.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Catalog {
    #[serde(default = "one")]
    pub format: u32,
    #[serde(default)]
    pub templates: Vec<CatalogEntry>,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub lang: String,
    #[serde(default)]
    pub pages: usize,
    #[serde(default)]
    pub palette: Vec<String>,
    /// Vignette : chemin dans le dépôt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    /// Dossier du modèle dans le dépôt (`templates/<id>`).
    pub path: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub updated: String,
}

impl Core {
    /// Modèle tiré d'un site : `full` : tous les fichiers (catalogue) ; sinon fichier léger
    /// (images en attente en ligne, vidéos, sons et documents retirés). Fichiers de code
    /// laissés de côté (nombre rendu).
    pub fn template_pack(
        &self,
        site_path: &Path,
        share: &TemplateShare,
        full: bool,
    ) -> Result<(Pack, usize)> {
        // Document en cours d'édition s'il est ouvert, sinon celui du disque.
        let site = match self.with_project(site_path, |p| Ok(p.site.clone())) {
            Ok(s) => s,
            Err(_) => revel_project::Project::open(site_path)?.site,
        };
        let mut site = site;
        site.base_url = None;
        site.settings.forms.recipient.clear();
        site.settings.code = Default::default();
        let name = if share.name.trim().is_empty() {
            site.name.clone()
        } else {
            share.name.trim().to_string()
        };
        let mut meta = PackMeta {
            format: FORMAT,
            id: if share.id.trim().is_empty() {
                slug(&name)
            } else {
                slug(&share.id)
            },
            name,
            category: share.category.clone(),
            description: share.description.trim().to_string(),
            author: share.author.trim().to_string(),
            license: share.license.trim().to_string(),
            lang: site.default_locale.clone(),
            pages: site.pages.len(),
            palette: palette(&site),
            preview: None,
            revel: env!("CARGO_PKG_VERSION").to_string(),
            created: today(),
            full,
            ..Default::default()
        };
        let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        let mut replaced: Vec<(String, String)> = Vec::new();
        let mut texts: Vec<(String, String)> = Vec::new();
        let mut skipped = 0;
        for rel in site_files(site_path) {
            let abs = site_path.join(&rel);
            match kind(&rel) {
                Kind::Code => skipped += 1,
                Kind::Text => match std::fs::read_to_string(&abs) {
                    Ok(t) => texts.push((rel, t)),
                    Err(_) => skipped += 1,
                },
                Kind::Image | Kind::Heavy if !full => {
                    if kind(&rel) == Kind::Image {
                        let url = placeholder_url(revel_assets::dimensions(&abs));
                        meta.placeholders.push(Placeholder {
                            file: rel.clone(),
                            url: url.clone(),
                        });
                        replaced.push((rel, url));
                    } else {
                        meta.missing.push(rel.clone());
                        replaced.push((rel, String::new()));
                    }
                }
                _ => {
                    files.insert(format!("site/{rel}"), std::fs::read(&abs)?);
                }
            }
        }
        // Remplacements : chemins les plus longs d'abord.
        replaced.sort_by_key(|(r, _)| std::cmp::Reverse(r.len()));
        for (rel, t) in texts {
            let t = if replaced.is_empty() {
                t
            } else {
                replace_refs(&t, &replaced)
            };
            files.insert(format!("site/{rel}"), t.into_bytes());
        }
        let doc =
            serde_json::to_string_pretty(&site).map_err(|e| CoreError::Other(e.to_string()))?;
        let doc = if replaced.is_empty() {
            doc
        } else {
            replace_refs(&doc, &replaced)
        };
        files.insert(
            format!("site/{}", revel_project::SITE_FILE),
            doc.into_bytes(),
        );
        // Vignette : image du site choisie, gardée telle quelle (2 Mo au plus).
        // Image du site (`media/…`) ou fichier choisi sur l'ordinateur (chemin complet).
        let preview_file = share.preview.as_deref().and_then(|p| {
            let abs = Path::new(p);
            if abs.is_absolute() && abs.is_file() {
                Some(abs.to_path_buf())
            } else {
                let rel = p.trim_start_matches('/');
                safe_rel(rel).then(|| site_path.join(rel))
            }
        });
        if let Some(file) = preview_file
            && let Some(p) = file.to_str()
            && kind(p) == Kind::Image
            && let Ok(b) = std::fs::read(&file)
            && b.len() <= 2 * 1024 * 1024
        {
            let ext = p
                .rsplit_once('.')
                .map(|(_, e)| e.to_ascii_lowercase())
                .unwrap_or_default();
            let name = format!("preview.{ext}");
            files.insert(name.clone(), b);
            meta.preview = Some(name);
        }
        meta.files = files
            .iter()
            .map(|(p, b)| PackFile {
                path: p.clone(),
                size: b.len() as u64,
            })
            .collect();
        Ok((Pack { meta, files }, skipped))
    }

    /// Enregistre le site `site_path` comme fichier de modèle léger à `dest`.
    pub fn template_save(
        &self,
        site_path: &Path,
        share: &TemplateShare,
        dest: &Path,
    ) -> Result<TemplateSaved> {
        let (pack, skipped) = self.template_pack(site_path, share, false)?;
        let bytes = pack.to_zip()?;
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(dest, &bytes)?;
        Ok(TemplateSaved {
            path: dest.display().to_string(),
            size: bytes.len() as u64,
            placeholders: pack.meta.placeholders.len(),
            missing: pack.meta.missing.len(),
            skipped,
        })
    }

    /// Description d'un fichier de modèle (galerie, avant la création).
    pub fn template_file_info(&self, file: &Path) -> Result<TemplateInfo> {
        let pack = Pack::from_zip(&std::fs::read(file)?)?;
        let preview = pack.meta.preview.as_ref().and_then(|p| {
            let b = pack.files.get(p)?;
            let ext = p.rsplit_once('.').map(|(_, e)| e).unwrap_or("png");
            let mime = match ext {
                "jpg" | "jpeg" => "image/jpeg",
                "svg" => "image/svg+xml",
                e => return Some(format!("data:image/{e};base64,{}", b64(b))),
            };
            Some(format!("data:{mime};base64,{}", b64(b)))
        });
        Ok(pack.info(format!("file:{}", file.display()), preview))
    }

    /// Crée un site d'après un modèle (fichier ou catalogue) dans le dossier `path`, nommé `name`.
    /// Nombre d'éléments de code retirés.
    pub fn install_pack(
        &self,
        path: &Path,
        name: &str,
        pack: Pack,
    ) -> Result<(crate::SiteSummary, usize)> {
        if path.join(revel_project::SITE_FILE).exists() {
            return Err(CoreError::AlreadyImported);
        }
        let doc_key = format!("site/{}", revel_project::SITE_FILE);
        let doc = pack
            .files
            .get(&doc_key)
            .ok_or_else(|| template_err("error-template-file", revel_project::SITE_FILE))?;
        let mut doc: Value = serde_json::from_slice(doc)
            .map_err(|e| template_err("error-template-file", &e.to_string()))?;
        let mut removed = sanitize(&mut doc);
        let mut site: revel_model::Site = serde_json::from_value(doc)
            .map_err(|e| template_err("error-template-file", &e.to_string()))?;
        let name = name.trim();
        if !name.is_empty() {
            site.name = name.to_string();
            if let Some(Value::Object(info)) = site.data.get_mut("site")
                && info.contains_key("title")
            {
                info.insert("title".into(), json!(name));
            }
        }
        std::fs::create_dir_all(path)?;
        for (p, b) in &pack.files {
            let Some(rel) = p.strip_prefix("site/") else {
                continue;
            };
            if rel == revel_project::SITE_FILE || !safe_rel(rel) {
                continue;
            }
            if kind(rel) == Kind::Code {
                removed += 1;
                continue;
            }
            let f = path.join(rel);
            if let Some(dir) = f.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&f, b)?;
        }
        let p = revel_project::Project::create(path, site)?;
        let _ = std::fs::create_dir_all(path.join("media"));
        let summary = crate::summarize(&p)?;
        self.touch_recent(&summary);
        Ok((summary, removed))
    }

    /// Modèles d'un catalogue GitHub (`propriétaire/nom`), lus dans son `catalog.json`.
    pub fn template_catalog(&self, repo: &str) -> Result<Vec<TemplateInfo>> {
        let repo = check_repo(repo)?;
        let cat = read_catalog(&repo)?;
        Ok(cat
            .templates
            .into_iter()
            .map(|e| TemplateInfo {
                id: format!("catalog:{repo}#{}", e.id),
                name: e.name,
                category: if e.category.is_empty() {
                    "personal".into()
                } else {
                    e.category
                },
                description: e.description,
                preview: e.preview.map(|p| raw_url(&repo, &p)),
                palette: e.palette,
                features: Vec::new(),
                pages: e.pages,
                author: e.author,
                license: e.license,
                lang: e.lang,
                size: e.size,
                placeholders: 0,
            })
            .collect())
    }

    /// Modèle d'un catalogue téléchargé (tous ses fichiers).
    pub fn catalog_pack(&self, repo: &str, id: &str) -> Result<Pack> {
        let repo = check_repo(repo)?;
        let cat = read_catalog(&repo)?;
        let e = cat
            .templates
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| template_err("error-template-catalog-missing", id))?;
        let dir = e.path.trim_matches('/').to_string();
        if !safe_rel(&dir) {
            return Err(template_err("error-template-catalog", &dir));
        }
        let agent = crate::fonts::agent(120);
        let meta_bytes = get_bytes(&agent, &raw_url(&repo, &format!("{dir}/template.json")))?;
        let meta: PackMeta = serde_json::from_slice(&meta_bytes)
            .map_err(|e| template_err("error-template-file", &e.to_string()))?;
        let mut files = BTreeMap::new();
        let mut total = 0;
        for f in &meta.files {
            if !safe_rel(&f.path) {
                continue;
            }
            total += f.size;
            if total > PACK_MAX {
                return Err(template_err("error-template-file", "trop lourd"));
            }
            let b = get_bytes(&agent, &raw_url(&repo, &format!("{dir}/{}", f.path)))?;
            files.insert(f.path.clone(), b);
        }
        Ok(Pack { meta, files })
    }

    /// Crée un site d'après un modèle partagé : `file:<chemin>` ou `catalog:<dépôt>#<id>`.
    pub fn create_site_shared(
        &self,
        parent: &Path,
        name: &str,
        template: &str,
    ) -> Result<(crate::SiteSummary, usize)> {
        let pack = if let Some(file) = template.strip_prefix("file:") {
            Pack::from_zip(&std::fs::read(file)?)?
        } else if let Some(rest) = template.strip_prefix("catalog:") {
            let (repo, id) = rest
                .split_once('#')
                .ok_or_else(|| template_err("error-template-catalog", rest))?;
            self.catalog_pack(repo, id)?
        } else {
            return Err(template_err("error-template-file", template));
        };
        self.install_pack(&crate::site_folder(parent, name), name, pack)
    }

    /// Envoie le site `site_path` comme modèle complet dans le catalogue `repo` : ajouté
    /// directement si le compte GitHub connecté peut écrire dans le dépôt, sinon par une copie
    /// du dépôt et une demande de fusion.
    pub fn template_publish(
        &self,
        site_path: &Path,
        repo: &str,
        share: &TemplateShare,
    ) -> Result<TemplatePublished> {
        let repo = check_repo(repo)?;
        let token = self.github_token()?;
        let gh = revel_git::github::Client::new(&token.access_token);
        let user = gh.user()?;
        let info = gh
            .repo(&repo)?
            .ok_or_else(|| CoreError::GithubRepoNotFound(repo.clone()))?;
        let (mut pack, _) = self.template_pack(site_path, share, true)?;
        let mut too_large = Vec::new();
        pack.files.retain(|p, b| {
            let ok = (b.len() as u64) < GITHUB_MAX;
            if !ok {
                too_large.push(p.clone());
            }
            ok
        });
        pack.meta.files.retain(|f| !too_large.contains(&f.path));
        pack.meta.missing.extend(too_large.iter().cloned());
        let id = pack.meta.id.clone();
        let dir = format!("templates/{id}");
        let base_repo = gh.call_ok("GET", &format!("/repos/{repo}"), None)?;
        let base_branch = base_repo
            .get("default_branch")
            .and_then(Value::as_str)
            .unwrap_or("main")
            .to_string();
        let can_push = info.permissions.as_ref().is_some_and(|p| p.push);
        // Dépôt où écrire : le catalogue, sinon une copie au nom du compte.
        let target = if can_push {
            repo.clone()
        } else {
            gh.call_ok("POST", &format!("/repos/{repo}/forks"), Some(&json!({})))?;
            let fork = format!("{}/{}", user.login, repo.split('/').nth(1).unwrap_or(""));
            // La copie est prête quand sa branche répond (quelques secondes).
            let mut ready = false;
            for _ in 0..30 {
                let (s, _) = gh.call(
                    "GET",
                    &format!("/repos/{fork}/git/ref/heads/{base_branch}"),
                    None,
                )?;
                if s == 200 {
                    ready = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            if !ready {
                return Err(template_err("error-template-publish", "fork"));
            }
            fork
        };
        // Index du catalogue (lu dans la branche de base du catalogue d'origine).
        // Index lu par l'API (les fichiers bruts restent en cache quelques minutes).
        let mut cat = read_catalog_api(&gh, &repo)?;
        let entry = CatalogEntry {
            id: id.clone(),
            name: pack.meta.name.clone(),
            category: pack.meta.category.clone(),
            description: pack.meta.description.clone(),
            author: if pack.meta.author.is_empty() {
                user.login.clone()
            } else {
                pack.meta.author.clone()
            },
            license: pack.meta.license.clone(),
            lang: pack.meta.lang.clone(),
            pages: pack.meta.pages,
            palette: pack.meta.palette.clone(),
            preview: pack.meta.preview.as_ref().map(|p| format!("{dir}/{p}")),
            path: dir.clone(),
            size: pack.size(),
            updated: today(),
        };
        cat.format = FORMAT;
        cat.templates.retain(|e| e.id != id);
        cat.templates.push(entry);
        cat.templates
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let meta_bytes =
            serde_json::to_vec_pretty(&pack.meta).map_err(|e| CoreError::Other(e.to_string()))?;
        let cat_bytes =
            serde_json::to_vec_pretty(&cat).map_err(|e| CoreError::Other(e.to_string()))?;
        let mut tree_files: Vec<(String, &[u8])> = vec![
            (format!("{dir}/template.json"), &meta_bytes),
            ("catalog.json".into(), &cat_bytes),
        ];
        for (p, b) in &pack.files {
            tree_files.push((format!("{dir}/{p}"), b));
        }
        // Objets Git : contenus, arbre (sur celui de la branche), version, branche.
        let head = gh.call_ok(
            "GET",
            &format!("/repos/{target}/git/ref/heads/{base_branch}"),
            None,
        )?;
        let head_sha = str_at(&head, "/object/sha")?;
        let commit = gh.call_ok(
            "GET",
            &format!("/repos/{target}/git/commits/{head_sha}"),
            None,
        )?;
        let base_tree = str_at(&commit, "/tree/sha")?;
        let mut entries = Vec::new();
        for (p, b) in &tree_files {
            let blob = gh.call_ok(
                "POST",
                &format!("/repos/{target}/git/blobs"),
                Some(&json!({ "content": b64(b), "encoding": "base64" })),
            )?;
            entries.push(json!({ "path": p, "mode": "100644", "type": "blob", "sha": str_at(&blob, "/sha")? }));
        }
        let tree = gh.call_ok(
            "POST",
            &format!("/repos/{target}/git/trees"),
            Some(&json!({ "base_tree": base_tree, "tree": entries })),
        )?;
        let message = format!("Modèle « {} » ({})", pack.meta.name, id);
        let new_commit = gh.call_ok(
            "POST",
            &format!("/repos/{target}/git/commits"),
            Some(&json!({ "message": message, "tree": str_at(&tree, "/sha")?, "parents": [head_sha] })),
        )?;
        let new_sha = str_at(&new_commit, "/sha")?;
        let files = tree_files.len();
        let size = pack.size();
        if can_push {
            gh.call_ok(
                "PATCH",
                &format!("/repos/{target}/git/refs/heads/{base_branch}"),
                Some(&json!({ "sha": new_sha })),
            )?;
            Ok(TemplatePublished {
                url: format!("https://github.com/{repo}/tree/{base_branch}/{dir}"),
                pull_request: false,
                files,
                size,
                too_large,
            })
        } else {
            let branch = format!(
                "modele-{id}-{}",
                chrono::Local::now().format("%Y%m%d%H%M%S")
            );
            gh.call_ok(
                "POST",
                &format!("/repos/{target}/git/refs"),
                Some(&json!({ "ref": format!("refs/heads/{branch}"), "sha": new_sha })),
            )?;
            let pr = gh.call_ok(
                "POST",
                &format!("/repos/{repo}/pulls"),
                Some(&json!({
                    "title": message,
                    "head": format!("{}:{branch}", user.login),
                    "base": base_branch,
                    "body": format!("{}\n\nAuteur : {} · Licence : {} · {} page(s)",
                        pack.meta.description, cat_author(&pack.meta, &user.login),
                        if pack.meta.license.is_empty() { "—" } else { &pack.meta.license },
                        pack.meta.pages),
                })),
            )?;
            Ok(TemplatePublished {
                url: pr
                    .get("html_url")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                pull_request: true,
                files,
                size,
                too_large,
            })
        }
    }

    /// Crée un catalogue de modèles sur le compte GitHub connecté : dépôt public `name` avec
    /// un `catalog.json` vide et une présentation. Rend `propriétaire/nom`.
    pub fn template_catalog_create(&self, name: &str) -> Result<String> {
        let token = self.github_token()?;
        let gh = revel_git::github::Client::new(&token.access_token);
        let user = gh.user()?;
        let name = slug(if name.trim().is_empty() {
            "revel-templates"
        } else {
            name
        });
        let full = format!("{}/{name}", user.login);
        if gh.repo(&full)?.is_none() {
            gh.call_ok(
                "POST",
                "/user/repos",
                Some(&json!({
                    "name": name,
                    "description": "Revel templates catalog",
                    "private": false,
                    "auto_init": true,
                    "has_wiki": false,
                })),
            )?;
        }
        // `catalog.json` et présentation, s'ils manquent (dépôt initialisé par GitHub).
        let readme = README.to_string();
        for (path, content) in [
            (
                "catalog.json",
                serde_json::to_string_pretty(&Catalog {
                    format: FORMAT,
                    templates: Vec::new(),
                })
                .unwrap_or_default(),
            ),
            ("README.md", readme),
        ] {
            let mut tries = 0;
            loop {
                let (s, v) = gh.call("GET", &format!("/repos/{full}/contents/{path}"), None)?;
                let sha = v.get("sha").and_then(Value::as_str).map(str::to_string);
                if s == 200 && path == "catalog.json" {
                    break;
                }
                let mut body = json!({
                    "message": format!("Catalogue de modèles : {path}"),
                    "content": b64(content.as_bytes()),
                });
                if let Some(sha) = sha {
                    body["sha"] = json!(sha);
                }
                let (s2, _) = gh.call(
                    "PUT",
                    &format!("/repos/{full}/contents/{path}"),
                    Some(&body),
                )?;
                // Dépôt tout juste créé : sa branche peut ne pas être encore prête.
                if (200..300).contains(&s2) || tries >= 10 {
                    break;
                }
                tries += 1;
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        Ok(full)
    }
}

fn cat_author(meta: &PackMeta, login: &str) -> String {
    if meta.author.is_empty() {
        login.to_string()
    } else {
        meta.author.clone()
    }
}

const README: &str = "# Revel templates\n\nCatalogue de modèles de site pour [Revel](https://github.com/matv-studio/Revel).\n\
Chaque modèle est un dossier `templates/<id>/` (`template.json`, `site/…`) listé dans `catalog.json`.\n\
Les modèles s'ajoutent depuis Revel (site › Enregistrer comme modèle › Catalogue) ; une proposition \
d'un autre compte arrive en demande de fusion, à relire avant de l'accepter.\n\n\
Site templates catalog for Revel: one folder per template, listed in `catalog.json`; proposals \
arrive as pull requests from Revel.\n";

fn str_at(v: &Value, ptr: &str) -> Result<String> {
    v.pointer(ptr)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| template_err("error-template-publish", ptr))
}

fn b64(b: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(b)
}

fn get_bytes(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>> {
    let mut res = agent
        .get(url)
        .call()
        .map_err(|e| template_err("error-template-catalog", &e.to_string()))?;
    if res.status().as_u16() != 200 {
        return Err(template_err(
            "error-template-catalog",
            &format!("{} ({url})", res.status().as_u16()),
        ));
    }
    res.body_mut()
        .with_config()
        .limit(GITHUB_MAX)
        .read_to_vec()
        .map_err(|e| template_err("error-template-catalog", &e.to_string()))
}

/// Index d'un catalogue lu par l'API (à jour) ; vide s'il n'existe pas encore.
fn read_catalog_api(gh: &revel_git::github::Client, repo: &str) -> Result<Catalog> {
    use base64::Engine;
    let (s, v) = gh.call("GET", &format!("/repos/{repo}/contents/catalog.json"), None)?;
    if s == 404 {
        return Ok(Catalog::default());
    }
    if s != 200 {
        return Err(template_err("error-template-catalog", &s.to_string()));
    }
    let b64text: String = v
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64text)
        .map_err(|e| template_err("error-template-catalog", &e.to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| template_err("error-template-catalog", &e.to_string()))
}

fn read_catalog(repo: &str) -> Result<Catalog> {
    let agent = crate::fonts::agent(30);
    let b = get_bytes(&agent, &raw_url(repo, "catalog.json"))?;
    serde_json::from_slice(&b).map_err(|e| template_err("error-template-catalog", &e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natures_des_fichiers() {
        assert_eq!(kind("content/a.md"), Kind::Text);
        assert_eq!(kind("media/icons/x.svg"), Kind::Text);
        assert_eq!(kind("media/fonts/a/b.woff2"), Kind::Font);
        assert_eq!(kind("media/photos/a.JPG"), Kind::Image);
        assert_eq!(kind("media/v.mp4"), Kind::Heavy);
        assert_eq!(kind("eleventy.config.js"), Kind::Code);
        assert_eq!(kind("package.json"), Kind::Code);
        assert_eq!(kind("form.php"), Kind::Code);
    }

    #[test]
    fn chemins_et_adresses() {
        assert!(safe_rel("site/content/a.md"));
        assert!(!safe_rel("../x"));
        assert!(!safe_rel("/etc/passwd"));
        assert!(!safe_rel("C:\\x"));
        assert_eq!(slug("Atelier Lumière !"), "atelier-lumiere");
        assert_eq!(
            placeholder_url(Some((4000, 2000))),
            "https://placehold.co/2400x1200.webp"
        );
        assert_eq!(placeholder_url(None), "https://placehold.co/1600x1000.webp");
        let t = replace_refs(
            "src=\"/media/a b.jpg\" image: media/a%20b.jpg",
            &[("media/a b.jpg".into(), "https://x/y.webp".into())],
        );
        assert_eq!(t, "src=\"https://x/y.webp\" image: https://x/y.webp");
        assert_eq!(
            check_repo("matv-studio/revel-templates").unwrap(),
            "matv-studio/revel-templates"
        );
        assert!(check_repo("../x").is_err());
    }

    #[test]
    fn document_nettoye() {
        let mut doc = json!({
            "settings": { "code": { "head": "<script>x</script>", "body": "" } },
            "passthrough": [{ "from": "media", "to": "media" }, { "from": "../etc", "to": "x" }],
            "pages": [{ "body": [
                { "node": "element", "tag": "script", "children": [] },
                { "node": "html", "html": "<SCRIPT>alert(1)</SCRIPT>" },
                { "node": "element", "tag": "a", "attrs": [
                    { "name": "href", "value": "javascript:alert(1)" },
                    { "name": "onclick", "value": "x()" },
                    { "name": "class", "value": "btn" }
                ] }
            ] }]
        });
        let n = sanitize(&mut doc);
        assert_eq!(n, 5);
        assert_eq!(doc["pages"][0]["body"].as_array().unwrap().len(), 1);
        assert_eq!(
            doc["pages"][0]["body"][0]["attrs"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(doc["passthrough"].as_array().unwrap().len(), 1);
    }
}
