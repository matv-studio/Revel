//! Configuration de Pages CMS (`.pages.yml`, https://pagescms.org/docs/configuration/) : schéma des
//! collections de contenus (étape 3.3). Revel lit les champs déclarés — libellés, types, aides,
//! valeurs par défaut, champs cachés, choix, références, objets — et la présentation des tableaux,
//! pour que la vue CMS et Pages CMS décrivent les mêmes contenus.

use revel_protocol::{CmsField, CmsTableView};
use serde_json::{Map, Value};
use std::path::Path;

/// Nom du fichier de configuration lu par Pages CMS, à la racine du dépôt.
pub const CONFIG_FILE: &str = ".pages.yml";

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
    if let Some(c) = def.get("component").and_then(Value::as_str)
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
    })
}

/// Collections décrites par le texte d'un `.pages.yml` (vide s'il est illisible).
pub fn parse(yaml: &str) -> Vec<PagesCollection> {
    let Ok(v) = revel_project::parse_yaml(yaml) else {
        return Vec::new();
    };
    let components = v
        .get("components")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    v.get("content")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter(|c| c.get("type").and_then(Value::as_str) == Some("collection"))
                .filter_map(|c| {
                    let view = c.get("view").map(|w| CmsTableView {
                        primary: text(w, "primary"),
                        fields: w
                            .get("fields")
                            .and_then(Value::as_array)
                            .map(|a| {
                                a.iter()
                                    .filter_map(|x| x.as_str().map(str::to_string))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        sort: text(w, "sort"),
                        descending: text(w, "order").is_some_and(|o| o == "desc"),
                    });
                    Some(PagesCollection {
                        name: text(c, "name")?,
                        label: text(c, "label"),
                        path: norm_path(&text(c, "path")?),
                        filename: text(c, "filename"),
                        view,
                        fields: c
                            .get("fields")
                            .and_then(Value::as_array)
                            .map(|a| a.iter().filter_map(|f| field(f, &components, 0)).collect())
                            .unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Collections du `.pages.yml` d'un site (aucune s'il n'existe pas).
pub fn load(site: &Path) -> Vec<PagesCollection> {
    std::fs::read_to_string(site.join(CONFIG_FILE))
        .map(|t| parse(&t))
        .unwrap_or_default()
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
        let c = parse(CONFIG);
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
        assert!(parse("pas: [du yaml").is_empty());
    }
}
