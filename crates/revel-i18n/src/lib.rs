//! `revel-i18n` : traductions de l'interface (anglais, français).
//!
//! Les textes sont au format Fluent (`locales/<langue>/revel.ftl`), intégrés au
//! binaire. Ajouter une langue = ajouter un dossier et une entrée dans [`LOCALES`].

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// Langues disponibles : (code, contenu Fluent).
pub const LOCALES: &[(&str, &str)] = &[
    ("en", include_str!("../locales/en/revel.ftl")),
    ("fr", include_str!("../locales/fr/revel.ftl")),
];

/// Langue utilisée si la langue demandée n'existe pas.
pub const FALLBACK: &str = "en";

pub struct Localizer {
    lang: String,
    bundle: FluentBundle<FluentResource>,
    fallback: Option<FluentBundle<FluentResource>>,
}

impl Localizer {
    /// Crée un traducteur. `lang` peut être `fr`, `fr-FR`, `en-US`…
    pub fn new(lang: &str) -> Self {
        let code = negotiate(lang);
        let fallback = (code != FALLBACK).then(|| bundle(FALLBACK));
        Self {
            lang: code.to_string(),
            bundle: bundle(code),
            fallback,
        }
    }

    pub fn lang(&self) -> &str {
        &self.lang
    }

    /// Traduit `key`. Repli sur l'anglais, puis sur la clé elle-même.
    pub fn t(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut fargs = FluentArgs::new();
        for (k, v) in args {
            // Les nombres sont passés comme nombres pour les règles de pluriel.
            match v.parse::<f64>() {
                Ok(n) => fargs.set(*k, FluentValue::from(n)),
                Err(_) => fargs.set(*k, FluentValue::from(*v)),
            }
        }
        for b in std::iter::once(&self.bundle).chain(self.fallback.as_ref()) {
            if let Some(pattern) = b.get_message(key).and_then(|m| m.value()) {
                let mut errors = vec![];
                return b
                    .format_pattern(pattern, Some(&fargs), &mut errors)
                    .into_owned();
            }
        }
        key.to_string()
    }
}

/// Choisit la langue disponible la plus proche (`fr-CA` -> `fr`).
pub fn negotiate(requested: &str) -> &'static str {
    let primary = requested
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    LOCALES
        .iter()
        .map(|(code, _)| *code)
        .find(|code| *code == primary)
        .unwrap_or(FALLBACK)
}

fn bundle(code: &str) -> FluentBundle<FluentResource> {
    let src = LOCALES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, s)| *s)
        .unwrap_or_default();
    let langid: LanguageIdentifier = code.parse().unwrap_or_default();
    let mut bundle = FluentBundle::new(vec![langid]);
    // Pas de caractères d'isolation Unicode autour des variables (texte brut).
    bundle.set_use_isolating(false);
    let res = FluentResource::try_new(src.to_string()).unwrap_or_else(|(res, _)| res);
    let _ = bundle.add_resource(res);
    bundle
}

/// Clés de messages d'un fichier Fluent (lignes `cle = ...`).
pub fn keys(src: &str) -> Vec<String> {
    let mut keys: Vec<String> = src
        .lines()
        .filter(|l| !l.starts_with([' ', '#', '.', '\t', '[', '*']) && l.contains('='))
        .filter_map(|l| l.split('=').next())
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect();
    keys.sort();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toutes_les_langues_ont_les_memes_cles() {
        let reference = keys(LOCALES[0].1);
        assert!(!reference.is_empty());
        for (code, src) in LOCALES {
            assert_eq!(keys(src), reference, "clés différentes pour {code}");
        }
    }

    #[test]
    fn les_fichiers_fluent_sont_valides() {
        for (code, src) in LOCALES {
            assert!(
                FluentResource::try_new(src.to_string()).is_ok(),
                "{code} invalide"
            );
        }
    }

    #[test]
    fn negociation_et_repli() {
        assert_eq!(negotiate("fr-FR"), "fr");
        assert_eq!(negotiate("de"), "en");
        let fr = Localizer::new("fr");
        assert_eq!(fr.t("app-title", &[]), "Revel");
        assert_eq!(fr.t("cle-inexistante", &[]), "cle-inexistante");
        assert!(
            fr.t("deploy-warn-no-php", &[("count", "1")])
                .contains("PHP")
        );
    }
}
