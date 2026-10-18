//! Ombres (`box-shadow`, `text-shadow`) : liste de calques modifiables un par un.

use super::{color::is_color, is_var, length::parse_num, split_top};

/// Un calque d'ombre. Les longueurs gardent leur écriture (`0`, `4px`, `.5rem`).
#[derive(Debug, Clone, PartialEq)]
pub struct Shadow {
    /// Ombre intérieure (`inset`).
    pub inset: bool,
    pub x: String,
    pub y: String,
    pub blur: String,
    pub spread: String,
    /// Couleur telle qu'écrite (vide : couleur du texte).
    pub color: String,
}

impl Default for Shadow {
    /// Ombre portée douce, proposée à l'ajout.
    fn default() -> Self {
        Self {
            inset: false,
            x: "0px".into(),
            y: "4px".into(),
            blur: "12px".into(),
            spread: "0px".into(),
            color: "rgba(0, 0, 0, 0.15)".into(),
        }
    }
}

fn is_length(t: &str) -> bool {
    parse_num(t).is_some_and(|n| !n.unit.is_empty() || n.value == 0.0) || t.starts_with("calc(")
}

/// Lit une valeur de `box-shadow` ; `none` -> liste vide ; forme non reconnue -> `None`.
pub fn parse_shadows(value: &str) -> Option<Vec<Shadow>> {
    let v = value.trim();
    if v.eq_ignore_ascii_case("none") {
        return Some(Vec::new());
    }
    split_top(v, ',')
        .iter()
        .map(|layer| {
            let mut s = Shadow {
                inset: false,
                x: String::new(),
                y: String::new(),
                blur: "0px".into(),
                spread: "0px".into(),
                color: String::new(),
            };
            let mut lengths = Vec::new();
            for t in split_top(layer, ' ') {
                if t.eq_ignore_ascii_case("inset") && !s.inset {
                    s.inset = true;
                } else if is_length(&t) && lengths.len() < 4 {
                    lengths.push(t);
                } else if s.color.is_empty() && (is_color(&t) || is_var(&t)) {
                    s.color = t;
                } else {
                    return None;
                }
            }
            if lengths.len() < 2 {
                return None;
            }
            let mut it = lengths.into_iter();
            s.x = it.next()?;
            s.y = it.next()?;
            if let Some(b) = it.next() {
                s.blur = b;
            }
            if let Some(sp) = it.next() {
                s.spread = sp;
            }
            Some(s)
        })
        .collect()
}

/// Écrit la liste ; vide -> `none`.
pub fn format_shadows(list: &[Shadow]) -> String {
    if list.is_empty() {
        return "none".into();
    }
    list.iter()
        .map(|s| {
            let mut parts = Vec::new();
            if s.inset {
                parts.push("inset".to_string());
            }
            parts.extend([s.x.clone(), s.y.clone(), s.blur.clone(), s.spread.clone()]);
            if !s.color.is_empty() {
                parts.push(s.color.clone());
            }
            parts.join(" ")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Écrit une ombre de texte (`text-shadow` : ni intérieure ni étendue) ; vide -> `none`.
pub fn format_text_shadows(list: &[Shadow]) -> String {
    if list.is_empty() {
        return "none".into();
    }
    list.iter()
        .map(|s| {
            let mut parts = vec![s.x.clone(), s.y.clone(), s.blur.clone()];
            if !s.color.is_empty() {
                parts.push(s.color.clone());
            }
            parts.join(" ")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod text_tests {
    use super::*;

    #[test]
    fn ombre_de_texte() {
        let v = "0 1px 6px rgba(0,0,0,0.5), 0 -1px 6px rgba(0,0,0,0.5)";
        let l = parse_shadows(v).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(
            format_text_shadows(&l),
            "0 1px 6px rgba(0,0,0,0.5), 0 -1px 6px rgba(0,0,0,0.5)"
        );
        assert_eq!(format_text_shadows(&[]), "none");
    }
}
