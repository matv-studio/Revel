//! Valeurs CSS pour le panneau de styles (D-20) : couleurs, nombres avec unité, raccourcis
//! (`margin: 0 auto`, `border: 1px solid #ddd`, `background: #fff`), ombres.
//!
//! Code sans dépendance, partagé par l'interface (WebAssembly) et vérifié par `cargo test`.

pub mod color;
pub mod fx;
pub mod length;
pub mod shadow;
pub mod shorthand;

#[cfg(test)]
mod tests;

pub use color::{Hsva, Rgba, contrast};
pub use fx::{
    Func, Tracks, Transition, format_funcs, format_tracks, format_transitions, parse_funcs,
    parse_tracks, parse_transitions, span_of,
};
pub use length::{Num, clamp_non_negative, eval, fmt_num, non_negative, step};
pub use shadow::{Shadow, format_shadows, format_text_shadows, parse_shadows};
pub use shorthand::{Edit, Found, edits, read};

/// Découpe `s` au premier niveau (hors parenthèses et guillemets) sur `sep` ; `' '` découpe sur
/// tout blanc et ignore les morceaux vides.
pub fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    for c in s.chars() {
        if let Some(q) = quote {
            cur.push(c);
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                quote = Some(c);
                cur.push(c);
            }
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            c if depth == 0 && (c == sep || (sep == ' ' && c.is_whitespace())) => {
                out.push(std::mem::take(&mut cur));
            }
            c => cur.push(c),
        }
    }
    out.push(cur);
    if sep == ' ' {
        out.retain(|t| !t.trim().is_empty());
    }
    out.into_iter().map(|t| t.trim().to_string()).collect()
}

/// Première famille d'une liste de polices, sans guillemets (`"Geist", sans-serif` -> `Geist`).
pub fn first_family(value: &str) -> String {
    split_top(value, ',')
        .into_iter()
        .next()
        .unwrap_or_default()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string()
}

/// Nombre de colonnes d'une grille : `repeat(3, 1fr)` ou `1fr 1fr 1fr` -> 3 ; autre forme -> `None`.
pub fn grid_count(value: &str) -> Option<u32> {
    let v = value.trim();
    if let Some(inner) = v.strip_prefix("repeat(").and_then(|r| r.strip_suffix(')')) {
        let parts = split_top(inner, ',');
        return parts.first()?.trim().parse().ok().filter(|n| *n > 0);
    }
    let tracks = split_top(v, ' ');
    let simple = |t: &str| {
        t == "auto"
            || t.starts_with("minmax(")
            || length::parse_num(t).is_some_and(|n| !n.unit.is_empty())
    };
    (!tracks.is_empty() && tracks.iter().all(|t| simple(t))).then_some(tracks.len() as u32)
}

/// Colonnes de même largeur, sans débordement du contenu.
pub fn grid_columns(n: u32) -> String {
    format!("repeat({n}, minmax(0, 1fr))")
}

/// Valeur d'appel de variable (`var(--x)`).
pub fn is_var(token: &str) -> bool {
    token.starts_with("var(") && token.ends_with(')')
}
