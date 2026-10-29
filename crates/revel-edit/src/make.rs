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

/// Première image écrite d'un élément (lui-même ou un descendant) : vignette et remplacement
/// des éléments d'un défilement ou d'un carrousel (D-123).
pub fn first_image(n: &Node) -> Option<&revel_model::ImageNode> {
    if let Node::Image(img) = n {
        return Some(img);
    }
    revel_model::ids::child_lists(n)
        .into_iter()
        .flat_map(|l| l.iter())
        .find_map(first_image)
}

fn first_image_mut(n: &mut Node) -> Option<&mut revel_model::ImageNode> {
    if let Node::Image(img) = n {
        return Some(img);
    }
    for l in revel_model::ids::child_lists_mut(n) {
        for c in l.iter_mut() {
            if let Some(i) = first_image_mut(c) {
                return Some(i);
            }
        }
    }
    None
}

/// Ajoute un élément à une suite d'éléments semblables (défilement, carrousel) avec l'image
/// `src` : copie du dernier élément (nouveaux identifiants), sa première image remplacée ; sans
/// élément ou sans image à copier, une image seule. Renvoie la commande et l'identifiant ajouté.
pub fn add_image_item(
    site: &Site,
    list: &Target,
    src: &str,
    alt: &str,
) -> Result<(Command, String), EditError> {
    let host = address::node(site, list)?;
    let kids = revel_model::ids::child_lists(host)
        .into_iter()
        .next()
        .ok_or(EditError::NoBranch)?;
    let mut taken = site.node_ids();
    let template = kids
        .iter()
        .rev()
        .find(|n| matches!(n, Node::Element { .. } | Node::Image(_)) && first_image(n).is_some())
        .cloned();
    let node = match template {
        Some(mut copy) => {
            refresh_ids(&mut copy, &mut taken);
            if let Some(img) = first_image_mut(&mut copy) {
                img.src = crate::canvas::quote(src);
                img.alt = Some(crate::canvas::quote(alt));
            }
            copy
        }
        None => Node::Image(revel_model::ImageNode {
            id: Some(fresh(&taken)),
            src: crate::canvas::quote(src),
            alt: Some(crate::canvas::quote(alt)),
            ..Default::default()
        }),
    };
    let id = revel_model::ids::node_id(&node)
        .unwrap_or_default()
        .to_string();
    Ok((
        Command::Insert {
            parent: Parent::of(list, Branch::Children),
            index: kids.len(),
            node,
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

type Rules = Vec<(&'static str, Vec<(&'static str, &'static str)>)>;

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
    /// Défilement continu (comme le Ticker de Framer) : ses éléments défilent sans fin
    /// (animation en boucle « ticker » : vitesse, sens, arrêt au survol, bords estompés).
    Ticker,
    /// Carrousel : diapositives côte à côte qui défilent (flèches, compteur), comme le Carousel
    /// de Framer.
    Carousel,
    /// Diaporama : une diapositive à la fois, lecture automatique, points (Slideshow de Framer,
    /// Slider de Webflow).
    Slideshow,
    /// Barre de navigation : marque, liens, bouton « Menu » qui ouvre les liens sur tablette et
    /// téléphone (Navbar de Webflow).
    Navbar,
    /// Lecteur vidéo : image de couverture et bouton de lecture ; l'appui lance la vidéo
    /// (YouTube, Vimeo ou fichier) à leur place, comme le composant YouTube de Framer.
    VideoPlayer,
    /// Filtres de liste : boutons qui n'affichent que les éléments d'une liste dont les
    /// étiquettes contiennent leur valeur.
    ListFilters,
    /// Bandeau de consentement aux cookies (mesure d'audience chargée après accord).
    Consent,
    /// Interrupteur de la musique ou des sons du site (« On / Off »).
    Switch,
    /// Interrupteur clair / sombre (D-149) : bascule le site entre ses couleurs claires et
    /// sombres (Paramètres › Couleurs), choix gardé par le navigateur.
    ThemeSwitch,
    /// Média au survol : vidéo qui montre le média de l'élément survolé (contenus d'une liste
    /// de collection ou éléments choisis), voir [`crate::hover`].
    HoverMedia,
    /// Barre de progression (D-110) : remplie selon le défilement de la colonne qui la contient
    /// (ou de la page), comme le composant ScrollProgress de Framer.
    ScrollProgress,
    /// Image révélée au survol (D-125) : une seconde image par-dessus la première, montrée en
    /// entier, jusqu'au pointeur (volet) ou autour de lui (halo), comme un croquis qui laisse
    /// voir l'œuvre finie.
    Reveal,
    /// Panier et bon de commande (D-129), bloc prêt à l'emploi.
    BlockCart,
    /// Bouton « Ajouter au panier » (D-129) : produit et prix ajoutés au panier du site.
    CartAdd,
    /// Horloge (D-158) : heure ou date du visiteur, dans la langue de la page, comme le
    /// composant « Time & Date » de Framer.
    Clock,
    /// Toile de pixels (D-158) : dessin ou traînée de pixels sous le pointeur, comme le
    /// composant « Pixel Trail » de Framer.
    Pixels,
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
            Preset::BlockCart => Block::Cart,
            _ => return None,
        })
    }

    pub const ALL: [Preset; 43] = [
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
        Preset::Ticker,
        Preset::Carousel,
        Preset::Slideshow,
        Preset::Navbar,
        Preset::VideoPlayer,
        Preset::ListFilters,
        Preset::Consent,
        Preset::Switch,
        Preset::ThemeSwitch,
        Preset::HoverMedia,
        Preset::ScrollProgress,
        Preset::Reveal,
        Preset::BlockCart,
        Preset::CartAdd,
        Preset::Clock,
        Preset::Pixels,
    ];

    /// Règles de style pour tablette et téléphone (point de rupture le plus large du site) :
    /// (sélecteur, déclarations). Les éléments ajoutés n'en écrivent plus (D-159).
    pub fn narrow_rules(self) -> Rules {
        Vec::new()
    }

    /// Règles de style dont l'élément a besoin (ajoutées si le site ne les a pas) : celles des
    /// blocs prêts à l'emploi. Les autres éléments ont l'allure propre de Revel
    /// (`data-revel-base`, D-159) et aucune classe.
    pub fn rules(self) -> Rules {
        match self.block() {
            Some(b) => b.rules(),
            None => Vec::new(),
        }
    }

    /// Nouveau nœud (identifiants libres dans `site`). Les textes par défaut sont fournis par
    /// l'appelant (traduits dans la langue de l'interface).
    pub fn node(self, site: &Site, text: &str) -> Node {
        if let Some(b) = self.block() {
            return b.node(site);
        }
        let n = self.raw_node(site, text);
        self.finish(n)
    }

    /// Élément construit fermé (D-159) : un seul calque, réglé depuis ses réglages.
    pub fn is_widget(self) -> bool {
        matches!(
            self,
            Preset::Switch
                | Preset::ThemeSwitch
                | Preset::Consent
                | Preset::ListFilters
                | Preset::VideoPlayer
                | Preset::Reveal
                | Preset::ScrollProgress
                | Preset::Tabs
                | Preset::Accordion
                | Preset::Ticker
                | Preset::Carousel
                | Preset::Slideshow
                | Preset::Navbar
                | Preset::LocaleSwitch
        )
    }

    /// Classes de la construction remplacées par l'allure propre de Revel (D-159) : la racine
    /// reçoit `data-revel-base` (styles de Revel, priorité nulle, voir `revel_render::toolcss`),
    /// ses parties `data-revel-part` ; aucune classe n'est posée, l'utilisateur ajoute les
    /// siennes. Racine d'un élément construit : `data-revel-widget` (fermé).
    fn finish(self, mut n: Node) -> Node {
        let widget = self.is_widget();
        fn walk(n: &mut Node, root: bool, widget: bool, in_part: bool) {
            match n {
                Node::Element {
                    tag,
                    attrs,
                    children,
                    ..
                } => {
                    let class = attrs
                        .iter()
                        .find(|a| a.name == "class")
                        .and_then(|a| a.value.clone())
                        .unwrap_or_default();
                    attrs.retain(|a| a.name != "class");
                    let role = role_of(class.split_whitespace().next().unwrap_or(""));
                    let mut part = false;
                    match role {
                        Some(Role::Base(kind)) if root => attrs.insert(
                            0,
                            Attr {
                                name: revel_model::BASE_ATTR.into(),
                                value: Some(kind.into()),
                                when: None,
                            },
                        ),
                        Some(Role::Part(name)) if widget => {
                            part = true;
                            attrs.insert(
                                0,
                                Attr {
                                    name: revel_model::PART_ATTR.into(),
                                    value: Some(name.into()),
                                    when: None,
                                },
                            );
                        }
                        // Contenu accueilli (diapositive, panneau) : calque ordinaire.
                        Some(Role::Content) => {}
                        // Élément sans nom dans une partie (dessin d'un bouton…) : partie aussi.
                        _ if widget && !root && (in_part || role.is_none()) => {
                            part = true;
                            attrs.insert(
                                0,
                                Attr {
                                    name: revel_model::PART_ATTR.into(),
                                    value: Some(tag.to_ascii_lowercase()),
                                    when: None,
                                },
                            );
                        }
                        _ => {}
                    }
                    if root && widget {
                        attrs.push(Attr {
                            name: revel_model::WIDGET_ATTR.into(),
                            value: None,
                            when: None,
                        });
                    }
                    // Contenu accueilli : ce qu'il contient reste ordinaire.
                    let inner = widget && role != Some(Role::Content);
                    for c in children {
                        walk(c, false, inner, part);
                    }
                }
                Node::Image(img) => img.class = None,
                Node::Each { children, .. } => {
                    for c in children {
                        walk(c, false, widget, in_part);
                    }
                }
                _ => {}
            }
        }
        walk(&mut n, true, widget, false);
        n
    }

    fn raw_node(self, site: &Site, text: &str) -> Node {
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
            | Preset::BlockForm
            | Preset::BlockCart => el(id(), "section", vec![], vec![]),
            // Produit et prix : textes écrits, ou reliés aux champs d'un contenu (page d'un
            // produit, liste de produits).
            Preset::CartAdd => el(
                id(),
                "button",
                vec![
                    attr("class", "bouton-panier"),
                    attr("type", "button"),
                    Attr {
                        name: "data-revel-cart-add".into(),
                        value: None,
                        when: None,
                    },
                    attr("data-revel-product", "Produit"),
                    attr("data-revel-price", "0"),
                ],
                vec![txt(text)],
            ),
            // Texte remplacé par l'heure du visiteur ; celui écrit ici sert sans script.
            Preset::Clock => el(
                id(),
                "p",
                vec![attr("data-revel-clock", "time")],
                vec![txt("12:00")],
            ),
            Preset::Pixels => el(
                id(),
                "div",
                vec![
                    attr("class", "toile-pixels"),
                    attr("data-revel-pixels", "trail size=20"),
                ],
                vec![],
            ),
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
            // Image de dessous, puis image révélée (la dernière) ; halo de 140 px par défaut.
            Preset::Reveal => el(
                id(),
                "div",
                vec![
                    attr("class", "revele"),
                    attr("data-revel-reveal", "spot radius=140 soft=40"),
                ],
                vec![
                    Node::Image(ImageNode {
                        id: id(),
                        src: "\"\"".into(),
                        alt: Some("\"\"".into()),
                        class: Some("\"revele__avant\"".into()),
                        ..Default::default()
                    }),
                    Node::Image(ImageNode {
                        id: id(),
                        src: "\"\"".into(),
                        alt: Some("\"\"".into()),
                        class: Some("\"revele__apres\"".into()),
                        ..Default::default()
                    }),
                ],
            ),
            // Piste (suit le défilement de son conteneur) et remplissage (largeur écrite par le script).
            Preset::ScrollProgress => el(
                id(),
                "div",
                vec![
                    attr("class", "barre-progression"),
                    attr("data-revel-progress", "parent"),
                    attr("aria-hidden", "true"),
                ],
                vec![el(
                    id(),
                    "div",
                    vec![attr("class", "barre-progression__remplissage")],
                    vec![],
                )],
            ),
            // Sans nom : relié à des éléments survolés depuis ses réglages (nom attribué alors).
            Preset::HoverMedia => {
                let flag = |n: &str| Attr {
                    name: n.into(),
                    value: None,
                    when: None,
                };
                el(
                    id(),
                    "video",
                    vec![
                        attr("class", "media-survol"),
                        flag("muted"),
                        flag("playsinline"),
                        flag("loop"),
                        flag("autoplay"),
                        attr("preload", "none"),
                        flag(crate::hover::DISPLAY),
                    ],
                    vec![],
                )
            }
            // `text` : nom d'une icône (`fleche-droite`), sinon l'étoile.
            Preset::Icon => icon_node(
                id(),
                if crate::icons::icon(text.trim()).is_some() {
                    text.trim()
                } else {
                    "etoile"
                },
            ),
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
            Preset::Switch | Preset::ThemeSwitch => {
                // `text` : « Libellé|On|Off » (thème : « Thème|Sombre|Clair »).
                let theme = self == Preset::ThemeSwitch;
                let mut parts = text.split('|');
                let label = parts.next().unwrap_or(text).to_string();
                let on = parts
                    .next()
                    .unwrap_or(if theme { "Dark" } else { "On" })
                    .to_string();
                let off = parts
                    .next()
                    .unwrap_or(if theme { "Light" } else { "Off" })
                    .to_string();
                el(
                    id(),
                    "button",
                    vec![
                        attr("type", "button"),
                        attr("class", "interrupteur"),
                        attr("data-revel-switch", if theme { "theme" } else { "music" }),
                    ],
                    vec![
                        el(
                            id(),
                            "span",
                            vec![attr("class", "interrupteur__libelle")],
                            vec![txt(&label)],
                        ),
                        el(
                            id(),
                            "span",
                            vec![
                                attr("class", "interrupteur__etat"),
                                attr("data-revel-switch-text", &format!("{on}|{off}")),
                            ],
                            vec![txt(&on)],
                        ),
                    ],
                )
            }
            Preset::Consent => {
                // `text` : « Message|Refuser|Accepter ».
                let mut parts = text.split('|');
                let message = parts.next().unwrap_or(text).to_string();
                let refuse = parts.next().unwrap_or("Refuser").to_string();
                let accept = parts.next().unwrap_or("Accepter").to_string();
                el(
                    id(),
                    "div",
                    vec![
                        attr("class", "consentement"),
                        attr("role", "dialog"),
                        attr("data-revel-consent", ""),
                    ],
                    vec![
                        el(id(), "p", vec![], vec![txt(&message)]),
                        el(
                            id(),
                            "div",
                            vec![attr("class", "consentement__actions")],
                            vec![
                                el(
                                    id(),
                                    "button",
                                    vec![
                                        attr("type", "button"),
                                        attr("data-revel-consent-choice", "refuse"),
                                    ],
                                    vec![txt(&refuse)],
                                ),
                                el(
                                    id(),
                                    "button",
                                    vec![
                                        attr("type", "button"),
                                        attr("data-revel-consent-choice", "accept"),
                                    ],
                                    vec![txt(&accept)],
                                ),
                            ],
                        ),
                    ],
                )
            }
            Preset::ListFilters => {
                // `text` : « Tout|Option 1|Option 2 ».
                let mut parts = text.split('|');
                let all = parts.next().unwrap_or(text).to_string();
                let rest: Vec<String> = parts.map(str::to_string).collect();
                let rest = if rest.is_empty() {
                    vec!["1".to_string(), "2".to_string()]
                } else {
                    rest
                };
                let mut buttons = vec![el(
                    id(),
                    "button",
                    vec![
                        attr("class", "filtre"),
                        attr("type", "button"),
                        attr("data-revel-filter", ""),
                    ],
                    vec![txt(&all)],
                )];
                for (i, label) in rest.iter().enumerate() {
                    buttons.push(el(
                        id(),
                        "button",
                        vec![
                            attr("class", "filtre"),
                            attr("type", "button"),
                            attr("data-revel-filter", &format!("option-{}", i + 1)),
                        ],
                        vec![txt(label)],
                    ));
                }
                el(
                    id(),
                    "div",
                    vec![
                        attr("class", "filtres"),
                        attr("data-revel-filters", "filtre"),
                    ],
                    buttons,
                )
            }
            Preset::VideoPlayer => {
                let flag = |n: &str| Attr {
                    name: n.into(),
                    value: None,
                    when: None,
                };
                let (fig, btn, img, icon, svg, path) = (id(), id(), id(), id(), id(), id());
                el(
                    fig,
                    "figure",
                    vec![attr("class", "lecteur"), attr("data-revel-video", "")],
                    vec![el(
                        btn,
                        "button",
                        vec![
                            attr("class", "lecteur__couverture"),
                            attr("type", "button"),
                            attr("aria-label", text),
                            flag("data-revel-play"),
                        ],
                        vec![
                            Node::Image(ImageNode {
                                id: img,
                                src: "\"\"".into(),
                                alt: Some("\"\"".into()),
                                class: Some("\"lecteur__image\"".into()),
                                ..Default::default()
                            }),
                            el(
                                icon,
                                "span",
                                vec![
                                    attr("class", "lecteur__bouton"),
                                    attr("aria-hidden", "true"),
                                ],
                                vec![el(
                                    svg,
                                    "svg",
                                    vec![attr("viewBox", "0 0 24 24")],
                                    vec![el(
                                        path,
                                        "path",
                                        vec![
                                            attr("d", "M8 5v14l11-7z"),
                                            attr("fill", "currentColor"),
                                        ],
                                        vec![],
                                    )],
                                )],
                            ),
                        ],
                    )],
                )
            }
            Preset::Ticker => {
                let items = (1..=4)
                    .map(|n| {
                        el(
                            id(),
                            "div",
                            vec![attr("class", "defilement__element")],
                            vec![txt(&format!("{text} {n}"))],
                        )
                    })
                    .collect();
                el(
                    id(),
                    "div",
                    vec![
                        attr("class", "defilement"),
                        attr("data-revel-loop", "ticker speed=60 pause"),
                    ],
                    items,
                )
            }
            Preset::Carousel | Preset::Slideshow => {
                // `text` : « Diapositive|Précédente|Suivante ».
                let mut parts = text.split('|');
                let slide_text = parts.next().unwrap_or(text);
                let prev = parts.next().unwrap_or("←");
                let next = parts.next().unwrap_or("→");
                let carousel = self == Preset::Carousel;
                let b = if carousel { "carrousel" } else { "diaporama" };
                let slides = (1..=if carousel { 4 } else { 3 })
                    .map(|n| {
                        el(
                            id(),
                            "div",
                            vec![attr("class", &format!("{b}__diapo"))],
                            vec![el(
                                id(),
                                "p",
                                vec![],
                                vec![txt(&format!("{slide_text} {n}"))],
                            )],
                        )
                    })
                    .collect();
                let flag = |n: &str| Attr {
                    name: n.into(),
                    value: None,
                    when: None,
                };
                let button = |id: Option<String>, flag_name: &str, label: &str, sign: &str| {
                    el(
                        id,
                        "button",
                        vec![
                            attr("class", &format!("{b}__fleche")),
                            attr("type", "button"),
                            attr("aria-label", label),
                            flag(flag_name),
                        ],
                        vec![txt(sign)],
                    )
                };
                let middle = if carousel {
                    el(
                        id(),
                        "p",
                        vec![
                            attr("class", "carrousel__compteur"),
                            flag("data-revel-count"),
                        ],
                        vec![txt("1 / 4")],
                    )
                } else {
                    el(
                        id(),
                        "div",
                        vec![attr("class", "diaporama__points"), flag("data-revel-dots")],
                        vec![],
                    )
                };
                el(
                    id(),
                    "div",
                    vec![
                        attr("class", b),
                        attr(
                            "data-revel-slider",
                            if carousel { "" } else { "autoplay=5 loop" },
                        ),
                    ],
                    vec![
                        el(
                            id(),
                            "div",
                            vec![
                                attr("class", &format!("{b}__piste")),
                                flag("data-revel-track"),
                            ],
                            slides,
                        ),
                        el(
                            id(),
                            "div",
                            vec![attr("class", &format!("{b}__commandes"))],
                            vec![
                                button(id(), "data-revel-prev", prev, "←"),
                                middle,
                                button(id(), "data-revel-next", next, "→"),
                            ],
                        ),
                    ],
                )
            }
            Preset::Navbar => {
                // `text` : « Marque|Menu|Accueil|À propos|Contact ».
                let parts: Vec<&str> = text.split('|').collect();
                let get = |i: usize, d: &'static str| parts.get(i).copied().unwrap_or(d);
                let links = (2..parts.len().max(5))
                    .map(|i| {
                        el(
                            id(),
                            "a",
                            vec![attr("class", "barre-nav__lien"), attr("href", "/")],
                            vec![txt(get(i, "Lien"))],
                        )
                    })
                    .collect();
                el(
                    id(),
                    "nav",
                    vec![attr("class", "barre-nav")],
                    vec![
                        el(
                            id(),
                            "a",
                            vec![attr("class", "barre-nav__marque"), attr("href", "/")],
                            vec![txt(get(0, "Marque"))],
                        ),
                        el(
                            id(),
                            "button",
                            vec![
                                attr("class", "barre-nav__bouton"),
                                attr("type", "button"),
                                attr("aria-expanded", "false"),
                                attr("data-revel-on", "click class:is-open parent"),
                            ],
                            vec![txt(get(1, "Menu"))],
                        ),
                        el(id(), "div", vec![attr("class", "barre-nav__menu")], links),
                    ],
                )
            }
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

/// Place d'un élément d'une construction, d'après la classe qu'elle lui donnait.
#[derive(Clone, Copy, PartialEq)]
enum Role {
    /// Racine : allure propre de Revel (`data-revel-base`).
    Base(&'static str),
    /// Partie de l'élément construit (`data-revel-part`).
    Part(&'static str),
    /// Contenu accueilli (diapositive, panneau, lien du menu) : calque ordinaire.
    Content,
}

fn role_of(class: &str) -> Option<Role> {
    use Role::*;
    Some(match class {
        "grille" => Base("grid"),
        "colonnes" => Base("columns"),
        "grille__cellule" | "colonne" => Content,
        "button" => Base("button"),
        "video" => Base("video"),
        "toile-pixels" => Base("pixels"),
        "bouton-panier" => Base("cart-add"),
        "media-survol" => Base("hover-media"),
        "lottie" => Base("lottie"),
        "code-html" => Content,
        "liste-collection" => Base("collection"),
        "liste-collection__element" => Content,
        "revele" => Base("reveal"),
        "barre-progression" => Base("progress"),
        "barre-progression__remplissage" => Part("fill"),
        "onglets" => Base("tabs"),
        "onglets__liste" => Part("tablist"),
        "onglet" => Part("tab"),
        "onglets__panneau" => Content,
        "accordeon" => Base("accordion"),
        "accordeon__titre" => Part("summary"),
        "accordeon__contenu" => Content,
        "interrupteur" => Base("switch"),
        "interrupteur__libelle" => Part("label"),
        "interrupteur__etat" => Part("state"),
        "consentement" => Base("consent"),
        "consentement__actions" => Part("actions"),
        "filtres" => Base("filters"),
        "filtre" => Part("filter"),
        "lecteur" => Base("video-player"),
        "lecteur__couverture" => Part("cover"),
        "lecteur__bouton" => Part("play"),
        "defilement" => Base("ticker"),
        "defilement__element" => Content,
        "carrousel" => Base("carousel"),
        "diaporama" => Base("slideshow"),
        "carrousel__piste" | "diaporama__piste" => Part("track"),
        "carrousel__diapo" | "diaporama__diapo" => Content,
        "carrousel__commandes" | "diaporama__commandes" => Part("controls"),
        "carrousel__fleche" | "diaporama__fleche" => Part("arrow"),
        "carrousel__compteur" => Part("count"),
        "diaporama__points" => Part("dots"),
        "barre-nav" => Base("navbar"),
        "barre-nav__marque" | "barre-nav__lien" => Content,
        "barre-nav__menu" => Part("menu"),
        "barre-nav__bouton" => Part("toggle"),
        "langues" => Base("locales"),
        "langue" => Part("link"),
        _ => return None,
    })
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
        fields: Vec::new(),
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
        fields: Vec::new(),
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
        fields: Vec::new(),
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
                        children: Vec::new(),
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
        None if p.kind == revel_model::PropKind::List => "[]".into(),
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
        // `not x` : branche inverse.
        let negated = |c: &str| {
            c.trim()
                .strip_prefix("not ")
                .is_some_and(|x| x.trim() == name)
        };
        if let Node::If {
            cond,
            then,
            otherwise,
            ..
        } = &nodes[i]
            && (cond.trim() == name || negated(cond))
            && matches!(expr.trim(), "true" | "false")
        {
            let truth = (expr.trim() == "true") != negated(cond);
            let keep = if truth {
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
                } else if negated(cond) {
                    *cond = format!("not ({expr})");
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

/// Élément emplacement d'un contenu de composant (D-118) : son attribut retiré et, si `own` n'est
/// pas vide, ses enfants remplacés par `own`.
fn fill_slot(nodes: &mut [Node], own: &[Node]) {
    for n in nodes {
        if let Node::Element {
            attrs, children, ..
        } = n
            && attrs.iter().any(|a| a.name == revel_model::SLOT_ATTR)
        {
            attrs.retain(|a| a.name != revel_model::SLOT_ATTR);
            if !own.is_empty() {
                *children = own.to_vec();
            }
            continue;
        }
        if matches!(n, Node::Use { .. }) {
            continue;
        }
        for list in revel_model::ids::child_lists_mut(n) {
            fill_slot(list, own);
        }
    }
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
        children: own,
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
    // Emplacement : le contenu propre de l'utilisation remplace le contenu par défaut.
    fill_slot(&mut body, own);
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

/// Propriété `name` du gabarit `id` retirée : ses usages dans le gabarit reçoivent sa valeur
/// par défaut (texte écrit, condition résolue), les pages qui lui donnaient une valeur la perdent.
pub fn remove_layout_field(site: &Site, id: &str, name: &str) -> Result<Command, EditError> {
    let l = site
        .layouts
        .iter()
        .find(|l| l.id == id)
        .ok_or_else(|| EditError::NotFound(format!("gabarit {id}")))?;
    let p = l
        .field(name)
        .ok_or_else(|| EditError::NotFound(format!("propriété {name}")))?;
    let mut body = l.body.clone();
    substitute(&mut body, name, &prop_expr(p, &Default::default()));
    let mut commands = vec![
        Command::SetLayoutBody {
            id: id.to_string(),
            body,
        },
        Command::SetLayoutFields {
            id: id.to_string(),
            fields: l
                .fields
                .iter()
                .filter(|p| p.name != name)
                .cloned()
                .collect(),
        },
    ];
    // Pages dont un gabarit de la chaîne est `id`.
    for page in &site.pages {
        let mut cur = page.layout.clone();
        let mut hops = 0;
        let mut uses = false;
        while let Some(lid) = cur {
            if lid == id {
                uses = true;
                break;
            }
            hops += 1;
            if hops > 32 {
                break;
            }
            cur = site
                .layouts
                .iter()
                .find(|x| x.id == lid)
                .and_then(|x| x.parent.clone());
        }
        if uses && page.props.contains_key(name) {
            commands.push(Command::SetPageProp {
                page: page.id.clone(),
                key: name.to_string(),
                value: None,
            });
        }
    }
    Ok(Command::Batch { commands })
}

/// Nom libre parmi des propriétés (`texte`, `texte_2`…).
pub fn free_name(props: &[revel_model::PropDef], base: &str) -> String {
    let mut name = base.to_string();
    let mut n = 2;
    while props.iter().any(|p| p.name == name) {
        name = format!("{base}_{n}");
        n += 1;
    }
    name
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

/// Emplacement d'un composant (D-118) : élément `data-revel-slot` de son contenu.
pub fn slot_of(c: &revel_model::Component) -> Option<&Node> {
    fn find(nodes: &[Node]) -> Option<&Node> {
        for n in nodes {
            if let Node::Element { attrs, .. } = n
                && attrs.iter().any(|a| a.name == revel_model::SLOT_ATTR)
            {
                return Some(n);
            }
            if matches!(n, Node::Use { .. }) {
                continue;
            }
            for list in revel_model::ids::child_lists(n) {
                if let Some(f) = find(list) {
                    return Some(f);
                }
            }
        }
        None
    }
    find(&c.body)
}

/// Contenu par défaut de l'emplacement copié dans l'utilisation `at` (nouveaux identifiants),
/// pour le modifier à cet endroit seulement.
pub fn customize_slot(site: &Site, at: &Target) -> Result<Command, EditError> {
    let Node::Use {
        component,
        children,
        args,
        ..
    } = address::node(site, at)?
    else {
        return Err(EditError::WrongKind(at.node.clone()));
    };
    let c = site
        .component(component)
        .ok_or_else(|| EditError::NotFound(component.clone()))?;
    let Some(Node::Element {
        children: default, ..
    }) = slot_of(c)
    else {
        return Err(EditError::NotFound(format!(
            "emplacement de « {component} »"
        )));
    };
    // Propriétés lues dans l'emplacement : valeur de cette utilisation écrite à leur place.
    let mut default = default.clone();
    for p in &c.props {
        substitute(&mut default, &p.name, &prop_expr(p, args));
    }
    let parent = Parent {
        owner: at.owner.clone(),
        node: Some(at.node.clone()),
        branch: Branch::Children,
    };
    let mut taken = site.node_ids();
    let start = children.len();
    let commands = default
        .into_iter()
        .enumerate()
        .map(|(i, mut n)| {
            refresh_ids(&mut n, &mut taken);
            Command::Insert {
                parent: parent.clone(),
                index: start + i,
                node: n,
            }
        })
        .collect();
    Ok(Command::Batch { commands })
}

/// Contenu propre de l'utilisation `at` retiré : l'emplacement montre de nouveau le contenu du
/// composant.
pub fn reset_slot(site: &Site, at: &Target) -> Result<Command, EditError> {
    let Node::Use { children, .. } = address::node(site, at)? else {
        return Err(EditError::WrongKind(at.node.clone()));
    };
    let commands = children
        .iter()
        .filter_map(revel_model::ids::node_id)
        .map(|id| Command::Remove {
            at: Target {
                owner: at.owner.clone(),
                node: id.to_string(),
            },
        })
        .collect();
    Ok(Command::Batch { commands })
}

/// L'élément `at` d'un composant devient (ou n'est plus) son emplacement ; un seul emplacement
/// par composant : l'ancien est retiré.
pub fn set_slot(site: &Site, at: &Target, on: bool) -> Result<Command, EditError> {
    let Owner::Component(cid) = &at.owner else {
        return Err(EditError::WrongKind(at.node.clone()));
    };
    let mut commands = Vec::new();
    if on
        && let Some(c) = site.component(cid)
        && let Some(old) = slot_of(c).and_then(revel_model::ids::node_id)
        && old != at.node
    {
        commands.push(Command::RemoveAttr {
            at: Target {
                owner: at.owner.clone(),
                node: old.to_string(),
            },
            name: revel_model::SLOT_ATTR.into(),
        });
    }
    commands.push(if on {
        Command::SetAttr {
            at: at.clone(),
            name: revel_model::SLOT_ATTR.into(),
            value: None,
        }
    } else {
        Command::RemoveAttr {
            at: at.clone(),
            name: revel_model::SLOT_ATTR.into(),
        }
    });
    Ok(Command::Batch { commands })
}
