//! Sélecteurs CSS lus pour la mise à plat : découpage en sélecteurs composés, spécificité,
//! états (`:hover`) et pseudo-éléments (`::after`) du sujet, forme sans état pour trouver les
//! éléments dans une page rendue.

/// Combinateur placé devant un sélecteur composé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comb {
    /// Premier composé.
    None,
    /// Descendant (espace).
    Desc,
    /// Enfant (`>`).
    Child,
    /// Frère suivant immédiat (`+`).
    Next,
    /// Frère suivant (`~`).
    After,
}

/// Sélecteur complexe : composés et combinateurs, dans l'ordre d'écriture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Complex {
    pub parts: Vec<(Comb, Vec<String>)>,
}

/// Coupe une liste de sélecteurs sur les virgules hors parenthèses, crochets et guillemets.
pub fn split_list(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut cur = String::new();
    for c in s.chars() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            cur.push(c);
            continue;
        }
        match c {
            '"' | '\'' => {
                quote = Some(c);
                cur.push(c);
            }
            '(' | '[' => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// Lit un sélecteur complexe (sans virgule). `None` : forme non reconnue.
pub fn parse(s: &str) -> Option<Complex> {
    let chars: Vec<char> = s.trim().chars().collect();
    let mut parts: Vec<(Comb, Vec<String>)> = Vec::new();
    let mut comb = Comb::None;
    let mut simples: Vec<String> = Vec::new();
    let mut i = 0;
    let flush =
        |parts: &mut Vec<(Comb, Vec<String>)>, comb: &mut Comb, simples: &mut Vec<String>| {
            if !simples.is_empty() {
                parts.push((*comb, std::mem::take(simples)));
                *comb = Comb::Desc;
            }
        };
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() || matches!(c, '>' | '+' | '~') {
            // Combinateur : espaces autour, `>`, `+`, `~`.
            let mut j = i;
            let mut sym = None;
            while j < chars.len()
                && (chars[j].is_whitespace() || matches!(chars[j], '>' | '+' | '~'))
            {
                if !chars[j].is_whitespace() {
                    if sym.is_some() {
                        return None;
                    }
                    sym = Some(chars[j]);
                }
                j += 1;
            }
            if simples.is_empty() {
                if parts.is_empty() {
                    return None;
                }
                return None;
            }
            flush(&mut parts, &mut comb, &mut simples);
            comb = match sym {
                None => Comb::Desc,
                Some('>') => Comb::Child,
                Some('+') => Comb::Next,
                Some(_) => Comb::After,
            };
            i = j;
            continue;
        }
        // Sélecteur simple.
        let start = i;
        match c {
            '[' => {
                let mut quote: Option<char> = None;
                i += 1;
                while i < chars.len() {
                    let d = chars[i];
                    match quote {
                        Some(q) if d == q => quote = None,
                        Some(_) => {}
                        None if d == '"' || d == '\'' => quote = Some(d),
                        None if d == ']' => break,
                        None => {}
                    }
                    i += 1;
                }
                if i >= chars.len() {
                    return None;
                }
                i += 1;
            }
            ':' => {
                i += 1;
                if i < chars.len() && chars[i] == ':' {
                    i += 1;
                }
                while i < chars.len() && is_ident(chars[i]) {
                    i += 1;
                }
                if i < chars.len() && chars[i] == '(' {
                    let mut depth = 0;
                    while i < chars.len() {
                        match chars[i] {
                            '(' => depth += 1,
                            ')' => {
                                depth -= 1;
                                if depth == 0 {
                                    i += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                    if depth != 0 {
                        return None;
                    }
                }
            }
            '.' | '#' => {
                i += 1;
                while i < chars.len() && (is_ident(chars[i]) || chars[i] == '\\') {
                    if chars[i] == '\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            '*' => i += 1,
            _ if is_ident(c) => {
                while i < chars.len() && is_ident(chars[i]) {
                    i += 1;
                }
            }
            _ => return None,
        }
        let tok: String = chars[start..i.min(chars.len())].iter().collect();
        if tok.is_empty() {
            return None;
        }
        simples.push(tok);
    }
    if simples.is_empty() {
        return None;
    }
    flush(&mut parts, &mut comb, &mut simples);
    Some(Complex { parts })
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_' || !c.is_ascii()
}

impl Complex {
    pub fn write(&self) -> String {
        let mut s = String::new();
        for (i, (c, simples)) in self.parts.iter().enumerate() {
            if i > 0 {
                s.push_str(match c {
                    Comb::None | Comb::Desc => " ",
                    Comb::Child => " > ",
                    Comb::Next => " + ",
                    Comb::After => " ~ ",
                });
            }
            s.push_str(&simples.concat());
        }
        s
    }

    pub fn subject(&self) -> &[String] {
        self.parts.last().map(|p| p.1.as_slice()).unwrap_or(&[])
    }
}

/// Pseudo-élément (`::after`, ou `:after` à l'ancienne) : son nom.
pub fn pseudo_element(tok: &str) -> Option<&str> {
    if let Some(n) = tok.strip_prefix("::") {
        return Some(n);
    }
    let n = tok.strip_prefix(':')?;
    matches!(n, "before" | "after" | "first-line" | "first-letter").then_some(n)
}

/// États qui dépendent du visiteur (jamais vrais dans une page rendue sans visiteur).
pub const DYNAMIC: &[&str] = &[
    ":hover",
    ":focus",
    ":focus-visible",
    ":focus-within",
    ":active",
    ":visited",
    ":target",
    ":checked",
    ":disabled",
    ":enabled",
    ":invalid",
    ":valid",
    ":placeholder-shown",
    ":user-invalid",
];

/// États du panneau Styles que la mise à plat sait reporter sur la classe de l'élément.
pub const PANEL_STATES: &[&str] = &[":hover", ":focus", ":focus-visible", ":active", ":disabled"];

/// Lecture d'un sélecteur pour la mise à plat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    /// Forme sans état ni pseudo-élément du sujet, pour chercher les éléments.
    pub static_sel: String,
    /// État du sujet (`:hover`…).
    pub state: Option<String>,
    /// Pseudo-élément du sujet (`after`…).
    pub pseudo: Option<String>,
    /// Classe de l'ancêtre le plus proche écrit dans le sélecteur, sinon balise du sujet (nom des
    /// nouvelles classes).
    pub context: Option<String>,
    /// État dans un ancêtre hors des états du panneau, ou forme non reportable.
    pub unsupported: bool,
    /// État d'un ancêtre reporté devant la classe (`[open] .x`, `.is-open .x`).
    pub outer: Option<String>,
    /// Classes posées par un script sur le sujet (`.is-active`), reportées en classes combinées.
    pub extra: Vec<String>,
}

/// `dynamic` : classe posée par un script (absente des pages).
pub fn shape(c: &Complex, dynamic: &dyn Fn(&str) -> bool) -> Shape {
    let mut unsupported = false;
    let mut state = None;
    let mut pseudo = None;
    let mut outer: Option<String> = None;
    let mut extra = Vec::new();
    let n = c.parts.len();
    let mut parts = c.parts.clone();
    let mut subject_tag = None;
    for (k, (_, simples)) in parts.iter_mut().enumerate() {
        let subject = k + 1 == n;
        let mut kept = Vec::new();
        for t in simples.drain(..) {
            if let Some(p) = pseudo_element(&t) {
                if subject && pseudo.is_none() {
                    pseudo = Some(p.to_string());
                } else {
                    unsupported = true;
                }
                continue;
            }
            if DYNAMIC.contains(&t.as_str()) {
                if subject && state.is_none() && PANEL_STATES.contains(&t.as_str()) {
                    state = Some(t);
                } else {
                    unsupported = true;
                }
                continue;
            }
            let is_dyn_class = t.strip_prefix('.').is_some_and(dynamic);
            // État d'un ancêtre (accordéon ouvert, menu ouvert, élément actif) : cherché sans
            // lui, reporté devant la classe.
            if !subject && (t == "[open]" || t == "[aria-pressed=\"false\"]" || is_dyn_class) {
                if OUTER.contains(&t.as_str()) && outer.is_none() {
                    outer = Some(t);
                } else {
                    unsupported = true;
                }
                continue;
            }
            if subject && is_dyn_class {
                extra.push(t);
                continue;
            }
            if subject && t.starts_with(|c: char| c.is_ascii_alphabetic()) {
                subject_tag = Some(t.clone());
            }
            kept.push(t);
        }
        if kept.is_empty() {
            kept.push("*".into());
        }
        *simples = kept;
    }
    // État et pseudo-élément ensemble (`.a:hover::after`) : non reporté.
    if state.is_some() && pseudo.is_some() {
        unsupported = true;
    }
    let context = c.parts[..n.saturating_sub(1)]
        .iter()
        .rev()
        .flat_map(|(_, s)| s.iter())
        .find_map(|t| {
            t.strip_prefix('.')
                .filter(|c| !dynamic(c))
                .map(str::to_string)
        })
        .or(subject_tag);
    Shape {
        static_sel: Complex { parts }.write(),
        state,
        pseudo,
        context,
        unsupported,
        outer,
        extra,
    }
}

/// États d'un ancêtre que le panneau sait régler (`revel_edit::styleaudit::OUTER_STATES`).
pub const OUTER: &[&str] = &[
    ".is-active",
    "[open]",
    "[aria-pressed=\"false\"]",
    ".is-paused",
    ".is-playing",
    ".is-open",
    ".is-empty",
];

/// Spécificité (identifiants, classes et attributs et pseudo-classes, balises et
/// pseudo-éléments).
pub fn specificity(c: &Complex) -> (u32, u32, u32) {
    let mut s = (0, 0, 0);
    for (_, simples) in &c.parts {
        for t in simples {
            let add = simple_specificity(t);
            s.0 += add.0;
            s.1 += add.1;
            s.2 += add.2;
        }
    }
    s
}

fn simple_specificity(t: &str) -> (u32, u32, u32) {
    if t == "*" {
        return (0, 0, 0);
    }
    if pseudo_element(t).is_some() {
        return (0, 0, 1);
    }
    match t.chars().next() {
        Some('#') => (1, 0, 0),
        Some('.') | Some('[') => (0, 1, 0),
        Some(':') => {
            let name = t.trim_start_matches(':');
            let (fname, args) = match name.find('(') {
                Some(i) => (&name[..i], Some(&name[i + 1..name.len().saturating_sub(1)])),
                None => (name, None),
            };
            match (fname, args) {
                ("where", _) => (0, 0, 0),
                ("is" | "not" | "has" | "matches", Some(a)) => split_list(a)
                    .iter()
                    .filter_map(|x| parse(x))
                    .map(|c| specificity(&c))
                    .max()
                    .unwrap_or((0, 0, 0)),
                _ => (0, 1, 0),
            }
        }
        _ => (0, 0, 1),
    }
}

/// Classes écrites dans le sélecteur (`.a`, `:not(.b)` compris).
pub fn classes(sel: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b: Vec<char> = sel.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '.' && (i == 0 || !b[i - 1].is_ascii_digit()) {
            let mut j = i + 1;
            while j < b.len() && (is_ident(b[j]) || b[j] == '\\') {
                if b[j] == '\\' {
                    j += 1;
                }
                j += 1;
            }
            if j > i + 1 {
                out.push(
                    b[i + 1..j.min(b.len())]
                        .iter()
                        .collect::<String>()
                        .replace('\\', ""),
                );
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_et_specificite() {
        assert_eq!(
            split_list("h1, h2 , .a:is(.b, .c)"),
            vec!["h1", "h2", ".a:is(.b, .c)"]
        );
        let c = parse(".page-hero  > .lead:hover::after").unwrap();
        assert_eq!(c.parts.len(), 2);
        assert_eq!(c.parts[1].0, Comb::Child);
        assert_eq!(c.write(), ".page-hero > .lead:hover::after");
        assert_eq!(specificity(&c), (0, 3, 1));
        assert_eq!(
            specificity(&parse("[data-framer-component-type=Text] div div span").unwrap()),
            (0, 1, 3)
        );
        assert_eq!(
            specificity(&parse("p.framer-text:not(:first-child)").unwrap()),
            (0, 2, 1)
        );
        assert_eq!(specificity(&parse(":where(.a) #b").unwrap()), (1, 0, 0));
        let dynamic = |c: &str| c.starts_with("is-");
        let s = shape(
            &parse(".faq[open] .faq__question::after").unwrap(),
            &dynamic,
        );
        assert_eq!(s.static_sel, ".faq .faq__question");
        assert_eq!(s.pseudo.as_deref(), Some("after"));
        assert!(s.outer.as_deref() == Some("[open]") && !s.unsupported);
        assert_eq!(s.context.as_deref(), Some("faq"));
        let s = shape(&parse(".card:hover .card__image").unwrap(), &dynamic);
        assert!(s.unsupported);
        let s = shape(
            &parse(".navbar__inner.is-open .navbar__menu").unwrap(),
            &dynamic,
        );
        assert_eq!(
            (s.static_sel.as_str(), s.outer.as_deref()),
            (".navbar__inner .navbar__menu", Some(".is-open"))
        );
        let s = shape(&parse(".tabs .tab.is-active").unwrap(), &dynamic);
        assert_eq!(
            (s.static_sel.as_str(), s.extra.clone()),
            (".tabs .tab", vec![".is-active".to_string()])
        );
        let s = shape(&parse("textarea.field").unwrap(), &dynamic);
        assert_eq!(s.context.as_deref(), Some("textarea"));
        let s = shape(&parse(".navbar__link:hover").unwrap(), &dynamic);
        assert_eq!(
            (s.static_sel.as_str(), s.state.as_deref()),
            (".navbar__link", Some(":hover"))
        );
        assert_eq!(classes(".a.b-c:not(.d) > p.e"), vec!["a", "b-c", "d", "e"]);
        assert!(parse("a >").is_none());
    }
}
