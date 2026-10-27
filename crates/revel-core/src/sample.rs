//! Contenu d'exemple du modèle des pages d'une collection (gabarit commun, D-28) : une page
//! interne de Revel qui réunit tous les champs de la collection, remplis ou non dans ses
//! contenus, pour que le gabarit montre chaque partie qui dépend d'un champ (image, date,
//! condition…). Il n'existe que dans l'éditeur ([`revel_model::TEMPLATE_ENTRY`]).

use revel_protocol::{CmsEntry, CmsField};
use serde_json::{Map, Value};

/// Texte d'exemple.
const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";

/// Clés de l'en-tête qui ne décrivent pas le contenu mais sa publication : le contenu d'exemple
/// suit le modèle (pas de design propre) et il est publié.
const SKIPPED: [&str; 3] = ["layout", "draft", "permalink"];

/// Valeur vide (absente, texte vide, liste ou objet vide).
fn empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

/// Fichier image (adresse ou chemin).
fn is_image(v: &str) -> bool {
    let l = v.to_ascii_lowercase();
    let l = l.split(['?', '#']).next().unwrap_or("");
    [".jpg", ".jpeg", ".png", ".webp", ".avif", ".gif", ".svg"]
        .iter()
        .any(|e| l.ends_with(e))
}

/// Première image trouvée dans les contenus (toutes collections), pour un champ image vide.
fn any_image(all: &[revel_render::Entry]) -> Option<String> {
    fn walk(v: &Value) -> Option<String> {
        match v {
            Value::String(s) if is_image(s) => Some(s.clone()),
            Value::Array(a) => a.iter().find_map(walk),
            Value::Object(o) => o.values().find_map(walk),
            _ => None,
        }
    }
    all.iter().find_map(|e| walk(&e.data))
}

/// Valeur d'exemple d'un champ qu'aucun contenu ne remplit.
fn synthetic(f: &CmsField, image: Option<&str>, today: &str) -> Value {
    let one = match f.kind.as_deref().unwrap_or("string") {
        "text" | "rich_text" | "markdown" | "code" => Value::from(LOREM),
        "image" | "file" => Value::from(image.unwrap_or_default()),
        "date" | "datetime" => Value::from(today),
        "boolean" => Value::Bool(true),
        "number" => Value::from(1),
        "select" => f
            .options
            .get("values")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .map(|v| v.get("value").cloned().unwrap_or_else(|| v.clone()))
            .unwrap_or_else(|| Value::from("")),
        "object" | "block" => {
            let mut m = Map::new();
            for sub in &f.fields {
                m.insert(sub.name.clone(), synthetic(sub, image, today));
            }
            Value::Object(m)
        }
        "reference" => Value::from(""),
        _ => Value::from(
            f.label
                .clone()
                .filter(|l| !l.trim().is_empty())
                .unwrap_or_else(|| title_case(&f.name)),
        ),
    };
    if f.list && !empty(&one) {
        Value::Array(vec![one.clone(), one])
    } else if f.list {
        Value::Array(Vec::new())
    } else {
        one
    }
}

/// `cover_image` -> `Cover image`.
fn title_case(name: &str) -> String {
    let s = name.replace(['_', '-'], " ");
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// En-tête et corps du contenu d'exemple : pour chaque champ (schéma, puis clés des en-têtes),
/// la première valeur non vide parmi les contenus de la collection, sinon une valeur d'exemple
/// selon son type ; corps : le premier corps non vide, sinon un texte d'exemple.
pub(crate) fn template_sample(
    fields: &[CmsField],
    entries: &[CmsEntry],
    all: &[revel_render::Entry],
    today: &str,
) -> (Value, String) {
    let first = |name: &str| {
        // Case à cocher : vraie si un contenu l'a cochée (parties conditionnelles visibles).
        let vals: Vec<&Value> = entries.iter().filter_map(|e| e.data.get(name)).collect();
        if vals.iter().any(|v| v.is_boolean()) {
            return Some(Value::Bool(vals.iter().any(|v| v.as_bool() == Some(true))));
        }
        vals.into_iter().find(|v| !empty(v)).cloned()
    };
    let image = any_image(all);
    let mut data = Map::new();
    for f in fields {
        if SKIPPED.contains(&f.name.as_str()) {
            continue;
        }
        let v = first(&f.name).unwrap_or_else(|| synthetic(f, image.as_deref(), today));
        data.insert(f.name.clone(), v);
    }
    for e in entries {
        if let Value::Object(m) = &e.data {
            for k in m.keys() {
                if !data.contains_key(k)
                    && !SKIPPED.contains(&k.as_str())
                    && let Some(v) = first(k)
                {
                    data.insert(k.clone(), v);
                }
            }
        }
    }
    if data.contains_key("published") {
        data.insert("published".into(), Value::Bool(true));
    }
    let body = entries
        .iter()
        .map(|e| e.body.trim())
        .find(|b| !b.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{LOREM}\n\n{LOREM}\n"));
    (Value::Object(data), body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn field(name: &str, kind: &str) -> CmsField {
        CmsField {
            name: name.into(),
            kind: Some(kind.into()),
            ..Default::default()
        }
    }

    fn entry(data: Value, body: &str) -> CmsEntry {
        CmsEntry {
            file: "x".into(),
            data,
            body: body.into(),
            url: None,
        }
    }

    #[test]
    fn exemple_reunit_tous_les_champs() {
        let fields = vec![
            field("title", "string"),
            field("cover", "image"),
            field("intro", "text"),
            field("featured", "boolean"),
            field("date", "date"),
            field("layout", "string"),
            CmsField {
                name: "tags".into(),
                kind: Some("string".into()),
                list: true,
                ..Default::default()
            },
        ];
        let entries = vec![
            entry(
                json!({"title": "", "featured": false, "layout": "propre"}),
                "",
            ),
            entry(
                json!({"title": "Wattr", "featured": true, "extra": "x"}),
                "Corps.",
            ),
        ];
        let all = vec![revel_render::Entry {
            folder: "autre".into(),
            file_stem: "a".into(),
            data: json!({"img": "/img/a.jpg"}),
            body: String::new(),
        }];
        let (d, body) = template_sample(&fields, &entries, &all, "2026-10-27");
        assert_eq!(d["title"], "Wattr");
        assert_eq!(d["cover"], "/img/a.jpg");
        assert_eq!(d["intro"], LOREM);
        assert_eq!(d["featured"], true);
        assert_eq!(d["date"], "2026-10-27");
        assert_eq!(d["tags"], json!(["Tags", "Tags"]));
        assert_eq!(d["extra"], "x");
        assert!(d.get("layout").is_none(), "le modèle, pas un design propre");
        assert_eq!(body, "Corps.");
    }
}
