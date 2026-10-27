//! `revel-assets` : images responsives (AVIF, WebP, format d'origine) et image de partage.
//!
//! Pendant la génération, [`Pipeline`] écrit le HTML (`<picture>` + `srcset`) et note les
//! variantes nécessaires ; [`Pipeline::process`] les produit ensuite, en parallèle, avec un
//! cache dans le projet (`.revel/img`) : une variante déjà produite n'est jamais recalculée.
//!
//! Noms de fichiers : `nom-<empreinte>-<largeur>.<format>`, l'empreinte dépendant du contenu
//! du fichier source et des réglages (un fichier modifié change de nom : pas de cache périmé
//! chez les visiteurs).

use base64::Engine;
use rayon::prelude::*;
use revel_model::Passthrough;
use revel_render::{AssetHooks, ImageArgs, escape};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

mod encode;

/// Largeurs par défaut (reprend le réglage du site Alunites).
pub const DEFAULT_WIDTHS: &[u32] = &[480, 800, 1200, 1800, 2400];
/// Largeur des images de partage (réseaux sociaux).
pub const SHARE_WIDTH: u32 = 1200;
/// Version du traitement : la changer invalide le cache.
const PIPELINE_VERSION: &str = "revel-img-1";
const RASTER: &[&str] = &["jpg", "jpeg", "png", "webp", "gif"];

/// Format produit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Format {
    Avif,
    Webp,
    Jpeg,
    Png,
    Gif,
}

impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Self::Avif => "avif",
            Self::Webp => "webp",
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::Gif => "gif",
        }
    }

    fn mime(self) -> &'static str {
        match self {
            Self::Avif => "image/avif",
            Self::Webp => "image/webp",
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Gif => "image/gif",
        }
    }

    /// Format d'origine (`auto`).
    fn of_ext(ext: &str) -> Self {
        match ext {
            "png" => Self::Png,
            "webp" => Self::Webp,
            "gif" => Self::Gif,
            _ => Self::Jpeg,
        }
    }
}

/// Dimensions d'une image (largeur, hauteur), lues dans son en-tête ; `None` si le format n'est
/// pas reconnu.
pub fn dimensions(path: &Path) -> Option<(u32, u32)> {
    image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

/// Qualités d'encodage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Quality {
    pub avif: u8,
    pub webp: u8,
    pub jpeg: u8,
}

pub const DEFAULT_QUALITY: Quality = Quality {
    avif: 55,
    webp: 78,
    jpeg: 80,
};
pub const SHARE_QUALITY: Quality = Quality {
    avif: 55,
    webp: 78,
    jpeg: 82,
};

/// Variante à produire.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variant {
    pub source: PathBuf,
    /// Nom du fichier produit (dans le dossier `img/`).
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: Format,
    pub quality: Quality,
}

#[derive(Debug, Default, Clone)]
struct SourceInfo {
    size: u64,
    mtime: u128,
    hash: String,
    width: u32,
    height: u32,
}

/// Statistiques d'un traitement.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub produced: usize,
    pub from_cache: usize,
    pub failed: usize,
}

pub struct Pipeline {
    dir: PathBuf,
    passthrough: Vec<Passthrough>,
    cache: PathBuf,
    url_prefix: String,
    widths: Vec<u32>,
    sources: Mutex<HashMap<PathBuf, SourceInfo>>,
    variants: Mutex<BTreeSet<Variant>>,
    errors: Mutex<Vec<String>>,
}

impl Pipeline {
    /// `dir` : dossier du projet ; `widths` vide = [`DEFAULT_WIDTHS`].
    pub fn new(dir: &Path, passthrough: &[Passthrough], widths: &[u32]) -> Self {
        let cache = dir.join(".revel").join("img");
        let sources = load_index(&cache.join("index.json"));
        Self {
            dir: dir.to_path_buf(),
            passthrough: passthrough.to_vec(),
            cache,
            url_prefix: "/img/".into(),
            widths: if widths.is_empty() {
                DEFAULT_WIDTHS.to_vec()
            } else {
                widths.to_vec()
            },
            sources: Mutex::new(sources),
            variants: Mutex::new(BTreeSet::new()),
            errors: Mutex::new(Vec::new()),
        }
    }

    /// Fichier source d'une URL copiée telle quelle (`/media/a.jpg` -> `src/media/a.jpg`).
    pub fn source_of(&self, url: &str) -> Option<PathBuf> {
        let url = url.split(['?', '#']).next().unwrap_or(url);
        let url = percent_decode(url);
        if url.split(['/', '\\']).any(|s| s == "..") {
            return None;
        }
        for p in &self.passthrough {
            let to = p.to.trim_matches('/');
            let prefix = if to.is_empty() {
                "/".to_string()
            } else {
                format!("/{to}/")
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

    fn info(&self, path: &Path) -> Option<SourceInfo> {
        let meta = std::fs::metadata(path).ok()?;
        let size = meta.len();
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis())
            .unwrap_or(0);
        if let Some(i) = self.sources.lock().ok()?.get(path)
            && i.size == size
            && i.mtime == mtime
        {
            return Some(i.clone());
        }
        let bytes = std::fs::read(path).ok()?;
        let mut h = blake3::Hasher::new();
        h.update(PIPELINE_VERSION.as_bytes());
        h.update(&bytes);
        let hash = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(&h.finalize().as_bytes()[..6])[..6]
            .to_string();
        let (width, height) = encode::dimensions(path).ok()?;
        let info = SourceInfo {
            size,
            mtime,
            hash,
            width,
            height,
        };
        self.sources
            .lock()
            .ok()?
            .insert(path.to_path_buf(), info.clone());
        Some(info)
    }

    /// Variantes d'une source. Largeurs valides : celles qui ne dépassent pas l'original
    /// (pas d'agrandissement) ; si aucune ne convient, la largeur d'origine (comme eleventy-img).
    fn plan(
        &self,
        path: &Path,
        info: &SourceInfo,
        widths: &[u32],
        formats: &[Format],
        quality: Quality,
        salt: &str,
    ) -> Vec<Variant> {
        let mut valid: Vec<u32> = widths
            .iter()
            .copied()
            .filter(|w| *w <= info.width)
            .collect();
        if valid.is_empty() {
            valid.push(info.width);
        }
        valid.sort_unstable();
        valid.dedup();
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let hash = if salt.is_empty() {
            info.hash.clone()
        } else {
            let h = blake3::hash(format!("{}{salt}", info.hash).as_bytes());
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&h.as_bytes()[..6])[..6]
                .to_string()
        };
        let mut out = Vec::new();
        for &f in formats {
            for &w in &valid {
                let h = ((info.height as f64) * (w as f64) / (info.width as f64))
                    .round()
                    .max(1.0) as u32;
                out.push(Variant {
                    source: path.to_path_buf(),
                    name: format!("{stem}-{hash}-{w}.{}", f.ext()),
                    width: w,
                    height: h,
                    format: f,
                    quality,
                });
            }
        }
        out
    }

    fn register(&self, vs: &[Variant]) {
        if let Ok(mut set) = self.variants.lock() {
            set.extend(vs.iter().cloned());
        }
    }

    /// Fichier à servir pour une variante (canevas de l'éditeur) : la variante si elle a déjà été
    /// produite (cache), sinon l'image d'origine, que le navigateur met à l'échelle.
    pub fn variant_or_source(&self, name: &str) -> Option<PathBuf> {
        if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
            return None;
        }
        let cached = self.cache.join(name);
        if cached.is_file() {
            return Some(cached);
        }
        self.variants
            .lock()
            .ok()?
            .iter()
            .find(|v| v.name == name)
            .map(|v| v.source.clone())
    }

    /// Variantes en attente.
    pub fn pending(&self) -> Vec<Variant> {
        self.variants
            .lock()
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn take_errors(&self) -> Vec<String> {
        self.errors
            .lock()
            .map(|mut e| std::mem::take(&mut *e))
            .unwrap_or_default()
    }

    /// Produit toutes les variantes dans `out_dir/img/`. `progress(fait, total)`.
    pub fn process(&self, out_dir: &Path, progress: &(dyn Fn(usize, usize) + Sync)) -> Stats {
        let variants = self.pending();
        let total = variants.len();
        let img_dir = out_dir.join(self.url_prefix.trim_matches('/'));
        let _ = std::fs::create_dir_all(&img_dir);
        let _ = std::fs::create_dir_all(&self.cache);
        // Regroupement par source : un seul décodage par fichier.
        let mut by_source: BTreeMap<PathBuf, Vec<Variant>> = BTreeMap::new();
        for v in variants {
            by_source.entry(v.source.clone()).or_default().push(v);
        }
        let done = std::sync::atomic::AtomicUsize::new(0);
        let produced = std::sync::atomic::AtomicUsize::new(0);
        let cached = std::sync::atomic::AtomicUsize::new(0);
        let failed = std::sync::atomic::AtomicUsize::new(0);
        use std::sync::atomic::Ordering::Relaxed;
        by_source.par_iter().for_each(|(src, vs)| {
            let missing: Vec<&Variant> = vs
                .iter()
                .filter(|v| !self.cache.join(&v.name).is_file())
                .collect();
            let decoded = if missing.is_empty() {
                None
            } else {
                match encode::decode(src) {
                    Ok(img) => Some(img),
                    Err(e) => {
                        if let Ok(mut errs) = self.errors.lock() {
                            errs.push(format!("{} : {e}", src.display()));
                        }
                        None
                    }
                }
            };
            for v in vs {
                let cached_file = self.cache.join(&v.name);
                let ok = if cached_file.is_file() {
                    cached.fetch_add(1, Relaxed);
                    true
                } else if let Some(img) = &decoded {
                    match encode::variant(img, v) {
                        Ok(bytes) => {
                            let tmp = cached_file.with_extension("tmp");
                            let w = std::fs::write(&tmp, bytes)
                                .and_then(|_| std::fs::rename(&tmp, &cached_file));
                            if w.is_ok() {
                                produced.fetch_add(1, Relaxed);
                            }
                            w.is_ok()
                        }
                        Err(e) => {
                            if let Ok(mut errs) = self.errors.lock() {
                                errs.push(format!("{} ({}) : {e}", src.display(), v.name));
                            }
                            false
                        }
                    }
                } else {
                    false
                };
                if ok {
                    let _ = std::fs::copy(&cached_file, img_dir.join(&v.name));
                } else {
                    failed.fetch_add(1, Relaxed);
                }
                let n = done.fetch_add(1, Relaxed) + 1;
                progress(n, total);
            }
        });
        self.save_index();
        Stats {
            produced: produced.load(Relaxed),
            from_cache: cached.load(Relaxed),
            failed: failed.load(Relaxed),
        }
    }

    fn save_index(&self) {
        let Ok(sources) = self.sources.lock() else {
            return;
        };
        let map: BTreeMap<String, serde_json::Value> = sources
            .iter()
            .map(|(p, i)| {
                (
                    p.to_string_lossy().into_owned(),
                    serde_json::json!([i.size, i.mtime.to_string(), i.hash, i.width, i.height]),
                )
            })
            .collect();
        let _ = std::fs::create_dir_all(&self.cache);
        if let Ok(text) = serde_json::to_string(&map) {
            let _ = std::fs::write(self.cache.join("index.json"), text);
        }
    }

    fn is_raster(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| RASTER.contains(&e.to_ascii_lowercase().as_str()))
    }
}

/// GIF animé : servi tel quel (ses variantes AVIF, WebP ou GIF redimensionné n'en garderaient que
/// la première image).
fn animated_gif(path: &Path) -> bool {
    let is_gif = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("gif"));
    if !is_gif {
        return false;
    }
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    gif_frames(&bytes) > 1
}

/// Nombre d'images d'un GIF (blocs de contrôle graphique, sinon descripteurs d'image), compté
/// en suivant la structure du fichier.
fn gif_frames(b: &[u8]) -> usize {
    if b.len() < 13 || !b.starts_with(b"GIF8") {
        return 0;
    }
    let mut i = 13;
    // Palette globale.
    if b[10] & 0x80 != 0 {
        i += 3 * (1 << ((b[10] & 0x07) + 1));
    }
    let skip_blocks = |mut i: usize| -> usize {
        while i < b.len() {
            let n = b[i] as usize;
            i += 1;
            if n == 0 {
                break;
            }
            i += n;
        }
        i
    };
    let mut frames = 0;
    while i < b.len() {
        match b[i] {
            0x2C => {
                frames += 1;
                if frames > 1 {
                    return frames;
                }
                if i + 10 > b.len() {
                    break;
                }
                let flags = b[i + 9];
                i += 10;
                if flags & 0x80 != 0 {
                    i += 3 * (1 << ((flags & 0x07) + 1));
                }
                // Taille minimale du code LZW, puis données.
                i = skip_blocks(i + 1);
            }
            0x21 => i = skip_blocks(i + 2),
            _ => break,
        }
    }
    frames
}

fn load_index(path: &Path) -> HashMap<PathBuf, SourceInfo> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    let Ok(map) = serde_json::from_str::<BTreeMap<String, serde_json::Value>>(&text) else {
        return HashMap::new();
    };
    map.into_iter()
        .filter_map(|(k, v)| {
            let a = v.as_array()?;
            Some((
                PathBuf::from(k),
                SourceInfo {
                    size: a.first()?.as_u64()?,
                    mtime: a.get(1)?.as_str()?.parse().ok()?,
                    hash: a.get(2)?.as_str()?.to_string(),
                    width: a.get(3)?.as_u64()? as u32,
                    height: a.get(4)?.as_u64()? as u32,
                },
            ))
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(x) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(x);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `encodeURI` : encode tout sauf les caractères réservés et non réservés d'une URL.
fn encode_uri(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric() || ";,/?:@&=+$-_.!~*'()#".contains(c) {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

impl AssetHooks for Pipeline {
    fn image(&self, a: &ImageArgs) -> Option<String> {
        let path = self.source_of(&a.src)?;
        let loading = if a.loading.is_empty() {
            "lazy"
        } else {
            &a.loading
        };
        let style = if a.ratio.is_empty() {
            String::new()
        } else {
            format!(" style=\"aspect-ratio:{}\"", escape(&a.ratio))
        };
        let class = if a.class.is_empty() {
            String::new()
        } else {
            format!(" class=\"{}\"", escape(&a.class))
        };
        if !Self::is_raster(&path) || animated_gif(&path) {
            return Some(format!(
                "<img src=\"{}\" alt=\"{}\"{class} loading=\"{loading}\" decoding=\"async\"{style}>",
                encode_uri(&a.src),
                escape(&a.alt)
            ));
        }
        let info = self.info(&path)?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let auto = Format::of_ext(&ext);
        let custom = !a.widths.is_empty();
        let mut formats = if custom {
            vec![Format::Webp, auto]
        } else {
            vec![Format::Avif, Format::Webp, auto]
        };
        formats.dedup();
        let widths = if custom { &a.widths } else { &self.widths };
        let vs = self.plan(&path, &info, widths, &formats, DEFAULT_QUALITY, "");
        self.register(&vs);
        let sizes = if a.sizes.is_empty() {
            "100vw"
        } else {
            &a.sizes
        };
        let srcset = |f: Format| -> String {
            vs.iter()
                .filter(|v| v.format == f)
                .map(|v| format!("{}{} {}w", self.url_prefix, v.name, v.width))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let fallback: Vec<&Variant> = vs.iter().filter(|v| v.format == auto).collect();
        let first = fallback.first()?;
        let last = fallback.last()?;
        let fetch = if loading == "eager" && !custom {
            " fetchpriority=\"high\""
        } else {
            ""
        };
        let img = format!(
            "<img src=\"{}{}\" alt=\"{}\"{class} loading=\"{loading}\" decoding=\"async\"{style}{fetch} width=\"{}\" height=\"{}\" srcset=\"{}\" sizes=\"{}\">",
            self.url_prefix,
            first.name,
            escape(&a.alt),
            last.width,
            last.height,
            srcset(auto),
            escape(sizes)
        );
        if formats.len() == 1 {
            return Some(img);
        }
        // Enveloppe sans boîte (D-151) : l'image se règle par rapport à son conteneur, comme
        // dans le canevas (largeur et hauteur à 100 %, recadrage) sans style pour `picture`.
        let mut html = String::from("<picture style=\"display:contents\">");
        for f in formats.iter().filter(|f| **f != auto) {
            html.push_str(&format!(
                "<source type=\"{}\" srcset=\"{}\" sizes=\"{}\">",
                f.mime(),
                srcset(*f),
                escape(sizes)
            ));
        }
        html.push_str(&img);
        html.push_str("</picture>");
        Some(html)
    }

    fn media_exists(&self, src: &str) -> bool {
        self.source_of(src).is_some()
    }

    fn share_image(&self, src: &str, base: &str) -> String {
        let absolute = |u: &str| -> String {
            url::Url::parse(base)
                .and_then(|b| b.join(u))
                .map(|u| u.to_string())
                .unwrap_or_else(|_| u.to_string())
        };
        if src.is_empty() {
            return String::new();
        }
        let Some(path) = self.source_of(src).filter(|p| Self::is_raster(p)) else {
            return absolute(&encode_uri(src));
        };
        let Some(info) = self.info(&path) else {
            return absolute(&encode_uri(src));
        };
        let vs = self.plan(
            &path,
            &info,
            &[SHARE_WIDTH],
            &[Format::Jpeg],
            SHARE_QUALITY,
            "share",
        );
        self.register(&vs);
        match vs.first() {
            Some(v) => absolute(&format!("{}{}", self.url_prefix, v.name)),
            None => absolute(&encode_uri(src)),
        }
    }
}

#[cfg(test)]
mod tests;
