//! Migrations du schéma de document.
//!
//! Règle : on n'édite jamais une migration publiée. Pour changer le schéma,
//! incrémenter [`crate::SCHEMA_VERSION`] et ajouter une fonction à [`STEPS`].

use crate::{ModelError, SCHEMA_VERSION};
use serde_json::Value;

type Step = fn(Value) -> Result<Value, ModelError>;

/// `STEPS[i]` fait passer un document de la version `i` à la version `i + 1`.
/// La version 0 correspond aux documents expérimentaux sans `schema_version`.
const STEPS: &[Step] = &[v0_to_v1];

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
    fn document_trop_recent_refuse() {
        let err = migrate(json!({"schema_version": SCHEMA_VERSION + 1})).unwrap_err();
        assert!(matches!(err, ModelError::TooNew { .. }));
    }
}
