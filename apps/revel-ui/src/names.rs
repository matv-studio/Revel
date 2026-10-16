//! Noms simples des éléments (calques, fil d'Ariane, étiquettes du canevas) : la classe principale
//! rendue lisible (`hero__media` -> « Hero media »), sinon la nature de l'élément dans la langue de
//! l'interface (« Titre 1 », « Paragraphe », « Bloc »…). La balise et les classes restent en
//! info-bulle.

use crate::{Ui, icons};
use leptos::prelude::*;

/// Balises et clé de leur nom (sans la balise des titres, traitée à part).
const TAGS: &[(&str, &str)] = &[
    ("section", "el-section"),
    ("div", "el-div"),
    ("p", "el-p"),
    ("a", "el-a"),
    ("button", "el-button"),
    ("img", "el-img"),
    ("picture", "el-img"),
    ("svg", "el-svg"),
    ("video", "el-video"),
    ("audio", "el-audio"),
    ("iframe", "el-iframe"),
    ("ul", "el-ul"),
    ("ol", "el-ol"),
    ("li", "el-li"),
    ("nav", "el-nav"),
    ("header", "el-header"),
    ("footer", "el-footer"),
    ("main", "el-main"),
    ("article", "el-article"),
    ("aside", "el-aside"),
    ("form", "el-form"),
    ("input", "el-input"),
    ("textarea", "el-textarea"),
    ("select", "el-select"),
    ("label", "el-label"),
    ("span", "el-span"),
    ("strong", "el-strong"),
    ("b", "el-strong"),
    ("em", "el-em"),
    ("i", "el-em"),
    ("blockquote", "el-blockquote"),
    ("figure", "el-figure"),
    ("figcaption", "el-figcaption"),
    ("table", "el-table"),
    ("hr", "el-hr"),
    ("body", "el-body"),
    ("code", "el-code"),
    ("pre", "el-code"),
    ("small", "el-small"),
    ("time", "el-time"),
    ("address", "el-address"),
    ("details", "el-details"),
    ("summary", "el-summary"),
    ("dialog", "el-dialog"),
];

/// Niveau d'un titre (`h1` -> 1).
fn heading(tag: &str) -> Option<u8> {
    let b = tag.as_bytes();
    (b.len() == 2 && b[0].eq_ignore_ascii_case(&b'h') && (b'1'..=b'6').contains(&b[1]))
        .then(|| b[1] - b'0')
}

/// Classe principale : la dernière qui prolonge la première (`bloc bloc-hero` -> `bloc-hero`,
/// `btn btn--primary` -> `btn--primary`), sinon la première (`lien is-active` -> `lien`).
pub fn main_class(classes: &str) -> Option<&str> {
    let first = classes.split_whitespace().next()?;
    Some(
        classes
            .split_whitespace()
            .rfind(|c| c.starts_with(first))
            .unwrap_or(first),
    )
}

/// `hero__media` -> « Hero media ».
pub fn humanize(class: &str) -> String {
    let words: Vec<&str> = class.split(['-', '_']).filter(|w| !w.is_empty()).collect();
    let s = words.join(" ");
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Nom affiché d'un composant : son nom, sinon la fin de son identifiant rendue lisible
/// (`partials/card-project` -> « Card project »).
pub fn component_name(c: &revel_model::Component) -> String {
    c.name
        .clone()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| humanize(c.id.rsplit('/').next().unwrap_or(&c.id)))
}

/// Nature de l'élément (« Titre 1 », « Paragraphe »…), réactive.
pub fn type_name(ui: Ui, tag: &str) -> String {
    let tag = tag.to_ascii_lowercase();
    if let Some(level) = heading(&tag) {
        return ui.tx("el-heading", &[("level", level.to_string())]);
    }
    match TAGS.iter().find(|(t, _)| *t == tag) {
        Some((_, k)) => ui.tx(k, &[]),
        None => ui.tx("el-other", &[]),
    }
}

/// Nom affiché d'un élément : classe principale rendue lisible, sinon sa nature.
pub fn element_name(ui: Ui, tag: &str, classes: &str) -> String {
    main_class(classes)
        .map(humanize)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| type_name(ui, tag))
}

/// Nom technique (`h1.bloc-titre.grand`), pour les info-bulles.
pub fn technical(tag: &str, classes: &str) -> String {
    let mut s = tag.to_ascii_lowercase();
    for c in classes.split_whitespace() {
        s.push('.');
        s.push_str(c);
    }
    s
}

/// Info-bulle d'un élément : nature et nom technique.
pub fn tooltip(ui: Ui, tag: &str, classes: &str) -> String {
    format!("{} · {}", type_name(ui, tag), technical(tag, classes))
}

/// Icône selon la nature de l'élément.
pub fn icon(tag: &str) -> AnyView {
    let tag = tag.to_ascii_lowercase();
    if heading(&tag).is_some() {
        return icons::heading();
    }
    match tag.as_str() {
        "p" | "blockquote" | "figcaption" | "address" | "pre" | "code" => icons::text_left(),
        "span" | "strong" | "b" | "em" | "i" | "small" | "label" | "time" | "summary" => {
            icons::text()
        }
        "a" => icons::link(),
        "button" => icons::button(),
        "img" | "picture" | "figure" | "svg" => icons::image(),
        "video" | "iframe" => icons::video(),
        "audio" => icons::audio(),
        "ul" | "li" => icons::list(),
        "ol" => icons::list_ordered(),
        "form" | "input" | "textarea" | "select" => icons::field(),
        "nav" => icons::menu(),
        "section" | "header" | "footer" | "main" | "article" | "aside" => icons::section(),
        "body" => icons::page(),
        _ => icons::box_(),
    }
}

/// Noms des natures pour le canevas (étiquettes de sélection et de glisser) : balise -> nom.
pub fn canvas_labels(ui: Ui) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (t, k) in TAGS {
        map.insert((*t).to_string(), ui.now(k, &[]).into());
    }
    for level in 1..=6u8 {
        map.insert(
            format!("h{level}"),
            ui.now("el-heading", &[("level", level.to_string())]).into(),
        );
    }
    serde_json::json!({ "type": "labels", "tags": map, "other": ui.now("el-other", &[]) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classe_principale_et_nom_lisible() {
        assert_eq!(main_class("bloc bloc-hero"), Some("bloc-hero"));
        assert_eq!(main_class("btn btn--primary"), Some("btn--primary"));
        assert_eq!(main_class("lien is-active"), Some("lien"));
        assert_eq!(main_class("  "), None);
        assert_eq!(humanize("hero__media"), "Hero media");
        assert_eq!(humanize("btn--primary"), "Btn primary");
        assert_eq!(humanize("éclat"), "Éclat");
        assert_eq!(humanize("--"), "");
        assert_eq!(heading("H2"), Some(2));
        assert_eq!(heading("h7"), None);
        assert_eq!(heading("hr"), None);
        assert_eq!(technical("H1", "a  b"), "h1.a.b");
    }
}
