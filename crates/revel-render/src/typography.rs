//! Typographie française appliquée au texte visible d'une page HTML :
//! apostrophe typographique (’), espace insécable avant `? ! ; : % »` et après `«`.
//! Les balises, les attributs et le contenu de `script`, `style`, `pre`, `code`, `textarea`
//! ne sont pas modifiés. Les repères de l'éditeur (`<!--rv:N-->`, `<!--/rv:N-->`) sont
//! transparents : le texte qu'ils entourent est traité comme s'ils n'existaient pas.

const NBSP: char = '\u{a0}';
const PROTECTED: &[&str] = &["script", "style", "pre", "code", "textarea"];
/// Place d'un repère de l'éditeur dans le texte en cours de traitement.
const MARK: char = '\u{e000}';

pub(crate) fn french(html: &str) -> String {
    // Texte contenant déjà le caractère réservé : repères traités comme des balises.
    let transparent = !html.contains(MARK);
    let mut out = String::with_capacity(html.len() + html.len() / 50);
    let mut text = String::new();
    let mut marks: Vec<&str> = Vec::new();
    let flush = |out: &mut String, text: &mut String, marks: &mut Vec<&str>| {
        let fixed = fix_text(text);
        let mut m = marks.drain(..);
        for c in fixed.chars() {
            if c == MARK {
                out.push_str(m.next().unwrap_or_default());
            } else {
                out.push(c);
            }
        }
        text.clear();
    };
    let mut rest = html;
    while !rest.is_empty() {
        let Some(lt) = rest.find('<') else {
            text.push_str(rest);
            break;
        };
        text.push_str(&rest[..lt]);
        rest = &rest[lt..];
        if transparent
            && (rest.starts_with("<!--rv:") || rest.starts_with("<!--/rv:"))
            && let Some(end) = rest.find("-->")
        {
            marks.push(&rest[..end + 3]);
            text.push(MARK);
            rest = &rest[end + 3..];
            continue;
        }
        flush(&mut out, &mut text, &mut marks);
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
    flush(&mut out, &mut text, &mut marks);
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
    // Caractère précédent / suivant, repères ignorés.
    let last = |out: &[char]| out.iter().rev().find(|c| **c != MARK).copied();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\''
            && last(&out).is_some_and(|p| p.is_alphabetic())
            && chars[i + 1..]
                .iter()
                .find(|n| **n != MARK)
                .is_some_and(|n| n.is_alphabetic())
        {
            out.push('’');
            i += 1;
            continue;
        }
        if c == ' ' || c == NBSP {
            // Suite d'espaces suivie d'un signe : une seule espace insécable (repères gardés).
            let mut j = i;
            let mut held = 0;
            while j < chars.len() && matches!(chars[j], ' ' | NBSP | MARK) {
                held += usize::from(chars[j] == MARK);
                j += 1;
            }
            let sign = j < chars.len() && matches!(chars[j], '?' | '!' | ';' | ':' | '%' | '»');
            if sign || last(&out) == Some('«') {
                out.push(NBSP);
                out.extend(std::iter::repeat_n(MARK, held));
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

    #[test]
    fn reperes_de_l_editeur_transparents() {
        let strip = |s: &str| {
            regex::Regex::new(r"<!--/?rv:\d+-->")
                .unwrap()
                .replace_all(s, "")
                .into_owned()
        };
        for h in [
            "<p>L<!--rv:0-->'<!--/rv:0-->agence <!--rv:1-->oui<!--/rv:1--> !</p>",
            "<p>«<!--rv:2--> oui <!--/rv:2-->» : a<!--rv:3--> <!--/rv:3-->?</p>",
            "<p><!--rv:4-->Bonjour<!--/rv:4--> <!--rv:5-->:<!--/rv:5--> 5 %</p>",
        ] {
            let r = french(h);
            assert_eq!(strip(&r), french(&strip(h)), "{h}");
            assert_eq!(
                r.matches("<!--").count(),
                h.matches("<!--").count(),
                "repères conservés"
            );
        }
        // Un autre commentaire reste une balise : le texte qui le suit est traité à part.
        assert_eq!(french("a <!-- x -->b !"), "a <!-- x -->b\u{a0}!");
        assert_eq!(french("a<!-- x -->'b"), "a<!-- x -->'b");
    }
}
