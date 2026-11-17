//! Site importé remis dans la forme d'un site construit dans Revel (D-178) : ce qu'un
//! webdesigner aurait fait à la main.
//!
//! - [`tools`] : constructions reconnues (accordéon, onglets, carrousel, barre de navigation)
//!   refaites avec les outils de Revel, réglables depuis leur panneau ; reconnaissance par la
//!   forme du code, les rôles ARIA, les bibliothèques connues et les noms de classes, même
//!   incertaine (un outil se modifie, un assemblage écrit à la main non) ;
//! - classes de mêmes valeurs regroupées ([`revel_edit::classes::organize`]) ;
//! - [`components`] : éléments répétés de même forme devenus des composants (textes, liens et
//!   images en propriétés) ;
//! - [`variables`] : couleurs et tailles répétées devenues des variables du site.
//!
//! Les trois dernières passes ne changent pas le rendu ; la première remplace l'allure des
//! pièces de l'outil (flèches, bouton du menu) par celle de Revel et garde le contenu.

use revel_model::{Attr, Node, Site};

mod components;
mod tools;
mod variables;

/// Passes choisies.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Constructions refaites avec les outils de Revel.
    pub tools: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { tools: true }
    }
}

/// Bilan.
#[derive(Debug, Default, PartialEq)]
pub struct Report {
    /// Outils reconnus : nature (`accordéon`…) et nombre.
    pub tools: Vec<(String, usize)>,
    /// Constructions reconnues laissées telles quelles (contenu qu'un outil ne reprendrait pas).
    pub skipped: Vec<String>,
    /// Composants créés : nom et nombre d'utilisations.
    pub components: Vec<(String, usize)>,
    /// Couleurs et tailles devenues des variables du site.
    pub colors: usize,
    pub sizes: usize,
    /// Classes retirées par le regroupement.
    pub classes: usize,
}

/// Remet le site dans la forme d'un site construit dans Revel.
pub fn refine(site: &mut Site, opts: &Options) -> Report {
    let mut report = Report::default();
    site.ensure_ids();
    if opts.tools {
        tools::run(site, &mut report);
        site.ensure_ids();
    }
    if let Some((cmd, n)) = revel_edit::classes::organize(site)
        && revel_edit::apply(site, &cmd).is_ok()
    {
        report.classes = n;
    }
    // Site qui a déjà ses composants (Eleventy, Framer) : pas de seconde lecture.
    if site.components.is_empty() {
        components::run(site, &mut report);
        site.ensure_ids();
    }
    variables::run(site, &mut report);
    site.ensure_ids();
    report
}

/// Bilan en une phrase (avertissement d'import) ; `None` : rien de changé.
pub fn summary(r: &Report) -> Option<String> {
    let mut parts = Vec::new();
    if !r.tools.is_empty() {
        let list: Vec<String> = r.tools.iter().map(|(k, n)| format!("{n} {k}")).collect();
        parts.push(format!(
            "outils de Revel reconnus : {} (allure des pièces à vérifier)",
            list.join(", ")
        ));
    }
    if !r.components.is_empty() {
        let list: Vec<String> = r
            .components
            .iter()
            .map(|(k, n)| format!("{k} ×{n}"))
            .collect();
        parts.push(format!("composants créés : {}", list.join(", ")));
    }
    if r.colors + r.sizes > 0 {
        parts.push(format!(
            "{} couleurs et {} tailles devenues des variables du site",
            r.colors, r.sizes
        ));
    }
    if r.classes > 0 {
        parts.push(format!("{} classes de mêmes valeurs regroupées", r.classes));
    }
    if !r.skipped.is_empty() {
        parts.push(format!(
            "laissés tels quels (contenu qu'un outil ne reprendrait pas) : {}",
            r.skipped.join(", ")
        ));
    }
    (!parts.is_empty()).then(|| format!("site remis en forme : {}", parts.join(" ; ")))
}

// --- Aides communes -----------------------------------------------------------------------------

pub(crate) fn attr<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .and_then(|a| a.value.as_deref())
}

pub(crate) fn has_attr(attrs: &[Attr], name: &str) -> bool {
    attrs.iter().any(|a| a.name == name)
}

/// Élément d'un outil de Revel (racine, partie, élément répété) ou marqué pour un outil.
pub(crate) fn tool_marked(attrs: &[Attr]) -> bool {
    attrs.iter().any(|a| {
        matches!(
            a.name.as_str(),
            revel_model::WIDGET_ATTR
                | revel_model::PART_ATTR
                | revel_model::ITEM_ATTR
                | revel_model::BASE_ATTR
                | revel_model::SLOT_ATTR
                | "data-revel-tabs"
                | "data-revel-tab"
                | "data-revel-panel"
                | "data-revel-slider"
                | "data-revel-track"
                | "data-revel-accordion"
                | "data-revel-form"
                | "data-revel-loop"
        )
    })
}

/// Valeur écrite en dur (ni expression, ni condition).
pub(crate) fn is_static(a: &Attr) -> bool {
    a.when.is_none()
        && a.value
            .as_deref()
            .is_none_or(|v| !v.contains("{{") && !v.contains("{%"))
}

/// Classes d'un élément.
pub(crate) fn classes(n: &Node) -> Vec<&str> {
    match n {
        Node::Element { attrs, .. } => attr(attrs, "class")
            .map(|c| c.split_whitespace().collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

pub(crate) fn tag(n: &Node) -> &str {
    match n {
        Node::Element { tag, .. } => tag.as_str(),
        _ => "",
    }
}

pub(crate) fn kids(n: &Node) -> &[Node] {
    match n {
        Node::Element { children, .. } => children,
        _ => &[],
    }
}

/// Enfants éléments (textes, commentaires et blancs passés).
pub(crate) fn element_kids(n: &Node) -> Vec<&Node> {
    kids(n)
        .iter()
        .filter(|c| matches!(c, Node::Element { .. } | Node::Image(_)))
        .collect()
}

/// Texte d'un sous-arbre (espaces réduits).
pub(crate) fn plain(n: &Node) -> String {
    fn go(n: &Node, out: &mut String) {
        match n {
            Node::Text { text } => out.push_str(text),
            Node::Element { tag, children, .. } => {
                if matches!(tag.as_str(), "script" | "style" | "svg") {
                    return;
                }
                if tag == "br" {
                    out.push(' ');
                }
                for c in children {
                    go(c, out);
                }
                // Fin d'un bloc : les mots de deux blocs ne se collent pas.
                if !matches!(
                    tag.as_str(),
                    "span"
                        | "a"
                        | "b"
                        | "i"
                        | "em"
                        | "strong"
                        | "small"
                        | "sup"
                        | "sub"
                        | "u"
                        | "mark"
                        | "abbr"
                        | "code"
                ) {
                    out.push(' ');
                }
            }
            _ => {}
        }
    }
    let mut s = String::new();
    go(n, &mut s);
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Mots d'un nom de classe ou d'identifiant, en minuscules (`navBar__menu-btn` : `nav`, `bar`,
/// `menu`, `btn`).
pub(crate) fn words(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in name.chars() {
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        prev_lower = c.is_lowercase();
        cur.extend(c.to_lowercase());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Un des mots `list` est dans les classes ou l'identifiant de l'élément.
pub(crate) fn named(n: &Node, list: &[&str]) -> bool {
    let Node::Element { attrs, .. } = n else {
        return false;
    };
    let mut all: Vec<String> = classes(n).iter().flat_map(|c| words(c)).collect();
    if let Some(id) = attr(attrs, "id") {
        all.extend(words(id));
    }
    all.iter().any(|w| list.contains(&w.as_str()))
}

/// Retire les identifiants d'un sous-arbre (copie d'un modèle : identifiants redonnés ensuite).
pub(crate) fn strip_ids(n: &mut Node) {
    match n {
        Node::Element { id, .. }
        | Node::If { id, .. }
        | Node::Each { id, .. }
        | Node::SetBlock { id, .. }
        | Node::Use { id, .. } => *id = None,
        Node::Image(i) => i.id = None,
        _ => {}
    }
    for list in revel_model::ids::child_lists_mut(n) {
        for c in list {
            strip_ids(c);
        }
    }
}

#[cfg(test)]
mod tests;
