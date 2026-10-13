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
