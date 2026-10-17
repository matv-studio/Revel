//! Vue CMS (étapes 2.12, 3.1, 3.3) : collections de contenus en tableau (recherche, tri) et fiche
//! du contenu choisi, modifiable champ par champ ; création et suppression de contenus. Le schéma
//! de Pages CMS (`.pages.yml`) donne les libellés, les colonnes, le tri et les contrôles
//! (`cmsfields`). Chaque modification est une commande (`SetEntryField`, `SetEntryBody`,
//! `CreateEntry`, `WriteEntry`) appliquée par le cœur, annulable (Ctrl+Z) ; le fichier Markdown
//! reste lisible par Pages CMS.

use crate::cmsfields::{self, Cols, field_row, label_of};
use crate::cmsschema::{FieldSettings, NewField, SchemaList};
use crate::dialog::Ask;
use crate::site::capitalize;
use crate::{Route, Tab, Ui, api, icons};
use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::{Command, EditState};
use revel_protocol::{CmsCollection, CmsEntry, CmsField, SiteSummary};
use serde_json::Value;
use std::collections::HashMap;

/// Variables techniques des en-têtes, gardées hors du tableau (visibles dans la fiche).
const HIDDEN: &[&str] = &[
    "layout",
    "permalink",
    "eleventyExcludeFromCollections",
    "eleventyComputed",
    "eleventyNavigation",
];
/// Noms habituels du champ titre.
const TITLES: &[&str] = &["title", "titre", "name", "nom"];
/// Colonnes affichées en plus du titre (sans colonnes déclarées par le schéma).
const MAX_COLUMNS: usize = 5;

/// Fusion d'un en-tête traduit sur celui de la langue principale (objets clé par clé).
pub(crate) fn merge_json(base: &mut Value, over: &Value) {
    match (base, over) {
        (Value::Object(b), Value::Object(o)) => {
            for (k, v) in o {
                match b.get_mut(k) {
                    Some(x) => merge_json(x, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (b, o) => *b = o.clone(),
    }
}

/// Collection vue dans une autre langue (3.8) : dossier de la langue (les modifications y sont
/// écrites, le contenu copié au besoin par le cœur), contenus traduits ou, à défaut, ceux de la
/// langue principale ; noms des fichiers traduits.
pub(crate) fn localized(c: &CmsCollection, code: &str) -> Option<(CmsCollection, Vec<String>)> {
    let own = c.locales.get(code)?;
    let mut l = c.clone();
    l.folder = revel_model::locale_folder(&c.folder, code);
    l.locales.clear();
    let mut done = Vec::new();
    for e in &mut l.entries {
        e.url = e.url.as_ref().map(|u| format!("/{code}{u}"));
        if let Some(t) = own.iter().find(|t| t.file == e.file) {
            merge_json(&mut e.data, &t.data);
            if !t.body.trim().is_empty() {
                e.body = t.body.clone();
            }
            done.push(e.file.clone());
        }
    }
    Some((l, done))
}

/// Nom affiché d'une collection.
pub(crate) fn col_label(c: &CmsCollection) -> String {
    c.label.clone().unwrap_or_else(|| capitalize(&c.id))
}

/// Champ principal (titre) : celui du schéma, sinon un nom habituel, sinon (schéma) le premier
/// champ simple.
pub(crate) fn primary_of(c: &CmsCollection) -> Option<String> {
    if let Some(p) = c.view.as_ref().and_then(|v| v.primary.clone()) {
        return Some(p);
    }
    c.fields
        .iter()
        .find(|f| TITLES.contains(&f.name.to_lowercase().as_str()))
        .or_else(|| {
            c.schema.as_ref().and(
                c.fields
                    .iter()
                    .find(|f| !f.hidden && !matches!(f.kind.as_deref(), Some("object" | "block"))),
            )
        })
        .map(|f| f.name.clone())
}

/// Colonnes du tableau : celles du schéma (`view.fields`), sinon les premiers champs simples.
pub(crate) fn columns_of(c: &CmsCollection, primary: Option<&String>) -> Vec<CmsField> {
    if let Some(v) = c.view.as_ref().filter(|v| !v.fields.is_empty()) {
        return v
            .fields
            .iter()
            .filter(|n| Some(*n) != primary)
            .map(|n| {
                c.fields
                    .iter()
                    .find(|f| &f.name == n)
                    .cloned()
                    .unwrap_or_else(|| CmsField {
                        name: n.clone(),
                        ..Default::default()
                    })
            })
            .collect();
    }
    c.fields
        .iter()
        .filter(|f| {
            Some(&f.name) != primary
                && !f.hidden
                && !HIDDEN.contains(&f.name.as_str())
                && !matches!(
                    f.kind.as_deref(),
                    Some("object" | "block" | "rich_text" | "code")
                )
        })
        .take(MAX_COLUMNS)
        .cloned()
        .collect()
}

/// Libellés des valeurs d'une colonne de choix ou de références.
fn value_labels(cols: &[CmsCollection], f: &CmsField) -> Option<HashMap<String, String>> {
    match f.kind.as_deref() {
        Some("select") => Some(cmsfields::choices(f).into_iter().collect()),
        Some("reference") => Some(cmsfields::reference_choices(cols, f).into_iter().collect()),
        _ => None,
    }
}

/// Valeur d'un champ en texte court (cellule du tableau).
fn cell(
    ui: Ui,
    f: &CmsField,
    v: Option<&Value>,
    labels: Option<&HashMap<String, String>>,
) -> String {
    let name = |s: &str| {
        labels
            .and_then(|l| l.get(s))
            .cloned()
            .unwrap_or_else(|| s.to_string())
    };
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::Bool(b)) => if *b { "✓" } else { "—" }.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => {
            // Date : date locale (et heure si le champ en a une).
            if cmsfields::is_date(s) || f.kind.as_deref() == Some("date") {
                let iso = if f.kind.as_deref() == Some("date") {
                    cmsfields::date_read(f, s)
                } else {
                    s.clone()
                };
                let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(&iso));
                if !iso.is_empty() && !d.get_time().is_nan() {
                    let opts = js_sys::Object::new();
                    let _ = js_sys::Reflect::set(&opts, &"dateStyle".into(), &"medium".into());
                    if iso.len() > 10 {
                        let _ = js_sys::Reflect::set(&opts, &"timeStyle".into(), &"short".into());
                    }
                    return d
                        .to_locale_string(&ui.lang(), &opts)
                        .as_string()
                        .unwrap_or_else(|| s.clone());
                }
            }
            name(s)
        }
        Some(Value::Array(a)) => {
            if a.iter().all(|x| x.is_string()) && (a.len() <= 3 || labels.is_some()) {
                a.iter()
                    .filter_map(|x| x.as_str())
                    .map(name)
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                ui.tx("cms-items", &[("count", a.len().to_string())])
            }
        }
        Some(Value::Object(m)) => ui.tx("cms-fields", &[("count", m.len().to_string())]),
    }
}

/// Titre d'une entrée : champ titre, sinon nom du fichier.
fn entry_title(e: &CmsEntry, title: &Option<String>) -> String {
    title
        .as_ref()
        .and_then(|t| e.data.get(t))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| e.file.clone())
}

/// Contenu non publié (`draft: true` ou `published: false`).
fn is_draft(e: &CmsEntry) -> bool {
    e.data.get("draft").and_then(Value::as_bool) == Some(true)
        || e.data.get("published").and_then(Value::as_bool) == Some(false)
}

/// Nouveau contenu : nom de fichier (modèle de la collection, sinon tiré du titre, unique) et
/// en-tête dans l'ordre du schéma : titre, adresse (`slug`) tirée du titre, valeurs par défaut,
/// date du jour pour les dates (comme Pages CMS).
fn new_entry_data(c: &CmsCollection, primary: &str, title: &str) -> (String, Value) {
    let taken = |s: &str| {
        c.entries
            .iter()
            .any(|e| e.file == s || e.data.get("slug").and_then(Value::as_str) == Some(s))
    };
    let unique = |base: String| {
        let mut s = base.clone();
        let mut n = 2;
        while taken(&s) {
            s = format!("{base}-{n}");
            n += 1;
        }
        s
    };
    let slug = unique(cmsfields::slugify(title));
    let mut data = serde_json::Map::new();
    if c.schema.is_none() {
        data.insert(primary.into(), Value::String(title.into()));
        if c.fields.iter().any(|f| f.name == "date") {
            let iso = js_sys::Date::new_0()
                .to_iso_string()
                .as_string()
                .unwrap_or_default();
            data.insert("date".into(), Value::String(iso.chars().take(10).collect()));
        }
        return (slug, Value::Object(data));
    }
    for f in c.fields.iter().filter(|f| !f.hidden) {
        let v = if f.name == primary {
            Some(Value::String(title.into()))
        } else if let Some(d) = &f.default {
            Some(d.clone())
        } else if f.name == "slug" && matches!(f.kind.as_deref(), None | Some("string")) {
            Some(Value::String(slug.clone()))
        } else if f.kind.as_deref() == Some("date") && !f.list {
            Some(Value::String(cmsfields::date_now(f)))
        } else {
            None
        };
        if let Some(v) = v {
            data.insert(f.name.clone(), v);
        }
    }
    if !data.contains_key(primary) {
        data.insert(primary.into(), Value::String(title.into()));
    }
    let data = Value::Object(data);
    let file = c
        .filename
        .as_deref()
        .and_then(|t| cmsfields::file_from_template(t, primary, &data))
        .map(|f| if f == slug { f } else { unique(f) })
        .unwrap_or(slug);
    (file, data)
}

#[component]
pub fn CmsView(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let cols = RwSignal::new(None::<Vec<CmsCollection>>);
    let current = RwSignal::new(None::<String>);
    let open = RwSignal::new(None::<String>);
    // Mode « Champs » (schéma de la collection) et champ choisi.
    let fields_mode = RwSignal::new(false);
    let sel_field = RwSignal::new(None::<String>);
    let query = RwSignal::new(String::new());
    // Tri choisi ; `None` : celui du schéma (`view.sort`).
    let sort = RwSignal::new(None::<Option<(String, bool)>>);
    let state = RwSignal::new(None::<EditState>);
    // Traductions automatiques à relire (D-36), par langue : clés `entry:<dossier>/<fichier>`.
    let marks = RwSignal::new(std::collections::BTreeMap::<
        String,
        std::collections::BTreeSet<String>,
    >::new());
    let path = StoredValue::new(site.path.clone());
    let site_locale = StoredValue::new(site.locale.clone());
    // Identifiants de toutes les collections du site (y compris sans dossier).
    let site_ids = StoredValue::new(
        site.collections
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>(),
    );
    let reload = move || {
        let Some(p) = path.try_get_value() else {
            return;
        };
        spawn_local(async move {
            let r = api::cms_collections(&p).await;
            // Vue fermée pendant la lecture.
            if current.is_disposed() {
                return;
            }
            match r {
                Ok(c) => {
                    if current.get_untracked().is_none() {
                        current.set(c.first().map(|c| c.id.clone()));
                    }
                    let localized = c.iter().any(|c| !c.locales.is_empty());
                    cols.set(Some(c));
                    if localized
                        && let Ok(o) = api::edit_open(&p).await
                        && !marks.is_disposed()
                    {
                        marks.set(
                            o.site
                                .locales
                                .into_iter()
                                .map(|l| (l.code, l.machine))
                                .collect(),
                        );
                    }
                }
                Err(e) => {
                    cols.set(Some(Vec::new()));
                    ui.fail(e)
                }
            }
        })
    };
    // Session d'édition du site (partagée avec l'éditeur : un seul historique).
    let p0 = site.path.clone();
    spawn_local(async move {
        match api::edit_open(&p0).await {
            Ok(o) => state.set(Some(o.state)),
            Err(e) => ui.fail(e),
        }
    });
    reload();
    let apply = Callback::new(move |cmd: Command| {
        let Some(p) = path.try_get_value() else {
            return;
        };
        spawn_local(async move {
            let r = api::edit_apply(&p, &cmd, false).await;
            if state.is_disposed() {
                return;
            }
            match r {
                Ok(st) => {
                    state.set(Some(st));
                    reload();
                }
                Err(e) => {
                    ui.fail(e);
                    reload();
                }
            }
        })
    });
    let undo = move || {
        let Some(p) = path.try_get_value() else {
            return;
        };
        spawn_local(async move {
            let r = api::edit_undo(&p).await;
            if state.is_disposed() {
                return;
            }
            match r {
                Ok(st) => {
                    if matches!(st.applied, Some(revel_edit::Command::SetSiteInfo { .. }))
                        && let Ok(o) = api::edit_open(&p).await
                    {
                        ui.site_identity(&o.site);
                    }
                    state.set(Some(st))
                }
                Err(e) => ui.fail(e),
            }
            reload();
        })
    };
    let redo = move || {
        let Some(p) = path.try_get_value() else {
            return;
        };
        spawn_local(async move {
            let r = api::edit_redo(&p).await;
            if state.is_disposed() {
                return;
            }
            match r {
                Ok(st) => {
                    if matches!(st.applied, Some(revel_edit::Command::SetSiteInfo { .. }))
                        && let Ok(o) = api::edit_open(&p).await
                    {
                        ui.site_identity(&o.site);
                    }
                    state.set(Some(st))
                }
                Err(e) => ui.fail(e),
            }
            reload();
        })
    };
    let can_undo = move || state.with(|s| s.as_ref().is_some_and(|s| s.can_undo));
    let can_redo = move || state.with(|s| s.as_ref().is_some_and(|s| s.can_redo));
    // Annuler, rétablir au clavier quand la vue CMS est affichée (hors champs de saisie).
    let on_key = window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if !matches!(ui.route.get_untracked(), Route::Site { tab: Tab::Cms, .. }) {
            return;
        }
        let in_field = document().active_element().is_some_and(|el| {
            matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                || el.has_attribute("contenteditable")
        });
        if in_field || !(e.ctrl_key() || e.meta_key()) {
            return;
        }
        match (e.key().to_lowercase().as_str(), e.shift_key()) {
            ("z", false) => undo(),
            ("z", true) | ("y", _) => redo(),
            _ => return,
        }
        e.prevent_default();
    });
    on_cleanup(move || on_key.remove());
    Effect::new(move |prev: Option<Option<String>>| {
        let now = current.get();
        // Collection changée (pas à la première lecture : une fiche demandée reste ouverte).
        if prev.is_some_and(|p| p != now) {
            open.set(None);
            sel_field.set(None);
            query.set(String::new());
            sort.set(None);
        }
        now
    });
    // Langue de travail (3.8) : collection vue dans cette langue.
    let lang = Memo::new(move |_| {
        let code = ui.locale.get()?;
        cols.with(|c| {
            c.as_ref()?
                .iter()
                .any(|c| c.locales.contains_key(&code))
                .then_some(code)
        })
    });
    Effect::new(move |_| {
        if lang.get().is_some() {
            fields_mode.set(false);
        }
    });
    let shown = Memo::new(move |_| {
        let id = current.get()?;
        let c = cols.with(|c| c.as_ref()?.iter().find(|c| c.id == id).cloned())?;
        Some(
            lang.get()
                .and_then(|code| localized(&c, &code))
                .unwrap_or((c, Vec::new())),
        )
    });
    let col = Memo::new(move |_| shown.get().map(|s| s.0));
    let translated = Memo::new(move |_| shown.get().map(|s| s.1).unwrap_or_default());
    // Dossier de la collection affichée dans la langue principale (clés des marques).
    let primary_folder = Memo::new(move |_| {
        let id = current.get()?;
        cols.with(|c| {
            c.as_ref()?
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.folder.clone())
        })
    });
    // Contenus traduits automatiquement, à relire, dans la langue affichée.
    let auto_files = Memo::new(move |_| {
        let (Some(code), Some(folder)) = (lang.get(), primary_folder.get()) else {
            return std::collections::BTreeSet::new();
        };
        let prefix = revel_edit::locale::entry_key(&folder, "");
        marks.with(|m| {
            m.get(&code)
                .map(|keys| {
                    keys.iter()
                        .filter_map(|k| k.strip_prefix(&prefix).map(str::to_string))
                        .collect()
                })
                .unwrap_or_default()
        })
    });
    let review = Callback::new(move |file: String| {
        if let (Some(code), Some(folder)) = (lang.get_untracked(), primary_folder.get_untracked()) {
            apply.run(Command::MarkMachine {
                locale: code,
                keys: vec![revel_edit::locale::entry_key(&folder, &file)],
                on: false,
            });
        }
    });
    let langs = Memo::new(move |_| {
        cols.with(|c| {
            c.as_ref()
                .and_then(|c| c.first())
                .map(|c| c.locales.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        })
    });
    // Contenu demandé par une autre vue (Localisation) : collection et fiche ouvertes.
    Effect::new(move |_| {
        let Some((id, file)) = ui.goto_entry.get() else {
            return;
        };
        if cols.with(|c| c.as_ref().is_some_and(|c| c.iter().any(|c| c.id == id))) {
            ui.goto_entry.set(None);
            fields_mode.set(false);
            current.set(Some(id));
            open.set(Some(file));
        }
    });
    // Champ titre et colonnes affichées.
    let layout = Memo::new(move |_| {
        let c = col.get()?;
        let title = primary_of(&c);
        let columns = columns_of(&c, title.as_ref());
        Some((title, columns))
    });
    let sorting = Memo::new(move |_| {
        sort.get().unwrap_or_else(|| {
            col.with(|c| {
                let v = c.as_ref()?.view.as_ref()?;
                Some((v.sort.clone()?, v.descending))
            })
        })
    });
    let rows = Memo::new(move |_| {
        let Some(c) = col.get() else {
            return Vec::new();
        };
        let title = layout.get().and_then(|l| l.0);
        let q = query.get().trim().to_lowercase();
        let mut list: Vec<CmsEntry> = c
            .entries
            .into_iter()
            .filter(|e| {
                q.is_empty()
                    || e.file.to_lowercase().contains(&q)
                    || e.data.to_string().to_lowercase().contains(&q)
            })
            .collect();
        if let Some((field, desc)) = sorting.get() {
            let key = |e: &CmsEntry| -> String {
                if Some(&field) == title.as_ref() {
                    entry_title(e, &title).to_lowercase()
                } else {
                    match e.data.get(&field) {
                        Some(Value::Number(n)) => format!("{:020.6}", n.as_f64().unwrap_or(0.0)),
                        Some(Value::String(s)) => s.to_lowercase(),
                        Some(v) => v.to_string(),
                        None => String::new(),
                    }
                }
            };
            list.sort_by_key(key);
            if desc {
                list.reverse();
            }
        }
        list
    });
    let sort_by = move |field: String| {
        let next = match sorting.get_untracked() {
            Some((f, false)) if f == field => Some((f, true)),
            Some((f, true)) if f == field => None,
            _ => Some((field, false)),
        };
        sort.set(Some(next));
    };
    let sort_mark = move |field: &str| {
        sorting.with(|s| match s {
            Some((f, false)) if f == field => "↑",
            Some((f, true)) if f == field => "↓",
            _ => "",
        })
    };
    let goto = move |url: String| {
        ui.goto_page.set(Some(url));
        ui.tab(Tab::Editor);
    };
    // Nouveau contenu : titre demandé ; nom de fichier et valeurs d'après le schéma.
    let new_entry = move |_| {
        let Some(c) = col.get_untracked() else {
            return;
        };
        let primary = layout
            .get_untracked()
            .and_then(|l| l.0)
            .unwrap_or_else(|| "title".into());
        let label = c
            .fields
            .iter()
            .find(|f| f.name == primary && f.label.is_some())
            .map(label_of)
            .unwrap_or_else(|| ui.now("cms-entry-title", &[]));
        ui.ask.set(Some(Ask::input(
            ui.now("cms-entry-new", &[]),
            label,
            String::new(),
            ui.now("action-create", &[]),
            move |title| {
                let (file, data) = new_entry_data(&c, &primary, &title);
                open.set(Some(file.clone()));
                apply.run(Command::CreateEntry {
                    folder: c.folder.clone(),
                    file,
                    data,
                    body: String::new(),
                });
            },
        )));
    };
    // Nouvelle collection : nom demandé ; identifiant et dossier (à côté des autres collections)
    // tirés du nom ; champs de départ : titre et adresse ; ouverte dans « Champs ».
    // Dossier de contenus repris d'un autre site, qu'aucune collection ne lit : proposé (ses
    // champs sont déduits des contenus).
    let new_collection = move |_: ()| {
        let existing = cols.get_untracked().unwrap_or_default();
        let path = path.get_value();
        leptos::task::spawn_local(async move {
            let folders = crate::api::cms_folders(&path).await.unwrap_or_default();
            let chosen = RwSignal::new(String::new());
            let mut ask = Ask::input(
                ui.now("cms-collection-new", &[]),
                ui.now("cms-collection-name", &[]),
                String::new(),
                ui.now("action-create", &[]),
                move |label| {
                    let label = label.trim().to_string();
                    if label.is_empty() {
                        return;
                    }
                    let slug = cmsfields::slugify(&label);
                    let mut ids: Vec<String> = site_ids.get_value();
                    ids.extend(existing.iter().map(|c| c.id.clone()));
                    ids.extend(existing.iter().filter_map(|c| c.schema.clone()));
                    let base_id = slug.replace('-', "_");
                    let mut id = base_id.clone();
                    let mut n = 2;
                    while ids.contains(&id) {
                        id = format!("{base_id}_{n}");
                        n += 1;
                    }
                    // Dossier parent commun des collections (`src/content`), sinon `content`.
                    let parents: Vec<String> = existing
                        .iter()
                        .filter_map(|c| c.folder.rsplit_once('/').map(|(p, _)| p.to_string()))
                        .collect();
                    let base = parents
                        .first()
                        .filter(|p| parents.iter().all(|x| x == *p))
                        .or(parents.first())
                        .cloned()
                        .unwrap_or_else(|| "content".into());
                    let picked = chosen.get_untracked();
                    let mut folder = format!("{base}/{slug}");
                    let mut n = 2;
                    while existing.iter().any(|c| c.folder == folder) {
                        folder = format!("{base}/{slug}-{n}");
                        n += 1;
                    }
                    if !picked.is_empty() {
                        folder = picked;
                    }
                    let fields = serde_json::json!([
                        { "name": "title", "label": ui.now("cms-new-title", &[]), "type": "string", "required": true },
                        {
                            "name": "slug",
                            "label": ui.now("cms-new-slug", &[]),
                            "type": "string",
                            "required": true,
                            "pattern": "^[a-z0-9]+(?:-[a-z0-9]+)*$",
                            "description": ui.now("cms-new-slug-help", &[]),
                        }
                    ]);
                    apply.run(Command::CreateCmsCollection {
                        id: id.clone(),
                        label,
                        folder,
                        fields,
                    });
                    current.set(Some(id));
                    fields_mode.set(true);
                },
            );
            if !folders.is_empty() {
                let mut options = vec![(
                    String::new(),
                    ui.now("cms-collection-folder-new", &[]),
                    String::new(),
                )];
                options.extend(folders.into_iter().map(|f| {
                    let name = f.folder.rsplit('/').next().unwrap_or(&f.folder).to_string();
                    let label = ui.now(
                        "cms-collection-folder-existing",
                        &[("folder", f.folder.clone()), ("count", f.count.to_string())],
                    );
                    (
                        f.folder,
                        label,
                        crate::site::capitalize(&name.replace(['-', '_'], " ")),
                    )
                }));
                ask = ask.with_choice(ui.now("cms-collection-folder", &[]), options, chosen);
            }
            let _ = ui.ask.try_set(Some(ask));
        });
    };
    let delete = Callback::new(move |(folder, file, title): (String, String, String)| {
        ui.ask.set(Some(
            Ask::confirm(
                ui.now("cms-delete-title", &[("name", title)]),
                Some(ui.now("cms-delete-body", &[])),
                ui.now("action-delete", &[]),
                move || {
                    open.set(None);
                    apply.run(Command::WriteEntry {
                        folder: folder.clone(),
                        file: file.clone(),
                        text: None,
                    });
                },
            )
            .danger(),
        ));
    });

    // Menus contextuels : contenu (ouvrir, page, supprimer), collection.
    let row_menu = move |e: &ev::MouseEvent, file: String, url: Option<String>, label: String| {
        use crate::ctxmenu::{CtxItem, open as open_menu};
        let folder =
            col.with_untracked(|c| c.as_ref().map(|c| c.folder.clone()).unwrap_or_default());
        let f2 = file.clone();
        let mut items = vec![
            CtxItem::new(ui.now("cms-entry-open", &[]), move || {
                open.set(Some(f2.clone()))
            })
            .icon(icons::edit),
        ];
        if let Some(u) = url {
            items.push(
                CtxItem::new(ui.now("cms-open-page", &[]), move || goto(u.clone()))
                    .icon(icons::external),
            );
        }
        items.push(
            CtxItem::new(ui.now("action-delete", &[]), move || {
                delete.run((folder.clone(), file.clone(), label.clone()))
            })
            .icon(icons::trash)
            .danger()
            .sep(),
        );
        open_menu(ui, e, items);
    };
    let col_menu = move |e: &ev::MouseEvent, id: String| {
        use crate::ctxmenu::{CtxItem, open as open_menu};
        let (a, b) = (id.clone(), id);
        let items = vec![
            CtxItem::new(ui.now("cms-entry-new", &[]), move || {
                current.set(Some(a.clone()));
                fields_mode.set(false);
            })
            .icon(icons::database),
            CtxItem::new(ui.now("cms-fields-edit", &[]), move || {
                current.set(Some(b.clone()));
                fields_mode.set(true);
            })
            .icon(icons::list),
            CtxItem::new(
                ui.now("cms-collection-new", &[]),
                move || new_collection(()),
            )
            .icon(icons::plus)
            .sep(),
        ];
        open_menu(ui, e, items);
    };
    view! {
        <div class="cms" class:has-item=move || if fields_mode.get() { sel_field.get().is_some() } else { open.get().is_some() }
            class:is-fields=move || fields_mode.get()>
            <aside class="cms-cols">
                <p class="cms-cols-title">{ui.t("cms-collections")}</p>
                <div class="cms-cols-list">
                    {move || cols.get().map(|list| {
                        if list.is_empty() {
                            return view! { <p class="panel-empty muted">{ui.t("cms-none")}</p> }.into_any();
                        }
                        list.into_iter().map(|c| {
                            let (id, id2) = (c.id.clone(), c.id.clone());
                            view! {
                                <button class="cms-col" class:is-on=move || current.get().as_deref() == Some(id.as_str())
                                    data-collection=c.id.clone() title=c.folder.clone()
                                    on:contextmenu={ let id = c.id.clone(); move |e: ev::MouseEvent| col_menu(&e, id.clone()) }
                                    on:click=move |_| current.set(Some(id2.clone()))>
                                    <span class="list-icon">{icons::database()}</span>
                                    <span class="list-label">{col_label(&c)}</span>
                                    <span class="list-count">{c.entries.len()}</span>
                                </button>
                            }
                        }).collect_view().into_any()
                    })}
                </div>
                <footer class="panel-foot">
                    <span class="panel-foot-text muted"></span>
                    <button class="icon-btn icon-btn--small cms-col-new" title=ui.t("cms-collection-new")
                        aria-label=ui.t("cms-collection-new") on:click=move |_| new_collection(())>{icons::plus()}</button>
                </footer>
            </aside>
            <section class="cms-main">
                {move || col.get().map(|c| view! {
                    <header class="cms-head">
                        <div class="cms-title">
                            <h1 title=c.folder.clone()>{col_label(&c)}</h1>
                            <span class="muted path" title=c.folder.clone()>{c.folder.clone()}</span>
                        </div>
                        <Show when=move || !fields_mode.get()>
                            <label class="panel-search cms-search">
                                {icons::search()}
                                <input type="search" placeholder=ui.t("cms-search") aria-label=ui.t("cms-search")
                                    prop:value=move || query.get() on:input=move |e| query.set(event_target_value(&e))/>
                            </label>
                        </Show>
                        {move || (!langs.get().is_empty()).then(|| view! {
                            <div class="select cms-locale" class:is-other=move || lang.get().is_some() title=ui.t("cms-locale-help")>
                                {icons::globe()}
                                <select class="input" aria-label=ui.t("editor-locale")
                                    on:change=move |e| {
                                        let v = event_target_value(&e);
                                        ui.locale.set((!v.is_empty()).then_some(v));
                                    }>
                                    <option value="" selected=move || lang.get().is_none()>{move || revel_model::language_name(&site_locale.get_value())}</option>
                                    {langs.get().into_iter().map(|code| {
                                        let c2 = code.clone();
                                        let label = revel_model::language_name(&code);
                                        view! { <option value=code selected=move || lang.get().as_deref() == Some(c2.as_str())>{label}</option> }
                                    }).collect_view()}
                                </select>
                            </div>
                        })}
                        <button class="icon-btn" title=ui.t("editor-undo") aria-label=ui.t("editor-undo")
                            prop:disabled=move || !can_undo() on:click=move |_| undo()>{icons::undo()}</button>
                        <button class="icon-btn" title=ui.t("editor-redo") aria-label=ui.t("editor-redo")
                            prop:disabled=move || !can_redo() on:click=move |_| redo()>{icons::redo()}</button>
                        <button class="btn cms-fields-btn" class:is-on=move || fields_mode.get()
                            aria-pressed=move || fields_mode.get().to_string()
                            title=move || ui.tx(if lang.get().is_some() { "cms-locale-shared" } else { "cms-fields-help" }, &[])
                            prop:disabled=move || lang.get().is_some()
                            on:click=move |_| fields_mode.update(|m| *m = !*m)>
                            {icons::database()}{ui.t("cms-fields-edit")}
                        </button>
                        {move || if fields_mode.get() {
                            view! { <NewField col=col cols=Signal::derive(move || cols.get()) apply=apply selected=sel_field/> }.into_any()
                        } else {
                            view! {
                                <button class="btn btn--primary cms-new" on:click=new_entry
                                    prop:disabled=move || lang.get().is_some()
                                    title=move || lang.get().map(|_| ui.tx("cms-locale-shared", &[]))>
                                    {icons::plus()}{ui.t("cms-entry-new")}
                                </button>
                            }.into_any()
                        }}
                    </header>
                })}
                <div class="cms-table-wrap">
                    {move || match (cols.get().is_none(), layout.get()) {
                        (false, Some(_)) if fields_mode.get() => view! {
                            <SchemaList col=col selected=sel_field apply=apply/>
                        }.into_any(),
                        (true, _) => view! { <p class="muted cms-wait">{ui.t("editor-loading")}</p> }.into_any(),
                        (false, None) => ().into_any(),
                        (false, Some((title, columns))) => {
                            let tkey = title.clone().unwrap_or_default();
                            let tkey2 = tkey.clone();
                            view! {
                                <table class="table cms-table">
                                    <thead><tr>
                                        <th class="cms-th-title">
                                            <button class="th-sort" on:click=move |_| sort_by(tkey.clone())>
                                                {title.as_ref().and_then(|t| col.get().and_then(|c| c.fields.into_iter().find(|f| &f.name == t))).map(|f| label_of(&f)).unwrap_or_else(|| ui.tx("cms-file", &[]))}
                                                <span class="th-mark">{move || sort_mark(&tkey2)}</span>
                                            </button>
                                        </th>
                                        {columns.iter().map(|f| {
                                            let (n1, n2) = (f.name.clone(), f.name.clone());
                                            view! {
                                                <th><button class="th-sort" on:click=move |_| sort_by(n1.clone())>
                                                    {label_of(f)}<span class="th-mark">{move || sort_mark(&n2)}</span>
                                                </button></th>
                                            }
                                        }).collect_view()}
                                        <th class="col-actions"></th>
                                    </tr></thead>
                                    <tbody>
                                        {move || {
                                            let title = title.clone();
                                            let columns = columns.clone();
                                            let labels: Vec<Option<HashMap<String, String>>> = cols.with(|all| {
                                                let all = all.as_deref().unwrap_or_default();
                                                columns.iter().map(|f| value_labels(all, f)).collect()
                                            });
                                            let list = rows.get();
                                            if list.is_empty() {
                                                return view! {
                                                    <tr><td class="muted cms-empty" colspan=(columns.len() + 2).to_string()>{ui.t("cms-no-entries")}</td></tr>
                                                }.into_any();
                                            }
                                            list.into_iter().map(|e| {
                                                let file = e.file.clone();
                                                let f2 = e.file.clone();
                                                let url = e.url.clone();
                                                let draft = is_draft(&e);
                                                view! {
                                                    <tr class="row-link" class:is-on=move || open.get().as_deref() == Some(f2.as_str())
                                                        data-entry=e.file.clone()
                                                        on:contextmenu={
                                                            let (file, url, label) = (e.file.clone(), e.url.clone(), entry_title(&e, &title));
                                                            move |ev: ev::MouseEvent| row_menu(&ev, file.clone(), url.clone(), label.clone())
                                                        }
                                                        on:click=move |_| open.set(Some(file.clone()))>
                                                        <td class="strong cms-td-title"><div class="cms-title-cell">
                                                            <span class="cms-cell">{entry_title(&e, &title)}</span>
                                                            {draft.then(|| view! { <span class="chip chip--draft">{ui.t("cms-draft")}</span> })}
                                                            {
                                                                // Autre langue : point bleu (traduit) ou orange (contenu de la
                                                                // langue principale), comme les valeurs propres et héritées ;
                                                                // violet : traduction automatique à relire.
                                                                let f3 = e.file.clone();
                                                                move || lang.get().map(|_| {
                                                                    let done = translated.with(|t| t.contains(&f3));
                                                                    let auto = done && auto_files.with(|a| a.contains(&f3));
                                                                    view! {
                                                                        <span class="loc-dot" class:is-done=done class:is-auto=auto
                                                                            title=ui.t(if auto { "locales-review-help" } else if done { "cms-translated-help" } else { "cms-untranslated-help" })></span>
                                                                    }
                                                                })
                                                            }
                                                        </div></td>
                                                        {columns.iter().zip(&labels).map(|(f, l)| view! {
                                                            <td class="muted"><span class="cms-cell">{cell(ui, f, e.data.get(&f.name), l.as_ref())}</span></td>
                                                        }).collect_view()}
                                                        <td class="col-actions">
                                                            {url.map(|u| view! {
                                                                <button class="icon-btn icon-btn--small" title=ui.t("cms-open-page") aria-label=ui.t("cms-open-page")
                                                                    on:click=move |ev| { ev.stop_propagation(); goto(u.clone()) }>{icons::edit()}</button>
                                                            })}
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect_view().into_any()
                                        }}
                                    </tbody>
                                </table>
                            }.into_any()
                        }
                    }}
                </div>
                <footer class="cms-foot muted">
                    {move || col.get().map(|c| {
                        if fields_mode.get() {
                            return ui.tx("cms-fields-count", &[("count", c.fields.len().to_string())]);
                        }
                        let shown = rows.with(|r| r.len());
                        let total = c.entries.len();
                        if shown == total {
                            ui.tx("site-entries", &[("count", crate::num(total as f64, 0))])
                        } else {
                            ui.tx("cms-shown", &[("shown", shown.to_string()), ("total", total.to_string())])
                        }
                    })}
                </footer>
            </section>
            {move || if fields_mode.get() {
                sel_field.get().map(|name| view! {
                    <FieldSettings col=col cols=Signal::derive(move || cols.get()) name=name apply=apply
                        close=Callback::new(move |()| sel_field.set(None))/>
                }.into_any())
            } else {
                open.get().map(|file| view! {
                    <EntryForm file=file.clone() col=col cols=Signal::derive(move || cols.get()) title=Signal::derive(move || layout.get().and_then(|l| l.0))
                        auto=Signal::derive({ let f = file.clone(); move || auto_files.with(|a| a.contains(&f)) })
                        review=review
                        apply=apply delete=delete close=Callback::new(move |()| open.set(None)) goto=Callback::new(goto)/>
                }.into_any())
            }}
        </div>
    }
}

/// Fiche d'un contenu : chaque champ du schéma modifiable (valeur écrite à la sortie du champ),
/// texte, suppression, ouverture de la page dans l'éditeur ; champs cachés et valeurs hors du
/// schéma repliés en bas. La fiche n'est pas reconstruite quand les données sont relues : le
/// champ en cours de saisie garde le focus.
#[component]
fn EntryForm(
    file: String,
    col: Memo<Option<CmsCollection>>,
    cols: Cols,
    title: Signal<Option<String>>,
    /// Traduction automatique à relire (autre langue, D-36).
    auto: Signal<bool>,
    review: Callback<String>,
    apply: Callback<Command>,
    delete: Callback<(String, String, String)>,
    close: Callback<()>,
    goto: Callback<String>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let file = StoredValue::new(file);
    let entry = Memo::new(move |_| {
        col.with(|c| {
            c.as_ref()?
                .entries
                .iter()
                .find(|e| e.file == file.get_value())
                .cloned()
        })
    });
    let folder =
        move || col.with_untracked(|c| c.as_ref().map(|c| c.folder.clone()).unwrap_or_default());
    let fields: Vec<CmsField> =
        col.with_untracked(|c| c.as_ref().map(|c| c.fields.clone()).unwrap_or_default());
    // Valeurs hors du schéma (gardées telles quelles, comme le fait Pages CMS).
    let extra: Vec<CmsField> = entry.with_untracked(|e| {
        e.as_ref()
            .and_then(|e| e.data.as_object())
            .map(|m| {
                m.keys()
                    .filter(|k| !fields.iter().any(|f| &f.name == *k))
                    .map(|k| CmsField {
                        name: k.clone(),
                        ..Default::default()
                    })
                    .collect()
            })
            .unwrap_or_default()
    });
    let (shown, mut more): (Vec<CmsField>, Vec<CmsField>) =
        fields.into_iter().partition(|f| !f.hidden);
    more.extend(extra);
    let set = move |key: String, value: Option<Value>| {
        // Rien à écrire si la valeur n'a pas changé.
        let old = entry.with_untracked(|e| e.as_ref().and_then(|e| e.data.get(&key).cloned()));
        if old == value {
            return;
        }
        apply.run(Command::SetEntryField {
            folder: folder(),
            file: file.get_value(),
            key,
            value,
        });
    };
    let row = move |f: CmsField| {
        let key = StoredValue::new(f.name.clone());
        let value = Signal::derive(move || {
            entry.with(|e| {
                e.as_ref()
                    .and_then(|e| e.data.get(key.get_value()).cloned())
            })
        });
        // Valeur absente du schéma : avertissement (Pages CMS la retirerait) et ajout au schéma.
        let undeclared = col.with_untracked(|c| {
            c.as_ref()
                .is_some_and(|c| !c.fields.iter().any(|x| x.name == f.name))
        });
        let aside = undeclared.then(|| {
            let (schema, merge) = col.with_untracked(|c| {
                c.as_ref()
                    .map_or((false, true), |c| (c.schema.is_some(), c.merge))
            });
            let valid = !f.name.is_empty()
                && f.name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
            view! {
                {(schema && !merge).then(|| view! {
                    <span class="cms-warn" title=ui.t("cms-undeclared")>{icons::warning()}</span>
                })}
                <button type="button" class="icon-btn icon-btn--small cms-declare" disabled=!valid
                    title=move || if valid { ui.tx("cms-undeclared-add", &[]) } else { ui.tx("cms-undeclared-name", &[]) }
                    aria-label=ui.t("cms-undeclared-add")
                    on:click=move |_| apply.run(Command::AddCmsField {
                        folder: folder(),
                        def: serde_json::json!({ "name": key.get_value() }),
                        index: None,
                    })>{icons::plus()}</button>
            }
            .into_any()
        });
        field_row(
            f,
            value,
            Callback::new(move |v| set(key.get_value(), v)),
            cols,
            aside,
        )
    };
    let heading = move || {
        entry
            .with(|e| e.as_ref().map(|e| entry_title(e, &title.get())))
            .unwrap_or_default()
    };
    let more_count = more.len();
    view! {
        <aside class="cms-item" aria-label=ui.t("cms-entry") data-entry=file.get_value()>
            <header class="cms-item-head">
                <h2>{heading}</h2>
                <button class="icon-btn" title=ui.t("action-close") aria-label=ui.t("action-close")
                    on:click=move |_| close.run(())>{icons::close()}</button>
            </header>
            <div class="cms-item-body">
                {move || auto.get().then(|| view! {
                    <div class="cms-auto" role="note">
                        <span class="cms-auto-icon">{icons::sparkle()}</span>
                        <span title=ui.t("locales-review-help")>{ui.t("cms-auto")}</span>
                        <button type="button" class="btn btn--small" on:click=move |_| review.run(file.get_value())>
                            {icons::check()}{ui.t("locales-review-ok")}
                        </button>
                    </div>
                })}
                {shown.into_iter().map(row).collect_view()}
                <div class="cms-item-field" data-field="--body">
                    <span class="label">{ui.t("cms-body")}</span>
                    {
                        let body_text = Signal::derive(move || entry.with(|e| e.as_ref().map(|e| e.body.clone()).unwrap_or_default()));
                        let format = if crate::richtext::looks_like_html(&body_text.get_untracked()) {
                            crate::richtext::Format::Html
                        } else {
                            crate::richtext::Format::Markdown
                        };
                        view! {
                            <crate::richtext::RichText value=body_text format=format placeholder=ui.now("cms-body-empty", &[])
                                on_commit=Callback::new(move |body: String| {
                                    if entry.with_untracked(|x| x.as_ref().is_some_and(|x| x.body.trim_end() == body.trim_end())) {
                                        return;
                                    }
                                    apply.run(Command::SetEntryBody { folder: folder(), file: file.get_value(), body });
                                })/>
                        }
                    }
                </div>
                {(more_count > 0).then(|| view! {
                    <details class="cms-more">
                        <summary title=ui.t("cms-more-help")>
                            {icons::chevron_right()}
                            <span>{ui.tx("cms-more", &[("count", more_count.to_string())])}</span>
                        </summary>
                        <div class="cms-more-body">{more.into_iter().map(row).collect_view()}</div>
                    </details>
                })}
            </div>
            <footer class="cms-item-foot">
                <span class="muted path cms-item-file" title=move || format!("{}/{}.md", folder(), file.get_value())>
                    {move || format!("{}.md", file.get_value())}
                </span>
                <button class="icon-btn" title=ui.t("action-delete") aria-label=ui.t("action-delete")
                    on:click=move |_| delete.run((folder(), file.get_value(), heading()))>{icons::trash()}</button>
                {move || entry.get().and_then(|e| e.url).map(|u| view! {
                    <button class="btn btn--primary" on:click=move |_| goto.run(u.clone())>{ui.t("cms-open-page")}</button>
                })}
            </footer>
        </aside>
    }
}
