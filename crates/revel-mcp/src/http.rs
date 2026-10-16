//! Serveur HTTP local des assistants : MCP en HTTP en flux (`/mcp`, protocoles 2025 et
//! 2026-07-28), mêmes outils en HTTP simple décrits en OpenAPI (`POST /api/<outil>`,
//! `GET /openapi.json`, 4.3) et commandes du relais stdio (`POST /invoke/<commande>`).
//! N'écoute que sur 127.0.0.1 ; chaque requête doit porter le jeton (`Authorization: Bearer …`) ;
//! les en-têtes `Host` et `Origin` sont vérifiés (pas de requête venue d'une page web).

use crate::{Invoke, Revel};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct Shared {
    inv: Arc<dyn Invoke>,
    token: Arc<String>,
}

/// Jeton présent et exact (comparaison en temps constant).
fn authorized(h: &HeaderMap, token: &str) -> bool {
    let Some(v) = h.get("authorization").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Some(given) = v.strip_prefix("Bearer ").map(str::trim) else {
        return false;
    };
    given.len() == token.len()
        && given
            .bytes()
            .zip(token.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

/// Hôte local (`127.0.0.1`, `localhost`, `[::1]`, `host.docker.internal` pour un assistant dans
/// Docker Desktop, avec ou sans port) : un autre nom de domaine redirigé vers cette machine
/// (« DNS rebinding ») est refusé.
fn local_host(h: &HeaderMap) -> bool {
    let Some(host) = h.get("host").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let name = match host.strip_prefix('[') {
        Some(rest) => rest.split(']').next().unwrap_or_default(),
        None => host.split(':').next().unwrap_or_default(),
    };
    matches!(
        name,
        "127.0.0.1" | "localhost" | "::1" | "host.docker.internal"
    )
}

async fn guard(State(s): State<Shared>, req: Request<Body>, next: Next) -> Response {
    // Requête d'une page web (navigateur) : refusée, quel que soit le jeton.
    if req.headers().contains_key("origin") || !local_host(req.headers()) {
        return (StatusCode::FORBIDDEN, "origin").into_response();
    }
    if !authorized(req.headers(), &s.token) {
        return (
            StatusCode::UNAUTHORIZED,
            [("www-authenticate", "Bearer")],
            "token",
        )
            .into_response();
    }
    next.run(req).await
}

async fn invoke(State(s): State<Shared>, Path(cmd): Path<String>, body: String) -> Response {
    let args: Value = if body.trim().is_empty() {
        Value::Object(Default::default())
    } else {
        match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        }
    };
    match s.inv.invoke(&cmd, args).await {
        Ok(v) => axum::Json(v).into_response(),
        Err(f) => (StatusCode::UNPROCESSABLE_ENTITY, axum::Json(f)).into_response(),
    }
}

/// Outil appelé sans MCP (`POST /api/<outil>`, corps : arguments) : `{"result"}` ou
/// `{"error"}` (422).
async fn api(State(s): State<Shared>, Path(tool): Path<String>, body: String) -> Response {
    let args: Value = if body.trim().is_empty() {
        Value::Object(Default::default())
    } else {
        match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        }
    };
    match Revel::new(s.inv.clone()).run(&tool, args, "OpenAPI").await {
        None => (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({ "error": format!("No tool {tool}.") })),
        )
            .into_response(),
        Some(r) if r.is_error == Some(true) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            axum::Json(serde_json::json!({ "error": crate::tools::text_of(&r) })),
        )
            .into_response(),
        Some(r) => {
            axum::Json(serde_json::json!({ "result": crate::tools::text_of(&r) })).into_response()
        }
    }
}

/// Description OpenAPI des outils, adresse du serveur reprise de la requête.
async fn openapi(h: HeaderMap) -> Response {
    let host = h
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1");
    axum::Json(Revel::openapi(&format!("http://{host}"))).into_response()
}

/// Routes du serveur : `/mcp`, `/invoke/<commande>`, `/api/<outil>` et `/openapi.json`, jeton
/// exigé.
pub fn router(inv: Arc<dyn Invoke>, token: String, ct: CancellationToken) -> axum::Router {
    let factory = inv.clone();
    let config = StreamableHttpServerConfig::default()
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .with_cancellation_token(ct);
    let mcp: StreamableHttpService<Revel, LocalSessionManager> = StreamableHttpService::new(
        move || Ok(Revel::new(factory.clone())),
        Default::default(),
        config,
    );
    let shared = Shared {
        inv,
        token: Arc::new(token),
    };
    axum::Router::new()
        .nest_service("/mcp", mcp)
        .route("/invoke/{cmd}", axum::routing::post(invoke))
        .route("/api/{tool}", axum::routing::post(api))
        .route("/openapi.json", axum::routing::get(openapi))
        .layer(axum::middleware::from_fn_with_state(shared.clone(), guard))
        .with_state(shared)
}

/// Serveur en marche ; arrêté par [`Running::shutdown`] (ou à sa disparition, sans attendre).
pub struct Running {
    pub port: u16,
    ct: CancellationToken,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl Running {
    pub fn stop(&self) {
        self.ct.cancel();
    }

    /// Arrête le serveur et attend la fermeture du port (2 s au plus).
    pub async fn shutdown(mut self) {
        self.ct.cancel();
        if let Some(t) = self.task.take() {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), t).await;
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.ct.cancel();
    }
}

/// Démarre le serveur sur 127.0.0.1:`port` (0 : port libre choisi par le système).
pub async fn start(inv: Arc<dyn Invoke>, token: String, port: u16) -> std::io::Result<Running> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let port = listener.local_addr()?.port();
    let ct = CancellationToken::new();
    let app = router(inv, token, ct.child_token());
    let stop = ct.clone();
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move { stop.cancelled_owned().await })
            .await;
    });
    Ok(Running {
        port,
        ct,
        task: Some(task),
    })
}
