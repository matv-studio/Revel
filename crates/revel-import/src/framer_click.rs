//! Composants Framer qui changent de variante au clic (carte « Outils » qui se déplie), D-111.
//!
//! La page publiée ne contient que la variante par défaut. La variante ouverte est relue dans
//! le code du composant (`framer_jsx`) ; ses parties propres sont ajoutées à la page, masquées
//! tant que l'élément racine n'a pas la classe `is-active` (état « Actif » des styles). Les
//! éléments qui réagissent à l'appui reçoivent l'interaction Revel
//! `click class:is-active closest:.<racine>` ; les règles CSS de la variante ouverte
//! (`.framer-v-ouverte`) deviennent celles de la racine active (`.framer-v-defaut.is-active`).

use crate::Report;
use crate::crawl::Fetch;
use crate::framer::{ClickComponent, Modules};
use crate::framer_jsx::{Out, Render, parse_at};
use revel_model::{Attr, Decl, Node, Site, StyleItem, StyleRule};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Classe principale d'un élément (première `framer-…` qui n'est pas une classe de variante).
fn main_class(n: &Node) -> Option<String> {
    let Node::Element { attrs, .. } = n else {
        return None;
    };
    attrs
        .iter()
        .find(|a| a.name == "class")?
        .value
        .as_deref()?
        .split_whitespace()
        .find(|c| c.starts_with("framer-") && !c.starts_with("framer-v-"))
        .map(str::to_string)
}

fn has_class(attrs: &[Attr], c: &str) -> bool {
    attrs
        .iter()
        .find(|a| a.name == "class")
        .and_then(|a| a.value.as_deref())
        .is_some_and(|v| v.split_whitespace().any(|x| x == c))
}

/// Racine rendue d'une variante (premier élément) : (classe principale, enfants) ; `texts` :
/// textes des propriétés de l'utilisation.
fn render_variant(
    cc: &ClickComponent,
    variant: &str,
    texts: &HashMap<String, String>,
    out: &mut Out,
) -> Option<(Option<String>, Vec<Node>)> {
    let src = &cc.source;
    let state = regex::Regex::new(&format!(
        r"\(\)=>([\w$]+)(?:===|!==)`(?:{}|{})`",
        cc.open, cc.default
    ))
    .expect("regex")
    .captures(src)?[1]
        .to_string();
    let mut r = Render {
        variant,
        conds: HashMap::new(),
        variants: cc.variants.clone(),
        tags: HashMap::new(),
        texts: texts.clone(),
    };
    r.read_defs(src, &state);
    let co = src.rfind("cycleOrder:")?;
    let ret = co + src[co..].find("return ")? + "return ".len();
    let (e, _) = parse_at(src, ret);
    let nodes = r.render(&e, out, false);
    let root = nodes
        .into_iter()
        .find(|n| matches!(n, Node::Element { .. }))?;
    let class = main_class(&root);
    let Node::Element { children, .. } = root else {
        return None;
    };
    Some((class, children))
}

/// Texte du premier élément de la classe `class` dans `nodes`.
fn text_in(nodes: &[Node], class: &str) -> Option<String> {
    fn text(nodes: &[Node], out: &mut String) {
        for n in nodes {
            match n {
                Node::Text { text: t } => out.push_str(t),
                Node::Element { children, .. } => text(children, out),
                _ => {}
            }
        }
    }
    for n in nodes {
        if let Node::Element {
            attrs, children, ..
        } = n
        {
            if has_class(attrs, class) {
                let mut s = String::new();
                text(children, &mut s);
                return Some(s.trim().to_string()).filter(|s| !s.is_empty());
            }
            if let Some(t) = text_in(children, class) {
                return Some(t);
            }
        }
    }
    None
}

fn walk_mut(nodes: &mut [Node], f: &mut dyn FnMut(&mut Node) -> bool) {
    for n in nodes.iter_mut() {
        if !f(n) {
            continue;
        }
        match n {
            Node::Element { children, .. } | Node::Each { children, .. } => walk_mut(children, f),
            Node::If {
                then, otherwise, ..
            } => {
                walk_mut(then, f);
                walk_mut(otherwise, f);
            }
            _ => {}
        }
    }
}

fn set_attr(attrs: &mut Vec<Attr>, name: &str, value: String) {
    match attrs.iter_mut().find(|a| a.name == name) {
        Some(a) => a.value = Some(value),
        None => attrs.push(Attr {
            name: name.into(),
            value: Some(value),
            when: None,
        }),
    }
}

fn rule(selector: String, decls: Vec<Decl>) -> StyleItem {
    StyleItem::Rule(StyleRule {
        id: None,
        selector,
        media: None,
        decls,
    })
}

fn decl(d: &str, important: bool) -> Option<Decl> {
    let (p, v) = d.split_once(':')?;
    Some(Decl {
        prop: p.trim().to_string(),
        value: v.trim().to_string(),
        important,
    })
}

/// Copie locale des images de framerusercontent.com ajoutées (D-36 : site indépendant).
fn localize_images(
    nodes: &mut [Node],
    dest: &Path,
    fetch: &dyn Fetch,
    done: &mut HashMap<String, Option<String>>,
) {
    walk_mut(nodes, &mut |n| {
        if let Node::Element { tag, attrs, .. } = n
            && tag == "img"
            && let Some(src) = attrs
                .iter()
                .find(|a| a.name == "src")
                .and_then(|a| a.value.clone())
            && src.starts_with("https://framerusercontent.com/")
        {
            let local = done
                .entry(src.clone())
                .or_insert_with(|| {
                    let url = url::Url::parse(&src).ok()?;
                    let rel = format!("_externe/framerusercontent.com{}", url.path());
                    let file = dest.join(&rel);
                    if !file.is_file() {
                        let got = fetch.get(&url).ok().filter(|f| f.status == 200)?;
                        std::fs::create_dir_all(file.parent()?).ok()?;
                        std::fs::write(&file, &got.body).ok()?;
                    }
                    Some(format!("/{rel}"))
                })
                .clone();
            if let Some(l) = local {
                set_attr(attrs, "src", l);
            }
        }
        true
    });
}

/// Ajoute aux pages la variante ouverte au clic des composants de `m`.
pub fn import_click_variants(
    site: &mut Site,
    dest: &Path,
    fetch: &dyn Fetch,
    report: &mut Report,
) -> usize {
    let m = Modules::load(dest);
    let mut done = 0;
    let mut images = HashMap::new();
    for cc in &m.clicks {
        let (Some(dclass), Some(oclass)) = (
            m.class_of_variant(&cc.default),
            m.class_of_variant(&cc.open),
        ) else {
            continue;
        };
        let mut first = Out::default();
        let Some((root_class, closed)) =
            render_variant(cc, &cc.default, &HashMap::new(), &mut first)
        else {
            continue;
        };
        let mut second = Out::default();
        let Some((_, open)) = render_variant(cc, &cc.open, &HashMap::new(), &mut second) else {
            continue;
        };
        let taps: Vec<String> = first.taps.iter().chain(&second.taps).cloned().collect();
        let text_vars: Vec<(String, String)> = first.text_vars.clone();
        let closed_cls: HashSet<String> = closed.iter().filter_map(main_class).collect();
        let open_cls: HashSet<String> = open.iter().filter_map(main_class).collect();
        let action = format!("click class:is-active closest:.{dclass}");
        let mut found = 0;
        let mut bodies: Vec<&mut Vec<Node>> = site.pages.iter_mut().map(|p| &mut p.body).collect();
        bodies.extend(site.layouts.iter_mut().map(|l| &mut l.body));
        for body in bodies {
            walk_mut(body, &mut |n| {
                let Node::Element {
                    attrs, children, ..
                } = n
                else {
                    return true;
                };
                if !has_class(attrs, dclass) {
                    return true;
                }
                found += 1;
                // Textes de cette utilisation (propriétés du composant), lus dans la page.
                let mut texts = HashMap::new();
                for (var, class) in &text_vars {
                    if let Some(t) = text_in(children, class) {
                        texts.insert(var.clone(), t);
                    }
                }
                let mut open = render_variant(cc, &cc.open, &texts, &mut Out::default())
                    .map(|(_, o)| o)
                    .unwrap_or_default();
                localize_images(&mut open, dest, fetch, &mut images);
                // Parties de la variante ouverte, à leur place parmi les parties communes.
                let mut pos = 0;
                for o in &open {
                    let c = main_class(o);
                    match children
                        .iter()
                        .position(|x| main_class(x) == c && c.is_some())
                    {
                        Some(i) => pos = i + 1,
                        None => {
                            children.insert(pos, o.clone());
                            pos += 1;
                        }
                    }
                }
                let tapset: HashSet<&String> = taps.iter().collect();
                walk_mut(children, &mut |x| {
                    let tap = main_class(x).is_some_and(|c| tapset.contains(&c));
                    if tap
                        && let Node::Element { attrs, .. } = x
                        && !attrs.iter().any(|a| a.name == "data-revel-on")
                    {
                        set_attr(attrs, "data-revel-on", action.clone());
                    }
                    true
                });
                false
            });
        }
        if found == 0 {
            continue;
        }
        done += 1;
        let Some(sheet) = site.stylesheet.as_mut() else {
            continue;
        };
        // Règles de la variante ouverte : celles de la racine active.
        let re =
            regex::Regex::new(&format!(r"\.{}([^\w-]|$)", regex::escape(oclass))).expect("regex");
        let to = format!(".{dclass}.is-active$1");
        for item in &mut sheet.items {
            match item {
                StyleItem::Rule(r) => {
                    r.selector = re.replace_all(&r.selector, to.as_str()).into_owned()
                }
                StyleItem::Raw { css } => *css = re.replace_all(css, to.as_str()).into_owned(),
                StyleItem::Comment { .. } => {}
            }
        }
        let hide = || {
            vec![Decl {
                prop: "display".into(),
                value: "none".into(),
                important: true,
            }]
        };
        for c in open_cls.difference(&closed_cls) {
            sheet
                .items
                .push(rule(format!(".{dclass}:not(.is-active) > .{c}"), hide()));
        }
        for c in closed_cls.difference(&open_cls) {
            sheet
                .items
                .push(rule(format!(".{dclass}.is-active > .{c}"), hide()));
        }
        // Styles écrits de la variante ouverte, élément par élément.
        let dur = m.variant_duration(&cc.open);
        for (el, decls) in m.variant_styles(&cc.open) {
            let sel = if root_class.as_deref() == Some(el) {
                format!(".{dclass}.is-active")
            } else {
                format!(".{dclass}.is-active .{el}")
            };
            let ds: Vec<Decl> = decls.iter().filter_map(|d| decl(d, true)).collect();
            let props: Vec<String> = ds
                .iter()
                .filter(|d| !d.prop.starts_with("--"))
                .map(|d| format!("{} {dur}s cubic-bezier(.33,1,.68,1)", d.prop))
                .collect();
            if !props.is_empty() {
                let base = if root_class.as_deref() == Some(el) {
                    format!(".{dclass}")
                } else {
                    format!(".{dclass} .{el}")
                };
                sheet.items.push(rule(
                    base,
                    vec![Decl {
                        prop: "transition".into(),
                        value: props.join(","),
                        important: false,
                    }],
                ));
            }
            sheet.items.push(rule(sel, ds));
        }
        report.warnings.push(format!(
            "Framer : « {} » s'ouvre au clic (classe is-active, interaction de l'élément appuyé) ; contenu ouvert repris du code du composant",
            cc.name
        ));
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carte_qui_se_deplie() {
        let mut m = Modules::default();
        m.read(
            r#"rn=t((()=>{Vt=v(E(f.div)),Wt={nYfgzyMOx:`framer-v-ouvert`,S5bmGwolh:`framer-v-ferme`},Qt={Default:`S5bmGwolh`,Expanded:`nYfgzyMOx`};function X(){let{variant:S}=ce({cycleOrder:Ht,defaultVariant:`S5bmGwolh`,variantClassNames:Wt}),P=M(async(...e)=>{await N(()=>O(`nYfgzyMOx`),200)}),re=M(async(...e)=>{await N(()=>O(`S5bmGwolh`),100)}),ae=()=>S!==`nYfgzyMOx`,I=()=>S===`nYfgzyMOx`;return a(p,{children:c(Vt,{className:R(ie,`framer-carte`),children:[ae()&&a(f.div,{className:`framer-resume`,children:`Résumé`}),I()&&c(f.div,{className:`framer-liste`,children:`Blender`}),a(f.div,{className:`framer-fleche`,onTap:P,variants:{nYfgzyMOx:{rotate:180}}})]})})}X.displayName=`3D`}));"#,
            "outils.mjs",
        );
        m.finish();
        assert_eq!(m.clicks.len(), 1, "{:?}", m.click_components);
        let carte = Node::Element {
            id: None,
            tag: "div".into(),
            attrs: vec![Attr {
                name: "class".into(),
                value: Some("framer-X framer-carte framer-v-ferme".into()),
                when: None,
            }],
            children: vec![
                Node::Element {
                    id: None,
                    tag: "div".into(),
                    attrs: vec![Attr {
                        name: "class".into(),
                        value: Some("framer-resume".into()),
                        when: None,
                    }],
                    children: vec![],
                },
                Node::Element {
                    id: None,
                    tag: "div".into(),
                    attrs: vec![Attr {
                        name: "class".into(),
                        value: Some("framer-fleche".into()),
                        when: None,
                    }],
                    children: vec![],
                },
            ],
        };
        let dir = std::env::temp_dir().join(format!("revel-click-{}", std::process::id()));
        let file = dir.join("_externe/framerusercontent.com/sites/a/outils.mjs");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        // Modules relus depuis le dossier : le même code.
        let src = r#"rn=t((()=>{Vt=v(E(f.div)),Wt={nYfgzyMOx:`framer-v-ouvert`,S5bmGwolh:`framer-v-ferme`},Qt={Default:`S5bmGwolh`,Expanded:`nYfgzyMOx`};function X(){let{variant:S}=ce({cycleOrder:Ht,defaultVariant:`S5bmGwolh`,variantClassNames:Wt}),P=M(async(...e)=>{await N(()=>O(`nYfgzyMOx`),200)}),re=M(async(...e)=>{await N(()=>O(`S5bmGwolh`),100)}),ae=()=>S!==`nYfgzyMOx`,I=()=>S===`nYfgzyMOx`;return a(p,{children:c(Vt,{className:R(ie,`framer-carte`),children:[ae()&&a(f.div,{className:`framer-resume`,children:`Résumé`}),I()&&c(f.div,{className:`framer-liste`,children:`Blender`}),a(f.div,{className:`framer-fleche`,onTap:P,variants:{nYfgzyMOx:{rotate:180}}})]})})}X.displayName=`3D`}));"#;
        std::fs::write(&file, src).unwrap();
        let mut site = Site::new("t", "fr");
        let mut page: revel_model::Page =
            serde_json::from_value(serde_json::json!({"id": "index", "path": "/", "body": []}))
                .unwrap();
        page.body = vec![carte];
        site.pages.push(page);
        site.stylesheet = Some(revel_model::Stylesheet {
            url: "/s.css".into(),
            source: None,
            items: vec![rule(
                ".framer-X.framer-v-ouvert .framer-liste".into(),
                vec![],
            )],
        });
        struct NoFetch;
        impl Fetch for NoFetch {
            fn get(&self, _: &url::Url) -> Result<crate::crawl::Fetched, String> {
                Err("hors ligne".into())
            }
        }
        let mut report = Report::default();
        assert_eq!(
            import_click_variants(&mut site, &dir, &NoFetch, &mut report),
            1
        );
        let html = serde_json::to_string(&site.pages[0].body).unwrap();
        assert!(
            html.contains("framer-liste") && html.contains("Blender"),
            "{html}"
        );
        assert!(
            html.contains("click class:is-active closest:.framer-v-ferme"),
            "{html}"
        );
        let css: Vec<String> = site
            .stylesheet
            .unwrap()
            .items
            .iter()
            .filter_map(|i| match i {
                StyleItem::Rule(r) => Some(format!(
                    "{}{{{}}}",
                    r.selector,
                    r.decls
                        .iter()
                        .map(|d| format!(
                            "{}:{}{}",
                            d.prop,
                            d.value,
                            if d.important { "!" } else { "" }
                        ))
                        .collect::<Vec<_>>()
                        .join(";")
                )),
                _ => None,
            })
            .collect();
        let css = css.join("\n");
        assert!(
            css.contains(".framer-X.framer-v-ferme.is-active .framer-liste{}"),
            "{css}"
        );
        assert!(
            css.contains(".framer-v-ferme:not(.is-active) > .framer-liste{display:none!}"),
            "{css}"
        );
        assert!(
            css.contains(".framer-v-ferme.is-active > .framer-resume{display:none!}"),
            "{css}"
        );
        assert!(
            css.contains(".framer-v-ferme.is-active .framer-fleche{rotate:180deg!}"),
            "{css}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
