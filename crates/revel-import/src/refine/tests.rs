use super::*;
use std::sync::Arc;

fn site(html: &str, css: &str) -> Site {
    let mut s = Site::new("Essai", "fr");
    let mut page: revel_model::Page =
        serde_json::from_value(serde_json::json!({"id": "index", "path": "/"})).unwrap();
    page.body = crate::html::fragment(html);
    s.pages.push(page);
    s.stylesheet = Some(revel_model::Stylesheet {
        url: "/site.css".into(),
        source: None,
        items: revel_style::parse(css).unwrap(),
    });
    s.ensure_ids();
    s
}

/// Page produite, sans les repères de l'éditeur ni les blancs entre balises.
fn page_html(s: &Site) -> String {
    let input = revel_render::BuildInput {
        site: s,
        entries: Vec::new(),
        locales: Vec::new(),
        environment: "production".into(),
        version: "v1".into(),
        hooks: Arc::new(revel_render::PlainAssets),
        now: chrono::Utc::now(),
    };
    let h = revel_render::render_editor(
        &input,
        "/",
        &revel_render::EditorOptions {
            scripts: false,
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap()
    .html;
    // Guillemets d'un texte : écrits tels quels ou en entité selon qu'il est écrit dans la page
    // ou lu d'une propriété ; le texte montré est le même.
    let h = revel_render::strip_editor_marks(&h).replace("&quot;", "\"");
    h.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn find<'a>(nodes: &'a [Node], pred: &dyn Fn(&Node) -> bool) -> Vec<&'a Node> {
    let mut out = Vec::new();
    revel_model::ids::walk(nodes, &mut |n| {
        if pred(n) {
            out.push(n);
        }
    });
    out
}

fn base(n: &Node) -> Option<&str> {
    match n {
        Node::Element { attrs, .. } => attr(attrs, revel_model::BASE_ATTR),
        _ => None,
    }
}

const CARDS: &str = r#"<section class="liste">
<article class="carte"><img class="carte__image" src="/a.jpg" alt="Projet A"><h3 class="carte__titre">Projet A</h3><p class="carte__texte">Un film "court" &amp; net.</p><a class="carte__lien" href="/a/">Voir</a></article>
<article class="carte"><img class="carte__image" src="/b.jpg" alt="Projet B"><h3 class="carte__titre">Projet B</h3><p class="carte__texte">Une série.</p><a class="carte__lien" href="/b/">Voir</a></article>
<article class="carte"><img class="carte__image" src="/c.jpg" alt="Projet C"><h3 class="carte__titre">Projet C</h3><p class="carte__texte">Un clip \ rapide.</p><a class="carte__lien" href="/c/">Voir</a></article>
</section>"#;

#[test]
fn elements_repetes_en_composant_meme_rendu() {
    let mut s = site(CARDS, ".carte { color: #111111; }");
    let before = page_html(&s);
    let r = refine(&mut s, &Options { tools: false });
    assert_eq!(r.components, vec![("Carte".to_string(), 3)]);
    let c = &s.components[0];
    assert_eq!(c.id, "carte");
    let names: Vec<&str> = c.props.iter().map(|p| p.name.as_str()).collect();
    // Le libellé du lien (« Voir ») ne change pas : pas de propriété.
    assert_eq!(names, ["image", "alt", "titre", "texte", "lien"]);
    assert_eq!(c.props[0].kind, revel_model::PropKind::Image);
    assert_eq!(c.props[4].kind, revel_model::PropKind::Link);
    let uses = find(&s.pages[0].body, &|n| matches!(n, Node::Use { .. }));
    assert_eq!(uses.len(), 3);
    assert_eq!(page_html(&s), before);
    assert!(
        revel_edit::editable::audit(&s).is_empty(),
        "{:?}",
        revel_edit::editable::audit(&s)
    );
}

#[test]
fn liste_de_cartes_pas_composant() {
    // Deux listes identiques de trois cartes : les cartes sont les composants, pas la liste.
    let html = format!("{CARDS}{CARDS}");
    let mut s = site(&html, "");
    let before = page_html(&s);
    let r = refine(&mut s, &Options { tools: false });
    assert_eq!(r.components, vec![("Carte".to_string(), 6)]);
    assert_eq!(page_html(&s), before);
}

#[test]
fn couleurs_et_tailles_en_variables() {
    let mut s = site(
        r#"<p class="a">A</p><p class="b">B</p><p class="c">C</p>"#,
        ":root { --marque: #ff5500; }\nbody { color: #111111; background: #ffffff; }\n\
.a { color: #2f6fed; font-size: 18px; max-width: 1200px; }\n\
.b { background-color: #2f6fed; font-size: 18px; max-width: 1200px; border-color: #ff5500; }\n\
.c { color: rgb(47, 111, 237); font-size: 18px; border: 1px solid #2f6fed; }",
    );
    let r = refine(&mut s, &Options { tools: false });
    assert_eq!((r.colors, r.sizes), (3, 2));
    let css = revel_style::generate(&s.stylesheet.as_ref().unwrap().items);
    assert!(css.contains("--texte: #111111"), "{css}");
    assert!(css.contains("--fond: #ffffff"), "{css}");
    assert!(css.contains("--bleu: #2f6fed"), "{css}");
    assert!(css.contains("--texte-18: 18px"), "{css}");
    assert!(css.contains("--largeur-site: 1200px"), "{css}");
    assert!(
        css.contains(
            ".a { color: var(--bleu); font-size: var(--texte-18); max-width: var(--largeur-site); }"
        ),
        "{css}"
    );
    // Variable déjà écrite reprise ; valeur composée laissée.
    assert!(css.contains("border-color: var(--marque)"), "{css}");
    assert!(css.contains("border: 1px solid #2f6fed"), "{css}");
    assert!(
        css.contains("body { color: var(--texte); background: var(--fond); }"),
        "{css}"
    );
}

#[test]
fn details_en_accordeon() {
    let mut s = site(
        r#"<h2>Questions</h2>
<details class="q" open><summary class="q__titre">Combien ?</summary><p class="q__texte">Dix.</p></details>
<details class="q"><summary class="q__titre">Quand ?</summary><p class="q__texte">Demain.</p><p>Ou après.</p></details>"#,
        "",
    );
    let r = refine(&mut s, &Options::default());
    assert_eq!(r.tools, vec![("accordéon".to_string(), 1)]);
    let roots = find(&s.pages[0].body, &|n| base(n) == Some("accordion-group"));
    assert_eq!(roots.len(), 1);
    let rows = element_kids(roots[0]);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| revel_model::is_item(r)));
    assert_eq!(classes(rows[0]), ["q"]);
    assert!(matches!(rows[0], Node::Element { attrs, .. } if has_attr(attrs, "open")));
    assert_eq!(plain(element_kids(rows[1])[0]), "Quand ?");
    assert_eq!(plain(element_kids(rows[1])[1]), "Demain. Ou après.");
    let html = page_html(&s);
    assert!(
        html.contains("Questions") && html.contains("q__texte"),
        "{html}"
    );
}

#[test]
fn blocs_nommes_en_accordeon() {
    let mut s = site(
        r##"<div class="faq"><div class="faq-item"><button class="faq-question" aria-expanded="false">Prix ?<span class="plus"></span></button><div class="faq-answer"><p>Sur devis.</p></div></div>
<div class="faq-item"><button class="faq-question" aria-expanded="false">Délai ?<span class="plus"></span></button><div class="faq-answer"><p>Un mois.</p></div></div></div>"##,
        "",
    );
    let r = refine(&mut s, &Options::default());
    assert_eq!(r.tools, vec![("accordéon".to_string(), 1)]);
    let root = find(&s.pages[0].body, &|n| base(n) == Some("accordion-group"))[0];
    assert_eq!(classes(root), ["faq"]);
    assert_eq!(plain(root), "Prix ? Sur devis. Délai ? Un mois.");
}

#[test]
fn roles_aria_en_onglets() {
    let mut s = site(
        r##"<div class="offres"><div role="tablist"><button role="tab" aria-selected="true">Studio</button><button role="tab">Agence</button></div>
<div role="tabpanel"><p>Petites équipes.</p></div><div role="tabpanel" hidden><p>Grandes équipes.</p><img src="/x.jpg" alt=""></div></div>"##,
        "",
    );
    let r = refine(&mut s, &Options::default());
    assert_eq!(r.tools, vec![("onglets".to_string(), 1)]);
    let root = find(&s.pages[0].body, &|n| base(n) == Some("tabs"))[0];
    let tabs = find(
        std::slice::from_ref(root),
        &|n| matches!(n, Node::Element { attrs, .. } if has_attr(attrs, "data-revel-tab")),
    );
    assert_eq!(
        tabs.iter().map(|t| plain(t)).collect::<Vec<_>>(),
        ["Studio", "Agence"]
    );
    let panels = find(
        std::slice::from_ref(root),
        &|n| matches!(n, Node::Element { attrs, .. } if has_attr(attrs, "data-revel-panel")),
    );
    assert_eq!(panels.len(), 2);
    assert!(!matches!(panels[1], Node::Element { attrs, .. } if has_attr(attrs, "hidden")));
    assert_eq!(
        find(std::slice::from_ref(panels[1]), &|n| tag(n) == "img").len(),
        1
    );
}

#[test]
fn onglet_choisi_classe_d_etat_renommee() {
    let mut s = site(
        r##"<div class="w-tabs"><div class="w-tab-menu"><a class="w-tab-link choix choix--actif">Un</a><a class="w-tab-link choix">Deux</a></div>
<div class="w-tab-content"><div class="w-tab-pane volet"><p>A</p></div><div class="w-tab-pane volet"><p>B</p></div></div></div>"##,
        ".choix { color: gray; }\n.choix.choix--actif { color: black; }\n.autre.choix--actif { color: red; }\n.volet { display: none; padding: 8px; }",
    );
    let r = refine(&mut s, &Options::default());
    assert_eq!(r.tools, vec![("onglets".to_string(), 1)]);
    let root = find(&s.pages[0].body, &|n| base(n) == Some("tabs"))[0];
    let tabs = find(
        std::slice::from_ref(root),
        &|n| matches!(n, Node::Element { attrs, .. } if has_attr(attrs, "data-revel-tab")),
    );
    assert!(tabs.iter().all(|t| classes(t) == ["w-tab-link", "choix"]));
    let css = revel_style::generate(&s.stylesheet.as_ref().unwrap().items);
    assert!(
        css.contains(".choix.is-active {") && css.contains(".autre.choix--actif {"),
        "{css}"
    );
    // Panneaux gardés avec leur classe, sans leur état caché.
    assert!(css.contains(".volet { padding: 8px; }"), "{css}");
    assert_eq!(
        find(std::slice::from_ref(root), &|n| classes(n)
            .contains(&"volet"))
        .len(),
        2
    );
}

#[test]
fn bibliotheque_et_noms_en_carrousel() {
    let mut s = site(
        r##"<div class="swiper temoignages"><div class="swiper-wrapper">
<div class="swiper-slide"><blockquote class="avis">Parfait.</blockquote></div>
<div class="swiper-slide"><blockquote class="avis">Rapide.</blockquote></div>
<div class="swiper-slide"><blockquote class="avis">Clair.</blockquote></div></div>
<div class="swiper-pagination"></div><button class="swiper-button-prev">Précédent</button></div>
<div class="galerie-slider"><img src="/1.jpg" alt="Un"><img src="/2.jpg" alt="Deux"></div>
<div class="slider"><h2>Nos clients</h2><div class="track"><p>A</p><p>B</p></div></div>"##,
        "",
    );
    let r = refine(&mut s, &Options::default());
    assert_eq!(r.tools, vec![("carrousels".to_string(), 2)]);
    // Titre hors de la piste : pas repris par l'outil, construction laissée et signalée.
    assert_eq!(r.skipped, ["carrousel"]);
    let roots = find(&s.pages[0].body, &|n| base(n) == Some("carousel"));
    assert_eq!(roots.len(), 2);
    assert_eq!(classes(roots[0]), ["temoignages"]);
    let track = element_kids(roots[0])[0];
    let slides = element_kids(track);
    assert_eq!(slides.len(), 3);
    // Enveloppe de la bibliothèque retirée : la diapositive est la citation.
    assert!(
        slides
            .iter()
            .all(|s| tag(s) == "blockquote" && classes(s) == ["avis"])
    );
    assert!(
        matches!(roots[0], Node::Element { attrs, .. } if attr(attrs, "style") == Some("--rv-show:3"))
    );
    assert_eq!(element_kids(element_kids(roots[1])[0]).len(), 2);
    assert!(page_html(&s).contains("Nos clients"));
}

#[test]
fn piste_a_crans_sans_nom_en_carrousel() {
    let mut s = site(
        r#"<ul class="rangee"><li class="vignette">A</li><li class="vignette">B</li><li class="vignette">C</li></ul>"#,
        ".rangee { display: flex; overflow-x: auto; scroll-snap-type: x mandatory; }\n.vignette { flex: 0 0 50%; }",
    );
    let r = refine(&mut s, &Options::default());
    assert_eq!(r.tools, vec![("carrousel".to_string(), 1)]);
    let root = find(&s.pages[0].body, &|n| base(n) == Some("carousel"))[0];
    let slides = element_kids(element_kids(root)[0]);
    assert!(
        slides
            .iter()
            .all(|s| tag(s) == "div" && classes(s) == ["vignette"])
    );
}

#[test]
fn entete_en_barre_de_navigation() {
    let mut s = site(
        r##"<header class="entete"><div class="entete__dans"><a class="logo" href="/"><img src="/logo.svg" alt="Studio"></a>
<nav class="menu"><ul class="menu__liste"><li><a class="menu__lien" href="/offres/">Offres</a></li><li><a class="menu__lien" href="/contact/">Contact</a></li></ul></nav>
<button class="burger" aria-label="Ouvrir le menu"><span></span><span></span><span></span></button></div></header>
<main><nav aria-label="Fil d'Ariane"><a href="/">Accueil</a><a href="/offres/">Offres</a></nav></main>
<footer><nav class="pied"><a href="/mentions/">Mentions</a><a href="/contact/">Contact</a></nav></footer>"##,
        ".entete__dans { display: flex; padding: 0 24px; }\n.menu__liste { display: flex; gap: 32px; }\n\
.burger { display: none; }\n@media (max-width: 991px) { .burger { display: block; } .menu__liste { display: none; } \
.menu__liste.open { display: flex; background: #ffffff; } }",
    );
    let r = refine(&mut s, &Options::default());
    assert_eq!(r.tools, vec![("barre de navigation".to_string(), 1)]);
    let root = find(&s.pages[0].body, &|n| base(n) == Some("navbar"))[0];
    // Dans l'en-tête gardé, avec les classes du conteneur remplacé.
    assert_eq!(classes(root), ["entete__dans"]);
    assert!(matches!(root, Node::Element { attrs, .. }
        if attr(attrs, "data-revel-collapse") == Some("tablet") && has_attr(attrs, "data-revel-own")));
    let k = element_kids(root);
    assert_eq!((tag(k[0]), classes(k[0])), ("a", vec!["logo"]));
    assert!(matches!(k[1], Node::Element { attrs, .. }
        if attr(attrs, PART) == Some("toggle") && attr(attrs, "aria-label") == Some("Ouvrir le menu")));
    assert_eq!(classes(k[1]), ["burger"]);
    // Classes de la liste des liens gardées sur le menu ; l'outil décide de son affichage, et
    // l'allure du menu ouvert du site (`.menu__liste.open`) devient celle du menu replié.
    assert_eq!(classes(k[2]), ["menu__liste"]);
    let css = revel_style::generate(&s.stylesheet.as_ref().unwrap().items);
    assert!(css.contains(".menu__liste { gap: 32px; }"), "{css}");
    assert!(
        css.contains(".menu__liste { background: #ffffff; }"),
        "{css}"
    );
    assert!(
        !css.contains(".menu__liste.open")
            && !css.contains("display: none")
            && !css.contains(".burger"),
        "{css}"
    );
    let links = element_kids(k[2]);
    assert_eq!(
        links.iter().map(|l| plain(l)).collect::<Vec<_>>(),
        ["Offres", "Contact"]
    );
    assert!(links.iter().all(|l| classes(l) == ["menu__lien"]));
    let html = page_html(&s);
    assert!(html.contains(r#"<header class="entete">"#), "{html}");
    // Fil d'Ariane et liens du pied de page laissés.
    assert!(
        html.contains(r#"<nav aria-label="Fil d'Ariane">"#)
            && html.contains(r#"<nav class="pied">"#),
        "{html}"
    );
}

#[test]
fn sous_menus_laisses_et_signales() {
    let html = r##"<header><a href="/"><img src="/l.svg" alt="L"></a><ul><li><a href="/a/">A</a><ul><li><a href="/a/1/">A1</a></li></ul></li><li><a href="/b/">B</a></li></ul></header>"##;
    let mut s = site(html, "");
    let before = page_html(&s);
    let r = refine(&mut s, &Options::default());
    assert!(r.tools.is_empty());
    assert_eq!(r.skipped, ["barre de navigation"]);
    assert_eq!(page_html(&s), before);
}

#[test]
fn sans_outils_rien_de_refait() {
    let html = r#"<details><summary>A</summary><p>B</p></details>"#;
    let mut s = site(html, "");
    let before = page_html(&s);
    let r = refine(&mut s, &Options { tools: false });
    assert_eq!(r, Report::default());
    assert_eq!(page_html(&s), before);
    assert!(summary(&r).is_none());
}

const PART: &str = revel_model::PART_ATTR;
