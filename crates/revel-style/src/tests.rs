use super::*;
use revel_model::{Decl, StyleItem};

const CSS: &str = r#"/* --- Base --- */
:root { --nuit: #16171b; --aube: linear-gradient(90deg, #f4684d 0%, #541cbf 100%); }
@font-face { font-family: "Montserrat"; src: url("/f.woff2") format("woff2"); font-display: swap; }
html,body{margin:0;padding:0}
a { color: inherit; text-decoration: underline; }
.button{display:inline-flex;gap:.5rem;background:rgba(22, 23, 27, 0.92)!important}
.button:hover { color: var(--corail, #f4684d) }
[data-theme="light"] .button:hover { border-color: #c93f24; }
.hero::before { content: "{ ; }"; background: url(data:image/svg+xml;base64,AAA=); }
@keyframes rise { from { opacity: 0; } to { opacity: 1; } }
@media (max-width: 860px) {
  /* Mobile */
  .button { width: 100%; }
  .hero { min-height: 60vh }
}
@media (max-width: 600px) { .button { padding: 0 } }
@media (prefers-reduced-motion: reduce) { *, *::before { animation: none !important; } }
@media (min-width: 861px) { .menu { display: none } }
@supports (display: grid) { .grid { display: grid } }
.vide {}
"#;

#[test]
fn lecture_fidele() {
    let items = parse(CSS).unwrap();
    let rules: Vec<&revel_model::StyleRule> = items
        .iter()
        .filter_map(|i| match i {
            StyleItem::Rule(r) => Some(r),
            _ => None,
        })
        .collect();
    let button = rules
        .iter()
        .find(|r| r.selector == ".button" && r.media.is_none())
        .unwrap();
    assert_eq!(
        button.decls[2],
        Decl {
            prop: "background".into(),
            value: "rgba(22, 23, 27, 0.92)".into(),
            important: true
        },
        "valeur conservée telle qu'écrite"
    );
    assert_eq!(rules[0].selector, ":root");
    assert_eq!(
        rules[0].decls[1].value,
        "linear-gradient(90deg, #f4684d 0%, #541cbf 100%)"
    );
    assert_eq!(rules[1].selector, "html,body");
    let hero = rules
        .iter()
        .find(|r| r.selector == ".hero::before")
        .unwrap();
    assert_eq!(
        hero.decls[0].value, "\"{ ; }\"",
        "accolades et points-virgules dans une chaîne"
    );
    assert_eq!(hero.decls[1].value, "url(data:image/svg+xml;base64,AAA=)");
    let mobile: Vec<_> = rules
        .iter()
        .filter(|r| r.media.as_deref() == Some("(max-width: 860px)"))
        .collect();
    assert_eq!(mobile.len(), 2);
    assert!(
        items
            .iter()
            .any(|i| matches!(i, StyleItem::Comment { text, media: Some(_) } if text == "Mobile"))
    );
    let reduced = rules
        .iter()
        .find(|r| r.media.as_deref() == Some("(prefers-reduced-motion: reduce)"))
        .unwrap();
    assert!(reduced.decls[0].important);
    let raws: Vec<&str> = items
        .iter()
        .filter_map(|i| match i {
            StyleItem::Raw { css } => Some(css.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(raws.len(), 3, "{raws:?}"); // @font-face, @keyframes, @supports
    assert!(raws[1].starts_with("@keyframes rise"));
    assert!(
        rules
            .iter()
            .any(|r| r.selector == ".vide" && r.decls.is_empty())
    );
}

#[test]
fn generation_equivalente_et_stable() {
    let items = parse(CSS).unwrap();
    let out = generate(&items);
    // Relire le CSS généré redonne le même modèle (aller-retour stable).
    assert_eq!(parse(&out).unwrap(), items);
    // Même forme canonique que l'original.
    assert_eq!(canonical(&out).unwrap(), canonical(CSS).unwrap());
    // Les règles consécutives d'une même requête restent dans un seul bloc.
    assert_eq!(out.matches("@media (max-width: 860px)").count(), 1);
    // Deux blocs identiques consécutifs : même forme canonique qu'un bloc unique.
    let a = "@media (max-width: 1px) { .a { x: 1 } } @media (max-width: 1px) { .b { y: 2 } }";
    let b = "@media (max-width: 1px) { .a { x: 1 } .b { y: 2 } }";
    assert_eq!(canonical(a).unwrap(), canonical(b).unwrap());
    // Une valeur différente change la forme canonique.
    assert_ne!(canonical(".a{x:1}").unwrap(), canonical(".a{x:2}").unwrap());
}

#[test]
fn erreurs() {
    assert!(matches!(
        parse(".a { color: red"),
        Err(ParseError::Unclosed(1))
    ));
    assert!(matches!(parse(".a { } }"), Err(ParseError::Unexpected(1))));
    assert!(matches!(parse("/* ouvert"), Err(ParseError::Comment(_))));
    // CSS imbriqué (nesting) : conservé tel quel.
    let items = parse(".a { color: red; &:hover { color: blue } }").unwrap();
    assert!(matches!(&items[0], StyleItem::Raw { .. }));
}

#[test]
fn points_de_rupture() {
    let bps = breakpoints_from(&parse(CSS).unwrap());
    let ids: Vec<&str> = bps.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(
        ids,
        ["desktop", "max-860", "max-600"],
        "861 = au-dessus de 860, pas un point à part"
    );
    assert_eq!(bps[1].media().as_deref(), Some("(max-width: 860px)"));
    assert_eq!(parse_width("( max-width:600px )"), Some(Width::Max(600)));
    assert_eq!(parse_width("(min-width: 1280px)"), Some(Width::Min(1280)));
    assert_eq!(parse_width("(prefers-reduced-motion: reduce)"), None);
    let with_large =
        breakpoints_from(&parse("@media (min-width: 1440px) { .a { x: 1 } }").unwrap());
    assert_eq!(with_large[1].min_width, Some(1440));
}

#[test]
fn place_dans_la_cascade() {
    let items = parse(
        ".a { x: 1 }
         .b { x: 1 }
         @media (max-width: 860px) { .a { x: 2 } }
         .c { x: 1 }
         @media (max-width: 600px) { /* Petit */ .a { x: 3 } }
         .d { x: 1 }
         @media (max-width: 860px) { .d { x: 2 } }",
    )
    .unwrap();
    // Indices : 0 .a, 1 .b, 2 .a≤860, 3 .c, 4 commentaire, 5 .a≤600, 6 .d, 7 .d≤860
    assert_eq!(
        insertion_index(&items, ".a", Some("(max-width: 700px)")),
        4,
        "après .a≤860, avant le commentaire qui précède .a≤600"
    );
    assert_eq!(
        insertion_index(&items, ".a", Some("(max-width: 900px)")),
        2,
        "après la base, avant 860"
    );
    assert_eq!(
        insertion_index(&items, ".a", Some("(max-width: 400px)")),
        8,
        "après 600 : fin"
    );
    assert_eq!(
        insertion_index(&items, ".b", Some("(max-width: 860px)")),
        8,
        "à la fin du dernier groupe 860"
    );
    assert_eq!(
        insertion_index(&items, ".z", Some("(max-width: 400px)")),
        8,
        "nouveau sélecteur : à la fin"
    );
    assert_eq!(
        insertion_index(&items, ".z", None),
        7,
        "nouvelle règle de base : après la dernière règle de base"
    );
    assert_eq!(
        insertion_index(&items, ".a", Some("(min-width: 1280px)")),
        2
    );
}
