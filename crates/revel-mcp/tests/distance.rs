//! Accès à distance (4.4) : un assistant en ligne découvre le serveur d'autorisation, s'inscrit,
//! demande l'accès (accordé dans Revel), échange le code (PKCE), appelle les outils, renouvelle
//! son jeton ; refus, révocation, hôtes et origines contrôlés.

use base64::Engine;
use revel_core::{Core, secrets};
use revel_mcp::oauth::OAuth;
use revel_mcp::{Invoke, Reply, http};
use revel_protocol::Failure;
use serde_json::{Value, json};
use sha2::Digest;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CoreInvoke(Arc<Core>);

impl Invoke for CoreInvoke {
    fn invoke(&self, cmd: &str, args: Value) -> Reply {
        let core = self.0.clone();
        let cmd = cmd.to_string();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || core.mcp_invoke(&cmd, &args))
                .await
                .map_err(|e| Failure::detail(e.to_string()))?
        })
    }
}

const BASE: &str = "https://revel.example.com";
const CALLBACK: &str = "https://claude.ai/api/mcp/auth_callback";

struct Http {
    root: String,
    agent: ureq::Agent,
}

impl Http {
    fn send(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: Option<(&str, String)>,
    ) -> (u16, String, Option<String>) {
        let url = format!("{}{path}", self.root);
        let mut res = match (method, body) {
            ("GET", _) => {
                let mut r = self.agent.get(&url).header("Host", "revel.example.com");
                for (k, v) in headers {
                    r = r.header(*k, *v);
                }
                r.call().unwrap()
            }
            (_, b) => {
                let mut r = self.agent.post(&url).header("Host", "revel.example.com");
                for (k, v) in headers {
                    r = r.header(*k, *v);
                }
                let (ct, body) = b.unwrap_or(("application/json", String::new()));
                r.header("Content-Type", ct).send(body).unwrap()
            }
        };
        let www = res
            .headers()
            .get("www-authenticate")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let status = res.status().as_u16();
        (
            status,
            res.body_mut().read_to_string().unwrap_or_default(),
            www,
        )
    }

    fn json(&self, method: &str, path: &str, body: Option<(&str, String)>) -> (u16, Value) {
        let (s, t, _) = self.send(method, path, &[], body);
        (s, serde_json::from_str(&t).unwrap_or(Value::Null))
    }
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

fn query(url: &str, key: &str) -> Option<String> {
    url.split(['?', '&'])
        .find_map(|kv| kv.strip_prefix(&format!("{key}=")))
        .map(|v| v.replace("%3A", ":").replace("%2F", "/"))
}

#[tokio::test(flavor = "multi_thread")]
async fn assistant_en_ligne() {
    let dir = tempfile::tempdir().unwrap();
    let core = Arc::new(Core::new(
        &dir.path().join("config"),
        Box::new(secrets::Memory::default()),
    ));
    let site = dir.path().join("atelier");
    core.new_site_from(
        &site,
        "Atelier",
        "fr",
        revel_protocol::SiteTemplate::Showcase,
    )
    .unwrap();
    core.edit_open(&site).unwrap();
    let asked = Arc::new(AtomicUsize::new(0));
    let a2 = asked.clone();
    let oauth = Arc::new(OAuth::new(
        dir.path(),
        BASE,
        Some(Box::new(move || {
            a2.fetch_add(1, Ordering::SeqCst);
        })),
    ));
    let inv: Arc<dyn Invoke> = Arc::new(CoreInvoke(core.clone()));
    let server = http::start_with(
        inv,
        "cle-locale-0123456789abcdef0123456789".into(),
        0,
        Some(oauth.clone()),
    )
    .await
    .unwrap();
    let root = format!("http://127.0.0.1:{}", server.port);
    let o = oauth.clone();
    tokio::task::spawn_blocking(move || {
        let h = Http {
            root,
            agent: ureq::Agent::config_builder()
                .http_status_as_error(false)
                .max_redirects(0)
                .build()
                .into(),
        };
        // Sans jeton : 401 et adresse des métadonnées.
        let (s, _, www) = h.send("POST", "/mcp", &[], Some(("application/json", "{}".into())));
        assert_eq!(s, 401);
        let www = www.unwrap();
        assert!(www.contains(&format!("resource_metadata=\"{BASE}/.well-known/oauth-protected-resource\"")), "{www}");
        // Découverte.
        let (s, prm) = h.json("GET", "/.well-known/oauth-protected-resource", None);
        assert_eq!(s, 200);
        assert_eq!(prm["resource"], format!("{BASE}/mcp"));
        assert_eq!(prm["authorization_servers"][0], BASE);
        let (_, asm) = h.json("GET", "/.well-known/oauth-authorization-server", None);
        assert_eq!(asm["issuer"], BASE);
        assert_eq!(asm["code_challenge_methods_supported"][0], "S256");
        // Inscription.
        let reg = json!({ "redirect_uris": [CALLBACK], "client_name": "Claude" }).to_string();
        let (s, c) = h.json("POST", "/oauth/register", Some(("application/json", reg)));
        assert_eq!(s, 201, "{c}");
        let client_id = c["client_id"].as_str().unwrap().to_string();
        let bad = json!({ "redirect_uris": ["http://evil.example.com/cb"] }).to_string();
        assert_eq!(h.json("POST", "/oauth/register", Some(("application/json", bad))).0, 400);
        // Demande d'accès : page d'attente, demande présentée dans Revel.
        let verifier = "v".repeat(10) + "erifier-de-test-assez-long-pour-pkce-0123456789";
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(sha2::Sha256::digest(verifier.as_bytes()));
        let authorize = |state: &str| {
            format!(
                "/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&code_challenge={challenge}&code_challenge_method=S256&state={state}&resource={}&scope=revel",
                enc(&client_id),
                enc(CALLBACK),
                enc(&format!("{BASE}/mcp"))
            )
        };
        let (s, page, _) = h.send("GET", &authorize("etat1"), &[("Accept-Language", "fr-FR")], None);
        assert_eq!(s, 200);
        assert!(page.contains("Claude demande l’accès à Revel"), "{page}");
        assert_eq!(asked.load(Ordering::SeqCst), 1);
        let id = page.split("status?id=").nth(1).unwrap().split('\'').next().unwrap().to_string();
        let pending = o.pending();
        assert_eq!((pending.len(), pending[0].client.as_str(), pending[0].host.as_str()), (1, "Claude", "claude.ai"));
        let (_, st) = h.json("GET", &format!("/oauth/authorize/status?id={id}"), None);
        assert_eq!(st["state"], "pending");
        // Page d'attente : son propre en-tête Origin accepté ; une autre origine non.
        let (s, _, _) = h.send("GET", &format!("/oauth/authorize/status?id={id}"), &[("Origin", BASE)], None);
        assert_eq!(s, 200);
        let (s, _, _) = h.send("GET", &format!("/oauth/authorize/status?id={id}"), &[("Origin", "https://evil.example.com")], None);
        assert_eq!(s, 403);
        // Accordé dans Revel : retour avec le code, l'état et l'émetteur.
        assert!(o.decide(&id, true));
        let (_, st) = h.json("GET", &format!("/oauth/authorize/status?id={id}"), None);
        let back = st["redirect"].as_str().unwrap().to_string();
        assert!(back.starts_with(CALLBACK) && back.contains("state=etat1"), "{back}");
        assert_eq!(query(&back, "iss").as_deref(), Some(BASE));
        let code = query(&back, "code").unwrap();
        // Échange du code (PKCE) : jetons.
        let form = |pairs: &[(&str, &str)]| {
            pairs.iter().map(|(k, v)| format!("{k}={}", enc(v))).collect::<Vec<_>>().join("&")
        };
        let body = form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", CALLBACK),
            ("client_id", &client_id),
            ("code_verifier", &verifier),
            ("resource", &format!("{BASE}/mcp")),
        ]);
        let (s, tok) = h.json("POST", "/oauth/token", Some(("application/x-www-form-urlencoded", body.clone())));
        assert_eq!(s, 200, "{tok}");
        let access = tok["access_token"].as_str().unwrap().to_string();
        let refresh = tok["refresh_token"].as_str().unwrap().to_string();
        // Code à usage unique.
        let (s, e) = h.json("POST", "/oauth/token", Some(("application/x-www-form-urlencoded", body)));
        assert_eq!((s, e["error"].as_str()), (400, Some("invalid_grant")));
        // Outils avec le jeton (protocole 2026-07-28).
        let call = |token: &str, origin: Option<&str>| {
            let auth = format!("Bearer {token}");
            let mut hd = vec![
                ("Authorization", auth.as_str()),
                ("Accept", "application/json, text/event-stream"),
                ("MCP-Protocol-Version", "2026-07-28"),
                ("Mcp-Method", "tools/call"),
                ("Mcp-Name", "get_site"),
            ];
            if let Some(o) = origin {
                hd.push(("Origin", o));
            }
            let body = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
                "name": "get_site", "arguments": {}, "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientInfo": { "name": "claude-ai", "version": "1" },
                    "io.modelcontextprotocol/clientCapabilities": {} } } });
            h.send("POST", "/mcp", &hd, Some(("application/json", body.to_string())))
        };
        let (s, t, _) = call(&access, None);
        assert_eq!(s, 200, "{t}");
        assert!(t.contains("Atelier"));
        assert_eq!(call(&access, Some(BASE)).0, 403);
        assert_eq!(call("faux", None).0, 401);
        // Renouvellement tournant : l'ancien jeton de renouvellement ne vaut plus.
        let rbody = form(&[("grant_type", "refresh_token"), ("refresh_token", &refresh), ("client_id", &client_id)]);
        let (s, tok2) = h.json("POST", "/oauth/token", Some(("application/x-www-form-urlencoded", rbody.clone())));
        assert_eq!(s, 200, "{tok2}");
        assert_ne!(tok2["refresh_token"], tok["refresh_token"]);
        assert_eq!(h.json("POST", "/oauth/token", Some(("application/x-www-form-urlencoded", rbody))).0, 400);
        let access2 = tok2["access_token"].as_str().unwrap().to_string();
        assert_eq!(call(&access2, None).0, 200);
        let conns = o.connections();
        assert_eq!((conns.len(), conns[0].client.as_str(), conns[0].host.as_str()), (1, "Claude", "claude.ai"));
        // Refus : retour avec access_denied.
        let (_, page, _) = h.send("GET", &authorize("etat2"), &[], None);
        let id2 = page.split("status?id=").nth(1).unwrap().split('\'').next().unwrap().to_string();
        assert!(o.decide(&id2, false));
        let (_, st) = h.json("GET", &format!("/oauth/authorize/status?id={id2}"), None);
        assert!(st["redirect"].as_str().unwrap().contains("error=access_denied&state=etat2"));
        // Trop de demandes en attente.
        for _ in 0..3 {
            assert_eq!(h.send("GET", &authorize("x"), &[], None).0, 200);
        }
        assert_eq!(h.send("GET", &authorize("x"), &[], None).0, 429);
        // PKCE obligatoire.
        let no_pkce = authorize("y").replace("code_challenge_method=S256", "code_challenge_method=plain");
        let (s, _, _) = h.send("GET", &no_pkce, &[], None);
        assert_eq!(s, 303);
        // Autre hôte : refusé.
        let mut r = h.agent.get(&format!("{}/.well-known/oauth-authorization-server", h.root))
            .header("Host", "evil.example.com").call().unwrap();
        assert_eq!(r.status().as_u16(), 403);
        let _ = r.body_mut().read_to_string();
        // Révocation : jeton refusé.
        o.revoke(&conns[0].id);
        assert_eq!(call(&access2, None).0, 401);
        assert!(o.connections().is_empty());
    })
    .await
    .unwrap();
    // Autorisations enregistrées hors du site, lisibles par le seul utilisateur.
    assert!(dir.path().join("mcp-oauth.json").is_file());
    server.shutdown().await;
}
