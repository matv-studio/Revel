//! Relevé des styles (D-151) : ce que le panneau Styles et les Paramètres du site ne savent ni
//! afficher ni régler, donc du code CSS pour l'utilisateur. Complète le relevé des calques
//! ([`crate::editable`]) : un site fait dans l'interface n'a aucun point.
//!
//! Formes reconnues, comme l'interface les écrit :
//! - sélecteur d'une classe, d'une combinaison de classes ou d'une balise proposée (« Tous les
//!   paragraphes »), avec au plus un état du panneau (`:hover`, `[open]`…) ;
//! - élément dans un élément actif, ouvert, éteint ou en pause (`.is-active .titre`) ;
//! - variante d'un composant : règle préfixée par la classe de son élément racine dans la variante
//!   (`.carte:hover .titre`, `.carte.is-sombre .titre`) ;
//! - point de rupture du site (`@media (max-width: 991px)`) ;
//! - `:root` : couleurs du site (et leur valeur du mode sombre) et tailles du site ;
//! - `@font-face` (Paramètres › Polices) ;
//! - propriétés réglées par une commande du panneau (raccourcis compris), variables du site.
//!
//! Le reste est relevé : sélecteur écrit (`.a > .b`, `::before`, liste `h1, h2`), requête média
//! écrite, bloc de code (`@keyframes`), variable locale (`--x` hors `:root`, `var(--x)` qui n'est
//! ni une couleur ni une taille du site), propriété sans réglage dans le panneau.

use crate::editable::{Issue, Kind};
use revel_model::{Breakpoint, Site, StyleItem, StyleRule};
use std::collections::HashSet;

/// Balises réglables toutes ensemble (panneau Styles, « Tous les paragraphes »).
pub const TAG_SELECTORS: &[&str] = &[
    "body",
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "a",
    "img",
    "ul",
    "ol",
    "li",
    "blockquote",
    "strong",
    "em",
    "figure",
    "section",
    "button",
    "video",
];

/// Balises réglables à l'intérieur d'une classe (« Tous les paragraphes dans .prose ») : texte
/// riche, mots mis en valeur d'un titre.
pub const NESTED_TAGS: &[&str] = &[
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "a",
    "img",
    "ul",
    "ol",
    "li",
    "blockquote",
    "strong",
    "em",
    "small",
    "figure",
    "button",
    "video",
];

/// États du panneau ajoutés au sélecteur (les états qui sont des classes, `.is-active`, sont des
/// combinaisons de classes).
pub const STATE_SUFFIXES: &[&str] = &[
    ":hover",
    ":active",
    ":focus-visible",
    "[aria-current=\"page\"]",
    "[open]",
    "[aria-pressed=\"false\"]",
];

/// États « dans un élément … » : sélecteur de l'ancêtre écrit devant.
pub const OUTER_STATES: &[&str] = &[
    ".is-active",
    "[open]",
    "[aria-pressed=\"false\"]",
    ".is-paused",
    ".is-playing",
];

/// Propriétés propres aux outils de Revel, réglées par le panneau (unité « écran » de la taille
/// du texte, D-153).
pub const REVEL_PROPS: &[&str] = &["--revel-fit"];

/// Requête du mode sombre (Paramètres › Couleurs).
pub const DARK: &str = "(prefers-color-scheme: dark)";

/// Propriétés réglées par une commande du panneau Styles (raccourcis lus et modifiés sur place).
pub const PANEL_PROPS: &[&str] = &[
    "display",
    "flex-direction",
    "flex-wrap",
    "flex",
    "justify-content",
    "align-items",
    "align-content",
    "justify-items",
    "justify-self",
    "align-self",
    "order",
    "gap",
    "row-gap",
    "column-gap",
    "grid-template-columns",
    "grid-template-rows",
    "grid-auto-flow",
    "grid-column",
    "grid-row",
    "margin",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "padding",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "width",
    "height",
    "min-width",
    "max-width",
    "min-height",
    "max-height",
    "overflow",
    "object-fit",
    "aspect-ratio",
    "position",
    "inset",
    "top",
    "right",
    "bottom",
    "left",
    "z-index",
    "font-family",
    "font-size",
    "font-weight",
    "line-height",
    "letter-spacing",
    "color",
    "text-align",
    "text-transform",
    "font-style",
    "text-decoration",
    "text-decoration-line",
    "text-overflow",
    "text-shadow",
    "text-wrap",
    "white-space",
    "list-style-type",
    "background",
    "background-color",
    "background-image",
    "background-position",
    "background-repeat",
    "background-size",
    "border",
    "border-width",
    "border-style",
    "border-color",
    "border-top",
    "border-right",
    "border-bottom",
    "border-left",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
    "border-top-style",
    "border-right-style",
    "border-bottom-style",
    "border-left-style",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    "border-radius",
    "border-top-left-radius",
    "border-top-right-radius",
    "border-bottom-right-radius",
    "border-bottom-left-radius",
    "box-shadow",
    "opacity",
    "cursor",
    "mix-blend-mode",
    "filter",
    "backdrop-filter",
    "transform",
    "transform-origin",
    "clip-path",
    "transition",
];

/// Variables du site : couleurs (Paramètres › Couleurs) et tailles (Paramètres › Tailles).
pub fn site_variables(site: &Site) -> HashSet<String> {
    site.style_rules()
        .filter(|r| r.selector == ":root" && r.media.is_none())
        .flat_map(|r| r.decls.iter())
        .filter(|d| d.prop.starts_with("--") && site_value(&d.value))
        .map(|d| d.prop.trim_start_matches("--").to_string())
        .collect()
}

/// Valeur d'une variable du site : couleur ou taille.
fn site_value(v: &str) -> bool {
    revel_style::values::color::Rgba::parse(v.trim()).is_some() || is_size(v)
}

/// Taille (même définition que Paramètres › Tailles) : longueur, ou `clamp`/`min`/`max`/`calc`.
pub fn is_size(v: &str) -> bool {
    let v = v.trim().to_ascii_lowercase();
    if v.is_empty()
        || revel_style::values::color::Rgba::parse(&v).is_some()
        || v.contains("gradient")
        || v.contains('"')
    {
        return false;
    }
    let unit = |t: &str| {
        let num: String = t
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
            .collect();
        !num.is_empty()
            && num.parse::<f64>().is_ok()
            && matches!(
                &t[num.len()..],
                "px" | "rem" | "em" | "vw" | "vh" | "svh" | "dvh" | "%" | "ch" | ""
            )
    };
    ["clamp(", "min(", "max(", "calc("]
        .iter()
        .any(|f| v.starts_with(f))
        || (!v.contains(' ') && unit(&v))
}

/// Sélecteur simple du panneau : balise proposée, ou une ou plusieurs classes ; état facultatif.
/// Rend (classes, état).
fn compound(s: &str) -> Option<(Vec<&str>, Option<&'static str>)> {
    let (base, state) = STATE_SUFFIXES
        .iter()
        .find_map(|st| s.strip_suffix(st).map(|b| (b, Some(*st))))
        .unwrap_or((s, None));
    if TAG_SELECTORS.contains(&base) {
        return Some((Vec::new(), state));
    }
    let rest = base.strip_prefix('.')?;
    let classes: Vec<&str> = rest.split('.').collect();
    classes
        .iter()
        .all(|c| {
            let mut ch = c.chars();
            ch.next()
                .is_some_and(|f| f.is_ascii_alphabetic() || f == '_' || f == '-')
                && ch.all(|x| x.is_ascii_alphanumeric() || x == '_' || x == '-')
        })
        .then_some((classes, state))
}

/// Racines de composant et suffixes de leurs variantes.
fn variant_roots(site: &Site) -> Vec<(String, Vec<String>)> {
    site.components
        .iter()
        .filter(|c| !c.variants.is_empty())
        .filter_map(|c| {
            Some((
                c.root_class()?,
                c.variants.iter().map(|v| v.suffix()).collect(),
            ))
        })
        .collect()
}

/// Racine d'un composant dans une de ses variantes (`.carte:hover`, `.carte.is-sombre`).
fn variant_root(part: &str, roots: &[(String, Vec<String>)]) -> bool {
    let Some((classes, state)) = compound(part) else {
        return false;
    };
    roots.iter().any(|(root, suffixes)| {
        classes.first() == Some(&root.as_str())
            && suffixes.iter().any(|s| match s.strip_prefix('.') {
                Some(cls) => state.is_none() && classes.contains(&cls),
                None => state == Some(s.as_str()),
            })
    })
}

/// Balise de [`NESTED_TAGS`], avec au plus un état.
fn nested_tag(s: &str) -> bool {
    let base = STATE_SUFFIXES
        .iter()
        .find_map(|st| s.strip_suffix(st))
        .unwrap_or(s);
    NESTED_TAGS.contains(&base)
}

/// Le panneau Styles sait régler ce sélecteur (voir l'en-tête du module).
pub fn panel_selector(sel: &str, roots: &[(String, Vec<String>)]) -> bool {
    let parts: Vec<&str> = sel.split_whitespace().collect();
    let plain = |p: &str| compound(p).is_some_and(|(_, st)| st.is_none());
    match parts.as_slice() {
        [one] => compound(one).is_some(),
        [outer, x] if OUTER_STATES.contains(outer) => plain(x),
        [root, x] if variant_root(root, roots) => compound(x).is_some(),
        // Balise dans une classe (texte riche, mots mis en valeur).
        [cls, tag] => {
            compound(cls).is_some_and(|(c, st)| !c.is_empty() && st.is_none()) && nested_tag(tag)
        }
        [root, outer, x] => variant_root(root, roots) && OUTER_STATES.contains(outer) && plain(x),
        _ => false,
    }
}

/// `*, *::before, *::after { box-sizing: border-box }` : fondation admise.
fn is_foundation(r: &StyleRule) -> bool {
    let sel: String = r.selector.split_whitespace().collect();
    matches!(sel.as_str(), "*,*::before,*::after" | "*")
        && r.media.is_none()
        && !r.decls.is_empty()
        && r.decls
            .iter()
            .all(|d| d.prop == "box-sizing" && d.value.trim() == "border-box")
}

/// `var(--nom` lus dans une valeur.
fn var_refs(v: &str) -> Vec<&str> {
    v.match_indices("var(--")
        .map(|(i, _)| {
            let rest = &v[i + 6..];
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
                .unwrap_or(rest.len());
            &rest[..end]
        })
        .collect()
}

/// Variables appelées avec une valeur de secours (`var(--bloc-accent, #2f6fed)`) : nom et valeur.
pub fn var_fallbacks(v: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, _) in v.match_indices("var(--") {
        let rest = &v[i + 6..];
        let Some(comma) = rest.find(',') else {
            continue;
        };
        let name = &rest[..comma];
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            continue;
        }
        // Valeur jusqu'à la parenthèse fermante du `var(` (parenthèses de `rgba()` comprises).
        let mut depth = 0usize;
        let after = &rest[comma + 1..];
        let end = after.char_indices().find_map(|(j, c)| match c {
            '(' => {
                depth += 1;
                None
            }
            ')' if depth == 0 => Some(j),
            ')' => {
                depth -= 1;
                None
            }
            _ => None,
        });
        if let Some(end) = end {
            out.push((name.to_string(), after[..end].trim().to_string()));
        }
    }
    out
}

/// Relevé des styles du site.
pub fn audit(site: &Site) -> Vec<Issue> {
    let mut out = Vec::new();
    let Some(sheet) = &site.stylesheet else {
        return out;
    };
    let roots = variant_roots(site);
    let vars = site_variables(site);
    let bps = if site.breakpoints.is_empty() {
        Breakpoint::defaults()
    } else {
        site.breakpoints.clone()
    };
    let medias: Vec<String> = bps.iter().filter_map(Breakpoint::media).collect();
    let push = |out: &mut Vec<Issue>, r: Option<&StyleRule>, kind: Kind, detail: String| {
        out.push(Issue {
            owner: "styles".into(),
            node: r.and_then(|r| r.id.clone()),
            kind,
            detail,
        })
    };
    let label = |r: &StyleRule| match &r.media {
        Some(m) => format!("@media {m} {{ {} }}", r.selector),
        None => r.selector.clone(),
    };
    for item in &sheet.items {
        let r = match item {
            StyleItem::Rule(r) => r,
            StyleItem::Raw { css } => {
                if !css.trim_start().starts_with("@font-face") {
                    let head: String = css.trim().chars().take(60).collect();
                    push(&mut out, None, Kind::StyleCode, head);
                }
                continue;
            }
            StyleItem::Comment { .. } => continue,
        };
        // Fondation : bordures comprises dans les tailles de tout élément (comme la feuille de
        // base de Webflow), seule règle universelle admise.
        if is_foundation(r) {
            continue;
        }
        if r.selector == ":root" {
            let dark = r.media.as_deref() == Some(DARK);
            if r.media.is_some() && !dark {
                push(&mut out, Some(r), Kind::StyleMedia, label(r));
            }
            for d in &r.decls {
                let ok = d.prop.starts_with("--")
                    && if dark {
                        vars.contains(d.prop.trim_start_matches("--"))
                            && revel_style::values::color::Rgba::parse(&d.value).is_some()
                    } else {
                        site_value(&d.value)
                    };
                if !ok {
                    push(
                        &mut out,
                        Some(r),
                        Kind::StyleVariable,
                        format!("{} {{ {}: {} }}", label(r), d.prop, d.value),
                    );
                }
            }
            continue;
        }
        if let Some(m) = &r.media
            && !medias.contains(m)
        {
            push(&mut out, Some(r), Kind::StyleMedia, label(r));
        }
        if !panel_selector(&r.selector, &roots) {
            push(&mut out, Some(r), Kind::StyleSelector, label(r));
        }
        for d in &r.decls {
            let unknown: Vec<&str> = var_refs(&d.value)
                .into_iter()
                .filter(|v| !vars.contains(*v))
                .collect();
            // Propriétés des outils de Revel (taille ajustée à l'écran : unité « écran »).
            if REVEL_PROPS.contains(&d.prop.as_str()) {
                continue;
            }
            if d.prop.starts_with("--") || !unknown.is_empty() {
                push(
                    &mut out,
                    Some(r),
                    Kind::StyleVariable,
                    format!("{} {{ {}: {} }}", label(r), d.prop, d.value),
                );
            } else if !PANEL_PROPS.contains(&d.prop.as_str()) {
                push(
                    &mut out,
                    Some(r),
                    Kind::StyleProperty,
                    format!("{} {{ {}: {} }}", label(r), d.prop, d.value),
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valeurs_de_secours() {
        assert_eq!(
            var_fallbacks("var(--bloc-fond, rgba(127, 127, 127, 0.12)) var(--a) var(--b,1px)"),
            vec![
                (
                    "bloc-fond".to_string(),
                    "rgba(127, 127, 127, 0.12)".to_string()
                ),
                ("b".to_string(), "1px".to_string())
            ]
        );
    }

    #[test]
    fn selecteurs_du_panneau() {
        let roots = vec![(
            "carte".to_string(),
            vec![":hover".to_string(), ".is-sombre".to_string()],
        )];
        for ok in [
            ".titre",
            ".titre.is-grand",
            "h1",
            "body",
            ".bouton:hover",
            "a[aria-current=\"page\"]",
            ".is-active .onglet__texte",
            "[open] .faq__icone",
            ".carte:hover .titre",
            ".carte.is-sombre .titre:hover",
            ".carte:hover",
            ".carte:hover .is-active .x",
            ".prose p",
            ".prose a:hover",
            ".hero__title strong",
        ] {
            assert!(panel_selector(ok, &roots), "{ok}");
        }
        for ko in [
            "h1, h2",
            ".a > .b",
            ".a .b",
            ".prose div",
            ".a:hover p",
            ".a::before",
            "a.bouton",
            ".x:not(.y)",
            "*",
            ".autre:hover .titre",
            ".carte .titre",
            "div",
        ] {
            assert!(!panel_selector(ko, &roots), "{ko}");
        }
    }

    #[test]
    fn variables_et_code() {
        let css = ":root { --texte: #111; --marge: clamp(1rem, 2vw, 2rem); --courbe: cubic-bezier(0.2, 0.7, 0.1, 1); }
@media (prefers-color-scheme: dark) { :root { --texte: #eee; } .carte { color: red; } }
@keyframes tourne { to { transform: rotate(1turn); } }
@font-face { font-family: X; src: url(x.woff2); }
.carte { --dot: red; color: var(--texte); padding: var(--marge); transition: color 0.3s var(--courbe); isolation: isolate; }
@media (max-width: 991px) { .carte { color: blue; } }
@media (max-width: 500px) { .carte { color: blue; } }";
        let mut site = Site::new("x", "fr");
        site.stylesheet = Some(revel_model::Stylesheet {
            url: "/s.css".into(),
            source: None,
            items: revel_style::parse(css).unwrap(),
        });
        let issues = audit(&site);
        let kinds = |k: Kind| issues.iter().filter(|i| i.kind == k).count();
        assert_eq!(kinds(Kind::StyleCode), 1, "{issues:#?}");
        // --courbe (ni couleur ni taille), --dot, var(--courbe).
        assert_eq!(kinds(Kind::StyleVariable), 3, "{issues:#?}");
        assert_eq!(kinds(Kind::StyleProperty), 1, "{issues:#?}");
        // Règle de classe dans le mode sombre, point de rupture inconnu.
        assert_eq!(kinds(Kind::StyleMedia), 2, "{issues:#?}");
    }
}
