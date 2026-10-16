//! Réglages lisibles (3.12) : valeurs des données reprises par les gabarits (listes de contenus
//! mis en avant, image de partage, adresse officielle), leur libellé dans Pages CMS et contenus
//! proposés. La reconnaissance des calculs est faite dans `revel_edit::readable`.

use crate::{Core, Result};
use revel_edit::canvas::{Binding, Seg, Source};
use revel_project::Project;
use revel_protocol::{PickEntry, ReadableData};
use serde_json::Value;
use std::path::Path;

/// Valeur des données à un chemin (`["pages", "accueil", "featured"]`).
fn value_at(data: &std::collections::BTreeMap<String, Value>, path: &[String]) -> Option<Value> {
    let (first, rest) = path.split_first()?;
    let mut cur = data.get(first)?;
    for k in rest {
        // Élément d'une liste par son numéro (`nav.0.label`).
        cur = match (cur, k.parse::<usize>()) {
            (Value::Array(items), Ok(i)) => items.get(i)?,
            _ => cur.get(k)?,
        };
    }
    Some(cur.clone())
}

/// Libellé d'un chemin de données dans `.pages.yml` (fichier décrit par une entrée `type: file`,
/// champ suivi dans ses sous-champs).
fn label_of(p: &Project, pages: &Value, path: &[String]) -> Option<String> {
    let (file, inner, _) = p
        .source_file(&Binding {
            source: Source::Data,
            path: path.iter().map(|k| Seg::Key(k.clone())).collect(),
        })
        .ok()?;
    let rel = file
        .strip_prefix(&p.dir)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    let item = pages.get("content")?.as_array()?.iter().find(|c| {
        c.get("type").and_then(Value::as_str) == Some("file")
            && c.get("path")
                .and_then(Value::as_str)
                .is_some_and(|x| crate::pagescms::norm_path(x) == crate::pagescms::norm_path(&rel))
    })?;
    let mut fields = item.get("fields")?.as_array()?.clone();
    let mut label = None;
    for seg in &inner {
        let Seg::Key(k) = seg else { return None };
        let f = fields
            .iter()
            .find(|f| f.get("name").and_then(Value::as_str) == Some(k.as_str()))?
            .clone();
        label = f.get("label").and_then(Value::as_str).map(str::to_string);
        fields = f
            .get("fields")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
    }
    label
}

impl Core {
    /// Données des réglages lisibles d'un site ouvert : valeurs et libellés des chemins,
    /// contenus des collections (identifiant `slug`, titre).
    pub fn readable_data(
        &self,
        path: &Path,
        paths: &[Vec<String>],
        collections: &[String],
    ) -> Result<ReadableData> {
        self.with_project(path, |p| {
            let data = p.load_data()?;
            let pages = std::fs::read_to_string(p.dir.join(".pages.yml"))
                .ok()
                .and_then(|t| revel_project::parse_yaml(&t).ok())
                .unwrap_or(Value::Null);
            let entries = if collections.is_empty() {
                Vec::new()
            } else {
                let all = p.load_entries()?;
                let mut out = Vec::new();
                for c in collections {
                    let Some(folder) = p
                        .site
                        .collections
                        .iter()
                        .find(|x| x.id == *c)
                        .map(|x| x.folder.clone())
                    else {
                        continue;
                    };
                    for e in all.iter().filter(|e| e.folder == folder) {
                        let Some(slug) = e.data.get("slug").and_then(Value::as_str) else {
                            continue;
                        };
                        let title = e
                            .data
                            .get("title")
                            .and_then(Value::as_str)
                            .unwrap_or(&e.file_stem)
                            .to_string();
                        out.push(PickEntry {
                            collection: c.clone(),
                            slug: slug.to_string(),
                            title,
                        });
                    }
                }
                out
            };
            Ok(ReadableData {
                values: paths.iter().map(|q| value_at(&data, q)).collect(),
                labels: paths.iter().map(|q| label_of(p, &pages, q)).collect(),
                entries,
            })
        })
    }
}
