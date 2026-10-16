//! Serveur HTTP local des assistants : MCP en HTTP en flux (`/mcp`, protocoles 2025 et
//! 2026-07-28), mêmes outils en HTTP simple décrits en OpenAPI (`POST /api/<outil>`,
//! `GET /openapi.json`, 4.3) et commandes du relais stdio (`POST /invoke/<commande>`).
//! N'écoute que sur 127.0.0.1 ; chaque requête doit porter le jeton (`Authorization: Bearer …`) ;
//! les en-têtes `Host` et `Origin` sont vérifiés (pas de requête venue d'une page web).

use crate::oauth::OAuth;
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
    /// Accès à distance (adresse publique, OAuth), s'il est réglé.
    oauth: Option<Arc<OAuth>>,
}

/// Jeton présenté (`Authorization: Bearer …`).
fn bearer(h: &HeaderMap) -> Option<&str> {
    h.get("authorization")
        .and_then(|v| v.to_str().ok())?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

/// Comparaison en temps constant.
fn same(given: &str, token: &str) -> bool {
    given.len() == token.len()
        && given
            .bytes()
            .zip(token.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

/// Nom d'hôte de l'en-tête `Host`, sans port.
fn host_name(h: &HeaderMap) -> Option<String> {
    let host = h.get("host").and_then(|v| v.to_str().ok())?;
    Some(
        match host.strip_prefix('[') {
            Some(rest) => rest.split(']').next().unwrap_or_default(),
            None => host.split(':').next().unwrap_or_default(),
        }
        .to_lowercase(),
    )
}

/// Hôte accepté : local (`127.0.0.1`, `localhost`, `[::1]`, `host.docker.internal` pour un
/// assistant dans Docker Desktop) ou l'adresse publique réglée. Un autre nom de domaine redirigé
/// vers cette machine (« DNS rebinding ») est refusé.
fn host_ok(h: &HeaderMap, oauth: Option<&OAuth>) -> bool {
    let Some(name) = host_name(h) else {
        return false;
    };
    matches!(
        name.as_str(),
        "127.0.0.1" | "localhost" | "::1" | "host.docker.internal"
    ) || oauth.is_some_and(|o| o.host().split(':').next() == Some(name.as_str()))
}

/// Hôte et origine : toutes les routes. Seule la page d'autorisation, ouverte dans le navigateur
/// à l'adresse publique, peut porter son propre en-tête `Origin`.
async fn guard_host(State(s): State<Shared>, req: Request<Body>, next: Next) -> Response {
    let h = req.headers();
    let origin_ok = match h.get("origin").and_then(|v| v.to_str().ok()) {
        None => true,
        Some(o) => {
            req.uri().path().starts_with("/oauth/authorize")
                && s.oauth
                    .as_ref()
                    .is_some_and(|a| a.base() == o.trim_end_matches('/'))
        }
    };
    if !origin_ok || !host_ok(h, s.oauth.as_deref()) {
        return (StatusCode::FORBIDDEN, "origin").into_response();
    }
    next.run(req).await
}

/// Clé locale ou jeton d'accès OAuth : routes des outils.
async fn guard(State(s): State<Shared>, req: Request<Body>, next: Next) -> Response {
    let ok = bearer(req.headers()).is_some_and(|t| {
        same(t, &s.token) || s.oauth.as_ref().is_some_and(|o| o.check(t).is_some())
    });
    if !ok {
        // Accès à distance : le client trouve le serveur d'autorisation (RFC 9728).
        let challenge = match &s.oauth {
            Some(o) => format!(
                "Bearer resource_metadata=\"{}/.well-known/oauth-protected-resource\", scope=\"{}\"",
                o.base(),
                crate::oauth::SCOPE
            ),
            None => "Bearer".to_string(),
        };
        return (
            StatusCode::UNAUTHORIZED,
            [("www-authenticate", challenge)],
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

/// Routes du serveur : `/mcp`, `/invoke/<commande>`, `/api/<outil>` et `/openapi.json` (clé ou
/// jeton exigé) ; avec l'accès à distance, découverte et routes OAuth (sans clé).
pub fn router(
    inv: Arc<dyn Invoke>,
    token: String,
    oauth: Option<Arc<OAuth>>,
    ct: CancellationToken,
) -> axum::Router {
    let factory = inv.clone();
    // Hôtes acceptés par le service MCP : les mêmes que le contrôle d'hôte de Revel.
    let mut hosts = vec![
        "localhost".to_string(),
        "127.0.0.1".into(),
        "::1".into(),
        "host.docker.internal".into(),
    ];
    if let Some(o) = &oauth {
        hosts.push(o.host());
    }
    let config = StreamableHttpServerConfig::default()
        .with_allowed_hosts(hosts)
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
        oauth: oauth.clone(),
    };
    let mut app = axum::Router::new()
        .nest_service("/mcp", mcp)
        .route("/invoke/{cmd}", axum::routing::post(invoke))
        .route("/api/{tool}", axum::routing::post(api))
        .route("/openapi.json", axum::routing::get(openapi))
        .layer(axum::middleware::from_fn_with_state(shared.clone(), guard))
        .with_state(shared.clone());
    if let Some(o) = oauth {
        app = app.merge(crate::oauth::router(o));
    }
    app.layer(axum::middleware::from_fn_with_state(shared, guard_host))
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
    start_with(inv, token, port, None).await
}

/// Comme [`start`], avec l'accès à distance (adresse publique, OAuth).
pub async fn start_with(
    inv: Arc<dyn Invoke>,
    token: String,
    port: u16,
    oauth: Option<Arc<OAuth>>,
) -> std::io::Result<Running> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let port = listener.local_addr()?.port();
    let ct = CancellationToken::new();
    let app = router(inv, token, oauth, ct.child_token());
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
