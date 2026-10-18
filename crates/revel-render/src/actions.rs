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

#[cfg(test)]
mod tests {
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
