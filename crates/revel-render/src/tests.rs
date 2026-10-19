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
        fields: Vec::new(),
        id: "base".into(),
        parent: None,
        props: Default::default(),
        body: parse(
            r#"<!doctype html><html><head><title>{{ seo | seo_title(site) }}</title></head><body class="{{ bodyClass }}">{% include "header" %}{{ content | safe }}</body></html>"#,
        ),
    });
    s.layouts.push(Layout {
        fields: Vec::new(),
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
    assert!(
        output_path("//greek/a/").is_err(),
        "chemin absolu une fois joint"
    );
    assert!(output_path("/a\\b/").is_err());
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
    // Page indexée : pas de noindex, aperçus d'image en grand permis.
    let home = file(&out, "index.html");
    assert!(
        !home.contains("noindex") && home.contains("max-image-preview:large"),
        "{home}"
    );
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
        ..Default::default()
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
            exclude_current: false,
            pick: Vec::new(),
            contains: None,
            first: None,
            first_full: false,
            also: Vec::new(),
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
fn lottie_interactions_et_transitions() {
    let mut s = site();
    s.settings.page_transition = Some("fade".into());
    s.pages[0].body.extend(parse(
        r#"<div class="lottie" data-lottie="/media/a.json" data-lottie-loop></div><button data-revel-on="click class:is-open #menu">Menu</button>"#,
    ));
    let out = build(&input(&s, vec![])).unwrap();
    let home = file(&out, "index.html");
    assert!(home.contains("/assets/revel-lottie.js"), "{home}");
    assert!(home.contains("/assets/revel-actions.js"), "{home}");
    assert!(home.contains("@view-transition{navigation:auto}"), "{home}");
    for f in ["assets/revel-lottie.js", "assets/revel-actions.js"] {
        assert!(out.files.iter().any(|x| x.path == f), "{f}");
    }
    // Sans animation Lottie ni interaction : aucun script produit.
    let out = build(&input(&site(), vec![])).unwrap();
    assert!(
        !out.files
            .iter()
            .any(|x| x.path.contains("revel-lottie") || x.path.contains("revel-actions"))
    );
    assert!(!file(&out, "index.html").contains("view-transition"));
}

#[test]
fn dates_en_toutes_lettres() {
    use chrono::TimeZone;
    let d = chrono::Utc.with_ymd_and_hms(2026, 10, 16, 0, 0, 0).unwrap();
    assert_eq!(crate::eval::date_text(&d, "fr", "long"), "16 octobre 2026");
    assert_eq!(crate::eval::date_text(&d, "fr", "month"), "octobre 2026");
    assert_eq!(crate::eval::date_text(&d, "fr", "short"), "16 oct. 2026");
    assert_eq!(crate::eval::date_text(&d, "en", "long"), "October 16, 2026");
    assert_eq!(
        crate::eval::date_text(&d, "de-CH", "long"),
        "16. Oktober 2026"
    );
    // Filtre dans une page ; liste sans le contenu affiché.
    let mut s = site();
    s.pages[0].body.extend(parse(
        r#"<p id="d">{{ "2026-10-16" | date_text("fr", "long") }}</p><p id="v">{{ none | date_text("fr") }}</p>"#,
    ));
    let out = build(&input(&s, vec![])).unwrap();
    let home = file(&out, "index.html");
    assert!(home.contains(r#"<p id="d">16 octobre 2026</p>"#), "{home}");
    assert!(home.contains(r#"<p id="v"></p>"#), "{home}");
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
    assert!(
        l.contains(r#"<link rel="canonical" href="https://alunites.com/en/langues/">"#),
        "{l}"
    );
    assert!(l.contains(r#"<a href="/langues/" hreflang="fr">Français</a><a href="/en/langues/" hreflang="en" aria-current="page">English</a><a href="/en/project/a/#x">a</a><a href="/media/f.pdf">f</a><p>en</p>"#), "{l}");
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

#[test]
fn contenus_en_brouillon_ecartes_de_la_generation() {
    let mut s = site();
    for c in s.collections.iter_mut() {
        c.filter = None;
    }
    let out = build(&input(
        &s,
        vec![
            entry(
                "a",
                json!({"slug": "a", "title": "Publié", "date": "2026-01-02"}),
            ),
            entry(
                "b",
                json!({"slug": "b", "title": "Brouillon", "draft": true}),
            ),
            entry(
                "c",
                json!({"slug": "c", "title": "Masqué", "published": false}),
            ),
        ],
    ))
    .unwrap();
    let paths: Vec<&str> = out.files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"project/a/index.html"), "{paths:?}");
    assert!(
        !paths
            .iter()
            .any(|p| p.starts_with("project/b") || p.starts_with("project/c")),
        "{paths:?}"
    );
    assert!(!file(&out, "index.html").contains("Brouillon"));
}

/// Calque masqué : absent du site publié, gardé (marqué) dans l'éditeur ; nom de calque jamais
/// écrit.
#[test]
fn calques_masques_et_noms() {
    let mut s = site();
    s.pages[0].body = parse(
        r#"<section data-revel-name="Bandeau"><p data-revel-hidden>secret</p><p>visible</p></section>"#,
    );
    s.pages[0].body.push(Node::Image(revel_model::ImageNode {
        src: "\"/media/a.jpg\"".into(),
        hidden: true,
        name: Some("Photo".into()),
        ..Default::default()
    }));
    let out = build(&input(&s, vec![])).unwrap();
    let home = file(&out, "index.html");
    assert!(home.contains("<section><p>visible</p></section>"), "{home}");
    assert!(
        !home.contains("secret") && !home.contains("a.jpg") && !home.contains("data-revel-name")
    );
    let e = render_editor(&input(&s, vec![]), "/", &EditorOptions { scripts: false })
        .unwrap()
        .unwrap();
    assert!(
        e.html.contains("data-revel-hidden") && e.html.contains("secret"),
        "{}",
        e.html
    );
    assert!(!e.html.contains("data-revel-name"));
}

/// Reconstruction dans la forme de l'interface (D-90) : même rendu ; variables, boucle de
/// contenus choisis et listes de données remplacées par des éléments écrits et une liste de
/// collection réglable.
#[test]
fn reconstruction_meme_rendu() {
    let mut s = site();
    s.pages[0].body.extend(parse(
        r#"{% set chiffres = [{"n": "20", "l": "ans"}, {"n": "45", "l": "experts"}] %}<dl>{% for c in chiffres %}<dt>{{ c.l }}</dt><dd>+{{ c.n }}</dd>{% endfor %}</dl>"#,
    ));
    let entries = || {
        vec![
            entry(
                "a",
                json!({"slug": "a", "title": "Projet A", "date": "2026-01-02"}),
            ),
            entry(
                "b",
                json!({"slug": "b", "title": "Projet B", "date": "2026-03-01"}),
            ),
        ]
    };
    let before = build(&input(&s, entries())).unwrap();
    let r = crate::flatten::rebuild(&input(&s, entries())).unwrap();
    let after = build(&input(&r.site, entries())).unwrap();
    assert_eq!(file(&before, "index.html"), file(&after, "index.html"));
    let issues = revel_edit::editable::audit(&r.site);
    // Classe du premier élément d'une liste : pas de forme dans l'interface (style « premier »).
    let page: Vec<_> = issues
        .iter()
        .filter(|i| i.owner == "page:index" && !i.detail.contains("loop.first"))
        .collect();
    assert!(page.is_empty(), "{page:?}");
    let json = serde_json::to_string(&r.site.pages[0].body).unwrap();
    assert!(
        json.contains(r#"collections.projets | by_slugs([\"b\", \"zz\", \"a\"])"#),
        "{json}"
    );
    assert!(json.contains("experts") && !json.contains("chiffres"));
}

/// Paramètres de la page : classe du corps de page et thème ajoutés par Revel ; référencement
/// écrit quand le gabarit ne l'écrit pas.
#[test]
fn apparence_et_referencement_de_la_page() {
    let mut s = Site::new("Test", "fr");
    s.base_url = Some("https://exemple.fr".into());
    s.layouts.push(revel_edit::make::base_layout(&s));
    let lid = s.layouts[0].id.clone();
    let mut p: Page = serde_json::from_value(json!({
        "id": "agence", "path": "/agence/", "layout": lid,
        "props": {
            "bodyClass": "page-agence",
            "theme": "light",
            "seo": { "title": "L'agence", "description": "Qui nous sommes", "image": "/media/a.jpg" }
        }
    }))
    .unwrap();
    p.body = parse("<h1>Agence</h1>");
    s.pages.push(p);
    let out = build(&input(&s, vec![])).unwrap();
    let html = file(&out, "agence/index.html");
    assert!(
        html.contains(r#"<html lang="fr" data-theme="light">"#),
        "{html}"
    );
    assert!(html.contains(r#"<body class="page-agence">"#), "{html}");
    for t in [
        r#"<meta name="description" content="Qui nous sommes">"#,
        r#"<link rel="canonical" href="https://exemple.fr/agence/">"#,
        r#"<meta property="og:image" content="https://exemple.fr/media/a.jpg">"#,
        r#"<meta property="og:type" content="website">"#,
    ] {
        assert!(html.contains(t), "{t}\n{html}");
    }
}

#[test]
fn liens_page_actuelle_et_flux() {
    let mut s = Site::new("Test", "fr");
    s.base_url = Some("https://exemple.fr".into());
    s.layouts.push(revel_edit::make::base_layout(&s));
    let lid = s.layouts[0].id.clone();
    let mut p: Page = serde_json::from_value(json!({
        "id": "projet", "path": "/projets/un/", "layout": lid
    }))
    .unwrap();
    p.body = parse(
        r#"<nav><a href="/">Accueil</a><a href="/projets/" data-revel-current="section">Projets</a><a href="/projets/un/">Un</a><a href="/agence/">Agence</a></nav>"#,
    );
    s.pages.push(p);
    let out = build(&input(&s, vec![])).unwrap();
    let html = file(&out, "projets/un/index.html");
    assert!(html.contains(r#"<a href="/">Accueil</a>"#), "{html}");
    assert!(
        html.contains(r#"<a href="/projets/" aria-current="page">Projets</a>"#),
        "{html}"
    );
    assert!(
        html.contains(r#"<a href="/projets/un/" aria-current="page">Un</a>"#),
        "{html}"
    );
    assert!(html.contains(r#"<a href="/agence/">Agence</a>"#), "{html}");
    assert!(!html.contains("data-revel-current"), "{html}");
}

#[test]
fn reconstruction_lien_actuel_theme_et_flux() {
    let mut s = Site::new("Test", "fr");
    s.base_url = Some("https://exemple.fr".into());
    let mut l = revel_edit::make::base_layout(&s);
    crate::flatten::walk_mut(&mut l.body, &mut |n| {
        if let Node::Element { tag, attrs, .. } = n {
            let (name, value) = match tag.as_str() {
                "body" => ("class", "{{ bodyClass }}"),
                "html" => ("data-theme", "{{ theme or 'dark' }}"),
                _ => return,
            };
            attrs.push(revel_model::Attr {
                name: name.into(),
                value: Some(value.into()),
                when: None,
            });
        }
    });
    let lid = l.id.clone();
    s.layouts.push(l);
    let mut p: Page = serde_json::from_value(json!({
        "id": "agence", "path": "/agence/", "layout": lid, "props": { "bodyClass": "page-agence" }
    }))
    .unwrap();
    p.body = parse(
        r#"{% set nav = [{"url": "/agence/", "label": "Agence"}, {"url": "/projets/", "label": "Projets"}] %}<nav>{% for item in nav %}<a href="{{ item.url }}"{% if (page.url or "") is startingwith(item.url) %} aria-current="page"{% endif %}>{{ item.label }}</a>{% endfor %}</nav>"#,
    );
    s.pages.push(p);
    let mut feed: Page =
        serde_json::from_value(json!({ "id": "feed", "path": "/feed.xml" })).unwrap();
    feed.body = revel_markup::parse(
        r#"<rss version="2.0"><channel><title>Actus</title><link>https://exemple.fr/actus/</link>{% for item in (collections.actus | sort(attribute="date", reverse=true)) | limit(5) %}<item><title>{{ item.data.title }}</title><link>{{ item.url | absolute_url("https://exemple.fr") }}</link></item>{% endfor %}</channel></rss>"#,
        &revel_markup::Options {
            mode: revel_markup::Mode::Xml,
            ..Default::default()
        },
    )
    .unwrap()
    .nodes;
    s.pages.push(feed);
    s.collections.push(
        serde_json::from_value(json!({
            "id": "actus", "folder": "content/actus", "path": "/actus/{{ slug }}/"
        }))
        .unwrap(),
    );
    let entries: Vec<Entry> = ["a", "b"]
        .iter()
        .enumerate()
        .map(|(i, slug)| Entry {
            folder: "content/actus".into(),
            file_stem: slug.to_string(),
            data: json!({ "title": slug.to_uppercase(), "date": format!("2026-0{}-01", i + 1) }),
            body: String::new(),
        })
        .collect();
    let inp = input(&s, entries.clone());
    let before = build(&inp).unwrap();
    let rb = crate::flatten::rebuild(&inp).unwrap();
    assert!(
        rb.site.pages.iter().all(|p| p.path != "/feed.xml"),
        "{:?}",
        rb.notes
    );
    assert_eq!(rb.site.settings.feeds.len(), 1);
    assert_eq!(rb.site.settings.feeds[0].limit, Some(5));
    assert_eq!(rb.site.settings.feeds[0].link.as_deref(), Some("/actus/"));
    let after = build(&input(&rb.site, entries)).unwrap();
    let a = file(&before, "agence/index.html");
    // Même page ; le flux devenu réglage du site est annoncé dans l'en-tête.
    let b = file(&after, "agence/index.html");
    // Titre écrit par le gabarit, puis d'après les réglages : même balise, ailleurs dans
    // l'en-tête.
    let title = "<title>Test</title>";
    assert!(a.contains(title) && b.contains(title), "{b}");
    assert_eq!(
        a.replace(title, ""),
        b.replace(
            r#"<link rel="alternate" type="application/rss+xml" title="Actus" href="/feed.xml">"#,
            ""
        )
        .replace(title, "")
    );
    assert!(
        b.contains(r#"<a href="/agence/" aria-current="page">Agence</a>"#),
        "{b}"
    );
    assert!(b.contains(r#"<body class="page-agence">"#), "{b}");
    assert!(b.contains(r#"data-theme="dark""#), "{b}");
    let json = serde_json::to_string(&rb.site.pages).unwrap();
    assert!(
        json.contains("data-revel-current") && !json.contains("startingwith"),
        "{json}"
    );
    let f = file(&after, "feed.xml");
    assert!(
        f.find("<title>B</title>").unwrap() < f.find("<title>A</title>").unwrap(),
        "{f}"
    );
}

/// Réglages du site : titre complété du nom du site, partage, icônes, organisation, article.
#[test]
fn partage_et_navigateur_du_site() {
    let mut s = Site::new("Test", "fr");
    s.base_url = Some("https://exemple.fr".into());
    let mut l = revel_edit::make::base_layout(&s);
    // Gabarit sans titre : Revel l'écrit.
    crate::flatten::walk_mut(&mut l.body, &mut |n| {
        if let Node::Element { tag, children, .. } = n
            && tag == "head"
        {
            children.retain(|c| !matches!(c, Node::Element { tag, .. } if tag == "title"));
        }
    });
    let lid = l.id.clone();
    s.layouts.push(l);
    s.settings.seo.site_title = Some("Agence Test".into());
    s.settings.seo.share_image = Some("/media/partage.jpg".into());
    s.settings.seo.theme_color = Some("#111111".into());
    s.settings.seo.favicon = Some("/media/icone.svg".into());
    s.settings.seo.organization = Some(revel_model::Organization {
        name: "Test".into(),
        city: "Lyon".into(),
        same_as: vec!["https://exemple.social/test".into()],
        ..Default::default()
    });
    let mut p: Page = serde_json::from_value(json!({
        "id": "actu", "path": "/actu/", "layout": lid,
        "props": { "seo": { "title": "Une actu" }, "ogType": "article", "date": "2026-03-01" }
    }))
    .unwrap();
    p.body = parse("<p>x</p>");
    s.pages.push(p);
    let out = build(&input(&s, vec![])).unwrap();
    let h = file(&out, "actu/index.html");
    for t in [
        "<title>Une actu | Agence Test</title>",
        r#"<meta property="og:title" content="Une actu | Agence Test">"#,
        r#"<meta property="og:site_name" content="Agence Test">"#,
        r#"<meta property="og:image" content="https://exemple.fr/media/partage.jpg">"#,
        r#"<meta name="twitter:image" content="https://exemple.fr/media/partage.jpg">"#,
        r#"<meta property="og:type" content="article">"#,
        r#"<meta property="article:published_time" content="2026-03-01T00:00:00.000Z">"#,
        r##"<meta name="theme-color" content="#111111">"##,
        r#"<link rel="icon" href="/media/icone.svg" type="image/svg+xml">"#,
        r#""@type":"Organization""#,
        r#""addressLocality":"Lyon""#,
        r#""@type":"Article""#,
    ] {
        assert!(h.contains(t), "{t}\n{h}");
    }
    // Balise déjà écrite dans le code ajouté de la page : pas écrite une seconde fois.
    s.pages[0].code.head = r#"<meta property="og:title" content="Autre titre">"#.into();
    let out = build(&input(&s, vec![])).unwrap();
    let h = file(&out, "actu/index.html");
    assert_eq!(h.matches("og:title").count(), 1, "{h}");
    assert!(h.contains("Autre titre"));
}

/// En-tête écrit à la main (gabarit, code ajouté d'une page) : réglages du site et de la page,
/// site produit identique.
#[test]
fn en_tete_ecrit_a_la_main_devient_reglages() {
    let mut s = Site::new("Atelier", "fr");
    s.base_url = Some("https://exemple.fr".into());
    s.layouts.push(Layout {
        fields: Vec::new(),
        id: "base".into(),
        parent: None,
        props: Default::default(),
        body: parse(
            r##"<!doctype html><html lang="fr"><head><meta charset="utf-8"><title>{{ seo.title if seo and seo.title else "Atelier" }}</title><meta property="og:site_name" content="Atelier"><meta name="theme-color" content="#123456"><link rel="icon" href="/icone.svg" type="image/svg+xml"><link rel="stylesheet" href="/s.css"></head><body>{{ content | safe }}</body></html>"##,
        ),
    });
    let mut p: Page = serde_json::from_value(json!({
        "id": "contact", "path": "/contact/", "layout": "base",
        "props": { "seo": { "title": "Contact | Atelier" } }
    }))
    .unwrap();
    p.body = parse("<p>Écrire</p>");
    p.code.head = concat!(
        r#"<meta property="og:title" content="Contact | Atelier">"#,
        "\n",
        r#"<meta property="og:image" content="https://exemple.fr/media/a.jpg">"#,
        "\n",
        r#"<link rel="canonical" href="https://exemple.fr/contact/">"#,
        "\n",
        r#"<script>suivi()</script>"#
    )
    .into();
    s.pages.push(p);
    let before = build(&input(&s, vec![])).unwrap();
    let r = crate::flatten::rebuild(&input(&s, vec![])).unwrap();
    let seo = &r.site.settings.seo;
    assert_eq!(seo.theme_color.as_deref(), Some("#123456"));
    assert_eq!(seo.favicon.as_deref(), Some("/icone.svg"));
    assert_eq!(seo.site_title.as_deref(), Some("Atelier"));
    let page = &r.site.pages[0];
    assert_eq!(page.props["seo"]["title"], "Contact");
    assert_eq!(page.props["seo"]["image"], "/media/a.jpg");
    assert_eq!(page.code.head, "<script>suivi()</script>");
    let head = serde_json::to_string(&r.site.layouts[0].body).unwrap();
    assert!(
        !head.contains("theme-color") && !head.contains("og:site_name"),
        "{head}"
    );
    assert!(!head.contains("\"title\""), "{head}");
    // Même site produit (en-tête comparé sans l'ordre des balises).
    let after = build(&input(&r.site, vec![])).unwrap();
    let h1 = file(&before, "contact/index.html");
    let h2 = file(&after, "contact/index.html");
    let tags = |h: &str| {
        let mut v: Vec<String> = h[..h.find("</head>").unwrap()]
            .split('<')
            .skip(1)
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty() && !t.starts_with('/'))
            .collect();
        v.sort();
        v
    };
    // Balises d'avant toutes là ; en plus, celles que l'image de partage réglée apporte.
    let (t1, t2) = (tags(h1), tags(h2));
    let extra: Vec<&String> = t2.iter().filter(|t| !t1.contains(t)).collect();
    assert!(t1.iter().all(|t| t2.contains(t)), "{h1}\n{h2}");
    assert_eq!(
        extra,
        [
            r#"meta name="twitter:card" content="summary_large_image">"#,
            r#"meta name="twitter:image" content="https://exemple.fr/media/a.jpg">"#
        ]
    );
    assert_eq!(h2.matches("og:title").count(), 1);
}

/// Variable de page lue par le gabarit (`{% if not cacherAppel %}`, `{{ sousTitre }}`) :
/// propriété du gabarit à la reconstruction, valeur gardée par chaque page ; la valeur par défaut
/// d'une propriété vaut pour les pages qui n'en donnent pas ; réglage de page tiré des données
/// (`seo = pages.x.seo`) écrit dans la page.
#[test]
fn proprietes_du_gabarit_et_reglages_tires_des_donnees() {
    let mut s = site();
    s.layouts[0].body = parse(
        r#"<!doctype html><html><head><title>{{ seo.title }}</title></head><body>{% if not cacherAppel %}<section class="appel">Écrivez-nous</section>{% endif %}<p class="st">{{ sousTitre }}</p>{{ content | safe }}</body></html>"#,
    );
    s.layouts[0]
        .props
        .insert("sousTitre".into(), json!("Agence"));
    s.data.insert(
        "pages".into(),
        json!({"contact": {"seo": {"title": "Contact"}}}),
    );
    s.pages.push(Page {
        id: "contact".into(),
        path: "/contact/".into(),
        layout: Some("base".into()),
        props: [("cacherAppel".to_string(), json!(true))].into(),
        body: parse("<main>Contact</main>"),
        computed: [("seo".to_string(), r#"pages["contact"].seo"#.to_string())].into(),
        draft: false,
        code: Default::default(),
        name: None,
    });
    let before = build(&input(&s, vec![])).unwrap();
    let r = crate::flatten::rebuild(&input(&s, vec![])).unwrap();
    let after = build(&input(&r.site, vec![])).unwrap();
    for f in ["index.html", "contact/index.html"] {
        assert_eq!(file(&before, f), file(&after, f), "{f}");
    }
    assert!(!file(&after, "contact/index.html").contains("appel"));
    assert!(file(&after, "index.html").contains("Agence"));
    let base = &r.site.layouts[0];
    let names: Vec<(&str, revel_model::PropKind)> = base
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.kind))
        .collect();
    assert!(
        names.contains(&("cacherAppel", revel_model::PropKind::Switch)),
        "{names:?}"
    );
    assert!(
        names.contains(&("sousTitre", revel_model::PropKind::Text)),
        "{names:?}"
    );
    assert_eq!(
        base.field("sousTitre").and_then(|f| f.default.clone()),
        Some(json!("Agence"))
    );
    assert!(!base.props.contains_key("sousTitre"));
    let contact = r.site.pages.iter().find(|p| p.id == "contact").unwrap();
    assert!(contact.computed.is_empty());
    assert_eq!(contact.props.get("seo"), Some(&json!({"title": "Contact"})));
    let left: Vec<_> = revel_edit::editable::audit(&r.site)
        .into_iter()
        .filter(|i| {
            i.detail.contains("cacherAppel")
                || i.detail.contains("sousTitre")
                || i.detail.contains("seo")
        })
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

/// Plan du site écrit à la main (boucle sur une collection, dates) : remplacé à la
/// reconstruction par celui de Revel avec les dates de mise à jour (mêmes adresses et dates).
#[test]
fn plan_du_site_ecrit_a_la_main_devient_reglage() {
    let mut s = site();
    s.base_url = Some("https://alunites.com".into());
    s.pages.retain(|p| p.path != "/sitemap.xml");
    s.pages.push(Page {
        id: "sitemap".into(),
        path: "/sitemap.xml".into(),
        layout: None,
        props: Default::default(),
        // Forme de l'import (page XML) : prologue écrit, liste sur la collection des pages.
        body: serde_json::from_value(json!([
            {"node": "html", "html": "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"},
            {"node": "text", "text": "\n"},
            {"node": "element", "tag": "urlset",
             "attrs": [{"name": "xmlns", "value": "http://www.sitemaps.org/schemas/sitemap/0.9"}],
             "children": [{"node": "each", "expr": "collections.sitemap", "item": "p", "children": [
                {"node": "text", "text": "\n  "},
                {"node": "element", "tag": "url", "children": [
                    {"node": "text", "text": "\n    "},
                    {"node": "element", "tag": "loc", "children": [{"node": "out", "expr": "p.url | absolute_url(\"https://alunites.com\")"}]},
                    {"node": "if", "cond": "p.data.date", "then": [
                        {"node": "element", "tag": "lastmod", "children": [{"node": "out", "expr": "p.date | iso_date"}]}]},
                    {"node": "text", "text": "\n  "}]}]},
                {"node": "text", "text": "\n"}]}
        ]))
        .unwrap(),
        computed: Default::default(),
        draft: false,
        code: Default::default(),
        name: None,
    });
    let entries = || {
        vec![
            entry(
                "a",
                json!({"slug": "a", "title": "A", "date": "2026-01-02"}),
            ),
            entry(
                "b",
                json!({"slug": "b", "title": "B", "date": "2026-03-01T10:00:00Z"}),
            ),
        ]
    };
    let before = build(&input(&s, entries())).unwrap();
    let r = crate::flatten::rebuild(&input(&s, entries())).unwrap();
    assert!(
        !r.site.pages.iter().any(|p| p.path == "/sitemap.xml"),
        "{:?}",
        r.notes
    );
    assert!(
        r.site.settings.seo.sitemap_lastmod && !r.site.settings.seo.no_sitemap,
        "{:?}",
        r.notes
    );
    let after = build(&input(&r.site, entries())).unwrap();
    let pairs = |xml: &str| {
        let re = regex::Regex::new(
            r"(?s)<url>\s*<loc>(.*?)</loc>(?:<lastmod>(.*?)</lastmod>)?\s*</url>",
        )
        .unwrap();
        let mut v: Vec<(String, String)> = re
            .captures_iter(xml)
            .map(|c| {
                (
                    c[1].to_string(),
                    c.get(2).map(|m| m.as_str().to_string()).unwrap_or_default(),
                )
            })
            .collect();
        v.sort();
        v
    };
    let (a, b) = (
        pairs(file(&before, "sitemap.xml")),
        pairs(file(&after, "sitemap.xml")),
    );
    assert_eq!(a, b);
    assert!(
        b.iter()
            .any(|(u, d)| u.ends_with("/project/b/") && d == "2026-03-01T10:00:00.000Z"),
        "{b:?}"
    );
}

/// « Suggestions du contenu s'il y en a assez, sinon les plus récents » écrit avec des variables :
/// liste de collection « Mettre en premier, seulement s'ils remplissent la liste » à la
/// reconstruction, même rendu.
#[test]
fn suggestions_sinon_les_plus_recents() {
    let mut s = site();
    s.layouts[1].body = parse(
        r#"{%- set others = (collections.projets | by_slugs(suggestions)) if suggestions and suggestions|length else [] -%}
{%- if others|length < 2 %}{% set others = collections.projets | without(page.url) | limit(2) %}{% endif %}
<article><h1>{{ title }}</h1>{{ content | safe }}{% if others|length %}<ul>{% for item in others | limit(2) %}<li>{{ item.data.title }}</li>{% endfor %}</ul>{% endif %}</article>"#,
    );
    let entries = || {
        vec![
            entry(
                "a",
                json!({"slug": "a", "title": "A", "date": "2026-01-01", "suggestions": ["c", "b"]}),
            ),
            entry(
                "b",
                json!({"slug": "b", "title": "B", "date": "2026-02-01", "suggestions": ["c"]}),
            ),
            entry(
                "c",
                json!({"slug": "c", "title": "C", "date": "2026-03-01"}),
            ),
        ]
    };
    let before = build(&input(&s, entries())).unwrap();
    let r = crate::flatten::rebuild(&input(&s, entries())).unwrap();
    let after = build(&input(&r.site, entries())).unwrap();
    for f in [
        "project/a/index.html",
        "project/b/index.html",
        "project/c/index.html",
    ] {
        assert_eq!(file(&before, f), file(&after, f), "{f}");
    }
    assert!(file(&after, "project/a/index.html").contains("<li>C</li><li>B</li>"));
    let body = serde_json::to_string(&r.site.layouts[1].body).unwrap();
    assert!(body.contains("pick_first(suggestions, 2)"), "{body}");
    assert!(!body.contains("\"set\""), "{body}");
    let left: Vec<_> = revel_edit::editable::audit(&r.site)
        .into_iter()
        .filter(|i| i.owner == "layout:project")
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

/// Contenus choisis un à un dans deux collections (`(projets | by_slug(s)) or (actualites |
/// by_slug(s))`), ajoutés à une liste, premier contenu lu à part : liste de collection à deux
/// collections, premier contenu dans une liste d'un seul contenu ; même rendu.
#[test]
fn contenus_choisis_dans_deux_collections() {
    let mut s = site();
    s.collections.push(Collection {
        id: "actualites".into(),
        source: CollectionSource::Folder,
        folder: "content/actualites".into(),
        path: Some("/{{ slug }}/".into()),
        layout: Some("project".into()),
        props: Default::default(),
        filter: None,
        sort: None,
        fields: vec![],
    });
    s.pages[0].body = parse(
        r#"{%- set featured = [] -%}{%- for slug in ["b", "n1", "zz", "a"] -%}{%- set it = (collections.projets | by_slug(slug)) or (collections.actualites | by_slug(slug)) -%}{%- if it %}{% set featured = (featured.push(it), featured) %}{% endif -%}{%- endfor -%}
{%- set first = featured[0] -%}
<div class="hero">{% if first and first.data.video %}<video data-src="{{ first.data.video }}"></video>{% endif %}</div>
<ol>{% for it in featured %}<li><a href="{{ it.url }}">{{ it.data.title }}</a></li>{% endfor %}</ol>"#,
    );
    let entries = || {
        let mut v = vec![
            entry(
                "a",
                json!({"slug": "a", "title": "A", "date": "2026-01-02"}),
            ),
            entry(
                "b",
                json!({"slug": "b", "title": "B", "date": "2026-03-01", "video": "/media/b.mp4"}),
            ),
        ];
        v.push(Entry {
            folder: "content/actualites".into(),
            file_stem: "n1".into(),
            data: json!({"slug": "n1", "title": "Nouvelle", "date": "2026-02-01"}),
            body: String::new(),
        });
        v
    };
    let before = build(&input(&s, entries())).unwrap();
    let r = crate::flatten::rebuild(&input(&s, entries())).unwrap();
    let after = build(&input(&r.site, entries())).unwrap();
    assert_eq!(file(&before, "index.html"), file(&after, "index.html"));
    assert!(file(&after, "index.html").contains(r#"<video data-src="/media/b.mp4">"#));
    let body = serde_json::to_string(&r.site.pages[0].body).unwrap();
    assert!(
        body.contains(r#"(collections.projets + collections.actualites) | by_slugs([\"b\", \"n1\", \"zz\", \"a\"])"#),
        "{body}"
    );
    let left: Vec<_> = revel_edit::editable::audit(&r.site)
        .into_iter()
        .filter(|i| i.owner == "page:index")
        .collect();
    assert!(left.is_empty(), "{left:?} {:?}", r.notes);
}

/// Contenu relié lu directement (`(collections.projets | by_slug(voisin)).data.title`) : liste
/// des contenus reliés par le champ à la reconstruction, même rendu.
#[test]
fn contenu_relie_lu_directement() {
    let mut s = site();
    s.layouts[1].body = parse(
        r#"<article><h1>{{ title }}</h1>{% if (collections.projets | by_slug(voisin)) %}<p class="voisin"><a href="{{ (collections.projets | by_slug(voisin)).url }}">{{ (collections.projets | by_slug(voisin)).data.title }}</a></p>{% endif %}</article>"#,
    );
    let entries = || {
        vec![
            entry(
                "a",
                json!({"slug": "a", "title": "A", "date": "2026-01-01", "voisin": "b"}),
            ),
            entry(
                "b",
                json!({"slug": "b", "title": "B", "date": "2026-02-01"}),
            ),
        ]
    };
    let before = build(&input(&s, entries())).unwrap();
    let r = crate::flatten::rebuild(&input(&s, entries())).unwrap();
    let after = build(&input(&r.site, entries())).unwrap();
    for f in ["project/a/index.html", "project/b/index.html"] {
        assert_eq!(file(&before, f), file(&after, f), "{f}");
    }
    assert!(file(&after, "project/a/index.html").contains(r#"<a href="/project/b/">B</a>"#));
    let left: Vec<_> = revel_edit::editable::audit(&r.site)
        .into_iter()
        .filter(|i| i.owner == "layout:project")
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

/// Valeur tirée d'un tableau gardée dans une variable du même nom, lue par un style : classe par
/// valeur du champ (réglable dans Styles), même rendu.
#[test]
fn valeur_de_tableau_dans_une_variable_lue_par_un_style() {
    let mut s = site();
    s.layouts[1].body = parse(
        r##"{%- set couleur = {"orange": "#F4684D", "violet": "#8C5CF0"}[couleur] -%}<article class="fiche" style="--fiche:{{ couleur }}"><h1>{{ title }}</h1></article>"##,
    );
    let entries = || {
        vec![
            entry(
                "a",
                json!({"slug": "a", "title": "A", "date": "2026-01-01", "couleur": "orange"}),
            ),
            entry(
                "b",
                json!({"slug": "b", "title": "B", "date": "2026-02-01", "couleur": "violet"}),
            ),
        ]
    };
    let r = crate::flatten::rebuild(&input(&s, entries())).unwrap();
    let after = build(&input(&r.site, entries())).unwrap();
    assert!(file(&after, "project/a/index.html").contains(r#"class="fiche fiche--orange""#));
    let css = serde_json::to_string(&r.site.stylesheet).unwrap();
    assert!(
        css.contains(".fiche--violet") && css.contains("#8C5CF0"),
        "{css}"
    );
    let left: Vec<_> = revel_edit::editable::audit(&r.site)
        .into_iter()
        .filter(|i| i.owner == "layout:project")
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

/// Site à plusieurs environnements de publication (production, recette) : ce qui dépend de
/// l'environnement (`{% if env.staging %}`) reste lu à la publication ; la reconstruction donne
/// les mêmes fichiers dans chaque environnement.
#[test]
fn reconstruction_garde_les_environnements() {
    let mut s = site();
    s.settings.environments = [
        ("production".to_string(), json!({"staging": false})),
        ("staging".to_string(), json!({"staging": true})),
    ]
    .into();
    s.layouts[0].body = parse(
        r#"<!doctype html><html><head><title>{{ seo.title }}</title>{% if env.staging %}<meta name="robots" content="noindex, nofollow">{% endif %}</head><body>{{ content | safe }}{% if env.staging %}<p class="recette">Recette</p>{% endif %}</body></html>"#,
    );
    s.pages.push(Page {
        id: "htaccess".into(),
        path: "/.htaccess".into(),
        layout: None,
        props: Default::default(),
        body: revel_markup::parse(
            "{%- if env.staging %}\nHeader always set X-Robots-Tag \"noindex, nofollow\"\n{%- endif %}\nOptions -Indexes\n",
            &revel_markup::Options {
                dialect: revel_markup::Dialect::Nunjucks,
                mode: revel_markup::Mode::Text,
            },
        )
        .unwrap()
        .nodes,
        computed: Default::default(),
        draft: false,
        code: Default::default(),
        name: None,
    });
    let staging = |site: &Site| {
        let mut inp = input(site, vec![]);
        inp.environment = "staging".into();
        build(&inp).unwrap()
    };
    let before = staging(&s);
    let r = crate::flatten::rebuild(&input(&s, vec![])).unwrap();
    let after = staging(&r.site);
    // Corps identique ; en-tête : mêmes balises (leur ordre peut changer).
    let body = |h: &str| h.split("</head>").nth(1).unwrap_or_default().to_string();
    assert_eq!(
        body(file(&before, "index.html")),
        body(file(&after, "index.html"))
    );
    assert_eq!(file(&before, ".htaccess"), file(&after, ".htaccess"));
    assert!(
        file(&after, "index.html").contains(r#"<meta name="robots" content="noindex, nofollow">"#)
    );
    assert!(file(&after, "index.html").contains("Recette"));
    assert!(file(&after, ".htaccess").contains("X-Robots-Tag"));
    let prod = build(&input(&r.site, vec![])).unwrap();
    assert!(!file(&prod, ".htaccess").contains("X-Robots-Tag"));
}
