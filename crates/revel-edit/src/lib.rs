//! `revel-edit` : modification du document de site par **commandes** (D-13).
//!
//! - Une [`Command`] est sérialisable (JSON) : l'interface, la ligne de commande et le futur serveur
//!   MCP envoient les mêmes commandes.
//! - [`apply`] modifie le site et renvoie la commande inverse ; une commande qui échoue ne modifie
//!   rien (les suites de commandes sont atomiques).
//! - [`History`] conserve les paires (commande, inverse) pour annuler et rétablir.
//!
//! Ne dépend que de `revel-model` et `revel-style` (compilable pour l'interface en WebAssembly).

pub mod address;
pub mod command;
pub mod history;
pub mod make;
pub mod style;

pub use address::{Branch, Owner, Parent, Target};
pub use command::{Command, apply, expand};
pub use history::{History, Step};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("introuvable : {0}")]
    NotFound(String),
    #[error("ce nœud n'a pas cette liste d'enfants")]
    NoBranch,
    #[error("position invalide : {0}")]
    Index(usize),
    #[error("le nœud {0} n'est pas un élément")]
    NotAnElement(String),
    #[error("le nœud {0} n'est pas du type attendu")]
    WrongKind(String),
    #[error("le nœud à la position {0} n'est pas un texte")]
    NotText(usize),
    #[error("identifiant déjà utilisé : {0}")]
    DuplicateId(String),
    #[error("un élément ne peut pas être déplacé dans lui-même")]
    InvalidMove,
    #[error("nom de balise invalide : {0}")]
    InvalidTag(String),
    #[error("CSS refusé : {0}")]
    InvalidCss(String),
    #[error("le site n'a pas de feuille de style")]
    NoStylesheet,
}

impl EditError {
    /// Clé de traduction (`revel-i18n`, D-16).
    pub fn key(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "error-edit-not-found",
            Self::NoBranch
            | Self::Index(_)
            | Self::NotText(_)
            | Self::WrongKind(_)
            | Self::NotAnElement(_) => "error-edit-invalid",
            Self::DuplicateId(_) => "error-edit-duplicate",
            Self::InvalidMove => "error-edit-move",
            Self::InvalidTag(_) => "error-edit-tag",
            Self::InvalidCss(_) => "error-edit-css",
            Self::NoStylesheet => "error-edit-no-stylesheet",
        }
    }
}

/// État d'un document ouvert dans l'éditeur, renvoyé après chaque modification.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditState {
    /// Numéro de version du document dans la session (augmente à chaque modification).
    pub revision: u64,
    pub can_undo: bool,
    pub can_redo: bool,
    /// Commande effectivement appliquée (pour que l'interface reproduise la modification sur sa
    /// copie du document) ; `None` si rien n'a changé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied: Option<Command>,
}

/// Document ouvert pour l'édition.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Opened {
    pub site: revel_model::Site,
    pub state: EditState,
}

#[cfg(test)]
mod tests;
