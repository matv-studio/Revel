//! Requêtes média évaluées à une largeur d'écran (points de rupture du site).

/// Résultat d'une requête à une largeur : vraie, fausse, ou dépendante d'autre chose que la
/// largeur (mouvement réduit, impression, orientation…) : non reportée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eval {
    Yes,
    No,
    Unknown,
}

/// Évalue `media` (sans `@media`) pour un écran de `width` pixels, pointeur fin, thème clair.
pub fn eval(media: &str, width: f64) -> Eval {
    let mut any_unknown = false;
    // Liste : vraie si l'une l'est.
    for q in media.split(',') {
        match eval_one(q.trim(), width) {
            Eval::Yes => return Eval::Yes,
            Eval::Unknown => any_unknown = true,
            Eval::No => {}
        }
    }
    if any_unknown { Eval::Unknown } else { Eval::No }
}

fn eval_one(q: &str, width: f64) -> Eval {
    let lower = q.to_ascii_lowercase();
    if lower.starts_with("not ") {
        return Eval::Unknown;
    }
    let mut result = Eval::Yes;
    for part in lower.split(" and ") {
        let p = part.trim().trim_start_matches("only ").trim();
        let r = match p {
            "screen" | "all" | "" => Eval::Yes,
            "print" | "speech" => Eval::No,
            _ if p.starts_with('(') && p.ends_with(')') => feature(&p[1..p.len() - 1], width),
            _ => Eval::Unknown,
        };
        match r {
            Eval::No => return Eval::No,
            Eval::Unknown => result = Eval::Unknown,
            Eval::Yes => {}
        }
    }
    result
}

fn length(v: &str) -> Option<f64> {
    let v = v.trim();
    let (num, mul) = if let Some(n) = v.strip_suffix("px") {
        (n, 1.0)
    } else if let Some(n) = v.strip_suffix("rem") {
        (n, 16.0)
    } else if let Some(n) = v.strip_suffix("em") {
        (n, 16.0)
    } else {
        (v, 1.0)
    };
    num.trim().parse::<f64>().ok().map(|n| n * mul)
}

fn feature(f: &str, width: f64) -> Eval {
    let yes = |b: bool| if b { Eval::Yes } else { Eval::No };
    // Syntaxe d'intervalle : `width <= 600px`, `600px < width`.
    for (op, rev) in [("<=", false), (">=", false), ("<", false), (">", false)] {
        let _ = rev;
        if let Some(i) = f.find(op) {
            let (a, b) = (f[..i].trim(), f[i + op.len()..].trim());
            let (val, flip) = if a == "width" {
                (length(b), false)
            } else if b == "width" {
                (length(a), true)
            } else {
                return Eval::Unknown;
            };
            let Some(v) = val else { return Eval::Unknown };
            let op = if flip {
                match op {
                    "<=" => ">=",
                    ">=" => "<=",
                    "<" => ">",
                    _ => "<",
                }
            } else {
                op
            };
            return yes(match op {
                "<=" => width <= v,
                ">=" => width >= v,
                "<" => width < v,
                _ => width > v,
            });
        }
    }
    let (name, value) = match f.split_once(':') {
        Some((n, v)) => (n.trim(), v.trim()),
        None => (f.trim(), ""),
    };
    match name {
        "max-width" => length(value).map_or(Eval::Unknown, |v| yes(width <= v)),
        "min-width" => length(value).map_or(Eval::Unknown, |v| yes(width >= v)),
        "hover" => yes(value == "hover" || value.is_empty()),
        "any-hover" => yes(value == "hover" || value.is_empty()),
        "pointer" | "any-pointer" => yes(value == "fine" || value.is_empty()),
        "prefers-color-scheme" => yes(value == "light"),
        "prefers-reduced-motion" if value == "no-preference" => Eval::Unknown,
        _ => Eval::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requetes() {
        assert_eq!(eval("(max-width: 767px)", 767.0), Eval::Yes);
        assert_eq!(eval("(max-width: 767px)", 991.0), Eval::No);
        assert_eq!(
            eval(
                "screen and (min-width: 810px) and (max-width: 1199.98px)",
                991.0
            ),
            Eval::Yes
        );
        assert_eq!(
            eval("(min-width: 810px) and (max-width: 1199.98px)", 1440.0),
            Eval::No
        );
        assert_eq!(eval("(max-width: 48em)", 767.0), Eval::Yes);
        assert_eq!(eval("(width <= 600px)", 478.0), Eval::Yes);
        assert_eq!(
            eval("(prefers-reduced-motion: reduce)", 1440.0),
            Eval::Unknown
        );
        assert_eq!(eval("print", 1440.0), Eval::No);
        assert_eq!(eval("(hover: hover)", 1440.0), Eval::Yes);
    }
}
