//! Import d'un fichier Figma (7.5) par l'API de Figma (jeton personnel, droit
//! `file_content:read`) : chaque cadre de premier niveau choisi devient une page.
//!
//! - **mise en page automatique** → flex (sens, espacement, marges intérieures, alignements,
//!   retour à la ligne) ; largeur « remplir » → étirée, « fixe » → largeur (au plus celle de
//!   l'écran) ; cadres sans mise en page automatique : positions gardées (absolues) ;
//! - **textes** → titres (`h1`…`h3`) ou paragraphes d'après la taille, styles de caractères
//!   (gras, italique, couleur), liens ;
//! - **remplissages, contours, coins, ombres, flous, opacité** → règles de la feuille du site
//!   (une classe par élément, réutilisée par les éléments identiques) ;
//! - **images** (remplissages d'image) téléchargées ; **formes et icônes** (vecteurs) exportées
//!   en SVG ;
//! - **composants** utilisés plusieurs fois avec la même structure → composants Revel, textes en
//!   propriétés.
//!
//! Polices : familles et graisses utilisées rendues à l'appelant (ajoutées depuis Google Fonts).

use crate::Report;
use crate::crawl::{Fetch, get_json};
use revel_model::{
    Attr, Component, Decl, Layout, Node, Page, PropDef, PropKind, Site, StyleItem, StyleRule,
    Stylesheet,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use url::Url;

const API: &str = "https://api.figma.com/v1";

#[derive(Debug, thiserror::Error)]
pub enum FigmaError {
    #[error("adresse de fichier Figma non reconnue")]
    BadUrl,
    #[error("jeton Figma refusé")]
    Token,
    #[error("fichier Figma introuvable ou inaccessible avec ce jeton")]
    NotFound,
    #[error("API de Figma : {0}")]
    Api(String),
    #[error("import annulé")]
    Cancelled,
    #[error("{0}")]
    Io(String),
}

/// Clé du fichier dans une adresse Figma (`figma.com/design/<clé>/…`, `/file/`, `/proto/`) ou
/// clé seule.
pub fn file_key(input: &str) -> Option<String> {
    let t = input.trim();
    let ok = |k: &str| k.len() >= 10 && k.chars().all(|c| c.is_ascii_alphanumeric());
    if ok(t) {
        return Some(t.to_string());
    }
    let u = Url::parse(t)
        .or_else(|_| Url::parse(&format!("https://{t}")))
        .ok()?;
    if !u.host_str()?.ends_with("figma.com") {
        return None;
    }
    let segs: Vec<&str> = u.path_segments()?.collect();
    let i = segs
        .iter()
        .position(|s| matches!(*s, "design" | "file" | "proto" | "board"))?;
    // Branche : `/design/<clé>/branch/<clé de branche>/…`.
    if segs.get(i + 2) == Some(&"branch") {
        return segs.get(i + 3).filter(|k| ok(k)).map(|k| k.to_string());
    }
    segs.get(i + 1).filter(|k| ok(k)).map(|k| k.to_string())
}

fn api_error((status, msg): (u16, String)) -> FigmaError {
    match status {
        401 | 403 => FigmaError::Token,
        404 => FigmaError::NotFound,
        _ => FigmaError::Api(format!("{status} {msg}")),
    }
}

/// Fichier Figma complet (`GET /v1/files/:clé`).
pub fn fetch_file(fetch: &dyn Fetch, key: &str, token: &str) -> Result<Value, FigmaError> {
    get_json(
        fetch,
        &format!("{API}/files/{key}"),
        &[("X-Figma-Token", token.trim())],
    )
    .map_err(api_error)
}

/// Cadre de premier niveau d'une page du fichier.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameInfo {
    pub id: String,
    pub name: String,
    pub width: f64,
    pub height: f64,
}

/// Pages du fichier et leurs cadres de premier niveau (cadres, composants, sections).
pub fn outline(file: &Value) -> Vec<(String, String, Vec<FrameInfo>)> {
    let mut out = Vec::new();
    for c in file["document"]["children"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let frames = c["children"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|n| {
                matches!(n["type"].as_str(), Some("FRAME" | "COMPONENT" | "SECTION"))
                    && n["visible"].as_bool() != Some(false)
            })
            .map(|n| FrameInfo {
                id: n["id"].as_str().unwrap_or("").to_string(),
                name: n["name"].as_str().unwrap_or("").to_string(),
                width: n["absoluteBoundingBox"]["width"].as_f64().unwrap_or(0.0),
                height: n["absoluteBoundingBox"]["height"].as_f64().unwrap_or(0.0),
            })
            .collect();
        out.push((
            c["id"].as_str().unwrap_or("").to_string(),
            c["name"].as_str().unwrap_or("").to_string(),
            frames,
        ));
    }
    out
}

/// Réglages de l'import.
#[derive(Default)]
pub struct Options<'a> {
    /// Cadres à importer (identifiants), dans l'ordre : le premier devient l'accueil.
    pub frames: Vec<String>,
    pub lang: String,
    pub cancel: Option<&'a AtomicBool>,
}

/// Résultat de l'import.
pub struct Imported {
    pub site: Site,
    pub report: Report,
    /// Polices utilisées : famille → variantes Google Fonts (`400`, `700italic`).
    pub fonts: BTreeMap<String, BTreeSet<String>>,
}

// -------------------------------------------------------------------------------------------------
// Conversion

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}

fn px(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r == 0.0 {
        "0".into()
    } else if r.fract() == 0.0 {
        format!("{r:.0}px")
    } else {
        format!("{r}px")
    }
}

fn color(c: &Value, opacity: f64) -> String {
    let ch = |k: &str| (num(&c[k]) * 255.0).round().clamp(0.0, 255.0) as u8;
    let a = c["a"].as_f64().unwrap_or(1.0) * opacity;
    if a >= 0.999 {
        format!("#{:02x}{:02x}{:02x}", ch("r"), ch("g"), ch("b"))
    } else {
        format!(
            "rgba({}, {}, {}, {})",
            ch("r"),
            ch("g"),
            ch("b"),
            (a * 1000.0).round() / 1000.0
        )
    }
}

fn visible(p: &Value) -> bool {
    p["visible"].as_bool() != Some(false)
}

/// Remplissage CSS d'une couche de remplissage (couleur ou dégradé linéaire/radial).
fn paint_css(p: &Value) -> Option<String> {
    let op = p["opacity"].as_f64().unwrap_or(1.0);
    match p["type"].as_str()? {
        "SOLID" => Some(color(&p["color"], op)),
        t @ ("GRADIENT_LINEAR" | "GRADIENT_RADIAL") => {
            let stops: Vec<String> = p["gradientStops"]
                .as_array()?
                .iter()
                .map(|s| {
                    format!(
                        "{} {}%",
                        color(&s["color"], op),
                        (num(&s["position"]) * 100.0).round()
                    )
                })
                .collect();
            if t == "GRADIENT_RADIAL" {
                return Some(format!("radial-gradient({})", stops.join(", ")));
            }
            let h = p["gradientHandlePositions"].as_array()?;
            let (a, b) = (h.first()?, h.get(1)?);
            let (dx, dy) = (num(&b["x"]) - num(&a["x"]), num(&b["y"]) - num(&a["y"]));
            let angle = (dy.atan2(dx).to_degrees() + 90.0).round();
            Some(format!("linear-gradient({angle}deg, {})", stops.join(", ")))
        }
        _ => None,
    }
}

/// Le nœud ne contient que des formes (icône, illustration vectorielle) : exporté en SVG.
fn vector_only(n: &Value) -> bool {
    match n["type"].as_str().unwrap_or("") {
        "VECTOR" | "BOOLEAN_OPERATION" | "STAR" | "LINE" | "REGULAR_POLYGON" => true,
        "ELLIPSE" | "RECTANGLE" => !has_image(n),
        "GROUP" | "FRAME" | "COMPONENT" | "INSTANCE" => {
            let kids: Vec<&Value> = n["children"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|c| visible(c))
                .collect();
            !kids.is_empty()
                && kids.iter().all(|c| vector_only(c))
                && kids.iter().any(|c| {
                    matches!(
                        c["type"].as_str(),
                        Some("VECTOR" | "BOOLEAN_OPERATION" | "STAR" | "LINE" | "REGULAR_POLYGON")
                    ) || c["children"].is_array()
                })
                && n["layoutMode"].as_str().is_none_or(|m| m == "NONE")
        }
        _ => false,
    }
}

fn has_image(n: &Value) -> bool {
    n["fills"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|f| f["type"] == "IMAGE" && visible(f))
}

/// Nom de couche produit par Figma (`Frame 12`, `Rectangle 3`) : sans intérêt pour une classe.
fn generic_name(name: &str) -> bool {
    let mut parts = name.rsplitn(2, ' ');
    let last = parts.next().unwrap_or("");
    let first = parts.next().unwrap_or(name);
    let generic = [
        "Frame",
        "Group",
        "Rectangle",
        "Ellipse",
        "Vector",
        "Text",
        "Line",
        "Polygon",
        "Star",
        "Union",
        "Subtract",
        "Intersect",
        "Exclude",
        "Image",
        "Section",
        "Container",
    ];
    generic.contains(&first) && (last.chars().all(|c| c.is_ascii_digit()) || first == name)
}

struct Cx {
    fr: bool,
    /// Classe → déclarations (ordre d'apparition).
    rules: Vec<(String, Vec<(String, String)>)>,
    by_decls: HashMap<(String, String), String>,
    used: HashMap<String, usize>,
    /// Références d'image à télécharger.
    images: BTreeSet<String>,
    /// Nœuds à exporter en SVG : identifiant → nom de fichier.
    svgs: BTreeMap<String, String>,
    fonts: BTreeMap<String, BTreeSet<String>>,
    /// Utilisations de composants : identifiant du composant → (nœud produit, textes).
    instances: Vec<(String, String)>,
    absolute: usize,
    h1: bool,
    max_font: f64,
    /// Styles de couleur du fichier (identifiant → nom), repris en couleurs du site.
    styles: HashMap<String, String>,
    /// Couleurs du site : variable CSS → valeur, dans l'ordre d'apparition.
    colors: Vec<(String, String)>,
}

impl Cx {
    /// Couleur d'un style de couleur de Figma (`styles.fill` du nœud) : variable du site
    /// (`var(--primaire-500)`), créée à la première utilisation ; sinon la couleur telle quelle.
    fn paint(&mut self, n: &Value, key: &str, value: String) -> String {
        let Some(name) = n["styles"][key]
            .as_str()
            .and_then(|id| self.styles.get(id))
            .cloned()
        else {
            return value;
        };
        let mut var = format!("--{}", revel_model::slugify(&name.replace('/', " ")));
        if var == "--" {
            return value;
        }
        match self.colors.iter().find(|(v, _)| *v == var) {
            Some((_, val)) if *val == value => {}
            Some(_) => {
                // Même nom, autre valeur (opacité de la couche) : couleur telle quelle.
                return value;
            }
            None => {
                if self
                    .colors
                    .iter()
                    .any(|(v, _)| v.eq_ignore_ascii_case(&var))
                {
                    var.push_str("-2");
                }
                self.colors.push((var.clone(), value));
            }
        }
        format!("var({var})")
    }

    fn word(&self, role: &str) -> &'static str {
        match (role, self.fr) {
            ("text", true) => "texte",
            ("text", false) => "text",
            ("title", true) => "titre",
            ("title", false) => "title",
            ("image", _) => "image",
            ("icon", true) => "icone",
            ("icon", false) => "icon",
            ("button", true) => "bouton",
            ("button", false) => "button",
            ("section", _) => "section",
            (_, true) => "bloc",
            (_, false) => "block",
        }
    }

    /// Classe pour des déclarations : réutilisée si un élément du même nom a les mêmes styles.
    fn class(&mut self, name: &str, role: &str, decls: Vec<(String, String)>) -> String {
        let base = if generic_name(name) || name.trim().is_empty() {
            self.word(role).to_string()
        } else {
            let s = revel_model::slugify(name);
            let s: String = s.chars().take(28).collect();
            let s = s.trim_end_matches('-').to_string();
            if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
                format!("{}-{s}", self.word(role))
                    .trim_end_matches('-')
                    .to_string()
            } else {
                s
            }
        };
        let sig = decls
            .iter()
            .map(|(p, v)| format!("{p}:{v}"))
            .collect::<Vec<_>>()
            .join(";");
        if let Some(c) = self.by_decls.get(&(base.clone(), sig.clone())) {
            return c.clone();
        }
        let n = self.used.entry(base.clone()).or_insert(0);
        *n += 1;
        let class = if *n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        self.by_decls.insert((base, sig), class.clone());
        self.rules.push((class.clone(), decls));
        class
    }
}

fn el(tag: &str, class: Option<String>, mut attrs: Vec<Attr>, children: Vec<Node>) -> Node {
    if let Some(c) = class {
        attrs.insert(
            0,
            Attr {
                name: "class".into(),
                value: Some(c),
                when: None,
            },
        );
    }
    Node::Element {
        id: None,
        tag: tag.into(),
        attrs,
        children,
    }
}

fn attr(name: &str, value: &str) -> Attr {
    Attr {
        name: name.into(),
        value: Some(value.into()),
        when: None,
    }
}

/// Parent du nœud converti : sa mise en page et sa boîte.
#[derive(Clone, Copy)]
struct Parent {
    /// `HORIZONTAL`, `VERTICAL` ou rien (positions).
    row: Option<bool>,
    x: f64,
    y: f64,
    /// Largeur intérieure (sans les marges intérieures).
    inner_w: f64,
}

fn box_of(n: &Value) -> (f64, f64, f64, f64) {
    let b = &n["absoluteBoundingBox"];
    (
        num(&b["x"]),
        num(&b["y"]),
        num(&b["width"]),
        num(&b["height"]),
    )
}

/// Déclarations communes : remplissage, contour, coins, ombres, opacité, taille et position.
fn base_decls(n: &Value, parent: Option<Parent>, cx: &mut Cx, text: bool) -> Vec<(String, String)> {
    let mut d: Vec<(String, String)> = Vec::new();
    let (x, y, w, h) = box_of(n);
    let push = |d: &mut Vec<(String, String)>, p: &str, v: String| d.push((p.into(), v));
    // Taille et position.
    let auto_child = parent.is_some_and(|p| p.row.is_some())
        && n["layoutPositioning"].as_str() != Some("ABSOLUTE");
    if let Some(p) = parent {
        if !auto_child {
            if p.row.is_some()
                || n["layoutPositioning"].as_str() == Some("ABSOLUTE")
                || p.row.is_none()
            {
                push(&mut d, "position", "absolute".into());
                push(&mut d, "left", px(x - p.x));
                push(&mut d, "top", px(y - p.y));
                cx.absolute += 1;
            }
            if !text || n["style"]["textAutoResize"].as_str() != Some("WIDTH_AND_HEIGHT") {
                push(&mut d, "width", px(w));
            }
            if !text {
                push(&mut d, "height", px(h));
            }
        } else {
            let row = p.row == Some(true);
            // Fichiers plus anciens : `layoutGrow` (sens du parent), `layoutAlign` (sens croisé).
            let grow = n["layoutGrow"].as_f64() == Some(1.0);
            let stretch = n["layoutAlign"].as_str() == Some("STRETCH");
            let hz = n["layoutSizingHorizontal"].as_str().unwrap_or(
                if (grow && row) || (stretch && !row) {
                    "FILL"
                } else {
                    "FIXED"
                },
            );
            let vt = n["layoutSizingVertical"].as_str().unwrap_or(
                if (grow && !row) || (stretch && row) {
                    "FILL"
                } else {
                    "FIXED"
                },
            );
            let has_kids = n["children"].as_array().is_some_and(|c| !c.is_empty());
            // Largeur fixe égale à celle du parent : étirée (mise en page souple).
            let full = (w - p.inner_w).abs() < 1.5 && !row;
            match hz {
                "FILL" if row => push(&mut d, "flex", "1 1 0".into()),
                "FILL" => push(&mut d, "align-self", "stretch".into()),
                "FIXED" if full => push(&mut d, "align-self", "stretch".into()),
                "FIXED" => {
                    push(&mut d, "width", px(w));
                    push(&mut d, "max-width", "100%".into());
                    if row {
                        push(&mut d, "flex-shrink", "0".into());
                    }
                }
                _ => {}
            }
            match vt {
                "FILL" if !row => push(&mut d, "flex", "1 1 0".into()),
                "FILL" => push(&mut d, "align-self", "stretch".into()),
                "FIXED" if !text => push(
                    &mut d,
                    if has_kids { "min-height" } else { "height" },
                    px(h),
                ),
                _ => {}
            }
        }
    }
    // Remplissage (texte : couleur du texte).
    if !text {
        let fills: Vec<&Value> = n["fills"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|f| visible(f) && f["type"] != "IMAGE")
            .collect();
        let layers: Vec<String> = fills.iter().rev().filter_map(|f| paint_css(f)).collect();
        match layers.len() {
            0 => {}
            1 if !layers[0].contains("gradient") => {
                let c = cx.paint(n, "fill", layers[0].clone());
                push(&mut d, "background-color", c)
            }
            _ => push(
                &mut d,
                "background",
                layers
                    .iter()
                    .enumerate()
                    .map(|(i, l)| {
                        if l.contains("gradient") || i == layers.len() - 1 {
                            l.clone()
                        } else {
                            format!("linear-gradient({l}, {l})")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        }
        // Contour.
        if let Some(s) = n["strokes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|s| visible(s))
            .find_map(paint_css)
        {
            let wgt = n["strokeWeight"].as_f64().unwrap_or(1.0);
            let iw = &n["individualStrokeWeights"];
            if iw.is_object() {
                for (side, k) in [
                    ("top", "top"),
                    ("right", "right"),
                    ("bottom", "bottom"),
                    ("left", "left"),
                ] {
                    let v = num(&iw[k]);
                    if v > 0.0 {
                        push(
                            &mut d,
                            &format!("border-{side}"),
                            format!("{} solid {s}", px(v)),
                        );
                    }
                }
            } else if wgt > 0.0 {
                push(&mut d, "border", format!("{} solid {s}", px(wgt)));
            }
        }
        // Coins.
        if let Some(r) = n["rectangleCornerRadii"]
            .as_array()
            .filter(|r| r.len() == 4)
        {
            let v: Vec<String> = r.iter().map(|x| px(num(x))).collect();
            if v.iter().any(|x| x != "0") {
                push(&mut d, "border-radius", v.join(" "));
            }
        } else if let Some(r) = n["cornerRadius"].as_f64().filter(|r| *r > 0.0) {
            push(&mut d, "border-radius", px(r));
        }
        if n["type"] == "ELLIPSE" {
            push(&mut d, "border-radius", "50%".into());
        }
        if n["clipsContent"].as_bool() == Some(true) {
            push(&mut d, "overflow", "hidden".into());
        }
    }
    // Effets.
    let mut shadows = Vec::new();
    for e in n["effects"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| visible(e))
    {
        match e["type"].as_str().unwrap_or("") {
            t @ ("DROP_SHADOW" | "INNER_SHADOW") => {
                let s = format!(
                    "{}{} {} {} {} {}",
                    if t == "INNER_SHADOW" { "inset " } else { "" },
                    px(num(&e["offset"]["x"])),
                    px(num(&e["offset"]["y"])),
                    px(num(&e["radius"])),
                    px(num(&e["spread"])),
                    color(&e["color"], 1.0)
                );
                shadows.push(s);
            }
            "LAYER_BLUR" => push(
                &mut d,
                "filter",
                format!("blur({})", px(num(&e["radius"]) / 2.0)),
            ),
            "BACKGROUND_BLUR" => push(
                &mut d,
                "backdrop-filter",
                format!("blur({})", px(num(&e["radius"]) / 2.0)),
            ),
            _ => {}
        }
    }
    if !shadows.is_empty() {
        push(
            &mut d,
            if text { "text-shadow" } else { "box-shadow" },
            if text {
                shadows
                    .iter()
                    .map(|s| {
                        // text-shadow : pas d'étalement.
                        let p: Vec<&str> = s.split(' ').collect();
                        if p.len() >= 5 {
                            format!("{} {} {} {}", p[0], p[1], p[2], p[4..].join(" "))
                        } else {
                            s.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                shadows.join(", ")
            },
        );
    }
    if let Some(o) = n["opacity"].as_f64().filter(|o| *o < 0.999) {
        push(
            &mut d,
            "opacity",
            format!("{}", (o * 100.0).round() / 100.0),
        );
    }
    d
}

/// Mise en page automatique du cadre.
fn layout_decls(n: &Value) -> Vec<(String, String)> {
    let mut d: Vec<(String, String)> = Vec::new();
    let mode = n["layoutMode"].as_str().unwrap_or("NONE");
    if mode == "NONE" {
        if n["children"].as_array().is_some_and(|c| !c.is_empty()) {
            d.push(("position".into(), "relative".into()));
        }
        return d;
    }
    d.push(("display".into(), "flex".into()));
    if mode == "VERTICAL" {
        d.push(("flex-direction".into(), "column".into()));
    }
    if n["layoutWrap"].as_str() == Some("WRAP") {
        d.push(("flex-wrap".into(), "wrap".into()));
    }
    let gap = num(&n["itemSpacing"]);
    let primary = n["primaryAxisAlignItems"].as_str().unwrap_or("MIN");
    if gap > 0.0 && primary != "SPACE_BETWEEN" {
        let cross = n["counterAxisSpacing"].as_f64().filter(|c| *c > 0.0);
        match cross {
            Some(c) if n["layoutWrap"].as_str() == Some("WRAP") => {
                d.push(("gap".into(), format!("{} {}", px(c), px(gap))));
            }
            _ => d.push(("gap".into(), px(gap))),
        }
    }
    let pad: Vec<f64> = ["paddingTop", "paddingRight", "paddingBottom", "paddingLeft"]
        .iter()
        .map(|k| num(&n[*k]))
        .collect();
    if pad.iter().any(|p| *p > 0.0) {
        let v: Vec<String> = pad.iter().map(|p| px(*p)).collect();
        let s = if v[0] == v[2] && v[1] == v[3] {
            if v[0] == v[1] {
                v[0].clone()
            } else {
                format!("{} {}", v[0], v[1])
            }
        } else {
            v.join(" ")
        };
        d.push(("padding".into(), s));
    }
    let jc = match primary {
        "CENTER" => Some("center"),
        "MAX" => Some("flex-end"),
        "SPACE_BETWEEN" => Some("space-between"),
        _ => None,
    };
    if let Some(j) = jc {
        d.push(("justify-content".into(), j.into()));
    }
    let ai = match n["counterAxisAlignItems"].as_str().unwrap_or("MIN") {
        "CENTER" => "center",
        "MAX" => "flex-end",
        "BASELINE" => "baseline",
        _ => "flex-start",
    };
    d.push(("align-items".into(), ai.into()));
    d
}

/// Styles de texte (Figma `TypeStyle`).
fn text_decls(n: &Value, cx: &mut Cx) -> Vec<(String, String)> {
    let s = &n["style"];
    let fills = &n["fills"];
    let mut d: Vec<(String, String)> = Vec::new();
    if let Some(f) = s["fontFamily"].as_str() {
        let w = s["fontWeight"].as_f64().unwrap_or(400.0).round() as u32;
        let italic = s["italic"].as_bool() == Some(true);
        cx.fonts
            .entry(f.to_string())
            .or_default()
            .insert(format!("{w}{}", if italic { "italic" } else { "" }));
        d.push(("font-family".into(), format!("\"{f}\", sans-serif")));
    }
    if let Some(w) = s["fontWeight"].as_f64() {
        d.push(("font-weight".into(), format!("{}", w.round())));
    }
    if let Some(sz) = s["fontSize"].as_f64() {
        d.push(("font-size".into(), px(sz)));
    }
    match s["lineHeightUnit"].as_str() {
        Some("PIXELS") => {
            if let Some(l) = s["lineHeightPx"].as_f64() {
                d.push(("line-height".into(), px(l)));
            }
        }
        Some("FONT_SIZE_%") => {
            if let Some(p) = s["lineHeightPercentFontSize"].as_f64() {
                d.push((
                    "line-height".into(),
                    format!("{}", (p / 100.0 * 1000.0).round() / 1000.0),
                ));
            }
        }
        _ => {}
    }
    if let Some(l) = s["letterSpacing"].as_f64().filter(|l| l.abs() > 0.001) {
        d.push(("letter-spacing".into(), px(l)));
    }
    match s["textAlignHorizontal"].as_str() {
        Some("CENTER") => d.push(("text-align".into(), "center".into())),
        Some("RIGHT") => d.push(("text-align".into(), "right".into())),
        Some("JUSTIFIED") => d.push(("text-align".into(), "justify".into())),
        _ => {}
    }
    match s["textCase"].as_str() {
        Some("UPPER") => d.push(("text-transform".into(), "uppercase".into())),
        Some("LOWER") => d.push(("text-transform".into(), "lowercase".into())),
        Some("TITLE") => d.push(("text-transform".into(), "capitalize".into())),
        _ => {}
    }
    match s["textDecoration"].as_str() {
        Some("UNDERLINE") => d.push(("text-decoration".into(), "underline".into())),
        Some("STRIKETHROUGH") => d.push(("text-decoration".into(), "line-through".into())),
        _ => {}
    }
    if s["italic"].as_bool() == Some(true) {
        d.push(("font-style".into(), "italic".into()));
    }
    if let Some(c) = fills
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| visible(f))
        .find_map(|f| {
            (f["type"] == "SOLID").then(|| color(&f["color"], f["opacity"].as_f64().unwrap_or(1.0)))
        })
    {
        let c = cx.paint(n, "fill", c);
        d.push(("color".into(), c));
    }
    d
}

/// Texte avec les styles de caractères : morceaux `<strong>`, `<em>`, `<span style>`, liens.
fn text_children(n: &Value) -> Vec<Node> {
    let chars: Vec<char> = n["characters"].as_str().unwrap_or("").chars().collect();
    let overrides: Vec<u64> = n["characterStyleOverrides"]
        .as_array()
        .map(|a| a.iter().map(|v| v.as_u64().unwrap_or(0)).collect())
        .unwrap_or_default();
    let table = &n["styleOverrideTable"];
    let base = &n["style"];
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let id = overrides.get(i).copied().unwrap_or(0);
        let mut j = i;
        while j < chars.len() && overrides.get(j).copied().unwrap_or(0) == id {
            j += 1;
        }
        let s: String = chars[i..j].iter().collect();
        let mut nodes = Vec::new();
        for (k, line) in s.split('\n').enumerate() {
            if k > 0 {
                nodes.push(el("br", None, Vec::new(), Vec::new()));
            }
            if !line.is_empty() {
                nodes.push(Node::Text { text: line.into() });
            }
        }
        let st = &table[id.to_string()];
        let link = st["hyperlink"]["url"].as_str().or_else(|| {
            (id == 0)
                .then(|| base["hyperlink"]["url"].as_str())
                .flatten()
        });
        let mut wrapped = nodes;
        if id != 0 && st.is_object() {
            let bw = base["fontWeight"].as_f64().unwrap_or(400.0);
            if st["fontWeight"]
                .as_f64()
                .is_some_and(|w| w >= 600.0 && bw < 600.0)
            {
                wrapped = vec![el("strong", None, Vec::new(), wrapped)];
            }
            if st["italic"].as_bool() == Some(true) && base["italic"].as_bool() != Some(true) {
                wrapped = vec![el("em", None, Vec::new(), wrapped)];
            }
            let mut style = Vec::new();
            if let Some(c) = st["fills"].as_array().into_iter().flatten().find_map(|f| {
                (f["type"] == "SOLID")
                    .then(|| color(&f["color"], f["opacity"].as_f64().unwrap_or(1.0)))
            }) {
                style.push(format!("color: {c}"));
            }
            if st["textDecoration"].as_str() == Some("UNDERLINE") {
                style.push("text-decoration: underline".into());
            }
            if !style.is_empty() && link.is_none() {
                wrapped = vec![el(
                    "span",
                    None,
                    vec![attr("style", &style.join("; "))],
                    wrapped,
                )];
            }
        }
        if let Some(u) = link {
            wrapped = vec![el("a", None, vec![attr("href", u)], wrapped)];
        }
        out.extend(wrapped);
        i = j;
    }
    out
}

/// Convertit un nœud Figma (et ses enfants).
fn convert(n: &Value, parent: Option<Parent>, cx: &mut Cx) -> Option<Node> {
    if !visible(n) || n["isMask"].as_bool() == Some(true) {
        return None;
    }
    let ty = n["type"].as_str().unwrap_or("");
    let name = n["name"].as_str().unwrap_or("");
    let id = n["id"].as_str().unwrap_or("");
    match ty {
        "SLICE" | "CONNECTOR" | "STICKY" | "WASHI_TAPE" | "COMMENT" => return None,
        _ => {}
    }
    if ty == "TEXT" {
        let size = n["style"]["fontSize"].as_f64().unwrap_or(16.0);
        let (tag, role) = if size >= 40.0 && size >= cx.max_font - 0.5 && !cx.h1 {
            cx.h1 = true;
            ("h1", "title")
        } else if size >= 32.0 {
            ("h2", "title")
        } else if size >= 22.0 {
            ("h3", "title")
        } else {
            ("p", "text")
        };
        let mut d = text_decls(n, cx);
        d.splice(0..0, [("margin".to_string(), "0".to_string())]);
        d.extend(base_decls(n, parent, cx, true));
        // Calque nommé d'après son texte (nom donné par Figma) : classe d'après son rôle.
        let chars = n["characters"].as_str().unwrap_or("").trim();
        let own = if !name.trim().is_empty() && !chars.starts_with(name.trim()) {
            name
        } else {
            ""
        };
        let class = cx.class(own, role, d);
        return Some(el(tag, Some(class), Vec::new(), text_children(n)));
    }
    if vector_only(n) {
        let file = format!(
            "images/figma/{}-{}.svg",
            {
                let s = revel_model::slugify(name);
                if s.is_empty() || generic_name(name) {
                    "forme".to_string()
                } else {
                    s.chars().take(32).collect()
                }
            },
            revel_model::slugify(id)
        );
        cx.svgs.insert(id.to_string(), file.clone());
        let (_, _, w, h) = box_of(n);
        let mut d = base_decls(n, parent, cx, true);
        d.retain(|(p, _)| !matches!(p.as_str(), "opacity" | "text-shadow"));
        d.push(("display".into(), "block".into()));
        let class = cx.class(name, "icon", d);
        return Some(el(
            "img",
            Some(class),
            vec![
                attr("src", &format!("/{file}")),
                attr("alt", ""),
                attr("width", &format!("{}", w.round())),
                attr("height", &format!("{}", h.round())),
            ],
            Vec::new(),
        ));
    }
    // Image (remplissage d'image d'une forme ou d'un cadre sans enfants).
    let kids: Vec<&Value> = n["children"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| visible(c))
        .collect();
    let image = n["fills"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| visible(f))
        .find(|f| f["type"] == "IMAGE")
        .and_then(|f| {
            Some((
                f["imageRef"].as_str()?.to_string(),
                f["scaleMode"].as_str().unwrap_or("FILL").to_string(),
            ))
        });
    if let Some((r, mode)) = &image
        && kids.is_empty()
    {
        cx.images.insert(r.clone());
        let mut d = base_decls(n, parent, cx, false);
        d.push((
            "object-fit".into(),
            if mode == "FIT" { "contain" } else { "cover" }.into(),
        ));
        d.push(("display".into(), "block".into()));
        let class = cx.class(name, "image", d);
        let (_, _, w, h) = box_of(n);
        return Some(el(
            "img",
            Some(class),
            vec![
                attr("src", &format!("figma-image:{r}")),
                attr("alt", if generic_name(name) { "" } else { name }),
                attr("width", &format!("{}", w.round())),
                attr("height", &format!("{}", h.round())),
                attr("loading", "lazy"),
            ],
            Vec::new(),
        ));
    }
    // Cadre, groupe, composant.
    let (x, y, w, _) = box_of(n);
    let mode = n["layoutMode"].as_str().unwrap_or("NONE");
    // Groupe : ses enfants sont placés par rapport au parent du groupe.
    let mut d = base_decls(n, parent, cx, false);
    d.extend(layout_decls(n));
    if let Some((r, mode)) = &image {
        cx.images.insert(r.clone());
        d.push((
            "background-image".into(),
            format!("url(\"figma-image:{r}\")"),
        ));
        d.push((
            "background-size".into(),
            if mode == "FIT" { "contain" } else { "cover" }.into(),
        ));
        d.push(("background-position".into(), "center".into()));
    }
    let pad_l = num(&n["paddingLeft"]);
    let pad_r = num(&n["paddingRight"]);
    let me = Parent {
        row: match mode {
            "HORIZONTAL" => Some(true),
            "VERTICAL" => Some(false),
            _ => None,
        },
        x,
        y,
        inner_w: w - pad_l - pad_r,
    };
    let lower = name.to_ascii_lowercase();
    let button = ["button", "bouton", "btn", "cta"]
        .iter()
        .any(|b| lower.contains(b))
        && kids.iter().filter(|k| k["type"] == "TEXT").count() == 1
        && kids.len() <= 3;
    let mut children: Vec<Node> = Vec::new();
    for k in &kids {
        if let Some(c) = convert(k, Some(me), cx) {
            children.push(c);
        }
    }
    let section = parent.is_some_and(|p| p.row == Some(false))
        && matches!(ty, "FRAME" | "SECTION")
        && cx.word("section") == "section"
        && depth_hint(n);
    let (tag, role, attrs) = if button {
        // Lien en forme de bouton : texte du bouton dans le lien.
        d.push(("text-decoration".into(), "none".into()));
        ("a", "button", vec![attr("href", "#")])
    } else if section {
        ("section", "section", Vec::new())
    } else {
        ("div", "block", Vec::new())
    };
    let class = cx.class(name, role, d);
    let node = el(tag, Some(class), attrs, children);
    if ty == "INSTANCE"
        && let Some(cid) = n["componentId"].as_str()
    {
        cx.instances.push((cid.to_string(), id.to_string()));
        // Repère retiré à la fin (composants).
        if let Node::Element {
            tag,
            attrs,
            children,
            ..
        } = node
        {
            let mut attrs = attrs;
            attrs.push(attr("data-figma-instance", &format!("{cid}|{id}")));
            return Some(Node::Element {
                id: None,
                tag,
                attrs,
                children,
            });
        }
    }
    Some(node)
}

/// Grand cadre placé dans la colonne du cadre de page : section.
fn depth_hint(n: &Value) -> bool {
    let (_, _, w, h) = box_of(n);
    w >= 600.0 && h >= 120.0
}

/// Composants : utilisations de même structure (textes à part) → composant Revel, textes en
/// propriétés ; les autres utilisations restent des éléments.
fn components(
    pages: &mut [Page],
    site: &mut Site,
    defs: &serde_json::Map<String, Value>,
    report: &mut Report,
) {
    // Utilisations par composant.
    fn collect(nodes: &[Node], out: &mut BTreeMap<String, Vec<Node>>) {
        for n in nodes {
            if let Node::Element {
                attrs, children, ..
            } = n
            {
                if let Some(v) = attrs
                    .iter()
                    .find(|a| a.name == "data-figma-instance")
                    .and_then(|a| a.value.clone())
                {
                    let cid = v.split('|').next().unwrap_or("").to_string();
                    out.entry(cid).or_default().push(n.clone());
                    continue;
                }
                collect(children, out);
            }
        }
    }
    fn texts(n: &Node, out: &mut Vec<String>) {
        match n {
            Node::Text { text } => out.push(text.clone()),
            Node::Element { children, .. } => children.iter().for_each(|c| texts(c, out)),
            _ => {}
        }
    }
    fn shape(n: &Node) -> String {
        match n {
            Node::Text { .. } => "#t".into(),
            Node::Element {
                tag,
                attrs,
                children,
                ..
            } => format!(
                "<{tag} {}>{}</{tag}>",
                attrs
                    .iter()
                    .filter(|a| a.name != "data-figma-instance")
                    .map(|a| format!("{}={:?}", a.name, a.value))
                    .collect::<Vec<_>>()
                    .join(" "),
                children.iter().map(shape).collect::<String>()
            ),
            _ => String::new(),
        }
    }
    let mut uses: BTreeMap<String, Vec<Node>> = BTreeMap::new();
    for p in pages.iter() {
        collect(&p.body, &mut uses);
    }
    let mut made: HashMap<String, (String, usize)> = HashMap::new();
    for (cid, list) in &uses {
        if list.len() < 2 {
            continue;
        }
        let s0 = shape(&list[0]);
        if !list.iter().all(|n| shape(n) == s0) {
            continue;
        }
        let name = defs
            .get(cid)
            .and_then(|d| d["name"].as_str())
            .unwrap_or("Composant")
            .to_string();
        let mut id = revel_model::slugify(&name);
        if id.is_empty() {
            id = "composant".into();
        }
        while site.components.iter().any(|c| c.id == id) {
            id.push_str("-2");
        }
        // Corps : premier usage, textes remplacés par des propriétés.
        let mut body = list[0].clone();
        let mut k = 0;
        fn props_of(n: &mut Node, k: &mut usize) {
            match n {
                Node::Text { .. } => {
                    *k += 1;
                    *n = Node::Out {
                        expr: format!("texte{k}"),
                    };
                }
                Node::Element {
                    attrs, children, ..
                } => {
                    attrs.retain(|a| a.name != "data-figma-instance");
                    children.iter_mut().for_each(|c| props_of(c, k));
                }
                _ => {}
            }
        }
        props_of(&mut body, &mut k);
        let mut first = Vec::new();
        texts(&list[0], &mut first);
        let mut c = Component::new(id.clone(), vec![body]);
        c.name = Some(name);
        c.props = (1..=k)
            .map(|i| PropDef {
                name: format!("texte{i}"),
                label: Some(format!("Texte {i}")),
                kind: PropKind::Text,
                default: first.get(i - 1).map(|t| Value::String(t.clone())),
            })
            .collect();
        site.components.push(c);
        made.insert(cid.clone(), (id, k));
        report.components += 1;
    }
    // Utilisations remplacées ; repères retirés partout.
    fn replace(nodes: &mut [Node], made: &HashMap<String, (String, usize)>) {
        for n in nodes.iter_mut() {
            let inst = match &*n {
                Node::Element { attrs, .. } => attrs
                    .iter()
                    .find(|a| a.name == "data-figma-instance")
                    .and_then(|a| a.value.clone()),
                _ => None,
            };
            if let Some(v) = inst {
                let cid = v.split('|').next().unwrap_or("").to_string();
                if let Some((id, _)) = made.get(&cid) {
                    let mut t = Vec::new();
                    texts(n, &mut t);
                    let args = t
                        .iter()
                        .enumerate()
                        .map(|(i, s)| {
                            (
                                format!("texte{}", i + 1),
                                serde_json::to_string(s).unwrap_or_default(),
                            )
                        })
                        .collect();
                    *n = Node::Use {
                        id: None,
                        component: id.clone(),
                        args,
                        variant: None,
                        attrs: Vec::new(),
                    };
                    continue;
                }
            }
            if let Node::Element {
                attrs, children, ..
            } = n
            {
                attrs.retain(|a| a.name != "data-figma-instance");
                replace(children, made);
            }
        }
    }
    for p in pages.iter_mut() {
        replace(&mut p.body, &made);
    }
}

/// Importe les cadres choisis du fichier `file` (clé `key`). Images et formes téléchargées dans
/// `dest` (`images/figma/`).
pub fn import(
    file: &Value,
    key: &str,
    token: &str,
    fetch: &dyn Fetch,
    dest: &Path,
    opts: &Options,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<Imported, FigmaError> {
    let stopped = || opts.cancel.is_some_and(|c| c.load(Ordering::SeqCst));
    let fr = opts.lang.to_ascii_lowercase().starts_with("fr");
    let mut index: HashMap<String, &Value> = HashMap::new();
    fn walk<'v>(n: &'v Value, depth: usize, index: &mut HashMap<String, &'v Value>) {
        if depth > 2 {
            return;
        }
        if let Some(id) = n["id"].as_str() {
            index.insert(id.to_string(), n);
        }
        for c in n["children"].as_array().into_iter().flatten() {
            walk(c, depth + 1, index);
        }
    }
    walk(&file["document"], 0, &mut index);
    let frames: Vec<&Value> = opts
        .frames
        .iter()
        .filter_map(|id| index.get(id).copied())
        .collect();
    if frames.is_empty() {
        return Err(FigmaError::Api("aucun cadre choisi".into()));
    }
    let empty = serde_json::Map::new();
    let defs = file["components"].as_object().unwrap_or(&empty);
    let mut cx = Cx {
        fr,
        rules: Vec::new(),
        by_decls: HashMap::new(),
        used: HashMap::new(),
        images: BTreeSet::new(),
        svgs: BTreeMap::new(),
        fonts: BTreeMap::new(),
        instances: Vec::new(),
        absolute: 0,
        h1: false,
        max_font: 0.0,
        styles: file["styles"]
            .as_object()
            .map(|m| {
                m.iter()
                    .filter(|(_, v)| v["styleType"] == "FILL")
                    .filter_map(|(k, v)| Some((k.clone(), v["name"].as_str()?.to_string())))
                    .collect()
            })
            .unwrap_or_default(),
        colors: Vec::new(),
    };
    let mut report = Report {
        tool: Some("Figma".into()),
        ..Default::default()
    };
    let name = file["name"].as_str().unwrap_or("Figma").to_string();
    let lang = if opts.lang.is_empty() {
        "en".to_string()
    } else {
        opts.lang.clone()
    };
    let mut site = Site::new(name.clone(), lang.clone());
    site.data.insert(
        "site".into(),
        json!({ "title": name, "lang": lang, "description": "" }),
    );
    let mut pages = Vec::new();
    let mut paths = std::collections::HashSet::new();
    for (i, f) in frames.iter().enumerate() {
        cx.h1 = false;
        cx.max_font = max_font(f);
        let fname = f["name"].as_str().unwrap_or("Page");
        let mut d: Vec<(String, String)> = Vec::new();
        let (x, y, w, h) = box_of(f);
        // Cadre de page : toute la largeur de l'écran, contenu centré à sa largeur d'origine
        // s'il n'a pas de mise en page automatique.
        let mode = f["layoutMode"].as_str().unwrap_or("NONE");
        d.extend(base_decls(f, None, &mut cx, false));
        d.extend(layout_decls(f));
        if mode == "NONE" {
            d.push(("width".into(), "100%".into()));
            d.push(("max-width".into(), px(w)));
            d.push(("min-height".into(), px(h)));
            d.push(("margin".into(), "0 auto".into()));
        } else {
            d.push(("min-height".into(), "100vh".into()));
        }
        let me = Parent {
            row: match mode {
                "HORIZONTAL" => Some(true),
                "VERTICAL" => Some(false),
                _ => None,
            },
            x,
            y,
            inner_w: w - num(&f["paddingLeft"]) - num(&f["paddingRight"]),
        };
        let children: Vec<Node> = f["children"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| convert(c, Some(me), &mut cx))
            .collect();
        let class = cx.class(fname, "page", d);
        let path = if i == 0 {
            "/".to_string()
        } else {
            let mut s = revel_model::slugify(fname);
            if s.is_empty() {
                s = format!("page-{i}");
            }
            let mut p = format!("/{s}/");
            while !paths.insert(p.clone()) {
                p = format!("/{s}-{}/", paths.len());
            }
            p
        };
        let mut props = BTreeMap::new();
        props.insert("seo".to_string(), json!({ "title": fname }));
        pages.push(Page {
            id: if i == 0 {
                "index".into()
            } else {
                revel_model::slugify(&path).trim_matches('-').to_string()
            },
            path,
            layout: Some("base".into()),
            props,
            computed: Default::default(),
            body: vec![el("main", Some(class), Vec::new(), children)],
            draft: false,
            name: Some(fname.to_string()),
            code: Default::default(),
        });
    }
    components(&mut pages, &mut site, defs, &mut report);
    report.pages = pages.len();
    if cx.absolute > 0 {
        report.warnings.push(format!(
            "Figma : {} élément(s) placé(s) à une position fixe (cadres sans mise en page automatique), à revoir pour les petits écrans",
            cx.absolute
        ));
    }

    // Images et formes téléchargées.
    let total = cx.images.len() + cx.svgs.len();
    let mut done = 0;
    let mut image_paths: HashMap<String, String> = HashMap::new();
    let auth = [("X-Figma-Token", token.trim())];
    if !cx.images.is_empty() {
        let v = get_json(fetch, &format!("{API}/files/{key}/images"), &auth).map_err(api_error)?;
        let map = v["meta"]["images"]
            .as_object()
            .or_else(|| v["images"].as_object())
            .cloned()
            .unwrap_or_default();
        let jobs: Vec<(Url, String, String)> = cx
            .images
            .iter()
            .filter_map(|r| {
                let u = Url::parse(map.get(r)?.as_str()?).ok()?;
                Some((u, format!("images/figma/{r}"), r.clone()))
            })
            .collect();
        let list: Vec<(Url, String)> = jobs
            .iter()
            .map(|(u, p, _)| (u.clone(), p.clone()))
            .collect();
        let ok = crate::crawl::download(fetch, &list, dest, 6, opts.cancel, &mut |n| {
            progress(done + n, total)
        });
        for ((_, rel, r), good) in jobs.iter().zip(&ok) {
            if !*good {
                continue;
            }
            // Extension d'après le contenu.
            let path = dest.join(rel);
            let ext = std::fs::read(&path)
                .ok()
                .map(|b| sniff(&b))
                .unwrap_or("png");
            let with = format!("{rel}.{ext}");
            if std::fs::rename(&path, dest.join(&with)).is_ok() {
                image_paths.insert(r.clone(), format!("/{with}"));
                report.files += 1;
            }
        }
        done += jobs.len();
        let missing = cx.images.len() - image_paths.len();
        if missing > 0 {
            report
                .warnings
                .push(format!("Figma : {missing} image(s) non téléchargée(s)"));
        }
    }
    if stopped() {
        return Err(FigmaError::Cancelled);
    }
    if !cx.svgs.is_empty() {
        let ids: Vec<&String> = cx.svgs.keys().collect();
        let mut failed = 0;
        for chunk in ids.chunks(50) {
            let q = chunk
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(",");
            let v = get_json(
                fetch,
                &format!(
                    "{API}/images/{key}?ids={}&format=svg",
                    percent_encoding::utf8_percent_encode(&q, percent_encoding::NON_ALPHANUMERIC)
                ),
                &auth,
            )
            .map_err(api_error)?;
            let map = v["images"].as_object().cloned().unwrap_or_default();
            let jobs: Vec<(Url, String)> = chunk
                .iter()
                .filter_map(|id| {
                    let u = Url::parse(map.get(*id)?.as_str()?).ok()?;
                    Some((u, cx.svgs[*id].clone()))
                })
                .collect();
            failed += chunk.len() - jobs.len();
            let ok = crate::crawl::download(fetch, &jobs, dest, 6, opts.cancel, &mut |n| {
                progress(done + n, total)
            });
            failed += ok.iter().filter(|o| !**o).count();
            report.files += ok.iter().filter(|o| **o).count();
            done += chunk.len();
            if stopped() {
                return Err(FigmaError::Cancelled);
            }
        }
        if failed > 0 {
            report
                .warnings
                .push(format!("Figma : {failed} forme(s) non exportée(s) en SVG"));
        }
    }
    // Adresses provisoires des images remplacées.
    let fix = |s: &str| -> String {
        let mut out = s.to_string();
        while let Some(i) = out.find("figma-image:") {
            let rest = &out[i + 12..];
            let end = rest
                .find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(rest.len());
            let r = rest[..end].to_string();
            let to = image_paths.get(&r).cloned().unwrap_or_default();
            out.replace_range(i..i + 12 + end, &to);
        }
        out
    };
    fn fix_nodes(nodes: &mut [Node], fix: &dyn Fn(&str) -> String) {
        for n in nodes {
            if let Node::Element {
                attrs, children, ..
            } = n
            {
                for a in attrs.iter_mut() {
                    if let Some(v) = &a.value
                        && v.contains("figma-image:")
                    {
                        a.value = Some(fix(v));
                    }
                }
                fix_nodes(children, fix);
            }
        }
    }
    for p in pages.iter_mut() {
        fix_nodes(&mut p.body, &fix);
    }
    for c in site.components.iter_mut() {
        fix_nodes(&mut c.body, &fix);
    }

    // Feuille du site.
    let mut items = Vec::new();
    if !cx.colors.is_empty() {
        items.push(StyleItem::Rule(StyleRule {
            id: None,
            selector: ":root".into(),
            media: None,
            decls: cx
                .colors
                .iter()
                .map(|(p, v)| Decl {
                    prop: p.clone(),
                    value: v.clone(),
                    important: false,
                })
                .collect(),
        }));
    }
    items.extend([
        rule("*, *::before, *::after", vec![("box-sizing", "border-box")]),
        rule("body", vec![("margin", "0")]),
        rule("img", vec![("max-width", "100%")]),
        rule("a", vec![("color", "inherit")]),
    ]);
    for (class, decls) in &cx.rules {
        if decls.is_empty() {
            continue;
        }
        items.push(StyleItem::Rule(StyleRule {
            id: None,
            selector: format!(".{class}"),
            media: None,
            decls: decls
                .iter()
                .map(|(p, v)| Decl {
                    prop: p.clone(),
                    value: fix(v),
                    important: false,
                })
                .collect(),
        }));
    }
    report.style_rules = items.len();
    site.stylesheet = Some(Stylesheet {
        url: "/assets/styles.css".into(),
        source: None,
        items,
    });
    site.layouts.push(Layout {
        id: "base".into(),
        parent: None,
        props: Default::default(),
        body: crate::html::base_document(&lang, "/assets/styles.css"),
    });
    report.layouts = 1;
    site.pages = pages;
    site.passthrough.push(revel_model::Passthrough {
        from: "images".into(),
        to: "images".into(),
    });
    site.ensure_ids();
    Ok(Imported {
        site,
        report,
        fonts: cx.fonts,
    })
}

fn rule(selector: &str, decls: Vec<(&str, &str)>) -> StyleItem {
    StyleItem::Rule(StyleRule {
        id: None,
        selector: selector.into(),
        media: None,
        decls: decls
            .into_iter()
            .map(|(p, v)| Decl {
                prop: p.into(),
                value: v.into(),
                important: false,
            })
            .collect(),
    })
}

/// Plus grande taille de texte d'un cadre (titre principal).
fn max_font(n: &Value) -> f64 {
    let mut m: f64 = 0.0;
    if n["type"] == "TEXT" {
        m = n["style"]["fontSize"].as_f64().unwrap_or(0.0);
    }
    for c in n["children"].as_array().into_iter().flatten() {
        m = m.max(max_font(c));
    }
    m
}

/// Extension d'une image d'après ses premiers octets.
fn sniff(b: &[u8]) -> &'static str {
    if b.starts_with(&[0x89, b'P', b'N', b'G']) {
        "png"
    } else if b.starts_with(&[0xff, 0xd8]) {
        "jpg"
    } else if b.starts_with(b"GIF8") {
        "gif"
    } else if b.len() > 12 && &b[8..12] == b"WEBP" {
        "webp"
    } else if b.starts_with(b"<svg") || b.starts_with(b"<?xml") {
        "svg"
    } else {
        "png"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crawl::Fetched;

    #[test]
    fn adresses_figma() {
        assert_eq!(
            file_key("https://www.figma.com/design/AbCdEf1234567890/Mon-site?node-id=0-1")
                .as_deref(),
            Some("AbCdEf1234567890")
        );
        assert_eq!(
            file_key("figma.com/file/AbCdEf1234567890/X").as_deref(),
            Some("AbCdEf1234567890")
        );
        assert_eq!(
            file_key("https://www.figma.com/design/AbCdEf1234567890/branch/ZyXw9876543210/X")
                .as_deref(),
            Some("ZyXw9876543210")
        );
        assert_eq!(
            file_key("AbCdEf1234567890").as_deref(),
            Some("AbCdEf1234567890")
        );
        assert_eq!(
            file_key("https://exemple.com/design/AbCdEf1234567890/"),
            None
        );
        assert_eq!(file_key("bonjour"), None);
    }

    /// Réponses de l'API de Figma (format de la documentation : nœuds, images).
    struct Mock;
    impl Fetch for Mock {
        fn get(&self, url: &Url) -> Result<Fetched, String> {
            let body: Vec<u8> = match url.path() {
                "/v1/files/KEY1234567890/images" => br#"{"error":false,"status":200,"meta":{"images":{"img1":"https://s3.example/img1"}}}"#.to_vec(),
                "/v1/images/KEY1234567890" => br#"{"err":null,"images":{"1:20":"https://s3.example/icon.svg"}}"#.to_vec(),
                "/img1" => vec![0x89, b'P', b'N', b'G', 0, 0],
                "/icon.svg" => b"<svg xmlns='http://www.w3.org/2000/svg'/>".to_vec(),
                _ => return Ok(Fetched { status: 404, url: url.clone(), content_type: String::new(), body: Vec::new() }),
            };
            Ok(Fetched {
                status: 200,
                url: url.clone(),
                content_type: String::new(),
                body,
            })
        }
    }

    pub fn sample() -> Value {
        let text = |id: &str, chars: &str, size: f64, w: f64| {
            json!({
                "id": id, "name": chars, "type": "TEXT", "characters": chars,
                "absoluteBoundingBox": { "x": 0, "y": 0, "width": w, "height": size * 1.2 },
                "layoutSizingHorizontal": "HUG", "layoutSizingVertical": "HUG",
                "style": { "fontFamily": "Inter", "fontWeight": 700, "fontSize": size, "lineHeightUnit": "PIXELS", "lineHeightPx": size * 1.2, "textAlignHorizontal": "LEFT" },
                "fills": [{ "type": "SOLID", "color": { "r": 0.1, "g": 0.1, "b": 0.1, "a": 1 } }]
            })
        };
        let card = |id: &str, title: &str| {
            json!({
                "id": id, "name": "Carte", "type": "INSTANCE", "componentId": "9:1",
                "absoluteBoundingBox": { "x": 0, "y": 0, "width": 300, "height": 200 },
                "layoutMode": "VERTICAL", "itemSpacing": 8, "paddingTop": 24, "paddingRight": 24, "paddingBottom": 24, "paddingLeft": 24,
                "layoutSizingHorizontal": "FILL", "layoutSizingVertical": "HUG",
                "cornerRadius": 12, "fills": [{ "type": "SOLID", "color": { "r": 1, "g": 1, "b": 1, "a": 1 } }],
                "effects": [{ "type": "DROP_SHADOW", "visible": true, "color": { "r": 0, "g": 0, "b": 0, "a": 0.1 }, "offset": { "x": 0, "y": 4 }, "radius": 16, "spread": 0 }],
                "children": [ text(&format!("{id}t"), title, 20.0, 252.0) ]
            })
        };
        json!({
            "name": "Atelier",
            "components": { "9:1": { "key": "k", "name": "Carte" } },
            "styles": { "S:1": { "key": "s", "name": "Couleurs/Fond clair", "styleType": "FILL" } },
            "document": { "id": "0:0", "type": "DOCUMENT", "children": [
                { "id": "0:1", "name": "Pages", "type": "CANVAS", "children": [
                    { "id": "1:1", "name": "Accueil", "type": "FRAME",
                      "absoluteBoundingBox": { "x": 0, "y": 0, "width": 1440, "height": 1200 },
                      "layoutMode": "VERTICAL", "itemSpacing": 0, "counterAxisAlignItems": "CENTER",
                      "fills": [{ "type": "SOLID", "color": { "r": 0.98, "g": 0.97, "b": 0.95, "a": 1 } }],
                      "styles": { "fill": "S:1" },
                      "children": [
                        { "id": "1:2", "name": "Hero", "type": "FRAME",
                          "absoluteBoundingBox": { "x": 0, "y": 0, "width": 1440, "height": 600 },
                          "layoutMode": "VERTICAL", "itemSpacing": 24, "paddingTop": 120, "paddingBottom": 120, "paddingLeft": 80, "paddingRight": 80,
                          "primaryAxisAlignItems": "CENTER", "counterAxisAlignItems": "CENTER",
                          "layoutSizingHorizontal": "FIXED", "layoutSizingVertical": "FIXED",
                          "children": [
                            text("1:3", "Céramiques faites main", 64.0, 900.0),
                            { "id": "1:4", "name": "Photo", "type": "RECTANGLE", "absoluteBoundingBox": { "x": 0, "y": 0, "width": 640, "height": 360 },
                              "layoutSizingHorizontal": "FIXED", "layoutSizingVertical": "FIXED", "cornerRadius": 8,
                              "fills": [{ "type": "IMAGE", "scaleMode": "FILL", "imageRef": "img1" }] },
                            { "id": "1:20", "name": "Logo", "type": "GROUP", "absoluteBoundingBox": { "x": 0, "y": 0, "width": 32, "height": 32 },
                              "children": [{ "id": "1:21", "name": "Vector 1", "type": "VECTOR", "absoluteBoundingBox": { "x": 0, "y": 0, "width": 32, "height": 32 } }] },
                            { "id": "1:22", "name": "Masqué", "type": "FRAME", "visible": false, "absoluteBoundingBox": { "x": 0, "y": 0, "width": 1, "height": 1 } }
                          ] },
                        { "id": "1:5", "name": "Cartes", "type": "FRAME",
                          "absoluteBoundingBox": { "x": 0, "y": 600, "width": 1000, "height": 300 },
                          "layoutMode": "HORIZONTAL", "itemSpacing": 32, "layoutSizingHorizontal": "FIXED", "layoutSizingVertical": "HUG",
                          "children": [ card("1:6", "Bols"), card("1:7", "Vases") ] }
                      ] },
                    { "id": "2:1", "name": "Contact", "type": "FRAME", "absoluteBoundingBox": { "x": 2000, "y": 0, "width": 1440, "height": 800 }, "children": [
                        { "id": "2:2", "name": "Titre", "type": "TEXT", "characters": "Écrire", "absoluteBoundingBox": { "x": 2100, "y": 100, "width": 300, "height": 50 },
                          "style": { "fontFamily": "Inter", "fontWeight": 400, "fontSize": 40 } }
                    ] }
                ] }
            ] }
        })
    }

    #[test]
    fn cadres_convertis() {
        let file = sample();
        let o = outline(&file);
        assert_eq!(o.len(), 1);
        assert_eq!(o[0].2.len(), 2);
        assert_eq!(o[0].2[0].name, "Accueil");
        let dir = tempfile::tempdir().unwrap();
        let r = import(
            &file,
            "KEY1234567890",
            "jeton",
            &Mock,
            dir.path(),
            &Options {
                frames: vec!["1:1".into(), "2:1".into()],
                lang: "fr".into(),
                cancel: None,
            },
            &mut |_, _| {},
        )
        .unwrap();
        let s = &r.site;
        assert_eq!(s.pages.len(), 2);
        assert_eq!(s.pages[0].path, "/");
        assert_eq!(s.pages[1].path, "/contact/");
        let html = crate::html::to_html(&s.pages[0].body);
        assert!(
            html.contains(r#"<h1 class="titre">Céramiques faites main</h1>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<img class="photo" src="/images/figma/img1.png""#),
            "{html}"
        );
        assert!(
            html.contains(r#"src="/images/figma/logo-1-20.svg""#),
            "{html}"
        );
        assert!(!html.contains("Masqué"));
        assert!(dir.path().join("images/figma/img1.png").is_file());
        assert!(dir.path().join("images/figma/logo-1-20.svg").is_file());
        // Composant : deux cartes de même structure.
        assert_eq!(s.components.len(), 1);
        assert_eq!(s.components[0].props.len(), 1);
        let uses: Vec<_> = {
            let mut v = Vec::new();
            revel_model::ids::walk(&s.pages[0].body, &mut |n| {
                if let Node::Use { args, .. } = n {
                    v.push(args.get("texte1").cloned().unwrap_or_default());
                }
            });
            v
        };
        assert_eq!(uses, vec!["\"Bols\"".to_string(), "\"Vases\"".to_string()]);
        // Styles.
        let css = revel_style::generate(&s.stylesheet.as_ref().unwrap().items);
        assert!(css.contains("flex-direction: column"), "{css}");
        assert!(
            css.contains("--couleurs-fond-clair: #faf7f2"),
            "couleur du site : {css}"
        );
        assert!(
            css.contains("background-color: var(--couleurs-fond-clair)"),
            "{css}"
        );
        assert!(css.contains("gap: 32px"), "{css}");
        assert!(
            css.contains("box-shadow: 0 4px 16px 0 rgba(0, 0, 0, 0.1)"),
            "{css}"
        );
        assert!(css.contains("font-family: \"Inter\", sans-serif"), "{css}");
        assert!(
            css.contains("position: absolute"),
            "page sans mise en page automatique : {css}"
        );
        assert_eq!(r.fonts.get("Inter").map(|v| v.len()), Some(2));
    }
}
