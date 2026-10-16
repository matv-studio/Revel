//! Animations prêtes à l'emploi (6.1) : les éléments portent un attribut par moment
//! (`data-revel-appear`, `data-revel-text`, `data-revel-hover`, `data-revel-press`,
//! `data-revel-loop`, `data-revel-scroll`), réglé dans le panneau Interactions. Les pages qui en
//! contiennent reçoivent, dans `<head>`, la règle qui masque les éléments en attente d'apparition
//! (seulement quand le script est actif et que le visiteur accepte le mouvement) et le script
//! `/assets/revel-animations.js`, produit une fois pour le site.

use crate::OutputFile;

/// Adresse du script dans le site.
pub const URL: &str = "/assets/revel-animations.js";

/// Script d'animation (source commentée).
pub const SOURCE: &str = include_str!("animations.js");

/// Moments d'animation : suffixes des attributs `data-revel-…`.
pub const FAMILIES: &[&str] = &["appear", "text", "hover", "press", "loop", "scroll"];

/// Ajouté dans `<head>` : éléments à faire apparaître masqués jusqu'à leur animation, seulement si
/// le visiteur accepte le mouvement ; montrés au bout de 3 s si le script ne s'est pas chargé.
const HEAD: &str = concat!(
    "<style>html.rv-anim [data-revel-appear]:not(.rv-in),html.rv-anim [data-revel-text]:not(.rv-in){opacity:0}</style>",
    "<script>(function(d){if(window.matchMedia&&matchMedia(\"(prefers-reduced-motion: reduce)\").matches)return;",
    "d.classList.add(\"rv-anim\");setTimeout(function(){if(!window.RevelAnimations)d.classList.remove(\"rv-anim\")},3000)})(document.documentElement)</script>",
);

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

/// La page contient un élément animé.
pub fn uses(html: &str) -> bool {
    FAMILIES
        .iter()
        .any(|f| html.contains(&format!(" data-revel-{f}=")))
}

/// Page animée : règle, détection du mouvement réduit et script ajoutés avant `</head>`.
/// `scripts` faux (canevas de l'éditeur) : rien n'est ajouté, les animations ne jouent qu'à la
/// demande (aperçu du panneau Interactions). Rend la page et vrai si elle est animée.
pub fn wire(html: String, scripts: bool) -> (String, bool) {
    if !uses(&html) {
        return (html, false);
    }
    if !scripts {
        return (html, true);
    }
    let code = format!("{HEAD}<script src=\"{URL}\" defer></script>");
    (crate::insert_before(&html, "</head>", &code, false), true)
}

/// Script ajouté au site si une page au moins est animée.
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
    fn page_animee_et_script_compact() {
        let html = "<html><head><title>x</title></head><body><h1 data-revel-appear=\"up dur=0.6\">A</h1></body></html>".to_string();
        let (out, used) = wire(html.clone(), true);
        assert!(used);
        assert!(
            out.contains("<script src=\"/assets/revel-animations.js\" defer></script>\n</head>")
        );
        assert!(out.contains("prefers-reduced-motion"));
        // Canevas (scripts arrêtés) : page inchangée, animation signalée.
        assert_eq!(wire(html, false), (
            "<html><head><title>x</title></head><body><h1 data-revel-appear=\"up dur=0.6\">A</h1></body></html>".to_string(),
            true
        ));
        let plain =
            "<html><head></head><body><p data-revel-form=\"x\">a</p></body></html>".to_string();
        assert_eq!(wire(plain.clone(), true), (plain, false));
        let s = script();
        assert!(s.len() < SOURCE.len() * 4 / 5, "{}", s.len());
        assert!(!s.contains("/*") && !s.lines().any(|l| l.starts_with("//")));
        assert!(s.contains("window.RevelAnimations"));
    }
}
