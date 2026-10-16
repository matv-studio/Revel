//! Serveur HTTP local du canevas (127.0.0.1) : mêmes réponses que le protocole `revel-canvas` de
//! l'application. Sert au diagnostic et aux essais de bout en bout dans un navigateur ordinaire
//! (`revel canvas-serve`).
//!
//! Avec `bridge`, il expose aussi les commandes d'édition (`POST /__revel/invoke/<commande>`, corps
//! JSON) pour l'interface ouverte dans un navigateur avec l'API Tauri simulée
//! (`apps/revel-ui/dev/mock-tauri.js`, paramètre `bridge=`). Ces commandes modifient le site : le
//! serveur n'écoute que sur l'adresse locale.

use crate::Core;
use crate::canvas::CanvasResponse;
use axum::body::Body;
use axum::http::{Method, Response, Uri};
use revel_protocol::Failure;
use serde_json::Value;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Préfixe des commandes du pont.
pub const INVOKE_PREFIX: &str = "/__revel/invoke/";
/// Aperçu du site (pont des essais).
pub const PREVIEW_PREFIX: &str = "/__preview/";

/// Contenu d'un fichier transmis en base64.
fn b64(s: &str) -> Result<Vec<u8>, Failure> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| Failure::detail(format!("base64 : {e}")))
}

fn arg<T: serde::de::DeserializeOwned>(args: &Value, name: &str) -> Result<T, Failure> {
    serde_json::from_value(args.get(name).cloned().unwrap_or(Value::Null))
        .map_err(|e| Failure::detail(format!("{name} : {e}")))
}

fn json<T: serde::Serialize>(v: T) -> Result<Value, Failure> {
    serde_json::to_value(v).map_err(|e| Failure::detail(e.to_string()))
}

impl Core {
    /// Site des assistants (MCP) : celui nommé (`site`), sinon le dernier ouvert pour l'édition.
    pub fn current_site(&self, named: Option<&str>) -> Result<PathBuf, Failure> {
        match named.filter(|s| !s.trim().is_empty()) {
            Some(s) => Ok(PathBuf::from(s)),
            None => self
                .last_site
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
                .ok_or_else(|| Failure::new("error-mcp-no-site")),
        }
    }

    /// Commande d'un assistant (MCP) : sur le site nommé (`args.site`) ou le site ouvert, ouvert
    /// pour l'édition au besoin. Mêmes commandes que le pont, plus le rendu d'une page.
    pub fn mcp_invoke(&self, cmd: &str, args: &Value) -> Result<Value, Failure> {
        let site = self.current_site(args.get("site").and_then(Value::as_str))?;
        let opened = self.open_session(&site).map_err(|e| e.failure())?;
        match cmd {
            "edit_open" => json(opened),
            "site_path" => json(site.display().to_string()),
            "page_html" => {
                let url = arg::<String>(args, "url")?;
                json(self.page_html(&site, &url).map_err(|e| e.failure())?)
            }
            _ => self.bridge_invoke(&site, "", cmd, args),
        }
    }

    /// Page rendue telle qu'elle sera publiée (repères de l'éditeur retirés), `None` : adresse
    /// inconnue.
    pub fn page_html(&self, site: &Path, url: &str) -> crate::Result<Option<String>> {
        self.with_project(site, |p| {
            let hooks: Arc<dyn revel_render::AssetHooks> =
                Arc::new(revel_project::ProjectAssets::new(p));
            Ok(p.render_editor(
                url,
                hooks,
                "production",
                &revel_render::EditorOptions { scripts: false },
            )?
            .map(|page| revel_render::strip_editor_marks(&page.html)))
        })
    }

    /// Commande d'édition reçue par le pont (le site est celui du serveur, quel que soit `path`).
    pub fn bridge_invoke(
        &self,
        site: &Path,
        base: &str,
        cmd: &str,
        args: &Value,
    ) -> Result<Value, Failure> {
        let f = |e: crate::CoreError| e.failure();
        match cmd {
            "edit_open" => json(self.edit_open(site).map_err(f)?),
            "edit_apply" => json(
                match arg::<Option<String>>(args, "group")? {
                    Some(g) => self.edit_apply_grouped(site, arg(args, "command")?, &g),
                    None => self.edit_apply(
                        site,
                        arg(args, "command")?,
                        arg::<Option<bool>>(args, "coalesce")?.unwrap_or(false),
                    ),
                }
                .map_err(f)?,
            ),
            "edit_seal" => json(self.edit_seal(site).map_err(f)?),
            "edit_pages" => json(self.edit_pages(site).map_err(f)?),
            "edit_media" => json(self.edit_media(site).map_err(f)?),
            "edit_locale_data" => json(
                self.edit_locale_data(site, &arg::<String>(args, "code")?)
                    .map_err(f)?,
            ),
            "edit_import_media" => {
                let data = b64(&arg::<String>(args, "data")?)?;
                json(
                    self.edit_import_media(site, &arg::<String>(args, "name")?, &data)
                        .map_err(f)?,
                )
            }
            "edit_replace_media" => {
                let data = b64(&arg::<String>(args, "data")?)?;
                json(
                    self.edit_replace_media(
                        site,
                        &arg::<String>(args, "url")?,
                        &arg::<String>(args, "name")?,
                        &data,
                    )
                    .map_err(f)?,
                )
            }
            "edit_stamp" => json(self.edit_stamp(site).map_err(f)?),
            "publish_state" => json(self.publish_state(site).map_err(f)?),
            "edit_undo" => json(self.edit_undo(site).map_err(f)?),
            "edit_redo" => json(self.edit_redo(site).map_err(f)?),
            "cms_collections" => json(self.cms(site).map_err(f)?),
            "translate_can" => {
                json(self.translate_can(&arg::<String>(args, "from")?, &arg::<String>(args, "to")?))
            }
            "readable_data" => json(
                self.readable_data(
                    site,
                    &arg::<Vec<Vec<String>>>(args, "paths")?,
                    &arg::<Vec<String>>(args, "collections")?,
                )
                .map_err(f)?,
            ),
            "versions_status" => json(self.versions_status(site).map_err(f)?),
            "versions_init" => json(
                self.versions_init(site, &arg::<String>(args, "message")?)
                    .map_err(f)?,
            ),
            "versions_list" => json(
                self.versions_list(site, arg::<usize>(args, "limit")?)
                    .map_err(f)?,
            ),
            "versions_save" => json(
                self.versions_save(site, &arg::<String>(args, "message")?)
                    .map_err(f)?,
            ),
            "versions_changes" => json(
                self.versions_changes(
                    site,
                    arg::<Option<String>>(args, "id")?.as_deref(),
                    arg::<Option<bool>>(args, "ofVersion")?.unwrap_or(false),
                )
                .map_err(f)?,
            ),
            "versions_diff" => {
                let id = arg::<Option<String>>(args, "id")?;
                let file = arg::<String>(args, "file")?;
                json(
                    match (id, arg::<Option<bool>>(args, "ofVersion")?.unwrap_or(false)) {
                        (Some(i), true) => self.versions_version_diff(site, &i, &file),
                        (id, _) => self.versions_diff(site, id.as_deref(), &file),
                    }
                    .map_err(f)?,
                )
            }
            "versions_restore" => json(
                self.versions_restore(
                    site,
                    &arg::<String>(args, "id")?,
                    &arg::<String>(args, "before")?,
                    &arg::<String>(args, "message")?,
                )
                .map_err(f)?,
            ),
            "versions_sync" => json(
                self.versions_sync(
                    site,
                    &arg::<String>(args, "message")?,
                    &arg::<String>(args, "merge")?,
                )
                .map_err(f)?,
            ),
            "versions_resolve" => json(
                self.versions_resolve(
                    site,
                    arg::<bool>(args, "keepOurs")?,
                    &arg::<String>(args, "message")?,
                )
                .map_err(f)?,
            ),
            "versions_set_remote" => json(
                self.versions_set_remote(site, &arg::<String>(args, "url")?)
                    .map_err(f)?,
            ),
            "fonts" => json(self.fonts(site).map_err(f)?),
            "fonts_google" => json(self.fonts_google().map_err(f)?),
            "fonts_add_google" => json(
                self.fonts_add_google(
                    site,
                    &arg::<String>(args, "family")?,
                    &arg::<Vec<String>>(args, "variants")?,
                )
                .map_err(f)?,
            ),
            "fonts_add_files" => json(
                self.fonts_add_files(
                    site,
                    &arg::<String>(args, "family")?,
                    &arg::<Vec<revel_protocol::FontUpload>>(args, "files")?,
                )
                .map_err(f)?,
            ),
            "fonts_remove" => json(
                self.fonts_remove(site, &arg::<String>(args, "family")?)
                    .map_err(f)?,
            ),
            "translate_packs" => json(self.translate_packs()),
            "translate_install" => json(
                self.translate_install(&arg::<String>(args, "code")?, |_| {})
                    .map_err(f)?,
            ),
            "translate_install_cancel" => {
                self.translate_install_cancel();
                json(())
            }
            "translate_remove" => json(
                self.translate_remove(&arg::<String>(args, "code")?)
                    .map_err(f)?,
            ),
            "translate" => json(
                self.translate(
                    &arg::<String>(args, "from")?,
                    &arg::<String>(args, "to")?,
                    &arg::<Vec<revel_protocol::TranslateItem>>(args, "items")?,
                    &arg::<Option<Vec<String>>>(args, "keep")?.unwrap_or_default(),
                )
                .map_err(f)?,
            ),
            // Aperçu servi sous `/__preview/` (essais de l'interface).
            "preview_url" => {
                let env = arg::<Option<String>>(args, "environment")?.unwrap_or_default();
                self.preview_open(site, &env).map_err(f)?;
                json(format!("{base}{}", PREVIEW_PREFIX.trim_start_matches('/')))
            }
            "canvas_open" => {
                let scripts = arg::<Option<bool>>(args, "scripts")?.unwrap_or(false);
                let current = self.canvas_options();
                self.canvas_open(
                    site,
                    crate::canvas::CanvasOptions {
                        scripts,
                        ..current.unwrap_or_default()
                    },
                )
                .map_err(f)?;
                json(base)
            }
            other => Err(Failure::detail(format!("commande inconnue : {other}"))),
        }
    }
}

fn response(status: u16, content_type: &str, body: Vec<u8>) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", content_type)
        .header("cache-control", "no-store")
        // Interface servie depuis une autre origine (essais) : réponses lisibles par elle.
        .header("access-control-allow-origin", "*")
        .body(Body::from(body))
        .unwrap_or_default()
}

/// Démarre le serveur ; `ready` reçoit l'adresse de base (`http://127.0.0.1:PORT/`). S'arrête
/// quand `stop` se termine.
pub async fn serve(
    core: Arc<Core>,
    site: PathBuf,
    port: u16,
    bridge: bool,
    ready: impl FnOnce(&str),
    stop: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await?;
    let base = format!("http://{}/", listener.local_addr()?);
    ready(&base);
    let app = axum::Router::new().fallback(move |method: Method, uri: Uri, body: String| {
        let core = core.clone();
        let site = site.clone();
        let base = base.clone();
        async move {
            if method == Method::OPTIONS {
                return response(204, "text/plain", Vec::new());
            }
            let path = uri.path().to_string();
            if let Some(cmd) = path.strip_prefix(INVOKE_PREFIX) {
                if !bridge || method != Method::POST {
                    return response(404, "text/plain", b"introuvable".to_vec());
                }
                let cmd = cmd.to_string();
                let r = tokio::task::spawn_blocking(move || {
                    let args: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                    core.bridge_invoke(&site, &base, &cmd, &args)
                })
                .await
                .unwrap_or_else(|e| Err(Failure::detail(e.to_string())));
                return match r {
                    Ok(v) => response(200, "application/json", v.to_string().into_bytes()),
                    Err(f) => response(
                        400,
                        "application/json",
                        serde_json::to_vec(&f).unwrap_or_default(),
                    ),
                };
            }
            // Adresse avec sa requête (canevas des composants : `?component=…`).
            let full = uri
                .path_and_query()
                .map(|p| p.as_str().to_string())
                .unwrap_or_else(|| path.clone());
            let r = tokio::task::spawn_blocking(move || match path.strip_prefix(PREVIEW_PREFIX) {
                Some(rest) if bridge => core.preview_respond(&format!("/{rest}")),
                _ => core.canvas_respond(&full),
            })
            .await
            .unwrap_or_else(|e| CanvasResponse {
                status: 500,
                content_type: "text/plain".into(),
                body: e.to_string().into_bytes(),
            });
            response(r.status, &r.content_type, r.body)
        }
    });
    axum::serve(listener, app)
        .with_graceful_shutdown(stop)
        .await
}
