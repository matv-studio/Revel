//! Reconstruction, formes composées (suite de [`super`]), chacune gardée seulement si les
//! fichiers produits restent identiques :
//! - liste de données lue par son adresse (`rubriques.items | find_by('slug', s)`) → collection
//!   du CMS (un contenu par élément), lue par `collections.x | by_slug(s)` ;
//! - boucle sur un champ qui ne contient qu'une liste de contenus reliés par la valeur
//!   (`for s in categories` puis `for ref in collections.x | by_slugs(s)`) → liste des contenus
//!   reliés par le champ (`collections.x | by_slugs(categories)`) ;
//! - utilisation de composant qui reçoit une liste construite pas à pas ou du HTML assemblé →
//!   contenu du composant à sa place (ses propriétés remplacées par les valeurs données) ;
//! - liste construite pas à pas (`set L = []`, `set L = L + [{ "label": ex.data.title }]`) puis
//!   parcourue → liste parcourue directement, valeurs lues dans chaque contenu ;
//! - HTML assemblé dans un texte (`'<time datetime="' + d + '">' + … + '</time>'`) → éléments ;
//! - condition connue d'avance (`none`, texte jamais vide) → son contenu.

use super::*;
use crate::Entry;
use revel_model::Attr;
use std::collections::HashMap;

/// Collection créée à la reconstruction : définition, nom affiché, champs, contenus à écrire.
#[derive(Debug, Clone)]
pub struct NewCollection {
    pub collection: revel_model::Collection,
    pub label: String,
    pub fields: Vec<String>,
    pub entries: Vec<Entry>,
}

// ------------------------------------------------------------------------------------------------
// Outils

/// Morceaux séparés par `sep` hors des parenthèses, crochets, accolades et guillemets.
fn split_all(s: &str, sep: char) -> Vec<String> {
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
            '(' | '[' | '{' => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' | '}' => {
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

/// Expression sans les parenthèses qui l'entourent.
fn unwrap_parens(e: &str) -> &str {
    let mut t = e.trim();
    while outer_parens(t) {
        t = t[1..t.len() - 1].trim();
    }
    t
}

/// Valeur mise à la place d'un nom : telle quelle si c'est un chemin ou un littéral, sinon
/// entre parenthèses.
fn grouped(e: &str) -> String {
    let t = e.trim();
    if revel_edit::visibility::is_path(t)
        || revel_edit::canvas::literal(t).is_some()
        || outer_parens(t)
        || matches!(t, "true" | "false" | "none" | "[]")
        || t.parse::<f64>().is_ok()
    {
        t.to_string()
    } else {
        format!("({t})")
    }
}

/// Parenthèses autour de toute l'expression retirées (après un remplacement).
fn tidy(e: &str) -> String {
    let t = e.trim();
    if outer_parens(t) {
        unwrap_parens(t).to_string()
    } else {
        e.to_string()
    }
}

/// Applique `f` à chaque expression propre au nœud (pas à celles de ses enfants).
fn map_exprs(n: &mut Node, f: &mut dyn FnMut(&str) -> String) {
    fn attr(a: &mut Attr, f: &mut dyn FnMut(&str) -> String) {
        if let Some(w) = a.when.as_mut() {
            *w = f(w);
        }
        if let Some(v) = a.value.as_mut()
            && (v.contains("{{") || v.contains("{%"))
        {
            *v = rewrite_template(v, &mut |e, _| Some(f(e)));
        }
    }
    match n {
        Node::Out { expr } | Node::Set { expr, .. } | Node::Each { expr, .. } => *expr = f(expr),
        Node::If { cond, .. } => *cond = f(cond),
        Node::Element { attrs, .. } => {
            for a in attrs.iter_mut() {
                attr(a, f);
            }
        }
        Node::Use { args, attrs, .. } => {
            for e in args.values_mut() {
                *e = f(e);
            }
            for a in attrs.iter_mut() {
                attr(a, f);
            }
        }
        Node::Image(img) => {
            img.src = f(&img.src);
            for o in [
                &mut img.alt,
                &mut img.sizes,
                &mut img.class,
                &mut img.ratio,
                &mut img.loading,
                &mut img.widths,
            ] {
                if let Some(e) = o.as_mut() {
                    *e = f(e);
                }
            }
        }
        _ => {}
    }
}

/// Noms remplacés par des valeurs (`titre` -> `item.data.title`, `kicker` -> `(… | safe)`).
fn subst_names(e: &str, map: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut at = 0;
    for r in refs(e) {
        if let Some(v) = map.get(&r.name) {
            out.push_str(&e[at..r.start]);
            out.push_str(&grouped(v));
            at = r.end;
        }
    }
    out.push_str(&e[at..]);
    out
}

/// Identifiants nouveaux pour tous les nœuds identifiés.
fn renew_ids(nodes: &mut [Node], taken: &mut HashSet<String>) {
    for n in nodes {
        if let Some(slot) = revel_model::ids::node_id_mut(n)
            && slot.is_some()
        {
            let id = revel_model::ids::fresh(taken);
            taken.insert(id.clone());
            *slot = Some(id);
        }
        for list in revel_model::ids::child_lists_mut(n) {
            renew_ids(list, taken);
        }
    }
}

/// Identifiants pour les nœuds qui n'en ont pas (nœuds lus dans du HTML).
fn give_ids(nodes: &mut [Node], taken: &mut HashSet<String>) {
    for n in nodes {
        if let Some(slot) = revel_model::ids::node_id_mut(n)
            && slot.is_none()
        {
            let id = revel_model::ids::fresh(taken);
            taken.insert(id.clone());
            *slot = Some(id);
        }
        for list in revel_model::ids::child_lists_mut(n) {
            give_ids(list, taken);
        }
    }
}

fn blank(n: &Node) -> bool {
    matches!(n, Node::Text { text } if text.trim().is_empty())
        || matches!(n, Node::Html { html } if html.trim().is_empty())
}

// ------------------------------------------------------------------------------------------------
// Liste de données → collection

/// Liste de données lue par adresse dans une expression : (début, fin, éléments, valeur cherchée).
fn data_lookups(e: &str) -> Vec<(usize, usize, Vec<serde_json::Value>, String)> {
    let tail = regex::Regex::new(r#"^\s*\|\s*find_by\(\s*['"]slug['"]\s*,\s*([^()]+?)\s*\)"#)
        .expect("regex");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = e[from..].find("[{").map(|i| i + from) {
        let mut stream =
            serde_json::Deserializer::from_str(&e[i..]).into_iter::<serde_json::Value>();
        let Some(Ok(serde_json::Value::Array(items))) = stream.next() else {
            from = i + 2;
            continue;
        };
        let end = i + stream.byte_offset();
        // Liste entre parenthèses : `([…]) | find_by(…)`.
        let (start, after) = match (e[..i].trim_end().ends_with('('), e[end..].trim_start()) {
            (true, rest) if rest.starts_with(')') => {
                let open = e[..i].trim_end().len() - 1;
                (open, end + (e[end..].len() - rest.len()) + 1)
            }
            _ => (i, end),
        };
        if let Some(c) = tail.captures(&e[after..]) {
            let ok = !items.is_empty()
                && items.iter().all(|x| {
                    x.get("slug").and_then(|s| s.as_str()).is_some()
                        && x.as_object()
                            .is_some_and(|o| o.values().all(|v| !v.is_object() && !v.is_array()))
                });
            if ok {
                let m = c.get(0).map(|m| m.end()).unwrap_or(0);
                out.push((start, after + m, items, c[1].trim().to_string()));
            }
        }
        from = end;
    }
    out
}

/// Chemin d'une donnée du site égale à `v` (`rubriques.items`).
fn data_path(globals: &BTreeMap<String, Value>, v: &serde_json::Value) -> Option<String> {
    fn find(x: &serde_json::Value, v: &serde_json::Value, path: &mut Vec<String>) -> bool {
        if x == v {
            return true;
        }
        if let serde_json::Value::Object(m) = x {
            for (k, y) in m {
                path.push(k.clone());
                if find(y, v, path) {
                    return true;
                }
                path.pop();
            }
        }
        false
    }
    for (k, g) in globals {
        if k == "collections" {
            continue;
        }
        let Ok(x) = serde_json::to_value(g) else {
            continue;
        };
        let mut path = vec![k.clone()];
        if find(&x, v, &mut path) {
            return Some(path.join("."));
        }
    }
    None
}

/// Listes de données lues par adresse (`find_by('slug', …)`) changées en collections du CMS
/// (contenus sans page). Renvoie les collections créées ; le site n'est changé que si les
/// fichiers produits restent les mêmes avec ces contenus.
pub(super) fn data_collections(
    input: &BuildInput,
    globals: &BTreeMap<String, Value>,
    site: &mut Site,
    notes: &mut Vec<String>,
) -> Vec<NewCollection> {
    // Listes distinctes, dans l'ordre où elles apparaissent.
    let mut lists: Vec<Vec<serde_json::Value>> = Vec::new();
    for (_, _, body) in site.owners() {
        for e in exprs_of(body) {
            for (_, _, items, _) in data_lookups(&e) {
                if !lists.contains(&items) {
                    lists.push(items);
                }
            }
        }
    }
    let mut made = Vec::new();
    for items in lists {
        let value = serde_json::Value::Array(items.clone());
        let path = data_path(globals, &value);
        let base = path
            .as_deref()
            .and_then(|p| {
                p.split('.')
                    .rev()
                    .find(|s| !matches!(*s, "items" | "list" | "values" | "data" | "entries"))
            })
            .unwrap_or("liste");
        let base = revel_model::slugify(base).replace('-', "_");
        let mut id = base.clone();
        let mut k = 2;
        let used = |id: &str| {
            site.collections.iter().any(|c| c.id == id)
                || input
                    .entries
                    .iter()
                    .any(|e| e.folder == format!("content/{id}"))
                || made.iter().any(|m: &NewCollection| m.collection.id == id)
        };
        while used(&id) {
            id = format!("{base}{k}");
            k += 1;
        }
        let folder = format!("content/{id}");
        let mut fields: Vec<String> = Vec::new();
        for x in &items {
            if let Some(o) = x.as_object() {
                for k in o.keys() {
                    if !fields.contains(k) {
                        fields.push(k.clone());
                    }
                }
            }
        }
        // Ordre des champs : slug d'abord retiré de la vue, les autres dans l'ordre de la donnée.
        let entries: Vec<Entry> = items
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let mut data = x.clone();
                if let Some(o) = data.as_object_mut() {
                    o.insert("order".into(), serde_json::json!(i + 1));
                }
                Entry {
                    folder: folder.clone(),
                    file_stem: x["slug"].as_str().unwrap_or_default().to_string(),
                    data,
                    body: String::new(),
                }
            })
            .collect();
        // Lectures réécrites : `(collections.x | by_slug(s)).name` -> `….data.name`.
        let mut cand = site.clone();
        let rewrite = |e: &str| -> String {
            let found = data_lookups(e);
            if found.iter().all(|(_, _, it, _)| *it != items) {
                return e.to_string();
            }
            let mut out = String::new();
            let mut at = 0;
            for (s, z, it, arg) in &found {
                if *it != items {
                    continue;
                }
                out.push_str(&e[at..*s]);
                out.push_str(&format!("collections.{id} | by_slug({arg})"));
                at = *z;
            }
            out.push_str(&e[at..]);
            let member = regex::Regex::new(&format!(
                r"(\(\s*collections\.{} \| by_slug\([^()]*\)\s*\))\.([A-Za-z_]\w*)",
                regex::escape(&id)
            ))
            .expect("regex");
            member
                .replace_all(&out, |c: &regex::Captures| {
                    if &c[2] == "url" || &c[2] == "data" {
                        c[0].to_string()
                    } else {
                        format!("{}.data.{}", &c[1], &c[2])
                    }
                })
                .into_owned()
        };
        let fix = |body: &mut Vec<Node>| {
            walk_mut(body, &mut |n| map_exprs(n, &mut |e| rewrite(e)));
        };
        for l in &mut cand.layouts {
            fix(&mut l.body);
        }
        for p in &mut cand.pages {
            fix(&mut p.body);
        }
        for c in &mut cand.components {
            fix(&mut c.body);
        }
        cand.collections.push(revel_model::Collection {
            id: id.clone(),
            source: Default::default(),
            folder: folder.clone(),
            path: None,
            layout: None,
            props: Default::default(),
            filter: None,
            sort: Some(revel_model::Sort {
                field: "order".into(),
                descending: false,
                kind: revel_model::SortKind::Number,
            }),
            fields: Vec::new(),
        });
        let with = |extra: &[Entry]| BuildInput {
            site: input.site,
            entries: input.entries.iter().chain(extra.iter()).cloned().collect(),
            locales: input.locales.clone(),
            environment: input.environment.clone(),
            version: input.version.clone(),
            hooks: input.hooks.clone(),
            now: input.now,
        };
        let before = crate::build_all_envs(&with(&[]), site);
        let after = crate::build_all_envs(&with(&entries), &cand);
        if before.is_some() && before == after {
            *site = cand;
            let label = super::humanize(&id).unwrap_or_else(|| id.clone());
            notes.push(format!(
                "liste de données {} : collection « {label} » ({} contenus, lue par adresse)",
                path.as_deref().unwrap_or("écrite"),
                entries.len()
            ));
            made.push(NewCollection {
                collection: site
                    .collections
                    .iter()
                    .find(|c| c.id == id)
                    .cloned()
                    .unwrap_or_else(|| revel_model::Collection {
                        id: id.clone(),
                        source: Default::default(),
                        folder: folder.clone(),
                        path: None,
                        layout: None,
                        props: Default::default(),
                        filter: None,
                        sort: None,
                        fields: Vec::new(),
                    }),
                label,
                fields,
                entries,
            });
        }
    }
    made
}

// ------------------------------------------------------------------------------------------------
// Boucle sur un champ autour d'une liste de contenus reliés

/// `for s in F` dont le seul contenu est `for r in collections.x | by_slugs(s)` → `for r in
/// collections.x | by_slugs(F)`.
pub(super) fn fuse_nested_refs(nodes: &mut [Node]) -> usize {
    let inner_re =
        regex::Regex::new(r"^collections\.(\w+)\s*\|\s*by_slugs\(\s*([A-Za-z_]\w*)\s*\)$")
            .expect("regex");
    let mut n = 0;
    walk_mut(nodes, &mut |node| {
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
        let field = unwrap_parens(expr).to_string();
        if !empty.is_empty() || !revel_edit::visibility::is_path(&field) {
            return;
        }
        // Variables de la boucle qui ne servent plus (`set cat = collections.x | by_slug(s)`,
        // remplacée par la liste reliée) : ignorées.
        let kept: Vec<&Node> = children
            .iter()
            .filter(|c| !blank(c) && !matches!(c, Node::Set { .. }))
            .collect();
        if kept.len() != 1 || children.iter().any(|c| blank(c) && !is_empty_text(c)) {
            return;
        }
        let sets: Vec<String> = children
            .iter()
            .filter_map(|c| match c {
                Node::Set { name, .. } => Some(name.trim().to_string()),
                _ => None,
            })
            .collect();
        let Node::Each {
            expr: e2,
            item: r,
            children: body,
            empty: em2,
            ..
        } = kept[0]
        else {
            return;
        };
        let Some(c) = inner_re.captures(e2.trim()) else {
            return;
        };
        if c[2] != *item.trim() || !em2.is_empty() {
            return;
        }
        let reads_outer = exprs_of(body).iter().any(|e| {
            refs(e)
                .iter()
                .any(|x| x.name == item.trim() || x.name == "loop" || sets.contains(&x.name))
        });
        if reads_outer {
            return;
        }
        let new_expr = format!("collections.{} | by_slugs({field})", &c[1]);
        let (r, body) = (r.clone(), body.clone());
        *expr = new_expr;
        *item = r;
        *children = body;
        n += 1;
    });
    n
}

fn is_empty_text(n: &Node) -> bool {
    matches!(n, Node::Text { text } if text.is_empty())
        || matches!(n, Node::Html { html } if html.is_empty())
}

// ------------------------------------------------------------------------------------------------
// Composant déplié

/// Noms des listes construites pas à pas dans `nodes` (`set L = L + […]`).
fn accumulated(nodes: &[Node]) -> HashSet<String> {
    let mut out = HashSet::new();
    revel_model::ids::walk(nodes, &mut |n| {
        if let Node::Set { name, expr } = n {
            let parts = split_all(unwrap_parens(expr), '+');
            if parts.len() >= 2 && unwrap_parens(&parts[0]) == name.trim() {
                out.insert(name.trim().to_string());
            }
        }
    });
    out
}

/// Texte assemblé avec du HTML (`'<time …>' + d + '</time>'`, éventuellement sous condition).
fn html_assembly(e: &str) -> bool {
    let core = e.trim();
    let core = core
        .strip_suffix("| safe")
        .or_else(|| core.strip_suffix("|safe"))
        .unwrap_or(core);
    let core = unwrap_parens(core);
    let core = match split_kw_top(core, " if ") {
        Some((a, _)) => unwrap_parens(a),
        None => core,
    };
    let parts = split_all(core, '+');
    parts.len() >= 2
        && parts
            .iter()
            .any(|p| revel_edit::canvas::literal(p).is_some_and(|t| t.contains('<')))
}

/// Utilisations de composant dont une valeur est une liste construite pas à pas ou du HTML
/// assemblé : remplacées par le contenu du composant, propriétés remplacées par leurs valeurs.
pub(super) fn inline_uses(
    nodes: &mut Vec<Node>,
    components: &[revel_model::Component],
    taken: &mut HashSet<String>,
) -> usize {
    let acc = accumulated(nodes);
    inline_in(nodes, components, &acc, taken)
}

fn inline_in(
    nodes: &mut Vec<Node>,
    components: &[revel_model::Component],
    acc: &HashSet<String>,
    taken: &mut HashSet<String>,
) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i < nodes.len() {
        let replacement = match &nodes[i] {
            Node::Use {
                component,
                args,
                variant: None,
                attrs,
                ..
            } if attrs.is_empty() => {
                let wanted = args
                    .values()
                    .any(|a| acc.contains(unwrap_parens(a)) || html_assembly(a));
                match components.iter().find(|c| c.id == *component) {
                    Some(c) if wanted => expand(c, args, taken),
                    _ => None,
                }
            }
            _ => None,
        };
        match replacement {
            Some(body) => {
                let k = body.len();
                nodes.splice(i..=i, body);
                count += 1;
                i += k;
            }
            None => {
                for list in revel_model::ids::child_lists_mut(&mut nodes[i]) {
                    count += inline_in(list, components, acc, taken);
                }
                i += 1;
            }
        }
    }
    count
}

/// Contenu d'un composant pour une utilisation : propriétés remplacées par les valeurs données
/// (ou leur valeur par défaut), nouveaux identifiants.
fn expand(
    c: &revel_model::Component,
    args: &BTreeMap<String, String>,
    taken: &mut HashSet<String>,
) -> Option<Vec<Node>> {
    let props: HashSet<String> = c.props.iter().map(|p| p.name.clone()).collect();
    // Nom de propriété repris par une variable ou une boucle du composant : pas déplié.
    let mut clash = false;
    revel_model::ids::walk(&c.body, &mut |n| match n {
        Node::Set { name, .. } | Node::SetBlock { name, .. } => clash |= props.contains(name),
        Node::Each { item, .. } => {
            clash |= item.split(',').any(|v| props.contains(v.trim()));
        }
        Node::Use { component, .. } => clash |= *component == c.id,
        _ => {}
    });
    if clash {
        return None;
    }
    let mut map: HashMap<String, String> = HashMap::new();
    for p in &c.props {
        let v = match args.get(&p.name) {
            Some(e) => e.clone(),
            None => match &p.default {
                Some(d) => literal_of(&crate::eval::from_json(d))?,
                None => match p.kind {
                    PropKind::Switch => "false".into(),
                    PropKind::List => "[]".into(),
                    _ => "\"\"".into(),
                },
            },
        };
        map.insert(p.name.clone(), v);
    }
    let mut body = c.body.clone();
    walk_mut(&mut body, &mut |n| {
        map_exprs(n, &mut |e| tidy(&subst_names(e, &map)))
    });
    renew_ids(&mut body, taken);
    Some(body)
}

// ------------------------------------------------------------------------------------------------
// Liste construite pas à pas

/// Objet écrit `{ "label": a, "url": b }` : clé -> expression.
fn object_literal(e: &str) -> Option<BTreeMap<String, String>> {
    let t = unwrap_parens(e);
    let inner = t.strip_prefix('{')?.strip_suffix('}')?;
    let mut out = BTreeMap::new();
    for part in split_all(inner, ',') {
        if part.is_empty() {
            continue;
        }
        let (k, v) = split_kw_top(&part, ":")?;
        let key = revel_edit::canvas::literal(k.trim())
            .or_else(|| revel_model::valid_prop_name(k.trim()).then(|| k.trim().to_string()))?;
        out.insert(key, v.trim().to_string());
    }
    Some(out)
}

/// `t.cle…` remplacé par la valeur de la clé ; `None` si `t` est lu entier ou par une clé
/// absente suivie d'un chemin.
fn subst_member(e: &str, t: &str, obj: &BTreeMap<String, String>) -> Option<String> {
    let mut out = String::new();
    let mut at = 0;
    for r in refs(e) {
        if r.name != t {
            continue;
        }
        let path = &e[r.start..r.path_end];
        let rest = path.strip_prefix(t)?.strip_prefix('.')?;
        let (key, more) = match rest.split_once('.') {
            Some((k, m)) => (k, Some(m)),
            None => (rest, None),
        };
        let v = match (obj.get(key), more) {
            (Some(v), None) => grouped(v),
            (Some(v), Some(m)) => format!("{}.{m}", grouped(v)),
            (None, None) => "none".to_string(),
            (None, Some(_)) => return None,
        };
        out.push_str(&e[at..r.start]);
        out.push_str(&v);
        at = r.path_end;
    }
    out.push_str(&e[at..]);
    Some(out)
}

/// Condition « la liste construite a des éléments » : `L`, `L|length`, `L and L|length`.
fn is_list_test(c: &str, l: &str) -> bool {
    let norm = |s: &str| -> String {
        s.chars()
            .filter(|c| !c.is_whitespace() && *c != '(' && *c != ')')
            .collect()
    };
    let c = norm(c);
    c == l || c == format!("{l}|length") || c == format!("{l}and{l}|length")
}

/// Liste construite pas à pas puis parcourue, dans une même suite de nœuds (et ses enfants) :
/// parcourue directement.
pub(super) fn fuse_accumulations(nodes: &mut Vec<Node>) -> usize {
    let mut count = 0;
    while let Some(true) = fuse_one(nodes) {
        count += 1;
    }
    for n in nodes.iter_mut() {
        for list in revel_model::ids::child_lists_mut(n) {
            count += fuse_accumulations(list);
        }
    }
    count
}

/// Une liste fusionnée dans `nodes` : `Some(true)` ; rien à faire : `Some(false)`.
fn fuse_one(nodes: &mut Vec<Node>) -> Option<bool> {
    for i in 0..nodes.len() {
        let Node::Set { name: l, expr } = &nodes[i] else {
            continue;
        };
        if unwrap_parens(expr).trim() != "[]" {
            continue;
        }
        let l = l.trim().to_string();
        // Boucle qui construit la liste : la suivante qui écrit `L`.
        let Some(j) = (i + 1..nodes.len()).find(|&j| {
            matches!(&nodes[j], Node::Each { .. })
                && exprs_of(std::slice::from_ref(&nodes[j]))
                    .iter()
                    .any(|e| refs(e).iter().any(|r| r.name == l))
        }) else {
            continue;
        };
        let Node::Each {
            expr: source,
            item: x,
            children,
            empty,
            ..
        } = &nodes[j]
        else {
            continue;
        };
        if !empty.is_empty() {
            return Some(false);
        }
        let kept: Vec<&Node> = children.iter().filter(|c| !is_empty_text(c)).collect();
        let [Node::Set { name, expr: add }] = kept.as_slice() else {
            return Some(false);
        };
        if name.trim() != l {
            return Some(false);
        }
        let parts = split_all(unwrap_parens(add), '+');
        if parts.len() != 2 || unwrap_parens(&parts[0]) != l {
            return Some(false);
        }
        let one = parts[1].trim();
        let Some(obj) = one
            .strip_prefix('[')
            .and_then(|r| r.strip_suffix(']'))
            .and_then(object_literal)
        else {
            return Some(false);
        };
        let (source, x) = (source.clone(), x.trim().to_string());
        // Les autres lectures de `L` : conditions « a des éléments », boucles sur `L`.
        let rest: Vec<Node> = nodes
            .iter()
            .enumerate()
            .filter(|(k, _)| *k != i && *k != j)
            .map(|(_, n)| n.clone())
            .collect();
        let mut ok = true;
        revel_model::ids::walk(&rest, &mut |n| {
            let own = super::own_exprs(n);
            let reads = own.iter().any(|e| refs(e).iter().any(|r| r.name == l));
            if !reads {
                return;
            }
            match n {
                Node::If { cond, .. } if is_list_test(cond, &l) => {}
                Node::Each { expr, item, .. } if unwrap_parens(expr) == l => {
                    // Variable de la boucle construite déjà employée : pas de fusion.
                    ok &= item.trim() == x
                        || !own.iter().any(|e| refs(e).iter().any(|r| r.name == x));
                }
                Node::Set { name, .. } if name.trim() == l => ok = false,
                _ => ok = false,
            }
        });
        if !ok {
            return Some(false);
        }
        // Réécriture.
        let list_expr = unwrap_parens(&source).to_string();
        let mut failed = false;
        let mut out: Vec<Node> = Vec::new();
        for (k, n) in nodes.drain(..).enumerate() {
            if k == i || k == j {
                continue;
            }
            out.push(n);
        }
        walk_mut(&mut out, &mut |n| match n {
            Node::If { cond, .. } if is_list_test(cond, &l) => {
                *cond = format!("({list_expr})|length");
            }
            Node::Each {
                expr,
                item,
                children,
                empty,
                ..
            } if unwrap_parens(expr) == l => {
                let t = item.trim().to_string();
                *expr = list_expr.clone();
                *item = x.clone();
                walk_mut(children, &mut |c| {
                    map_exprs(c, &mut |e| match subst_member(e, &t, &obj) {
                        Some(s) => tidy(&s),
                        None => {
                            failed = true;
                            e.to_string()
                        }
                    })
                });
                walk_mut(empty, &mut |c| {
                    map_exprs(c, &mut |e| match subst_member(e, &t, &obj) {
                        Some(s) => tidy(&s),
                        None => {
                            failed = true;
                            e.to_string()
                        }
                    })
                });
            }
            _ => {}
        });
        *nodes = out;
        if failed {
            // Lecture que la fusion ne sait pas écrire : le rendu vérifié refusera la réécriture.
            return Some(true);
        }
        fold_const_ifs(nodes);
        return Some(true);
    }
    Some(false)
}

// ------------------------------------------------------------------------------------------------
// Conditions connues d'avance

/// Condition toujours vraie ou toujours fausse : `none`, `(1) == 1`, texte assemblé avec un
/// morceau écrit non vide (`'<time…' + d`).
fn const_cond(c: &str) -> Option<bool> {
    let t = unwrap_parens(c);
    if refs(t).is_empty() {
        let v = minijinja::Environment::new()
            .compile_expression(t)
            .ok()?
            .eval(())
            .ok()?;
        return Some(v.is_true());
    }
    let parts = split_all(t, '+');
    (parts.len() >= 2
        && parts
            .iter()
            .any(|p| revel_edit::canvas::literal(p).is_some_and(|s| !s.is_empty())))
    .then_some(true)
}

/// Conditions connues d'avance remplacées par leur contenu.
pub(super) fn fold_const_ifs(nodes: &mut Vec<Node>) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i < nodes.len() {
        if let Node::If {
            cond,
            then,
            otherwise,
            ..
        } = &nodes[i]
            && let Some(v) = const_cond(cond)
        {
            let keep = if v { then.clone() } else { otherwise.clone() };
            let k = keep.len();
            nodes.splice(i..=i, keep);
            count += 1;
            // Les nœuds remis en place sont relus (conditions imbriquées).
            let _ = k;
            continue;
        }
        for list in revel_model::ids::child_lists_mut(&mut nodes[i]) {
            count += fold_const_ifs(list);
        }
        i += 1;
    }
    count
}

// ------------------------------------------------------------------------------------------------
// HTML assemblé

/// `a + b + c` dont des morceaux sont du HTML écrit : gabarit (`<time datetime="{{ d }}">`).
fn assembly_template(e: &str) -> Option<String> {
    let parts = split_all(unwrap_parens(e), '+');
    if parts.len() < 2 {
        return None;
    }
    let mut out = String::new();
    let mut html = false;
    for p in &parts {
        match revel_edit::canvas::literal(p) {
            Some(t) => {
                html |= t.contains('<');
                if t.contains("{{") || t.contains("{%") {
                    return None;
                }
                out.push_str(&t);
            }
            None => out.push_str(&format!("{{{{ {} }}}}", unwrap_parens(p))),
        }
    }
    html.then_some(out)
}

/// Texte affiché tel quel (`… | safe`) : (expression sans `| safe`).
fn safe_core(e: &str) -> Option<&str> {
    let t = unwrap_parens(e.trim());
    t.strip_suffix("| safe")
        .or_else(|| t.strip_suffix("|safe"))
        .map(|c| unwrap_parens(c.trim()))
}

/// `(A if c else "")` : (A, c).
fn guarded(e: &str) -> Option<(String, String)> {
    let t = unwrap_parens(e);
    let (a, rest) = split_kw_top(t, " if ")?;
    let (c, b) = split_kw_top(rest, " else ")?;
    (revel_edit::canvas::literal(b.trim()).is_some_and(|s| s.is_empty()))
        .then(|| (unwrap_parens(a).to_string(), c.trim().to_string()))
}

fn markup(template: &str, taken: &mut HashSet<String>) -> Option<Vec<Node>> {
    let parsed = revel_markup::parse(template, &revel_markup::Options::default()).ok()?;
    if !parsed.warnings.is_empty() {
        return None;
    }
    let mut nodes = parsed.nodes;
    give_ids(&mut nodes, taken);
    Some(nodes)
}

/// HTML assemblé dans un texte affiché tel quel → éléments ; la condition « texte rempli » qui
/// l'entoure devient la sienne.
pub(super) fn html_strings(nodes: &mut Vec<Node>, taken: &mut HashSet<String>) -> usize {
    let mut count = 0;
    // Conditions sur le même texte : `{% if K %}…{{ K | safe }}…{% endif %}`.
    walk_mut(nodes, &mut |n| {
        let Node::If { cond, then, .. } = n else {
            return;
        };
        let k = unwrap_parens(cond).to_string();
        if !html_assembly(&k) {
            return;
        }
        let new_cond = match guarded(&k) {
            Some((_, c)) => c,
            None if const_cond(&k) == Some(true) => "true".to_string(),
            None => return,
        };
        // Le texte lu dans la condition : sans la condition interne (déjà vérifiée).
        let inner = guarded(&k).map(|(a, _)| a);
        let mut seen = false;
        walk_mut(then, &mut |x| {
            if let Node::Out { expr } = x
                && safe_core(expr) == Some(k.as_str())
                && let Some(a) = &inner
            {
                *expr = format!("({a}) | safe");
                seen = true;
            } else if let Node::Out { expr } = x
                && safe_core(expr) == Some(k.as_str())
            {
                seen = true;
            }
        });
        if seen {
            *cond = new_cond;
        }
    });
    // Textes assemblés → éléments.
    count += splice_markup(nodes, taken);
    count += fold_const_ifs(nodes);
    count
}

fn splice_markup(nodes: &mut Vec<Node>, taken: &mut HashSet<String>) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i < nodes.len() {
        let rep = match &nodes[i] {
            Node::Out { expr } => safe_core(expr).and_then(|core| {
                if let Some(t) = assembly_template(core) {
                    markup(&t, taken)
                } else if let Some((a, c)) = guarded(core)
                    && let Some(t) = assembly_template(&a)
                {
                    let then = markup(&t, taken)?;
                    let id = revel_model::ids::fresh(taken);
                    taken.insert(id.clone());
                    Some(vec![Node::If {
                        id: Some(id),
                        cond: c,
                        then,
                        otherwise: Vec::new(),
                    }])
                } else {
                    None
                }
            }),
            _ => None,
        };
        match rep {
            Some(new) => {
                let k = new.len();
                nodes.splice(i..=i, new);
                count += 1;
                i += k;
            }
            None => {
                for list in revel_model::ids::child_lists_mut(&mut nodes[i]) {
                    count += splice_markup(list, taken);
                }
                i += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formes_composees() {
        let found = data_lookups(
            r#"(([{"slug":"a","name":"A","color":"orange"},{"slug":"b","name":"B","color":"purple"}]) | find_by('slug', slug)).name"#,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].3, "slug");
        assert_eq!(found[0].2.len(), 2);
        assert_eq!(
            object_literal(r##"{ "label": ex.data.title, "url": ex.url, "color": ({"a":"#fff","b":"#000"})[ex.data.color] }"##)
                .unwrap()
                .len(),
            3
        );
        let obj = object_literal(r#"{ "label": ex.data.title }"#).unwrap();
        assert_eq!(
            subst_member("t.label", "t", &obj).as_deref(),
            Some("ex.data.title")
        );
        assert_eq!(subst_member("t.url", "t", &obj).as_deref(), Some("none"));
        assert!(subst_member("t", "t", &obj).is_none());
        assert!(is_list_test("(heroTags) and (heroTags)|length", "heroTags"));
        assert_eq!(
            assembly_template(
                r#"'<time datetime="' + (d | iso_date) + '">' + (d | date_fr) + '</time>'"#
            )
            .as_deref(),
            Some(r#"<time datetime="{{ d | iso_date }}">{{ d | date_fr }}</time>"#)
        );
        assert_eq!(
            guarded(r#"(("pour <strong>" + client + "</strong>") if client else "")"#),
            Some((
                r#""pour <strong>" + client + "</strong>""#.to_string(),
                "client".to_string()
            ))
        );
        assert_eq!(const_cond("(1) == 2"), Some(false));
        assert_eq!(const_cond("'<b>' + x"), Some(true));
        assert_eq!(const_cond("x"), None);
    }
}

#[cfg(test)]
mod tests_nested {
    use super::*;

    #[test]
    fn boucles_reliees_reunies() {
        let mut nodes: Vec<Node> = serde_json::from_str(r#"[{"node": "each", "id": "u7oatvas", "expr": "categories", "item": "slug", "children": [{"node": "each", "id": "g9waedbx", "expr": "collections.rubriques | by_slugs(slug)", "item": "ref", "children": [{"node": "text", "text": "\n    "}, {"node": "element", "id": "aaaa", "tag": "li", "children": [{"node": "out", "expr": "ref.data.name"}]}]}]}]"#).unwrap();
        assert_eq!(fuse_nested_refs(&mut nodes), 1);
        let Node::Each { expr, item, .. } = &nodes[0] else {
            panic!()
        };
        assert_eq!(expr, "collections.rubriques | by_slugs(categories)");
        assert_eq!(item, "ref");
    }
}
