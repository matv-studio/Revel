//! Animations Lottie (6.2) : un élément `data-lottie="/media/x.json"` (fichier exporté d'After
//! Effects par Bodymovin ou de LottieFiles) est animé par le lecteur lottie-web (licence MIT,
//! `vendor/lottie-web/LICENSE.md`), hébergé avec le site : aucun service extérieur. Les pages qui
//! en contiennent reçoivent `/assets/revel-lottie.js` (lecteur et réglages de lecture), produit
//! une fois pour le site ; le canevas de l'éditeur le reçoit aussi (l'animation s'y voit).

use crate::OutputFile;

/// Adresse du script dans le site.
pub const URL: &str = "/assets/revel-lottie.js";

/// Lecteur lottie-web 5.13.0, rendu SVG avec expressions (`build/player/lottie_svg.min.js`).
const PLAYER: &str = include_str!("../vendor/lottie-web/lottie_svg.min.js");

/// Réglages de lecture (source commentée).
const LOADER: &str = include_str!("lottie.js");

/// Script du site : lecteur, puis réglages de lecture (sans commentaires de bloc).
pub fn script() -> String {
    let mut out = String::with_capacity(PLAYER.len() + LOADER.len() + 64);
    out.push_str("/*! lottie-web 5.13.0 | MIT License | (c) 2015 Bodymovin */\n");
    out.push_str(PLAYER.trim_end());
    out.push('\n');
    let mut in_block = false;
    for line in LOADER.lines() {
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

/// La page contient une animation Lottie.
pub fn uses(html: &str) -> bool {
    html.contains(" data-lottie=")
}

/// Page avec une animation : script ajouté avant `</head>` (aussi dans le canevas). Rend la page
/// et vrai si elle en contient.
pub fn wire(html: String) -> (String, bool) {
    if !uses(&html) {
        return (html, false);
    }
    let code = format!("<script src=\"{URL}\" defer></script>");
    (crate::insert_before(&html, "</head>", &code, false), true)
}

/// Script ajouté au site si une page au moins contient une animation.
pub fn script_file(used: bool, files: &mut Vec<OutputFile>) {
    if used {
        files.push(OutputFile {
            path: URL.trim_start_matches('/').to_string(),
            bytes: script().into_bytes(),
        });
    }
}

/// Fichier JSON d'une animation Lottie (cadence, début, fin, calques), d'après son début.
pub fn is_lottie(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(64 * 1024)]);
    let t = head.trim_start();
    t.starts_with('{')
        && ["\"fr\"", "\"ip\"", "\"op\"", "\"layers\""]
            .iter()
            .all(|k| head.contains(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_avec_animation() {
        let html =
            "<html><head></head><body><div data-lottie=\"/media/a.json\"></div></body></html>";
        let (out, used) = wire(html.to_string());
        assert!(used);
        assert!(out.contains("<script src=\"/assets/revel-lottie.js\" defer></script>\n</head>"));
        let (same, used) = wire("<html><head></head><body></body></html>".into());
        assert!(!used && !same.contains("lottie"));
        let s = script();
        assert!(s.starts_with("/*! lottie-web 5.13.0 | MIT License"));
        assert!(s.contains("window.RevelLottie"));
        assert!(!s.contains("data-lottie-play : auto"));
        assert!(is_lottie(
            br#"{"v":"5.7.4","fr":30,"ip":0,"op":60,"w":100,"h":100,"layers":[]}"#
        ));
        assert!(!is_lottie(br#"{"title":"Accueil"}"#));
    }
}
