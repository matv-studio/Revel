//! Langues du site (3.8) : textes écrits dans les pages à traduire, et site vu dans une autre
//! langue.
//!
//! Les textes écrits dans les pages, gabarits et composants (textes, attributs lisibles comme
//! `alt`, propriétés texte des composants, variables texte des pages) sont traduits dans
//! [`revel_model::Locale::texts`], par leur texte dans la langue principale (espaces regroupés) :
//! un même texte écrit à plusieurs endroits n'est traduit qu'une fois, et la traduction ne dépend
//! pas de la place de l'élément. Les textes des fichiers de données et des contenus sont traduits
//! dans le dossier de la langue (D-23), pas ici.

use std::collections::BTreeMap;

use revel_model::ids::{child_lists, child_lists_mut};
use revel_model::{Node, Site};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::EditError;
use crate::address::{Owner, Parent, Target};
use crate::canvas::{literal, quote};

/// Attributs dont la valeur est un texte lu par les visiteurs.
pub const TEXT_ATTRS: &[&str] = &["alt", "title", "placeholder", "aria-label"];

/// Éléments dont le texte n'est pas lu par les visiteurs.
const CODE_TAGS: &[&str] = &["script", "style", "template", "svg", "code", "pre"];

/// Texte regroupé : espaces successifs réduits à un seul, sans espaces autour.
pub fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Texte à traduire : au moins une lettre, sans expression.
pub fn translatable(s: &str) -> bool {
    s.chars().any(char::is_alphabetic) && !s.contains("{{") && !s.contains("{%")
}

/// Valeur de variable qui ressemble à un texte lu (pas une adresse, une classe, un nom de
/// fichier ou un identifiant).
pub fn text_like(s: &str) -> bool {
    let t = s.trim();
    if !translatable(t)
        || t.starts_with(['/', '#', '.'])
        || t.contains("://")
        || t.starts_with("mailto:")
        || t.starts_with("tel:")
    {
        return false;
    }
    if t.contains(char::is_whitespace) {
        return true;
    }
    // Un seul mot : texte s'il porte une majuscule ou une lettre accentuée et pas de séparateur
    // de chemin.
    !t.contains(['/', '_', '.', '='])
        && t.chars()
            .any(|c| c.is_uppercase() || (c.is_alphabetic() && !c.is_ascii()))
}

/// Marque « à relire » d'un texte des pages ([`revel_model::Locale::machine`]).
pub fn text_key(source: &str) -> String {
    format!("text:{}", normalize(source))
}

/// Marque « à relire » d'une valeur d'un fichier de données (chemin comme dans les expressions).
pub fn data_key(path: &[crate::canvas::Seg]) -> String {
    let parts: Vec<String> = path
        .iter()
        .map(|p| match p {
            crate::canvas::Seg::Key(k) => k.clone(),
            crate::canvas::Seg::Index(i) => i.to_string(),
        })
        .collect();
    format!("data:{}", parts.join("."))
}

/// Marque « à relire » d'un contenu (dossier de la collection dans la langue principale).
pub fn entry_key(folder: &str, file: &str) -> String {
    format!("entry:{}/{file}", folder.trim_end_matches('/'))
}

/// Texte écrit dans le site et endroits où il apparaît.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteText {
    /// Texte dans la langue principale (espaces regroupés) : clé de la traduction.
    pub source: String,
    /// Pages, gabarits et composants où il est écrit (sans doublon, dans l'ordre du site).
    pub places: Vec<Owner>,
}

/// Texte d'un fichier de données à traduire (vue Localisation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataText {
    /// Chemin de la valeur comme dans les expressions (`pages`, `accueil`, `hero`, `title`).
    pub path: Vec<crate::canvas::Seg>,
    /// Texte de la langue principale.
    pub source: String,
    /// Valeur dans le fichier de la langue ; `None` : fichier ou valeur absents (non traduit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Endroit d'un texte traduisible, désigné depuis le canevas ou l'inspecteur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TextPlace {
    /// Texte écrit (repère `text` du canevas).
    Text { parent: Parent, index: usize },
    /// Attribut d'un élément (`alt`, `title`…) ou texte alternatif d'une image.
    Attr { at: Target, name: String },
    /// Propriété texte d'une utilisation de composant (sa valeur, sinon celle par défaut).
    Arg { at: Target, name: String },
}

/// Textes à traduire du site, dans l'ordre : pages, gabarits, composants.
pub fn texts(site: &Site) -> Vec<SiteText> {
    let mut out: Vec<SiteText> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    let mut add = |s: &str, owner: &Owner| {
        let n = normalize(s);
        if n.is_empty() || !translatable(&n) {
            return;
        }
        match index.get(&n) {
            Some(&i) => {
                if !out[i].places.contains(owner) {
                    out[i].places.push(owner.clone());
                }
            }
            None => {
                index.insert(n.clone(), out.len());
                out.push(SiteText {
                    source: n,
                    places: vec![owner.clone()],
                });
            }
        }
    };
    for p in site
        .pages
        .iter()
        .filter(|p| revel_model::localizable_path(&p.path))
    {
        let o = Owner::Page(p.id.clone());
        for v in p.props.values() {
            value_texts(v, &mut |s| add(s, &o));
        }
        node_texts(site, &p.body, &mut |s| add(s, &o));
    }
    for l in &site.layouts {
        let o = Owner::Layout(l.id.clone());
        for v in l.props.values() {
            value_texts(v, &mut |s| add(s, &o));
        }
        node_texts(site, &l.body, &mut |s| add(s, &o));
    }
    for c in &site.components {
        let o = Owner::Component(c.id.clone());
        for p in &c.props {
            if p.kind == revel_model::PropKind::Text
                && let Some(Value::String(s)) = &p.default
            {
                add(s, &o);
            }
        }
        node_texts(site, &c.body, &mut |s| add(s, &o));
    }
    out
}

/// Textes d'une valeur de variable (textes des objets et des listes compris).
fn value_texts(v: &Value, f: &mut dyn FnMut(&str)) {
    match v {
        Value::String(s) if text_like(s) => f(s),
        Value::Array(a) => a.iter().for_each(|x| value_texts(x, f)),
        Value::Object(m) => m.values().for_each(|x| value_texts(x, f)),
        _ => {}
    }
}

/// Propriétés texte d'un composant.
fn text_props(site: &Site, component: &str) -> Vec<String> {
    site.component(component)
        .map(|c| {
            c.props
                .iter()
                .filter(|p| p.kind == revel_model::PropKind::Text)
                .map(|p| p.name.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn node_texts(site: &Site, nodes: &[Node], f: &mut dyn FnMut(&str)) {
    for n in nodes {
        match n {
            Node::Text { text } => f(text),
            Node::Element { tag, attrs, .. } => {
                if CODE_TAGS.contains(&tag.to_ascii_lowercase().as_str()) {
                    continue;
                }
                for a in attrs {
                    if TEXT_ATTRS.contains(&a.name.as_str())
                        && let Some(v) = &a.value
                    {
                        f(v);
                    }
                }
            }
            Node::Image(img) => {
                if let Some(s) = img.alt.as_deref().and_then(literal) {
                    f(&s);
                }
            }
            Node::Use {
                component, args, ..
            } => {
                for name in text_props(site, component) {
                    if let Some(s) = args.get(&name).and_then(|e| literal(e)) {
                        f(&s);
                    }
                }
            }
            _ => {}
        }
        for list in child_lists(n) {
            node_texts(site, list, f);
        }
    }
}

/// Traduction d'un texte (espaces autour gardés) ; `None` : pas de traduction.
fn lookup(texts: &BTreeMap<String, String>, s: &str) -> Option<String> {
    let t = texts.get(&normalize(s))?;
    let lead = &s[..s.len() - s.trim_start().len()];
    let trail = &s[s.trim_end().len()..];
    Some(format!("{lead}{t}{trail}"))
}

fn translate_value(texts: &BTreeMap<String, String>, v: &mut Value) {
    match v {
        Value::String(s) => {
            if text_like(s)
                && let Some(t) = lookup(texts, s)
            {
                *s = t;
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| translate_value(texts, x)),
        Value::Object(m) => m.values_mut().for_each(|x| translate_value(texts, x)),
        _ => {}
    }
}

fn translate_nodes(
    texts: &BTreeMap<String, String>,
    props: &BTreeMap<String, Vec<String>>,
    nodes: &mut [Node],
) {
    for n in nodes.iter_mut() {
        match n {
            Node::Text { text } => {
                if let Some(t) = lookup(texts, text) {
                    *text = t;
                }
            }
            Node::Element { tag, attrs, .. } => {
                if CODE_TAGS.contains(&tag.to_ascii_lowercase().as_str()) {
                    continue;
                }
                for a in attrs.iter_mut() {
                    if TEXT_ATTRS.contains(&a.name.as_str())
                        && let Some(v) = &a.value
                        && translatable(v)
                        && let Some(t) = lookup(texts, v)
                    {
                        a.value = Some(t);
                    }
                }
            }
            Node::Image(img) => {
                if let Some(t) = img
                    .alt
                    .as_deref()
                    .and_then(literal)
                    .and_then(|s| lookup(texts, &s))
                {
                    img.alt = Some(quote(&t));
                }
            }
            Node::Use {
                component, args, ..
            } => {
                for name in props.get(component.as_str()).into_iter().flatten() {
                    if let Some(e) = args.get_mut(name)
                        && let Some(t) = literal(e).and_then(|s| lookup(texts, &s))
                    {
                        *e = quote(&t);
                    }
                }
            }
            _ => {}
        }
        for list in child_lists_mut(n) {
            translate_nodes(texts, props, list);
        }
    }
}

/// Site vu dans une langue : textes écrits remplacés par leur traduction (les textes non
/// traduits restent dans la langue principale). Même structure, mêmes identifiants : les repères
/// du canevas restent valables.
pub fn translate_site(site: &Site, texts: &BTreeMap<String, String>) -> Site {
    let mut s = site.clone();
    if texts.is_empty() {
        return s;
    }
    let props: BTreeMap<String, Vec<String>> = site
        .components
        .iter()
        .map(|c| (c.id.clone(), text_props(site, &c.id)))
        .collect();
    for p in &mut s.pages {
        p.props.values_mut().for_each(|v| translate_value(texts, v));
        translate_nodes(texts, &props, &mut p.body);
    }
    for l in &mut s.layouts {
        l.props.values_mut().for_each(|v| translate_value(texts, v));
        translate_nodes(texts, &props, &mut l.body);
    }
    for c in &mut s.components {
        for p in &mut c.props {
            if p.kind == revel_model::PropKind::Text
                && let Some(Value::String(d)) = &mut p.default
                && let Some(t) = lookup(texts, d)
            {
                *d = t;
            }
        }
        translate_nodes(texts, &props, &mut c.body);
    }
    s
}

/// Texte de la langue principale à un endroit (clé de sa traduction).
pub fn source_at(site: &Site, place: &TextPlace) -> Result<String, EditError> {
    let text = match place {
        TextPlace::Text { parent, index } => {
            match crate::address::list(site, parent)?.get(*index) {
                Some(Node::Text { text }) => text.clone(),
                Some(_) => return Err(EditError::NotText(*index)),
                None => return Err(EditError::Index(*index)),
            }
        }
        TextPlace::Attr { at, name } => match crate::address::node(site, at)? {
            Node::Element { attrs, .. } => attrs
                .iter()
                .find(|a| &a.name == name)
                .and_then(|a| a.value.clone())
                .ok_or_else(|| EditError::NotFound(name.clone()))?,
            Node::Image(img) if name == "alt" => img
                .alt
                .as_deref()
                .and_then(literal)
                .ok_or_else(|| EditError::NotFound(name.clone()))?,
            _ => return Err(EditError::WrongKind(at.node.clone())),
        },
        TextPlace::Arg { at, name } => match crate::address::node(site, at)? {
            Node::Use {
                component, args, ..
            } => match args.get(name) {
                Some(e) => literal(e).ok_or_else(|| EditError::Bound(name.clone()))?,
                None => match site
                    .component(component)
                    .and_then(|c| c.prop(name))
                    .and_then(|p| p.default.clone())
                {
                    Some(Value::String(s)) => s,
                    _ => return Err(EditError::NotFound(name.clone())),
                },
            },
            _ => return Err(EditError::WrongKind(at.node.clone())),
        },
    };
    let n = normalize(&text);
    if n.is_empty() || !translatable(&n) {
        return Err(EditError::Bound(n));
    }
    Ok(n)
}

/// Code de langue accepté pour une nouvelle langue du site.
pub(crate) fn check_new_locale(site: &Site, code: &str) -> Result<(), EditError> {
    if !revel_model::valid_locale_code(code)
        || code.eq_ignore_ascii_case(&site.default_locale)
        || site
            .locales
            .iter()
            .any(|l| l.code.eq_ignore_ascii_case(code))
    {
        return Err(EditError::InvalidLocale(code.to_string()));
    }
    // Une page de la langue principale ne doit pas déjà occuper le préfixe.
    let prefix = format!("/{code}/");
    if let Some(p) = site
        .pages
        .iter()
        .find(|p| p.path.starts_with(&prefix) || p.path == format!("/{code}"))
    {
        return Err(EditError::PathTaken(p.path.clone()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textes_lisibles() {
        for s in [
            "Bonjour le monde",
            "Agence",
            "Éditions",
            "Qui sommes-nous ?",
        ] {
            assert!(text_like(s), "{s}");
        }
        for s in [
            "/agence/",
            "page-accueil",
            "accueil",
            "img/logo.svg",
            "https://a.b",
            "#contact",
            "12",
            "{{ x }}",
        ] {
            assert!(!text_like(s), "{s}");
        }
        assert_eq!(
            normalize("  Nous sommes\n   une agence "),
            "Nous sommes une agence"
        );
        let t = BTreeMap::from([(
            "Nous sommes une agence".to_string(),
            "We are an agency".to_string(),
        )]);
        assert_eq!(
            lookup(&t, "\n  Nous sommes\n une agence  ").as_deref(),
            Some("\n  We are an agency  ")
        );
    }
}
