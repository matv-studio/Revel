//! Évaluation des expressions, portée des variables et rendu des nœuds.

use crate::html::{VOID, escape, escape_attr, escape_text};
use crate::{BuildInput, DocKind, ImageArgs, Job, RenderError};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use minijinja::value::{Object, Rest};
use minijinja::{Environment, Error, ErrorKind, State, UndefinedBehavior, Value};
use revel_model::{Layout, Node, Site};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

type Frame = Arc<BTreeMap<String, Value>>;

/// Pile de portées. La première est celle des données globales.
#[derive(Clone)]
pub(crate) struct Scope {
    frames: Vec<Frame>,
}

#[derive(Debug)]
struct ScopeView(Vec<Frame>);

impl Object for ScopeView {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let k = key.as_str()?;
        self.0.iter().rev().find_map(|f| f.get(k).cloned())
    }
}

impl Scope {
    pub(crate) fn new(globals: Frame, data: BTreeMap<String, Value>) -> Self {
        Self {
            frames: vec![globals, Arc::new(data)],
        }
    }

    fn push(&mut self, m: BTreeMap<String, Value>) {
        self.frames.push(Arc::new(m));
    }

    fn pop(&mut self) {
        self.frames.pop();
    }

    fn get(&self, k: &str) -> Option<Value> {
        self.frames.iter().rev().find_map(|f| f.get(k).cloned())
    }

    /// Met à jour la variable existante la plus proche, sinon la crée dans la portée courante.
    fn set(&mut self, k: &str, v: Value) {
        let n = self.frames.len();
        for i in (1..n).rev() {
            if self.frames[i].contains_key(k) {
                Arc::make_mut(&mut self.frames[i]).insert(k.to_string(), v);
                return;
            }
        }
        if let Some(last) = self.frames.last_mut() {
            Arc::make_mut(last).insert(k.to_string(), v);
        }
    }

    fn view(&self) -> Value {
        Value::from_object(ScopeView(self.frames.clone()))
    }
}

/// Mode de sortie : règles d'échappement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Html,
    Xml,
    Text,
    /// Valeur d'attribut : le texte littéral est aussi échappé.
    Attr,
}

pub(crate) struct Builder<'a> {
    site: &'a Site,
    env: Environment<'static>,
    input: &'a BuildInput<'a>,
    warnings: RefCell<Vec<String>>,
    attr_cache: RefCell<HashMap<String, Vec<Node>>>,
}

impl<'a> Builder<'a> {
    pub(crate) fn new(input: &'a BuildInput<'a>) -> Self {
        let mut env = Environment::new();
        env.set_undefined_behavior(UndefinedBehavior::Chainable);
        register_filters(&mut env, input);
        for (name, f) in &input.site.settings.filters {
            let f = f.clone();
            env.add_filter(
                name.clone(),
                move |state: &State, value: Value, args: Rest<Value>| -> Result<Value, Error> {
                    let mut ctx: BTreeMap<String, Value> = BTreeMap::new();
                    if let Some(p) = f.params.first() {
                        ctx.insert(p.clone(), value);
                    }
                    for (p, a) in f.params.iter().skip(1).zip(args.iter()) {
                        ctx.insert(p.clone(), a.clone());
                    }
                    state
                        .env()
                        .compile_expression(&f.expr)?
                        .eval(Value::from(ctx))
                },
            );
        }
        Self {
            site: input.site,
            env,
            input,
            warnings: RefCell::new(Vec::new()),
            attr_cache: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn take_warnings(&self) -> Vec<String> {
        std::mem::take(&mut self.warnings.borrow_mut())
    }

    fn warn(&self, w: String) {
        let mut ws = self.warnings.borrow_mut();
        if !ws.contains(&w) {
            ws.push(w);
        }
    }

    /// Données globales sans les collections (pour les filtres et chemins de collection).
    pub(crate) fn base_globals(&self) -> Frame {
        Arc::new(self.globals_map(BTreeMap::new()))
    }

    pub(crate) fn globals(&self, collections: BTreeMap<String, Vec<Value>>) -> Frame {
        let cols: BTreeMap<String, Value> = collections
            .into_iter()
            .map(|(k, v)| (k, Value::from(v)))
            .collect();
        Arc::new(self.globals_map(cols))
    }

    fn globals_map(&self, collections: BTreeMap<String, Value>) -> BTreeMap<String, Value> {
        let mut g: BTreeMap<String, Value> = self
            .site
            .data
            .iter()
            .map(|(k, v)| (k.clone(), crate::eval::from_json(v)))
            .collect();
        g.insert("collections".into(), Value::from(collections));
        let mut env: BTreeMap<String, Value> = BTreeMap::new();
        if let Some(serde_json::Value::Object(m)) =
            self.site.settings.environments.get(&self.input.environment)
        {
            for (k, v) in m {
                env.insert(k.clone(), crate::eval::from_json(v));
            }
        }
        env.insert("name".into(), Value::from(self.input.environment.clone()));
        g.insert("env".into(), Value::from(env));
        g.insert("version".into(), Value::from(self.input.version.clone()));
        g
    }

    pub(crate) fn eval(&self, scope: &Scope, expr: &str, ctx: &str) -> Result<Value, RenderError> {
        let e = self
            .env
            .compile_expression(expr)
            .map_err(|e| RenderError::Expr {
                context: ctx.into(),
                expr: expr.into(),
                msg: e.to_string(),
            })?;
        e.eval(scope.view()).map_err(|e| RenderError::Expr {
            context: ctx.into(),
            expr: expr.into(),
            msg: e.to_string(),
        })
    }

    /// Rendu d'une chaîne contenant des `{{ }}` (chemins de collection) : texte brut.
    pub(crate) fn render_inline(
        &self,
        scope: &Scope,
        src: &str,
        ctx: &str,
    ) -> Result<String, RenderError> {
        let nodes = self.template_nodes(src)?;
        let mut out = String::new();
        let mut s = scope.clone();
        self.nodes(&nodes, &mut s, &mut out, Mode::Text, ctx)?;
        Ok(out)
    }

    fn template_nodes(&self, src: &str) -> Result<Vec<Node>, RenderError> {
        if let Some(n) = self.attr_cache.borrow().get(src) {
            return Ok(n.clone());
        }
        let parsed = revel_markup::parse(
            src,
            &revel_markup::Options {
                mode: revel_markup::Mode::Text,
                ..Default::default()
            },
        )
        .map_err(|e| RenderError::Markup(format!("« {src} » : {e}")))?;
        self.attr_cache
            .borrow_mut()
            .insert(src.to_string(), parsed.nodes.clone());
        Ok(parsed.nodes)
    }

    pub(crate) fn layout_chain(&self, first: Option<&str>) -> Result<Vec<&'a Layout>, RenderError> {
        let mut chain = Vec::new();
        let mut cur = first.map(str::to_string);
        while let Some(id) = cur {
            let l = self
                .site
                .layout(&id)
                .ok_or_else(|| RenderError::MissingLayout(id.clone()))?;
            if chain.iter().any(|c: &&Layout| c.id == l.id) {
                break;
            }
            chain.push(l);
            cur = l.parent.clone();
        }
        Ok(chain)
    }

    pub(crate) fn render_job(
        &self,
        globals: &Frame,
        j: &Job,
        kind: DocKind,
        _warnings: &mut Vec<String>,
    ) -> Result<String, RenderError> {
        let mode = match kind {
            DocKind::Html => Mode::Html,
            DocKind::Xml => Mode::Xml,
            DocKind::Text => Mode::Text,
        };
        let chain = self.layout_chain(j.layout.as_deref())?;
        let mut data = j.data.clone();
        // Variables des gabarits : priorité la plus faible (le plus proche de la page l'emporte).
        for l in &chain {
            for (k, v) in &l.props {
                data.entry(k.clone())
                    .or_insert_with(|| crate::eval::from_json(v));
            }
        }
        let mut page: BTreeMap<String, Value> = BTreeMap::new();
        page.insert("url".into(), Value::from(j.url.clone()));
        page.insert("fileSlug".into(), Value::from(j.file_slug.clone()));
        page.insert(
            "date".into(),
            j.data.get("date").cloned().unwrap_or(Value::UNDEFINED),
        );
        page.insert(
            "outputPath".into(),
            Value::from(crate::output_path(&j.url).unwrap_or_default()),
        );
        data.insert("page".into(), Value::from(page));
        let ctx = format!("page {}", j.url);
        let mut content = match (j.body, &j.content) {
            (Some(body), _) => {
                let mut scope = Scope::new(globals.clone(), data.clone());
                let mut out = String::new();
                self.nodes(body, &mut scope, &mut out, mode, &ctx)?;
                out
            }
            (None, Some(c)) => c.clone(),
            (None, None) => String::new(),
        };
        for l in chain {
            let mut d = data.clone();
            d.insert("content".into(), Value::from_safe_string(content));
            let mut scope = Scope::new(globals.clone(), d);
            let mut out = String::new();
            self.nodes(
                &l.body,
                &mut scope,
                &mut out,
                mode,
                &format!("{ctx} (gabarit {})", l.id),
            )?;
            content = out;
        }
        Ok(content)
    }

    fn nodes(
        &self,
        nodes: &[Node],
        scope: &mut Scope,
        out: &mut String,
        mode: Mode,
        ctx: &str,
    ) -> Result<(), RenderError> {
        for n in nodes {
            self.node(n, scope, out, mode, ctx)?;
        }
        Ok(())
    }

    fn node(
        &self,
        n: &Node,
        scope: &mut Scope,
        out: &mut String,
        mode: Mode,
        ctx: &str,
    ) -> Result<(), RenderError> {
        match n {
            Node::Element {
                tag,
                attrs,
                children,
                ..
            } => {
                out.push('<');
                out.push_str(tag);
                for a in attrs {
                    if let Some(w) = &a.when
                        && !self.eval(scope, w, ctx)?.is_true()
                    {
                        continue;
                    }
                    out.push(' ');
                    out.push_str(&a.name);
                    if let Some(v) = &a.value {
                        out.push_str("=\"");
                        if v.contains("{{") || v.contains("{%") {
                            let nodes = self.template_nodes(v)?;
                            self.nodes(&nodes, scope, out, Mode::Attr, ctx)?;
                        } else {
                            out.push_str(&escape_attr(v));
                        }
                        out.push('"');
                    }
                }
                // XML : élément vide écrit `<x/>`.
                if mode == Mode::Xml && children.is_empty() {
                    out.push_str("/>");
                    return Ok(());
                }
                out.push('>');
                let void = mode == Mode::Html && VOID.contains(&tag.to_ascii_lowercase().as_str());
                if !void {
                    self.nodes(children, scope, out, mode, ctx)?;
                    out.push_str("</");
                    out.push_str(tag);
                    out.push('>');
                }
            }
            Node::Text { text } => match mode {
                Mode::Html | Mode::Xml => out.push_str(&escape_text(text)),
                Mode::Attr => out.push_str(&escape_attr(text)),
                Mode::Text => out.push_str(text),
            },
            Node::Html { html } => match mode {
                Mode::Attr => out.push_str(&escape_attr(html)),
                _ => out.push_str(html),
            },
            Node::Out { expr } => {
                let v = self.eval(scope, expr, ctx)?;
                out.push_str(&display(&v, true));
            }
            Node::If {
                cond,
                then,
                otherwise,
            } => {
                if self.eval(scope, cond, ctx)?.is_true() {
                    self.nodes(then, scope, out, mode, ctx)?;
                } else {
                    self.nodes(otherwise, scope, out, mode, ctx)?;
                }
            }
            Node::Each {
                expr,
                item,
                children,
                empty,
            } => {
                let v = self.eval(scope, expr, ctx)?;
                let names: Vec<&str> = item.split(',').map(str::trim).collect();
                let items: Vec<Value> = if v.is_undefined() || v.is_none() {
                    Vec::new()
                } else {
                    match v.try_iter() {
                        Ok(it) => it.collect(),
                        Err(e) => {
                            return Err(RenderError::Expr {
                                context: ctx.into(),
                                expr: expr.clone(),
                                msg: e.to_string(),
                            });
                        }
                    }
                };
                if items.is_empty() {
                    return self.nodes(empty, scope, out, mode, ctx);
                }
                let len = items.len();
                for (i, it) in items.into_iter().enumerate() {
                    let mut f: BTreeMap<String, Value> = BTreeMap::new();
                    if names.len() >= 2 {
                        // Dictionnaire : clé, valeur.
                        let val = v.get_item(&it).unwrap_or(Value::UNDEFINED);
                        f.insert(names[0].to_string(), it);
                        f.insert(names[1].to_string(), val);
                    } else {
                        f.insert(names[0].to_string(), it);
                    }
                    let mut lp: BTreeMap<&str, Value> = BTreeMap::new();
                    lp.insert("index", Value::from(i + 1));
                    lp.insert("index0", Value::from(i));
                    lp.insert("first", Value::from(i == 0));
                    lp.insert("last", Value::from(i + 1 == len));
                    lp.insert("length", Value::from(len));
                    lp.insert("revindex", Value::from(len - i));
                    f.insert("loop".into(), Value::from(lp));
                    scope.push(f);
                    let r = self.nodes(children, scope, out, mode, ctx);
                    scope.pop();
                    r?;
                }
            }
            Node::Set { name, expr } => {
                let v = self.eval(scope, expr, ctx)?;
                scope.set(name, v);
            }
            Node::SetBlock { name, children } => {
                let mut s = String::new();
                self.nodes(children, scope, &mut s, mode, ctx)?;
                scope.set(name, Value::from_safe_string(s));
            }
            Node::Slot => {
                if let Some(c) = scope.get("content") {
                    out.push_str(&display(&c, false));
                }
            }
            Node::Use { component } => match self.site.component(component) {
                Some(c) => {
                    scope.push(BTreeMap::new());
                    let r = self.nodes(
                        &c.body,
                        scope,
                        out,
                        mode,
                        &format!("{ctx} (composant {component})"),
                    );
                    scope.pop();
                    r?;
                }
                None => self.warn(format!("{ctx} : composant introuvable « {component} »")),
            },
            Node::Image(img) => {
                let s = |e: &Option<String>, scope: &Scope| -> Result<String, RenderError> {
                    match e {
                        Some(e) => Ok(display(&self.eval(scope, e, ctx)?, false)),
                        None => Ok(String::new()),
                    }
                };
                let args = ImageArgs {
                    src: display(&self.eval(scope, &img.src, ctx)?, false),
                    alt: s(&img.alt, scope)?,
                    sizes: s(&img.sizes, scope)?,
                    class: s(&img.class, scope)?,
                    ratio: s(&img.ratio, scope)?,
                    loading: s(&img.loading, scope)?,
                    widths: s(&img.widths, scope)?
                        .split(',')
                        .filter_map(|w| w.trim().parse().ok())
                        .collect(),
                };
                if img.optional && args.src.is_empty() {
                    return Ok(());
                }
                match self.input.hooks.image(&args) {
                    Some(html) => out.push_str(&html),
                    None => {
                        if let Some(c) = self.site.component("image-placeholder") {
                            let mut f: BTreeMap<String, Value> = BTreeMap::new();
                            f.insert("src".into(), Value::from(args.src.clone()));
                            f.insert("alt".into(), Value::from(args.alt.clone()));
                            f.insert("ratio".into(), Value::from(args.ratio.clone()));
                            f.insert("class".into(), Value::from(args.class.clone()));
                            scope.push(f);
                            let r = self.nodes(&c.body, scope, out, mode, ctx);
                            scope.pop();
                            r?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Valeur affichée : rien pour `none`/indéfini ; échappée sauf valeur sûre (`| safe`).
fn display(v: &Value, escape_unsafe: bool) -> String {
    if v.is_undefined() || v.is_none() {
        return String::new();
    }
    let s = v.to_string();
    if escape_unsafe && !v.is_safe() {
        escape(&s)
    } else {
        s
    }
}

// ------------------------------------------------------------------ filtres

pub(crate) fn absolute_url(url: &str, base: &str) -> String {
    if url.is_empty() {
        return base.to_string();
    }
    match url::Url::parse(base).and_then(|b| b.join(url)) {
        Ok(u) => u.to_string(),
        Err(_) => url.to_string(),
    }
}

/// Date ISO (avec ou sans heure/fuseau) ou horodatage -> UTC.
pub(crate) fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Utc));
    }
    for f in [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M",
    ] {
        if let Ok(d) = NaiveDateTime::parse_from_str(s, f) {
            return Some(d.and_utc());
        }
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return d.and_hms_opt(0, 0, 0).map(|d| d.and_utc());
    }
    s.parse::<i64>()
        .ok()
        .and_then(DateTime::from_timestamp_millis)
}

fn items_of(v: &Value) -> Vec<Value> {
    if v.is_undefined() || v.is_none() {
        return Vec::new();
    }
    v.try_iter().map(|i| i.collect()).unwrap_or_default()
}

/// Lit un chemin `a.b.c` dans une valeur.
fn path(v: &Value, p: &str) -> Value {
    let mut cur = v.clone();
    for part in p.split('.') {
        cur = cur.get_attr(part).unwrap_or(Value::UNDEFINED);
    }
    cur
}

fn register_filters(env: &mut Environment<'static>, input: &BuildInput) {
    let now = input.now;
    env.add_function("now", move || Value::from(now.to_rfc3339()));
    // Comme Nunjucks : longueur d'une valeur absente = 0 (au lieu d'une erreur).
    env.add_filter("length", |v: Value| -> Result<Value, Error> {
        if v.is_undefined() || v.is_none() {
            return Ok(Value::from(0));
        }
        v.len()
            .map(Value::from)
            .ok_or_else(|| Error::new(ErrorKind::InvalidOperation, "longueur indisponible"))
    });
    env.add_filter("limit", |v: Value, n: usize| {
        Value::from(items_of(&v).into_iter().take(n).collect::<Vec<_>>())
    });
    env.add_filter("without", |v: Value, url: Value| {
        Value::from(
            items_of(&v)
                .into_iter()
                .filter(|i| i.get_attr("url").ok().as_ref() != Some(&url))
                .collect::<Vec<_>>(),
        )
    });
    env.add_filter("by_slug", |v: Value, slug: Value| {
        items_of(&v)
            .into_iter()
            .find(|i| path(i, "data.slug") == slug)
            .unwrap_or(Value::UNDEFINED)
    });
    env.add_filter("by_slugs", |v: Value, slugs: Value| {
        let items = items_of(&v);
        Value::from(
            items_of(&slugs)
                .iter()
                .filter_map(|s| items.iter().find(|i| &path(i, "data.slug") == s).cloned())
                .collect::<Vec<_>>(),
        )
    });
    env.add_filter(
        "where_contains",
        |v: Value, field: String, needle: Value| {
            Value::from(
                items_of(&v)
                    .into_iter()
                    .filter(|i| items_of(&path(i, &field)).contains(&needle))
                    .collect::<Vec<_>>(),
            )
        },
    );
    env.add_filter("find_by", |v: Value, field: String, needle: Value| {
        items_of(&v)
            .into_iter()
            .find(|i| path(i, &field) == needle)
            .unwrap_or(Value::UNDEFINED)
    });
    env.add_filter("date", |v: Value, fmt: String| -> Result<Value, Error> {
        if v.is_undefined() || v.is_none() {
            return Ok(Value::from(""));
        }
        Ok(Value::from(
            parse_date(&v.to_string())
                .map(|d| d.format(&fmt).to_string())
                .unwrap_or_default(),
        ))
    });
    env.add_filter("iso_date", |v: Value| {
        Value::from(
            parse_date(&v.to_string())
                .map(|d| d.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
                .unwrap_or_default(),
        )
    });
    env.add_filter("utc_string", |v: Value| {
        Value::from(
            parse_date(&v.to_string())
                .map(|d| d.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
                .unwrap_or_default(),
        )
    });
    env.add_filter("strip_tags", |v: Value| {
        let s = if v.is_undefined() || v.is_none() {
            String::new()
        } else {
            v.to_string()
        };
        let re = regex::Regex::new(r"<[^>]+>").expect("regex");
        let t = re.replace_all(&s, " ");
        Value::from(t.split_whitespace().collect::<Vec<_>>().join(" "))
    });
    env.add_filter("truncate", |v: Value, n: Option<usize>| {
        let s = v.to_string();
        let n = n.unwrap_or(160);
        let chars: Vec<char> = s.chars().collect();
        if chars.len() <= n {
            return Value::from(s);
        }
        let cut: String = chars[..n.saturating_sub(1)].iter().collect();
        let re = regex::Regex::new(r"\s+\S*$").expect("regex");
        Value::from(format!("{}…", re.replace(&cut, "")))
    });
    env.add_filter("json", |v: Value| -> Result<Value, Error> {
        serde_json::to_string(&v)
            .map(Value::from)
            .map_err(|e| Error::new(ErrorKind::InvalidOperation, e.to_string()))
    });
    env.add_filter("absolute_url", |v: Value, base: String| {
        let u = if v.is_undefined() || v.is_none() {
            String::new()
        } else {
            v.to_string()
        };
        Value::from(absolute_url(&u, &base))
    });
    env.add_filter(
        "regex_replace",
        |v: Value, pattern: String, rep: String| -> Result<Value, Error> {
            let re = regex::Regex::new(&pattern)
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, e.to_string()))?;
            let safe = v.is_safe();
            let s = if v.is_undefined() || v.is_none() {
                String::new()
            } else {
                v.to_string()
            };
            let r = re.replace_all(&s, rep.as_str()).into_owned();
            Ok(if safe {
                Value::from_safe_string(r)
            } else {
                Value::from(r)
            })
        },
    );
    env.add_filter(
        "regex_capture",
        |v: Value, pattern: String, group: Option<usize>| -> Result<Value, Error> {
            let re = regex::Regex::new(&pattern)
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, e.to_string()))?;
            let s = if v.is_undefined() || v.is_none() {
                String::new()
            } else {
                v.to_string()
            };
            Ok(re
                .captures(&s)
                .and_then(|c| c.get(group.unwrap_or(1)))
                .map(|m| Value::from(m.as_str()))
                .unwrap_or(Value::from("")))
        },
    );
    let hooks = input.hooks.clone();
    env.add_filter("media_exists", move |v: Value| {
        Value::from(!v.is_undefined() && !v.is_none() && hooks.media_exists(&v.to_string()))
    });
    let hooks = input.hooks.clone();
    env.add_filter("share_image", move |v: Value, base: String| {
        let s = if v.is_undefined() || v.is_none() {
            String::new()
        } else {
            v.to_string()
        };
        Value::from(hooks.share_image(&s, &base))
    });
}

/// Valeur JSON -> valeur d'expression.
pub(crate) fn from_json(v: &serde_json::Value) -> Value {
    Value::from(minijinja::value::Serde(v))
}
