//! Évaluation des expressions, portée des variables et rendu des nœuds.

use crate::html::{VOID, escape, escape_attr, escape_text};
use crate::{BuildInput, DocKind, ImageArgs, Job, RenderError};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use minijinja::value::{Object, Rest};
use minijinja::{Environment, Error, ErrorKind, State, UndefinedBehavior, Value};
use revel_edit::canvas::{Binding, Seg, Slot, Source, path_expr, source_path};
use revel_edit::{Branch, Owner, Parent};
use revel_model::{Layout, Node, Site};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

type Frame = Arc<BTreeMap<String, Value>>;
pub(crate) type FrameRef = Frame;

/// Origines des variables d'une portée (éditeur seulement).
type Origins = Option<Arc<BTreeMap<String, Binding>>>;

/// Pile de portées. La première est celle des données globales. En mode éditeur, chaque portée
/// garde aussi l'origine de ses variables (D-19).
#[derive(Clone)]
pub(crate) struct Scope {
    frames: Vec<Frame>,
    origins: Vec<Origins>,
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
            origins: vec![None, None],
        }
    }

    /// Origines des données globales et de la page.
    fn with_origins(
        mut self,
        globals: BTreeMap<String, Binding>,
        data: BTreeMap<String, Binding>,
    ) -> Self {
        self.origins = vec![Some(Arc::new(globals)), Some(Arc::new(data))];
        self
    }

    fn push(&mut self, m: BTreeMap<String, Value>) {
        self.frames.push(Arc::new(m));
        self.origins.push(None);
    }

    fn push_with(&mut self, m: BTreeMap<String, Value>, o: BTreeMap<String, Binding>) {
        self.frames.push(Arc::new(m));
        self.origins.push((!o.is_empty()).then(|| Arc::new(o)));
    }

    fn pop(&mut self) {
        self.frames.pop();
        self.origins.pop();
    }

    fn get(&self, k: &str) -> Option<Value> {
        self.frames.iter().rev().find_map(|f| f.get(k).cloned())
    }

    /// Origine de la variable `k` (portée la plus proche qui la définit).
    fn origin(&self, k: &str) -> Option<&Binding> {
        let i = self.frames.iter().rposition(|f| f.contains_key(k))?;
        self.origins.get(i)?.as_ref()?.get(k)
    }

    /// Met à jour la variable existante la plus proche, sinon la crée dans la portée courante.
    fn set(&mut self, k: &str, v: Value, origin: Option<Binding>) {
        let n = self.frames.len();
        let i = (1..n)
            .rev()
            .find(|&i| self.frames[i].contains_key(k))
            .unwrap_or(n - 1);
        Arc::make_mut(&mut self.frames[i]).insert(k.to_string(), v);
        let slot = &mut self.origins[i];
        match (origin, slot.as_mut()) {
            (Some(o), Some(m)) => {
                Arc::make_mut(m).insert(k.to_string(), o);
            }
            (Some(o), None) => *slot = Some(Arc::new(BTreeMap::from([(k.to_string(), o)]))),
            (None, Some(m)) => {
                Arc::make_mut(m).remove(k);
            }
            (None, None) => {}
        }
    }

    fn view(&self) -> Value {
        Value::from_object(ScopeView(self.frames.clone()))
    }
}

/// Emplacement de la liste de nœuds rendue (repères de l'éditeur).
#[derive(Clone)]
pub(crate) struct At {
    owner: Option<Owner>,
    parent: Option<String>,
    branch: Branch,
    /// Repères écrits (faux dans un attribut, un bloc capturé `{% set %}`, `<title>`…).
    mark: bool,
}

impl At {
    pub(crate) const NONE: At = At {
        owner: None,
        parent: None,
        branch: Branch::Children,
        mark: false,
    };

    pub(crate) fn root(owner: Owner) -> At {
        At {
            owner: Some(owner),
            parent: None,
            branch: Branch::Children,
            mark: true,
        }
    }

    /// Branche d'un nœud : sans identifiant, ses textes ne sont pas adressables.
    fn child(&self, id: Option<&str>, branch: Branch, mark: bool) -> At {
        At {
            owner: self.owner.clone(),
            parent: id.map(str::to_string),
            branch,
            mark: self.mark && mark && id.is_some(),
        }
    }

    fn parent(&self) -> Option<Parent> {
        Some(Parent {
            owner: self.owner.clone()?,
            node: self.parent.clone(),
            branch: self.branch,
        })
    }
}

/// Éléments dont le contenu est du texte brut (un commentaire y serait affiché).
const RAW_TEXT: &[&str] = &[
    "title",
    "textarea",
    "script",
    "style",
    "noscript",
    "xmp",
    "iframe",
    "noembed",
    "noframes",
    "plaintext",
];

/// Types de `<script>` qui ne s'exécutent pas (gardés dans le canevas).
const DATA_SCRIPTS: &[&str] = &["application/ld+json", "application/json", "text/plain"];

/// État du rendu pour l'éditeur.
pub(crate) struct Editor {
    pub(crate) slots: RefCell<Vec<Slot>>,
    /// Scripts du site exécutés dans le canevas.
    pub(crate) scripts: bool,
    /// Éléments de collection : `fileSlug` -> (dossier, fichier, adresse).
    pub(crate) entries: HashMap<String, Vec<(String, String, Value)>>,
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
    pub(crate) editor: Option<Editor>,
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
            editor: None,
        }
    }

    fn marking(&self, at: &At) -> bool {
        at.mark && self.editor.is_some()
    }

    /// Ajoute un repère ; renvoie son numéro.
    fn slot(&self, s: Slot) -> usize {
        let e = self.editor.as_ref().expect("mode éditeur");
        let mut v = e.slots.borrow_mut();
        v.push(s);
        v.len() - 1
    }

    /// Origine d'une expression qui se réduit à un chemin.
    fn origin_of(&self, scope: &Scope, expr: &str) -> Option<Binding> {
        self.editor.as_ref()?;
        let (head, segs) = path_expr(expr)?;
        scope.origin(&head)?.extend(&segs)
    }

    /// Origine d'un élément de collection, retrouvé par son `fileSlug` et son adresse.
    fn entry_item(&self, item: &Value) -> Option<Binding> {
        let e = self.editor.as_ref()?;
        let slug = item.get_attr("fileSlug").ok()?;
        let cands = e.entries.get(slug.as_str()?)?;
        let url = item.get_attr("url").unwrap_or(Value::UNDEFINED);
        let (folder, file, _) = match cands.as_slice() {
            [one] => one,
            many => many.iter().find(|(_, _, u)| *u == url)?,
        };
        Some(Binding::new(Source::EntryItem {
            folder: folder.clone(),
            file: file.clone(),
        }))
    }

    /// Origine d'une valeur obtenue autrement que par un chemin (filtre, recherche) : élément de
    /// collection reconnu, ou objet présent une seule fois dans les données globales.
    fn origin_of_value(&self, v: &Value) -> Option<Binding> {
        self.editor.as_ref()?;
        if v.kind() != minijinja::value::ValueKind::Map {
            return None;
        }
        if let Some(b) = self.entry_item(v) {
            return Some(b);
        }
        let target = serde_json::to_value(v).ok()?;
        let mut found: Vec<Vec<Seg>> = Vec::new();
        fn search(
            j: &serde_json::Value,
            target: &serde_json::Value,
            path: &mut Vec<Seg>,
            found: &mut Vec<Vec<Seg>>,
        ) {
            if found.len() > 1 {
                return;
            }
            if j == target {
                found.push(path.clone());
                return;
            }
            match j {
                serde_json::Value::Object(m) => {
                    for (k, x) in m {
                        path.push(Seg::Key(k.clone()));
                        search(x, target, path, found);
                        path.pop();
                    }
                }
                serde_json::Value::Array(a) => {
                    for (i, x) in a.iter().enumerate() {
                        path.push(Seg::Index(i));
                        search(x, target, path, found);
                        path.pop();
                    }
                }
                _ => {}
            }
        }
        for (k, j) in &self.site.data {
            let mut path = vec![Seg::Key(k.clone())];
            search(j, &target, &mut path, &mut found);
        }
        match found.as_slice() {
            [one] => Some(Binding {
                source: Source::Data,
                path: one.clone(),
            }),
            _ => None,
        }
    }

    /// Origines des données globales (fichiers du dossier de données).
    pub(crate) fn global_origins(&self) -> BTreeMap<String, Binding> {
        self.site
            .data
            .keys()
            .map(|k| {
                (
                    k.clone(),
                    Binding {
                        source: Source::Data,
                        path: vec![Seg::Key(k.clone())],
                    },
                )
            })
            .collect()
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
        self.nodes(&nodes, &mut s, &mut out, Mode::Text, ctx, &At::NONE)?;
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
        let mut origins = j.origins.clone();
        // Variables des gabarits : priorité la plus faible (le plus proche de la page l'emporte).
        for l in &chain {
            for (k, v) in &l.props {
                if !data.contains_key(k) {
                    data.insert(k.clone(), crate::eval::from_json(v));
                    origins.insert(
                        k.clone(),
                        Binding {
                            source: Source::Layout {
                                layout: l.id.clone(),
                            },
                            path: vec![Seg::Key(k.clone())],
                        },
                    );
                }
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
        origins.remove("page");
        let global_origins = if self.editor.is_some() {
            self.global_origins()
        } else {
            BTreeMap::new()
        };
        let scope_for = |d: BTreeMap<String, Value>, o: &BTreeMap<String, Binding>| {
            let s = Scope::new(globals.clone(), d);
            if self.editor.is_some() {
                s.with_origins(global_origins.clone(), o.clone())
            } else {
                s
            }
        };
        let ctx = format!("page {}", j.url);
        let mut content = match (j.body, &j.content) {
            (Some(body), _) => {
                let mut scope = scope_for(data.clone(), &origins);
                let mut out = String::new();
                let at = match &j.page_id {
                    Some(id) => At::root(Owner::Page(id.clone())),
                    None => At::NONE,
                };
                self.nodes(body, &mut scope, &mut out, mode, &ctx, &at)?;
                out
            }
            (None, Some(c)) => c.clone(),
            (None, None) => String::new(),
        };
        origins.remove("content");
        for l in chain {
            let mut d = data.clone();
            d.insert("content".into(), Value::from_safe_string(content));
            let mut scope = scope_for(d, &origins);
            let mut out = String::new();
            self.nodes(
                &l.body,
                &mut scope,
                &mut out,
                mode,
                &format!("{ctx} (gabarit {})", l.id),
                &At::root(Owner::Layout(l.id.clone())),
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
        at: &At,
    ) -> Result<(), RenderError> {
        for (i, n) in nodes.iter().enumerate() {
            self.node(n, i, scope, out, mode, ctx, at)?;
        }
        Ok(())
    }

    /// Écrit `body` entouré des repères d'un [`Slot`].
    fn wrapped(
        &self,
        out: &mut String,
        slot: Slot,
        body: impl FnOnce(&mut String) -> Result<(), RenderError>,
    ) -> Result<(), RenderError> {
        let n = self.slot(slot);
        out.push_str(&format!("<!--rv:{n}-->"));
        body(out)?;
        out.push_str(&format!("<!--/rv:{n}-->"));
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn node(
        &self,
        n: &Node,
        index: usize,
        scope: &mut Scope,
        out: &mut String,
        mode: Mode,
        ctx: &str,
        at: &At,
    ) -> Result<(), RenderError> {
        let marking = self.marking(at) && mode == Mode::Html;
        match n {
            Node::Element {
                id,
                tag,
                attrs,
                children,
            } => {
                out.push('<');
                out.push_str(tag);
                if marking && let Some(id) = id {
                    out.push_str(" data-rv=\"");
                    out.push_str(&escape_attr(id));
                    out.push('"');
                }
                // Canevas : scripts du site neutralisés (sauf données JSON).
                let lower = tag.to_ascii_lowercase();
                let neutralize = lower == "script"
                    && self.editor.as_ref().is_some_and(|e| !e.scripts)
                    && !attrs.iter().any(|a| {
                        a.name.eq_ignore_ascii_case("type")
                            && a.value.as_deref().is_some_and(|v| {
                                DATA_SCRIPTS.contains(&v.trim().to_ascii_lowercase().as_str())
                            })
                    });
                if neutralize {
                    out.push_str(" type=\"text/plain\" data-rv-script");
                }
                for a in attrs {
                    if let Some(w) = &a.when
                        && !self.eval(scope, w, ctx)?.is_true()
                    {
                        continue;
                    }
                    out.push(' ');
                    if neutralize && a.name.eq_ignore_ascii_case("type") {
                        out.push_str("data-rv-type");
                    } else {
                        out.push_str(&a.name);
                    }
                    if let Some(v) = &a.value {
                        out.push_str("=\"");
                        if v.contains("{{") || v.contains("{%") {
                            let nodes = self.template_nodes(v)?;
                            self.nodes(&nodes, scope, out, Mode::Attr, ctx, &At::NONE)?;
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
                let void = mode == Mode::Html && VOID.contains(&lower.as_str());
                if !void {
                    let inner = at.child(
                        id.as_deref(),
                        Branch::Children,
                        !RAW_TEXT.contains(&lower.as_str()),
                    );
                    self.nodes(children, scope, out, mode, ctx, &inner)?;
                    out.push_str("</");
                    out.push_str(tag);
                    out.push('>');
                }
            }
            Node::Text { text } => {
                let write = |out: &mut String| {
                    match mode {
                        Mode::Html | Mode::Xml => out.push_str(&escape_text(text)),
                        Mode::Attr => out.push_str(&escape_attr(text)),
                        Mode::Text => out.push_str(text),
                    }
                    Ok(())
                };
                match at.parent().filter(|_| marking && !text.trim().is_empty()) {
                    Some(parent) => self.wrapped(out, Slot::Text { parent, index }, write)?,
                    None => write(out)?,
                }
            }
            Node::Html { html } => match mode {
                Mode::Attr => out.push_str(&escape_attr(html)),
                _ => out.push_str(html),
            },
            Node::Out { expr } => {
                let v = self.eval(scope, expr, ctx)?;
                let shown = display(&v, true);
                match at.parent().filter(|_| marking && !shown.is_empty()) {
                    Some(parent) => {
                        // Seule une valeur simple (texte, nombre) lue telle quelle est reliée
                        // à sa source.
                        let simple = matches!(
                            v.kind(),
                            minijinja::value::ValueKind::String
                                | minijinja::value::ValueKind::Number
                        );
                        let (binding, transformed) = match source_path(expr) {
                            Some((head, segs, transformed)) if simple => (
                                scope.origin(&head).and_then(|o| o.extend(&segs)),
                                transformed,
                            ),
                            _ => (None, false),
                        };
                        let slot = Slot::Expr {
                            parent,
                            index,
                            expr: expr.clone(),
                            transformed: transformed && binding.is_some(),
                            binding,
                            html: v.is_safe(),
                        };
                        self.wrapped(out, slot, |o| {
                            o.push_str(&shown);
                            Ok(())
                        })?;
                    }
                    None => out.push_str(&shown),
                }
            }
            Node::If {
                id,
                cond,
                then,
                otherwise,
            } => {
                if self.eval(scope, cond, ctx)?.is_true() {
                    let inner = at.child(id.as_deref(), Branch::Then, true);
                    self.nodes(then, scope, out, mode, ctx, &inner)?;
                } else {
                    let inner = at.child(id.as_deref(), Branch::Otherwise, true);
                    self.nodes(otherwise, scope, out, mode, ctx, &inner)?;
                }
            }
            Node::Each {
                id,
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
                    let inner = at.child(id.as_deref(), Branch::Empty, true);
                    return self.nodes(empty, scope, out, mode, ctx, &inner);
                }
                let base = self.origin_of(scope, expr);
                let is_map = v.kind() == minijinja::value::ValueKind::Map;
                let inner = at.child(id.as_deref(), Branch::Children, true);
                let len = items.len();
                for (i, it) in items.into_iter().enumerate() {
                    let mut f: BTreeMap<String, Value> = BTreeMap::new();
                    let mut o: BTreeMap<String, Binding> = BTreeMap::new();
                    if names.len() >= 2 {
                        // Dictionnaire : clé, valeur.
                        let val = v.get_item(&it).unwrap_or(Value::UNDEFINED);
                        if let Some(b) = it
                            .as_str()
                            .and_then(|k| base.as_ref()?.extend(&[Seg::Key(k.to_string())]))
                        {
                            o.insert(names[1].to_string(), b);
                        }
                        f.insert(names[0].to_string(), it);
                        f.insert(names[1].to_string(), val);
                    } else {
                        if self.editor.is_some() {
                            let b = match &base {
                                Some(b) if !is_map => b.extend(&[Seg::Index(i)]),
                                Some(b) => it
                                    .as_str()
                                    .and_then(|k| b.extend(&[Seg::Key(k.to_string())])),
                                None => self.origin_of_value(&it),
                            };
                            if let Some(b) = b {
                                o.insert(names[0].to_string(), b);
                            }
                        }
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
                    scope.push_with(f, o);
                    let r = self.nodes(children, scope, out, mode, ctx, &inner);
                    scope.pop();
                    r?;
                }
            }
            Node::Set { name, expr } => {
                let v = self.eval(scope, expr, ctx)?;
                let origin = match path_expr(expr) {
                    Some(_) => self.origin_of(scope, expr),
                    None => self.origin_of_value(&v),
                };
                scope.set(name, v, origin);
            }
            Node::SetBlock { name, children, .. } => {
                // Contenu capturé dans une variable : sans repères.
                let mut s = String::new();
                self.nodes(children, scope, &mut s, mode, ctx, &At::NONE)?;
                scope.set(name, Value::from_safe_string(s), None);
            }
            Node::Slot => {
                if let Some(c) = scope.get("content") {
                    out.push_str(&display(&c, false));
                }
            }
            Node::Use { id, component } => match self.site.component(component) {
                Some(c) => {
                    let ctx = format!("{ctx} (composant {component})");
                    let mut body = |out: &mut String| {
                        scope.push(BTreeMap::new());
                        let inner = At {
                            mark: at.mark,
                            ..At::root(Owner::Component(c.id.clone()))
                        };
                        let r = self.nodes(&c.body, scope, out, mode, &ctx, &inner);
                        scope.pop();
                        r
                    };
                    match (marking, &at.owner, id) {
                        (true, Some(owner), Some(id)) => self.wrapped(
                            out,
                            Slot::Block {
                                owner: owner.clone(),
                                node: id.clone(),
                            },
                            body,
                        )?,
                        _ => body(out)?,
                    }
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
                    Some(html) => match (marking, &img.id) {
                        (true, Some(id)) => out.push_str(&with_marker(&html, id)),
                        _ => out.push_str(&html),
                    },
                    None => {
                        if let Some(c) = self.site.component("image-placeholder") {
                            let mut f: BTreeMap<String, Value> = BTreeMap::new();
                            f.insert("src".into(), Value::from(args.src.clone()));
                            f.insert("alt".into(), Value::from(args.alt.clone()));
                            f.insert("ratio".into(), Value::from(args.ratio.clone()));
                            f.insert("class".into(), Value::from(args.class.clone()));
                            let body = |out: &mut String| {
                                scope.push(f);
                                let inner = At {
                                    mark: at.mark,
                                    ..At::root(Owner::Component(c.id.clone()))
                                };
                                let r = self.nodes(&c.body, scope, out, mode, ctx, &inner);
                                scope.pop();
                                r
                            };
                            match (marking, &at.owner, &img.id) {
                                (true, Some(owner), Some(id)) => self.wrapped(
                                    out,
                                    Slot::Block {
                                        owner: owner.clone(),
                                        node: id.clone(),
                                    },
                                    body,
                                )?,
                                _ => body(out)?,
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Ajoute `data-rv` à la première balise d'un fragment HTML (`<picture …>`, `<img …>`).
fn with_marker(html: &str, id: &str) -> String {
    let Some(start) = html.find('<') else {
        return html.to_string();
    };
    let name_end = html[start + 1..]
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .map(|i| start + 1 + i);
    match name_end {
        Some(e) if e > start + 1 => format!(
            "{} data-rv=\"{}\"{}",
            &html[..e],
            escape_attr(id),
            &html[e..]
        ),
        _ => html.to_string(),
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
