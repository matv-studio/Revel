//! Blocs prêts à l'emploi (3.7) : sections complètes (en-tête de page, avantages, texte et image,
//! appel à l'action, témoignage, questions fréquentes, contact), comme les « Layouts » de
//! Webflow. Textes dans la langue du site ; styles sobres, réglables par les variables CSS
//! `--bloc-accent`, `--bloc-accent-texte` et `--bloc-fond`.

use revel_model::{Node, Site};

/// Bloc prêt à l'emploi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    Hero,
    Features,
    TextImage,
    CallToAction,
    Testimonial,
    Faq,
    Contact,
    /// Formulaire de contact reçu par l'hébergement (5.8).
    Form,
    /// Panier et bon de commande (D-129) : lignes ajoutées par les boutons « Ajouter au
    /// panier » du site, commande envoyée par un formulaire du site, sans service de paiement.
    Cart,
}

type Rules = Vec<(&'static str, Vec<(&'static str, &'static str)>)>;

/// Règles communes à tous les blocs.
fn common() -> Rules {
    vec![
        (".bloc", vec![("padding", "96px 24px")]),
        (
            ".bloc__inner",
            vec![("max-width", "1120px"), ("margin", "0 auto")],
        ),
        (
            ".bloc-titre",
            vec![
                ("margin", "0 0 16px"),
                ("font-size", "clamp(32px, 5vw, 56px)"),
                ("line-height", "1.1"),
            ],
        ),
        (
            ".bloc-sous-titre",
            vec![
                ("margin", "0 0 12px"),
                ("font-size", "clamp(26px, 3.5vw, 40px)"),
                ("line-height", "1.15"),
            ],
        ),
        (
            ".bloc-texte",
            vec![
                ("margin", "0 0 24px"),
                ("max-width", "640px"),
                ("font-size", "18px"),
                ("opacity", "0.8"),
            ],
        ),
        (
            ".bloc-actions",
            vec![("display", "flex"), ("flex-wrap", "wrap"), ("gap", "12px")],
        ),
        (
            ".bloc-bouton",
            vec![
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("padding", "12px 22px"),
                ("border-radius", "999px"),
                ("background", "var(--bloc-accent, #2f6fed)"),
                ("color", "var(--bloc-accent-texte, #ffffff)"),
                ("font-weight", "600"),
                ("text-decoration", "none"),
            ],
        ),
        (
            ".bloc-bouton--secondaire",
            vec![
                ("background", "transparent"),
                ("color", "inherit"),
                ("box-shadow", "inset 0 0 0 1.5px currentColor"),
            ],
        ),
    ]
}

impl Block {
    pub const ALL: [Block; 9] = [
        Block::Hero,
        Block::Features,
        Block::TextImage,
        Block::CallToAction,
        Block::Testimonial,
        Block::Faq,
        Block::Contact,
        Block::Form,
        Block::Cart,
    ];

    /// Contenu en balisage (langue du site : français, sinon anglais). `<i data-icone="…">` :
    /// icône remplacée par son dessin.
    fn markup(self, fr: bool) -> &'static str {
        match (self, fr) {
            (Block::Hero, true) => {
                r##"<section class="bloc bloc-hero"><div class="bloc__inner"><h1 class="bloc-titre">Un titre clair pour votre activité</h1><p class="bloc-texte">Une phrase qui dit ce que vous faites, pour qui, et pourquoi on vous choisit.</p><div class="bloc-actions"><a class="bloc-bouton" href="/contact/">Nous contacter</a><a class="bloc-bouton bloc-bouton--secondaire" href="#">En savoir plus</a></div></div></section>"##
            }
            (Block::Hero, false) => {
                r##"<section class="bloc bloc-hero"><div class="bloc__inner"><h1 class="bloc-titre">A clear headline for your business</h1><p class="bloc-texte">One sentence about what you do, for whom, and why people choose you.</p><div class="bloc-actions"><a class="bloc-bouton" href="/contact/">Contact us</a><a class="bloc-bouton bloc-bouton--secondaire" href="#">Learn more</a></div></div></section>"##
            }
            (Block::Features, true) => {
                r##"<section class="bloc bloc-avantages"><div class="bloc__inner"><h2 class="bloc-sous-titre">Ce que nous apportons</h2><div class="bloc-avantages__grille"><article class="bloc-avantage"><i data-icone="eclair"></i><h3>Rapide</h3><p>Un premier résultat en quelques jours, puis des améliorations régulières.</p></article><article class="bloc-avantage"><i data-icone="cible"></i><h3>Précis</h3><p>Des choix guidés par vos objectifs et par ce que vos clients attendent.</p></article><article class="bloc-avantage"><i data-icone="coeur"></i><h3>Attentif</h3><p>Un interlocuteur unique, disponible du début à la fin du projet.</p></article></div></div></section>"##
            }
            (Block::Features, false) => {
                r##"<section class="bloc bloc-avantages"><div class="bloc__inner"><h2 class="bloc-sous-titre">What we bring</h2><div class="bloc-avantages__grille"><article class="bloc-avantage"><i data-icone="eclair"></i><h3>Fast</h3><p>A first result within days, then regular improvements.</p></article><article class="bloc-avantage"><i data-icone="cible"></i><h3>Focused</h3><p>Choices driven by your goals and by what your customers expect.</p></article><article class="bloc-avantage"><i data-icone="coeur"></i><h3>Caring</h3><p>A single contact, available from start to finish.</p></article></div></div></section>"##
            }
            (Block::TextImage, true) => {
                r##"<section class="bloc bloc-texte-image"><div class="bloc__inner bloc-texte-image__inner"><div><h2 class="bloc-sous-titre">Notre façon de travailler</h2><p class="bloc-texte">Racontez votre méthode, votre histoire ou un projet dont vous êtes fiers. Une image vaut mieux qu'un long discours.</p><a class="bloc-bouton bloc-bouton--secondaire" href="#">Découvrir</a></div><img class="bloc-texte-image__image" src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 800 600'%3E%3Crect width='800' height='600' fill='%23e6e6e9'/%3E%3Cpath d='M220 420l130-150 95 105 65-65 130 110z' fill='%23c9c9ce'/%3E%3Ccircle cx='540' cy='220' r='42' fill='%23c9c9ce'/%3E%3C/svg%3E" alt="Illustration" width="800" height="600"></div></section>"##
            }
            (Block::TextImage, false) => {
                r##"<section class="bloc bloc-texte-image"><div class="bloc__inner bloc-texte-image__inner"><div><h2 class="bloc-sous-titre">How we work</h2><p class="bloc-texte">Tell your method, your story or a project you are proud of. A picture is worth a thousand words.</p><a class="bloc-bouton bloc-bouton--secondaire" href="#">Discover</a></div><img class="bloc-texte-image__image" src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 800 600'%3E%3Crect width='800' height='600' fill='%23e6e6e9'/%3E%3Cpath d='M220 420l130-150 95 105 65-65 130 110z' fill='%23c9c9ce'/%3E%3Ccircle cx='540' cy='220' r='42' fill='%23c9c9ce'/%3E%3C/svg%3E" alt="Illustration" width="800" height="600"></div></section>"##
            }
            (Block::CallToAction, true) => {
                r##"<section class="bloc bloc-appel"><div class="bloc__inner bloc-appel__inner"><h2 class="bloc-sous-titre">Un projet en tête ?</h2><p class="bloc-texte">Parlons-en : la première rencontre est sans engagement.</p><div class="bloc-actions"><a class="bloc-bouton" href="/contact/">Prendre rendez-vous</a></div></div></section>"##
            }
            (Block::CallToAction, false) => {
                r##"<section class="bloc bloc-appel"><div class="bloc__inner bloc-appel__inner"><h2 class="bloc-sous-titre">Have a project in mind?</h2><p class="bloc-texte">Let's talk: the first meeting comes with no commitment.</p><div class="bloc-actions"><a class="bloc-bouton" href="/contact/">Book a meeting</a></div></div></section>"##
            }
            (Block::Testimonial, true) => {
                r##"<section class="bloc bloc-temoignage"><div class="bloc__inner"><blockquote><p class="bloc-temoignage__citation">« Une équipe à l'écoute, des délais tenus et un résultat qui a dépassé nos attentes. »</p><cite class="bloc-temoignage__auteur">Prénom Nom, fonction, entreprise</cite></blockquote></div></section>"##
            }
            (Block::Testimonial, false) => {
                r##"<section class="bloc bloc-temoignage"><div class="bloc__inner"><blockquote><p class="bloc-temoignage__citation">“A team that listens, deadlines met, and a result beyond our expectations.”</p><cite class="bloc-temoignage__auteur">First Last, role, company</cite></blockquote></div></section>"##
            }
            (Block::Faq, true) => {
                r##"<section class="bloc bloc-faq"><div class="bloc__inner"><h2 class="bloc-sous-titre">Questions fréquentes</h2><details><summary>Combien de temps dure un projet ?</summary><p>De quelques semaines à quelques mois selon son ampleur ; un calendrier est fixé dès le départ.</p></details><details><summary>Comment se passe le premier rendez-vous ?</summary><p>Nous écoutons vos besoins, puis nous proposons une démarche et un budget.</p></details><details><summary>Travaillez-vous à distance ?</summary><p>Oui, et nous nous déplaçons quand le projet le demande.</p></details></div></section>"##
            }
            (Block::Faq, false) => {
                r##"<section class="bloc bloc-faq"><div class="bloc__inner"><h2 class="bloc-sous-titre">Frequently asked questions</h2><details><summary>How long does a project take?</summary><p>From a few weeks to a few months depending on its scope; a schedule is set from the start.</p></details><details><summary>How does the first meeting go?</summary><p>We listen to your needs, then propose an approach and a budget.</p></details><details><summary>Do you work remotely?</summary><p>Yes, and we travel when the project calls for it.</p></details></div></section>"##
            }
            (Block::Contact, true) => {
                r##"<section class="bloc bloc-contact"><div class="bloc__inner"><h2 class="bloc-sous-titre">Nous contacter</h2><p class="bloc-texte">Écrivez-nous ou appelez-nous, nous répondons sous 48 heures.</p><ul class="bloc-contact__liste"><li><i data-icone="courriel"></i><a href="mailto:bonjour@exemple.fr">bonjour@exemple.fr</a></li><li><i data-icone="telephone"></i><a href="tel:+33100000000">01 00 00 00 00</a></li><li><i data-icone="lieu"></i><span>1 rue de l'Exemple, 75000 Paris</span></li></ul></div></section>"##
            }
            (Block::Form, true) => {
                r##"<section class="bloc bloc-formulaire"><div class="bloc__inner"><h2 class="bloc-sous-titre">Écrivez-nous</h2><form class="formulaire" data-revel-form="contact"><div class="formulaire__ligne"><label class="champ"><span class="champ__libelle">Nom</span><input class="champ__saisie" type="text" name="nom" autocomplete="name" required></label><label class="champ"><span class="champ__libelle">E-mail</span><input class="champ__saisie" type="email" name="email" autocomplete="email" required></label></div><label class="champ"><span class="champ__libelle">Message</span><textarea class="champ__saisie" name="message" rows="5" required></textarea></label><button class="bloc-bouton" type="submit">Envoyer</button><p class="formulaire__etat" data-revel-form-status role="status" aria-live="polite"></p></form></div></section>"##
            }
            (Block::Form, false) => {
                r##"<section class="bloc bloc-formulaire"><div class="bloc__inner"><h2 class="bloc-sous-titre">Write to us</h2><form class="formulaire" data-revel-form="contact"><div class="formulaire__ligne"><label class="champ"><span class="champ__libelle">Name</span><input class="champ__saisie" type="text" name="name" autocomplete="name" required></label><label class="champ"><span class="champ__libelle">Email</span><input class="champ__saisie" type="email" name="email" autocomplete="email" required></label></div><label class="champ"><span class="champ__libelle">Message</span><textarea class="champ__saisie" name="message" rows="5" required></textarea></label><button class="bloc-bouton" type="submit">Send</button><p class="formulaire__etat" data-revel-form-status role="status" aria-live="polite"></p></form></div></section>"##
            }
            (Block::Cart, true) => {
                r##"<section class="bloc bloc-panier"><div class="bloc__inner"><h2 class="bloc-sous-titre">Votre panier</h2><div class="panier" data-revel-cart="EUR"><ul class="panier__lignes"><li class="panier__ligne" data-revel-cart-item><span class="panier__nom" data-revel-cart-name>Produit</span><span class="panier__quantite"><button class="panier__moins" type="button" aria-label="Retirer un" data-revel-cart-minus>−</button><span data-revel-cart-qty>1</span><button class="panier__plus" type="button" aria-label="Ajouter un" data-revel-cart-plus>+</button></span><span class="panier__prix" data-revel-cart-price>0,00 €</span><button class="panier__retirer" type="button" aria-label="Retirer du panier" data-revel-cart-remove>×</button></li></ul><p class="panier__vide" data-revel-cart-empty>Votre panier est vide.</p><p class="panier__total">Total : <strong data-revel-cart-total>0,00 €</strong></p><form class="formulaire panier__commande" data-revel-form="commande"><input type="hidden" name="commande" data-revel-cart-field><div class="formulaire__ligne"><label class="champ"><span class="champ__libelle">Nom</span><input class="champ__saisie" type="text" name="nom" autocomplete="name" required></label><label class="champ"><span class="champ__libelle">E-mail</span><input class="champ__saisie" type="email" name="email" autocomplete="email" required></label></div><label class="champ"><span class="champ__libelle">Adresse de livraison</span><textarea class="champ__saisie" name="adresse" rows="3" required></textarea></label><button class="bloc-bouton" type="submit">Envoyer la commande</button><p class="formulaire__etat" data-revel-form-status role="status" aria-live="polite"></p></form></div></div></section>"##
            }
            (Block::Cart, false) => {
                r##"<section class="bloc bloc-panier"><div class="bloc__inner"><h2 class="bloc-sous-titre">Your cart</h2><div class="panier" data-revel-cart="USD"><ul class="panier__lignes"><li class="panier__ligne" data-revel-cart-item><span class="panier__nom" data-revel-cart-name>Product</span><span class="panier__quantite"><button class="panier__moins" type="button" aria-label="Remove one" data-revel-cart-minus>−</button><span data-revel-cart-qty>1</span><button class="panier__plus" type="button" aria-label="Add one" data-revel-cart-plus>+</button></span><span class="panier__prix" data-revel-cart-price>$0.00</span><button class="panier__retirer" type="button" aria-label="Remove from cart" data-revel-cart-remove>×</button></li></ul><p class="panier__vide" data-revel-cart-empty>Your cart is empty.</p><p class="panier__total">Total: <strong data-revel-cart-total>$0.00</strong></p><form class="formulaire panier__commande" data-revel-form="order"><input type="hidden" name="order" data-revel-cart-field><div class="formulaire__ligne"><label class="champ"><span class="champ__libelle">Name</span><input class="champ__saisie" type="text" name="name" autocomplete="name" required></label><label class="champ"><span class="champ__libelle">Email</span><input class="champ__saisie" type="email" name="email" autocomplete="email" required></label></div><label class="champ"><span class="champ__libelle">Shipping address</span><textarea class="champ__saisie" name="address" rows="3" required></textarea></label><button class="bloc-bouton" type="submit">Send the order</button><p class="formulaire__etat" data-revel-form-status role="status" aria-live="polite"></p></form></div></div></section>"##
            }
            (Block::Contact, false) => {
                r##"<section class="bloc bloc-contact"><div class="bloc__inner"><h2 class="bloc-sous-titre">Contact us</h2><p class="bloc-texte">Write or call us, we answer within 48 hours.</p><ul class="bloc-contact__liste"><li><i data-icone="courriel"></i><a href="mailto:hello@example.com">hello@example.com</a></li><li><i data-icone="telephone"></i><a href="tel:+15550000000">+1 555 000 0000</a></li><li><i data-icone="lieu"></i><span>1 Example Street, City</span></li></ul></div></section>"##
            }
        }
    }

    /// Règles de style du bloc (communes comprises).
    pub fn rules(self) -> Rules {
        let mut r = common();
        r.extend(match self {
            Block::Hero => vec![(".bloc-hero .bloc-titre", vec![("max-width", "820px")])],
            Block::Features => vec![
                (
                    ".bloc-avantages__grille",
                    vec![
                        ("display", "grid"),
                        (
                            "grid-template-columns",
                            "repeat(auto-fit, minmax(220px, 1fr))",
                        ),
                        ("gap", "32px"),
                        ("margin-top", "40px"),
                    ],
                ),
                (
                    ".bloc-avantage .icone",
                    vec![
                        ("width", "32px"),
                        ("height", "32px"),
                        ("margin-bottom", "12px"),
                        ("color", "var(--bloc-accent, #2f6fed)"),
                    ],
                ),
                (
                    ".bloc-avantage h3",
                    vec![("margin", "0 0 8px"), ("font-size", "20px")],
                ),
                (
                    ".bloc-avantage p",
                    vec![("margin", "0"), ("opacity", "0.8")],
                ),
            ],
            Block::TextImage => vec![
                (
                    ".bloc-texte-image__inner",
                    vec![
                        ("display", "grid"),
                        (
                            "grid-template-columns",
                            "repeat(auto-fit, minmax(280px, 1fr))",
                        ),
                        ("gap", "48px"),
                        ("align-items", "center"),
                    ],
                ),
                (
                    ".bloc-texte-image img",
                    vec![
                        ("display", "block"),
                        ("width", "100%"),
                        ("height", "auto"),
                        ("border-radius", "12px"),
                    ],
                ),
            ],
            Block::CallToAction => vec![
                (
                    ".bloc-appel__inner",
                    vec![
                        ("padding", "56px 32px"),
                        ("border-radius", "20px"),
                        ("background", "var(--bloc-fond, rgba(127, 127, 127, 0.12))"),
                        ("text-align", "center"),
                    ],
                ),
                (".bloc-appel .bloc-texte", vec![("margin", "0 auto 24px")]),
                (
                    ".bloc-appel .bloc-actions",
                    vec![("justify-content", "center")],
                ),
            ],
            Block::Testimonial => vec![
                (
                    ".bloc-temoignage blockquote",
                    vec![
                        ("max-width", "820px"),
                        ("margin", "0 auto"),
                        ("text-align", "center"),
                    ],
                ),
                (
                    ".bloc-temoignage__citation",
                    vec![
                        ("margin", "0 0 20px"),
                        ("font-size", "clamp(22px, 3vw, 32px)"),
                        ("line-height", "1.35"),
                    ],
                ),
                (
                    ".bloc-temoignage__auteur",
                    vec![("font-style", "normal"), ("opacity", "0.8")],
                ),
            ],
            Block::Faq => vec![
                (
                    ".bloc-faq details",
                    vec![
                        ("padding", "18px 0"),
                        ("border-top", "1px solid rgba(127, 127, 127, 0.3)"),
                    ],
                ),
                (
                    ".bloc-faq summary",
                    vec![
                        ("font-size", "18px"),
                        ("font-weight", "600"),
                        ("cursor", "pointer"),
                    ],
                ),
                (
                    ".bloc-faq details p",
                    vec![("margin", "12px 0 0"), ("opacity", "0.8")],
                ),
            ],
            Block::Cart => {
                let mut r = Block::Form.rules();
                r.extend(vec![
                    (".panier__lignes", vec![("margin", "24px 0 0"), ("padding", "0"), ("list-style", "none")]),
                    (
                        ".panier__ligne",
                        vec![
                            ("display", "grid"),
                            ("grid-template-columns", "1fr auto auto auto"),
                            ("gap", "16px"),
                            ("align-items", "center"),
                            ("padding", "14px 0"),
                            ("border-bottom", "1px solid rgba(127, 127, 127, 0.3)"),
                        ],
                    ),
                    (".panier__quantite", vec![("display", "inline-flex"), ("align-items", "center"), ("gap", "10px")]),
                    (
                        ".panier__moins, .panier__plus, .panier__retirer",
                        vec![
                            ("width", "30px"),
                            ("height", "30px"),
                            ("border", "1px solid rgba(127, 127, 127, 0.4)"),
                            ("border-radius", "50%"),
                            ("background", "none"),
                            ("color", "inherit"),
                            ("font", "inherit"),
                            ("cursor", "pointer"),
                        ],
                    ),
                    (".panier__total", vec![("margin", "20px 0 0"), ("font-size", "20px"), ("text-align", "right")]),
                    (".panier.is-empty .panier__total, .panier.is-empty .panier__commande", vec![("display", "none")]),
                ]);
                return r;
            }
            Block::Form => vec![
                (
                    ".formulaire",
                    vec![
                        ("display", "grid"),
                        ("gap", "16px"),
                        ("max-width", "640px"),
                        ("margin-top", "24px"),
                    ],
                ),
                (
                    ".formulaire__ligne",
                    vec![
                        ("display", "grid"),
                        (
                            "grid-template-columns",
                            "repeat(auto-fit, minmax(220px, 1fr))",
                        ),
                        ("gap", "16px"),
                    ],
                ),
                (".champ", vec![("display", "grid"), ("gap", "6px")]),
                (
                    ".champ__libelle",
                    vec![("font-size", "14px"), ("font-weight", "600")],
                ),
                (
                    ".champ__saisie",
                    vec![
                        ("width", "100%"),
                        ("padding", "12px 14px"),
                        ("border", "1px solid rgba(127, 127, 127, 0.35)"),
                        ("border-radius", "8px"),
                        ("background", "transparent"),
                        ("color", "inherit"),
                        ("font", "inherit"),
                    ],
                ),
                (
                    ".formulaire .bloc-bouton",
                    vec![
                        ("justify-self", "start"),
                        ("border", "0"),
                        ("cursor", "pointer"),
                    ],
                ),
                (
                    ".formulaire__etat",
                    vec![("margin", "0"), ("min-height", "1.5em")],
                ),
            ],
            Block::Contact => vec![
                (
                    ".bloc-contact__liste",
                    vec![
                        ("display", "grid"),
                        ("gap", "16px"),
                        ("margin", "24px 0 0"),
                        ("padding", "0"),
                        ("list-style", "none"),
                    ],
                ),
                (
                    ".bloc-contact__liste li",
                    vec![
                        ("display", "flex"),
                        ("align-items", "center"),
                        ("gap", "12px"),
                        ("font-size", "18px"),
                    ],
                ),
                (
                    ".bloc-contact .icone",
                    vec![
                        ("width", "24px"),
                        ("height", "24px"),
                        ("color", "var(--bloc-accent, #2f6fed)"),
                    ],
                ),
            ],
        });
        r
    }

    /// Nœuds du bloc (nouveaux identifiants libres dans `site`), dans la langue du site.
    pub fn node(self, site: &Site) -> Node {
        let fr = site.default_locale.to_ascii_lowercase().starts_with("fr");
        let mut nodes = revel_markup::parse(
            self.markup(fr),
            &revel_markup::Options {
                dialect: revel_markup::Dialect::Nunjucks,
                ..Default::default()
            },
        )
        .map(|p| p.nodes)
        .unwrap_or_default();
        icons_in(&mut nodes);
        let mut node = nodes.into_iter().next().unwrap_or(Node::Element {
            id: None,
            tag: "section".into(),
            attrs: vec![],
            children: vec![],
        });
        let mut taken = site.node_ids();
        crate::make::refresh_ids(&mut node, &mut taken);
        node
    }
}

/// `<i data-icone="nom">` -> icône dessinée.
fn icons_in(nodes: &mut [Node]) {
    for n in nodes.iter_mut() {
        if let Node::Element { tag, attrs, .. } = n
            && tag == "i"
            && let Some(name) = attrs
                .iter()
                .find(|a| a.name == "data-icone")
                .and_then(|a| a.value.clone())
        {
            *n = crate::make::icon_node(None, &name);
            continue;
        }
        for list in revel_model::ids::child_lists_mut(n) {
            icons_in(list);
        }
    }
}
