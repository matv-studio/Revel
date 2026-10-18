//! Import d'un site HTML statique (7.1) : pages, gabarits, styles, fichiers copiés.

use revel_import::html::{self, Options};
use revel_import::{SiteKind, detect};
use revel_model::{Node, StyleItem};
use std::path::Path;

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

const HEAD: &str = r#"<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="stylesheet" href="https://fonts.example.com/css?family=Inter">
<link rel="stylesheet" href="{ROOT}css/base.css">
<link rel="stylesheet" href="{ROOT}css/style.css?v=3">
<link rel="icon" href="{ROOT}favicon.ico">"#;

const NAV: &str = r#"<header class="top"><nav><a href="{ROOT}index.html">Accueil</a> <a href="{ROOT}about.html">À propos</a> <a href="{ROOT}blog/">Blog</a></nav></header>"#;
const FOOT: &str = r#"<footer class="foot"><p>© Atelier</p></footer>"#;

fn page(
    root_prefix: &str,
    title: &str,
    extra_head: &str,
    body_attr: &str,
    main: &str,
    tail: &str,
) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"fr\">\n<head>\n{}\n<title>{title}</title>\n<meta name=\"description\" content=\"Page {title}\">\n{extra_head}\n</head>\n<body{body_attr}>\n<div class=\"wrap\">\n{}\n<main>{main}</main>\n{}\n</div>\n<script src=\"{root_prefix}js/site.js\"></script>\n{tail}\n</body>\n</html>\n",
        HEAD.replace("{ROOT}", root_prefix),
        NAV.replace("{ROOT}", root_prefix),
        FOOT,
    )
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(
        r,
        "index.html",
        &page(
            "",
            "Accueil — Atelier",
            "<meta property=\"og:site_name\" content=\"Atelier\">\n<link rel=\"canonical\" href=\"https://atelier.example/\">",
            " class=\"home\"",
            "<h1>Bienvenue</h1><p>Texte {{ pas une expression }} et <img src=\"img/a.png\" srcset=\"img/a.png 1x, img/a@2x.png 2x\" alt=\"\" data-x=\"{{y}}\"><ul><li>un<li>deux</ul>",
            "",
        ),
    );
    write(
        r,
        "about.html",
        &page(
            "",
            "À propos",
            "<meta property=\"og:site_name\" content=\"Atelier\">\n<link rel=\"stylesheet\" href=\"css/about.css\">\n<script src=\"js/about-head.js\"></script>",
            "",
            "<h1>À propos</h1>",
            "<script>console.log('about')</script>",
        ),
    );
    write(
        r,
        "blog/index.html",
        &page(
            "../",
            "Blog",
            "<meta property=\"og:site_name\" content=\"Atelier\">",
            "",
            "<h1>Blog</h1><a href=\"post.html\">Article</a>",
            "",
        ),
    );
    write(
        r,
        "blog/post.html",
        &page(
            "../",
            "Article",
            "<meta property=\"og:site_name\" content=\"Atelier\">",
            "",
            "<article style=\"background:url(../img/b.png)\">Article</article>",
            "",
        ),
    );
    // Page sans navigation : gabarit de base seulement.
    write(
        r,
        "404.html",
        "<!DOCTYPE html><html lang=\"fr\"><head><meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><link rel=\"stylesheet\" href=\"https://fonts.example.com/css?family=Inter\"><link rel=\"stylesheet\" href=\"/css/base.css\"><link rel=\"stylesheet\" href=\"/css/style.css?v=3\"><link rel=\"icon\" href=\"/favicon.ico\"><meta property=\"og:site_name\" content=\"Atelier\"><title>Introuvable</title></head><body><p class=\"lost\">Page introuvable</p><script src=\"/js/site.js\"></script></body></html>",
    );
    write(r, "css/base.css", "html { box-sizing: border-box; }\n");
    write(
        r,
        "css/style.css",
        "@font-face { font-family: Inter; src: url(\"../fonts/inter.woff2\"); }\n.top { background: url(../img/bg.png) no-repeat; }\n@media (max-width: 700px) { .top { padding: 0; } }\n",
    );
    write(r, "css/about.css", ".about { color: red; }\n");
    write(r, "js/site.js", "console.log('site');\n");
    write(r, "js/about-head.js", "");
    write(r, "img/a.png", "png");
    write(r, "favicon.ico", "ico");
    write(r, "robots.txt", "User-agent: *\nDisallow: /brouillons/\n");
    write(r, "README.md", "# Dépôt\n");
    write(r, ".gitignore", "node_modules\n");
    dir
}

fn find_tag<'a>(nodes: &'a [Node], name: &str) -> Option<&'a Node> {
    for n in nodes {
        if let Node::Element { tag, children, .. } = n {
            if tag == name {
                return Some(n);
            }
            if let Some(f) = find_tag(children, name) {
                return Some(f);
            }
        }
    }
    None
}

fn attr<'a>(n: &'a Node, name: &str) -> Option<&'a str> {
    match n {
        Node::Element { attrs, .. } => attrs
            .iter()
            .find(|a| a.name == name)
            .and_then(|a| a.value.as_deref()),
        _ => None,
    }
}

#[test]
fn nature_du_dossier() {
    let dir = fixture();
    assert_eq!(detect(dir.path()), SiteKind::Static { pages: 5 });
    // Site à construire : signalé, avec son dossier produit.
    let app = tempfile::tempdir().unwrap();
    write(
        app.path(),
        "package.json",
        r#"{"devDependencies":{"@sveltejs/kit":"^2.50.2","vite":"^7"}}"#,
    );
    write(app.path(), "index.html", "<div id=app></div>");
    write(app.path(), "build/index.html", "<p>ok</p>");
    assert_eq!(
        detect(app.path()),
        SiteKind::NeedsBuild {
            tool: "SvelteKit".into(),
            outputs: vec!["build".into()]
        }
    );
    // Outils seulement (pas de cadre d'application) : site statique.
    let tools = tempfile::tempdir().unwrap();
    write(
        tools.path(),
        "package.json",
        r#"{"devDependencies":{"prettier":"3"}}"#,
    );
    write(tools.path(), "index.html", "<p>ok</p>");
    assert_eq!(detect(tools.path()), SiteKind::Static { pages: 1 });
    // Pages dans docs/ seulement.
    let gh = tempfile::tempdir().unwrap();
    write(gh.path(), "docs/index.html", "<p>ok</p>");
    assert_eq!(
        detect(gh.path()),
        SiteKind::Unknown {
            outputs: vec!["docs".into()]
        }
    );
}

#[test]
fn site_statique_importe() {
    let dir = fixture();
    let (site, report) = html::import(
        dir.path(),
        &Options {
            name: "dossier".into(),
            lang: "en".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(site.name, "Atelier");
    assert_eq!(site.default_locale, "fr");
    assert_eq!(site.base_url.as_deref(), Some("https://atelier.example"));
    let paths: Vec<&str> = site.pages.iter().map(|p| p.path.as_str()).collect();
    assert_eq!(
        paths,
        ["/", "/404.html", "/about.html", "/blog/", "/blog/post.html"],
        "{paths:?}"
    );
    assert_eq!(report.pages, 5);

    // Gabarits : base (en-tête, scripts communs) et site (enveloppe, navigation, pied).
    let ids: Vec<&str> = site.layouts.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, ["base", "site"]);
    let layout_of = |p: &str| {
        site.pages
            .iter()
            .find(|x| x.path == p)
            .unwrap()
            .layout
            .clone()
            .unwrap()
    };
    assert_eq!(layout_of("/"), "site");
    assert_eq!(layout_of("/blog/post.html"), "site");
    assert_eq!(layout_of("/404.html"), "base");
    let base = &site.layouts[0].body;
    let head = find_tag(base, "head").unwrap();
    let head_json = serde_json::to_string(head).unwrap();
    assert!(head_json.contains("seo.title"), "{head_json}");
    assert!(head_json.contains("fonts.example.com"), "{head_json}");
    assert!(head_json.contains("/favicon.ico"), "{head_json}");
    // Feuilles locales fusionnées : un seul lien, vers la feuille du site.
    assert!(head_json.contains("\"/css/base.css\""), "{head_json}");
    assert!(!head_json.contains("style.css"), "{head_json}");
    let body = find_tag(base, "body").unwrap();
    // Classe du corps propre à chaque page : réglage de la page (Apparence), rien dans le gabarit.
    assert_eq!(attr(body, "class"), None);
    let body_json = serde_json::to_string(body).unwrap();
    assert!(body_json.contains("/js/site.js"), "{body_json}");
    let shell = serde_json::to_string(&site.layouts[1].body).unwrap();
    assert!(shell.contains("\"wrap\"") && shell.contains("\"top\"") && shell.contains("\"foot\""));
    assert!(
        shell.contains("\"/about.html\""),
        "liens réécrits depuis la racine : {shell}"
    );
    assert!(shell.contains("\"slot\""), "{shell}");

    // Feuille du site : règles et adresses réécrites.
    let sheet = site.stylesheet.as_ref().unwrap();
    assert_eq!(sheet.url, "/css/base.css");
    assert_eq!(sheet.source.as_deref(), Some("css/base.css"));
    let css = revel_style::render(sheet);
    assert!(css.contains("url(/img/bg.png)"), "{css}");
    assert!(css.contains("url(\"/fonts/inter.woff2\")"), "{css}");
    assert!(css.contains("box-sizing"), "{css}");
    assert!(
        sheet
            .items
            .iter()
            .any(|i| matches!(i, StyleItem::Rule(r) if r.media.is_some()))
    );
    assert!(site.breakpoints.iter().any(|b| b.max_width == Some(700)));

    // Pages : référencement, contenu, attributs propres, code ajouté.
    let home = &site.pages[0];
    assert_eq!(home.props["seo"]["title"], "Accueil — Atelier");
    assert_eq!(home.props["seo"]["description"], "Page Accueil — Atelier");
    assert_eq!(home.props["bodyClass"], "home");
    let home_json = serde_json::to_string(&home.body).unwrap();
    assert!(
        home_json.contains("{{ pas une expression }}"),
        "texte gardé : {home_json}"
    );
    assert!(
        home_json.contains("{{ '{{' }}y}}"),
        "attribut gardé en texte : {home_json}"
    );
    assert!(home_json.contains("/img/a@2x.png 2x"), "{home_json}");
    let ul = find_tag(&home.body, "ul").unwrap();
    let Node::Element { children, .. } = ul else {
        panic!()
    };
    assert_eq!(
        children
            .iter()
            .filter(|c| matches!(c, Node::Element { tag, .. } if tag == "li"))
            .count(),
        2,
        "<li> fermés comme dans un navigateur"
    );
    assert!(
        !home_json.contains("\"top\""),
        "navigation dans le gabarit : {home_json}"
    );
    let about = site.pages.iter().find(|p| p.path == "/about.html").unwrap();
    let about_json = serde_json::to_string(&about.body).unwrap();
    assert!(
        about_json.contains("/css/about.css"),
        "feuille propre à la page : {about_json}"
    );
    assert!(
        about.code.head.contains("/js/about-head.js"),
        "{:?}",
        about.code
    );
    assert!(
        about.code.body.contains("console.log('about')"),
        "{:?}",
        about.code
    );
    let post = site
        .pages
        .iter()
        .find(|p| p.path == "/blog/post.html")
        .unwrap();
    let post_json = serde_json::to_string(&post.body).unwrap();
    assert!(post_json.contains("url(/img/b.png)"), "{post_json}");
    let lost = site.pages.iter().find(|p| p.path == "/404.html").unwrap();
    assert!(serde_json::to_string(&lost.body).unwrap().contains("lost"));

    // Fichiers copiés tels quels, références du site.
    let pass: Vec<&str> = site.passthrough.iter().map(|p| p.from.as_str()).collect();
    assert!(pass.contains(&"css") && pass.contains(&"img") && pass.contains(&"favicon.ico"));
    assert!(
        !pass.iter().any(|p| p.ends_with(".html")
            || *p == "README.md"
            || p.starts_with('.')
            || *p == "robots.txt"),
        "{pass:?}"
    );
    assert_eq!(
        site.settings.seo.robots.as_deref(),
        Some("User-agent: *\nDisallow: /brouillons/\n")
    );
}

#[test]
fn une_seule_page() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "index.html",
        "<html><head><title>Seule</title></head><body><h1>Une page</h1></body></html>",
    );
    let (site, _) = html::import(dir.path(), &Options::default()).unwrap();
    assert_eq!(site.layouts.len(), 1);
    assert_eq!(site.default_locale, "en");
    let json = serde_json::to_string(&site.pages[0].body).unwrap();
    assert!(json.contains("Une page"), "{json}");
}

#[test]
fn css_urls_reecrites() {
    let css = "a{background:url('x.png')} @import \"b.css\"; b{src:url(data:image/png;base64,AA)} c{x:url(/abs.png)} d{x:url(https://cdn/x.png)}";
    let out = html::rewrite_css_urls(css, "/css/");
    assert!(out.contains("url('/css/x.png')"), "{out}");
    assert!(out.contains("@import \"/css/b.css\""), "{out}");
    assert!(out.contains("url(data:image/png;base64,AA)"), "{out}");
    assert!(
        out.contains("url(/abs.png)") && out.contains("url(https://cdn/x.png)"),
        "{out}"
    );
}

#[test]
fn comparaison_des_pages() {
    let a = "<!doctype html><html><head><link rel=stylesheet href=\"css/a.css\"><title>T</title></head><body>\n<p class=x>Un  texte</p>\n<a href=\"b.html\">b</a></body></html>";
    // Même page écrite autrement : blancs, adresses depuis la racine, paramètre de version,
    // feuille de style placée dans le corps.
    let b = "<!DOCTYPE html><html><head><title>T</title></head><body><link rel=\"stylesheet\" href=\"/css/a.css?v=2\"><p class=\"x\">Un texte</p><a href=\"/b.html\">b</a></body></html>";
    assert!(
        html::compare(a, b, "index.html", 5).is_empty(),
        "{:?}",
        html::compare(a, b, "index.html", 5)
    );
    let c = "<!DOCTYPE html><html><head><title>T</title></head><body><link rel=\"stylesheet\" href=\"/css/a.css\"><p class=\"y\">Un texte</p><a href=\"/b.html\">b</a></body></html>";
    let d = html::compare(a, c, "index.html", 5);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].contains("attributs"), "{d:?}");
    let e = "<!DOCTYPE html><html><head><title>U</title></head><body><p class=x>Un texte</p></body></html>";
    assert!(html::compare(a, e, "index.html", 5).len() >= 2);
}

#[test]
fn html_peu_soigne() {
    let dir = tempfile::tempdir().unwrap();
    // Balises non fermées, attributs sans guillemets, tableau sans <tbody>, majuscules, entités.
    write(
        dir.path(),
        "index.html",
        "<HTML><HEAD><TITLE>Vieux site</TITLE><META NAME=description CONTENT=\"Un &amp; deux\"></HEAD><BODY BGCOLOR=white><P>Un<P>Deux &eacute;t&eacute;<TABLE><TR><TD>a<TD>b</TABLE><IMG SRC=img/x.gif></BODY></HTML>",
    );
    write(dir.path(), "page2.htm", "<p>Seconde");
    let (site, report) = html::import(dir.path(), &Options::default()).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let home = &site.pages[0];
    assert_eq!(home.props["seo"]["title"], "Vieux site");
    assert_eq!(home.props["seo"]["description"], "Un & deux");
    let json = serde_json::to_string(&home.body).unwrap();
    assert!(json.contains("Deux été"), "{json}");
    assert!(json.contains("\"tbody\""), "{json}");
    assert!(json.contains("\"/img/x.gif\""), "{json}");
    assert_eq!(site.pages[1].path, "/page2.htm");
}

#[test]
fn noms_des_pages() {
    let dir = tempfile::tempdir().unwrap();
    let p = |t: &str| {
        format!("<html lang=fr><head><title>{t}</title></head><body><p>x</p></body></html>")
    };
    write(dir.path(), "index.html", &p("Atelier — Studio de design"));
    write(dir.path(), "agence.html", &p("L'agence | Atelier"));
    write(dir.path(), "contact.html", &p("Contact | Atelier"));
    write(dir.path(), "a/index.html", &p("Atelier"));
    write(dir.path(), "b/index.html", &p("Atelier"));
    let (site, _) = html::import(
        dir.path(),
        &Options {
            name: "Atelier".into(),
            lang: "fr".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let name = |path: &str| {
        let pg = site.pages.iter().find(|p| p.path == path).unwrap();
        pg.name
            .clone()
            .unwrap_or_else(|| pg.props["seo"]["title"].as_str().unwrap().to_string())
    };
    assert_eq!(name("/agence.html"), "L'agence");
    assert_eq!(name("/contact.html"), "Contact");
    assert_eq!(name("/a/"), "A");
    assert_eq!(name("/b/"), "B");
    assert_eq!(name("/"), "Atelier — Studio de design");
}

/// Navigation marquée « page en cours » (comme un export Webflow) : partagée dans le gabarit, la
/// marque écrite sous condition sur l'adresse de la page.
#[test]
fn navigation_avec_page_en_cours() {
    let dir = tempfile::tempdir().unwrap();
    let nav = |cur: &str| {
        let link = |href: &str, label: &str| {
            let on = cur == href || (href != "/" && cur.starts_with(href));
            format!(
                "<a href=\"{href}\" class=\"nav-link{}\"{}>{label}</a>",
                if on { " w--current" } else { "" },
                if cur == href {
                    " aria-current=\"page\""
                } else {
                    ""
                }
            )
        };
        format!(
            "<nav class=\"nav\">{}{}{}</nav>",
            link("/", "Accueil"),
            link("/blog/", "Blog"),
            link("/contact.html", "Contact")
        )
    };
    let page = |cur: &str, h1: &str| {
        format!(
            "<!DOCTYPE html><html><head><title>{h1}</title></head><body>{}<main><h1>{h1}</h1></main><footer>pied</footer></body></html>",
            nav(cur)
        )
    };
    write(dir.path(), "index.html", &page("/", "Accueil"));
    write(
        dir.path(),
        "contact.html",
        &page("/contact.html", "Contact"),
    );
    write(dir.path(), "blog/index.html", &page("/blog/", "Blog"));
    write(
        dir.path(),
        "blog/un.html",
        &page("/blog/un.html", "Article"),
    );
    let (site, _) = html::import(dir.path(), &Options::default()).unwrap();
    assert_eq!(site.layouts.len(), 2);
    let shell = serde_json::to_string(&site.layouts[1].body).unwrap();
    assert!(
        shell.contains("\"nav\""),
        "navigation dans le gabarit : {shell}"
    );
    assert!(
        shell.contains(r#"page.url == \"/contact.html\""#),
        "{shell}"
    );
    assert!(
        shell.contains(r#"page.url is startingwith(\"/blog/\")"#),
        "rubrique : {shell}"
    );
    for p in &site.pages {
        let body = serde_json::to_string(&p.body).unwrap();
        assert!(!body.contains("nav-link"), "{} : {body}", p.path);
    }
    // Pages produites équivalentes aux originales.
    let out = tempfile::tempdir().unwrap();
    let files = revel_render::build(&revel_render::BuildInput {
        site: &site,
        entries: Default::default(),
        locales: Default::default(),
        environment: "production".into(),
        version: String::new(),
        hooks: std::sync::Arc::new(revel_render::PlainAssets),
        now: Default::default(),
    })
    .unwrap()
    .files;
    let _ = out;
    for (rel, f) in [
        ("index.html", "index.html"),
        ("contact.html", "contact.html"),
        ("blog/index.html", "blog/index.html"),
        ("blog/un.html", "blog/un.html"),
    ] {
        let a = std::fs::read_to_string(dir.path().join(rel)).unwrap();
        let b = files
            .iter()
            .find(|x| x.path == f)
            .map(|x| String::from_utf8_lossy(&x.bytes).to_string())
            .unwrap();
        let d = html::compare(&a, &b, rel, 5);
        assert!(d.is_empty(), "{rel} : {d:?}\n{b}");
    }
}

/// Type de document gardé (sans lui, le navigateur passe en mode de compatibilité) ; son absence
/// est une différence.
#[test]
fn type_de_document() {
    let dir = fixture();
    let (site, _) = html::import(dir.path(), &Options::default()).unwrap();
    assert!(matches!(&site.layouts[0].body[0], Node::Html { html } if html == "<!DOCTYPE html>"));
    let a = "<!DOCTYPE html><html><head><title>T</title></head><body><p>x</p></body></html>";
    let b = "<html><head><title>T</title></head><body><p>x</p></body></html>";
    assert_eq!(html::compare(a, b, "index.html", 5).len(), 1);
}

/// Site publié par Framer, lu à son adresse (7.4) : styles écrits dans les pages, moteur de
/// Framer, animations d'apparition.
fn framer_page(title: &str, page_css: &str, body: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width">
<meta name="generator" content="Framer 9a2b3c4">
<title>{title}</title>
<link rel="modulepreload" href="/_externe/framerusercontent.com/sites/x/chunk-A.mjs">
<style data-framer-css-ssr-minified>@font-face{{font-family:"Inter";src:url(/_externe/fonts.gstatic.com/s/inter/v1/a.woff2)}}body{{--framer-will-change-override:none}}.framer-body{{margin:0}}{page_css}</style>
<script type="framer/appear" id="__framer__appearAnimationsContent">{{"a1":{{"default":{{"initial":{{"opacity":0.001,"y":30}},"animate":{{"opacity":1,"y":0,"transition":{{"type":"spring","delay":0.2,"duration":0.6,"bounce":0}}}}}}}}}}</script>
</head>
<body>
<div id="main" data-framer-hydrate-v2="x">{body}</div>
<div id="__framer-badge-container"></div>
<script type="module" data-framer-bundle="main" src="/_externe/framerusercontent.com/sites/x/script_main.mjs"></script>
</body>
</html>"#
    )
}

#[test]
fn site_framer_lu_en_ligne() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(
        r,
        "index.html",
        &framer_page(
            "Accueil",
            "@media (max-width:809px){.framer-p{font-size:24px}}.framer-p{font-size:36px}@media (max-width:809px){.framer-p{font-size:24px}}.framer-A1 .framer-titre{font-size:64px}",
            r#"<h1 class="framer-titre" data-framer-appear-id="a1" style="will-change:transform;opacity:0.001;transform:translateY(30px)">Studio</h1>"#,
        ),
    );
    write(
        r,
        "contact/index.html",
        &framer_page(
            "Contact",
            ".framer-B2 .framer-form{display:grid}",
            r#"<p class="framer-form">Écrire</p>"#,
        ),
    );
    let (site, report) = html::import(
        r,
        &Options {
            name: "studio".into(),
            lang: "fr".into(),
            merge_styles: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.tool.as_deref(), Some("Framer"));
    assert_eq!(report.animations, 1);
    let sheet = site.stylesheet.as_ref().expect("feuille du site");
    assert_eq!(sheet.url, "/assets/styles.css");
    let css = revel_style::generate(&sheet.items);
    // Règles communes une seule fois, règles propres à chaque page reprises.
    assert_eq!(css.matches(".framer-body").count(), 1, "{css}");
    assert!(css.contains(".framer-A1 .framer-titre"));
    // Bloc répété : gardé à sa dernière place (après la règle générale, comme dans la page).
    let base = css.find(".framer-p {").expect("règle générale");
    let media = css
        .rfind("@media (max-width:809px)")
        .expect("règle d'écran");
    assert!(media > base, "{css}");
    assert_eq!(css.matches("@media (max-width:809px)").count(), 1);
    assert!(css.contains(".framer-B2 .framer-form"));
    let base = format!("{:?}", site.layouts[0].body);
    assert!(base.contains("/assets/styles.css"));
    assert!(!base.contains("framer/appear"));
    assert!(!base.contains("modulepreload"));
    let home = format!("{:?}", site.pages[0].body);
    assert!(home.contains("data-revel-appear"), "{home}");
    assert!(home.contains("up dur=0.6 delay=0.2 dist=30"), "{home}");
    assert!(!home.contains("opacity:0.001"));
    assert!(!home.contains("__framer-badge"));
    for p in &site.pages {
        assert!(
            !format!("{:?}", p.body).contains("<style"),
            "styles retirés des pages"
        );
        assert!(!p.code.body.contains("framer"), "{}", p.code.body);
    }
}

/// Export du code de Webflow (7.3) : interactions, formulaire, contenus du CMS (CSV).
#[test]
fn export_webflow_avec_cms() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    let page = |title: &str, main: &str| {
        format!(
            r#"<!DOCTYPE html><!-- This site was created in Webflow. https://webflow.com --><html data-wf-page="p1" data-wf-site="65f0abc"><head><meta charset="utf-8"><title>{title}</title><meta content="Webflow" name="generator"><link href="css/normalize.css" rel="stylesheet" type="text/css"><link href="css/webflow.css" rel="stylesheet" type="text/css"><link href="css/atelier.webflow.css" rel="stylesheet" type="text/css"></head><body><div class="nav"><a href="index.html" aria-current="page" class="w--current">Accueil</a></div>{main}<script src="js/webflow.js" type="text/javascript"></script></body></html>"#
        )
    };
    write(
        r,
        "index.html",
        &page(
            "Atelier",
            r#"<h1 data-w-id="w1" style="opacity:0" class="hero">Bienvenue</h1><div class="w-form"><form id="email-form" name="email-form" data-name="Email Form" method="get"><input type="email" name="email"><input type="submit" value="Envoyer" class="w-button"></form><div class="w-form-done"><div>Merci !</div></div><div class="w-form-fail"><div>Erreur.</div></div></div><div class="w-dyn-list"><div class="w-dyn-empty"><div>No items found.</div></div></div>"#,
        ),
    );
    write(
        r,
        "detail_blog-posts.html",
        &page(
            "Article",
            r#"<div class="article"><h1 class="heading w-dyn-bind-empty"></h1><img src="https://d3e54v103j8qbb.cloudfront.net/plugins/Basic/assets/placeholder.60f9b1840c.svg" loading="lazy" alt="" class="w-dyn-bind-empty"><div class="w-richtext w-dyn-bind-empty"></div></div>"#,
        ),
    );
    write(r, "css/normalize.css", "html{font-family:sans-serif}");
    write(
        r,
        "css/webflow.css",
        ".w-form{margin:0 0 15px}.w-form-done{display:none}",
    );
    write(r, "css/atelier.webflow.css", ".hero{font-size:48px}");
    write(
        r,
        "js/webflow.js",
        r#"/*! webflow */!function(){}();Webflow.require('ix2').init({"events":{"e-1":{"id":"e-1","eventTypeId":"SCROLL_INTO_VIEW","action":{"actionTypeId":"FADE_EFFECT","config":{"actionListId":"fadeIn","autoStopEventId":"e-2","duration":1000,"delay":0,"easing":""}},"target":{"id":"p1|w1","appliesTo":"ELEMENT"}}},"actionLists":{},"site":{}});"#,
    );
    let (mut site, mut report) = html::import(
        r,
        &Options {
            name: "atelier".into(),
            lang: "fr".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.tool.as_deref(), Some("Webflow"));
    assert_eq!(report.animations, 1);
    assert_eq!(report.forms, 1);
    let home = site.pages.iter().find(|p| p.path == "/").unwrap();
    let h = format!("{:?}", home.body);
    assert!(h.contains("fade dur=1 delay=0"), "{h}");
    assert!(h.contains("data-revel-form"), "{h}");
    assert!(h.contains("revel-form-box"), "{h}");
    let css = revel_style::generate(&site.stylesheet.as_ref().unwrap().items);
    assert!(css.contains(".revel-form-box"), "{css}");

    // Contenus du CMS.
    let csv = "Name,Slug,Collection ID,Locale ID,Item ID,Archived,Draft,Created On,Updated On,Published On,Post Body,Main Image\n\"Bols d'hiver\",bols-d-hiver,c,l,i1,false,false,x,x,x,<p>Texte.</p>,\n";
    let cols = vec![revel_import::webflow::from_csv("Atelier - Blog Posts - 1.csv", csv).unwrap()];
    struct Off;
    impl revel_import::crawl::Fetch for Off {
        fn get(&self, _: &url::Url) -> Result<revel_import::crawl::Fetched, String> {
            Err("hors ligne".into())
        }
    }
    revel_import::webflow::apply(&mut site, r, cols, &Off, None, &mut |_| {}, &mut report).unwrap();
    assert_eq!(report.entries, 1);
    assert!(
        !site.pages.iter().any(|p| p.path.contains("detail_")),
        "page modèle retirée"
    );
    let col = site
        .collections
        .iter()
        .find(|c| c.id == "blog_posts")
        .unwrap();
    assert_eq!(col.path.as_deref(), Some("/blog-posts/{{ slug }}/"));
    assert_eq!(col.layout.as_deref(), Some("cms-blog-posts"));
    let entry = std::fs::read_to_string(r.join("content/blog-posts/bols-d-hiver.md")).unwrap();
    assert!(
        entry.contains("name: Bols d'hiver") || entry.contains("name: \"Bols d'hiver\""),
        "{entry}"
    );
    assert!(entry.ends_with("<p>Texte.</p>\n"), "{entry}");
    let yml = std::fs::read_to_string(r.join(".pages.yml")).unwrap();
    assert!(yml.contains("name: blog_posts"), "{yml}");
    assert!(yml.contains("type: rich-text"), "{yml}");
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("liste(s) du CMS"))
    );
}
