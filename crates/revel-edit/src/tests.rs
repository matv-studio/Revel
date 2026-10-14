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
    });
    s.components.push(Component {
        id: "carte".into(),
        body: vec![el("article", &[], vec![el("h3", &[], vec![text("Carte")])])],
    });
    s.ensure_ids();
    s
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
            let cmd = match rng.next(6) {
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
            if h.apply(&mut s, cmd, false).is_ok() {
                applied += 1;
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
}
