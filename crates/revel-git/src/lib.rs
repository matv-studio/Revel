//! `revel-git` : Git et GitHub.
//!
//! - [`publish`] : publication d'un dossier (site généré) sur une branche d'un dépôt distant,
//!   sans copie de travail, en n'envoyant que les changements (GitHub Pages, et tout dépôt Git) ;
//! - [`github`] (fonction `github`) : connexion par autorisation sur github.com (device flow,
//!   D-11), compte, dépôt, activation de GitHub Pages ;
//! - [`versions`] : versions du site (dépôt Git du dossier du site), comparaison, restauration,
//!   synchronisation avec son dépôt distant (D-25).
//!
//! - [`clone`] : copie d'un dépôt distant dans un dossier (import depuis Git, D-80).
//!
//! libgit2 est compilée dans Revel (D-12) : aucune installation de Git n'est nécessaire.

pub mod clone;
#[cfg(feature = "github")]
pub mod github;
pub mod publish;
pub mod versions;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Git : {0}")]
    Git(#[from] git2::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("dossier introuvable : {0}")]
    MissingDir(String),
    #[error("nom de fichier non pris en charge : {0}")]
    FileName(String),
    #[error("envoi refusé par le dépôt : {0}")]
    Rejected(String),
    #[error("réseau : {0}")]
    Http(String),
    #[error("GitHub ({0}) : {1}")]
    Api(u16, String),
    #[error("code d'autorisation expiré")]
    CodeExpired,
    #[error("autorisation refusée sur github.com")]
    Denied,
    #[error("application GitHub mal configurée : {0}")]
    AppConfig(String),
    #[error("connexion GitHub expirée ou révoquée")]
    SignedOut,
}

impl GitError {
    /// Erreur d'authentification lors d'un échange Git (jeton refusé).
    pub fn is_auth(&self) -> bool {
        match self {
            Self::SignedOut => true,
            Self::Git(e) => {
                e.class() == git2::ErrorClass::Http && e.code() == git2::ErrorCode::Auth
                    || e.message().contains("authentification refusée")
                    || e.message().contains("401")
            }
            _ => false,
        }
    }
}
