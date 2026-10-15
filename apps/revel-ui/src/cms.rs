//! Vue CMS (étapes 2.12, 3.1) : collections de contenus en tableau (recherche, tri) et fiche du
//! contenu choisi, modifiable champ par champ ; création et suppression de contenus. Chaque
//! modification est une commande (`SetEntryField`, `SetEntryBody`, `CreateEntry`, `WriteEntry`)
//! appliquée par le cœur, annulable (Ctrl+Z) ; le fichier Markdown reste lisible par Pages CMS.

use crate::dialog::Ask;
use crate::panels::PlannedButton;
use crate::site::capitalize;
use crate::{Route, Tab, Ui, api, icons};
use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::{Command, EditState};
use revel_protocol::{CmsCollection, CmsEntry, CmsField, SiteSummary};
use serde_json::Value;

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
/// Colonnes affichées en plus du titre.
const MAX_COLUMNS: usize = 5;

/// Libellé d'un champ : celui du schéma, sinon le nom mis en forme (`date_debut` -> « Date debut »).
fn field_label(f: &CmsField) -> String {
    f.label
        .clone()
        .unwrap_or_else(|| capitalize(&f.name.replace(['_', '-'], " ")))
}

/// Valeur d'un champ en texte court (cellule du tableau).
fn cell(ui: Ui, v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::Bool(b)) => if *b { "✓" } else { "—" }.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => {
            // Date ISO : date locale ; chemin de fichier : son nom.
            if s.len() >= 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
                let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(s));
                if !d.get_time().is_nan() {
                    let opts = js_sys::Object::new();
                    let _ = js_sys::Reflect::set(&opts, &"dateStyle".into(), &"medium".into());
                    return d
                        .to_locale_date_string(&ui.lang(), &opts)
                        .as_string()
                        .unwrap_or_else(|| s.clone());
                }
            }
            s.clone()
        }
        Some(Value::Array(a)) => {
            if a.iter().all(|x| x.is_string()) && a.len() <= 3 {
                a.iter()
                    .filter_map(|x| x.as_str())
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

#[component]
pub fn CmsView(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let cols = RwSignal::new(None::<Vec<CmsCollection>>);
    let current = RwSignal::new(None::<String>);
    let open = RwSignal::new(None::<String>);
    let query = RwSignal::new(String::new());
    let sort = RwSignal::new(None::<(String, bool)>);
    let state = RwSignal::new(None::<EditState>);
    let path = StoredValue::new(site.path.clone());
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
                    cols.set(Some(c));
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
                Ok(st) => state.set(Some(st)),
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
                Ok(st) => state.set(Some(st)),
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
    Effect::new(move |_| {
        let _ = current.get();
        open.set(None);
        query.set(String::new());
        sort.set(None);
    });
    let col = Memo::new(move |_| {
        let id = current.get()?;
        cols.with(|c| c.as_ref()?.iter().find(|c| c.id == id).cloned())
    });
    // Champ titre et colonnes affichées.
    let layout = Memo::new(move |_| {
        let c = col.get()?;
        let title = c
            .fields
            .iter()
            .find(|f| TITLES.contains(&f.name.to_lowercase().as_str()))
            .map(|f| f.name.clone());
        let columns: Vec<CmsField> = c
            .fields
            .iter()
            .filter(|f| Some(&f.name) != title.as_ref() && !HIDDEN.contains(&f.name.as_str()))
            .take(MAX_COLUMNS)
            .cloned()
            .collect();
        Some((title, columns))
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
        if let Some((field, desc)) = sort.get() {
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
        sort.update(|s| {
            *s = match s.take() {
                Some((f, false)) if f == field => Some((f, true)),
                Some((f, true)) if f == field => None,
                _ => Some((field, false)),
            }
        })
    };
    let sort_mark = move |field: &str| {
        sort.with(|s| match s {
            Some((f, false)) if f == field => "↑",
            Some((f, true)) if f == field => "↓",
            _ => "",
        })
    };
    let goto = move |url: String| {
        ui.goto_page.set(Some(url));
        ui.tab(Tab::Editor);
    };
    // Nouveau contenu : titre demandé, nom de fichier tiré du titre (unique dans la collection).
    let new_entry = move |_| {
        let Some(c) = col.get_untracked() else {
            return;
        };
        let title_key = layout
            .get_untracked()
            .and_then(|l| l.0)
            .unwrap_or_else(|| "title".into());
        ui.ask.set(Some(Ask::input(
            ui.now("cms-entry-new", &[]),
            ui.now("cms-entry-title", &[]),
            String::new(),
            ui.now("action-create", &[]),
            move |title| {
                let base = slugify(&title);
                let mut file = base.clone();
                let mut n = 2;
                while c.entries.iter().any(|e| e.file == file) {
                    file = format!("{base}-{n}");
                    n += 1;
                }
                let mut data = serde_json::Map::new();
                data.insert(title_key.clone(), Value::String(title));
                if c.fields.iter().any(|f| f.name == "date") {
                    let d = js_sys::Date::new_0();
                    let iso = d.to_iso_string().as_string().unwrap_or_default();
                    data.insert("date".into(), Value::String(iso.chars().take(10).collect()));
                }
                open.set(Some(file.clone()));
                apply.run(Command::CreateEntry {
                    folder: c.folder.clone(),
                    file,
                    data: Value::Object(data),
                    body: String::new(),
                });
            },
        )));
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

    view! {
        <div class="cms" class:has-item=move || open.get().is_some()>
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
                                    on:click=move |_| current.set(Some(id2.clone()))>
                                    <span class="list-icon">{icons::database()}</span>
                                    <span class="list-label">{capitalize(&c.id)}</span>
                                    <span class="list-count">{c.entries.len()}</span>
                                </button>
                            }
                        }).collect_view().into_any()
                    })}
                </div>
                <footer class="panel-foot">
                    <span class="panel-foot-text muted"></span>
                    <PlannedButton key="cms-collection-new" step=3 icon=icons::plus/>
                </footer>
            </aside>
            <section class="cms-main">
                {move || col.get().map(|c| view! {
                    <header class="cms-head">
                        <div class="cms-title">
                            <h1>{capitalize(&c.id)}</h1>
                            <span class="muted path" title=c.folder.clone()>{c.folder.clone()}</span>
                        </div>
                        <label class="panel-search cms-search">
                            {icons::search()}
                            <input type="search" placeholder=ui.t("cms-search") aria-label=ui.t("cms-search")
                                prop:value=move || query.get() on:input=move |e| query.set(event_target_value(&e))/>
                        </label>
                        <button class="icon-btn" title=ui.t("editor-undo") aria-label=ui.t("editor-undo")
                            prop:disabled=move || !can_undo() on:click=move |_| undo()>{icons::undo()}</button>
                        <button class="icon-btn" title=ui.t("editor-redo") aria-label=ui.t("editor-redo")
                            prop:disabled=move || !can_redo() on:click=move |_| redo()>{icons::redo()}</button>
                        <button class="btn" disabled=true title=move || ui.tx("planned-step", &[("step", "3".to_string())])>
                            {icons::database()}{ui.t("cms-fields-edit")}
                        </button>
                        <button class="btn btn--primary cms-new" on:click=new_entry>
                            {icons::plus()}{ui.t("cms-entry-new")}
                        </button>
                    </header>
                })}
                <div class="cms-table-wrap">
                    {move || match (cols.get().is_none(), layout.get()) {
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
                                                {title.as_ref().and_then(|t| col.get().and_then(|c| c.fields.into_iter().find(|f| &f.name == t))).map(|f| field_label(&f)).unwrap_or_else(|| ui.tx("cms-file", &[]))}
                                                <span class="th-mark">{move || sort_mark(&tkey2)}</span>
                                            </button>
                                        </th>
                                        {columns.iter().map(|f| {
                                            let (n1, n2) = (f.name.clone(), f.name.clone());
                                            view! {
                                                <th><button class="th-sort" on:click=move |_| sort_by(n1.clone())>
                                                    {field_label(f)}<span class="th-mark">{move || sort_mark(&n2)}</span>
                                                </button></th>
                                            }
                                        }).collect_view()}
                                        <th class="col-actions"></th>
                                    </tr></thead>
                                    <tbody>
                                        {move || {
                                            let title = title.clone();
                                            let columns = columns.clone();
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
                                                        on:click=move |_| open.set(Some(file.clone()))>
                                                        <td class="strong cms-td-title"><div class="cms-title-cell">
                                                            <span class="cms-cell">{entry_title(&e, &title)}</span>
                                                            {draft.then(|| view! { <span class="chip chip--draft">{ui.t("cms-draft")}</span> })}
                                                        </div></td>
                                                        {columns.iter().map(|f| view! {
                                                            <td class="muted"><span class="cms-cell">{cell(ui, e.data.get(&f.name))}</span></td>
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
            {move || open.get().map(|file| view! {
                <EntryForm file=file col=col title=Signal::derive(move || layout.get().and_then(|l| l.0))
                    apply=apply delete=delete close=Callback::new(move |()| open.set(None)) goto=Callback::new(goto)/>
            })}
        </div>
    }
}

/// Nom de fichier tiré d'un titre (`L'Été à Oka` -> `l-ete-a-oka`).
fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.trim().to_lowercase().chars() {
        let c = match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            'œ' => {
                out.push('o');
                'e'
            }
            'æ' => {
                out.push('a');
                'e'
            }
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if c != '-' || !out.ends_with('-') {
            out.push(c);
        }
    }
    let out = out.trim_matches('-').chars().take(80).collect::<String>();
    if out.is_empty() {
        "contenu".into()
    } else {
        out
    }
}

/// Type de contrôle d'un champ : celui du schéma, sinon déduit de la valeur.
#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Text,
    LongText,
    Rich,
    Image,
    Date,
    Bool,
    Number,
    List,
    Reference(Option<String>),
    Json,
}

fn kind_of(f: &CmsField, v: Option<&Value>) -> Kind {
    match f.kind.as_deref() {
        Some("string") => return Kind::Text,
        Some("text") => return Kind::LongText,
        Some("rich_text") => return Kind::Rich,
        Some("image") => return Kind::Image,
        Some("date") => return Kind::Date,
        Some("boolean") => return Kind::Bool,
        Some("number") => return Kind::Number,
        Some("list")
            if v.is_none_or(|v| v.as_array().is_some_and(|a| a.iter().all(Value::is_string))) =>
        {
            return Kind::List;
        }
        Some("reference") => return Kind::Reference(None),
        _ => {}
    }
    match v {
        Some(Value::Bool(_)) => Kind::Bool,
        Some(Value::Number(_)) => Kind::Number,
        Some(Value::Array(a)) if a.iter().all(Value::is_string) => Kind::List,
        Some(Value::Array(_) | Value::Object(_)) => Kind::Json,
        Some(Value::String(s)) if is_date(s) => Kind::Date,
        Some(Value::String(s)) if is_image(s) => Kind::Image,
        Some(Value::String(s)) if crate::richtext::looks_like_html(s) => Kind::Rich,
        Some(Value::String(s)) if s.contains('\n') || s.chars().count() > 90 => Kind::LongText,
        _ => Kind::Text,
    }
}

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 10
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[7] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit)
}

fn is_image(s: &str) -> bool {
    let l = s.to_lowercase();
    [".jpg", ".jpeg", ".png", ".webp", ".avif", ".gif", ".svg"]
        .iter()
        .any(|e| l.ends_with(e))
}

/// Fiche d'un contenu : chaque champ modifiable (valeur écrite à la sortie du champ), texte,
/// suppression, ouverture de la page dans l'éditeur. La fiche n'est pas reconstruite quand les
/// données sont relues : le champ en cours de saisie garde le focus.
#[component]
fn EntryForm(
    file: String,
    col: Memo<Option<CmsCollection>>,
    title: Signal<Option<String>>,
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
    let value_of =
        move |key: &str| entry.with(|e| e.as_ref().and_then(|e| e.data.get(key).cloned()));
    let heading = move || {
        entry
            .with(|e| e.as_ref().map(|e| entry_title(e, &title.get())))
            .unwrap_or_default()
    };
    view! {
        <aside class="cms-item" aria-label=ui.t("cms-entry") data-entry=file.get_value()>
            <header class="cms-item-head">
                <h2>{heading}</h2>
                <button class="icon-btn" title=ui.t("action-close") aria-label=ui.t("action-close")
                    on:click=move |_| close.run(())>{icons::close()}</button>
            </header>
            <div class="cms-item-body">
                {fields.into_iter().map(|f| {
                    let kind = kind_of(&f, entry.with_untracked(|e| e.as_ref().and_then(|e| e.data.get(&f.name).cloned())).as_ref());
                    let label = field_label(&f);
                    let key = StoredValue::new(f.name.clone());
                    let text = move || match value_of(&key.get_value()) {
                        Some(Value::String(s)) => s,
                        Some(Value::Number(n)) => n.to_string(),
                        Some(v @ (Value::Array(_) | Value::Object(_))) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                        _ => String::new(),
                    };
                    let commit_text = move |v: String| set(key.get_value(), Some(Value::String(v)));
                    let date_text = move || text().chars().take(10).collect::<String>();
                    let control = match kind {
                        Kind::Bool => view! {
                            <label class="switch">
                                <input type="checkbox" prop:checked=move || matches!(value_of(&key.get_value()), Some(Value::Bool(true)))
                                    on:change=move |e| set(key.get_value(), Some(Value::Bool(event_target_checked(&e))))/>
                                <span class="switch-track"></span>
                            </label>
                        }.into_any(),
                        Kind::Number => view! {
                            <input class="input" type="number" step="any" prop:value=text
                                on:change=move |e| {
                                    let v = event_target_value(&e);
                                    let n = v.trim().parse::<f64>().ok().and_then(|x| {
                                        if x.fract() == 0.0 && x.abs() < 1e15 { Some(serde_json::Number::from(x as i64)) } else { serde_json::Number::from_f64(x) }
                                    });
                                    set(key.get_value(), Some(n.map(Value::Number).unwrap_or(Value::Null)));
                                }/>
                        }.into_any(),
                        Kind::Date => view! {
                            <input class="input" type="date" prop:value=date_text
                                on:change=move |e| {
                                    let v = event_target_value(&e);
                                    // L'heure éventuelle de l'ancienne valeur est gardée.
                                    let old = text();
                                    let v = if v.is_empty() { v } else if old.len() > 10 && is_date(&old) { format!("{v}{}", &old[10..]) } else { v };
                                    commit_text(v)
                                }/>
                        }.into_any(),
                        Kind::Rich => {
                            let format = if text().trim().is_empty() || crate::richtext::looks_like_html(&text()) {
                                crate::richtext::Format::Html
                            } else {
                                crate::richtext::Format::Markdown
                            };
                            view! {
                                <crate::richtext::RichText value=Signal::derive(text) format=format
                                    on_commit=Callback::new(move |v: String| commit_text(v))/>
                            }.into_any()
                        }
                        Kind::LongText => view! {
                            <textarea class="input cms-textarea" rows="4" prop:value=text
                                on:change=move |e| commit_text(event_target_value(&e))></textarea>
                        }.into_any(),
                        Kind::Json => view! {
                            <textarea class="input cms-textarea cms-json" rows="5" spellcheck="false" prop:value=text
                                on:change=move |e| {
                                    match serde_json::from_str::<Value>(&event_target_value(&e)) {
                                        Ok(v) => set(key.get_value(), Some(v)),
                                        Err(err) => ui.fail(revel_protocol::Failure::detail(err.to_string())),
                                    }
                                }></textarea>
                        }.into_any(),
                        Kind::List => view! { <TagsInput value=Signal::derive(move || value_of(&key.get_value())) set=Callback::new(move |v: Value| set(key.get_value(), Some(v)))/> }.into_any(),
                        Kind::Image => view! {
                            <div class="cms-image">
                                <input class="input" type="text" spellcheck="false" prop:value=text
                                    on:change=move |e| commit_text(event_target_value(&e))/>
                            </div>
                        }.into_any(),
                        Kind::Text | Kind::Reference(_) => view! {
                            <input class="input" type="text" prop:value=text
                                on:change=move |e| commit_text(event_target_value(&e))/>
                        }.into_any(),
                    };
                    let long = matches!(kind, Kind::LongText | Kind::Rich | Kind::Json);
                    // Contrôle composé (texte riche, liste à pastilles) : pas dans un `<label>`, qui
                    // renverrait chaque clic vers son premier bouton.
                    if matches!(kind, Kind::Rich | Kind::List) {
                        view! {
                            <div class="cms-item-field" class:is-long=long data-field=f.name.clone()>
                                <span class="label" title=f.name.clone()>{label}</span>
                                {control}
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <label class="cms-item-field" class:is-long=long data-field=f.name.clone()>
                                <span class="label" title=f.name.clone()>{label}</span>
                                {control}
                            </label>
                        }.into_any()
                    }
                }).collect_view()}
                <div class="cms-item-field is-long" data-field="--body">
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

/// Liste de textes : pastilles (× pour retirer), champ pour ajouter (Entrée).
#[component]
fn TagsInput(value: Signal<Option<Value>>, set: Callback<Value>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let items = move || -> Vec<String> {
        value
            .get()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect()
    };
    view! {
        <div class="tags-input">
            {move || items().into_iter().enumerate().map(|(i, t)| view! {
                <span class="chip chip--tag">{t}
                    <button class="chip-x" title=ui.t("action-delete") aria-label=ui.t("action-delete")
                        on:click=move |e| {
                            e.prevent_default();
                            let mut v = items();
                            if i < v.len() { v.remove(i); }
                            set.run(Value::Array(v.into_iter().map(Value::String).collect()));
                        }>{icons::close()}</button>
                </span>
            }).collect_view()}
            <input class="tags-add" type="text" placeholder=ui.t("cms-tag-add")
                on:keydown=move |e: ev::KeyboardEvent| {
                    if e.key() != "Enter" { return; }
                    e.prevent_default();
                    let input = event_target::<web_sys::HtmlInputElement>(&e);
                    let t = input.value().trim().to_string();
                    if t.is_empty() { return; }
                    input.set_value("");
                    let mut v = items();
                    if !v.contains(&t) { v.push(t); }
                    set.run(Value::Array(v.into_iter().map(Value::String).collect()));
                }/>
        </div>
    }
}
