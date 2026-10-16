//! Commandes sur la feuille de style du modèle (D-18) : règles et déclarations.
//!
//! Les règles sont désignées par leur identifiant (`StyleRule::id`), les positions sont celles de
//! `Stylesheet::items` (règles, commentaires et blocs bruts confondus). Les valeurs sont vérifiées
//! avec le lecteur CSS de `revel-style` : une valeur qui fermerait la règle (`}`), ajouterait une
//! déclaration (`;` hors chaîne ou parenthèses) ou contiendrait `!important` est refusée.

use revel_model::{Decl, Site, StyleItem, StyleRule};

use crate::EditError;

fn items_mut(site: &mut Site) -> Result<&mut Vec<StyleItem>, EditError> {
    site.stylesheet
        .as_mut()
        .map(|s| &mut s.items)
        .ok_or(EditError::NoStylesheet)
}

pub(crate) fn items(site: &Site) -> Result<&Vec<StyleItem>, EditError> {
    site.stylesheet
        .as_ref()
        .map(|s| &s.items)
        .ok_or(EditError::NoStylesheet)
}

/// Position d'une règle dans la feuille.
pub fn rule_index(site: &Site, id: &str) -> Result<usize, EditError> {
    items(site)?
        .iter()
        .position(|i| matches!(i, StyleItem::Rule(r) if r.id.as_deref() == Some(id)))
        .ok_or_else(|| EditError::NotFound(format!("règle {id}")))
}

pub(crate) fn rule_mut<'a>(site: &'a mut Site, id: &str) -> Result<&'a mut StyleRule, EditError> {
    let i = rule_index(site, id)?;
    match &mut items_mut(site)?[i] {
        StyleItem::Rule(r) => Ok(r),
        _ => Err(EditError::NotFound(format!("règle {id}"))),
    }
}

/// Nom de propriété : un seul mot, sans caractère de structure CSS. Les noms hérités de CSS
/// anciens (`*zoom`, `_height`) restent acceptés pour que toute déclaration importée puisse être
/// retirée puis remise.
fn valid_prop(p: &str) -> bool {
    !p.is_empty()
        && !p
            .chars()
            .any(|c| c.is_whitespace() || ":;{}()[]\"'/\\!,".contains(c))
}

/// Déclaration vérifiée : la valeur relue par le lecteur CSS est une seule déclaration, sans
/// `!important` écrit dans la valeur. Renvoie la valeur normalisée (espaces réduits). Une valeur
/// vide est acceptée ici (déclaration importée telle quelle) ; `SetDecl` la refuse.
pub fn checked_decl(prop: &str, value: &str, important: bool) -> Result<Decl, EditError> {
    let bad = || EditError::InvalidCss(format!("{prop}: {value}"));
    if !valid_prop(prop) {
        return Err(bad());
    }
    let parsed = revel_style::parse(&format!("x{{{prop}: {value}}}")).map_err(|_| bad())?;
    match parsed.as_slice() {
        [StyleItem::Rule(r)] => match r.decls.as_slice() {
            [d] if d.prop == prop && !d.important => Ok(Decl {
                prop: prop.to_string(),
                value: d.value.clone(),
                important,
            }),
            _ => Err(bad()),
        },
        _ => Err(bad()),
    }
}

/// Sélecteur vérifié (une seule règle, pas de règle `@`) ; renvoyé normalisé.
pub fn checked_selector(selector: &str) -> Result<String, EditError> {
    let bad = || EditError::InvalidCss(selector.to_string());
    if selector.trim_start().starts_with('@') {
        return Err(bad());
    }
    match revel_style::parse(&format!("{selector} {{}}"))
        .map_err(|_| bad())?
        .as_slice()
    {
        [StyleItem::Rule(r)] if !r.selector.is_empty() && r.media.is_none() => {
            Ok(r.selector.clone())
        }
        _ => Err(bad()),
    }
}

/// Requête média vérifiée ; renvoyée normalisée.
pub fn checked_media(media: &str) -> Result<String, EditError> {
    let bad = || EditError::InvalidCss(format!("@media {media}"));
    match revel_style::parse(&format!("@media {media} {{x {{}}}}"))
        .map_err(|_| bad())?
        .as_slice()
    {
        [StyleItem::Rule(r)] => r.media.clone().filter(|m| !m.is_empty()).ok_or_else(bad),
        _ => Err(bad()),
    }
}

/// Règle vérifiée avant insertion (sélecteur, requête, déclarations, identifiant libre).
pub(crate) fn checked_rule(site: &Site, rule: &StyleRule) -> Result<StyleRule, EditError> {
    let id = rule
        .id
        .clone()
        .ok_or_else(|| EditError::InvalidCss(format!("{} (sans identifiant)", rule.selector)))?;
    if site.node_ids().contains(&id) {
        return Err(EditError::DuplicateId(id));
    }
    Ok(StyleRule {
        id: Some(id),
        selector: checked_selector(&rule.selector)?,
        media: rule.media.as_deref().map(checked_media).transpose()?,
        decls: rule
            .decls
            .iter()
            .map(|d| checked_decl(&d.prop, &d.value, d.important))
            .collect::<Result<_, _>>()?,
    })
}

/// Élément de feuille vérifié avant insertion.
pub(crate) fn checked_item(site: &Site, item: &StyleItem) -> Result<StyleItem, EditError> {
    match item {
        StyleItem::Rule(r) => Ok(StyleItem::Rule(checked_rule(site, r)?)),
        StyleItem::Comment { text, media } => {
            if text.contains("*/") {
                return Err(EditError::InvalidCss(text.clone()));
            }
            Ok(StyleItem::Comment {
                text: text.clone(),
                media: media.as_deref().map(checked_media).transpose()?,
            })
        }
        StyleItem::Raw { css } => match revel_style::parse(css)
            .map_err(|_| EditError::InvalidCss(css.clone()))?
            .as_slice()
        {
            // Un bloc brut reste un seul bloc `@…` (ou une règle imbriquée), relu à l'identique.
            [StyleItem::Raw { css: c }] if c == css.trim() => Ok(item.clone()),
            _ => Err(EditError::InvalidCss(css.clone())),
        },
    }
}

pub(crate) fn insert_item(site: &mut Site, index: usize, item: StyleItem) -> Result<(), EditError> {
    let items = items_mut(site)?;
    if index > items.len() {
        return Err(EditError::Index(index));
    }
    items.insert(index, item);
    Ok(())
}

pub(crate) fn remove_item(site: &mut Site, index: usize) -> Result<StyleItem, EditError> {
    let items = items_mut(site)?;
    if index >= items.len() {
        return Err(EditError::Index(index));
    }
    Ok(items.remove(index))
}

/// Applique une modification à une copie des déclarations, comme `SetDecl` ; `false` si elle
/// ne change rien (retrait d'une propriété absente).
fn simulate(decls: &mut Vec<Decl>, e: &revel_style::values::Edit) -> bool {
    let pos = decls.iter().rposition(|d| d.prop == e.prop);
    match (pos, &e.value) {
        (Some(i), Some(v)) => {
            decls[i] = Decl {
                prop: e.prop.clone(),
                value: v.clone(),
                important: e.important,
            };
            true
        }
        (Some(i), None) => {
            decls.remove(i);
            true
        }
        (None, Some(v)) => {
            decls.push(Decl {
                prop: e.prop.clone(),
                value: v.clone(),
                important: e.important,
            });
            true
        }
        (None, None) => false,
    }
}

/// Dernière règle d'un sélecteur complet (classe et état) pour un média.
pub fn find_rule(site: &Site, selector: &str, media: &Option<String>) -> Option<StyleRule> {
    site.stylesheet
        .as_ref()?
        .items
        .iter()
        .rev()
        .find_map(|i| match i {
            StyleItem::Rule(r) if r.selector == selector && &r.media == media => Some(r.clone()),
            _ => None,
        })
}

/// Commande qui règle des propriétés (`None` ou vide : retirée) dans la règle `rule` (sélecteur
/// complet `selector`, média `media`), créée si elle n'existe pas ; raccourcis (`margin`…)
/// respectés. `None` si rien ne change.
pub fn style_command(
    rule: Option<StyleRule>,
    selector: String,
    media: Option<String>,
    list: Vec<(String, Option<String>)>,
) -> Option<crate::Command> {
    use crate::Command;
    let before = rule.as_ref().map(|r| r.decls.clone()).unwrap_or_default();
    let mut decls = before.clone();
    let mut out: Vec<revel_style::values::Edit> = Vec::new();
    for (prop, value) in list {
        let value = value
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let Some(es) = revel_style::values::edits(&decls, &prop, value.as_deref()) else {
            continue;
        };
        for e in es {
            if simulate(&mut decls, &e) {
                out.push(e);
            }
        }
    }
    if decls == before {
        return None;
    }
    Some(match rule {
        Some(r) => {
            let id = r.id.clone().unwrap_or_default();
            let mut cmds: Vec<Command> = out
                .into_iter()
                .map(|e| Command::SetDecl {
                    rule: id.clone(),
                    prop: e.prop,
                    value: e.value,
                    important: e.important,
                })
                .collect();
            if cmds.len() == 1 {
                cmds.remove(0)
            } else {
                Command::Batch { commands: cmds }
            }
        }
        None => Command::AddRule {
            selector,
            media,
            decls,
        },
    })
}
