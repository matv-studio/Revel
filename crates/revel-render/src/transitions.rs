//! Transitions entre les pages (6.4, D-142) : fondu, glissements, poussée, zoom, flou, volet, rideau
//! ou bascule d'une page à l'autre du site, par les
//! transitions de vue du navigateur (CSS seul, sans script : Chrome et Edge 126, Safari 18.2 ;
//! les autres navigateurs changent de page sans transition). Coupées pour les visiteurs qui
//! demandent moins de mouvement. Pas dans le canevas de l'éditeur.

/// Transitions proposées, dans l'ordre de l'interface.
pub const KINDS: &[&str] = &[
    "fade", "slide", "slide-up", "push", "zoom", "blur", "wipe", "curtain", "flip",
];

/// Style ajouté à chaque page pour la transition `kind` (voir [`KINDS`]).
pub fn style(kind: &str) -> Option<&'static str> {
    // Ancienne page (`old`) et nouvelle (`new`) : animations propres à chaque effet.
    macro_rules! vt {
        ($dur:literal, $old:literal, $new:literal, $kf:literal) => {
            concat!(
                "<style>@view-transition{navigation:auto}",
                "::view-transition-old(root){animation:",
                $dur,
                " cubic-bezier(.4,0,.2,1) both ",
                $old,
                "}",
                "::view-transition-new(root){animation:",
                $dur,
                " cubic-bezier(.4,0,.2,1) both ",
                $new,
                "}",
                $kf,
                "@media (prefers-reduced-motion:reduce){@view-transition{navigation:none}}</style>"
            )
        };
    }
    Some(match kind {
        "fade" => concat!(
            "<style>@view-transition{navigation:auto}",
            "::view-transition-group(root){animation-duration:.35s}",
            "@media (prefers-reduced-motion:reduce){@view-transition{navigation:none}}</style>"
        ),
        "slide" => vt!(
            ".35s",
            "rv-page-out",
            "rv-page-in",
            "@keyframes rv-page-out{to{opacity:0;transform:translateX(-48px)}}@keyframes rv-page-in{from{opacity:0;transform:translateX(48px)}}"
        ),
        "slide-up" => vt!(
            ".4s",
            "rv-page-out",
            "rv-page-in",
            "@keyframes rv-page-out{to{opacity:0;transform:translateY(-40px)}}@keyframes rv-page-in{from{opacity:0;transform:translateY(40px)}}"
        ),
        "push" => vt!(
            ".5s",
            "rv-page-out",
            "rv-page-in",
            "@keyframes rv-page-out{to{transform:translateX(-100%)}}@keyframes rv-page-in{from{transform:translateX(100%)}}"
        ),
        "zoom" => vt!(
            ".4s",
            "rv-page-out",
            "rv-page-in",
            "@keyframes rv-page-out{to{opacity:0;transform:scale(.94)}}@keyframes rv-page-in{from{opacity:0;transform:scale(1.06)}}"
        ),
        "blur" => vt!(
            ".45s",
            "rv-page-out",
            "rv-page-in",
            "@keyframes rv-page-out{to{opacity:0;filter:blur(12px)}}@keyframes rv-page-in{from{opacity:0;filter:blur(12px)}}"
        ),
        "wipe" => vt!(
            ".55s",
            "none",
            "rv-page-in",
            "@keyframes rv-page-in{from{clip-path:inset(0 100% 0 0)}to{clip-path:inset(0)}}"
        ),
        "curtain" => vt!(
            ".6s",
            "rv-page-out",
            "rv-page-in",
            "@keyframes rv-page-out{to{opacity:.4;transform:scale(.97)}}@keyframes rv-page-in{from{transform:translateY(100%)}}"
        ),
        "flip" => vt!(
            ".6s",
            "rv-page-out",
            "rv-page-in",
            "@keyframes rv-page-out{0%{transform:perspective(1600px) rotateY(0)}50%,100%{opacity:0;transform:perspective(1600px) rotateY(90deg)}}@keyframes rv-page-in{0%,50%{opacity:0;transform:perspective(1600px) rotateY(-90deg)}100%{transform:perspective(1600px) rotateY(0)}}"
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
        for k in KINDS {
            assert!(
                style(k).is_some_and(|s| s.contains("@view-transition")),
                "{k}"
            );
        }
        assert_eq!(wire(page.clone(), Some("slide"), false), page);
        assert_eq!(wire(page.clone(), None, true), page);
        assert_eq!(wire(page.clone(), Some("spin"), true), page);
    }
}
