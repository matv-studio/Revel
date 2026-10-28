use super::*;
use std::sync::Arc;

fn nodes(src: &str) -> Vec<Node> {
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

fn site(html: &str, css: &str) -> Site {
    let mut s = Site::new("Essai", "fr");
    let mut page: revel_model::Page =
        serde_json::from_value(serde_json::json!({"id": "index", "path": "/"})).unwrap();
    page.body = nodes(html);
    s.pages.push(page);
    s.stylesheet = Some(revel_model::Stylesheet {
        url: "/site.css".into(),
        source: None,
        items: revel_style::parse(css).unwrap(),
    });
    s.ensure_ids();
    s
}

fn render(s: &Site) -> Vec<String> {
    let input = revel_render::BuildInput {
        site: s,
        entries: Vec::new(),
        locales: Vec::new(),
        environment: "production".into(),
        version: "v1".into(),
        hooks: Arc::new(revel_render::PlainAssets),
        now: chrono::Utc::now(),
    };
    vec![
        revel_render::render_editor(&input, "/", &revel_render::EditorOptions { scripts: false })
            .unwrap()
            .unwrap()
            .html,
    ]
}

fn run(html: &str, css: &str) -> (Site, Report, String) {
    let mut s = site(html, css);
    let (cmds, report) = flatten(&s, &render(&s), &[]);
    revel_edit::apply(&mut s, &Command::Batch { commands: cmds }).unwrap();
    let out = revel_style::generate(&s.stylesheet.as_ref().unwrap().items);
    (s, report, out)
}

fn page_html(s: &Site) -> String {
    let h = render(s).remove(0);
    revel_render::strip_editor_marks(&h)
}

#[test]
fn descendant_avec_classe_combinee() {
    let (s, r, css) = run(
        r#"<section class="hero"><p class="lead">A</p></section><p class="lead">B</p>"#,
        ".lead { color: black; }\n.hero .lead { color: red; }\n@media (max-width: 767px) { .hero .lead { color: blue; } }\n.hero .lead:hover { color: green; }",
    );
    assert!(!css.contains(".hero .lead"), "{css}");
    assert!(css.contains(".lead.lead--hero { color: red; }"), "{css}");
    assert!(
        css.contains("@media (max-width: 767px) {\n  .lead.lead--hero { color: blue; }"),
        "{css}"
    );
    assert!(
        css.contains(".lead.lead--hero:hover { color: green; }"),
        "{css}"
    );
    let h = page_html(&s);
    assert!(
        h.contains(r#"<p class="lead lead--hero">A</p>"#) && h.contains(r#"<p class="lead">B</p>"#),
        "{h}"
    );
    assert_eq!((r.flattened, r.classes), (3, 1));
    assert!(
        revel_edit::styleaudit::audit(&s).is_empty(),
        "{:?}",
        revel_edit::styleaudit::audit(&s)
    );
}

#[test]
fn classe_gardee_quand_elle_ne_sert_qu_ici() {
    let (s, _, css) = run(
        r#"<section class="hero"><p class="lead">A</p></section>"#,
        ".hero > .lead { margin: 0; }",
    );
    assert!(css.contains(".lead { margin: 0; }"), "{css}");
    assert!(page_html(&s).contains(r#"<p class="lead">A</p>"#));
}

#[test]
fn listes_variables_raccourcis() {
    let (s, r, css) = run(
        r#"<h1>T</h1><h2>U</h2><a class="btn">B</a>"#,
        ":root { --font-body: \"Inter\", sans-serif; --couleur-1: #112233; }\nh1, h2 { margin: 0; }\n.btn { font: 600 16px/1 var(--font-body); color: var(--couleur-1); -webkit-font-smoothing: antialiased; }",
    );
    assert!(
        css.contains("h1 { margin: 0; }") && css.contains("h2 { margin: 0; }"),
        "{css}"
    );
    assert!(css.contains("font-family: \"Inter\", sans-serif"), "{css}");
    assert!(css.contains("color: var(--couleur-1)"), "{css}");
    assert!(
        !css.contains("--font-body") && css.contains("--couleur-1: #112233"),
        "{css}"
    );
    assert!(!css.contains("smoothing"));
    assert!(r.translated >= 1 && r.dropped == 1);
    assert!(
        revel_edit::styleaudit::audit(&s).is_empty(),
        "{:?}",
        revel_edit::styleaudit::audit(&s)
    );
}

#[test]
fn pseudo_element_devenu_element() {
    let (s, r, css) = run(
        r#"<details class="faq"><summary class="faq__q">Question</summary><p>R</p></details>"#,
        ".faq__q::after { content: \"+\"; margin-left: 4px; }\n.faq[open] .faq__q::after { transform: rotate(45deg); }",
    );
    let h = page_html(&s);
    assert!(
        h.contains(r#"Question<span class="faq__q__after" aria-hidden="true">+</span>"#),
        "{h}"
    );
    assert!(
        css.contains(".faq__q__after { margin-left: 4px; }"),
        "{css}"
    );
    assert!(
        css.contains("[open] .faq__q__after { transform: rotate(45deg); }"),
        "{css}"
    );
    assert_eq!(r.pseudo, 1);
    assert!(
        revel_edit::styleaudit::audit(&s).is_empty(),
        "{:?}",
        revel_edit::styleaudit::audit(&s)
    );
}

#[test]
fn regles_gardees_et_mortes() {
    let (_, r, css) = run(
        r#"<div class="card"><img class="card__image" src="/a.jpg" alt=""></div>"#,
        ".card:hover .card__image { transform: scale(1.05); }\n.absent .x { color: red; }",
    );
    assert!(css.contains(".card:hover .card__image"), "{css}");
    assert!(!css.contains(".absent"), "{css}");
    assert_eq!(r.dead, 1);
    assert_eq!(r.kept.len(), 1);
}

#[test]
fn styles_ecrits_sur_les_elements() {
    let (s, r, css) = run(
        r#"<a class="lien" href="/" style="color: red; padding: 4px 8px">A</a><a class="lien" href="/b">B</a><p style="margin:0">C</p>"#,
        ".lien { color: blue; }",
    );
    let h = page_html(&s);
    assert!(!h.contains("style="), "{h}");
    assert!(
        css.contains(".lien.lien--variante { color: red; padding: 4px 8px; }"),
        "{css}"
    );
    assert!(
        css.contains(".p--variante { margin: 0; }") || css.contains("margin: 0"),
        "{css}"
    );
    assert!(r.flattened >= 2);
    assert!(
        revel_edit::styleaudit::audit(&s).is_empty(),
        "{:?}",
        revel_edit::styleaudit::audit(&s)
    );
}

#[test]
fn unset_passe_devant_le_navigateur() {
    // Le lien prend la couleur de son parent (`unset`), pas le bleu du navigateur.
    let (_, _, css) = run(
        r#"<div class="nav"><a class="lien" href="/">A</a></div>"#,
        ".nav > a { color: unset; }",
    );
    assert!(css.contains(".lien { color: unset; }"), "{css}");
}

#[test]
fn replis_de_navigateurs_retires() {
    let (_, _, out) = run(
        "<p class=\"t\">a</p>",
        "@supports not (color: color(display-p3 1 1 1)) { .t { color: red } }\n\
         @supports not (overflow: clip) { :root { --repli: hidden } }\n\
         @supports (aspect-ratio: 1) { body { --ratio-ok: auto } }\n\
         @supports (aspect-ratio: 1) { body { --lue: 1 } }\n\
         .t { color: blue; order: var(--lue) }",
    );
    assert!(
        !out.contains("display-p3") && !out.contains("--repli") && !out.contains("--ratio-ok"),
        "{out}"
    );
    // Variable déclarée sous une condition toujours vraie : lue comme sans condition.
    assert!(out.contains("order: 1"), "{out}");
}

#[test]
fn survol_d_un_ancetre_garde() {
    let (_, _, out) = run(
        "<div class=\"person\"><span class=\"swap\"><img class=\"a\" src=\"a.jpg\" alt=\"\"><img class=\"b\" src=\"b.jpg\" alt=\"\"></span></div>",
        ".swap > :last-child { position: absolute; opacity: 0; }\n\
         .person:hover .swap > :last-child { opacity: 1; }",
    );
    assert!(out.contains("opacity: 0"), "{out}");
    assert!(out.contains(":hover"), "{out}");
}

#[test]
fn classe_d_une_image() {
    let (s, _, out) = run(
        "<details class=\"set\" open><summary>a</summary><img class=\"chev\" src=\"a.svg\" alt=\"\"></details>",
        ".chev { width: 10px; }\n.set[open] .chev { rotate: 180deg; }",
    );
    assert!(!out.contains("\\\"") && !out.contains(".\""), "{out}");
    let json = serde_json::to_string(&s.pages[0].body).unwrap();
    assert!(!json.contains("\\\"chev\\\" "), "{json}");
}

#[test]
fn utilisation_de_composant_selon_l_endroit() {
    let mut s = site(
        "<header class=\"intro\"><x-use-placeholder></x-use-placeholder></header><footer></footer>",
        ".bouton { padding: 4px; }\n.intro .bouton { margin-top: 40px; }",
    );
    let comp: revel_model::Component = serde_json::from_value(serde_json::json!({
        "id": "bouton", "name": "Bouton",
        "body": [{"node": "element", "id": "racine1", "tag": "a", "attrs": [{"name": "class", "value": "bouton"}], "children": [{"node": "text", "text": "Go"}]}]
    }))
    .unwrap();
    s.components.push(comp);
    let use_ = |id: &str| revel_model::Node::Use {
        id: Some(id.into()),
        component: "bouton".into(),
        args: Default::default(),
        variant: None,
        attrs: vec![],
        children: vec![],
    };
    // Une utilisation dans l'en-tête « intro », une dans le pied de page.
    if let revel_model::Node::Element { children, .. } = &mut s.pages[0].body[0] {
        *children = vec![use_("u1")];
    }
    if let revel_model::Node::Element { children, .. } = &mut s.pages[0].body[1] {
        *children = vec![use_("u2")];
    }
    let (cmds, _) = flatten(&s, &render(&s), &[]);
    revel_edit::apply(&mut s, &Command::Batch { commands: cmds }).unwrap();
    let out = revel_style::generate(&s.stylesheet.as_ref().unwrap().items);
    assert!(
        out.contains(".bouton.bouton--intro { margin-top: 40px; }"),
        "{out}"
    );
    let html = page_html(&s);
    assert_eq!(html.matches("bouton bouton--intro").count(), 1, "{html}");
}
