use super::*;
use revel_model::Node;

fn p(src: &str) -> Vec<Node> {
    parse(
        src,
        &Options {
            dialect: Dialect::Nunjucks,
            ..Default::default()
        },
    )
    .unwrap()
    .nodes
}

fn tag(n: &Node) -> &str {
    match n {
        Node::Element { tag, .. } => tag,
        _ => panic!("pas un élément : {n:?}"),
    }
}

#[test]
fn elements_attributs_texte() {
    let n = p(r#"<a class="x" href="{{ item.url }}" data-on>Voir &amp; lire</a>"#);
    assert_eq!(n.len(), 1);
    let Node::Element {
        attrs, children, ..
    } = &n[0]
    else {
        panic!()
    };
    assert_eq!(attrs[1].value.as_deref(), Some("{{ item.url }}"));
    assert_eq!(attrs[2].value, None);
    assert_eq!(
        children,
        &vec![Node::Text {
            text: "Voir & lire".into()
        }]
    );
}

#[test]
fn conditions_boucles_espaces() {
    let n = p(
        "<ul>\n{%- for it in items %}<li>{{ it.title }}</li>{% else %}<li>Vide</li>{% endfor -%}\n</ul>",
    );
    let Node::Element { children, .. } = &n[0] else {
        panic!()
    };
    assert_eq!(children.len(), 1);
    let Node::Each {
        item,
        children: body,
        empty,
        ..
    } = &children[0]
    else {
        panic!("{children:?}")
    };
    assert_eq!(item, "it");
    assert_eq!(tag(&body[0]), "li");
    assert_eq!(empty.len(), 1);

    let n = p("{% if a %}A{% elif b.length %}B{% else %}C{% endif %}");
    let Node::If {
        cond, otherwise, ..
    } = &n[0]
    else {
        panic!()
    };
    assert_eq!(cond, "a");
    let Node::If {
        cond,
        otherwise: o2,
        ..
    } = &otherwise[0]
    else {
        panic!()
    };
    assert_eq!(cond, "b|length");
    assert_eq!(o2, &vec![Node::Text { text: "C".into() }]);
}

#[test]
fn attributs_conditionnels() {
    let n = p(
        r#"<video src="{{ v }}" controls hidden{% if cover %} poster="{{ cover }}"{% endif %}></video>"#,
    );
    let Node::Element { attrs, .. } = &n[0] else {
        panic!()
    };
    assert_eq!(attrs.len(), 4);
    assert_eq!(attrs[3].name, "poster");
    assert_eq!(attrs[3].when.as_deref(), Some("cover"));
    let n = p(
        r#"<li class="m{% if loop.first %} is-active{% endif %}"{% if not loop.first %} hidden{% endif %}>x</li>"#,
    );
    let Node::Element { attrs, .. } = &n[0] else {
        panic!()
    };
    assert_eq!(
        attrs[0].value.as_deref(),
        Some("m{% if loop.first %} is-active{% endif %}")
    );
    assert_eq!(attrs[1].name, "hidden");
    assert_eq!(attrs[1].when.as_deref(), Some("not loop.first"));
}

#[test]
fn set_include_image() {
    let n = p(
        r#"{% set x = (a.push(b), a) %}{% include "partials/header.njk" %}{% image b.image, b.alt or "", "100vw", "c" %}"#,
    );
    assert_eq!(
        n[0],
        Node::Set {
            name: "x".into(),
            expr: "(a + [b])".into()
        }
    );
    assert_eq!(
        n[1],
        Node::Use {
            id: None,
            component: "partials/header".into(),
            args: Default::default(),
            variant: None,
            attrs: Vec::new(),
        }
    );
    let Node::Image(img) = &n[2] else { panic!() };
    assert_eq!(img.src, "b.image");
    assert_eq!(img.class.as_deref(), Some(r#""c""#));
}

#[test]
fn script_et_vides() {
    let n = p(
        r#"<head><meta charset="utf-8"><link rel="x" href="/a.css"><script type="application/ld+json">{"a": {{ site | json }}}</script></head>"#,
    );
    let Node::Element { children, .. } = &n[0] else {
        panic!()
    };
    assert_eq!(children.len(), 3);
    let Node::Element { children: s, .. } = &children[2] else {
        panic!()
    };
    assert!(matches!(&s[0], Node::Html { html } if html == r#"{"a": "#));
    assert!(matches!(&s[1], Node::Out { .. }));
}

#[test]
fn svg_autofermant_et_commentaires() {
    let n =
        p(r#"<!doctype html>{# note #}<svg viewBox="0 0 2 2"><path d="M0 0"/></svg><!-- c -->"#);
    assert!(matches!(&n[0], Node::Html { .. }));
    assert_eq!(tag(&n[1]), "svg");
    assert!(matches!(&n[2], Node::Html { html } if html == "<!-- c -->"));
}

#[test]
fn structure_croisee_refusee() {
    let r = parse("<div>{% if a %}</div>{% endif %}", &Options::default());
    assert!(r.is_err() || !r.unwrap().warnings.is_empty());
}

#[test]
fn modes_xml_et_texte() {
    let o = Options {
        dialect: Dialect::Nunjucks,
        mode: Mode::Xml,
    };
    let n = parse(
        "<rss><channel><link>{{ u }}</link><item>x</item></channel></rss>",
        &o,
    )
    .unwrap();
    assert!(n.warnings.is_empty(), "{:?}", n.warnings);
    let o = Options {
        dialect: Dialect::Nunjucks,
        mode: Mode::Text,
    };
    let n = parse(
        "RewriteRule ^a$ /b [R=301,L]\n{% if x %}<Files x>{% endif %}",
        &o,
    )
    .unwrap()
    .nodes;
    assert!(matches!(&n[0], Node::Html { .. }));
    assert!(matches!(&n[1], Node::If { .. }));
    // Texte qui commence par une lettre accentuée (après une balise) : coupé entre deux caractères.
    let n = parse("{{ a }}Été {{ b }}É", &o).unwrap().nodes;
    assert_eq!(
        n[1],
        Node::Html {
            html: "Été ".into()
        }
    );
    assert_eq!(n[3], Node::Html { html: "É".into() });
}

#[test]
fn commentaire_avec_apostrophe() {
    let n = p("{#- L'équipe -#}\n{%- set a = 1 -%}<p>x</p>");
    assert_eq!(
        n[0],
        Node::Set {
            name: "a".into(),
            expr: "1".into()
        }
    );
}

#[test]
fn dictionnaire_imbrique() {
    let n = p(r#"<script>{{ {"a": {"b": 1}} | json | safe }}</script>"#);
    let Node::Element { children, .. } = &n[0] else {
        panic!()
    };
    assert_eq!(
        children,
        &vec![Node::Out {
            expr: r#"{"a": {"b": 1}} | json | safe"#.into()
        }]
    );
}
