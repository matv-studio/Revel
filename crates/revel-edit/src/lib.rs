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
pub mod canvas;
pub mod collection;
pub mod command;
pub mod history;
pub mod icons;
pub mod locale;
pub mod make;
pub mod readable;
pub mod ready;
pub mod style;

pub use address::{Branch, Owner, Parent, Target};
pub use command::{Command, ShelfKind, apply, expand};
pub use history::{Exec, History, Step};

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
    #[error("points de rupture invalides : {0}")]
    InvalidBreakpoints(String),
    #[error("commande appliquée par le cœur (fichiers du projet)")]
    External,
    #[error("valeur liée non modifiable : {0}")]
    Bound(String),
    #[error("adresse de page invalide : {0}")]
    InvalidPath(String),
    #[error("adresse déjà utilisée par une autre page : {0}")]
    PathTaken(String),
    #[error("redirection invalide : {0}")]
    InvalidRedirect(String),
    /// Expression dont la syntaxe est invalide : l'expression et le message du moteur.
    #[error("{0}")]
    InvalidExpr(String),
    /// Composant encore utilisé (retrait refusé).
    #[error("composant encore utilisé : {0}")]
    InUse(String),
    /// Nom de propriété invalide ou déjà pris.
    #[error("nom de propriété refusé : {0}")]
    InvalidProp(String),
    /// Composant placé dans lui-même (directement ou par un autre composant).
    #[error("composant placé dans lui-même : {0}")]
    SelfUse(String),
    /// Code de langue invalide, déjà présent ou langue inconnue.
    #[error("langue refusée : {0}")]
    InvalidLocale(String),
    /// Réglage du site refusé (nom vide, adresse invalide).
    #[error("{0}")]
    InvalidSite(String),
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
            Self::InvalidBreakpoints(_) => "error-edit-breakpoints",
            Self::External | Self::Bound(_) => "error-edit-bound",
            Self::InvalidExpr(_) => "error-edit-expr",
            Self::InvalidPath(_) => "error-edit-path",
            Self::PathTaken(_) => "error-edit-path-taken",
            Self::InvalidRedirect(_) => "error-edit-redirect",
            Self::InUse(_) => "error-edit-in-use",
            Self::InvalidProp(_) => "error-edit-prop",
            Self::SelfUse(_) => "error-edit-self-use",
            Self::InvalidLocale(_) => "error-edit-locale",
            Self::InvalidSite(_) => "error-edit-site",
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

/// Empreinte d'un site ouvert, comparée régulièrement par l'éditeur : `revision` change quand une
/// autre vue ou un assistant (même session du cœur) modifie le document, `files` quand un fichier
/// lu par le canevas change hors de Revel (Pages CMS, éditeur de texte).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EditStamp {
    pub revision: u64,
    pub files: String,
}

/// Document ouvert pour l'édition.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Opened {
    pub site: revel_model::Site,
    pub state: EditState,
}

#[cfg(test)]
mod tests;
