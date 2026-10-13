//! `revel-git` : opérations Git (cloner, valider, pousser, étiqueter).
//!
//! Interface posée à l'étape 0 ; implémentation prévue : Étape 1.

/// Opérations Git utilisées par Revel, indépendantes de la bibliothèque choisie.
pub trait Repository {
    type Error: std::error::Error;
    fn current_commit(&self) -> Result<String, Self::Error>;
    fn commit_all(&self, message: &str) -> Result<String, Self::Error>;
}
