use super::*;
use revel_model::build::*;
use revel_model::ids::node_id;
use revel_model::{Component, Node, Page, Site};

fn sample() -> Site {
    let mut s = Site::new("Essai", "fr");
    s.pages.push(Page {
        id: "accueil".into(),
        path: "/".into(),
        layout: None,
        props: Default::default(),
        computed: Default::default(),
        body: vec![
            el(
                "main",
                &[("class", "page")],
                vec![
                    el("h1", &[("class", "titre")], vec![text("Bonjour")]),
                    el("p", &[], vec![text("Texte "), out("nom"), text(" fin")]),
                    Node::If {
                        id: None,
                        cond: "a".into(),
                        then: vec![el("span", &[], vec![text("oui")])],
                        otherwise: vec![],
                    },
                    Node::Each {
                        id: None,
                        expr: "items".into(),
                        item: "i".into(),
                        children: vec![el("li", &[], vec![out("i")])],
                        empty: vec![],
                    },
                ],
            ),
            el("footer", &[], vec![]),
        ],
        draft: false,
        code: Default::default(),
        name: None,
    });
    s.components.push(Component::new(
        "carte",
        vec![el("article", &[], vec![el("h3", &[], vec![text("Carte")])])],
    ));
    s.stylesheet = Some(revel_model::Stylesheet {
        url: "/assets/site.css".into(),
        source: None,
        items: revel_style::parse(
            ".titre { color: red; margin: 0 }
             .page { display: grid }
             @media (max-width: 860px) { /* Mobile */ .titre { color: blue } }
             @keyframes k { from { opacity: 0 } }",
        )
        .unwrap(),
    });
    s.ensure_ids();
    s
}

/// Identifiant de la règle `selector` (requête `media`).
fn rule_id(s: &Site, selector: &str, media: Option<&str>) -> String {
    s.style_rules()
        .find(|r| r.selector == selector && r.media.as_deref() == media)
        .and_then(|r| r.id.clone())
        .unwrap_or_else(|| panic!("{selector}"))
}

fn css(s: &Site) -> String {
    revel_style::render(s.stylesheet.as_ref().unwrap())
}

fn page() -> Owner {
    Owner::Page("accueil".into())
}

/// Identifiant du n-ième élément de balise `tag` dans la page.
fn id_of(s: &Site, tag: &str) -> String {
    let mut found = None;
    revel_model::ids::walk(&s.pages[0].body, &mut |n| {
        if found.is_none()
            && let Node::Element { tag: t, id, .. } = n
            && t == tag
        {
            found = id.clone();
        }
    });
    found.unwrap_or_else(|| panic!("{tag}"))
}

fn at(s: &Site, tag: &str) -> Target {
    Target {
        owner: page(),
        node: id_of(s, tag),
    }
}

fn json(s: &Site) -> String {
    s.to_json().unwrap()
}

/// Applique, vérifie l'effet, applique l'inverse : le document redevient identique octet pour octet.
fn round_trip(cmd: Command, check: impl Fn(&Site)) {
    let mut s = sample();
    let before = json(&s);
    let inv = apply(&mut s, &cmd).unwrap();
    check(&s);
    // Aller-retour JSON des deux commandes.
    let c2: Command = serde_json::from_str(&serde_json::to_string(&cmd).unwrap()).unwrap();
    assert_eq!(c2, cmd);
    let i2: Command = serde_json::from_str(&serde_json::to_string(&inv).unwrap()).unwrap();
    apply(&mut s, &i2).unwrap();
    assert_eq!(json(&s), before, "inverse de {cmd:?}");
}

#[test]
fn attributs() {
    let s = sample();
    round_trip(
        Command::SetAttr {
            at: at(&s, "h1"),
            name: "class".into(),
            value: Some("titre grand".into()),
        },
        |s| assert!(json(s).contains("titre grand")),
    );
    round_trip(
        Command::SetAttr {
            at: at(&s, "h1"),
            name: "hidden".into(),
            value: None,
        },
        |s| assert!(json(s).contains("\"hidden\"")),
    );
    round_trip(
        Command::RemoveAttr {
            at: at(&s, "main"),
            name: "class".into(),
        },
        |s| assert!(!json(s).contains("\"page\"")),
    );
    let mut t = sample();
    let err = apply(
        &mut t,
        &Command::RemoveAttr {
            at: at(&s, "h1"),
            name: "absent".into(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, EditError::NotFound(_)));
}

#[test]
fn balise_texte_remplacement() {
    let s = sample();
    round_trip(
        Command::SetTag {
            at: at(&s, "h1"),
            tag: "h2".into(),
        },
        |s| assert!(json(s).contains("\"h2\"")),
    );
    let p = Parent::of(&at(&s, "p"), Branch::Children);
    round_trip(
        Command::SetText {
            parent: p.clone(),
            index: 0,
            text: "Autre ".into(),
        },
        |s| assert!(json(s).contains("Autre ")),
    );
    round_trip(
        Command::Replace {
            parent: p.clone(),
            index: 1,
            node: out("prenom"),
        },
        |s| assert!(json(s).contains("prenom")),
    );
    let mut t = sample();
    assert!(matches!(
        apply(
            &mut t,
            &Command::SetText {
                parent: p,
                index: 1,
                text: "x".into()
            }
        ),
        Err(EditError::NotText(1))
    ));
    assert!(matches!(
        apply(
            &mut t,
            &Command::SetTag {
                at: at(&s, "h1"),
                tag: "h1 onclick".into()
            }
        ),
        Err(EditError::InvalidTag(_))
    ));
}

#[test]
fn insertion_suppression_deplacement() {
    let s = sample();
    let main = Parent::of(&at(&s, "main"), Branch::Children);
    let mut n = el("div", &[], vec![text("neuf")]);
    let mut taken = s.node_ids();
    make::refresh_ids(&mut n, &mut taken);
    round_trip(
        Command::Insert {
            parent: main.clone(),
            index: 1,
            node: n.clone(),
        },
        |s| assert!(json(s).contains("neuf")),
    );
    round_trip(Command::Remove { at: at(&s, "p") }, |s| {
        assert!(!json(s).contains("Texte "))
    });
    // Déplacement dans la même liste, vers la racine, vers une branche de condition.
    round_trip(
        Command::Move {
            at: at(&s, "h1"),
            to: main.clone(),
            index: 2,
        },
        |_| {},
    );
    round_trip(
        Command::Move {
            at: at(&s, "h1"),
            to: Parent::root(page()),
            index: 2,
        },
        |s| assert_eq!(s.pages[0].body.len(), 3),
    );
    let cond = {
        let mut id = None;
        revel_model::ids::walk(&s.pages[0].body, &mut |n| {
            if let Node::If { id: i, .. } = n {
                id = i.clone();
            }
        });
        Target {
            owner: page(),
            node: id.unwrap(),
        }
    };
    round_trip(
        Command::Move {
            at: at(&s, "h1"),
            to: Parent::of(&cond, Branch::Otherwise),
            index: 0,
        },
        |_| {},
    );
    // Vers un autre propriétaire (composant).
    let carte = Parent::root(Owner::Component("carte".into()));
    round_trip(
        Command::Move {
            at: at(&s, "footer"),
            to: carte,
            index: 1,
        },
        |s| assert_eq!(s.components[0].body.len(), 2),
    );
    // Interdits : identifiant en double, déplacement dans soi-même, position hors liste.
    let mut t = sample();
    let dup = el("div", &[], vec![]);
    let mut dup = dup;
    if let Node::Element { id, .. } = &mut dup {
        *id = Some(id_of(&s, "h1"));
    }
    assert!(matches!(
        apply(
            &mut t,
            &Command::Insert {
                parent: main.clone(),
                index: 0,
                node: dup
            }
        ),
        Err(EditError::DuplicateId(_))
    ));
    assert!(matches!(
        apply(
            &mut t,
            &Command::Move {
                at: at(&s, "main"),
                to: Parent::of(&at(&s, "p"), Branch::Children),
                index: 0
            }
        ),
        Err(EditError::InvalidMove)
    ));
    let before = json(&t);
    assert!(matches!(
        apply(
            &mut t,
            &Command::Move {
                at: at(&s, "h1"),
                to: main,
                index: 99
            }
        ),
        Err(EditError::Index(99))
    ));
    assert_eq!(json(&t), before, "déplacement refusé : document inchangé");
}

#[test]
fn suite_atomique() {
    let s = sample();
    let mut t = sample();
    let before = json(&t);
    let err = apply(
        &mut t,
        &Command::Batch {
            commands: vec![
                Command::SetAttr {
                    at: at(&s, "h1"),
                    name: "id".into(),
                    value: Some("haut".into()),
                },
                Command::Remove {
                    at: Target {
                        owner: page(),
                        node: "inexistant".into(),
                    },
                },
            ],
        },
    );
    assert!(err.is_err());
    assert_eq!(json(&t), before, "suite échouée : rien n'est appliqué");
}

#[test]
fn dupliquer_et_envelopper() {
    let s = sample();
    let (cmd, copy) = make::duplicate(&s, &at(&s, "p")).unwrap();
    round_trip(cmd, |t| {
        assert_eq!(json(t).matches("Texte ").count(), 2);
        assert!(t.node_ids().contains(&copy));
        assert_eq!(t.node_ids().len(), s.node_ids().len() + 1);
    });
    let (cmd, wrapper) = make::wrap(&s, &at(&s, "h1"), "header").unwrap();
    round_trip(cmd, |t| {
        let w = address::node(
            t,
            &Target {
                owner: page(),
                node: wrapper.clone(),
            },
        )
        .unwrap();
        let Node::Element { tag, children, .. } = w else {
            panic!()
        };
        assert_eq!(tag, "header");
        assert_eq!(node_id(&children[0]), Some(id_of(&s, "h1").as_str()));
    });
    for p in make::Preset::ALL {
        let n = p.node(&s, "Texte");
        round_trip(
            Command::Insert {
                parent: Parent::root(page()),
                index: 0,
                node: n,
            },
            |_| {},
        );
    }
}

#[test]
fn intentions_developpees_et_retablies_a_l_identique() {
    let s0 = sample();
    let mut s = sample();
    let mut h = History::default();
    let dup = h
        .apply(&mut s, Command::Duplicate { at: at(&s0, "h1") }, false)
        .unwrap();
    let Command::Insert { node, .. } = &dup else {
        panic!("{dup:?}")
    };
    let copy = node_id(node).unwrap().to_string();
    let copy_at = Target {
        owner: page(),
        node: copy.clone(),
    };
    h.apply(
        &mut s,
        Command::Wrap {
            at: copy_at.clone(),
            tag: "header".into(),
        },
        false,
    )
    .unwrap();
    h.apply(
        &mut s,
        Command::SetAttr {
            at: copy_at.clone(),
            name: "id".into(),
            value: Some("haut".into()),
        },
        false,
    )
    .unwrap();
    h.apply(
        &mut s,
        Command::InsertPreset {
            parent: Parent::root(page()),
            index: 0,
            preset: make::Preset::Heading,
            text: "Titre".into(),
        },
        false,
    )
    .unwrap();
    let after = json(&s);
    for _ in 0..4 {
        h.undo(&mut s).unwrap();
    }
    assert_eq!(json(&s), json(&s0));
    for _ in 0..4 {
        h.redo(&mut s).unwrap();
    }
    assert_eq!(json(&s), after, "rétablir redonne les mêmes identifiants");
    assert!(matches!(
        expand(
            &s0,
            &Command::Wrap {
                at: at(&s0, "h1"),
                tag: "x y".into()
            }
        ),
        Err(EditError::InvalidTag(_))
    ));
}

#[test]
fn historique() {
    let mut s = sample();
    let before = json(&s);
    let mut h = History::default();
    let p = Parent::of(&at(&s, "h1"), Branch::Children);
    // Saisie continue : une seule étape.
    for t in ["B", "Bo", "Bon", "Bons"] {
        h.apply(
            &mut s,
            Command::SetText {
                parent: p.clone(),
                index: 0,
                text: t.into(),
            },
            true,
        )
        .unwrap();
    }
    h.seal();
    h.apply(
        &mut s,
        Command::Remove {
            at: at(&sample(), "footer"),
        },
        false,
    )
    .unwrap();
    let after = json(&s);
    assert!(h.undo(&mut s).unwrap().is_some());
    assert!(h.undo(&mut s).unwrap().is_some());
    assert_eq!(json(&s), before);
    assert!(h.undo(&mut s).unwrap().is_none());
    assert!(h.redo(&mut s).unwrap().is_some());
    assert!(h.redo(&mut s).unwrap().is_some());
    assert_eq!(json(&s), after);
    assert!(!h.can_redo());
    // Une nouvelle commande vide la pile « rétablir ».
    h.undo(&mut s).unwrap();
    h.apply(
        &mut s,
        Command::SetAttr {
            at: at(&sample(), "h1"),
            name: "id".into(),
            value: Some("x".into()),
        },
        false,
    )
    .unwrap();
    assert!(!h.can_redo());
}

#[test]
fn styles_regles_et_declarations() {
    let s0 = sample();
    let titre = rule_id(&s0, ".titre", None);
    // Valeur changée sur place (l'ordre des déclarations est conservé).
    round_trip(
        Command::SetDecl {
            rule: titre.clone(),
            prop: "color".into(),
            value: Some("rgb(1,  2, 3)".into()),
            important: false,
        },
        |s| {
            assert!(
                css(s).contains(".titre { color: rgb(1, 2, 3); margin: 0; }"),
                "{}",
                css(s)
            )
        },
    );
    // Propriété ajoutée à la fin, avec !important.
    round_trip(
        Command::SetDecl {
            rule: titre.clone(),
            prop: "padding".into(),
            value: Some("1rem".into()),
            important: true,
        },
        |s| assert!(css(s).contains("margin: 0; padding: 1rem !important; }")),
    );
    // Propriété retirée.
    round_trip(
        Command::SetDecl {
            rule: titre.clone(),
            prop: "color".into(),
            value: None,
            important: false,
        },
        |s| assert!(css(s).contains(".titre { margin: 0; }")),
    );
    round_trip(
        Command::SetSelector {
            rule: titre.clone(),
            selector: ".titre,  .sous-titre".into(),
        },
        |s| assert!(css(s).starts_with(".titre, .sous-titre {")),
    );
    round_trip(
        Command::RemoveRule {
            rule: titre.clone(),
        },
        |s| {
            assert!(!css(s).starts_with(".titre"));
        },
    );
    // Le commentaire et le bloc brut peuvent aussi être retirés et remis.
    round_trip(Command::RemoveStyleAt { index: 2 }, |s| {
        assert!(!css(s).contains("Mobile"))
    });
    round_trip(Command::RemoveStyleAt { index: 4 }, |s| {
        assert!(!css(s).contains("@keyframes"));
    });

    // Nouvelle règle de point de rupture : placée dans la cascade, entre la base et 860.
    let mut s = sample();
    let mut h = History::default();
    let applied = h
        .apply(
            &mut s,
            Command::AddRule {
                selector: ".titre".into(),
                media: Some("(max-width: 1200px)".into()),
                decls: vec![revel_model::Decl {
                    prop: "color".into(),
                    value: "green".into(),
                    important: false,
                }],
            },
            false,
        )
        .unwrap();
    let Command::InsertStyle { index, item } = &applied else {
        panic!("{applied:?}")
    };
    assert_eq!(
        *index, 2,
        "après les règles de base, avant le bloc 860 et son commentaire"
    );
    let revel_model::StyleItem::Rule(r) = item else {
        panic!()
    };
    assert!(r.id.is_some());
    assert!(
        css(&s).contains(
            ".page { display: grid; }\n@media (max-width: 1200px) {\n  .titre { color: green; }\n}\n@media (max-width: 860px) {\n  /* Mobile */"
        ),
        "{}",
        css(&s)
    );
    let after = json(&s);
    h.undo(&mut s).unwrap();
    assert_eq!(json(&s), json(&s0));
    h.redo(&mut s).unwrap();
    assert_eq!(json(&s), after, "même identifiant au rétablissement");

    // Valeurs refusées : site inchangé.
    let mut s = sample();
    let before = json(&s);
    for bad in [
        "red; display: none",
        "red }",
        "red !important",
        "",
        "red /* x",
        "a{b}",
    ] {
        let e = apply(
            &mut s,
            &Command::SetDecl {
                rule: titre.clone(),
                prop: "color".into(),
                value: Some(bad.into()),
                important: false,
            },
        );
        assert!(
            matches!(e, Err(EditError::InvalidCss(_))),
            "{bad:?} : {e:?}"
        );
    }
    for bad in ["co lor", "color:", "a;b", ""] {
        let e = apply(
            &mut s,
            &Command::SetDecl {
                rule: titre.clone(),
                prop: bad.into(),
                value: Some("red".into()),
                important: false,
            },
        );
        assert!(matches!(e, Err(EditError::InvalidCss(_))), "{bad:?}");
    }
    for bad in [".a { x: 1 } .b", "@media x", ".a {", ""] {
        let e = apply(
            &mut s,
            &Command::SetSelector {
                rule: titre.clone(),
                selector: bad.into(),
            },
        );
        assert!(e.is_err(), "{bad:?}");
    }
    assert!(matches!(
        apply(
            &mut s,
            &Command::AddRule {
                selector: ".a".into(),
                media: Some("screen { .x".into()),
                decls: vec![],
            }
        ),
        Err(EditError::InvalidCss(_))
    ));
    // Identifiant de règle déjà pris par un nœud.
    let h1 = id_of(&s, "h1");
    let e = apply(
        &mut s,
        &Command::InsertStyle {
            index: 0,
            item: revel_model::StyleItem::Rule(revel_model::StyleRule {
                id: Some(h1),
                selector: ".x".into(),
                media: None,
                decls: vec![],
            }),
        },
    );
    assert!(matches!(e, Err(EditError::DuplicateId(_))));
    assert_eq!(json(&s), before);

    // Sans feuille de style : créée à la première règle.
    let mut s = sample();
    s.stylesheet = None;
    apply(
        &mut s,
        &Command::AddRule {
            selector: ".a".into(),
            media: None,
            decls: vec![],
        },
    )
    .unwrap();
    let sheet = s.stylesheet.as_ref().unwrap();
    assert_eq!(sheet.url, crate::style::DEFAULT_URL);
    assert_eq!(sheet.items.len(), 1);
}

#[test]
fn styles_saisie_continue_fusionnee() {
    let mut s = sample();
    let before = json(&s);
    let titre = rule_id(&s, ".titre", None);
    let mut h = History::default();
    for v in ["1", "12", "12p", "12px"] {
        h.apply(
            &mut s,
            Command::SetDecl {
                rule: titre.clone(),
                prop: "font-size".into(),
                value: Some(v.into()),
                important: false,
            },
            true,
        )
        .unwrap();
    }
    h.seal();
    assert!(css(&s).contains("font-size: 12px"));
    h.undo(&mut s).unwrap();
    assert_eq!(json(&s), before, "une seule étape pour la saisie");
    assert!(!h.can_undo());
    h.redo(&mut s).unwrap();
    assert!(css(&s).contains("margin: 0; font-size: 12px; }"));
}

#[test]
fn points_de_rupture() {
    use revel_model::Breakpoint;
    let bp = |id: &str, max: Option<u32>, min: Option<u32>| Breakpoint {
        id: id.into(),
        max_width: max,
        min_width: min,
    };
    let list = vec![
        bp("desktop", None, None),
        bp("max-991", Some(991), None),
        bp("max-478", Some(478), None),
        bp("min-1440", None, Some(1440)),
    ];
    round_trip(
        Command::SetBreakpoints {
            breakpoints: list.clone(),
        },
        |s| assert_eq!(s.breakpoints, list),
    );
    let mut s = sample();
    for bad in [
        vec![bp("max-1", Some(1), None)],
        vec![
            bp("desktop", None, None),
            bp("a", Some(600), None),
            bp("b", Some(600), None),
        ],
        vec![bp("desktop", None, None), bp("a", Some(600), Some(300))],
        vec![bp("desktop", None, None), bp("a", None, None)],
        vec![bp("desktop", None, None), bp("desktop", Some(5), None)],
    ] {
        assert!(matches!(
            apply(&mut s, &Command::SetBreakpoints { breakpoints: bad }),
            Err(EditError::InvalidBreakpoints(_))
        ));
    }
}

#[test]
fn point_de_rupture_redimensionne() {
    use revel_model::Breakpoint;
    let bp = |id: &str, max: Option<u32>| Breakpoint {
        id: id.into(),
        max_width: max,
        min_width: None,
    };
    let with_bps = |s: &mut Site| {
        s.breakpoints = vec![
            bp("desktop", None),
            bp("max-860", Some(860)),
            bp("max-478", Some(478)),
        ];
    };
    let mut s = sample();
    with_bps(&mut s);
    let inv = apply(
        &mut s,
        &Command::ResizeBreakpoint {
            id: "max-860".into(),
            width: 900,
        },
    )
    .unwrap();
    assert_eq!(s.breakpoints[1].max_width, Some(900));
    // Règles et commentaire du bloc `@media` suivent.
    assert!(css(&s).contains("@media (max-width: 900px)"), "{}", css(&s));
    assert!(!css(&s).contains("860px"));
    apply(&mut s, &inv).unwrap();
    assert!(css(&s).contains("@media (max-width: 860px)"));
    // Ordre de la cascade gardé : pas au-delà d'un voisin, ni sur sa largeur.
    for w in [478, 400, 50] {
        assert!(
            apply(
                &mut s,
                &Command::ResizeBreakpoint {
                    id: "max-860".into(),
                    width: w,
                },
            )
            .is_err(),
            "{w}"
        );
    }
    assert!(
        apply(
            &mut s,
            &Command::ResizeBreakpoint {
                id: "desktop".into(),
                width: 1200,
            },
        )
        .is_err()
    );
}

/// Générateur pseudo-aléatoire reproductible (xorshift).
struct Rng(u64);
impl Rng {
    fn next(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n.max(1) as u64) as usize
    }
}

/// Suites aléatoires de commandes : tout annuler rend le document d'origine ; tout rétablir rend
/// le document modifié.
#[test]
fn suites_aleatoires_reversibles() {
    let mut styled = 0;
    for seed in 1..=40u64 {
        let mut s = sample();
        let origin = json(&s);
        let mut h = History::default();
        let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let mut applied = 0;
        for _ in 0..30 {
            let ids: Vec<String> = {
                let mut v: Vec<String> = Vec::new();
                revel_model::ids::walk(&s.pages[0].body, &mut |n| {
                    if let Node::Element { id: Some(i), .. } = n {
                        v.push(i.clone());
                    }
                });
                v
            };
            if ids.is_empty() {
                break;
            }
            let pick = |r: &mut Rng| Target {
                owner: page(),
                node: ids[r.next(ids.len())].clone(),
            };
            let rules: Vec<String> = s.style_rules().filter_map(|r| r.id.clone()).collect();
            let rule = |r: &mut Rng| rules.get(r.next(rules.len())).cloned().unwrap_or_default();
            let cmd = match rng.next(10) {
                6 => Command::SetDecl {
                    rule: rule(&mut rng),
                    prop: ["color", "margin", "padding"][rng.next(3)].into(),
                    value: (rng.next(4) > 0).then(|| format!("{}px", rng.next(50))),
                    important: rng.next(5) == 0,
                },
                7 => Command::AddRule {
                    selector: [".titre", ".page", ".neuve"][rng.next(3)].into(),
                    media: [
                        None,
                        Some("(max-width: 600px)"),
                        Some("(max-width: 1200px)"),
                    ][rng.next(3)]
                    .map(str::to_string),
                    decls: vec![],
                },
                8 => Command::RemoveRule {
                    rule: rule(&mut rng),
                },
                9 => Command::SetSelector {
                    rule: rule(&mut rng),
                    selector: format!(".s{}", rng.next(9)),
                },
                0 => Command::SetAttr {
                    at: pick(&mut rng),
                    name: ["class", "id", "title"][rng.next(3)].into(),
                    value: Some(format!("v{}", rng.next(100))),
                },
                1 => {
                    let t = pick(&mut rng);
                    let mut n = el("div", &[], vec![text("x")]);
                    let mut taken = s.node_ids();
                    make::refresh_ids(&mut n, &mut taken);
                    Command::Insert {
                        parent: Parent::of(&t, Branch::Children),
                        index: 0,
                        node: n,
                    }
                }
                2 => Command::Remove { at: pick(&mut rng) },
                3 => Command::Move {
                    at: pick(&mut rng),
                    to: Parent::of(&pick(&mut rng), Branch::Children),
                    index: 0,
                },
                4 => make::duplicate(&s, &pick(&mut rng)).unwrap().0,
                _ => make::wrap(&s, &pick(&mut rng), "div").unwrap().0,
            };
            let is_style = matches!(
                cmd,
                Command::SetDecl { .. }
                    | Command::AddRule { .. }
                    | Command::RemoveRule { .. }
                    | Command::SetSelector { .. }
            );
            if h.apply(&mut s, cmd, false).is_ok() {
                applied += 1;
                styled += usize::from(is_style);
            }
        }
        let modified = json(&s);
        while h.undo(&mut s).unwrap().is_some() {}
        assert_eq!(
            json(&s),
            origin,
            "graine {seed} : {applied} commandes annulées"
        );
        while h.redo(&mut s).unwrap().is_some() {}
        assert_eq!(json(&s), modified, "graine {seed} : rétablissement");
    }
    assert!(styled > 200, "commandes de style appliquées : {styled}");
}

#[test]
fn rangement_des_medias_et_composants() {
    let mut s = sample();
    let before = serde_json::to_string(&s).unwrap();
    let shelf = revel_model::Shelf {
        folders: vec!["logos".into(), "logos/clients".into()],
        items: [("/media/a.png".to_string(), "logos/clients".to_string())].into(),
        order: Vec::new(),
    };
    let inv = apply(
        &mut s,
        &Command::SetShelf {
            kind: ShelfKind::Media,
            shelf: shelf.clone(),
        },
    )
    .unwrap();
    assert_eq!(s.library.media, shelf);
    assert!(s.library.components.is_empty());
    assert!(serde_json::to_string(&s).unwrap().contains("\"library\""));
    apply(&mut s, &inv).unwrap();
    // Rangement vide : rien n'est écrit dans le document.
    assert_eq!(serde_json::to_string(&s).unwrap(), before);
}

#[test]
fn gabarit_copie_puis_retire() {
    let mut s = sample();
    s.layouts.push(revel_model::Layout {
        fields: Vec::new(),
        id: "layouts/base".into(),
        parent: None,
        props: Default::default(),
        body: vec![el("main", &[], vec![Node::Slot])],
    });
    s.layouts.push(revel_model::Layout {
        fields: Vec::new(),
        id: "layouts/projet".into(),
        parent: Some("layouts/base".into()),
        props: Default::default(),
        body: vec![el("article", &[("class", "projet")], vec![Node::Slot])],
    });
    s.ensure_ids();
    let before = s.clone();
    let id = make::own_layout_id(&s, "layouts/projet", "wattr");
    assert_eq!(id, "layouts/projet--wattr");
    let copy = make::layout_copy(&s, "layouts/projet", &id).unwrap();
    assert_eq!(copy.parent.as_deref(), Some("layouts/base"));
    let src_id = node_id(&s.layouts[1].body[0]).unwrap().to_string();
    assert_ne!(
        node_id(&copy.body[0]).unwrap(),
        src_id,
        "nouveaux identifiants"
    );
    let inv = apply(
        &mut s,
        &Command::AddLayout {
            layout: copy.clone(),
            index: None,
        },
    )
    .unwrap();
    assert_eq!(
        make::own_layout_id(&s, "layouts/projet", "wattr"),
        "layouts/projet--wattr-2"
    );
    // Même identifiant : refusé.
    assert!(
        apply(
            &mut s,
            &Command::AddLayout {
                layout: copy,
                index: None
            }
        )
        .is_err()
    );
    // Gabarit parent d'un autre : retrait refusé.
    assert!(matches!(
        apply(
            &mut s,
            &Command::RemoveLayout {
                id: "layouts/base".into()
            }
        ),
        Err(EditError::WrongKind(_))
    ));
    apply(&mut s, &inv).unwrap();
    assert_eq!(s, before);
}

#[test]
fn nouvelle_page_et_copie() {
    let mut s = sample();
    let p = make::new_page(&s, "Nos offres d'été", "");
    assert_eq!(p.path, "/nos-offres-d-ete/");
    assert_eq!(p.name.as_deref(), Some("Nos offres d'été"));
    apply(
        &mut s,
        &Command::AddPage {
            page: p.clone(),
            index: None,
        },
    )
    .unwrap();
    let p2 = make::new_page(&s, "Nos offres d'été", "emplois/2026");
    assert_eq!(p2.path, "/emplois/2026/nos-offres-d-ete/");
    let p3 = make::new_page(&s, "Nos offres d'été", "");
    assert_eq!(p3.path, "/nos-offres-d-ete-2/");
    assert_ne!(p3.id, p.id);
    let c = make::duplicate_page(&s, &p.id, "copie").unwrap();
    assert_eq!(c.path, "/nos-offres-d-ete-copie/");
    assert_eq!(c.name.as_deref(), Some("Nos offres d'été (copie)"));
    apply(
        &mut s,
        &Command::AddPage {
            page: c,
            index: None,
        },
    )
    .unwrap();
    let home = make::duplicate_page(&s, "accueil", "copie").unwrap();
    assert_eq!(home.path, "/accueil-copie/");
    assert!(revel_model::valid_page_path("/a/b-c_d.e/"));
    assert!(
        !revel_model::valid_page_path("/A/")
            && !revel_model::valid_page_path("a/")
            && !revel_model::valid_page_path("/a//b/")
    );
    assert_eq!(
        revel_model::slugify("  Été 2026 : l'œuvre !"),
        "ete-2026-l-oeuvre"
    );
    // Site sans gabarit : gabarit de base (document, feuille de style, emplacement), page sans
    // en-tête de document.
    let mut bare = revel_model::Site::new("Atelier \"A\"", "fr");
    let l = make::base_layout(&bare);
    assert_eq!(l.id, "base");
    let text = serde_json::to_string(&l.body).unwrap();
    assert!(
        text.contains("\"tag\":\"head\"")
            && text.contains("/assets/site.css")
            && text.contains("\"node\":\"slot\""),
        "{text}"
    );
    apply(
        &mut bare,
        &Command::AddLayout {
            layout: l,
            index: None,
        },
    )
    .unwrap();
    let p = make::new_page(&bare, "Accueil", "");
    assert_eq!(p.layout.as_deref(), Some("base"));
    assert!(
        !serde_json::to_string(&p.body)
            .unwrap()
            .contains("\"tag\":\"head\"")
    );
    assert_eq!(make::base_layout(&bare).id, "base-2");
}

#[test]
fn composants_creer_proprietes_detacher() {
    let mut s = sample();
    let before = json(&s);
    // Créer un composant à partir du titre : une utilisation prend sa place.
    let h1 = at(&s, "h1");
    let id = make::free_component_id(&s, "Grand titre");
    assert_eq!(id, "grand-titre");
    let inv = apply(
        &mut s,
        &Command::MakeComponent {
            at: h1.clone(),
            id: id.clone(),
            name: Some("Grand titre".into()),
        },
    )
    .unwrap();
    let c = s.component(&id).unwrap().clone();
    assert_eq!(c.label(), "Grand titre");
    assert_eq!(node_id(&c.body[0]), Some(h1.node.as_str()));
    let Node::Element { children, .. } = &s.pages[0].body[0] else {
        panic!()
    };
    let Node::Use {
        id: Some(_),
        component,
        ..
    } = children[0].clone()
    else {
        panic!("{:?}", children[0])
    };
    assert_eq!(component, id);
    assert_eq!(make::uses_of(&s, &id), 1);
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), before, "annulation");
    apply(
        &mut s,
        &Command::MakeComponent {
            at: h1.clone(),
            id: id.clone(),
            name: None,
        },
    )
    .unwrap();
    // Propriété « titre » : le texte du composant la lit ; valeur par défaut, puis valeur de
    // l'utilisation.
    let comp = Owner::Component(id.clone());
    let h1_in = Target {
        owner: comp.clone(),
        node: h1.node.clone(),
    };
    apply(
        &mut s,
        &Command::SetComponentProps {
            id: id.clone(),
            props: vec![
                revel_model::PropDef {
                    name: "titre".into(),
                    label: Some("Titre".into()),
                    kind: revel_model::PropKind::Text,
                    default: Some(serde_json::json!("Bonjour")),
                },
                revel_model::PropDef {
                    name: "visible".into(),
                    label: None,
                    kind: revel_model::PropKind::Switch,
                    default: Some(serde_json::json!(true)),
                },
            ],
        },
    )
    .unwrap();
    assert!(matches!(
        apply(
            &mut s,
            &Command::SetComponentProps {
                id: id.clone(),
                props: vec![revel_model::PropDef {
                    name: "page".into(),
                    label: None,
                    kind: revel_model::PropKind::Text,
                    default: None,
                }],
            },
        ),
        Err(EditError::InvalidProp(_))
    ));
    apply(
        &mut s,
        &Command::Replace {
            parent: Parent::of(&h1_in, Branch::Children),
            index: 0,
            node: out("titre"),
        },
    )
    .unwrap();
    // Nouvelle utilisation (nouvel identifiant après l'annulation).
    let Node::Element { children, .. } = &s.pages[0].body[0] else {
        panic!()
    };
    let use_id = revel_model::ids::node_id(&children[0]).unwrap().to_string();
    assert_ne!(use_id, h1.node);
    let use_at = Target {
        owner: page(),
        node: use_id.clone(),
    };
    let inv_arg = apply(
        &mut s,
        &Command::SetUseArg {
            at: use_at.clone(),
            name: "titre".into(),
            value: Some("\"Salut\"".into()),
        },
    )
    .unwrap();
    assert_eq!(
        inv_arg,
        Command::SetUseArg {
            at: use_at.clone(),
            name: "titre".into(),
            value: None
        }
    );
    // Retrait refusé tant que le composant est utilisé ; placé dans lui-même : refusé.
    assert!(matches!(
        apply(&mut s, &Command::RemoveComponent { id: id.clone() }),
        Err(EditError::InUse(_))
    ));
    assert!(matches!(
        apply(
            &mut s,
            &Command::Insert {
                parent: Parent::of(&h1_in, Branch::Children),
                index: 0,
                node: Node::Use {
                    id: None,
                    component: id.clone(),
                    args: Default::default(),
                    variant: None,
                    attrs: Vec::new(),
                },
            },
        ),
        Err(EditError::SelfUse(_))
    ));
    // Détacher : copie du contenu, valeur de la propriété écrite à la place.
    let doc = json(&s);
    let inv_d = apply(&mut s, &Command::Detach { at: use_at.clone() }).unwrap();
    let Node::Element { children, .. } = &s.pages[0].body[0] else {
        panic!()
    };
    let Node::Element {
        tag,
        id: copy_id,
        children: inner,
        ..
    } = &children[0]
    else {
        panic!("{:?}", children[0])
    };
    assert_eq!(tag, "h1");
    assert_ne!(copy_id.as_deref(), Some(h1.node.as_str()));
    assert_eq!(inner, &vec![text("Salut")]);
    assert_eq!(make::uses_of(&s, &id), 0);
    apply(&mut s, &inv_d).unwrap();
    assert_eq!(json(&s), doc, "annulation du détachement");
}

#[test]
fn detacher_condition_et_expression() {
    let mut s = sample();
    let body = vec![Node::If {
        id: None,
        cond: "visible".into(),
        then: vec![el(
            "a",
            &[("href", "{{ lien }}")],
            vec![out("titre | upper")],
        )],
        otherwise: vec![],
    }];
    let mut c = Component::new("bouton", body);
    c.props = vec![
        revel_model::PropDef {
            name: "visible".into(),
            label: None,
            kind: revel_model::PropKind::Switch,
            default: Some(serde_json::json!(true)),
        },
        revel_model::PropDef {
            name: "lien".into(),
            label: None,
            kind: revel_model::PropKind::Link,
            default: None,
        },
        revel_model::PropDef {
            name: "titre".into(),
            label: None,
            kind: revel_model::PropKind::Text,
            default: Some(serde_json::json!("Voir")),
        },
    ];
    s.components.push(c);
    s.pages[0].body.push(Node::Use {
        id: Some("u1".into()),
        component: "bouton".into(),
        args: [("lien".to_string(), "p.url".to_string())].into(),
        variant: None,
        attrs: Vec::new(),
    });
    s.ensure_ids();
    let at = Target {
        owner: page(),
        node: "u1".into(),
    };
    apply(&mut s, &Command::Detach { at }).unwrap();
    let n = s.pages[0].body.len();
    // `titre` lu dans une expression plus complexe : affectation gardée avant la copie.
    assert_eq!(
        s.pages[0].body[n - 2],
        Node::Set {
            name: "titre".into(),
            expr: "\"Voir\"".into()
        }
    );
    let Node::Element { attrs, .. } = &s.pages[0].body[n - 1] else {
        panic!("{:?}", s.pages[0].body[n - 1])
    };
    assert_eq!(attrs[0].value.as_deref(), Some("{{ p.url }}"));
}

#[test]
fn variantes_et_attributs_d_une_utilisation() {
    let mut s = sample();
    s.components.push(Component::new(
        "tuile",
        vec![el("div", &[("class", "tuile")], vec![text("Titre")])],
    ));
    s.pages[0].body.push(Node::Use {
        id: Some("u1".into()),
        component: "tuile".into(),
        args: Default::default(),
        variant: None,
        attrs: Vec::new(),
    });
    s.ensure_ids();
    let doc = json(&s);
    let use_at = Target {
        owner: page(),
        node: "u1".into(),
    };
    // Variantes : identifiant vérifié ; remplacement annulable.
    let bad = Command::SetComponentVariants {
        id: "tuile".into(),
        variants: vec![revel_model::Variant {
            id: "Mauvais Nom".into(),
            name: None,
            kind: revel_model::VariantKind::Custom,
        }],
    };
    assert!(apply(&mut s, &bad).is_err());
    let set = Command::SetComponentVariants {
        id: "tuile".into(),
        variants: vec![revel_model::Variant {
            id: "sombre".into(),
            name: Some("Sombre".into()),
            kind: revel_model::VariantKind::Custom,
        }],
    };
    let inv1 = apply(&mut s, &set).unwrap();
    let inv2 = apply(
        &mut s,
        &Command::SetUseVariant {
            at: use_at.clone(),
            variant: Some("sombre".into()),
        },
    )
    .unwrap();
    // Animation posée sur l'utilisation (élément racine du composant), sans l'envelopper.
    let inv3 = apply(
        &mut s,
        &Command::SetAttr {
            at: use_at.clone(),
            name: "data-revel-appear".into(),
            value: Some("up".into()),
        },
    )
    .unwrap();
    let Some(Node::Use { variant, attrs, .. }) = s.pages[0].body.last() else {
        panic!()
    };
    assert_eq!(variant.as_deref(), Some("sombre"));
    assert_eq!(attrs.len(), 1);
    // Détacher garde la variante et l'animation sur la copie.
    let mut d = s.clone();
    apply(&mut d, &Command::Detach { at: use_at }).unwrap();
    let Some(Node::Element { attrs, .. }) = d.pages[0].body.last() else {
        panic!()
    };
    assert_eq!(
        attrs[0].value.as_deref(),
        Some("tuile is-sombre"),
        "{attrs:?}"
    );
    assert_eq!(attrs[1].name, "data-revel-appear");
    for inv in [inv3, inv2, inv1] {
        apply(&mut s, &inv).unwrap();
    }
    assert_eq!(json(&s), doc);
}

#[test]
fn elements_avec_leurs_styles() {
    let mut s = sample();
    let before = json(&s);
    let parent = Parent::of(&at(&s, "main"), Branch::Children);
    let inv = apply(
        &mut s,
        &Command::InsertPreset {
            parent: parent.clone(),
            index: 0,
            preset: make::Preset::Grid,
            text: String::new(),
        },
    )
    .unwrap();
    assert!(
        css(&s).contains(".grille { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 24px; }"),
        "{}",
        css(&s)
    );
    assert!(css(&s).contains(".grille__cellule { min-height: 80px; }"));
    // Deuxième grille : règles déjà là, pas de doublon.
    let n = s.style_rules().count();
    apply(
        &mut s,
        &Command::InsertPreset {
            parent: parent.clone(),
            index: 0,
            preset: make::Preset::Grid,
            text: String::new(),
        },
    )
    .unwrap();
    assert_eq!(s.style_rules().count(), n);
    let _ = inv;
    // Annulation de la première insertion sur un site neuf : identique.
    let mut s3 = sample();
    let main3 = Parent::of(&at(&s3, "main"), Branch::Children);
    let inv3 = apply(
        &mut s3,
        &Command::InsertPreset {
            parent: main3,
            index: 0,
            preset: make::Preset::Icon,
            text: String::new(),
        },
    )
    .unwrap();
    let Node::Element { children, .. } = &s3.pages[0].body[0] else {
        panic!()
    };
    let Node::Element {
        tag,
        children: inner,
        ..
    } = &children[0]
    else {
        panic!()
    };
    assert_eq!(tag, "svg");
    assert!(matches!(&inner[0], Node::Html { html } if html.contains("<path")));
    apply(&mut s3, &inv3).unwrap();
    assert_eq!(json(&s3), before);
    // Toutes les icônes sont dessinées.
    assert!(icons::ICONS.iter().all(|(_, p)| p.starts_with('<')));
    assert!(icons::ICONS.len() >= 30);
}

#[test]
fn blocs_prets_a_l_emploi() {
    let mut s = sample();
    let before = json(&s);
    let parent = Parent::of(&at(&s, "main"), Branch::Children);
    let mut inverses = Vec::new();
    for p in make::Preset::ALL
        .into_iter()
        .filter(|p| p.block().is_some())
    {
        let inv = apply(
            &mut s,
            &Command::InsertPreset {
                parent: parent.clone(),
                index: 0,
                preset: p,
                text: String::new(),
            },
        )
        .unwrap_or_else(|e| panic!("{p:?} : {e}"));
        inverses.push(inv);
    }
    let c = css(&s);
    for sel in [
        ".bloc {",
        ".bloc-bouton--secondaire {",
        ".bloc-avantage .icone {",
        ".bloc-faq details p {",
        ".bloc-contact__liste li {",
    ] {
        assert!(c.contains(sel), "{sel}\n{c}");
    }
    assert_eq!(
        c.matches(".bloc {").count(),
        1,
        "règles communes une seule fois"
    );
    let txt = json(&s);
    assert!(txt.contains("Questions fréquentes") && txt.contains("\"data-icon\""));
    for inv in inverses.iter().rev() {
        apply(&mut s, inv).unwrap();
    }
    assert_eq!(json(&s), before);
    // Site en anglais : textes en anglais.
    let mut en = Site::new("Test", "en");
    en.pages.push(s.pages[0].clone());
    let n = make::Preset::BlockFaq.node(&en, "");
    assert!(
        serde_json::to_string(&n)
            .unwrap()
            .contains("Frequently asked questions")
    );
}

#[test]
fn langues_et_traductions() {
    // Langue ajoutée, réglée, retirée : chaque inverse rend le document identique.
    round_trip(
        Command::AddLocale {
            locale: revel_model::Locale::new("en"),
            index: None,
        },
        |s| assert_eq!(s.locales[0].code, "en"),
    );
    let mut s = sample();
    for bad in ["fr", "EN", "e", "en_US", "english"] {
        assert!(
            matches!(
                apply(
                    &mut s,
                    &Command::AddLocale {
                        locale: revel_model::Locale::new(bad),
                        index: None,
                    }
                ),
                Err(EditError::InvalidLocale(_))
            ),
            "{bad}"
        );
    }
    apply(
        &mut s,
        &Command::AddLocale {
            locale: revel_model::Locale::new("en"),
            index: None,
        },
    )
    .unwrap();
    assert!(
        apply(
            &mut s,
            &Command::AddLocale {
                locale: revel_model::Locale::new("en"),
                index: None,
            }
        )
        .is_err()
    );
    // Adresses de la langue réservées.
    assert!(matches!(
        apply(
            &mut s,
            &Command::SetPagePath {
                id: "accueil".into(),
                path: "/en/x/".into()
            }
        ),
        Err(EditError::PathTaken(_))
    ));
    let before = json(&s);
    let inv = apply(
        &mut s,
        &Command::SetLocale {
            code: "en".into(),
            name: Some(" Anglais ".into()),
            draft: true,
        },
    )
    .unwrap();
    assert_eq!(s.locales[0].label(), "Anglais");
    assert!(s.locales[0].draft);
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), before);
    assert_eq!(s.locales[0].label(), "English");

    // Traduction depuis le canevas : texte écrit, clé = texte de la langue principale regroupé.
    let h1 = at(&s, "h1");
    let t = expand(
        &s,
        &Command::Translate {
            locale: "en".into(),
            place: crate::locale::TextPlace::Text {
                parent: Parent::of(&h1, Branch::Children),
                index: 0,
            },
            text: " Hello ".into(),
        },
    )
    .unwrap();
    assert_eq!(
        t,
        Command::SetTranslation {
            locale: "en".into(),
            source: "Bonjour".into(),
            text: Some(" Hello ".into()),
            machine: false,
        }
    );
    let inv = apply(&mut s, &t).unwrap();
    assert_eq!(
        s.locales[0].texts.get("Bonjour").map(String::as_str),
        Some("Hello")
    );
    let texts = crate::locale::texts(&s);
    assert!(
        texts
            .iter()
            .any(|x| x.source == "Bonjour" && x.places == vec![page()])
    );
    assert!(texts.iter().any(|x| x.source == "Carte"));
    assert!(
        !texts
            .iter()
            .any(|x| x.source.trim() == "fin" && x.source != "fin")
    );
    // Site vu en anglais : même structure, texte traduit.
    let en = crate::locale::translate_site(&s, &s.locales[0].texts);
    assert!(json(&en).contains("\"Hello\""));
    assert_eq!(
        revel_model::ids::node_id(&en.pages[0].body[0]),
        revel_model::ids::node_id(&s.pages[0].body[0])
    );
    apply(&mut s, &inv).unwrap();
    assert!(s.locales[0].texts.is_empty());
    // Langue inconnue ; endroit sans texte.
    assert!(
        expand(
            &s,
            &Command::Translate {
                locale: "de".into(),
                place: crate::locale::TextPlace::Text {
                    parent: Parent::of(&h1, Branch::Children),
                    index: 0,
                },
                text: "Hallo".into(),
            }
        )
        .is_err()
    );
    // Retrait : la langue revient à sa place avec ses textes.
    apply(
        &mut s,
        &Command::SetTranslation {
            locale: "en".into(),
            source: "Carte".into(),
            text: Some("Card".into()),
            machine: false,
        },
    )
    .unwrap();
    let before = json(&s);
    let inv = apply(&mut s, &Command::RemoveLocale { code: "en".into() }).unwrap();
    assert!(s.locales.is_empty());
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), before);
}

#[test]
fn traduction_automatique_a_relire() {
    let mut s = sample();
    apply(
        &mut s,
        &Command::AddLocale {
            locale: revel_model::Locale::new("en"),
            index: None,
        },
    )
    .unwrap();
    let before = json(&s);
    let auto = |text: &str, machine: bool| Command::SetTranslation {
        locale: "en".into(),
        source: "Bonjour".into(),
        text: Some(text.into()),
        machine,
    };
    // Traduction automatique : marquée ; annuler retire texte et marque.
    let inv = apply(&mut s, &auto("Hello", true)).unwrap();
    assert!(s.locales[0].machine.contains("text:Bonjour"));
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), before);
    // Saisie après une traduction automatique : marque retirée, rétablie à l'annulation.
    apply(&mut s, &auto("Hello", true)).unwrap();
    let marked = json(&s);
    let inv = apply(&mut s, &auto("Hi", false)).unwrap();
    assert!(s.locales[0].machine.is_empty());
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), marked);
    // Marques posées et retirées par lot ; annuler ne touche que celles qui ont changé.
    let inv = apply(
        &mut s,
        &Command::MarkMachine {
            locale: "en".into(),
            keys: vec!["text:Bonjour".into(), "data:site.title".into()],
            on: false,
        },
    )
    .unwrap();
    assert!(s.locales[0].machine.is_empty());
    assert_eq!(
        inv,
        Command::MarkMachine {
            locale: "en".into(),
            keys: vec!["text:Bonjour".into()],
            on: true,
        }
    );
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), marked);
    // Mots gardés : espaces retirés, doublons et vides ignorés.
    let inv = apply(
        &mut s,
        &Command::SetNoTranslate {
            terms: vec![
                " Alunites ".into(),
                "".into(),
                "Alunites".into(),
                "Wattr".into(),
            ],
        },
    )
    .unwrap();
    assert_eq!(s.settings.no_translate, vec!["Alunites", "Wattr"]);
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), marked);
}

#[test]
fn historique_par_groupe() {
    let mut s = sample();
    apply(
        &mut s,
        &Command::AddLocale {
            locale: revel_model::Locale::new("en"),
            index: None,
        },
    )
    .unwrap();
    let before = json(&s);
    let mut h = crate::History::default();
    let tr = |source: &str, text: &str| Command::SetTranslation {
        locale: "en".into(),
        source: source.into(),
        text: Some(text.into()),
        machine: true,
    };
    let mut ex = |site: &mut revel_model::Site, c: &Command| apply(site, c);
    h.apply_grouped(&mut s, tr("Bonjour", "Hello"), "auto", &mut ex)
        .unwrap();
    h.apply_grouped(&mut s, tr("Carte", "Card"), "auto", &mut ex)
        .unwrap();
    assert_eq!(s.locales[0].texts.len(), 2);
    // Un seul retour en arrière pour tout le groupe ; rétablir refait tout.
    h.undo_with(&mut s, &mut ex).unwrap();
    assert_eq!(json(&s), before);
    assert!(!h.can_undo());
    h.redo_with(&mut s, &mut ex).unwrap();
    assert_eq!(s.locales[0].texts.len(), 2);
    // Autre commande entre deux lots : nouveau groupe, nouvelle étape.
    h.apply_with(&mut s, tr("Carte", "Map"), false, &mut ex)
        .unwrap();
    h.apply_grouped(&mut s, tr("Bonjour", "Hi"), "auto", &mut ex)
        .unwrap();
    h.undo_with(&mut s, &mut ex).unwrap();
    assert_eq!(
        s.locales[0].texts.get("Bonjour").map(String::as_str),
        Some("Hello")
    );
    assert_eq!(
        s.locales[0].texts.get("Carte").map(String::as_str),
        Some("Map")
    );
}

#[test]
fn reglages_du_site() {
    let mut s = sample();
    s.locales.push(revel_model::Locale::new("en"));
    let before = json(&s);
    let info = |name: &str, url: Option<&str>, lang: &str| Command::SetSiteInfo {
        name: name.into(),
        base_url: url.map(str::to_string),
        default_locale: lang.into(),
        typography: Some("fr".into()),
    };
    let inv = apply(
        &mut s,
        &info(" Alunites ", Some(" https://alunites.com/ "), "fr"),
    )
    .unwrap();
    assert_eq!(s.name, "Alunites");
    assert_eq!(s.base_url.as_deref(), Some("https://alunites.com"));
    assert_eq!(s.settings.typography.as_deref(), Some("fr"));
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), before);
    // Refusés : nom vide, adresse sans http(s), langue déjà utilisée par une autre langue.
    assert!(apply(&mut s, &info(" ", None, "fr")).is_err());
    assert!(apply(&mut s, &info("A", Some("alunites.com"), "fr")).is_err());
    assert!(apply(&mut s, &info("A", None, "en")).is_err());
    assert!(apply(&mut s, &info("A", None, "x y")).is_err());
    assert_eq!(json(&s), before);
    // Langue principale changée ; adresse retirée (vide).
    let inv = apply(&mut s, &info("A", Some(""), "de")).unwrap();
    assert_eq!(
        (s.default_locale.as_str(), s.base_url.as_deref()),
        ("de", None)
    );
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), before);
    // Code ajouté au site et à une page.
    let code = |head: &str| revel_model::CustomCode {
        head: head.into(),
        body: String::new(),
    };
    let inv = apply(
        &mut s,
        &Command::SetCode {
            page: None,
            code: code("<meta name=\"a\">\n"),
        },
    )
    .unwrap();
    assert_eq!(s.settings.code.head, "<meta name=\"a\">");
    apply(&mut s, &inv).unwrap();
    let inv = apply(
        &mut s,
        &Command::SetCode {
            page: Some("accueil".into()),
            code: code("<style></style>"),
        },
    )
    .unwrap();
    assert_eq!(s.pages[0].code.head, "<style></style>");
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), before);
    assert!(
        apply(
            &mut s,
            &Command::SetCode {
                page: Some("absente".into()),
                code: code("x"),
            }
        )
        .is_err()
    );
}

#[test]
fn selecteur_de_langue() {
    let mut s = sample();
    let main = at(&s, "main");
    let cmd = expand(
        &s,
        &Command::InsertPreset {
            parent: Parent::of(&main, Branch::Children),
            index: 0,
            preset: crate::make::Preset::LocaleSwitch,
            text: "Langues".into(),
        },
    )
    .unwrap();
    apply(&mut s, &cmd).unwrap();
    let c = css(&s);
    assert!(c.contains(".langue[aria-current=\"page\"]"), "{c}");
    assert!(json(&s).contains("\"locales\""));
}

#[test]
fn pages_d_une_collection() {
    let mut s = sample();
    s.collections.push(revel_model::Collection {
        id: "projets".into(),
        source: Default::default(),
        folder: "content/projets".into(),
        path: None,
        layout: None,
        props: Default::default(),
        filter: None,
        sort: None,
        fields: vec![],
    });
    let base = s.clone();
    let l = crate::make::collection_template(&s, "projets", "title");
    assert_eq!(l.id, "modele-projets");
    // Le modèle garde le titre du contenu et son texte (emplacement).
    let mut slot = false;
    revel_model::ids::walk(&l.body, &mut |n| slot |= matches!(n, Node::Slot));
    assert!(slot);
    let cmd = Command::Batch {
        commands: vec![
            Command::AddLayout {
                layout: l.clone(),
                index: None,
            },
            Command::SetCollectionPages {
                id: "projets".into(),
                path: Some("/projets/{{ slug }}/".into()),
                layout: Some(l.id.clone()),
            },
        ],
    };
    let inv = apply(&mut s, &cmd).unwrap();
    assert_eq!(
        s.collections[0].path.as_deref(),
        Some("/projets/{{ slug }}/")
    );
    assert_eq!(s.collections[0].layout.as_deref(), Some("modele-projets"));
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), json(&base));
    // Adresse sans partie propre à chaque contenu, gabarit inconnu : refusés.
    for (path, layout) in [("/projets/", None), ("/p/{{ slug }}/", Some("absent"))] {
        let r = apply(
            &mut s,
            &Command::SetCollectionPages {
                id: "projets".into(),
                path: Some(path.into()),
                layout: layout.map(String::from),
            },
        );
        assert!(r.is_err(), "{path}");
    }
}

#[test]
fn regle_ajoutee_deux_fois_reste_unique() {
    let mut s = sample();
    let add = |v: &str| Command::AddRule {
        selector: ".bouton".into(),
        media: None,
        decls: vec![revel_model::Decl {
            prop: "color".into(),
            value: v.into(),
            important: false,
        }],
    };
    let base = json(&s);
    let e1 = expand(&s, &add("red")).unwrap();
    let i1 = apply(&mut s, &e1).unwrap();
    // Deuxième réglage envoyé avant que le panneau ne connaisse la règle : même règle modifiée.
    let e2 = expand(&s, &add("blue")).unwrap();
    let i2 = apply(&mut s, &e2).unwrap();
    assert_eq!(
        s.style_rules().filter(|r| r.selector == ".bouton").count(),
        1
    );
    assert!(css(&s).contains(".bouton { color: blue; }"), "{}", css(&s));
    apply(&mut s, &i2).unwrap();
    assert!(css(&s).contains(".bouton { color: red; }"));
    apply(&mut s, &i1).unwrap();
    assert_eq!(json(&s), base);
}

#[test]
fn texte_mis_en_forme_modifie_d_un_bloc() {
    use crate::inline::{Piece, is_rich_host, rich_hosts, set_inline};
    let mut s = sample();
    // `<h2>Le <span class="accent">bureau</span> d’études</h2>` ajouté à la page.
    let h2 = el(
        "h2",
        &[],
        vec![
            text("Le "),
            el(
                "span",
                &[("class", "accent"), ("data-anim", "fade")],
                vec![text("bureau")],
            ),
            text(" d'études"),
        ],
    );
    s.pages[0].body.push(h2);
    s.ensure_ids();
    let h2_id = id_of(&s, "h2");
    let span_id = match address::node(
        &s,
        &Target {
            owner: page(),
            node: h2_id.clone(),
        },
    ) {
        Ok(Node::Element { children, .. }) => node_id(&children[1]).unwrap().to_string(),
        _ => panic!(),
    };
    let hosts = rich_hosts(&s);
    assert!(hosts.contains(&h2_id));
    // Texte avec une expression : modifié texte par texte.
    let p = address::node(
        &s,
        &Target {
            owner: page(),
            node: id_of(&s, "p"),
        },
    )
    .unwrap();
    assert!(!is_rich_host(p));
    let at = Target {
        owner: page(),
        node: h2_id.clone(),
    };
    let t = |s: &str| Piece::Text { text: s.into() };
    let span = |id: Option<&str>, kids: Vec<Piece>| Piece::Element {
        tag: "span".into(),
        id: id.map(String::from),
        class: None,
        href: None,
        target: None,
        children: kids,
    };
    // Contenu inchangé (typographie de la page comprise) : pas de commande.
    let same = vec![
        t("Le "),
        span(Some(&span_id), vec![t("bureau")]),
        t(" d’études"),
    ];
    assert_eq!(set_inline(&s, &at, &same, true).unwrap(), None);
    // Mot ajouté dans le span, mot mis en gras, copie du span collée à la fin.
    let base = json(&s);
    let new = vec![
        t("Le "),
        span(Some(&span_id), vec![t("grand bureau")]),
        Piece::Element {
            tag: "b".into(),
            id: None,
            class: None,
            href: None,
            target: None,
            children: vec![t(" d’études")],
        },
        span(Some(&span_id), vec![t(" !")]),
        Piece::Element {
            tag: "br".into(),
            id: None,
            class: None,
            href: None,
            target: None,
            children: vec![],
        },
    ];
    let cmd = set_inline(&s, &at, &new, true).unwrap().unwrap();
    let inv = apply(&mut s, &cmd).unwrap();
    let Node::Element { children, .. } = address::node(&s, &at).unwrap().clone() else {
        panic!()
    };
    assert_eq!(children.len(), 4, "{children:?}");
    // Span repris : même identifiant, mêmes attributs.
    assert_eq!(node_id(&children[1]), Some(span_id.as_str()));
    let Node::Element { attrs, .. } = &children[1] else {
        panic!()
    };
    assert_eq!(attrs.len(), 2);
    // Gras : `strong`, apostrophe de la source rétablie.
    let Node::Element {
        tag, children: b, ..
    } = &children[2]
    else {
        panic!()
    };
    assert_eq!(tag, "strong");
    assert_eq!(b, &vec![text(" d'études")]);
    // Copie : mêmes attributs, nouvel identifiant ; retour à la ligne final retiré.
    let Node::Element { id, attrs, .. } = &children[3] else {
        panic!()
    };
    assert_ne!(id.as_deref(), Some(span_id.as_str()));
    assert_eq!(attrs.len(), 2);
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), base);
    // Lien de la barre de mise en forme : adresse modifiée, lien retiré.
    let link = vec![Piece::Element {
        tag: "a".into(),
        id: None,
        class: None,
        href: Some("/contact/".into()),
        target: None,
        children: vec![t("Le bureau")],
    }];
    let cmd = set_inline(&s, &at, &link, false).unwrap().unwrap();
    apply(&mut s, &cmd).unwrap();
    let Node::Element { children, .. } = address::node(&s, &at).unwrap().clone() else {
        panic!()
    };
    let Node::Element { tag, attrs, .. } = &children[0] else {
        panic!()
    };
    assert_eq!(tag, "a");
    assert_eq!(attrs[0].value.as_deref(), Some("/contact/"));
}

/// Éléments complexes du panneau Ajouter (Défilement, Carrousel, Diaporama, Barre de
/// navigation) : leurs règles, celles de la barre de navigation aussi pour tablette (point de
/// rupture le plus large du site) ; annulés à l'identique.
#[test]
fn elements_complexes() {
    let mut s = sample();
    let before = json(&s);
    let parent = Parent::of(&at(&s, "main"), Branch::Children);
    let mut inverses = Vec::new();
    for (p, text) in [
        (make::Preset::Ticker, "Élément"),
        (make::Preset::Carousel, "Diapositive|Précédente|Suivante"),
        (make::Preset::Slideshow, "Diapositive|Précédente|Suivante"),
        (make::Preset::Navbar, "Marque|Menu|Accueil|Projets|Contact"),
    ] {
        inverses.push(
            apply(
                &mut s,
                &Command::InsertPreset {
                    parent: parent.clone(),
                    index: 0,
                    preset: p,
                    text: text.into(),
                },
            )
            .unwrap_or_else(|e| panic!("{p:?} : {e}")),
        );
    }
    let c = css(&s);
    for sel in [
        ".defilement {",
        ".carrousel__piste {",
        ".diaporama__points button.is-active {",
        ".barre-nav__menu {",
    ] {
        assert!(c.contains(sel), "{sel}\n{c}");
    }
    let tablet = s
        .breakpoints
        .iter()
        .filter_map(|b| b.max_width)
        .max()
        .unwrap();
    rule_id(
        &s,
        ".barre-nav.is-open .barre-nav__menu",
        Some(&format!("(max-width: {tablet}px)")),
    );
    let html = serde_json::to_string(&s.pages[0].body).unwrap();
    assert!(html.contains("ticker speed=60 pause") && html.contains("autoplay=5 loop"));
    assert!(html.contains("click class:is-open parent") && html.contains("Projets"));
    for inv in inverses.into_iter().rev() {
        apply(&mut s, &inv).unwrap();
    }
    assert_eq!(json(&s), before);
}

#[test]
fn proprietes_du_gabarit() {
    use revel_model::{PropDef, PropKind};
    let mut s = sample();
    s.layouts.push(revel_model::Layout {
        id: "layouts/base".into(),
        parent: None,
        props: Default::default(),
        fields: Vec::new(),
        body: vec![
            Node::If {
                id: None,
                cond: "not masquer".into(),
                then: vec![el(
                    "section",
                    &[("class", "appel")],
                    vec![text("Écrivez-nous")],
                )],
                otherwise: vec![],
            },
            el("main", &[], vec![Node::Slot]),
        ],
    });
    s.pages[0].layout = Some("layouts/base".into());
    s.pages[0]
        .props
        .insert("masquer".into(), serde_json::json!(true));
    s.ensure_ids();
    let flagged = |s: &Site| {
        editable::audit(s)
            .into_iter()
            .filter(|i| i.detail.contains("masquer"))
            .count()
    };
    // Variable de page lue par le gabarit : relevée (donnée de page et condition écrite).
    assert_eq!(flagged(&s), 2);
    let field = PropDef {
        name: "masquer".into(),
        label: Some("Masquer l'appel".into()),
        kind: PropKind::Switch,
        default: None,
    };
    let before = json(&s);
    let inv = apply(
        &mut s,
        &Command::SetLayoutFields {
            id: "layouts/base".into(),
            fields: vec![field.clone()],
        },
    )
    .unwrap();
    let after = json(&s);
    apply(&mut s, &inv).unwrap();
    assert_eq!(json(&s), before);
    apply(
        &mut s,
        &Command::SetLayoutFields {
            id: "layouts/base".into(),
            fields: vec![field.clone()],
        },
    )
    .unwrap();
    assert_eq!(json(&s), after);
    // Propriété du gabarit : valeur de la page et condition dans la forme de l'interface.
    assert_eq!(flagged(&s), 0);
    // Nom invalide ou en double : refusé.
    let mut twice = vec![field.clone(), field.clone()];
    assert!(
        apply(
            &mut s,
            &Command::SetLayoutFields {
                id: "layouts/base".into(),
                fields: std::mem::take(&mut twice),
            },
        )
        .is_err()
    );
    // Retrait : condition résolue avec la valeur par défaut (éteint : l'appel est affiché),
    // valeur des pages retirée.
    let cmd = make::remove_layout_field(&s, "layouts/base", "masquer").unwrap();
    let inv = apply(&mut s, &cmd).unwrap();
    assert!(s.layouts[0].fields.is_empty());
    assert!(!s.pages[0].props.contains_key("masquer"));
    assert!(
        !serde_json::to_string(&s.layouts[0].body)
            .unwrap()
            .contains("masquer")
    );
    apply(&mut s, &inv).unwrap();
    assert_eq!(s.layouts[0].fields.len(), 1);
    assert_eq!(
        s.pages[0].props.get("masquer"),
        Some(&serde_json::json!(true))
    );
}
