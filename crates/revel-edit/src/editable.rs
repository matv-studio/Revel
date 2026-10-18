//! Relevé de ce que l'interface de Revel ne crée ni ne règle (non bloquant) : pour un site fait
//! par un assistant, une importation ou à la main dans le code, liste les nœuds dont la forme
//! n'est pas celle que produisent les panneaux (calques, Styles, liste de collection, propriétés
//! de composant, champs du contenu).
//!
//! Formes reconnues, comme l'interface les écrit :
//! - texte écrit, éléments, images à adresse écrite, code HTML d'un élément « Code » ;
//! - dans une liste de collection : `item.data.<champ>` (texte, avec écriture de date ou `| safe`),
//!   `item.url`, conditions « champ rempli / vide » ; dans le modèle des pages d'une collection :
//!   `<champ>`, `page.url` ;
//! - dans un composant : propriétés (`{{ titre }}` seul dans un élément, adresse d'image ou de
//!   lien, condition d'affichage) ;
//! - listes : [`Query`] (liste de collection) ou [`FieldList`] (liste tirée d'un champ) ;
//! - sélecteur de langue (`locales`), emplacement de la page dans un gabarit.
//!
//! Tout le reste (variables `{% set %}`, expressions calculées, filtres du site, conditions
//! écrites, attributs conditionnels) est relevé : ce sont les outils à créer pour que
//! l'utilisateur garde la main.

use crate::address::Owner;
use crate::collection::{FieldList, Query};
use revel_model::{Node, PropKind, Site};
use serde::Serialize;

/// Nature d'un point relevé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Variable (`{% set %}`).
    Variable,
    /// Valeur calculée affichée (`{{ … }}`).
    Expression,
    /// Condition écrite.
    Condition,
    /// Liste calculée (ni collection, ni champ).
    List,
    /// Attribut calculé ou conditionnel.
    Attribute,
    /// Valeur de propriété calculée (utilisation de composant).
    Argument,
    /// Image dont l'adresse ou le texte alternatif est calculé.
    Image,
    /// HTML écrit hors d'un élément « Code ».
    Html,
    /// Gabarit sans emplacement de page (la page n'apparaît pas dans les calques).
    NoSlot,
    /// Filtre propre au site (paramètres du site).
    Filter,
    /// Données de page autres que le référencement (variables de page).
    PageData,
}

/// Point relevé : où, quoi, forme écrite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub owner: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    pub kind: Kind,
    pub detail: String,
}

/// Contexte d'un nœud : ce que les panneaux savent relier à cet endroit.
#[derive(Clone, Default)]
struct Ctx {
    /// Début des champs reliables (`item.data.` dans une liste, vide dans un modèle).
    fields: Option<String>,
    /// Adresse du contenu (`item.url`, `page.url`).
    url: Option<String>,
    /// Liste tirée d'un champ : nom de la valeur.
    value: Option<String>,
    /// Propriétés du composant (nom, nature).
    props: Vec<(String, PropKind)>,
    /// Sélecteur de langue : variable de la langue.
    locale: Option<String>,
    /// Dans `<head>` (titre de la page).
    head: bool,
}

fn is_field(f: &str) -> bool {
    !f.is_empty()
        && f.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

impl Ctx {
    /// Champ relié (`item.data.titre`, `titre` dans un modèle) ou valeur d'une liste de champ.
    fn field(&self, e: &str) -> bool {
        let e = e.trim();
        if let Some(v) = self.value.as_deref() {
            // Valeur d'une liste de champ, ou sous-champ d'une liste d'objets (`a.title`).
            if e == v
                || e.strip_prefix(v)
                    .and_then(|r| r.strip_prefix('.'))
                    .is_some_and(is_field)
            {
                return true;
            }
        }
        self.fields
            .as_deref()
            .and_then(|p| e.strip_prefix(p))
            .is_some_and(is_field)
    }
    fn prop(&self, e: &str, kind: Option<PropKind>) -> bool {
        let e = e.trim();
        self.props
            .iter()
            .any(|(n, k)| n == e && kind.is_none_or(|x| x == *k))
    }
    /// Valeur affichée reconnue : champ (écriture de date, `| safe`), propriété, langue.
    fn output(&self, expr: &str) -> bool {
        let e = expr.trim();
        // Année en cours (pied de page).
        if matches!(
            squash(e).as_str(),
            "now()|date(\"%Y\")" | "now()|date('%Y')"
        ) {
            return true;
        }
        let (head, tail) = match e.split_once('|') {
            Some((h, t)) => (h.trim(), Some(t.trim())),
            None => (e, None),
        };
        // Nombre de valeurs d'un champ liste ou d'une propriété liste.
        if tail == Some("length") && (self.field(head) || self.prop(head, Some(PropKind::List))) {
            return true;
        }
        let filter_ok = match tail {
            None => true,
            Some("safe") => true,
            // Mots entre astérisques mis en valeur.
            Some(t) if t.starts_with("emphasis(") => true,
            Some(t) => crate::dates::format_of(t).is_some(),
        };
        // Champ, sinon un autre champ : `(a or b)`.
        let fallback = || {
            let inner = head
                .strip_prefix('(')
                .and_then(|h| h.strip_suffix(')'))
                .unwrap_or(head);
            let parts: Vec<&str> = inner.split(" or ").map(str::trim).collect();
            parts.len() == 2 && parts.iter().all(|p| self.field(p))
        };
        if filter_ok && (self.field(head) || fallback()) {
            return true;
        }
        if tail.is_none() && self.prop(head, Some(PropKind::Text)) {
            return true;
        }
        if let Some(l) = &self.locale
            && tail.is_none()
            && [".name", ".code", ".url"]
                .iter()
                .any(|s| head == format!("{l}{s}"))
        {
            return true;
        }
        false
    }
    /// Valeur d'attribut reconnue (`{{ x }}` seul : champ, adresse, propriété, langue).
    fn attr_value(&self, name: &str, v: &str) -> bool {
        if !v.contains("{{") && !v.contains("{%") {
            return true;
        }
        let inner = |s: &str| {
            s.trim()
                .strip_prefix("{{")
                .and_then(|x| x.strip_suffix("}}"))
                .map(|x| x.trim().to_string())
        };
        if name == "style" {
            let s = v.trim().trim_end_matches(';').trim();
            if let Some(e) = s
                .strip_prefix("background-image:")
                .and_then(|x| x.trim().strip_prefix("url("))
                .and_then(|x| x.strip_suffix(')'))
                .map(|x| x.trim().trim_matches(|c| c == '\'' || c == '"'))
                .and_then(inner)
            {
                return self.field(&e);
            }
            return false;
        }
        // Classe tirée de la valeur d'un champ : `block block--{{ b.style or 'plein' }}`.
        if name == "class"
            && let Some((_, _, f, _)) = crate::style::value_class(v)
        {
            return self.field(&f) || self.prop(&f, Some(PropKind::Text));
        }
        // Classe ajoutée selon un champ ou une propriété : `carte{% if x %} carte--y{% endif %}`.
        if name == "class" && v.contains("{%") && !v.contains("{{") {
            return crate::style::cond_classes(v)
                .is_some_and(|(_, conds)| conds.iter().all(|(c, _)| self.condition(c)));
        }
        // Adresse écrite autour d'un champ (`/projets/?expertise={{ slug }}`).
        if name == "href"
            && let (Some(i), Some(j)) = (v.find("{{"), v.rfind("}}"))
            && i < j
            && !v[..i].contains('{')
            && !v[j + 2..].contains('{')
            && (i > 0 || j + 2 < v.len())
        {
            let e = v[i + 2..j].trim();
            return self.field(e) || self.prop(e, Some(PropKind::Text));
        }
        let Some(e) = inner(v) else { return false };
        // Étiquettes d'un élément filtré : valeurs d'un champ liste (`x | join(' ')`).
        if name == "data-revel-tags"
            && let Some((h, t)) = e.split_once('|')
            && t.trim().starts_with("join(")
        {
            let h = h.trim();
            return self.field(h) || self.prop(h, Some(PropKind::List));
        }
        self.field(&e)
            || self.url.as_deref() == Some(e.as_str())
            || (name == "href" && self.prop(&e, Some(PropKind::Link)))
            || (matches!(name, "src" | "poster") && self.prop(&e, Some(PropKind::Image)))
            || self.output(&e)
    }
    fn condition(&self, cond: &str) -> bool {
        let c = cond.trim();
        let c = c.strip_prefix("not ").map(str::trim).unwrap_or(c);
        // Liste non vide : champ liste, liste de collection.
        let c = c
            .strip_suffix("|length")
            .or_else(|| c.strip_suffix("| length"))
            .map(|x| {
                let x = x.trim();
                x.strip_prefix('(')
                    .and_then(|y| y.strip_suffix(')'))
                    .unwrap_or(x)
                    .trim()
            })
            .unwrap_or(c);
        self.field(c)
            || self.prop(c, Some(PropKind::Switch))
            || Query::parse(c).is_some()
            || FieldList::parse(c).is_some_and(|fl| match fl {
                FieldList::Values { field } | FieldList::Refs { field, .. } => {
                    self.field(&field) || self.prop(&field, Some(PropKind::List))
                }
            })
            // Clauses de l'interface (rempli, vide, égal à…) sur des champs ou des propriétés.
            || crate::visibility::read(cond).is_some_and(|c| {
                c.clauses
                    .iter()
                    .all(|k| self.field(&k.field) || self.prop(&k.field, None))
            })
    }
}

/// Expression sans espaces (comparaison de formes).
fn squash(e: &str) -> String {
    e.chars().filter(|c| !c.is_whitespace()).collect()
}

struct Audit<'a> {
    site: &'a Site,
    out: Vec<Issue>,
    owner: String,
}

impl Audit<'_> {
    fn push(&mut self, node: Option<&str>, kind: Kind, detail: impl Into<String>) {
        self.out.push(Issue {
            owner: self.owner.clone(),
            node: node.map(str::to_string),
            kind,
            detail: detail.into(),
        });
    }

    fn nodes(&mut self, nodes: &[Node], ctx: &Ctx) {
        // Élément « Code » : un élément dont le seul contenu est du HTML écrit.
        for n in nodes {
            self.node(n, ctx);
        }
    }

    fn node(&mut self, n: &Node, ctx: &Ctx) {
        let id = revel_model::ids::node_id(n);
        match n {
            Node::Text { .. } | Node::Slot => {}
            Node::Html { html } => {
                let t = html.trim_start();
                if !(t.is_empty() || t.starts_with("<!") || ctx.head) {
                    self.push(None, Kind::Html, short(html));
                }
            }
            Node::Element {
                tag,
                attrs,
                children,
                ..
            } => {
                // `<head>` (titre, référencement, styles) : réglé par les paramètres de la page et
                // du site, absent des calques ; attributs de `<html>` : langue du site.
                if tag.eq_ignore_ascii_case("head") {
                    return;
                }
                let html = tag.eq_ignore_ascii_case("html");
                let body = tag.eq_ignore_ascii_case("body");
                for a in attrs.iter().filter(|a| {
                    !html
                        && !(body
                            && a.name == "class"
                            && squash(a.value.as_deref().unwrap_or("")) == "{{bodyClass}}")
                }) {
                    if let Some(w) = &a.when {
                        let locale_current = ctx
                            .locale
                            .as_deref()
                            .is_some_and(|l| w.trim() == format!("{l}.current"));
                        if !locale_current {
                            self.push(id, Kind::Attribute, format!("{} si {w}", a.name));
                        }
                    }
                    if let Some(v) = &a.value
                        && !ctx.head
                        && !ctx.attr_value(&a.name, v)
                    {
                        self.push(id, Kind::Attribute, format!("{}=\"{}\"", a.name, short(v)));
                    }
                }
                let head = ctx.head || tag.eq_ignore_ascii_case("head");
                let inner = Ctx {
                    head,
                    ..ctx.clone()
                };
                // Élément « Code » : HTML écrit seul dans l'élément.
                if let [Node::Html { .. }] = children.as_slice() {
                    return;
                }
                self.nodes(children, &inner);
            }
            Node::Out { expr } => {
                let base_title = ctx.head && expr.trim_start().starts_with("seo.title if seo");
                if !ctx.output(expr) && !base_title {
                    self.push(None, Kind::Expression, format!("{{{{ {} }}}}", short(expr)));
                }
            }
            Node::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                if !ctx.condition(cond) {
                    self.push(id, Kind::Condition, short(cond));
                }
                self.nodes(then, ctx);
                self.nodes(otherwise, ctx);
            }
            Node::Each {
                expr,
                item,
                children,
                empty,
                ..
            } => {
                let mut inner = ctx.clone();
                if let Some(q) = Query::parse(expr) {
                    // Contenus mis en premier : champ du contenu affiché ou contenus choisis.
                    if let Some(f) = q.first.as_deref()
                        && crate::collection::first_picks(f).is_none()
                        && !(ctx.field(f) || ctx.prop(f, Some(PropKind::List)))
                    {
                        self.push(id, Kind::List, short(expr));
                    }
                    inner.fields = Some(format!("{item}.data."));
                    inner.url = Some(format!("{item}.url"));
                    inner.value = None;
                } else if let Some(fl) = FieldList::parse(expr).filter(|fl| match fl {
                    FieldList::Values { field } | FieldList::Refs { field, .. } => {
                        ctx.field(field) || ctx.prop(field, Some(PropKind::List))
                    }
                }) {
                    match fl {
                        FieldList::Values { .. } => inner.value = Some(item.trim().to_string()),
                        FieldList::Refs { .. } => {
                            inner.fields = Some(format!("{item}.data."));
                            inner.url = Some(format!("{item}.url"));
                            inner.value = None;
                        }
                    }
                } else if expr.trim() == "locales" {
                    inner.locale = Some(item.trim().to_string());
                } else {
                    self.push(id, Kind::List, short(expr));
                }
                self.nodes(children, &inner);
                self.nodes(empty, &inner);
            }
            Node::Set { name, expr } => {
                self.push(None, Kind::Variable, format!("{name} = {}", short(expr)));
            }
            Node::SetBlock { name, children, .. } => {
                self.push(id, Kind::Variable, format!("{name} (bloc)"));
                self.nodes(children, ctx);
            }
            Node::Use {
                component, args, ..
            } => {
                let props = self
                    .site
                    .component(component)
                    .map(|c| c.props.clone())
                    .unwrap_or_default();
                for (name, e) in args {
                    let literal =
                        crate::canvas::literal(e).is_some() || matches!(e.trim(), "true" | "false");
                    let kind = props.iter().find(|p| &p.name == name).map(|p| p.kind);
                    let bound = ctx.output(e)
                        || (kind == Some(PropKind::Link) && ctx.url.as_deref() == Some(e.trim()))
                        || (kind == Some(PropKind::List)
                            && (ctx.field(e) || ctx.prop(e, Some(PropKind::List))))
                        || (kind == Some(PropKind::Switch) && ctx.condition(e));
                    if !literal && !bound {
                        self.push(id, Kind::Argument, format!("{name} = {}", short(e)));
                    }
                }
            }
            Node::Image(img) => {
                let ok = |e: &Option<String>| {
                    e.as_deref().is_none_or(|e| {
                        crate::canvas::literal(e).is_some()
                            || ctx.field(e)
                            || ctx.prop(e, Some(PropKind::Text))
                    })
                };
                let src_ok = crate::canvas::literal(&img.src).is_some()
                    || ctx.field(&img.src)
                    || ctx.prop(&img.src, Some(PropKind::Image));
                let fixed = [
                    &img.sizes,
                    &img.class,
                    &img.ratio,
                    &img.loading,
                    &img.widths,
                ]
                .iter()
                .all(|e| {
                    e.as_deref()
                        .is_none_or(|e| crate::canvas::literal(e).is_some())
                });
                if !src_ok || !ok(&img.alt) || !fixed {
                    self.push(id, Kind::Image, short(&img.src));
                }
            }
        }
    }
}

fn short(s: &str) -> String {
    let t: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > 90 {
        format!("{}…", t.chars().take(90).collect::<String>())
    } else {
        t
    }
}

fn owner_name(o: &Owner) -> String {
    match o {
        Owner::Page(id) => format!("page:{id}"),
        Owner::Layout(id) => format!("layout:{id}"),
        Owner::Component(id) => format!("component:{id}"),
    }
}

/// Variables de page réglées dans les paramètres de la page : référencement, indexation, classe
/// du corps de page, thème.
pub const PAGE_SETTINGS: &[&str] = &["seo", "noindex", "bodyClass", "theme", "ogType", "date"];

/// Relevé du site entier, dans l'ordre : paramètres, gabarits, pages, composants.
pub fn audit(site: &Site) -> Vec<Issue> {
    let mut a = Audit {
        site,
        out: Vec::new(),
        owner: "site".into(),
    };
    // Filtres propres au site : relevés là où une expression les emploie (le titre de l'onglet,
    // dans `<head>`, se règle dans Paramètres › Référencement).
    for l in &site.layouts {
        a.owner = owner_name(&Owner::Layout(l.id.clone()));
        let mut ctx = Ctx::default();
        // Modèle des pages d'une collection : champs du contenu affiché.
        let template = site.collections.iter().any(|c| {
            c.layout.as_deref().is_some_and(|t| {
                t == l.id || l.id.strip_prefix(t).is_some_and(|r| r.starts_with("--"))
            })
        });
        if template {
            ctx.fields = Some(String::new());
            ctx.url = Some("page.url".into());
        }
        let mut slot = false;
        revel_model::ids::walk(&l.body, &mut |n| slot |= matches!(n, Node::Slot));
        if !slot && !template {
            a.push(None, Kind::NoSlot, l.id.clone());
        }
        for k in l
            .props
            .keys()
            .filter(|k| !PAGE_SETTINGS.contains(&k.as_str()))
        {
            a.push(None, Kind::PageData, k.clone());
        }
        a.nodes(&l.body, &ctx);
    }
    for p in &site.pages {
        a.owner = owner_name(&Owner::Page(p.id.clone()));
        // Fichier du serveur (`.htaccess`, `robots.txt`, `.php`) : texte écrit tel quel, réglé
        // dans les paramètres de la page.
        let html_page = p.path.ends_with('/') || p.path.ends_with(".html");
        if !html_page
            && !p.path.ends_with(".xml")
            && p.body
                .iter()
                .all(|n| matches!(n, Node::Text { .. } | Node::Html { .. }))
        {
            continue;
        }
        for k in p
            .props
            .keys()
            .filter(|k| !PAGE_SETTINGS.contains(&k.as_str()))
        {
            a.push(None, Kind::PageData, k.clone());
        }
        for (k, e) in &p.computed {
            a.push(None, Kind::PageData, format!("{k} = {}", short(e)));
        }
        a.nodes(&p.body, &Ctx::default());
    }
    for c in &site.components {
        a.owner = owner_name(&Owner::Component(c.id.clone()));
        let ctx = Ctx {
            props: c.props.iter().map(|p| (p.name.clone(), p.kind)).collect(),
            ..Default::default()
        };
        a.nodes(&c.body, &ctx);
    }
    a.out
}

/// Nombre de points par nature.
pub fn summary(issues: &[Issue]) -> Vec<(Kind, usize)> {
    let mut m: std::collections::BTreeMap<Kind, usize> = Default::default();
    for i in issues {
        *m.entry(i.kind).or_default() += 1;
    }
    m.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use revel_model::{Attr, Component, PropDef};

    fn el(tag: &str, attrs: &[(&str, &str)], children: Vec<Node>) -> Node {
        Node::Element {
            id: Some(format!("e-{tag}")),
            tag: tag.into(),
            attrs: attrs
                .iter()
                .map(|(n, v)| Attr {
                    name: (*n).into(),
                    value: Some((*v).into()),
                    when: None,
                })
                .collect(),
            children,
        }
    }
    fn out(e: &str) -> Node {
        Node::Out { expr: e.into() }
    }

    #[test]
    fn formes_de_l_interface_et_points_releves() {
        let mut s = Site::new("Essai", "fr");
        s.layouts.push(crate::make::base_layout(&s));
        let mut p: revel_model::Page =
            serde_json::from_value(serde_json::json!({ "id": "index", "path": "/" })).unwrap();
        let list = Query {
            collection: "projets".into(),
            ..Default::default()
        }
        .to_expr();
        p.body = vec![
            el(
                "h1",
                &[],
                vec![Node::Text {
                    text: "Bonjour".into(),
                }],
            ),
            Node::Each {
                id: Some("l".into()),
                expr: list,
                item: "item".into(),
                children: vec![el(
                    "a",
                    &[("href", "{{ item.url }}")],
                    vec![
                        out("item.data.title"),
                        out("item.data.date | date_text(locale, \"long\")"),
                    ],
                )],
                empty: vec![],
            },
            // Relevés : variable, valeur calculée, liste calculée.
            Node::Set {
                name: "x".into(),
                expr: "[1, 2]".into(),
            },
            el("p", &[], vec![out("home.texte | safe")]),
            Node::Each {
                id: Some("m".into()),
                expr: "home.featured".into(),
                item: "slug".into(),
                children: vec![],
                empty: vec![],
            },
        ];
        s.pages.push(p);
        let mut c = Component::new(
            "chiffre",
            vec![el("div", &[], vec![el("strong", &[], vec![out("valeur")])])],
        );
        c.props.push(PropDef {
            name: "valeur".into(),
            label: None,
            kind: PropKind::Text,
            default: None,
        });
        s.components.push(c);
        let issues = audit(&s);
        let kinds: Vec<Kind> = issues.iter().map(|i| i.kind).collect();
        assert_eq!(
            kinds,
            vec![Kind::Variable, Kind::Expression, Kind::List],
            "{issues:?}"
        );
        assert!(issues.iter().all(|i| i.owner == "page:index"));
        // Gabarit dont la page est affichée par une expression : signalé.
        s.layouts[0].body = vec![el("main", &[], vec![out("content | safe")])];
        let issues = audit(&s);
        assert!(issues.iter().any(|i| i.kind == Kind::NoSlot));
    }
}
