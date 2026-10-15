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
    s.components.push(Component {
        id: "header".into(),
        body: parse(r#"<header><a href="/"{% if page.url == "/" %} aria-current="page"{% endif %}>Accueil</a></header>"#),
    });
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
