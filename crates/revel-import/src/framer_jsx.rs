//! Lecture du rendu d'un composant Framer dans son module compilé (D-111).
//!
//! La page publiée par Framer ne contient que la première variante de chaque composant ; une
//! variante choisie au clic (carte « Outils » dépliée…) n'existe que dans le code. Ce module lit
//! l'arbre des appels `jsx(balise, {props})` du rendu et le traduit en nœuds Revel pour une
//! variante donnée : conditions de variante (`I()&&…`), classes, noms de calques, styles
//! écrits, textes, images de fond, surcharges par variante (`...Bt({id:{…}},S,D)`).
//!
//! Analyse volontairement partielle : ce qui n'est pas reconnu est ignoré.

use crate::framer::{css_decl, literal_end};
use revel_model::{Attr, Node};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Expression JavaScript, lue dans la mesure utile.
#[derive(Debug, Clone, PartialEq)]
pub enum Js {
    /// Appel : appelé (texte brut, `a`, `f.div`, `R`) et arguments.
    Call(String, Vec<Js>),
    /// Objet : entrées dans l'ordre ; `...x` : clé `...`.
    Obj(Vec<(String, Js)>),
    Arr(Vec<Js>),
    /// Chaîne sans substitution.
    Str(String),
    /// `a && b`.
    And(Box<Js>, Box<Js>),
    /// `a ?? b`, `a || b`.
    Or(Box<Js>, Box<Js>),
    /// Tout le reste (texte brut).
    Raw(String),
}

struct P<'a> {
    s: &'a str,
    b: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.b.get(self.i).copied()
    }
    fn eat(&mut self, t: &str) -> bool {
        self.ws();
        if self.s[self.i..].starts_with(t) {
            self.i += t.len();
            true
        } else {
            false
        }
    }
    fn ident(&mut self) -> Option<String> {
        self.ws();
        let st = self.i;
        while self.i < self.b.len()
            && (self.b[self.i].is_ascii_alphanumeric() || matches!(self.b[self.i], b'_' | b'$'))
        {
            self.i += 1;
        }
        (self.i > st).then(|| self.s[st..self.i].to_string())
    }

    fn expr(&mut self) -> Js {
        let c = self.or();
        // Ternaire : gardé brut.
        if self.peek() == Some(b'?')
            && !self.s[self.i..].starts_with("??")
            && !self.s[self.i..].starts_with("?.")
        {
            self.i += 1;
            let _ = self.expr();
            self.eat(":");
            let _ = self.expr();
            return Js::Raw(format!("{c:?}?"));
        }
        c
    }
    fn or(&mut self) -> Js {
        let mut l = self.and();
        loop {
            if self.eat("??") || self.eat("||") {
                let r = self.and();
                l = Js::Or(Box::new(l), Box::new(r));
            } else {
                return l;
            }
        }
    }
    fn and(&mut self) -> Js {
        let mut l = self.binary();
        while self.eat("&&") {
            let r = self.binary();
            l = Js::And(Box::new(l), Box::new(r));
        }
        l
    }
    fn binary(&mut self) -> Js {
        let st = self.i;
        let l = self.unary();
        let mut raw = false;
        loop {
            self.ws();
            let rest = &self.s[self.i..];
            let op = [
                "===",
                "!==",
                "==",
                "!=",
                "<=",
                ">=",
                "<",
                ">",
                "+",
                "-",
                "*",
                "/",
                "%",
                " instanceof ",
                " in ",
            ]
            .into_iter()
            .find(|o| rest.starts_with(o.trim_start()) && !rest.starts_with("=>"));
            match op {
                Some(o) => {
                    self.i += o.trim_start().len();
                    let _ = self.unary();
                    raw = true;
                }
                None => break,
            }
        }
        if raw {
            Js::Raw(self.s[st..self.i].trim().to_string())
        } else {
            l
        }
    }
    fn unary(&mut self) -> Js {
        self.ws();
        let st = self.i;
        for kw in ["typeof ", "void ", "await ", "new "] {
            if self.s[self.i..].starts_with(kw) {
                self.i += kw.len();
                let _ = self.unary();
                return Js::Raw(self.s[st..self.i].to_string());
            }
        }
        if matches!(self.peek(), Some(b'!') | Some(b'-') | Some(b'+')) {
            self.i += 1;
            let _ = self.unary();
            return Js::Raw(self.s[st..self.i].to_string());
        }
        self.postfix()
    }
    fn postfix(&mut self) -> Js {
        let mut e = self.primary();
        loop {
            self.ws();
            if self.b.get(self.i) == Some(&b'(') {
                let callee = match &e {
                    Js::Raw(r) => r.clone(),
                    other => format!("{other:?}"),
                };
                self.i += 1;
                let mut args = Vec::new();
                while self.peek().is_some_and(|c| c != b')') {
                    if self.eat("...") {
                        let _ = self.expr();
                    } else {
                        args.push(self.expr());
                    }
                    if !self.eat(",") {
                        break;
                    }
                }
                self.eat(")");
                e = Js::Call(callee, args);
            } else if self.s[self.i..].starts_with("?.")
                || (self.b.get(self.i) == Some(&b'.') && !self.s[self.i..].starts_with("..."))
            {
                let st = self.i;
                self.i += if self.b[self.i] == b'?' { 2 } else { 1 };
                if self.b.get(self.i) == Some(&b'(') || self.b.get(self.i) == Some(&b'[') {
                    continue;
                }
                let _ = self.ident();
                let tail = self.s[st..self.i].to_string();
                e = match e {
                    Js::Raw(r) => Js::Raw(r + &tail),
                    other => Js::Raw(format!("{other:?}{tail}")),
                };
            } else if self.b.get(self.i) == Some(&b'[') {
                let end = literal_end(self.s, self.i).unwrap_or(self.b.len());
                self.i = end;
                e = Js::Raw(format!("{e:?}[]"));
            } else if self.s[self.i..].starts_with("=>") {
                self.i += 2;
                self.ws();
                if self.b.get(self.i) == Some(&b'{') {
                    self.i = literal_end(self.s, self.i).unwrap_or(self.b.len());
                } else {
                    let _ = self.expr();
                }
                e = Js::Raw("=>".into());
            } else {
                return e;
            }
        }
    }
    fn primary(&mut self) -> Js {
        match self.peek() {
            Some(b'`') | Some(b'"') | Some(b'\'') => {
                let st = self.i;
                let end = literal_end(self.s, st).unwrap_or(self.b.len());
                self.i = end;
                let inner = &self.s[st + 1..end.saturating_sub(1)];
                if self.b[st] == b'`' && inner.contains("${") {
                    Js::Raw(self.s[st..end].to_string())
                } else {
                    Js::Str(unescape(inner))
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut out = Vec::new();
                loop {
                    match self.peek() {
                        None => break,
                        Some(b'}') => {
                            self.i += 1;
                            break;
                        }
                        _ => {}
                    }
                    if self.eat("...") {
                        out.push(("...".to_string(), self.expr()));
                    } else {
                        let key = match self.peek() {
                            Some(b'"') | Some(b'\'') | Some(b'`') => match self.primary() {
                                Js::Str(k) => k,
                                _ => String::new(),
                            },
                            Some(b'[') => {
                                self.i = literal_end(self.s, self.i).unwrap_or(self.b.len());
                                String::new()
                            }
                            _ => self.ident().unwrap_or_default(),
                        };
                        if self.eat(":") {
                            out.push((key, self.expr()));
                        } else if self.peek() == Some(b'(') {
                            // Méthode `a(){…}` : ignorée.
                            self.i = literal_end(self.s, self.i).unwrap_or(self.b.len());
                            self.ws();
                            if self.b.get(self.i) == Some(&b'{') {
                                self.i = literal_end(self.s, self.i).unwrap_or(self.b.len());
                            }
                        } else {
                            out.push((key.clone(), Js::Raw(key)));
                        }
                    }
                    if !self.eat(",") {
                        self.eat("}");
                        break;
                    }
                }
                Js::Obj(out)
            }
            Some(b'[') => {
                self.i += 1;
                let mut out = Vec::new();
                while self.peek().is_some_and(|c| c != b']') {
                    if self.eat("...") {
                        let _ = self.expr();
                    } else {
                        out.push(self.expr());
                    }
                    if !self.eat(",") {
                        break;
                    }
                }
                self.eat("]");
                Js::Arr(out)
            }
            Some(b'(') => {
                // Groupe ou paramètres d'une fonction fléchée.
                let end = literal_end(self.s, self.i).unwrap_or(self.b.len());
                let inner = &self.s[self.i + 1..end.saturating_sub(1)];
                self.i = end;
                self.ws();
                if self.s[self.i..].starts_with("=>") {
                    return Js::Raw(String::new());
                }
                parse(inner)
            }
            Some(_) => {
                let st = self.i;
                match self.ident() {
                    Some(_) => Js::Raw(self.s[st..self.i].to_string()),
                    None => {
                        // Nombre (`.5`) ou caractère inattendu.
                        self.i += 1;
                        while self.i < self.b.len()
                            && (self.b[self.i].is_ascii_alphanumeric() || self.b[self.i] == b'.')
                        {
                            self.i += 1;
                        }
                        Js::Raw(self.s[st..self.i].to_string())
                    }
                }
            }
            None => Js::Raw(String::new()),
        }
    }
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Lit une expression.
pub fn parse(s: &str) -> Js {
    let mut p = P {
        s,
        b: s.as_bytes(),
        i: 0,
    };
    p.expr()
}

/// Lit l'expression qui commence à `at` dans `s` ; renvoie aussi sa fin.
pub fn parse_at(s: &str, at: usize) -> (Js, usize) {
    let mut p = P {
        s,
        b: s.as_bytes(),
        i: at,
    };
    let e = p.expr();
    (e, p.i)
}

// -------------------------------------------------------------------------------------------------
// Rendu d'une variante

/// Contexte d'un composant : variante rendue, fonctions de condition, balises nommées.
pub struct Render<'a> {
    /// Variante rendue (identifiant de Framer).
    pub variant: &'a str,
    /// Fonction de condition → variantes pour lesquelles elle est vraie (`I=()=>S===`id``).
    pub conds: HashMap<String, Vec<String>>,
    /// Variantes connues du composant.
    pub variants: Vec<String>,
    /// Composants enveloppes → balise HTML (`Vt=v(E(f.div))`).
    pub tags: HashMap<String, String>,
    /// Textes des propriétés de l'utilisation (variable du composant → texte lu dans la page).
    pub texts: HashMap<String, String>,
}

/// Élément de la variante rendu : nœud et classe des éléments qui changent de variante au clic.
#[derive(Debug, Default)]
pub struct Out {
    pub nodes: Vec<Node>,
    /// Classes (principales) des éléments qui réagissent au clic (`onTap`).
    pub taps: Vec<String>,
    /// Textes tirés d'une propriété (`text:b`) : (variable, classe du bloc de texte).
    pub text_vars: Vec<(String, String)>,
}

impl Render<'_> {
    /// Lit les conditions et balises nommées d'un module : `X=()=>S===`id``, `X=()=>S!==`id``,
    /// `X=()=>[`a`,`b`].includes(S)`, `X=()=>!![`a`].includes(S)` ; `Vt=v(E(f.div))`.
    pub fn read_defs(&mut self, js: &str, state: &str) {
        let s = regex::escape(state);
        let eq = regex::Regex::new(&format!(r"([\w$]+)=\(\)=>{s}(===|!==)`([A-Za-z0-9_]+)`"))
            .expect("regex");
        for c in eq.captures_iter(js) {
            let vs: Vec<String> = if &c[2] == "===" {
                vec![c[3].to_string()]
            } else {
                self.variants
                    .iter()
                    .filter(|v| **v != c[3])
                    .cloned()
                    .collect()
            };
            self.conds.insert(c[1].to_string(), vs);
        }
        let inc = regex::Regex::new(&format!(
            r"([\w$]+)=\(\)=>(!?)!?\[([^\]]*)\]\.includes\({s}\)"
        ))
        .expect("regex");
        let id = regex::Regex::new(r"`([A-Za-z0-9_]+)`").expect("regex");
        for c in inc.captures_iter(js) {
            let list: Vec<String> = id.captures_iter(&c[3]).map(|x| x[1].to_string()).collect();
            let vs = if &c[2] == "!" && !c.get(0).expect("m").as_str().contains("!![") {
                self.variants
                    .iter()
                    .filter(|v| !list.contains(v))
                    .cloned()
                    .collect()
            } else {
                list
            };
            self.conds.insert(c[1].to_string(), vs);
        }
        let tag = regex::Regex::new(r"([\w$]+)=[\w$]+\((?:[\w$]+\()?[\w$]+\.([a-z][a-z0-9]*)\)\)?")
            .expect("regex");
        for c in tag.captures_iter(js) {
            self.tags
                .entry(c[1].to_string())
                .or_insert_with(|| c[2].to_string());
        }
    }

    fn cond(&self, c: &Js) -> bool {
        match c {
            Js::Call(name, args) if args.is_empty() => self
                .conds
                .get(name)
                .is_none_or(|vs| vs.iter().any(|v| v == self.variant)),
            Js::And(a, b) => self.cond(a) && self.cond(b),
            _ => true,
        }
    }

    /// Nœuds d'une expression de rendu.
    pub fn render(&self, e: &Js, out: &mut Out, in_text: bool) -> Vec<Node> {
        match e {
            Js::Arr(items) => items
                .iter()
                .flat_map(|x| self.render(x, out, in_text))
                .collect(),
            Js::And(c, x) => {
                if self.cond(c) {
                    self.render(x, out, in_text)
                } else {
                    Vec::new()
                }
            }
            // Texte traduit (`W(`v0`,l)??…`) : texte de la langue principale.
            Js::Or(a, b) => {
                let first = self.render(a, out, in_text);
                if first.is_empty() {
                    self.render(b, out, in_text)
                } else {
                    first
                }
            }
            Js::Str(t) => vec![Node::Text { text: t.clone() }],
            Js::Call(_, args) if args.len() >= 2 => match (&args[0], &args[1]) {
                (Js::Raw(tag), Js::Obj(props)) => self.element(tag, props, out, in_text),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// Props d'un élément avec les surcharges de la variante rendue.
    fn props(&self, props: &[(String, Js)]) -> Vec<(String, Js)> {
        let mut out: Vec<(String, Js)> = Vec::new();
        for (k, v) in props {
            if k == "..." {
                if let Js::Call(_, args) = v
                    && let Some(Js::Obj(by)) = args.first()
                {
                    for (vid, o) in by {
                        if vid == self.variant
                            && let Js::Obj(o) = o
                        {
                            for (k2, v2) in o {
                                out.retain(|(x, _)| x != k2);
                                out.push((k2.clone(), v2.clone()));
                            }
                        }
                    }
                }
                continue;
            }
            out.retain(|(x, _)| x != k);
            out.push((k.clone(), v.clone()));
        }
        out
    }

    fn element(
        &self,
        tag: &str,
        props: &[(String, Js)],
        out: &mut Out,
        in_text: bool,
    ) -> Vec<Node> {
        let props = self.props(props);
        let get = |k: &str| props.iter().find(|(x, _)| x == k).map(|(_, v)| v);
        let html = tag
            .rsplit_once('.')
            .map(|(_, t)| t.to_string())
            .filter(|t| {
                t.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            })
            .or_else(|| self.tags.get(tag).cloned());
        let class = get("className").map(class_of).unwrap_or_default();
        let mut attrs: Vec<Attr> = Vec::new();
        let push = |attrs: &mut Vec<Attr>, n: &str, v: String| {
            attrs.push(Attr {
                name: n.into(),
                value: Some(v),
                when: None,
            })
        };
        let mut classes = class.clone();
        if in_text
            && html.as_deref().is_some_and(|t| {
                matches!(
                    t,
                    "p" | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "span"
                        | "a"
                        | "li"
                        | "ul"
                        | "ol"
                )
            })
        {
            classes.insert(0, "framer-text".into());
        }
        if !classes.is_empty() {
            push(&mut attrs, "class", classes.join(" "));
        }
        if let Some(Js::Str(n)) = get("data-framer-name") {
            push(&mut attrs, "data-framer-name", n.clone());
        }
        if let Some(Js::Str(h)) = get("href") {
            push(&mut attrs, "href", h.clone());
        }
        let style = get("style").map(style_of).unwrap_or_default();
        if get("onTap").is_some()
            && let Some(c) = class
                .iter()
                .find(|c| c.starts_with("framer-") && !c.starts_with("framer-v-"))
        {
            out.taps.push(c.clone());
        }
        // Image de fond (composant Image de Framer).
        if let Some(Js::Obj(bg)) = get("background") {
            let src = bg
                .iter()
                .find(|(k, _)| k == "src")
                .and_then(|(_, v)| match v {
                    Js::Str(s) => Some(s.clone()),
                    _ => None,
                });
            let Some(src) = src else { return Vec::new() };
            let alt = bg
                .iter()
                .find(|(k, _)| k == "alt")
                .and_then(|(_, v)| match v {
                    Js::Str(s) => Some(s.clone()),
                    _ => None,
                });
            let fit = match bg.iter().find(|(k, _)| k == "fit").map(|(_, v)| v) {
                Some(Js::Str(f)) if f == "fit" => "contain",
                _ => "cover",
            };
            let num = |k: &str| {
                bg.iter().find(|(x, _)| x == k).and_then(|(_, v)| match v {
                    Js::Raw(r) => r.parse::<f64>().ok(),
                    _ => None,
                })
            };
            let mut style = style.clone();
            // Image qui garde son rapport (`fitImageDimension`) : l'autre côté suit.
            if let (Some(Js::Str(side)), Some(w), Some(h)) = (
                get("fitImageDimension"),
                num("intrinsicWidth"),
                num("intrinsicHeight"),
            ) && h > 0.0
            {
                let free = if side == "height" { "height" } else { "width" };
                style = [style, format!("{free}:auto;aspect-ratio:{w} / {h}")]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(";");
            }
            if !style.is_empty() {
                push(&mut attrs, "style", style);
            }
            let img = Node::Element {
                id: None,
                tag: "img".into(),
                attrs: vec![
                    Attr {
                        name: "decoding".into(),
                        value: Some("async".into()),
                        when: None,
                    },
                    Attr {
                        name: "src".into(),
                        value: Some(src),
                        when: None,
                    },
                    Attr {
                        name: "alt".into(),
                        value: Some(alt.unwrap_or_default()),
                        when: None,
                    },
                    Attr {
                        name: "style".into(),
                        value: Some(format!(
                            "display:block;width:100%;height:100%;border-radius:inherit;corner-shape:inherit;object-position:center;object-fit:{fit}"
                        )),
                        when: None,
                    },
                ],
                children: Vec::new(),
            };
            let wrap = Node::Element {
                id: None,
                tag: "div".into(),
                attrs: vec![
                    Attr {
                        name: "style".into(),
                        value: Some("position:absolute;border-radius:inherit;corner-shape:inherit;top:0;right:0;bottom:0;left:0".into()),
                        when: None,
                    },
                    Attr { name: "data-framer-background-image-wrapper".into(), value: Some("true".into()), when: None },
                ],
                children: vec![img],
            };
            return vec![Node::Element {
                id: None,
                tag: "div".into(),
                attrs,
                children: vec![wrap],
            }];
        }
        let text_box = get("__fromCanvasComponent").is_some();
        let mut children = get("children")
            .map(|c| self.render(c, out, in_text || text_box))
            .unwrap_or_default();
        // Texte venu d'une propriété de l'utilisation (`text:b`) : celui de la page.
        if text_box && let Some(Js::Raw(var)) = get("text") {
            if let Some(c) = class.iter().find(|c| c.starts_with("framer-")) {
                out.text_vars.push((var.clone(), c.clone()));
            }
            if let Some(t) = self.texts.get(var) {
                set_text(&mut children, t);
            }
        }
        match html {
            Some(t) => {
                if !style.is_empty() {
                    push(&mut attrs, "style", style);
                }
                vec![Node::Element {
                    id: None,
                    tag: t,
                    attrs,
                    children,
                }]
            }
            None if text_box => {
                push(
                    &mut attrs,
                    "data-framer-component-type",
                    "RichTextContainer".into(),
                );
                if !style.is_empty() {
                    push(&mut attrs, "style", style);
                }
                vec![Node::Element {
                    id: None,
                    tag: "div".into(),
                    attrs,
                    children,
                }]
            }
            // Enveloppe (fournisseur, fragment, groupe) : ses enfants.
            None => children,
        }
    }
}

/// Remplace le texte d'un bloc (premier texte trouvé ; les suivants retirés).
fn set_text(nodes: &mut Vec<Node>, t: &str) {
    fn go(nodes: &mut Vec<Node>, t: &str, done: &mut bool) {
        nodes.retain_mut(|n| match n {
            Node::Text { text } => {
                if *done {
                    false
                } else {
                    *text = t.to_string();
                    *done = true;
                    true
                }
            }
            Node::Element { children, .. } => {
                go(children, t, done);
                true
            }
            _ => true,
        });
    }
    let mut done = false;
    go(nodes, t, &mut done);
}

/// Classes écrites d'une prop `className` (`R(ie,`framer-x`,h)`, chaîne).
fn class_of(v: &Js) -> Vec<String> {
    let mut out = Vec::new();
    fn go(v: &Js, out: &mut Vec<String>) {
        match v {
            Js::Str(s) => out.extend(s.split_whitespace().map(str::to_string)),
            Js::Call(_, args) => args.iter().for_each(|a| go(a, out)),
            _ => {}
        }
    }
    go(v, &mut out);
    out
}

/// Style écrit (`style:{backgroundColor:`…`,borderRadius:30}`) en déclarations CSS.
fn style_of(v: &Js) -> String {
    let Js::Obj(entries) = v else {
        return String::new();
    };
    let mut m = Map::new();
    for (k, v) in entries {
        let val = match v {
            Js::Str(s) => Value::String(s.clone()),
            Js::Raw(r) => match r.parse::<f64>() {
                Ok(n) => serde_json::Number::from_f64(n)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
                Err(_) => continue,
            },
            _ => continue,
        };
        m.insert(k.clone(), val);
    }
    m.iter()
        .filter(|(k, _)| {
            !matches!(
                k.as_str(),
                "x" | "y" | "scale" | "rotate" | "opacity" | "transformPerspective"
            )
        })
        .filter_map(|(k, v)| css_decl(k, v))
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expressions() {
        let e = parse(
            "a(f.div,{className:R(ie,`framer-1`,h),children:[I()&&`Oui`,W(`v0`,l)??a(i,{children:`Non`}),...x]})",
        );
        let Js::Call(c, args) = e else { panic!() };
        assert_eq!(c, "a");
        assert_eq!(args[0], Js::Raw("f.div".into()));
        let Js::Obj(p) = &args[1] else { panic!() };
        assert_eq!(p[0].0, "className");
        let Js::Arr(ch) = &p[1].1 else {
            panic!("{:?}", p[1])
        };
        assert_eq!(ch.len(), 2);
        assert!(matches!(&ch[0], Js::And(..)));
        assert!(matches!(&ch[1], Js::Or(..)));
        assert!(matches!(parse("e=>e.x+1"), Js::Raw(_)));
        assert!(matches!(parse("U((u?.y||0)+(0+1)/2)"), Js::Call(..)));
    }

    #[test]
    fn variante_depliee() {
        let js = r#"Vt=v(E(f.div)),ae=()=>S!==`B`,I=()=>S===`B`;return a(p,{children:c(Vt,{className:R(ie,`framer-root`),"data-framer-name":`Default`,...Bt({B:{"data-framer-name":`Expanded`}},S,D),children:[ae()&&a(f.div,{className:`framer-ferme`,onTap:P,children:`Fermé`}),I()&&c(f.div,{className:`framer-ouvert`,style:{backgroundColor:`rgb(1, 2, 3)`,borderRadius:30},children:[a(C,{__fromCanvasComponent:!0,children:W(`v0`,l)??a(i,{children:a(f.h3,{className:`framer-styles-preset-x`,children:`Outils 3D`})}),className:`framer-titre`}),a(V,{background:{alt:``,fit:`fit`,src:`https://framerusercontent.com/images/a.svg?width=18`},className:`framer-icone`,onTap:re})]})]})})"#;
        let mut r = Render {
            variant: "B",
            conds: HashMap::new(),
            variants: vec!["A".into(), "B".into()],
            tags: HashMap::new(),
            texts: HashMap::new(),
        };
        r.read_defs(js, "S");
        assert_eq!(r.tags.get("Vt").map(String::as_str), Some("div"));
        let at = js.find("return ").unwrap() + 7;
        let (e, _) = parse_at(js, at);
        let mut out = Out::default();
        let nodes = r.render(&e, &mut out, false);
        let html = serde_json::to_string(&nodes).unwrap();
        assert!(
            html.contains("Expanded") && !html.contains("Fermé"),
            "{html}"
        );
        assert!(
            html.contains("\"framer-text framer-styles-preset-x\"") && html.contains("Outils 3D"),
            "{html}"
        );
        assert!(
            html.contains("background-color:rgb(1, 2, 3)") && html.contains("border-radius:30px"),
            "{html}"
        );
        assert!(
            html.contains("a.svg") && html.contains("object-fit:contain"),
            "{html}"
        );
        assert_eq!(out.taps, vec!["framer-icone".to_string()]);
        let r = Render { variant: "A", ..r };
        let mut out = Out::default();
        let html = serde_json::to_string(&r.render(&e, &mut out, false)).unwrap();
        assert!(
            html.contains("Fermé") && !html.contains("Outils 3D"),
            "{html}"
        );
        assert_eq!(out.taps, vec!["framer-ferme".to_string()]);
    }
}
