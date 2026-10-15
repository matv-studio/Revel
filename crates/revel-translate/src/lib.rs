//! `revel-translate` : traduction automatique hors ligne (D-36). Modèles Marian OPUS-MT de
//! l'université d'Helsinki (licence Apache-2.0) exécutés sur le processeur : aucun service
//! extérieur, aucune connexion pendant la traduction.
//!
//! Le français et l'anglais sont livrés avec Revel ; les autres langues du [`catalog`] se
//! téléchargent une fois depuis les réglages ([`install`]). Un dossier de modèles contient un
//! dossier par sens de traduction (`opus-mt-fr-en`) ; entre deux langues sans modèle direct, la
//! traduction passe par l'anglais (allemand → français : allemand → anglais → français).

pub mod catalog;
#[cfg(feature = "download")]
pub mod install;
pub mod model;
pub mod prepare;
pub mod spm;
pub mod text;

pub use text::Format;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum TranslateError {
    #[error("modèle de traduction : {0}")]
    Model(String),
    #[error("pas de modèle de traduction de « {0} » vers « {1} »")]
    NoPair(String, String),
    #[error("téléchargement : {0}")]
    Download(String),
    #[error("fichier téléchargé altéré : {0}")]
    Integrity(String),
    #[error("téléchargement annulé")]
    Cancelled,
}

/// Langue d'un code (`pt-BR` -> `pt`).
pub fn base(code: &str) -> String {
    code.split(['-', '_'])
        .next()
        .unwrap_or(code)
        .to_ascii_lowercase()
}

/// Dossier d'un modèle prêt (poids présents).
fn ready(dir: &Path) -> bool {
    dir.join("model.safetensors").is_file()
}

/// Paires disponibles dans des dossiers de modèles : (de, vers), sans doublon ; `hidden` : noms
/// de modèles ignorés.
pub fn pairs(roots: &[PathBuf], hidden: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if let Some(rest) = name.strip_prefix("opus-mt-")
                && let Some((a, b)) = rest.split_once('-')
                && !hidden.contains(&name)
                && ready(&e.path())
            {
                out.push((a.to_string(), b.to_string()));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Chemin de traduction : paire directe, sinon par l'anglais, sinon par une autre langue.
pub fn route(pairs: &[(String, String)], from: &str, to: &str) -> Option<Vec<(String, String)>> {
    let (from, to) = (base(from), base(to));
    if from == to {
        return Some(Vec::new());
    }
    let has = |a: &str, b: &str| pairs.iter().any(|(x, y)| x == a && y == b);
    if has(&from, &to) {
        return Some(vec![(from, to)]);
    }
    let hub = catalog::HUB.to_string();
    if has(&from, &hub) && has(&hub, &to) {
        return Some(vec![(from, hub.clone()), (hub, to)]);
    }
    pairs
        .iter()
        .filter(|(a, _)| *a == from)
        .find(|(_, mid)| has(mid, &to))
        .map(|(a, mid)| vec![(a.clone(), mid.clone()), (mid.clone(), to.clone())])
}

/// Texte à traduire et sa forme.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Item {
    pub text: String,
    #[serde(default)]
    pub format: Format,
}

/// Moteur : modèles chargés à la demande (deux au plus en mémoire), phrases déjà traduites
/// gardées. Dossiers de modèles lus dans l'ordre (modèles livrés, puis téléchargés).
pub struct Engine {
    roots: Vec<PathBuf>,
    /// Modèles ignorés (langues livrées retirées par l'utilisateur).
    hidden: Vec<String>,
    models: Mutex<Vec<(String, model::Model)>>,
    cache: Mutex<HashMap<(String, String), String>>,
}

/// Traduction d'une phrase par une suite de modèles, avec mémoire des phrases déjà traduites.
struct Chain<'a> {
    /// Modèles du chemin et jeton de langue de chacun.
    models: Vec<(&'a mut model::Model, Option<&'static str>)>,
    key: String,
    cache: &'a mut HashMap<(String, String), String>,
    error: Option<TranslateError>,
}

impl text::Sentences for Chain<'_> {
    fn sentence(&mut self, text: &str) -> String {
        let k = (self.key.clone(), text.to_string());
        if let Some(t) = self.cache.get(&k) {
            return t.clone();
        }
        let mut cur = text.to_string();
        for (m, token) in self.models.iter_mut() {
            match m.sentence(&cur, 1, *token) {
                Ok(t) => cur = t,
                Err(e) => {
                    self.error.get_or_insert(e);
                    return text.to_string();
                }
            }
        }
        if self.cache.len() > 50_000 {
            self.cache.clear();
        }
        self.cache.insert(k, cur.clone());
        cur
    }
}

impl Engine {
    pub fn new(roots: impl IntoIterator<Item = PathBuf>) -> Self {
        Self::with_hidden(roots, Vec::new())
    }

    /// Moteur sans les modèles `hidden` (langues livrées retirées par l'utilisateur).
    pub fn with_hidden(roots: impl IntoIterator<Item = PathBuf>, hidden: Vec<String>) -> Self {
        Self {
            roots: roots.into_iter().collect(),
            hidden,
            models: Mutex::new(Vec::new()),
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Dossiers des modèles.
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Paires de langues disponibles.
    pub fn pairs(&self) -> Vec<(String, String)> {
        pairs(&self.roots, &self.hidden)
    }

    /// Dossier d'un modèle prêt.
    fn find(&self, name: &str) -> Option<PathBuf> {
        if self.hidden.iter().any(|h| h == name) {
            return None;
        }
        self.roots.iter().map(|r| r.join(name)).find(|d| ready(d))
    }

    /// Modèles retirés de la mémoire (langue supprimée).
    pub fn forget(&self, names: &[&str]) {
        let mut loaded = self.models.lock().unwrap_or_else(|e| e.into_inner());
        loaded.retain(|(n, _)| !names.contains(&n.as_str()));
    }

    /// Traduction possible de `from` vers `to` (directe ou par une langue intermédiaire).
    pub fn can(&self, from: &str, to: &str) -> bool {
        route(&self.pairs(), from, to).is_some()
    }

    /// Traduit des textes ; `keep` : mots gardés tels quels (noms de marque).
    pub fn translate(
        &self,
        from: &str,
        to: &str,
        items: &[Item],
        keep: &[String],
    ) -> Result<Vec<String>, TranslateError> {
        let path = route(&self.pairs(), from, to)
            .ok_or_else(|| TranslateError::NoPair(from.to_string(), to.to_string()))?;
        if path.is_empty() {
            return Ok(items.iter().map(|i| i.text.clone()).collect());
        }
        let mut loaded = self.models.lock().unwrap_or_else(|e| e.into_inner());
        // Modèles du chemin chargés (les autres libérés : deux au plus en mémoire).
        let names: Vec<String> = path
            .iter()
            .map(|(a, b)| format!("opus-mt-{a}-{b}"))
            .collect();
        loaded.retain(|(n, _)| names.contains(n));
        for n in &names {
            if !loaded.iter().any(|(x, _)| x == n) {
                let dir = self
                    .find(n)
                    .ok_or_else(|| TranslateError::NoPair(from.to_string(), to.to_string()))?;
                loaded.push((n.clone(), model::Model::open(&dir)?));
            }
        }
        let mut chain_models: Vec<(&mut model::Model, Option<&'static str>)> = Vec::new();
        // Dans l'ordre du chemin ; jeton de langue : celle visée par le modèle (la dernière
        // étape vise la langue demandée, avec sa variante : chinois traditionnel…).
        let mut rest: Vec<&mut (String, model::Model)> = loaded.iter_mut().collect();
        for (k, n) in names.iter().enumerate() {
            if let Some(i) = rest.iter().position(|(x, _)| x == n) {
                let (_, m) = rest.remove(i);
                let target = if k + 1 == names.len() {
                    to.to_string()
                } else {
                    path[k].1.clone()
                };
                chain_models.push((m, catalog::target_token(n, &target)));
            }
        }
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let mut chain = Chain {
            models: chain_models,
            key: names.join(">"),
            cache: &mut cache,
            error: None,
        };
        let out = items
            .iter()
            .map(|i| text::translate(&i.text, i.format, keep, &mut chain))
            .collect();
        match chain.error {
            Some(e) => Err(e),
            None => Ok(out),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chemins() {
        let pair = |a: &str, b: &str| (a.to_string(), b.to_string());
        let mut p = vec![pair("fr", "en"), pair("en", "fr")];
        assert_eq!(route(&p, "fr", "en-GB"), Some(vec![pair("fr", "en")]));
        assert_eq!(route(&p, "fr", "fr-CA"), Some(vec![]));
        assert_eq!(route(&p, "de", "fr"), None);
        // Allemand téléchargé : allemand ↔ français par l'anglais.
        p.extend([pair("de", "en"), pair("en", "de"), pair("de", "es")]);
        assert_eq!(
            route(&p, "de", "fr"),
            Some(vec![pair("de", "en"), pair("en", "fr")])
        );
        assert_eq!(
            route(&p, "fr", "de-AT"),
            Some(vec![pair("fr", "en"), pair("en", "de")])
        );
        assert_eq!(route(&p, "de", "es"), Some(vec![pair("de", "es")]));
    }

    /// Essai avec les vrais modèles : `REVEL_MODELS=dossier cargo test -p revel-translate -- --ignored`.
    #[test]
    #[ignore]
    fn modeles_reels() {
        let Ok(root) = std::env::var("REVEL_MODELS") else {
            return;
        };
        let e = Engine::new([PathBuf::from(root)]);
        let keep = vec!["Alunites".to_string()];
        let r = e
            .translate(
                "fr",
                "en",
                &[
                    Item {
                        text: "Alunites est une agence de communication à Lyon. Découvrez nos projets !".into(),
                        format: Format::Plain,
                    },
                    Item {
                        text: "<p>Une <strong>agence</strong> à <a href=\"/lyon/\">Lyon</a>.</p>".into(),
                        format: Format::Html,
                    },
                    Item {
                        text: "## Nos projets\n\nUn **beau** texte avec [un lien](/projets/a/).".into(),
                        format: Format::Markdown,
                    },
                ],
                &keep,
            )
            .unwrap();
        eprintln!("{r:#?}");
        assert!(r[0].starts_with("Alunites"), "{}", r[0]);
        assert!(
            r[1].contains("<strong>") && r[1].contains("href=\"/lyon/\""),
            "{}",
            r[1]
        );
        assert!(
            r[2].starts_with("## ") && r[2].contains("](/projets/a/)"),
            "{}",
            r[2]
        );
        let es = e
            .translate(
                "en",
                "es",
                &[Item {
                    text: "Discover our projects".into(),
                    format: Format::Plain,
                }],
                &[],
            )
            .unwrap();
        eprintln!("{es:?}");
        assert!(!es[0].is_empty());
    }
}
