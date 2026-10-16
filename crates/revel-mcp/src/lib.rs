//! `revel-mcp` : serveur MCP de Revel (étape 4, D-13, D-31, D-41). Les outils lisent et
//! modifient le site ouvert dans Revel avec les mêmes commandes que l'interface : chaque
//! modification est visible en direct, annulable et présente dans l'historique.
//!
//! Les outils sont définis une fois et appellent une [`Invoke`] : soit le cœur directement (dans
//! l'application et la ligne de commande), soit l'application ouverte par HTTP local (relais
//! stdio de `revel --mcp`, voir [`relay`]). Transports : stdio et HTTP en flux sur 127.0.0.1 avec
//! jeton ([`http`]).

pub mod activity;
pub mod config;
#[cfg(feature = "core")]
pub mod direct;
pub mod http;
mod outline;
pub mod relay;
mod tools;

pub use tools::Revel;

use revel_protocol::Failure;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Réponse d'une commande.
pub type Reply = Pin<Box<dyn Future<Output = Result<Value, Failure>> + Send>>;

/// Exécutant des commandes de Revel (`edit_open`, `edit_apply`, `edit_undo`, `cms_collections`,
/// `page_html`…) sur le site nommé par `args.site` ou, à défaut, le site ouvert dans Revel.
pub trait Invoke: Send + Sync + 'static {
    fn invoke(&self, cmd: &str, args: Value) -> Reply;
}

/// Texte d'un échec, en anglais (langue des descriptions des outils).
pub fn failure_text(f: &Failure) -> String {
    let l = revel_i18n::Localizer::new("en");
    let args: Vec<(&str, &str)> = f
        .args
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    l.t(&f.key, &args)
}
