//! Site de départ d'un nouveau site : gabarit de base (en-tête de document, référencement),
//! page d'accueil, feuille de style, nom du site dans les données, dossier des médias ; modèles
//! (3.7) : vitrine (accueil, à propos, contact, menu) et blog (collection d'articles décrite
//! dans `.pages.yml`, liste des articles, gabarit d'article), faits de blocs prêts à l'emploi.
//!
//! Les conventions sont celles des paramètres de page de Revel (D-32) : `seo.title`,
//! `seo.description`, `seo.image` ; `noindex` est traité à la génération.

use revel_edit::make::Preset;
use revel_model::{Layout, Node, Page, Passthrough, Site, Stylesheet};
use revel_protocol::SiteTemplate;
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
<h1 class="titre">{{ site.title }}</h1>
<p class="intro">Première page du site.</p>
</section>
</main>"#;

/// Couleurs du site (fond sombre par défaut, ou clair), réglables dans le panneau de styles.
const DARK: &str = ":root { color-scheme: dark; --fond: #0f0f10; --texte: #f2f2f2; --texte-doux: #a3a3a3; --bloc-accent: #2f6fed; }\n";
const LIGHT: &str = ":root { color-scheme: light; --fond: #ffffff; --texte: #1b1b1b; --texte-doux: #555555; --bloc-accent: #2f6fed; }\n";

const CSS: &str = r#"*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; font-family: system-ui, sans-serif; line-height: 1.5; color: var(--texte); background: var(--fond); }
img { max-width: 100%; height: auto; }
.page { min-height: 100vh; }
.section { max-width: 1120px; margin: 0 auto; padding: 96px 24px; }
.titre { margin: 0 0 16px; font-size: 48px; line-height: 1.1; }
.intro { margin: 0; font-size: 20px; color: var(--texte-doux); }
@media (max-width: 767px) {
  .section { padding: 64px 20px; }
  .titre { font-size: 36px; }
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

/// Gabarit de base avec en-tête (nom du site, menu `site.menu`) et pied de page.
const BASE_MENU: &str = r#"<!doctype html>
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
<header class="entete"><div class="entete__inner"><a class="entete__marque" href="/">{{ site.title }}</a><nav class="entete__nav">{% for lien in site.menu %}<a href="{{ lien.url }}"{% if page.url == lien.url %} aria-current="page"{% endif %}>{{ lien.label }}</a>{% endfor %}</nav></div></header>
{{ content | safe }}
<footer class="pied"><div class="pied__inner">© {{ site.title }}</div></footer>
</body>
</html>
"#;

const MENU_CSS: &str = r#".entete { border-bottom: 1px solid rgba(127, 127, 127, 0.2); }
.entete__inner { max-width: 1168px; margin: 0 auto; padding: 20px 24px; display: flex; align-items: center; justify-content: space-between; gap: 24px; flex-wrap: wrap; }
.entete__marque { font-weight: 700; font-size: 20px; color: inherit; text-decoration: none; }
.entete__nav { display: flex; gap: 24px; flex-wrap: wrap; }
.entete__nav a { color: inherit; text-decoration: none; opacity: 0.75; }
.entete__nav a[aria-current="page"] { opacity: 1; font-weight: 600; }
.pied { border-top: 1px solid rgba(127, 127, 127, 0.2); }
.pied__inner { max-width: 1168px; margin: 0 auto; padding: 32px 24px; font-size: 14px; opacity: 0.7; }
"#;

/// Gabarit d'un article (blog) : date, titre, résumé, texte.
const ARTICLE: &str = r#"<main class="page"><article class="article"><p class="article__date">{{ date | date("%d/%m/%Y") }}</p><h1 class="article__titre">{{ title }}</h1>{% if description %}<p class="article__resume">{{ description }}</p>{% endif %}<div class="article__corps">{{ content | safe }}</div></article></main>"#;

const ARTICLE_CSS: &str = r#".article { max-width: 720px; margin: 0 auto; padding: 96px 24px; }
.article__date { margin: 0 0 12px; font-size: 14px; opacity: 0.7; }
.article__titre { margin: 0 0 16px; font-size: clamp(32px, 5vw, 48px); line-height: 1.1; }
.article__resume { margin: 0 0 32px; font-size: 20px; opacity: 0.8; }
.article__corps { font-size: 18px; line-height: 1.7; }
.liste-collection { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 32px; margin-top: 32px; }
.liste-collection__element h3 { margin: 0 0 8px; font-size: 22px; }
.liste-collection__element p { margin: 0 0 12px; opacity: 0.8; }
.liste-collection__element a { color: var(--bloc-accent, #2f6fed); font-weight: 600; text-decoration: none; }
"#;

/// Fichiers écrits avec un modèle (chemin relatif au site, contenu) : contenus d'exemple et
/// description des collections pour Pages CMS.
pub fn template_files(template: SiteTemplate, lang: &str) -> Vec<(&'static str, String)> {
    let fr = lang.to_ascii_lowercase().starts_with("fr");
    match template {
        SiteTemplate::Blog => {
            let (l_articles, l_title, l_slug, l_date, l_desc, l_image, l_body) = if fr {
                (
                    "Articles",
                    "Titre",
                    "Adresse de la page",
                    "Date",
                    "Résumé",
                    "Image",
                    "Texte",
                )
            } else {
                (
                    "Articles",
                    "Title",
                    "Page address",
                    "Date",
                    "Summary",
                    "Image",
                    "Text",
                )
            };
            let pages_yml = format!(
                "# Configuration de Pages CMS (https://pagescms.org), lue aussi par Revel.\n\
media:\n  input: media\n  output: /media\n\
content:\n  - name: articles\n    label: {l_articles}\n    type: collection\n    path: content/articles\n    filename: \"{{fields.slug}}.md\"\n    view:\n      fields: [title, date]\n      primary: title\n      default:\n        sort: date\n        order: desc\n    fields:\n      - name: title\n        label: {l_title}\n        type: string\n        required: true\n      - name: slug\n        label: {l_slug}\n        type: string\n        required: true\n      - name: date\n        label: {l_date}\n        type: date\n      - name: description\n        label: {l_desc}\n        type: text\n      - name: image\n        label: {l_image}\n        type: image\n      - name: body\n        label: {l_body}\n        type: rich-text\n"
            );
            let (a1, a2) = if fr {
                (
                    "---\ntitle: Bienvenue sur le blog\nslug: bienvenue\ndate: 2026-01-05\ndescription: Le premier article, à modifier dans la vue CMS.\n---\n<p>Écrivez ici votre premier article. Le texte se modifie dans la vue CMS de Revel, ou dans Pages CMS.</p>\n".to_string(),
                    "---\ntitle: Un deuxième article\nslug: deuxieme-article\ndate: 2026-02-10\ndescription: Les articles les plus récents s'affichent en premier.\n---\n<p>Ajoutez autant d'articles que vous le souhaitez : la liste de l'accueil montre les trois plus récents.</p>\n".to_string(),
                )
            } else {
                (
                    "---\ntitle: Welcome to the blog\nslug: welcome\ndate: 2026-01-05\ndescription: The first post, to edit in the CMS view.\n---\n<p>Write your first post here. The text is edited in Revel's CMS view, or in Pages CMS.</p>\n".to_string(),
                    "---\ntitle: A second post\nslug: second-post\ndate: 2026-02-10\ndescription: The latest posts come first.\n---\n<p>Add as many posts as you like: the home page lists the three latest.</p>\n".to_string(),
                )
            };
            let (f1, f2) = if fr {
                (
                    "content/articles/bienvenue.md",
                    "content/articles/deuxieme-article.md",
                )
            } else {
                (
                    "content/articles/welcome.md",
                    "content/articles/second-post.md",
                )
            };
            vec![(".pages.yml", pages_yml), (f1, a1), (f2, a2)]
        }
        _ => Vec::new(),
    }
}

/// Page `id` à l'adresse `path`, faite de blocs (et d'éléments) placés dans `main.page`.
fn page_of(
    s: &mut Site,
    id: &str,
    path: &str,
    name: &str,
    title: Option<&str>,
    presets: &[(Preset, &str)],
) {
    let main = Node::Element {
        id: None,
        tag: "main".into(),
        attrs: vec![revel_model::Attr {
            name: "class".into(),
            value: Some("page".into()),
            when: None,
        }],
        children: Vec::new(),
    };
    let mut props = std::collections::BTreeMap::new();
    if let Some(t) = title {
        props.insert("seo".to_string(), json!({ "title": t }));
    }
    s.pages.push(Page {
        id: id.into(),
        path: path.into(),
        layout: Some("base".into()),
        props,
        computed: Default::default(),
        body: vec![main],
        draft: false,
        code: Default::default(),
        name: Some(name.into()),
    });
    s.ensure_ids();
    let main_id = s
        .pages
        .iter()
        .find(|p| p.id == id)
        .and_then(|p| revel_model::ids::node_id(&p.body[0]).map(str::to_string))
        .unwrap_or_default();
    let parent = revel_edit::Parent::of(
        &revel_edit::Target {
            owner: revel_edit::Owner::Page(id.into()),
            node: main_id,
        },
        revel_edit::Branch::Children,
    );
    for (i, (p, text)) in presets.iter().enumerate() {
        let _ = revel_edit::apply(
            s,
            &revel_edit::Command::InsertPreset {
                parent: parent.clone(),
                index: i,
                preset: *p,
                text: text.to_string(),
            },
        );
    }
}

/// Nouveau site prêt à modifier.
pub fn starter_site(name: &str, lang: &str) -> Site {
    starter_from(name, lang, SiteTemplate::Blank, false)
}

/// Site vide aux couleurs claires, sombres, ou claires et sombres selon l'appareil du visiteur.
pub fn blank(name: &str, lang: &str, scheme: crate::templates::Scheme) -> Site {
    use crate::templates::Scheme;
    let mut s = starter_from(name, lang, SiteTemplate::Blank, scheme == Scheme::Light);
    if scheme == Scheme::Auto {
        let dark = DARK
            .replace(":root", "@media (prefers-color-scheme: dark) { :root")
            .replace("}\n", "} }\n");
        s.stylesheet = Some(Stylesheet {
            url: "/assets/site.css".into(),
            source: None,
            items: revel_style::parse(&format!("{LIGHT}{dark}{CSS}")).unwrap_or_default(),
        });
    }
    s
}

/// Nouveau site d'après un ancien modèle (vide, vitrine, blog) : forme reprise par l'import
/// WordPress ; les nouveaux sites viennent des modèles de [`crate::templates`].
pub fn starter_from(name: &str, lang: &str, template: SiteTemplate, light: bool) -> Site {
    let colors = if light { LIGHT } else { DARK };
    if template != SiteTemplate::Blank {
        return template_site(name, lang, template, colors);
    }
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
        fields: Vec::new(),
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
        code: Default::default(),
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
        items: revel_style::parse(&format!("{colors}{CSS}")).unwrap_or_default(),
    });
    s.ensure_ids();
    s
}

/// Site vitrine ou blog.
fn template_site(name: &str, lang: &str, template: SiteTemplate, colors: &str) -> Site {
    let fr = lang.to_ascii_lowercase().starts_with("fr");
    let t = |a: &'static str, b: &'static str| if fr { a } else { b };
    let mut s = Site::new(name, lang);
    let menu = match template {
        SiteTemplate::Blog => json!([
            { "label": t("Accueil", "Home"), "url": "/" },
            { "label": "Articles", "url": "/articles/" },
        ]),
        _ => json!([
            { "label": t("Accueil", "Home"), "url": "/" },
            { "label": t("À propos", "About"), "url": t("/a-propos/", "/about/") },
            { "label": "Contact", "url": "/contact/" },
        ]),
    };
    s.data.insert(
        "site".into(),
        json!({ "title": name, "lang": lang, "description": "", "menu": menu }),
    );
    s.passthrough = vec![Passthrough {
        from: "media".into(),
        to: "media".into(),
    }];
    s.layouts.push(Layout {
        fields: Vec::new(),
        id: "base".into(),
        parent: None,
        props: Default::default(),
        body: parse(BASE_MENU),
    });
    let mut css = format!("{colors}{CSS}");
    css.push_str(MENU_CSS);
    if template == SiteTemplate::Blog {
        css.push_str(ARTICLE_CSS);
    }
    s.stylesheet = Some(Stylesheet {
        url: "/assets/site.css".into(),
        source: None,
        items: revel_style::parse(&css).unwrap_or_default(),
    });
    s.ensure_ids();
    let home = t("Accueil", "Home");
    match template {
        SiteTemplate::Blog => {
            s.layouts.push(Layout {
                fields: Vec::new(),
                id: "article".into(),
                parent: Some("base".into()),
                props: Default::default(),
                body: parse(ARTICLE),
            });
            if let Ok(c) = serde_json::from_value::<revel_model::Collection>(json!({
                "id": "articles",
                "folder": "content/articles",
                "path": "/articles/{{ slug }}/",
                "layout": "article",
                "sort": { "field": "date", "descending": true, "kind": "date" },
            })) {
                s.collections.push(c);
            }
            s.ensure_ids();
            page_of(&mut s, "index", "/", home, None, &[(Preset::BlockHero, "")]);
            page_of(
                &mut s,
                "articles",
                "/articles/",
                "Articles",
                Some("Articles"),
                &[],
            );
            // Accueil : les trois articles les plus récents ; liste : tous, du plus récent.
            for (page, limit, title, tag) in [
                (
                    "index",
                    Some(3),
                    t("Derniers articles", "Latest posts"),
                    "h2",
                ),
                ("articles", None, "Articles", "h1"),
            ] {
                let q = revel_edit::collection::Query {
                    collection: "articles".into(),
                    sort: Some(("date".into(), true)),
                    limit,
                    ..Default::default()
                };
                let class = if tag == "h1" {
                    "bloc-titre"
                } else {
                    "bloc-sous-titre"
                };
                let markup = format!(
                    r#"<section class="bloc"><div class="bloc__inner"><{tag} class="{class}">{title}</{tag}><div class="liste-collection">{{% for item in {e} %}}<article class="liste-collection__element"><h3>{{{{ item.data.title }}}}</h3><p>{{{{ item.data.description }}}}</p><a href="{{{{ item.url }}}}">{read}</a></article>{{% endfor %}}</div></div></section>"#,
                    e = q.to_expr(),
                    read = t("Lire", "Read"),
                );
                let nodes = parse(&markup);
                if let Some(p) = s.pages.iter_mut().find(|p| p.id == page)
                    && let Some(Node::Element { children, .. }) = p.body.first_mut()
                {
                    children.extend(nodes);
                }
                s.ensure_ids();
            }
        }
        _ => {
            page_of(
                &mut s,
                "index",
                "/",
                home,
                None,
                &[
                    (Preset::BlockHero, ""),
                    (Preset::BlockFeatures, ""),
                    (Preset::BlockTextImage, ""),
                    (Preset::BlockCta, ""),
                ],
            );
            page_of(
                &mut s,
                "a-propos",
                t("/a-propos/", "/about/"),
                t("À propos", "About"),
                Some(t("À propos", "About")),
                &[(Preset::BlockTextImage, ""), (Preset::BlockTestimonial, "")],
            );
            page_of(
                &mut s,
                "contact",
                "/contact/",
                "Contact",
                Some("Contact"),
                &[(Preset::BlockContact, ""), (Preset::BlockFaq, "")],
            );
        }
    }
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
