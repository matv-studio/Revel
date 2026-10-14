//! `revel-style` : feuille de style du modèle (D-18).
//!
//! - [`parse`] lit un CSS existant en règles du modèle (texte d'origine conservé) ;
//! - [`generate`] écrit le CSS publié, dans l'ordre du modèle ;
//! - [`canonical`] donne une forme normalisée d'un CSS (sans commentaires, espaces réduits,
//!   blocs `@media` identiques consécutifs fusionnés) pour vérifier l'équivalence ;
//! - [`breakpoints_from`] déduit les points de rupture des requêtes média ;
//! - [`insertion_index`] place une nouvelle règle dans la cascade (comme Webflow).

use revel_model::{Breakpoint, StyleItem, StyleRule, Stylesheet};

mod parse;

pub use parse::{ParseError, collapse_ws, parse};

/// Écrit une déclaration.
fn decl(d: &revel_model::Decl) -> String {
    if d.important {
        format!("{}: {} !important", d.prop, d.value)
    } else {
        format!("{}: {}", d.prop, d.value)
    }
}

fn rule_line(r: &StyleRule) -> String {
    if r.decls.is_empty() {
        return format!("{} {{}}", r.selector);
    }
    let body: Vec<String> = r.decls.iter().map(decl).collect();
    format!("{} {{ {}; }}", r.selector, body.join("; "))
}

fn media_of(i: &StyleItem) -> Option<&str> {
    match i {
        StyleItem::Rule(r) => r.media.as_deref(),
        StyleItem::Comment { media, .. } => media.as_deref(),
        StyleItem::Raw { .. } => None,
    }
}

/// CSS publié : une ligne par règle ; règles consécutives d'une même requête média regroupées.
pub fn generate(items: &[StyleItem]) -> String {
    let mut out = String::new();
    let mut open: Option<&str> = None;
    for item in items {
        let m = media_of(item);
        if m != open {
            if open.is_some() {
                out.push_str("}\n");
            }
            if let Some(q) = m {
                out.push_str("@media ");
                out.push_str(q);
                out.push_str(" {\n");
            }
            open = m;
        }
        let indent = if open.is_some() { "  " } else { "" };
        match item {
            StyleItem::Rule(r) => {
                out.push_str(indent);
                out.push_str(&rule_line(r));
                out.push('\n');
            }
            StyleItem::Comment { text, .. } => {
                out.push_str(indent);
                out.push_str("/* ");
                out.push_str(text);
                out.push_str(" */\n");
            }
            StyleItem::Raw { css } => {
                out.push_str(css);
                out.push('\n');
            }
        }
    }
    if open.is_some() {
        out.push_str("}\n");
    }
    out
}

/// CSS d'une feuille du modèle.
pub fn render(sheet: &Stylesheet) -> String {
    generate(&sheet.items)
}

/// Forme canonique d'un CSS : sert à vérifier que deux feuilles sont équivalentes (même suite de
/// règles, mêmes valeurs), indépendamment de la mise en forme et des commentaires.
pub fn canonical(css: &str) -> Result<String, ParseError> {
    let items: Vec<StyleItem> = parse(css)?
        .into_iter()
        .filter(|i| !matches!(i, StyleItem::Comment { .. }))
        .map(|i| match i {
            StyleItem::Raw { css } => StyleItem::Raw {
                css: collapse_ws(&css),
            },
            other => other,
        })
        .collect();
    Ok(generate(&items))
}

/// Points de rupture déduits des requêtes média simples (`(max-width: Npx)`, `(min-width: Npx)`) :
/// la base, puis les largeurs maximales décroissantes, puis les largeurs minimales croissantes.
pub fn breakpoints_from(items: &[StyleItem]) -> Vec<Breakpoint> {
    let mut max: Vec<u32> = Vec::new();
    let mut min: Vec<u32> = Vec::new();
    for i in items {
        if let StyleItem::Rule(r) = i
            && let Some(m) = &r.media
        {
            match parse_width(m) {
                Some(Width::Max(w)) if !max.contains(&w) => max.push(w),
                Some(Width::Min(w)) if !min.contains(&w) => min.push(w),
                _ => {}
            }
        }
    }
    max.sort_unstable_by(|a, b| b.cmp(a));
    min.sort_unstable();
    // Une largeur minimale qui est le complément exact d'une largeur maximale (861 pour 860)
    // n'est pas un point de rupture à part : c'est « au-dessus de 860 ».
    min.retain(|w| !max.contains(&(w.saturating_sub(1))));
    let mut v = vec![Breakpoint {
        id: "desktop".into(),
        max_width: None,
        min_width: None,
    }];
    v.extend(max.into_iter().map(|w| Breakpoint {
        id: format!("max-{w}"),
        max_width: Some(w),
        min_width: None,
    }));
    v.extend(min.into_iter().map(|w| Breakpoint {
        id: format!("min-{w}"),
        max_width: None,
        min_width: Some(w),
    }));
    v
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    Max(u32),
    Min(u32),
}

/// `(max-width: 860px)` -> `Max(860)` ; autre requête -> `None`.
pub fn parse_width(media: &str) -> Option<Width> {
    let m = collapse_ws(media).to_ascii_lowercase();
    let inner = m.strip_prefix('(')?.strip_suffix(')')?;
    let (k, v) = inner.split_once(':')?;
    let px: u32 = v.trim().strip_suffix("px")?.trim().parse().ok()?;
    match k.trim() {
        "max-width" => Some(Width::Max(px)),
        "min-width" => Some(Width::Min(px)),
        _ => None,
    }
}

/// Position d'une nouvelle règle `selector` pour la requête `media`, selon la cascade de Webflow :
/// les styles d'un point de rupture l'emportent sur ceux des écrans plus larges, quel que soit le
/// sélecteur (D-18).
///
/// - après la dernière règle du même sélecteur pour des écrans plus larges ou égaux (base comprise) ;
/// - avant la première règle suivante du même sélecteur pour des écrans plus étroits (et avant les
///   commentaires qui la précèdent) ;
/// - entre les deux : à la fin du dernier groupe de règles de la même requête, sinon le plus loin
///   possible (en fin de feuille s'il n'y a pas de règle plus étroite du même sélecteur), pour
///   qu'une règle de base écrite plus loin dans le fichier ne l'emporte pas sur elle.
pub fn insertion_index(items: &[StyleItem], selector: &str, media: Option<&str>) -> usize {
    let rank = |m: Option<&str>| -> i64 {
        match m.and_then(parse_width) {
            None if m.is_none() => i64::MIN, // base
            Some(Width::Min(w)) => i64::MIN + 1 + i64::from(w),
            Some(Width::Max(w)) => i64::from(u32::MAX - w),
            None => 0, // autre requête : au milieu
        }
    };
    let me = rank(media);
    let same = |i: &StyleItem| match i {
        StyleItem::Rule(r) if r.selector == selector => Some(rank(r.media.as_deref())),
        _ => None,
    };
    let start = items
        .iter()
        .rposition(|i| same(i).is_some_and(|r| r <= me))
        .map_or(0, |a| a + 1);
    let mut limit = items[start..]
        .iter()
        .position(|i| same(i).is_some_and(|r| r > me))
        .map_or(items.len(), |b| start + b);
    if limit < items.len() {
        while limit > start && matches!(items[limit - 1], StyleItem::Comment { .. }) {
            limit -= 1;
        }
    }
    items[start..limit]
        .iter()
        .rposition(|i| matches!(i, StyleItem::Rule(r) if r.media.as_deref() == media))
        .map_or(limit, |j| start + j + 1)
}

#[cfg(test)]
mod tests;
