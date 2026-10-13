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
    // Nombres mis en forme selon la langue (Fluent ne groupe pas les milliers).
    bundle.set_formatter(Some(match code {
        "fr" => number_fr,
        _ => number_en,
    }));
    let res = FluentResource::try_new(src.to_string()).unwrap_or_else(|(res, _)| res);
    let _ = bundle.add_resource(res);
    bundle
}

fn number_fr<M>(v: &FluentValue, _: &M) -> Option<String> {
    number(v, "\u{202F}", ",")
}

fn number_en<M>(v: &FluentValue, _: &M) -> Option<String> {
    number(v, ",", ".")
}

/// Nombre avec séparateur de milliers et marque décimale (règles CLDR : groupes de trois
/// chiffres dès 1 000 ; espace fine insécable en français, virgule en anglais).
fn number(v: &FluentValue, group: &str, decimal: &str) -> Option<String> {
    let FluentValue::Number(n) = v else {
        return None;
    };
    let raw = n.as_string();
    let (sign, digits) = match raw.strip_prefix('-') {
        Some(d) => ("-", d),
        None => ("", raw.as_ref()),
    };
    let (int, frac) = digits.split_once('.').unwrap_or((digits, ""));
    let mut out = String::from(sign);
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push_str(group);
        }
        out.push(c);
    }
    if !frac.is_empty() {
        out.push_str(decimal);
        out.push_str(frac);
    }
    Some(out)
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

    #[test]
    fn nombres_selon_la_langue() {
        let fr = Localizer::new("fr");
        let en = Localizer::new("en");
        let args = [
            ("upload", "4356"),
            ("size", "716.5"),
            ("delete", "0"),
            ("total", "12"),
        ];
        let f = fr.t("publish-planned", &args);
        assert!(f.starts_with("4\u{202F}356 fichiers"), "{f}");
        assert!(f.contains("(716,5 Mo)"), "{f}");
        let e = en.t("publish-planned", &args);
        assert!(e.starts_with("4,356 files"), "{e}");
        assert!(e.contains("(716.5 MB)"), "{e}");
        assert_eq!(
            fr.t("publish-done", &[("uploaded", "0"), ("deleted", "0")]),
            "Rien à publier : le site en ligne est déjà à jour."
        );
        assert_eq!(
            fr.t("publish-done", &[("uploaded", "1"), ("deleted", "0")]),
            "Publication terminée : 1 fichier envoyé, 0 supprimé."
        );
        assert_eq!(
            en.t("publish-done", &[("uploaded", "1200"), ("deleted", "3")]),
            "Published: 1,200 files sent, 3 deleted."
        );
        assert_eq!(
            number(&FluentValue::from(-1234567.0), ",", "."),
            Some("-1,234,567".into())
        );
        assert_eq!(
            number(&FluentValue::from(999.0), ",", "."),
            Some("999".into())
        );
    }
}
