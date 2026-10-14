//! Nombres avec unité : lecture, écriture, calculs saisis (`12+4`, `*2`), pas des flèches et du
//! glisser.

/// Nombre et unité (`""` sans unité, `%`, `px`, `rem`…).
#[derive(Debug, Clone, PartialEq)]
pub struct Num {
    pub value: f64,
    pub unit: String,
}

impl Num {
    pub fn to_css(&self) -> String {
        format!("{}{}", fmt_num(self.value), self.unit)
    }
}

/// Nombre écrit sans zéros inutiles, au plus trois décimales (`1.5`, `0.125`, `-2`).
pub fn fmt_num(v: f64) -> String {
    let r = (v * 1000.0).round() / 1000.0;
    if r == 0.0 {
        return "0".into();
    }
    let s = format!("{r:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

fn is_unit_char(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '%'
}

/// `12px`, `-1.5rem`, `.5`, `50%` ; un mot-clé ou une fonction -> `None`.
pub fn parse_num(s: &str) -> Option<Num> {
    let s = s.trim();
    let split = s
        .char_indices()
        .find(|(i, c)| {
            is_unit_char(*c)
                && !(*c == 'e' && s[i + 1..].starts_with(|d: char| d.is_ascii_digit() || d == '-'))
        })
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let (num, unit) = s.split_at(split);
    if num.is_empty() || !unit.chars().all(is_unit_char) {
        return None;
    }
    let value: f64 = num.parse().ok()?;
    value.is_finite().then(|| Num {
        value,
        unit: unit.to_ascii_lowercase(),
    })
}

/// Lecteur des calculs saisis : `+ - * /`, parenthèses, nombres avec une unité facultative.
struct Calc<'a> {
    s: &'a [u8],
    i: usize,
    unit: Option<String>,
}

impl Calc<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }
    fn expr(&mut self) -> Option<f64> {
        let mut v = self.term()?;
        while let Some(op @ (b'+' | b'-')) = self.peek() {
            self.i += 1;
            let r = self.term()?;
            v = if op == b'+' { v + r } else { v - r };
        }
        Some(v)
    }
    fn term(&mut self) -> Option<f64> {
        let mut v = self.factor()?;
        while let Some(op @ (b'*' | b'/' | b'x')) = self.peek() {
            self.i += 1;
            let r = self.factor()?;
            v = if op == b'/' {
                if r == 0.0 {
                    return None;
                }
                v / r
            } else {
                v * r
            };
        }
        Some(v)
    }
    fn factor(&mut self) -> Option<f64> {
        match self.peek()? {
            b'-' => {
                self.i += 1;
                Some(-self.factor()?)
            }
            b'+' => {
                self.i += 1;
                self.factor()
            }
            b'(' => {
                self.i += 1;
                let v = self.expr()?;
                (self.peek()? == b')').then(|| self.i += 1)?;
                Some(v)
            }
            _ => {
                let start = self.i;
                while self.i < self.s.len()
                    && (self.s[self.i].is_ascii_digit()
                        || self.s[self.i] == b'.'
                        || self.s[self.i] == b',')
                {
                    self.i += 1;
                }
                let num = std::str::from_utf8(&self.s[start..self.i])
                    .ok()?
                    .replace(',', ".");
                let v: f64 = num.parse().ok()?;
                let ustart = self.i;
                while (self.i < self.s.len()
                    && is_unit_char(self.s[self.i] as char)
                    && self.s[self.i] != b'x')
                    || self.unit_x()
                {
                    self.i += 1;
                }
                let unit = std::str::from_utf8(&self.s[ustart..self.i]).ok()?;
                if !unit.is_empty() {
                    match &self.unit {
                        Some(u) if u != unit => return None,
                        _ => self.unit = Some(unit.to_ascii_lowercase()),
                    }
                }
                Some(v)
            }
        }
    }
    /// `x` fait partie d'une unité (`px`, `ex`) quand il suit une lettre ; seul, c'est « fois ».
    fn unit_x(&self) -> bool {
        self.i < self.s.len()
            && self.s[self.i] == b'x'
            && self.i > 0
            && self.s[self.i - 1].is_ascii_alphabetic()
    }
}

/// Valeur saisie dans un champ numérique : calcul évalué (`12+4` -> `16px`, `*2` appliqué à la
/// valeur actuelle), unité par défaut ajoutée à un nombre seul. Un mot-clé, une fonction (`calc()`,
/// `var()`) ou un texte non reconnu est renvoyé tel quel ; vide -> `None`.
pub fn eval(input: &str, current: Option<&str>, default_unit: &str) -> Option<String> {
    let t = input.trim();
    if t.is_empty() {
        return None;
    }
    let cur = current.and_then(parse_num);
    let expr = match (t.as_bytes()[0], &cur) {
        (b'*' | b'/', Some(c)) => format!("{}{}{t}", fmt_num(c.value), c.unit),
        _ => t.to_string(),
    };
    let looks_numeric = expr
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"+-*/(). ,%".contains(&b))
        && expr.bytes().any(|b| b.is_ascii_digit());
    if looks_numeric {
        let mut c = Calc {
            s: expr.as_bytes(),
            i: 0,
            unit: None,
        };
        if let Some(v) = c.expr()
            && c.peek().is_none()
            && v.is_finite()
        {
            let unit = c
                .unit
                .or_else(|| {
                    cur.as_ref()
                        .map(|c| c.unit.clone())
                        .filter(|u| !u.is_empty())
                })
                .unwrap_or_else(|| default_unit.to_string());
            return Some(format!("{}{unit}", fmt_num(v)));
        }
    }
    Some(t.to_string())
}

/// Valeur augmentée de `delta` (flèches, glisser), même unité ; part de `fallback` (valeur
/// effective) si `value` n'est pas un nombre. Sans unité -> `default_unit`.
pub fn step(value: &str, fallback: &str, delta: f64, default_unit: &str) -> Option<String> {
    let n = parse_num(value).or_else(|| parse_num(fallback))?;
    let unit = if n.unit.is_empty() {
        default_unit.to_string()
    } else {
        n.unit
    };
    Some(format!("{}{unit}", fmt_num(n.value + delta)))
}
