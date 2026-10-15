//! Canevas de l'éditeur (étape 2.2) : réponses du protocole propre à l'application
//! (`revel-canvas://localhost/…`, `http://revel-canvas.localhost/…` sous Windows), sans port réseau.
//!
//! - page HTML : rendue à la demande depuis le document en cours d'édition, avec les repères des
//!   nœuds (`data-rv`, `<!--rv:N-->`), la liste des repères et le script du canevas ;
//! - feuille de style du modèle : générée depuis le document (modifications visibles aussitôt) ;
//! - images responsives : variante déjà produite (cache), sinon image d'origine ;
//! - autres fichiers : copies du projet (`passthrough`).
//!
//! Un seul site à la fois dans le canevas (celui de la fenêtre d'édition).
//!
//! **Aperçu** (protocole `revel-preview`, étape 2.13) : mêmes réponses, page sans repères ni script
//! du canevas, scripts du site actifs — identique à la page générée (vérifié par
//! `canvas-check`) —, rendue à la demande depuis le document en cours : rien à générer avant.

use crate::{Core, CoreError, Result};
use revel_edit::canvas::{Slot, VarValue};
use revel_render::EditorOptions;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Script injecté dans chaque page du canevas.
pub const CANVAS_JS: &str = include_str!("canvas.js");
/// Adresse du script dans le canevas (préfixe réservé à Revel).
pub const CANVAS_JS_URL: &str = "/__revel/canvas.js";

/// Réponse du protocole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasResponse {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

impl CanvasResponse {
    fn ok(content_type: &str, body: Vec<u8>) -> Self {
        Self {
            status: 200,
            content_type: content_type.to_string(),
            body,
        }
    }

    fn not_found() -> Self {
        Self {
            status: 404,
            content_type: "text/plain; charset=utf-8".into(),
            body: b"introuvable".to_vec(),
        }
    }
}

pub(crate) struct Canvas {
    site: PathBuf,
    pipeline: Arc<revel_assets::Pipeline>,
    opts: CanvasOptions,
    /// Environnement des pages rendues (`production` pour le canevas).
    environment: String,
}

/// Réglages du canevas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasOptions {
    /// Exécuter les scripts du site.
    pub scripts: bool,
    /// Images responsives comme dans le site généré (`<picture>`, variantes) ; sinon `<img>`
    /// simple, comme une génération sans images (`revel build --no-images`).
    pub responsive_images: bool,
}

impl Default for CanvasOptions {
    fn default() -> Self {
        Self {
            scripts: false,
            responsive_images: true,
        }
    }
}

/// Adresse de base de l'aperçu selon le système.
pub fn preview_base_url() -> &'static str {
    if cfg!(any(windows, target_os = "android")) {
        "http://revel-preview.localhost/"
    } else {
        "revel-preview://localhost/"
    }
}

/// Adresse de base du canevas selon le système (origine des protocoles propres de Tauri).
pub fn base_url() -> &'static str {
    if cfg!(any(windows, target_os = "android")) {
        "http://revel-canvas.localhost/"
    } else {
        "revel-canvas://localhost/"
    }
}

fn content_type(path: &str) -> &'static str {
    let ext = path
        .rsplit('/')
        .next()
        .and_then(|f| f.rsplit_once('.'))
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "webmanifest" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "pdf" => "application/pdf",
        "xml" => "application/xml",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// `%C3%A9` -> `é` (chemins des requêtes du canevas).
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

/// Ajoute la liste des repères et le script du canevas avant `</body>`.
/// Nombre de médias listés au plus (un dossier énorme ne bloque pas l'éditeur).
const MEDIA_MAX: usize = 20_000;

/// Ajoute le fichier `file` (adresse `url`) s'il s'agit d'un média.
fn media_entry(file: &Path, url: &str, out: &mut Vec<revel_protocol::MediaFile>) {
    let Some(kind) = revel_protocol::MediaKind::of(url) else {
        return;
    };
    let meta = std::fs::metadata(file).ok();
    out.push(revel_protocol::MediaFile {
        url: url.to_string(),
        kind,
        size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
        modified: meta
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| i64::try_from(d.as_secs()).ok()),
    });
}

pub fn inject(html: &str, slots: &[Slot], vars: &[VarValue]) -> String {
    // `</` échappé : la liste ne peut pas fermer l'élément <script>.
    let json = |v: String| v.replace("</", "<\\/");
    let slots = json(serde_json::to_string(slots).unwrap_or_else(|_| "[]".into()));
    let vars = json(serde_json::to_string(vars).unwrap_or_else(|_| "[]".into()));
    let tail = format!(
        "<script type=\"application/json\" id=\"rv-slots\">{slots}</script>\
         <script type=\"application/json\" id=\"rv-vars\">{vars}</script>\
         <script src=\"{CANVAS_JS_URL}\"></script>"
    );
    match html.to_ascii_lowercase().rfind("</body") {
        Some(i) => format!("{}{tail}{}", &html[..i], &html[i..]),
        None => format!("{html}{tail}"),
    }
}

impl Core {
    /// Affiche dans le canevas un site ouvert pour l'édition ; renvoie l'adresse de base.
    pub fn canvas_open(&self, path: &Path, opts: CanvasOptions) -> Result<String> {
        let pipeline = self.with_project(path, |p| {
            Ok(Arc::new(revel_assets::Pipeline::new(
                &p.dir,
                &p.site.passthrough,
                &p.site.settings.image_widths,
            )))
        })?;
        let mut c = self.canvas.lock().unwrap_or_else(|e| e.into_inner());
        *c = Some(Canvas {
            site: path.to_path_buf(),
            pipeline,
            opts,
            environment: "production".into(),
        });
        Ok(base_url().to_string())
    }

    /// Aperçu d'un site pour un environnement : pages rendues à la demande depuis le document en
    /// cours (session d'édition ouverte si besoin) ; renvoie l'adresse de base du protocole.
    pub fn preview_open(&self, path: &Path, environment: &str) -> Result<String> {
        self.edit_open(path)?;
        let pipeline = self.with_project(path, |p| {
            Ok(Arc::new(revel_assets::Pipeline::new(
                &p.dir,
                &p.site.passthrough,
                &p.site.settings.image_widths,
            )))
        })?;
        let mut c = self.preview.lock().unwrap_or_else(|e| e.into_inner());
        *c = Some(Canvas {
            site: path.to_path_buf(),
            pipeline,
            opts: CanvasOptions {
                scripts: true,
                responsive_images: true,
            },
            environment: environment.to_string(),
        });
        Ok(preview_base_url().to_string())
    }

    /// Réponse du protocole de l'aperçu.
    pub fn preview_respond(&self, uri_path: &str) -> CanvasResponse {
        respond(self.canvas_response(uri_path, true))
    }

    /// Pages HTML d'un site ouvert pour l'édition (document en mémoire).
    pub fn edit_pages(&self, path: &Path) -> Result<Vec<revel_protocol::EditorPageInfo>> {
        let pages = self.with_project(path, |p| Ok(p.list_pages()?))?;
        Ok(pages
            .into_iter()
            .map(|p| revel_protocol::EditorPageInfo {
                url: p.url,
                title: p.title,
                page: p.page,
                collection: p.collection,
                layout: p.layout,
                file: p.file,
                draft: p.draft,
                name: p.name,
                seo: p.seo.map(|s| revel_protocol::PageSeoInfo {
                    title: s.title,
                    description: s.description,
                    image: s.image,
                    noindex: s.noindex,
                    computed: s.source == revel_render::SeoSource::Computed,
                    data_path: match s.source {
                        revel_render::SeoSource::Data(k) => k,
                        _ => Vec::new(),
                    },
                }),
            })
            .collect())
    }

    /// Médias du site : fichiers des dossiers copiés tels quels (`passthrough`), hors code et
    /// fichiers cachés, triés par adresse.
    pub fn edit_media(&self, path: &Path) -> Result<Vec<revel_protocol::MediaFile>> {
        let (dir, pass) =
            self.with_project(path, |p| Ok((p.dir.clone(), p.site.passthrough.clone())))?;
        let mut out = Vec::new();
        for pt in &pass {
            let src = dir.join(&pt.from);
            let to = pt.to.trim_matches('/');
            let url = |rel: &str| match to {
                "" => format!("/{rel}"),
                t => format!("/{t}/{rel}"),
            };
            if src.is_file() {
                let name = src.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                media_entry(
                    &src,
                    &match to {
                        "" => format!("/{name}"),
                        t => format!("/{t}"),
                    },
                    &mut out,
                );
                continue;
            }
            let mut stack = vec![(src.clone(), String::new(), 0usize)];
            while let Some((d, rel, depth)) = stack.pop() {
                let Ok(rd) = std::fs::read_dir(&d) else {
                    continue;
                };
                for e in rd.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') || out.len() >= MEDIA_MAX {
                        continue;
                    }
                    let r = if rel.is_empty() {
                        name.clone()
                    } else {
                        format!("{rel}/{name}")
                    };
                    let p = e.path();
                    if p.is_dir() {
                        if depth < 12 {
                            stack.push((p, r, depth + 1));
                        }
                    } else {
                        media_entry(&p, &url(&r), &mut out);
                    }
                }
            }
        }
        out.sort_by(|a, b| a.url.cmp(&b.url));
        out.dedup_by(|a, b| a.url == b.url);
        Ok(out)
    }

    /// Réglages du canevas ouvert.
    pub fn canvas_options(&self) -> Option<CanvasOptions> {
        let c = self.canvas.lock().unwrap_or_else(|e| e.into_inner());
        c.as_ref().map(|c| c.opts)
    }

    /// Réponse du protocole du canevas pour le chemin demandé (`/agence/`, `/assets/css/site.css`).
    pub fn canvas_respond(&self, uri_path: &str) -> CanvasResponse {
        respond(self.canvas_response(uri_path, false))
    }

    fn canvas_response(&self, uri_path: &str, preview: bool) -> Result<Option<CanvasResponse>> {
        let raw = uri_path.split(['?', '#']).next().unwrap_or("/");
        let decoded = percent_decode(raw);
        let path = decoded.as_str();
        // Rien hors du site : ni `..`, ni chemin absolu de fichier.
        if !path.starts_with('/') || path.split(['/', '\\']).any(|s| s == "..") {
            return Ok(None);
        }
        if path == CANVAS_JS_URL && !preview {
            return Ok(Some(CanvasResponse::ok(
                "text/javascript; charset=utf-8",
                CANVAS_JS.as_bytes().to_vec(),
            )));
        }
        let (site, pipeline, opts, environment) = {
            let slot = if preview { &self.preview } else { &self.canvas };
            let c = slot.lock().unwrap_or_else(|e| e.into_inner());
            let c = c.as_ref().ok_or(CoreError::NotOpen)?;
            (
                c.site.clone(),
                c.pipeline.clone(),
                c.opts,
                c.environment.clone(),
            )
        };
        // Feuille de style du modèle.
        let sheet = self.with_project(&site, |p| {
            Ok(p.site
                .stylesheet
                .as_ref()
                .filter(|s| s.url == path)
                .map(revel_style::render))
        })?;
        if let Some(css) = sheet {
            return Ok(Some(CanvasResponse::ok(
                "text/css; charset=utf-8",
                css.into_bytes(),
            )));
        }
        // Variante d'image : produite (cache) ou image d'origine.
        if let Some(name) = path.strip_prefix("/img/")
            && let Some(file) = pipeline.variant_or_source(name)
        {
            let ct = content_type(&file.to_string_lossy());
            return Ok(std::fs::read(&file).ok().map(|b| CanvasResponse::ok(ct, b)));
        }
        // Page HTML.
        if revel_render::DocKind::of_path(path) == revel_render::DocKind::Html {
            let page = self.with_project(&site, |p| {
                let hooks: Arc<dyn revel_render::AssetHooks> = if opts.responsive_images {
                    pipeline.clone()
                } else {
                    Arc::new(revel_project::ProjectAssets::new(p))
                };
                Ok(p.render_editor(
                    path,
                    hooks,
                    &environment,
                    &EditorOptions {
                        scripts: opts.scripts,
                    },
                )?)
            })?;
            if let Some(page) = page {
                let html = if preview {
                    revel_render::strip_editor_marks(&page.html)
                } else {
                    inject(&page.html, &page.slots, &page.vars)
                };
                return Ok(Some(CanvasResponse::ok(
                    "text/html; charset=utf-8",
                    html.into_bytes(),
                )));
            }
        }
        // Copie du projet.
        let file = self.with_project(&site, |p| Ok(p.source_of_url(path)))?;
        Ok(file.and_then(|f| {
            std::fs::read(&f)
                .ok()
                .map(|b| CanvasResponse::ok(content_type(path), b))
        }))
    }
}

fn respond(r: Result<Option<CanvasResponse>>) -> CanvasResponse {
    match r {
        Ok(Some(r)) => r,
        Ok(None) => CanvasResponse::not_found(),
        Err(e) => CanvasResponse {
            status: 500,
            content_type: "text/plain; charset=utf-8".into(),
            body: e.to_string().into_bytes(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injection() {
        let slots = vec![Slot::Block {
            owner: revel_edit::Owner::Page("a".into()),
            node: "x</script>".into(),
        }];
        let h = inject("<html><body><p>a</p></BODY></html>", &slots, &[]);
        assert!(
            h.starts_with("<html><body><p>a</p><script type=\"application/json\" id=\"rv-slots\">")
        );
        assert!(h.contains("x<\\/script>"), "{h}");
        assert!(h.ends_with("<script src=\"/__revel/canvas.js\"></script></BODY></html>"));
        assert_eq!(content_type("/a/b.WOFF2"), "font/woff2");
        assert_eq!(content_type("/a/"), "application/octet-stream");
    }
}
