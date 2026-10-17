//! Raccourcis CSS lus et modifiés sur place (D-20) : `margin: 0 auto` se règle côté par côté sans
//! être réécrit en quatre déclarations, `border: 1px solid #ddd` par épaisseur, style et couleur,
//! `background: #fff` par sa couleur.

use revel_model::Decl;

use super::{color::is_color, is_var, length::parse_num, split_top};

/// Valeur d'une propriété dans une règle.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    /// Valeur ; `None` : fixée par un raccourci que Revel ne décompose pas (`margin: var(--m)`).
    pub value: Option<String>,
    /// Raccourci qui porte la valeur (`margin`) ; `None` : déclaration de la propriété elle-même.
    pub via: Option<String>,
    pub important: bool,
}

/// Déclaration à écrire (`SetDecl`) : valeur `None` = retirée.
#[derive(Debug, Clone, PartialEq)]
pub struct Edit {
    pub prop: String,
    pub value: Option<String>,
    pub important: bool,
}

/// Raccourcis à quatre côtés (haut, droite, bas, gauche) ou quatre angles.
const FOUR: &[(&str, [&str; 4])] = &[
    (
        "margin",
        ["margin-top", "margin-right", "margin-bottom", "margin-left"],
    ),
    (
        "padding",
        [
            "padding-top",
            "padding-right",
            "padding-bottom",
            "padding-left",
        ],
    ),
    ("inset", ["top", "right", "bottom", "left"]),
    (
        "border-width",
        [
            "border-top-width",
            "border-right-width",
            "border-bottom-width",
            "border-left-width",
        ],
    ),
    (
        "border-style",
        [
            "border-top-style",
            "border-right-style",
            "border-bottom-style",
            "border-left-style",
        ],
    ),
    (
        "border-color",
        [
            "border-top-color",
            "border-right-color",
            "border-bottom-color",
            "border-left-color",
        ],
    ),
    (
        "border-radius",
        [
            "border-top-left-radius",
            "border-top-right-radius",
            "border-bottom-right-radius",
            "border-bottom-left-radius",
        ],
    ),
];

const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];
const BORDER_STYLES: &[&str] = &[
    "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset", "outset",
];
const LINES: &[&str] = &["none", "underline", "overline", "line-through", "blink"];
/// Propriétés fixées par `font` (non décomposé).
const FONT: &[&str] = &[
    "font-style",
    "font-variant",
    "font-weight",
    "font-stretch",
    "font-size",
    "line-height",
    "font-family",
];

/// Quatre valeurs d'un raccourci (`0 auto` -> `0 auto 0 auto`). Variable ou forme elliptique
/// (`10px / 20px`) : non décomposé.
pub fn expand4(v: &str) -> Option<[String; 4]> {
    if v.contains("var(") || split_top(v, '/').len() > 1 {
        return None;
    }
    let t = split_top(v, ' ');
    Some(match t.as_slice() {
        [a] => [a.clone(), a.clone(), a.clone(), a.clone()],
        [a, b] => [a.clone(), b.clone(), a.clone(), b.clone()],
        [a, b, c] => [a.clone(), b.clone(), c.clone(), b.clone()],
        [a, b, c, d] => [a.clone(), b.clone(), c.clone(), d.clone()],
        _ => return None,
    })
}

/// Écriture la plus courte de quatre valeurs.
pub fn compact4(v: &[String; 4]) -> String {
    let [a, b, c, d] = v;
    if b == d {
        if a == c {
            if a == b {
                a.clone()
            } else {
                format!("{a} {b}")
            }
        } else {
            format!("{a} {b} {c}")
        }
    } else {
        format!("{a} {b} {c} {d}")
    }
}

fn is_border_width(t: &str) -> bool {
    matches!(t, "thin" | "medium" | "thick")
        || parse_num(t).is_some_and(|n| !n.unit.is_empty() || n.value == 0.0)
        || t.starts_with("calc(")
}

/// Épaisseur, style et couleur d'une valeur de `border` (absents : `None`).
fn border_parts(v: &str) -> Option<[Option<String>; 3]> {
    let mut parts: [Option<String>; 3] = [None, None, None];
    for t in split_top(v, ' ') {
        let lower = t.to_ascii_lowercase();
        let slot = if BORDER_STYLES.contains(&lower.as_str()) {
            1
        } else if is_border_width(&lower) {
            0
        } else if is_color(&t) || is_var(&t) {
            2
        } else {
            return None;
        };
        if parts[slot].is_some() {
            return None;
        }
        parts[slot] = Some(t);
    }
    Some(parts)
}

/// Partie (0 épaisseur, 1 style, 2 couleur) désignée par `prop` dans le raccourci `sh`
/// (`border`, `border-top`).
fn border_part(sh: &str, prop: &str) -> Option<usize> {
    let part = |rest: &str| match rest {
        "width" => Some(0),
        "style" => Some(1),
        "color" => Some(2),
        _ => None,
    };
    if sh == "border" {
        if let Some(rest) = prop.strip_prefix("border-") {
            if let Some(i) = part(rest) {
                return Some(i);
            }
            let (side, rest) = rest.split_once('-')?;
            return SIDES.contains(&side).then(|| part(rest)).flatten();
        }
        return None;
    }
    let side = sh.strip_prefix("border-")?;
    if !SIDES.contains(&side) {
        return None;
    }
    part(prop.strip_prefix(sh)?.strip_prefix('-')?)
}

/// Couleur du dernier calque d'un `background` (indice du mot), si présente.
fn background_color_at(v: &str) -> (Vec<String>, Vec<String>, Option<usize>) {
    let mut layers = split_top(v, ',');
    let last = layers.pop().unwrap_or_default();
    let tokens = split_top(&last, ' ');
    let at = tokens
        .iter()
        .position(|t| is_color(t))
        .or_else(|| (tokens.len() == 1 && is_var(&tokens[0])).then_some(0));
    (layers, tokens, at)
}

fn background_compose(layers: Vec<String>, tokens: Vec<String>) -> Option<String> {
    let last = tokens.join(" ");
    let mut all = layers;
    if !last.is_empty() {
        all.push(last);
    }
    (!all.is_empty()).then(|| all.join(", "))
}

/// Valeur de `prop` portée par la déclaration `sh: v` : `None` si `sh` ne la fixe pas,
/// `Some(None)` si elle la fixe sans que Revel sache la lire.
fn derive(sh: &str, v: &str, prop: &str) -> Option<Option<String>> {
    if let Some((_, longs)) = FOUR.iter().find(|(s, _)| *s == sh) {
        let i = longs.iter().position(|l| *l == prop)?;
        return Some(expand4(v).map(|e| e[i].clone()));
    }
    if sh == "border" || sh.starts_with("border-") && SIDES.contains(&&sh[7..]) {
        let i = border_part(sh, prop)?;
        return Some(border_parts(v).map(|p| {
            p[i].clone()
                .unwrap_or_else(|| ["medium", "none", "currentcolor"][i].into())
        }));
    }
    match (sh, prop) {
        ("background", "background-color") => {
            let (_, tokens, at) = background_color_at(v);
            if tokens.len() == 1 && tokens[0].contains("var(") && at.is_none() {
                return Some(None);
            }
            Some(Some(match at {
                Some(i) => tokens[i].clone(),
                None => "transparent".into(),
            }))
        }
        ("background", "background-image") => {
            let imgs: Vec<String> = split_top(v, ',')
                .iter()
                .filter_map(|l| {
                    split_top(l, ' ')
                        .into_iter()
                        .find(|t| t.starts_with("url(") || t.contains("gradient("))
                })
                .collect();
            Some(Some(if imgs.is_empty() {
                "none".into()
            } else {
                imgs.join(", ")
            }))
        }
        ("text-decoration", "text-decoration-line") => {
            let lines: Vec<String> = split_top(v, ' ')
                .into_iter()
                .filter(|t| LINES.contains(&t.to_ascii_lowercase().as_str()))
                .collect();
            Some(Some(if lines.is_empty() {
                "none".into()
            } else {
                lines.join(" ")
            }))
        }
        ("font", p) if FONT.contains(&p) => Some(None),
        // `gap: 24px` ou `gap: 24px 48px` (lignes puis colonnes).
        ("gap", "row-gap" | "column-gap") => {
            let t = split_top(v, ' ');
            Some(match t.as_slice() {
                [a] => Some(a.clone()),
                [r, c] => Some(if prop == "row-gap" {
                    r.clone()
                } else {
                    c.clone()
                }),
                _ => None,
            })
        }
        _ => None,
    }
}

/// Nouvelle valeur du raccourci `sh` (valeur actuelle `v`) quand `prop` prend `value` ;
/// `None` : impossible sans le décomposer. `Some(None)` : raccourci vidé, à retirer.
fn compose(sh: &str, v: &str, prop: &str, value: Option<&str>) -> Option<Option<String>> {
    if let Some((_, longs)) = FOUR.iter().find(|(s, _)| *s == sh) {
        let i = longs.iter().position(|l| *l == prop)?;
        let mut e = expand4(v)?;
        e[i] = value?.trim().to_string();
        return Some(Some(compact4(&e)));
    }
    if sh == "border" || sh.starts_with("border-") && SIDES.contains(&&sh[7..]) {
        let i = border_part(sh, prop)?;
        // Un côté seul ne se règle pas dans `border` (tous les côtés).
        if sh == "border" && !matches!(prop, "border-width" | "border-style" | "border-color") {
            return None;
        }
        let mut p = border_parts(v)?;
        p[i] = value.map(|x| x.trim().to_string());
        let out: Vec<String> = p.into_iter().flatten().collect();
        return Some((!out.is_empty()).then(|| out.join(" ")));
    }
    match (sh, prop) {
        ("background", "background-color") => {
            let (layers, mut tokens, at) = background_color_at(v);
            if tokens.len() == 1 && tokens[0].contains("var(") && at.is_none() {
                return None;
            }
            match (at, value) {
                (Some(i), Some(x)) => tokens[i] = x.trim().to_string(),
                (Some(i), None) => {
                    tokens.remove(i);
                }
                (None, Some(x)) => tokens.push(x.trim().to_string()),
                (None, None) => {}
            }
            Some(background_compose(layers, tokens))
        }
        ("text-decoration", "text-decoration-line") => {
            let rest: Vec<String> = split_top(v, ' ')
                .into_iter()
                .filter(|t| !LINES.contains(&t.to_ascii_lowercase().as_str()))
                .collect();
            let mut out: Vec<String> = value.map(|x| split_top(x, ' ')).unwrap_or_default();
            out.extend(rest);
            Some((!out.is_empty()).then(|| out.join(" ")))
        }
        ("gap", "row-gap" | "column-gap") => {
            let t = split_top(v, ' ');
            let (mut r, mut c) = match t.as_slice() {
                [a] => (a.clone(), a.clone()),
                [r, c] => (r.clone(), c.clone()),
                _ => return None,
            };
            // Écart retiré d'un seul sens : écart par défaut (`normal`) dans ce sens.
            let x = value
                .map(|x| x.trim().to_string())
                .unwrap_or_else(|| "normal".into());
            if prop == "row-gap" {
                r = x;
            } else {
                c = x;
            }
            Some(Some(if r == c { r } else { format!("{r} {c}") }))
        }
        _ => None,
    }
}

/// Valeur de `prop` dans les déclarations d'une règle : la dernière qui la fixe (déclaration
/// directe ou raccourci), `!important` l'emportant sur une déclaration ordinaire qui suit.
pub fn read(decls: &[Decl], prop: &str) -> Option<Found> {
    let mut found: Option<Found> = None;
    for d in decls {
        let hit = if d.prop == prop {
            Some(Found {
                value: Some(d.value.clone()),
                via: None,
                important: d.important,
            })
        } else {
            derive(&d.prop, &d.value, prop).map(|value| Found {
                value,
                via: Some(d.prop.clone()),
                important: d.important,
            })
        };
        if let Some(h) = hit
            && !(found.as_ref().is_some_and(|f| f.important) && !h.important)
        {
            found = Some(h);
        }
    }
    found
}

/// Déclarations à écrire pour donner à `prop` la valeur `value` (`None` : la retirer) en gardant
/// les raccourcis de la règle. `None` : retrait impossible (valeur portée par un raccourci que
/// Revel ne décompose pas, ou par `border` pour un seul côté).
pub fn edits(decls: &[Decl], prop: &str, value: Option<&str>) -> Option<Vec<Edit>> {
    let edit = |p: &str, v: Option<&str>, important: bool| Edit {
        prop: p.to_string(),
        value: v.map(|x| x.trim().to_string()),
        important,
    };
    let Some(f) = read(decls, prop) else {
        return Some(
            value
                .map(|v| vec![edit(prop, Some(v), false)])
                .unwrap_or_default(),
        );
    };
    let Some(sh) = f.via.clone() else {
        return Some(vec![edit(prop, value, f.important)]);
    };
    let sh_value = decls.iter().rev().find(|d| d.prop == sh)?.value.clone();
    if let Some(new) = compose(&sh, &sh_value, prop, value) {
        return Some(vec![Edit {
            prop: sh,
            value: new,
            important: f.important,
        }]);
    }
    match value {
        // Déclaration de la propriété placée après le raccourci (retirée d'abord si elle le précède).
        Some(v) => {
            let mut out = Vec::new();
            if decls.iter().any(|d| d.prop == prop) {
                out.push(edit(prop, None, false));
            }
            out.push(edit(prop, Some(v), f.important));
            Some(out)
        }
        // Un côté retiré d'un raccourci à quatre valeurs : raccourci remplacé par les trois autres.
        None => {
            let (_, longs) = FOUR.iter().find(|(s, _)| *s == sh)?;
            let values = expand4(&sh_value)?;
            let mut out = vec![edit(&sh, None, false)];
            for (l, v) in longs.iter().zip(values.iter()) {
                if *l != prop
                    && read(decls, l).is_some_and(|x| x.via.as_deref() == Some(sh.as_str()))
                {
                    out.push(edit(l, Some(v), f.important));
                }
            }
            Some(out)
        }
    }
}
