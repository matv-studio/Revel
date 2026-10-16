//! Serveur des assistants de bout en bout : HTTP local (jeton, origine), MCP (protocole 2025 avec
//! session, 2026-07-28 sans état), outils de lecture et d'écriture sur un vrai site, relais.

use revel_core::{Core, secrets};
use revel_mcp::config::McpConfig;
use revel_mcp::relay::HttpInvoke;
use revel_mcp::{Invoke, Reply, http};
use revel_protocol::Failure;
use serde_json::{Value, json};
use std::sync::Arc;

/// Le cœur, comme dans l'application (`direct::CoreInvoke`, sans la fonction `core`).
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

fn site() -> (tempfile::TempDir, Arc<Core>, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let core = Arc::new(Core::new(
        &dir.path().join("config"),
        Box::new(secrets::Memory::default()),
    ));
    let path = dir.path().join("atelier");
    core.new_site_from(
        &path,
        "Atelier",
        "fr",
        revel_protocol::SiteTemplate::Showcase,
    )
    .unwrap();
    core.edit_open(&path).unwrap();
    (dir, core, path)
}

/// Client minimal : requête JSON-RPC, réponse JSON (ou premier message d'un flux SSE).
struct Client {
    url: String,
    token: String,
    session: Option<String>,
    agent: ureq::Agent,
}

impl Client {
    fn post(&mut self, body: Value, extra: &[(&str, &str)]) -> (u16, Value) {
        let mut req = self
            .agent
            .post(&self.url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");
        if let Some(s) = &self.session {
            req = req.header("Mcp-Session-Id", s);
        }
        for (k, v) in extra {
            req = req.header(*k, *v);
        }
        let mut res = req.send(body.to_string()).unwrap();
        if let Some(s) = res
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
        {
            self.session = Some(s.to_string());
        }
        let status = res.status().as_u16();
        let text = res.body_mut().read_to_string().unwrap_or_default();
        let json_text = if text.trim_start().starts_with('{') {
            text
        } else {
            text.lines()
                .filter_map(|l| l.strip_prefix("data:"))
                .map(str::trim)
                .find(|l| l.starts_with('{'))
                .unwrap_or("null")
                .to_string()
        };
        (
            status,
            serde_json::from_str(&json_text).unwrap_or(Value::Null),
        )
    }

    fn call(&mut self, id: u64, name: &str, args: Value) -> String {
        let (s, v) = self.post(
            json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call",
                    "params": { "name": name, "arguments": args } }),
            &[],
        );
        assert_eq!(s, 200, "{v}");
        assert_ne!(v["result"]["isError"], json!(true), "{name} : {v}");
        v["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
}

#[tokio::test(flavor = "multi_thread")]
async fn assistant_de_bout_en_bout() {
    let (_d, core, path) = site();
    let log = revel_mcp::activity::ActivityLog::default();
    let inv: Arc<dyn Invoke> = Arc::new(revel_mcp::activity::Logged {
        inner: Arc::new(CoreInvoke(core.clone())),
        log: log.clone(),
    });
    let token = "jeton-essai-0123456789abcdef0123456789abcdef".to_string();
    let server = http::start(inv, token.clone(), 0).await.unwrap();
    let url = format!("http://127.0.0.1:{}/mcp", server.port);

    let res = tokio::task::spawn_blocking(move || {
        let mut c = Client {
            url: url.clone(),
            token: token.clone(),
            session: None,
            agent: agent(),
        };
        // Sans jeton : refusé ; depuis une page web : refusé.
        let bad = c
            .agent
            .post(&url)
            .header("Content-Type", "application/json")
            .send("{}")
            .unwrap();
        assert_eq!(bad.status().as_u16(), 401);
        let (s, _) = c.post(json!({}), &[("Origin", "https://exemple.com")]);
        assert_eq!(s, 403);
        let (s, _) = c.post(json!({}), &[("Host", "exemple.com")]);
        assert_eq!(s, 403);
        // Assistant dans Docker Desktop (Open WebUI) : hôte accepté jusqu'au service MCP.
        let mut docker = Client {
            url: url.clone(),
            token: token.clone(),
            session: None,
            agent: agent(),
        };
        let (s, v) = docker.post(
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": { "name": "open-webui", "version": "1" } } }),
            &[("Host", "host.docker.internal:47520")],
        );
        assert_eq!(s, 200, "{v}");

        // Protocole 2025 : ouverture, liste des outils.
        let (s, v) = c.post(
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": { "name": "essai", "version": "1" } } }),
            &[],
        );
        assert_eq!(s, 200, "{v}");
        assert!(v["result"]["instructions"].as_str().unwrap_or_default().contains("Revel"));
        let _ = c.post(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }), &[]);
        let (_, v) = c.post(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }), &[]);
        let names: Vec<String> = v["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        for n in ["get_site", "get_page", "set_text", "set_styles", "insert_html", "undo"] {
            assert!(names.contains(&n.to_string()), "{names:?}");
        }

        // Lecture : le site, le plan de l'accueil.
        let site = c.call(3, "get_site", json!({}));
        assert!(site.contains("Atelier") && site.contains("Pages:"), "{site}");
        let page = c.call(4, "get_page", json!({ "url": "/" }));
        let h1 = page
            .lines()
            .find(|l| l.contains("<h1"))
            .and_then(|l| l.split('#').nth(1))
            .map(|r| r.split_whitespace().next().unwrap().to_string())
            .unwrap_or_else(|| panic!("titre introuvable :\n{page}"));

        // Écriture : texte, styles, HTML inséré ; puis annulations.
        let r = c.call(5, "set_text", json!({ "id": h1, "text": "Bonjour depuis un assistant" }));
        assert!(r.contains("replaced"), "{r}");
        let r = c.call(6, "set_styles", json!({ "selector": ".essai-mcp", "properties": { "color": "#123456" } }));
        assert!(r.contains("set"), "{r}");
        let r = c.call(7, "insert_html", json!({ "html": "<p class=\"essai-mcp\">Ajouté</p>", "after": h1 }));
        assert!(r.contains("Inserted"), "{r}");
        let page = c.call(8, "get_page", json!({ "url": "/" }));
        assert!(page.contains("Bonjour depuis un assistant") && page.contains("<p.essai-mcp>"), "{page}");
        // Erreur lisible (élément inconnu), sans erreur de protocole.
        let (_, v) = c.post(
            json!({ "jsonrpc": "2.0", "id": 9, "method": "tools/call",
                    "params": { "name": "set_text", "arguments": { "id": "inconnu", "text": "x" } } }),
            &[],
        );
        assert_eq!(v["result"]["isError"], json!(true));

        // Protocole 2026-07-28, sans état : appel direct avec ses en-têtes.
        let mut s26 = Client {
            url: url.clone(),
            token: token.clone(),
            session: None,
            agent: agent(),
        };
        let (s, v) = s26.post(
            json!({ "jsonrpc": "2.0", "id": 10, "method": "tools/call", "params": {
                "name": "undo", "arguments": {},
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientInfo": { "name": "essai", "version": "1" },
                    "io.modelcontextprotocol/clientCapabilities": {} } } }),
            &[
                ("MCP-Protocol-Version", "2026-07-28"),
                ("Mcp-Method", "tools/call"),
                ("Mcp-Name", "undo"),
            ],
        );
        assert_eq!(s, 200, "{v}");
        assert!(v["result"]["content"][0]["text"].as_str().unwrap_or_default().contains("Undone"), "{v}");
        c.call(11, "undo", json!({}));
        c.call(12, "undo", json!({}));
        let page = c.call(13, "get_page", json!({ "url": "/" }));
        assert!(!page.contains("Bonjour depuis un assistant") && !page.contains("essai-mcp"), "{page}");
    })
    .await;
    res.unwrap();
    // Mêmes outils sans MCP (OpenAPI) : description, appel, erreur lisible, outil inconnu.
    let base = format!("http://127.0.0.1:{}", server.port);
    let tok = "jeton-essai-0123456789abcdef0123456789abcdef".to_string();
    let res = tokio::task::spawn_blocking(move || {
        let a = agent();
        let mut r = a
            .get(&format!("{base}/openapi.json"))
            .header("Authorization", &format!("Bearer {tok}"))
            .call()
            .unwrap();
        let doc: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(doc["openapi"], "3.1.0");
        assert_eq!(doc["servers"][0]["url"], base.as_str());
        let set = &doc["paths"]["/api/set_text"]["post"];
        assert_eq!(set["operationId"], "set_text");
        assert!(
            set["requestBody"]["content"]["application/json"]["schema"]["properties"]["text"]
                .is_object()
        );
        let post = |tool: &str, body: Value| {
            let mut r = a
                .post(&format!("{base}/api/{tool}"))
                .header("Authorization", &format!("Bearer {tok}"))
                .header("Content-Type", "application/json")
                .send(body.to_string())
                .unwrap();
            let status = r.status().as_u16();
            let v: Value =
                serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap_or_default();
            (status, v)
        };
        let (st, v) = post("get_site", json!({}));
        assert_eq!(st, 200);
        assert!(v["result"].as_str().unwrap().contains("Atelier"));
        let (st, v) = post("set_text", json!({ "id": "inconnu", "text": "x" }));
        assert_eq!(st, 422);
        assert!(v["error"].as_str().unwrap().contains("inconnu"), "{v}");
        let body = json!({ "selector": ".essai-api", "properties": { "color": "red" } });
        assert_eq!(post("set_styles", body).0, 200);
        assert_eq!(post("undo", json!({})).0, 200);
        assert_eq!(post("inexistant", json!({})).0, 404);
        assert_eq!(post("set_text", json!({ "text": 3 })).0, 422);
    })
    .await;
    res.unwrap();
    // Dernières actions : chaque modification (assistant, outil, cible), la plus récente d'abord ;
    // l'erreur (élément inconnu) n'y est pas.
    let acts = log.list();
    let tools: Vec<&str> = acts.iter().map(|a| a.tool.as_str()).collect();
    assert_eq!(
        tools,
        [
            "undo",
            "set_styles",
            "undo",
            "undo",
            "undo",
            "insert_html",
            "set_styles",
            "set_text"
        ],
        "{acts:?}"
    );
    assert!(acts[..2].iter().all(|a| a.client == "OpenAPI"), "{acts:?}");
    assert!(acts[2..].iter().all(|a| a.client == "essai"), "{acts:?}");
    assert_eq!(acts[6].target.as_deref(), Some(".essai-mcp"));
    // Le site sur le disque : retour à l'état d'origine.
    let doc = std::fs::read_to_string(path.join("site.revel.json")).unwrap();
    assert!(!doc.contains("essai-mcp"));
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn relais_vers_l_application() {
    let (d, core, _path) = site();
    let inv: Arc<dyn Invoke> = Arc::new(CoreInvoke(core.clone()));
    let cfg_dir = d.path().join("app");
    let mut cfg = McpConfig::load(&cfg_dir);
    let server = http::start(inv, cfg.token.clone(), 0).await.unwrap();
    cfg.port = server.port;
    cfg.enabled = true;
    cfg.save(&cfg_dir).unwrap();
    // Le relais lit le port et le jeton dans les réglages, appelle l'application.
    let relay: Arc<dyn Invoke> = Arc::new(Arc::new(HttpInvoke::new(cfg_dir.clone(), None)));
    let pages = relay.invoke("edit_pages", json!({})).await.unwrap();
    assert!(pages.as_array().is_some_and(|p| !p.is_empty()), "{pages}");
    // Échec du cœur : transmis tel quel (clé de traduction).
    let err = relay.invoke("inconnue", json!({})).await.unwrap_err();
    assert!(!err.key.is_empty());
    // Jeton faux : refusé.
    let mut wrong = cfg.clone();
    wrong.token = "faux".repeat(16);
    wrong.save(&cfg_dir).unwrap();
    let err = relay.invoke("edit_pages", json!({})).await.unwrap_err();
    assert_eq!(err.key, "error-mcp-token");
    // Arrêt puis redémarrage sur le même port (nouvelle clé) : le port est libéré.
    let port = server.port;
    server.shutdown().await;
    let inv: Arc<dyn Invoke> = Arc::new(CoreInvoke(core.clone()));
    let again = http::start(inv, wrong.token.clone(), port).await.unwrap();
    let pages = relay.invoke("edit_pages", json!({})).await.unwrap();
    assert!(pages.as_array().is_some_and(|p| !p.is_empty()));
    // Serveur désactivé et arrêté : le relais le dit.
    again.shutdown().await;
    wrong.enabled = false;
    wrong.save(&cfg_dir).unwrap();
    let err = relay.invoke("edit_pages", json!({})).await.unwrap_err();
    assert_eq!(err.key, "error-mcp-disabled");
}
