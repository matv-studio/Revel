//! Connexion à GitHub (autorisation par code, « device flow ») et API utile à la publication
//! (compte, dépôt, GitHub Pages).
//!
//! Références (consultées le 13/10/2026) :
//! - <https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow>
//! - <https://docs.github.com/en/rest/pages/pages>
//! - <https://docs.github.com/en/rest/about-the-rest-api/api-versions>

use crate::GitError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Identifiant public de l'application OAuth « Revel » (compte matv-studio, device flow activé).
/// Ce n'est pas un secret : le device flow n'utilise pas de secret client.
pub const CLIENT_ID: &str = "Ov23liuo9dnJsp95C633";

/// Droits demandés : `repo` (envoi sur les dépôts publics et privés, réglages GitHub Pages).
pub const SCOPES: &str = "repo";

const API: &str = "https://api.github.com";
const API_VERSION: &str = "2026-03-10";
const LOGIN: &str = "https://github.com/login";

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .user_agent(concat!("Revel/", env!("CARGO_PKG_VERSION")))
        .timeout_global(Some(Duration::from_secs(60)))
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

fn http(e: ureq::Error) -> GitError {
    GitError::Http(e.to_string())
}

/// Erreur d'API : le champ `message` de la réponse si elle en a un, sinon le corps.
fn api_error(status: u16, body: &str) -> GitError {
    let msg = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("message")
                .and_then(|m| m.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| body.chars().take(300).collect());
    GitError::Api(status, msg)
}

/// Codes affichés à l'utilisateur pour autoriser Revel.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    /// Code à saisir sur `verification_uri` (8 caractères, tiret au milieu).
    pub user_code: String,
    pub verification_uri: String,
    /// Durée de validité (secondes).
    pub expires_in: u64,
    /// Délai minimal entre deux demandes de jeton (secondes).
    pub interval: u64,
}

/// Jeton d'accès, conservé dans le trousseau du système.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Expiration du jeton d'accès (secondes depuis 1970) ; `None` : sans expiration.
    #[serde(default)]
    pub expires_at: Option<u64>,
    #[serde(default)]
    pub refresh_expires_at: Option<u64>,
    #[serde(default)]
    pub scope: String,
}

impl Token {
    /// Le jeton d'accès expire dans moins de cinq minutes.
    pub fn needs_refresh(&self, now: u64) -> bool {
        self.expires_at.is_some_and(|t| now + 300 >= t) && self.refresh_token.is_some()
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    refresh_token_expires_in: Option<u64>,
    #[serde(default)]
    scope: String,
    error: Option<String>,
    interval: Option<u64>,
}

impl TokenResponse {
    fn into_token(self, now: u64) -> Option<Token> {
        Some(Token {
            access_token: self.access_token?,
            refresh_token: self.refresh_token,
            expires_at: self.expires_in.map(|s| now + s),
            refresh_expires_at: self.refresh_token_expires_in.map(|s| now + s),
            scope: self.scope,
        })
    }
}

/// Résultat d'une demande de jeton pendant l'autorisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Poll {
    Granted(Token),
    /// L'utilisateur n'a pas encore saisi le code.
    Pending,
    /// Demandes trop rapprochées : nouveau délai (secondes).
    SlowDown(u64),
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Étape 1 : codes d'autorisation.
pub fn request_device_code() -> Result<DeviceCode, GitError> {
    let mut res = agent()
        .post(format!("{LOGIN}/device/code"))
        .header("Accept", "application/json")
        .send_json(serde_json::json!({ "client_id": CLIENT_ID, "scope": SCOPES }))
        .map_err(http)?;
    let status = res.status().as_u16();
    let body = res.body_mut().read_to_string().map_err(http)?;
    if status != 200 {
        return Err(api_error(status, &body));
    }
    if let Some(e) = oauth_error(&body) {
        return Err(e);
    }
    serde_json::from_str(&body).map_err(|e| GitError::Api(status, e.to_string()))
}

/// Erreur OAuth contenue dans une réponse (`{"error": "..."}`), hors attente normale.
fn oauth_error(body: &str) -> Option<GitError> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let e = v.get("error")?.as_str()?;
    Some(match e {
        "expired_token" | "token_expired" => GitError::CodeExpired,
        "access_denied" => GitError::Denied,
        "device_flow_disabled" | "incorrect_client_credentials" => {
            GitError::AppConfig(e.to_string())
        }
        "bad_refresh_token" => GitError::SignedOut,
        _ => GitError::Api(200, e.to_string()),
    })
}

/// Étape 3 : demande du jeton (à répéter toutes les `interval` secondes).
pub fn poll_token(device_code: &str) -> Result<Poll, GitError> {
    let mut res = agent()
        .post(format!("{LOGIN}/oauth/access_token"))
        .header("Accept", "application/json")
        .send_json(serde_json::json!({
            "client_id": CLIENT_ID,
            "device_code": device_code,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
        }))
        .map_err(http)?;
    let body = res.body_mut().read_to_string().map_err(http)?;
    parse_poll(&body, now())
}

fn parse_poll(body: &str, now: u64) -> Result<Poll, GitError> {
    let r: TokenResponse =
        serde_json::from_str(body).map_err(|e| GitError::Api(200, e.to_string()))?;
    match r.error.as_deref() {
        Some("authorization_pending") => Ok(Poll::Pending),
        Some("slow_down") => Ok(Poll::SlowDown(r.interval.unwrap_or(10))),
        Some(_) => Err(oauth_error(body).unwrap_or(GitError::Api(200, body.to_string()))),
        None => r
            .into_token(now)
            .map(Poll::Granted)
            .ok_or_else(|| GitError::Api(200, body.to_string())),
    }
}

/// Renouvelle un jeton expiré (le jeton de renouvellement est remplacé).
pub fn refresh(token: &Token) -> Result<Token, GitError> {
    let Some(rt) = token.refresh_token.as_deref() else {
        return Err(GitError::SignedOut);
    };
    let mut res = agent()
        .post(format!("{LOGIN}/oauth/access_token"))
        .header("Accept", "application/json")
        .send_json(serde_json::json!({
            "client_id": CLIENT_ID,
            "grant_type": "refresh_token",
            "refresh_token": rt,
        }))
        .map_err(http)?;
    let body = res.body_mut().read_to_string().map_err(http)?;
    if let Some(e) = oauth_error(&body) {
        return Err(e);
    }
    let r: TokenResponse =
        serde_json::from_str(&body).map_err(|e| GitError::Api(200, e.to_string()))?;
    r.into_token(now()).ok_or(GitError::SignedOut)
}

/// Compte GitHub connecté.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct User {
    pub login: String,
    pub id: u64,
    #[serde(default)]
    pub name: Option<String>,
}

impl User {
    /// Adresse privée fournie par GitHub, pour signer les commits sans exposer d'adresse.
    pub fn noreply_email(&self) -> String {
        format!("{}+{}@users.noreply.github.com", self.id, self.login)
    }
}

/// Dépôt : informations utiles à la publication.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Repo {
    pub full_name: String,
    pub private: bool,
    pub html_url: String,
    pub clone_url: String,
    #[serde(default)]
    pub permissions: Option<Permissions>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Permissions {
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub push: bool,
}

/// Site GitHub Pages d'un dépôt.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Pages {
    #[serde(default)]
    pub html_url: Option<String>,
    #[serde(default)]
    pub cname: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub source: Option<PagesSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PagesSource {
    pub branch: String,
    pub path: String,
}

/// Client de l'API REST, avec un jeton d'accès.
pub struct Client {
    agent: ureq::Agent,
    token: String,
}

impl Client {
    pub fn new(access_token: &str) -> Self {
        Self {
            agent: agent(),
            token: access_token.to_string(),
        }
    }

    /// En-têtes communs : format, jeton, version de l'API.
    fn req<B>(&self, r: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        r.header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("X-GitHub-Api-Version", API_VERSION)
    }

    /// Réponse : statut et corps.
    fn read(
        res: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ) -> Result<(u16, String), GitError> {
        let mut res = res.map_err(http)?;
        let status = res.status().as_u16();
        let body = res.body_mut().read_to_string().map_err(http)?;
        if status == 401 {
            return Err(GitError::SignedOut);
        }
        Ok((status, body))
    }

    fn json<T: serde::de::DeserializeOwned>(status: u16, body: &str) -> Result<T, GitError> {
        serde_json::from_str(body).map_err(|e| GitError::Api(status, e.to_string()))
    }

    pub fn user(&self) -> Result<User, GitError> {
        let (s, b) = Self::read(self.req(self.agent.get(format!("{API}/user"))).call())?;
        match s {
            200 => Self::json(s, &b),
            _ => Err(api_error(s, &b)),
        }
    }

    /// Dépôt `proprietaire/nom` ; `None` s'il n'existe pas ou n'est pas accessible.
    pub fn repo(&self, full_name: &str) -> Result<Option<Repo>, GitError> {
        let (s, b) = Self::read(
            self.req(self.agent.get(format!("{API}/repos/{full_name}")))
                .call(),
        )?;
        match s {
            200 => Self::json(s, &b).map(Some),
            404 => Ok(None),
            _ => Err(api_error(s, &b)),
        }
    }

    /// Site GitHub Pages ; `None` s'il n'est pas activé.
    pub fn pages(&self, full_name: &str) -> Result<Option<Pages>, GitError> {
        let (s, b) = Self::read(
            self.req(self.agent.get(format!("{API}/repos/{full_name}/pages")))
                .call(),
        )?;
        match s {
            200 => Self::json(s, &b).map(Some),
            404 => Ok(None),
            _ => Err(api_error(s, &b)),
        }
    }

    /// Active GitHub Pages depuis une branche (racine).
    pub fn enable_pages(&self, full_name: &str, branch: &str) -> Result<Pages, GitError> {
        let body = serde_json::json!({
            "build_type": "legacy",
            "source": { "branch": branch, "path": "/" },
        });
        let (s, b) = Self::read(
            self.req(self.agent.post(format!("{API}/repos/{full_name}/pages")))
                .send_json(&body),
        )?;
        match s {
            201 => Self::json(s, &b),
            // Déjà activé (requête concurrente) : relecture.
            409 => self.pages(full_name)?.ok_or_else(|| api_error(s, &b)),
            _ => Err(api_error(s, &b)),
        }
    }

    /// Met à jour la source et le domaine personnalisé.
    pub fn update_pages(
        &self,
        full_name: &str,
        branch: &str,
        cname: Option<&str>,
    ) -> Result<(), GitError> {
        let body = serde_json::json!({
            "cname": cname,
            "source": { "branch": branch, "path": "/" },
        });
        let (s, b) = Self::read(
            self.req(self.agent.put(format!("{API}/repos/{full_name}/pages")))
                .send_json(&body),
        )?;
        match s {
            204 | 200 => Ok(()),
            _ => Err(api_error(s, &b)),
        }
    }
}

/// Adresse publique attendue d'un site GitHub Pages (sans domaine personnalisé).
pub fn default_pages_url(owner: &str, repo: &str) -> String {
    let owner = owner.to_ascii_lowercase();
    if repo.eq_ignore_ascii_case(&format!("{owner}.github.io")) {
        format!("https://{owner}.github.io/")
    } else {
        format!("https://{owner}.github.io/{repo}/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reponses_du_device_flow() {
        assert_eq!(
            parse_poll(r#"{"error":"authorization_pending"}"#, 0).unwrap(),
            Poll::Pending
        );
        assert_eq!(
            parse_poll(r#"{"error":"slow_down","interval":10}"#, 0).unwrap(),
            Poll::SlowDown(10)
        );
        assert!(matches!(
            parse_poll(r#"{"error":"expired_token"}"#, 0),
            Err(GitError::CodeExpired)
        ));
        assert!(matches!(
            parse_poll(r#"{"error":"access_denied"}"#, 0),
            Err(GitError::Denied)
        ));
        assert!(matches!(
            parse_poll(r#"{"error":"device_flow_disabled"}"#, 0),
            Err(GitError::AppConfig(_))
        ));
        let ok = r#"{"access_token":"ghu_x","token_type":"bearer","scope":"repo",
            "expires_in":28800,"refresh_token":"ghr_y","refresh_token_expires_in":15897600}"#;
        let Poll::Granted(t) = parse_poll(ok, 1000).unwrap() else {
            panic!("jeton attendu")
        };
        assert_eq!(t.expires_at, Some(29800));
        assert!(!t.needs_refresh(1000));
        assert!(t.needs_refresh(29600));
    }

    #[test]
    fn adresse_github_pages() {
        assert_eq!(
            default_pages_url("Matv-Studio", "alunites-revel-lab"),
            "https://matv-studio.github.io/alunites-revel-lab/"
        );
        assert_eq!(
            default_pages_url("matv-studio", "matv-studio.github.io"),
            "https://matv-studio.github.io/"
        );
    }

    #[test]
    fn adresse_privee_de_commit() {
        let u = User {
            login: "matv-studio".into(),
            id: 285979032,
            name: None,
        };
        assert_eq!(
            u.noreply_email(),
            "285979032+matv-studio@users.noreply.github.com"
        );
    }

    /// Demande réelle d'un code (réseau requis ; vérifie que le device flow est activé pour
    /// l'application) : `cargo test -p revel-git -- --ignored`.
    #[test]
    #[ignore]
    fn demande_de_code_reelle() {
        let c = request_device_code().unwrap();
        assert_eq!(c.user_code.len(), 9, "{c:?}");
        assert!(c.verification_uri.starts_with("https://github.com/"));
        assert_eq!(poll_token(&c.device_code).unwrap(), Poll::Pending);
    }
}
