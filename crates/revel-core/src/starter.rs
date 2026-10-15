//! Site de départ d'un nouveau site : gabarit de base (en-tête de document, référencement),
//! page d'accueil, feuille de style, nom du site dans les données, dossier des médias.
//!
//! Les conventions sont celles des paramètres de page de Revel (D-32) : `seo.title`,
//! `seo.description`, `seo.image` ; `noindex` est traité à la génération.

use revel_model::{Layout, Node, Page, Passthrough, Site, Stylesheet};
use serde_json::json;

const BASE: &str = r#"<!doctype html>
<html lang="{{ site.lang }}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{{ (seo.title ~ " | " ~ site.title) if seo and seo.title else site.title }}</title>
{%- if seo and seo.description %}
<meta name="description" content="{{ seo.description }}">
{%- endif %}
<meta property="og:title" content="{{ seo.title if seo and seo.title else site.title }}">
{%- if seo and seo.image %}
<meta property="og:image" content="{{ seo.image }}">
{%- endif %}
<link rel="stylesheet" href="/assets/site.css">
</head>
<body>
{{ content | safe }}
</body>
</html>
"#;

const HOME: &str = r#"<main class="page">
<section class="section">
<h1 class="title">{{ site.title }}</h1>
<p class="lead">Première page du site.</p>
</section>
</main>"#;

const CSS: &str = r#"*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; font-family: system-ui, sans-serif; line-height: 1.5; color: #1b1b1b; background: #ffffff; }
img { max-width: 100%; height: auto; }
.page { min-height: 100vh; }
.section { max-width: 1120px; margin: 0 auto; padding: 96px 24px; }
.title { margin: 0 0 16px; font-size: 48px; line-height: 1.1; }
.lead { margin: 0; font-size: 20px; color: #555555; }
@media (max-width: 767px) {
  .section { padding: 64px 20px; }
  .title { font-size: 36px; }
}
"#;

fn parse(src: &str) -> Vec<Node> {
    let mut nodes = revel_markup::parse(
        src,
        &revel_markup::Options {
            dialect: revel_markup::Dialect::Nunjucks,
            ..Default::default()
        },
    )
    .map(|p| p.nodes)
    .unwrap_or_default();
    slot(&mut nodes);
    nodes
}

/// `{{ content | safe }}` -> emplacement de la page.
fn slot(nodes: &mut [Node]) {
    for n in nodes {
        if let Node::Out { expr } = n
            && expr.split_whitespace().collect::<String>() == "content|safe"
        {
            *n = Node::Slot;
        }
        for list in revel_model::ids::child_lists_mut(n) {
            slot(list);
        }
    }
}

/// Nouveau site prêt à modifier.
pub fn starter_site(name: &str, lang: &str) -> Site {
    let mut s = Site::new(name, lang);
    s.data.insert(
        "site".into(),
        json!({ "title": name, "lang": lang, "description": "" }),
    );
    s.passthrough = vec![Passthrough {
        from: "media".into(),
        to: "media".into(),
    }];
    s.layouts.push(Layout {
        id: "base".into(),
        parent: None,
        props: Default::default(),
        body: parse(BASE),
    });
    s.pages.push(Page {
        id: "index".into(),
        path: "/".into(),
        layout: Some("base".into()),
        props: Default::default(),
        computed: Default::default(),
        body: parse(HOME),
        draft: false,
        name: Some(
            if lang.starts_with("fr") {
                "Accueil"
            } else {
                "Home"
            }
            .into(),
        ),
    });
    s.stylesheet = Some(Stylesheet {
        url: "/assets/site.css".into(),
        source: None,
        items: revel_style::parse(CSS).unwrap_or_default(),
    });
    s.ensure_ids();
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_de_depart() {
        let s = starter_site("Atelier", "fr");
        assert_eq!(s.pages.len(), 1);
        assert!(
            s.layouts[0]
                .body
                .iter()
                .any(|n| matches!(n, Node::Element { tag, .. } if tag == "html"))
        );
        let mut slots = 0;
        revel_model::ids::walk(&s.layouts[0].body, &mut |n| {
            if matches!(n, Node::Slot) {
                slots += 1
            }
        });
        assert_eq!(slots, 1);
        assert!(!s.stylesheet.as_ref().unwrap().items.is_empty());
        assert!(s.node_ids().len() > 5);
    }
}
