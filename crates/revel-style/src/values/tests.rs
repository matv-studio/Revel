use super::*;
use revel_model::Decl;

fn decls(list: &[(&str, &str)]) -> Vec<Decl> {
    list.iter()
        .map(|(p, v)| Decl {
            prop: (*p).into(),
            value: (*v).into(),
            important: false,
        })
        .collect()
}

#[test]
fn couleurs_lues() {
    assert_eq!(Rgba::parse("#fff").unwrap().hex6(), "#ffffff");
    assert_eq!(Rgba::parse("#336699").unwrap().hex6(), "#336699");
    assert_eq!(Rgba::parse("#33669980").unwrap().alpha(), 0.5);
    assert_eq!(Rgba::parse("rgb(10, 20, 30)").unwrap().hex6(), "#0a141e");
    assert_eq!(Rgba::parse("rgba(10,20,30,.25)").unwrap().alpha(), 0.25);
    assert_eq!(Rgba::parse("rgb(10 20 30 / 50%)").unwrap().alpha(), 0.5);
    assert_eq!(Rgba::parse("rgb(100% 0% 0%)").unwrap().hex6(), "#ff0000");
    assert_eq!(
        Rgba::parse("hsl(120, 100%, 25%)").unwrap().hex6(),
        "#008000"
    );
    assert_eq!(
        Rgba::parse("hsl(0.5turn 100% 50%)").unwrap().hex6(),
        "#00ffff"
    );
    assert_eq!(Rgba::parse("RebeccaPurple").unwrap().hex6(), "#663399");
    assert_eq!(Rgba::parse("transparent").unwrap().alpha(), 0.0);
    assert!(Rgba::parse("var(--x)").is_none());
    assert!(Rgba::parse("#12").is_none());
    assert_eq!(color::named_count(), 148);
}

#[test]
fn couleurs_ecrites_et_converties() {
    let c = Rgba::parse("#3d7eff").unwrap();
    assert_eq!(c.to_css(), "#3d7eff");
    assert_eq!(Rgba { a: 0.4, ..c }.to_css(), "rgba(61, 126, 255, 0.4)");
    // Aller-retour TSV et TSL sans perte visible.
    for hex in [
        "#000000", "#ffffff", "#808080", "#ff0000", "#3d7eff", "#1f9d6b", "#e8890c",
    ] {
        let c = Rgba::parse(hex).unwrap();
        assert_eq!(Rgba::from_hsva(c.to_hsva()).hex6(), hex);
        let (h, s, l) = c.to_hsl();
        assert_eq!(Rgba::from_hsl(h, s, l).hex6(), hex);
    }
    let hsv = Rgba::parse("#ff0000").unwrap().to_hsva();
    assert_eq!((hsv.h, hsv.s, hsv.v), (0.0, 1.0, 1.0));
}

#[test]
fn contraste_wcag() {
    let r = contrast(Rgba::BLACK, Rgba::WHITE);
    assert!((r - 21.0).abs() < 0.01, "{r}");
    let r = contrast(Rgba::parse("#777777").unwrap(), Rgba::WHITE);
    assert!((r - 4.48).abs() < 0.01, "{r}");
    // Fond transparent : posé sur du blanc.
    let r = contrast(Rgba::BLACK, Rgba::parse("transparent").unwrap());
    assert!((r - 21.0).abs() < 0.01, "{r}");
}

#[test]
fn nombres_et_calculs() {
    assert_eq!(fmt_num(1.5), "1.5");
    assert_eq!(fmt_num(2.0), "2");
    assert_eq!(fmt_num(-0.0001), "0");
    assert_eq!(fmt_num(0.125), "0.125");
    assert_eq!(length::parse_num("1.5rem").unwrap().unit, "rem");
    assert_eq!(length::parse_num(".5").unwrap().value, 0.5);
    assert!(length::parse_num("auto").is_none());
    assert!(length::parse_num("1px 2px").is_none());
    assert_eq!(eval("12", None, "px").as_deref(), Some("12px"));
    assert_eq!(eval("12+4", None, "px").as_deref(), Some("16px"));
    assert_eq!(eval("2rem", None, "px").as_deref(), Some("2rem"));
    assert_eq!(eval("10px*3", None, "px").as_deref(), Some("30px"));
    assert_eq!(eval("2x3", None, "px").as_deref(), Some("6px"));
    assert_eq!(eval("*2", Some("16px"), "px").as_deref(), Some("32px"));
    assert_eq!(eval("(10+2)/4", None, "").as_deref(), Some("3"));
    assert_eq!(eval("1,5", None, "").as_deref(), Some("1.5"));
    assert_eq!(eval("-8", None, "px").as_deref(), Some("-8px"));
    assert_eq!(eval("50%", None, "px").as_deref(), Some("50%"));
    assert_eq!(eval("20", Some("50%"), "px").as_deref(), Some("20%"));
    assert_eq!(eval("auto", None, "px").as_deref(), Some("auto"));
    assert_eq!(
        eval("calc(100% - 20px)", None, "px").as_deref(),
        Some("calc(100% - 20px)")
    );
    assert_eq!(eval("1px+1rem", None, "px").as_deref(), Some("1px+1rem"));
    assert_eq!(eval("  ", None, "px"), None);
    assert_eq!(step("16px", "", 1.0, "px").as_deref(), Some("17px"));
    assert_eq!(step("", "24px", -10.0, "px").as_deref(), Some("14px"));
    assert_eq!(step("1.4", "", 0.1, "").as_deref(), Some("1.5"));
    assert_eq!(step("auto", "normal", 1.0, "px"), None);
}

#[test]
fn quatre_cotes() {
    use shorthand::{compact4, expand4};
    assert_eq!(
        expand4("0 auto").unwrap(),
        ["0", "auto", "0", "auto"].map(String::from)
    );
    assert_eq!(expand4("1px 2px 3px").unwrap()[3], "2px");
    assert!(expand4("var(--m)").is_none());
    assert!(expand4("10px / 20px").is_none());
    assert_eq!(
        compact4(&["0", "auto", "0", "auto"].map(String::from)),
        "0 auto"
    );
    assert_eq!(compact4(&["1px"; 4].map(String::from)), "1px");
    assert_eq!(
        compact4(&["1px", "2px", "3px", "2px"].map(String::from)),
        "1px 2px 3px"
    );
    assert_eq!(
        compact4(&["1px", "2px", "3px", "4px"].map(String::from)),
        "1px 2px 3px 4px"
    );
}

#[test]
fn raccourcis_lus() {
    let d = decls(&[("margin", "0 auto"), ("padding", "8px 16px")]);
    assert_eq!(
        read(&d, "margin-left").unwrap().value.as_deref(),
        Some("auto")
    );
    assert_eq!(
        read(&d, "margin-left").unwrap().via.as_deref(),
        Some("margin")
    );
    assert_eq!(
        read(&d, "padding-bottom").unwrap().value.as_deref(),
        Some("8px")
    );
    assert!(read(&d, "width").is_none());
    // La dernière déclaration l'emporte.
    let d = decls(&[("margin-top", "4px"), ("margin", "0")]);
    assert_eq!(read(&d, "margin-top").unwrap().value.as_deref(), Some("0"));
    let d = decls(&[("margin", "0"), ("margin-top", "4px")]);
    assert_eq!(read(&d, "margin-top").unwrap().via, None);
    // Variable : fixée, valeur inconnue.
    let d = decls(&[("margin", "var(--m)")]);
    assert_eq!(read(&d, "margin-top").unwrap().value, None);
    // Bordures.
    let d = decls(&[("border", "1px solid #ddd")]);
    assert_eq!(
        read(&d, "border-color").unwrap().value.as_deref(),
        Some("#ddd")
    );
    assert_eq!(
        read(&d, "border-top-style").unwrap().value.as_deref(),
        Some("solid")
    );
    let d = decls(&[("border-bottom", "2px dashed var(--line)")]);
    assert_eq!(
        read(&d, "border-bottom-color").unwrap().value.as_deref(),
        Some("var(--line)")
    );
    assert!(read(&d, "border-top-color").is_none());
    let d = decls(&[("border-radius", "4px 8px")]);
    assert_eq!(
        read(&d, "border-bottom-right-radius")
            .unwrap()
            .value
            .as_deref(),
        Some("4px")
    );
    // Fond et soulignement.
    let d = decls(&[("background", "url(a.png) center / cover no-repeat #fafafa")]);
    assert_eq!(
        read(&d, "background-color").unwrap().value.as_deref(),
        Some("#fafafa")
    );
    assert_eq!(
        read(&d, "background-image").unwrap().value.as_deref(),
        Some("url(a.png)")
    );
    let d = decls(&[("background", "var(--surface)")]);
    assert_eq!(
        read(&d, "background-color").unwrap().value.as_deref(),
        Some("var(--surface)")
    );
    let d = decls(&[("text-decoration", "underline dotted red")]);
    assert_eq!(
        read(&d, "text-decoration-line").unwrap().value.as_deref(),
        Some("underline")
    );
    let d = decls(&[("font", "600 16px/1.4 Geist")]);
    assert_eq!(read(&d, "font-weight").unwrap().value, None);
}

#[test]
fn important_l_emporte() {
    let mut d = decls(&[("color", "red"), ("color", "blue")]);
    d[0].important = true;
    assert_eq!(read(&d, "color").unwrap().value.as_deref(), Some("red"));
}

fn e(p: &str, v: Option<&str>) -> Edit {
    Edit {
        prop: p.into(),
        value: v.map(String::from),
        important: false,
    }
}

#[test]
fn raccourcis_modifies_sur_place() {
    let d = decls(&[("margin", "0 auto")]);
    assert_eq!(
        edits(&d, "margin-top", Some("20px")).unwrap(),
        vec![e("margin", Some("20px auto 0"))]
    );
    // Retrait d'un côté : raccourci remplacé par les trois autres.
    assert_eq!(
        edits(&d, "margin-top", None).unwrap(),
        vec![
            e("margin", None),
            e("margin-right", Some("auto")),
            e("margin-bottom", Some("0")),
            e("margin-left", Some("auto")),
        ]
    );
    // Propriété absente : ajoutée ; présente : remplacée.
    let d = decls(&[("color", "red")]);
    assert_eq!(
        edits(&d, "width", Some("10px")).unwrap(),
        vec![e("width", Some("10px"))]
    );
    assert_eq!(edits(&d, "color", None).unwrap(), vec![e("color", None)]);
    assert_eq!(edits(&d, "width", None).unwrap(), vec![]);
    // Bordure : partie remplacée ou retirée.
    let d = decls(&[("border", "1px solid #ddd")]);
    assert_eq!(
        edits(&d, "border-color", Some("red")).unwrap(),
        vec![e("border", Some("1px solid red"))]
    );
    assert_eq!(
        edits(&d, "border-width", None).unwrap(),
        vec![e("border", Some("solid #ddd"))]
    );
    // Un côté seul : déclaration ajoutée après `border`.
    assert_eq!(
        edits(&d, "border-top-width", Some("3px")).unwrap(),
        vec![e("border-top-width", Some("3px"))]
    );
    assert_eq!(edits(&d, "border-top-width", None), None);
    // Fond.
    let d = decls(&[("background", "#fff")]);
    assert_eq!(
        edits(&d, "background-color", Some("#000")).unwrap(),
        vec![e("background", Some("#000"))]
    );
    assert_eq!(
        edits(&d, "background-color", None).unwrap(),
        vec![e("background", None)]
    );
    let d = decls(&[("background", "url(a.png) no-repeat")]);
    assert_eq!(
        edits(&d, "background-color", Some("red")).unwrap(),
        vec![e("background", Some("url(a.png) no-repeat red"))]
    );
    // Soulignement.
    let d = decls(&[("text-decoration", "underline dotted")]);
    assert_eq!(
        edits(&d, "text-decoration-line", Some("line-through")).unwrap(),
        vec![e("text-decoration", Some("line-through dotted"))]
    );
    // Écart : lignes et colonnes réglées à part dans `gap`.
    let d = decls(&[("gap", "24px")]);
    assert_eq!(
        read(&d, "column-gap").unwrap().value.as_deref(),
        Some("24px")
    );
    assert_eq!(
        edits(&d, "column-gap", Some("48px")).unwrap(),
        vec![e("gap", Some("24px 48px"))]
    );
    let d = decls(&[("gap", "24px 48px")]);
    assert_eq!(read(&d, "row-gap").unwrap().value.as_deref(), Some("24px"));
    assert_eq!(
        edits(&d, "row-gap", Some("48px")).unwrap(),
        vec![e("gap", Some("48px"))]
    );
    // Variable : propriété écrite après le raccourci (retirée d'abord si elle le précède).
    let d = decls(&[("margin-top", "4px"), ("margin", "var(--m)")]);
    assert_eq!(
        edits(&d, "margin-top", Some("8px")).unwrap(),
        vec![e("margin-top", None), e("margin-top", Some("8px"))]
    );
}

#[test]
fn ombres() {
    let s = parse_shadows("0 4px 12px rgba(0, 0, 0, 0.15), inset 0 0 0 1px #ddd").unwrap();
    assert_eq!(s.len(), 2);
    assert_eq!(
        (s[0].x.as_str(), s[0].blur.as_str(), s[0].color.as_str()),
        ("0", "12px", "rgba(0, 0, 0, 0.15)")
    );
    assert!(s[1].inset);
    assert_eq!(s[1].spread, "1px");
    assert_eq!(
        format_shadows(&s),
        "0 4px 12px 0px rgba(0, 0, 0, 0.15), inset 0 0 0 1px #ddd"
    );
    assert_eq!(parse_shadows("none").unwrap(), vec![]);
    assert_eq!(format_shadows(&[]), "none");
    assert_eq!(parse_shadows("var(--shadow)"), None);
    assert_eq!(
        parse_shadows("2px 2px var(--c)").unwrap()[0].color,
        "var(--c)"
    );
}

#[test]
fn divers() {
    assert_eq!(
        split_top("a, rgb(1, 2, 3), 'x, y'", ','),
        vec!["a", "rgb(1, 2, 3)", "'x, y'"]
    );
    assert_eq!(first_family("\"Geist\", system-ui, sans-serif"), "Geist");
    assert_eq!(grid_count("repeat(3, 1fr)"), Some(3));
    assert_eq!(grid_count("1fr 2fr"), Some(2));
    assert_eq!(grid_count("[a] 1fr"), None);
    assert_eq!(grid_columns(4), "repeat(4, minmax(0, 1fr))");
}

#[test]
fn valeurs_negatives_ramenees_a_zero() {
    // D-125 : intérieur, taille, écart, arrondi s'arrêtent à zéro ; marge extérieure et décalage
    // restent libres.
    assert_eq!(super::clamp_non_negative("padding-left", "-4px"), "0px");
    assert_eq!(super::clamp_non_negative("gap", "-2rem 12px"), "0rem 12px");
    assert_eq!(super::clamp_non_negative("opacity", "-0.2"), "0");
    assert_eq!(super::clamp_non_negative("margin-left", "-4px"), "-4px");
    assert_eq!(super::clamp_non_negative("top", "-10%"), "-10%");
    assert_eq!(
        super::clamp_non_negative("width", "calc(100% - 4px)"),
        "calc(100% - 4px)"
    );
}
