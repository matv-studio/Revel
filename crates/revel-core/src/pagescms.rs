//! Configuration de Pages CMS (`.pages.yml`, https://pagescms.org/docs/configuration/) : schéma des
//! collections de contenus (étape 3.3, D-27). Revel lit les champs déclarés — libellés, types,
//! aides, valeurs par défaut, champs cachés, choix, références, objets — et la présentation des
//! tableaux, pour que la vue CMS et Pages CMS décrivent les mêmes contenus. Une collection absente
//! du fichier y est décrite d'après ses contenus (`collection_def`) à la première modification de
//! ses champs ; le texte est modifié par `revel_project::pagesyml`.

use revel_protocol::{CmsField, CmsTableView};
use serde_json::{Map, Value};
use std::path::Path;

pub use revel_project::pagesyml::CONFIG_FILE;

/// Configuration lue.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PagesConfig {
    /// `settings.content.merge` : Pages CMS garde, en enregistrant un contenu, les valeurs absentes
    /// du schéma (sinon il les retire).
    pub merge: bool,
    pub collections: Vec<PagesCollection>,
}

/// Collection décrite par `.pages.yml`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PagesCollection {
    pub name: String,
    pub label: Option<String>,
    /// Dossier des contenus (`src/content/projets`).
    pub path: String,
    pub filename: Option<String>,
    pub view: Option<CmsTableView>,
    pub fields: Vec<CmsField>,
}

fn text(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_string)
}

/// Dossier sans `./` ni barre finale, pour comparer les chemins.
pub fn norm_path(p: &str) -> String {
    p.trim()
        .trim_start_matches("./")
        .trim_end_matches('/')
        .replace('\\', "/")
}

/// Champ de Pages CMS ; `component` remplacé par sa définition (les clés écrites sur le champ
/// l'emportent).
fn field(v: &Value, components: &Map<String, Value>, depth: usize) -> Option<CmsField> {
    let mut def = v.as_object()?.clone();
    let component = def
        .get("component")
        .and_then(Value::as_str)
        .map(str::to_string);
    if let Some(c) = &component
        && let Some(Value::Object(base)) = components.get(c)
    {
        let mut merged = base.clone();
        for (k, x) in def.iter().filter(|(k, _)| k.as_str() != "component") {
            merged.insert(k.clone(), x.clone());
        }
        def = merged;
    }
    let v = Value::Object(def);
    let name = text(&v, "name")?;
    let kind = text(&v, "type").map(|t| match t.as_str() {
        "rich-text" => "rich_text".to_string(),
        other => other.to_string(),
    });
    let fields = if depth < 4 {
        v.get("fields")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|f| field(f, components, depth + 1))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let mut options = v.get("options").cloned().unwrap_or(Value::Null);
    // Blocs : définitions gardées dans les options (éditeur de blocs à venir).
    if let Some(b) = v.get("blocks") {
        let mut o = options.as_object().cloned().unwrap_or_default();
        o.insert("blocks".into(), b.clone());
        if let Some(k) = v.get("blockKey") {
            o.insert("blockKey".into(), k.clone());
        }
        options = Value::Object(o);
    }
    Some(CmsField {
        label: text(&v, "label"),
        kind,
        description: text(&v, "description"),
        required: v.get("required").and_then(Value::as_bool).unwrap_or(false),
        hidden: v.get("hidden").and_then(Value::as_bool).unwrap_or(false),
        list: match v.get("list") {
            Some(Value::Bool(b)) => *b,
            Some(Value::Object(_)) => true,
            _ => false,
        },
        default: v.get("default").cloned(),
        options,
        pattern: match v.get("pattern") {
            Some(Value::String(p)) => Some(p.clone()),
            Some(Value::Object(o)) => o.get("regex").and_then(Value::as_str).map(str::to_string),
            _ => None,
        },
        fields,
        name,
        component,
    })
}

/// Présentation du tableau, comme Pages CMS : tri par défaut `view.default.sort` et
/// `view.default.order` (ou, écriture ancienne, `view.sort` et `view.order`), sinon par date
/// décroissante quand la collection a un champ `date` affiché.
fn table_view(w: Option<&Value>, fields: &[CmsField]) -> CmsTableView {
    let empty = Value::Null;
    let w = w.unwrap_or(&empty);
    let columns: Vec<String> = w
        .get("fields")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let sort = w
        .pointer("/default/sort")
        .and_then(Value::as_str)
        .or_else(|| w.get("sort").and_then(Value::as_str))
        .map(str::to_string);
    let order = w
        .pointer("/default/order")
        .and_then(Value::as_str)
        .or_else(|| w.get("order").and_then(Value::as_str));
    let date = fields.iter().any(|f| f.name == "date")
        && (columns.is_empty() || columns.iter().any(|c| c == "date"));
    let (sort, descending) = match sort {
        Some(s) => (
            Some(s.clone()),
            order == Some("desc") || (s == "date" && order.is_none()),
        ),
        None if date => (Some("date".into()), true),
        None => (None, false),
    };
    CmsTableView {
        primary: text(w, "primary"),
        fields: columns,
        sort,
        descending,
    }
}

/// Configuration décrite par le texte d'un `.pages.yml` (vide s'il est illisible).
pub fn parse(yaml: &str) -> PagesConfig {
    let Ok(v) = revel_project::parse_yaml(yaml) else {
        return PagesConfig::default();
    };
    let components = v
        .get("components")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let collections = v
        .get("content")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter(|c| c.get("type").and_then(Value::as_str) == Some("collection"))
                .filter_map(|c| {
                    let fields: Vec<CmsField> = c
                        .get("fields")
                        .and_then(Value::as_array)
                        .map(|a| a.iter().filter_map(|f| field(f, &components, 0)).collect())
                        .unwrap_or_default();
                    Some(PagesCollection {
                        name: text(c, "name")?,
                        label: text(c, "label"),
                        path: norm_path(&text(c, "path")?),
                        filename: text(c, "filename"),
                        view: Some(table_view(c.get("view"), &fields)),
                        fields,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    PagesConfig {
        merge: v
            .pointer("/settings/content/merge")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        collections,
    }
}

/// Configuration du `.pages.yml` d'un site (vide s'il n'existe pas).
pub fn load(site: &Path) -> PagesConfig {
    std::fs::read_to_string(site.join(CONFIG_FILE))
        .map(|t| parse(&t))
        .unwrap_or_default()
}

// ------------------------------------------------------------------ Schéma déduit des contenus

/// Variables techniques d'Eleventy : déclarées cachées (Pages CMS garde les champs cachés).
const TECHNICAL: &[&str] = &[
    "layout",
    "permalink",
    "eleventyExcludeFromCollections",
    "eleventyComputed",
    "eleventyNavigation",
    "eleventyImport",
];

pub(crate) fn humanize(name: &str) -> String {
    let s = name.replace(['_', '-'], " ");
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 10
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[7] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit)
}

/// Type et options d'une valeur de texte, au format de Pages CMS.
fn string_kind(s: &str) -> (&'static str, Option<Value>) {
    let t = s.trim();
    let digits = |r: std::ops::Range<usize>| {
        t.get(r)
            .is_some_and(|x| x.bytes().all(|b| b.is_ascii_digit()))
    };
    if is_iso_date(t) {
        // Format d'écriture des valeurs existantes (Pages CMS relit avec ce format).
        let time = t.len() >= 16
            && matches!(t.as_bytes()[10], b'T')
            && t.as_bytes()[13] == b':'
            && digits(11..13)
            && digits(14..16);
        return match t.len() {
            10 => ("date", None),
            16 if time => ("date", Some(serde_json::json!({ "time": true }))),
            19 if time && t.as_bytes()[16] == b':' && digits(17..19) => (
                "date",
                Some(serde_json::json!({ "time": true, "format": "yyyy-MM-dd'T'HH:mm:ss" })),
            ),
            20 if time && t.ends_with('Z') && t.as_bytes()[16] == b':' && digits(17..19) => (
                "date",
                Some(serde_json::json!({ "time": true, "format": "yyyy-MM-dd'T'HH:mm:ss'Z'" })),
            ),
            _ => ("string", None),
        };
    }
    let l = t.to_lowercase();
    if t.starts_with('<') && t.contains("</") {
        ("rich-text", Some(serde_json::json!({ "format": "html" })))
    } else if [".jpg", ".jpeg", ".png", ".webp", ".avif", ".gif", ".svg"]
        .iter()
        .any(|e| l.ends_with(e))
    {
        ("image", None)
    } else if s.contains('\n') || s.chars().count() > 90 {
        ("text", None)
    } else {
        ("string", None)
    }
}

/// Définition (Pages CMS) d'un champ d'après ses valeurs dans les contenus.
pub fn infer_field(name: &str, values: &[&Value], depth: usize) -> Value {
    let mut def = Map::new();
    def.insert("name".into(), Value::String(name.into()));
    def.insert("label".into(), Value::String(humanize(name)));
    let first = values.iter().copied().find(|v| !v.is_null());
    let (kind, options, list, fields): (&str, Option<Value>, bool, Option<Vec<Value>>) = match first
    {
        Some(Value::Bool(_)) => ("boolean", None, false, None),
        Some(Value::Number(_)) => ("number", None, false, None),
        Some(Value::String(s)) => {
            let (k, o) = string_kind(s);
            (k, o, false, None)
        }
        Some(Value::Array(_)) => {
            let items: Vec<&Value> = values
                .iter()
                .filter_map(|v| v.as_array())
                .flatten()
                .collect();
            if items.iter().all(|x| x.is_object()) && !items.is_empty() && depth < 3 {
                ("object", None, true, Some(infer_fields(&items, depth + 1)))
            } else {
                let image = !items.is_empty()
                    && items
                        .iter()
                        .all(|x| x.as_str().is_some_and(|s| string_kind(s).0 == "image"));
                (if image { "image" } else { "string" }, None, true, None)
            }
        }
        Some(Value::Object(_)) if depth < 3 => {
            let objs: Vec<&Value> = values.iter().copied().filter(|v| v.is_object()).collect();
            ("object", None, false, Some(infer_fields(&objs, depth + 1)))
        }
        _ => ("string", None, false, None),
    };
    def.insert("type".into(), Value::String(kind.into()));
    if list {
        def.insert("list".into(), Value::Bool(true));
    }
    if let Some(o) = options {
        def.insert("options".into(), o);
    }
    if let Some(f) = fields {
        def.insert("fields".into(), Value::Array(f));
    }
    if TECHNICAL.contains(&name) && depth == 0 {
        def.insert("hidden".into(), Value::Bool(true));
    }
    Value::Object(def)
}

/// Champs d'après les valeurs de plusieurs objets (en-têtes), dans l'ordre d'apparition.
pub fn infer_fields(objects: &[&Value], depth: usize) -> Vec<Value> {
    let mut names: Vec<&str> = Vec::new();
    for o in objects {
        if let Some(m) = o.as_object() {
            for k in m.keys() {
                if !names.contains(&k.as_str()) {
                    names.push(k);
                }
            }
        }
    }
    // Noms acceptés par Pages CMS (lettres, chiffres, `-`, `_`) ; les autres valeurs restent hors
    // du schéma.
    names
        .into_iter()
        .filter(|n| {
            !n.is_empty()
                && n.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        })
        .map(|n| {
            let values: Vec<&Value> = objects.iter().filter_map(|o| o.get(n)).collect();
            infer_field(n, &values, depth)
        })
        .collect()
}

/// Description (Pages CMS) d'une collection d'après ses contenus : champs déduits des valeurs,
/// champ principal, nom de fichier tiré de l'adresse (`slug`) ou du champ principal.
pub fn collection_def(name: &str, label: &str, folder: &str, entries: &[&Value]) -> Value {
    let fields = infer_fields(entries, 0);
    let has = |n: &str| {
        fields
            .iter()
            .any(|f| f.get("name").and_then(Value::as_str) == Some(n))
    };
    let primary = ["title", "titre", "name", "nom"]
        .into_iter()
        .find(|n| has(n));
    let mut def = Map::new();
    def.insert("name".into(), Value::String(name.into()));
    def.insert("label".into(), Value::String(label.into()));
    def.insert("type".into(), Value::String("collection".into()));
    def.insert("path".into(), Value::String(norm_path(folder)));
    def.insert(
        "filename".into(),
        Value::String(
            if has("slug") {
                "{fields.slug}.md"
            } else {
                "{primary}.md"
            }
            .into(),
        ),
    );
    if let Some(p) = primary {
        def.insert("view".into(), serde_json::json!({ "primary": p }));
    }
    def.insert("fields".into(), Value::Array(fields));
    Value::Object(def)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r#"
media:
  input: src/media
components:
  seo:
    name: seo
    label: Référencement
    type: object
    fields:
      - name: title
        label: Titre SEO
        type: string
content:
  - name: projets
    label: Projets
    type: collection
    path: ./src/content/projets/
    filename: "{fields.slug}.md"
    view:
      primary: title
      fields: [title, client, date]
      sort: date
      order: desc
    fields:
      - name: title
        label: Titre
        type: string
        required: true
      - name: slug
        type: string
        pattern: ^[a-z0-9-]+$
      - name: published
        type: boolean
        default: true
      - name: intro
        type: rich-text
        options:
          format: html
      - name: expertises
        type: reference
        options: { collection: expertises, multiple: true }
      - name: hook
        type: string
        list: true
      - name: seo
        component: seo
        label: SEO de la page
      - name: wp_id
        type: number
        hidden: true
  - name: accueil
    type: file
    path: src/_data/accueil.yml
"#;

    #[test]
    fn schema_lu() {
        let c = parse(CONFIG).collections;
        assert_eq!(c.len(), 1);
        let p = &c[0];
        assert_eq!(p.path, "src/content/projets");
        assert_eq!(p.label.as_deref(), Some("Projets"));
        assert_eq!(p.filename.as_deref(), Some("{fields.slug}.md"));
        let v = p.view.as_ref().unwrap();
        assert_eq!(v.fields, ["title", "client", "date"]);
        assert!(v.descending);
        let f = |n: &str| p.fields.iter().find(|f| f.name == n).unwrap();
        assert!(f("title").required);
        assert_eq!(f("slug").pattern.as_deref(), Some("^[a-z0-9-]+$"));
        assert_eq!(f("published").default, Some(Value::Bool(true)));
        assert_eq!(f("intro").kind.as_deref(), Some("rich_text"));
        assert_eq!(f("intro").options["format"], "html");
        assert_eq!(f("expertises").options["multiple"], true);
        assert!(f("hook").list);
        assert!(f("wp_id").hidden);
        // Composant remplacé, libellé du champ gardé.
        let seo = f("seo");
        assert_eq!(seo.kind.as_deref(), Some("object"));
        assert_eq!(seo.label.as_deref(), Some("SEO de la page"));
        assert_eq!(seo.fields[0].label.as_deref(), Some("Titre SEO"));
        assert!(parse("pas: [du yaml").collections.is_empty());
        assert!(!parse(CONFIG).merge);
        assert!(parse("settings:\n  content:\n    merge: true\n").merge);
    }

    #[test]
    fn schema_deduit_des_contenus() {
        let a = serde_json::json!({ "title": "A", "date": "2026-03-19T09:29:24", "tags": ["x"], "layout": "p.njk",
            "seo": { "title": "S" }, "intro": "<p>Texte</p>", "cover": "/m/a.jpg", "n": 2, "on": true });
        let b = serde_json::json!({ "title": "B", "slug": "b", "mot clé": 1, "blocks": [{ "type": "text", "body": "x" }, { "type": "image", "image": "/m/b.png" }] });
        let d = collection_def("projets", "Projets", "./src/content/projets/", &[&a, &b]);
        assert_eq!(d["path"], "src/content/projets");
        assert_eq!(d["filename"], "{fields.slug}.md");
        assert_eq!(d["view"]["primary"], "title");
        let f = |n: &str| {
            d["fields"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["name"] == n)
                .unwrap()
                .clone()
        };
        assert_eq!(f("date")["options"]["format"], "yyyy-MM-dd'T'HH:mm:ss");
        assert_eq!(f("tags")["list"], true);
        assert!(f("tags").get("hidden").is_none());
        assert_eq!(f("layout")["hidden"], true);
        assert_eq!(f("seo")["fields"][0]["name"], "title");
        assert_eq!(f("intro")["type"], "rich-text");
        assert_eq!(f("cover")["type"], "image");
        assert_eq!(
            (f("n")["type"].clone(), f("on")["type"].clone()),
            ("number".into(), "boolean".into())
        );
        assert_eq!(f("blocks")["fields"].as_array().unwrap().len(), 3);
        assert_eq!(f("blocks")["label"], "Blocks");
        assert!(
            d["fields"]
                .as_array()
                .unwrap()
                .iter()
                .all(|f| f["name"] != "mot clé")
        );
        // Relu comme un schéma de Revel.
        let cfg = revel_project::pagesyml::add_collection("", &d).unwrap();
        assert!(cfg.contains("label: \"N\"\n"));
        let p = parse(&cfg).collections;
        assert_eq!(p[0].fields.len(), d["fields"].as_array().unwrap().len());
    }
}
