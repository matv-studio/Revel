//! Commandes composées (dupliquer, envelopper) et nouveaux nœuds pour l'éditeur.

use revel_model::ids::{child_lists_mut, fresh, node_id_mut};
use revel_model::{Attr, ImageNode, Node, Site};
use std::collections::HashSet;

use crate::EditError;
use crate::address::{self, Branch, Owner, Parent, Target};
use crate::command::Command;

/// Donne de nouveaux identifiants à tous les nœuds structurels d'un sous-arbre.
pub fn refresh_ids(n: &mut Node, taken: &mut HashSet<String>) {
    if let Some(slot) = node_id_mut(n) {
        let id = fresh(taken);
        taken.insert(id.clone());
        *slot = Some(id);
    }
    for list in child_lists_mut(n) {
        for c in list.iter_mut() {
            refresh_ids(c, taken);
        }
    }
}

/// Copie d'un nœud insérée juste après lui. Renvoie la commande et l'identifiant de la copie.
pub fn duplicate(site: &Site, at: &Target) -> Result<(Command, String), EditError> {
    let (parent, index) = address::locate(site, at)?;
    let mut copy = address::node(site, at)?.clone();
    let mut taken = site.node_ids();
    refresh_ids(&mut copy, &mut taken);
    let id = revel_model::ids::node_id(&copy)
        .unwrap_or_default()
        .to_string();
    Ok((
        Command::Insert {
            parent,
            index: index + 1,
            node: copy,
        },
        id,
    ))
}

/// Enveloppe un nœud dans un nouvel élément (`div` par défaut). Renvoie la commande et
/// l'identifiant de l'enveloppe.
pub fn wrap(site: &Site, at: &Target, tag: &str) -> Result<(Command, String), EditError> {
    let (parent, index) = address::locate(site, at)?;
    let taken = site.node_ids();
    let id = fresh(&taken);
    let wrapper = Node::Element {
        id: Some(id.clone()),
        tag: tag.to_string(),
        attrs: Vec::new(),
        children: Vec::new(),
    };
    let wrapper_target = Target {
        owner: at.owner.clone(),
        node: id.clone(),
    };
    Ok((
        Command::Batch {
            commands: vec![
                Command::Insert {
                    parent,
                    index,
                    node: wrapper,
                },
                Command::Move {
                    at: at.clone(),
                    to: Parent::of(&wrapper_target, Branch::Children),
                    index: 0,
                },
            ],
        },
        id,
    ))
}

/// Éléments proposés par le panneau « Ajouter ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    Section,
    Container,
    Heading,
    Paragraph,
    Link,
    Button,
    Image,
    List,
    /// Grille de deux colonnes (cellules).
    Grid,
    /// Colonnes côte à côte, qui passent l'une sous l'autre quand la place manque.
    Columns,
    /// Texte riche : titre, paragraphe, liste.
    RichText,
    /// Vidéo (fichier du site).
    Video,
    /// Icône (dessin au trait, couleur du texte).
    Icon,
    /// Code HTML écrit tel quel.
    Embed,
    /// Animation Lottie (6.2) : fichier `.json` des médias, lu par le lecteur du site.
    Lottie,
    /// Onglets (6.3) : un bouton par panneau, le script du site affiche le panneau choisi.
    Tabs,
    /// Accordéon (6.3) : titre qui ouvre son contenu (`details`, sans script).
    Accordion,
    /// Liste de collection : un élément par contenu.
    CollectionList,
    /// Sélecteur de langue : un lien par langue du site vers la même page (3.8).
    LocaleSwitch,
    /// Blocs prêts à l'emploi (sections complètes, [`crate::ready`]).
    BlockHero,
    BlockFeatures,
    BlockTextImage,
    BlockCta,
    BlockTestimonial,
    BlockFaq,
    BlockContact,
    /// Formulaire de contact reçu par l'hébergement (5.8).
    BlockForm,
}

impl Preset {
    /// Bloc prêt à l'emploi correspondant.
    pub fn block(self) -> Option<crate::ready::Block> {
        use crate::ready::Block;
        Some(match self {
            Preset::BlockHero => Block::Hero,
            Preset::BlockFeatures => Block::Features,
            Preset::BlockTextImage => Block::TextImage,
            Preset::BlockCta => Block::CallToAction,
            Preset::BlockTestimonial => Block::Testimonial,
            Preset::BlockFaq => Block::Faq,
            Preset::BlockContact => Block::Contact,
            Preset::BlockForm => Block::Form,
            _ => return None,
        })
    }

    pub const ALL: [Preset; 27] = [
        Preset::Section,
        Preset::Container,
        Preset::Heading,
        Preset::Paragraph,
        Preset::Link,
        Preset::Button,
        Preset::Image,
        Preset::List,
        Preset::Grid,
        Preset::Columns,
        Preset::RichText,
        Preset::Video,
        Preset::Icon,
        Preset::Embed,
        Preset::Lottie,
        Preset::Tabs,
        Preset::Accordion,
        Preset::CollectionList,
        Preset::LocaleSwitch,
        Preset::BlockHero,
        Preset::BlockFeatures,
        Preset::BlockTextImage,
        Preset::BlockCta,
        Preset::BlockTestimonial,
        Preset::BlockFaq,
        Preset::BlockContact,
        Preset::BlockForm,
    ];

    /// Règles de style dont l'élément a besoin (ajoutées si le site ne les a pas).
    pub fn rules(self) -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
        if let Some(b) = self.block() {
            return b.rules();
        }
        match self {
            Preset::Grid => vec![
                (
                    ".grille",
                    vec![
                        ("display", "grid"),
                        ("grid-template-columns", "repeat(2, minmax(0, 1fr))"),
                        ("gap", "24px"),
                    ],
                ),
                (".grille__cellule", vec![("min-height", "80px")]),
            ],
            Preset::Columns => vec![
                (
                    ".colonnes",
                    vec![("display", "flex"), ("flex-wrap", "wrap"), ("gap", "24px")],
                ),
                (
                    ".colonne",
                    vec![
                        ("flex", "1 1 240px"),
                        ("min-width", "0"),
                        ("min-height", "80px"),
                    ],
                ),
            ],
            Preset::Video => vec![(
                ".video",
                vec![("display", "block"), ("width", "100%"), ("height", "auto")],
            )],
            Preset::Tabs => vec![
                (
                    ".onglets__liste",
                    vec![
                        ("display", "flex"),
                        ("flex-wrap", "wrap"),
                        ("gap", "4px"),
                        ("border-bottom", "1px solid rgba(127, 127, 127, 0.35)"),
                    ],
                ),
                (
                    ".onglet",
                    vec![
                        ("padding", "10px 16px"),
                        ("border", "0"),
                        ("border-bottom", "2px solid transparent"),
                        ("margin-bottom", "-1px"),
                        ("background", "none"),
                        ("color", "inherit"),
                        ("font", "inherit"),
                        ("cursor", "pointer"),
                    ],
                ),
                (
                    ".onglet.is-active",
                    vec![("border-bottom-color", "currentColor")],
                ),
                (".onglets__panneau", vec![("padding", "16px 0")]),
            ],
            Preset::Accordion => vec![
                (
                    ".accordeon",
                    vec![("border-bottom", "1px solid rgba(127, 127, 127, 0.35)")],
                ),
                (
                    ".accordeon__titre",
                    vec![
                        ("padding", "14px 0"),
                        ("font-weight", "600"),
                        ("cursor", "pointer"),
                    ],
                ),
                (".accordeon__contenu", vec![("padding-bottom", "14px")]),
            ],
            Preset::Lottie => vec![(
                ".lottie",
                vec![
                    ("display", "block"),
                    ("width", "100%"),
                    ("max-width", "480px"),
                ],
            )],
            Preset::Icon => vec![(
                ".icone",
                vec![
                    ("display", "inline-block"),
                    ("width", "24px"),
                    ("height", "24px"),
                    ("vertical-align", "middle"),
                ],
            )],
            Preset::LocaleSwitch => vec![
                (
                    ".langues",
                    vec![
                        ("display", "flex"),
                        ("align-items", "center"),
                        ("gap", "12px"),
                    ],
                ),
                (
                    ".langue",
                    vec![
                        ("color", "inherit"),
                        ("text-decoration", "none"),
                        ("opacity", "0.6"),
                    ],
                ),
                (
                    ".langue[aria-current=\"page\"]",
                    vec![("opacity", "1"), ("font-weight", "600")],
                ),
            ],
            Preset::CollectionList => vec![(
                ".liste-collection",
                vec![
                    ("display", "grid"),
                    (
                        "grid-template-columns",
                        "repeat(auto-fill, minmax(240px, 1fr))",
                    ),
                    ("gap", "24px"),
                ],
            )],
            _ => Vec::new(),
        }
    }

    /// Nouveau nœud (identifiants libres dans `site`). Les textes par défaut sont fournis par
    /// l'appelant (traduits dans la langue de l'interface).
    pub fn node(self, site: &Site, text: &str) -> Node {
        if let Some(b) = self.block() {
            return b.node(site);
        }
        let mut taken = site.node_ids();
        let mut id = || {
            let i = fresh(&taken);
            taken.insert(i.clone());
            Some(i)
        };
        let el =
            |id: Option<String>, tag: &str, attrs: Vec<Attr>, children: Vec<Node>| Node::Element {
                id,
                tag: tag.into(),
                attrs,
                children,
            };
        let attr = |n: &str, v: &str| Attr {
            name: n.into(),
            value: Some(v.into()),
            when: None,
        };
        let txt = |t: &str| Node::Text { text: t.into() };
        match self {
            // Blocs : traités plus haut.
            Preset::BlockHero
            | Preset::BlockFeatures
            | Preset::BlockTextImage
            | Preset::BlockCta
            | Preset::BlockTestimonial
            | Preset::BlockFaq
            | Preset::BlockContact
            | Preset::BlockForm => el(id(), "section", vec![], vec![]),
            Preset::Section => el(id(), "section", vec![], vec![]),
            Preset::Container => el(id(), "div", vec![], vec![]),
            Preset::Heading => el(id(), "h2", vec![], vec![txt(text)]),
            Preset::Paragraph => el(id(), "p", vec![], vec![txt(text)]),
            Preset::Link => el(id(), "a", vec![attr("href", "#")], vec![txt(text)]),
            Preset::Button => el(
                id(),
                "a",
                vec![attr("href", "#"), attr("class", "button")],
                vec![txt(text)],
            ),
            // Texte alternatif vide (image décorative) tant qu'il n'est pas écrit : celui du
            // média choisi le remplace (`text` : ignoré, un mot générique n'aide personne).
            Preset::Image => Node::Image(ImageNode {
                id: id(),
                src: "\"\"".into(),
                alt: Some("\"\"".into()),
                ..Default::default()
            }),
            Preset::List => {
                let ul = id();
                let li1 = id();
                let li2 = id();
                el(
                    ul,
                    "ul",
                    vec![],
                    vec![
                        el(li1, "li", vec![], vec![txt(text)]),
                        el(li2, "li", vec![], vec![txt(text)]),
                    ],
                )
            }
            Preset::Grid => {
                let g = id();
                let cells: Vec<Node> = (0..2)
                    .map(|_| el(id(), "div", vec![attr("class", "grille__cellule")], vec![]))
                    .collect();
                el(g, "div", vec![attr("class", "grille")], cells)
            }
            Preset::Columns => {
                let g = id();
                let cols: Vec<Node> = (0..2)
                    .map(|_| el(id(), "div", vec![attr("class", "colonne")], vec![]))
                    .collect();
                el(g, "div", vec![attr("class", "colonnes")], cols)
            }
            Preset::RichText => {
                // `text` : titre, puis paragraphe (une ligne chacun).
                let (title, para) = text.split_once('\n').unwrap_or((text, text));
                let w = id();
                let h = id();
                let p = id();
                let ul = id();
                let li1 = id();
                let li2 = id();
                el(
                    w,
                    "div",
                    vec![attr("class", "texte-riche")],
                    vec![
                        el(h, "h2", vec![], vec![txt(title)]),
                        el(p, "p", vec![], vec![txt(para)]),
                        el(
                            ul,
                            "ul",
                            vec![],
                            vec![
                                el(li1, "li", vec![], vec![txt(para)]),
                                el(li2, "li", vec![], vec![txt(para)]),
                            ],
                        ),
                    ],
                )
            }
            Preset::Video => el(
                id(),
                "video",
                vec![
                    attr("class", "video"),
                    attr("src", ""),
                    Attr {
                        name: "controls".into(),
                        value: None,
                        when: None,
                    },
                    Attr {
                        name: "playsinline".into(),
                        value: None,
                        when: None,
                    },
                    attr("preload", "metadata"),
                ],
                vec![],
            ),
            Preset::Icon => icon_node(id(), "etoile"),
            Preset::Tabs => {
                let flag = |n: &str| Attr {
                    name: n.into(),
                    value: None,
                    when: None,
                };
                let buttons: Vec<Node> = (1..=2)
                    .map(|n| {
                        el(
                            id(),
                            "button",
                            vec![
                                attr("class", "onglet"),
                                attr("type", "button"),
                                flag("data-revel-tab"),
                            ],
                            vec![txt(&format!("{text} {n}"))],
                        )
                    })
                    .collect();
                let mut kids = vec![el(
                    id(),
                    "div",
                    vec![attr("class", "onglets__liste"), attr("role", "tablist")],
                    buttons,
                )];
                for n in 1..=2 {
                    kids.push(el(
                        id(),
                        "div",
                        vec![attr("class", "onglets__panneau"), flag("data-revel-panel")],
                        vec![el(id(), "p", vec![], vec![txt(&format!("{text} {n}"))])],
                    ));
                }
                el(
                    id(),
                    "div",
                    vec![attr("class", "onglets"), flag("data-revel-tabs")],
                    kids,
                )
            }
            Preset::Accordion => el(
                id(),
                "details",
                vec![attr("class", "accordeon")],
                vec![
                    el(
                        id(),
                        "summary",
                        vec![attr("class", "accordeon__titre")],
                        vec![txt(text)],
                    ),
                    el(
                        id(),
                        "div",
                        vec![attr("class", "accordeon__contenu")],
                        vec![el(id(), "p", vec![], vec![txt(text)])],
                    ),
                ],
            ),
            Preset::Lottie => el(
                id(),
                "div",
                vec![
                    attr("class", "lottie"),
                    attr("data-lottie", ""),
                    attr("data-lottie-play", "auto"),
                    Attr {
                        name: "data-lottie-loop".into(),
                        value: None,
                        when: None,
                    },
                ],
                vec![],
            ),
            Preset::Embed => el(
                id(),
                "div",
                vec![attr("class", "code-html")],
                vec![Node::Html {
                    html: format!("<p>{}</p>", html_escape(text)),
                }],
            ),
            Preset::LocaleSwitch => {
                // `text` : nom du menu pour les lecteurs d'écran (« Langues »).
                let nav = id();
                let each = id();
                let a = id();
                el(
                    nav,
                    "nav",
                    vec![attr("class", "langues"), attr("aria-label", text)],
                    vec![Node::Each {
                        id: each,
                        expr: "locales".into(),
                        item: "l".into(),
                        children: vec![el(
                            a,
                            "a",
                            vec![
                                attr("class", "langue"),
                                attr("href", "{{ l.url }}"),
                                attr("hreflang", "{{ l.code }}"),
                                attr("lang", "{{ l.code }}"),
                                Attr {
                                    name: "aria-current".into(),
                                    value: Some("page".into()),
                                    when: Some("l.current".into()),
                                },
                            ],
                            vec![Node::Out {
                                expr: "l.name".into(),
                            }],
                        )],
                        empty: vec![],
                    }],
                )
            }
            Preset::CollectionList => {
                let col = site
                    .collections
                    .first()
                    .map(|c| c.id.clone())
                    .unwrap_or_else(|| "all".into());
                let q = crate::collection::Query {
                    collection: col,
                    ..Default::default()
                };
                let w = id();
                let each = id();
                let card = id();
                let h = id();
                let a = id();
                el(
                    w,
                    "div",
                    vec![attr("class", "liste-collection")],
                    vec![Node::Each {
                        id: each,
                        expr: q.to_expr(),
                        item: "item".into(),
                        children: vec![el(
                            card,
                            "article",
                            vec![attr("class", "liste-collection__element")],
                            vec![
                                el(
                                    h,
                                    "h3",
                                    vec![],
                                    vec![Node::Out {
                                        expr: "item.data.title".into(),
                                    }],
                                ),
                                el(
                                    a,
                                    "a",
                                    vec![attr("href", "{{ item.url }}")],
                                    vec![txt(text)],
                                ),
                            ],
                        )],
                        empty: vec![],
                    }],
                )
            }
        }
    }
}

/// Icône `name` en `<svg>` (dessin au trait, couleur du texte).
pub fn icon_node(id: Option<String>, name: &str) -> Node {
    let a = |n: &str, v: &str| Attr {
        name: n.into(),
        value: Some(v.into()),
        when: None,
    };
    Node::Element {
        id,
        tag: "svg".into(),
        attrs: vec![
            a("class", "icone"),
            a("data-icon", name),
            a("viewBox", "0 0 24 24"),
            a("width", "24"),
            a("height", "24"),
            a("fill", "none"),
            a("stroke", "currentColor"),
            a("stroke-width", "2"),
            a("stroke-linecap", "round"),
            a("stroke-linejoin", "round"),
            a("aria-hidden", "true"),
        ],
        children: vec![Node::Html {
            html: crate::icons::icon(name).unwrap_or_default().to_string(),
        }],
    }
}

fn html_escape(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Identifiant libre pour le design propre d'un contenu : `<gabarit>--<fichier>` (suffixe
/// `-2`, `-3`… si pris).
pub fn own_layout_id(site: &Site, template: &str, file: &str) -> String {
    let base = format!("{template}--{file}");
    let mut id = base.clone();
    let mut n = 2;
    while site.layouts.iter().any(|l| l.id == id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Modèle des pages d'une collection (page de collection de Webflow) : gabarit enfant du
/// gabarit habituel des pages, avec le titre du contenu (`title_field`) et son texte.
/// Identifiant libre `modele-<collection>`.
pub fn collection_template(
    site: &Site,
    collection: &str,
    title_field: &str,
) -> revel_model::Layout {
    let base = format!("modele-{}", collection.replace('_', "-"));
    let mut id = base.clone();
    let mut n = 2;
    while site.layouts.iter().any(|l| l.id == id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    let mut taken = site.node_ids();
    let mut fresh_id = || {
        let i = fresh(&taken);
        taken.insert(i.clone());
        Some(i)
    };
    let class = |v: &str| Attr {
        name: "class".into(),
        value: Some(v.into()),
        when: None,
    };
    let (s, h, c) = (fresh_id(), fresh_id(), fresh_id());
    let body = vec![Node::Element {
        id: s,
        tag: "section".into(),
        attrs: vec![class("section")],
        children: vec![
            Node::Element {
                id: h,
                tag: "h1".into(),
                attrs: vec![],
                children: vec![Node::Out {
                    expr: title_field.into(),
                }],
            },
            Node::Element {
                id: c,
                tag: "div".into(),
                attrs: vec![class("contenu")],
                children: vec![Node::Slot],
            },
        ],
    }];
    revel_model::Layout {
        id,
        parent: usual_layout(site).or_else(|| site.layouts.first().map(|l| l.id.clone())),
        props: Default::default(),
        body,
    }
}

/// Copie du gabarit `from` sous l'identifiant `id` (même parent, mêmes variables), avec de
/// nouveaux identifiants de nœuds.
pub fn layout_copy(site: &Site, from: &str, id: &str) -> Result<revel_model::Layout, EditError> {
    let src = site
        .layout(from)
        .ok_or_else(|| EditError::NotFound(from.to_string()))?;
    let mut taken = site.node_ids();
    let mut body = src.body.clone();
    for n in body.iter_mut() {
        refresh_ids(n, &mut taken);
    }
    Ok(revel_model::Layout {
        id: id.to_string(),
        parent: src.parent.clone(),
        props: src.props.clone(),
        body,
    })
}

/// Adresse libre pour une page : `/<dossier>/<fragment>/` (`-2`, `-3`… si prise).
pub fn free_page_path(site: &Site, folder: &str, slug: &str) -> String {
    let slug = if slug.is_empty() { "page" } else { slug };
    let prefix = match folder.trim_matches('/') {
        "" => "/".to_string(),
        f => format!("/{f}/"),
    };
    let mut path = format!("{prefix}{slug}/");
    let mut n = 2;
    while site.pages.iter().any(|p| p.path == path) {
        path = format!("{prefix}{slug}-{n}/");
        n += 1;
    }
    path
}

fn free_page_id(site: &Site, base: &str) -> String {
    let base = if base.is_empty() { "page" } else { base };
    let mut id = base.to_string();
    let mut n = 2;
    while site.pages.iter().any(|p| p.id == id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Gabarit des pages fixes le plus utilisé (celui d'une nouvelle page).
pub fn usual_layout(site: &Site) -> Option<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for l in site
        .pages
        .iter()
        .filter(|p| p.path.ends_with('/') || p.path.ends_with(".html"))
        .filter_map(|p| p.layout.clone())
    {
        match counts.iter_mut().find(|(x, _)| *x == l) {
            Some(c) => c.1 += 1,
            None => counts.push((l, 1)),
        }
    }
    counts.into_iter().max_by_key(|c| c.1).map(|c| c.0)
}

/// Gabarit de base pour un site qui n'en a pas : en-tête du document (langue, titre, feuille de
/// style) et emplacement de la page. Les pages créées ensuite n'ont que leur contenu.
pub fn base_layout(site: &Site) -> revel_model::Layout {
    let el = |tag: &str, attrs: Vec<Attr>, children: Vec<Node>| Node::Element {
        id: None,
        tag: tag.into(),
        attrs,
        children,
    };
    let at = |name: &str, value: &str| Attr {
        name: name.into(),
        value: Some(value.into()),
        when: None,
    };
    let sheet = site
        .stylesheet
        .as_ref()
        .map(|s| s.url.clone())
        .unwrap_or_else(|| crate::style::DEFAULT_URL.into());
    let name = site.name.replace('\\', "").replace('"', "'");
    let mut body = vec![
        Node::Html {
            html: "<!doctype html>".into(),
        },
        el(
            "html",
            vec![at("lang", &site.default_locale)],
            vec![
                el(
                    "head",
                    vec![],
                    vec![
                        el("meta", vec![at("charset", "utf-8")], vec![]),
                        el(
                            "meta",
                            vec![
                                at("name", "viewport"),
                                at("content", "width=device-width, initial-scale=1"),
                            ],
                            vec![],
                        ),
                        el(
                            "title",
                            vec![],
                            vec![Node::Out {
                                expr: format!("seo.title if seo and seo.title else \"{name}\""),
                            }],
                        ),
                        el(
                            "link",
                            vec![at("rel", "stylesheet"), at("href", &sheet)],
                            vec![],
                        ),
                    ],
                ),
                el("body", vec![], vec![Node::Slot]),
            ],
        ),
    ];
    let mut taken = site.node_ids();
    for n in body.iter_mut() {
        refresh_ids(n, &mut taken);
    }
    let mut id = "base".to_string();
    let mut k = 2;
    while site.layouts.iter().any(|l| l.id == id) {
        id = format!("base-{k}");
        k += 1;
    }
    revel_model::Layout {
        id,
        parent: None,
        props: Default::default(),
        body,
    }
}

/// Nouvelle page `name` dans le dossier `folder` (chemin d'adresse, `""` : racine) : gabarit le
/// plus utilisé (sinon le premier), titre de référencement, un titre dans une section.
pub fn new_page(site: &Site, name: &str, folder: &str) -> revel_model::Page {
    let slug = revel_model::slugify(name);
    let path = free_page_path(site, folder, &slug);
    let id = free_page_id(site, &path.trim_matches('/').replace('/', "-"));
    let layout = usual_layout(site).or_else(|| site.layouts.first().map(|l| l.id.clone()));
    let mut taken = site.node_ids();
    let heading = Node::Element {
        id: None,
        tag: "h1".into(),
        attrs: vec![],
        children: vec![Node::Text {
            text: name.to_string(),
        }],
    };
    let mut body = match &layout {
        Some(_) => vec![Node::Element {
            id: None,
            tag: "section".into(),
            attrs: vec![Attr {
                name: "class".into(),
                value: Some("section".into()),
                when: None,
            }],
            children: vec![heading],
        }],
        // Sans gabarit : document complet.
        None => vec![
            Node::Html {
                html: "<!doctype html>".into(),
            },
            Node::Element {
                id: None,
                tag: "html".into(),
                attrs: vec![],
                children: vec![
                    Node::Element {
                        id: None,
                        tag: "head".into(),
                        attrs: vec![],
                        children: vec![
                            Node::Element {
                                id: None,
                                tag: "meta".into(),
                                attrs: vec![Attr {
                                    name: "charset".into(),
                                    value: Some("utf-8".into()),
                                    when: None,
                                }],
                                children: vec![],
                            },
                            Node::Element {
                                id: None,
                                tag: "title".into(),
                                attrs: vec![],
                                children: vec![Node::Text {
                                    text: name.to_string(),
                                }],
                            },
                        ],
                    },
                    Node::Element {
                        id: None,
                        tag: "body".into(),
                        attrs: vec![],
                        children: vec![heading],
                    },
                ],
            },
        ],
    };
    for n in body.iter_mut() {
        refresh_ids(n, &mut taken);
    }
    let mut props = std::collections::BTreeMap::new();
    if layout.is_some() {
        props.insert("seo".into(), serde_json::json!({ "title": name }));
    }
    revel_model::Page {
        id,
        path,
        layout,
        props,
        computed: Default::default(),
        body,
        draft: false,
        code: Default::default(),
        name: Some(name.trim().to_string()).filter(|n| !n.is_empty()),
    }
}

/// Copie d'une page (nouveaux identifiants, adresse `…-copie/`, nom « … (copie) »).
pub fn duplicate_page(
    site: &Site,
    id: &str,
    copy_word: &str,
) -> Result<revel_model::Page, EditError> {
    let src = site
        .pages
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| EditError::NotFound(format!("page {id}")))?;
    let mut p = src.clone();
    let mut taken = site.node_ids();
    for n in p.body.iter_mut() {
        refresh_ids(n, &mut taken);
    }
    let trimmed = src.path.trim_end_matches('/');
    let (folder, last) = trimmed.rsplit_once('/').unwrap_or(("", trimmed));
    let last = if last.is_empty() { "accueil" } else { last };
    p.path = free_page_path(
        site,
        folder,
        &format!("{last}-{}", revel_model::slugify(copy_word)),
    );
    p.id = free_page_id(
        site,
        &format!("{}-{}", src.id, revel_model::slugify(copy_word)),
    );
    p.name = Some(format!(
        "{} ({copy_word})",
        src.name.clone().unwrap_or_else(|| last.to_string())
    ));
    Ok(p)
}

// ------------------------------------------------------------------ composants

/// Nombre d'utilisations du composant `id` dans le site (pages, gabarits, composants).
pub fn uses_of(site: &Site, id: &str) -> usize {
    let mut n = 0;
    for (_, _, body) in site.owners() {
        revel_model::ids::walk(body, &mut |x| {
            if matches!(x, Node::Use { component, .. } if component == id) {
                n += 1;
            }
        });
    }
    n
}

/// Identifiant libre pour un nouveau composant nommé `name` (`carte-projet`, `-2`, `-3`…).
pub fn free_component_id(site: &Site, name: &str) -> String {
    let base = match revel_model::slugify(name) {
        s if s.is_empty() => "composant".to_string(),
        s => s,
    };
    let mut id = base.clone();
    let mut n = 2;
    while site.components.iter().any(|c| c.id == id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Composants utilisés (directement ou non) par le composant `id`, lui compris.
fn reach(site: &Site, id: &str) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut todo = vec![id.to_string()];
    while let Some(c) = todo.pop() {
        if !seen.insert(c.clone()) {
            continue;
        }
        if let Some(comp) = site.component(&c) {
            revel_model::ids::walk(&comp.body, &mut |x| {
                if let Node::Use { component, .. } = x {
                    todo.push(component.clone());
                }
            });
        }
    }
    seen
}

/// Le composant `component` peut être placé dans l'arbre de `owner` (pas dans lui-même, ni
/// dans un composant qu'il utilise).
pub fn can_use_in(site: &Site, owner: &Owner, component: &str) -> bool {
    match owner {
        Owner::Component(c) => !reach(site, component).contains(c),
        _ => true,
    }
}

/// Crée le composant `id` à partir du nœud `at` : le nœud devient son contenu (mêmes
/// identifiants), une utilisation prend sa place. Renvoie la commande et l'identifiant de
/// l'utilisation.
pub fn make_component(
    site: &Site,
    at: &Target,
    id: &str,
    name: Option<String>,
) -> Result<(Command, String), EditError> {
    if id.trim().is_empty() || site.components.iter().any(|c| c.id == id) {
        return Err(EditError::DuplicateId(id.to_string()));
    }
    let (parent, index) = address::locate(site, at)?;
    let node = address::node(site, at)?.clone();
    // L'emplacement du contenu d'un gabarit ne peut pas entrer dans un composant.
    let mut has_slot = false;
    revel_model::ids::walk(std::slice::from_ref(&node), &mut |x| {
        has_slot |= matches!(x, Node::Slot);
    });
    if has_slot {
        return Err(EditError::WrongKind(at.node.clone()));
    }
    let use_id = fresh(&site.node_ids());
    let component = revel_model::Component {
        id: id.to_string(),
        name: name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()),
        props: Vec::new(),
        variants: Vec::new(),
        body: vec![node],
    };
    Ok((
        Command::Batch {
            commands: vec![
                Command::Remove { at: at.clone() },
                Command::AddComponent {
                    component,
                    index: None,
                },
                Command::Insert {
                    parent,
                    index,
                    node: Node::Use {
                        id: Some(use_id.clone()),
                        component: id.to_string(),
                        args: Default::default(),
                        variant: None,
                        attrs: Vec::new(),
                    },
                },
            ],
        },
        use_id,
    ))
}

/// Le texte `expr` mentionne la variable `name` (mot entier, pas un attribut `x.name`).
fn mentions(expr: &str, name: &str) -> bool {
    let b = expr.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    expr.match_indices(name).any(|(i, _)| {
        let before = i.checked_sub(1).map(|j| b[j]);
        let after = b.get(i + name.len()).copied();
        !before.is_some_and(|c| word(c) || c == b'.') && !after.is_some_and(word)
    })
}

/// `{{ name }}` seul (espaces de bord permis).
fn only_var(text: &str, name: &str) -> bool {
    text.trim()
        .strip_prefix("{{")
        .and_then(|t| t.strip_suffix("}}"))
        .is_some_and(|t| t.trim() == name)
}

/// Valeur d'une propriété pour une utilisation : son expression, sinon la valeur par défaut
/// écrite en expression.
fn prop_expr(
    p: &revel_model::PropDef,
    args: &std::collections::BTreeMap<String, String>,
) -> String {
    if let Some(e) = args.get(&p.name) {
        return e.clone();
    }
    match &p.default {
        Some(serde_json::Value::String(s)) => crate::canvas::quote(s),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(v @ serde_json::Value::Number(_)) => v.to_string(),
        Some(v) => v.to_string(),
        None if p.kind == revel_model::PropKind::Switch => "false".into(),
        None => "\"\"".into(),
    }
}

/// Remplace dans `nodes` les usages directs de la propriété `name` par sa valeur `expr`
/// (texte écrit si c'est un texte littéral) ; renvoie `true` s'il reste des usages indirects.
fn substitute(nodes: &mut Vec<Node>, name: &str, expr: &str) -> bool {
    let lit = crate::canvas::literal(expr);
    let mut rest = false;
    let mut i = 0;
    while i < nodes.len() {
        // Condition sur la propriété avec une valeur connue : branche gardée à la place.
        if let Node::If {
            cond,
            then,
            otherwise,
            ..
        } = &nodes[i]
            && cond.trim() == name
            && matches!(expr.trim(), "true" | "false")
        {
            let keep = if expr.trim() == "true" {
                then.clone()
            } else {
                otherwise.clone()
            };
            // Les nœuds gardés sont examinés à leur tour.
            nodes.splice(i..=i, keep);
            continue;
        }
        let n = &mut nodes[i];
        match n {
            Node::Out { expr: e } if e.trim() == name => {
                *n = match &lit {
                    Some(t) => Node::Text { text: t.clone() },
                    None => Node::Out {
                        expr: expr.to_string(),
                    },
                };
            }
            Node::Out { expr: e } | Node::Set { expr: e, .. } => rest |= mentions(e, name),
            Node::If { cond, .. } => {
                if cond.trim() == name {
                    *cond = expr.to_string();
                } else {
                    rest |= mentions(cond, name);
                }
            }
            Node::Each { expr: e, .. } => {
                if e.trim() == name {
                    *e = expr.to_string();
                } else {
                    rest |= mentions(e, name);
                }
            }
            Node::Element { attrs, .. } => {
                for a in attrs.iter_mut() {
                    if let Some(v) = a.value.as_mut() {
                        if only_var(v, name) {
                            *v = match &lit {
                                Some(t) => t.clone(),
                                None => format!("{{{{ {expr} }}}}"),
                            };
                        } else {
                            rest |= v.contains("{{") && mentions(v, name);
                        }
                    }
                    if let Some(w) = a.when.as_mut() {
                        if w.trim() == name {
                            *w = expr.to_string();
                        } else {
                            rest |= mentions(w, name);
                        }
                    }
                }
            }
            Node::Image(img) => {
                for f in [
                    Some(&mut img.src),
                    img.alt.as_mut(),
                    img.sizes.as_mut(),
                    img.class.as_mut(),
                    img.ratio.as_mut(),
                    img.loading.as_mut(),
                    img.widths.as_mut(),
                ]
                .into_iter()
                .flatten()
                {
                    if f.trim() == name {
                        *f = expr.to_string();
                    } else {
                        rest |= mentions(f, name);
                    }
                }
            }
            Node::Use { args, .. } => {
                for v in args.values_mut() {
                    if v.trim() == name {
                        *v = expr.to_string();
                    } else {
                        rest |= mentions(v, name);
                    }
                }
            }
            _ => {}
        }
        for list in child_lists_mut(&mut nodes[i]) {
            rest |= substitute(list, name, expr);
        }
        i += 1;
    }
    rest
}

/// Utilisation de composant remplacée par une copie de son contenu (nouveaux identifiants),
/// les valeurs des propriétés écrites dedans (une affectation au début si une propriété est
/// lue dans une expression plus complexe).
pub fn detach(site: &Site, at: &Target) -> Result<Command, EditError> {
    let (parent, index) = address::locate(site, at)?;
    let Node::Use {
        component,
        args,
        variant,
        attrs,
        ..
    } = address::node(site, at)?
    else {
        return Err(EditError::WrongKind(at.node.clone()));
    };
    let c = site
        .component(component)
        .ok_or_else(|| EditError::NotFound(component.clone()))?;
    // Variante et attributs de l'utilisation gardés sur l'élément racine.
    let mut body = c
        .patched_body(variant.as_deref(), attrs)
        .unwrap_or_else(|| c.body.clone());
    let mut sets = Vec::new();
    for p in &c.props {
        let e = prop_expr(p, args);
        if substitute(&mut body, &p.name, &e) {
            sets.push(Node::Set {
                name: p.name.clone(),
                expr: e,
            });
        }
    }
    sets.extend(body);
    let mut taken = site.node_ids();
    let mut commands = vec![Command::Remove { at: at.clone() }];
    for (i, mut n) in sets.into_iter().enumerate() {
        refresh_ids(&mut n, &mut taken);
        commands.push(Command::Insert {
            parent: parent.clone(),
            index: index + i,
            node: n,
        });
    }
    Ok(Command::Batch { commands })
}

/// Propriété `name` du composant `id` retirée : ses usages reçoivent sa valeur par défaut (texte
/// écrit, condition résolue), les utilisations perdent leur valeur.
pub fn remove_prop(site: &Site, id: &str, name: &str) -> Result<Command, EditError> {
    let c = site
        .component(id)
        .ok_or_else(|| EditError::NotFound(id.to_string()))?;
    let p = c
        .prop(name)
        .ok_or_else(|| EditError::NotFound(format!("propriété {name}")))?;
    let mut body = c.body.clone();
    substitute(&mut body, name, &prop_expr(p, &Default::default()));
    let mut commands = vec![
        Command::SetComponentBody {
            id: id.to_string(),
            body,
        },
        Command::SetComponentProps {
            id: id.to_string(),
            props: c.props.iter().filter(|p| p.name != name).cloned().collect(),
        },
    ];
    for (kind, owner, nodes) in site.owners() {
        revel_model::ids::walk(nodes, &mut |n| {
            if let Node::Use {
                id: Some(uid),
                component,
                args,
                ..
            } = n
                && component == id
                && args.contains_key(name)
            {
                let owner = match kind {
                    revel_model::ids::OwnerKind::Page => Owner::Page(owner.to_string()),
                    revel_model::ids::OwnerKind::Layout => Owner::Layout(owner.to_string()),
                    revel_model::ids::OwnerKind::Component => Owner::Component(owner.to_string()),
                };
                commands.push(Command::SetUseArg {
                    at: Target {
                        owner,
                        node: uid.clone(),
                    },
                    name: name.to_string(),
                    value: None,
                });
            }
        });
    }
    Ok(Command::Batch { commands })
}

/// Nom libre pour une nouvelle propriété (`texte`, `texte_2`…).
pub fn free_prop_name(c: &revel_model::Component, base: &str) -> String {
    let mut name = base.to_string();
    let mut n = 2;
    while c.prop(&name).is_some() {
        name = format!("{base}_{n}");
        n += 1;
    }
    name
}
