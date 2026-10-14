//! Migrations du schéma de document.
//!
//! Règle : on n'édite jamais une migration publiée. Pour changer le schéma,
//! incrémenter [`crate::SCHEMA_VERSION`] et ajouter une fonction à [`STEPS`].

use crate::{ModelError, SCHEMA_VERSION};
use serde_json::Value;

type Step = fn(Value) -> Result<Value, ModelError>;

/// `STEPS[i]` fait passer un document de la version `i` à la version `i + 1`.
/// La version 0 correspond aux documents expérimentaux sans `schema_version`.
const STEPS: &[Step] = &[v0_to_v1, v1_to_v2, v2_to_v3];

/// Migre un document JSON brut jusqu'à la version courante.
pub fn migrate(mut doc: Value) -> Result<Value, ModelError> {
    let mut version = match doc.get("schema_version") {
        None => 0,
        Some(v) => v
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .ok_or(ModelError::MissingVersion)?,
    };
    if version > SCHEMA_VERSION {
        return Err(ModelError::TooNew {
            found: version,
            supported: SCHEMA_VERSION,
        });
    }
    while version < SCHEMA_VERSION {
        doc = STEPS[version as usize](doc)?;
        version += 1;
        doc["schema_version"] = Value::from(version);
    }
    Ok(doc)
}

/// v0 -> v1 : ajout du champ `schema_version` (aucune autre modification).
fn v0_to_v1(doc: Value) -> Result<Value, ModelError> {
    Ok(doc)
}

/// v1 -> v2 : identifiants des nœuds structurels (`id` sur les conditions, boucles, composants
/// utilisés, blocs et images). Champs facultatifs : le document v1 reste valide tel quel ; les
/// identifiants manquants sont attribués à la lecture (`Site::ensure_ids`).
fn v1_to_v2(doc: Value) -> Result<Value, ModelError> {
    Ok(doc)
}

/// v2 -> v3 : feuille de style ordonnée (`stylesheet`) à la place des classes `styles`
/// (classe -> point de rupture -> propriétés), jamais remplies jusqu'ici. Une classe devient une
/// règle `.classe` par point de rupture : la base d'abord, puis du plus large au plus étroit.
fn v2_to_v3(mut doc: Value) -> Result<Value, ModelError> {
    let Some(Value::Object(styles)) = doc.as_object_mut().and_then(|o| o.remove("styles")) else {
        return Ok(doc);
    };
    if styles.is_empty() {
        return Ok(doc);
    }
    let widths: std::collections::BTreeMap<String, Option<u64>> = doc
        .get("breakpoints")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|b| {
                    Some((
                        b.get("id")?.as_str()?.to_string(),
                        b.get("max_width").and_then(Value::as_u64),
                    ))
                })
                .collect()
        })
        .unwrap_or_else(|| {
            [
                ("desktop", None),
                ("tablet", Some(991)),
                ("mobile", Some(767)),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
        });
    let mut rules: Vec<(Option<u64>, Value)> = Vec::new();
    for (class, def) in styles {
        if let Some(Value::Object(per_bp)) = def.get("rules") {
            for (bp, props) in per_bp {
                let w = widths.get(bp).copied().flatten();
                let decls: Vec<Value> = props
                    .as_object()
                    .map(|m| {
                        m.iter()
                            .map(|(k, v)| {
                                serde_json::json!({ "prop": k, "value": v.as_str().unwrap_or_default() })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let mut rule = serde_json::json!({
                    "item": "rule", "selector": format!(".{class}"), "decls": decls
                });
                if let Some(w) = w {
                    rule["media"] = Value::from(format!("(max-width: {w}px)"));
                }
                rules.push((w, rule));
            }
        }
    }
    // Base d'abord, puis largeurs décroissantes.
    rules.sort_by_key(|(w, _)| std::cmp::Reverse(w.unwrap_or(u64::MAX)));
    doc["stylesheet"] = serde_json::json!({
        "url": "/styles.css",
        "items": rules.into_iter().map(|(_, r)| r).collect::<Vec<_>>(),
    });
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chaque_version_a_sa_migration() {
        assert_eq!(STEPS.len(), SCHEMA_VERSION as usize);
    }

    #[test]
    fn document_sans_version_migre() {
        let out = migrate(json!({"name": "x", "default_locale": "fr"})).unwrap();
        assert_eq!(out["schema_version"], SCHEMA_VERSION);
    }

    #[test]
    fn classes_v2_converties_en_regles() {
        let doc = json!({
            "schema_version": 2, "name": "x", "default_locale": "fr",
            "styles": { "titre": { "rules": {
                "mobile": { "color": "red" },
                "desktop": { "color": "blue" }
            } } }
        });
        let out = migrate(doc).unwrap();
        assert!(out.get("styles").is_none());
        let items = out["stylesheet"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["selector"], ".titre");
        assert!(items[0].get("media").is_none(), "base d'abord");
        assert_eq!(items[1]["media"], "(max-width: 767px)");
        let site: crate::Site = serde_json::from_value(out).unwrap();
        assert_eq!(site.stylesheet.unwrap().items.len(), 2);
    }

    #[test]
    fn document_trop_recent_refuse() {
        let err = migrate(json!({"schema_version": SCHEMA_VERSION + 1})).unwrap_err();
        assert!(matches!(err, ModelError::TooNew { .. }));
    }
}
