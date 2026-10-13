//! `revel-dns` : gestion des noms de domaine (API OVH, Cloudflare).
//!
//! Interface posée à l'étape 0 ; implémentation prévue : Étape 5.

/// Enregistrement DNS simplifié.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub kind: String,
    pub name: String,
    pub value: String,
    pub ttl: u32,
}

/// Fournisseur DNS. Toute écriture demande une confirmation explicite dans l'interface.
pub trait DnsProvider {
    type Error: std::error::Error;
    fn list(&self, zone: &str) -> Result<Vec<Record>, Self::Error>;
}
