//! Styles propres aux outils de Revel (D-151) : ce qu'un outil exige pour fonctionner (piste d'un
//! carrousel qui défile de côté), écrit par Revel dans la page qui l'emploie, sur le site comme dans
//! le canevas, avec une priorité nulle (`:where`) : les styles du site, réglés dans le panneau,
//! passent toujours devant. L'utilisateur n'a jamais à les écrire.

/// (attribut qui signale l'outil, règles).
const TOOLS: &[(&str, &str)] = &[(
    " data-revel-track",
    ":where([data-revel-track]){overflow-x:auto;scroll-snap-type:x mandatory;scrollbar-width:none;overscroll-behavior-x:contain}\
:where([data-revel-track])::-webkit-scrollbar{display:none}\
:where([data-revel-track]>*){scroll-snap-align:start}",
)];

/// Règles des outils employés par la page.
fn css(html: &str) -> String {
    TOOLS
        .iter()
        .filter(|(attr, _)| html.contains(attr))
        .map(|(_, css)| *css)
        .collect()
}

/// Feuille ajoutée au début de `<head>` (avant celles du site).
pub fn wire(html: String) -> String {
    let rules = css(&html);
    if rules.is_empty() {
        return html;
    }
    let tag = format!("<style data-revel-tools>{rules}</style>");
    // `<head>` peut porter des attributs (repères du canevas).
    let open = html
        .find("<head>")
        .or_else(|| html.find("<head "))
        .and_then(|i| html[i..].find('>').map(|j| i + j + 1));
    match open {
        Some(i) => format!("{}{tag}{}", &html[..i], &html[i..]),
        None => html,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_des_outils() {
        let page = "<html><head><link rel=\"stylesheet\" href=\"/s.css\"></head><body><div data-revel-track>x</div></body></html>".to_string();
        let out = wire(page);
        let s = out.find("data-revel-tools").unwrap();
        assert!(s < out.find("/s.css").unwrap() && out.contains(":where([data-revel-track])"));
        let plain = "<html><head></head><body></body></html>".to_string();
        assert_eq!(wire(plain.clone()), plain);
        let canvas = wire(
            "<html><head data-rv=\"h\"></head><body><div data-revel-track></div></body></html>"
                .into(),
        );
        assert!(canvas.starts_with("<html><head data-rv=\"h\"><style data-revel-tools>"));
    }
}
