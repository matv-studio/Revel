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

fn arg<T: serde::de::DeserializeOwned>(args: &Value, name: &str) -> Result<T, Failure> {
    serde_json::from_value(args.get(name).cloned().unwrap_or(Value::Null))
        .map_err(|e| Failure::detail(format!("{name} : {e}")))
}

fn json<T: serde::Serialize>(v: T) -> Result<Value, Failure> {
    serde_json::to_value(v).map_err(|e| Failure::detail(e.to_string()))
}

impl Core {
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
                self.edit_apply(
                    site,
                    arg(args, "command")?,
                    arg::<Option<bool>>(args, "coalesce")?.unwrap_or(false),
                )
                .map_err(f)?,
            ),
            "edit_seal" => json(self.edit_seal(site).map_err(f)?),
            "edit_pages" => json(self.edit_pages(site).map_err(f)?),
            "edit_media" => json(self.edit_media(site).map_err(f)?),
            "edit_stamp" => json(self.edit_stamp(site).map_err(f)?),
            "edit_undo" => json(self.edit_undo(site).map_err(f)?),
            "edit_redo" => json(self.edit_redo(site).map_err(f)?),
            "cms_collections" => json(self.cms(site).map_err(f)?),
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
            let r = tokio::task::spawn_blocking(move || match path.strip_prefix(PREVIEW_PREFIX) {
                Some(rest) if bridge => core.preview_respond(&format!("/{rest}")),
                _ => core.canvas_respond(&path),
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
