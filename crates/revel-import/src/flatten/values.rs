//! Déclarations ramenées aux réglages du panneau Styles : raccourcis que le panneau n'écrit pas
//! (`flex-flow`, `place-items`, `font`…) traduits en propriétés qu'il règle, doublons préfixés
//! retirés, variables remplacées par leur valeur.

use std::collections::HashMap;

/// Déclaration traduite : propriétés du panneau, ou `None` (rien à changer).
pub fn translate(prop: &str, value: &str) -> Option<Vec<(String, String)>> {
    let v = value.trim();
    let pair = |a: &str, b: &str| Some(vec![(a.to_string(), b.to_string())]);
    let two = |a: &str, x: &str, b: &str, y: &str| {
        Some(vec![
            (a.to_string(), x.to_string()),
            (b.to_string(), y.to_string()),
        ])
    };
    match prop {
        "flex-flow" => {
            let mut dir = None;
            let mut wrap = None;
            for t in v.split_whitespace() {
                match t {
                    "row" | "row-reverse" | "column" | "column-reverse" => dir = Some(t),
                    "wrap" | "nowrap" | "wrap-reverse" => wrap = Some(t),
                    _ => return None,
                }
            }
            let mut out = Vec::new();
            if let Some(d) = dir {
                out.push(("flex-direction".to_string(), d.to_string()));
            }
            if let Some(w) = wrap {
                out.push(("flex-wrap".to_string(), w.to_string()));
            }
            (!out.is_empty()).then_some(out)
        }
        "place-content" | "place-items" | "place-self" => {
            let (a, j) = match prop {
                "place-content" => ("align-content", "justify-content"),
                "place-items" => ("align-items", "justify-items"),
                _ => ("align-self", "justify-self"),
            };
            if v.contains('(') {
                return None;
            }
            let toks: Vec<&str> = v.split_whitespace().collect();
            match toks.as_slice() {
                [x] => two(a, x, j, x),
                [x, y] => two(a, x, j, y),
                _ => None,
            }
        }
        "list-style" => {
            let toks: Vec<&str> = v.split_whitespace().collect();
            let kinds = [
                "none",
                "disc",
                "circle",
                "square",
                "decimal",
                "decimal-leading-zero",
                "lower-alpha",
                "upper-alpha",
                "lower-roman",
                "upper-roman",
            ];
            match toks.as_slice() {
                // Marqueur écrit (`""` : aucun marqueur visible).
                [t] if t.starts_with('"') || t.starts_with('\'') => pair("list-style-type", t),
                [t] if kinds.contains(t) => pair("list-style-type", t),
                [t, "outside"] | ["outside", t] if kinds.contains(t) => pair("list-style-type", t),
                _ => None,
            }
        }
        "font" => font(v),
        "outline" => outline(v),
        "word-wrap" => pair("overflow-wrap", v),
        // Valeur ancienne : mots coupés seulement s'ils ne tiennent pas sur la ligne.
        "word-break" if v == "break-word" => {
            Some(vec![("overflow-wrap".to_string(), "anywhere".to_string())])
        }
        _ => None,
    }
}

/// Raccourci `outline` : épaisseur, style, couleur.
fn outline(v: &str) -> Option<Vec<(String, String)>> {
    let styles = [
        "none", "auto", "solid", "dashed", "dotted", "double", "groove", "ridge", "inset", "outset",
    ];
    if v == "0" || v == "none" {
        return Some(vec![("outline-style".into(), "none".into())]);
    }
    let mut width = None;
    let mut style = None;
    let mut color = Vec::new();
    for t in split_top(v) {
        if styles.contains(&t.as_str()) && style.is_none() {
            style = Some(t);
        } else if width.is_none()
            && (t.starts_with(|c: char| c.is_ascii_digit() || c == '.')
                || matches!(t.as_str(), "thin" | "medium" | "thick"))
        {
            width = Some(t);
        } else {
            color.push(t);
        }
    }
    let mut out = Vec::new();
    if let Some(w) = width {
        out.push(("outline-width".to_string(), w));
    }
    out.push((
        "outline-style".to_string(),
        style.unwrap_or_else(|| "none".into()),
    ));
    if !color.is_empty() {
        out.push(("outline-color".to_string(), color.join(" ")));
    }
    Some(out)
}

/// Mots d'une valeur, parenthèses comprises (`2px solid rgba(0, 0, 0, 0.5)`).
fn split_top(v: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for c in v.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ' ' if depth == 0 => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Raccourci `font` : style, graisse, taille, interligne, famille.
fn font(v: &str) -> Option<Vec<(String, String)>> {
    let s = |a: &str, b: &str| (a.to_string(), b.to_string());
    if matches!(v, "inherit" | "initial" | "unset" | "revert") {
        return Some(vec![
            s("font-family", v),
            s("font-size", v),
            s("font-weight", v),
            s("font-style", v),
            s("line-height", v),
        ]);
    }
    // Taille : premier mot qui commence par un chiffre ou un mot de taille, suivi de la famille.
    let mut style = "normal";
    let mut weight = "normal";
    let mut rest = v;
    loop {
        let t = rest.split_whitespace().next()?;
        let size_like = t.starts_with(|c: char| c.is_ascii_digit() || c == '.')
            && !t.chars().all(|c| c.is_ascii_digit())
            || t.starts_with("clamp(")
            || t.starts_with("calc(")
            || t.starts_with("var(")
            || matches!(
                t,
                "small"
                    | "medium"
                    | "large"
                    | "x-large"
                    | "xx-large"
                    | "x-small"
                    | "smaller"
                    | "larger"
            );
        if size_like {
            break;
        }
        match t {
            "italic" | "oblique" => style = t,
            "normal" | "small-caps" => {}
            "bold" | "bolder" | "lighter" => weight = t,
            _ if t.chars().all(|c| c.is_ascii_digit()) => weight = t,
            _ => return None,
        }
        rest = rest.trim_start()[t.len()..].trim_start();
    }
    // Taille et interligne (`16px/1.5`), puis la famille.
    let (size_part, family) = split_size(rest)?;
    let (size, lh) = match size_part.split_once('/') {
        Some((a, b)) => (a.trim(), Some(b.trim())),
        None => (size_part.trim(), None),
    };
    if family.trim().is_empty() {
        return None;
    }
    let mut out = vec![
        s("font-style", style),
        s("font-weight", weight),
        s("font-size", size),
        s("line-height", lh.unwrap_or("normal")),
        s("font-family", family.trim()),
    ];
    out.retain(|(_, v)| !v.is_empty());
    Some(out)
}

/// `16px/1 "Inter", sans-serif` → (`16px/1`, famille), parenthèses comprises dans la taille.
fn split_size(s: &str) -> Option<(&str, &str)> {
    let mut depth = 0;
    let b = s.as_bytes();
    let mut slash = false;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            '/' if depth == 0 => slash = true,
            ' ' if depth == 0 => {
                // Après `/`, la valeur d'interligne peut suivre un espace.
                if slash && i > 0 && b[i - 1] == b'/' {
                    continue;
                }
                let next = s[i..].trim_start();
                if next.starts_with('/') {
                    slash = true;
                    continue;
                }
                return Some((&s[..i], &s[i..]));
            }
            _ => {}
        }
    }
    None
}

/// Remplace `var(--x)` et `var(--x, secours)` par la valeur de `vars` (sans recommencer à
/// l'infini). `keep` : variables du site gardées telles quelles. `None` : variable inconnue sans
/// valeur de secours (la déclaration ne s'applique pas).
pub fn substitute(
    value: &str,
    vars: &HashMap<String, String>,
    keep: &dyn Fn(&str) -> bool,
) -> Option<String> {
    let mut v = value.to_string();
    for _ in 0..12 {
        let Some(i) = find_var(&v, keep) else {
            return Some(simplify(&v));
        };
        let rest = &v[i + 4..];
        let end = close_paren(rest)?;
        let inner = &rest[..end];
        let (name, fallback) = match top_comma(inner) {
            Some(c) => (inner[..c].trim(), Some(inner[c + 1..].trim())),
            None => (inner.trim(), None),
        };
        let name = name.trim_start_matches("--");
        let repl = match vars.get(name) {
            Some(x) => x.clone(),
            None => fallback?.to_string(),
        };
        v = format!("{}{}{}", &v[..i], repl, &rest[end + 1..]);
    }
    None
}

/// Premier `var(` dont la variable n'est pas gardée.
fn find_var(v: &str, keep: &dyn Fn(&str) -> bool) -> Option<usize> {
    let mut from = 0;
    while let Some(off) = v[from..].find("var(") {
        let i = from + off;
        let name: String = v[i + 4..]
            .trim_start()
            .trim_start_matches("--")
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !keep(&name) {
            return Some(i);
        }
        from = i + 4;
    }
    None
}

fn close_paren(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(i),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn top_comma(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Calculs sans unité réduits à leur nombre (`min(calc(0 + 400), 900)` → `400`).
pub fn simplify(v: &str) -> String {
    let t = v.trim();
    if (t.starts_with("calc(") || t.starts_with("min(") || t.starts_with("max("))
        && let Some(n) = number(t)
    {
        return fmt(n);
    }
    t.to_string()
}

fn fmt(n: f64) -> String {
    let r = (n * 10000.0).round() / 10000.0;
    if r.fract() == 0.0 {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

/// Valeur numérique pure d'une expression (`calc`, `min`, `max`, `+ - * /`).
fn number(s: &str) -> Option<f64> {
    let mut p = Num {
        s: s.as_bytes(),
        i: 0,
    };
    let v = p.expr()?;
    p.ws();
    (p.i == p.s.len()).then_some(v)
}

struct Num<'a> {
    s: &'a [u8],
    i: usize,
}

impl Num<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i] == b' ' {
            self.i += 1;
        }
    }
    fn expr(&mut self) -> Option<f64> {
        let mut v = self.term()?;
        loop {
            self.ws();
            match self.s.get(self.i) {
                Some(b'+') => {
                    self.i += 1;
                    v += self.term()?;
                }
                Some(b'-') => {
                    self.i += 1;
                    v -= self.term()?;
                }
                _ => return Some(v),
            }
        }
    }
    fn term(&mut self) -> Option<f64> {
        let mut v = self.factor()?;
        loop {
            self.ws();
            match self.s.get(self.i) {
                Some(b'*') => {
                    self.i += 1;
                    v *= self.factor()?;
                }
                Some(b'/') => {
                    self.i += 1;
                    let d = self.factor()?;
                    if d == 0.0 {
                        return None;
                    }
                    v /= d;
                }
                _ => return Some(v),
            }
        }
    }
    fn factor(&mut self) -> Option<f64> {
        self.ws();
        let rest = &self.s[self.i..];
        for f in [&b"calc("[..], b"min(", b"max("] {
            if rest.starts_with(f) {
                self.i += f.len();
                let mut args = vec![self.expr()?];
                loop {
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => {
                            self.i += 1;
                            args.push(self.expr()?);
                        }
                        Some(b')') => {
                            self.i += 1;
                            break;
                        }
                        _ => return None,
                    }
                }
                return match f {
                    b"min(" => args.into_iter().reduce(f64::min),
                    b"max(" => args.into_iter().reduce(f64::max),
                    _ => (args.len() == 1).then(|| args[0]),
                };
            }
        }
        if rest.first() == Some(&b'(') {
            self.i += 1;
            let v = self.expr()?;
            self.ws();
            (self.s.get(self.i) == Some(&b')')).then(|| self.i += 1)?;
            return Some(v);
        }
        let start = self.i;
        if matches!(self.s.get(self.i), Some(b'-' | b'+')) {
            self.i += 1;
        }
        while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.') {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[start..self.i])
            .ok()?
            .parse()
            .ok()
    }
}

/// Propriétés héritées par les enfants (celles du panneau et quelques autres).
pub fn inherited(prop: &str) -> bool {
    prop.starts_with("--")
        || matches!(
            prop,
            "color"
                | "font-family"
                | "font-size"
                | "font-weight"
                | "font-style"
                | "font-variant"
                | "font-feature-settings"
                | "font-variation-settings"
                | "line-height"
                | "letter-spacing"
                | "word-spacing"
                | "text-align"
                | "text-transform"
                | "text-indent"
                | "text-shadow"
                | "text-wrap"
                | "white-space"
                | "word-break"
                | "overflow-wrap"
                | "hyphens"
                | "list-style-type"
                | "list-style-position"
                | "cursor"
                | "visibility"
                | "direction"
                | "quotes"
                | "-webkit-font-smoothing"
                | "-moz-osx-font-smoothing"
                | "color-scheme"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traductions() {
        assert_eq!(
            translate("flex-flow", "column"),
            Some(vec![("flex-direction".into(), "column".into())])
        );
        assert_eq!(
            translate("place-content", "center flex-start"),
            Some(vec![
                ("align-content".into(), "center".into()),
                ("justify-content".into(), "flex-start".into())
            ])
        );
        assert_eq!(
            translate("list-style", "none"),
            Some(vec![("list-style-type".into(), "none".into())])
        );
        let f = translate("font", "600 16px/1 var(--font-body)").unwrap();
        assert!(f.contains(&("font-weight".into(), "600".into())));
        assert!(f.contains(&("font-size".into(), "16px".into())));
        assert!(f.contains(&("line-height".into(), "1".into())));
        assert!(f.contains(&("font-family".into(), "var(--font-body)".into())));
        let f = translate(
            "font",
            "italic 300 clamp(1rem, 2vw, 2rem) / 1.2 \"Inter\", sans-serif",
        )
        .unwrap();
        assert!(f.contains(&("font-style".into(), "italic".into())));
        assert!(f.contains(&("font-size".into(), "clamp(1rem, 2vw, 2rem)".into())));
        assert!(f.contains(&("font-family".into(), "\"Inter\", sans-serif".into())));
    }

    #[test]
    fn variables() {
        let vars: HashMap<String, String> = [
            ("font-weight".to_string(), "400".to_string()),
            ("framer-font-size".to_string(), "20px".to_string()),
            (
                "font-size".to_string(),
                "var(--framer-font-size)".to_string(),
            ),
            ("couleur-1".to_string(), "#111".to_string()),
        ]
        .into_iter()
        .collect();
        let keep = |n: &str| n.starts_with("couleur-");
        assert_eq!(
            substitute("min(calc(0 + var(--font-weight)), 900)", &vars, &keep).as_deref(),
            Some("400")
        );
        assert_eq!(
            substitute("var(--font-size)", &vars, &keep).as_deref(),
            Some("20px")
        );
        assert_eq!(
            substitute("var(--couleur-1)", &vars, &keep).as_deref(),
            Some("var(--couleur-1)")
        );
        assert_eq!(
            substitute("var(--x, 1px) solid", &vars, &keep).as_deref(),
            Some("1px solid")
        );
        assert_eq!(substitute("var(--x)", &vars, &keep), None);
    }
}

/// Côtés dans l'ordre des raccourcis (`haut droite bas gauche`).
const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];
const CORNERS: [&str; 4] = ["top-left", "top-right", "bottom-right", "bottom-left"];

/// 1 à 4 valeurs → 4 côtés.
fn four(v: &str) -> Option<[String; 4]> {
    let t = split_top(v);
    let g = |i: usize| t[i].clone();
    Some(match t.len() {
        1 => [g(0), g(0), g(0), g(0)],
        2 => [g(0), g(1), g(0), g(1)],
        3 => [g(0), g(1), g(2), g(1)],
        4 => [g(0), g(1), g(2), g(3)],
        _ => return None,
    })
}

/// Trait d'une bordure (`1px solid red`) : épaisseur, style, couleur (valeurs initiales sinon).
fn line(v: &str) -> (String, String, String) {
    let styles = [
        "none", "hidden", "solid", "dashed", "dotted", "double", "groove", "ridge", "inset",
        "outset",
    ];
    let mut w = "medium".to_string();
    let mut s = "none".to_string();
    let mut c = Vec::new();
    for t in split_top(v) {
        if styles.contains(&t.as_str()) {
            s = t;
        } else if t.starts_with(|ch: char| ch.is_ascii_digit() || ch == '.')
            || matches!(t.as_str(), "thin" | "medium" | "thick")
        {
            w = t;
        } else {
            c.push(t);
        }
    }
    let c = if c.is_empty() {
        "currentcolor".to_string()
    } else {
        c.join(" ")
    };
    (w, s, c)
}

/// Propriétés détaillées d'un raccourci (pour comparer les valeurs d'une propriété à l'autre) ;
/// la déclaration elle-même si ce n'est pas un raccourci connu.
pub fn longhands(prop: &str, value: &str) -> Vec<(String, String)> {
    let v = value.trim();
    let wide = matches!(
        v,
        "inherit" | "initial" | "unset" | "revert" | "revert-layer"
    );
    let one = || vec![(prop.to_string(), v.to_string())];
    let sides = |pre: &str, post: &str, vals: [String; 4]| -> Vec<(String, String)> {
        SIDES
            .iter()
            .zip(vals)
            .map(|(s, x)| (format!("{pre}{s}{post}"), x))
            .collect()
    };
    // Propriétés logiques (sens de lecture de gauche à droite, écriture horizontale).
    let logical = |p: &str| -> Option<String> {
        for (base, phys) in [
            ("padding", "padding-"),
            ("margin", "margin-"),
            ("inset", ""),
            ("border", "border-"),
        ] {
            for (log, side) in [
                ("-inline-start", "left"),
                ("-inline-end", "right"),
                ("-block-start", "top"),
                ("-block-end", "bottom"),
            ] {
                if p == format!("{base}{log}") {
                    return Some(format!("{phys}{side}"));
                }
                if base == "border" {
                    for part in ["-width", "-style", "-color"] {
                        if p == format!("border{log}{part}") {
                            return Some(format!("border-{side}{part}"));
                        }
                    }
                }
            }
        }
        None
    };
    if let Some(phys) = logical(prop) {
        return longhands(&phys, v);
    }
    let pair_axis = |a: &str, b: &str| -> Vec<(String, String)> {
        let t = split_top(v);
        match t.len() {
            1 => vec![(a.to_string(), t[0].clone()), (b.to_string(), t[0].clone())],
            2 => vec![(a.to_string(), t[0].clone()), (b.to_string(), t[1].clone())],
            _ => vec![(prop.to_string(), v.to_string())],
        }
    };
    match prop {
        "padding-inline" | "margin-inline" | "padding-block" | "margin-block" | "inset-inline"
        | "inset-block"
            if !v.contains("var(") =>
        {
            let base = prop.split('-').next().unwrap_or("");
            let pre = if base == "inset" {
                String::new()
            } else {
                format!("{base}-")
            };
            let (a, b) = if prop.ends_with("inline") {
                ("left", "right")
            } else {
                ("top", "bottom")
            };
            return pair_axis(&format!("{pre}{a}"), &format!("{pre}{b}"));
        }
        _ => {}
    }
    // Variable : sa valeur n'est connue que pour chaque élément. Chaque propriété détaillée reçoit
    // le raccourci entier, repris après la substitution ([`pending`]).
    if v.contains("var(") && SHORTHANDS.contains(&prop) {
        let names: Vec<String> = longhands(prop, "0px 0px 0px 0px")
            .into_iter()
            .map(|(p, _)| p)
            .filter(|p| p != prop)
            .collect();
        if !names.is_empty() {
            return names
                .into_iter()
                .map(|p| (p, format!("{PENDING}{prop}{PENDING}{v}")))
                .collect();
        }
    }
    match prop {
        "margin" | "padding" => match four(v) {
            Some(f) => sides(&format!("{prop}-"), "", f),
            None => one(),
        },
        "inset" => match four(v) {
            Some(f) => sides("", "", f),
            None => one(),
        },
        "gap" => {
            let t = split_top(v);
            match t.len() {
                1 => vec![
                    ("row-gap".into(), t[0].clone()),
                    ("column-gap".into(), t[0].clone()),
                ],
                2 => vec![
                    ("row-gap".into(), t[0].clone()),
                    ("column-gap".into(), t[1].clone()),
                ],
                _ => one(),
            }
        }
        "border-radius" if !v.contains('/') => match four(v) {
            Some(f) => CORNERS
                .iter()
                .zip(f)
                .map(|(c, x)| (format!("border-{c}-radius"), x))
                .collect(),
            None => one(),
        },
        "border-width" | "border-style" | "border-color" => {
            let kind = &prop[7..];
            match four(v) {
                Some(f) => sides("border-", &format!("-{kind}"), f),
                None => one(),
            }
        }
        "border" => {
            let (w, s, c) = if wide {
                (v.into(), v.into(), v.into())
            } else {
                line(v)
            };
            SIDES
                .iter()
                .flat_map(|side| {
                    [
                        (format!("border-{side}-width"), w.clone()),
                        (format!("border-{side}-style"), s.clone()),
                        (format!("border-{side}-color"), c.clone()),
                    ]
                })
                .collect()
        }
        "border-top" | "border-right" | "border-bottom" | "border-left" => {
            let (w, s, c) = if wide {
                (v.into(), v.into(), v.into())
            } else {
                line(v)
            };
            vec![
                (format!("{prop}-width"), w),
                (format!("{prop}-style"), s),
                (format!("{prop}-color"), c),
            ]
        }
        "flex" => {
            let t = split_top(v);
            let num = |x: &str| x.parse::<f64>().is_ok();
            let (g, s, b) = match t.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
                ["none"] => ("0", "0", "auto".to_string()),
                ["auto"] => ("1", "1", "auto".to_string()),
                ["initial"] => ("0", "1", "auto".to_string()),
                [x] if wide => {
                    return vec![
                        ("flex-grow".into(), x.to_string()),
                        ("flex-shrink".into(), x.to_string()),
                        ("flex-basis".into(), x.to_string()),
                    ];
                }
                [x] if num(x) => (*x, "1", "0%".to_string()),
                [x] => ("1", "1", x.to_string()),
                [x, y] if num(x) && num(y) => (*x, *y, "0%".to_string()),
                [x, y] if num(x) => (*x, "1", y.to_string()),
                [x, y, z] if num(x) && num(y) => (*x, *y, z.to_string()),
                _ => return one(),
            };
            vec![
                ("flex-grow".into(), g.to_string()),
                ("flex-shrink".into(), s.to_string()),
                ("flex-basis".into(), b),
            ]
        }
        "overflow" => {
            let t = split_top(v);
            match t.len() {
                1 => vec![
                    ("overflow-x".into(), t[0].clone()),
                    ("overflow-y".into(), t[0].clone()),
                ],
                2 => vec![
                    ("overflow-x".into(), t[0].clone()),
                    ("overflow-y".into(), t[1].clone()),
                ],
                _ => one(),
            }
        }
        _ => one(),
    }
}

/// Marque d'une propriété détaillée dont le raccourci contient une variable.
pub const PENDING: char = '\u{1}';

/// Raccourcis développés en propriétés détaillées.
const SHORTHANDS: &[&str] = &[
    "margin",
    "padding",
    "inset",
    "gap",
    "border-radius",
    "border-width",
    "border-style",
    "border-color",
    "border",
    "border-top",
    "border-right",
    "border-bottom",
    "border-left",
    "flex",
    "overflow",
];

/// Valeur d'une propriété détaillée en attente (`\u{1}padding\u{1}var(--x)`) une fois la valeur du
/// raccourci connue (`resolved`) : sa part.
///
/// Une variable du site gardée (`var(--trait)`) est découpée d'après sa valeur (`resolve_all`, toutes
/// les variables remplacées) puis remise à sa place : la part de chaque propriété garde la variable.
pub fn pending(
    prop: &str,
    value: &str,
    resolve: &dyn Fn(&str) -> Option<String>,
    resolve_all: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    let rest = value.strip_prefix(PENDING)?;
    let (short, raw) = rest.split_once(PENDING)?;
    let full = resolve(raw)?;
    if !full.contains("var(") {
        return longhands(short, &full)
            .into_iter()
            .find(|(p, _)| p == prop)
            .map(|(_, v)| v);
    }
    let all = resolve_all(raw)?;
    if all.contains("var(") {
        return None;
    }
    let part = longhands(short, &all)
        .into_iter()
        .find(|(p, _)| p == prop)
        .map(|(_, v)| v)?;
    let (kept, done) = (split_top(&full), split_top(&all));
    if kept.len() != done.len() {
        // Variable à plusieurs mots : sa valeur, sans le lien.
        return Some(part);
    }
    // Chaque mot de la part repris de la valeur gardée, à la même place.
    let mut used = vec![false; done.len()];
    let words: Vec<String> = split_top(&part)
        .into_iter()
        .map(
            |w| match (0..done.len()).find(|&i| !used[i] && done[i] == w) {
                Some(i) => {
                    used[i] = true;
                    kept[i].clone()
                }
                None => w,
            },
        )
        .collect();
    Some(words.join(" "))
}

/// Valeur initiale d'une propriété détaillée de groupe (raccourci réécrit en entier).
pub fn initial(prop: &str) -> &'static str {
    match prop {
        "flex-grow" => "0",
        "flex-shrink" => "1",
        "flex-basis" => "auto",
        "overflow-x" | "overflow-y" => "visible",
        _ => "initial",
    }
}

/// Groupes que le panneau règle par leur raccourci seulement : (raccourci, propriétés détaillées).
pub const GROUPS: &[(&str, &[&str])] = &[
    ("flex", &["flex-grow", "flex-shrink", "flex-basis"]),
    ("overflow", &["overflow-x", "overflow-y"]),
];

/// Report réécrit pour le panneau : un groupe touché est écrit par son raccourci, avec les
/// valeurs complètes de l'élément (`before`).
pub fn recompose(
    t: &std::collections::BTreeMap<String, String>,
    before: &std::collections::BTreeMap<String, String>,
) -> std::collections::BTreeMap<String, String> {
    let mut out = t.clone();
    for (short, parts) in GROUPS {
        if !parts.iter().any(|p| t.contains_key(*p)) {
            continue;
        }
        let vals: Vec<String> = parts
            .iter()
            .map(|p| {
                t.get(*p)
                    .or_else(|| before.get(*p))
                    .cloned()
                    .unwrap_or_else(|| initial(p).to_string())
            })
            .collect();
        for p in *parts {
            out.remove(*p);
        }
        let wide = |v: &String| {
            matches!(
                v.as_str(),
                "inherit" | "initial" | "unset" | "revert" | "revert-layer"
            )
        };
        let joined =
            if vals.iter().all(|v| *v == vals[0]) && (*short == "overflow" || wide(&vals[0])) {
                vals[0].clone()
            } else if vals.iter().any(wide) {
                // Mélange impossible dans le raccourci : valeurs initiales à la place des mots-clés.
                parts
                    .iter()
                    .zip(&vals)
                    .map(|(p, v)| {
                        if wide(v) {
                            initial(p).to_string()
                        } else {
                            v.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                vals.join(" ")
            };
        out.insert(short.to_string(), joined);
    }
    // Quatre côtés (ou angles) écrits ensemble : leur raccourci.
    let boxes: [(&str, [String; 4]); 6] = [
        ("margin", SIDES.map(|x| format!("margin-{x}"))),
        ("padding", SIDES.map(|x| format!("padding-{x}"))),
        ("inset", SIDES.map(str::to_string)),
        (
            "border-radius",
            CORNERS.map(|x| format!("border-{x}-radius")),
        ),
        ("border-width", SIDES.map(|x| format!("border-{x}-width"))),
        ("border-color", SIDES.map(|x| format!("border-{x}-color"))),
    ];
    for (short, parts) in boxes {
        let vals: Vec<String> = parts.iter().filter_map(|p| out.get(p).cloned()).collect();
        // Mot-clé global (`unset`, `revert`…) : seul dans un raccourci, jamais mêlé à d'autres
        // valeurs (la déclaration serait ignorée).
        let wide = |v: &String| {
            matches!(
                v.as_str(),
                "inherit" | "initial" | "unset" | "revert" | "revert-layer"
            )
        };
        if vals.len() != 4 || (vals.iter().any(wide) && !vals.iter().all(|v| *v == vals[0])) {
            continue;
        }
        for p in &parts {
            out.remove(p);
        }
        let v = if vals.iter().all(|x| *x == vals[0]) {
            vals[0].clone()
        } else if vals[0] == vals[2] && vals[1] == vals[3] {
            format!("{} {}", vals[0], vals[1])
        } else if vals[1] == vals[3] {
            format!("{} {} {}", vals[0], vals[1], vals[2])
        } else {
            vals.join(" ")
        };
        out.insert(short.to_string(), v);
    }
    // Écarts égaux : `gap`.
    if let (Some(r), Some(c)) = (out.get("row-gap").cloned(), out.get("column-gap").cloned())
        && (r == c
            || ![&r, &c]
                .iter()
                .any(|v| matches!(v.as_str(), "inherit" | "initial" | "unset" | "revert")))
    {
        out.remove("row-gap");
        out.remove("column-gap");
        out.insert("gap".into(), if r == c { r } else { format!("{r} {c}") });
    }
    out
}

#[cfg(test)]
mod tests_longhands {
    use super::*;

    #[test]
    fn detail_des_raccourcis() {
        assert_eq!(
            longhands("margin", "0 auto"),
            vec![
                ("margin-top".to_string(), "0".to_string()),
                ("margin-right".to_string(), "auto".to_string()),
                ("margin-bottom".to_string(), "0".to_string()),
                ("margin-left".to_string(), "auto".to_string())
            ]
        );
        assert_eq!(
            longhands("flex", "1")[2],
            ("flex-basis".to_string(), "0%".to_string())
        );
        assert_eq!(
            longhands("flex", "0 0 min(86%, 480px)")[2].1,
            "min(86%, 480px)"
        );
        let b = longhands("border", "1px solid var(--x)");
        let red: std::collections::HashMap<String, String> =
            [("x".to_string(), "red".to_string())].into_iter().collect();
        let sub_red = |raw: &str| substitute(raw, &red, &|_| false);
        assert_eq!(
            pending("border-top-color", &b[2].1, &sub_red, &sub_red).as_deref(),
            Some("red")
        );
        let m = longhands("margin", "var(--m)");
        assert_eq!(m.len(), 4);
        let vars: std::collections::HashMap<String, String> =
            [("m".to_string(), "0 auto".to_string())]
                .into_iter()
                .collect();
        let sub = |raw: &str| substitute(raw, &vars, &|_| false);
        assert_eq!(
            pending("margin-left", &m[3].1, &sub, &sub).as_deref(),
            Some("auto")
        );
        assert_eq!(
            longhands("padding-inline-start", "2ch"),
            vec![("padding-left".to_string(), "2ch".to_string())]
        );
        assert_eq!(
            longhands("margin-inline", "0 auto")[1],
            ("margin-right".to_string(), "auto".to_string())
        );
        assert_eq!(longhands("border-top", "1px solid red")[2].1, "red");
        let t: std::collections::BTreeMap<String, String> =
            [("flex-basis".to_string(), "100%".to_string())]
                .into_iter()
                .collect();
        let r = recompose(&t, &std::collections::BTreeMap::new());
        assert_eq!(r.get("flex").map(String::as_str), Some("0 1 100%"));
    }

    #[test]
    fn raccourci_a_variable_du_site() {
        let v = longhands("border", "1px solid var(--trait)");
        let c = &v.iter().find(|(p, _)| p == "border-top-color").unwrap().1;
        let kept = |r: &str| Some(r.to_string());
        let all = |r: &str| Some(r.replace("var(--trait)", "rgb(1, 2, 3)"));
        assert_eq!(
            pending("border-top-color", c, &kept, &all).as_deref(),
            Some("var(--trait)")
        );
        assert_eq!(
            pending("border-top-width", c, &kept, &all).as_deref(),
            Some("1px")
        );
        let v = longhands("border-color", "var(--trait)");
        assert_eq!(
            pending("border-left-color", &v[0].1, &kept, &all).as_deref(),
            Some("var(--trait)")
        );
    }
}
