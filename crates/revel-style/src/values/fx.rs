//! Effets réglés sans CSS (D-73) : filtres (`filter`, `backdrop-filter`), transformations
//! (`transform`), transitions, pistes d'une grille (`grid-template-columns`).

use super::{length::parse_num, split_top};

/// Fonction d'une liste (`blur(4px)`, `rotate(15deg)`) : nom et argument tel qu'écrit.
#[derive(Debug, Clone, PartialEq)]
pub struct Func {
    pub name: String,
    pub arg: String,
}

/// Lit une liste de fonctions séparées par des blancs ; `none` -> liste vide ; autre forme
/// (texte hors fonction) -> `None`.
pub fn parse_funcs(value: &str) -> Option<Vec<Func>> {
    let v = value.trim();
    if v.is_empty() || v.eq_ignore_ascii_case("none") {
        return Some(Vec::new());
    }
    split_top(v, ' ')
        .into_iter()
        .map(|t| {
            let open = t.find('(')?;
            let inner = t.strip_suffix(')')?;
            let name = t[..open].trim().to_ascii_lowercase();
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return None;
            }
            Some(Func {
                name,
                arg: inner[open + 1..].trim().to_string(),
            })
        })
        .collect()
}

/// Écrit la liste ; vide -> `none`.
pub fn format_funcs(list: &[Func]) -> String {
    if list.is_empty() {
        return "none".into();
    }
    list.iter()
        .map(|f| format!("{}({})", f.name, f.arg))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Une transition : propriété, durée, courbe, délai (tels qu'écrits).
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub prop: String,
    pub duration: String,
    pub easing: String,
    pub delay: String,
}

impl Default for Transition {
    fn default() -> Self {
        Self {
            prop: "all".into(),
            duration: "300ms".into(),
            easing: "ease".into(),
            delay: "0ms".into(),
        }
    }
}

fn is_time(t: &str) -> bool {
    parse_num(t).is_some_and(|n| matches!(n.unit.as_str(), "s" | "ms") || n.value == 0.0)
}

fn is_easing(t: &str) -> bool {
    matches!(
        t,
        "ease" | "linear" | "ease-in" | "ease-out" | "ease-in-out" | "step-start" | "step-end"
    ) || t.starts_with("cubic-bezier(")
        || t.starts_with("steps(")
        || t.starts_with("linear(")
}

/// Lit `transition` ; `none` -> liste vide ; forme non reconnue -> `None`.
pub fn parse_transitions(value: &str) -> Option<Vec<Transition>> {
    let v = value.trim();
    if v.is_empty() || v.eq_ignore_ascii_case("none") {
        return Some(Vec::new());
    }
    split_top(v, ',')
        .iter()
        .map(|layer| {
            let mut t = Transition {
                prop: String::new(),
                duration: String::new(),
                easing: String::new(),
                delay: String::new(),
            };
            for tok in split_top(layer, ' ') {
                if is_time(&tok) {
                    if t.duration.is_empty() {
                        t.duration = tok;
                    } else if t.delay.is_empty() {
                        t.delay = tok;
                    } else {
                        return None;
                    }
                } else if is_easing(&tok) && t.easing.is_empty() {
                    t.easing = tok;
                } else if t.prop.is_empty()
                    && tok
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-')
                {
                    t.prop = tok;
                } else {
                    return None;
                }
            }
            if t.prop.is_empty() {
                t.prop = "all".into();
            }
            if t.duration.is_empty() {
                t.duration = "0ms".into();
            }
            if t.easing.is_empty() {
                t.easing = "ease".into();
            }
            if t.delay.is_empty() {
                t.delay = "0ms".into();
            }
            Some(t)
        })
        .collect()
}

/// Écrit `transition` ; délai nul omis ; vide -> `none`.
pub fn format_transitions(list: &[Transition]) -> String {
    if list.is_empty() {
        return "none".into();
    }
    list.iter()
        .map(|t| {
            let mut s = format!("{} {} {}", t.prop, t.duration, t.easing);
            if parse_num(&t.delay).is_some_and(|n| n.value != 0.0) {
                s.push(' ');
                s.push_str(&t.delay);
            }
            s
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Colonnes d'une grille, lues pour le panneau.
#[derive(Debug, Clone, PartialEq)]
pub enum Tracks {
    /// `repeat(N, minmax(0, 1fr))` ou N fois `1fr` : colonnes égales.
    Equal(u32),
    /// Largeurs écrites une à une (`2fr 1fr`, `240px 1fr auto`).
    List(Vec<String>),
    /// Autant de colonnes que la place permet, d'une largeur minimale
    /// (`repeat(auto-fill, minmax(240px, 1fr))`) ; `fit` : `auto-fit` (colonnes étirées s'il en
    /// manque).
    Auto { min: String, fit: bool },
}

/// Largeur d'une colonne écrite simplement (`1fr`, `240px`, `auto`, `minmax(0, 2fr)` -> `2fr`).
fn track(t: &str) -> Option<String> {
    let t = t.trim();
    if let Some(inner) = t.strip_prefix("minmax(").and_then(|r| r.strip_suffix(')')) {
        let parts = split_top(inner, ',');
        if parts.len() == 2 && (parts[0] == "0" || parts[0] == "0px") {
            return track(&parts[1]);
        }
        return Some(t.to_string());
    }
    if matches!(t, "auto" | "min-content" | "max-content")
        || parse_num(t).is_some_and(|n| !n.unit.is_empty())
    {
        return Some(t.to_string());
    }
    None
}

/// Lit `grid-template-columns` ; forme non reconnue (noms de lignes, `subgrid`…) -> `None`.
pub fn parse_tracks(value: &str) -> Option<Tracks> {
    let v = value.trim();
    if let Some(inner) = v.strip_prefix("repeat(").and_then(|r| r.strip_suffix(')')) {
        let parts = split_top(inner, ',');
        if parts.len() != 2 {
            return None;
        }
        let what = parts[0].trim();
        if what == "auto-fill" || what == "auto-fit" {
            let m = parts[1].trim();
            let min = m
                .strip_prefix("minmax(")
                .and_then(|r| r.strip_suffix(')'))
                .map(|r| split_top(r, ','))
                .filter(|p| p.len() == 2 && p[1].trim() == "1fr")
                .map(|p| p[0].trim().to_string())?;
            return Some(Tracks::Auto {
                min,
                fit: what == "auto-fit",
            });
        }
        let n: u32 = what.parse().ok().filter(|n| *n > 0)?;
        let t = track(&parts[1])?;
        return Some(if t == "1fr" {
            Tracks::Equal(n)
        } else {
            Tracks::List(vec![t; n as usize])
        });
    }
    let list: Vec<String> = split_top(v, ' ')
        .iter()
        .map(|t| track(t))
        .collect::<Option<_>>()?;
    if list.is_empty() {
        return None;
    }
    // Liste écrite colonne par colonne : sur mesure, même si les largeurs sont égales.
    Some(Tracks::List(list))
}

/// Écrit des colonnes ; les fractions sans débordement du contenu (`minmax(0, 2fr)`).
pub fn format_tracks(t: &Tracks) -> String {
    match t {
        Tracks::Equal(n) => super::grid_columns((*n).max(1)),
        Tracks::List(list) => {
            let fr = |t: &String| {
                if parse_num(t).is_some_and(|n| n.unit == "fr") {
                    format!("minmax(0, {t})")
                } else {
                    t.clone()
                }
            };
            // Écrites une à une (relues sur mesure ; `repeat` serait relu « égales »).
            list.iter().map(fr).collect::<Vec<_>>().join(" ")
        }
        Tracks::Auto { min, fit } => format!(
            "repeat({}, minmax({min}, 1fr))",
            if *fit { "auto-fit" } else { "auto-fill" }
        ),
    }
}

/// Nombre de cases occupées (`span 2` -> 2) ; `None` : placement autre (lignes nommées…).
pub fn span_of(value: &str) -> Option<u32> {
    let v = value.trim();
    if v.is_empty() || v == "auto" {
        return Some(1);
    }
    let rest = v.strip_prefix("span")?.trim();
    let rest = rest.split('/').next()?.trim();
    rest.parse().ok().filter(|n| *n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonctions() {
        let l = parse_funcs("blur(4px) grayscale(100%)").unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].name, "blur");
        assert_eq!(l[1].arg, "100%");
        assert_eq!(format_funcs(&l), "blur(4px) grayscale(100%)");
        assert_eq!(parse_funcs("none").unwrap(), vec![]);
        assert!(parse_funcs("blur(4px) url(#f)").is_some());
        assert!(parse_funcs("4px").is_none());
        assert_eq!(
            parse_funcs("drop-shadow(0 2px 4px rgba(0, 0, 0, 0.2))").unwrap()[0].arg,
            "0 2px 4px rgba(0, 0, 0, 0.2)"
        );
    }

    #[test]
    fn transitions() {
        let l = parse_transitions("opacity 0.3s ease-out, transform 200ms cubic-bezier(.2,.8,.2,1) 50ms").unwrap();
        assert_eq!(l[0].prop, "opacity");
        assert_eq!(l[0].delay, "0ms");
        assert_eq!(l[1].easing, "cubic-bezier(.2,.8,.2,1)");
        assert_eq!(l[1].delay, "50ms");
        assert_eq!(
            format_transitions(&l),
            "opacity 0.3s ease-out, transform 200ms cubic-bezier(.2,.8,.2,1) 50ms"
        );
        assert_eq!(parse_transitions("0.2s").unwrap()[0].prop, "all");
        assert!(parse_transitions("opacity var(--d)").is_none());
    }

    #[test]
    fn colonnes() {
        assert_eq!(parse_tracks("repeat(3, minmax(0, 1fr))"), Some(Tracks::Equal(3)));
        assert_eq!(parse_tracks("1fr 1fr"), Some(Tracks::List(vec!["1fr".into(); 2])));
        assert_eq!(parse_tracks("repeat(2, 1fr)"), Some(Tracks::Equal(2)));
        assert_eq!(
            parse_tracks("minmax(0, 2fr) 1fr 240px"),
            Some(Tracks::List(vec!["2fr".into(), "1fr".into(), "240px".into()]))
        );
        assert_eq!(
            parse_tracks("repeat(auto-fill, minmax(240px, 1fr))"),
            Some(Tracks::Auto { min: "240px".into(), fit: false })
        );
        assert_eq!(parse_tracks("[a] 1fr [b] 1fr"), None);
        assert_eq!(
            format_tracks(&Tracks::List(vec!["2fr".into(), "1fr".into()])),
            "minmax(0, 2fr) minmax(0, 1fr)"
        );
        assert_eq!(format_tracks(&Tracks::List(vec!["200px".into(); 2])), "200px 200px");
        assert_eq!(
            parse_tracks(&format_tracks(&Tracks::List(vec!["1fr".into(); 2]))),
            Some(Tracks::List(vec!["1fr".into(); 2]))
        );
        assert_eq!(
            format_tracks(&Tracks::Auto { min: "18rem".into(), fit: true }),
            "repeat(auto-fit, minmax(18rem, 1fr))"
        );
        assert_eq!(span_of("span 2"), Some(2));
        assert_eq!(span_of("auto"), Some(1));
        assert_eq!(span_of("1 / 3"), None);
    }
}
