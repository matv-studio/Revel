//! Réécriture d'expressions : tournures JavaScript de Nunjucks -> syntaxe Jinja (minijinja).

/// Jeton d'expression : chaîne littérale ou autre texte.
#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Str(String),
    Code(String),
}

fn tokenize(src: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' || c == '\'' {
            if !cur.is_empty() {
                out.push(Tok::Code(std::mem::take(&mut cur)));
            }
            let mut s = String::from(c);
            while let Some(d) = chars.next() {
                s.push(d);
                if d == '\\' {
                    if let Some(e) = chars.next() {
                        s.push(e);
                    }
                } else if d == c {
                    break;
                }
            }
            out.push(Tok::Str(s));
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(Tok::Code(cur));
    }
    out
}

fn join(toks: &[Tok]) -> String {
    toks.iter()
        .map(|t| match t {
            Tok::Str(s) | Tok::Code(s) => s.as_str(),
        })
        .collect()
}

/// Applique `f` au code hors chaînes littérales.
fn map_code(src: &str, f: impl Fn(&str) -> String) -> String {
    let toks: Vec<Tok> = tokenize(src)
        .into_iter()
        .map(|t| match t {
            Tok::Code(c) => Tok::Code(f(&c)),
            s => s,
        })
        .collect();
    join(&toks)
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Remplace un mot entier (hors chaînes).
fn replace_word(code: &str, word: &str, by: &str) -> String {
    let mut out = String::new();
    let mut rest = code;
    while let Some(i) = rest.find(word) {
        let before_ok = rest[..i]
            .chars()
            .last()
            .is_none_or(|c| !is_ident(c) && c != '.');
        let after_ok = rest[i + word.len()..]
            .chars()
            .next()
            .is_none_or(|c| !is_ident(c));
        out.push_str(&rest[..i]);
        out.push_str(if before_ok && after_ok { by } else { word });
        rest = &rest[i + word.len()..];
    }
    out.push_str(rest);
    out
}

/// Convertit une expression Nunjucks en expression Jinja équivalente.
pub fn from_nunjucks(src: &str) -> String {
    let mut s = rewrite_push(src);
    s = map_code(&s, |c| {
        let mut c = c.replace("===", "==").replace("!==", "!=");
        c = c.replace("&&", " and ").replace("||", " or ");
        c = c.replace(".startsWith(", " is startingwith(");
        c = c.replace(".endsWith(", " is endingwith(");
        for (m, f) in [
            (".trim()", "|trim"),
            (".toLowerCase()", "|lower"),
            (".toUpperCase()", "|upper"),
            (".toUTCString()", "|utc_string"),
            (".toISOString()", "|iso_date"),
        ] {
            c = c.replace(m, f);
        }
        c = replace_length(&c);
        c = replace_word(&c, "null", "none");
        c = replace_word(&c, "undefined", "none");
        replace_not(&c)
    });
    s = quote_object_keys(&s);
    s.trim().to_string()
}

/// `x.length` -> `x|length` (hors appel de méthode).
fn replace_length(code: &str) -> String {
    let mut out = String::new();
    let mut rest = code;
    while let Some(i) = rest.find(".length") {
        let after = rest[i + 7..].chars().next();
        out.push_str(&rest[..i]);
        if after.is_none_or(|c| !is_ident(c) && c != '(') {
            out.push_str("|length");
        } else {
            out.push_str(".length");
        }
        rest = &rest[i + 7..];
    }
    out.push_str(rest);
    out
}

/// `!x` -> `not x` (sans toucher `!=`).
fn replace_not(code: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = code.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '!' && chars.get(i + 1) != Some(&'=') {
            out.push_str("not ");
        } else {
            out.push(c);
        }
    }
    out
}

/// `(a.push(x), a)` -> `(a + [x])`.
fn rewrite_push(src: &str) -> String {
    let mut s = src.to_string();
    while let Some(i) = s.find(".push(") {
        // Nom avant `.push(`.
        let name_start = s[..i]
            .rfind(|c: char| !is_ident(c) && c != '.')
            .map(|p| p + 1)
            .unwrap_or(0);
        let name = s[name_start..i].to_string();
        let args_start = i + 6;
        let Some(args_end) = matching_paren(&s, args_start - 1) else {
            break;
        };
        let arg = s[args_start..args_end].to_string();
        // Motif complet attendu : `.push(arg), name)`.
        let tail = s[args_end + 1..].trim_start().to_string();
        let consumed = [format!(", {name})"), format!(",{name})")]
            .into_iter()
            .find(|p| tail.starts_with(p.as_str()))
            .map(|p| p.len());
        s = match consumed {
            Some(n) => format!("{}{} + [{}]){}", &s[..name_start], name, arg, &tail[n..]),
            None => format!(
                "{}{} + [{}]{}",
                &s[..name_start],
                name,
                arg,
                &s[args_end + 1..]
            ),
        };
    }
    s
}

/// Position de la parenthèse fermante correspondant à celle en `open` (hors chaînes).
fn matching_paren(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    let mut quote: Option<char> = None;
    let mut prev = ' ';
    for (i, c) in s[open..].char_indices() {
        match quote {
            Some(q) => {
                if c == q && prev != '\\' {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' => quote = Some(c),
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(open + i);
                    }
                }
                _ => {}
            },
        }
        prev = c;
    }
    None
}

/// `{ label: x }` -> `{"label": x}`.
fn quote_object_keys(src: &str) -> String {
    let toks = tokenize(src);
    let mut out = Vec::new();
    let mut brace_depth = 0usize;
    for t in toks {
        match t {
            Tok::Code(c) => {
                let mut res = String::new();
                let chars: Vec<char> = c.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    let ch = chars[i];
                    if ch == '{' {
                        brace_depth += 1;
                    } else if ch == '}' {
                        brace_depth = brace_depth.saturating_sub(1);
                    }
                    if brace_depth > 0 && (is_ident(ch) && !ch.is_numeric()) {
                        // Début d'identifiant après `{` ou `,` ?
                        let prev = res.trim_end().chars().last();
                        if matches!(prev, Some('{') | Some(',')) {
                            let start = i;
                            while i < chars.len() && is_ident(chars[i]) {
                                i += 1;
                            }
                            let ident: String = chars[start..i].iter().collect();
                            let mut j = i;
                            while j < chars.len() && chars[j] == ' ' {
                                j += 1;
                            }
                            if j < chars.len() && chars[j] == ':' {
                                res.push('"');
                                res.push_str(&ident);
                                res.push('"');
                            } else {
                                res.push_str(&ident);
                            }
                            continue;
                        }
                    }
                    res.push(ch);
                    i += 1;
                }
                out.push(Tok::Code(res));
            }
            s => out.push(s),
        }
    }
    join(&out)
}

/// Renomme des filtres (`| bySlug` -> `| by_slug`) hors chaînes littérales.
pub fn rename_filters(src: &str, map: &[(&str, &str)]) -> String {
    map_code(src, |c| {
        let mut c = c.to_string();
        for (from, to) in map {
            let mut out = String::new();
            let mut rest = c.as_str();
            while let Some(i) = rest.find('|') {
                out.push_str(&rest[..=i]);
                let after = &rest[i + 1..];
                let trimmed = after.trim_start();
                let ws = &after[..after.len() - trimmed.len()];
                if trimmed.starts_with(from)
                    && trimmed[from.len()..]
                        .chars()
                        .next()
                        .is_none_or(|c| !is_ident(c))
                {
                    out.push_str(ws);
                    out.push_str(to);
                    rest = &trimmed[from.len()..];
                } else {
                    rest = after;
                }
            }
            out.push_str(rest);
            c = out;
        }
        c
    })
}

/// Découpe une liste d'arguments séparés par des virgules au premier niveau.
pub fn split_args(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut prev = ' ';
    for c in src.chars() {
        match quote {
            Some(q) => {
                cur.push(c);
                if c == q && prev != '\\' {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' => {
                    quote = Some(c);
                    cur.push(c);
                }
                '(' | '[' | '{' => {
                    depth += 1;
                    cur.push(c);
                }
                ')' | ']' | '}' => {
                    depth -= 1;
                    cur.push(c);
                }
                ',' if depth == 0 => out.push(std::mem::take(&mut cur).trim().to_string()),
                _ => cur.push(c),
            },
        }
        prev = c;
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// Applique `f` aux expressions d'un gabarit de valeur (`{{ e }}` et conditions `{% if c %}`/`{% elif c %}`).
pub fn map_template(src: &str, f: &dyn Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut rest = src;
    loop {
        let a = rest.find("{{");
        let b = rest.find("{%");
        let start = match (a, b) {
            (Some(a), Some(b)) => a.min(b),
            (Some(x), None) | (None, Some(x)) => x,
            (None, None) => break,
        };
        out.push_str(&rest[..start]);
        let is_out = rest[start..].starts_with("{{");
        let close = if is_out { "}}" } else { "%}" };
        let Some(rel) = rest[start + 2..].find(close) else {
            break;
        };
        let inner = rest[start + 2..start + 2 + rel].trim();
        if is_out {
            out.push_str(&format!("{{{{ {} }}}}", f(inner)));
        } else {
            let kw = inner.split_whitespace().next().unwrap_or("");
            let args = inner[kw.len()..].trim();
            if matches!(kw, "if" | "elif") {
                out.push_str(&format!("{{% {kw} {} %}}", f(args)));
            } else {
                out.push_str(&format!("{{% {inner} %}}"));
            }
        }
        rest = &rest[start + 2 + rel + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longueur_et_methodes() {
        assert_eq!(
            from_nunjucks("hook.length or chapeau"),
            "hook|length or chapeau"
        );
        assert_eq!(from_nunjucks("not news.length"), "not news|length");
        assert_eq!(
            from_nunjucks(r#"(page.url or "").startsWith(item.url)"#),
            r#"(page.url or "") is startingwith(item.url)"#
        );
        assert_eq!(from_nunjucks(r#""a.length""#), r#""a.length""#);
        assert_eq!(
            from_nunjucks("it.data.client and it.data.client.trim()"),
            "it.data.client and it.data.client|trim"
        );
        assert_eq!(
            from_nunjucks("item.date.toUTCString()"),
            "item.date|utc_string"
        );
    }

    #[test]
    fn push_et_objets() {
        assert_eq!(
            from_nunjucks("(heroTags.push({ label: ex.data.title, url: ex.url }), heroTags)"),
            r#"(heroTags + [{ "label": ex.data.title, "url": ex.url }])"#
        );
        assert_eq!(
            from_nunjucks("(featured.push(it), featured)"),
            "(featured + [it])"
        );
    }

    #[test]
    fn filtres_renommes() {
        assert_eq!(
            rename_filters(
                "collections.projets | bySlug(s) | bySlugs(x)",
                &[("bySlug", "by_slug"), ("bySlugs", "by_slugs")]
            ),
            "collections.projets | by_slug(s) | by_slugs(x)"
        );
        assert_eq!(
            rename_filters(r#""| bySlug""#, &[("bySlug", "x")]),
            r#""| bySlug""#
        );
    }

    #[test]
    fn arguments() {
        assert_eq!(
            split_args(
                r#"b.image, b.alt or "", ("100vw" if b.style != "column" else "a, b"), "block__img""#
            ),
            vec![
                "b.image",
                r#"b.alt or """#,
                r#"("100vw" if b.style != "column" else "a, b")"#,
                r#""block__img""#
            ]
        );
    }
}
