//! Typographie française appliquée au texte visible d'une page HTML :
//! apostrophe typographique (’), espace insécable avant `? ! ; : % »` et après `«`.
//! Les balises, les attributs et le contenu de `script`, `style`, `pre`, `code`, `textarea`
//! ne sont pas modifiés.

const NBSP: char = '\u{a0}';
const PROTECTED: &[&str] = &["script", "style", "pre", "code", "textarea"];

pub(crate) fn french(html: &str) -> String {
    let mut out = String::with_capacity(html.len() + html.len() / 50);
    let mut rest = html;
    while !rest.is_empty() {
        let Some(lt) = rest.find('<') else {
            out.push_str(&fix_text(rest));
            break;
        };
        out.push_str(&fix_text(&rest[..lt]));
        rest = &rest[lt..];
        // Élément protégé : recopié jusqu'à sa fermeture.
        let lower = rest[1..].to_ascii_lowercase();
        if let Some(tag) = PROTECTED.iter().find(|t| {
            lower.starts_with(*t)
                && lower[t.len()..].starts_with(|c: char| c == '>' || c.is_whitespace())
        }) {
            let close = format!("</{tag}");
            let end = rest
                .to_ascii_lowercase()
                .find(&close)
                .and_then(|i| rest[i..].find('>').map(|j| i + j + 1))
                .unwrap_or(rest.len());
            out.push_str(&rest[..end]);
            rest = &rest[end..];
            continue;
        }
        let end = tag_end(rest);
        out.push_str(&rest[..end]);
        rest = &rest[end..];
    }
    out
}

/// Fin d'une balise (`>` hors guillemets), commentaires compris.
fn tag_end(s: &str) -> usize {
    if s.starts_with("<!--") {
        return s.find("-->").map(|i| i + 3).unwrap_or(s.len());
    }
    let mut quote: Option<char> = None;
    for (i, c) in s.char_indices().skip(1) {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '>' => return i + 1,
            None => {}
        }
    }
    s.len()
}

fn fix_text(t: &str) -> String {
    if t.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = t.chars().collect();
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\''
            && out.last().is_some_and(|p| p.is_alphabetic())
            && chars.get(i + 1).is_some_and(|n| n.is_alphabetic())
        {
            out.push('’');
            i += 1;
            continue;
        }
        if c == ' ' || c == NBSP {
            // Suite d'espaces suivie d'un signe : une seule espace insécable.
            let mut j = i;
            while j < chars.len() && (chars[j] == ' ' || chars[j] == NBSP) {
                j += 1;
            }
            if j < chars.len() && matches!(chars[j], '?' | '!' | ';' | ':' | '%' | '»') {
                out.push(NBSP);
                i = j;
                continue;
            }
            if out.last() == Some(&'«') {
                out.push(NBSP);
                i = j;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn espaces_et_apostrophes() {
        let h = r#"<p title="a : b">L'agence : « oui » ! 50 %</p><script>a = 'x' ? 1 : 2</script>"#;
        let r = french(h);
        assert_eq!(
            r,
            "<p title=\"a : b\">L’agence\u{a0}: «\u{a0}oui\u{a0}»\u{a0}! 50\u{a0}%</p><script>a = 'x' ? 1 : 2</script>"
        );
    }
}
