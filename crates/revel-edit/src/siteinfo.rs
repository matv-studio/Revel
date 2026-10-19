//! Informations du site : valeurs écrites une fois (coordonnées, réseaux, menus) et reprises
//! dans les pages, les gabarits et les composants (`{{ site.phone }}`,
//! `href="tel:{{ site.phone | tel }}"`, `{% for lien in site.menus.principal %}`). Elles sont
//! la donnée globale `site` : dans le document (`site.revel.json`) pour un site fait dans Revel,
//! ou dans le fichier de données `site.yml` d'un site importé (modifiable aussi par Pages CMS).
//!
//! Ce module donne les valeurs à afficher, la commande qui écrit une valeur là où elle est
//! rangée, et les menus : listes de liens (`label`, `url`) que les listes de la page parcourent.

use crate::address::{Branch, Target};
use crate::canvas::{Binding, Seg, Source};
use crate::command::Command;
use revel_model::{Attr, Node, Site};
use serde_json::{Value, json};

/// Racine des informations du site dans les données globales.
pub const ROOT: &str = "site";
/// Clé des menus dans les informations du site.
pub const MENUS: &str = "menus";
/// Nom de l'élément d'une liste de menu.
pub const ITEM: &str = "lien";

/// Lien d'un menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuLink {
    pub label: String,
    pub url: String,
}

/// Information du site : chemin sous `site` (`["address", "city"]`) et valeur (texte).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoValue {
    pub path: Vec<String>,
    pub value: String,
}

/// Valeurs simples (texte, nombre) des informations du site, dans l'ordre, sans les menus ni
/// les listes.
pub fn values(site_obj: &Value) -> Vec<InfoValue> {
    fn walk(v: &Value, path: &mut Vec<String>, out: &mut Vec<InfoValue>) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    if path.is_empty() && k == MENUS {
                        continue;
                    }
                    path.push(k.clone());
                    walk(x, path, out);
                    path.pop();
                }
            }
            Value::String(s) => out.push(InfoValue {
                path: path.clone(),
                value: s.clone(),
            }),
            Value::Number(n) => out.push(InfoValue {
                path: path.clone(),
                value: n.to_string(),
            }),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(site_obj, &mut Vec::new(), &mut out);
    out
}

/// Menus des informations du site : (nom, liens).
pub fn menus(site_obj: &Value) -> Vec<(String, Vec<MenuLink>)> {
    site_obj
        .get(MENUS)
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(name, list)| {
                    let links = list
                        .as_array()?
                        .iter()
                        .map(|l| MenuLink {
                            label: l
                                .get("label")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            url: l
                                .get("url")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                        })
                        .collect();
                    Some((name.clone(), links))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Informations du site rangées dans le document (sinon dans le fichier `site.yml`, si le site a
/// un dossier de données).
pub fn in_document(site: &Site) -> bool {
    site.data.contains_key(ROOT) || site.data_dir.is_none()
}

/// Commande qui écrit une information du site (chemin sous `site`) là où elle est rangée.
pub fn set_value(site: &Site, path: &[String], value: Option<Value>) -> Command {
    let mut full = vec![ROOT.to_string()];
    full.extend(path.iter().cloned());
    if in_document(site) {
        Command::SetGlobal { path: full, value }
    } else {
        Command::SetDataValue {
            binding: Binding {
                source: Source::Data,
                path: full.into_iter().map(Seg::Key).collect(),
            },
            value,
        }
    }
}

/// Liste JSON des liens d'un menu.
pub fn menu_value(links: &[MenuLink]) -> Value {
    Value::Array(
        links
            .iter()
            .map(|l| json!({ "label": l.label, "url": l.url }))
            .collect(),
    )
}

/// Expression d'une liste de menu.
pub fn menu_expr(name: &str) -> String {
    format!("{ROOT}.{MENUS}.{name}")
}

/// Nom du menu parcouru par une liste (`site.menus.principal` -> `principal`).
pub fn menu_of(expr: &str) -> Option<String> {
    let name = expr
        .trim()
        .strip_prefix(&format!("{ROOT}.{MENUS}."))?
        .to_string();
    (!name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-'))
    .then_some(name)
}

/// Nom de clé tiré d'un libellé (« Horaires d'ouverture » -> `horaires_d_ouverture`).
pub fn key_from(label: &str) -> String {
    let mut out = String::new();
    for c in label.chars() {
        let c = match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            c => c.to_ascii_lowercase(),
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_').to_string();
    if out.is_empty() { "valeur".into() } else { out }
}

fn attr<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .and_then(|a| a.value.as_deref())
}

/// Élément « lien » d'une liste : un lien seul dont le texte est écrit (`<a>` ou `<li><a>`),
/// avec son texte et son adresse.
fn link_item(n: &Node) -> Option<(String, String)> {
    let Node::Element {
        tag,
        attrs,
        children,
        ..
    } = n
    else {
        return None;
    };
    if tag.eq_ignore_ascii_case("a") {
        let href = attr(attrs, "href")?;
        if href.contains("{{") || href.contains("{%") {
            return None;
        }
        let text: Option<String> = children
            .iter()
            .map(|c| match c {
                Node::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect();
        let text = text?.trim().to_string();
        return (!text.is_empty()).then(|| (text, href.to_string()));
    }
    let kids: Vec<&Node> = children
        .iter()
        .filter(|c| !matches!(c, Node::Text { text } if text.trim().is_empty()))
        .collect();
    match kids.as_slice() {
        [one] => link_item(one),
        _ => None,
    }
}

/// Liens écrits à la main parmi les enfants de l'élément `at` : (position, texte, adresse).
pub fn link_items(site: &Site, at: &Target) -> Vec<(usize, String, String)> {
    let Ok(Node::Element { children, .. }) = crate::address::node(site, at) else {
        return Vec::new();
    };
    children
        .iter()
        .enumerate()
        .filter_map(|(i, c)| link_item(c).map(|(t, u)| (i, t, u)))
        .collect()
}

/// Modèle d'un élément de menu : l'élément « lien » donné, son lien relié au texte et à
/// l'adresse de chaque lien du menu (identifiants gardés).
fn template(n: &Node) -> Node {
    let mut n = n.clone();
    fn bind(n: &mut Node) {
        if let Node::Element {
            tag,
            attrs,
            children,
            ..
        } = n
        {
            if tag.eq_ignore_ascii_case("a") {
                for a in attrs.iter_mut() {
                    if a.name == "href" {
                        a.value = Some(format!("{{{{ {ITEM}.url }}}}"));
                    }
                }
                *children = vec![Node::Out {
                    expr: format!("{ITEM}.label"),
                }];
                return;
            }
            for c in children.iter_mut() {
                bind(c);
            }
        }
    }
    bind(&mut n);
    n
}

/// Les liens écrits de l'élément `at` deviennent le menu `name` (créé avec ces liens, ou relié
/// s'il existe : seuls les liens qu'il contient sont remplacés, les autres restent écrits) :
/// une liste qui parcourt le menu, dont le premier lien remplacé sert de modèle.
pub fn use_menu(
    site: &Site,
    at: &Target,
    name: &str,
    existing: Option<&[MenuLink]>,
) -> Option<Command> {
    let Ok(Node::Element {
        id,
        tag,
        attrs,
        children,
    }) = crate::address::node(site, at)
    else {
        return None;
    };
    let items = link_items(site, at);
    if items.is_empty() {
        return None;
    }
    let menu: Vec<MenuLink> = match existing {
        Some(links) => links.to_vec(),
        None => items
            .iter()
            .map(|(_, t, u)| MenuLink {
                label: t.clone(),
                url: u.clone(),
            })
            .collect(),
    };
    // Liens remplacés : ceux du menu, à la suite (un lien hors du menu reste écrit).
    let replaced: Vec<usize> = items
        .iter()
        .filter(|(_, t, u)| menu.iter().any(|m| m.url == *u && m.label == *t))
        .map(|(i, _, _)| *i)
        .collect();
    let first = *replaced.first()?;
    let mut taken = site.node_ids();
    let each_id = revel_model::ids::fresh(&taken);
    taken.insert(each_id.clone());
    let last = *replaced.last().unwrap_or(&first);
    let blank = |c: &Node| matches!(c, Node::Text { text } if text.trim().is_empty());
    let mut kids = Vec::new();
    for (i, c) in children.iter().enumerate() {
        if i == first {
            kids.push(Node::Each {
                id: Some(each_id.clone()),
                expr: menu_expr(name),
                item: ITEM.into(),
                children: vec![template(c)],
                empty: Vec::new(),
            });
        } else if replaced.contains(&i) || (i > first && i < last && blank(c)) {
            // Liens remplacés et blancs entre eux : la liste les répète.
            continue;
        } else {
            kids.push(c.clone());
        }
    }
    let (parent, index) = crate::address::locate(site, at).ok()?;
    let mut commands = Vec::new();
    if existing.is_none() {
        commands.push(set_value(
            site,
            &[MENUS.to_string(), name.to_string()],
            Some(menu_value(&menu)),
        ));
    }
    commands.push(Command::Replace {
        parent,
        index,
        node: Node::Element {
            id: id.clone(),
            tag: tag.clone(),
            attrs: attrs.clone(),
            children: kids,
        },
    });
    Some(Command::Batch { commands })
}

/// Liste qui parcourt un menu (`site.menus.<nom>`) : la boucle `each`.
pub fn is_menu_list(site: &Site, each: &Target) -> Option<String> {
    match crate::address::node(site, each).ok()? {
        Node::Each { expr, .. } => menu_of(expr),
        _ => None,
    }
}

/// Liste de menu qui contient `at` : (boucle, nom du menu, nom de l'élément).
pub fn enclosing_menu(site: &Site, at: &Target) -> Option<(Target, String, String)> {
    let mut cur = at.clone();
    for _ in 0..64 {
        let (parent, _) = crate::address::locate(site, &cur).ok()?;
        let id = parent.node?;
        let t = Target {
            owner: at.owner.clone(),
            node: id,
        };
        if let Ok(Node::Each { expr, item, .. }) = crate::address::node(site, &t)
            && parent.branch == Branch::Children
        {
            return menu_of(expr).map(|m| (t, m, item.trim().to_string()));
        }
        cur = t;
    }
    None
}

/// Liste d'un menu changée pour un autre menu.
pub fn switch_menu(each: &Target, name: &str) -> Command {
    Command::SetLogic {
        at: each.clone(),
        cond: None,
        expr: Some(menu_expr(name)),
        item: None,
    }
}

/// Nombre d'endroits du site qui reprennent l'information `path` (textes, attributs, conditions,
/// listes, images).
pub fn uses(site: &Site, path: &[String]) -> usize {
    let needle = format!("{ROOT}.{}", path.join("."));
    let count_in = |s: &str| -> usize {
        s.match_indices(&needle)
            .filter(|(i, _)| {
                let next = s[i + needle.len()..].chars().next();
                let prev = s[..*i].chars().next_back();
                !next.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
                    && !prev.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
            })
            .count()
    };
    let mut n = 0;
    for (_, _, body) in site.owners() {
        revel_model::ids::walk(body, &mut |node| {
            n += match node {
                Node::Out { expr } => count_in(expr),
                Node::If { cond, .. } => count_in(cond),
                Node::Each { expr, .. } => count_in(expr),
                Node::Element { attrs, .. } => attrs
                    .iter()
                    .map(|a| {
                        a.value.as_deref().map_or(0, count_in)
                            + a.when.as_deref().map_or(0, count_in)
                    })
                    .sum(),
                Node::Image(img) => count_in(&img.src),
                Node::Use { args, .. } => args.values().map(|v| count_in(v)).sum(),
                _ => 0,
            };
        });
    }
    n
}
