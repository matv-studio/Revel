//! `revel-markup` : conversion de gabarits HTML (avec balises Jinja / Nunjucks) en arbre de nœuds Revel.
//!
//! Reconnu :
//! - HTML : éléments, attributs, commentaires, doctype, éléments vides, `<script>`/`<style>` en texte brut ;
//! - `{{ expr }}` (dans le texte et dans les valeurs d'attributs) ;
//! - `{% if %}/{% elif %}/{% else %}/{% endif %}`, `{% for x in y %}/{% else %}/{% endfor %}`,
//!   `{% set x = e %}`, `{% set x %}…{% endset %}`, `{% include "…" %}`, `{% raw %}` ;
//! - `{% if c %} attr="…"{% endif %}` à l'intérieur d'une balise : attribut conditionnel ;
//! - contrôle des espaces `{%-`, `-%}`, `{{-`, `-}}` ; commentaires `{# … #}` ;
//! - extensions : `{% image … %}` et `{% imageOptional … %}` (shortcodes d'images).
//!
//! Le dialecte [`Dialect::Nunjucks`] réécrit les quelques tournures JavaScript de Nunjucks
//! (`x.length`, `s.startsWith(y)`, `(a.push(x), a)`, clés d'objet sans guillemets) en syntaxe Jinja.

use revel_model::{Attr, ImageNode, Node};

pub mod expr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dialect {
    #[default]
    Jinja,
    Nunjucks,
}

/// Nature du document : règles HTML (éléments vides, script/style), XML ou texte brut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Html,
    Xml,
    /// Aucun balisage : seules les balises de gabarit sont reconnues (`.htaccess`, `robots.txt`).
    Text,
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub dialect: Dialect,
    pub mode: Mode,
}

/// Résultat d'une conversion : nœuds et avertissements (constructions non reconnues).
#[derive(Debug, Default)]
pub struct Parsed {
    pub nodes: Vec<Node>,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum MarkupError {
    #[error("ligne {line} : {msg}")]
    Syntax { line: usize, msg: String },
}

/// Convertit un gabarit en nœuds.
pub fn parse(src: &str, opts: &Options) -> Result<Parsed, MarkupError> {
    let pre = preprocess(src);
    let mut p = Parser {
        s: &pre,
        pos: 0,
        opts: opts.clone(),
        warnings: Vec::new(),
    };
    let (nodes, end) = p.nodes(&[])?;
    if let End::Close(tag) = end {
        p.warnings
            .push(format!("balise fermante </{tag}> sans ouverture"));
    }
    Ok(Parsed {
        nodes: merge_text(nodes),
        warnings: p.warnings,
    })
}

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];
const RAW_TEXT: &[&str] = &["script", "style", "textarea", "title"];

/// Applique le contrôle des espaces et retire les commentaires de gabarit.
fn preprocess(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let b = src.as_bytes();
    let mut trim_next = false;
    while i < b.len() {
        if b[i] == b'{' && i + 1 < b.len() && matches!(b[i + 1], b'{' | b'%' | b'#') {
            let kind = b[i + 1];
            let close = match kind {
                b'{' => "}}",
                b'%' => "%}",
                _ => "#}",
            };
            // Les commentaires peuvent contenir des apostrophes : recherche simple.
            let found = if kind == b'#' {
                src[i + 2..].find(close)
            } else {
                find_close(&src[i + 2..], close)
            };
            let Some(rel) = found else {
                out.push_str(&src[i..]);
                break;
            };
            let end = i + 2 + rel + 2;
            let inner = &src[i + 2..i + 2 + rel];
            let trim_left = inner.starts_with('-');
            let trim_right = inner.ends_with('-');
            if trim_left {
                let t = out.trim_end().len();
                out.truncate(t);
            }
            if kind != b'#' {
                let body = inner.trim_start_matches('-').trim_end_matches('-').trim();
                out.push_str(match kind {
                    b'{' => "{{ ",
                    _ => "{% ",
                });
                out.push_str(body);
                out.push_str(if kind == b'{' { " }}" } else { " %}" });
            }
            trim_next = trim_right;
            i = end;
            continue;
        }
        let c = src[i..].chars().next().unwrap_or(' ');
        if trim_next && c.is_whitespace() {
            i += c.len_utf8();
            continue;
        }
        trim_next = false;
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Position de `close` en ignorant les chaînes entre guillemets et, pour `}}`,
/// les accolades d'un dictionnaire littéral (`{{ {"a": {"b": 1}} | json }}`).
fn find_close(s: &str, close: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut depth = 0usize;
    let mut it = s.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        match quote {
            Some(q) => {
                if c == '\\' {
                    it.next();
                } else if c == q {
                    quote = None;
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    quote = Some(c);
                } else if depth == 0 && s[i..].starts_with(close) {
                    return Some(i);
                } else if close == "}}" && c == '{' {
                    depth += 1;
                } else if close == "}}" && c == '}' {
                    depth = depth.saturating_sub(1);
                }
            }
        }
    }
    None
}

enum End {
    Eof,
    Close(String),
    Stmt(String),
}

struct Parser<'a> {
    s: &'a str,
    pos: usize,
    opts: Options,
    warnings: Vec<String>,
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        let s: &'a str = self.s;
        &s[self.pos..]
    }

    fn line(&self) -> usize {
        self.s[..self.pos].matches('\n').count() + 1
    }

    fn err(&self, msg: impl Into<String>) -> MarkupError {
        MarkupError::Syntax {
            line: self.line(),
            msg: msg.into(),
        }
    }

    fn ex(&self, e: &str) -> String {
        match self.opts.dialect {
            Dialect::Jinja => e.trim().to_string(),
            Dialect::Nunjucks => expr::from_nunjucks(e.trim()),
        }
    }

    /// Lit `{{ … }}` ou `{% … %}` à la position courante ; renvoie le contenu.
    fn tag_body(&mut self, close: &str) -> Result<String, MarkupError> {
        let rel =
            find_close(&self.rest()[2..], close).ok_or_else(|| self.err("balise non fermée"))?;
        let body = self.rest()[2..2 + rel].trim().to_string();
        self.pos += 2 + rel + 2;
        Ok(body)
    }

    /// Analyse des nœuds jusqu'à une instruction de fin (`until`), une balise fermante ou la fin.
    fn nodes(&mut self, until: &[&str]) -> Result<(Vec<Node>, End), MarkupError> {
        let mut out = Vec::new();
        loop {
            let r = self.rest();
            if r.is_empty() {
                return Ok((out, End::Eof));
            }
            if r.starts_with("{{") {
                let e = self.tag_body("}}")?;
                out.push(Node::Out { expr: self.ex(&e) });
            } else if r.starts_with("{%") {
                let stmt = self.tag_body("%}")?;
                let kw = stmt.split_whitespace().next().unwrap_or("").to_string();
                if until.contains(&kw.as_str()) {
                    return Ok((out, End::Stmt(stmt)));
                }
                if let Some(n) = self.statement(&kw, &stmt)? {
                    out.push(n);
                }
            } else if self.opts.mode == Mode::Text {
                let end = next_template_tag(r);
                out.push(Node::Html {
                    html: r[..end].to_string(),
                });
                self.pos += end;
            } else if r.starts_with("<!--") {
                let end = r.find("-->").map(|i| i + 3).unwrap_or(r.len());
                out.push(Node::Html {
                    html: r[..end].to_string(),
                });
                self.pos += end;
            } else if r.starts_with("<!") || r.starts_with("<?") {
                let end = r.find('>').map(|i| i + 1).unwrap_or(r.len());
                out.push(Node::Html {
                    html: r[..end].to_string(),
                });
                self.pos += end;
            } else if r.starts_with("</") {
                let end = r
                    .find('>')
                    .ok_or_else(|| self.err("balise fermante incomplète"))?;
                let tag = r[2..end].trim().to_ascii_lowercase();
                self.pos += end + 1;
                return Ok((out, End::Close(tag)));
            } else if r.starts_with('<') && r[1..].starts_with(|c: char| c.is_ascii_alphabetic()) {
                let n = self.element()?;
                out.push(n);
            } else {
                // Texte jusqu'au prochain `<` ou `{{` / `{%`.
                let mut end = r.len();
                for (i, c) in r.char_indices().skip(1) {
                    if c == '<'
                        || (c == '{' && (r[i..].starts_with("{{") || r[i..].starts_with("{%")))
                    {
                        end = i;
                        break;
                    }
                }
                if r.starts_with('<') && end == r.len() {
                    end = 1.max(end);
                }
                let raw = &r[..end];
                out.push(Node::Text {
                    text: htmlize::unescape(raw).into_owned(),
                });
                self.pos += end;
            }
        }
    }

    fn statement(&mut self, kw: &str, stmt: &str) -> Result<Option<Node>, MarkupError> {
        let args = stmt[kw.len()..].trim();
        Ok(Some(match kw {
            "if" => self.if_chain(args)?,
            "for" => {
                let (item, expr) = args
                    .split_once(" in ")
                    .ok_or_else(|| self.err(format!("for invalide : {stmt}")))?;
                let (children, end) = self.nodes(&["else", "endfor"])?;
                let mut empty = Vec::new();
                if let End::Stmt(s) = &end
                    && s.starts_with("else")
                {
                    let (e, _) = self.nodes(&["endfor"])?;
                    empty = e;
                }
                Node::Each {
                    id: None,
                    expr: self.ex(expr),
                    item: item.trim().to_string(),
                    children: merge_text(children),
                    empty: merge_text(empty),
                }
            }
            "set" => match args.split_once('=') {
                Some((name, e)) if !name.contains('(') => Node::Set {
                    name: name.trim().to_string(),
                    expr: self.ex(e),
                },
                _ => {
                    let (children, _) = self.nodes(&["endset"])?;
                    Node::SetBlock {
                        id: None,
                        name: args.to_string(),
                        children: merge_text(children),
                    }
                }
            },
            "include" => {
                let name = args.trim_matches(|c| c == '"' || c == '\'');
                Node::Use {
                    id: None,
                    component: component_id(name),
                    args: Default::default(),
                    variant: None,
                    attrs: Vec::new(),
                }
            }
            "raw" => {
                let r = self.rest();
                let end = r.find("{% endraw %}").unwrap_or(r.len());
                let html = r[..end].to_string();
                self.pos += (end + "{% endraw %}".len()).min(r.len());
                Node::Html { html }
            }
            "image" | "imageOptional" => {
                let parts = expr::split_args(args);
                let get = |i: usize| parts.get(i).map(|e| self.ex(e));
                Node::Image(ImageNode {
                    id: None,
                    src: get(0).unwrap_or_default(),
                    alt: get(1),
                    sizes: get(2),
                    class: get(3),
                    ratio: get(4),
                    loading: get(5),
                    widths: get(6),
                    optional: kw == "imageOptional",
                    name: None,
                    hidden: false,
                })
            }
            _ => {
                self.warnings.push(format!(
                    "ligne {} : instruction non reconnue : {{% {stmt} %}}",
                    self.line()
                ));
                Node::Html {
                    html: format!("{{% {stmt} %}}"),
                }
            }
        }))
    }

    fn if_chain(&mut self, cond: &str) -> Result<Node, MarkupError> {
        let (then, end) = self.nodes(&["elif", "else", "endif"])?;
        let otherwise = match end {
            End::Stmt(s) if s.starts_with("elif") => vec![self.if_chain(s[4..].trim())?],
            End::Stmt(s) if s.starts_with("else") => self.nodes(&["endif"])?.0,
            _ => Vec::new(),
        };
        Ok(Node::If {
            id: None,
            cond: self.ex(cond),
            then: merge_text(then),
            otherwise: merge_text(otherwise),
        })
    }

    fn element(&mut self) -> Result<Node, MarkupError> {
        self.pos += 1; // '<'
        let name_len = self
            .rest()
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/' || c == '{')
            .unwrap_or(self.rest().len());
        let tag = self.rest()[..name_len].to_string();
        let lower = tag.to_ascii_lowercase();
        self.pos += name_len;
        let mut attrs = Vec::new();
        let self_closing = self.attributes(&mut attrs, None)?;
        let mut children = Vec::new();
        let html = self.opts.mode == Mode::Html;
        if !self_closing && !(html && VOID.contains(&lower.as_str())) {
            if html && RAW_TEXT.contains(&lower.as_str()) {
                children = self.raw_text(&lower)?;
            } else {
                let (kids, end) = self.nodes(&[])?;
                children = kids;
                match end {
                    End::Close(t) if t == lower => {}
                    End::Close(t) => {
                        // Fermeture d'un ancêtre : fermeture implicite de cet élément.
                        self.pos -= t.len() + 3;
                        self.warnings.push(format!(
                            "ligne {} : <{lower}> fermé implicitement par </{t}>",
                            self.line()
                        ));
                    }
                    End::Stmt(s) => {
                        return Err(self.err(format!(
                            "instruction {{% {s} %}} à l'intérieur de <{lower}> sans ouverture correspondante (structure croisée)"
                        )));
                    }
                    End::Eof => {}
                }
            }
        }
        Ok(Node::Element {
            id: None,
            tag,
            attrs,
            children: merge_text(children),
        })
    }

    /// Lit les attributs jusqu'à `>` (ou `{% endif %}` si `when` est donné). Renvoie `true` si `/>`.
    fn attributes(
        &mut self,
        attrs: &mut Vec<Attr>,
        when: Option<&str>,
    ) -> Result<bool, MarkupError> {
        loop {
            let r = self.rest();
            let trimmed = r.trim_start();
            self.pos += r.len() - trimmed.len();
            let r = self.rest();
            if r.is_empty() {
                return Err(self.err("balise non fermée"));
            }
            if when.is_none() && r.starts_with("/>") {
                self.pos += 2;
                return Ok(true);
            }
            if when.is_none() && r.starts_with('>') {
                self.pos += 1;
                return Ok(false);
            }
            if r.starts_with("{%") {
                let stmt = self.tag_body("%}")?;
                if let Some(cond) = stmt.strip_prefix("if ") {
                    let cond = self.ex(cond);
                    let combined = match when {
                        Some(w) => format!("({w}) and ({cond})"),
                        None => cond,
                    };
                    self.attributes(attrs, Some(&combined))?;
                    continue;
                }
                if stmt == "endif" && when.is_some() {
                    return Ok(false);
                }
                return Err(self.err(format!(
                    "instruction {{% {stmt} %}} non prise en charge dans une balise"
                )));
            }
            if r.starts_with("{{") {
                return Err(self.err("expression {{ }} à la place d'un nom d'attribut"));
            }
            let name_len = r
                .find(|c: char| c.is_whitespace() || c == '=' || c == '>' || c == '{' || c == '/')
                .unwrap_or(r.len())
                .max(1);
            let name = r[..name_len].to_string();
            self.pos += name_len;
            let r = self.rest();
            let after = r.trim_start();
            let mut value = None;
            if let Some(v) = after.strip_prefix('=') {
                self.pos += r.len() - after.len() + 1;
                let v_trim = v.trim_start();
                self.pos += v.len() - v_trim.len();
                let r = self.rest();
                let q = r.chars().next().unwrap_or(' ');
                if q == '"' || q == '\'' {
                    // Valeur entre guillemets ; les expressions {{ }} peuvent contenir l'autre guillemet.
                    let mut end = None;
                    let mut i = 1;
                    while i < r.len() {
                        if r[i..].starts_with("{{") || r[i..].starts_with("{%") {
                            let close = if r[i..].starts_with("{{") { "}}" } else { "%}" };
                            let rel = find_close(&r[i + 2..], close).ok_or_else(|| {
                                self.err("expression non fermée dans un attribut")
                            })?;
                            i += 2 + rel + 2;
                            continue;
                        }
                        let c = r[i..].chars().next().unwrap_or(' ');
                        if c == q {
                            end = Some(i);
                            break;
                        }
                        i += c.len_utf8();
                    }
                    let end = end.ok_or_else(|| self.err("valeur d'attribut non fermée"))?;
                    let raw = &r[1..end];
                    value = Some(self.attr_value(raw));
                    self.pos += end + 1;
                } else {
                    let end = r
                        .find(|c: char| c.is_whitespace() || c == '>')
                        .unwrap_or(r.len());
                    value = Some(self.attr_value(&r[..end]));
                    self.pos += end;
                }
            }
            attrs.push(Attr {
                name,
                value,
                when: when.map(str::to_string),
            });
        }
    }

    /// Valeur d'attribut : entités décodées hors expressions, expressions réécrites selon le dialecte.
    fn attr_value(&self, raw: &str) -> String {
        let mut out = String::new();
        let mut rest = raw;
        while let Some(start) = rest.find("{{").into_iter().chain(rest.find("{%")).min() {
            out.push_str(&htmlize::unescape(&rest[..start]));
            let close = if rest[start..].starts_with("{{") {
                "}}"
            } else {
                "%}"
            };
            let Some(rel) = find_close(&rest[start + 2..], close) else {
                out.push_str(&rest[start..]);
                return out;
            };
            let inner = rest[start + 2..start + 2 + rel].trim();
            if close == "}}" {
                out.push_str(&format!("{{{{ {} }}}}", self.ex(inner)));
            } else {
                // Instruction dans une valeur : conservée telle quelle (rendu par le moteur de gabarits).
                let kw = inner.split_whitespace().next().unwrap_or("");
                let args = inner[kw.len()..].trim();
                let args = if matches!(kw, "if" | "elif") {
                    self.ex(args)
                } else {
                    args.to_string()
                };
                out.push_str(&format!("{{% {kw} {args} %}}").replace("  %}", " %}"));
            }
            rest = &rest[start + 2 + rel + 2..];
        }
        out.push_str(&htmlize::unescape(rest));
        out
    }

    /// Contenu brut de <script>/<style>/<textarea>/<title> : texte et expressions seulement.
    fn raw_text(&mut self, tag: &str) -> Result<Vec<Node>, MarkupError> {
        let close = format!("</{tag}");
        let r = self.rest();
        let end = r
            .to_ascii_lowercase()
            .find(&close)
            .ok_or_else(|| self.err(format!("<{tag}> non fermé")))?;
        let inner = r[..end].to_string();
        let gt = r[end..].find('>').map(|i| end + i + 1).unwrap_or(r.len());
        self.pos += gt;
        let mut nodes = Vec::new();
        let mut rest = inner.as_str();
        while let Some(start) = rest.find("{{") {
            if start > 0 {
                nodes.push(Node::Html {
                    html: rest[..start].to_string(),
                });
            }
            let rel = find_close(&rest[start + 2..], "}}")
                .ok_or_else(|| self.err("expression non fermée"))?;
            nodes.push(Node::Out {
                expr: self.ex(&rest[start + 2..start + 2 + rel]),
            });
            rest = &rest[start + 2 + rel + 2..];
        }
        if !rest.is_empty() {
            nodes.push(Node::Html {
                html: rest.to_string(),
            });
        }
        if tag == "title" || tag == "textarea" {
            // Texte ordinaire : décodé, échappé à la génération.
            nodes = nodes
                .into_iter()
                .map(|n| match n {
                    Node::Html { html } => Node::Text {
                        text: htmlize::unescape(&html).into_owned(),
                    },
                    n => n,
                })
                .collect();
        }
        Ok(nodes)
    }
}

/// Position de la prochaine balise de gabarit (`{{` ou `{%`), au moins 1.
fn next_template_tag(r: &str) -> usize {
    let a = r[1..].find("{{").map(|i| i + 1);
    let b = r[1..].find("{%").map(|i| i + 1);
    match (a, b) {
        (Some(a), Some(b)) => a.min(b),
        (Some(x), None) | (None, Some(x)) => x,
        (None, None) => r.len(),
    }
}

/// `partials/header.njk` -> `partials/header`.
pub fn component_id(path: &str) -> String {
    path.trim_end_matches(".njk")
        .trim_end_matches(".html")
        .trim_end_matches(".jinja")
        .to_string()
}

/// Fusionne les nœuds texte adjacents et retire les textes vides.
fn merge_text(nodes: Vec<Node>) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
    for n in nodes {
        match (out.last_mut(), n) {
            (Some(Node::Text { text: a }), Node::Text { text: b }) => a.push_str(&b),
            (_, Node::Text { text }) if text.is_empty() => {}
            (_, n) => out.push(n),
        }
    }
    out
}

#[cfg(test)]
mod tests;
