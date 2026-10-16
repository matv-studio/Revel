use super::*;
use revel_model::{Collection, Component, Layout, Page, Settings, Sort};
use serde_json::json;

fn parse(src: &str) -> Vec<Node> {
    revel_markup::parse(
        src,
        &revel_markup::Options {
            dialect: revel_markup::Dialect::Nunjucks,
            ..Default::default()
        },
    )
    .unwrap()
    .nodes
}

fn input(site: &Site, entries: Vec<Entry>) -> BuildInput<'_> {
    BuildInput {
        site,
        entries,
        locales: Vec::new(),
        environment: "production".into(),
        version: "v1".into(),
        hooks: Arc::new(PlainAssets),
        now: DateTime::parse_from_rfc3339("2026-10-13T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
    }
}

fn entry(stem: &str, data: serde_json::Value) -> Entry {
    Entry {
        folder: "content/projets".into(),
        file_stem: stem.into(),
        data,
        body: String::new(),
    }
}

fn site() -> Site {
    let mut s = Site::new("Test", "fr");
    s.data.insert(
        "site".into(),
        json!({"title": "Alunites", "url": "https://alunites.com"}),
    );
    s.settings = Settings {
        typography: Some("fr".into()),
        filters: [(
            "seo_title".to_string(),
            revel_model::CustomFilter {
                params: vec!["seo".into(), "site".into()],
                expr: r#"(seo.title ~ " | " ~ site.title) if seo and seo.title else site.title"#
                    .into(),
                description: None,
            },
        )]
        .into(),
        ..Default::default()
    };
    s.layouts.push(Layout {
        id: "base".into(),
        parent: None,
        props: Default::default(),
        body: parse(
            r#"<!doctype html><html><head><title>{{ seo | seo_title(site) }}</title></head><body class="{{ bodyClass }}">{% include "header" %}{{ content | safe }}</body></html>"#,
        ),
    });
    s.layouts.push(Layout {
        id: "project".into(),
        parent: Some("base".into()),
        props: [("bodyClass".to_string(), json!("page-entry"))].into(),
        body: parse(r#"<article><h1>{{ title }}</h1>{{ content | safe }}</article>"#),
    });
    s.components.push(Component::new(
        "header",
        parse(r#"<header><a href="/"{% if page.url == "/" %} aria-current="page"{% endif %}>Accueil</a></header>"#),
    ));
    s.collections.push(Collection {
        id: "projets".into(),
        source: CollectionSource::Folder,
        folder: "content/projets".into(),
        path: Some("/project/{{ slug }}/".into()),
        layout: Some("project".into()),
        props: Default::default(),
        filter: Some("published != false".into()),
        sort: Some(Sort {
            field: "date".into(),
            descending: true,
            kind: SortKind::Date,
        }),
        fields: vec![],
    });
    s.collections.push(Collection {
        id: "sitemap".into(),
        source: CollectionSource::Pages,
        folder: String::new(),
        path: None,
        layout: None,
        props: Default::default(),
        filter: Some(r#"not (url is endingwith(".xml"))"#.into()),
        sort: None,
        fields: vec![],
    });
    s.pages.push(Page {
        id: "index".into(),
        path: "/".into(),
        layout: Some("base".into()),
        props: [
            ("seo".to_string(), json!({"title": "Accueil"})),
            ("bodyClass".to_string(), json!("page-home")),
        ]
        .into(),
        body: parse(
            r#"{%- set featured = [] -%}{%- for slug in ["b", "zz", "a"] -%}{%- set it = collections.projets | by_slug(slug) -%}{%- if it %}{% set featured = (featured.push(it), featured) %}{% endif -%}{%- endfor -%}
<ul>{% for p in featured %}<li class="i{% if loop.first %} first{% endif %}"><a href="{{ p.url }}">{{ p.data.title }}</a></li>{% endfor %}</ul><p>L'agence : « oui » !</p>"#,
        ),
        computed: Default::default(),
        draft: false,
        code: Default::default(),
        name: None,
    });
    s.pages.push(Page {
        id: "sitemap".into(),
        path: "/sitemap.xml".into(),
        layout: None,
        props: Default::default(),
        body: revel_markup::parse(
            r#"<urlset>{% for p in collections.sitemap %}<url><loc>{{ p.url | absolute_url(site.url) }}</loc></url>{% endfor %}</urlset>"#,
            &revel_markup::Options {
                mode: revel_markup::Mode::Xml,
                ..Default::default()
            },
        )
        .unwrap()
        .nodes,
        computed: Default::default(),
        draft: false,
        code: Default::default(),
        name: None,
    });
    s
}

fn file<'a>(out: &'a BuildOutput, path: &str) -> &'a str {
    std::str::from_utf8(&out.files.iter().find(|f| f.path == path).unwrap().bytes).unwrap()
}

#[test]
fn site_complet() {
    let s = site();
    let out = build(&input(
        &s,
        vec![
            entry(
                "a",
                json!({"slug": "a", "title": "Projet <A>", "date": "2026-01-02"}),
            ),
            entry(
                "b",
                json!({"slug": "b", "title": "Projet B", "date": "2026-03-01"}),
            ),
            entry(
                "c",
                json!({"slug": "c", "title": "Caché", "published": false}),
            ),
        ],
    ))
    .unwrap();
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let paths: Vec<&str> = out.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "index.html",
            "project/a/index.html",
            "project/b/index.html",
            "robots.txt",
            "sitemap.xml"
        ]
    );
    // `robots.txt` produit (le site n'en a pas), plan du site du site gardé.
    assert!(file(&out, "robots.txt").starts_with("User-agent: *\nAllow: /\n"));

    let home = file(&out, "index.html");
    assert!(home.contains("<title>Accueil | Alunites</title>"), "{home}");
    assert!(home.contains(r#"<body class="page-home">"#));
    assert!(home.contains(r#"<a href="/" aria-current="page">"#));
    assert!(home.contains(r#"<li class="i first"><a href="/project/b/">Projet B</a></li><li class="i"><a href="/project/a/">Projet &lt;A&gt;</a></li>"#), "{home}");
    assert!(home.contains("L’agence\u{a0}: «\u{a0}oui\u{a0}»\u{a0}!"));

    let a = file(&out, "project/a/index.html");
    assert!(a.contains(r#"<body class="page-entry">"#), "{a}");
    assert!(a.contains("<title>Alunites</title>"));
    assert!(a.contains("<article><h1>Projet &lt;A&gt;</h1></article>"));
    assert!(a.contains(r#"<a href="/">Accueil</a>"#));

    let sm = file(&out, "sitemap.xml");
    assert!(sm.contains("<loc>https://alunites.com/project/a/</loc>"));
    assert!(sm.contains("<loc>https://alunites.com/</loc>"));
    assert!(!sm.contains("sitemap.xml"));
}

#[test]
fn code_ajoute() {
    let mut s = site();
    s.settings.code = revel_model::CustomCode {
        head: "<meta name=\"verif\" content=\"x'y\">".into(),
        body: "<script>track('site')</script>".into(),
    };
    s.pages[0].code.body = "<script>track('page')</script>".into();
    let out = build(&input(&s, vec![])).unwrap();
    let home = file(&out, "index.html");
    // Avant `</head>` et `</body>`, site puis page, sans retouche typographique.
    let head = home.find("<meta name=\"verif\" content=\"x'y\">\n</head>");
    let site_js = home.find("<script>track('site')</script>");
    let page_js = home.find("<script>track('page')</script>\n</body>");
    assert!(
        head.is_some() && site_js.is_some() && page_js.is_some(),
        "{home}"
    );
    assert!(site_js < page_js);
    // Pas dans le canevas de l'éditeur (scripts arrêtés).
    let e = render_editor(&input(&s, vec![]), "/", &EditorOptions { scripts: false })
        .unwrap()
        .unwrap();
    assert!(!e.html.contains("track("), "{}", e.html);
}

#[test]
fn chemins_de_sortie() {
    assert_eq!(output_path("/").unwrap(), "index.html");
    assert_eq!(output_path("/agence/").unwrap(), "agence/index.html");
    assert_eq!(output_path("/.htaccess").unwrap(), ".htaccess");
    assert!(output_path("/../x").is_err());
    assert_eq!(DocKind::of_path("/feed.xml"), DocKind::Xml);
    assert_eq!(DocKind::of_path("/robots.txt"), DocKind::Text);
}

#[test]
fn rendu_pour_l_editeur() {
    use revel_edit::canvas::{Binding, Seg, Slot, Source};
    use revel_edit::{Branch, Owner, Parent};
    let mut s = site();
    s.data.insert(
        "pages".into(),
        json!({"accueil": {"hero": {"title": "Bienvenue"}, "items": [{"t": "Un"}, {"t": "Deux"}]}}),
    );
    s.pages[0].body.extend(parse(
        r#"{% set home = pages.accueil %}<h2 class="hero">{{ home.hero.title }}</h2><ol>{% for it in home.items %}<li>{{ it.t }} {{ it.t | upper }}</li>{% endfor %}</ol><p>{{ bodyClass }}</p><script src="/a.js"></script><script type="application/ld+json">{"a":1}</script><title>Pas de repère {{ site.title }}</title>"#,
    ));
    s.ensure_ids();
    let entries = vec![
        entry(
            "a",
            json!({"slug": "a", "title": "Projet <A>", "date": "2026-01-02"}),
        ),
        entry(
            "b",
            json!({"slug": "b", "title": "Projet B", "date": "2026-03-01"}),
        ),
    ];
    let inp = input(&s, entries.clone());
    let built = build(&inp).unwrap();
    let ed = render_editor(&inp, "/", &EditorOptions { scripts: true })
        .unwrap()
        .unwrap();
    // Sans les repères : la page générée, octet pour octet.
    assert_eq!(strip_editor_marks(&ed.html), file(&built, "index.html"));
    assert!(
        ed.html.contains(">Accueil | Alunites</title>"),
        "pas de repère dans <title>"
    );
    assert!(ed.html.contains(">Pas de repère Alunites</title>"));

    let page = Owner::Page("index".into());
    let data = |path: &[Seg]| Binding {
        source: Source::Data,
        path: path.to_vec(),
    };
    let k = |x: &str| Seg::Key(x.into());
    let exprs: Vec<(&str, Option<&Binding>)> = ed
        .slots
        .iter()
        .filter_map(|sl| match sl {
            Slot::Expr { expr, binding, .. } => Some((expr.as_str(), binding.as_ref())),
            _ => None,
        })
        .collect();
    // Données globales suivies à travers `set` et les boucles.
    assert!(exprs.contains(&(
        "home.hero.title",
        Some(&data(&[k("pages"), k("accueil"), k("hero"), k("title")]))
    )));
    assert!(exprs.contains(&(
        "it.t",
        Some(&data(&[
            k("pages"),
            k("accueil"),
            k("items"),
            Seg::Index(1),
            k("t")
        ]))
    )));
    let upper = ed
        .slots
        .iter()
        .find(|sl| matches!(sl, Slot::Expr { expr, .. } if expr == "it.t | upper"))
        .unwrap();
    assert!(
        matches!(
            upper,
            Slot::Expr {
                transformed: true,
                binding: Some(_),
                ..
            }
        ),
        "valeur transformée : reliée à sa source, modifiable dans un champ"
    );
    // Élément de collection (liste construite par filtres) : relié au contenu.
    assert!(exprs.contains(&(
        "p.data.title",
        Some(&Binding {
            source: Source::Entry {
                folder: "content/projets".into(),
                file: "b".into()
            },
            path: vec![k("title")]
        })
    )));
    // Variable de page.
    assert!(exprs.contains(&(
        "bodyClass",
        Some(&Binding {
            source: Source::Page {
                page: "index".into()
            },
            path: vec![k("bodyClass")]
        })
    )));
    // Texte écrit dans la page : adresse pour SetText.
    let p_id = revel_model::ids::node_id(
        s.pages[0]
            .body
            .iter()
            .find(|n| matches!(n, Node::Element { tag, .. } if tag == "p"))
            .unwrap(),
    )
    .unwrap()
    .to_string();
    assert!(ed.slots.contains(&Slot::Text {
        parent: Parent {
            owner: page.clone(),
            node: Some(p_id.clone()),
            branch: Branch::Children
        },
        index: 0
    }));
    assert!(ed.html.contains(&format!("<p data-rv=\"{p_id}\">")));
    // Composant utilisé : bloc repéré ; ses éléments appartiennent au composant.
    assert!(
        ed.slots
            .iter()
            .any(|sl| matches!(sl, Slot::Block { owner: Owner::Layout(l), .. } if l == "base"))
    );
    // Contenu d'une entrée : la page de projet a ses variables reliées à l'en-tête.
    let pa = render_editor(&inp, "/project/a", &EditorOptions::default())
        .unwrap()
        .unwrap();
    assert!(pa.slots.iter().any(|sl| matches!(sl,
        Slot::Expr { expr, binding: Some(Binding { source: Source::Entry { file, .. }, path }), .. }
            if expr == "title" && file == "a" && path == &vec![k("title")])));
    // Scripts du site neutralisés par défaut, données JSON gardées.
    let off = render_editor(&inp, "/", &EditorOptions::default())
        .unwrap()
        .unwrap();
    assert!(off.html.contains(r#"<script data-rv="#));
    assert!(
        off.html
            .contains(r#" type="text/plain" data-rv-script src="/a.js"></script>"#),
        "{}",
        off.html
    );
    assert!(
        off.html
            .contains(r#"type="application/ld+json">{"a":1}</script>"#)
    );
    assert!(
        render_editor(&inp, "/sitemap.xml", &EditorOptions::default())
            .unwrap()
            .is_none()
    );
    assert!(
        render_editor(&inp, "/absente/", &EditorOptions::default())
            .unwrap()
            .is_none()
    );
}

#[test]
fn valeurs_des_variables_pour_l_editeur() {
    use revel_edit::canvas::Shown;
    use revel_edit::{Owner, Parent};
    let mut s = site();
    s.data.insert(
        "pages".into(),
        json!({"accueil": {"title": "Bienvenue", "items": [1, 2, 3]}}),
    );
    s.pages[0].body.extend(parse(
        r#"{% set home = pages.accueil %}{% set n = home.items | length %}{% set vide = "" %}<ul>{% for i in home.items %}{% set double = i * 2 %}<li>{{ double }}</li>{% endfor %}</ul>"#,
    ));
    s.ensure_ids();
    let inp = input(&s, Vec::new());
    let ed = render_editor(&inp, "/", &EditorOptions::default())
        .unwrap()
        .unwrap();
    let page = Owner::Page("index".into());
    let at = |i: usize| {
        ed.vars
            .iter()
            .find(|v| v.parent == Parent::root(page.clone()) && v.index == i)
            .unwrap_or_else(|| panic!("affectation {i} : {:?}", ed.vars))
    };
    let first = s.pages[0]
        .body
        .iter()
        .position(|n| matches!(n, Node::Set { name, .. } if name == "home"))
        .unwrap();
    assert_eq!(
        at(first).value,
        Shown::Object {
            len: 2,
            label: Some("Bienvenue".into())
        }
    );
    assert_eq!(at(first + 1).value, Shown::Literal { text: "3".into() });
    assert_eq!(at(first + 2).value, Shown::Empty);
    // Dans la boucle : première valeur, puis « varie ».
    assert!(
        ed.vars.iter().any(|v| v.parent.node.is_some()
            && v.value == Shown::Literal { text: "2".into() }
            && v.varies),
        "{:?}",
        ed.vars
    );
    // Syntaxe des expressions (variables et filtres inconnus acceptés).
    assert!(check_expression("a | f(b) if c else d").is_ok());
    assert!(check_expression("a |").is_err());
    assert!(check_expression("(a").is_err());
}

#[test]
fn pages_brouillon_indexation_et_plan_du_site() {
    let mut s = Site::new("Essai", "fr");
    s.base_url = Some("https://exemple.fr/".into());
    let page = |id: &str, path: &str| Page {
        id: id.into(),
        path: path.into(),
        layout: None,
        props: Default::default(),
        computed: Default::default(),
        body: parse(
            "<html><head><title>x</title></head><body><p>{{ seo.title }}</p></body></html>",
        ),
        draft: false,
        code: Default::default(),
        name: None,
    };
    let mut accueil = page("index", "/");
    accueil.name = Some("Accueil".into());
    accueil.props.insert(
        "seo".into(),
        json!({"title": "Bienvenue", "description": "D"}),
    );
    let mut cachee = page("cachee", "/cachee/");
    cachee.props.insert("noindex".into(), json!(true));
    let mut brouillon = page("brouillon", "/brouillon/");
    brouillon.draft = true;
    let mut liee = page("liee", "/liee/");
    liee.computed
        .insert("seo".into(), "pages[\"liee\"].seo".into());
    s.data
        .insert("pages".into(), json!({"liee": {"seo": {"title": "Liée"}}}));
    s.pages = vec![accueil, cachee, brouillon, liee];
    s.ensure_ids();
    let inp = input(&s, Vec::new());
    let out = build(&inp).unwrap();
    let paths: Vec<&str> = out.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "cachee/index.html",
            "index.html",
            "liee/index.html",
            "robots.txt",
            "sitemap.xml"
        ],
        "brouillon non publié"
    );
    assert!(
        file(&out, "cachee/index.html")
            .contains(r#"<meta name="robots" content="noindex"></head>"#)
    );
    assert!(!file(&out, "index.html").contains("robots"));
    let sm = file(&out, "sitemap.xml");
    assert!(
        sm.contains("<loc>https://exemple.fr/</loc>")
            && sm.contains("<loc>https://exemple.fr/liee/</loc>")
    );
    assert!(!sm.contains("cachee") && !sm.contains("brouillon"), "{sm}");
    assert_eq!(
        file(&out, "robots.txt"),
        "User-agent: *\nAllow: /\nSitemap: https://exemple.fr/sitemap.xml\n"
    );
    // Réglages : plan du site désactivé, robots.txt personnalisé.
    let mut s2 = s.clone();
    s2.settings.seo = revel_model::SeoSettings {
        no_sitemap: true,
        robots: Some("User-agent: *\nDisallow: /".into()),
    };
    let out2 = build(&input(&s2, Vec::new())).unwrap();
    assert!(!out2.files.iter().any(|f| f.path == "sitemap.xml"));
    assert_eq!(file(&out2, "robots.txt"), "User-agent: *\nDisallow: /\n");
    // Éditeur : brouillon listé, référencement et sa source.
    let pages = list_pages(&inp).unwrap();
    let get = |u: &str| pages.iter().find(|p| p.url == u).unwrap();
    assert!(get("/brouillon/").draft);
    assert_eq!(get("/").name.as_deref(), Some("Accueil"));
    let seo = get("/").seo.clone().unwrap();
    assert_eq!(
        (seo.title.as_str(), seo.description.as_str(), seo.source),
        ("Bienvenue", "D", SeoSource::Page)
    );
    assert!(get("/cachee/").seo.as_ref().unwrap().noindex);
    assert_eq!(
        get("/liee/").seo.as_ref().unwrap().source,
        SeoSource::Data(vec!["pages".into(), "liee".into(), "seo".into()])
    );
    assert_eq!(get("/liee/").seo.as_ref().unwrap().title, "Liée");
}

#[test]
fn proprietes_de_composant() {
    use revel_edit::Owner;
    use revel_edit::canvas::{Slot, Source};
    let mut s = site();
    let mut carte = Component::new(
        "carte",
        parse(
            r#"<article>{% if visible %}<h3>{{ titre }}</h3>{% endif %}<a href="{{ lien }}">Voir</a></article>"#,
        ),
    );
    carte.props = vec![
        revel_model::PropDef {
            name: "titre".into(),
            label: None,
            kind: revel_model::PropKind::Text,
            default: Some(json!("Sans titre")),
        },
        revel_model::PropDef {
            name: "lien".into(),
            label: None,
            kind: revel_model::PropKind::Link,
            default: None,
        },
        revel_model::PropDef {
            name: "visible".into(),
            label: None,
            kind: revel_model::PropKind::Switch,
            default: Some(json!(true)),
        },
    ];
    s.components.push(carte);
    // Composant utilisé dans lui-même : ignoré, avec un avertissement.
    s.components.push(Component::new(
        "boucle",
        parse(r#"<i>{% include "boucle" %}</i>"#),
    ));
    let use_ = |id: &str, args: &[(&str, &str)]| Node::Use {
        id: Some(id.into()),
        component: "carte".into(),
        args: args
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        variant: None,
        attrs: Vec::new(),
    };
    s.pages[0].body.push(use_("u1", &[]));
    s.pages[0]
        .body
        .push(use_("u2", &[("titre", "\"Projet\""), ("lien", "site.url")]));
    s.pages[0]
        .body
        .push(use_("u3", &[("visible", "false"), ("lien", "\"/a/\"")]));
    s.pages[0].body.extend(parse(r#"{% include "boucle" %}"#));
    s.ensure_ids();
    let inp = input(&s, Vec::new());
    let out = build(&inp).unwrap();
    let home = file(&out, "index.html");
    assert!(
        home.contains(r#"<article><h3>Sans titre</h3><a href="">Voir</a></article><article><h3>Projet</h3><a href="https://alunites.com">Voir</a></article><article><a href="/a/">Voir</a></article><i></i>"#),
        "{home}"
    );
    assert!(
        out.warnings.iter().any(|w| w.contains("boucle")),
        "{:?}",
        out.warnings
    );
    // Éditeur : texte d'une propriété relié à l'utilisation ; utilisation repérée avec son
    // composant.
    let ed = render_editor(&inp, "/", &EditorOptions { scripts: false })
        .unwrap()
        .unwrap();
    let props: Vec<(String, String)> = ed
        .slots
        .iter()
        .filter_map(|sl| match sl {
            Slot::Expr {
                binding: Some(b), ..
            } => match &b.source {
                Source::Prop { node, name, .. } => Some((node.clone(), name.clone())),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(
        props,
        vec![("u1".into(), "titre".into()), ("u2".into(), "titre".into())]
    );
    assert!(ed.slots.iter().any(|sl| matches!(sl,
        Slot::Block { owner: Owner::Page(_), node, component: Some(c) } if node == "u2" && c == "carte")));
}

#[test]
fn variante_et_animation_d_une_utilisation() {
    let mut s = site();
    let mut carte = Component::new("carte", parse(r#"<div class="carte"><b>Titre</b></div>"#));
    carte.variants = vec![
        revel_model::Variant {
            id: "sombre".into(),
            name: Some("Sombre".into()),
            kind: revel_model::VariantKind::Custom,
        },
        revel_model::Variant {
            id: "survol".into(),
            name: None,
            kind: revel_model::VariantKind::Hover,
        },
    ];
    s.components.push(carte);
    let use_ = |variant: Option<&str>, attrs: Vec<revel_model::Attr>| Node::Use {
        id: None,
        component: "carte".into(),
        args: Default::default(),
        variant: variant.map(str::to_string),
        attrs,
    };
    s.pages[0].body = vec![
        use_(None, Vec::new()),
        use_(
            Some("sombre"),
            vec![revel_model::Attr {
                name: "data-revel-appear".into(),
                value: Some("up".into()),
                when: None,
            }],
        ),
        // Variante inconnue ou geste : pas de classe.
        use_(Some("survol"), Vec::new()),
    ];
    s.ensure_ids();
    let inp = input(&s, Vec::new());
    let out = build(&inp).unwrap();
    let home = file(&out, "index.html");
    assert!(
        home.contains(r#"<div class="carte"><b>Titre</b></div><div class="carte is-sombre" data-revel-appear="up"><b>Titre</b></div><div class="carte"><b>Titre</b></div>"#),
        "{home}"
    );
    // Page animée : script ajouté.
    assert!(home.contains("revel-animations.js"), "{home}");
}

#[test]
fn liste_de_collection() {
    use revel_edit::collection::Query;
    let mut s = site();
    let q = |filter, sort, limit| {
        Query {
            collection: "projets".into(),
            filter,
            sort,
            limit,
        }
        .to_expr()
    };
    let lists = [
        q(None, Some(("data.title".to_string(), false)), None),
        q(None, Some(("date".to_string(), true)), Some(1)),
        q(Some(("data.une".to_string(), None)), None, None),
        q(
            Some(("data.client".to_string(), Some("\"Volvo\"".to_string()))),
            None,
            None,
        ),
    ];
    let mut src = String::new();
    for (i, e) in lists.iter().enumerate() {
        src.push_str(&format!(
            "<ul id=\"l{i}\">{{% for item in {e} %}}<li>{{{{ item.data.title }}}}</li>{{% endfor %}}</ul>"
        ));
    }
    s.pages[0].body.extend(parse(&src));
    let out = build(&input(
        &s,
        vec![
            entry(
                "a",
                json!({"slug": "a", "title": "Alpha", "date": "2026-01-02", "client": "Volvo"}),
            ),
            entry(
                "b",
                json!({"slug": "b", "title": "Bêta", "date": "2026-03-01", "une": true}),
            ),
        ],
    ))
    .unwrap();
    let home = file(&out, "index.html");
    assert!(
        home.contains(r#"<ul id="l0"><li>Alpha</li><li>Bêta</li></ul>"#),
        "{home}"
    );
    assert!(home.contains(r#"<ul id="l1"><li>Bêta</li></ul>"#), "{home}");
    assert!(home.contains(r#"<ul id="l2"><li>Bêta</li></ul>"#), "{home}");
    assert!(
        home.contains(r#"<ul id="l3"><li>Alpha</li></ul>"#),
        "{home}"
    );
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
}

#[test]
fn langues() {
    let mut s = site();
    s.base_url = Some("https://alunites.com".into());
    let mut en = revel_model::Locale::new("en");
    en.texts = [
        ("Accueil".to_string(), "Home".to_string()),
        (
            "L'agence : « oui » !".to_string(),
            "The agency: “yes”!".to_string(),
        ),
    ]
    .into();
    let mut de = revel_model::Locale::new("de");
    de.draft = true;
    s.locales = vec![en, de];
    s.pages.push(Page {
        id: "langues".into(),
        path: "/langues/".into(),
        layout: None,
        props: Default::default(),
        body: parse(
            r#"<html><head></head><body>{% for l in locales %}<a href="{{ l.url }}" hreflang="{{ l.code }}">{{ l.name }}</a>{% endfor %}<a href="/project/a/#x">a</a><a href="/media/f.pdf">f</a><p>{{ locale }}</p></body></html>"#,
        ),
        computed: Default::default(),
        draft: false,
        code: Default::default(),
        name: None,
    });
    s.ensure_ids();
    let entries = vec![
        entry(
            "a",
            json!({"slug": "a", "title": "Projet <A>", "date": "2026-01-02"}),
        ),
        entry(
            "b",
            json!({"slug": "b", "title": "Projet B", "date": "2026-03-01"}),
        ),
    ];
    let mut inp = input(&s, entries);
    inp.locales = vec![LocaleInput {
        code: "en".into(),
        data: [("site".to_string(), json!({"title": "Alunites EN"}))].into(),
        entries: vec![Entry {
            folder: "content/en/projets".into(),
            file_stem: "b".into(),
            data: json!({"title": "Project B"}),
            body: String::new(),
        }],
    }];
    let out = build(&inp).unwrap();
    let paths: Vec<&str> = out.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "en/index.html",
            "en/langues/index.html",
            "en/project/a/index.html",
            "en/project/b/index.html",
            "index.html",
            "langues/index.html",
            "project/a/index.html",
            "project/b/index.html",
            "robots.txt",
            "sitemap.xml"
        ],
        "langue en préparation (de) non publiée, plan du site seulement dans la langue principale"
    );
    let home = file(&out, "index.html");
    assert!(home.contains(r#"<link rel="alternate" hreflang="fr" href="https://alunites.com/"><link rel="alternate" hreflang="en" href="https://alunites.com/en/"><link rel="alternate" hreflang="x-default" href="https://alunites.com/"></head>"#), "{home}");
    assert!(home.contains("<title>Accueil | Alunites</title>"));
    let en = file(&out, "en/index.html");
    assert!(en.starts_with("<!doctype html><html lang=\"en\">"), "{en}");
    // Textes traduits (titre de la page compris), données de la langue, typographie française
    // seulement en français.
    assert!(en.contains("<title>Home | Alunites EN</title>"), "{en}");
    assert!(en.contains(r#"<a href="/en/">Home</a>"#), "{en}");
    assert!(en.contains("The agency: “yes”!"), "{en}");
    // Contenus : traduit (b), sinon celui de la langue principale (a) ; adresses préfixées.
    assert!(en.contains(r#"<a href="/en/project/b/">Project B</a></li><li class="i"><a href="/en/project/a/">Projet &lt;A&gt;</a>"#), "{en}");
    assert!(file(&out, "en/project/a/index.html").contains("<h1>Projet &lt;A&gt;</h1>"));
    let l = file(&out, "en/langues/index.html");
    assert!(l.contains(r#"<a href="/langues/" hreflang="fr">Français</a><a href="/en/langues/" hreflang="en">English</a><a href="/en/project/a/#x">a</a><a href="/media/f.pdf">f</a><p>en</p>"#), "{l}");
    assert!(file(&out, "langues/index.html").contains("<p>fr</p>"));

    // Éditeur : page d'une langue en préparation, valeurs reliées aux fichiers de la langue.
    let p = render_editor(&inp, "/de/langues/", &EditorOptions::default())
        .unwrap()
        .unwrap();
    assert!(
        p.html.contains(r#"hreflang="de""#) && p.html.contains("<html lang=\"de\""),
        "{}",
        p.html
    );
    let p = render_editor(&inp, "/en/project/b/", &EditorOptions::default())
        .unwrap()
        .unwrap();
    assert!(p.slots.iter().any(|x| matches!(x, Slot::Expr { binding: Some(Binding { source: Source::Entry { folder, file }, .. }), .. } if folder == "content/en/projets" && file == "b")), "{:?} {}", p.slots, p.html);
    assert!(
        render_editor(&inp, "/en/sitemap.xml", &EditorOptions::default())
            .unwrap()
            .is_none()
    );
}

/// Formulaires reçus par l'hébergement (5.8) : formulaire relié au script, script produit avec
/// l'adresse de réception ; sans adresse, avertissement et pas de script.
#[test]
fn formulaires_du_site() {
    let mut s = site();
    s.base_url = Some("https://alunites.com".into());
    s.pages[0].body.extend(parse(
        r#"<form class="formulaire" data-revel-form="contact"><input name="email" type="email"><button type="submit">Envoyer</button></form>"#,
    ));
    let out = build(&input(&s, vec![])).unwrap();
    assert!(!out.files.iter().any(|f| f.path == "revel-form.php"));
    assert!(
        out.warnings.iter().any(|w| w.contains("Formulaires")),
        "{:?}",
        out.warnings
    );
    s.settings.forms.recipient = "contact@alunites.com".into();
    let out = build(&input(&s, vec![])).unwrap();
    let home = file(&out, "index.html");
    assert!(
        home.contains(r#"data-revel-form="contact" action="/revel-form.php" method="post""#),
        "{home}"
    );
    assert!(home.contains("_revel_hp") && home.contains("document.addEventListener(\"submit\""));
    let php = file(&out, "revel-form.php");
    assert!(
        php.contains("const RECIPIENT = 'contact@alunites.com';")
            && php.contains("const FROM = 'site@alunites.com';")
    );
}

/// Feuille de style reliée aux pages qui ne la relient pas (page sans gabarit, feuille créée au
/// premier style), une seule fois.
#[test]
fn feuille_de_style_reliee() {
    let mut s = Site::new("Essai", "fr");
    s.pages.push(Page {
        id: "index".into(),
        path: "/".into(),
        layout: None,
        props: Default::default(),
        computed: Default::default(),
        body: parse(
            "<!doctype html><html><head><title>A</title></head><body><h1>A</h1></body></html>",
        ),
        draft: false,
        code: Default::default(),
        name: None,
    });
    let out = build(&input(&s, vec![])).unwrap();
    assert!(!file(&out, "index.html").contains("stylesheet"));
    s.stylesheet = Some(revel_model::Stylesheet {
        url: "/assets/site.css".into(),
        source: None,
        items: vec![],
    });
    let out = build(&input(&s, vec![])).unwrap();
    let home = file(&out, "index.html");
    assert_eq!(
        home.matches(r#"<link rel="stylesheet" href="/assets/site.css">"#)
            .count(),
        1,
        "{home}"
    );
    let out2 = build(&input(&s, vec![])).unwrap();
    assert_eq!(file(&out2, "index.html"), home);
}

#[test]
fn animations_produites_si_utilisees() {
    let page = |body: &str| Page {
        id: "index".into(),
        path: "/".into(),
        layout: None,
        props: Default::default(),
        computed: Default::default(),
        body: parse(body),
        draft: false,
        code: Default::default(),
        name: None,
    };
    let mut s = Site::new("Essai", "fr");
    s.pages.push(page(
        "<!doctype html><html><head><title>A</title></head><body><h1>A</h1></body></html>",
    ));
    let out = build(&input(&s, vec![])).unwrap();
    assert!(
        !out.files
            .iter()
            .any(|f| f.path == "assets/revel-animations.js")
    );
    assert!(!file(&out, "index.html").contains("revel-animations"));
    s.pages[0] = page(
        "<!doctype html><html><head><title>A</title></head><body><h1 data-revel-appear=\"up dur=0.8\">A</h1></body></html>",
    );
    let out = build(&input(&s, vec![])).unwrap();
    let home = file(&out, "index.html");
    assert!(
        home.contains("<script src=\"/assets/revel-animations.js\" defer></script>"),
        "{home}"
    );
    assert!(home.contains("data-revel-appear=\"up dur=0.8\""));
    let js = file(&out, "assets/revel-animations.js");
    assert!(js.contains("RevelAnimations") && !js.contains("/*"));
}
