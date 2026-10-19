//! Reconstruction d'un site dans la forme de l'interface (D-90) : un site écrit avec des gabarits
//! (Eleventy importé, site fait par un assistant ou à la main dans le code) devient un site tel
//! que les panneaux de Revel le construisent, au même rendu.
//!
//! Tout ce qui ne dépend que des données (variables, textes et listes de données, conditions
//! connues d'avance, attributs calculés) est évalué et écrit : textes, éléments, images, valeurs
//! des propriétés. Restent vivants seulement :
//! - les listes de collection, réécrites dans la forme réglée par le panneau
//!   ([`revel_edit::collection::Query`] : filtre, tri, nombre, contenus choisis) ;
//! - les champs du contenu dans une liste ou dans le modèle des pages d'une collection ;
//! - les valeurs d'un composant lues chez son utilisateur, changées en propriétés du composant
//!   (texte, image, lien, affichage) dont chaque utilisation donne la valeur ;
//! - ce qui dépend de la page dans un gabarit partagé.
//!
//! Ce qui ne peut pas prendre la forme de l'interface reste tel quel et paraît dans le relevé
//! ([`revel_edit::editable`]).

use crate::eval::{Builder, Scope, display};
use crate::{BuildInput, RenderError};
use minijinja::Value;
use revel_edit::collection::Query;
use revel_model::{Attr, Node, PropDef, PropKind, Site};
use std::collections::{BTreeMap, HashSet};

mod fuse;
pub use fuse::NewCollection;

/// Site reconstruit et ce qui a été fait.
pub struct Rebuilt {
    pub site: Site,
    pub notes: Vec<String>,
    /// Collections créées (listes de données lues par adresse) : contenus à écrire.
    pub collections: Vec<NewCollection>,
}

// ------------------------------------------------------------------------------------------------
// Lecture des expressions

/// Morceau d'expression : nom de variable (racine d'un chemin) et sa position.
#[derive(Debug, Clone, PartialEq)]
struct Ref {
    name: String,
    start: usize,
    end: usize,
    /// Fin du chemin (`a.b.c`, sans appel ni indice).
    path_end: usize,
}

const KEYWORDS: &[&str] = &[
    "and", "or", "not", "in", "is", "if", "else", "true", "false", "none", "True", "False", "None",
];

/// Variables lues par une expression (racines des chemins), dans l'ordre.
fn refs(expr: &str) -> Vec<Ref> {
    let b = expr.as_bytes();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0;
    // Dernier signe utile : `.` (attribut), `|` (filtre), `is` (test).
    let mut prev: Option<u8> = None;
    let mut prev_is = false;
    while i < n {
        let c = b[i];
        if c == b'"' || c == b'\'' {
            let mut j = i + 1;
            while j < n && b[j] != c {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            i = (j + 1).min(n);
            prev = Some(b'"');
            prev_is = false;
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < n && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let word = &expr[start..i];
            let mut k = i;
            while k < n && b[k] == b' ' {
                k += 1;
            }
            let next = b.get(k).copied();
            let kwarg = next == Some(b'=') && b.get(k + 1) != Some(&b'=');
            let call = next == Some(b'(');
            let skip = prev == Some(b'.')
                || prev == Some(b'|')
                || prev_is
                || kwarg
                || call
                || KEYWORDS.contains(&word)
                || (start > 0 && b[start - 1].is_ascii_digit());
            if !skip {
                // Fin du chemin : `.nom` répétés.
                let mut e = i;
                while e + 1 < n
                    && b[e] == b'.'
                    && (b[e + 1].is_ascii_alphabetic() || b[e + 1] == b'_')
                {
                    e += 1;
                    while e < n && (b[e].is_ascii_alphanumeric() || b[e] == b'_') {
                        e += 1;
                    }
                }
                out.push(Ref {
                    name: word.to_string(),
                    start,
                    end: i,
                    path_end: e,
                });
            }
            prev_is = word == "is" || (prev_is && word == "not");
            prev = Some(b'a');
            continue;
        }
        if !c.is_ascii_whitespace() {
            prev = Some(c);
            prev_is = false;
        }
        i += 1;
    }
    out
}

/// Expressions d'une valeur d'attribut écrite avec `{{ … }}` et `{% … %}`.
fn template_exprs(v: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = v;
    while let Some(i) = rest.find(['{']) {
        let after = &rest[i..];
        let close = if after.starts_with("{{") {
            "}}"
        } else if after.starts_with("{%") {
            "%}"
        } else {
            rest = &rest[i + 1..];
            continue;
        };
        let Some(j) = after[2..].find(close) else {
            break;
        };
        let inner = after[2..2 + j].trim().trim_matches('-').trim();
        // `{% if x %}` : la condition ; `{% endif %}`, `{% else %}` : rien.
        let inner = inner
            .strip_prefix("if ")
            .or_else(|| inner.strip_prefix("elif "))
            .unwrap_or(inner);
        if !matches!(inner, "endif" | "else") {
            out.push(inner.to_string());
        }
        rest = &after[2 + j + 2..];
    }
    out
}

/// Condition simplifiée : vérification d'existence d'un média retirée (`x and (x |
/// media_exists)` -> `x` ; un média du site existe).
fn simplify_cond(c: &str) -> String {
    let parts: Vec<String> = c.split(" and ").map(|p| p.trim().to_string()).collect();
    if parts.len() < 2 {
        return c.to_string();
    }
    let kept: Vec<String> = parts
        .into_iter()
        .filter(|p| {
            !p.trim_matches(|x| x == '(' || x == ')')
                .trim()
                .ends_with("media_exists")
        })
        .collect();
    if kept.is_empty() {
        c.to_string()
    } else {
        kept.join(" and ")
    }
}

/// Texte rendu sans les entités HTML (la valeur d'attribut écrite est échappée à la génération).
fn unescape(s: &str) -> String {
    s.replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#x2f;", "/")
        .replace("&amp;", "&")
}

/// Réécrit les expressions d'une valeur d'attribut (`{{ e }}`, `{% if e %}`) : `f(e, condition)`
/// donne la nouvelle expression, ou `None` pour la garder.
fn rewrite_template(v: &str, f: &mut dyn FnMut(&str, bool) -> Option<String>) -> String {
    let mut out = String::new();
    let mut rest = v;
    while let Some(i) = rest.find('{') {
        let after = &rest[i..];
        let (close, is_tag) = if after.starts_with("{{") {
            ("}}", false)
        } else if after.starts_with("{%") {
            ("%}", true)
        } else {
            out.push_str(&rest[..=i]);
            rest = &rest[i + 1..];
            continue;
        };
        let Some(j) = after[2..].find(close) else {
            break;
        };
        out.push_str(&rest[..i]);
        let inner = &after[2..2 + j];
        let t = inner.trim();
        let replaced = if is_tag {
            ["if ", "elif "].iter().find_map(|kw| {
                let e = t.strip_prefix(kw)?;
                f(e.trim(), true).map(|n| format!(" {kw}{n} "))
            })
        } else {
            f(t, false).map(|n| format!(" {n} "))
        };
        out.push_str(&after[..2]);
        out.push_str(replaced.as_deref().unwrap_or(inner));
        out.push_str(close);
        rest = &after[2 + j + 2..];
    }
    out.push_str(rest);
    out
}

/// Valeur écrite comme expression littérale (propriété, image) : texte entre guillemets, nombre,
/// booléen.
fn literal_of(v: &Value) -> Option<String> {
    use minijinja::value::ValueKind;
    match v.kind() {
        ValueKind::Bool => Some(v.is_true().to_string()),
        ValueKind::Number => Some(v.to_string()),
        ValueKind::Undefined | ValueKind::None => Some("\"\"".into()),
        ValueKind::String => Some(revel_edit::canvas::quote(&display(v, false))),
        // Liste, objet : gardés en expression.
        _ => None,
    }
}

// ------------------------------------------------------------------------------------------------
// Aplatissement

/// Noms jamais remplacés par leur valeur dans une expression vivante (la forme de l'interface
/// les garde : adresse de la page, langue, collections).
const KEEP: &[&str] = &[
    "page",
    "locale",
    "locales",
    "content",
    "loop",
    "collections",
];

struct Fl<'a, 'b> {
    b: &'a Builder<'b>,
    /// Noms connus d'avance (données du site, de la page, variables déjà évaluées).
    known: HashSet<String>,
    /// Noms qui varient (élément d'une liste de collection, propriété, valeur de la page dans un
    /// gabarit).
    dynamic: Vec<HashSet<String>>,
    /// Variables vivantes remplacées par leur expression là où elles servent.
    aliases: Vec<(String, String)>,
    taken: HashSet<String>,
    notes: Vec<String>,
    ctx: String,
    /// Variables affectées plusieurs fois ou dans une liste (accumulateurs) : gardées telles
    /// quelles.
    mutable: HashSet<String>,
    /// Dans `<head>` : écrit par Revel d'après les réglages (référencement), les informations
    /// du site y restent écrites comme avant.
    in_head: bool,
}

/// Variables gardées telles quelles : accumulateurs (affectation qui relit la variable) et
/// variables réaffectées dans une condition. Une variable affectée à chaque passage d'une liste
/// de données (`set l = ligne`) reste écrite passage par passage.
fn mutable_names(nodes: &[Node]) -> HashSet<String> {
    #[derive(Default)]
    struct Seen {
        count: BTreeMap<String, usize>,
        self_ref: HashSet<String>,
        in_if: HashSet<String>,
    }
    fn go(nodes: &[Node], in_if: bool, s: &mut Seen) {
        for n in nodes {
            match n {
                Node::Set { name, expr } => {
                    *s.count.entry(name.clone()).or_default() += 1;
                    if refs(expr).iter().any(|r| r.name == *name) {
                        s.self_ref.insert(name.clone());
                    }
                    if in_if {
                        s.in_if.insert(name.clone());
                    }
                }
                Node::SetBlock { name, children, .. } => {
                    *s.count.entry(name.clone()).or_default() += 1;
                    if in_if {
                        s.in_if.insert(name.clone());
                    }
                    go(children, in_if, s);
                }
                Node::Element { children, .. } => go(children, in_if, s),
                Node::If {
                    then, otherwise, ..
                } => {
                    go(then, true, s);
                    go(otherwise, true, s);
                }
                Node::Each {
                    children, empty, ..
                } => {
                    go(children, in_if, s);
                    go(empty, in_if, s);
                }
                _ => {}
            }
        }
    }
    let mut s = Seen::default();
    go(nodes, false, &mut s);
    s.count
        .into_iter()
        .filter(|(n, c)| *c > 1 && (s.self_ref.contains(n) || s.in_if.contains(n)))
        .map(|(n, _)| n)
        .collect()
}

/// Filtres propres au site remplacés par leur définition (`d | date_fr` ->
/// `(d) | date('%d.%m.%Y')`) : l'expression n'utilise plus que des filtres de Revel.
fn inline_filters(expr: &str, filters: &BTreeMap<String, revel_model::CustomFilter>) -> String {
    if filters.is_empty() || !expr.contains('|') {
        return expr.to_string();
    }
    let parts = split_top(expr, '|');
    if parts.len() < 2 {
        return expr.to_string();
    }
    let mut base = parts[0].clone();
    let mut changed = false;
    for (i, f) in parts.iter().enumerate().skip(1) {
        let (name, args) = match f.split_once('(') {
            Some((n, a)) if f.ends_with(')') => (n.trim(), split_top(&a[..a.len() - 1], ',')),
            _ => (f.trim(), Vec::new()),
        };
        // Écriture du site (filtre à une valeur) appliquée à une valeur reliée, en dernier (ou
        // suivie de `| safe`) : gardée, elle se choisit dans le panneau (« Prénom NOM »).
        let last = parts[i + 1..].iter().all(|r| r.trim() == "safe");
        let format = filters.get(name).is_some_and(|d| d.params.len() == 1)
            && args.is_empty()
            && last
            && revel_edit::visibility::is_path(
                base.trim().trim_start_matches('(').trim_end_matches(')'),
            );
        if format {
            base = format!("{} | {}", base.trim(), f.trim());
            continue;
        }
        match filters.get(name) {
            Some(def) => {
                let mut body = def.expr.clone();
                let mut values = vec![base.clone()];
                values.extend(args.into_iter().filter(|a| !a.is_empty()));
                for r in refs(&def.expr).into_iter().rev() {
                    if let Some(i) = def.params.iter().position(|p| *p == r.name)
                        && let Some(v) = values.get(i)
                    {
                        // Parenthèses seulement si la valeur en a besoin (`((a or b))` -> `(a or b)`).
                        let v = v.trim();
                        let wrapped = if revel_edit::visibility::is_path(v) || outer_parens(v) {
                            v.to_string()
                        } else {
                            format!("({v})")
                        };
                        body.replace_range(r.start..r.end, &wrapped);
                    }
                }
                base = body;
                changed = true;
            }
            None => base = format!("{base} | {f}"),
        }
    }
    if changed { base } else { expr.to_string() }
}

/// `(collections.x | sort(…)) | by_slugs(f)` : le tri ne change rien (contenus dans l'ordre des
/// adresses) ; écrit comme une liste de contenus reliés de l'interface.
fn simplify_refs(nodes: &mut [Node]) {
    let re = regex::Regex::new(
        r"^\(\s*collections\.([A-Za-z0-9_]+)\s*\|\s*sort\([^()]*\)\s*\)\s*\|\s*by_slugs\(\s*([A-Za-z0-9_.]+)\s*\)$",
    )
    .expect("regex");
    walk_mut(nodes, &mut |n| {
        if let Node::Each { expr, .. } = n
            && let Some(c) = re.captures(expr.trim())
        {
            *expr = format!("collections.{} | by_slugs({})", &c[1], &c[2]);
        }
    });
}

/// Contenus choisis écrits à la main, réécrits en liste de collection (avant l'aplatissement) :
/// - `for slug in L` puis `set it = collections.x | by_slug(slug)` et `if it` -> liste des
///   contenus `collections.x | by_slugs(L)` ;
/// - même boucle qui ajoute `it` à une liste (`set V = V + [it]`) -> `set V = collections.x |
///   by_slugs(L)` (la liste vide de départ retirée) ;
/// - `set ref = collections.x | by_slug(champ)` puis `if ref` -> liste du contenu relié
///   (`collections.x | by_slugs(champ)`).
fn rewrite_picks(nodes: &mut Vec<Node>) {
    // Source d'une recherche `… | by_slug(arg)` : (expression sans le dernier filtre, arg).
    fn by_slug(e: &str) -> Option<(String, String)> {
        let parts = split_top(e.trim(), '|');
        let last = parts.last()?.trim();
        let arg = last
            .strip_prefix("by_slug(")?
            .strip_suffix(')')?
            .trim()
            .to_string();
        let src = parts[..parts.len() - 1].join(" | ");
        refs(&src)
            .first()
            .is_some_and(|r| r.name == "collections")
            .then_some((src, arg))
    }
    fn significant(v: &[Node]) -> Vec<usize> {
        v.iter()
            .enumerate()
            .filter(|(_, n)| !matches!(n, Node::Text { text } if text.trim().is_empty()))
            .map(|(i, _)| i)
            .collect()
    }
    let mut i = 0;
    while i < nodes.len() {
        // Boucle sur une liste d'identifiants.
        if let Node::Each {
            id,
            expr,
            item,
            children,
            empty,
        } = &nodes[i]
            && empty.is_empty()
        {
            let idx = significant(children);
            if let [a, b] = idx.as_slice()
                && let Node::Set {
                    name: it,
                    expr: find,
                } = &children[*a]
                && let Node::If {
                    cond,
                    then,
                    otherwise,
                    ..
                } = &children[*b]
                && cond.trim() == it
                && otherwise.is_empty()
                && let Some((src, arg)) = by_slug(find)
                && arg == item.trim()
            {
                let list = format!("{src} | by_slugs({expr})");
                let t = significant(then);
                // Accumulation dans une liste.
                if let [k] = t.as_slice()
                    && let Node::Set {
                        name: acc,
                        expr: add,
                    } = &then[*k]
                {
                    let squash: String = add.chars().filter(|c| !c.is_whitespace()).collect();
                    if squash == format!("({acc}+[{it}])")
                        || squash == format!("{acc}+[{it}]")
                        || squash == format!("({acc}.push({it}),{acc})")
                    {
                        let acc = acc.clone();
                        nodes[i] = Node::Set {
                            name: acc.clone(),
                            expr: list,
                        };
                        // Liste vide de départ retirée.
                        if let Some(j) = nodes[..i].iter().rposition(|n| {
                            matches!(n, Node::Set { name, expr } if *name == acc && expr.trim() == "[]")
                        }) {
                            nodes.remove(j);
                            i -= 1;
                        }
                        i += 1;
                        continue;
                    }
                }
                nodes[i] = Node::Each {
                    id: id.clone(),
                    expr: list,
                    item: it.clone(),
                    children: then.clone(),
                    empty: Vec::new(),
                };
            }
        }
        // Contenu relié : `set ref = … by_slug(champ)` puis `if ref`.
        if let Node::Set { name, expr } = &nodes[i]
            && let Some((src, arg)) = by_slug(expr)
            && refs(&arg).len() == 1
        {
            let name = name.clone();
            let list = format!("{src} | by_slugs({arg})");
            let next = (i + 1..nodes.len())
                .find(|&j| !matches!(&nodes[j], Node::Text { text } if text.trim().is_empty()));
            if let Some(j) = next
                && let Node::If {
                    id,
                    cond,
                    then,
                    otherwise,
                } = &nodes[j]
                && cond.trim() == name
                && otherwise.is_empty()
            {
                nodes[j] = Node::Each {
                    id: id.clone(),
                    expr: list,
                    item: name,
                    children: then.clone(),
                    empty: Vec::new(),
                };
                nodes.remove(i);
                continue;
            }
        }
        i += 1;
    }
    for n in nodes.iter_mut() {
        for list in revel_model::ids::child_lists_mut(n) {
            rewrite_picks(list);
        }
    }
}

impl Fl<'_, '_> {
    /// Variables d'une liste vivante : elles masquent les variables de même nom remplacées
    /// par leur expression.
    fn enter(&mut self, vars: HashSet<String>) -> Vec<(String, String)> {
        let (hidden, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.aliases)
            .into_iter()
            .partition(|(n, _)| vars.contains(n));
        self.aliases = kept;
        self.dynamic.push(vars);
        hidden
    }

    fn leave(&mut self, hidden: Vec<(String, String)>) {
        self.dynamic.pop();
        self.aliases.extend(hidden);
    }

    fn is_dynamic(&self, name: &str) -> bool {
        self.mutable.contains(name)
            || self.dynamic.iter().any(|d| d.contains(name))
            || self.aliases.iter().any(|(n, _)| n == name)
    }

    /// Les informations du site (`site.…`) restent reliées : elles se règlent une fois pour
    /// tout le site (Paramètres › Informations du site) au lieu d'être recopiées partout.
    fn keeps_site(&self) -> bool {
        !self.in_head && self.known.contains("site") && !self.is_dynamic("site")
    }

    /// Texte relié à une information du site, dans la forme du panneau (filtres du site
    /// développés) ; `None` sinon.
    fn site_output(&self, expr: &str) -> Option<String> {
        if !self.keeps_site() {
            return None;
        }
        let e = bare_parens(&inline_filters(expr, &self.b.site_filters()));
        revel_edit::editable::is_site_output(&e).then_some(e)
    }

    /// Condition sur des informations du site (« Fax rempli ») dans la forme du panneau.
    fn site_condition(&self, cond: &str) -> bool {
        self.keeps_site()
            && revel_edit::visibility::read(cond).is_some_and(|c| {
                !c.clauses.is_empty()
                    && c.clauses
                        .iter()
                        .all(|k| revel_edit::editable::site_path(&k.field))
            })
    }

    /// Lien vers une information du site écrit dans une forme du panneau : numéro d'appel
    /// (`replace(' ', '') | …` -> `| tel`) et adresse d'un plan (`(rue + ' ' + ville) |
    /// urlencode` -> `adresse | map_query`) quand ils donnent la même adresse ; `None` sinon.
    fn site_href(&mut self, v: &str, scope: &Scope) -> Option<String> {
        if !self.keeps_site() {
            return None;
        }
        let v = v.trim();
        if revel_edit::editable::is_site_href(v) {
            return Some(v.to_string());
        }
        let exprs = template_exprs(v);
        let [e] = exprs.as_slice() else {
            return None;
        };
        let e = inline_filters(e, &self.b.site_filters());
        let site_refs: Vec<String> = refs(&e)
            .into_iter()
            .filter(|r| r.name == "site")
            .map(|r| e[r.start..r.path_end].to_string())
            .collect();
        if site_refs.is_empty() || refs(&e).iter().any(|r| r.name != "site") {
            return None;
        }
        let value = self.b.eval(scope, &e, &self.ctx).ok()?;
        let produced = display(&value, false);
        if produced.is_empty() {
            return None;
        }
        let (start, end) = (v.find("{{")?, v.rfind("}}")?);
        let (before, after) = (&v[..start], &v[end + 2..]);
        // Numéro d'appel.
        if site_refs.len() == 1 && before == "tel:" && after.is_empty() {
            let base = self.b.eval(scope, &site_refs[0], &self.ctx).ok()?;
            if crate::eval::tel_link(&display(&base, false)) == produced {
                return Some(format!("tel:{{{{ {} | tel }}}}", site_refs[0]));
            }
        }
        // Adresse d'un plan : chemin commun des valeurs lues (`site.address`).
        if before == revel_edit::editable::MAP_PREFIX && after.is_empty() {
            let parent = |p: &str| p.rsplit_once('.').map(|(a, _)| a.to_string());
            let common = parent(&site_refs[0])?;
            if site_refs
                .iter()
                .all(|r| parent(r).as_deref() == Some(common.as_str()))
                && common.contains('.')
            {
                let obj = self.b.eval(scope, &common, &self.ctx).ok()?;
                if crate::eval::map_query(&obj) == produced {
                    return Some(format!(
                        "{}{{{{ {common} | map_query }}}}",
                        revel_edit::editable::MAP_PREFIX
                    ));
                }
            }
        }
        None
    }

    /// Expression connue d'avance (aucune variable vivante ni collection).
    fn constant(&self, expr: &str) -> bool {
        refs(expr).iter().all(|r| {
            r.name != "collections" && !self.is_dynamic(&r.name) && self.known.contains(&r.name)
        })
    }

    /// Variables vivantes remplacées par leur expression ; chemins connus d'avance par leur
    /// valeur (`by_slugs(home.news)` -> `by_slugs(["a", "b"])`).
    fn inline(&self, expr: &str, scope: &Scope) -> String {
        let expr = &inline_filters(expr, &self.b.site_filters());
        let mut out = expr.to_string();
        for r in refs(expr).into_iter().rev() {
            if let Some((_, e)) = self.aliases.iter().rev().find(|(n, _)| *n == r.name) {
                out.replace_range(r.start..r.end, &format!("({e})"));
            } else if !KEEP.contains(&r.name.as_str())
                && !self.is_dynamic(&r.name)
                && self.known.contains(&r.name)
                && let Ok(v) = self.b.eval(scope, &expr[r.start..r.path_end], &self.ctx)
                && !v.is_undefined()
                && let Ok(json) = serde_json::to_string(&v)
            {
                out.replace_range(r.start..r.path_end, &format!("({json})"));
            }
        }
        out
    }

    fn eval(&mut self, scope: &Scope, expr: &str) -> Option<Value> {
        match self.b.eval(scope, expr, &self.ctx) {
            Ok(v) => Some(v),
            Err(e) => {
                self.notes.push(format!("{} : {e}", self.ctx));
                None
            }
        }
    }

    fn nodes(&mut self, nodes: &[Node], scope: &mut Scope) -> Vec<Node> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < nodes.len() {
            self.node(&nodes[i], scope, &mut out);
            i += 1;
        }
        out
    }

    /// Liste de collection « contenus choisis » écrite à la main : `for slug in liste` puis
    /// `set it = collections.x | by_slug(slug)` et `if it`.
    fn picks(&mut self, n: &Node, scope: &Scope) -> Option<Node> {
        let Node::Each {
            id,
            expr,
            item,
            children,
            ..
        } = n
        else {
            return None;
        };
        if !self.constant(expr) {
            return None;
        }
        let kids: Vec<&Node> = children
            .iter()
            .filter(|c| !matches!(c, Node::Text { text } if text.trim().is_empty()))
            .collect();
        let [
            Node::Set { name, expr: find },
            Node::If {
                cond,
                then,
                otherwise,
                ..
            },
        ] = kids.as_slice()
        else {
            return None;
        };
        if cond.trim() != name || !otherwise.is_empty() {
            return None;
        }
        let f: String = find.chars().filter(|c| !c.is_whitespace()).collect();
        let col = f
            .strip_prefix("collections.")?
            .strip_suffix(&format!("|by_slug({})", item.trim()))?
            .to_string();
        let list = self.b.eval(scope, expr, &self.ctx).ok()?;
        let pick: Vec<String> = list.try_iter().ok()?.map(|v| display(&v, false)).collect();
        let q = Query {
            collection: col,
            pick,
            ..Default::default()
        };
        Some(Node::Each {
            id: id.clone(),
            expr: q.to_expr(),
            item: name.clone(),
            children: then.clone(),
            empty: Vec::new(),
        })
    }

    /// Expression d'une liste de collection dans la forme du panneau, si possible.
    fn collection_list(&self, expr: &str) -> String {
        canonical_list(expr).unwrap_or_else(|| expr.trim().to_string())
    }

    fn attrs(&mut self, attrs: &[Attr], scope: &mut Scope) -> Vec<Attr> {
        let mut out = Vec::new();
        for a in attrs {
            let mut a = a.clone();
            if let Some(w) = a.when.clone() {
                let w = self.inline(&w, scope);
                if self.constant(&w) {
                    match self.eval(scope, &w) {
                        Some(v) if v.is_true() => a.when = None,
                        _ => continue,
                    }
                } else {
                    a.when = Some(w);
                }
            }
            if let Some(v) = a.value.clone()
                && (v.contains("{{") || v.contains("{%"))
                && matches!(a.name.as_str(), "href" | "src" | "poster")
                && let Some(kept) = self.site_href(&v, scope)
            {
                a.value = Some(kept);
            } else if let Some(v) = a.value.clone()
                && (v.contains("{{") || v.contains("{%"))
            {
                let exprs = template_exprs(&v);
                if exprs.iter().all(|e| self.constant(e)) {
                    match self.b.render_attr(scope, &v, &self.ctx) {
                        Ok(s) => a.value = Some(unescape(&s)),
                        Err(e) => self.notes.push(format!("{} : {e}", self.ctx)),
                    }
                } else {
                    // Variables vivantes remplacées dans chaque expression.
                    let mut s = v.clone();
                    for e in exprs {
                        let new = self.inline(&e, scope);
                        if new != e {
                            s = s.replacen(&e, &new, 1);
                        }
                    }
                    a.value = Some(s);
                }
            }
            out.push(a);
        }
        out
    }

    fn node(&mut self, n: &Node, scope: &mut Scope, out: &mut Vec<Node>) {
        match n {
            Node::Text { .. } | Node::Html { .. } | Node::Slot => out.push(n.clone()),
            Node::Element {
                id,
                tag,
                attrs,
                children,
            } => {
                let head = tag.eq_ignore_ascii_case("head");
                let was = self.in_head;
                self.in_head |= head;
                let attrs = self.attrs(attrs, scope);
                let children = self.nodes(children, scope);
                self.in_head = was;
                out.push(Node::Element {
                    id: id.clone(),
                    tag: tag.clone(),
                    attrs,
                    children,
                });
            }
            Node::Out { expr } => {
                if let Some(kept) = self.site_output(expr) {
                    out.push(Node::Out { expr: kept });
                    return;
                }
                let e = self.inline(expr, scope);
                if !self.constant(&e) {
                    out.push(Node::Out { expr: e });
                    return;
                }
                let Some(v) = self.eval(scope, &e) else {
                    out.push(Node::Out { expr: e });
                    return;
                };
                if v.is_undefined() || v.is_none() {
                    return;
                }
                let s = display(&v, false);
                if s.is_empty() {
                    return;
                }
                if v.is_safe() && (s.contains('<') || s.contains('&')) {
                    if s.contains("{{") || s.contains("{%") {
                        out.push(Node::Html { html: s });
                        return;
                    }
                    match revel_markup::parse(&s, &revel_markup::Options::default()) {
                        Ok(p) => {
                            let mut nodes = p.nodes;
                            for x in &mut nodes {
                                revel_edit::make::refresh_ids(x, &mut self.taken);
                            }
                            out.extend(nodes);
                        }
                        Err(_) => out.push(Node::Html { html: s }),
                    }
                } else {
                    out.push(Node::Text { text: s });
                }
            }
            Node::If {
                id,
                cond,
                then,
                otherwise,
            } => {
                if self.site_condition(cond) {
                    let then = self.nodes(then, scope);
                    let otherwise = self.nodes(otherwise, scope);
                    out.push(Node::If {
                        id: id.clone(),
                        cond: simplify_cond(cond),
                        then,
                        otherwise,
                    });
                    return;
                }
                let c = simplify_cond(&self.inline(cond, scope));
                if self.constant(&c)
                    && let Some(v) = self.eval(scope, &c)
                {
                    let branch = if v.is_true() { then } else { otherwise };
                    let nodes = self.nodes(branch, scope);
                    out.extend(nodes);
                    return;
                }
                let then = self.nodes(then, scope);
                let otherwise = self.nodes(otherwise, scope);
                out.push(Node::If {
                    id: id.clone(),
                    cond: c,
                    then,
                    otherwise,
                });
            }
            Node::Each {
                id,
                expr,
                item,
                children,
                empty,
            } => {
                if let Some(Node::Each {
                    id,
                    expr,
                    item,
                    children,
                    empty,
                }) = self.picks(n, scope)
                {
                    self.notes.push(format!(
                        "{} : contenus choisis -> liste de collection",
                        self.ctx
                    ));
                    let hidden =
                        self.enter([item.clone(), "loop".to_string()].into_iter().collect());
                    let children = self.nodes(&children, scope);
                    self.leave(hidden);
                    out.push(Node::Each {
                        id,
                        expr,
                        item,
                        children,
                        empty,
                    });
                    return;
                }
                let e = self.inline(expr, scope);
                let vars: HashSet<String> = item
                    .split(',')
                    .map(|v| v.trim().to_string())
                    .chain(["loop".to_string()])
                    .collect();
                // Liste de collection : gardée vivante, dans la forme du panneau.
                if refs(&e).iter().any(|r| r.name == "collections") {
                    let e = self.collection_list(&e);
                    let hidden = self.enter(vars);
                    let children = self.nodes(children, scope);
                    let empty = self.nodes(empty, scope);
                    self.leave(hidden);
                    out.push(Node::Each {
                        id: id.clone(),
                        expr: e,
                        item: item.clone(),
                        children,
                        empty,
                    });
                    return;
                }
                if self.constant(&e)
                    && let Some(list) = self.eval(scope, &e)
                {
                    // Liste de données : un élément écrit par valeur.
                    let values: Vec<Value> = if list.is_undefined() || list.is_none() {
                        Vec::new()
                    } else {
                        list.try_iter().map(|i| i.collect()).unwrap_or_default()
                    };
                    if values.is_empty() {
                        let nodes = self.nodes(empty, scope);
                        out.extend(nodes);
                        return;
                    }
                    let names: Vec<String> =
                        item.split(',').map(|v| v.trim().to_string()).collect();
                    let len = values.len();
                    for (i, v) in values.into_iter().enumerate() {
                        let mut frame: BTreeMap<String, Value> = BTreeMap::new();
                        if names.len() == 2 {
                            // Dictionnaire : `cle, valeur`.
                            frame.insert(names[0].clone(), v.clone());
                            frame.insert(
                                names[1].clone(),
                                list.get_item(&v).unwrap_or(Value::UNDEFINED),
                            );
                        } else {
                            frame.insert(names[0].clone(), v);
                        }
                        let mut lp: BTreeMap<String, Value> = BTreeMap::new();
                        lp.insert("index".into(), Value::from(i + 1));
                        lp.insert("index0".into(), Value::from(i));
                        lp.insert("first".into(), Value::from(i == 0));
                        lp.insert("last".into(), Value::from(i + 1 == len));
                        lp.insert("length".into(), Value::from(len));
                        lp.insert("revindex".into(), Value::from(len - i));
                        frame.insert("loop".into(), Value::from(lp));
                        for k in frame.keys() {
                            self.known.insert(k.clone());
                        }
                        scope.push(frame);
                        let mut nodes = self.nodes(children, scope);
                        scope.pop();
                        if i > 0 {
                            for x in &mut nodes {
                                revel_edit::make::refresh_ids(x, &mut self.taken);
                            }
                        }
                        out.extend(nodes);
                    }
                    self.notes
                        .push(format!("{} : liste de données écrite ({len})", self.ctx));
                    return;
                }
                let hidden = self.enter(vars);
                let children = self.nodes(children, scope);
                let empty = self.nodes(empty, scope);
                self.leave(hidden);
                out.push(Node::Each {
                    id: id.clone(),
                    expr: e,
                    item: item.clone(),
                    children,
                    empty,
                });
            }
            Node::Set { name, expr } => {
                let e = self.inline(expr, scope);
                // Variable mutable ou qui se relit elle-même (`set c = couleurs[c]`) : gardée.
                if self.mutable.contains(name) || refs(expr).iter().any(|r| r.name == *name) {
                    self.mutable.insert(name.clone());
                    out.push(Node::Set {
                        name: name.clone(),
                        expr: e,
                    });
                    return;
                }
                let live = refs(&e).iter().any(|r| r.name == "collections") || !self.constant(&e);
                if live {
                    // Gardée aussi en place : un composant utilisé ensuite peut la lire.
                    out.push(Node::Set {
                        name: name.clone(),
                        expr: e.clone(),
                    });
                    self.aliases.push((name.clone(), e));
                    return;
                }
                match self.eval(scope, &e) {
                    Some(v) => {
                        scope.set(name, v, None);
                        self.known.insert(name.clone());
                        self.aliases.retain(|(n, _)| n != name);
                    }
                    None => self.aliases.push((name.clone(), e)),
                }
            }
            Node::SetBlock { id, name, children } => {
                let mut all_const = true;
                revel_model::ids::walk(children, &mut |x| {
                    let exprs: Vec<String> = match x {
                        Node::Out { expr } | Node::Set { expr, .. } | Node::Each { expr, .. } => {
                            vec![expr.clone()]
                        }
                        Node::If { cond, .. } => vec![cond.clone()],
                        _ => Vec::new(),
                    };
                    for e in exprs {
                        if !self.constant(&e) {
                            all_const = false;
                        }
                    }
                });
                if all_const && let Ok(html) = self.b.render_html(children, scope, &self.ctx) {
                    scope.set(name, Value::from_safe_string(html), None);
                    self.known.insert(name.clone());
                    return;
                }
                let children = self.nodes(children, scope);
                out.push(Node::SetBlock {
                    id: id.clone(),
                    name: name.clone(),
                    children,
                });
            }
            Node::Use {
                id,
                component,
                args,
                variant,
                attrs,
            } => {
                let mut new_args = BTreeMap::new();
                for (k, e) in args {
                    let e2 = self.inline(e, scope);
                    let v = if self.constant(&e2)
                        && revel_edit::canvas::literal(&e2).is_none()
                        && let Some(v) = self.eval(scope, &e2)
                        && let Some(l) = literal_of(&v)
                    {
                        l
                    } else {
                        e2
                    };
                    new_args.insert(k.clone(), v);
                }
                let attrs = self.attrs(attrs, scope);
                out.push(Node::Use {
                    id: id.clone(),
                    component: component.clone(),
                    args: new_args,
                    variant: variant.clone(),
                    attrs,
                });
            }
            Node::Image(img) => {
                let mut img = img.clone();
                let fix = |e: &mut String, me: &mut Self| {
                    let e2 = me.inline(e, scope);
                    *e = if me.constant(&e2)
                        && revel_edit::canvas::literal(&e2).is_none()
                        && let Some(v) = me.eval(scope, &e2)
                        && let Some(l) = literal_of(&v)
                    {
                        l
                    } else {
                        e2
                    };
                };
                fix(&mut img.src, self);
                for f in [
                    &mut img.alt,
                    &mut img.sizes,
                    &mut img.class,
                    &mut img.ratio,
                    &mut img.loading,
                    &mut img.widths,
                ] {
                    if let Some(e) = f.as_mut() {
                        fix(e, self);
                    }
                }
                out.push(Node::Image(img));
            }
        }
    }
}

/// Découpe `s` sur `sep` hors des guillemets, parenthèses et crochets.
fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut esc = false;
    for c in s.chars() {
        if let Some(q) = quote {
            cur.push(c);
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == q {
                quote = None;
            }
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
            c if c == sep && depth == 0 => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter().map(|p| p.trim().to_string()).collect()
}

/// `(…)` entoure toute l'expression.
fn outer_parens(s: &str) -> bool {
    let s = s.trim();
    if !(s.starts_with('(') && s.ends_with(')')) {
        return false;
    }
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && i + 1 < s.len() {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

/// Suite des filtres d'une expression (`(a | b) | c` -> `[a, b, c]`) ; `(…)[:n]` -> `limit(n)`.
fn pipeline(e: &str) -> Vec<String> {
    let mut e = e.trim().to_string();
    let mut tail = Vec::new();
    if e.starts_with('(')
        && let Some(i) = e.rfind(")[:")
        && e.ends_with(']')
        && outer_parens(&e[..=i])
    {
        tail.push(format!("limit({})", e[i + 3..e.len() - 1].trim()));
        e = e[..=i].to_string();
    }
    while outer_parens(&e) {
        e = e.trim()[1..e.trim().len() - 1].trim().to_string();
    }
    let parts = split_top(&e, '|');
    let mut out = match parts.first() {
        Some(first) if outer_parens(first) => pipeline(first),
        Some(first) => vec![first.clone()],
        None => Vec::new(),
    };
    out.extend(parts.into_iter().skip(1));
    out.extend(tail);
    out
}

/// Liste de collection écrite dans la forme du panneau : contenus choisis, filtre, sans la page
/// affichée, tri, nombre. `None` : forme que le panneau ne règle pas.
fn canonical_list(expr: &str) -> Option<String> {
    let parts = pipeline(expr);
    let (base, filters) = parts.split_first()?;
    base.strip_prefix("collections.")?;
    let mut pick = None;
    let mut select = None;
    let mut contains = None;
    let mut without = None;
    let mut sort = None;
    let mut limit = None;
    for f in filters {
        let name = f.split('(').next().unwrap_or("").trim();
        let slot = match name {
            "by_slugs" => &mut pick,
            "selectattr" => &mut select,
            "where_contains" => &mut contains,
            "without" => &mut without,
            "sort" => &mut sort,
            "limit" => &mut limit,
            _ => return None,
        };
        if slot.is_some() {
            return None;
        }
        *slot = Some(f.clone());
    }
    // Contenus choisis à la main : leur ordre l'emporte sur un tri.
    if pick.is_some() {
        sort = None;
    }
    let mut e = base.clone();
    for f in [pick, select, contains, without, sort]
        .into_iter()
        .flatten()
    {
        e.push_str(" | ");
        e.push_str(&f);
    }
    if let Some(l) = limit {
        let n = l.trim_start_matches("limit(").trim_end_matches(')').trim();
        e = format!("({e})[:{n}]");
    }
    Query::parse(&e).map(|q| q.to_expr())
}

/// Expressions d'une liste de nœuds (valeurs, conditions, listes, attributs, propriétés).
fn exprs_of(nodes: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    revel_model::ids::walk(nodes, &mut |n| match n {
        Node::Out { expr } | Node::Set { expr, .. } | Node::Each { expr, .. } => {
            out.push(expr.clone())
        }
        Node::If { cond, .. } => out.push(cond.clone()),
        Node::Element { attrs, .. } => {
            for a in attrs {
                out.extend(a.value.as_deref().map(template_exprs).unwrap_or_default());
                out.extend(a.when.clone());
            }
        }
        Node::Use { attrs, args, .. } => {
            for a in attrs {
                out.extend(a.value.as_deref().map(template_exprs).unwrap_or_default());
                out.extend(a.when.clone());
            }
            out.extend(args.values().cloned());
        }
        Node::Image(img) => {
            out.push(img.src.clone());
            for e in [
                &img.alt,
                &img.sizes,
                &img.class,
                &img.ratio,
                &img.loading,
                &img.widths,
            ]
            .into_iter()
            .flatten()
            {
                out.push(e.clone());
            }
        }
        _ => {}
    });
    out
}

/// Retire les affectations dont la variable n'est plus lue (`keep(nom)` : encore lue).
fn prune_sets(nodes: &mut Vec<Node>, keep: &dyn Fn(&str) -> bool) {
    nodes.retain(|n| !matches!(n, Node::Set { name, .. } if !keep(name)));
    for n in nodes.iter_mut() {
        for list in revel_model::ids::child_lists_mut(n) {
            prune_sets(list, keep);
        }
    }
}

/// Parcourt les nœuds (et leurs enfants) en les modifiant.
pub(crate) fn walk_mut(nodes: &mut [Node], f: &mut dyn FnMut(&mut Node)) {
    for n in nodes {
        f(n);
        for list in revel_model::ids::child_lists_mut(n) {
            walk_mut(list, f);
        }
    }
}

// ------------------------------------------------------------------------------------------------
// Propriétés déduites

/// Nature de l'endroit où une valeur lue chez l'utilisateur sert dans le composant.
fn prop_name(expr: &str, taken: &HashSet<String>) -> String {
    let base = refs(expr)
        .into_iter()
        .map(|r| {
            expr[r.start..r.path_end]
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_string()
        })
        .rfind(|s| !s.is_empty() && s != "data")
        .unwrap_or_else(|| "valeur".into());
    let base: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let base = if revel_model::valid_prop_name(&base) {
        base
    } else {
        format!("p_{base}")
    };
    let mut name = base.clone();
    let mut k = 2;
    while taken.contains(&name) {
        name = format!("{base}{k}");
        k += 1;
    }
    name
}

/// Valeurs qu'un composant lit chez son utilisateur (variables qui ne sont ni ses propriétés,
/// ni ses variables, ni des données du site) changées en propriétés : chaque utilisation donne
/// l'expression lue, que le panneau relie au champ du contenu.
fn deduce_props(site: &mut Site, globals: &HashSet<String>, notes: &mut Vec<String>) {
    let ids: Vec<String> = site.components.iter().map(|c| c.id.clone()).collect();
    for cid in ids {
        let Some(ci) = site.components.iter().position(|c| c.id == cid) else {
            continue;
        };
        let mut c = site.components[ci].clone();
        let own: HashSet<String> = c.props.iter().map(|p| p.name.clone()).collect();
        // Expression -> (propriété, nature).
        let mut found: Vec<(String, String, PropKind)> = Vec::new();
        let mut taken: HashSet<String> = own.clone();
        // Variables du composant lui-même (`{% set %}`) : pas lues chez l'utilisateur.
        let mut sets: HashSet<String> = HashSet::new();
        revel_model::ids::walk(&c.body, &mut |n| {
            if let Node::Set { name, .. } | Node::SetBlock { name, .. } = n {
                sets.insert(name.clone());
            }
        });
        let outside = |name: &str, local: &HashSet<String>| {
            !own.contains(name)
                && !sets.contains(name)
                && !local.contains(name)
                && !globals.contains(name)
                && !KEEP.contains(&name)
        };
        // Listes parcourues dans le composant et lues chez l'utilisateur (`item.data.expertises`,
        // `blocks`) : propriétés « liste », reliées à chaque utilisation.
        let mut list_paths: Vec<String> = Vec::new();
        walk_body(&mut c.body.clone(), &mut Vec::new(), &mut |n, local| {
            if let Node::Each { expr, .. } = n {
                for r in refs(expr) {
                    let path = expr[r.start..r.path_end].to_string();
                    if outside(&r.name, local) && !list_paths.contains(&path) {
                        list_paths.push(path);
                    }
                }
            }
        });
        let mut lists: Vec<(String, String)> = Vec::new();
        for path in &list_paths {
            let p = prop_name(path, &taken);
            taken.insert(p.clone());
            lists.push((path.clone(), p));
        }
        // Chemin d'une liste remplacé par sa propriété (sources de boucles, conditions).
        let subst = |e: &str, local: &HashSet<String>| -> Option<String> {
            let mut out = String::new();
            let mut at = 0;
            for r in refs(e) {
                if !outside(&r.name, local) {
                    continue;
                }
                if let Some((_, p)) = lists
                    .iter()
                    .find(|(path, _)| *path == e[r.start..r.path_end])
                {
                    out.push_str(&e[at..r.start]);
                    out.push_str(p);
                    at = r.path_end;
                }
            }
            (at > 0).then(|| {
                out.push_str(&e[at..]);
                out
            })
        };
        let list_names: HashSet<String> = lists.iter().map(|(_, p)| p.clone()).collect();
        // Expression lue chez l'utilisateur ; pas celle qui lit un objet dont une liste est
        // parcourue (l'objet entier reste lu chez l'utilisateur).
        let external = |e: &str, local: &HashSet<String>| {
            let rs = refs(e);
            rs.iter()
                .any(|r| outside(&r.name, local) && !list_names.contains(&r.name))
                && !rs.iter().any(|r| {
                    let path = &e[r.start..r.path_end];
                    outside(&r.name, local)
                        && list_paths
                            .iter()
                            .any(|l| l.starts_with(&format!("{path}.")))
                })
        };
        fn walk_body(
            nodes: &mut [Node],
            local: &mut Vec<HashSet<String>>,
            f: &mut dyn FnMut(&mut Node, &HashSet<String>),
        ) {
            for n in nodes {
                let flat: HashSet<String> = local.iter().flatten().cloned().collect();
                f(n, &flat);
                match n {
                    Node::Element { children, .. } | Node::SetBlock { children, .. } => {
                        walk_body(children, local, f)
                    }
                    Node::If {
                        then, otherwise, ..
                    } => {
                        walk_body(then, local, f);
                        walk_body(otherwise, local, f);
                    }
                    Node::Each {
                        item,
                        children,
                        empty,
                        ..
                    } => {
                        local.push(
                            item.split(',')
                                .map(|v| v.trim().to_string())
                                .chain(["loop".to_string()])
                                .collect(),
                        );
                        walk_body(children, local, f);
                        walk_body(empty, local, f);
                        local.pop();
                    }
                    _ => {}
                }
            }
        }
        let mut prop_for =
            |e: &str, kind: PropKind, found: &mut Vec<(String, String, PropKind)>| {
                // `x or ""` : vide si absent, comme `x` (valeur reliée telle quelle).
                let t = e.trim();
                let key = t
                    .strip_suffix("or \"\"")
                    .or_else(|| t.strip_suffix("or ''"))
                    .map(str::trim)
                    .filter(|h| revel_edit::visibility::is_path(h))
                    .unwrap_or(t)
                    .to_string();
                // `x or 'Vidéo'` : propriété `x`, le texte de repli reste dans le composant.
                let (key, fallback) = match split_kw_top(&key, " or ") {
                    Some((h, lit))
                        if revel_edit::visibility::is_path(h.trim())
                            && revel_edit::canvas::literal(lit.trim()).is_some() =>
                    {
                        (h.trim().to_string(), Some(lit.trim().to_string()))
                    }
                    _ => (key, None),
                };
                let with = |p: &str| match &fallback {
                    Some(l) => format!("{p} or {l}"),
                    None => p.to_string(),
                };
                if let Some((_, p, k)) = found.iter_mut().find(|(x, _, _)| *x == key) {
                    // Lu comme condition et comme valeur : une valeur (vraie si remplie).
                    if *k == PropKind::Switch && kind != PropKind::Switch {
                        *k = kind;
                    }
                    return with(p);
                }
                let p = prop_name(&key, &taken);
                taken.insert(p.clone());
                found.push((key, p.clone(), kind));
                with(&p)
            };
        // Variables du composant parcourues par une boucle : leur valeur est une liste.
        let mut iterated_sets: HashSet<String> = HashSet::new();
        revel_model::ids::walk(&c.body, &mut |n| {
            if let Node::Each { expr, .. } = n {
                for r in refs(expr) {
                    if sets.contains(&r.name) {
                        iterated_sets.insert(r.name.clone());
                    }
                }
            }
        });
        let mut body = std::mem::take(&mut c.body);
        if !lists.is_empty() {
            walk_body(&mut body, &mut Vec::new(), &mut |n, local| match n {
                Node::Each { expr, .. } => {
                    if let Some(e) = subst(expr, local) {
                        *expr = e;
                    }
                }
                Node::If { cond, .. } => {
                    if let Some(e) = subst(cond, local) {
                        *cond = e;
                    }
                }
                Node::Element { attrs, .. } => {
                    for a in attrs.iter_mut() {
                        if let Some(w) = a.when.as_deref().and_then(|w| subst(w, local)) {
                            a.when = Some(w);
                        }
                    }
                }
                _ => {}
            });
        }
        // Condition `not x` : propriété `x` (interrupteur), la négation reste dans le composant
        // (« affiché si la propriété est désactivée »).
        let negated = |e: &str| {
            e.trim()
                .strip_prefix("not ")
                .map(str::trim)
                .filter(|x| revel_edit::visibility::is_path(x))
                .map(str::to_string)
        };
        walk_body(&mut body, &mut Vec::new(), &mut |n, local| match n {
            Node::Element { tag, attrs, .. } => {
                for a in attrs.iter_mut() {
                    if let Some(w) = a.when.clone()
                        && external(&w, local)
                    {
                        a.when = Some(match negated(&w) {
                            Some(x) => {
                                format!("not {}", prop_for(&x, PropKind::Switch, &mut found))
                            }
                            None => prop_for(&w, PropKind::Switch, &mut found),
                        });
                    }
                    let Some(v) = a.value.clone() else { continue };
                    if !v.contains("{{") && !v.contains("{%") {
                        continue;
                    }
                    let whole = v
                        .trim()
                        .strip_prefix("{{")
                        .and_then(|x| x.strip_suffix("}}"))
                        .filter(|x| !x.contains("{{"))
                        .map(|x| x.trim().to_string());
                    let kind = match (&whole, a.name.as_str()) {
                        (Some(_), "href") if tag.eq_ignore_ascii_case("a") => PropKind::Link,
                        (Some(_), "src" | "poster") => PropKind::Image,
                        _ => PropKind::Text,
                    };
                    a.value = Some(rewrite_template(&v, &mut |e, cond| {
                        external(e, local).then(|| {
                            prop_for(e, if cond { PropKind::Switch } else { kind }, &mut found)
                        })
                    }));
                }
            }
            Node::Out { expr } if external(expr, local) => {
                *expr = prop_for(expr, PropKind::Text, &mut found);
            }
            // Variable du composant calculée d'après une valeur de l'utilisateur
            // (`{% set vid = video or "" %}`) : chaque valeur lue devient une propriété.
            Node::Set { expr, name } => {
                let iterated = iterated_sets.contains(name.as_str());
                let mut out = String::new();
                let mut at = 0;
                for r in refs(expr) {
                    let path = expr[r.start..r.path_end].to_string();
                    if !outside(&r.name, local)
                        || list_names.contains(&r.name)
                        || list_paths
                            .iter()
                            .any(|l| l.starts_with(&format!("{path}.")))
                    {
                        continue;
                    }
                    let kind = if iterated {
                        PropKind::List
                    } else {
                        PropKind::Text
                    };
                    out.push_str(&expr[at..r.start]);
                    out.push_str(&prop_for(&path, kind, &mut found));
                    at = r.path_end;
                }
                if at > 0 {
                    out.push_str(&expr[at..]);
                    *expr = out;
                }
            }
            Node::If { cond, .. } if external(cond, local) => {
                *cond = match negated(cond) {
                    Some(x) => format!("not {}", prop_for(&x, PropKind::Switch, &mut found)),
                    None => prop_for(cond, PropKind::Switch, &mut found),
                };
            }
            Node::Image(img) => {
                if external(&img.src, local) {
                    img.src = prop_for(&img.src, PropKind::Image, &mut found);
                }
                if let Some(alt) = img.alt.as_mut()
                    && external(alt, local)
                {
                    *alt = prop_for(alt, PropKind::Text, &mut found);
                }
            }
            Node::Use { args, .. } => {
                for e in args.values_mut() {
                    if external(e, local) {
                        *e = prop_for(e, PropKind::Text, &mut found);
                    }
                }
            }
            _ => {}
        });
        c.body = body;
        for (path, p) in &lists {
            found.push((path.clone(), p.clone(), PropKind::List));
        }
        if found.is_empty() {
            continue;
        }
        for (_, p, kind) in &found {
            c.props.push(PropDef {
                name: p.clone(),
                label: humanize(p),
                kind: *kind,
                default: None,
            });
        }
        notes.push(format!(
            "composant {} : {} propriété(s) déduite(s)",
            c.id,
            found.len()
        ));
        site.components[ci] = c;
        // Chaque utilisation donne la valeur lue.
        let args: Vec<(String, String)> = found
            .iter()
            .map(|(e, p, _)| (p.clone(), e.clone()))
            .collect();
        let fix = |nodes: &mut Vec<Node>| {
            walk_mut(nodes, &mut |n| {
                if let Node::Use {
                    component, args: a, ..
                } = n
                    && *component == cid
                {
                    for (p, e) in &args {
                        a.entry(p.clone()).or_insert_with(|| e.clone());
                    }
                }
            });
        };
        for p in &mut site.pages {
            fix(&mut p.body);
        }
        for l in &mut site.layouts {
            fix(&mut l.body);
        }
        for k in 0..site.components.len() {
            let mut b = std::mem::take(&mut site.components[k].body);
            fix(&mut b);
            site.components[k].body = b;
        }
    }
}

// ------------------------------------------------------------------------------------------------

/// `{% for e in <liste> %}{% if e.url != page.url %}…{% endif %}{% endfor %}` (sans nombre
/// maximal) : liste de collection réglée « sans le contenu affiché ».
fn exclude_current_ifs(site: &mut Site, notes: &mut Vec<String>) {
    let mut n = 0usize;
    let mut fix = |body: &mut Vec<Node>| {
        walk_mut(body, &mut |node| {
            let Node::Each {
                expr,
                item,
                children,
                empty,
                ..
            } = node
            else {
                return;
            };
            let Some(mut q) = Query::parse(expr) else {
                return;
            };
            if q.limit.is_some() || q.exclude_current || !empty.is_empty() || q.first.is_some() {
                return;
            }
            let idx: Vec<usize> = children
                .iter()
                .enumerate()
                .filter(|(_, c)| !matches!(c, Node::Text { text } if text.trim().is_empty()))
                .map(|(i, _)| i)
                .collect();
            let [i] = idx.as_slice() else {
                return;
            };
            let Node::If {
                cond,
                then,
                otherwise,
                ..
            } = &children[*i]
            else {
                return;
            };
            let it = item.trim();
            let c: String = cond.split_whitespace().collect::<Vec<_>>().join(" ");
            let ok = c == format!("{it}.url != page.url") || c == format!("page.url != {it}.url");
            let uses_loop = exprs_of(then)
                .iter()
                .any(|e| refs(e).iter().any(|r| r.name == "loop"));
            if !ok || !otherwise.is_empty() || uses_loop {
                return;
            }
            let then = then.clone();
            q.exclude_current = true;
            *expr = q.to_expr();
            *children = then;
            n += 1;
        });
    };
    for l in &mut site.layouts {
        fix(&mut l.body);
    }
    for p in &mut site.pages {
        fix(&mut p.body);
    }
    for c in &mut site.components {
        fix(&mut c.body);
    }
    if n > 0 {
        notes.push(format!(
            "{n} liste(s) qui sautaient le contenu affiché : réglage « sans le contenu affiché »"
        ));
    }
}

/// `style="--dot:{{ {"orange": "#F4684D", …}[ex.data.color] }}"` (valeur choisie dans un tableau
/// selon un champ) : une classe par valeur du champ, ajoutée sous condition (« Classe selon un
/// champ »), et sa règle de style (`.tag--orange { --dot: #F4684D }`), réglable dans Styles.
fn map_styles(site: &mut Site, notes: &mut Vec<String>) {
    let style_re =
        regex::Regex::new(r"^\s*(--[A-Za-z0-9_-]+|[a-z-]+)\s*:\s*\{\{-?\s*(.+?)\s*-?\}\}\s*;?\s*$")
            .expect("regex");
    let map_re =
        regex::Regex::new(r"^\(?\s*(\{.*\})\s*\)?\s*\[\s*([A-Za-z_][A-Za-z0-9_.-]*)\s*\]$")
            .expect("regex");
    let slug = |k: &str| -> String {
        let mut out = String::new();
        for c in k.chars() {
            if c.is_ascii_uppercase() {
                if !out.is_empty() {
                    out.push('-');
                }
                out.push(c.to_ascii_lowercase());
            } else if c.is_ascii_alphanumeric() {
                out.push(c);
            } else if !out.ends_with('-') {
                out.push('-');
            }
        }
        out.trim_matches('-').to_string()
    };
    let mut rules: Vec<(String, String, String)> = Vec::new();
    let mut n = 0usize;
    let mut fix = |body: &mut Vec<Node>| {
        walk_mut(body, &mut |node| {
            let Node::Element { attrs, .. } = node else {
                return;
            };
            let Some(si) = attrs
                .iter()
                .position(|a| a.name == "style" && a.when.is_none() && a.value.is_some())
            else {
                return;
            };
            let v = attrs[si].value.clone().unwrap_or_default();
            let Some(c) = style_re.captures(&v) else {
                return;
            };
            let (prop, expr) = (c[1].to_string(), c[2].to_string());
            let Some(m) = map_re.captures(expr.trim()) else {
                return;
            };
            let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&m[1])
            else {
                return;
            };
            let path = m[2].to_string();
            if map.is_empty() || !map.values().all(|v| v.is_string()) {
                return;
            }
            let class = attrs
                .iter()
                .find(|a| a.name == "class" && a.when.is_none())
                .and_then(|a| a.value.clone())
                .unwrap_or_default();
            if class.contains("{{") {
                return;
            }
            let base =
                crate::style_base(&class).unwrap_or_else(|| slug(prop.trim_start_matches('-')));
            let mut value = class.clone();
            for (k, val) in &map {
                let cls = format!("{base}--{}", slug(k));
                value.push_str(&format!(
                    "{{% if {path} == {} %}} {cls}{{% endif %}}",
                    serde_json::to_string(k).unwrap_or_default()
                ));
                let decl_value = val.as_str().unwrap_or_default().to_string();
                if !rules.iter().any(|(s, _, _)| *s == format!(".{cls}")) {
                    rules.push((format!(".{cls}"), prop.clone(), decl_value));
                }
            }
            attrs.remove(si);
            match attrs
                .iter_mut()
                .find(|a| a.name == "class" && a.when.is_none())
            {
                Some(a) => a.value = Some(value.trim_start().to_string()),
                None => attrs.push(Attr {
                    name: "class".into(),
                    value: Some(value.trim_start().to_string()),
                    when: None,
                }),
            }
            n += 1;
        });
    };
    for l in &mut site.layouts {
        fix(&mut l.body);
    }
    for p in &mut site.pages {
        fix(&mut p.body);
    }
    for c in &mut site.components {
        fix(&mut c.body);
    }
    if n == 0 {
        return;
    }
    let sheet = site
        .stylesheet
        .get_or_insert_with(|| revel_model::Stylesheet {
            url: "/assets/css/site.css".into(),
            source: None,
            items: Vec::new(),
        });
    let mut taken: HashSet<String> = sheet
        .items
        .iter()
        .filter_map(|i| match i {
            revel_model::StyleItem::Rule(r) => r.id.clone(),
            _ => None,
        })
        .collect();
    for (sel, prop, value) in rules {
        let exists = sheet.items.iter().any(|i| {
            matches!(i, revel_model::StyleItem::Rule(r) if r.selector == sel && r.media.is_none())
        });
        if exists {
            continue;
        }
        let id = revel_model::ids::fresh(&taken);
        taken.insert(id.clone());
        sheet
            .items
            .push(revel_model::StyleItem::Rule(revel_model::StyleRule {
                id: Some(id),
                selector: sel,
                media: None,
                decls: vec![revel_model::Decl {
                    prop,
                    value,
                    important: false,
                }],
            }));
    }
    notes.push(format!(
        "{n} valeur(s) de style selon un champ : classes conditionnelles (Styles)"
    ));
}

/// `<body class="{{ bodyClass }}">` et `<html data-theme="{{ theme or 'dark' }}">` des gabarits :
/// la génération pose ces réglages de la page (Apparence) ; l'attribut garde la valeur par
/// défaut écrite.
fn page_look_attrs(site: &mut Site, notes: &mut Vec<String>) {
    let body_re = regex::Regex::new(r"^\s*\{\{-?\s*bodyClass\s*-?\}\}\s*$").expect("regex");
    let theme_re = regex::Regex::new(
        r#"^\s*\{\{-?\s*\(?\s*theme\s*(?:or\s*(?:'([^']*)'|"([^"]*)"))?\s*\)?\s*-?\}\}\s*$"#,
    )
    .expect("regex");
    for l in &mut site.layouts {
        let mut changed = false;
        walk_mut(&mut l.body, &mut |n| {
            let Node::Element { tag, attrs, .. } = n else {
                return;
            };
            match tag.as_str() {
                "body" => {
                    let before = attrs.len();
                    attrs.retain(|a| {
                        !(a.name == "class"
                            && a.when.is_none()
                            && a.value.as_deref().is_some_and(|v| body_re.is_match(v)))
                    });
                    changed |= attrs.len() != before;
                }
                "html" => {
                    let mut drop = false;
                    for a in attrs
                        .iter_mut()
                        .filter(|a| a.name == "data-theme" && a.when.is_none())
                    {
                        if let Some(c) = a.value.as_deref().and_then(|v| theme_re.captures(v)) {
                            match c.get(1).or(c.get(2)) {
                                Some(d) => a.value = Some(d.as_str().to_string()),
                                None => drop = true,
                            }
                            changed = true;
                        }
                    }
                    if drop {
                        attrs.retain(|a| a.name != "data-theme");
                    }
                }
                _ => {}
            }
        });
        if changed {
            notes.push(format!(
                "gabarit {} : classe du corps et thème réglés par page (Réglages de la page > Apparence)",
                l.id
            ));
        }
    }
}

/// `aria-current="page"` d'un lien sous condition « adresse de la page = adresse du lien »
/// (ou « commence par ») : retiré, la génération le pose ; « commence par » devient le réglage
/// « Actif sur les sous-pages » du lien.
fn current_links(site: &mut Site, notes: &mut Vec<String>) {
    let norm = |s: &str| -> String {
        let t: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
        let t = t.trim();
        let t = t
            .strip_prefix("{{")
            .and_then(|t| t.strip_suffix("}}"))
            .map(|t| t.trim_matches('-').trim().to_string())
            .unwrap_or_else(|| format!("{t:?}"));
        t.replace('\'', "\"")
    };
    let url = r#"(?:\(\s*page\.url\s+or\s+(?:""|'')\s*\)|page\.url)"#;
    let exact = regex::Regex::new(&format!(
        r"^\s*(?:{url}\s*==\s*(.+?)|(.+?)\s*==\s*{url})\s*$"
    ))
    .expect("regex");
    let prefix = regex::Regex::new(&format!(
        r"^\s*{url}\s*(?:is\s+startingwith\s*\(\s*(.+?)\s*\)|\.startswith\s*\(\s*(.+?)\s*\))\s*$"
    ))
    .expect("regex");
    let mut n = 0usize;
    let mut fix = |body: &mut Vec<Node>| {
        walk_mut(body, &mut |node| {
            let Node::Element { tag, attrs, .. } = node else {
                return;
            };
            if tag != "a" {
                return;
            }
            let Some(href) = attrs
                .iter()
                .find(|a| a.name == "href" && a.when.is_none())
                .and_then(|a| a.value.clone())
            else {
                return;
            };
            let target = norm(&href);
            let Some(i) = attrs.iter().position(|a| {
                a.name == "aria-current" && a.value.as_deref() == Some("page") && a.when.is_some()
            }) else {
                return;
            };
            let when = attrs[i].when.clone().unwrap_or_default();
            let same = |c: &regex::Captures| {
                c.iter()
                    .skip(1)
                    .flatten()
                    .next()
                    .is_some_and(|m| norm(&format!("{{{{ {} }}}}", m.as_str())) == target)
            };
            if exact.captures(&when).is_some_and(|c| same(&c)) {
                attrs.remove(i);
                n += 1;
            } else if prefix.captures(&when).is_some_and(|c| same(&c)) {
                attrs.remove(i);
                if !attrs.iter().any(|a| a.name == "data-revel-current") {
                    attrs.push(Attr {
                        name: "data-revel-current".into(),
                        value: Some("section".into()),
                        when: None,
                    });
                }
                n += 1;
            }
        });
    };
    for l in &mut site.layouts {
        fix(&mut l.body);
    }
    for p in &mut site.pages {
        fix(&mut p.body);
    }
    for c in &mut site.components {
        fix(&mut c.body);
    }
    if n > 0 {
        notes.push(format!(
            "{n} lien(s) « page actuelle » : marqués par la génération (état Page actuelle des styles)"
        ));
    }
}

/// Page de flux RSS écrite à la main (une boucle sur une collection dans `<rss>`) :
/// remplacée par un flux des réglages du site quand celui-ci liste les mêmes contenus.
fn convert_feeds(input: &BuildInput, site: &mut Site, notes: &mut Vec<String>) {
    let col_re = regex::Regex::new(r"collections\.([A-Za-z0-9_]+)").expect("regex");
    let limit_re = regex::Regex::new(r"\blimit\(\s*(\d+)\s*\)").expect("regex");
    let cands: Vec<(String, String, Option<usize>)> = site
        .pages
        .iter()
        .filter(|p| {
            p.path.ends_with(".xml") && !site.settings.feeds.iter().any(|f| f.path == p.path)
        })
        .filter_map(|p| {
            let (mut rss, mut col, mut limit) = (false, None, None);
            revel_model::ids::walk(&p.body, &mut |n| match n {
                Node::Element { tag, .. } if tag == "rss" => rss = true,
                Node::Each { expr, .. } if col.is_none() => {
                    col = col_re.captures(expr).map(|c| c[1].to_string());
                    limit = limit_re.captures(expr).and_then(|c| c[1].parse().ok());
                }
                _ => {}
            });
            Some((p.id.clone(), col.filter(|_| rss)?, limit))
        })
        .collect();
    if cands.is_empty() {
        return;
    }
    let Ok(out) = crate::build(input) else {
        return;
    };
    let tag = |xml: &str, t: &str| {
        regex::Regex::new(&format!(r"(?s)<{t}>(.*?)</{t}>"))
            .ok()?
            .captures(xml)
            .map(|c| unescape(c[1].trim()))
    };
    let links = |xml: &str| -> Vec<String> {
        xml.split("<item>")
            .skip(1)
            .filter_map(|it| tag(it, "link"))
            .collect()
    };
    for (pid, col, limit) in cands {
        let Some(page) = site.pages.iter().find(|p| p.id == pid) else {
            continue;
        };
        let path = page.path.clone();
        let Ok(file) = crate::output_path(&path) else {
            continue;
        };
        let Some(f) = out.files.iter().find(|f| f.path == file) else {
            continue;
        };
        let xml = String::from_utf8_lossy(&f.bytes).to_string();
        let head = xml.split("<item>").next().unwrap_or_default().to_string();
        let base = site
            .base_url
            .clone()
            .or_else(|| {
                site.data
                    .get("site")
                    .and_then(|s| s.get("url"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_default();
        let base = base.trim_end_matches('/');
        let feed = revel_model::FeedSettings {
            collection: col.clone(),
            path: path.clone(),
            title: tag(&head, "title"),
            description: tag(&head, "description").filter(|d| !d.is_empty()),
            link: tag(&head, "link")
                .map(|l| l.strip_prefix(base).unwrap_or(&l).to_string())
                .filter(|l| l.starts_with('/') && l != "/"),
            limit: limit.filter(|n| *n != 20),
        };
        // Même liste de contenus, dans le même ordre : flux remplacé.
        let mut test = input.site.clone();
        test.settings.feeds = vec![feed.clone()];
        let tinput = BuildInput {
            site: &test,
            entries: input.entries.clone(),
            locales: Vec::new(),
            environment: input.environment.clone(),
            version: input.version.clone(),
            hooks: input.hooks.clone(),
            now: input.now,
        };
        let (mut files, mut warns) = (Vec::new(), Vec::new());
        if crate::feeds(&tinput, &mut files, &mut warns).is_err() {
            continue;
        }
        let same = files
            .iter()
            .find(|f| f.path == file)
            .is_some_and(|f| links(&String::from_utf8_lossy(&f.bytes)) == links(&xml));
        if same {
            site.pages.retain(|p| p.id != pid);
            site.settings.feeds.push(feed);
            notes.push(format!(
                "flux RSS {path} : réglage du site (collection {col}, Paramètres > Général)"
            ));
        } else {
            notes.push(format!(
                "flux RSS {path} laissé en page : ses contenus diffèrent du flux de la collection {col}"
            ));
        }
    }
}

/// Plan du site écrit à la main (`/sitemap.xml` parcourant une collection) : remplacé par celui
/// que Revel produit (Paramètres › Référencement), avec les dates de mise à jour si la page en
/// donnait, quand il liste les mêmes adresses avec les mêmes dates.
fn convert_sitemap(input: &BuildInput, site: &mut Site, notes: &mut Vec<String>) {
    let Some(pid) = site
        .pages
        .iter()
        .find(|p| {
            p.path == "/sitemap.xml"
                && p.body
                    .iter()
                    .any(|n| !matches!(n, Node::Text { .. } | Node::Html { .. }))
        })
        .map(|p| p.id.clone())
    else {
        return;
    };
    let entries = |xml: &str| -> Vec<(String, Option<String>)> {
        let tag = |it: &str, t: &str| {
            regex::Regex::new(&format!(r"(?s)<{t}>(.*?)</{t}>"))
                .ok()?
                .captures(it)
                .map(|c| unescape(c[1].trim()))
        };
        let mut v: Vec<(String, Option<String>)> = xml
            .split("<url>")
            .skip(1)
            .filter_map(|it| Some((tag(it, "loc")?, tag(it, "lastmod"))))
            .collect();
        v.sort();
        v
    };
    let sitemap = |s: &Site| -> Option<String> {
        let inp = BuildInput {
            site: s,
            entries: input.entries.clone(),
            locales: Vec::new(),
            environment: input.environment.clone(),
            version: input.version.clone(),
            hooks: input.hooks.clone(),
            now: input.now,
        };
        let out = crate::build(&inp).ok()?;
        out.files
            .iter()
            .find(|f| f.path == "sitemap.xml")
            .map(|f| String::from_utf8_lossy(&f.bytes).to_string())
    };
    let Some(own) = sitemap(input.site) else {
        return;
    };
    let want = entries(&own);
    if want.is_empty() {
        return;
    }
    let mut test = input.site.clone();
    test.pages.retain(|p| p.id != pid);
    test.settings.seo.no_sitemap = false;
    test.settings.seo.sitemap_lastmod = want.iter().any(|(_, d)| d.is_some());
    if sitemap(&test).is_some_and(|x| entries(&x) == want) {
        site.pages.retain(|p| p.id != pid);
        site.settings.seo.no_sitemap = false;
        site.settings.seo.sitemap_lastmod = test.settings.seo.sitemap_lastmod;
        notes.push(
            "plan du site écrit à la main : produit par Revel (Paramètres › Référencement), mêmes adresses et dates"
                .into(),
        );
    } else {
        notes.push("plan du site laissé en page : ses adresses diffèrent de celles du plan produit par Revel".into());
    }
}

/// Expression d'un seul nœud (sans ses enfants).
fn own_exprs(n: &Node) -> Vec<String> {
    let mut one = n.clone();
    for list in revel_model::ids::child_lists_mut(&mut one) {
        list.clear();
    }
    exprs_of(std::slice::from_ref(&one))
}

/// Parenthèses autour d'un nom seul retirées (`((collections.x))` -> `collections.x`,
/// `(slug)` -> `slug`) : forme de la liste de collection du panneau. Le texte entre guillemets et
/// les parenthèses d'un appel (`limit(3)`) sont gardés.
fn bare_parens(e: &str) -> String {
    let re = regex::Regex::new(r"\(\s*([A-Za-z_][\w.]*)\s*\)").expect("regex");
    // Morceaux hors guillemets (réécrits) et entre guillemets (gardés).
    let pieces = |e: &str| -> Vec<(bool, String)> {
        let mut out: Vec<(bool, String)> = Vec::new();
        let mut cur = String::new();
        let mut quote: Option<char> = None;
        let mut esc = false;
        for c in e.chars() {
            match quote {
                Some(q) => {
                    cur.push(c);
                    if esc {
                        esc = false;
                    } else if c == '\\' {
                        esc = true;
                    } else if c == q {
                        out.push((false, std::mem::take(&mut cur)));
                        quote = None;
                    }
                }
                None if c == '"' || c == '\'' => {
                    out.push((true, std::mem::take(&mut cur)));
                    cur.push(c);
                    quote = Some(c);
                }
                None => cur.push(c),
            }
        }
        out.push((quote.is_none(), cur));
        out
    };
    let mut cur = e.trim().to_string();
    loop {
        let mut next = String::new();
        for (code, piece) in pieces(&cur) {
            if !code {
                next.push_str(&piece);
                continue;
            }
            let before = next.clone();
            let replaced = re.replace_all(&piece, |c: &regex::Captures| {
                let start = c.get(0).map(|m| m.start()).unwrap_or(0);
                let prev = piece[..start]
                    .chars()
                    .next_back()
                    .or_else(|| before.chars().next_back());
                if prev.is_some_and(|p| p.is_alphanumeric() || p == '_') {
                    c[0].to_string()
                } else {
                    c[1].to_string()
                }
            });
            next.push_str(&replaced);
        }
        if next == cur {
            return cur;
        }
        cur = next;
    }
}

/// Listes « contenus mis en avant, sinon une liste » écrites avec des variables, réécrites en
/// liste de collection « Mettre en premier » :
/// - suggestions d'un champ s'il y en a assez :
///   `{% set V = (collections.x | by_slugs(F)) if F and F|length else [] %}`,
///   `{% if V|length < N %}{% set V = <liste de x> %}{% endif %}`, `{% for i in V | limit(N) %}`
///   -> « Mettre en premier : F, seulement s'ils remplissent la liste », N au plus ;
/// - contenus choisis, sinon les plus récents : `{% set V = collections.x | by_slugs(["a", "b"]) %}`,
///   `{% if not V|length %}{% set V = <liste de x> %}{% endif %}`, `{% for i in V | limit(N) %}`
///   -> « Mettre en premier : a, b, complétés par la liste », N au plus.
///
/// Une lecture `{% if V|length %}` devient le nombre de contenus de la liste. Renvoie le nombre
/// de listes réécrites (le rendu est vérifié par l'appelant).
fn rewrite_suggestions(nodes: &mut Vec<Node>) -> usize {
    let mut count = 0;
    for n in nodes.iter_mut() {
        for list in revel_model::ids::child_lists_mut(n) {
            count += rewrite_suggestions(list);
        }
    }
    let by_field = regex::Regex::new(
        r"^\(\s*collections\.(\w+)\s*\|\s*by_slugs\(\s*([\w.]+)\s*\)\s*\)\s+if\s+([\w.]+)\s+and\s+([\w.]+)\s*\|\s*length\s+else\s+\[\s*\]$",
    )
    .expect("regex");
    let by_list =
        regex::Regex::new(r"^collections\.(\w+)\s*\|\s*by_slugs\((.*)\)$").expect("regex");
    let mut i = 0;
    while i < nodes.len() {
        let found = (|| {
            let Node::Set { name: v, expr } = &nodes[i] else {
                return None;
            };
            let v = v.clone();
            // (collection, contenus mis en premier, seulement s'ils remplissent la liste)
            let (col, first, full) = if let Some(c) = by_field.captures(expr.trim()) {
                let field = c[2].to_string();
                if c[3] != field || c[4] != field {
                    return None;
                }
                (c[1].to_string(), field, true)
            } else {
                let e = bare_parens(expr);
                let c = by_list.captures(e.trim())?;
                let inner = c[2]
                    .trim()
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .trim();
                let picks = revel_edit::collection::first_picks(inner)?;
                (
                    c[1].to_string(),
                    revel_edit::collection::first_expr(&picks),
                    false,
                )
            };
            let j = (i + 1..nodes.len()).find(|&k| !matches!(nodes[k], Node::Text { .. }))?;
            let Node::If {
                cond,
                then,
                otherwise,
                ..
            } = &nodes[j]
            else {
                return None;
            };
            let esc = regex::escape(&v);
            let threshold = if full {
                let less =
                    regex::Regex::new(&format!(r"^{esc}\s*\|\s*length\s*<\s*(\d+)$")).ok()?;
                Some(less.captures(cond.trim())?[1].parse::<usize>().ok()?)
            } else {
                let none =
                    regex::Regex::new(&format!(r"^not\s+\(?{esc}\s*\|\s*length\)?$")).ok()?;
                none.is_match(cond.trim()).then_some(None)?
            };
            let sets: Vec<&Node> = then
                .iter()
                .filter(|x| !matches!(x, Node::Text { .. }))
                .collect();
            let (true, [Node::Set { name: v2, expr: e2 }]) =
                (otherwise.is_empty(), sets.as_slice())
            else {
                return None;
            };
            if *v2 != v {
                return None;
            }
            let e2 = bare_parens(e2);
            let mut q = Query::parse(&canonical_list(&e2).unwrap_or(e2))?;
            if q.collection != col || q.first.is_some() || !q.pick.is_empty() {
                return None;
            }
            // Lectures de V après : boucle `V | limit(N)`, condition `V|length`, rien d'autre.
            let each_ok = regex::Regex::new(&format!(
                r"^{esc}\s*(?:\|\s*limit\(\s*(\d+)\s*\)|\[\s*:\s*(\d+)\s*\])$"
            ))
            .ok()?;
            let len_ok = regex::Regex::new(&format!(r"^\(?{esc}\)?\s*\|\s*length$")).ok()?;
            let mut ok = true;
            let mut limits: Vec<usize> = Vec::new();
            revel_model::ids::walk(&nodes[j + 1..], &mut |x| {
                let reads = own_exprs(x)
                    .iter()
                    .any(|e| refs(e).iter().any(|r| r.name == v));
                if !reads {
                    return;
                }
                ok &= match x {
                    Node::Each { expr, .. } => match each_ok.captures(expr.trim()) {
                        Some(c) => {
                            let n = c
                                .get(1)
                                .or(c.get(2))
                                .and_then(|m| m.as_str().parse::<usize>().ok());
                            limits.extend(n);
                            n.is_some()
                        }
                        None => false,
                    },
                    Node::If { cond, .. } => len_ok.is_match(cond.trim()),
                    _ => false,
                };
            });
            let n = *limits.first()?;
            if !ok || limits.iter().any(|l| *l != n) || threshold.is_some_and(|t| t != n) {
                return None;
            }
            if q.limit.is_some_and(|l| l != n) {
                return None;
            }
            q.first = Some(first);
            q.first_full = full;
            q.limit = Some(n);
            Some((j, v, q))
        })();
        let Some((j, v, q)) = found else {
            i += 1;
            continue;
        };
        let list = q.to_expr();
        let each_re = regex::Regex::new(&format!(r"^{}\b", regex::escape(&v))).expect("regex");
        walk_mut(&mut nodes[j + 1..], &mut |x| match x {
            Node::Each { expr, .. } if each_re.is_match(expr.trim()) => *expr = list.clone(),
            Node::If { cond, .. } if each_re.is_match(cond.trim().trim_start_matches('(')) => {
                *cond = format!("({list})|length")
            }
            _ => {}
        });
        nodes.remove(j);
        nodes.remove(i);
        count += 1;
    }
    count
}

/// Réécriture appliquée gabarit par gabarit, page par page, composant par composant, gardée là
/// où le site produit reste le même (fichiers identiques). Renvoie le nombre de réécritures
/// gardées et refusées.
fn verified_pass(
    input: &BuildInput,
    site: &mut Site,
    rewrite: &dyn Fn(&mut Vec<Node>, &mut HashSet<String>) -> usize,
) -> (usize, usize) {
    // Mêmes fichiers dans chaque environnement de publication.
    let out = |s: &Site| crate::build_all_envs(input, s);
    let mut base: Option<Option<Vec<crate::OutputFile>>> = None;
    let (mut kept, mut refused) = (0, 0);
    let owners = site.layouts.len() + site.pages.len() + site.components.len();
    for k in 0..owners {
        let mut cand = site.clone();
        let mut taken = cand.node_ids();
        let body = if k < cand.layouts.len() {
            &mut cand.layouts[k].body
        } else if k < cand.layouts.len() + cand.pages.len() {
            &mut cand.pages[k - cand.layouts.len()].body
        } else {
            &mut cand.components[k - cand.layouts.len() - cand.pages.len()].body
        };
        let n = rewrite(body, &mut taken);
        if n == 0 {
            continue;
        }
        let before = base.get_or_insert_with(|| out(site)).clone();
        let after = out(&cand);
        if before.is_some() && before == after {
            *site = cand;
            base = Some(after);
            kept += n;
        } else {
            refused += n;
        }
    }
    (kept, refused)
}

/// [`rewrite_suggestions`] sur tout le site, gardé là où le site produit reste le même.
fn suggestion_lists(input: &BuildInput, site: &mut Site, notes: &mut Vec<String>) {
    let (kept, refused) = verified_pass(input, site, &|b, _| rewrite_suggestions(b));
    if kept > 0 {
        notes.push(format!(
            "{kept} liste(s) « contenus mis en avant, sinon une liste » : liste de collection (Mettre en premier)"
        ));
    }
    if refused > 0 {
        notes.push(format!(
            "{refused} liste(s) « mis en avant, sinon… » laissée(s) en variables : le site produit changerait"
        ));
    }
}

/// Contenus choisis un à un dans une ou plusieurs collections, ajoutés à une liste :
/// `{% set V = [] %}` puis, pour chaque contenu, `{% set it = (collections.a | by_slug("s")) or
/// (collections.b | by_slug("s")) %}{% if it %}{% set V = (V + [it]) %}{% endif %}` ->
/// `{% set V = (collections.a + collections.b) | by_slugs(["s", …]) %}` (liste de collection à
/// plusieurs collections, contenus choisis). Renvoie le nombre de listes réécrites.
fn rewrite_accumulated_picks(nodes: &mut Vec<Node>) -> usize {
    let mut count = 0;
    for n in nodes.iter_mut() {
        for list in revel_model::ids::child_lists_mut(n) {
            count += rewrite_accumulated_picks(list);
        }
    }
    let one = regex::Regex::new(r#"^\(?\s*collections\.(\w+)\s*\|\s*by_slug\(\s*\(?\s*("(?:[^"\\]|\\.)*")\s*\)?\s*\)\s*\)?$"#)
        .expect("regex");
    // `(a | by_slug("s")) or (b | by_slug("s"))` -> ([a, b], "s").
    let lookup = |e: &str| -> Option<(Vec<String>, String)> {
        let e = e.trim();
        let e = if outer_parens(e) {
            &e[1..e.len() - 1]
        } else {
            e
        };
        let mut cols = Vec::new();
        let mut slug: Option<String> = None;
        for part in e.split(" or ") {
            let c = one.captures(part.trim())?;
            cols.push(c[1].to_string());
            let s = revel_edit::canvas::literal(&c[2])?;
            if slug.as_ref().is_some_and(|x| *x != s) {
                return None;
            }
            slug = Some(s);
        }
        Some((cols, slug?))
    };
    let mut i = 0;
    while i < nodes.len() {
        let Node::Set { name: v, expr } = &nodes[i] else {
            i += 1;
            continue;
        };
        if expr.trim() != "[]" {
            i += 1;
            continue;
        }
        let v = v.clone();
        let push = regex::Regex::new(&format!(
            r"^\(?\s*{}\s*\+\s*\[\s*(.*)\s*\]\s*\)?$",
            regex::escape(&v)
        ))
        .expect("regex");
        let mut cols: Option<Vec<String>> = None;
        let mut slugs: Vec<String> = Vec::new();
        let mut last = i;
        let mut k = i + 1;
        let mut it_expr: Option<String> = None;
        while k < nodes.len() {
            match &nodes[k] {
                Node::Text { text } if text.trim().is_empty() => {}
                Node::Set { name, expr } if name != &v && lookup(expr).is_some() => {
                    it_expr = Some(expr.clone());
                }
                Node::If {
                    cond,
                    then,
                    otherwise,
                    ..
                } if otherwise.is_empty() => {
                    let sets: Vec<&Node> = then
                        .iter()
                        .filter(|x| !matches!(x, Node::Text { .. }))
                        .collect();
                    let [Node::Set { name, expr: pe }] = sets.as_slice() else {
                        break;
                    };
                    if *name != v {
                        break;
                    }
                    let Some(c) = push.captures(pe.trim()) else {
                        break;
                    };
                    // Contenu ajouté : l'expression elle-même, ou la variable qui la porte.
                    let resolve = |e: &str| -> Option<(Vec<String>, String)> {
                        lookup(e).or_else(|| {
                            let x = e
                                .trim()
                                .trim_start_matches('(')
                                .trim_end_matches(')')
                                .trim();
                            it_expr
                                .as_deref()
                                .filter(|_| revel_model::valid_prop_name(x))
                                .and_then(lookup)
                        })
                    };
                    let (Some(a), Some(b)) = (resolve(cond), resolve(&c[1])) else {
                        break;
                    };
                    if a != b || cols.as_ref().is_some_and(|c| *c != a.0) {
                        break;
                    }
                    cols = Some(a.0);
                    slugs.push(a.1);
                    last = k;
                    it_expr = None;
                }
                _ => break,
            }
            k += 1;
        }
        let Some(cols) = cols.filter(|_| !slugs.is_empty()) else {
            i += 1;
            continue;
        };
        let q = Query {
            collection: cols[0].clone(),
            also: cols[1..].to_vec(),
            pick: slugs,
            ..Default::default()
        };
        nodes.splice(
            i..=last,
            [Node::Set {
                name: v,
                expr: q.to_expr(),
            }],
        );
        count += 1;
        i += 1;
    }
    count
}

/// Liste de collection gardée dans une variable (`{% set V = <liste> %}`), lue ensuite :
/// - `{% for x in V %}` (`| limit(n)`, `[:n]`) -> la liste elle-même dans la boucle ;
/// - `{% if V|length %}` -> nombre de contenus de la liste ;
/// - un élément qui lit le premier contenu (`V[0].data.cover`) -> enveloppé d'une liste d'un
///   seul contenu (`first`), qui lit ses champs (`first.data.cover`).
///
/// Toute autre lecture garde la variable. Renvoie le nombre de variables remplacées.
fn rewrite_list_vars(nodes: &mut Vec<Node>, taken: &mut HashSet<String>) -> usize {
    let mut count = 0;
    for n in nodes.iter_mut() {
        for list in revel_model::ids::child_lists_mut(n) {
            count += rewrite_list_vars(list, taken);
        }
    }
    let mut i = 0;
    while i < nodes.len() {
        let Node::Set { name: v, expr } = &nodes[i] else {
            i += 1;
            continue;
        };
        let Some(q) = Query::parse(expr) else {
            i += 1;
            continue;
        };
        let v = v.clone();
        let esc = regex::escape(&v);
        let each_re = regex::Regex::new(&format!(
            r"^{esc}\s*(?:\|\s*limit\(\s*(\d+)\s*\)|\[\s*:\s*(\d+)\s*\])?$"
        ))
        .expect("regex");
        let len_re =
            regex::Regex::new(&format!(r"^(not\s+)?\(?{esc}\)?\s*\|\s*length$")).expect("regex");
        let first_re = regex::Regex::new(&format!(r"\b{esc}\[0\]")).expect("regex");
        // Variables qui ne gardent que le premier contenu (`{% set first = V[0] %}`), plus lues
        // ensuite (leurs lectures déjà écrites avec `V[0]`) : retirées.
        let read_later: HashSet<String> = exprs_of(&nodes[i + 1..])
            .iter()
            .flat_map(|e| refs(e).into_iter().map(|r| r.name))
            .collect();
        let dead_first = |x: &Node| match x {
            Node::Set { name, expr } => first_re.is_match(expr) && !read_later.contains(name),
            _ => false,
        };
        // Lectures permises seulement.
        let mut ok = true;
        let mut uses = 0;
        revel_model::ids::walk(&nodes[i + 1..], &mut |x| {
            let es = own_exprs(x);
            if !es.iter().any(|e| refs(e).iter().any(|r| r.name == v)) {
                return;
            }
            uses += 1;
            if dead_first(x) {
                return;
            }
            ok &= match x {
                Node::Each { expr, .. } if each_re.is_match(expr.trim()) => true,
                Node::If { cond, .. } if len_re.is_match(cond.trim()) => true,
                // Premier contenu : toutes les lectures passent par `V[0]`.
                _ => es.iter().all(|e| {
                    let stripped = first_re.replace_all(e, "");
                    !refs(&stripped).iter().any(|r| r.name == v)
                }),
            };
        });
        if !ok || uses == 0 {
            i += 1;
            continue;
        }
        let list_of = |limit: Option<usize>| {
            let mut q = q.clone();
            if let Some(n) = limit {
                q.limit = Some(q.limit.map_or(n, |l| l.min(n)));
            }
            q.to_expr()
        };
        let first_list = list_of(Some(1));
        // Remplacement, du plus profond au plus haut : un élément qui lit `V[0]` est enveloppé.
        /// Motifs et listes du remplacement.
        struct Swap<'a> {
            each_re: &'a regex::Regex,
            len_re: &'a regex::Regex,
            first_re: &'a regex::Regex,
            list_of: &'a dyn Fn(Option<usize>) -> String,
            first_list: &'a str,
        }
        fn apply(nodes: &mut [Node], w: &Swap, taken: &mut HashSet<String>) {
            for node in nodes.iter_mut() {
                for list in revel_model::ids::child_lists_mut(node) {
                    apply(list, w, taken);
                }
                let reads_first = own_exprs(node).iter().any(|e| w.first_re.is_match(e));
                match node {
                    Node::Each { expr, .. } if w.each_re.is_match(expr.trim()) => {
                        let c = w.each_re.captures(expr.trim()).expect("motif vérifié");
                        let n = c
                            .get(1)
                            .or(c.get(2))
                            .and_then(|m| m.as_str().parse::<usize>().ok());
                        *expr = (w.list_of)(n);
                    }
                    Node::If { cond, .. } if w.len_re.is_match(cond.trim()) => {
                        let not = w
                            .len_re
                            .captures(cond.trim())
                            .and_then(|c| c.get(1))
                            .is_some();
                        *cond = format!(
                            "{}({})|length",
                            if not { "not " } else { "" },
                            (w.list_of)(None)
                        );
                    }
                    _ if reads_first => {
                        let mut inner = std::mem::replace(
                            node,
                            Node::Text {
                                text: String::new(),
                            },
                        );
                        rename_first(&mut inner, w.first_re);
                        let id = revel_model::ids::fresh(taken);
                        taken.insert(id.clone());
                        *node = Node::Each {
                            id: Some(id),
                            expr: w.first_list.to_string(),
                            item: "first".into(),
                            children: vec![inner],
                            empty: Vec::new(),
                        };
                    }
                    _ => {}
                }
            }
        }
        let mut rest: Vec<Node> = nodes.split_off(i + 1);
        fn drop_sets(nodes: &mut Vec<Node>, dead: &dyn Fn(&Node) -> bool) {
            nodes.retain(|x| !dead(x));
            for n in nodes.iter_mut() {
                for list in revel_model::ids::child_lists_mut(n) {
                    drop_sets(list, dead);
                }
            }
        }
        drop_sets(&mut rest, &dead_first);
        let swap = Swap {
            each_re: &each_re,
            len_re: &len_re,
            first_re: &first_re,
            list_of: &list_of,
            first_list: &first_list,
        };
        apply(&mut rest, &swap, taken);
        nodes.remove(i);
        nodes.extend(rest);
        count += 1;
    }
    count
}

/// `V[0]` devient `first` dans les expressions du nœud (pas ses enfants) ; `first and first.x`
/// -> `first.x` ; `first.x if first else ""` -> `first.x` (le contenu existe dans sa liste).
fn rename_first(n: &mut Node, first_re: &regex::Regex) {
    let fix = |e: &str| -> String {
        // `(V[0]).x` -> `first.x` (parenthèses d'un appel gardées).
        let mut e = bare_parens(&first_re.replace_all(e, "first"));
        if let Some(rest) = e.trim().strip_prefix("first and ") {
            e = rest.trim().to_string();
        }
        let re =
            regex::Regex::new(r#"^(first\.[\w.]+)\s+if\s+first\s+else\s+(""|'')$"#).expect("regex");
        if let Some(c) = re.captures(e.trim()) {
            e = c[1].to_string();
        }
        e
    };
    match n {
        Node::Out { expr } | Node::Set { expr, .. } | Node::Each { expr, .. } => *expr = fix(expr),
        Node::If { cond, .. } => *cond = fix(cond),
        Node::Element { attrs, .. } | Node::Use { attrs, .. } => {
            for a in attrs.iter_mut() {
                if let Some(w) = a.when.as_mut() {
                    *w = fix(w);
                }
                if let Some(val) = a.value.as_mut()
                    && first_re.is_match(val)
                {
                    *val = rewrite_template(val, &mut |e, _| Some(fix(e)));
                }
            }
        }
        Node::Image(img) => {
            img.src = fix(&img.src);
            if let Some(alt) = img.alt.as_mut() {
                *alt = fix(alt);
            }
        }
        _ => {}
    }
    if let Node::Use { args, .. } = n {
        for e in args.values_mut() {
            *e = fix(e);
        }
    }
}

/// Contenus choisis ajoutés un à un à une liste, puis cette liste lue : liste de collection.
fn picked_lists(input: &BuildInput, site: &mut Site, notes: &mut Vec<String>) {
    let (kept, refused) = verified_pass(input, site, &|b, taken| {
        let n = rewrite_accumulated_picks(b);
        if n == 0 {
            return 0;
        }
        rewrite_list_vars(b, taken).max(n)
    });
    if kept > 0 {
        notes.push(format!(
            "{kept} liste(s) de contenus choisis (une ou plusieurs collections) : liste de collection"
        ));
    }
    if refused > 0 {
        notes.push(format!(
            "{refused} liste(s) de contenus choisis laissée(s) en variables : le site produit changerait"
        ));
    }
}

/// Gardes redondantes retirées, formes du panneau : `x and x|trim` -> `x` (texte rempli),
/// `(x or "")` -> `x` dans une condition « contient » ou une valeur affichée. Renvoie le nombre
/// d'expressions simplifiées (gardées seulement si les fichiers produits restent identiques).
fn rewrite_guards(nodes: &mut [Node]) -> usize {
    let path = r"[A-Za-z_][\w]*(?:\.[A-Za-z_][\w-]*)*";
    let trim_re = regex::Regex::new(&format!(
        r"^\(?\s*({path})\s+and\s+\(?\s*({path})\s*\|\s*trim\s*\)?\s*\)?$"
    ))
    .expect("regex");
    let or_empty =
        regex::Regex::new(&format!(r#"\(\s*({path})\s+or\s+(?:""|'')\s*\)"#)).expect("regex");
    let whole_or_empty =
        regex::Regex::new(&format!(r#"^\s*({path})\s+or\s+(?:""|'')\s*$"#)).expect("regex");
    let simplify = |e: &str| -> Option<String> {
        // Choix écrit d'avance (`(nom if (1) == 1 else "")`, boucle dépliée) : la branche choisie.
        if let Some(f) = fold_ternary(e) {
            return Some(f);
        }
        if let Some(c) = trim_re.captures(e.trim())
            && c[1] == c[2]
        {
            return Some(c[1].to_string());
        }
        if let Some(c) = whole_or_empty.captures(e) {
            return Some(c[1].to_string());
        }
        let out = or_empty.replace_all(e, "$1").into_owned();
        (out != e).then_some(out)
    };
    let mut n = 0;
    walk_mut(nodes, &mut |node| match node {
        Node::If { cond, .. } => {
            if let Some(c) = simplify(cond) {
                *cond = c;
                n += 1;
            }
        }
        Node::Out { expr } => {
            if let Some(c) = simplify(expr) {
                *expr = c;
                n += 1;
            }
        }
        Node::Element { attrs, .. } => {
            for a in attrs.iter_mut() {
                if let Some(v) = a.value.as_mut()
                    && v.contains("{{")
                {
                    let mut changed = false;
                    for e in template_exprs(v) {
                        if let Some(c) = simplify(&e) {
                            *v = v.replacen(&e, &c, 1);
                            changed = true;
                        }
                    }
                    if changed {
                        n += 1;
                    }
                }
                if let Some(w) = a.when.as_mut()
                    && let Some(c) = simplify(w)
                {
                    *w = c;
                    n += 1;
                }
            }
        }
        Node::Use { args, .. } => {
            for e in args.values_mut() {
                if let Some(c) = simplify(e) {
                    *e = c;
                    n += 1;
                }
            }
        }
        Node::Image(img) => {
            if let Some(c) = img.alt.as_deref().and_then(simplify) {
                img.alt = Some(c);
                n += 1;
            }
            if let Some(c) = simplify(&img.src) {
                img.src = c;
                n += 1;
            }
        }
        _ => {}
    });
    n
}

/// Premier `mot` (` if `, ` else `) hors des parenthèses, crochets, accolades et guillemets :
/// (avant, après).
fn split_kw_top<'a>(s: &'a str, kw: &str) -> Option<(&'a str, &'a str)> {
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut esc = false;
    for (i, c) in s.char_indices() {
        if let Some(q) = quote {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ if depth == 0 && s[i..].starts_with(kw) => {
                return Some((&s[..i], &s[i + kw.len()..]));
            }
            _ => {}
        }
    }
    None
}

/// `a if C else b` dont la condition ne lit aucune variable (`(1) == 1`) : la branche choisie
/// (entre parenthèses si elle n'est pas un nom seul).
fn fold_ternary(e: &str) -> Option<String> {
    let mut t = e.trim();
    while outer_parens(t) {
        t = t[1..t.len() - 1].trim();
    }
    let (a, rest) = split_kw_top(t, " if ")?;
    let (c, b) = split_kw_top(rest, " else ")?;
    if !refs(c).is_empty() {
        return None;
    }
    let v = minijinja::Environment::new()
        .compile_expression(c)
        .ok()?
        .eval(())
        .ok()?;
    let pick = if v.is_true() { a.trim() } else { b.trim() };
    Some(
        if revel_edit::visibility::is_path(pick) || revel_edit::canvas::literal(pick).is_some() {
            pick.to_string()
        } else {
            format!("({pick})")
        },
    )
}

fn guards(input: &BuildInput, site: &mut Site, notes: &mut Vec<String>) {
    let (kept, refused) = verified_pass(input, site, &|b, _| rewrite_guards(b));
    if kept > 0 {
        notes.push(format!(
            "{kept} garde(s) redondante(s) retirée(s) (texte rempli, valeur vide)"
        ));
    }
    if refused > 0 {
        notes.push(format!(
            "{refused} garde(s) gardée(s) : le site produit changerait"
        ));
    }
    let (kept, _) = verified_pass(input, site, &|b, taken| rewrite_image_choices(b, taken));
    if kept > 0 {
        notes.push(format!(
            "{kept} image(s) aux réglages choisis selon une condition : une image par cas (condition d'affichage)"
        ));
    }
}

/// Image dont un réglage écrit est choisi selon une condition du panneau
/// (`sizes = ("100vw" if b.style != "column" else "760px")`) : une image par cas, sous une
/// condition d'affichage (`Si b.style != "column"` : image « 100vw », sinon image « 760px »).
fn rewrite_image_choices(nodes: &mut [Node], taken: &mut HashSet<String>) -> usize {
    let mut n = 0;
    walk_mut(nodes, &mut |node| {
        let Node::Image(img) = node else {
            return;
        };
        // Premier réglage choisi : (champ, condition, valeur si vraie, valeur sinon).
        let choice = |e: &Option<String>| -> Option<(String, String, String)> {
            let e = e.as_deref()?;
            let mut t = e.trim();
            while outer_parens(t) {
                t = t[1..t.len() - 1].trim();
            }
            let (a, rest) = split_kw_top(t, " if ")?;
            let (c, b) = split_kw_top(rest, " else ")?;
            let (a, b, c) = (a.trim(), b.trim(), c.trim());
            (revel_edit::canvas::literal(a).is_some()
                && revel_edit::canvas::literal(b).is_some()
                && revel_edit::visibility::read(c).is_some())
            .then(|| (c.to_string(), a.to_string(), b.to_string()))
        };
        let fields: [(&str, &Option<String>); 5] = [
            ("sizes", &img.sizes),
            ("class", &img.class),
            ("ratio", &img.ratio),
            ("loading", &img.loading),
            ("widths", &img.widths),
        ];
        let Some((field, (cond, a, b))) =
            fields.iter().find_map(|(f, e)| choice(e).map(|c| (*f, c)))
        else {
            return;
        };
        let with = |v: &str, fresh: bool, taken: &mut HashSet<String>| {
            let mut i = img.clone();
            let slot = match field {
                "sizes" => &mut i.sizes,
                "class" => &mut i.class,
                "ratio" => &mut i.ratio,
                "loading" => &mut i.loading,
                _ => &mut i.widths,
            };
            *slot = Some(v.to_string());
            if fresh {
                let id = revel_model::ids::fresh(taken);
                taken.insert(id.clone());
                i.id = Some(id);
            }
            Node::Image(i)
        };
        let first = with(&a, false, taken);
        let second = with(&b, true, taken);
        let id = revel_model::ids::fresh(taken);
        taken.insert(id.clone());
        *node = Node::If {
            id: Some(id),
            cond,
            then: vec![first],
            otherwise: vec![second],
        };
        n += 1;
    });
    n
}

/// Contenu relié lu directement : `{% if (collections.x | by_slug(champ)) %}` dont le contenu lit
/// `(collections.x | by_slug(champ)).data.nom` -> liste des contenus reliés par le champ
/// (`{% for ref in collections.x | by_slugs(champ) %}`, `ref.data.nom`), forme du panneau. Renvoie
/// le nombre de conditions réécrites.
fn rewrite_ref_lookups(nodes: &mut [Node]) -> usize {
    let mut count = 0;
    for n in nodes.iter_mut() {
        for list in revel_model::ids::child_lists_mut(n) {
            count += rewrite_ref_lookups(list);
        }
    }
    let cond_re = regex::Regex::new(
        r"^\(*\s*collections\.(\w+)\s*\|\s*by_slug\(\s*\(?\s*([A-Za-z_][\w.]*)\s*\)?\s*\)\s*\)*$",
    )
    .expect("regex");
    for n in nodes.iter_mut() {
        let Node::If {
            id,
            cond,
            then,
            otherwise,
        } = n
        else {
            continue;
        };
        if !otherwise.is_empty() {
            continue;
        }
        let Some(c) = cond_re.captures(cond.trim()) else {
            continue;
        };
        let (col, field) = (c[1].to_string(), c[2].to_string());
        // Chaque lecture du contenu relié dans le contenu de la condition.
        // Recherche seule, ou entre une paire de parenthèses (pas celles qui l'entourent).
        let lookup = format!(
            r"collections\.{}\s*\|\s*by_slug\(\s*(?:\(\s*{f}\s*\)|{f})\s*\)",
            regex::escape(&col),
            f = regex::escape(&field)
        );
        let look = regex::Regex::new(&format!(r"\(\s*{lookup}\s*\)|{lookup}")).expect("regex");
        let item = "ref";
        // Nom déjà employé dans le contenu : pas de réécriture.
        if exprs_of(then)
            .iter()
            .any(|e| refs(e).iter().any(|r| r.name == item))
        {
            continue;
        }
        let mut body = std::mem::take(then);
        walk_mut(&mut body, &mut |x| {
            let fix = |e: &str| look.replace_all(e, item).into_owned();
            match x {
                Node::Out { expr } | Node::Set { expr, .. } | Node::Each { expr, .. } => {
                    *expr = fix(expr)
                }
                Node::If { cond, .. } => *cond = fix(cond),
                Node::Element { attrs, .. } | Node::Use { attrs, .. } => {
                    for a in attrs.iter_mut() {
                        if let Some(w) = a.when.as_mut() {
                            *w = fix(w);
                        }
                        if let Some(v) = a.value.as_mut()
                            && look.is_match(v)
                        {
                            *v = rewrite_template(v, &mut |e, _| Some(fix(e)));
                        }
                    }
                }
                Node::Image(img) => {
                    img.src = fix(&img.src);
                    if let Some(alt) = img.alt.as_mut() {
                        *alt = fix(alt);
                    }
                }
                _ => {}
            }
            if let Node::Use { args, .. } = x {
                for e in args.values_mut() {
                    *e = fix(e);
                }
            }
        });
        *n = Node::Each {
            id: id.take(),
            expr: format!("collections.{col} | by_slugs({field})"),
            item: item.into(),
            children: body,
            empty: Vec::new(),
        };
        count += 1;
    }
    count
}

/// [`rewrite_ref_lookups`] gardé là où le site produit reste le même.
fn ref_lookups(input: &BuildInput, site: &mut Site, notes: &mut Vec<String>) {
    let (kept, _) = verified_pass(input, site, &|b, _| rewrite_ref_lookups(b));
    if kept > 0 {
        notes.push(format!(
            "{kept} contenu(s) relié(s) lu(s) directement : liste des contenus reliés par le champ"
        ));
    }
}

/// Valeur tirée d'un tableau selon un champ, gardée dans une variable du même nom
/// (`{% set color = {"orange": "#F4684D", …}[color] %}`), lue seulement par des attributs
/// `style` (`--x:{{ color }}`) : le tableau est écrit dans l'attribut, que [`map_styles`] change
/// ensuite en classes. Renvoie le nombre de variables remplacées.
fn inline_mapped_sets(nodes: &mut Vec<Node>) -> usize {
    let mut count = 0;
    for n in nodes.iter_mut() {
        for list in revel_model::ids::child_lists_mut(n) {
            count += inline_mapped_sets(list);
        }
    }
    let map_re = regex::Regex::new(r"^\(?\s*\{.*\}\s*\)?\s*\[\s*([A-Za-z_][A-Za-z0-9_.-]*)\s*\]$")
        .expect("regex");
    let mut i = 0;
    while i < nodes.len() {
        let Node::Set { name, expr } = &nodes[i] else {
            i += 1;
            continue;
        };
        let (v, e) = (name.clone(), expr.trim().to_string());
        if !map_re.captures(&e).is_some_and(|c| c[1] == v) {
            i += 1;
            continue;
        }
        let whole = regex::Regex::new(&format!(r"\{{\{{-?\s*{}\s*-?\}}\}}", regex::escape(&v)))
            .expect("regex");
        // Lectures après : seulement `{{ v }}` dans un attribut `style`.
        let mut ok = true;
        let mut uses = 0;
        revel_model::ids::walk(&nodes[i + 1..], &mut |x| {
            for ex in own_exprs(x) {
                if !refs(&ex).iter().any(|r| r.name == v) {
                    continue;
                }
                uses += 1;
                ok &= match x {
                    Node::Element { attrs, .. } => {
                        attrs.iter().any(|a| {
                            a.name == "style"
                                && a.when.is_none()
                                && a.value.as_deref().is_some_and(|val| whole.is_match(val))
                        }) && ex.trim() == v
                    }
                    _ => false,
                };
            }
        });
        if !ok || uses == 0 {
            i += 1;
            continue;
        }
        walk_mut(&mut nodes[i + 1..], &mut |x| {
            if let Node::Element { attrs, .. } = x {
                for a in attrs.iter_mut().filter(|a| a.name == "style") {
                    if let Some(val) = a.value.as_mut() {
                        *val = whole
                            .replace_all(val, format!("{{{{ {e} }}}}").as_str())
                            .into_owned();
                    }
                }
            }
        });
        nodes.remove(i);
        count += 1;
    }
    count
}

/// Noms lus par des nœuds (racines des chemins des valeurs, conditions, listes, attributs,
/// propriétés, images).
fn names_read(nodes: &[Node]) -> HashSet<String> {
    let mut used: HashSet<String> = HashSet::new();
    let attr_exprs = |attrs: &[Attr]| -> Vec<String> {
        attrs
            .iter()
            .flat_map(|a| {
                a.value
                    .as_deref()
                    .map(template_exprs)
                    .unwrap_or_default()
                    .into_iter()
                    .chain(a.when.clone())
            })
            .collect()
    };
    revel_model::ids::walk(nodes, &mut |n| {
        let exprs: Vec<String> = match n {
            Node::Out { expr } | Node::Set { expr, .. } | Node::Each { expr, .. } => {
                vec![expr.clone()]
            }
            Node::If { cond, .. } => vec![cond.clone()],
            Node::Use { attrs, args, .. } => attr_exprs(attrs)
                .into_iter()
                .chain(args.values().cloned())
                .collect(),
            Node::Element { attrs, .. } => attr_exprs(attrs),
            Node::Image(img) => std::iter::once(img.src.clone())
                .chain(img.alt.clone())
                .collect(),
            _ => Vec::new(),
        };
        for e in exprs {
            used.extend(refs(&e).into_iter().map(|r| r.name));
        }
    });
    used
}

/// Libellé lisible d'un nom de propriété (`hideFooterCta` -> « Hide footer cta », `cover_alt`
/// -> « Cover alt », `date2` -> « Date 2 ») ; `None` s'il est déjà le nom.
fn humanize(name: &str) -> Option<String> {
    let mut words: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut prev: Option<char> = None;
    for c in name.chars() {
        let split = c == '_'
            || c == '-'
            || (c.is_uppercase() && prev.is_some_and(|p| p.is_lowercase() || p.is_ascii_digit()))
            || (c.is_ascii_digit() && prev.is_some_and(|p| !p.is_ascii_digit()));
        if split && !cur.is_empty() {
            words.push(std::mem::take(&mut cur));
        }
        if c != '_' && c != '-' {
            cur.extend(c.to_lowercase());
        }
        prev = Some(c);
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    let mut label = words.join(" ");
    if let Some(f) = label.chars().next() {
        label = f.to_uppercase().collect::<String>() + &label[f.len_utf8()..];
    }
    (!label.is_empty() && label != name).then_some(label)
}

/// Variables d'une page lues par son gabarit (`{% if not hideFooterCta %}`, `{{ subtitle }}`) :
/// propriétés du gabarit le plus extérieur qui les lit, chaque page en donne la valeur dans ses
/// réglages (comme une utilisation de composant). Nature : interrupteur pour un booléen, image
/// ou lien selon l'endroit où le gabarit l'emploie, texte sinon ; une liste ou un objet reste
/// une variable de page. Une variable par défaut du gabarit lue par lui devient la valeur par
/// défaut de la propriété.
fn layout_fields_from_pages(site: &mut Site, notes: &mut Vec<String>) {
    use revel_edit::editable::PAGE_SETTINGS;
    let reads: BTreeMap<String, HashSet<String>> = site
        .layouts
        .iter()
        .map(|l| (l.id.clone(), names_read(&l.body)))
        .collect();
    let chain = |site: &Site, first: Option<&str>| -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut cur = first.map(str::to_string);
        while let Some(id) = cur {
            if out.contains(&id) {
                break;
            }
            cur = site
                .layouts
                .iter()
                .find(|l| l.id == id)
                .and_then(|l| l.parent.clone());
            out.push(id);
        }
        out
    };
    let scalar = |v: &serde_json::Value| {
        matches!(
            v,
            serde_json::Value::Bool(_)
                | serde_json::Value::String(_)
                | serde_json::Value::Number(_)
        )
    };
    // (gabarit, nom) -> valeurs des pages.
    let mut wanted: BTreeMap<(String, String), Vec<serde_json::Value>> = BTreeMap::new();
    for p in &site.pages {
        let ids = chain(site, p.layout.as_deref());
        let declared = revel_edit::editable::layout_fields(site, p.layout.as_deref());
        for (k, v) in &p.props {
            if PAGE_SETTINGS.contains(&k.as_str())
                || declared.iter().any(|f| &f.name == k)
                || !revel_model::valid_prop_name(k)
            {
                continue;
            }
            // Le plus extérieur des gabarits qui la lisent : visible de tous ceux qu'il contient.
            let Some(outer) = ids
                .iter()
                .rev()
                .find(|id| reads.get(*id).is_some_and(|r| r.contains(k)))
            else {
                continue;
            };
            wanted
                .entry((outer.clone(), k.clone()))
                .or_default()
                .push(v.clone());
        }
    }
    // Variables par défaut d'un gabarit lues par lui.
    for l in &site.layouts {
        for (k, v) in &l.props {
            if !PAGE_SETTINGS.contains(&k.as_str())
                && revel_model::valid_prop_name(k)
                && l.field(k).is_none()
                && reads.get(&l.id).is_some_and(|r| r.contains(k))
            {
                wanted
                    .entry((l.id.clone(), k.clone()))
                    .or_default()
                    .push(v.clone());
            }
        }
    }
    for ((lid, name), values) in wanted {
        if !values.iter().all(scalar) {
            continue;
        }
        let Some(li) = site.layouts.iter().position(|l| l.id == lid) else {
            continue;
        };
        let l = &mut site.layouts[li];
        let kind = if values.iter().all(serde_json::Value::is_boolean) {
            PropKind::Switch
        } else {
            // Endroit où le gabarit l'emploie seule : image, lien.
            let mut image = false;
            let mut link = false;
            revel_model::ids::walk(&l.body, &mut |n| match n {
                Node::Image(img) if img.src.trim() == name => image = true,
                Node::Element { attrs, .. } => {
                    for a in attrs {
                        let whole = a
                            .value
                            .as_deref()
                            .and_then(|v| v.trim().strip_prefix("{{"))
                            .and_then(|v| v.strip_suffix("}}"))
                            .map(str::trim);
                        if whole == Some(name.as_str()) {
                            match a.name.as_str() {
                                "src" | "poster" => image = true,
                                "href" => link = true,
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            });
            if image {
                PropKind::Image
            } else if link {
                PropKind::Link
            } else {
                PropKind::Text
            }
        };
        let default = l.props.remove(&name);
        l.fields.push(PropDef {
            name: name.clone(),
            label: humanize(&name),
            kind,
            default,
        });
        notes.push(format!(
            "gabarit {lid} : propriété {name} (réglée par chaque page)"
        ));
    }
}

/// Formes composées ([`fuse`]) : boucles sur des contenus reliés réunies, composants qui reçoivent
/// une liste construite ou du HTML assemblé dépliés, listes construites parcourues directement,
/// HTML assemblé en éléments ; puis valeurs de style choisies dans un tableau en classes.
fn composite_forms(input: &BuildInput, site: &mut Site, notes: &mut Vec<String>) {
    let (kept, _) = verified_pass(input, site, &|b, _| fuse::fuse_nested_refs(b));
    if kept > 0 {
        notes.push(format!(
            "{kept} boucle(s) sur un champ autour de contenus reliés : liste des contenus reliés par le champ"
        ));
    }
    let used_before: HashSet<String> = used_components(site);
    let components = site.components.clone();
    let (kept, _) = verified_pass(input, site, &|b, taken| {
        fuse::inline_uses(b, &components, taken)
    });
    if kept > 0 {
        notes.push(format!(
            "{kept} utilisation(s) de composant dépliée(s) (liste construite ou HTML assemblé reçus)"
        ));
    }
    let (kept, _) = verified_pass(input, site, &|b, _| fuse::fuse_accumulations(b));
    if kept > 0 {
        notes.push(format!(
            "{kept} liste(s) construite(s) pas à pas : parcourue(s) directement"
        ));
    }
    let (kept, _) = verified_pass(input, site, &|b, taken| fuse::html_strings(b, taken));
    if kept > 0 {
        notes.push(format!("{kept} HTML assemblé(s) dans un texte : éléments"));
    }
    let (kept, _) = verified_pass(input, site, &|b, _| fuse::fold_const_ifs(b));
    if kept > 0 {
        notes.push(format!(
            "{kept} condition(s) connue(s) d'avance : contenu gardé"
        ));
    }
    // Composants qui ne servent plus (dépliés partout).
    let used_after = used_components(site);
    let gone: Vec<String> = used_before
        .iter()
        .filter(|c| !used_after.contains(*c))
        .cloned()
        .collect();
    if !gone.is_empty() {
        site.components.retain(|c| !gone.contains(&c.id));
        notes.push(format!(
            "composant(s) déplié(s) partout, retiré(s) : {}",
            gone.join(", ")
        ));
    }
    map_styles(site, notes);
}

/// Composants utilisés dans le site.
fn used_components(site: &Site) -> HashSet<String> {
    let mut out = HashSet::new();
    for (_, _, body) in site.owners() {
        revel_model::ids::walk(body, &mut |n| {
            if let Node::Use { component, .. } = n {
                out.insert(component.clone());
            }
        });
    }
    out
}

/// Reconstruit le site de `input` dans la forme de l'interface (même rendu).
pub fn rebuild(input: &BuildInput) -> Result<Rebuilt, RenderError> {
    let b = Builder::new(input);
    let prepared = crate::prepare(&b, input, true)?;
    let globals = prepared.globals.clone();
    // Plusieurs environnements de publication : leurs variables (`env.staging`) restent lues à
    // la publication, jamais écrites avec les valeurs d'un seul.
    let several_envs = crate::environments_of(input.site, &input.environment).len() > 1;
    let global_names: HashSet<String> = globals
        .keys()
        .filter(|k| *k != "collections" && !(several_envs && *k == "env"))
        .cloned()
        .collect();
    let mut site = input.site.clone();
    let mut notes = Vec::new();
    // Flux RSS écrits à la main : réglages du site (un interrupteur par collection).
    convert_feeds(input, &mut site, &mut notes);
    // Plan du site écrit à la main : celui de Revel (mêmes adresses et dates).
    convert_sitemap(input, &mut site, &mut notes);
    // Classe du corps et thème lus dans la page : posés par la génération (réglages de la page).
    page_look_attrs(&mut site, &mut notes);
    // Lien « page actuelle » écrit en condition : marqué par la génération.
    current_links(&mut site, &mut notes);
    // Contenus choisis et contenus reliés écrits à la main : listes de collection.
    for l in &mut site.layouts {
        rewrite_picks(&mut l.body);
        simplify_refs(&mut l.body);
    }
    for p in &mut site.pages {
        rewrite_picks(&mut p.body);
        simplify_refs(&mut p.body);
    }
    for c in &mut site.components {
        rewrite_picks(&mut c.body);
        simplify_refs(&mut c.body);
    }
    let mut taken = site.node_ids();
    let fl = |known: HashSet<String>, ctx: String, taken: &mut HashSet<String>| Fl {
        b: &b,
        known,
        dynamic: Vec::new(),
        aliases: Vec::new(),
        taken: std::mem::take(taken),
        notes: Vec::new(),
        ctx,
        mutable: HashSet::new(),
        in_head: false,
    };
    // Valeurs lues chez l'utilisateur : propriétés, données par chaque utilisation (écrites
    // ensuite là où elles sont connues d'avance).
    // Un composant qui en utilise un autre lit à son tour les valeurs données à ce dernier :
    // déduction répétée jusqu'à ce qu'aucune propriété ne s'ajoute.
    for _ in 0..4 {
        let before = notes.len();
        deduce_props(&mut site, &global_names, &mut notes);
        if notes.len() == before {
            break;
        }
    }
    // Composants : données du site connues d'avance ; ce qui vient de l'utilisateur, vivant.
    for ci in 0..site.components.len() {
        let mut f = fl(
            global_names.clone(),
            format!("composant {}", site.components[ci].id),
            &mut taken,
        );
        let props: HashSet<String> = site.components[ci]
            .props
            .iter()
            .map(|p| p.name.clone())
            .collect();
        f.dynamic.push(props);
        let mut scope = Scope::new(globals.clone(), BTreeMap::new());
        let body = site.components[ci].body.clone();
        f.mutable = mutable_names(&body);
        site.components[ci].body = f.nodes(&body, &mut scope);
        taken = std::mem::take(&mut f.taken);
        notes.append(&mut f.notes);
    }
    // Gabarits : la page s'y place par l'emplacement ; ce qui dépend de la page reste vivant.
    for li in 0..site.layouts.len() {
        let mut body = std::mem::take(&mut site.layouts[li].body);
        revel_model::content_to_slot(&mut body);
        let mut f = fl(
            global_names.clone(),
            format!("gabarit {}", site.layouts[li].id),
            &mut taken,
        );
        let mut scope = Scope::new(globals.clone(), BTreeMap::new());
        f.mutable = mutable_names(&body);
        site.layouts[li].body = f.nodes(&body, &mut scope);
        taken = std::mem::take(&mut f.taken);
        notes.append(&mut f.notes);
    }
    // Pages : données de la page connues d'avance.
    for j in prepared.jobs.iter().filter(|j| j.page_id.is_some()) {
        let pid = j.page_id.clone().unwrap_or_default();
        let Some(pi) = site.pages.iter().position(|p| p.id == pid) else {
            continue;
        };
        // Réglage de la page calculé depuis les données du site (`seo = pages["agence"].seo`) :
        // valeur écrite dans les réglages de la page.
        let settings: Vec<String> = site.pages[pi]
            .computed
            .keys()
            .filter(|k| revel_edit::editable::PAGE_SETTINGS.contains(&k.as_str()))
            .cloned()
            .collect();
        for k in settings {
            let p = &mut site.pages[pi];
            match j.data.get(&k) {
                // Donnée absente : le réglage n'existait pas (réglages du site par défaut).
                None => {
                    p.computed.remove(&k);
                }
                Some(v) if v.is_undefined() || v.is_none() => {
                    p.computed.remove(&k);
                }
                Some(v) => {
                    if let Ok(json) = serde_json::to_value(v) {
                        p.computed.remove(&k);
                        p.props.insert(k.clone(), json);
                        notes.push(format!("page {} : réglage {k} écrit dans la page", j.url));
                    }
                }
            }
        }
        let mut data = j.data.clone();
        let mut page: BTreeMap<String, Value> = BTreeMap::new();
        page.insert("url".into(), Value::from(j.url.clone()));
        page.insert("fileSlug".into(), Value::from(j.file_slug.clone()));
        data.insert("page".into(), Value::from(page));
        let mut known = global_names.clone();
        known.extend(data.keys().cloned());
        let mut f = fl(known, format!("page {}", j.url), &mut taken);
        let mut scope = Scope::new(globals.clone(), data);
        let body = site.pages[pi].body.clone();
        f.mutable = mutable_names(&body);
        site.pages[pi].body = f.nodes(&body, &mut scope);
        taken = std::mem::take(&mut f.taken);
        notes.append(&mut f.notes);
    }
    // Listes de données lues par adresse (`rubriques.items | find_by('slug', s)`) : collections
    // du CMS ; leurs contenus servent ensuite à chaque vérification.
    let collections = fuse::data_collections(input, &globals, &mut site, &mut notes);
    let input_with = BuildInput {
        site: input.site,
        entries: input
            .entries
            .iter()
            .cloned()
            .chain(collections.iter().flat_map(|c| c.entries.iter().cloned()))
            .collect(),
        locales: input.locales.clone(),
        environment: input.environment.clone(),
        version: input.version.clone(),
        hooks: input.hooks.clone(),
        now: input.now,
    };
    let input = &input_with;
    // Valeur tirée d'un tableau gardée dans une variable, lue par un style : écrite dans le style.
    let (kept, _) = verified_pass(input, &mut site, &|b, _| inline_mapped_sets(b));
    if kept > 0 {
        notes.push(format!(
            "{kept} valeur(s) tirée(s) d'un tableau écrite(s) dans le style"
        ));
    }
    // Valeurs de style choisies dans un tableau selon un champ : classes conditionnelles.
    map_styles(&mut site, &mut notes);
    // Liste dont la boucle saute le contenu affiché : réglage « sans le contenu affiché ».
    exclude_current_ifs(&mut site, &mut notes);
    // Suggestions du contenu, sinon une liste : liste de collection « Mettre en premier ».
    suggestion_lists(input, &mut site, &mut notes);
    // Contenus choisis ajoutés un à un (une ou plusieurs collections) : liste de collection.
    picked_lists(input, &mut site, &mut notes);
    // Contenu relié lu directement (`(collections.x | by_slug(champ)).data.nom`) : liste reliée.
    ref_lookups(input, &mut site, &mut notes);
    // Gardes redondantes (`x and x|trim`, `(x or "")`) : conditions et valeurs du panneau.
    guards(input, &mut site, &mut notes);
    composite_forms(input, &mut site, &mut notes);
    // Parenthèses autour d'un nom seul (`(b.cover)`, `(title) or 'x'`) retirées : formes du panneau.
    let simplify = |nodes: &mut Vec<Node>| {
        walk_mut(nodes, &mut |n| match n {
            Node::Out { expr } => *expr = bare_parens(expr),
            Node::If { cond, .. } => *cond = bare_parens(cond),
            Node::Use { args, .. } => {
                for e in args.values_mut() {
                    *e = bare_parens(e);
                }
            }
            Node::Image(img) => {
                img.src = bare_parens(&img.src);
                if let Some(a) = img.alt.as_mut() {
                    *a = bare_parens(a);
                }
            }
            _ => {}
        });
    };
    for l in &mut site.layouts {
        simplify(&mut l.body);
    }
    for p in &mut site.pages {
        simplify(&mut p.body);
    }
    for c in &mut site.components {
        simplify(&mut c.body);
    }
    // Variables vivantes qui ne servent plus (remplacées là où elles servaient) : retirées,
    // sauf celles qu'un composant lit chez son utilisateur.
    let mut read_by_components: HashSet<String> = HashSet::new();
    for c in &site.components {
        let own: HashSet<String> = c.props.iter().map(|p| p.name.clone()).collect();
        for e in exprs_of(&c.body) {
            read_by_components.extend(
                refs(&e)
                    .into_iter()
                    .map(|r| r.name)
                    .filter(|n| !own.contains(n) && !global_names.contains(n)),
            );
        }
    }
    let prune = |body: &mut Vec<Node>| {
        let used: HashSet<String> = exprs_of(body)
            .into_iter()
            .flat_map(|e| refs(&e).into_iter().map(|r| r.name))
            .collect();
        prune_sets(body, &|name: &str| {
            used.contains(name) || read_by_components.contains(name)
        });
    };
    for l in &mut site.layouts {
        prune(&mut l.body);
    }
    for p in &mut site.pages {
        prune(&mut p.body);
    }
    for c in &mut site.components {
        prune(&mut c.body);
    }
    // Variables de page qui ne servent plus (écrites dans la page) : retirées ; celles que lisent
    // les gabarits restent.
    let mut used: HashSet<String> = HashSet::new();
    for l in &site.layouts {
        used.extend(names_read(&l.body));
    }
    for p in &mut site.pages {
        p.props.retain(|k, _| {
            revel_edit::editable::PAGE_SETTINGS.contains(&k.as_str()) || used.contains(k)
        });
        p.computed.retain(|k, _| used.contains(k));
    }
    // Variables des pages lues par leur gabarit : propriétés du gabarit.
    layout_fields_from_pages(&mut site, &mut notes);
    // Filtres propres au site qui ne servent plus.
    let text =
        serde_json::to_string(&(&site.layouts, &site.pages, &site.components)).unwrap_or_default();
    let unused: Vec<String> = site
        .settings
        .filters
        .keys()
        .filter(|f| {
            !regex::Regex::new(&format!(r"\|\s*{}\b", regex::escape(f)))
                .map(|re| re.is_match(&text))
                .unwrap_or(true)
        })
        .cloned()
        .collect();
    for f in unused {
        site.settings.filters.remove(&f);
        notes.push(format!("filtre {f} retiré (remplacé par sa définition)"));
    }
    // En-tête écrit à la main : réglages du site et des pages, là où le site produit reste le
    // même.
    crate::head::head_settings(input, &mut site, &mut notes);
    Ok(Rebuilt {
        site,
        notes,
        collections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parentheses_autour_d_un_nom() {
        assert_eq!(
            bare_parens("((collections.projets)) | limit(3)"),
            "collections.projets | limit(3)"
        );
        assert_eq!(
            bare_parens("(title) or 'Vidéo (courte)'"),
            "title or 'Vidéo (courte)'"
        );
        assert_eq!(
            bare_parens("where_contains('data.x', (slug))"),
            "where_contains('data.x', slug)"
        );
        assert_eq!(bare_parens("f(x)"), "f(x)");
    }

    #[test]
    fn libelles_lisibles() {
        assert_eq!(
            humanize("hideFooterCta").as_deref(),
            Some("Hide footer cta")
        );
        assert_eq!(humanize("cover_alt").as_deref(), Some("Cover alt"));
        assert_eq!(humanize("date2").as_deref(), Some("Date 2"));
        assert_eq!(humanize("Titre"), None);
        assert_eq!(humanize("titre").as_deref(), Some("Titre"));
    }

    #[test]
    fn variables_lues() {
        let names = |e: &str| refs(e).into_iter().map(|r| r.name).collect::<Vec<_>>();
        assert_eq!(
            names(
                r#"(collections.projets | sort(attribute="date", reverse=true)) | by_slugs(home.news)"#
            ),
            vec!["collections", "home"]
        );
        assert_eq!(names("a.title | accent | safe"), vec!["a"]);
        assert_eq!(names("x is defined and not y.z"), vec!["x", "y"]);
        assert_eq!(names("'it is' ~ name"), vec!["name"]);
        assert_eq!(names("range(3) | join(', ')"), Vec::<String>::new());
        let r = &refs("by_slugs(home.news) | limit(3)")[0];
        assert_eq!((r.start, r.path_end), (9, 18));
        assert_eq!(
            template_exprs("card {% if a.b %}is-x{% endif %} {{ c }}"),
            vec!["a.b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn listes_de_collection_dans_la_forme_du_panneau() {
        let input_site = Site::new("Essai", "fr");
        let input = crate::BuildInput {
            site: &input_site,
            entries: Vec::new(),
            locales: Vec::new(),
            environment: "production".into(),
            version: "v1".into(),
            hooks: std::sync::Arc::new(crate::PlainAssets),
            now: chrono::Utc::now(),
        };
        let b = Builder::new(&input);
        let f = Fl {
            b: &b,
            known: HashSet::new(),
            dynamic: Vec::new(),
            aliases: Vec::new(),
            taken: HashSet::new(),
            notes: Vec::new(),
            ctx: String::new(),
            mutable: HashSet::new(),
            in_head: false,
        };
        assert_eq!(
            f.collection_list(
                r#"(collections.projets | sort(attribute="date", reverse=true)) | limit(6)"#
            ),
            r#"(collections.projets | sort(attribute="date", reverse=true))[:6]"#
        );
        assert_eq!(
            f.collection_list(r#"((collections.actualites | sort(attribute="date", reverse=true)) | by_slugs(["a", "b"])) | limit(3)"#),
            r#"(collections.actualites | by_slugs(["a", "b"]))[:3]"#
        );
    }
}
