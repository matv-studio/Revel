//! Interactions entre éléments (6.3) : éléments `data-revel-on` (déclencheur, action, élément
//! visé ; voir `revel_edit::actions`) et onglets `data-revel-tabs`. Les pages qui en contiennent
//! reçoivent `/assets/revel-actions.js`, produit une fois pour le site ; pas dans le canevas de
//! l'éditeur (un clic y sélectionne), mais dans l'aperçu.

use crate::OutputFile;

/// Adresse du script dans le site.
pub const URL: &str = "/assets/revel-actions.js";

/// Script (source commentée).
const SOURCE: &str = include_str!("actions.js");

/// Script sans commentaires ni indentation (le site publié).
pub fn script() -> String {
    let mut out = String::with_capacity(SOURCE.len());
    let mut in_block = false;
    for line in SOURCE.lines() {
        let t = line.trim();
        if in_block {
            if t.ends_with("*/") {
                in_block = false;
            }
            continue;
        }
        if t.starts_with("/*") {
            in_block = !t.ends_with("*/");
            continue;
        }
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        out.push_str(t);
        out.push('\n');
    }
    out
}

/// La page contient une interaction, des onglets, un carrousel, un diaporama ou un lecteur vidéo.
pub fn uses(html: &str) -> bool {
    html.contains(" data-revel-on=")
        || html.contains(" data-revel-tabs")
        || html.contains(" data-revel-slider")
        || html.contains(" data-revel-video")
        || html.contains(" data-revel-filters")
        || html.contains(" data-revel-consent")
        || html.contains(" data-revel-switch")
        || html.contains(" data-revel-hover-media")
        || html.contains(" data-revel-reveal")
}

/// Page avec des interactions : script ajouté avant `</head>` (pas dans le canevas). Rend la page
/// et vrai si elle en contient.
pub fn wire(html: String, scripts: bool) -> (String, bool) {
    if !uses(&html) {
        return (html, false);
    }
    if !scripts {
        return (html, true);
    }
    let mut code = format!("<script src=\"{URL}\" defer></script>");
    // Bandeau de consentement : masqué dès l'affichage, ouvert par le script s'il le faut.
    if html.contains(" data-revel-consent=") || html.contains(" data-revel-consent>") {
        code.push_str("<style>[data-revel-consent]:not(.rv-open){display:none}</style>");
    }
    (crate::insert_before(&html, "</head>", &code, false), true)
}

/// Script ajouté au site si une page au moins contient une interaction.
pub fn script_file(used: bool, files: &mut Vec<OutputFile>) {
    if used {
        files.push(OutputFile {
            path: URL.trim_start_matches('/').to_string(),
            bytes: script().into_bytes(),
        });
    }
}

/// Média au survol (`data-revel-hover-media`) : image d'attente (vidéo) ou image (image) du
/// premier élément survolé écrite dans le média, visible sans attendre le script ni sans
/// JavaScript ; le média suit ainsi les contenus de la liste qui le pilote.
pub(crate) fn prefill_hover(html: &str) -> String {
    if !html.contains(" data-revel-hover-media=") {
        return html.to_string();
    }
    let disp_re =
        regex::Regex::new(r#"(?s)<(video|img)\b[^>]*?\sdata-revel-hover-media="([^"]+)"[^>]*>"#)
            .expect("regex");
    let value = |tag: &str, name: &str| -> Option<String> {
        regex::Regex::new(&format!(r#"\s{}="([^"]*)""#, regex::escape(name)))
            .expect("regex")
            .captures(tag)
            .map(|c| c[1].to_string())
    };
    let first_trigger = |name: &str| -> Option<String> {
        let re = regex::Regex::new(&format!(
            r#"(?s)<[a-zA-Z][^>]*\sdata-revel-media-for="{}"[^>]*>"#,
            regex::escape(name)
        ))
        .expect("regex");
        re.find(html).map(|m| m.as_str().to_string())
    };
    disp_re
        .replace_all(html, |c: &regex::Captures| {
            let tag = c[0].to_string();
            let Some(first) = first_trigger(&c[2]) else {
                return tag;
            };
            let (attr, v) = if c[1].eq_ignore_ascii_case("video") {
                ("poster", value(&first, "data-revel-media-poster"))
            } else {
                ("src", value(&first, "data-revel-media"))
            };
            let Some(v) = v.filter(|v| !v.trim().is_empty()) else {
                return tag;
            };
            let current = value(&tag, attr);
            if current.as_deref().is_some_and(|x| !x.trim().is_empty()) {
                return tag;
            }
            let mut out = match current {
                // Attribut vide (`src=""`) : remplacé.
                Some(_) => regex::Regex::new(&format!(r#"\s{attr}="[^"]*""#))
                    .expect("regex")
                    .replace(&tag, "")
                    .into_owned(),
                None => tag,
            };
            let at = if out.ends_with("/>") {
                out.len() - 2
            } else {
                out.len() - 1
            };
            out.insert_str(at, &format!(" {attr}=\"{v}\""));
            out
        })
        .into_owned()
}

#[cfg(test)]
mod tests {

    #[test]
    fn media_au_survol_prerempli() {
        let html = r#"<div><video class="v" muted data-revel-hover-media="h"></video><ol><li><a href="/a/" data-revel-media-for="h" data-revel-media="/a.mp4" data-revel-media-poster="/a.jpg?x=1&amp;y=2">A</a></li><li><a data-revel-media-for="h" data-revel-media="/b.mp4" data-revel-media-poster="/b.jpg">B</a></li></ol><img data-revel-hover-media="i" src=""><p data-revel-media-for="i" data-revel-media="/c.png">C</p></div>"#;
        let out = prefill_hover(html);
        assert!(
            out.contains(
                r#"<video class="v" muted data-revel-hover-media="h" poster="/a.jpg?x=1&amp;y=2">"#
            ),
            "{out}"
        );
        assert!(
            out.contains(r#"<img data-revel-hover-media="i" src="/c.png">"#),
            "{out}"
        );
        assert_eq!(prefill_hover("<p>x</p>"), "<p>x</p>");
        assert!(uses(r#"<video data-revel-hover-media="h">"#));
        assert!(uses(r#"<div class="revele" data-revel-reveal="spot">"#));
    }
    use super::*;

    #[test]
    fn page_avec_interactions() {
        let html = "<html><head></head><body><button data-revel-on=\"click class:is-open #menu\">M</button></body></html>";
        let (out, used) = wire(html.to_string(), true);
        assert!(
            used && out
                .contains("<script src=\"/assets/revel-actions.js\" defer></script>\n</head>")
        );
        let (canvas, used) = wire(html.to_string(), false);
        assert!(used && !canvas.contains("revel-actions"));
        let tabs =
            "<html><head></head><body><div class=\"onglets\" data-revel-tabs></div></body></html>";
        assert!(wire(tabs.to_string(), true).1);
        assert!(!wire("<html><head></head><body></body></html>".into(), true).1);
        // Interrupteur des sons : le script est ajouté.
        let sw =
            "<html><head></head><body><button data-revel-switch=\"music\">M</button></body></html>";
        assert!(wire(sw.to_string(), true).1);
        let s = script();
        assert!(s.contains("data-revel-on") && !s.contains("Sans JavaScript, la page"));
        assert!(s.contains("rv-switch:") && s.contains("sound=") && s.contains("music:"));
    }
}
