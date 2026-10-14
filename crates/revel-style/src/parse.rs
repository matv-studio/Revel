//! Lecture d'un CSS existant en éléments du modèle, en conservant le texte d'origine des
//! sélecteurs et des valeurs (aucune réécriture des couleurs, unités ou raccourcis).
//!
//! Structure reconnue :
//! - règle `sélecteur { déclarations }` -> [`StyleItem::Rule`] ;
//! - `@media requête { règles }` -> règles avec `media` (si le bloc ne contient que des règles
//!   et des commentaires ; sinon bloc brut) ;
//! - commentaire de premier niveau ou dans un bloc `@media` -> [`StyleItem::Comment`] ;
//! - toute autre règle `@…` (`@font-face`, `@keyframes`, `@import`, `@supports`…), règle
//!   imbriquée (CSS nesting) -> [`StyleItem::Raw`], texte conservé.

use revel_model::{Decl, StyleItem, StyleRule};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("bloc non fermé (ligne {0})")]
    Unclosed(usize),
    #[error("accolade fermante en trop (ligne {0})")]
    Unexpected(usize),
    #[error("commentaire non fermé (ligne {0})")]
    Comment(usize),
}

struct Cursor<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn line(&self) -> usize {
        self.s[..self.pos].matches('\n').count() + 1
    }

    fn rest(&self) -> &'a str {
        &self.s[self.pos..]
    }

    fn skip_ws(&mut self) {
        let r = self.rest();
        self.pos += r.len() - r.trim_start().len();
    }

    /// Avance jusqu'au premier `stop` de niveau 0 (hors chaînes, parenthèses, crochets,
    /// commentaires) ; renvoie le texte parcouru (commentaires compris) sans le délimiteur.
    fn until(&mut self, stops: &[char]) -> Result<(&'a str, Option<char>), ParseError> {
        let start = self.pos;
        let b = self.s.as_bytes();
        let mut depth = 0i32;
        let mut i = self.pos;
        while i < b.len() {
            let c = b[i] as char;
            match c {
                '/' if b.get(i + 1) == Some(&b'*') => {
                    let end = self.s[i + 2..]
                        .find("*/")
                        .ok_or(ParseError::Comment(self.line()))?;
                    i += 2 + end + 2;
                    continue;
                }
                '"' | '\'' => {
                    let mut j = i + 1;
                    while j < b.len() && b[j] as char != c {
                        if b[j] == b'\\' {
                            j += 1;
                        }
                        j += 1;
                    }
                    i = j + 1;
                    continue;
                }
                '\\' => {
                    i += 2;
                    continue;
                }
                '(' | '[' => depth += 1,
                ')' | ']' => depth -= 1,
                _ if depth <= 0 && stops.contains(&c) => {
                    self.pos = i + 1;
                    return Ok((&self.s[start..i], Some(c)));
                }
                _ => {}
            }
            i += c.len_utf8().max(1);
        }
        self.pos = b.len();
        Ok((&self.s[start..], None))
    }

    /// Contenu d'un bloc après `{` jusqu'à l'accolade fermante correspondante.
    fn block(&mut self) -> Result<&'a str, ParseError> {
        let line = self.line();
        let start = self.pos;
        let mut depth = 1;
        loop {
            let (_, c) = self.until(&['{', '}'])?;
            match c {
                Some('{') => depth += 1,
                Some('}') => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(&self.s[start..self.pos - 1]);
                    }
                }
                _ => return Err(ParseError::Unclosed(line)),
            }
        }
    }
}

/// Retire les commentaires d'un texte (hors chaînes).
fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c == '/' && b.get(i + 1) == Some(&b'*') {
            match s[i + 2..].find("*/") {
                Some(e) => {
                    i += 2 + e + 2;
                    out.push(' ');
                    continue;
                }
                None => break,
            }
        }
        if c == '"' || c == '\'' {
            let mut j = i + 1;
            while j < b.len() && b[j] as char != c {
                if b[j] == b'\\' {
                    j += 1;
                }
                j += 1;
            }
            let j = (j + 1).min(b.len());
            out.push_str(&s[i..j]);
            i = j;
            continue;
        }
        let ch = s[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Espaces consécutifs ramenés à un seul (hors chaînes), bords retirés.
pub fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut quote: Option<char> = None;
    let mut space = false;
    for c in s.chars() {
        if let Some(q) = quote {
            out.push(c);
            if c == q {
                quote = None;
            }
            continue;
        }
        if c == '"' || c == '\'' {
            quote = Some(c);
        }
        if c.is_whitespace() {
            space = true;
            continue;
        }
        if space && !out.is_empty() {
            out.push(' ');
        }
        space = false;
        out.push(c);
    }
    out
}

/// Déclarations d'un bloc ; `None` si le bloc contient des règles imbriquées.
fn declarations(block: &str) -> Result<Option<Vec<Decl>>, ParseError> {
    let text = strip_comments(block);
    let mut c = Cursor { s: &text, pos: 0 };
    let mut decls = Vec::new();
    loop {
        let (part, stop) = c.until(&[';', '{', '}'])?;
        if matches!(stop, Some('{') | Some('}')) {
            return Ok(None);
        }
        let part = part.trim();
        if !part.is_empty() {
            let mut pc = Cursor { s: part, pos: 0 };
            let (prop, colon) = pc.until(&[':'])?;
            if colon.is_none() {
                // Texte sans « : » : non reconnu, bloc conservé tel quel.
                return Ok(None);
            }
            let mut value = collapse_ws(pc.rest());
            let mut important = false;
            let lower = value.to_ascii_lowercase();
            if let Some(i) = lower.rfind('!')
                && lower[i + 1..].trim() == "important"
            {
                important = true;
                value = value[..i].trim_end().to_string();
            }
            decls.push(Decl {
                prop: prop.trim().to_string(),
                value,
                important,
            });
        }
        if stop.is_none() {
            return Ok(Some(decls));
        }
    }
}

/// Lit une feuille de style.
pub fn parse(css: &str) -> Result<Vec<StyleItem>, ParseError> {
    parse_list(css, None)
}

fn parse_list(css: &str, media: Option<&str>) -> Result<Vec<StyleItem>, ParseError> {
    let mut c = Cursor { s: css, pos: 0 };
    let mut items = Vec::new();
    loop {
        c.skip_ws();
        let rest = c.rest();
        if rest.is_empty() {
            return Ok(items);
        }
        if let Some(r) = rest.strip_prefix("/*") {
            let end = r.find("*/").ok_or(ParseError::Comment(c.line()))?;
            items.push(StyleItem::Comment {
                text: r[..end].trim().to_string(),
                media: media.map(str::to_string),
            });
            c.pos += 2 + end + 2;
            continue;
        }
        if rest.starts_with('}') {
            return Err(ParseError::Unexpected(c.line()));
        }
        let start = c.pos;
        let (prelude, stop) = c.until(&['{', ';', '}'])?;
        if stop == Some('}') {
            return Err(ParseError::Unexpected(c.line()));
        }
        let is_at = prelude.trim_start().starts_with('@');
        if stop != Some('{') {
            // Instruction sans bloc (`@import …;`, `@charset`), ou texte final.
            let raw = c.s[start..c.pos].trim();
            if !raw.is_empty() {
                items.push(StyleItem::Raw {
                    css: raw.to_string(),
                });
            }
            if stop.is_none() {
                return Ok(items);
            }
            continue;
        }
        let block = c.block()?;
        let whole = c.s[start..c.pos].trim();
        if is_at {
            let p = collapse_ws(&strip_comments(prelude));
            let name = p[1..]
                .split([' ', '('])
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if name == "media" && media.is_none() {
                let query = p["@media".len()..].trim().to_string();
                match parse_list(block, Some(&query)) {
                    Ok(inner)
                        if inner.iter().all(|i| {
                            matches!(i, StyleItem::Rule(_) | StyleItem::Comment { .. })
                        }) =>
                    {
                        items.extend(inner);
                    }
                    _ => items.push(StyleItem::Raw {
                        css: whole.to_string(),
                    }),
                }
            } else {
                items.push(StyleItem::Raw {
                    css: whole.to_string(),
                });
            }
            continue;
        }
        match declarations(block)? {
            Some(decls) => items.push(StyleItem::Rule(StyleRule {
                id: None,
                selector: collapse_ws(&strip_comments(prelude)),
                media: media.map(str::to_string),
                decls,
            })),
            None => items.push(StyleItem::Raw {
                css: whole.to_string(),
            }),
        }
    }
}
