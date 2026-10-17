//! Adaptations propres à l'outil qui a produit le site importé (7.3, 7.4, 7.7) :
//!
//! - **Framer** : scripts du moteur de Framer retirés (le HTML publié est complet) ; animations
//!   d'apparition (`data-framer-appear-id`) reprises en animations Revel (`data-revel-appear`),
//!   état de départ masqué retiré ;
//! - **Webflow** : interactions d'apparition (au défilement, au chargement) reprises en animations
//!   Revel quand elles correspondent ; formulaires envoyés par Revel (`data-revel-form`) au lieu
//!   des serveurs de Webflow ;
//! - **AOS** (bibliothèque d'animations au défilement, `data-aos`) : reprise en animations Revel.
//!
//! Une animation reprise devient modifiable dans le panneau Interactions et le contenu est
//! visible dans le canevas (où les scripts du site ne tournent pas).

use revel_model::{Attr, Node};
use serde_json::Value;
use std::collections::HashMap;

/// Outil d'origine reconnu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Framer,
    Webflow,
    WordPress,
}

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Tool::Framer => "Framer",
            Tool::Webflow => "Webflow",
            Tool::WordPress => "WordPress",
        }
    }
}

fn attr<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(name))
        .and_then(|a| a.value.as_deref())
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

/// Outil d'après la balise `generator` et les marques propres à chaque outil.
pub fn tool(head: &[Node], html_attrs: &[Attr]) -> Option<Tool> {
    let generator = head.iter().find_map(|n| match n {
        Node::Element { tag, attrs, .. }
            if tag == "meta"
                && attr(attrs, "name").is_some_and(|v| v.eq_ignore_ascii_case("generator")) =>
        {
            attr(attrs, "content").map(str::to_ascii_lowercase)
        }
        _ => None,
    });
    let g = generator.unwrap_or_default();
    if g.starts_with("framer") {
        return Some(Tool::Framer);
    }
    if g.contains("webflow") || attr(html_attrs, "data-wf-site").is_some() {
        return Some(Tool::Webflow);
    }
    if g.starts_with("wordpress") {
        return Some(Tool::WordPress);
    }
    None
}

/// Parcourt les éléments (enfants compris) ; `f` renvoie `false` pour retirer l'élément.
fn retain_deep(nodes: &mut Vec<Node>, f: &mut dyn FnMut(&mut Node) -> bool) {
    nodes.retain_mut(|n| {
        if !f(n) {
            return false;
        }
        if let Node::Element { children, .. } = n {
            retain_deep(children, f);
        }
        true
    });
}

/// Déclarations d'un attribut `style` sans les propriétés `props`.
fn without_props(style: &str, props: &[&str]) -> String {
    style
        .split(';')
        .filter(|d| {
            let name = d
                .split(':')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            !name.is_empty() && !props.contains(&name.as_str())
        })
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(";")
}

/// Retire de l'attribut `style` les propriétés d'un état de départ d'animation.
fn clear_initial_state(attrs: &mut Vec<Attr>) {
    const PROPS: &[&str] = &[
        "opacity",
        "transform",
        "-webkit-transform",
        "-moz-transform",
        "-ms-transform",
        "transform-style",
        "will-change",
        "filter",
        "visibility",
    ];
    if let Some(i) = attrs.iter().position(|a| a.name == "style") {
        let v = without_props(attrs[i].value.as_deref().unwrap_or(""), PROPS);
        if v.is_empty() {
            attrs.remove(i);
        } else {
            attrs[i].value = Some(v);
        }
    }
}

/// Valeur `data-revel-appear` : animation, durée et délai (secondes), courbe, distance.
fn appear_value(preset: &str, dur: f64, delay: f64, ease: &str, dist: Option<f64>) -> String {
    let mut v = format!("{preset} dur={} delay={}", round(dur), round(delay));
    if ease != "smooth" {
        v.push_str(&format!(" ease={ease}"));
    }
    if let Some(d) = dist
        && d > 0.0
        && (d - 32.0).abs() > 0.5
    {
        v.push_str(&format!(" dist={}", d.round()));
    }
    v
}

fn round(x: f64) -> String {
    let r = (x * 100.0).round() / 100.0;
    if r.fract() == 0.0 {
        format!("{r:.0}")
    } else {
        format!("{r}")
    }
}

/// Animation Revel d'après un état de départ (décalage, échelle, opacité).
fn preset_for(x: f64, y: f64, scale: f64, rotate: f64) -> (&'static str, Option<f64>) {
    if y.abs() >= x.abs() && y.abs() > 0.5 {
        if y > 0.0 {
            ("up", Some(y))
        } else {
            ("down", Some(-y))
        }
    } else if x.abs() > 0.5 {
        if x < 0.0 {
            ("left", Some(-x))
        } else {
            ("right", Some(x))
        }
    } else if scale < 0.75 {
        ("pop", None)
    } else if scale < 0.999 {
        ("zoom-in", None)
    } else if scale > 1.001 {
        ("zoom-out", None)
    } else if rotate.abs() > 0.5 {
        ("rotate", None)
    } else {
        ("fade", None)
    }
}

// -------------------------------------------------------------------------------------------------
// Framer

/// Contenu d'un script `type="framer/appear"` d'identifiant `id`.
fn framer_json(nodes: &[Node], id: &str) -> Option<Value> {
    for n in nodes {
        if let Node::Element {
            tag,
            attrs,
            children,
            ..
        } = n
        {
            if tag == "script" && attr(attrs, "id") == Some(id) {
                let text: String = children
                    .iter()
                    .map(|c| match c {
                        Node::Html { html } => html.as_str(),
                        Node::Text { text } => text.as_str(),
                        _ => "",
                    })
                    .collect();
                return serde_json::from_str(&text).ok();
            }
            if let Some(v) = framer_json(children, id) {
                return Some(v);
            }
        }
    }
    None
}

/// Script ou lien du moteur de Framer (inutile une fois le HTML publié repris).
fn framer_runtime(n: &Node) -> bool {
    let Node::Element { tag, attrs, .. } = n else {
        return matches!(n, Node::Html { html } if matches!(html.as_str(), "<!--$-->" | "<!--/$-->" | "<!--$?-->" | "<!--$!-->"));
    };
    let src = attr(attrs, "src")
        .or_else(|| attr(attrs, "href"))
        .unwrap_or("");
    match tag.as_str() {
        "script" => {
            attr(attrs, "data-framer-bundle").is_some()
                || attr(attrs, "type").is_some_and(|t| t.starts_with("framer/"))
                || attr(attrs, "data-framer-appear-animation").is_some()
                || src.contains("events.framer.com")
                || src.contains("framer.com/m/")
                || (src.contains("framerusercontent.com") && src.ends_with(".mjs"))
                || attr(attrs, "data-preserve-internal-params").is_some()
        }
        "link" => {
            let rel = attr(attrs, "rel").unwrap_or("");
            (rel.contains("modulepreload")
                && (src.contains("framerusercontent") || src.ends_with(".mjs")))
                || (rel.contains("preconnect") && src.contains("framer"))
        }
        "div" => attr(attrs, "id") == Some("__framer-badge-container"),
        _ => false,
    }
}

/// Framer : moteur retiré, apparitions reprises. Renvoie le nombre d'animations reprises.
pub fn framer(head: &mut Vec<Node>, body: &mut Vec<Node>) -> usize {
    let appear = framer_json(head, "__framer__appearAnimationsContent")
        .or_else(|| framer_json(body, "__framer__appearAnimationsContent"));
    retain_deep(head, &mut |n| !framer_runtime(n));
    retain_deep(body, &mut |n| !framer_runtime(n));
    let mut count = 0;
    retain_deep(body, &mut |n| {
        if let Node::Element { attrs, .. } = n
            && let Some(id) = attr(attrs, "data-framer-appear-id").map(str::to_string)
        {
            let cfg = appear.as_ref().and_then(|a| a.get(&id)).and_then(|c| {
                c.get("default")
                    .or_else(|| c.as_object().and_then(|o| o.values().next()))
                    .cloned()
            });
            if let Some(cfg) = cfg {
                let init = &cfg["initial"];
                let num =
                    |v: &Value, k: &str, d: f64| v.get(k).and_then(Value::as_f64).unwrap_or(d);
                let (preset, dist) = preset_for(
                    num(init, "x", 0.0),
                    num(init, "y", 0.0),
                    num(init, "scale", 1.0),
                    num(init, "rotate", 0.0),
                );
                let t = &cfg["animate"]["transition"];
                let spring = t.get("type").and_then(Value::as_str) == Some("spring");
                let dur = if spring {
                    num(t, "duration", 0.6).max(0.4)
                } else {
                    num(t, "duration", 0.6)
                };
                let ease = if spring && num(t, "bounce", 0.0) > 0.3 {
                    "bounce"
                } else {
                    "smooth"
                };
                set_attr(
                    attrs,
                    "data-revel-appear",
                    appear_value(preset, dur, num(t, "delay", 0.0), ease, dist),
                );
                count += 1;
            }
            attrs.retain(|a| a.name != "data-framer-appear-id");
            clear_initial_state(attrs);
        }
        true
    });
    count += framer_text_effects(body);
    // État de départ d'un autre effet du moteur (élément masqué) : élément montré.
    retain_deep(body, &mut |n| {
        if let Node::Element { attrs, .. } = n
            && attr(attrs, "style").is_some_and(hidden_start)
        {
            clear_initial_state(attrs);
        }
        true
    });
    count
}

/// Style d'un élément masqué en attendant une animation du moteur de Framer.
fn hidden_start(style: &str) -> bool {
    let s = style.replace(' ', "");
    s.contains("opacity:0.001") || s.contains("opacity:0;")
}

/// Morceau d'un effet de texte de Framer : `<span style="display:inline-block;opacity:0.001;…">`
/// contenant du texte.
fn text_piece(n: &Node) -> bool {
    matches!(n, Node::Element { tag, attrs, children, .. } if text_piece_node(tag, attrs, children))
}

/// Effets de texte de Framer (lettres ou mots qui apparaissent l'un après l'autre) : morceaux
/// remis en texte, animation de texte Revel (`data-revel-text`) sur l'élément qui les contient.
fn framer_text_effects(nodes: &mut [Node]) -> usize {
    const BLOCKS: &[&str] = &[
        "p", "h1", "h2", "h3", "h4", "h5", "h6", "div", "a", "li", "span",
    ];
    let mut count = 0;
    for n in nodes.iter_mut() {
        let Node::Element {
            tag,
            attrs,
            children,
            ..
        } = n
        else {
            continue;
        };
        if BLOCKS.contains(&tag.as_str())
            && !text_piece_node(tag, attrs, children)
            && children.iter().any(text_piece_or_wrap)
        {
            // Réglages lus sur le premier morceau.
            let mut first = None;
            let mut single = true;
            fn unwrap(nodes: &mut Vec<Node>, first: &mut Option<String>, single: &mut bool) {
                let mut out = Vec::with_capacity(nodes.len());
                for n in nodes.drain(..) {
                    if text_piece(&n) {
                        if let Node::Element {
                            attrs, children, ..
                        } = n
                        {
                            if first.is_none() {
                                *first = attr(&attrs, "style").map(str::to_string);
                            }
                            let text: String = children
                                .iter()
                                .map(|c| match c {
                                    Node::Text { text } => text.as_str(),
                                    _ => "",
                                })
                                .collect();
                            if text.trim().chars().count() > 1 {
                                *single = false;
                            }
                            out.extend(children);
                        }
                    } else {
                        let mut n = n;
                        if let Node::Element { children, .. } = &mut n {
                            unwrap(children, first, single);
                        }
                        out.push(n);
                    }
                }
                *nodes = out;
            }
            unwrap(children, &mut first, &mut single);
            let style = first.unwrap_or_default().replace(' ', "");
            let preset = if style.contains("blur(") {
                "blur"
            } else if style.contains("translateY(") && !style.contains("translateY(0px)") {
                "up"
            } else {
                "fade"
            };
            set_attr(
                attrs,
                "data-revel-text",
                format!("{preset} split={}", if single { "chars" } else { "words" }),
            );
            count += 1;
            continue;
        }
        count += framer_text_effects(children);
    }
    count
}

fn text_piece_node(tag: &str, attrs: &[Attr], children: &[Node]) -> bool {
    tag == "span"
        && attr(attrs, "style").is_some_and(|s| {
            let s = s.replace(' ', "");
            s.contains("display:inline-block") && hidden_start(&s)
        })
        && children.iter().all(|c| matches!(c, Node::Text { .. }))
}

/// Morceau, ou enveloppe de morceaux (`<span style="white-space:nowrap">` par mot).
fn text_piece_or_wrap(n: &Node) -> bool {
    text_piece(n)
        || matches!(n, Node::Element { tag, children, .. }
            if tag == "span" && !children.is_empty() && children.iter().all(|c| text_piece(c) || matches!(c, Node::Text { text } if text.trim().is_empty())))
}

// -------------------------------------------------------------------------------------------------
// AOS

/// AOS : `data-aos="fade-up"` (durée et délai en millisecondes) repris en animation Revel.
pub fn aos(body: &mut Vec<Node>) -> usize {
    let mut count = 0;
    retain_deep(body, &mut |n| {
        if let Node::Element { attrs, .. } = n
            && let Some(kind) = attr(attrs, "data-aos").map(str::to_string)
        {
            let preset = match kind.as_str() {
                "fade-up" | "slide-up" => "up",
                "fade-down" | "slide-down" => "down",
                // `fade-left` : arrive de la droite en allant vers la gauche.
                "fade-left" | "slide-left" | "fade-up-left" | "fade-down-left" => "right",
                "fade-right" | "slide-right" | "fade-up-right" | "fade-down-right" => "left",
                "zoom-in" | "zoom-in-up" | "zoom-in-down" | "zoom-in-left" | "zoom-in-right" => {
                    "zoom-in"
                }
                "zoom-out" | "zoom-out-up" | "zoom-out-down" | "zoom-out-left"
                | "zoom-out-right" => "zoom-out",
                k if k.starts_with("flip") => "flip",
                _ => "fade",
            };
            let ms = |k: &str, d: f64| {
                attr(attrs, k)
                    .and_then(|v| v.trim().parse::<f64>().ok())
                    .unwrap_or(d)
                    / 1000.0
            };
            let dur = ms("data-aos-duration", 400.0);
            let delay = ms("data-aos-delay", 0.0);
            let ease = match attr(attrs, "data-aos-easing").unwrap_or("") {
                "linear" => "linear",
                e if e.contains("back") => "bounce",
                _ => "smooth",
            };
            let v = appear_value(preset, dur, delay, ease, Some(100.0));
            attrs.retain(|a| !a.name.starts_with("data-aos"));
            set_attr(attrs, "data-revel-appear", v);
            count += 1;
        }
        true
    });
    count
}

// -------------------------------------------------------------------------------------------------
// Webflow

/// Données des interactions Webflow (`Webflow.require('ix2').init({…})`) lues dans le script du
/// site.
pub fn ix2_config(js: &str) -> Option<Value> {
    // `Webflow.require('ix2').init({"events":…` ou `.init(\n{"events":…`.
    let mut from = 0;
    let start = loop {
        let k = from + js[from..].find(".init(")?;
        let after = k + ".init(".len();
        let rest = js[after..].trim_start();
        if rest.starts_with("{\"events\"")
            && js.as_bytes()[k.saturating_sub(40)..k]
                .windows(3)
                .any(|w| w == b"ix2")
        {
            break after + (js[after..].len() - rest.len());
        }
        from = after;
    };
    let b = js.as_bytes();
    let (mut depth, mut k, mut in_str) = (0i32, start, false);
    while k < b.len() {
        let c = b[k];
        if in_str {
            if c == b'\\' {
                k += 1;
            } else if c == b'"' {
                in_str = false;
            }
        } else if c == b'"' {
            in_str = true;
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
            if depth == 0 {
                return serde_json::from_str(&js[start..=k]).ok();
            }
        }
        k += 1;
    }
    None
}

/// Animation d'apparition d'une interaction Webflow : (élément visé par `data-w-id`, valeur
/// `data-revel-appear`). Interactions sur une classe, au défilement continu, au survol : gardées
/// pour Webflow (`webflow.js`).
fn ix2_appears(cfg: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(events) = cfg.get("events").and_then(Value::as_object) else {
        return out;
    };
    let lists = cfg.get("actionLists").and_then(Value::as_object);
    let ms = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0) / 1000.0;
    let ease = |v: &Value| match v.get("easing").and_then(Value::as_str).unwrap_or("") {
        "linear" => "linear",
        "outBack" | "inOutBack" | "outElastic" | "outBounce" => "bounce",
        _ => "smooth",
    };
    for e in events.values() {
        let kind = e.get("eventTypeId").and_then(Value::as_str).unwrap_or("");
        let on_load = kind == "PAGE_START" || kind == "PAGE_FINISH";
        if kind != "SCROLL_INTO_VIEW" && !on_load {
            continue;
        }
        if e["target"]["appliesTo"]
            .as_str()
            .is_some_and(|a| a != "ELEMENT")
        {
            continue;
        }
        let Some(target) = e["target"]["id"].as_str() else {
            continue;
        };
        let id = target.rsplit('|').next().unwrap_or(target).to_string();
        let action = &e["action"];
        let cfgs = &action["config"];
        // Effets prêts à l'emploi : réglages dans l'interaction (délai, sens, durée).
        let ev = &e["config"];
        let pick = |k: &str| {
            if ev.get(k).is_some_and(|v| v.is_number()) {
                ms(ev, k)
            } else {
                ms(cfgs, k)
            }
        };
        let preset_dur = {
            let d = pick("duration");
            if d > 0.0 { d } else { 1.0 }
        };
        let preset = |p: &str| {
            let e2 = if ev.get("easing").is_some() {
                ease(ev)
            } else {
                ease(cfgs)
            };
            Some(appear_value(p, preset_dur, pick("delay"), e2, None))
        };
        let mut value = match action["actionTypeId"].as_str().unwrap_or("") {
            "FADE_EFFECT" => preset("fade"),
            "SLIDE_EFFECT" => {
                let dir = ev["direction"]
                    .as_str()
                    .or_else(|| cfgs["direction"].as_str())
                    .unwrap_or("BOTTOM");
                preset(match dir {
                    "TOP" => "down",
                    "LEFT" => "left",
                    "RIGHT" => "right",
                    _ => "up",
                })
            }
            "GROW_EFFECT" => preset("zoom-in"),
            "SHRINK_EFFECT" => preset("zoom-out"),
            "GROW_BIG_EFFECT" => preset("pop"),
            "FLIP_EFFECT" => preset("flip"),
            "SPIN_EFFECT" => preset("rotate"),
            _ => None,
        };
        // Animation sur mesure : premier groupe = état de départ, groupes suivants = arrivée.
        if value.is_none()
            && let Some(list) = cfgs["actionListId"].as_str().and_then(|l| lists?.get(l))
            && list["useFirstGroupAsInitialState"].as_bool() == Some(true)
            && let Some(groups) = list["actionItemGroups"].as_array()
            && groups.len() >= 2
        {
            let (mut x, mut y, mut scale, mut rot, mut fades) = (0.0, 0.0, 1.0, 0.0, false);
            let mut ok = true;
            let mine = |c: &Value| {
                c["target"]["useEventTarget"].as_bool() == Some(true)
                    || c["target"]["id"]
                        .as_str()
                        .is_some_and(|t| t.rsplit('|').next() == Some(id.as_str()))
            };
            for item in groups[0]["actionItems"].as_array().into_iter().flatten() {
                let c = &item["config"];
                if !mine(c) {
                    ok = false;
                }
                let unit_ok = |v: &str, u: &str| {
                    c[v].as_f64().unwrap_or(0.0) == 0.0
                        || c[u].as_str().is_none_or(|u| u.eq_ignore_ascii_case("px"))
                };
                match item["actionTypeId"].as_str().unwrap_or("") {
                    "STYLE_OPACITY" => fades = c["value"].as_f64().unwrap_or(1.0) < 0.5,
                    "TRANSFORM_MOVE" => {
                        x = c["xValue"].as_f64().unwrap_or(0.0);
                        y = c["yValue"].as_f64().unwrap_or(0.0);
                        if !unit_ok("xValue", "xUnit") || !unit_ok("yValue", "yUnit") {
                            ok = false;
                        }
                    }
                    "TRANSFORM_SCALE" => scale = c["xValue"].as_f64().unwrap_or(1.0),
                    "TRANSFORM_ROTATE" => rot = c["zValue"].as_f64().unwrap_or(0.0),
                    _ => ok = false,
                }
            }
            // Groupes intermédiaires : attente ; dernier groupe : arrivée.
            let mut delay: f64 = 0.0;
            let mut dur: f64 = 0.0;
            let mut end_ease = Value::Null;
            let last = groups.len() - 1;
            for (gi, g) in groups.iter().enumerate().skip(1) {
                let mut longest: f64 = 0.0;
                for item in g["actionItems"].as_array().into_iter().flatten() {
                    let c = &item["config"];
                    if !mine(c) {
                        ok = false;
                    }
                    let d = c["duration"].as_f64().unwrap_or(500.0) / 1000.0;
                    let dl = c["delay"].as_f64().unwrap_or(0.0) / 1000.0;
                    longest = longest.max(d + dl);
                    if gi == last {
                        dur = dur.max(d);
                        if end_ease.is_null()
                            && c.get("easing")
                                .and_then(Value::as_str)
                                .is_some_and(|e| !e.is_empty())
                        {
                            end_ease = c.clone();
                        }
                    }
                }
                if gi < last {
                    delay += longest;
                } else {
                    let first_delay = g["actionItems"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|i| i["config"]["delay"].as_f64().unwrap_or(0.0) / 1000.0)
                        .fold(f64::MAX, f64::min);
                    if first_delay < f64::MAX {
                        delay += first_delay;
                    }
                }
            }
            if ok && (fades || x != 0.0 || y != 0.0 || scale != 1.0 || rot != 0.0) {
                let (p, d) = preset_for(x, y, scale, rot);
                value = Some(appear_value(
                    p,
                    dur.max(0.1),
                    delay + ms(cfgs, "delay"),
                    ease(&end_ease),
                    d,
                ));
            }
        }
        if let Some(mut v) = value {
            if on_load {
                v.push_str(" trigger=load");
            }
            out.insert(id, v);
        }
    }
    out
}

/// Webflow : interactions d'apparition reprises (élément détaché de l'interaction de Webflow),
/// formulaires envoyés par Revel. Renvoie (animations reprises, formulaires repris).
pub fn webflow(body: &mut Vec<Node>, ix2: Option<&Value>) -> (usize, usize) {
    let appears = ix2.map(ix2_appears).unwrap_or_default();
    // Éléments visés par d'autres interactions (survol, clic) : gardés pour Webflow.
    let mut others: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(events) = ix2.and_then(|c| c.get("events")).and_then(Value::as_object) {
        for e in events.values() {
            let kind = e.get("eventTypeId").and_then(Value::as_str).unwrap_or("");
            if kind != "SCROLL_INTO_VIEW"
                && kind != "PAGE_START"
                && kind != "PAGE_FINISH"
                && let Some(t) = e["target"]["id"].as_str()
            {
                others.insert(t.rsplit('|').next().unwrap_or(t).to_string());
            }
        }
    }
    let (mut anim, mut forms) = (0, 0);
    retain_deep(body, &mut |n| {
        let Node::Element { tag, attrs, .. } = n else {
            return true;
        };
        if let Some(id) = attr(attrs, "data-w-id").map(str::to_string)
            && let Some(v) = appears.get(&id)
        {
            set_attr(attrs, "data-revel-appear", v.clone());
            clear_initial_state(attrs);
            if !others.contains(&id) {
                attrs.retain(|a| a.name != "data-w-id");
            }
            anim += 1;
        }
        // Bloc de formulaire de Webflow : envoyé par Revel (`revel-form.php`), plus par Webflow.
        if tag == "div"
            && let Some(class) = attr(attrs, "class")
            && class.split_whitespace().any(|c| c == "w-form")
        {
            let new: Vec<&str> = class
                .split_whitespace()
                .map(|c| if c == "w-form" { "revel-form-box" } else { c })
                .collect();
            set_attr(attrs, "class", new.join(" "));
        }
        if tag == "form" && attr(attrs, "data-revel-form").is_none() {
            let name = attr(attrs, "data-name")
                .or_else(|| attr(attrs, "name"))
                .unwrap_or("formulaire")
                .to_string();
            if attr(attrs, "action")
                .is_none_or(|a| a.is_empty() || a == "#" || a.contains("webflow"))
            {
                set_attr(attrs, "data-revel-form", revel_model::slugify(&name));
                attrs.retain(|a| a.name != "action" && a.name != "method");
                forms += 1;
            }
        }
        true
    });
    (anim, forms)
}

/// Règles ajoutées à la feuille du site pour les formulaires Webflow repris : marge du bloc,
/// message de réussite de Webflow montré après l'envoi.
pub const WEBFLOW_FORM_CSS: &str = ".revel-form-box{margin:0 0 15px}.revel-form-box form.is-sent{display:none}.revel-form-box form.is-sent~.w-form-done{display:block}";

#[cfg(test)]
mod tests {
    use super::*;

    fn el(tag: &str, attrs: &[(&str, &str)], children: Vec<Node>) -> Node {
        Node::Element {
            id: None,
            tag: tag.into(),
            attrs: attrs
                .iter()
                .map(|(n, v)| Attr {
                    name: (*n).into(),
                    value: Some((*v).into()),
                    when: None,
                })
                .collect(),
            children,
        }
    }

    fn attrs_of(n: &Node) -> &[Attr] {
        match n {
            Node::Element { attrs, .. } => attrs,
            _ => &[],
        }
    }

    #[test]
    fn framer_apparitions_et_moteur() {
        let json = r#"{"a1":{"default":{"initial":{"opacity":0.001,"x":0,"y":40,"scale":1},"animate":{"opacity":1,"x":0,"y":0,"transition":{"type":"spring","bounce":0.2,"delay":0.1,"duration":0.8}}}},"a2":{"default":{"initial":{"opacity":0.001,"scale":0.9},"animate":{"opacity":1,"transition":{"type":"tween","duration":0.5,"delay":0}}}}}"#;
        let mut head = vec![
            el(
                "script",
                &[
                    ("type", "framer/appear"),
                    ("id", "__framer__appearAnimationsContent"),
                ],
                vec![Node::Html { html: json.into() }],
            ),
            el(
                "script",
                &[
                    ("type", "module"),
                    ("data-framer-bundle", "main"),
                    (
                        "src",
                        "https://framerusercontent.com/sites/x/script_main.mjs",
                    ),
                ],
                vec![],
            ),
            el("link", &[("rel", "stylesheet"), ("href", "/a.css")], vec![]),
        ];
        let mut body = vec![
            el(
                "div",
                &[
                    ("data-framer-appear-id", "a1"),
                    (
                        "style",
                        "will-change:transform;opacity:0.001;transform:translateY(40px);color:red",
                    ),
                ],
                vec![],
            ),
            el(
                "div",
                &[("data-framer-appear-id", "a2"), ("style", "opacity:0.001")],
                vec![],
            ),
            el("div", &[("id", "__framer-badge-container")], vec![]),
            Node::Html {
                html: "<!--$-->".into(),
            },
        ];
        assert_eq!(framer(&mut head, &mut body), 2);
        assert_eq!(head.len(), 1, "seule la feuille de style reste");
        assert_eq!(body.len(), 2);
        assert_eq!(
            attr(attrs_of(&body[0]), "data-revel-appear"),
            Some("up dur=0.8 delay=0.1 dist=40")
        );
        assert_eq!(attr(attrs_of(&body[0]), "style"), Some("color:red"));
        assert_eq!(
            attr(attrs_of(&body[1]), "data-revel-appear"),
            Some("zoom-in dur=0.5 delay=0")
        );
        assert_eq!(attr(attrs_of(&body[1]), "style"), None);
        assert_eq!(attr(attrs_of(&body[1]), "data-framer-appear-id"), None);
    }

    #[test]
    fn framer_effet_de_texte() {
        let piece = |c: &str| {
            el(
                "span",
                &[(
                    "style",
                    "display:inline-block;opacity:0.001;filter:blur(10px);transform:translateX(0px) translateY(10px)",
                )],
                vec![Node::Text { text: c.into() }],
            )
        };
        let mut head = Vec::new();
        let mut body = vec![
            el(
                "div",
                &[("class", "framer-x")],
                vec![el(
                    "p",
                    &[("class", "framer-text")],
                    vec![el(
                        "span",
                        &[("style", "white-space:nowrap")],
                        vec![piece("+"), piece("S"), piece("F")],
                    )],
                )],
            ),
            el(
                "div",
                &[("style", "opacity:0.001;transform:translateY(20px)")],
                vec![Node::Text {
                    text: "Masqué".into(),
                }],
            ),
        ];
        assert_eq!(framer(&mut head, &mut body), 1);
        let html = crate::html::to_html(&body);
        assert_eq!(
            html,
            r#"<div class="framer-x"><p class="framer-text" data-revel-text="blur split=chars"><span style="white-space:nowrap">+SF</span></p></div><div>Masqué</div>"#
        );
    }

    #[test]
    fn aos_repris() {
        let mut body = vec![el(
            "section",
            &[
                ("data-aos", "fade-left"),
                ("data-aos-duration", "800"),
                ("data-aos-delay", "200"),
            ],
            vec![],
        )];
        assert_eq!(aos(&mut body), 1);
        assert_eq!(
            attr(attrs_of(&body[0]), "data-revel-appear"),
            Some("right dur=0.8 delay=0.2 dist=100")
        );
        assert_eq!(attr(attrs_of(&body[0]), "data-aos"), None);
    }

    #[test]
    fn webflow_interactions_et_formulaires() {
        let js = r#"Webflow.require("ix2").init({"events":{"e":{"id":"e","eventTypeId":"SCROLL_INTO_VIEW","action":{"actionTypeId":"SLIDE_EFFECT","config":{"actionListId":"slideInBottom","autoStopEventId":"e-2","duration":800,"delay":100,"easing":"ease","direction":"BOTTOM","effectIn":true}},"target":{"id":"page|w1","appliesTo":"ELEMENT"}},"e-3":{"id":"e-3","eventTypeId":"PAGE_START","action":{"actionTypeId":"GENERAL_START_ACTION","config":{"actionListId":"a","delay":0}},"target":{"id":"page|w2","appliesTo":"ELEMENT"}},"e-5":{"id":"e-5","eventTypeId":"MOUSE_OVER","action":{"actionTypeId":"GENERAL_START_ACTION","config":{"actionListId":"b"}},"target":{"id":"page|w2","appliesTo":"ELEMENT"}}},"actionLists":{"a":{"id":"a","useFirstGroupAsInitialState":true,"actionItemGroups":[{"actionItems":[{"actionTypeId":"STYLE_OPACITY","config":{"target":{"useEventTarget":true},"value":0}},{"actionTypeId":"TRANSFORM_MOVE","config":{"target":{"useEventTarget":true},"xValue":-50,"yValue":0,"xUnit":"PX","yUnit":"PX"}}]},{"actionItems":[{"actionTypeId":"STYLE_OPACITY","config":{"target":{"useEventTarget":true},"duration":600,"easing":"outBack","value":1}},{"actionTypeId":"TRANSFORM_MOVE","config":{"target":{"useEventTarget":true},"duration":600,"xValue":0}}]}]}},"site":{}});"#;
        let cfg = ix2_config(js).unwrap();
        // Format des sites publiés : `.init(` suivi d'un retour à la ligne ; effet prêt à
        // l'emploi réglé dans l'interaction ; animation des enfants gardée pour Webflow.
        let real = r#"/*!
 * Webflow: Interactions 2.0: Init
 */
Webflow.require('ix2').init(
{"events":{"e-79":{"id":"e-79","animationType":"preset","eventTypeId":"SCROLL_INTO_VIEW","action":{"actionTypeId":"SLIDE_EFFECT","instant":false,"config":{"actionListId":"slideInBottom","autoStopEventId":"e-80"}},"target":{"id":"p|w9","appliesTo":"ELEMENT"},"config":{"loop":false,"delay":100,"direction":"BOTTOM","effectIn":true}},"e":{"id":"e","eventTypeId":"SCROLL_INTO_VIEW","action":{"actionTypeId":"GENERAL_START_ACTION","config":{"delay":0,"actionListId":"a"}},"target":{"id":"p|w8","appliesTo":"ELEMENT"}}},"actionLists":{"a":{"id":"a","useFirstGroupAsInitialState":true,"actionItemGroups":[{"actionItems":[{"actionTypeId":"STYLE_OPACITY","config":{"target":{"useEventTarget":"CHILDREN","selector":".x"},"value":0}}]},{"actionItems":[{"actionTypeId":"STYLE_OPACITY","config":{"target":{"useEventTarget":"CHILDREN","selector":".x"},"duration":500,"value":1}}]}]}},"site":{}});"#;
        let m = ix2_appears(&ix2_config(real).expect("données lues"));
        assert_eq!(m.get("w9").map(String::as_str), Some("up dur=1 delay=0.1"));
        assert!(!m.contains_key("w8"), "enfants : gardé pour Webflow");
        let mut body = vec![
            el(
                "div",
                &[
                    ("data-w-id", "w1"),
                    (
                        "style",
                        "opacity:0;-webkit-transform:translate3d(0, 100px, 0)",
                    ),
                ],
                vec![],
            ),
            el("h1", &[("data-w-id", "w2"), ("style", "opacity:0")], vec![]),
            el(
                "div",
                &[("class", "w-form")],
                vec![el(
                    "form",
                    &[("data-name", "Contact Form"), ("method", "get")],
                    vec![],
                )],
            ),
        ];
        assert_eq!(webflow(&mut body, Some(&cfg)), (2, 1));
        assert_eq!(
            attr(attrs_of(&body[0]), "data-revel-appear"),
            Some("up dur=0.8 delay=0.1")
        );
        assert_eq!(
            attr(attrs_of(&body[0]), "data-w-id"),
            None,
            "plus d'interaction Webflow"
        );
        assert_eq!(attr(attrs_of(&body[0]), "style"), None);
        assert_eq!(
            attr(attrs_of(&body[1]), "data-revel-appear"),
            Some("left dur=0.6 delay=0 ease=bounce dist=50 trigger=load")
        );
        assert_eq!(
            attr(attrs_of(&body[1]), "data-w-id"),
            Some("w2"),
            "survol gardé pour Webflow"
        );
        assert_eq!(attr(attrs_of(&body[2]), "class"), Some("revel-form-box"));
        let Node::Element { children, .. } = &body[2] else {
            panic!()
        };
        assert_eq!(
            attr(attrs_of(&children[0]), "data-revel-form"),
            Some("contact-form")
        );
        assert_eq!(attr(attrs_of(&children[0]), "method"), None);
    }
}
