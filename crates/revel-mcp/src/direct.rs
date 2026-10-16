//! Outils branchés directement sur le cœur de Revel (dans l'application et la ligne de
//! commande) : mêmes commandes que l'interface, sur le site ouvert.

use crate::{Invoke, Reply};
use revel_core::Core;
use revel_protocol::Failure;
use serde_json::Value;
use std::sync::Arc;

/// Commandes exécutées par le cœur (hors de la boucle des tâches : lectures et écritures de
/// fichiers).
#[derive(Clone)]
pub struct CoreInvoke(pub Arc<Core>);

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
