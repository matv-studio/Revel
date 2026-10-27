//! Variables du site (propriétés CSS personnalisées de `:root` : couleurs, tailles, espaces) :
//! renommées partout (règles, valeurs qui les lisent, styles écrits sur les éléments).

use crate::EditError;
use revel_model::{Node, Site, StyleItem};

/// Nom de variable accepté (sans `--`) : lettre, puis lettres, chiffres, tirets, soulignés.
pub fn valid_name(n: &str) -> bool {
    let mut c = n.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic())
        && c.all(|x| x.is_ascii_alphanumeric() || x == '-' || x == '_')
}

fn ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii()
}

/// `--from` remplacé par `--to` là où il est un nom entier (pas `--from-2`).
pub fn replace_var(text: &str, from: &str, to: &str) -> String {
    let needle = format!("--{from}");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(&needle) {
        let before = rest[..i].chars().next_back();
        let after = rest[i + needle.len()..].chars().next();
        out.push_str(&rest[..i]);
        if before.is_none_or(|c| !ident(c)) && after.is_none_or(|c| !ident(c)) {
            out.push_str("--");
            out.push_str(to);
        } else {
            out.push_str(&needle);
        }
        rest = &rest[i + needle.len()..];
    }
    out.push_str(rest);
    out
}

/// Variables déclarées par le site (noms sans `--`).
pub fn declared(site: &Site) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for r in site.style_rules() {
        for d in &r.decls {
            if let Some(n) = d.prop.strip_prefix("--")
                && !out.iter().any(|m| m == n)
            {
                out.push(n.to_string());
            }
        }
    }
    out
}

/// Renomme la variable `from` en `to` partout ; refusé si `to` est invalide ou déjà déclarée.
pub(crate) fn rename(site: &mut Site, from: &str, to: &str) -> Result<usize, EditError> {
    let (from, to) = (from.trim_start_matches("--"), to.trim_start_matches("--"));
    if !valid_name(to) || from == to {
        return Err(EditError::InvalidProp(to.to_string()));
    }
    let names = declared(site);
    if !names.iter().any(|n| n == from) {
        return Err(EditError::NotFound(format!("--{from}")));
    }
    if names.iter().any(|n| n == to) {
        return Err(EditError::InUse(format!("--{to}")));
    }
    let mut changed = 0;
    if let Some(sheet) = site.stylesheet.as_mut() {
        for item in &mut sheet.items {
            match item {
                StyleItem::Rule(r) => {
                    for d in &mut r.decls {
                        let p = replace_var(&d.prop, from, to);
                        let v = replace_var(&d.value, from, to);
                        if p != d.prop || v != d.value {
                            d.prop = p;
                            d.value = v;
                            changed += 1;
                        }
                    }
                }
                StyleItem::Raw { css } => {
                    let n = replace_var(css, from, to);
                    if n != *css {
                        *css = n;
                        changed += 1;
                    }
                }
                StyleItem::Comment { .. } => {}
            }
        }
    }
    fn walk(nodes: &mut [Node], from: &str, to: &str, changed: &mut usize) {
        for n in nodes {
            if let Node::Element { attrs, .. } | Node::Use { attrs, .. } = n {
                for a in attrs.iter_mut() {
                    if let Some(v) = &a.value
                        && v.contains("--")
                    {
                        let nv = replace_var(v, from, to);
                        if nv != *v {
                            a.value = Some(nv);
                            *changed += 1;
                        }
                    }
                }
            }
            for list in revel_model::ids::child_lists_mut(n) {
                walk(list, from, to, changed);
            }
        }
    }
    for p in &mut site.pages {
        walk(&mut p.body, from, to, &mut changed);
    }
    for l in &mut site.layouts {
        walk(&mut l.body, from, to, &mut changed);
    }
    for c in &mut site.components {
        walk(&mut c.body, from, to, &mut changed);
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_entiers_seulement() {
        assert_eq!(
            replace_var(
                "var(--step-1) var(--step-10) var(--step-1,2px) --step-1:",
                "step-1",
                "texte-l"
            ),
            "var(--texte-l) var(--step-10) var(--texte-l,2px) --texte-l:"
        );
        assert!(valid_name("titre-xl") && !valid_name("1a") && !valid_name("a b"));
    }
}
