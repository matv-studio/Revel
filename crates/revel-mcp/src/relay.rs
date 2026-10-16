//! Relais stdio (`revel --mcp`) : l'assistant lance cette commande et lui parle par l'entrée et
//! la sortie standard (MCP stdio) ; chaque outil appelle l'application Revel ouverte par HTTP
//! local (`/invoke/<commande>`, jeton lu dans les réglages). Une seule écriture du site (celle de
//! l'application) : rien ne se perd, l'interface suit en direct. Application fermée : elle est
//! lancée, puis attendue quelques secondes.

use crate::config::McpConfig;
use crate::{Invoke, Reply, Revel};
use revel_protocol::Failure;
use rmcp::ServiceExt;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Commandes envoyées à l'application ouverte.
pub struct HttpInvoke {
    config_dir: PathBuf,
    /// Programme qui lance l'application (elle-même, sans `--mcp`) ; `None` : pas de lancement.
    launch: Option<PathBuf>,
    agent: ureq::Agent,
}

impl HttpInvoke {
    pub fn new(config_dir: PathBuf, launch: Option<PathBuf>) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(120)))
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            config_dir,
            launch,
            agent,
        }
    }

    fn post(&self, cmd: &str, args: &Value) -> Result<Value, Failure> {
        let c = McpConfig::load(&self.config_dir);
        let url = format!("http://127.0.0.1:{}/invoke/{cmd}", c.port);
        let send = || {
            self.agent
                .post(&url)
                .header("Authorization", &format!("Bearer {}", c.token))
                .header("Content-Type", "application/json")
                .send(args.to_string())
        };
        let mut res = match send() {
            Ok(r) => r,
            Err(_) => {
                // Application fermée (ou serveur désactivé) : lancée, puis attendue.
                if !c.enabled {
                    return Err(Failure::new("error-mcp-disabled"));
                }
                let Some(exe) = &self.launch else {
                    return Err(Failure::new("error-mcp-unreachable"));
                };
                // Sans les entrées et sorties du relais (réservées au protocole).
                let _ = std::process::Command::new(exe)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
                let until = Instant::now() + Duration::from_secs(25);
                loop {
                    std::thread::sleep(Duration::from_millis(500));
                    if let Ok(r) = send() {
                        break r;
                    }
                    if Instant::now() > until {
                        return Err(Failure::new("error-mcp-unreachable"));
                    }
                }
            }
        };
        let status = res.status().as_u16();
        let text = res
            .body_mut()
            .read_to_string()
            .map_err(|e| Failure::detail(e.to_string()))?;
        match status {
            200 => serde_json::from_str(&text).map_err(|e| Failure::detail(e.to_string())),
            401 => Err(Failure::new("error-mcp-token")),
            _ => Err(serde_json::from_str::<Failure>(&text)
                .unwrap_or_else(|_| Failure::detail(format!("HTTP {status} : {text}")))),
        }
    }
}

impl Invoke for Arc<HttpInvoke> {
    fn invoke(&self, cmd: &str, args: Value) -> Reply {
        let me = self.clone();
        let cmd = cmd.to_string();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || me.post(&cmd, &args))
                .await
                .map_err(|e| Failure::detail(e.to_string()))?
        })
    }
}

/// Serveur MCP sur l'entrée et la sortie standard, outils branchés sur `inv`.
pub async fn serve_stdio(inv: Arc<dyn Invoke>) -> Result<(), String> {
    let server = Revel::new(inv)
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| e.to_string())?;
    server.waiting().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Relais complet : outils branchés sur l'application ouverte.
pub async fn run(config_dir: PathBuf, launch: Option<PathBuf>) -> Result<(), String> {
    let inv: Arc<dyn Invoke> = Arc::new(Arc::new(HttpInvoke::new(config_dir, launch)));
    serve_stdio(inv).await
}
