//! Réglages lisibles (3.12, D-28) : les calculs les plus utiles des gabarits d'un site importé,
//! reconnus dans le document et présentés comme des réglages simples. Chaque réglage réécrit
//! l'endroit exact où le calcul est fait (filtre du site, affectation `{% set %}`, expression),
//! par une commande annulable ; un calcul qui ne suit pas une forme reconnue n'est pas proposé
//! (il reste modifiable dans le panneau Variables).
//!
//! - titre de l'onglet : `(seo.title ~ " | " ~ site.title) if seo and seo.title else site.title`
//!   (titre puis site, site puis titre, titre seul ; séparateur au choix) ;
//! - listes de liens (menu du site) : `{% set nav = [{ "url": …, "label": … }, …] %}` ;
//! - contenus mis en avant : liste d'identifiants (`slug`) lue dans les données
//!   (`for slug in home.featured` avec `collections.projets | by_slug(slug)`, ou
//!   `collections.actualites | by_slugs(home.news)`) ;
//! - image de partage par défaut et adresse officielle : valeur de données (`site.share_image`,
//!   `site.url`) reprise par l'image de partage et l'adresse canonique des pages.

use crate::address::{Branch, Owner, Parent, branches};
use crate::command::Command;
use revel_model::ids::{child_lists, node_id};
use revel_model::{Node, Site};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Nœud d'une liste et son adresse (liste, position).
struct At<'a> {
    parent: Parent,
    index: usize,
    node: &'a Node,
}

/// Parcourt les nœuds d'un propriétaire avec leur adresse (les listes d'un bloc sans identifiant
/// ne sont pas adressables : ignorées).
fn walk<'a>(
    owner: &Owner,
    nodes: &'a [Node],
    parent: Option<&str>,
    branch: Branch,
    out: &mut Vec<At<'a>>,
) {
    for (index, n) in nodes.iter().enumerate() {
        out.push(At {
            parent: Parent {
                owner: owner.clone(),
                node: parent.map(str::to_string),
                branch,
            },
            index,
            node: n,
        });
        let Some(id) = node_id(n) else {
            continue;
        };
        for (list, b) in child_lists(n).into_iter().zip(branches(n)) {
            walk(owner, list, Some(id), *b, out);
        }
    }
}

/// Propriétaires (gabarits, pages, composants) et leurs nœuds adressés.
fn owners(site: &Site) -> Vec<(Owner, Vec<At<'_>>)> {
    let mut v = Vec::new();
    for l in &site.layouts {
        let o = Owner::Layout(l.id.clone());
        let mut out = Vec::new();
        walk(&o, &l.body, None, Branch::Children, &mut out);
        v.push((o, out));
    }
    for p in &site.pages {
        let o = Owner::Page(p.id.clone());
        let mut out = Vec::new();
        walk(&o, &p.body, None, Branch::Children, &mut out);
        v.push((o, out));
    }
    for c in &site.components {
        let o = Owner::Component(c.id.clone());
        let mut out = Vec::new();
        walk(&o, &c.body, None, Branch::Children, &mut out);
        v.push((o, out));
    }
    v
}

/// Espaces successifs réduits à un seul.
fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Nom de variable ou chemin (`site.title`, `pages.accueil`).
fn is_path(s: &str) -> bool {
    !s.is_empty()
        && s.split('.').all(|p| {
            let mut c = p.chars();
            c.next()
                .is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
                && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
        })
}

/// Chaîne littérale (`" | "`, `' — '`) : son texte.
fn literal(s: &str) -> Option<String> {
    let s = s.trim();
    let q = s.chars().next()?;
    if !(q == '"' || q == '\'') || s.len() < 2 || !s.ends_with(q) {
        return None;
    }
    let inner = &s[1..s.len() - 1];
    let mut out = String::new();
    let mut esc = false;
    for c in inner.chars() {
        if esc {
            out.push(c);
            esc = false;
        } else if c == '\\' {
            esc = true;
        } else if c == q {
            return None;
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// Chaîne écrite entre guillemets.
fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Découpe au premier niveau (hors chaînes et parenthèses) sur `sep`.
fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let (mut depth, mut quote_ch) = (0i32, None::<char>);
    let mut esc = false;
    for c in s.chars() {
        if let Some(q) = quote_ch {
            cur.push(c);
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == q {
                quote_ch = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                quote_ch = Some(c);
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
            c if c == sep && depth == 0 => parts.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    parts.push(cur);
    parts.into_iter().map(|p| p.trim().to_string()).collect()
}

/// Expression sans parenthèses englobantes.
fn unwrap_parens(s: &str) -> &str {
    let t = s.trim();
    if t.starts_with('(') && t.ends_with(')') {
        // Une seule paire englobante (pas `(a) ~ (b)`).
        let inner = &t[1..t.len() - 1];
        let mut depth = 0i32;
        for c in inner.chars() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth < 0 {
                        return t;
                    }
                }
                _ => {}
            }
        }
        if depth == 0 {
            return inner.trim();
        }
    }
    t
}

// ------------------------------------------------------------------------------------------------
// Titre de l'onglet

/// Format du titre de l'onglet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitleFormat {
    /// Titre de la page seul (le nom du site seulement quand la page n'a pas de titre).
    pub title_only: bool,
    /// Nom du site avant le titre.
    pub site_first: bool,
    /// Séparateur (` | `).
    pub sep: String,
}

/// Endroit où le titre de l'onglet est calculé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TitlePlace {
    /// Filtre du site (`seo | seo_title(site)`).
    Filter { name: String },
    /// Affectation `{% set pageTitle = … %}`.
    Set {
        parent: Parent,
        index: usize,
        name: String,
    },
    /// Expression écrite dans `<title>`.
    Out { parent: Parent, index: usize },
}

/// Réglage « titre de l'onglet » reconnu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TitleSetting {
    pub format: TitleFormat,
    pub place: TitlePlace,
    /// Noms utilisés dans l'expression : référencement de la page (`seo`), données du site
    /// (`site`) et clé du nom (`title`).
    pub seo: String,
    pub site: String,
    pub key: String,
    /// Chemin des données du nom du site (`["site", "title"]`) : aperçu.
    pub site_path: Vec<String>,
}

/// Forme reconnue d'une expression de titre.
fn parse_title(expr: &str) -> Option<(TitleFormat, String, String, String)> {
    let e = squash(expr);
    let e = unwrap_parens(&e);
    let (left, rest) = e.split_once(" if ")?;
    let (cond, fallback) = rest.split_once(" else ")?;
    let fallback = unwrap_parens(fallback);
    let (q, k) = fallback.split_once('.')?;
    if !is_path(q) || !is_path(k) || k.contains('.') {
        return None;
    }
    let (p, p_title) = unwrap_parens(cond).split_once(" and ")?;
    let p = p.trim();
    if !is_path(p) || p.contains('.') || p_title.trim() != format!("{p}.title") {
        return None;
    }
    let left = unwrap_parens(left);
    let title = format!("{p}.title");
    let site = format!("{q}.{k}");
    let format = if left == title {
        TitleFormat {
            title_only: true,
            site_first: false,
            sep: " | ".into(),
        }
    } else {
        let parts = split_top(left, '~');
        let [a, s, b] = parts.as_slice() else {
            return None;
        };
        let sep = literal(s)?;
        let site_first = if *a == title && *b == site {
            false
        } else if *a == site && *b == title {
            true
        } else {
            return None;
        };
        TitleFormat {
            title_only: false,
            site_first,
            sep,
        }
    };
    Some((format, p.to_string(), q.to_string(), k.to_string()))
}

impl TitleSetting {
    /// Expression d'un format.
    pub fn expr(&self, f: &TitleFormat) -> String {
        let (p, site) = (&self.seo, format!("{}.{}", self.site, self.key));
        let title = format!("{p}.title");
        let head = if f.title_only {
            title
        } else if f.site_first {
            format!("({site} ~ {} ~ {title})", quote(&f.sep))
        } else {
            format!("({title} ~ {} ~ {site})", quote(&f.sep))
        };
        format!("{head} if {p} and {p}.title else {site}")
    }

    /// Commande qui applique un format (`None` : filtre introuvable).
    pub fn command(&self, site: &Site, f: &TitleFormat) -> Option<Command> {
        Some(match &self.place {
            TitlePlace::Filter { name } => {
                let mut filter = site.settings.filters.get(name)?.clone();
                filter.expr = self.expr(f);
                Command::SetFilter {
                    name: name.clone(),
                    filter: Some(filter),
                }
            }
            TitlePlace::Set {
                parent,
                index,
                name,
            } => Command::Replace {
                parent: parent.clone(),
                index: *index,
                node: Node::Set {
                    name: name.clone(),
                    expr: self.expr(f),
                },
            },
            TitlePlace::Out { parent, index } => Command::Replace {
                parent: parent.clone(),
                index: *index,
                node: Node::Out { expr: self.expr(f) },
            },
        })
    }

    /// Titre affiché pour une page (`page` : son titre, vide sans titre ; `site` : nom du site).
    pub fn sample(f: &TitleFormat, page: &str, site: &str) -> String {
        if page.is_empty() {
            site.to_string()
        } else if f.title_only {
            page.to_string()
        } else if f.site_first {
            format!("{site}{}{page}", f.sep)
        } else {
            format!("{page}{}{site}", f.sep)
        }
    }
}

/// Titre de l'onglet calculé par le gabarit (premier `<title>` des gabarits, puis des pages).
pub fn title(site: &Site) -> Option<TitleSetting> {
    let all = owners(site);
    for (_, nodes) in &all {
        for at in nodes {
            let Node::Element {
                tag, children, id, ..
            } = at.node
            else {
                continue;
            };
            if !tag.eq_ignore_ascii_case("title") {
                continue;
            }
            let Some((i, expr)) = children.iter().enumerate().find_map(|(i, c)| match c {
                Node::Out { expr } => Some((i, expr.clone())),
                _ => None,
            }) else {
                continue;
            };
            let here = Parent {
                owner: at.parent.owner.clone(),
                node: id.clone(),
                branch: Branch::Children,
            };
            if let Some(s) = title_from(
                site,
                nodes,
                expr.trim(),
                TitlePlace::Out {
                    parent: here,
                    index: i,
                },
            ) {
                return Some(s);
            }
        }
    }
    None
}

/// Réglage d'après une expression (variable, filtre du site ou forme reconnue).
fn title_from(
    site: &Site,
    nodes: &[At<'_>],
    expr: &str,
    place: TitlePlace,
) -> Option<TitleSetting> {
    let e = squash(expr);
    // Variable : son affectation dans le même gabarit.
    if is_path(&e) && !e.contains('.') {
        let at = nodes
            .iter()
            .rev()
            .find(|a| matches!(a.node, Node::Set { name, .. } if *name == e))?;
        let Node::Set { name, expr } = at.node else {
            return None;
        };
        return title_from(
            site,
            nodes,
            expr,
            TitlePlace::Set {
                parent: at.parent.clone(),
                index: at.index,
                name: name.clone(),
            },
        );
    }
    // Filtre du site : `seo | seo_title(site)`.
    let parts = split_top(&e, '|');
    if let [value, call] = parts.as_slice()
        && let Some((name, args)) = call.split_once('(')
        && let Some(args) = args.strip_suffix(')')
        && let Some(f) = site.settings.filters.get(name.trim())
    {
        let args: Vec<String> = split_top(args, ',');
        let (fmt, p, q, k) = parse_title(&f.expr)?;
        // Paramètres du filtre -> valeurs de l'appel (chemin des données du nom du site).
        let param = |n: &str| -> Option<String> {
            let i = f.params.iter().position(|x| x == n)?;
            if i == 0 {
                Some(value.clone())
            } else {
                args.get(i - 1).cloned()
            }
        };
        let site_arg = param(&q).unwrap_or_else(|| q.clone());
        let mut site_path: Vec<String> = site_arg.split('.').map(str::to_string).collect();
        site_path.push(k.clone());
        return Some(TitleSetting {
            format: fmt,
            place: TitlePlace::Filter {
                name: name.trim().to_string(),
            },
            seo: p,
            site: q,
            key: k,
            site_path,
        });
    }
    let (fmt, p, q, k) = parse_title(&e)?;
    Some(TitleSetting {
        format: fmt,
        place,
        site_path: vec![q.clone(), k.clone()],
        seo: p,
        site: q,
        key: k,
    })
}

// ------------------------------------------------------------------------------------------------
// Listes de liens

/// Lien d'une liste (menu du site).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub label: String,
    pub url: String,
}

/// Liste de liens reconnue (`{% set nav = [{ "url": …, "label": … }, …] %}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkList {
    pub owner: Owner,
    pub parent: Parent,
    pub index: usize,
    /// Nom de la variable (`nav`).
    pub name: String,
    pub url_key: String,
    pub label_key: String,
    pub links: Vec<Link>,
    /// Éléments d'origine (autres clés gardées).
    pub items: Vec<Map<String, Value>>,
}

const URL_KEYS: &[&str] = &["url", "href", "link", "path"];
const LABEL_KEYS: &[&str] = &["label", "title", "name", "text"];

/// Listes de liens écrites dans les gabarits, pages et composants.
pub fn link_lists(site: &Site) -> Vec<LinkList> {
    let mut v = Vec::new();
    for (owner, nodes) in owners(site) {
        for at in nodes {
            let Node::Set { name, expr } = at.node else {
                continue;
            };
            let Ok(items) = serde_json::from_str::<Vec<Map<String, Value>>>(expr.trim()) else {
                continue;
            };
            let Some(first) = items.first() else {
                continue;
            };
            let pick = |keys: &[&str]| {
                keys.iter()
                    .find(|k| first.contains_key(**k))
                    .map(|k| k.to_string())
            };
            let (Some(url_key), Some(label_key)) = (pick(URL_KEYS), pick(LABEL_KEYS)) else {
                continue;
            };
            let links: Option<Vec<Link>> = items
                .iter()
                .map(|i| {
                    Some(Link {
                        label: i.get(&label_key)?.as_str()?.to_string(),
                        url: i.get(&url_key)?.as_str()?.to_string(),
                    })
                })
                .collect();
            let Some(links) = links else {
                continue;
            };
            v.push(LinkList {
                owner: owner.clone(),
                parent: at.parent.clone(),
                index: at.index,
                name: name.clone(),
                url_key,
                label_key,
                links,
                items,
            });
        }
    }
    v
}

impl LinkList {
    /// Expression d'une liste : un lien par ligne, clés dans l'ordre d'origine ; les autres clés
    /// d'un lien gardé à sa place sont reprises.
    pub fn expr(&self, links: &[Link]) -> String {
        let keys: Vec<String> = self
            .items
            .first()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        let lines: Vec<String> =
            links
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let mut m = self.items.get(i).cloned().unwrap_or_default();
                    // Lien d'origine retrouvé par son adresse (ordre changé) : ses autres clés suivent.
                    if let Some(o) = self.items.iter().find(|o| {
                        o.get(&self.url_key).and_then(Value::as_str) == Some(l.url.as_str())
                    }) {
                        m = o.clone();
                    }
                    m.insert(self.url_key.clone(), Value::String(l.url.clone()));
                    m.insert(self.label_key.clone(), Value::String(l.label.clone()));
                    let mut order: Vec<String> = keys
                        .iter()
                        .filter(|k| m.contains_key(*k))
                        .cloned()
                        .collect();
                    order.extend(m.keys().filter(|k| !keys.contains(k)).cloned());
                    let fields: Vec<String> = order
                        .iter()
                        .map(|k| format!("{}: {}", Value::String(k.clone()), m[k]))
                        .collect();
                    format!("  {{ {} }}", fields.join(", "))
                })
                .collect();
        if lines.is_empty() {
            "[]".into()
        } else {
            format!("[\n{}\n]", lines.join(",\n"))
        }
    }

    pub fn command(&self, links: &[Link]) -> Command {
        Command::Replace {
            parent: self.parent.clone(),
            index: self.index,
            node: Node::Set {
                name: self.name.clone(),
                expr: self.expr(links),
            },
        }
    }
}

// ------------------------------------------------------------------------------------------------
// Contenus mis en avant

/// Liste d'identifiants de contenus lue dans les données (`pages.accueil.featured`) et
/// collections où ils sont cherchés.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pick {
    pub owner: Owner,
    pub path: Vec<String>,
    pub collections: Vec<String>,
}

/// Collection devant `| by_slug(` ou `| by_slugs(` (`collections.projets`).
fn collection_before(expr: &str, at: usize) -> Option<String> {
    let before = expr[..at].trim_end().strip_suffix('|')?.trim_end();
    let start = before
        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
        .map_or(0, |i| i + 1);
    before[start..]
        .strip_prefix("collections.")
        .map(str::to_string)
}

/// Appels `by_slug(…)` / `by_slugs(…)` d'une expression : (collection, argument, pluriel).
fn slug_calls(expr: &str) -> Vec<(String, String, bool)> {
    let mut v = Vec::new();
    for (word, many) in [("by_slugs(", true), ("by_slug(", false)] {
        let mut from = 0;
        while let Some(k) = expr[from..].find(word) {
            let at = from + k;
            from = at + word.len();
            let Some(end) = expr[from..].find(')') else {
                break;
            };
            let arg = expr[from..from + end].trim().to_string();
            if let Some(c) = collection_before(expr, at) {
                v.push((c, arg, many));
            }
        }
    }
    v
}

/// Expressions d'un nœud (affectation, expression, condition, boucle).
fn exprs(n: &Node) -> Vec<&str> {
    match n {
        Node::Set { expr, .. } | Node::Out { expr } | Node::Each { expr, .. } => vec![expr],
        Node::If { cond, .. } => vec![cond],
        _ => Vec::new(),
    }
}

/// Listes d'identifiants mis en avant (chemins candidats : le cœur garde ceux qui désignent
/// une liste de textes dans les données).
pub fn picks(site: &Site) -> Vec<Pick> {
    let mut out: Vec<Pick> = Vec::new();
    for (owner, nodes) in owners(site) {
        // Alias : `{% set home = pages.accueil %}`.
        let alias: Vec<(String, String)> = nodes
            .iter()
            .filter_map(|a| match a.node {
                Node::Set { name, expr } if is_path(expr.trim()) => {
                    Some((name.clone(), expr.trim().to_string()))
                }
                _ => None,
            })
            .collect();
        let resolve = |p: &str| -> Option<Vec<String>> {
            if !is_path(p) {
                return None;
            }
            let mut segs: Vec<String> = p.split('.').map(str::to_string).collect();
            if let Some((_, to)) = alias.iter().rev().find(|(n, _)| *n == segs[0]) {
                let mut head: Vec<String> = to.split('.').map(str::to_string).collect();
                head.extend(segs.drain(1..));
                segs = head;
            }
            (segs.len() >= 2).then_some(segs)
        };
        let mut add = |path: Vec<String>, c: String| match out
            .iter_mut()
            .find(|p| p.path == path && p.owner == owner)
        {
            Some(p) => {
                if !p.collections.contains(&c) {
                    p.collections.push(c);
                }
            }
            None => out.push(Pick {
                owner: owner.clone(),
                path,
                collections: vec![c],
            }),
        };
        for at in &nodes {
            // `collections.x | by_slugs(chemin)`.
            for e in exprs(at.node) {
                for (c, arg, many) in slug_calls(e) {
                    if many && let Some(p) = resolve(&arg) {
                        add(p, c);
                    }
                }
            }
            // `for slug in chemin` + `collections.x | by_slug(slug)` dans la boucle.
            if let Node::Each {
                expr,
                item,
                id: Some(id),
                children,
                ..
            } = at.node
            {
                let Some(p) = resolve(expr.trim()) else {
                    continue;
                };
                let var = item.trim();
                let mut inner = Vec::new();
                walk(&owner, children, Some(id), Branch::Children, &mut inner);
                for a in &inner {
                    for e in exprs(a.node) {
                        for (c, arg, many) in slug_calls(e) {
                            if !many && arg == var {
                                add(p.clone(), c);
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

// ------------------------------------------------------------------------------------------------
// Image de partage et adresse officielle

/// Valeur de données reprise par le gabarit (`site.share_image`, `site.url`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataUse {
    pub path: Vec<String>,
}

/// Chemins `site.<clé>` d'une expression.
fn site_keys(expr: &str, root: &str) -> Vec<Vec<String>> {
    let mut v = Vec::new();
    let pat = format!("{root}.");
    let mut from = 0;
    while let Some(k) = expr[from..].find(&pat) {
        let at = from + k;
        from = at + pat.len();
        if at > 0 {
            let prev = expr[..at].chars().next_back().unwrap_or(' ');
            if prev.is_ascii_alphanumeric() || prev == '_' || prev == '.' {
                continue;
            }
        }
        let key: String = expr[from..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !key.is_empty() {
            v.push(vec![root.to_string(), key]);
        }
    }
    v
}

/// Image de partage par défaut : clé du site reprise quand une page n'a pas d'image
/// (`(seo and seo.image) or cover or site.share_image`).
pub fn share_image(site: &Site) -> Option<DataUse> {
    for (_, nodes) in owners(site) {
        for at in nodes {
            let Node::Set { expr, .. } = at.node else {
                continue;
            };
            if !expr.contains("seo.image") {
                continue;
            }
            if let Some(p) = site_keys(expr, "site")
                .into_iter()
                .find(|p| p[1].contains("image") || p[1].contains("share"))
            {
                return Some(DataUse { path: p });
            }
        }
    }
    None
}

/// Adresse officielle : clé du site d'où partent les adresses canoniques
/// (`page.url | absolute_url(site.url)`).
pub fn canonical_base(site: &Site) -> Option<DataUse> {
    for (_, nodes) in owners(site) {
        for at in nodes {
            let Node::Set { name, expr } = at.node else {
                continue;
            };
            if !(name.to_ascii_lowercase().contains("canonical") || expr.contains("absolute_url"))
                || !expr.contains("page.url")
            {
                continue;
            }
            if let Some(p) = site_keys(expr, "site").into_iter().next() {
                return Some(DataUse { path: p });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use revel_model::{Component, CustomFilter, Layout, Page};

    fn nodes(src: &str) -> Vec<Node> {
        revel_markup::parse(
            src,
            &revel_markup::Options {
                dialect: revel_markup::Dialect::Nunjucks,
                ..Default::default()
            },
        )
        .map(|p| p.nodes)
        .unwrap()
    }

    fn site() -> Site {
        let mut s = Site::new("Essai", "fr");
        s.settings.filters.insert(
            "seo_title".into(),
            CustomFilter {
                params: vec!["seo".into(), "site".into()],
                expr: "(seo.title ~ ' | ' ~ site.title) if seo and seo.title else site.title"
                    .into(),
                description: Some("Titre".into()),
            },
        );
        s.layouts.push(Layout {
            fields: Vec::new(),
            id: "base".into(),
            parent: None,
            props: Default::default(),
            body: nodes(
                "<html><head>{%- set pageTitle = seo | seoTitle(site) -%}\
                 {%- set canonical = page.url | absoluteUrl(site.url) -%}\
                 {%- set shareSrc = (seo and seo.image) or cover or site.share_image -%}\
                 <title>{{ pageTitle }}</title></head><body>{{ content | safe }}</body></html>",
            )
            .into_iter()
            .map(|mut n| {
                rename(&mut n);
                n
            })
            .collect(),
        });
        s.components.push(Component::new(
            "header",
            nodes(
                "{%- set nav = [\n  { url: \"/agence/\", label: \"Agence\", icon: \"a\" },\n  { url: \"/contact/\", label: \"Contact\", icon: \"c\" }\n] -%}\
                 <nav>{% for item in nav %}<a href=\"{{ item.url }}\">{{ item.label }}</a>{% endfor %}</nav>",
            ),
        ));
        s.pages.push(Page {
            id: "index".into(),
            path: "/".into(),
            layout: Some("base".into()),
            props: Default::default(),
            computed: Default::default(),
            body: nodes(
                "{%- set home = pages.accueil -%}\
                 {%- for slug in home.featured -%}{%- set it = (collections.projets | bySlug(slug)) or (collections.actualites | bySlug(slug)) -%}<p>{{ it.url }}</p>{%- endfor -%}\
                 {%- set news = collections.actualites | bySlugs(home.news) -%}",
            )
            .into_iter()
            .map(|mut n| {
                rename(&mut n);
                n
            })
            .collect(),
            draft: false,
            code: Default::default(),
            name: None,
        });
        s.ensure_ids();
        s
    }

    /// Noms des filtres comme après l'import (`seoTitle` -> `seo_title`).
    fn rename(n: &mut Node) {
        if let Node::Set { expr, .. } | Node::Each { expr, .. } = n {
            *expr = expr
                .replace("seoTitle", "seo_title")
                .replace("absoluteUrl", "absolute_url")
                .replace("bySlug", "by_slug");
        }
        for list in revel_model::ids::child_lists_mut(n) {
            list.iter_mut().for_each(rename);
        }
    }

    #[test]
    fn titre_de_l_onglet() {
        let mut s = site();
        let t = title(&s).expect("titre reconnu");
        assert_eq!(
            t.place,
            TitlePlace::Filter {
                name: "seo_title".into()
            }
        );
        assert_eq!(
            t.format,
            TitleFormat {
                title_only: false,
                site_first: false,
                sep: " | ".into()
            }
        );
        assert_eq!(t.site_path, vec!["site", "title"]);
        let f = TitleFormat {
            title_only: false,
            site_first: true,
            sep: " — ".into(),
        };
        assert_eq!(
            TitleSetting::sample(&f, "Agence", "Alunites"),
            "Alunites — Agence"
        );
        let cmd = t.command(&s, &f).unwrap();
        crate::apply(&mut s, &cmd).unwrap();
        assert_eq!(
            s.settings.filters["seo_title"].expr,
            "(site.title ~ \" — \" ~ seo.title) if seo and seo.title else site.title"
        );
        // Relu après écriture : même format.
        assert_eq!(title(&s).unwrap().format, f);
        // Titre seul.
        let only = TitleFormat {
            title_only: true,
            site_first: false,
            sep: " | ".into(),
        };
        let c = t.command(&s, &only).unwrap();
        crate::apply(&mut s, &c).unwrap();
        assert_eq!(title(&s).unwrap().format, only);
        // Expression non reconnue : pas de réglage.
        s.settings.filters.get_mut("seo_title").unwrap().expr = "seo.title | upper".into();
        assert!(title(&s).is_none());
    }

    #[test]
    fn titre_ecrit_dans_la_balise() {
        let mut s = Site::new("Essai", "fr");
        s.layouts.push(Layout {
            fields: Vec::new(),
            id: "base".into(),
            parent: None,
            props: Default::default(),
            body: nodes("<html><head><title>{{ (seo.title ~ \" · \" ~ site.name) if seo and seo.title else site.name }}</title></head></html>"),
        });
        s.ensure_ids();
        let t = title(&s).unwrap();
        assert!(matches!(t.place, TitlePlace::Out { .. }));
        assert_eq!((t.format.sep.as_str(), t.key.as_str()), (" · ", "name"));
        let f = TitleFormat {
            title_only: false,
            site_first: false,
            sep: " | ".into(),
        };
        let c = t.command(&s, &f).unwrap();
        crate::apply(&mut s, &c).unwrap();
        assert_eq!(title(&s).unwrap().format, f);
    }

    #[test]
    fn menu_du_site() {
        let mut s = site();
        let lists = link_lists(&s);
        assert_eq!(lists.len(), 1);
        let l = &lists[0];
        assert_eq!(
            (l.name.as_str(), l.url_key.as_str(), l.label_key.as_str()),
            ("nav", "url", "label")
        );
        assert_eq!(
            l.links[1],
            Link {
                label: "Contact".into(),
                url: "/contact/".into()
            }
        );
        // Ordre changé, lien ajouté, autre clé gardée avec son lien.
        let next = vec![
            Link {
                label: "Contact".into(),
                url: "/contact/".into(),
            },
            Link {
                label: "L'agence".into(),
                url: "/agence/".into(),
            },
            Link {
                label: "Projets".into(),
                url: "/projets/".into(),
            },
        ];
        assert_eq!(
            l.expr(&next),
            "[\n  { \"url\": \"/contact/\", \"label\": \"Contact\", \"icon\": \"c\" },\n  { \"url\": \"/agence/\", \"label\": \"L'agence\", \"icon\": \"a\" },\n  { \"url\": \"/projets/\", \"label\": \"Projets\" }\n]"
        );
        crate::apply(&mut s, &l.command(&next)).unwrap();
        assert_eq!(link_lists(&s)[0].links, next);
    }

    #[test]
    fn contenus_mis_en_avant_et_donnees() {
        let s = site();
        let p = picks(&s);
        assert_eq!(p.len(), 2, "{p:?}");
        assert_eq!(p[0].path, vec!["pages", "accueil", "featured"]);
        assert_eq!(p[0].collections, vec!["projets", "actualites"]);
        assert_eq!(p[1].path, vec!["pages", "accueil", "news"]);
        assert_eq!(p[1].collections, vec!["actualites"]);
        assert_eq!(share_image(&s).unwrap().path, vec!["site", "share_image"]);
        assert_eq!(canonical_base(&s).unwrap().path, vec!["site", "url"]);
    }
}

#[cfg(test)]
mod cobaye {
    /// Site réel (`REVEL_COBAYE=…/site.revel.json`) : réglages reconnus affichés.
    #[test]
    #[ignore]
    fn reglages_du_cobaye() {
        let p = std::env::var("REVEL_COBAYE").unwrap();
        let s: revel_model::Site =
            serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        println!("titre : {:?}", super::title(&s));
        for l in super::link_lists(&s) {
            println!("liens : {} {:?} {:?}", l.name, l.owner, l.links);
        }
        println!("mis en avant : {:?}", super::picks(&s));
        println!("partage : {:?}", super::share_image(&s));
        println!("adresse : {:?}", super::canonical_base(&s));
    }
}
