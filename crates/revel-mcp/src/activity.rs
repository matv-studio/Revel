//! Dernières actions des assistants (Réglages › Assistants IA) : chaque modification faite par
//! un outil (assistant, outil, cible, heure), relevée dans l'application, que l'outil tourne dans
//! l'application (HTTP) ou dans le relais stdio (`via` transmis avec la commande).

use crate::{Invoke, Reply};
use revel_protocol::McpActivity;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Actions gardées au plus.
const KEEP: usize = 50;

/// Commandes qui modifient le site.
pub(crate) const WRITES: &[&str] = &["edit_apply", "edit_undo", "edit_redo"];

#[derive(Clone, Default)]
pub struct ActivityLog(Arc<Mutex<VecDeque<McpActivity>>>);

impl ActivityLog {
    pub fn push(&self, a: McpActivity) {
        let mut q = self.0.lock().unwrap_or_else(|e| e.into_inner());
        q.push_front(a);
        q.truncate(KEEP);
    }

    /// Actions, la plus récente d'abord.
    pub fn list(&self) -> Vec<McpActivity> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect()
    }
}

/// Exécutant qui relève les modifications des assistants (`args.via`).
pub struct Logged {
    pub inner: Arc<dyn Invoke>,
    pub log: ActivityLog,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

impl Invoke for Logged {
    fn invoke(&self, cmd: &str, args: Value) -> Reply {
        let via = WRITES
            .contains(&cmd)
            .then(|| args.get("via").cloned())
            .flatten();
        let fut = self.inner.invoke(cmd, args);
        let log = self.log.clone();
        Box::pin(async move {
            let r = fut.await;
            // Modification effectivement faite (`applied` présent) : relevée.
            if let (Ok(v), Some(via)) = (&r, via)
                && v.get("applied").is_some_and(|a| !a.is_null())
            {
                let s = |k: &str| via.get(k).and_then(Value::as_str).map(str::to_string);
                log.push(McpActivity {
                    at: now_ms(),
                    client: s("client").unwrap_or_default(),
                    tool: s("tool").unwrap_or_default(),
                    target: s("target"),
                });
            }
            r
        })
    }
}
