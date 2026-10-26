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

/// Animation d'une archive dotLottie (`.lottie` : fichier zip, `animations/<nom>.json`) : la
/// première, telle qu'écrite.
fn dotlottie_json(data: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(data)).ok()?;
    let mut names: Vec<String> = zip
        .file_names()
        .filter(|n| n.starts_with("animations/") && n.ends_with(".json"))
        .map(str::to_string)
        .collect();
    names.sort();
    let mut f = zip.by_name(names.first()?).ok()?;
    let mut out = Vec::new();
    f.read_to_end(&mut out).ok()?;
    revel_render::lottie::is_lottie(&out).then_some(out)
}

/// Fichier JSON d'une animation Lottie (début du fichier lu).
fn lottie_file(file: &Path) -> bool {
    use std::io::Read;
    let mut head = vec![0u8; 64 * 1024];
    let n = std::fs::File::open(file)
        .and_then(|mut f| f.read(&mut head))
        .unwrap_or(0);
    revel_render::lottie::is_lottie(&head[..n])
}

/// Ajoute le fichier `file` (adresse `url`) s'il s'agit d'un média (une animation Lottie
/// enregistrée en `.json` comprise).
fn media_entry(file: &Path, url: &str, out: &mut Vec<revel_protocol::MediaFile>) {
    let kind = match revel_protocol::MediaKind::of(url) {
        Some(k) => k,
        None if url.to_ascii_lowercase().ends_with(".json") && lottie_file(file) => {
            revel_protocol::MediaKind::Animation
        }
        None => return,
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

/// `rev` : révision de la session d'édition au moment du rendu (l'éditeur demande une mise à jour
/// sur place d'une page rendue avant sa dernière modification).
/// Règle qui cache les calques masqués (`data-revel-hidden`) dans le canevas et l'aperçu ; le site
/// publié ne les contient pas.
pub fn hide_hidden(html: &str) -> String {
    let rule = format!(
        "<style data-rv-hidden>[{}]{{display:none!important}}</style>",
        revel_model::HIDDEN_ATTR
    );
    if !html.contains(revel_model::HIDDEN_ATTR) {
        return html.to_string();
    }
    match html.to_ascii_lowercase().find("</head") {
        Some(i) => format!("{}{rule}{}", &html[..i], &html[i..]),
        None => format!("{rule}{html}"),
    }
}

/// Aperçu dans l'espace de travail (cadre) : un lien vers un autre site qui s'ouvre dans un nouvel
/// onglet est confié à l'interface (navigateur du système) ; les autres liens externes se chargent
/// dans le cadre. Dans la fenêtre d'aperçu, rien ne change (l'application ouvre les nouveaux
/// onglets dans le navigateur du système).
const PREVIEW_LINKS: &str = concat!(
    "<script data-rv-preview>(function(){if(window.top===window)return;",
    "document.addEventListener(\"click\",function(e){if(e.defaultPrevented||e.button)return;",
    "var a=e.target.closest&&e.target.closest(\"a[href]\");if(!a)return;var u;try{u=new URL(a.href,location.href)}catch(_){return}",
    "if(!/^https?:$/.test(u.protocol)||u.origin===location.origin)return;",
    "if(a.target===\"_blank\"||e.ctrlKey||e.metaKey||e.shiftKey){e.preventDefault();",
    "parent.postMessage({revel:1,type:\"open-external\",url:u.href},\"*\")}},true)})()</script>",
);

/// Page de l'aperçu : repères de l'éditeur retirés, liens externes ouverts (voir [`PREVIEW_LINKS`]).
fn preview_page(html: &str) -> String {
    let html = revel_render::strip_editor_marks(html);
    match html.to_ascii_lowercase().rfind("</body") {
        Some(i) => format!("{}{PREVIEW_LINKS}{}", &html[..i], &html[i..]),
        None => format!("{html}{PREVIEW_LINKS}"),
    }
}

pub fn inject(html: &str, slots: &[Slot], vars: &[VarValue], rev: Option<u64>) -> String {
    // `</` échappé : la liste ne peut pas fermer l'élément <script>.
    let json = |v: String| v.replace("</", "<\\/");
    let slots = json(serde_json::to_string(slots).unwrap_or_else(|_| "[]".into()));
    let vars = json(serde_json::to_string(vars).unwrap_or_else(|_| "[]".into()));
    let rev = rev
        .map(|r| format!(" data-rev=\"{r}\""))
        .unwrap_or_default();
    let tail = format!(
        "<script type=\"application/json\" id=\"rv-slots\"{rev}>{slots}</script>\
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

    /// Textes des fichiers de données à traduire dans une autre langue (vue Localisation, 3.8).
    pub fn edit_locale_data(
        &self,
        path: &Path,
        code: &str,
    ) -> Result<Vec<revel_edit::locale::DataText>> {
        self.with_project(path, |p| Ok(p.locale_data_texts(code)?))
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

    /// Dossier des médias du site : dossier copié tel quel publié sous `/media` (sinon le premier
    /// dossier copié tel quel) ; sans dossier copié, `media/` est créé et déclaré.
    pub(crate) fn media_dir(&self, path: &Path) -> Result<(PathBuf, String)> {
        let found = self.with_project(path, |p| {
            let pick = p
                .site
                .passthrough
                .iter()
                .find(|x| x.to.trim_matches('/') == "media")
                .or_else(|| {
                    p.site.passthrough.iter().find(|x| {
                        p.dir.join(&x.from).is_dir() && !x.to.trim_matches('/').is_empty()
                    })
                })
                .map(|x| (p.dir.join(&x.from), format!("/{}", x.to.trim_matches('/'))));
            Ok(pick)
        })?;
        if let Some(f) = found {
            std::fs::create_dir_all(&f.0)?;
            return Ok(f);
        }
        self.with_session_mut(path, |p| {
            p.site.passthrough.push(revel_model::Passthrough {
                from: "media".into(),
                to: "media".into(),
            });
            Ok(())
        })?;
        let dir = path.join("media");
        std::fs::create_dir_all(&dir)?;
        Ok((dir, "/media".into()))
    }

    /// Importe un fichier média (`name` : nom d'origine, `data` : contenu) dans le dossier des
    /// médias (`folder` : sous-dossier sur le disque, facultatif) ; nom en minuscules sans accent,
    /// numéroté s'il est pris. Refusé si ce n'est pas un média.
    pub fn edit_import_media(
        &self,
        path: &Path,
        name: &str,
        data: &[u8],
    ) -> Result<revel_protocol::MediaFile> {
        let (stem, mut ext) = match name.rsplit_once('.') {
            Some((s, e)) => (s, e.to_ascii_lowercase()),
            None => (name, String::new()),
        };
        // Animation Lottie : fichier `.json` (Bodymovin), ou `.lottie` (archive de LottieFiles)
        // dont l'animation est extraite en `.json` (lue par le lecteur du site).
        let mut data = std::borrow::Cow::Borrowed(data);
        if ext == "lottie" {
            data =
                std::borrow::Cow::Owned(dotlottie_json(&data).ok_or_else(|| {
                    CoreError::Media(format!("animation Lottie illisible : {name}"))
                })?);
            ext = "json".into();
        }
        let kind = if ext == "json" {
            revel_render::lottie::is_lottie(&data).then_some(revel_protocol::MediaKind::Animation)
        } else {
            revel_protocol::MediaKind::of(&format!("x.{ext}"))
        }
        .ok_or_else(|| CoreError::Media(format!("type de fichier refusé : {name}")))?;
        let (dir, prefix) = self.media_dir(path)?;
        let base = {
            let s = revel_model::slugify(stem);
            if s.is_empty() { "media".to_string() } else { s }
        };
        let mut file = format!("{base}.{ext}");
        let mut n = 2;
        while dir.join(&file).exists() {
            file = format!("{base}-{n}.{ext}");
            n += 1;
        }
        let target = dir.join(&file);
        std::fs::write(&target, &*data)?;
        let url = format!("{prefix}/{file}");
        let mut out = Vec::new();
        media_entry(&target, &url, &mut out);
        out.pop()
            .filter(|m| m.kind == kind)
            .ok_or_else(|| CoreError::Media(format!("import impossible : {name}")))
    }

    /// Remplace le contenu d'un média (même adresse, donc toutes ses utilisations suivent) ; même
    /// extension exigée ; l'ancien fichier est gardé dans `.revel/corbeille/`.
    pub fn edit_replace_media(
        &self,
        path: &Path,
        url: &str,
        name: &str,
        data: &[u8],
    ) -> Result<revel_protocol::MediaFile> {
        let ext_of = |s: &str| {
            s.rsplit_once('.')
                .map(|(_, e)| e.to_ascii_lowercase())
                .unwrap_or_default()
        };
        if ext_of(url) != ext_of(name) {
            return Err(CoreError::Media(format!(
                "même format exigé : {} attendu",
                ext_of(url)
            )));
        }
        let file = self
            .with_project(path, |p| Ok(p.source_of_url(url)))?
            .filter(|f| f.is_file())
            .ok_or_else(|| CoreError::Media(format!("média introuvable : {url}")))?;
        let trash = path.join(".revel").join("corbeille");
        std::fs::create_dir_all(&trash)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let fname = file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        std::fs::copy(&file, trash.join(format!("{stamp}-{fname}")))?;
        std::fs::write(&file, data)?;
        let mut out = Vec::new();
        media_entry(&file, url, &mut out);
        out.pop()
            .ok_or_else(|| CoreError::Media(format!("remplacement impossible : {url}")))
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
        // Script des animations (6.1) : site animé (aperçu, canevas avec scripts) et aperçu d'une
        // animation dans le canevas.
        if path == revel_render::animations::URL {
            return Ok(Some(CanvasResponse::ok(
                "text/javascript; charset=utf-8",
                revel_render::animations::script().into_bytes(),
            )));
        }
        // Interactions entre éléments (6.3) : aperçu du site.
        if path == revel_render::actions::URL {
            return Ok(Some(CanvasResponse::ok(
                "text/javascript; charset=utf-8",
                revel_render::actions::script().into_bytes(),
            )));
        }
        // Lecteur des animations Lottie (6.2).
        if path == revel_render::lottie::URL {
            return Ok(Some(CanvasResponse::ok(
                "text/javascript; charset=utf-8",
                revel_render::lottie::script().into_bytes(),
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
            let css = if preview { css } else { viewport_units(&css) };
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
        // Composant seul (canevas des composants) : `?component=<id>&variant=<id>&page=<adresse>`.
        // Aperçu : le composant seul avec les scripts du site, pour l'essayer avant de le placer.
        if path == revel_project::COMPONENT_URL {
            let query = uri_path.split_once('?').map(|(_, q)| q).unwrap_or_default();
            let param = |k: &str| {
                query
                    .split('&')
                    .filter_map(|kv| kv.split_once('='))
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| percent_decode(v))
                    .filter(|v| !v.is_empty())
            };
            let Some(component) = param("component") else {
                return Ok(None);
            };
            let variant = param("variant");
            let reference = param("page").unwrap_or_else(|| "/".into());
            let rev = self.session_revision(&site);
            let page = self.with_project(&site, |p| {
                let hooks: Arc<dyn revel_render::AssetHooks> = if opts.responsive_images {
                    pipeline.clone()
                } else {
                    Arc::new(revel_project::ProjectAssets::new(p))
                };
                Ok(p.render_component_editor(
                    &component,
                    variant.as_deref(),
                    &reference,
                    hooks,
                    &environment,
                    &EditorOptions {
                        scripts: preview && opts.scripts,
                    },
                )?)
            })?;
            return Ok(page.map(|page| {
                let html = hide_hidden(&if preview {
                    preview_page(&page.html)
                } else {
                    html_viewport_units(&inject(&page.html, &page.slots, &page.vars, rev))
                });
                CanvasResponse::ok("text/html; charset=utf-8", html.into_bytes())
            }));
        }
        // Page HTML.
        if revel_render::DocKind::of_path(path) == revel_render::DocKind::Html {
            // Révision lue avant le rendu : une modification faite pendant le rendu sera signalée.
            let rev = self.session_revision(&site);
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
                let html = hide_hidden(&if preview {
                    preview_page(&page.html)
                } else {
                    html_viewport_units(&inject(&page.html, &page.slots, &page.vars, rev))
                });
                return Ok(Some(CanvasResponse::ok(
                    "text/html; charset=utf-8",
                    html.into_bytes(),
                )));
            }
        }
        // Copie du projet (feuilles de style : unités de hauteur de fenêtre, comme ci-dessus).
        let file = self.with_project(&site, |p| Ok(p.source_of_url(path)))?;
        Ok(file.and_then(|f| {
            let b = std::fs::read(&f).ok()?;
            let b = match String::from_utf8(b) {
                Ok(css) if !preview && path.to_ascii_lowercase().ends_with(".css") => {
                    viewport_units(&css).into_bytes()
                }
                Ok(text) => text.into_bytes(),
                Err(e) => e.into_bytes(),
            };
            Some(CanvasResponse::ok(content_type(path), b))
        }))
    }
}

/// Caractère d'identifiant CSS (nom de classe, unité…).
fn ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b >= 0x80
}

/// Unités de hauteur de fenêtre (`vh`, `svh`, `lvh`, `dvh`) d'une feuille servie au canevas
/// rapportées à `--rv-vh`, que le canevas règle sur la hauteur normale du cadre quand le cadre
/// est allongé (à défaut : l'unité elle-même, rendu identique). Allonger le cadre montre plus
/// de la page sans la remettre en page. Commentaires, chaînes et `url()` laissés tels quels.
pub(crate) fn viewport_units(css: &str) -> String {
    let b = css.as_bytes();
    let n = b.len();
    let mut out = String::with_capacity(n + 64);
    let mut i = 0;
    let mut last = 0;
    while i < n {
        let c = b[i];
        // Commentaire.
        if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i = css[i + 2..].find("*/").map_or(n, |k| i + 2 + k + 2);
            continue;
        }
        // Chaîne.
        if c == b'"' || c == b'\'' {
            let mut j = i + 1;
            while j < n && b[j] != c {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            i = (j + 1).min(n);
            continue;
        }
        // Adresse.
        if (c == b'u' || c == b'U')
            && (i == 0 || !ident_byte(b[i - 1]))
            && css[i..].len() >= 4
            && css[i..i + 4].eq_ignore_ascii_case("url(")
        {
            i = css[i..].find(')').map_or(n, |k| i + k + 1);
            continue;
        }
        let digit = |k: usize| b.get(k).is_some_and(u8::is_ascii_digit);
        let starts = c.is_ascii_digit()
            || (c == b'.' && digit(i + 1))
            || ((c == b'-' || c == b'+')
                && (digit(i + 1) || (b.get(i + 1) == Some(&b'.') && digit(i + 2))));
        if starts && (i == 0 || !(ident_byte(b[i - 1]) || b[i - 1] == b'.' || b[i - 1] == b'#')) {
            let start = i;
            let mut j = i;
            if c == b'-' || c == b'+' {
                j += 1;
            }
            while digit(j) {
                j += 1;
            }
            if b.get(j) == Some(&b'.') && digit(j + 1) {
                j += 1;
                while digit(j) {
                    j += 1;
                }
            }
            let num_end = j;
            while j < n && b[j].is_ascii_alphabetic() {
                j += 1;
            }
            let unit = css[num_end..j].to_ascii_lowercase();
            if matches!(unit.as_str(), "vh" | "svh" | "lvh" | "dvh")
                && (j == n || !ident_byte(b[j]))
            {
                out.push_str(&css[last..start]);
                out.push_str(&format!(
                    "calc({} * var(--rv-vh, 1{unit}))",
                    &css[start..num_end]
                ));
                last = j;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    out.push_str(&css[last..]);
    out
}

/// Même traitement dans une page : blocs `<style>` et attributs `style`.
pub(crate) fn html_viewport_units(html: &str) -> String {
    if !html.contains("vh") {
        return html.to_string();
    }
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len() + 64);
    let mut last = 0;
    let mut i = 0;
    while let Some(k) = lower[i..].find("style") {
        let at = i + k;
        let before = lower.as_bytes().get(at.wrapping_sub(1)).copied();
        // `<style …>…</style>`
        if before == Some(b'<') {
            let Some(open_end) = lower[at..].find('>').map(|e| at + e + 1) else {
                break;
            };
            let Some(close) = lower[open_end..].find("</style").map(|e| open_end + e) else {
                break;
            };
            out.push_str(&html[last..open_end]);
            out.push_str(&viewport_units(&html[open_end..close]));
            last = close;
            i = close + 7;
            continue;
        }
        // ` style="…"`
        if before.is_some_and(|b| b.is_ascii_whitespace())
            && let Some(rest) = lower[at + 5..].strip_prefix("=\"")
        {
            let vstart = html.len() - rest.len();
            let Some(vend) = rest.find('"').map(|e| vstart + e) else {
                break;
            };
            out.push_str(&html[last..vstart]);
            out.push_str(&viewport_units(&html[vstart..vend]));
            last = vend;
            i = vend;
            continue;
        }
        i = at + 5;
    }
    out.push_str(&html[last..]);
    out
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

    #[test]
    fn unites_de_hauteur_de_fenetre() {
        let css = ".hero{min-height:100vh;height:calc(50dvh - 2rem)}.col-50vh{top:-10.5svh}\
            /* 30vh */ .a::after{content:\"5vh\"}.b{background:url(x-20vh.png);margin:1.5e2px 2lvh}";
        assert_eq!(
            viewport_units(css),
            ".hero{min-height:calc(100 * var(--rv-vh, 1vh));height:calc(calc(50 * var(--rv-vh, 1dvh)) - 2rem)}\
            .col-50vh{top:calc(-10.5 * var(--rv-vh, 1svh))}/* 30vh */ .a::after{content:\"5vh\"}\
            .b{background:url(x-20vh.png);margin:1.5e2px calc(2 * var(--rv-vh, 1lvh))}"
        );
        // Sans unité de fenêtre : inchangée.
        assert_eq!(
            viewport_units("a{width:100vw;height:10px}"),
            "a{width:100vw;height:10px}"
        );
        let html =
            "<style>.x{height:80vh}</style><div style=\"height:20vh\" data-x=\"10vh\">10vh</div>";
        assert_eq!(
            html_viewport_units(html),
            "<style>.x{height:calc(80 * var(--rv-vh, 1vh))}</style>\
             <div style=\"height:calc(20 * var(--rv-vh, 1vh))\" data-x=\"10vh\">10vh</div>"
        );
    }

    use super::*;

    #[test]
    fn injection() {
        let slots = vec![Slot::Block {
            owner: revel_edit::Owner::Page("a".into()),
            node: "x</script>".into(),
            component: None,
        }];
        let h = inject("<html><body><p>a</p></BODY></html>", &slots, &[], None);
        assert!(
            h.starts_with("<html><body><p>a</p><script type=\"application/json\" id=\"rv-slots\">")
        );
        assert!(h.contains("x<\\/script>"), "{h}");
        assert!(h.ends_with("<script src=\"/__revel/canvas.js\"></script></BODY></html>"));
        assert_eq!(content_type("/a/b.WOFF2"), "font/woff2");
        assert_eq!(content_type("/a/"), "application/octet-stream");
    }
}
