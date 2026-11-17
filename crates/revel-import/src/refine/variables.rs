//! Couleurs et tailles répétées devenues des variables du site (D-178) : écrites dans `:root`
//! (Paramètres › Couleurs et Tailles), lues par les règles (`var(--nom)`). Le rendu ne change
//! pas ; une couleur du site se change ensuite à un seul endroit.

use super::Report;
use revel_model::{Decl, Site, StyleItem, StyleRule};
use revel_style::values::color::Rgba;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Propriétés dont la valeur entière est une couleur.
const COLOR_PROPS: &[&str] = &[
    "color",
    "background-color",
    "background",
    "border-color",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    "outline-color",
    "fill",
    "stroke",
    "text-decoration-color",
    "caret-color",
    "accent-color",
];

/// Variables créées au plus, par nature.
const MAX_COLORS: usize = 16;
const MAX_SIZES: usize = 8;

/// Couleur arrondie (rouge, vert, bleu, opacité en centièmes) : deux écritures de la même
/// couleur ont la même clé.
type Key = (u8, u8, u8, u8);

fn key(c: &Rgba) -> Key {
    (
        c.r.round() as u8,
        c.g.round() as u8,
        c.b.round() as u8,
        (c.a * 100.0).round() as u8,
    )
}

fn color_of(d: &Decl) -> Option<Rgba> {
    if !COLOR_PROPS.contains(&d.prop.as_str()) {
        return None;
    }
    let v = d.value.trim();
    if v.contains("var(") || v.eq_ignore_ascii_case("transparent") {
        return None;
    }
    Rgba::parse(v).filter(|c| c.a > 0.0)
}

/// Nom d'une couleur d'après sa teinte (`bleu`, `gris-clair`, `noir-50` pour une transparence).
fn color_name(c: &Rgba, fr: bool) -> String {
    let (h, s, l) = c.to_hsl();
    let pick = |f: &'static str, e: &'static str| if fr { f } else { e };
    let mut name = if l > 0.97 {
        pick("blanc", "white").to_string()
    } else if l < 0.08 {
        pick("noir", "black").to_string()
    } else if s < 0.12 {
        pick("gris", "gray").to_string()
    } else {
        match h {
            h if !(15.0..340.0).contains(&h) => pick("rouge", "red"),
            h if h < 45.0 => "orange",
            h if h < 70.0 => pick("jaune", "yellow"),
            h if h < 165.0 => pick("vert", "green"),
            h if h < 200.0 => "turquoise",
            h if h < 255.0 => pick("bleu", "blue"),
            h if h < 290.0 => pick("violet", "purple"),
            _ => pick("rose", "pink"),
        }
        .to_string()
    };
    if (0.08..=0.97).contains(&l) {
        if l > 0.75 {
            name.push_str(pick("-clair", "-light"));
        } else if l < 0.3 {
            name.push_str(pick("-fonce", "-dark"));
        }
    }
    if c.a < 1.0 {
        name.push_str(&format!("-{}", (c.a * 100.0).round() as u32));
    }
    name
}

fn free(taken: &mut HashSet<String>, base: &str) -> String {
    let mut name = base.to_string();
    let mut n = 2;
    while taken.contains(&name) {
        name = format!("{base}-{n}");
        n += 1;
    }
    taken.insert(name.clone());
    name
}

/// Taille de texte en pixels d'une valeur écrite (`1.5rem`, `24px`, `clamp(…, 3rem)` : la plus
/// grande) ; `None` : valeur relative ou non reconnue.
fn px_of(v: &str) -> Option<(f64, bool)> {
    let v = v.trim().to_ascii_lowercase();
    let one = |t: &str| -> Option<f64> {
        let t = t.trim();
        if let Some(n) = t.strip_suffix("rem") {
            n.trim().parse::<f64>().ok().map(|n| n * 16.0)
        } else if let Some(n) = t.strip_suffix("px") {
            n.trim().parse::<f64>().ok()
        } else {
            None
        }
    };
    if let Some(inner) = v.strip_prefix("clamp(").and_then(|t| t.strip_suffix(')')) {
        // Dernier argument de premier niveau.
        let mut depth = 0;
        let mut last = 0;
        for (i, c) in inner.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                ',' if depth == 0 => last = i + 1,
                _ => {}
            }
        }
        return one(&inner[last..]).map(|n| (n, true));
    }
    one(&v).map(|n| (n, false))
}

fn rules(site: &mut Site) -> impl Iterator<Item = &mut StyleRule> {
    site.stylesheet.iter_mut().flat_map(|s| {
        s.items.iter_mut().filter_map(|i| match i {
            StyleItem::Rule(r) if r.selector.trim() != ":root" => Some(r),
            _ => None,
        })
    })
}

pub(super) fn run(site: &mut Site, report: &mut Report) {
    if site.stylesheet.is_none() {
        return;
    }
    let fr = site.default_locale.starts_with("fr");
    // Variables déjà écrites : reprises pour les mêmes valeurs.
    let mut taken: HashSet<String> = HashSet::new();
    let mut color_var: HashMap<Key, String> = HashMap::new();
    // Tailles de texte et largeur du site (valeur écrite : nom) ; une taille déjà écrite dans
    // `:root` n'est pas reprise, rien ne dit à quoi elle sert.
    let mut text_var: HashMap<String, String> = HashMap::new();
    let mut width_var: Option<(String, String)> = None;
    for r in site
        .style_rules()
        .filter(|r| r.selector.trim() == ":root" && r.media.is_none())
    {
        for d in &r.decls {
            let Some(name) = d.prop.strip_prefix("--") else {
                continue;
            };
            taken.insert(name.to_string());
            if let Some(c) = Rgba::parse(d.value.trim()) {
                color_var.entry(key(&c)).or_insert_with(|| name.to_string());
            }
        }
    }
    // Couleurs du texte et du fond de la page : nommées d'après leur rôle.
    let mut role: HashMap<Key, &str> = HashMap::new();
    for r in site.style_rules().filter(|r| {
        r.media.is_none()
            && matches!(
                r.selector.trim(),
                "body" | "html" | "html, body" | "html,body"
            )
    }) {
        for d in &r.decls {
            if let Some(c) = color_of(d) {
                let name = match (d.prop.as_str(), fr) {
                    ("color", true) => "texte",
                    ("color", false) => "text",
                    ("background" | "background-color", true) => "fond",
                    ("background" | "background-color", false) => "background",
                    _ => continue,
                };
                role.entry(key(&c)).or_insert(name);
            }
        }
    }
    let mut colors: BTreeMap<Key, (usize, Rgba)> = BTreeMap::new();
    let mut sizes: BTreeMap<String, usize> = BTreeMap::new();
    let mut widths: BTreeMap<String, usize> = BTreeMap::new();
    for r in rules(site) {
        for d in &r.decls {
            if let Some(c) = color_of(d) {
                colors.entry(key(&c)).or_insert((0, c)).0 += 1;
            } else if d.prop == "font-size" && px_of(&d.value).is_some() {
                *sizes
                    .entry(d.value.trim().to_ascii_lowercase())
                    .or_default() += 1;
            } else if d.prop == "max-width"
                && px_of(&d.value).is_some_and(|(n, fluid)| !fluid && n >= 640.0)
            {
                *widths
                    .entry(d.value.trim().to_ascii_lowercase())
                    .or_default() += 1;
            }
        }
    }
    let mut new_decls: Vec<Decl> = Vec::new();
    // Couleurs : les plus employées d'abord.
    let mut list: Vec<(&Key, &(usize, Rgba))> = colors
        .iter()
        .filter(|(k, (n, _))| !color_var.contains_key(*k) && (*n >= 2 || role.contains_key(*k)))
        .collect();
    list.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(b.0)));
    list.truncate(MAX_COLORS);
    let mut made: Vec<(Key, String, String)> = Vec::new();
    for (k, (_, c)) in list {
        let base = role
            .get(k)
            .map(|r| r.to_string())
            .unwrap_or_else(|| color_name(c, fr));
        let name = free(&mut taken, &base);
        made.push((*k, name, c.to_css()));
    }
    for (k, name, css) in made {
        new_decls.push(Decl {
            prop: format!("--{name}"),
            value: css,
            important: false,
        });
        color_var.insert(k, name);
        report.colors += 1;
    }
    // Tailles de texte employées au moins trois fois, de la plus grande à la plus petite.
    let mut list: Vec<(&String, &usize)> = sizes.iter().filter(|(_, n)| **n >= 3).collect();
    list.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    list.truncate(MAX_SIZES);
    for (v, _) in list {
        let Some((px, fluid)) = px_of(v) else {
            continue;
        };
        let n = if px.fract() == 0.0 {
            format!("{px:.0}")
        } else {
            format!("{px:.1}").replace('.', "-")
        };
        let base = match (fr, fluid) {
            (true, false) => format!("texte-{n}"),
            (true, true) => format!("texte-fluide-{n}"),
            (false, false) => format!("text-{n}"),
            (false, true) => format!("text-fluid-{n}"),
        };
        let name = free(&mut taken, &base);
        new_decls.push(Decl {
            prop: format!("--{name}"),
            value: v.clone(),
            important: false,
        });
        text_var.insert(v.clone(), name);
        report.sizes += 1;
    }
    // Largeur du site : la largeur maximale la plus employée (au moins deux fois).
    if let Some((v, _)) = widths
        .iter()
        .filter(|(_, n)| **n >= 2)
        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
    {
        let name = free(&mut taken, if fr { "largeur-site" } else { "site-width" });
        new_decls.push(Decl {
            prop: format!("--{name}"),
            value: v.clone(),
            important: false,
        });
        width_var = Some((v.clone(), name));
        report.sizes += 1;
    }
    if color_var.is_empty() && text_var.is_empty() && width_var.is_none() {
        return;
    }
    for r in rules(site) {
        for d in &mut r.decls {
            if let Some(c) = color_of(d) {
                if let Some(name) = color_var.get(&key(&c)) {
                    d.value = format!("var(--{name})");
                }
            } else if d.prop == "font-size" {
                if let Some(name) = text_var.get(&d.value.trim().to_ascii_lowercase()) {
                    d.value = format!("var(--{name})");
                }
            } else if d.prop == "max-width"
                && let Some((v, name)) = &width_var
                && d.value.trim().eq_ignore_ascii_case(v)
            {
                d.value = format!("var(--{name})");
            }
        }
    }
    if new_decls.is_empty() {
        return;
    }
    let Some(sheet) = site.stylesheet.as_mut() else {
        return;
    };
    let root = sheet.items.iter_mut().find_map(|i| match i {
        StyleItem::Rule(r) if r.selector.trim() == ":root" && r.media.is_none() => Some(r),
        _ => None,
    });
    match root {
        Some(r) => r.decls.extend(new_decls),
        None => sheet.items.insert(
            0,
            StyleItem::Rule(StyleRule {
                id: None,
                selector: ":root".into(),
                media: None,
                decls: new_decls,
            }),
        ),
    }
}
