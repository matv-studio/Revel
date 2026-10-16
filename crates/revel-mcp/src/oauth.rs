//! Accès à distance (4.4, D-44) : Revel est son propre serveur d'autorisation OAuth 2.1 pour les
//! assistants en ligne (connecteurs de claude.ai, ChatGPT) qui joignent l'ordinateur par une
//! adresse publique (tunnel choisi par l'utilisateur).
//!
//! - Découverte : métadonnées de la ressource protégée (RFC 9728) et du serveur d'autorisation
//!   (RFC 8414) ; inscription dynamique des clients (RFC 7591), clients publics.
//! - Autorisation : code avec PKCE S256 obligatoire, `resource` (RFC 8707) lié au jeton, `iss`
//!   dans la réponse (RFC 9207). **Chaque demande est accordée ou refusée dans Revel**, sur
//!   l'ordinateur : la page ouverte dans le navigateur attend la décision.
//! - Jetons : accès 1 h (en mémoire), renouvellement tournant (empreintes enregistrées dans
//!   `mcp-oauth.json`), révocables par application connectée.

use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use base64::Engine;
use revel_protocol::{McpAuthRequest, McpConnection};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Digest;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Portée unique : lire et modifier le site ouvert dans Revel.
pub const SCOPE: &str = "revel";
/// Durée d'un jeton d'accès.
const ACCESS_TTL: Duration = Duration::from_secs(3600);
/// Durée d'un code d'autorisation.
const CODE_TTL: Duration = Duration::from_secs(120);
/// Durée d'une demande en attente de décision.
const PENDING_TTL: Duration = Duration::from_secs(600);
/// Demandes en attente au plus (une adresse publique peut être visitée par n'importe qui).
const MAX_PENDING: usize = 3;
/// Clients inscrits au plus (sans autorisation, les plus anciens sont oubliés).
const MAX_CLIENTS: usize = 50;

fn random() -> String {
    let mut b = [0u8; 32];
    let _ = getrandom::fill(&mut b);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}

fn digest(token: &str) -> String {
    blake3::hash(token.as_bytes()).to_hex().to_string()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

/// Adresse de base publique normalisée (`https://hôte[:port]`, sans chemin), ou `None`.
pub fn normalize_base(url: &str) -> Option<String> {
    let u = url.trim().trim_end_matches('/');
    let rest = u.strip_prefix("https://")?;
    let host = rest.split('/').next()?.to_lowercase();
    let valid = !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'));
    valid.then(|| format!("https://{host}"))
}

/// Adresse de redirection acceptée à l'inscription : https, ou boucle locale en http.
fn redirect_ok(uri: &str) -> bool {
    if uri.contains('#') {
        return false;
    }
    uri.starts_with("https://") || loopback(uri).is_some()
}

/// Adresse de boucle locale (`http://127.0.0.1:port/…`, `http://localhost:port/…`) : hôte et
/// chemin, port ignoré (RFC 8252).
fn loopback(uri: &str) -> Option<(String, String)> {
    let rest = uri.strip_prefix("http://")?;
    let (hostport, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = hostport
        .rsplit_once(':')
        .map(|(h, _)| h)
        .unwrap_or(hostport);
    matches!(host, "127.0.0.1" | "localhost" | "[::1]").then(|| (host.to_string(), path.into()))
}

/// Adresse de redirection demandée égale à une adresse inscrite (port libre en boucle locale).
fn redirect_matches(registered: &[String], asked: &str) -> bool {
    registered.iter().any(|r| {
        r == asked || matches!((loopback(r), loopback(asked)), (Some(a), Some(b)) if a == b)
    })
}

/// Hôte d'une adresse (pour l'affichage).
fn host_of(uri: &str) -> String {
    uri.split("://")
        .nth(1)
        .unwrap_or(uri)
        .split(['/', '?'])
        .next()
        .unwrap_or_default()
        .to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Client {
    id: String,
    name: String,
    redirect_uris: Vec<String>,
    created: u64,
}

/// Autorisation accordée : empreinte du jeton de renouvellement actuel.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Grant {
    client_id: String,
    refresh: String,
    resource: String,
    created: u64,
    used: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Stored {
    #[serde(default)]
    clients: Vec<Client>,
    #[serde(default)]
    grants: Vec<Grant>,
}

struct Pending {
    id: String,
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    challenge: String,
    resource: String,
    at: Instant,
    created: u64,
    /// Décision : adresse de retour (code ou refus).
    decision: Option<String>,
}

struct Code {
    hash: String,
    client_id: String,
    redirect_uri: String,
    challenge: String,
    resource: String,
    until: Instant,
}

struct Access {
    hash: String,
    client_id: String,
    resource: String,
    until: Instant,
}

#[derive(Default)]
struct St {
    stored: Stored,
    pending: Vec<Pending>,
    codes: Vec<Code>,
    access: Vec<Access>,
}

/// Serveur d'autorisation de Revel pour une adresse publique.
pub struct OAuth {
    base: String,
    file: PathBuf,
    st: Mutex<St>,
    /// Appelé à chaque nouvelle demande (l'application la présente à l'utilisateur).
    notify: Option<Box<dyn Fn() + Send + Sync>>,
}

impl OAuth {
    /// `base` : adresse publique normalisée ; réglages enregistrés dans `config_dir`.
    pub fn new(config_dir: &Path, base: &str, notify: Option<Box<dyn Fn() + Send + Sync>>) -> Self {
        let file = config_dir.join("mcp-oauth.json");
        let stored = std::fs::read_to_string(&file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Self {
            base: base.trim_end_matches('/').to_string(),
            file,
            st: Mutex::new(St {
                stored,
                ..Default::default()
            }),
            notify,
        }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    /// Adresse MCP publique (ressource protégée).
    pub fn resource(&self) -> String {
        format!("{}/mcp", self.base)
    }

    /// Hôte public (`abc.example.com`), accepté par le contrôle d'hôte.
    pub fn host(&self) -> String {
        host_of(&self.base)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, St> {
        self.st.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn save(&self, st: &St) {
        if let Ok(t) = serde_json::to_string_pretty(&st.stored) {
            let tmp = self.file.with_extension("json.tmp");
            if std::fs::write(&tmp, t).is_ok() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
                }
                let _ = std::fs::rename(&tmp, &self.file);
            }
        }
    }

    /// Ressource demandée acceptée : absente, l'adresse MCP publique ou la base.
    fn resource_ok(&self, asked: Option<&str>) -> Option<String> {
        let r = self.resource();
        match asked.map(|a| a.trim().trim_end_matches('/').to_string()) {
            None => Some(r),
            Some(a) if a.eq_ignore_ascii_case(&r) || a.eq_ignore_ascii_case(&self.base) => Some(r),
            Some(_) => None,
        }
    }

    fn purge(st: &mut St) {
        let now = Instant::now();
        st.codes.retain(|c| c.until > now);
        st.access.retain(|a| a.until > now);
        st.pending
            .retain(|p| now.duration_since(p.at) < PENDING_TTL);
    }

    /// Jeton d'accès valable pour l'adresse MCP publique : nom de l'application.
    pub fn check(&self, token: &str) -> Option<String> {
        let mut st = self.lock();
        Self::purge(&mut st);
        let h = digest(token);
        let r = self.resource();
        let a = st.access.iter().find(|a| a.hash == h && a.resource == r)?;
        let id = a.client_id.clone();
        let name = st
            .stored
            .clients
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        Some(name)
    }

    /// Demandes en attente de décision.
    pub fn pending(&self) -> Vec<McpAuthRequest> {
        let mut st = self.lock();
        Self::purge(&mut st);
        st.pending
            .iter()
            .filter(|p| p.decision.is_none())
            .map(|p| {
                let name = st
                    .stored
                    .clients
                    .iter()
                    .find(|c| c.id == p.client_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                McpAuthRequest {
                    id: p.id.clone(),
                    client: name,
                    host: host_of(&p.redirect_uri),
                    at: p.created * 1000,
                }
            })
            .collect()
    }

    /// Décision de l'utilisateur sur une demande.
    pub fn decide(&self, id: &str, allow: bool) -> bool {
        let mut st = self.lock();
        Self::purge(&mut st);
        let Some(i) = st
            .pending
            .iter()
            .position(|p| p.id == id && p.decision.is_none())
        else {
            return false;
        };
        let iss = urlencode(&self.base);
        let p = &st.pending[i];
        let state = p
            .state
            .as_ref()
            .map(|s| format!("&state={}", urlencode(s)))
            .unwrap_or_default();
        let sep = if p.redirect_uri.contains('?') {
            '&'
        } else {
            '?'
        };
        let url = if allow {
            let code = random();
            let c = Code {
                hash: digest(&code),
                client_id: p.client_id.clone(),
                redirect_uri: p.redirect_uri.clone(),
                challenge: p.challenge.clone(),
                resource: p.resource.clone(),
                until: Instant::now() + CODE_TTL,
            };
            let url = format!("{}{sep}code={code}{state}&iss={iss}", p.redirect_uri);
            st.codes.push(c);
            url
        } else {
            format!(
                "{}{sep}error=access_denied{state}&iss={iss}",
                p.redirect_uri
            )
        };
        st.pending[i].decision = Some(url);
        true
    }

    /// Applications connectées (autorisations accordées).
    pub fn connections(&self) -> Vec<McpConnection> {
        let st = self.lock();
        let mut out: Vec<McpConnection> = Vec::new();
        for g in &st.stored.grants {
            let Some(c) = st.stored.clients.iter().find(|c| c.id == g.client_id) else {
                continue;
            };
            if let Some(o) = out.iter_mut().find(|o| o.id == c.id) {
                o.used = o.used.max(g.used * 1000);
                continue;
            }
            out.push(McpConnection {
                id: c.id.clone(),
                client: c.name.clone(),
                host: c
                    .redirect_uris
                    .first()
                    .map(|u| host_of(u))
                    .unwrap_or_default(),
                since: g.created * 1000,
                used: g.used * 1000,
            });
        }
        out
    }

    /// Accès retiré à une application (jetons d'accès et de renouvellement).
    pub fn revoke(&self, client_id: &str) {
        let mut st = self.lock();
        st.stored.grants.retain(|g| g.client_id != client_id);
        st.access.retain(|a| a.client_id != client_id);
        st.codes.retain(|c| c.client_id != client_id);
        self.save(&st);
    }

    fn issue(
        &self,
        st: &mut St,
        client_id: &str,
        resource: &str,
        refresh_hash: Option<&str>,
    ) -> serde_json::Value {
        let access = random();
        let refresh = random();
        st.access.push(Access {
            hash: digest(&access),
            client_id: client_id.into(),
            resource: resource.into(),
            until: Instant::now() + ACCESS_TTL,
        });
        let now = now_secs();
        match refresh_hash.and_then(|h| st.stored.grants.iter_mut().find(|g| g.refresh == h)) {
            // Renouvellement : l'ancien jeton cesse de valoir (rotation).
            Some(g) => {
                g.refresh = digest(&refresh);
                g.used = now;
            }
            None => st.stored.grants.push(Grant {
                client_id: client_id.into(),
                refresh: digest(&refresh),
                resource: resource.into(),
                created: now,
                used: now,
            }),
        }
        self.save(st);
        json!({
            "access_token": access,
            "token_type": "Bearer",
            "expires_in": ACCESS_TTL.as_secs(),
            "refresh_token": refresh,
            "scope": SCOPE,
        })
    }
}

/// Encodage d'une valeur dans une adresse.
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn oauth_error(status: StatusCode, error: &str, description: &str) -> Response {
    (
        status,
        [("cache-control", "no-store")],
        axum::Json(json!({ "error": error, "error_description": description })),
    )
        .into_response()
}

/// Métadonnées de la ressource protégée (RFC 9728).
pub async fn protected_resource(State(o): State<Arc<OAuth>>) -> Response {
    axum::Json(json!({
        "resource": o.resource(),
        "authorization_servers": [o.base()],
        "scopes_supported": [SCOPE],
        "bearer_methods_supported": ["header"],
        "resource_name": "Revel",
    }))
    .into_response()
}

/// Métadonnées du serveur d'autorisation (RFC 8414).
pub async fn authorization_server(State(o): State<Arc<OAuth>>) -> Response {
    let b = o.base();
    axum::Json(json!({
        "issuer": b,
        "authorization_endpoint": format!("{b}/oauth/authorize"),
        "token_endpoint": format!("{b}/oauth/token"),
        "registration_endpoint": format!("{b}/oauth/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": [SCOPE, "offline_access"],
        "authorization_response_iss_parameter_supported": true,
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct Registration {
    #[serde(default)]
    redirect_uris: Vec<String>,
    #[serde(default)]
    client_name: Option<String>,
}

/// Inscription dynamique d'un client public (RFC 7591).
pub async fn register(State(o): State<Arc<OAuth>>, body: String) -> Response {
    let Ok(r) = serde_json::from_str::<Registration>(&body) else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_client_metadata",
            "JSON expected",
        );
    };
    if r.redirect_uris.is_empty() || !r.redirect_uris.iter().all(|u| redirect_ok(u)) {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "https or loopback redirect URIs required",
        );
    }
    let name: String = r
        .client_name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| host_of(&r.redirect_uris[0]))
        .chars()
        .filter(|c| !c.is_control())
        .take(60)
        .collect();
    let client = Client {
        id: random(),
        name,
        redirect_uris: r.redirect_uris,
        created: now_secs(),
    };
    let mut st = o.lock();
    // Clients sans autorisation : les plus anciens oubliés au-delà de la limite.
    while st.stored.clients.len() >= MAX_CLIENTS {
        let unused = st
            .stored
            .clients
            .iter()
            .position(|c| !st.stored.grants.iter().any(|g| g.client_id == c.id));
        match unused {
            Some(i) => {
                st.stored.clients.remove(i);
            }
            None => break,
        }
    }
    st.stored.clients.push(client.clone());
    o.save(&st);
    (
        StatusCode::CREATED,
        axum::Json(json!({
            "client_id": client.id,
            "client_name": client.name,
            "redirect_uris": client.redirect_uris,
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none",
            "client_id_issued_at": client.created,
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct AuthorizeQuery {
    response_type: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    state: Option<String>,
    resource: Option<String>,
}

/// Langue de la page d'attente (en-tête `Accept-Language`).
fn page_lang(h: &HeaderMap) -> &'static str {
    let l = h
        .get("accept-language")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_lowercase();
    if l.starts_with("fr") { "fr" } else { "en" }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Page affichée dans le navigateur pendant que l'utilisateur décide dans Revel.
fn page(lang: &str, title: &str, text: &str, poll: Option<&str>) -> Response {
    let script = poll
        .map(|id| {
            format!(
                "<script>(async function w(){{try{{const r=await fetch('/oauth/authorize/status?id={id}',{{cache:'no-store'}});const j=await r.json();if(j.redirect){{location.replace(j.redirect);return}}}}catch(e){{}}setTimeout(w,1000)}})()</script>"
            )
        })
        .unwrap_or_default();
    Html(format!(
        "<!doctype html><html lang=\"{lang}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>Revel</title>\
<style>body{{margin:0;min-height:100vh;display:grid;place-items:center;font:15px/1.5 system-ui,sans-serif;background:#111;color:#eee}}\
main{{max-width:380px;padding:32px;text-align:center}}h1{{font-size:18px;font-weight:600;margin:0 0 8px}}p{{margin:0;color:#aaa}}\
@media (prefers-color-scheme:light){{body{{background:#f6f6f6;color:#111}}p{{color:#555}}}}</style>\
<main><h1>{}</h1><p>{}</p></main>{script}</html>",
        escape(title),
        escape(text)
    ))
    .into_response()
}

/// Demande d'autorisation : vérifiée, puis présentée dans Revel ; la page attend la décision.
pub async fn authorize(
    State(o): State<Arc<OAuth>>,
    h: HeaderMap,
    Query(q): Query<AuthorizeQuery>,
) -> Response {
    let lang = page_lang(&h);
    let l = revel_i18n::Localizer::new(lang);
    let bad = |key: &str| {
        let r = page(lang, &l.t("mcp-auth-page-title", &[]), &l.t(key, &[]), None);
        (StatusCode::BAD_REQUEST, r).into_response()
    };
    let (Some(client_id), Some(redirect_uri)) = (q.client_id, q.redirect_uri) else {
        return bad("mcp-auth-page-invalid");
    };
    let mut st = o.lock();
    OAuth::purge(&mut st);
    let Some(client) = st
        .stored
        .clients
        .iter()
        .find(|c| c.id == client_id)
        .cloned()
    else {
        return bad("mcp-auth-page-invalid");
    };
    // Adresse de retour non inscrite : aucune redirection (RFC 6749, 4.1.2.1).
    if !redirect_matches(&client.redirect_uris, &redirect_uri) {
        return bad("mcp-auth-page-invalid");
    }
    let sep = if redirect_uri.contains('?') { '&' } else { '?' };
    let state_part = q
        .state
        .as_ref()
        .map(|s| format!("&state={}", urlencode(s)))
        .unwrap_or_default();
    let fail = |error: &str| {
        axum::response::Redirect::to(&format!(
            "{redirect_uri}{sep}error={error}{state_part}&iss={}",
            urlencode(o.base())
        ))
        .into_response()
    };
    if q.response_type.as_deref() != Some("code") {
        return fail("unsupported_response_type");
    }
    let Some(challenge) = q.code_challenge.filter(|c| (43..=128).contains(&c.len())) else {
        return fail("invalid_request");
    };
    if q.code_challenge_method.as_deref() != Some("S256") {
        return fail("invalid_request");
    }
    let Some(resource) = o.resource_ok(q.resource.as_deref()) else {
        return fail("invalid_target");
    };
    if st.pending.iter().filter(|p| p.decision.is_none()).count() >= MAX_PENDING {
        let r = page(
            lang,
            &l.t("mcp-auth-page-title", &[]),
            &l.t("mcp-auth-page-busy", &[]),
            None,
        );
        return (StatusCode::TOO_MANY_REQUESTS, r).into_response();
    }
    let id = random();
    st.pending.push(Pending {
        id: id.clone(),
        client_id,
        redirect_uri,
        state: q.state,
        challenge,
        resource,
        at: Instant::now(),
        created: now_secs(),
        decision: None,
    });
    drop(st);
    if let Some(n) = &o.notify {
        n();
    }
    page(
        lang,
        &l.t("mcp-auth-page-title", &[]),
        &l.t("mcp-auth-page-wait", &[("client", client.name.as_str())]),
        Some(&id),
    )
}

#[derive(Deserialize)]
pub struct StatusQuery {
    id: String,
}

/// État d'une demande : adresse de retour une fois décidée.
pub async fn authorize_status(
    State(o): State<Arc<OAuth>>,
    Query(q): Query<StatusQuery>,
) -> Response {
    let mut st = o.lock();
    OAuth::purge(&mut st);
    let Some(i) = st.pending.iter().position(|p| p.id == q.id) else {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(json!({ "state": "gone" })),
        )
            .into_response();
    };
    match st.pending[i].decision.clone() {
        Some(url) => {
            st.pending.remove(i);
            axum::Json(json!({ "state": "done", "redirect": url })).into_response()
        }
        None => axum::Json(json!({ "state": "pending" })).into_response(),
    }
}

#[derive(Deserialize)]
pub struct TokenForm {
    grant_type: String,
    code: Option<String>,
    redirect_uri: Option<String>,
    client_id: Option<String>,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
    resource: Option<String>,
}

/// Échange d'un code (PKCE vérifié) ou renouvellement (rotation).
pub async fn token(State(o): State<Arc<OAuth>>, Form(f): Form<TokenForm>) -> Response {
    let mut st = o.lock();
    OAuth::purge(&mut st);
    match f.grant_type.as_str() {
        "authorization_code" => {
            let (Some(code), Some(verifier), Some(client_id)) =
                (f.code, f.code_verifier, f.client_id)
            else {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                    "code, code_verifier and client_id required",
                );
            };
            let h = digest(&code);
            // Code à usage unique : retiré dès sa présentation.
            let Some(i) = st.codes.iter().position(|c| c.hash == h) else {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "unknown or expired code",
                );
            };
            let c = st.codes.remove(i);
            let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(sha2::Sha256::digest(verifier.as_bytes()));
            let resource_ok = f
                .resource
                .as_deref()
                .is_none_or(|r| o.resource_ok(Some(r)).as_deref() == Some(c.resource.as_str()));
            if c.client_id != client_id
                || f.redirect_uri.as_deref() != Some(c.redirect_uri.as_str())
                || challenge != c.challenge
                || !resource_ok
            {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "code does not match",
                );
            }
            let body = o.issue(&mut st, &c.client_id, &c.resource, None);
            (
                StatusCode::OK,
                [("cache-control", "no-store")],
                axum::Json(body),
            )
                .into_response()
        }
        "refresh_token" => {
            let Some(rt) = f.refresh_token else {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                    "refresh_token required",
                );
            };
            let h = digest(&rt);
            let Some(g) = st.stored.grants.iter().find(|g| g.refresh == h).cloned() else {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "unknown refresh token",
                );
            };
            if f.client_id.as_deref().is_some_and(|c| c != g.client_id) {
                return oauth_error(StatusCode::BAD_REQUEST, "invalid_grant", "client mismatch");
            }
            let body = o.issue(&mut st, &g.client_id, &g.resource, Some(&h));
            (
                StatusCode::OK,
                [("cache-control", "no-store")],
                axum::Json(body),
            )
                .into_response()
        }
        _ => oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            "authorization_code or refresh_token",
        ),
    }
}

/// Routes publiques du serveur d'autorisation (sans clé).
pub fn router(o: Arc<OAuth>) -> axum::Router {
    use axum::routing::{get, post};
    axum::Router::new()
        .route(
            "/.well-known/oauth-protected-resource",
            get(protected_resource),
        )
        .route(
            "/.well-known/oauth-protected-resource/mcp",
            get(protected_resource),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(authorization_server),
        )
        .route("/oauth/register", post(register))
        .route("/oauth/authorize", get(authorize))
        .route("/oauth/authorize/status", get(authorize_status))
        .route("/oauth/token", post(token))
        .with_state(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresses() {
        assert_eq!(
            normalize_base("https://Abc.trycloudflare.com/"),
            Some("https://abc.trycloudflare.com".into())
        );
        assert_eq!(normalize_base("http://abc.example.com"), None);
        assert_eq!(normalize_base("https://"), None);
        assert!(redirect_ok("https://claude.ai/api/mcp/auth_callback"));
        assert!(redirect_ok("http://localhost:3118/callback"));
        assert!(!redirect_ok("http://evil.example.com/cb"));
        let reg = vec!["http://localhost/callback".to_string()];
        assert!(redirect_matches(&reg, "http://localhost:51234/callback"));
        assert!(!redirect_matches(&reg, "http://localhost:51234/other"));
        assert!(!redirect_matches(
            &["https://claude.ai/api/mcp/auth_callback".to_string()],
            "https://claude.ai/api/mcp/auth_callback2"
        ));
    }
}
