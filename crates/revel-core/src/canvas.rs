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

use crate::{Core, CoreError, Result};
use revel_edit::canvas::Slot;
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
pub fn inject(html: &str, slots: &[Slot]) -> String {
    // `</` échappé : la liste ne peut pas fermer l'élément <script>.
    let json = serde_json::to_string(slots)
        .unwrap_or_else(|_| "[]".into())
        .replace("</", "<\\/");
    let tail = format!(
        "<script type=\"application/json\" id=\"rv-slots\">{json}</script>\
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
        });
        Ok(base_url().to_string())
    }

    /// Réponse du protocole du canevas pour le chemin demandé (`/agence/`, `/assets/css/site.css`).
    pub fn canvas_respond(&self, uri_path: &str) -> CanvasResponse {
        match self.canvas_response(uri_path) {
            Ok(Some(r)) => r,
            Ok(None) => CanvasResponse::not_found(),
            Err(e) => CanvasResponse {
                status: 500,
                content_type: "text/plain; charset=utf-8".into(),
                body: e.to_string().into_bytes(),
            },
        }
    }

    fn canvas_response(&self, uri_path: &str) -> Result<Option<CanvasResponse>> {
        let raw = uri_path.split(['?', '#']).next().unwrap_or("/");
        let decoded = percent_decode(raw);
        let path = decoded.as_str();
        // Rien hors du site : ni `..`, ni chemin absolu de fichier.
        if !path.starts_with('/') || path.split(['/', '\\']).any(|s| s == "..") {
            return Ok(None);
        }
        if path == CANVAS_JS_URL {
            return Ok(Some(CanvasResponse::ok(
                "text/javascript; charset=utf-8",
                CANVAS_JS.as_bytes().to_vec(),
            )));
        }
        let (site, pipeline, opts) = {
            let c = self.canvas.lock().unwrap_or_else(|e| e.into_inner());
            let c = c.as_ref().ok_or(CoreError::NotOpen)?;
            (c.site.clone(), c.pipeline.clone(), c.opts)
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
                    "production",
                    &EditorOptions {
                        scripts: opts.scripts,
                    },
                )?)
            })?;
            if let Some(page) = page {
                let html = inject(&page.html, &page.slots);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injection() {
        let slots = vec![Slot::Block {
            owner: revel_edit::Owner::Page("a".into()),
            node: "x</script>".into(),
        }];
        let h = inject("<html><body><p>a</p></BODY></html>", &slots);
        assert!(
            h.starts_with("<html><body><p>a</p><script type=\"application/json\" id=\"rv-slots\">")
        );
        assert!(h.contains("x<\\/script>"), "{h}");
        assert!(h.ends_with("<script src=\"/__revel/canvas.js\"></script></BODY></html>"));
        assert_eq!(content_type("/a/b.WOFF2"), "font/woff2");
        assert_eq!(content_type("/a/"), "application/octet-stream");
    }
}
