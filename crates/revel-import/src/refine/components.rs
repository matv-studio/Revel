//! Éléments répétés de même forme devenus des composants (D-178).
//!
//! Deux éléments ont la même forme quand leurs balises, leurs classes et leurs attributs sont
//! les mêmes à tous les niveaux ; seuls changent les textes, les adresses des liens et les
//! images, qui deviennent les propriétés du composant. Le rendu des pages ne change pas.

use super::{Report, classes, is_static, strip_ids, tag, tool_marked, words};
use revel_model::{Attr, Component, Node, PropDef, PropKind, Site};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Attributs dont la valeur change d'une utilisation à l'autre sans changer la forme.
const VARIABLE: &[&str] = &[
    "href",
    "src",
    "srcset",
    "sizes",
    "alt",
    "title",
    "width",
    "height",
    "datetime",
    "aria-label",
    "id",
];

/// Balises qui ne font pas la racine d'un composant.
const NOT_ROOT: &[&str] = &[
    "svg", "html", "head", "body", "main", "header", "footer", "nav", "tr", "td", "th", "thead",
    "tbody", "table", "option", "select", "picture", "source", "br", "img",
];

/// Balises jamais reprises dans un composant.
const NEVER: &[&str] = &[
    "script", "style", "form", "html", "head", "body", "template", "title", "meta", "link", "base",
];

/// Composants créés au plus (les plus grands d'abord).
const MAX: usize = 12;

struct Cand {
    id: String,
    sig: String,
    /// Éléments du sous-arbre.
    size: usize,
    /// Identifiants des éléments parents, du plus lointain au plus proche.
    anc: Vec<String>,
    /// Page ou gabarit (rang dans le parcours).
    owner: usize,
}

/// Forme d'un sous-arbre, nombre d'éléments, présence d'un contenu (texte ou image).
type Shape = (String, usize, bool);

/// `in_tool` : dans un outil de Revel, dont le contenu ne devient pas un composant.
fn collect(n: &Node, anc: &mut Vec<String>, cands: &mut Vec<Cand>, in_tool: bool) -> Option<Shape> {
    match n {
        Node::Text { text } => Some(if text.trim().is_empty() {
            (String::new(), 0, false)
        } else {
            ("T".into(), 0, true)
        }),
        Node::Html { html } => Some(if html.trim().is_empty() {
            (String::new(), 0, false)
        } else {
            (format!("H<{html}>"), 0, false)
        }),
        Node::Image(i) => {
            let mut c = i.clone();
            c.id = None;
            c.src = String::new();
            c.alt = None;
            let literal = |e: &str| e.starts_with('"') && e.ends_with('"') && e.len() >= 2;
            (literal(&i.src) && i.alt.as_deref().is_none_or(literal)).then(|| {
                (
                    format!("I<{}>", serde_json::to_string(&c).unwrap_or_default()),
                    1,
                    true,
                )
            })
        }
        Node::Element {
            id,
            tag,
            attrs,
            children,
        } => {
            if tag == "svg" {
                // Dessin : repris tel quel, identique ou pas du tout.
                let mut c = n.clone();
                strip_ids(&mut c);
                return attrs.iter().all(is_static).then(|| {
                    (
                        format!("S<{}>", serde_json::to_string(&c).unwrap_or_default()),
                        1,
                        false,
                    )
                });
            }
            if let Some(i) = id {
                anc.push(i.clone());
            }
            let inner = in_tool || tool_marked(attrs);
            let parts: Vec<Option<Shape>> = children
                .iter()
                .map(|c| collect(c, anc, cands, inner))
                .collect();
            if id.is_some() {
                anc.pop();
            }
            let ok = !NEVER.contains(&tag.as_str())
                && attrs.iter().all(is_static)
                && !tool_marked(attrs);
            if !ok {
                return None;
            }
            let mut sig = tag.clone();
            for a in attrs {
                sig.push('[');
                sig.push_str(&a.name);
                if !VARIABLE.contains(&a.name.as_str()) {
                    sig.push('=');
                    sig.push_str(a.value.as_deref().unwrap_or(""));
                }
                sig.push(']');
            }
            sig.push('{');
            let mut size = 1;
            let mut content = tag == "img";
            for p in parts {
                let (s, n, c) = p?;
                sig.push_str(&s);
                size += n;
                content |= c;
            }
            sig.push('}');
            if let Some(i) = id
                && !in_tool
                && content
                && size >= 3
                && !NOT_ROOT.contains(&tag.as_str())
            {
                cands.push(Cand {
                    id: i.clone(),
                    sig: sig.clone(),
                    size,
                    anc: anc.clone(),
                    owner: 0,
                });
            }
            Some((sig, size, content))
        }
        Node::If {
            then, otherwise, ..
        } => {
            for c in then.iter().chain(otherwise) {
                collect(c, anc, cands, in_tool);
            }
            None
        }
        Node::Each {
            children, empty, ..
        } => {
            for c in children.iter().chain(empty) {
                collect(c, anc, cands, in_tool);
            }
            None
        }
        Node::SetBlock { children, .. } | Node::Use { children, .. } => {
            for c in children {
                collect(c, anc, cands, in_tool);
            }
            None
        }
        _ => None,
    }
}

/// Assez répété pour un composant : trois fois, ou deux fois sur deux pages pour un grand bloc
/// (section commune à plusieurs pages).
fn enough(group: &[&Cand]) -> bool {
    let Some(first) = group.first() else {
        return false;
    };
    (group.len() >= 3 && first.size >= 3)
        || (group.len() == 2 && first.size >= 8 && group[0].owner != group[1].owner)
}

#[derive(Clone, PartialEq)]
enum Kind {
    Text,
    Attr(String),
    ImageSrc,
    ImageAlt,
}

struct Slot {
    kind: Kind,
    /// Classe et balise de l'élément qui porte la valeur.
    class: Option<String>,
    tag: String,
    value: String,
}

fn slots(n: &Node, out: &mut Vec<Slot>) {
    match n {
        Node::Image(i) => {
            out.push(Slot {
                kind: Kind::ImageSrc,
                class: i.class.clone(),
                tag: "img".into(),
                value: i.src.clone(),
            });
            out.push(Slot {
                kind: Kind::ImageAlt,
                class: i.class.clone(),
                tag: "img".into(),
                value: i.alt.clone().unwrap_or_default(),
            });
        }
        Node::Element {
            tag: t,
            attrs,
            children,
            ..
        } if t != "svg" => {
            let class = classes(n).first().map(|c| c.to_string());
            for a in attrs {
                if VARIABLE.contains(&a.name.as_str()) {
                    out.push(Slot {
                        kind: Kind::Attr(a.name.clone()),
                        class: class.clone(),
                        tag: t.clone(),
                        value: a.value.clone().unwrap_or_default(),
                    });
                }
            }
            for c in children {
                match c {
                    Node::Text { text } if !text.trim().is_empty() => out.push(Slot {
                        kind: Kind::Text,
                        class: class.clone(),
                        tag: t.clone(),
                        value: text.clone(),
                    }),
                    _ => slots(c, out),
                }
            }
        }
        _ => {}
    }
}

/// Copie du premier exemplaire où les valeurs qui changent lisent une propriété (`names[i]` :
/// nom de la propriété de la valeur `i`, `None` si elle ne change pas).
fn template(n: &Node, names: &[Option<String>], at: &mut usize) -> Node {
    match n {
        Node::Image(i) => {
            let mut i = i.clone();
            if let Some(Some(p)) = names.get(*at) {
                i.src = p.clone();
            }
            if let Some(Some(p)) = names.get(*at + 1) {
                i.alt = Some(p.clone());
            }
            *at += 2;
            Node::Image(i)
        }
        Node::Element {
            id,
            tag: t,
            attrs,
            children,
        } if t != "svg" => {
            let mut out_attrs = Vec::with_capacity(attrs.len());
            for a in attrs {
                let mut a = a.clone();
                if VARIABLE.contains(&a.name.as_str()) {
                    if let Some(Some(p)) = names.get(*at) {
                        a.value = Some(format!("{{{{ {p} }}}}"));
                    }
                    *at += 1;
                }
                out_attrs.push(a);
            }
            let mut out = Vec::with_capacity(children.len());
            for c in children {
                match c {
                    Node::Text { text } if !text.trim().is_empty() => {
                        out.push(match names.get(*at) {
                            Some(Some(p)) => Node::Out { expr: p.clone() },
                            _ => c.clone(),
                        });
                        *at += 1;
                    }
                    _ => out.push(template(c, names, at)),
                }
            }
            Node::Element {
                id: id.clone(),
                tag: t.clone(),
                attrs: out_attrs,
                children: out,
            }
        }
        _ => n.clone(),
    }
}

/// Nom d'expression tiré d'un nom de classe (`carte__titre-court` : `titre_court`).
fn ident(s: &str) -> String {
    let w = words(s);
    let mut out = w.join("_");
    out.retain(|c| c.is_ascii_alphanumeric() || c == '_');
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, 'p');
    }
    out
}

fn base_name(s: &Slot, root_class: Option<&str>, fr: bool) -> String {
    let pick = |f: &'static str, e: &'static str| (if fr { f } else { e }).to_string();
    // Partie propre de la classe : après le nom du bloc (`carte__titre`, `carte-titre`).
    let own = s.class.as_deref().and_then(|c| {
        let tail = c
            .rsplit_once("__")
            .map(|(_, t)| t)
            .or_else(|| {
                root_class
                    .and_then(|r| c.strip_prefix(r))
                    .map(|t| t.trim_start_matches(['-', '_']))
            })
            .unwrap_or(c);
        let i = ident(tail);
        (!i.is_empty() && i.len() <= 24 && Some(c) != root_class).then_some(i)
    });
    match &s.kind {
        Kind::Text => own.unwrap_or_else(|| match s.tag.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => pick("titre", "title"),
            "time" => "date".into(),
            "a" => pick("libelle", "label"),
            "button" => pick("bouton", "button"),
            _ => pick("texte", "text"),
        }),
        Kind::ImageSrc => "image".into(),
        Kind::ImageAlt => "alt".into(),
        Kind::Attr(a) => match a.as_str() {
            "href" => pick("lien", "link"),
            "src" => "image".into(),
            "alt" => "alt".into(),
            "srcset" => "image_srcset".into(),
            "sizes" => "image_sizes".into(),
            "width" => pick("largeur", "width"),
            "height" => pick("hauteur", "height"),
            "title" => pick("infobulle", "tooltip"),
            "datetime" => "date_iso".into(),
            "aria-label" => pick("libelle_aria", "aria_label"),
            _ => pick("ancre", "anchor"),
        },
    }
}

fn label(name: &str) -> String {
    let s = name.replace('_', " ");
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => s,
    }
}

fn find<'a>(nodes: &'a [Node], ids: &HashSet<&str>, out: &mut HashMap<String, &'a Node>) {
    revel_model::ids::walk(nodes, &mut |n| {
        if let Node::Element { id: Some(i), .. } = n
            && ids.contains(i.as_str())
        {
            out.insert(i.clone(), n);
        }
    });
}

pub(super) fn run(site: &mut Site, report: &mut Report) {
    let mut cands = Vec::new();
    for (owner, body) in site
        .pages
        .iter()
        .map(|p| &p.body)
        .chain(site.layouts.iter().map(|l| &l.body))
        .enumerate()
    {
        let from = cands.len();
        for n in body {
            collect(n, &mut Vec::new(), &mut cands, false);
        }
        for c in &mut cands[from..] {
            c.owner = owner;
        }
    }
    let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, c) in cands.iter().enumerate() {
        groups.entry(c.sig.as_str()).or_default().push(i);
    }
    groups.retain(|_, g| enough(&g.iter().map(|&i| &cands[i]).collect::<Vec<_>>()));
    // Liste d'éléments répétés : ce sont eux les composants, pas la liste.
    let group_of: HashMap<&str, &str> = groups
        .iter()
        .flat_map(|(sig, g)| g.iter().map(|&i| (cands[i].id.as_str(), *sig)))
        .collect();
    let mut inside: HashMap<(&str, &str), usize> = HashMap::new();
    for (sig, g) in &groups {
        for &i in g {
            for a in &cands[i].anc {
                if group_of.contains_key(a.as_str()) {
                    *inside.entry((a.as_str(), *sig)).or_default() += 1;
                }
            }
        }
    }
    let size_of: HashMap<&str, usize> = groups
        .iter()
        .map(|(sig, g)| (*sig, cands[g[0]].size))
        .collect();
    let mut lists: HashSet<&str> = HashSet::new();
    for ((outer, inner), n) in &inside {
        let (Some(o), Some(os), Some(is)) = (
            group_of.get(outer),
            group_of.get(outer).and_then(|o| size_of.get(o)),
            size_of.get(inner),
        ) else {
            continue;
        };
        if *n >= 2 && (n * is) as f64 >= 0.6 * *os as f64 {
            lists.insert(o);
        }
    }
    let mut order: Vec<(&str, &Vec<usize>)> = groups
        .iter()
        .filter(|(sig, _)| !lists.contains(*sig))
        .map(|(s, g)| (*s, g))
        .collect();
    // Les plus grands d'abord (un bloc passe avant ses parties) ; à taille égale, les plus
    // répétés.
    order.sort_by(|a, b| {
        let (sa, sb) = (cands[a.1[0]].size, cands[b.1[0]].size);
        sb.cmp(&sa)
            .then(b.1.len().cmp(&a.1.len()))
            .then(a.0.cmp(b.0))
    });
    // Douze au plus : ceux qui retirent le plus de répétitions (taille × utilisations en trop).
    if order.len() > MAX {
        let mut best: Vec<(usize, &str)> = order
            .iter()
            .map(|(sig, g)| (cands[g[0]].size * (g.len() - 1), *sig))
            .collect();
        best.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
        let keep: HashSet<&str> = best.iter().take(MAX * 2).map(|(_, s)| *s).collect();
        order.retain(|(sig, _)| keep.contains(sig));
    }
    let fr = site.default_locale.starts_with("fr");
    let mut chosen: HashSet<String> = HashSet::new();
    // (identifiant du composant, exemplaires : identifiant de l'élément et valeurs).
    let mut uses: HashMap<String, (String, BTreeMap<String, String>)> = HashMap::new();
    let mut made: Vec<Component> = Vec::new();
    for (_, g) in order {
        if made.len() >= MAX {
            break;
        }
        let live: Vec<&Cand> = g
            .iter()
            .map(|&i| &cands[i])
            .filter(|c| !c.anc.iter().any(|a| chosen.contains(a)))
            .collect();
        if !enough(&live) {
            continue;
        }
        let ids: HashSet<&str> = live.iter().map(|c| c.id.as_str()).collect();
        let mut nodes = HashMap::new();
        for p in &site.pages {
            find(&p.body, &ids, &mut nodes);
        }
        for l in &site.layouts {
            find(&l.body, &ids, &mut nodes);
        }
        let per: Vec<(&str, Vec<Slot>)> = live
            .iter()
            .filter_map(|c| {
                let mut s = Vec::new();
                slots(nodes.get(&c.id)?, &mut s);
                Some((c.id.as_str(), s))
            })
            .collect();
        let Some((first_id, first)) = per.first() else {
            continue;
        };
        if per.iter().any(|(_, s)| s.len() != first.len()) {
            continue;
        }
        let Some(root) = nodes.get(*first_id) else {
            continue;
        };
        let root_class = classes(root).first().map(|c| c.to_string());
        let mut cid = root_class
            .as_deref()
            .map(|c| words(c).join("-"))
            .filter(|c| revel_model::valid_variant_id(c))
            .unwrap_or_else(|| tag(root).to_string());
        let taken = |id: &str| {
            site.components.iter().any(|c| c.id == id) || made.iter().any(|c| c.id == id)
        };
        if taken(&cid) {
            let mut n = 2;
            while taken(&format!("{cid}-{n}")) {
                n += 1;
            }
            cid = format!("{cid}-{n}");
        }
        let mut props: Vec<PropDef> = Vec::new();
        let mut names: Vec<Option<String>> = Vec::with_capacity(first.len());
        for (i, s) in first.iter().enumerate() {
            if per.iter().all(|(_, v)| v[i].value == s.value) {
                names.push(None);
                continue;
            }
            let mut base = base_name(s, root_class.as_deref(), fr);
            if !revel_model::valid_prop_name(&base) {
                base = format!("p_{base}");
            }
            let name = revel_edit::make::free_name(&props, &base);
            let literal = !matches!(s.kind, Kind::ImageSrc | Kind::ImageAlt);
            props.push(PropDef {
                name: name.clone(),
                label: Some(label(&name)),
                kind: match &s.kind {
                    Kind::Attr(a) if a == "href" => PropKind::Link,
                    Kind::Attr(a) if a == "src" => PropKind::Image,
                    Kind::ImageSrc => PropKind::Image,
                    _ => PropKind::Text,
                },
                default: literal.then(|| serde_json::Value::String(s.value.clone())),
                collection: None,
            });
            names.push(Some(name));
        }
        let mut body = template(root, &names, &mut 0);
        strip_ids(&mut body);
        for (id, vals) in &per {
            let args = names
                .iter()
                .zip(vals)
                .filter_map(|(n, v)| {
                    let n = n.as_ref()?;
                    let literal = !matches!(v.kind, Kind::ImageSrc | Kind::ImageAlt);
                    Some((
                        n.clone(),
                        if literal {
                            revel_edit::canvas::quote(&v.value)
                        } else if v.value.is_empty() {
                            "\"\"".into()
                        } else {
                            v.value.clone()
                        },
                    ))
                })
                .collect();
            uses.insert(id.to_string(), (cid.clone(), args));
        }
        for c in &live {
            chosen.insert(c.id.clone());
        }
        let mut name = root_class
            .as_deref()
            .map(|c| label(&words(c).join(" ")))
            .unwrap_or_else(|| label(tag(root)));
        if report.components.iter().any(|(n, _)| *n == name) {
            let mut n = 2;
            while report
                .components
                .iter()
                .any(|(x, _)| *x == format!("{name} {n}"))
            {
                n += 1;
            }
            name = format!("{name} {n}");
        }
        report.components.push((name, per.len()));
        made.push(Component {
            id: cid,
            name: report.components.last().map(|(n, _)| n.clone()),
            props,
            variants: Vec::new(),
            body: vec![body],
        });
    }
    if made.is_empty() {
        return;
    }
    fn replace(list: &mut [Node], uses: &HashMap<String, (String, BTreeMap<String, String>)>) {
        for n in list.iter_mut() {
            let hit = match n {
                Node::Element { id: Some(i), .. } => uses.get(i.as_str()),
                _ => None,
            };
            if let Some((component, args)) = hit {
                // Nom du calque et masquage gardés sur l'utilisation.
                let keep: Vec<Attr> = match n {
                    Node::Element { attrs, .. } => attrs
                        .iter()
                        .filter(|a| a.name == revel_model::LAYER_NAME_ATTR)
                        .cloned()
                        .collect(),
                    _ => Vec::new(),
                };
                *n = Node::Use {
                    id: None,
                    component: component.clone(),
                    args: args.clone(),
                    variant: None,
                    attrs: keep,
                    children: Vec::new(),
                };
                continue;
            }
            for l in revel_model::ids::child_lists_mut(n) {
                replace(l, uses);
            }
        }
    }
    for p in &mut site.pages {
        replace(&mut p.body, &uses);
    }
    for l in &mut site.layouts {
        replace(&mut l.body, &uses);
    }
    site.components.extend(made);
}
