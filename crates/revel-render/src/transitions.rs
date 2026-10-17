//! Transitions entre les pages (6.4) : fondu ou glissement d'une page à l'autre du site, par les
//! transitions de vue du navigateur (CSS seul, sans script : Chrome et Edge 126, Safari 18.2 ;
//! les autres navigateurs changent de page sans transition). Coupées pour les visiteurs qui
//! demandent moins de mouvement. Pas dans le canevas de l'éditeur.

/// Style ajouté à chaque page pour la transition `kind` (`fade`, `slide`).
pub fn style(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "fade" => concat!(
            "<style>@view-transition{navigation:auto}",
            "::view-transition-group(root){animation-duration:.35s}",
            "@media (prefers-reduced-motion:reduce){@view-transition{navigation:none}}</style>"
        ),
        "slide" => concat!(
            "<style>@view-transition{navigation:auto}",
            "::view-transition-old(root){animation:.35s cubic-bezier(.4,0,.2,1) both rv-page-out}",
            "::view-transition-new(root){animation:.35s cubic-bezier(.4,0,.2,1) both rv-page-in}",
            "@keyframes rv-page-out{to{opacity:0;transform:translateX(-48px)}}",
            "@keyframes rv-page-in{from{opacity:0;transform:translateX(48px)}}",
            "@media (prefers-reduced-motion:reduce){@view-transition{navigation:none}}</style>"
        ),
        _ => return None,
    })
}

/// Page du site : style de la transition ajouté avant `</head>` (pas dans le canevas).
pub fn wire(html: String, kind: Option<&str>, scripts: bool) -> String {
    match kind.and_then(style) {
        Some(css) if scripts => crate::insert_before(&html, "</head>", css, false),
        _ => html,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_des_pages() {
        let page = "<html><head></head><body></body></html>".to_string();
        let out = wire(page.clone(), Some("slide"), true);
        assert!(out.contains("@view-transition{navigation:auto}"));
        assert!(out.contains("rv-page-in") && out.contains("prefers-reduced-motion"));
        assert!(wire(page.clone(), Some("fade"), true).contains("view-transition-group(root)"));
        assert_eq!(wire(page.clone(), Some("slide"), false), page);
        assert_eq!(wire(page.clone(), None, true), page);
        assert_eq!(wire(page.clone(), Some("spin"), true), page);
    }
}
