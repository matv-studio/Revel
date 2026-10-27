//! Icônes des animations prêtes à l'emploi (panneau Interactions) et des transitions entre les
//! pages : un dessin qui montre le mouvement (repères fixes) et un sujet qui le joue lentement, en
//! boucle, au survol (`styles/inspector.css`, `.anim-ico`).

/// Sujet : carré arrondi au centre.
const SQ: &str = r#"<rect class="s" x="8" y="8" width="8" height="8" rx="2"/>"#;

/// Dessin d'une animation (`family` : `appear`, `text`, `hover`, `press`, `loop`, `scroll`,
/// `active`) ; `data-a` choisit le mouvement joué au survol.
pub fn icon(family: &str, preset: &str) -> String {
    let (cue, subject, anim): (&str, &str, &str) = match (family, preset) {
        ("text", p) => (
            "",
            r#"<text class="s" x="12" y="15.5" text-anchor="middle">Abc</text>"#,
            match p {
                "up" => "rise",
                "typewriter" => "type",
                "blur" => "blur",
                "pop" => "pop",
                "flip" => "flip",
                _ => "fade",
            },
        ),
        (_, "fade" | "fade-in") => (
            r#"<rect class="ghost" x="8" y="8" width="8" height="8" rx="2"/>"#,
            SQ,
            "fade",
        ),
        (_, "fade-out") => (
            r#"<rect class="ghost" x="8" y="8" width="8" height="8" rx="2"/>"#,
            SQ,
            "fade-out",
        ),
        (_, "up") => (
            r#"<rect class="ghost" x="8" y="13" width="8" height="8" rx="2"/><path class="cue" d="M12 6V3m-2 2 2-2 2 2"/>"#,
            SQ,
            "rise",
        ),
        (_, "down") => (
            r#"<rect class="ghost" x="8" y="3" width="8" height="8" rx="2"/><path class="cue" d="M12 18v3m-2-2 2 2 2-2"/>"#,
            SQ,
            "drop",
        ),
        (_, "left") => (
            r#"<rect class="ghost" x="3" y="8" width="8" height="8" rx="2"/><path class="cue" d="M18 12h3m-2-2 2 2-2 2"/>"#,
            SQ,
            "from-left",
        ),
        (_, "right") => (
            r#"<rect class="ghost" x="13" y="8" width="8" height="8" rx="2"/><path class="cue" d="M6 12H3m2-2-2 2 2 2"/>"#,
            SQ,
            "from-right",
        ),
        (_, "slide-x") => (
            r#"<path class="cue" d="M3 20h18m-2-2 2 2-2 2"/>"#,
            SQ,
            "slide",
        ),
        ("scroll", "zoom-in") | (_, "zoom-in") | (_, "grow") => (
            r#"<path class="cue" d="M4 7V4h3M20 7V4h-3M4 17v3h3M20 17v3h-3"/>"#,
            SQ,
            if preset == "grow" { "grow" } else { "zoom-in" },
        ),
        (_, "zoom-out") | (_, "shrink") => (
            r#"<rect class="ghost" x="4" y="4" width="16" height="16" rx="3"/><path class="cue" d="m5 5 2 2m12-2-2 2M5 19l2-2m12 2-2-2"/>"#,
            SQ,
            if preset == "shrink" {
                "shrink"
            } else {
                "zoom-out"
            },
        ),
        (_, "blur") => (
            r#"<rect class="halo" x="5" y="5" width="14" height="14" rx="4"/>"#,
            SQ,
            "blur",
        ),
        (_, "pop") => (
            r#"<path class="cue" d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6 7 7M17 17l1.4 1.4M5.6 18.4 7 17M17 7l1.4-1.4"/>"#,
            SQ,
            "pop",
        ),
        (_, "flip") => (
            r#"<path class="cue" d="M4 5.5c4-2.4 12-2.4 16 0m-2-1.6 2 1.6-2 1.2"/>"#,
            SQ,
            "flip",
        ),
        (_, "rotate") | (_, "spin") => (
            r#"<path class="cue" d="M19.5 9A8 8 0 1 0 20 13.5m-.5-4.5-.5-3m.5 3-3 .3"/>"#,
            SQ,
            "spin",
        ),
        (_, "reveal") => (
            r#"<rect class="ghost" x="6" y="6" width="12" height="12" rx="2"/><path class="cue" d="M12 4v16"/>"#,
            r#"<rect class="s" x="6" y="6" width="12" height="12" rx="2"/>"#,
            "reveal",
        ),
        (_, "lift") => (
            r#"<ellipse class="shade" cx="12" cy="20" rx="5" ry="1.4"/>"#,
            SQ,
            "lift",
        ),
        (_, "shadow") => (
            r#"<rect class="shade" x="10" y="10" width="8" height="8" rx="2"/>"#,
            SQ,
            "shadow",
        ),
        (_, "dim") => (
            r#"<rect class="ghost" x="8" y="8" width="8" height="8" rx="2"/>"#,
            SQ,
            "dim",
        ),
        (_, "brighten") => (
            r#"<path class="cue" d="M12 3v2M12 19v2M3 12h2M19 12h2"/>"#,
            SQ,
            "brighten",
        ),
        (_, "tilt") => (
            r#"<rect class="ghost" x="8" y="8" width="8" height="8" rx="2"/>"#,
            SQ,
            "tilt",
        ),
        (_, "underline") => (
            r#"<path class="cue-line" d="M5 17h14"/>"#,
            r#"<text class="s txt" x="12" y="14.5" text-anchor="middle">Lien</text>"#,
            "underline",
        ),
        (_, "press") => (
            r#"<path class="cue" d="M12 2.5v3m-2-2 2 2 2-2"/>"#,
            SQ,
            "press",
        ),
        (_, "squish") => (
            r#"<path class="cue" d="M4 12h2m14 0h-2M12 3v3m0 12v3"/>"#,
            SQ,
            "squish",
        ),
        (_, "push") => (
            r#"<rect class="ghost" x="7" y="7" width="10" height="10" rx="2.5"/>"#,
            SQ,
            "push",
        ),
        (_, "pulse") | (_, "heartbeat") => (
            r#"<rect class="ghost" x="5" y="5" width="14" height="14" rx="3.5"/>"#,
            SQ,
            if preset == "heartbeat" {
                "heartbeat"
            } else {
                "pulse"
            },
        ),
        (_, "float") => (
            r#"<path class="cue" d="M4 20c2-1.5 4-1.5 6 0s4 1.5 6 0 4-1.5 4 0"/>"#,
            SQ,
            "float",
        ),
        (_, "swing") => (
            r#"<path class="cue" d="M12 2v4"/><path class="cue" d="M7 20c3 1.5 7 1.5 10 0"/>"#,
            SQ,
            "swing",
        ),
        (_, "shake") => (
            r#"<path class="cue" d="M4 9v6M20 9v6M2 11v2M22 11v2"/>"#,
            SQ,
            "shake",
        ),
        (_, "blink") | (_, "flash") => (
            r#"<path class="cue" d="M12 3v2M12 19v2M3 12h2M19 12h2"/>"#,
            SQ,
            "blink",
        ),
        (_, "bounce") | (_, "jump") => (
            r#"<path class="cue" d="M4 21h16"/><path class="cue-dash" d="M6 18c2-6 4-6 6 0 2-6 4-6 6 0"/>"#,
            SQ,
            "bounce",
        ),
        (_, "ticker") => (
            "",
            r#"<g class="s"><circle cx="3" cy="12" r="1.6"/><circle cx="9" cy="12" r="1.6"/><circle cx="15" cy="12" r="1.6"/><circle cx="21" cy="12" r="1.6"/><circle cx="27" cy="12" r="1.6"/></g>"#,
            "ticker",
        ),
        (_, "parallax") => (
            r#"<rect class="ghost" x="4" y="4" width="16" height="16" rx="3"/>"#,
            SQ,
            "parallax",
        ),
        _ => ("", SQ, "fade"),
    };
    format!(
        r#"<svg class="anim-ico" viewBox="0 0 24 24" data-a="{anim}" aria-hidden="true">{cue}{subject}</svg>"#
    )
}

/// Dessin d'une transition entre les pages : deux pages, la nouvelle entre selon l'effet.
pub fn transition(effect: &str) -> String {
    let anim = match effect {
        "" | "none" => "none",
        e => e,
    };
    format!(
        r#"<svg class="anim-ico tr-ico" viewBox="0 0 32 24" data-t="{anim}" aria-hidden="true"><rect class="tr-old" x="3" y="3" width="26" height="18" rx="2.5"/><g class="tr-new"><rect x="3" y="3" width="26" height="18" rx="2.5"/><path d="M7 8h10M7 12h14M7 16h8"/></g></svg>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_animation_a_son_dessin() {
        use std::collections::HashSet;
        let appear = [
            "fade", "up", "down", "left", "right", "zoom-in", "zoom-out", "blur", "pop", "flip",
            "rotate", "reveal",
        ];
        let icons: HashSet<String> = appear.iter().map(|p| icon("appear", p)).collect();
        assert_eq!(icons.len(), appear.len(), "dessins en double");
        assert!(icon("loop", "ticker").contains(r#"data-a="ticker""#));
        assert!(transition("slide").contains(r#"data-t="slide""#));
    }
}
