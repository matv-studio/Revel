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
    /// Sélecteur que le panneau Styles ne règle pas (`.a > .b`, `::before`, `h1, h2`) (D-151).
    StyleSelector,
    /// Requête média écrite (ni point de rupture du site, ni couleurs du mode sombre).
    StyleMedia,
    /// Bloc de code CSS (`@keyframes`, `@supports`…).
    StyleCode,
    /// Variable locale (`--x` hors des couleurs et tailles du site) ou valeur qui en lit une.
    StyleVariable,
    /// Propriété sans réglage dans le panneau Styles.
    StyleProperty,
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
    /// Variables des environnements de publication (`env.staging`).
    env: Vec<String>,
    /// Fichier du serveur (`.htaccess`) : son texte écrit est du texte, pas du HTML.
    text_file: bool,
    /// Dans une liste : « premier contenu », « dernier contenu » (`loop.first`, `loop.last`).
    in_loop: bool,
    /// Écritures propres au site (filtres du site à une valeur : `person_name`), choisies comme
    /// une écriture de date.
    formats: Vec<String>,
}

/// Écritures propres au site : ses filtres qui ne prennent que la valeur (« Prénom NOM »).
pub fn site_formats(site: &Site) -> Vec<String> {
    site.settings
        .filters
        .iter()
        .filter(|(_, f)| f.params.len() == 1)
        .map(|(n, _)| n.clone())
        .collect()
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
    /// Information du site (`site.email`, `site.address.street`) : réglée dans Paramètres ›
    /// Informations du site, reliée aux textes, liens, images, listes et conditions.
    fn site_value(&self, e: &str) -> bool {
        site_path(e)
    }
    fn env_var(&self, e: &str) -> bool {
        e.trim()
            .strip_prefix("env.")
            .is_some_and(|v| self.env.iter().any(|x| x == v))
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
        if tail == Some("length")
            && (self.field(head)
                || self.prop(head, Some(PropKind::List))
                // Nombre de contenus d'une collection.
                || Query::parse(head).is_some_and(|q| q.to_expr() == head.trim()))
        {
            return true;
        }
        let filter_ok = match tail {
            None => true,
            Some("safe") => true,
            // Mots entre astérisques mis en valeur.
            Some(t) if t.starts_with("emphasis(") => true,
            Some(t) => crate::dates::format_of(t).is_some() || self.format_ok(t),
        };
        // Champ, sinon un autre champ ou un texte écrit : `(a or b)`, `a or 'Vidéo'`.
        let fallback = || {
            let inner = head
                .strip_prefix('(')
                .and_then(|h| h.strip_suffix(')'))
                .unwrap_or(head);
            let parts: Vec<&str> = inner.split(" or ").map(str::trim).collect();
            let value = |p: &str| self.field(p) || self.prop(p, Some(PropKind::Text));
            parts.len() == 2
                && value(parts[0])
                && (value(parts[1]) || crate::canvas::literal(parts[1]).is_some())
        };
        if filter_ok && (self.field(head) || self.site_value(head) || fallback()) {
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
    /// Valeur d'attribut reconnue : texte écrit, valeur reliée (texte autour, « seulement si
    /// remplie »), style relié, classes selon un champ.
    fn attr_value(&self, name: &str, v: &str) -> bool {
        if !v.contains("{{") && !v.contains("{%") {
            return true;
        }
        if name == "style" {
            let s = v.trim().trim_end_matches(';').trim();
            let inner = |s: &str| {
                s.trim()
                    .strip_prefix("{{")
                    .and_then(|x| x.strip_suffix("}}"))
                    .map(|x| x.trim().to_string())
            };
            if let Some(e) = s
                .strip_prefix("background-image:")
                .and_then(|x| x.trim().strip_prefix("url("))
                .and_then(|x| x.strip_suffix(')'))
                .map(|x| x.trim().trim_matches(|c| c == '\'' || c == '"'))
                .and_then(inner)
            {
                return self.field(&e);
            }
            // Propriétés de style reliées (`--dot:{{ couleur }}`).
            return crate::attrbind::style_decls(v).is_some_and(|decls| {
                decls
                    .iter()
                    .filter_map(|d| d.bound.as_ref())
                    .all(|b| self.bound_ok("style", &b.value))
            });
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
        if name == "class" {
            return false;
        }
        // Valeur reliée, texte écrit autour, partie écrite seulement si la valeur est remplie
        // (`Lire la vidéo{% if label %} : {{ label }}{% endif %}`).
        match crate::attrbind::Bound::parse(v) {
            Some(b) => {
                self.bound_ok(name, &b.value)
                    && (!b.only_if || self.condition(crate::attrbind::head(&b.value)))
            }
            None => false,
        }
    }
    /// Valeur reliée dans un attribut : champ, information du site, adresse du contenu,
    /// propriété (lien, image, texte), valeur affichée (écriture, repli), numéro d'appel ou
    /// adresse pour un plan, nombre de valeurs ou de contenus.
    fn bound_ok(&self, name: &str, e: &str) -> bool {
        let e = e.trim();
        // Étiquettes d'un élément filtré : valeurs d'un champ liste (`x | join(' ')`).
        if name == "data-revel-tags"
            && let Some((h, t)) = e.split_once('|')
            && t.trim().starts_with("join(")
        {
            let h = h.trim();
            return self.field(h) || self.prop(h, Some(PropKind::List));
        }
        // Lien d'appel (`tel:{{ site.phone | tel }}`), plan, écriture du site
        // (`embed/{{ video | youtube_id }}`).
        let written = e
            .strip_suffix("| tel")
            .or_else(|| e.strip_suffix("|tel"))
            .or_else(|| e.strip_suffix("| map_query"))
            .map(str::trim)
            .or_else(|| {
                e.split_once('|')
                    .filter(|(_, f)| self.format_ok(f.trim()))
                    .map(|(h, _)| h.trim())
            });
        self.field(e)
            || self.site_value(e)
            || self.url.as_deref() == Some(e)
            || (name == "href" && self.prop(e, Some(PropKind::Link)))
            || (matches!(name, "src" | "poster") && self.prop(e, Some(PropKind::Image)))
            || self.prop(e, Some(PropKind::Text))
            || written.is_some_and(|t| self.site_value(t) || self.field(t) || self.prop(t, None))
            || self.output(e)
    }
    /// Écriture choisie d'une valeur : filtre propre au site (« Prénom NOM »), valeurs d'une
    /// liste séparées (`join(' ')`), éventuellement suivis de `| safe`.
    fn format_ok(&self, tail: &str) -> bool {
        let parts: Vec<&str> = tail.split('|').map(str::trim).collect();
        let (first, rest) = match parts.split_first() {
            Some(x) => x,
            None => return false,
        };
        if !rest.iter().all(|r| *r == "safe") {
            return false;
        }
        if self.formats.iter().any(|f| f == first) {
            return true;
        }
        first
            .strip_prefix("join(")
            .and_then(|a| a.strip_suffix(')'))
            .is_some_and(|a| crate::canvas::literal(a.trim()).is_some())
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
            // Contenu qui a une page.
            || self.url.as_deref() == Some(c)
            || self.prop(c, Some(PropKind::Switch))
            || Query::parse(c).is_some()
            || FieldList::parse(c).is_some_and(|fl| match fl {
                FieldList::Values { field } | FieldList::Refs { field, .. } => {
                    self.field(&field) || self.prop(&field, Some(PropKind::List))
                }
            })
            // Clauses de l'interface (rempli, vide, égal à…) sur des champs, des propriétés ou
            // des variables de l'environnement de publication.
            || crate::visibility::read(cond).is_some_and(|c| {
                c.clauses.iter().all(|k| {
                    self.field(&k.field)
                        || self.url.as_deref() == Some(k.field.as_str())
                        || self.prop(&k.field, None)
                        || self.env_var(&k.field)
                        || self.site_value(&k.field)
                        // Premier ou dernier contenu de la liste.
                        || (self.in_loop && matches!(k.field.as_str(), "loop.first" | "loop.last"))
                })
            })
    }
}

/// Texte relié à une information du site dans une forme du panneau : `site.x`, avec au plus
/// `| safe`, `| emphasis(…)` ou une écriture de date.
pub fn is_site_output(expr: &str) -> bool {
    let e = expr.trim();
    let (head, tail) = match e.split_once('|') {
        Some((h, t)) => (h.trim(), Some(t.trim())),
        None => (e, None),
    };
    let filter_ok = match tail {
        None | Some("safe") => true,
        Some(t) if t.starts_with("emphasis(") && !t.contains('|') => true,
        Some(t) => crate::dates::format_of(t).is_some(),
    };
    filter_ok && site_path(head)
}

/// Lien relié à une information du site : `{{ site.x }}`, `mailto:{{ site.x }}`,
/// `tel:{{ site.x | tel }}`, plan `https://maps.google.com/?q={{ site.x | map_query }}`.
pub fn is_site_href(v: &str) -> bool {
    let v = v.trim();
    let inner = |s: &str| -> Option<String> {
        Some(s.strip_prefix("{{")?.strip_suffix("}}")?.trim().to_string())
    };
    let with = |e: &str, filter: &str| -> bool {
        e.split_once('|')
            .is_some_and(|(h, f)| f.trim() == filter && site_path(h.trim()))
    };
    if let Some(r) = v.strip_prefix("mailto:") {
        return inner(r).is_some_and(|e| site_path(&e));
    }
    if let Some(r) = v.strip_prefix("tel:") {
        return inner(r).is_some_and(|e| with(&e, "tel"));
    }
    if let Some(r) = v.strip_prefix(MAP_PREFIX) {
        return inner(r).is_some_and(|e| with(&e, "map_query"));
    }
    inner(v).is_some_and(|e| site_path(&e))
}

/// Début d'un lien « plan » (adresse sur une carte).
pub const MAP_PREFIX: &str = "https://maps.google.com/?q=";

/// Chemin d'une information du site (`site.email`, `site.address.street`).
pub fn site_path(e: &str) -> bool {
    e.trim()
        .strip_prefix("site.")
        .is_some_and(|r| r.split('.').all(is_field))
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
                if !(t.is_empty() || t.starts_with("<!") || ctx.head || ctx.text_file) {
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
                        // Attribut écrit seulement si le champ qu'il reprend est rempli
                        // (`poster="{{ cover }}"` si `cover`) : comportement d'un attribut relié.
                        let same_field =
                            a.value.as_deref().is_some_and(|v| {
                                v.trim()
                                    .strip_prefix("{{")
                                    .and_then(|x| x.strip_suffix("}}"))
                                    .is_some_and(|x| x.trim() == w.trim())
                            }) && (ctx.field(w) || ctx.prop(w, None) || ctx.site_value(w));
                        if !locale_current && !same_field {
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
                inner.in_loop = true;
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
                        ctx.field(field)
                            || ctx.prop(field, Some(PropKind::List))
                            // Liste des informations du site (menu : liens et leurs textes).
                            || ctx.site_value(field)
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
                component,
                args,
                children,
                ..
            } => {
                // Contenu propre de l'utilisation (emplacement du composant) : relevé comme la page.
                self.nodes(children, ctx);
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
                    || ctx.site_value(&img.src)
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

/// Variables des environnements de publication d'un site qui en a plusieurs (`staging` pour
/// `env.staging`) : une condition peut les lire (« affiché seulement en recette »).
pub fn env_fields(site: &Site) -> Vec<String> {
    let envs = &site.settings.environments;
    if envs.len() < 2 && !envs.keys().any(|k| k != "production") {
        return Vec::new();
    }
    let mut out: Vec<String> = vec!["name".into()];
    for v in envs.values() {
        if let serde_json::Value::Object(m) = v {
            for k in m.keys() {
                if !out.contains(k) && is_field(k) {
                    out.push(k.clone());
                }
            }
        }
    }
    out
}

/// Propriétés d'un gabarit et de ses parents (valeurs choisies par chaque page).
pub fn layout_fields(site: &Site, layout: Option<&str>) -> Vec<revel_model::PropDef> {
    let mut out: Vec<revel_model::PropDef> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut cur = layout.map(str::to_string);
    while let Some(id) = cur {
        if seen.contains(&id) {
            break;
        }
        seen.push(id.clone());
        let Some(l) = site.layouts.iter().find(|l| l.id == id) else {
            break;
        };
        for f in &l.fields {
            if !out.iter().any(|o| o.name == f.name) {
                out.push(f.clone());
            }
        }
        cur = l.parent.clone();
    }
    out
}

/// Relevé du site entier, dans l'ordre : paramètres, gabarits, pages, composants.
pub fn audit(site: &Site) -> Vec<Issue> {
    let mut a = Audit {
        site,
        out: Vec::new(),
        owner: "site".into(),
    };
    let env = env_fields(site);
    let formats = site_formats(site);
    // Filtres propres au site : relevés là où une expression les emploie (le titre de l'onglet,
    // dans `<head>`, se règle dans Paramètres › Référencement).
    for l in &site.layouts {
        a.owner = owner_name(&Owner::Layout(l.id.clone()));
        let fields = layout_fields(site, Some(&l.id));
        let mut ctx = Ctx {
            props: fields.iter().map(|p| (p.name.clone(), p.kind)).collect(),
            env: env.clone(),
            formats: formats.clone(),
            ..Default::default()
        };
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
        for k in l.props.keys().filter(|k| {
            !PAGE_SETTINGS.contains(&k.as_str()) && !fields.iter().any(|f| &f.name == *k)
        }) {
            a.push(None, Kind::PageData, k.clone());
        }
        a.nodes(&l.body, &ctx);
    }
    for p in &site.pages {
        a.owner = owner_name(&Owner::Page(p.id.clone()));
        // Fichier du serveur (`.htaccess`, `robots.txt`, `.php`) : texte écrit tel quel, réglé
        // dans les paramètres de la page.
        let html_page = p.path.ends_with('/') || p.path.ends_with(".html");
        // Texte avec ses balises (valeurs, conditions, listes), modifié tel quel dans les
        // paramètres de la page.
        if crate::textfile::is_server_file(&p.path) && crate::textfile::round_trips(&p.body) {
            continue;
        }
        // Valeurs des propriétés de son gabarit : réglages de la page.
        let fields = layout_fields(site, p.layout.as_deref());
        for k in p.props.keys().filter(|k| {
            !PAGE_SETTINGS.contains(&k.as_str()) && !fields.iter().any(|f| &f.name == *k)
        }) {
            a.push(None, Kind::PageData, k.clone());
        }
        for (k, e) in &p.computed {
            a.push(None, Kind::PageData, format!("{k} = {}", short(e)));
        }
        // Fichier du serveur : texte modifiable seulement s'il est tout écrit (sans boucle ni
        // condition, même d'environnement).
        a.nodes(
            &p.body,
            &Ctx {
                env: if html_page || p.path.ends_with(".xml") {
                    env.clone()
                } else {
                    Vec::new()
                },
                text_file: !html_page && !p.path.ends_with(".xml"),
                formats: formats.clone(),
                ..Default::default()
            },
        );
    }
    for c in &site.components {
        a.owner = owner_name(&Owner::Component(c.id.clone()));
        let ctx = Ctx {
            props: c.props.iter().map(|p| (p.name.clone(), p.kind)).collect(),
            env: env.clone(),
            formats: formats.clone(),
            ..Default::default()
        };
        a.nodes(&c.body, &ctx);
    }
    a.out
}

/// Relevé complet : calques ([`audit`]) puis styles ([`crate::styleaudit::audit`]).
pub fn audit_all(site: &Site) -> Vec<Issue> {
    let mut out = audit(site);
    out.extend(crate::styleaudit::audit(site));
    out
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
