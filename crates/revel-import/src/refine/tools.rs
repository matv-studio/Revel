//! Constructions reconnues refaites avec les outils de Revel (D-178) : accordéon, onglets,
//! carrousel, barre de navigation.
//!
//! Indices lus, du plus sûr au moins sûr : forme du code (`details`, rôles ARIA `tab` et
//! `tabpanel`, `aria-expanded`), bibliothèques connues (Swiper, Slick, Splide, Glide, Flickity,
//! Owl, Embla, Keen, composants de Webflow et de Bootstrap), styles (`scroll-snap-type`), noms
//! de classes (`carousel`, `slider`, `tabs`, `accordion`, `faq`, `navbar`, `burger`…). Un indice
//! incertain suffit : l'outil reprend le contenu (textes, liens, images, classes du contenu) et
//! se règle dans son panneau, ce qu'un assemblage écrit à la main ne permet pas.
//!
//! Garde-fou : la construction n'est refaite que si tout son contenu est repris (mêmes textes,
//! mêmes médias) ; sinon elle est laissée telle quelle et signalée.

use super::{
    Report, attr, classes, element_kids, has_attr, is_static, kids, named, plain, strip_ids, tag,
    tool_marked,
};
use revel_edit::make::Preset;
use revel_model::{Attr, Node, PART_ATTR, Site, StyleItem, WIDGET_ATTR};
use std::collections::{BTreeSet, HashMap};

// --- Styles lus -----------------------------------------------------------------------------------

/// Déclarations des règles d'une seule classe (`.nom`), par classe.
struct Css {
    by_class: HashMap<String, Vec<(Option<String>, String, String)>>,
}

impl Css {
    fn new(site: &Site) -> Self {
        let mut by_class: HashMap<String, Vec<(Option<String>, String, String)>> = HashMap::new();
        for item in site.stylesheet.iter().flat_map(|s| &s.items) {
            let StyleItem::Rule(r) = item else { continue };
            let sel = r.selector.trim();
            let Some(name) = sel.strip_prefix('.') else {
                continue;
            };
            if name.is_empty()
                || !name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                continue;
            }
            let list = by_class.entry(name.to_string()).or_default();
            for d in &r.decls {
                list.push((r.media.clone(), d.prop.clone(), d.value.clone()));
            }
        }
        Self { by_class }
    }

    /// Valeur de base (hors requête média) d'une propriété, pour la première classe qui l'écrit.
    fn base(&self, classes: &[&str], prop: &str) -> Option<&str> {
        classes.iter().find_map(|c| {
            self.by_class
                .get(*c)?
                .iter()
                .find_map(|(m, p, v)| (m.is_none() && p == prop).then_some(v.as_str()))
        })
    }

    fn any_base(&self, classes: &[&str], pred: impl Fn(&str, &str) -> bool) -> bool {
        classes.iter().any(|c| {
            self.by_class
                .get(*c)
                .is_some_and(|l| l.iter().any(|(m, p, v)| m.is_none() && pred(p, v)))
        })
    }

    /// Plus grande largeur `max-width` d'une requête média où une déclaration vérifie `pred`.
    fn media_max(&self, classes: &[&str], pred: impl Fn(&str, &str) -> bool) -> Option<u32> {
        classes
            .iter()
            .filter_map(|c| self.by_class.get(*c))
            .flatten()
            .filter(|(_, p, v)| pred(p, v))
            .filter_map(|(m, _, _)| max_width(m.as_deref()?))
            .max()
    }
}

/// Largeur d'une requête `(max-width: 991px)`.
fn max_width(media: &str) -> Option<u32> {
    let rest = &media[media.find("max-width")? + "max-width".len()..];
    let digits: String = rest
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let n: f64 = digits.parse().ok()?;
    let unit = rest
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
    Some(if unit.starts_with("em") || unit.starts_with("rem") {
        (n * 16.0) as u32
    } else {
        n as u32
    })
}

// --- Aides ----------------------------------------------------------------------------------------

/// Classes posées par une bibliothèque de carrousel : leurs styles (diapositives cachées avant
/// le script, largeurs calculées) n'ont plus lieu d'être.
fn lib_class(c: &str) -> bool {
    [
        "swiper",
        "slick",
        "splide",
        "glide",
        "flickity",
        "owl-",
        "embla",
        "keen-slider",
        "w-slide",
        "w-slider",
        "carousel-cell",
        "carousel-item",
        "carousel-inner",
    ]
    .iter()
    .any(|p| c.starts_with(p))
}

fn kept_classes(n: &Node, strip_lib: bool) -> Option<String> {
    let list: Vec<&str> = classes(n)
        .into_iter()
        .filter(|c| !(strip_lib && lib_class(c)))
        .collect();
    (!list.is_empty()).then(|| list.join(" "))
}

fn set_attr(attrs: &mut Vec<Attr>, name: &str, value: Option<&str>) {
    attrs.retain(|a| a.name != name);
    attrs.push(Attr {
        name: name.into(),
        value: value.map(str::to_string),
        when: None,
    });
}

/// Pose (ou retire) les classes d'un élément, en tête de ses attributs.
fn set_class(n: &mut Node, class: Option<&str>) {
    if let Node::Element { attrs, .. } = n {
        attrs.retain(|a| a.name != "class");
        if let Some(c) = class.filter(|c| !c.trim().is_empty()) {
            attrs.insert(
                0,
                Attr {
                    name: "class".into(),
                    value: Some(c.to_string()),
                    when: None,
                },
            );
        }
    }
}

fn nth_el_mut(children: &mut [Node], n: usize) -> Option<&mut Node> {
    children
        .iter_mut()
        .filter(|c| matches!(c, Node::Element { .. } | Node::Image(_)))
        .nth(n)
}

fn kids_mut(n: &mut Node) -> Option<&mut Vec<Node>> {
    match n {
        Node::Element { children, .. } => Some(children),
        _ => None,
    }
}

fn find_part_mut<'a>(n: &'a mut Node, part: &str) -> Option<&'a mut Node> {
    if matches!(&*n, Node::Element { attrs, .. } if attr(attrs, PART_ATTR) == Some(part)) {
        return Some(n);
    }
    kids_mut(n)?.iter_mut().find_map(|c| find_part_mut(c, part))
}

fn text(t: &str) -> Node {
    Node::Text {
        text: t.to_string(),
    }
}

fn blank(n: &Node) -> bool {
    match n {
        Node::Text { text } => text.trim().is_empty(),
        Node::Html { html } => html.trim().is_empty() || html.trim_start().starts_with("<!--"),
        _ => false,
    }
}

/// Poids du contenu d'un sous-arbre : caractères de texte, médias et éléments de logique.
fn weight(n: &Node) -> (usize, usize) {
    match n {
        Node::Text { text } => (text.chars().filter(|c| !c.is_whitespace()).count(), 0),
        Node::Html { .. } => (0, usize::from(!blank(n))),
        Node::Image(_) => (0, 1),
        Node::Element { tag, children, .. } => {
            if matches!(tag.as_str(), "script" | "style" | "svg") {
                return (0, 0);
            }
            let own = usize::from(matches!(
                tag.as_str(),
                "img"
                    | "video"
                    | "audio"
                    | "iframe"
                    | "form"
                    | "input"
                    | "select"
                    | "textarea"
                    | "canvas"
                    | "object"
                    | "embed"
            ));
            children
                .iter()
                .map(weight)
                .fold((0, own), |a, b| (a.0 + b.0, a.1 + b.1))
        }
        _ => (0, 1),
    }
}

fn sum(list: &[(usize, usize)]) -> (usize, usize) {
    list.iter().fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1))
}

/// Élément écrit en dur (ni condition, ni expression dans ses attributs).
fn plain_element(n: &Node) -> bool {
    matches!(n, Node::Element { attrs, .. } if attrs.iter().all(is_static))
}

fn is_tool(n: &Node) -> bool {
    matches!(n, Node::Element { attrs, .. } if tool_marked(attrs))
}

/// Outil repris d'un site : il garde l'allure donnée par ses classes, celle de Revel ne s'ajoute
/// pas (`revel_render::toolcss`, règles `:not([data-revel-own])`).
const OWN_ATTR: &str = "data-revel-own";

/// Animations et identifiant de la construction d'origine, gardés sur l'outil.
fn carry(src: &Node, dst: &mut Node) {
    let (Node::Element { attrs: from, .. }, Node::Element { attrs: to, .. }) = (src, dst) else {
        return;
    };
    set_attr(to, OWN_ATTR, None);
    for a in from {
        if (a.name == "id" || a.name.starts_with("data-revel-")) && !has_attr(to, &a.name) {
            to.push(a.clone());
        }
    }
}

/// Descendants (préordre) jusqu'à `depth` niveaux.
fn descendants<'a>(n: &'a Node, depth: usize, out: &mut Vec<&'a Node>) {
    if depth == 0 {
        return;
    }
    for c in kids(n) {
        if matches!(c, Node::Element { .. }) {
            out.push(c);
            descendants(c, depth - 1, out);
        }
    }
}

fn has_class(n: &Node, list: &[&str]) -> bool {
    classes(n).iter().any(|c| list.contains(c))
}

fn el_attr<'a>(n: &'a Node, name: &str) -> Option<&'a str> {
    match n {
        Node::Element { attrs, .. } => attr(attrs, name),
        _ => None,
    }
}

fn el_has(n: &Node, name: &str) -> bool {
    matches!(n, Node::Element { attrs, .. } if has_attr(attrs, name))
}

/// Titre d'un en-tête (bouton, `summary`…) : son texte, s'il ne contient rien d'autre qu'un
/// dessin d'ouverture.
fn head_title(n: &Node) -> Option<String> {
    let t = plain(n);
    let (chars, media) = weight(n);
    (!t.is_empty() && chars <= 300 && media == 0).then_some(t)
}

// --- Modèles des outils -----------------------------------------------------------------------------

struct Protos {
    accordion: Node,
    tabs: Node,
    carousel: Node,
    navbar: Node,
}

impl Protos {
    fn new(site: &Site) -> Self {
        let fr = site.default_locale.starts_with("fr");
        let mut p = Self {
            accordion: Preset::Accordion.node(site, "Question|Réponse"),
            tabs: Preset::Tabs.node(site, "Onglet"),
            carousel: Preset::Carousel.node(
                site,
                if fr {
                    "Précédente|Suivante|"
                } else {
                    "Previous|Next|"
                },
            ),
            navbar: Preset::Navbar.node(site, "Marque|Menu|Lien"),
        };
        for n in [
            &mut p.accordion,
            &mut p.tabs,
            &mut p.carousel,
            &mut p.navbar,
        ] {
            strip_ids(n);
        }
        p
    }
}

// --- Accordéon --------------------------------------------------------------------------------------

struct AccItem {
    title: String,
    content: Vec<Node>,
    open: bool,
    class: Option<String>,
    head_class: Option<String>,
}

/// Volet écrit avec `details` et `summary`.
fn details_item(n: &Node) -> Option<AccItem> {
    if tag(n) != "details" || !plain_element(n) || is_tool(n) {
        return None;
    }
    let els = element_kids(n);
    let summary = *els.first().filter(|s| tag(s) == "summary")?;
    let title = head_title(summary)?;
    let content: Vec<Node> = kids(n)
        .iter()
        .filter(|c| !std::ptr::eq(*c, summary))
        .cloned()
        .collect();
    Some(AccItem {
        title,
        content,
        open: el_has(n, "open"),
        class: kept_classes(n, false),
        head_class: kept_classes(summary, false),
    })
}

const ACC_NAMES: &[&str] = &[
    "accordion",
    "accordions",
    "accordeon",
    "accordeons",
    "faq",
    "faqs",
    "collapse",
    "collapsible",
    "collapsibles",
    "expander",
    "depliant",
    "depliants",
];

fn open_state(n: &Node) -> bool {
    named(n, &["open", "opened", "active", "show", "expanded"])
        || el_attr(n, "aria-expanded") == Some("true")
}

/// Accordéon écrit avec des blocs (en-tête puis panneau) : volets et indice de confiance.
fn block_items(root: &Node) -> Option<(Vec<AccItem>, u32)> {
    let mut score = if named(root, ACC_NAMES) { 2 } else { 0 };
    let mut items = Vec::new();
    // Liste de définitions : `dt` puis `dd`.
    if tag(root) == "dl" {
        let els = element_kids(root);
        if els.len() < 2 || !els.len().is_multiple_of(2) {
            return None;
        }
        for pair in els.chunks(2) {
            if tag(pair[0]) != "dt" || tag(pair[1]) != "dd" {
                return None;
            }
            items.push(AccItem {
                title: head_title(pair[0])?,
                content: kids(pair[1]).to_vec(),
                open: false,
                class: None,
                head_class: kept_classes(pair[0], false),
            });
        }
        return Some((items, score));
    }
    let (mut aria, mut item_named, mut head_named, mut panel_named) = (false, false, false, false);
    for it in element_kids(root) {
        if !plain_element(it) || is_tool(it) {
            return None;
        }
        let els = element_kids(it);
        let [head, panel] = els.as_slice() else {
            return None;
        };
        if !matches!(
            tag(head),
            "button" | "a" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "div" | "span" | "p" | "dt"
        ) || !matches!(
            tag(panel),
            "div" | "section" | "p" | "dd" | "ul" | "article"
        ) {
            return None;
        }
        let title = head_title(head)?;
        if weight(panel) == (0, 0) {
            return None;
        }
        let mut inner = vec![*head];
        descendants(head, 3, &mut inner);
        aria |= inner.iter().any(|x| {
            el_has(x, "aria-expanded")
                || el_has(x, "aria-controls")
                || matches!(
                    el_attr(x, "data-toggle").or(el_attr(x, "data-bs-toggle")),
                    Some("collapse")
                )
        });
        item_named |= named(it, &["item", "question", "row", "entry"]) || named(it, ACC_NAMES);
        head_named |= inner.iter().any(|x| {
            named(
                x,
                &[
                    "header", "head", "title", "question", "trigger", "toggle", "button",
                    "summary", "titre",
                ],
            )
        });
        panel_named |= named(
            panel,
            &[
                "panel", "content", "body", "answer", "reponse", "collapse", "text", "contenu",
            ],
        );
        items.push(AccItem {
            title,
            // Panneau gardé avec ses classes d'allure ; son état replié n'a plus lieu d'être
            // (retiré de ses styles, voir `unhide`).
            content: vec![shown(panel)],
            open: open_state(it)
                || inner
                    .iter()
                    .any(|x| el_attr(x, "aria-expanded") == Some("true")),
            class: kept_classes(it, false),
            head_class: kept_classes(head, false),
        });
    }
    score += u32::from(aria) * 2
        + u32::from(item_named)
        + u32::from(head_named)
        + u32::from(panel_named);
    Some((items, score))
}

fn build_accordion(proto: &Node, root_class: Option<&str>, items: Vec<AccItem>) -> Option<Node> {
    let mut root = proto.clone();
    let row0 = element_kids(proto).first().map(|n| (*n).clone())?;
    let rows: Option<Vec<Node>> = items
        .into_iter()
        .map(|it| {
            let mut row = row0.clone();
            set_class(&mut row, it.class.as_deref());
            if let Node::Element { attrs, .. } = &mut row
                && it.open
            {
                set_attr(attrs, "open", None);
            }
            let list = kids_mut(&mut row)?;
            let summary = nth_el_mut(list, 0)?;
            set_class(summary, it.head_class.as_deref());
            *kids_mut(find_part_mut(summary, "name")?)? = vec![text(&it.title)];
            *kids_mut(nth_el_mut(list, 1)?)? = it.content;
            Some(row)
        })
        .collect();
    *kids_mut(&mut root)? = rows?;
    set_class(&mut root, root_class);
    Some(root)
}

// --- Onglets ----------------------------------------------------------------------------------------

struct Tab {
    label: String,
    class: Option<String>,
    content: Vec<Node>,
}

/// Copie d'un panneau (onglet, volet) montrée par l'outil : sans `hidden` ni rôle ARIA, ses
/// classes gardées (leur état caché est retiré des styles, voir `unhide`).
fn shown(panel: &Node) -> Node {
    let mut p = panel.clone();
    if let Node::Element { tag, attrs, .. } = &mut p {
        attrs.retain(|a| {
            !matches!(
                a.name.as_str(),
                "hidden" | "role" | "aria-labelledby" | "aria-hidden" | "tabindex" | "id" | "style"
            ) || (a.name == "style"
                && a.value
                    .as_deref()
                    .is_some_and(|v| !v.contains("display") && !v.contains("height")))
        });
        if matches!(tag.as_str(), "dd" | "li") {
            *tag = "div".into();
        }
    }
    p
}

fn is_tab(n: &Node) -> bool {
    el_attr(n, "role") == Some("tab") || has_class(n, &["w-tab-link"])
}

fn is_panel(n: &Node) -> bool {
    el_attr(n, "role") == Some("tabpanel") || has_class(n, &["w-tab-pane", "tab-pane"])
}

/// Onglets reconnus : onglets, classes de leur liste, classes des panneaux (état caché à retirer
/// des styles), classes d'état de l'onglet choisi.
type TabsFound = (Vec<Tab>, Option<String>, Vec<String>, Vec<String>);

fn tabs_of(root: &Node) -> Option<TabsFound> {
    let mut all = Vec::new();
    descendants(root, 5, &mut all);
    let mut tabs: Vec<&Node> = all.iter().copied().filter(|n| is_tab(n)).collect();
    let mut panels: Vec<&Node> = all.iter().copied().filter(|n| is_panel(n)).collect();
    if tabs.len() < 2 || tabs.len() != panels.len() {
        // Par les noms : une liste de boutons courts, puis autant de panneaux.
        if !named(root, &["tab", "tabs", "tabbed", "onglet", "onglets"]) {
            return None;
        }
        let list = all.iter().copied().find(|n| {
            let k = element_kids(n);
            k.len() >= 2
                && k.iter().all(|c| {
                    matches!(tag(c), "button" | "a" | "li")
                        && tag(c) == tag(k[0])
                        && !plain(c).is_empty()
                        && plain(c).chars().count() <= 40
                        && weight(c).1 == 0
                })
        })?;
        tabs = element_kids(list);
        // Panneaux : les autres enfants de la racine, ou les enfants de l'unique autre enfant.
        let branch = |c: &Node| {
            let mut d = vec![c];
            descendants(c, 5, &mut d);
            d.iter().any(|x| std::ptr::eq(*x, list))
        };
        let others: Vec<&Node> = element_kids(root)
            .into_iter()
            .filter(|c| !branch(c))
            .collect();
        panels = if others.len() == tabs.len() {
            others
        } else if let [one] = others.as_slice() {
            element_kids(one)
        } else {
            return None;
        };
        if panels.len() != tabs.len() {
            return None;
        }
    }
    if tabs
        .iter()
        .chain(&panels)
        .any(|n| !plain_element(n) || is_tool(n))
    {
        return None;
    }
    let mut out: Vec<Tab> = tabs
        .iter()
        .zip(&panels)
        .map(|(t, p)| {
            let label = plain(t);
            (!label.is_empty() && weight(t).1 == 0).then(|| Tab {
                label,
                class: kept_classes(t, true).filter(|_| tag(t) != "li"),
                // Panneau sans classe : son contenu seul.
                content: if classes(p).is_empty() {
                    kids(p).to_vec()
                } else {
                    vec![shown(p)]
                },
            })
        })
        .collect::<Option<_>>()?;
    // Tout le contenu est dans les onglets et les panneaux.
    let parts: Vec<(usize, usize)> = tabs.iter().chain(&panels).map(|n| weight(n)).collect();
    if weight(root) != sum(&parts) {
        return None;
    }
    // Classes de la liste des onglets (leur parent commun).
    let list_class = all
        .iter()
        .copied()
        .find(|n| kids(n).iter().any(|c| std::ptr::eq(c, tabs[0])))
        .filter(|l| !std::ptr::eq(*l, root))
        .and_then(|l| kept_classes(l, true));
    let hide: Vec<String> = panels
        .iter()
        .flat_map(|p| classes(p))
        .map(str::to_string)
        .collect();
    // Classes portées par un seul onglet (celui choisi dans la page d'origine) : état « actif ».
    let state: Vec<String> = tabs
        .iter()
        .flat_map(|t| classes(t))
        .filter(|c| tabs.iter().filter(|t| has_class(t, &[c])).count() == 1 && tabs.len() > 1)
        .map(str::to_string)
        .collect();
    for t in &mut out {
        if let Some(c) = &t.class {
            let kept: Vec<&str> = c
                .split_whitespace()
                .filter(|x| !state.iter().any(|s| s == x))
                .collect();
            t.class = (!kept.is_empty()).then(|| kept.join(" "));
        }
    }
    Some((out, list_class, hide, state))
}

fn build_tabs(
    proto: &Node,
    root_class: Option<&str>,
    list_class: Option<&str>,
    tabs: Vec<Tab>,
) -> Option<Node> {
    let mut root = proto.clone();
    let els = element_kids(proto);
    let (mut list, panel0) = ((*els.first()?).clone(), (*els.get(1)?).clone());
    set_class(&mut list, list_class);
    let button0 = element_kids(&list).first().map(|n| (*n).clone())?;
    let mut buttons = Vec::new();
    let mut panels = Vec::new();
    for t in tabs {
        let mut b = button0.clone();
        set_class(&mut b, t.class.as_deref());
        *kids_mut(&mut b)? = vec![text(&t.label)];
        buttons.push(b);
        let mut p = panel0.clone();
        *kids_mut(&mut p)? = t.content;
        panels.push(p);
    }
    *kids_mut(&mut list)? = buttons;
    let mut children = vec![list];
    children.extend(panels);
    *kids_mut(&mut root)? = children;
    set_class(&mut root, root_class);
    Some(root)
}

// --- Carrousel --------------------------------------------------------------------------------------

const CAROUSEL_NAMES: &[&str] = &[
    "carousel",
    "carrousel",
    "slider",
    "slideshow",
    "diaporama",
    "swiper",
    "slick",
    "splide",
    "glide",
    "flickity",
    "embla",
];

const TRACK_CLASSES: &[&str] = &[
    "swiper-wrapper",
    "splide__list",
    "glide__slides",
    "embla__container",
    "w-slider-mask",
    "slick-track",
    "owl-stage",
    "carousel-inner",
];

enum Found<T> {
    Yes(T),
    /// Reconnu, mais du contenu ne serait pas repris.
    Lossy,
    No,
}

/// Diapositives d'un carrousel et nombre montré à la fois.
fn slides_of(root: &Node, css: &Css) -> Found<(Vec<Node>, u32)> {
    let mut all = vec![root];
    descendants(root, 4, &mut all);
    let lib_root = classes(root).iter().any(|c| lib_class(c)) || el_has(root, "data-flickity");
    let by_name = lib_root || named(root, CAROUSEL_NAMES);
    let same_kids = |n: &Node, min: usize| {
        let k = element_kids(n);
        k.len() >= min
            && k.iter()
                .all(|c| tag(c) == tag(k[0]) && plain_element(c) && !is_tool(c))
    };
    // Piste d'une bibliothèque : celle de cette racine (enfant direct, ou racine nommée), pas
    // celle d'un carrousel plus bas dans la page.
    let lib_track = all
        .iter()
        .copied()
        .find(|n| has_class(n, TRACK_CLASSES))
        .filter(|t| by_name || kids(root).iter().any(|c| std::ptr::eq(c, *t)));
    let track = if let Some(t) = lib_track {
        Some(t)
    } else if by_name {
        // Piste nommée, sinon premier élément dont les enfants sont de même balise.
        all.iter()
            .copied()
            .find(|n| {
                !std::ptr::eq(*n, root)
                    && named(
                        n,
                        &[
                            "track", "wrapper", "list", "slides", "inner", "items", "stage",
                            "mask", "piste",
                        ],
                    )
                    && same_kids(n, 2)
            })
            .or_else(|| all.iter().copied().find(|n| same_kids(n, 2)))
    } else {
        // Sans nom : piste qui défile par crans (styles).
        let c = classes(root);
        let snap = css.any_base(&c, |p, v| p == "scroll-snap-type" && v != "none");
        let scroll = css.any_base(&c, |p, v| {
            matches!(p, "overflow-x" | "overflow") && matches!(v, "auto" | "scroll")
        }) && css.any_base(&c, |p, v| p == "display" && v.contains("flex"));
        ((snap || scroll) && same_kids(root, 3)).then_some(root)
    };
    let Some(track) = track else {
        return Found::No;
    };
    let slides: Vec<&Node> = element_kids(track)
        .into_iter()
        .filter(|s| {
            !has_class(
                s,
                &["slick-cloned", "swiper-slide-duplicate", "clone", "cloned"],
            )
        })
        .collect();
    if slides.len() < 2 || slides.iter().any(|s| !plain_element(s) || is_tool(s)) {
        return if by_name { Found::Lossy } else { Found::No };
    }
    // Hors de la piste : commandes seulement (flèches, points, compteur).
    let parts: Vec<(usize, usize)> = slides.iter().map(|s| weight(s)).collect();
    let (total, kept) = (weight(root), sum(&parts));
    let controls_only = {
        fn outside<'a>(n: &'a Node, track: &Node, out: &mut Vec<&'a Node>) {
            for c in kids(n) {
                if std::ptr::eq(c, track) || !matches!(c, Node::Element { .. }) {
                    continue;
                }
                out.push(c);
                outside(c, track, out);
            }
        }
        let mut out = Vec::new();
        if !std::ptr::eq(root, track) {
            outside(root, track, &mut out);
        }
        out.iter().all(|n| {
            weight(n).0 == 0
                || matches!(tag(n), "button")
                || named(
                    n,
                    &[
                        "prev",
                        "next",
                        "previous",
                        "arrow",
                        "arrows",
                        "fleche",
                        "fleches",
                        "dot",
                        "dots",
                        "bullet",
                        "bullets",
                        "pagination",
                        "control",
                        "controls",
                        "nav",
                        "navigation",
                        "count",
                        "counter",
                        "fraction",
                        "progress",
                        "scrollbar",
                        "indicators",
                        "button",
                        "btn",
                    ],
                )
        })
    };
    if total.1 != kept.1 || !controls_only || total.0.saturating_sub(kept.0) > 60 {
        return Found::Lossy;
    }
    let mut cards = false;
    let out: Vec<Node> = slides
        .iter()
        .map(|s| {
            let mut s = (*s).clone();
            let class = kept_classes(&s, true);
            set_class(&mut s, class.as_deref());
            // Enveloppe vide de sens (classe de la bibliothèque seulement) : son contenu.
            let single = {
                let k = kids(&s);
                let els: Vec<&Node> = k.iter().filter(|c| !blank(c)).collect();
                (class.is_none()
                    && els.len() == 1
                    && matches!(els[0], Node::Element { .. } | Node::Image(_)))
                .then(|| els[0].clone())
            };
            let mut s = single.unwrap_or(s);
            if let Node::Element { tag, .. } = &mut s
                && tag == "li"
            {
                *tag = "div".into();
            }
            cards |= weight(&s).0 > 0;
            s
        })
        .collect();
    // Largeur écrite des diapositives (`33.33%`) : autant de diapositives montrées.
    let show = slides
        .first()
        .and_then(|s| {
            let c = classes(s);
            css.base(&c, "width")
                .or(css.base(&c, "flex-basis"))
                .or(css.base(&c, "min-width"))
                .and_then(|v| v.trim().strip_suffix('%')?.trim().parse::<f64>().ok())
        })
        .filter(|p| *p > 5.0 && *p <= 100.0)
        .map(|p| (100.0 / p).round() as u32);
    // Une largeur écrite reste portée par la classe de la diapositive ; sinon cartes par trois.
    let fixed = slides.first().is_some_and(|s| {
        css.any_base(&classes(s), |p, _| {
            matches!(p, "width" | "flex-basis" | "flex" | "min-width")
        })
    });
    let show = show.unwrap_or(if cards && !fixed && out.len() >= 3 {
        3
    } else {
        1
    });
    Found::Yes((out, show.clamp(1, 6)))
}

fn build_carousel(
    proto: &Node,
    root_class: Option<&str>,
    slides: Vec<Node>,
    show: u32,
) -> Option<Node> {
    let mut root = proto.clone();
    let track = nth_el_mut(kids_mut(&mut root)?, 0)?;
    *kids_mut(track)? = slides;
    if show > 1
        && let Node::Element { attrs, .. } = &mut root
    {
        set_attr(attrs, "style", Some(&format!("--rv-show:{show}")));
    }
    set_class(&mut root, root_class);
    Some(root)
}

// --- Barre de navigation ----------------------------------------------------------------------------

/// Bouton du menu replié (trois barres).
fn is_burger(n: &Node) -> bool {
    if !matches!(tag(n), "button" | "a" | "div" | "span" | "label") || !plain_element(n) {
        return false;
    }
    let t = plain(n);
    let label = el_attr(n, "aria-label").unwrap_or("").to_lowercase();
    let short = t.chars().count() <= 10;
    let menu = named(
        n,
        &["burger", "hamburger", "toggler", "toggle", "menu", "nav"],
    ) || label.contains("menu")
        || label.contains("navigation")
        || el_has(n, "aria-controls")
        || el_has(n, "aria-expanded");
    match tag(n) {
        "button" => short && weight(n).1 == 0 && menu,
        // Lien ou bloc : seulement s'il porte un nom de bouton de menu et pas d'adresse de page.
        _ => {
            t.is_empty()
                && weight(n).1 == 0
                && (named(n, &["burger", "hamburger", "toggler"])
                    || has_class(n, &["w-nav-button"]))
                && el_attr(n, "href").is_none_or(|h| h.is_empty() || h.starts_with('#'))
        }
    }
}

fn home_link(href: &str) -> bool {
    matches!(
        href.trim(),
        "" | "/" | "./" | "#" | "#top" | "index.html" | "/index.html" | "./index.html"
    )
}

struct NavParts {
    /// Chemin (indices d'enfants) de l'élément remplacé, depuis l'en-tête.
    at: Vec<usize>,
    node: Node,
    /// Classes du menu et reprise de l'allure du menu ouvert (voir `Cx::menus`).
    menus: Vec<(String, bool)>,
}

fn navbar_of(h: &Node, css: &Css, proto: &Node) -> Found<NavParts> {
    // Liens, bouton et logo, avec leur chemin.
    #[derive(Clone, Copy, PartialEq)]
    enum K {
        Link,
        Burger,
        Logo,
    }
    fn scan<'a>(n: &'a Node, path: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, &'a Node, K)>) {
        for (i, c) in kids(n).iter().enumerate() {
            if !matches!(c, Node::Element { .. }) {
                continue;
            }
            path.push(i);
            if is_burger(c) {
                out.push((path.clone(), c, K::Burger));
            } else if tag(c) == "a" {
                out.push((path.clone(), c, K::Link));
            } else if named(c, &["logo", "brand", "marque"])
                && !{
                    let mut d = Vec::new();
                    descendants(c, 6, &mut d);
                    d.iter().any(|x| tag(x) == "a")
                }
            {
                out.push((path.clone(), c, K::Logo));
            } else {
                scan(c, path, out);
            }
            path.pop();
        }
    }
    let mut found = Vec::new();
    scan(h, &mut Vec::new(), &mut found);
    let links: Vec<&(Vec<usize>, &Node, K)> = found.iter().filter(|f| f.2 == K::Link).collect();
    let burger = found.iter().find(|f| f.2 == K::Burger);
    let logo = found.iter().find(|f| f.2 == K::Logo);
    if found.iter().filter(|f| f.2 == K::Burger).count() > 1
        || found.iter().filter(|f| f.2 == K::Logo).count() > 1
    {
        return Found::Lossy;
    }
    // Marque : le logo nommé, sinon le premier lien s'il mène à l'accueil ou porte une image.
    let first_is_brand = logo.is_none()
        && links.first().is_some_and(|(_, a, _)| {
            let img = {
                let mut d = Vec::new();
                descendants(a, 4, &mut d);
                d.iter()
                    .any(|x| matches!(tag(x), "img" | "svg" | "picture"))
                    || kids(a).iter().any(|c| matches!(c, Node::Image(_)))
            };
            img || named(a, &["logo", "brand", "marque"])
                || (el_attr(a, "href").is_some_and(home_link) && links.len() >= 3)
        });
    let menu: Vec<&(Vec<usize>, &Node, K)> = links
        .iter()
        .copied()
        .skip(usize::from(first_is_brand))
        .collect();
    if menu.len() < 2
        || menu
            .iter()
            .any(|(_, a, _)| !plain_element(a) || plain(a).is_empty() && weight(a).1 == 0)
    {
        return Found::No;
    }
    let brand: Option<Node> = if let Some((_, l, _)) = logo {
        // Logo sans lien : devient le lien de la marque, vers l'accueil.
        let mut b = (*l).clone();
        if let Node::Element { tag, attrs, .. } = &mut b {
            *tag = "a".into();
            set_attr(attrs, "href", Some("/"));
        }
        Some(b)
    } else if first_is_brand {
        links.first().map(|(_, a, _)| (*a).clone())
    } else {
        None
    };
    // Sous-menus : non repris par l'outil.
    let mut all = Vec::new();
    descendants(h, 8, &mut all);
    let nested = all.iter().any(|n| {
        (tag(n) == "li" && {
            let mut d = Vec::new();
            descendants(n, 4, &mut d);
            d.iter().any(|x| matches!(tag(x), "ul" | "ol"))
        }) || named(n, &["dropdown", "submenu", "mega", "megamenu"])
    });
    let mut parts: Vec<(usize, usize)> = menu.iter().map(|(_, a, _)| weight(a)).collect();
    parts.extend(brand.as_ref().map(weight));
    parts.extend(burger.map(|(_, b, _)| weight(b)));
    if nested || weight(h) != sum(&parts) {
        return Found::Lossy;
    }
    // Élément remplacé : le plus petit qui contient la marque, les liens et le bouton.
    let mut paths: Vec<&Vec<usize>> = menu.iter().map(|(p, _, _)| p).collect();
    paths.extend(logo.map(|(p, _, _)| p));
    if first_is_brand {
        paths.extend(links.first().map(|(p, _, _)| p));
    }
    paths.extend(burger.map(|(p, _, _)| p));
    let mut at: Vec<usize> = paths[0][..paths[0].len() - 1].to_vec();
    for p in &paths {
        let common = at.iter().zip(p.iter()).take_while(|(a, b)| a == b).count();
        at.truncate(common.min(p.len() - 1));
    }
    let mut wrap = h;
    for i in &at {
        wrap = &kids(wrap)[*i];
    }
    // Conteneur des liens : son écart devient celui du menu.
    let mut menu_at: Vec<usize> = menu[0].0[..menu[0].0.len() - 1].to_vec();
    for (p, _, _) in &menu {
        let common = menu_at
            .iter()
            .zip(p.iter())
            .take_while(|(a, b)| a == b)
            .count();
        menu_at.truncate(common.min(p.len() - 1));
    }
    let mut menu_box = h;
    for i in &menu_at {
        menu_box = &kids(menu_box)[*i];
    }
    let box_classes = classes(menu_box);
    let gap = css
        .base(&box_classes, "column-gap")
        .or(css.base(&box_classes, "gap"))
        .map(|g| g.split_whitespace().last().unwrap_or(g).to_string())
        .or_else(|| {
            // Écart donné par les marges ou le remplissage des liens : pas d'écart en plus.
            menu.iter()
                .any(|(_, a, _)| {
                    css.any_base(&classes(a), |p, _| {
                        p.starts_with("margin") || p.starts_with("padding")
                    })
                })
                .then(|| "0".to_string())
        });
    // Largeur sous laquelle le site repliait son menu.
    let fold = burger
        .and_then(|(_, b, _)| css.media_max(&classes(b), |p, v| p == "display" && v != "none"))
        .or_else(|| css.media_max(&box_classes, |p, v| p == "display" && v == "none"));
    // Repli de l'outil : le plus petit des siens qui couvre celui du site.
    let collapse = match (burger.is_some(), fold) {
        (false, _) => Some("never"),
        (true, Some(w)) if w <= 479 => Some("small"),
        (true, Some(w)) if w > 767 => Some("tablet"),
        _ => None,
    };
    // Conteneur propre aux liens : ses classes passent au menu (écart, alignement).
    let menu_class = (!std::ptr::eq(menu_box, wrap) && !std::ptr::eq(menu_box, h))
        .then(|| kept_classes(menu_box, false))
        .flatten();
    let mut root = proto.clone();
    let els = element_kids(proto);
    let (Some(t), Some(m)) = (els.get(1), els.get(2)) else {
        return Found::No;
    };
    let (mut toggle, mut menu_node) = ((*t).clone(), (*m).clone());
    if let (Some((_, b, _)), Node::Element { attrs, .. }) = (burger, &mut toggle)
        && let Some(l) = el_attr(b, "aria-label").filter(|l| !l.trim().is_empty())
    {
        attrs.retain(|a| a.name != "aria-label");
        attrs.push(Attr {
            name: "aria-label".into(),
            value: Some(l.to_string()),
            when: None,
        });
    }
    // Classes du bouton d'origine gardées (taille, fond ; un script du site qui le cherche le
    // trouve encore) ; l'outil décide de son affichage.
    let toggle_class = burger.and_then(|(_, b, _)| kept_classes(b, false));
    set_class(&mut toggle, toggle_class.as_deref());
    let Some(list) = kids_mut(&mut menu_node) else {
        return Found::No;
    };
    *list = menu.iter().map(|(_, a, _)| (*a).clone()).collect();
    if menu_class.is_some() {
        set_class(&mut menu_node, menu_class.as_deref());
    } else if let (Some(g), Node::Element { attrs, .. }) = (&gap, &mut menu_node) {
        set_attr(attrs, "style", Some(&format!("gap:{g}")));
    }
    let Some(children) = kids_mut(&mut root) else {
        return Found::No;
    };
    *children = brand.into_iter().chain([toggle, menu_node]).collect();
    // Classes de l'élément remplacé (fond, largeur, alignement) ; `header` gardé autour.
    let keep_header = at.is_empty() && tag(h) == "header";
    let class = if keep_header {
        None
    } else {
        kept_classes(wrap, false)
    };
    if let Node::Element { tag: t, attrs, .. } = &mut root {
        if let Some(c) = collapse {
            set_attr(attrs, "data-revel-collapse", Some(c));
        }
        set_attr(attrs, OWN_ATTR, None);
        if let Some(id) = el_attr(wrap, "id").filter(|_| !keep_header) {
            set_attr(attrs, "id", Some(id));
        }
        // Barre dans un `nav` gardé : pas de `nav` dans un `nav`.
        if !at.is_empty() && tag(h) == "nav" {
            *t = "div".into();
        }
    }
    set_class(&mut root, class.as_deref());
    let node = if keep_header {
        let mut outer = h.clone();
        if let Some(k) = kids_mut(&mut outer) {
            *k = vec![root];
        }
        outer
    } else {
        root
    };
    // Allure du menu ouvert reprise si le repli de l'outil couvre celui du site.
    let menus = menu_class
        .iter()
        .flat_map(|c| c.split_whitespace())
        .map(|c| (c.to_string(), fold.is_some_and(|w| w <= 991)))
        .chain(
            toggle_class
                .iter()
                .flat_map(|c| c.split_whitespace())
                .map(|c| (c.to_string(), false)),
        )
        .collect();
    Found::Yes(NavParts { at, node, menus })
}

/// Classes d'état d'un menu ouvert par le script du site.
const OPEN_STATES: &[&str] = &[
    "open",
    "is-open",
    "opened",
    "active",
    "is-active",
    "show",
    "visible",
    "expanded",
    "w--open",
];

/// `.from` remplacé par `.to` dans un sélecteur (nom de classe entier).
fn rename_class(selector: &str, from: &str, to: &str) -> String {
    let needle = format!(".{from}");
    let mut out = String::with_capacity(selector.len());
    let mut rest = selector;
    while let Some(i) = rest.find(&needle) {
        let after = &rest[i + needle.len()..];
        out.push_str(&rest[..i]);
        if after
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            out.push_str(&needle);
        } else {
            out.push('.');
            out.push_str(to);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

// --- Parcours ---------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct Zone {
    header: bool,
    footer: bool,
}

struct Cx<'a> {
    protos: &'a Protos,
    css: &'a Css,
    made: [usize; 4],
    skipped: BTreeSet<&'static str>,
    /// Classes de panneaux que le site cachait lui-même (replié, onglet inactif) : l'outil s'en
    /// charge, leur état caché est retiré des styles.
    unhide: BTreeSet<String>,
    /// Classes du menu d'une barre de navigation : l'outil décide de l'affichage (`display`
    /// retiré de leurs règles) ; l'allure du menu ouvert du site (`.menu.open`) devient celle du
    /// menu (`true`), sinon ces règles sont laissées.
    menus: Vec<(String, bool)>,
    /// Classes d'état d'un onglet choisi, renommées `is-active` (classe posée par l'outil) dans
    /// les règles qui visent aussi une classe des onglets (seconde liste).
    active: Vec<(String, Vec<String>)>,
}

const KINDS: [(&str, &str); 4] = [
    ("accordéon", "accordéons"),
    ("onglets", "onglets"),
    ("carrousel", "carrousels"),
    ("barre de navigation", "barres de navigation"),
];

fn recognize(n: &Node, cx: &mut Cx, zone: Zone) -> Option<Node> {
    if !plain_element(n) || is_tool(n) {
        return None;
    }
    let t = tag(n);
    if matches!(
        t,
        "html" | "head" | "body" | "main" | "svg" | "form" | "table" | "footer"
    ) {
        return None;
    }
    // Barre de navigation : en-tête de page, ou bloc nommé, avec ses liens.
    let nav_named = named(n, &["navbar", "topbar", "masthead", "navigation"]);
    let not_nav = named(
        n,
        &[
            "breadcrumb",
            "breadcrumbs",
            "pagination",
            "pager",
            "toc",
            "sommaire",
            "social",
            "socials",
            "footer",
            "lang",
            "langue",
            "langues",
            "languages",
            "tabs",
            "tab",
        ],
    ) || el_attr(n, "aria-label").is_some_and(|l| {
        let l = l.to_lowercase();
        l.contains("breadcrumb") || l.contains("fil d") || l.contains("pagination")
    });
    if !zone.footer
        && !not_nav
        && (t == "header" || (t == "nav" && !zone.header) || (nav_named && !zone.header))
    {
        let mut d = Vec::new();
        descendants(n, 8, &mut d);
        let has_burger = d.iter().any(|x| is_burger(x));
        if t == "header" || nav_named || has_burger {
            match navbar_of(n, cx.css, &cx.protos.navbar) {
                Found::Yes(p) => {
                    cx.made[3] += 1;
                    cx.menus.extend(p.menus);
                    if p.at.is_empty() {
                        return Some(p.node);
                    }
                    let mut out = n.clone();
                    let mut cur = &mut out;
                    for (k, i) in p.at.iter().enumerate() {
                        let list = kids_mut(cur)?;
                        if k + 1 == p.at.len() {
                            list[*i] = p.node;
                            return Some(out);
                        }
                        cur = &mut list[*i];
                    }
                    return Some(out);
                }
                Found::Lossy => {
                    cx.skipped.insert("barre de navigation");
                }
                Found::No => {}
            }
        }
    }
    // Onglets.
    if let Some((tabs, list_class, hide, state)) = tabs_of(n) {
        cx.made[1] += 1;
        cx.unhide.extend(hide);
        let own: Vec<String> = tabs
            .iter()
            .flat_map(|t| t.class.iter().flat_map(|c| c.split_whitespace()))
            .map(str::to_string)
            .collect();
        cx.active
            .extend(state.into_iter().map(|st| (st, own.clone())));
        return build_tabs(
            &cx.protos.tabs,
            kept_classes(n, true).as_deref(),
            list_class.as_deref(),
            tabs,
        )
        .map(|mut t| {
            carry(n, &mut t);
            t
        });
    }
    // Accordéon : tous les enfants sont des volets.
    let els = element_kids(n);
    let only_els = kids(n)
        .iter()
        .all(|c| matches!(c, Node::Element { .. }) || blank(c));
    if only_els && els.len() >= 2 && els.iter().all(|c| tag(c) == "details") {
        let items: Option<Vec<AccItem>> = els.iter().map(|c| details_item(c)).collect();
        if let Some(items) = items {
            cx.made[0] += 1;
            return build_accordion(
                &cx.protos.accordion,
                kept_classes(n, false).as_deref(),
                items,
            )
            .map(|mut t| {
                carry(n, &mut t);
                t
            });
        }
    }
    if only_els
        && !els.is_empty()
        && let Some((items, score)) = block_items(n)
    {
        // Un seul volet : il faut un indice de plus.
        if score >= 3 && (items.len() >= 2 || score >= 4) {
            cx.made[0] += 1;
            for it in &els {
                if let Some(panel) = element_kids(it).get(1) {
                    cx.unhide
                        .extend(classes(panel).iter().map(|c| c.to_string()));
                }
            }
            return build_accordion(
                &cx.protos.accordion,
                kept_classes(n, false).as_deref(),
                items,
            )
            .map(|mut t| {
                carry(n, &mut t);
                t
            });
        }
    }
    // Carrousel.
    match slides_of(n, cx.css) {
        Found::Yes((slides, show)) => {
            cx.made[2] += 1;
            return build_carousel(
                &cx.protos.carousel,
                kept_classes(n, true).as_deref(),
                slides,
                show,
            )
            .map(|mut t| {
                carry(n, &mut t);
                t
            });
        }
        Found::Lossy => {
            cx.skipped.insert("carrousel");
        }
        Found::No => {}
    }
    None
}

fn pass(list: &mut Vec<Node>, cx: &mut Cx, zone: Zone) {
    let mut i = 0;
    while i < list.len() {
        // Volets `details` qui se suivent : un accordéon.
        if details_item(&list[i]).is_some() {
            let mut end = i;
            let mut j = i + 1;
            while j < list.len() {
                if details_item(&list[j]).is_some() {
                    end = j;
                } else if !blank(&list[j]) {
                    break;
                }
                j += 1;
            }
            let items: Vec<AccItem> = list[i..=end].iter().filter_map(details_item).collect();
            if let Some(mut node) = build_accordion(&cx.protos.accordion, None, items) {
                if let Node::Element { attrs, .. } = &mut node {
                    set_attr(attrs, OWN_ATTR, None);
                }
                list.splice(i..=end, [node]);
                cx.made[0] += 1;
            }
        } else if let Some(node) = recognize(&list[i], cx, zone) {
            list[i] = node;
        }
        match &mut list[i] {
            Node::Element {
                tag,
                attrs,
                children,
                ..
            } => {
                let z = Zone {
                    header: zone.header || tag == "header" || has_attr(attrs, WIDGET_ATTR),
                    footer: zone.footer
                        || tag == "footer"
                        || attr(attrs, "class")
                            .is_some_and(|c| super::words(c).iter().any(|w| w == "footer")),
                };
                pass(children, cx, z);
            }
            n => {
                for l in revel_model::ids::child_lists_mut(n) {
                    pass(l, cx, zone);
                }
            }
        }
        i += 1;
    }
}

pub(super) fn run(site: &mut Site, report: &mut Report) {
    let protos = Protos::new(site);
    let css = Css::new(site);
    let mut cx = Cx {
        protos: &protos,
        css: &css,
        made: [0; 4],
        skipped: BTreeSet::new(),
        unhide: BTreeSet::new(),
        menus: Vec::new(),
        active: Vec::new(),
    };
    for p in &mut site.pages {
        pass(&mut p.body, &mut cx, Zone::default());
    }
    for l in &mut site.layouts {
        pass(&mut l.body, &mut cx, Zone::default());
    }
    for c in &mut site.components {
        pass(&mut c.body, &mut cx, Zone::default());
    }
    let (made, skipped, unhide, menus, active) =
        (cx.made, cx.skipped, cx.unhide, cx.menus, cx.active);
    if let Some(sheet) = site.stylesheet.as_mut() {
        for item in &mut sheet.items {
            let StyleItem::Rule(r) = item else { continue };
            // Onglet choisi : la classe d'état du site devient celle que l'outil pose.
            for (state, tabs) in &active {
                if tabs
                    .iter()
                    .any(|t| rename_class(&r.selector, t, "") != r.selector)
                {
                    r.selector = rename_class(&r.selector, state, "is-active");
                }
            }
            // Menu : affichage décidé par l'outil ; allure du menu ouvert reprise.
            for (class, open) in &menus {
                let sel = r.selector.trim();
                let own = sel.strip_prefix('.') == Some(class.as_str());
                let opened = *open
                    && sel
                        .strip_prefix('.')
                        .and_then(|t| t.strip_prefix(class.as_str()))
                        .and_then(|t| t.strip_prefix('.'))
                        .is_some_and(|state| OPEN_STATES.contains(&state));
                if opened {
                    r.selector = format!(".{class}");
                }
                if own || opened {
                    r.decls.retain(|d| d.prop != "display");
                }
            }
        }
        // Règles vidées par ces retouches.
        let touched: Vec<&str> = menus.iter().map(|(c, _)| c.as_str()).collect();
        sheet.items.retain(|i| {
            !matches!(i, StyleItem::Rule(r) if r.decls.is_empty()
                && r.selector.trim().strip_prefix('.').is_some_and(|c| touched.contains(&c)))
        });
    }
    // État caché des panneaux repris : retiré de leurs règles de base.
    if let Some(sheet) = site.stylesheet.as_mut() {
        for item in &mut sheet.items {
            let StyleItem::Rule(r) = item else { continue };
            let Some(class) = r.selector.trim().strip_prefix('.') else {
                continue;
            };
            if r.media.is_some() || !unhide.contains(class) {
                continue;
            }
            r.decls.retain(|d| {
                let v = d.value.trim();
                !matches!(
                    (d.prop.as_str(), v),
                    ("display", "none")
                        | ("visibility", "hidden")
                        | ("opacity", "0")
                        | ("max-height" | "height", "0" | "0px")
                )
            });
        }
    }
    for (i, n) in made.iter().enumerate() {
        if *n > 0 {
            let name = if *n > 1 { KINDS[i].1 } else { KINDS[i].0 };
            report.tools.push((name.to_string(), *n));
        }
    }
    report.skipped.extend(skipped.iter().map(|s| s.to_string()));
}
