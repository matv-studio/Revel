//! Textes à traduire : découpage en phrases, mise en forme gardée (Markdown, HTML), mots protégés.
//!
//! Le modèle traduit des phrases de texte brut. Les éléments à garder tels quels (noms de marque,
//! adresses web, expressions `{{ … }}`) et les éléments en ligne (gras, lien, code) sont remplacés
//! dans la phrase par des repères que le modèle recopie (`XQZ`, `XQY`…) ; le texte d'un lien ou
//! d'un passage en gras est traduit à part, puis remis à sa place.

use serde::{Deserialize, Serialize};

/// Forme d'un texte.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    /// Texte simple (une ligne ou plusieurs).
    #[default]
    Plain,
    /// Markdown (texte d'un contenu).
    Markdown,
    /// Fragment HTML (texte riche).
    Html,
}

impl Format {
    /// Forme probable d'un texte sans forme connue (valeur d'un fichier de données) : HTML s'il
    /// contient une balise, Markdown s'il contient une mise en forme Markdown courante.
    pub fn guess(text: &str) -> Self {
        let tag = text.match_indices('<').any(|(i, _)| {
            let rest = &text[i + 1..];
            let name = rest.strip_prefix('/').unwrap_or(rest);
            name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && name
                    .find('>')
                    .is_some_and(|j| !name[..j].contains('<') && !name[..j].contains('\n'))
        });
        if tag {
            return Self::Html;
        }
        let md_line = text.lines().any(|l| {
            let l = l.trim_start();
            l.starts_with("# ")
                || l.starts_with("## ")
                || l.starts_with("### ")
                || l.starts_with("- ")
                || l.starts_with("* ")
                || l.starts_with("> ")
                || l.starts_with("```")
        });
        if md_line || text.contains("**") || text.contains("](") || text.contains('`') {
            Self::Markdown
        } else {
            Self::Plain
        }
    }
}

/// Repères recopiés par le modèle (suites de capitales sans sens).
const MARKS: &[&str] = &[
    "XQZ", "XQY", "XQW", "XQV", "XQK", "XQJ", "ZQX", "ZQY", "ZQW", "ZQV", "ZQK", "ZQJ", "WQX",
    "WQZ", "WQY", "WQV", "VQX", "VQZ", "VQY", "VQW",
];

/// Traduction d'une phrase de texte brut (fournie par le moteur).
pub trait Sentences {
    fn sentence(&mut self, text: &str) -> String;
}

/// Position de début (octets) de chaque élément protégé : mots gardés, adresses, expressions.
fn protected(s: &str, keep: &[String]) -> Vec<(usize, usize)> {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut add = |a: usize, b: usize| {
        if !spans.iter().any(|&(x, y)| a < y && x < b) {
            spans.push((a, b));
        }
    };
    // Expressions et adresses.
    for (open, close) in [("{{", "}}"), ("{%", "%}")] {
        let mut from = 0;
        while let Some(i) = s[from..].find(open) {
            let a = from + i;
            let b = s[a..].find(close).map_or(s.len(), |j| a + j + close.len());
            add(a, b);
            from = b;
        }
    }
    let word_end = |a: usize| s[a..].find(char::is_whitespace).map_or(s.len(), |j| a + j);
    for pre in ["https://", "http://", "www.", "mailto:"] {
        let mut from = 0;
        while let Some(i) = s[from..].find(pre) {
            let a = from + i;
            let mut b = word_end(a);
            while b > a && s[..b].ends_with(['.', ',', ';', ':', ')', '!', '?']) {
                b -= 1;
            }
            add(a, b);
            from = b.max(a + pre.len());
        }
    }
    // Adresses électroniques.
    for (i, _) in s.match_indices('@') {
        let a = s[..i]
            .rfind(|c: char| c.is_whitespace() || c == '(' || c == '<')
            .map_or(0, |j| j + 1);
        let b = word_end(i);
        if a < i && s[i..b].contains('.') {
            add(a, b.min(s.len()));
        }
    }
    // Mots gardés tels quels (noms de marque) : mots entiers.
    for k in keep.iter().filter(|k| !k.trim().is_empty()) {
        let mut from = 0;
        while let Some(i) = s[from..].find(k.as_str()) {
            let a = from + i;
            let b = a + k.len();
            let before = s[..a]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric());
            let after = s[b..].chars().next().is_none_or(|c| !c.is_alphanumeric());
            if before && after {
                add(a, b);
            }
            from = b;
        }
    }
    spans.sort();
    spans
}

/// Phrases d'un texte d'une ligne : (phrase, espaces qui suivent).
fn sentences(line: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (pos, c) = chars[i];
        if matches!(c, '.' | '!' | '?' | '…') {
            // Ponctuation fermante collée (guillemets, parenthèses).
            let mut j = i + 1;
            while j < chars.len()
                && matches!(chars[j].1, '»' | '"' | ')' | '\'' | '’' | '.' | '!' | '?')
            {
                j += 1;
            }
            let mut k = j;
            while k < chars.len() && chars[k].1.is_whitespace() {
                k += 1;
            }
            let next_upper = chars.get(k).is_some_and(|&(_, n)| {
                n.is_uppercase() || n.is_numeric() || matches!(n, '«' | '"' | '(' | '[')
            });
            // Abréviations courantes : pas de fin de phrase.
            let word: String = line[start..pos]
                .rsplit(char::is_whitespace)
                .next()
                .unwrap_or("")
                .to_lowercase();
            let abbr = c == '.'
                && (word.len() <= 2 && word.chars().all(char::is_alphabetic)
                    || ["etc", "env", "cf", "p", "n°", "mr", "mme", "dr", "st"]
                        .contains(&word.as_str()));
            if k > j && next_upper && !abbr {
                let end = chars.get(j).map_or(line.len(), |x| x.0);
                let next = chars.get(k).map_or(line.len(), |x| x.0);
                out.push((&line[start..end], &line[end..next]));
                start = next;
                i = k;
                continue;
            }
        }
        i += 1;
    }
    if start < line.len() {
        out.push((&line[start..], ""));
    }
    out
}

/// Morceau d'un texte à traduire.
#[derive(Debug, Clone)]
enum Piece {
    /// Texte à traduire.
    Text(String),
    /// Élément gardé tel quel, dans sa forme finale (nom de marque, adresse, code, balise).
    Keep(String),
    /// Élément en ligne (gras, lien) : son contenu est traduit, il est remis autour de sa
    /// traduction dans la phrase.
    Wrap {
        open: String,
        close: String,
        inner: Vec<Piece>,
    },
}

/// Morceaux d'un texte brut : texte et éléments protégés.
fn text_pieces(s: &str, keep: &[String], out: &mut Vec<Piece>) {
    let mut at = 0;
    for (a, b) in protected(s, keep) {
        if a > at {
            out.push(Piece::Text(s[at..a].to_string()));
        }
        out.push(Piece::Keep(s[a..b].to_string()));
        at = b;
    }
    if at < s.len() {
        out.push(Piece::Text(s[at..].to_string()));
    }
}

/// Premier caractère d'usage privé : repère d'un élément gardé dans le texte aplati.
const PRIVATE: u32 = 0xE000;

/// Texte aplati : éléments gardés en caractères d'usage privé (leur numéro), éléments en ligne
/// réduits à leur texte et situés (octets).
struct Flat<'a> {
    text: String,
    keeps: Vec<String>,
    wraps: Vec<(usize, usize, &'a Piece)>,
}

fn flatten<'a>(pieces: &'a [Piece], f: &mut Flat<'a>, top: bool) {
    for p in pieces {
        match p {
            Piece::Text(t) => f.text.push_str(t),
            Piece::Keep(k) => {
                let c = char::from_u32(PRIVATE + f.keeps.len() as u32).unwrap_or('\u{E000}');
                f.text.push(c);
                f.keeps.push(k.clone());
            }
            Piece::Wrap { inner, .. } => {
                let a = f.text.len();
                flatten(inner, f, false);
                if top {
                    f.wraps.push((a, f.text.len(), p));
                }
            }
        }
    }
}

/// Recherche sans tenir compte de la casse ; renvoie la plage (octets).
fn find_ci(hay: &str, needle: &str, from: usize) -> Option<(usize, usize)> {
    let n: Vec<char> = needle.chars().flat_map(char::to_lowercase).collect();
    if n.is_empty() {
        return None;
    }
    let idx: Vec<(usize, char)> = hay[from..]
        .char_indices()
        .map(|(i, c)| (i + from, c))
        .collect();
    'outer: for s in 0..idx.len() {
        let mut k = 0;
        let mut j = s;
        while k < n.len() {
            let Some(&(_, c)) = idx.get(j) else {
                continue 'outer;
            };
            for lc in c.to_lowercase() {
                if n.get(k) != Some(&lc) {
                    continue 'outer;
                }
                k += 1;
            }
            j += 1;
        }
        let end = idx.get(j).map_or(hay.len(), |x| x.0);
        return Some((idx[s].0, end));
    }
    None
}

/// Traduit une phrase dont les éléments gardés sont des caractères d'usage privé : repères
/// `XQZ`… pour le modèle, puis éléments remis (un repère perdu : élément ajouté à la fin).
fn sentence_with_keeps(s: &str, keeps: &[String], html: bool, t: &mut dyn Sentences) -> String {
    let mut text = String::with_capacity(s.len());
    let mut local: Vec<&str> = Vec::new();
    let mut overflow = false;
    for c in s.chars() {
        let n = c as u32;
        if (PRIVATE..PRIVATE + 0x1000).contains(&n) {
            let k = keeps.get((n - PRIVATE) as usize).map_or("", String::as_str);
            match MARKS.get(local.len()) {
                Some(m) if !overflow => {
                    text.push_str(m);
                    local.push(k);
                }
                _ => {
                    overflow = true;
                    text.push_str(k);
                }
            }
        } else {
            text.push(c);
        }
    }
    let only_marks = !text.chars().any(char::is_alphabetic)
        || text
            .split_whitespace()
            .all(|w| MARKS.contains(&w.trim_matches(|c: char| !c.is_alphanumeric())));
    let mut out = if only_marks {
        text.clone()
    } else {
        t.sentence(&text)
    };
    if html {
        out = escape(&out);
    }
    let mut lost = Vec::new();
    for (m, k) in MARKS.iter().zip(local) {
        match out.find(m) {
            Some(i) => out.replace_range(i..i + m.len(), k),
            None => lost.push(k),
        }
    }
    for k in lost {
        out.push(' ');
        out.push_str(k);
    }
    out
}

/// Traduction de morceaux : phrase par phrase, éléments gardés remis, éléments en ligne remis
/// autour de la traduction de leur contenu (sinon gardés comme repères : la mise en forme n'est
/// jamais perdue).
fn run(pieces: &[Piece], html: bool, t: &mut dyn Sentences) -> String {
    let mut f = Flat {
        text: String::new(),
        keeps: Vec::new(),
        wraps: Vec::new(),
    };
    flatten(pieces, &mut f, true);
    let full = f.text.clone();
    let body = full.trim();
    if !body.chars().any(char::is_alphabetic) {
        // Rien à traduire : texte et éléments tels quels (contenu des éléments compris).
        return pieces
            .iter()
            .map(|p| match p {
                Piece::Text(s) => {
                    if html {
                        escape(s)
                    } else {
                        s.clone()
                    }
                }
                Piece::Keep(k) => k.clone(),
                Piece::Wrap { open, close, inner } => {
                    format!("{open}{}{close}", run(inner, html, t))
                }
            })
            .collect();
    }
    let off = full.len() - full.trim_start().len();
    let lead = &full[..off];
    let trail = &full[full.trim_end().len()..];
    let mut out = if html { escape(lead) } else { lead.to_string() };
    for (sent, sep) in sentences(body) {
        let a = off + (sent.as_ptr() as usize - body.as_ptr() as usize);
        let b = a + sent.len();
        let inside: Vec<&(usize, usize, &Piece)> = f
            .wraps
            .iter()
            .filter(|w| w.0 >= a && w.1 <= b && w.1 > w.0)
            .collect();
        let mut done = sentence_with_keeps(sent, &f.keeps, html, t);
        let mut ok = true;
        let mut cursor = 0;
        for (_, _, p) in &inside {
            let Piece::Wrap { open, close, inner } = p else {
                continue;
            };
            let mut fi = Flat {
                text: String::new(),
                keeps: Vec::new(),
                wraps: Vec::new(),
            };
            flatten(inner, &mut fi, false);
            let plain_tr = sentence_with_keeps(fi.text.trim(), &fi.keeps, html, t);
            match find_ci(&done, &plain_tr, cursor) {
                Some((x, y)) if !plain_tr.is_empty() => {
                    let formatted = run(inner, html, t);
                    let piece = format!("{open}{}{close}", formatted.trim());
                    done.replace_range(x..y, &piece);
                    cursor = x + piece.len();
                }
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            // Contenu introuvable dans la traduction : éléments en ligne gardés comme repères.
            let mut keeps = f.keeps.clone();
            let mut s2 = String::new();
            let mut at = a;
            for (wa, wb, p) in &inside {
                s2.push_str(&full[at..*wa]);
                if let Piece::Wrap { open, close, inner } = p {
                    let c = char::from_u32(PRIVATE + keeps.len() as u32).unwrap_or('\u{E000}');
                    keeps.push(format!("{open}{}{close}", run(inner, html, t).trim()));
                    s2.push(c);
                }
                at = *wb;
            }
            s2.push_str(&full[at..b]);
            done = sentence_with_keeps(&s2, &keeps, html, t);
        }
        // Éléments en ligne à cheval sur deux phrases : contenu traduit à sa place (rare).
        out.push_str(&done);
        if html {
            out.push_str(&escape(sep));
        } else {
            out.push_str(sep);
        }
    }
    // Éléments à cheval sur plusieurs phrases : non repris ci-dessus.
    for (wa, wb, p) in &f.wraps {
        let crosses = sentences(body).iter().all(|(sent, _)| {
            let a = off + (sent.as_ptr() as usize - body.as_ptr() as usize);
            !(*wa >= a && *wb <= a + sent.len())
        });
        if crosses && let Piece::Wrap { open, close, .. } = p {
            out.push_str(open);
            out.push_str(close);
        }
    }
    if html {
        out.push_str(&escape(trail));
    } else {
        out.push_str(trail);
    }
    out
}

/// Traduit une ligne de texte brut (phrase par phrase, éléments protégés gardés).
fn plain_line(line: &str, keep: &[String], t: &mut dyn Sentences) -> String {
    if !line.chars().any(char::is_alphabetic) {
        return line.to_string();
    }
    let mut pieces = Vec::new();
    text_pieces(line, keep, &mut pieces);
    run(&pieces, false, t)
}

/// Traduit un texte simple (lignes gardées).
pub fn plain(text: &str, keep: &[String], t: &mut dyn Sentences) -> String {
    text.split('\n')
        .map(|l| plain_line(l, keep, t))
        .collect::<Vec<_>>()
        .join("\n")
}

// ------------------------------------------------------------------ Markdown

/// Morceaux d'une ligne Markdown : code et balises gardés, liens, gras et italique en éléments en
/// ligne, image avec son texte alternatif traduit.
fn md_pieces(line: &str, keep: &[String], t: &mut dyn Sentences) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut text_start = 0;
    while i < line.len() {
        let rest = &line[i..];
        let elem: Option<(usize, Piece)> = if let Some(r) = rest.strip_prefix('`') {
            r.find('`')
                .map(|j| (j + 2, Piece::Keep(rest[..j + 2].to_string())))
        } else if let Some(r) = rest.strip_prefix("![") {
            r.find("](").and_then(|j| {
                let alt = &rest[2..2 + j];
                let after = 2 + j + 2;
                rest[after..].find(')').map(|k| {
                    let url = &rest[after..after + k];
                    let tr = if alt.chars().any(char::is_alphabetic) {
                        run(&md_pieces(alt, keep, t), false, t)
                    } else {
                        alt.to_string()
                    };
                    (after + k + 1, Piece::Keep(format!("![{tr}]({url})")))
                })
            })
        } else if let Some(r) = rest.strip_prefix('[') {
            r.find("](").and_then(|j| {
                let label = &rest[1..1 + j];
                let after = 1 + j + 2;
                rest[after..].find(')').map(|k| {
                    let url = &rest[after..after + k];
                    (
                        after + k + 1,
                        Piece::Wrap {
                            open: "[".into(),
                            close: format!("]({url})"),
                            inner: md_pieces(label, keep, t),
                        },
                    )
                })
            })
        } else if rest.starts_with("**") || rest.starts_with("__") {
            let mark = &rest[..2];
            rest[2..].find(mark).filter(|&j| j > 0).map(|j| {
                (
                    j + 4,
                    Piece::Wrap {
                        open: mark.to_string(),
                        close: mark.to_string(),
                        inner: md_pieces(&rest[2..2 + j], keep, t),
                    },
                )
            })
        } else if rest.starts_with('*') || rest.starts_with('_') {
            let mark = &rest[..1];
            // `_` à l'intérieur d'un mot (nom_de_fichier) : pas une mise en forme.
            let inside = mark == "_"
                && line[..i]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_alphanumeric);
            if inside || rest[1..].starts_with(' ') {
                None
            } else {
                rest[1..].find(mark).filter(|&j| j > 0).map(|j| {
                    (
                        j + 2,
                        Piece::Wrap {
                            open: mark.to_string(),
                            close: mark.to_string(),
                            inner: md_pieces(&rest[1..1 + j], keep, t),
                        },
                    )
                })
            }
        } else if rest.starts_with('<') {
            rest.find('>')
                .map(|j| (j + 1, Piece::Keep(rest[..j + 1].to_string())))
        } else {
            None
        };
        match elem {
            Some((len, piece)) => {
                text_pieces(&line[text_start..i], keep, &mut out);
                out.push(piece);
                i += len;
                text_start = i;
            }
            None => i += rest.chars().next().map_or(1, char::len_utf8),
        }
    }
    text_pieces(&line[text_start..], keep, &mut out);
    out
}

/// Traduit une ligne Markdown (sans son préfixe de bloc).
fn markdown_inline(line: &str, keep: &[String], t: &mut dyn Sentences) -> String {
    let pieces = md_pieces(line, keep, t);
    run(&pieces, false, t)
}

/// Traduit un texte Markdown : titres, listes, citations et tableaux gardés, code non traduit.
pub fn markdown(text: &str, keep: &[String], t: &mut dyn Sentences) -> String {
    let mut out = Vec::new();
    let mut fence: Option<String> = None;
    for line in text.split('\n') {
        let trimmed = line.trim_start();
        if let Some(f) = &fence {
            if trimmed.starts_with(f.as_str()) {
                fence = None;
            }
            out.push(line.to_string());
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = Some(trimmed[..3].to_string());
            out.push(line.to_string());
            continue;
        }
        // Code en retrait, ligne vide, séparateur : gardés.
        if line.starts_with("    ")
            || line.starts_with('\t')
            || trimmed.is_empty()
            || trimmed
                .chars()
                .all(|c| matches!(c, '-' | '*' | '_' | '=' | '|' | ':' | ' '))
        {
            out.push(line.to_string());
            continue;
        }
        // Bloc HTML : traduit comme du HTML.
        if trimmed.starts_with('<') && !trimmed.starts_with("<http") {
            out.push(html(line, keep, t));
            continue;
        }
        // Préfixe de bloc : retrait, titre, citation, puce, numéro.
        let indent = &line[..line.len() - trimmed.len()];
        let mut rest = trimmed;
        let mut prefix = String::from(indent);
        loop {
            let p = if let Some(r) = rest.strip_prefix("> ") {
                Some(("> ", r))
            } else if rest.starts_with('#') {
                let n = rest.chars().take_while(|c| *c == '#').count();
                rest[n..].strip_prefix(' ').map(|r| (&rest[..n + 1], r))
            } else if let Some(r) = rest
                .strip_prefix("- ")
                .or_else(|| rest.strip_prefix("* "))
                .or_else(|| rest.strip_prefix("+ "))
            {
                Some((&rest[..2], r))
            } else {
                let n = rest.chars().take_while(char::is_ascii_digit).count();
                if n > 0 && (rest[n..].starts_with(". ") || rest[n..].starts_with(") ")) {
                    Some((&rest[..n + 2], &rest[n + 2..]))
                } else {
                    None
                }
            };
            match p {
                Some((pre, r)) => {
                    prefix.push_str(pre);
                    rest = r;
                }
                None => break,
            }
        }
        // Case à cocher d'une liste.
        for cb in ["[ ] ", "[x] ", "[X] "] {
            if let Some(r) = rest.strip_prefix(cb) {
                prefix.push_str(cb);
                rest = r;
            }
        }
        // Ligne de tableau : cellules traduites une à une.
        if rest.starts_with('|') {
            let cells: Vec<String> = rest
                .split('|')
                .map(|c| {
                    if c.trim().is_empty() {
                        c.to_string()
                    } else {
                        markdown_inline(c, keep, t)
                    }
                })
                .collect();
            out.push(format!("{prefix}{}", cells.join("|")));
            continue;
        }
        out.push(format!("{prefix}{}", markdown_inline(rest, keep, t)));
    }
    out.join("\n")
}

// ------------------------------------------------------------------ HTML

/// Éléments en ligne : leur contenu est traduit à part et remis à leur place dans la phrase.
const INLINE: &[&str] = &[
    "a", "strong", "b", "em", "i", "u", "span", "mark", "small", "sup", "sub", "abbr", "cite", "q",
    "s", "del", "ins", "font",
];
/// Éléments dont le contenu n'est pas traduit.
const RAW: &[&str] = &["script", "style", "code", "pre", "svg", "template"];

/// Nom d'une balise (`<a href=…>` -> `a`, `</p>` -> `p`) et fermeture.
fn tag_name(tag: &str) -> (String, bool) {
    let inner = tag.trim_start_matches('<').trim_end_matches('>');
    let closing = inner.starts_with('/');
    let name: String = inner
        .trim_start_matches('/')
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    (name.to_ascii_lowercase(), closing)
}

/// Texte HTML (entités) -> texte ; et retour.
fn unescape(s: &str) -> String {
    s.replace("&nbsp;", "\u{a0}")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&rsquo;", "’")
        .replace("&amp;", "&")
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Attributs lus par les visiteurs dans une balise (`alt`, `title`) : traduits.
fn tag_attrs(tag: &str, keep: &[String], t: &mut dyn Sentences) -> String {
    let mut out = tag.to_string();
    for attr in ["alt=\"", "title=\"", "aria-label=\""] {
        if let Some(i) = out.find(attr) {
            let a = i + attr.len();
            if let Some(j) = out[a..].find('"') {
                let v = unescape(&out[a..a + j]);
                if v.chars().any(char::is_alphabetic) {
                    let tr = escape(&plain(&v, keep, t)).replace('"', "&quot;");
                    out.replace_range(a..a + j, &tr);
                }
            }
        }
    }
    out
}

/// Traduit un fragment HTML : texte de chaque bloc (paragraphe, titre, élément de liste) traduit
/// avec ses éléments en ligne, balises et attributs gardés.
pub fn html(text: &str, keep: &[String], t: &mut dyn Sentences) -> String {
    // Jetons : balises et textes.
    let mut tokens: Vec<(bool, &str)> = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with("<!--") {
            let end = text[i..].find("-->").map_or(text.len(), |j| i + j + 3);
            tokens.push((true, &text[i..end]));
            i = end;
        } else if text[i..].starts_with('<') {
            let end = text[i..].find('>').map_or(text.len(), |j| i + j + 1);
            tokens.push((true, &text[i..end]));
            i = end;
        } else {
            let end = text[i..].find('<').map_or(text.len(), |j| i + j);
            tokens.push((false, &text[i..end]));
            i = end;
        }
    }
    let mut out = String::new();
    let mut k = 0;
    while k < tokens.len() {
        let (is_tag, tok) = tokens[k];
        if is_tag {
            let (name, closing) = tag_name(tok);
            if !closing && RAW.contains(&name.as_str()) {
                // Contenu gardé jusqu'à la fermeture.
                out.push_str(tok);
                k += 1;
                while k < tokens.len() {
                    let (it, tk) = tokens[k];
                    out.push_str(tk);
                    k += 1;
                    if it && tag_name(tk) == (name.clone(), true) {
                        break;
                    }
                }
                continue;
            }
            if INLINE.contains(&name.as_str()) && !closing {
                // Suite en ligne : traitée avec le texte qui l'entoure.
            } else {
                out.push_str(&tag_attrs(tok, keep, t));
                k += 1;
                continue;
            }
        }
        // Suite de texte et d'éléments en ligne jusqu'à la prochaine balise de bloc.
        let start = k;
        while k < tokens.len() {
            let (it, tk) = tokens[k];
            if it {
                let (name, _) = tag_name(tk);
                if !INLINE.contains(&name.as_str()) {
                    break;
                }
            }
            k += 1;
        }
        out.push_str(&html_run(&tokens[start..k], keep, t));
    }
    out
}

/// Morceaux d'une suite de textes et d'éléments HTML en ligne.
fn html_pieces(tokens: &[(bool, &str)], keep: &[String], t: &mut dyn Sentences) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut k = 0;
    while k < tokens.len() {
        let (it, tk) = tokens[k];
        if !it {
            text_pieces(&unescape(tk), keep, &mut out);
            k += 1;
            continue;
        }
        let (name, closing) = tag_name(tk);
        if closing || tk.ends_with("/>") || !INLINE.contains(&name.as_str()) {
            out.push(Piece::Keep(tag_attrs(tk, keep, t)));
            k += 1;
            continue;
        }
        // Élément en ligne : jusqu'à sa fermeture (même nom, imbrication comprise).
        let mut depth = 0;
        let mut end = k + 1;
        while end < tokens.len() {
            let (it2, tk2) = tokens[end];
            if it2 {
                let (n2, c2) = tag_name(tk2);
                if n2 == name {
                    if c2 {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                    } else {
                        depth += 1;
                    }
                }
            }
            end += 1;
        }
        out.push(Piece::Wrap {
            open: tag_attrs(tk, keep, t),
            close: tokens.get(end).map_or(String::new(), |x| x.1.to_string()),
            inner: html_pieces(&tokens[k + 1..end.min(tokens.len())], keep, t),
        });
        k = end + 1;
    }
    out
}

/// Traduit une suite de textes et d'éléments en ligne.
fn html_run(tokens: &[(bool, &str)], keep: &[String], t: &mut dyn Sentences) -> String {
    if !tokens
        .iter()
        .any(|(it, s)| !it && unescape(s).chars().any(char::is_alphabetic))
    {
        return tokens.iter().map(|x| x.1).collect();
    }
    let pieces = html_pieces(tokens, keep, t);
    run(&pieces, true, t)
}

/// Traduit un texte selon sa forme.
pub fn translate(text: &str, format: Format, keep: &[String], t: &mut dyn Sentences) -> String {
    match format {
        Format::Plain => plain(text, keep, t),
        Format::Markdown => markdown(text, keep, t),
        Format::Html => html(text, keep, t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Faux traducteur : majuscules, repères gardés.
    struct Upper(Vec<String>);
    impl Sentences for Upper {
        fn sentence(&mut self, text: &str) -> String {
            self.0.push(text.to_string());
            text.to_uppercase()
        }
    }

    #[test]
    fn phrases() {
        let s: Vec<&str> = sentences("Bonjour. Ceci est un essai, etc. Fin ! Voilà « oui ». Ok")
            .into_iter()
            .map(|x| x.0)
            .collect();
        assert_eq!(
            s,
            vec![
                "Bonjour.",
                "Ceci est un essai, etc. Fin !",
                "Voilà « oui ».",
                "Ok"
            ]
        );
    }

    #[test]
    fn texte_simple_et_mots_gardes() {
        let mut t = Upper(Vec::new());
        let keep = vec!["Alunites".to_string()];
        let r = plain(
            "  Alunites est à Lyon. Écrivez à contact@alunites.com ou https://alunites.com/contact/.",
            &keep,
            &mut t,
        );
        assert_eq!(
            r,
            "  Alunites EST À LYON. ÉCRIVEZ À contact@alunites.com OU https://alunites.com/contact/."
        );
        assert_eq!(t.0, vec!["XQZ est à Lyon.", "Écrivez à XQZ ou XQY."]);
        assert_eq!(
            plain("{{ site.title }} — 2026", &keep, &mut t),
            "{{ site.title }} — 2026"
        );
    }

    #[test]
    fn markdown_garde_la_forme() {
        let mut t = Upper(Vec::new());
        let md = "# Titre du projet\n\nUn **beau** texte avec [un lien](/projets/a/) et `code`.\n\n- Premier point\n1. Second\n\n```\nnon traduit\n```\n> Citation";
        let r = markdown(md, &[], &mut t);
        assert_eq!(
            r,
            "# TITRE DU PROJET\n\nUN **BEAU** TEXTE AVEC [UN LIEN](/projets/a/) ET `code`.\n\n- PREMIER POINT\n1. SECOND\n\n```\nnon traduit\n```\n> CITATION"
        );
    }

    #[test]
    fn forme_devinee() {
        assert_eq!(Format::guess("Bonjour à tous"), Format::Plain);
        assert_eq!(Format::guess("a < b et c > d"), Format::Plain);
        assert_eq!(
            Format::guess("<p>Un <strong>texte</strong></p>"),
            Format::Html
        );
        assert_eq!(Format::guess("Un **texte** gras"), Format::Markdown);
        assert_eq!(Format::guess("## Titre\n\nTexte"), Format::Markdown);
        assert_eq!(Format::guess("Voir [la page](/a/)"), Format::Markdown);
    }

    #[test]
    fn html_garde_les_balises() {
        let mut t = Upper(Vec::new());
        let h = "<p>Une <strong>agence</strong> à Lyon&nbsp;!</p><p><a href=\"/c/\">Contact</a></p><img alt=\"Logo bleu\" src=\"/a.png\"><script>var a = 'x';</script>";
        let r = html(h, &[], &mut t);
        assert_eq!(
            r,
            "<p>UNE <strong>AGENCE</strong> À LYON\u{a0}!</p><p><a href=\"/c/\">CONTACT</a></p><img alt=\"LOGO BLEU\" src=\"/a.png\"><script>var a = 'x';</script>"
        );
    }
}
