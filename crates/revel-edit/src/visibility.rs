//! Conditions d'affichage écrites par l'interface (« Conditional visibility » de Webflow) :
//! clauses jointes par « et » (toutes vraies) ou par « ou » (une au moins), chacune sur un champ
//! du contenu, une valeur de liste ou une propriété de composant : rempli, vide, égal à,
//! différent de, contient, plus de / moins de N valeurs.
//!
//! Formes écrites : `x`, `not x`, `x == "v"`, `x != "v"`, `"v" in x`, `x|length > 2`,
//! `x|length < 3` ; `x|length` seul vaut « rempli ».

use serde::{Deserialize, Serialize};

/// Valeur comparée : texte (écrit entre guillemets) ou nombre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lit {
    pub text: String,
    #[serde(default)]
    pub number: bool,
}

impl Lit {
    pub fn text(s: &str) -> Lit {
        Lit {
            text: s.to_string(),
            number: false,
        }
    }
    fn expr(&self) -> String {
        if self.number && self.text.trim().parse::<f64>().is_ok() {
            self.text.trim().to_string()
        } else {
            serde_json::to_string(&self.text).unwrap_or_else(|_| "\"\"".into())
        }
    }
}

/// Test d'une clause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "test", content = "value", rename_all = "snake_case")]
pub enum Test {
    Set,
    Empty,
    Equals(Lit),
    Differs(Lit),
    Contains(Lit),
    MoreThan(usize),
    LessThan(usize),
}

impl Test {
    /// Nom court (`set`, `empty`, `equals`…), pour les listes de choix.
    pub fn key(&self) -> &'static str {
        match self {
            Test::Set => "set",
            Test::Empty => "empty",
            Test::Equals(_) => "equals",
            Test::Differs(_) => "differs",
            Test::Contains(_) => "contains",
            Test::MoreThan(_) => "more",
            Test::LessThan(_) => "less",
        }
    }
    /// Test d'après son nom court, en gardant la valeur quand elle a un sens.
    pub fn with_key(&self, key: &str) -> Test {
        let lit = match self {
            Test::Equals(l) | Test::Differs(l) | Test::Contains(l) => l.clone(),
            _ => Lit::text(""),
        };
        let n = match self {
            Test::MoreThan(n) | Test::LessThan(n) => *n,
            _ => 1,
        };
        match key {
            "empty" => Test::Empty,
            "equals" => Test::Equals(lit),
            "differs" => Test::Differs(lit),
            "contains" => Test::Contains(lit),
            "more" => Test::MoreThan(n),
            "less" => Test::LessThan(n),
            _ => Test::Set,
        }
    }
}

/// Clause : expression du champ (`item.data.client`, `b.type`, `titre`) et test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clause {
    pub field: String,
    #[serde(flatten)]
    pub test: Test,
}

/// Condition complète : clauses toutes vraies (`and`) ou une au moins (`or`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cond {
    pub any: bool,
    pub clauses: Vec<Clause>,
}

/// Condition lue (clauses jointes par `and`, ou toutes par `or`).
pub fn read(cond: &str) -> Option<Cond> {
    if let Some(c) = parse(cond) {
        return Some(Cond {
            any: false,
            clauses: c,
        });
    }
    let parts = split_kw(cond, " or ", " and ")?;
    let clauses: Option<Vec<Clause>> = parts.iter().map(|p| clause(p)).collect();
    Some(Cond {
        any: true,
        clauses: clauses?,
    })
}

/// Condition écrite.
pub fn write(c: &Cond) -> String {
    if c.any {
        c.clauses
            .iter()
            .map(|k| to_expr(std::slice::from_ref(k)))
            .collect::<Vec<_>>()
            .join(" or ")
    } else {
        to_expr(&c.clauses)
    }
}

/// Condition écrite (clauses toutes vraies).
pub fn to_expr(clauses: &[Clause]) -> String {
    clauses
        .iter()
        .map(|c| {
            let f = &c.field;
            match &c.test {
                Test::Set => f.clone(),
                Test::Empty => format!("not {f}"),
                Test::Equals(l) => format!("{f} == {}", l.expr()),
                Test::Differs(l) => format!("{f} != {}", l.expr()),
                Test::Contains(l) => format!("{} in {f}", l.expr()),
                Test::MoreThan(n) => format!("{f}|length > {n}"),
                Test::LessThan(n) => format!("{f}|length < {n}"),
            }
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

/// Clauses d'une condition, si elle a la forme écrite par l'interface (champs : chemins).
pub fn parse(cond: &str) -> Option<Vec<Clause>> {
    let parts = split_and(cond)?;
    parts.iter().map(|p| clause(p)).collect()
}

/// Parties jointes par `and` au premier niveau (hors guillemets et parenthèses).
fn split_and(s: &str) -> Option<Vec<String>> {
    split_kw(s, " and ", " or ")
}

/// Parties jointes par `kw` au premier niveau ; `None` si `other` y paraît aussi (mélange).
fn split_kw(s: &str, kw: &str, other: &str) -> Option<Vec<String>> {
    let s = strip_parens(s.trim());
    let b = s.as_bytes();
    let (mut depth, mut quote, mut start) = (0i32, None::<u8>, 0usize);
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if let Some(q) = quote {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == q {
                quote = None;
            }
        } else {
            match c {
                b'"' | b'\'' => quote = Some(c),
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                b' ' if depth == 0 && s[i..].starts_with(kw) => {
                    out.push(s[start..i].trim().to_string());
                    i += kw.len();
                    start = i;
                    continue;
                }
                // Mélange de « et » et de « ou » : pas une forme de l'interface.
                b' ' if depth == 0 && s[i..].starts_with(other) => return None,
                _ => {}
            }
        }
        i += 1;
    }
    out.push(s[start..].trim().to_string());
    out.iter().all(|p| !p.is_empty()).then_some(out)
}

/// Parenthèses qui entourent toute l'expression, retirées.
fn strip_parens(s: &str) -> &str {
    let mut s = s.trim();
    while s.starts_with('(') && s.ends_with(')') && closes_at_end(s) {
        s = s[1..s.len() - 1].trim();
    }
    s
}

fn closes_at_end(s: &str) -> bool {
    let b = s.as_bytes();
    let (mut depth, mut quote) = (0i32, None::<u8>);
    for (i, &c) in b.iter().enumerate() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            b'"' | b'\'' => quote = Some(c),
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 && i != b.len() - 1 {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

/// Chemin d'un champ (`item.data.client`, `b.type`, `titre`).
pub fn is_path(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty()
        && s.split('.').all(|p| {
            let mut ch = p.chars();
            ch.next().is_some_and(|c| c.is_alphabetic() || c == '_')
                && ch.all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        })
        && !matches!(s, "and" | "or" | "not" | "true" | "false" | "none")
}

/// Texte entre guillemets, ou nombre.
fn literal(s: &str) -> Option<Lit> {
    let s = s.trim();
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        let inner = &s[1..s.len() - 1];
        if s.starts_with('"') {
            return serde_json::from_str::<String>(s)
                .ok()
                .map(|t| Lit::text(&t));
        }
        return Some(Lit::text(&inner.replace("\\'", "'")));
    }
    s.parse::<f64>().is_ok().then(|| Lit {
        text: s.to_string(),
        number: true,
    })
}

/// `x|length` ou `(x | length)` : le chemin `x`.
fn length_of(s: &str) -> Option<String> {
    let s = strip_parens(s);
    let (head, tail) = s.split_once('|')?;
    (tail.trim() == "length" && is_path(strip_parens(head))).then(|| strip_parens(head).to_string())
}

fn clause(p: &str) -> Option<Clause> {
    let p = strip_parens(p);
    if let Some(rest) = p.strip_prefix("not ") {
        let r = strip_parens(rest);
        let field = if is_path(r) {
            r.to_string()
        } else {
            length_of(r)?
        };
        return Some(Clause {
            field,
            test: Test::Empty,
        });
    }
    for (op, make) in [
        ("==", (|l| Test::Equals(l)) as fn(Lit) -> Test),
        ("!=", |l| Test::Differs(l)),
    ] {
        if let Some((a, b)) = p.split_once(op) {
            let (a, b) = (strip_parens(a), strip_parens(b));
            if is_path(a)
                && let Some(l) = literal(b)
            {
                return Some(Clause {
                    field: a.to_string(),
                    test: make(l),
                });
            }
            if is_path(b)
                && let Some(l) = literal(a)
            {
                return Some(Clause {
                    field: b.to_string(),
                    test: make(l),
                });
            }
            return None;
        }
    }
    for (op, more) in [(">=", true), ("<=", false), (">", true), ("<", false)] {
        if let Some((a, b)) = p.split_once(op) {
            let field = length_of(a)?;
            let n: usize = b.trim().parse().ok()?;
            let test = match (op, more) {
                (">=", _) => Test::MoreThan(n.checked_sub(1)?),
                ("<=", _) => Test::LessThan(n + 1),
                (_, true) => Test::MoreThan(n),
                _ => Test::LessThan(n),
            };
            return Some(Clause { field, test });
        }
    }
    if let Some((a, b)) = p.split_once(" in ")
        && let Some(l) = literal(a)
        && is_path(strip_parens(b))
    {
        return Some(Clause {
            field: strip_parens(b).to_string(),
            test: Test::Contains(l),
        });
    }
    if is_path(p) {
        return Some(Clause {
            field: p.to_string(),
            test: Test::Set,
        });
    }
    length_of(p).map(|field| Clause {
        field,
        test: Test::Set,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formes_reconnues() {
        let c = parse(r#"item.data.client and b.type == "text""#).unwrap();
        assert_eq!(c[0].test, Test::Set);
        assert_eq!(c[1].field, "b.type");
        assert_eq!(c[1].test, Test::Equals(Lit::text("text")));
        assert_eq!(to_expr(&c), r#"item.data.client and b.type == "text""#);
        let c = parse("(b.images|length > 1)").unwrap();
        assert_eq!(c[0].test, Test::MoreThan(1));
        let c = parse("not (others | length)").unwrap();
        assert_eq!(c[0].test, Test::Empty);
        assert_eq!(c[0].field, "others");
        let c = parse("'web' in item.data.tags and n != 3").unwrap();
        assert_eq!(c[0].test, Test::Contains(Lit::text("web")));
        assert_eq!(to_expr(&c), r#""web" in item.data.tags and n != 3"#);
        let c = parse("x|length >= 2").unwrap();
        assert_eq!(c[0].test, Test::MoreThan(1));
        assert!(parse("a or b").is_none());
        let c = read("hook|length or chapeau or (keywords | length)").unwrap();
        assert!(c.any);
        assert_eq!(c.clauses.len(), 3);
        assert_eq!(write(&c), "hook or chapeau or keywords");
        assert!(read("a and b or c").is_none());
        assert!(parse("a.b(1)").is_none());
        assert!(parse("x == y").is_none());
        // Toute forme écrite se relit à l'identique.
        for t in [
            Test::Set,
            Test::Empty,
            Test::Equals(Lit::text("a \"b\"")),
            Test::Differs(Lit {
                text: "2".into(),
                number: true,
            }),
            Test::Contains(Lit::text("x")),
            Test::MoreThan(0),
            Test::LessThan(3),
        ] {
            let c = vec![Clause {
                field: "item.data.f".into(),
                test: t,
            }];
            assert_eq!(parse(&to_expr(&c)).unwrap(), c, "{}", to_expr(&c));
        }
    }
}
