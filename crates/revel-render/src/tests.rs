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
            "sitemap.xml"
        ]
    );

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
