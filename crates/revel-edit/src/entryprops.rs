//! Composants reliés au CMS (D-191) : la propriété « Contenu » (collection fixée dans le composant,
//! champs lus dedans : `projet.data.title`) devient une propriété par champ lu (`title`, `image`,
//! adresse de la page). Le composant ne connaît plus de collection ; chaque utilisation dit d'où
//! viennent ses valeurs (`item.data.title`, `(collections.projets | by_slug("x")).data.title`, ou
//! une valeur écrite). Même rendu avant et après.

use crate::{Command, Owner, Target};
use revel_model::{Attr, Component, ImageNode, Node, PropDef, PropKind, Site};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Champ d'une collection, pour nommer et typer la propriété qui le remplace.
#[derive(Debug, Clone, Default)]
pub struct FieldInfo {
    pub label: Option<String>,
    /// Nature du champ dans le CMS (`image`, `boolean`, `string`…).
    pub kind: Option<String>,
    pub list: bool,
    /// Valeur du premier contenu : valeur par défaut de la propriété (aperçu du composant).
    pub sample: Option<Value>,
    /// Une valeur remplie d'un contenu quelconque : dit la nature du champ (liste, oui / non)
    /// quand le CMS ne la donne pas.
    pub example: Option<Value>,
}

/// Collection : ses champs et l'adresse de la page de son premier contenu.
#[derive(Debug, Clone, Default)]
pub struct CollectionInfo {
    pub fields: BTreeMap<String, FieldInfo>,
    pub url: Option<String>,
}

/// Ce qu'une expression lit dans le contenu.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Read {
    Field(String),
    Url,
}

fn ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Réécrit dans l'expression `expr` les lectures de la variable `var` (`var.data.champ`,
/// `var.url`) par `f`. `Err` : `var` est lu autrement (contenu entier passé plus loin, autre
/// attribut), le composant ne peut pas être converti.
fn rewrite_expr(expr: &str, var: &str, f: &mut dyn FnMut(Read) -> String) -> Result<String, ()> {
    let cs: Vec<char> = expr.chars().collect();
    let v: Vec<char> = var.chars().collect();
    let mut out = String::with_capacity(expr.len());
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        // Texte entre guillemets : laissé tel quel.
        if c == '"' || c == '\'' {
            out.push(c);
            i += 1;
            while i < cs.len() {
                out.push(cs[i]);
                if cs[i] == '\\' && i + 1 < cs.len() {
                    out.push(cs[i + 1]);
                    i += 2;
                    continue;
                }
                i += 1;
                if cs[i - 1] == c {
                    break;
                }
            }
            continue;
        }
        let starts = cs[i..].starts_with(&v)
            && (i == 0 || !(ident(cs[i - 1]) || cs[i - 1] == '.'))
            && cs.get(i + v.len()).is_none_or(|n| !ident(*n));
        if !starts {
            // Identifiant entier sauté d'un coup (`sousprojet` ne contient pas `projet`).
            if ident(c) {
                while i < cs.len() && ident(cs[i]) {
                    out.push(cs[i]);
                    i += 1;
                }
            } else {
                out.push(c);
                i += 1;
            }
            continue;
        }
        let rest: String = cs[i + v.len()..].iter().collect();
        let word = |s: &str| -> String { s.chars().take_while(|c| ident(*c)).collect() };
        if let Some(r) = rest.strip_prefix(".data.") {
            let name = word(r);
            if name.is_empty() {
                return Err(());
            }
            i += v.len() + ".data.".len() + name.chars().count();
            out.push_str(&f(Read::Field(name)));
        } else if let Some(r) = rest.strip_prefix(".url")
            && r.chars().next().is_none_or(|c| !ident(c) && c != '.')
        {
            i += v.len() + ".url".len();
            out.push_str(&f(Read::Url));
        } else {
            return Err(());
        }
    }
    Ok(out)
}

/// Même réécriture dans un texte à expressions (`{{ … }}`, valeur d'attribut).
fn rewrite_template(
    text: &str,
    var: &str,
    f: &mut dyn FnMut(Read) -> String,
) -> Result<String, ()> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(a) = rest.find("{{") {
        let Some(b) = rest[a..].find("}}") else {
            break;
        };
        out.push_str(&rest[..a + 2]);
        out.push_str(&rewrite_expr(&rest[a + 2..a + b], var, f)?);
        out.push_str("}}");
        rest = &rest[a + b + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

fn attrs_mut(attrs: &mut [Attr], var: &str, f: &mut dyn FnMut(Read) -> String) -> Result<(), ()> {
    for a in attrs {
        if let Some(v) = &mut a.value {
            *v = rewrite_template(v, var, f)?;
        }
        if let Some(w) = &mut a.when {
            *w = rewrite_expr(w, var, f)?;
        }
    }
    Ok(())
}

fn image_mut(img: &mut ImageNode, var: &str, f: &mut dyn FnMut(Read) -> String) -> Result<(), ()> {
    img.src = rewrite_expr(&img.src, var, f)?;
    for x in [
        &mut img.alt,
        &mut img.sizes,
        &mut img.class,
        &mut img.ratio,
        &mut img.loading,
        &mut img.widths,
    ]
    .into_iter()
    .flatten()
    {
        *x = rewrite_expr(x, var, f)?;
    }
    Ok(())
}

/// Toutes les expressions des nœuds, réécrites. Une boucle ou une affectation qui reprend le nom
/// `var` le masquerait : refusé.
fn nodes_mut(nodes: &mut [Node], var: &str, f: &mut dyn FnMut(Read) -> String) -> Result<(), ()> {
    for n in nodes {
        match n {
            Node::Element {
                attrs, children, ..
            } => {
                attrs_mut(attrs, var, f)?;
                nodes_mut(children, var, f)?;
            }
            Node::Out { expr } => *expr = rewrite_expr(expr, var, f)?,
            Node::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                *cond = rewrite_expr(cond, var, f)?;
                nodes_mut(then, var, f)?;
                nodes_mut(otherwise, var, f)?;
            }
            Node::Each {
                expr,
                item,
                children,
                empty,
                ..
            } => {
                if item.split(',').any(|x| x.trim() == var) {
                    return Err(());
                }
                *expr = rewrite_expr(expr, var, f)?;
                nodes_mut(children, var, f)?;
                nodes_mut(empty, var, f)?;
            }
            Node::Set { name, expr } => {
                if name == var {
                    return Err(());
                }
                *expr = rewrite_expr(expr, var, f)?;
            }
            Node::SetBlock { name, children, .. } => {
                if name == var {
                    return Err(());
                }
                nodes_mut(children, var, f)?;
            }
            Node::Use {
                args,
                attrs,
                children,
                ..
            } => {
                for a in args.values_mut() {
                    *a = rewrite_expr(a, var, f)?;
                }
                attrs_mut(attrs, var, f)?;
                nodes_mut(children, var, f)?;
            }
            Node::Image(img) => image_mut(img, var, f)?,
            Node::Text { .. } | Node::Html { .. } | Node::Slot => {}
        }
    }
    Ok(())
}

/// Noms que le composant ne peut pas donner à une propriété : variables de ses boucles et
/// affectations, données et variables du site.
fn reserved(site: &Site, c: &Component, var: &str) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = [
        "collections",
        "env",
        "version",
        "locale",
        "loop",
        "item",
        "range",
        "namespace",
        "dict",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    out.extend(site.data.keys().cloned());
    out.extend(
        c.props
            .iter()
            .filter(|p| p.name != var)
            .map(|p| p.name.clone()),
    );
    revel_model::ids::walk(&c.body, &mut |n| match n {
        Node::Each { item, .. } => out.extend(item.split(',').map(|x| x.trim().to_string())),
        Node::Set { name, .. } | Node::SetBlock { name, .. } => {
            out.insert(name.clone());
        }
        _ => {}
    });
    out
}

/// Nom de propriété libre le plus proche de `wanted`.
fn free_name(wanted: &str, taken: &BTreeSet<String>) -> String {
    let mut base: String = wanted
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if !revel_model::valid_prop_name(&base) {
        base = format!("champ_{base}");
    }
    if !taken.contains(&base) {
        return base;
    }
    (2..)
        .map(|i| format!("{base}_{i}"))
        .find(|n| !taken.contains(n))
        .unwrap_or(base)
}

/// Chemin simple (`item`, `a.b`) : se prolonge sans parenthèses.
fn is_path(e: &str) -> bool {
    !e.is_empty() && e.chars().all(|c| ident(c) || c == '.')
}

/// Valeur d'une utilisation pour une lecture du contenu `entry`.
fn arg_of(entry: &str, read: &Read) -> String {
    let e = entry.trim();
    let head = if is_path(e) {
        e.to_string()
    } else {
        format!("({e})")
    };
    match read {
        Read::Field(f) => format!("{head}.data.{f}"),
        Read::Url => format!("{head}.url"),
    }
}

/// Propriété qui remplace une lecture du contenu.
fn prop_for(
    name: String,
    read: &Read,
    info: Option<&CollectionInfo>,
    page_label: &str,
    href: bool,
    src: bool,
) -> PropDef {
    match read {
        Read::Url => PropDef {
            options: Vec::new(),
            name,
            label: Some(page_label.to_string()),
            kind: PropKind::Link,
            default: Some(Value::String(
                info.and_then(|c| c.url.clone())
                    .unwrap_or_else(|| "#".into()),
            )),
            collection: None,
        },
        Read::Field(f) => {
            let fi = info.and_then(|c| c.fields.get(f));
            // Nature : celle du champ dans le CMS, sinon sa valeur (liste, oui / non), sinon
            // son usage dans le composant (source d'une image, adresse d'un lien).
            let sample = fi.and_then(|x| x.example.as_ref().or(x.sample.as_ref()));
            let kind = match fi {
                Some(x) if x.list => PropKind::List,
                _ if matches!(sample, Some(Value::Array(_))) => PropKind::List,
                Some(x) if x.kind.as_deref() == Some("image") => PropKind::Image,
                Some(x) if x.kind.as_deref() == Some("boolean") => PropKind::Switch,
                _ if matches!(sample, Some(Value::Bool(_))) => PropKind::Switch,
                _ if src => PropKind::Image,
                _ if href => PropKind::Link,
                _ => PropKind::Text,
            };
            let label = fi
                .and_then(|x| x.label.clone())
                .unwrap_or_else(|| crate::overrides::field_label(f));
            let default = match (kind, fi.and_then(|x| x.sample.clone())) {
                (PropKind::Switch | PropKind::List, _) => None,
                (_, Some(Value::String(s))) if !s.is_empty() => Some(Value::String(s)),
                (_, Some(Value::Number(n))) => Some(Value::String(n.to_string())),
                _ => None,
            };
            PropDef {
                options: Vec::new(),
                name,
                label: Some(label),
                kind,
                default,
                collection: None,
            }
        }
    }
}

/// Lectures qui sont, seules, la valeur de l'attribut `attr` (`href="{{ p.data.link }}"`).
fn attr_reads(body: &[Node], var: &str, attr: &str) -> BTreeSet<Read> {
    let mut out = BTreeSet::new();
    revel_model::ids::walk(body, &mut |n| {
        if let Node::Element { attrs, .. } = n {
            for a in attrs.iter().filter(|a| a.name == attr) {
                let Some(v) = a.value.as_deref() else {
                    continue;
                };
                let inner = v
                    .trim()
                    .strip_prefix("{{")
                    .and_then(|x| x.strip_suffix("}}"));
                if let Some(e) = inner
                    && let Some(f) = e.trim().strip_prefix(&format!("{var}.data."))
                    && f.chars().all(ident)
                {
                    out.insert(Read::Field(f.to_string()));
                }
            }
        }
    });
    out
}

/// Convertit le composant d'indice `ci` pour sa propriété « Contenu » `var` dans `s`. `false` :
/// laissé tel quel (contenu lu autrement que par ses champs, utilisation sans identifiant).
fn convert_one(
    s: &mut Site,
    ci: usize,
    var: &str,
    schema: &BTreeMap<String, CollectionInfo>,
    page_label: &str,
) -> bool {
    let c = s.components[ci].clone();
    let Some(ent) = c.props.iter().find(|p| p.name == var) else {
        return false;
    };
    let collection = ent.collection.clone().unwrap_or_default();
    let info = schema.get(&collection);
    // Lectures, dans l'ordre du composant ; nom de propriété de chacune.
    let mut taken = reserved(s, &c, var);
    let mut names: Vec<(Read, String)> = Vec::new();
    let mut body = c.body.clone();
    let done = nodes_mut(&mut body, var, &mut |r| {
        if let Some((_, n)) = names.iter().find(|(x, _)| *x == r) {
            return n.clone();
        }
        let wanted = match &r {
            Read::Field(f) => f.as_str(),
            Read::Url => "lien_page",
        };
        let n = free_name(wanted, &taken);
        taken.insert(n.clone());
        names.push((r, n.clone()));
        n
    });
    if done.is_err() {
        return false;
    }
    // Utilisations : toutes doivent pouvoir recevoir leurs valeurs.
    let id = c.id.clone();
    let default_entry = ent
        .entry_expr(None)
        .unwrap_or_else(|| format!("(collections.{collection})[0]"));
    fn uses_mut(
        nodes: &mut [Node],
        id: &str,
        f: &mut dyn FnMut(&mut std::collections::BTreeMap<String, String>),
    ) {
        for n in nodes {
            match n {
                Node::Use {
                    component,
                    args,
                    children,
                    ..
                } => {
                    if component == id {
                        f(args);
                    }
                    uses_mut(children, id, f);
                }
                Node::Element { children, .. } | Node::SetBlock { children, .. } => {
                    uses_mut(children, id, f)
                }
                Node::If {
                    then, otherwise, ..
                } => {
                    uses_mut(then, id, f);
                    uses_mut(otherwise, id, f);
                }
                Node::Each {
                    children, empty, ..
                } => {
                    uses_mut(children, id, f);
                    uses_mut(empty, id, f);
                }
                _ => {}
            }
        }
    }
    let hrefs = attr_reads(&c.body, var, "href");
    let srcs = attr_reads(&c.body, var, "src");
    let mut props: Vec<PropDef> = Vec::new();
    for p in &c.props {
        if p.name == var {
            props.extend(names.iter().map(|(r, n)| {
                prop_for(
                    n.clone(),
                    r,
                    info,
                    page_label,
                    hrefs.contains(r),
                    srcs.contains(r),
                )
            }));
        } else {
            props.push(p.clone());
        }
    }
    s.components[ci].body = body;
    s.components[ci].props = props;
    let mut fill = |args: &mut BTreeMap<String, String>| {
        let entry = args.remove(var).unwrap_or_else(|| default_entry.clone());
        for (r, n) in &names {
            args.entry(n.clone()).or_insert_with(|| arg_of(&entry, r));
        }
    };
    for p in &mut s.pages {
        uses_mut(&mut p.body, &id, &mut fill);
    }
    for l in &mut s.layouts {
        uses_mut(&mut l.body, &id, &mut fill);
    }
    for k in &mut s.components {
        uses_mut(&mut k.body, &id, &mut fill);
    }
    true
}

/// Commande qui convertit tous les composants à propriété « Contenu » du site (une seule étape
/// d'annulation). `None` : rien à convertir. `page_label` : libellé de la propriété qui reçoit
/// l'adresse de la page du contenu.
pub fn convert(
    site: &Site,
    schema: &BTreeMap<String, CollectionInfo>,
    page_label: &str,
) -> Option<Command> {
    let mut s = site.clone();
    let mut any = false;
    for ci in 0..s.components.len() {
        let vars: Vec<String> = s.components[ci]
            .props
            .iter()
            .filter(|p| p.kind == PropKind::Entry)
            .map(|p| p.name.clone())
            .collect();
        for var in vars {
            any |= convert_one(&mut s, ci, &var, schema, page_label);
        }
    }
    // Reste d'import : carte d'une liste filtrée qui porte aussi le rôle de bouton de filtre
    // (`data-revel-filter=""`), sans effet et trompeur dans le panneau.
    let mut strays: Vec<Command> = Vec::new();
    let tagged = |id: &str| {
        site.component(id).is_some_and(|c| {
            c.body.iter().any(|n| {
                matches!(n, Node::Element { attrs, .. } if attrs.iter().any(|a| a.name == "data-revel-tags"))
            })
        })
    };
    for (owner, nodes) in site
        .pages
        .iter()
        .map(|p| (Owner::Page(p.id.clone()), &p.body))
        .chain(
            site.layouts
                .iter()
                .map(|l| (Owner::Layout(l.id.clone()), &l.body)),
        )
    {
        revel_model::ids::walk(nodes, &mut |n| {
            if let Node::Use {
                id: Some(id),
                component,
                attrs,
                ..
            } = n
                && attrs.iter().any(|a| {
                    a.name == "data-revel-filter" && a.value.as_deref().unwrap_or("").is_empty()
                })
                && tagged(component)
            {
                strays.push(Command::RemoveAttr {
                    at: Target {
                        owner: owner.clone(),
                        node: id.clone(),
                    },
                    name: "data-revel-filter".into(),
                });
            }
        });
    }
    if !any && strays.is_empty() {
        return None;
    }
    Some(Command::Batch {
        commands: changes(site, &s, strays),
    })
}

/// Commandes qui mènent de `site` à `s` : propriétés et contenu des composants, valeurs des
/// utilisations dans les pages et les gabarits (après `first`).
fn changes(site: &Site, s: &Site, first: Vec<Command>) -> Vec<Command> {
    let mut commands = first;
    for (a, b) in site.components.iter().zip(&s.components) {
        if a.props != b.props {
            commands.push(Command::SetComponentProps {
                id: b.id.clone(),
                props: b.props.clone(),
            });
        }
        if a.body != b.body {
            commands.push(Command::SetComponentBody {
                id: b.id.clone(),
                body: b.body.clone(),
            });
        }
    }
    // Pages et gabarits : valeurs des utilisations, une à une.
    let mut uses = |owner: Owner, before: &[Node], after: &[Node]| {
        let mut old: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        revel_model::ids::walk(before, &mut |n| {
            if let Node::Use {
                id: Some(id), args, ..
            } = n
            {
                old.insert(id.clone(), args.clone());
            }
        });
        revel_model::ids::walk(after, &mut |n| {
            if let Node::Use {
                id: Some(id), args, ..
            } = n
                && let Some(o) = old.get(id)
                && o != args
            {
                let at = Target {
                    owner: owner.clone(),
                    node: id.clone(),
                };
                for k in o.keys().filter(|k| !args.contains_key(*k)) {
                    commands.push(Command::SetUseArg {
                        at: at.clone(),
                        name: k.clone(),
                        value: None,
                    });
                }
                for (k, v) in args.iter().filter(|(k, v)| o.get(*k) != Some(*v)) {
                    commands.push(Command::SetUseArg {
                        at: at.clone(),
                        name: k.clone(),
                        value: Some(v.clone()),
                    });
                }
            }
        });
    };
    for (a, b) in site.pages.iter().zip(&s.pages) {
        uses(Owner::Page(b.id.clone()), &a.body, &b.body);
    }
    for (a, b) in site.layouts.iter().zip(&s.layouts) {
        uses(Owner::Layout(b.id.clone()), &a.body, &b.body);
    }
    commands
}

/// Composant tout juste créé à partir d'une carte de liste (audit du 23/11/2026, D-193) : ses
/// lectures du contenu de la liste (`item.data.title`, `item.url`) deviennent des propriétés, et
/// ses utilisations dans la liste leur donnent `item.data.title`… Le composant ne dépend plus de
/// la liste : posé ailleurs, il prend des valeurs écrites ou celles d'un contenu choisi. Rendu
/// inchangé. `None` : le composant ne lit rien du contenu `var`. Rend aussi le nombre de
/// propriétés créées.
pub fn lift_list_reads(
    site: &Site,
    component: &str,
    var: &str,
    collection: &str,
    schema: &BTreeMap<String, CollectionInfo>,
    page_label: &str,
) -> Option<(Vec<Command>, usize)> {
    let ci = site.components.iter().position(|c| c.id == component)?;
    let before = site.components[ci].props.len();
    let mut s = site.clone();
    // Propriété « Contenu » provisoire, que la conversion de D-191 remplace par une propriété par
    // lecture ; les utilisations lui donnent l'élément de la liste.
    s.components[ci].props.push(PropDef {
        options: Vec::new(),
        name: var.to_string(),
        label: None,
        kind: PropKind::Entry,
        default: None,
        collection: Some(collection.to_string()),
    });
    fn give(nodes: &mut [Node], id: &str, var: &str) {
        for n in nodes {
            match n {
                Node::Use {
                    component,
                    args,
                    children,
                    ..
                } => {
                    if component == id {
                        args.entry(var.to_string())
                            .or_insert_with(|| var.to_string());
                    }
                    give(children, id, var);
                }
                Node::Element { children, .. } | Node::SetBlock { children, .. } => {
                    give(children, id, var)
                }
                Node::If {
                    then, otherwise, ..
                } => {
                    give(then, id, var);
                    give(otherwise, id, var);
                }
                Node::Each {
                    children, empty, ..
                } => {
                    give(children, id, var);
                    give(empty, id, var);
                }
                _ => {}
            }
        }
    }
    for p in &mut s.pages {
        give(&mut p.body, component, var);
    }
    for l in &mut s.layouts {
        give(&mut l.body, component, var);
    }
    if !convert_one(&mut s, ci, var, schema, page_label) {
        return None;
    }
    let made = s.components[ci].props.len().saturating_sub(before);
    if made == 0
        || s.components[ci]
            .props
            .iter()
            .any(|p| p.kind == PropKind::Entry && p.name == var)
    {
        return None;
    }
    Some((changes(site, &s, Vec::new()), made))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Carte de liste devenue composant : une propriété par lecture, valeurs `item.…` dans la
    /// liste, composant sans référence à la liste.
    #[test]
    fn carte_de_liste_devenue_composant() {
        let mut site = Site::new("Essai", "fr");
        site.components = serde_json::from_value(serde_json::json!([{
            "id": "carte",
            "body": [{"node": "element", "id": "a1", "tag": "article", "children": [
                {"node": "element", "id": "a2", "tag": "img", "attrs": [{"name": "src", "value": "{{ item.data.image }}"}]},
                {"node": "element", "id": "a3", "tag": "h3", "children": [{"node": "out", "expr": "item.data.title"}]},
                {"node": "element", "id": "a4", "tag": "a", "attrs": [{"name": "href", "value": "{{ item.url }}"}],
                 "children": [{"node": "text", "text": "Voir"}]}
            ]}]
        }]))
        .unwrap();
        site.pages = serde_json::from_value(serde_json::json!([{"id": "index", "path": "/", "body": [
            {"node": "each", "id": "p1", "expr": "collections.projets", "item": "item", "children": [
                {"node": "use", "id": "u1", "component": "carte"}
            ]}
        ]}]))
        .unwrap();
        let mut schema = BTreeMap::new();
        let mut fields = BTreeMap::new();
        fields.insert(
            "title".to_string(),
            FieldInfo {
                label: Some("Titre".into()),
                sample: Some(Value::String("Maison Oka".into())),
                ..Default::default()
            },
        );
        fields.insert(
            "image".to_string(),
            FieldInfo {
                kind: Some("image".into()),
                ..Default::default()
            },
        );
        schema.insert(
            "projets".to_string(),
            CollectionInfo {
                fields,
                url: Some("/projets/maison-oka/".into()),
            },
        );
        let (cmds, n) = lift_list_reads(
            &site,
            "carte",
            "item",
            "projets",
            &schema,
            "Page du contenu",
        )
        .unwrap();
        assert_eq!(n, 3);
        for c in &cmds {
            crate::apply(&mut site, c).unwrap();
        }
        let c = site.component("carte").unwrap();
        let names: Vec<&str> = c.props.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["image", "title", "lien_page"]);
        assert_eq!(c.props[0].kind, PropKind::Image);
        assert_eq!(c.props[1].label.as_deref(), Some("Titre"));
        assert_eq!(c.props[2].kind, PropKind::Link);
        let body = serde_json::to_string(&c.body).unwrap();
        assert!(!body.contains("item."), "{body}");
        let mut args = None;
        revel_model::ids::walk(&site.pages[0].body, &mut |n| {
            if let Node::Use { args: a, .. } = n {
                args = Some(a.clone());
            }
        });
        let args = args.unwrap();
        assert_eq!(
            args.get("title").map(String::as_str),
            Some("item.data.title")
        );
        assert_eq!(args.get("lien_page").map(String::as_str), Some("item.url"));
        assert!(!args.contains_key("item"));
        // Composant qui ne lit rien du contenu : rien à faire.
        assert!(lift_list_reads(&site, "carte", "item", "projets", &schema, "x").is_none());
    }

    fn rw(e: &str) -> Result<String, ()> {
        rewrite_expr(e, "p", &mut |r| match r {
            Read::Field(f) => format!("F_{f}"),
            Read::Url => "U".into(),
        })
    }

    #[test]
    fn lectures_reecrites() {
        assert_eq!(rw("p.data.title").unwrap(), "F_title");
        assert_eq!(rw("p.data.link or p.url").unwrap(), "F_link or U");
        assert_eq!(
            rw("collections.m | by_slugs(p.data.source)").unwrap(),
            "collections.m | by_slugs(F_source)"
        );
        assert_eq!(
            rw(r#"" p.data.x" if p.data.ev else """#).unwrap(),
            r#"" p.data.x" if F_ev else """#
        );
        assert_eq!(
            rw("p.data.date | date('%Y')").unwrap(),
            "F_date | date('%Y')"
        );
        // Autres variables : intactes.
        assert_eq!(
            rw("sp.data.a ~ x.p.data.b ~ pp").unwrap(),
            "sp.data.a ~ x.p.data.b ~ pp"
        );
        // Contenu lu autrement : refusé.
        assert!(rw("p").is_err() && rw("p.fileSlug").is_err() && rw("f(p)").is_err());
        assert_eq!(
            rewrite_template(
                "a {{ p.data.x }} p.data.y {{ p.url }}",
                "p",
                &mut |r| format!("{r:?}")
            )
            .unwrap(),
            r#"a {{ Field("x") }} p.data.y {{ Url }}"#
        );
    }

    #[test]
    fn site_converti_puis_annule() {
        let mut site = Site::new("Essai", "fr");
        site.components = serde_json::from_value(serde_json::json!([{
                "id": "carte",
                "props": [
                    {"name": "projet", "label": "Projet", "type": "entry", "collection": "projets"},
                    {"name": "ton", "type": "text", "default": "clair"}
                ],
                "body": [{"node": "element", "id": "a1", "tag": "article",
                    "attrs": [{"name": "data-revel-tags", "value": "{{ projet.data.source }}"}],
                    "children": [
                        {"node": "element", "id": "a2", "tag": "a", "attrs": [{"name": "href", "value": "{{ projet.url }}"}], "children": [
                            {"node": "element", "id": "a3", "tag": "img", "attrs": [{"name": "src", "value": "{{ projet.data.image }}"}]},
                            {"node": "if", "id": "a4", "cond": "projet.data.ev", "then": [{"node": "out", "expr": "projet.data.title"}]},
                            {"node": "each", "id": "a5", "expr": "projet.data.mots", "item": "site", "children": [{"node": "out", "expr": "site"}]}
                        ]}
                    ]}]
            }])).unwrap();
        site.pages = serde_json::from_value(serde_json::json!([{"id": "index", "path": "/", "body": [
                {"node": "each", "id": "p1", "expr": "collections.projets", "item": "item", "children": [
                    {"node": "use", "id": "p2", "component": "carte", "args": {"projet": "item", "ton": "\"sombre\""},
                     "attrs": [{"name": "data-revel-filter", "value": ""}]}
                ]},
                {"node": "use", "id": "p3", "component": "carte", "args": {"projet": "collections.projets | by_slug(\"x\")"}},
                {"node": "use", "id": "p4", "component": "carte"}
            ]}])).unwrap();
        let mut schema = BTreeMap::new();
        let mut fields = BTreeMap::new();
        fields.insert(
            "title".to_string(),
            FieldInfo {
                label: Some("Titre".into()),
                sample: Some("Un projet".into()),
                ..Default::default()
            },
        );
        fields.insert(
            "ev".to_string(),
            FieldInfo {
                example: Some(Value::Bool(true)),
                ..Default::default()
            },
        );
        fields.insert(
            "mots".to_string(),
            FieldInfo {
                list: true,
                ..Default::default()
            },
        );
        schema.insert(
            "projets".to_string(),
            CollectionInfo {
                fields,
                url: Some("/projets/un/".into()),
            },
        );
        let cmd = convert(&site, &schema, "Page").unwrap();
        let mut s = site.clone();
        let undo = crate::apply(&mut s, &cmd).unwrap();
        let c = s.component("carte").unwrap();
        let props: Vec<(&str, PropKind)> =
            c.props.iter().map(|p| (p.name.as_str(), p.kind)).collect();
        assert_eq!(
            props,
            [
                ("source", PropKind::Text),
                ("lien_page", PropKind::Link),
                ("image", PropKind::Image),
                ("ev", PropKind::Switch),
                ("title", PropKind::Text),
                ("mots", PropKind::List),
                ("ton", PropKind::Text),
            ]
        );
        let title = c.props.iter().find(|p| p.name == "title").unwrap();
        assert_eq!(
            (title.label(), title.default.clone()),
            ("Titre", Some(Value::String("Un projet".into())))
        );
        let body = serde_json::to_string(&c.body).unwrap();
        assert!(
            !body.contains("projet")
                && body.contains(r#""expr":"title""#)
                && body.contains("{{ lien_page }}"),
            "{body}"
        );
        let args = |id: &str| {
            let mut out = None;
            revel_model::ids::walk(&s.pages[0].body, &mut |n| {
                if let Node::Use {
                    id: Some(i),
                    args,
                    attrs,
                    ..
                } = n
                    && i == id
                {
                    out = Some((args.clone(), attrs.len()));
                }
            });
            out.unwrap()
        };
        // Carte d'une liste : champs du contenu de la liste ; reste d'import retiré ; valeur écrite gardée.
        let (a, attrs) = args("p2");
        assert_eq!(a["title"], "item.data.title");
        assert_eq!(a["lien_page"], "item.url");
        assert_eq!(
            (a["ton"].as_str(), a.contains_key("projet"), attrs),
            ("\"sombre\"", false, 0)
        );
        // Contenu choisi, puis contenu par défaut (le premier).
        assert_eq!(
            args("p3").0["image"],
            r#"(collections.projets | by_slug("x")).data.image"#
        );
        assert_eq!(args("p4").0["ev"], "((collections.projets)[0]).data.ev");
        // Une étape d'annulation rend le site d'origine ; rien à refaire ensuite.
        assert!(convert(&s, &schema, "Page").is_none());
        crate::apply(&mut s, &undo).unwrap();
        assert_eq!(
            serde_json::to_value(&s).unwrap(),
            serde_json::to_value(&site).unwrap()
        );
    }

    #[test]
    fn valeur_d_une_utilisation() {
        assert_eq!(
            arg_of("item", &Read::Field("title".into())),
            "item.data.title"
        );
        assert_eq!(
            arg_of(r#"collections.m | by_slug("a")"#, &Read::Url),
            r#"(collections.m | by_slug("a")).url"#
        );
    }
}
