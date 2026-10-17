//! Champs d'une collection (étape 3.3, D-27) : liste des champs du schéma de Pages CMS
//! (`.pages.yml`), ordre changé au glisser (ou par les flèches), réglages du champ choisi
//! (libellé, aide, type, obligatoire, plusieurs valeurs, caché, colonne du tableau, titre des
//! contenus, options du type), nouveau champ, champ retiré. Chaque modification est une commande
//! appliquée par le cœur (`SetCmsField`, `AddCmsField`, `MoveCmsField`, `RemoveCmsField`,
//! `SetCmsCollection`), annulable ; sans `.pages.yml`, la première modification y décrit la
//! collection d'après ses contenus.

use crate::cms::{columns_of, primary_of};
use crate::cmsfields::{Cols, choices, label_of, slugify};
use crate::dialog::Ask;
use crate::site::capitalize;
use crate::{Ui, icons};
use leptos::ev;
use leptos::prelude::*;
use revel_edit::Command;
use revel_protocol::{CmsCollection, CmsField};
use serde_json::{Map, Value, json};
use wasm_bindgen::JsCast;

/// Type de champ de Pages CMS : nom écrit dans `.pages.yml`, clé de son libellé, icône, proposé
/// à la création.
pub struct FieldType {
    pub id: &'static str,
    pub key: &'static str,
    pub icon: fn() -> AnyView,
    pub create: bool,
}

pub const TYPES: &[FieldType] = &[
    FieldType {
        id: "string",
        key: "type-string",
        icon: icons::text,
        create: true,
    },
    FieldType {
        id: "text",
        key: "type-text",
        icon: icons::text_left,
        create: true,
    },
    FieldType {
        id: "rich-text",
        key: "type-rich-text",
        icon: icons::bold,
        create: true,
    },
    FieldType {
        id: "image",
        key: "type-image",
        icon: icons::image,
        create: true,
    },
    FieldType {
        id: "file",
        key: "type-file",
        icon: icons::page,
        create: true,
    },
    FieldType {
        id: "date",
        key: "type-date",
        icon: icons::calendar,
        create: true,
    },
    FieldType {
        id: "boolean",
        key: "type-boolean",
        icon: icons::toggle,
        create: true,
    },
    FieldType {
        id: "number",
        key: "type-number",
        icon: icons::hash,
        create: true,
    },
    FieldType {
        id: "select",
        key: "type-select",
        icon: icons::list,
        create: true,
    },
    FieldType {
        id: "reference",
        key: "type-reference",
        icon: icons::link,
        create: true,
    },
    FieldType {
        id: "code",
        key: "type-code",
        icon: icons::code,
        create: true,
    },
    FieldType {
        id: "object",
        key: "type-object",
        icon: icons::braces,
        create: true,
    },
    FieldType {
        id: "block",
        key: "type-block",
        icon: icons::component,
        create: true,
    },
    FieldType {
        id: "uuid",
        key: "type-uuid",
        icon: icons::hash,
        create: false,
    },
];

/// Type écrit dans `.pages.yml` (`rich_text` du protocole -> `rich-text`).
pub fn yaml_type(f: &CmsField) -> String {
    match f.kind.as_deref() {
        Some("rich_text") => "rich-text".into(),
        Some(k) => k.into(),
        None => "string".into(),
    }
}

pub fn type_of(id: &str) -> &'static FieldType {
    TYPES.iter().find(|t| t.id == id).unwrap_or(&TYPES[0])
}

/// Famille de types dont les valeurs restent lisibles d'un type à l'autre.
fn family(id: &str) -> &str {
    match id {
        "string" | "text" | "rich-text" | "code" | "uuid" => "text",
        "image" | "file" => "media",
        other => other,
    }
}

/// Nom de collection pour une référence (celui de `.pages.yml`, sinon l'identifiant).
fn ref_name(c: &CmsCollection) -> String {
    c.schema.clone().unwrap_or_else(|| c.id.clone())
}

/// Options d'une référence vers `c` : valeur écrite (adresse si la collection en a une) et
/// libellé (champ principal).
fn ref_options(c: &CmsCollection) -> Value {
    let value = if c.fields.iter().any(|f| f.name == "slug") {
        "{fields.slug}".to_string()
    } else {
        "{path}".to_string()
    };
    let label = primary_of(c)
        .map(|p| format!("{{fields.{p}}}"))
        .unwrap_or_else(|| "{name}".into());
    json!({ "collection": ref_name(c), "value": value, "label": label })
}

/// Nom d'un nouveau champ tiré de son libellé (`Date de début` -> `date_de_debut`), unique.
fn field_key(label: &str, taken: &[String]) -> String {
    let mut base = slugify(label).replace('-', "_");
    if base.starts_with(|c: char| c.is_ascii_digit()) {
        base = format!("champ_{base}");
    }
    let mut k = base.clone();
    let mut n = 2;
    while taken.contains(&k) {
        k = format!("{base}_{n}");
        n += 1;
    }
    k
}

/// Expression régulière JavaScript acceptée.
fn valid_regex(p: &str) -> bool {
    let Some(ctor) = js_sys::Reflect::get(&js_sys::global(), &"RegExp".into())
        .ok()
        .and_then(|c| c.dyn_into::<js_sys::Function>().ok())
    else {
        return true;
    };
    js_sys::Reflect::construct(&ctor, &js_sys::Array::of1(&p.into())).is_ok()
}

/// Liste des champs : type, libellé, nom dans les fichiers, marques (obligatoire, titre,
/// colonne, caché) ; poignée pour changer l'ordre.
#[component]
pub fn SchemaList(
    col: Memo<Option<CmsCollection>>,
    selected: RwSignal<Option<String>>,
    apply: Callback<Command>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let list = NodeRef::<leptos::html::Ul>::new();
    // Glisser : (champ déplacé, position d'insertion de 0 à n). Le champ est retenu par son nom :
    // la liste peut être relue pendant le geste.
    let drag = RwSignal::new(None::<(String, usize)>);
    let fields =
        Memo::new(move |_| col.with(|c| c.as_ref().map(|c| c.fields.clone()).unwrap_or_default()));
    let marks = Memo::new(move |_| {
        col.with(|c| {
            let c = c.as_ref()?;
            let primary = primary_of(c);
            let columns: Vec<String> = columns_of(c, primary.as_ref())
                .into_iter()
                .map(|f| f.name)
                .collect();
            Some((primary, columns))
        })
        .unwrap_or_default()
    });
    let folder =
        move || col.with_untracked(|c| c.as_ref().map(|c| c.folder.clone()).unwrap_or_default());
    let insertion = move |y: f64| -> usize {
        let Some(ul) = list.get_untracked() else {
            return 0;
        };
        let Ok(rows) = ul.query_selector_all(".schema-row") else {
            return 0;
        };
        for i in 0..rows.length() {
            if let Some(el) = rows
                .item(i)
                .and_then(|n| n.dyn_into::<web_sys::Element>().ok())
            {
                let r = el.get_bounding_client_rect();
                if y < r.top() + r.height() / 2.0 {
                    return i as usize;
                }
            }
        }
        rows.length() as usize
    };
    // Suivi du geste sur toute la fenêtre (la ligne saisie peut être redessinée).
    let on_move = window_event_listener(ev::pointermove, move |e: web_sys::PointerEvent| {
        if let Some((name, at)) = drag.get_untracked() {
            let to = insertion(e.client_y() as f64);
            if to != at {
                drag.set(Some((name, to)));
            }
        }
    });
    let on_up = window_event_listener(ev::pointerup, move |_| {
        let Some((name, at)) = drag.get_untracked() else {
            return;
        };
        drag.set(None);
        let Some(from) = fields.with_untracked(|f| f.iter().position(|x| x.name == name)) else {
            return;
        };
        let to = if at > from { at - 1 } else { at };
        if to != from {
            apply.run(Command::MoveCmsField {
                folder: folder(),
                field: name,
                index: to,
            });
        }
    });
    let on_cancel = window_event_listener(ev::pointercancel, move |_| drag.set(None));
    on_cleanup(move || {
        on_move.remove();
        on_up.remove();
        on_cancel.remove();
    });
    // Menu contextuel d'un champ : monter, descendre, retirer.
    let row_menu = move |e: &ev::MouseEvent, i: usize, n: usize, field: String, label: String| {
        use crate::ctxmenu::{CtxItem, open};
        selected.set(Some(field.clone()));
        let (f1, f2, f3) = (field.clone(), field.clone(), field);
        let items = vec![
            CtxItem::new(ui.now("cms-field-up", &[]), move || {
                apply.run(Command::MoveCmsField {
                    folder: folder(),
                    field: f1.clone(),
                    index: i.saturating_sub(1),
                })
            })
            .icon(icons::arrow_up)
            .disabled(i == 0),
            CtxItem::new(ui.now("cms-field-down", &[]), move || {
                apply.run(Command::MoveCmsField {
                    folder: folder(),
                    field: f2.clone(),
                    index: i + 1,
                })
            })
            .icon(icons::arrow_down)
            .disabled(i + 1 >= n),
            CtxItem::new(ui.now("cms-field-remove", &[]), move || {
                let f3 = f3.clone();
                ui.ask.set(Some(
                    Ask::confirm(
                        ui.now("cms-field-remove-title", &[("name", label.clone())]),
                        Some(ui.now("cms-field-remove-body", &[])),
                        ui.now("cms-field-remove", &[]),
                        move || {
                            selected.set(None);
                            apply.run(Command::RemoveCmsField {
                                folder: folder(),
                                field: f3.clone(),
                            });
                        },
                    )
                    .danger(),
                ));
            })
            .icon(icons::trash)
            .danger()
            .sep(),
        ];
        open(ui, e, items);
    };
    view! {
        <div class="schema">
            {move || col.with(|c| c.as_ref().is_some_and(|c| c.schema.is_none())).then(|| view! {
                <p class="schema-note">{icons::info()}<span>{ui.t("cms-schema-inferred")}</span></p>
            })}
            <ul class="schema-list" node_ref=list class:is-dragging=move || drag.get().is_some()>
                {move || {
                    let (primary, columns) = marks.get();
                    let d = drag.get();
                    let all = fields.get();
                    let n = all.len();
                    let from = d.as_ref().and_then(|(name, _)| all.iter().position(|f| f.name == *name));
                    let at = d.as_ref().map(|(_, at)| *at);
                    all.into_iter().enumerate().map(|(i, f)| {
                        let (name, n2, n3) = (f.name.clone(), f.name.clone(), f.name.clone());
                        let t = type_of(&yaml_type(&f));
                        let is_primary = primary.as_deref() == Some(f.name.as_str());
                        let is_column = columns.contains(&f.name);
                        let moves = |a: usize| from.is_some_and(|fr| a != fr && a != fr + 1);
                        let before = at == Some(i) && moves(i);
                        let after = at == Some(n) && i + 1 == n && moves(n);
                        view! {
                            <li class="schema-row" data-field=f.name.clone()
                                class:is-on=move || selected.get().as_deref() == Some(n2.as_str())
                                class:is-drop-before=before class:is-drop-after=after
                                class:is-dragged=from == Some(i)
                                on:contextmenu={
                                    let (field, label) = (f.name.clone(), label_of(&f));
                                    move |e: ev::MouseEvent| row_menu(&e, i, n, field.clone(), label.clone())
                                }
                                on:click=move |_| selected.set(Some(name.clone()))>
                                <span class="schema-grip" title=ui.t("cms-field-drag")
                                    on:pointerdown=move |e: ev::PointerEvent| {
                                        if e.button() != 0 { return; }
                                        e.prevent_default();
                                        drag.set(Some((n3.clone(), i)));
                                    }>{icons::grip()}</span>
                                <span class="schema-type" title=ui.t(t.key)>{(t.icon)()}</span>
                                <span class="schema-label">
                                    <span class="schema-label-text">{label_of(&f)}</span>
                                    {f.required.then(|| view! { <span class="cms-req" title=ui.t("cms-required-mark")>"*"</span> })}
                                </span>
                                <span class="schema-name">{f.name.clone()}</span>
                                <span class="schema-marks">
                                    {is_primary.then(|| view! { <span class="chip" title=ui.t("cms-field-primary-help")>{ui.t("cms-field-primary-chip")}</span> })}
                                    {is_column.then(|| view! { <span class="chip" title=ui.t("cms-field-column")>{ui.t("cms-field-column-chip")}</span> })}
                                    {f.hidden.then(|| view! { <span class="schema-hidden" title=ui.t("cms-field-hidden")>{icons::eye_off()}</span> })}
                                </span>
                            </li>
                        }
                    }).collect_view()
                }}
            </ul>
        </div>
    }
}

/// Ligne de réglage à interrupteur.
#[component]
fn SwitchRow(
    label: &'static str,
    #[prop(optional)] help: Option<&'static str>,
    checked: Signal<bool>,
    set: Callback<bool>,
    #[prop(optional)] name: &'static str,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    view! {
        <label class="cms-item-field is-inline" data-setting=name>
            <span class="label cms-label" title=move || help.map(|h| ui.tx(h, &[])).unwrap_or_default()>
                <span class="cms-label-text">{ui.t(label)}</span>
                {help.is_some().then(icons::info)}
            </span>
            <span class="switch">
                <input type="checkbox" prop:checked=move || checked.get()
                    on:change=move |e| set.run(event_target_checked(&e))/>
                <span class="switch-track"></span>
            </span>
        </label>
    }
}

/// Réglages du champ choisi.
#[component]
pub fn FieldSettings(
    col: Memo<Option<CmsCollection>>,
    cols: Cols,
    name: String,
    apply: Callback<Command>,
    close: Callback<()>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let name = StoredValue::new(name);
    let field = Memo::new(move |_| {
        col.with(|c| {
            c.as_ref()?
                .fields
                .iter()
                .find(|f| f.name == name.get_value())
                .cloned()
        })
    });
    let folder =
        move || col.with_untracked(|c| c.as_ref().map(|c| c.folder.clone()).unwrap_or_default());
    let set = move |patch: Value| {
        apply.run(Command::SetCmsField {
            folder: folder(),
            field: name.get_value(),
            patch,
        })
    };
    let set_view = move |patch: Value| {
        apply.run(Command::SetCmsCollection {
            folder: folder(),
            patch: json!({ "view": patch }),
        })
    };
    let kind = Memo::new(move |_| {
        field
            .with(|f| f.as_ref().map(yaml_type))
            .unwrap_or_default()
    });
    let component =
        Memo::new(move |_| field.with(|f| f.as_ref().and_then(|f| f.component.clone())));
    let flag = move |g: fn(&CmsField) -> bool| {
        Signal::derive(move || field.with(|f| f.as_ref().is_some_and(g)))
    };
    let position = Memo::new(move |_| {
        col.with(|c| {
            c.as_ref()
                .and_then(|c| c.fields.iter().position(|f| f.name == name.get_value()))
        })
    });
    let count = move || col.with(|c| c.as_ref().map_or(0, |c| c.fields.len()));
    let move_to = move |index: usize| {
        apply.run(Command::MoveCmsField {
            folder: folder(),
            field: name.get_value(),
            index,
        })
    };
    let label = move || field.with(|f| f.as_ref().map(label_of)).unwrap_or_default();
    let text_of = move |g: fn(&CmsField) -> Option<String>| {
        move || field.with(|f| f.as_ref().and_then(g)).unwrap_or_default()
    };
    // Colonnes et titre du tableau (vue de la collection).
    let marks = Memo::new(move |_| {
        col.with(|c| {
            let c = c.as_ref()?;
            let primary = primary_of(c);
            let columns: Vec<String> = columns_of(c, primary.as_ref())
                .into_iter()
                .map(|f| f.name)
                .collect();
            Some((primary, columns))
        })
        .unwrap_or_default()
    });
    let is_column = Signal::derive(move || marks.with(|(_, c)| c.contains(&name.get_value())));
    let is_primary = Signal::derive(move || {
        marks.with(|(p, _)| p.as_deref() == Some(name.get_value().as_str()))
    });
    let set_column = Callback::new(move |on: bool| {
        let (primary, mut columns) = marks.get_untracked();
        // Les colonnes du tableau commencent par son titre (comme dans Pages CMS).
        if let Some(p) = &primary
            && !columns.contains(p)
        {
            columns.insert(0, p.clone());
        }
        let n = name.get_value();
        if on && !columns.contains(&n) {
            columns.push(n);
        } else if !on {
            columns.retain(|c| *c != n);
        }
        set_view(json!({ "fields": columns }));
    });
    let set_primary = Callback::new(move |on: bool| {
        set_view(
            json!({ "primary": if on { Value::String(name.get_value()) } else { Value::Null } }),
        );
    });
    let change_type = move |new: String| {
        let old = kind.get_untracked();
        if new == old {
            return;
        }
        let used = col.with_untracked(|c| {
            c.as_ref().is_some_and(|c| {
                c.entries
                    .iter()
                    .any(|e| e.data.get(name.get_value()).is_some_and(|v| !v.is_null()))
            })
        });
        // Options propres à l'ancien type retirées.
        let patch = json!({ "type": new, "options": null });
        if used && family(&old) != family(&new) {
            ui.ask.set(Some(Ask::confirm(
                ui.now("cms-field-retype-title", &[("name", label())]),
                Some(ui.now("cms-field-retype-body", &[])),
                ui.now("cms-field-retype", &[]),
                move || set(patch.clone()),
            )));
        } else {
            set(patch);
        }
    };
    let remove = move |_| {
        ui.ask.set(Some(
            Ask::confirm(
                ui.now("cms-field-remove-title", &[("name", label())]),
                Some(ui.now("cms-field-remove-body", &[])),
                ui.now("cms-field-remove", &[]),
                move || {
                    close.run(());
                    apply.run(Command::RemoveCmsField {
                        folder: folder(),
                        field: name.get_value(),
                    });
                },
            )
            .danger(),
        ));
    };
    let multi = Signal::derive(move || {
        field.with(|f| {
            f.as_ref().is_some_and(|f| {
                f.list
                    || matches!(
                        f.options.get("multiple"),
                        Some(Value::Bool(true) | Value::Object(_))
                    )
            })
        })
    });
    view! {
        <aside class="cms-item schema-item" aria-label=ui.t("cms-fields-edit") data-field=name.get_value()>
            <header class="cms-item-head">
                <h2>{label}</h2>
                <button class="icon-btn" title=ui.t("cms-field-up") aria-label=ui.t("cms-field-up")
                    prop:disabled=move || position.get().is_none_or(|p| p == 0)
                    on:click=move |_| if let Some(p) = position.get_untracked() { move_to(p.saturating_sub(1)) }>{icons::arrow_up()}</button>
                <button class="icon-btn" title=ui.t("cms-field-down") aria-label=ui.t("cms-field-down")
                    prop:disabled=move || position.get().is_none_or(|p| p + 1 >= count())
                    on:click=move |_| if let Some(p) = position.get_untracked() { move_to(p + 1) }>{icons::arrow_down()}</button>
                <button class="icon-btn" title=ui.t("action-close") aria-label=ui.t("action-close")
                    on:click=move |_| close.run(())>{icons::close()}</button>
            </header>
            <div class="cms-item-body">
                <label class="cms-item-field" data-setting="label">
                    <span class="label">{ui.t("cms-field-label")}</span>
                    <input class="input" type="text" prop:value=text_of(|f| f.label.clone())
                        placeholder=move || field.with(|f| f.as_ref().map(label_of)).unwrap_or_default()
                        on:change=move |e| {
                            let v = event_target_value(&e).trim().to_string();
                            set(json!({ "label": if v.is_empty() { Value::Null } else { Value::String(v) } }))
                        }/>
                </label>
                <label class="cms-item-field" data-setting="name">
                    <span class="label cms-label" title=ui.t("cms-field-name-help")>
                        <span class="cms-label-text">{ui.t("cms-field-name")}</span>{icons::info()}
                    </span>
                    <input class="input input--mono" type="text" readonly=true prop:value=move || name.get_value()/>
                </label>
                <div class="cms-item-field" data-setting="type">
                    <span class="label cms-label" title=move || component.get().map(|c| ui.tx("cms-field-component", &[("name", c)])).unwrap_or_default()>
                        <span class="cms-label-text">{ui.t("cms-field-type")}</span>
                        {move || component.get().is_some().then(icons::info)}
                    </span>
                    <div class="select">
                        <select class="input" prop:disabled=move || component.get().is_some()
                            on:change=move |e| change_type(event_target_value(&e))>
                            {move || {
                                let cur = kind.get();
                                TYPES.iter().filter(|t| t.create || t.id == cur).map(|t| {
                                    let on = t.id == cur;
                                    view! { <option value=t.id prop:selected=on>{ui.t(t.key)}</option> }
                                }).collect_view()
                            }}
                        </select>
                    </div>
                </div>
                <label class="cms-item-field" data-setting="description">
                    <span class="label">{ui.t("cms-field-help")}</span>
                    <textarea class="input cms-textarea" rows="2" placeholder=ui.t("cms-field-help-placeholder")
                        prop:value=text_of(|f| f.description.clone())
                        on:change=move |e| {
                            let v = event_target_value(&e).trim().to_string();
                            set(json!({ "description": if v.is_empty() { Value::Null } else { Value::String(v) } }))
                        }></textarea>
                </label>
                <div class="schema-switches">
                    <SwitchRow label="cms-field-required" name="required" checked=flag(|f| f.required)
                        set=Callback::new(move |on: bool| set(json!({ "required": if on { Value::Bool(true) } else { Value::Null } })))/>
                    {move || (!matches!(kind.get().as_str(), "select" | "reference" | "boolean" | "object" | "block")).then(|| view! {
                        <SwitchRow label="cms-field-list" name="list" checked=multi
                            set=Callback::new(move |on: bool| {
                                if on {
                                    set(json!({ "list": true }))
                                } else {
                                    set(json!({ "list": null, "options": { "multiple": null } }))
                                }
                            })/>
                    })}
                    <SwitchRow label="cms-field-hidden" help="cms-field-hidden-help" name="hidden" checked=flag(|f| f.hidden)
                        set=Callback::new(move |on: bool| set(json!({ "hidden": if on { Value::Bool(true) } else { Value::Null } })))/>
                    <SwitchRow label="cms-field-column" name="column" checked=is_column set=set_column/>
                    {move || matches!(kind.get().as_str(), "string" | "text" | "date" | "number").then(|| view! {
                        <SwitchRow label="cms-field-primary" help="cms-field-primary-help" name="primary" checked=is_primary set=set_primary/>
                    })}
                </div>
                {move || type_options(kind.get(), field, cols, Callback::new(set))}
            </div>
            <footer class="cms-item-foot">
                <span class="muted path cms-item-file" title=".pages.yml">".pages.yml"</span>
                <button class="btn btn--danger-ghost" on:click=remove>{icons::trash()}{ui.t("cms-field-remove")}</button>
            </footer>
        </aside>
    }
}

/// Réglages propres au type du champ.
fn type_options(
    kind: String,
    field: Memo<Option<CmsField>>,
    cols: Cols,
    set: Callback<Value>,
) -> AnyView {
    let ui = expect_context::<Ui>();
    let opt =
        move |k: &'static str| field.with(|f| f.as_ref().and_then(|f| f.options.get(k).cloned()));
    match kind.as_str() {
        "date" => {
            let time = Signal::derive(move || opt("time") == Some(Value::Bool(true)));
            view! {
                <div class="schema-switches">
                    <SwitchRow label="cms-field-time" name="time" checked=time set=Callback::new(move |on: bool| {
                        // Format d'écriture accordé à l'heure.
                        let fmt = opt("format").and_then(|v| v.as_str().map(str::to_string));
                        let mut o = Map::new();
                        o.insert("time".into(), if on { Value::Bool(true) } else { Value::Null });
                        match (on, fmt.as_deref()) {
                            (true, Some("yyyy-MM-dd")) => { o.insert("format".into(), "yyyy-MM-dd'T'HH:mm:ss".into()); }
                            (false, Some(f)) if f.contains('H') => { o.insert("format".into(), "yyyy-MM-dd".into()); }
                            _ => {}
                        }
                        set.run(json!({ "options": o }))
                    })/>
                </div>
            }
            .into_any()
        }
        "rich-text" => {
            let cur = move || {
                opt("format")
                    .and_then(|v| v.as_str().map(str::to_lowercase))
                    .unwrap_or_else(|| "html".into())
            };
            view! {
                <div class="cms-item-field is-inline" data-setting="format">
                    <span class="label">{ui.t("cms-field-format")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| set.run(json!({ "options": { "format": event_target_value(&e) } }))>
                            <option value="html" prop:selected=move || cur() == "html">"HTML"</option>
                            <option value="markdown" prop:selected=move || cur() == "markdown">"Markdown"</option>
                        </select>
                    </div>
                </div>
            }
            .into_any()
        }
        "select" => {
            let values = move || field.with(|f| f.as_ref().map(choices).unwrap_or_default());
            let raw = move || {
                opt("values")
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default()
            };
            let multiple = Signal::derive(move || opt("multiple") == Some(Value::Bool(true)));
            view! {
                <div class="cms-item-field" data-setting="values">
                    <span class="label">{ui.t("cms-field-choices")}</span>
                    <div class="tags-input">
                        {move || values().into_iter().enumerate().map(|(i, (v, l))| view! {
                            <span class="chip chip--tag" title=v>{l}
                                <button type="button" class="chip-x" title=ui.t("action-delete") aria-label=ui.t("action-delete")
                                    on:click=move |_| {
                                        let mut list = raw();
                                        if i < list.len() { list.remove(i); }
                                        set.run(json!({ "options": { "values": list } }))
                                    }>{icons::close()}</button>
                            </span>
                        }).collect_view()}
                        <input class="tags-add" type="text" placeholder=ui.t("cms-field-choice-add")
                            on:keydown=move |e: ev::KeyboardEvent| {
                                if e.key() != "Enter" { return; }
                                e.prevent_default();
                                let input = event_target::<web_sys::HtmlInputElement>(&e);
                                let label = input.value().trim().to_string();
                                if label.is_empty() { return; }
                                input.set_value("");
                                let mut list = raw();
                                // Écriture des choix existants gardée (textes simples ou valeur et libellé).
                                let simple = !list.is_empty() && list.iter().all(Value::is_string);
                                let item = if simple { Value::String(label.clone()) } else { json!({ "name": slugify(&label), "label": label }) };
                                let key = if simple { label } else { slugify(&label) };
                                if values().iter().any(|(v, _)| *v == key) { return; }
                                list.push(item);
                                set.run(json!({ "options": { "values": list } }))
                            }/>
                    </div>
                </div>
                <div class="schema-switches">
                    <SwitchRow label="cms-field-multiple" name="multiple" checked=multiple
                        set=Callback::new(move |on: bool| set.run(json!({ "options": { "multiple": if on { Value::Bool(true) } else { Value::Null } } })))/>
                </div>
            }
            .into_any()
        }
        "reference" => {
            let target = move || {
                opt("collection")
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default()
            };
            let multiple = Signal::derive(move || {
                matches!(opt("multiple"), Some(Value::Bool(true) | Value::Object(_)))
            });
            let target_fields = move || {
                let t = target();
                cols.with(|c| {
                    c.as_deref()
                        .unwrap_or_default()
                        .iter()
                        .find(|c| ref_name(c) == t)
                        .map(|c| c.fields.clone())
                        .unwrap_or_default()
                })
            };
            let shown = move || {
                opt("label")
                    .and_then(|v| v.as_str().map(str::to_string))
                    .and_then(|l| {
                        l.strip_prefix("{fields.")
                            .and_then(|x| x.strip_suffix('}'))
                            .map(str::to_string)
                    })
                    .unwrap_or_default()
            };
            view! {
                <div class="cms-item-field" data-setting="collection">
                    <span class="label">{ui.t("cms-field-collection")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| {
                            let v = event_target_value(&e);
                            let opts = cols.with_untracked(|c| c.as_deref().unwrap_or_default().iter().find(|c| ref_name(c) == v).map(ref_options));
                            if let Some(o) = opts { set.run(json!({ "options": o })) }
                        }>
                            {move || {
                                let t = target();
                                let mut list: Vec<(String, String)> = cols.with(|c| c.as_deref().unwrap_or_default().iter()
                                    .map(|c| (ref_name(c), c.label.clone().unwrap_or_else(|| capitalize(&c.id)))).collect());
                                if !t.is_empty() && !list.iter().any(|(v, _)| *v == t) {
                                    list.insert(0, (t.clone(), t.clone()));
                                }
                                let none = t.is_empty().then(|| view! { <option value="" prop:selected=true>"—"</option> });
                                view! {
                                    {none}
                                    {list.into_iter().map(|(v, l)| { let on = v == t; view! { <option value=v prop:selected=on>{l}</option> } }).collect_view()}
                                }
                            }}
                        </select>
                    </div>
                </div>
                <div class="cms-item-field" data-setting="ref-label">
                    <span class="label">{ui.t("cms-field-ref-label")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| {
                            let v = event_target_value(&e);
                            set.run(json!({ "options": { "label": format!("{{fields.{v}}}") } }))
                        }>
                            {move || {
                                let cur = shown();
                                target_fields().into_iter()
                                    .filter(|f| matches!(f.kind.as_deref(), None | Some("string" | "text" | "number" | "date")))
                                    .map(|f| { let on = f.name == cur; let n = f.name.clone(); view! { <option value=n prop:selected=on>{label_of(&f)}</option> } })
                                    .collect_view()
                            }}
                        </select>
                    </div>
                </div>
                <div class="schema-switches">
                    <SwitchRow label="cms-field-multiple" name="multiple" checked=multiple
                        set=Callback::new(move |on: bool| set.run(json!({ "options": { "multiple": if on { Value::Bool(true) } else { Value::Null } } })))/>
                </div>
            }
            .into_any()
        }
        "string" => {
            let error = RwSignal::new(false);
            view! {
                <label class="cms-item-field" class:is-invalid=move || error.get() data-setting="pattern">
                    <span class="label cms-label" title=ui.t("cms-field-pattern-help")>
                        <span class="cms-label-text">{ui.t("cms-field-pattern")}</span>{icons::info()}
                    </span>
                    <input class="input input--mono" type="text" spellcheck="false"
                        prop:value=move || field.with(|f| f.as_ref().and_then(|f| f.pattern.clone())).unwrap_or_default()
                        on:change=move |e| {
                            let v = event_target_value(&e).trim().to_string();
                            if !v.is_empty() && !valid_regex(&v) {
                                error.set(true);
                                return;
                            }
                            error.set(false);
                            set.run(json!({ "pattern": if v.is_empty() { Value::Null } else { Value::String(v) } }))
                        }/>
                    {move || error.get().then(|| view! { <span class="field-error" role="alert">{ui.t("cms-field-pattern-invalid")}</span> })}
                </label>
            }
            .into_any()
        }
        "object" => {
            let comp = field.with_untracked(|f| f.as_ref().and_then(|f| f.component.clone()));
            let subs = Signal::derive(move || {
                field.with(|f| f.as_ref().map(|f| f.fields.clone()).unwrap_or_default())
            });
            view! {
                <div class="cms-item-field" data-setting="fields">
                    <span class="label">{ui.t("cms-field-subfields")}</span>
                    <SubFields fields=subs locked=comp.is_some()
                        on_change=Callback::new(move |defs: Vec<Value>| set.run(json!({ "fields": defs })))/>
                </div>
            }
            .into_any()
        }
        "block" => {
            let comp = field.with_untracked(|f| f.as_ref().is_some_and(|f| f.component.is_some()));
            // Définitions d'origine des types de blocs (réécrites telles quelles).
            let raw = move || -> Vec<Value> {
                field.with_untracked(|f| {
                    f.as_ref()
                        .and_then(|f| f.options.get("blocks").and_then(Value::as_array).cloned())
                        .unwrap_or_default()
                })
            };
            let types = Signal::derive(move || {
                field.with(|f| f.as_ref().map(|f| f.fields.clone()).unwrap_or_default())
            });
            let write = move |blocks: Vec<Value>| set.run(json!({ "blocks": blocks }));
            let add = move |_| {
                let taken: Vec<String> = types
                    .get_untracked()
                    .iter()
                    .map(|b| b.name.clone())
                    .collect();
                ui.ask.set(Some(Ask::input(
                    ui.now("cms-block-new", &[]),
                    ui.now("cms-field-label", &[]),
                    String::new(),
                    ui.now("action-create", &[]),
                    move |label| {
                        let label = label.trim().to_string();
                        if label.is_empty() {
                            return;
                        }
                        let mut blocks = raw();
                        blocks.push(json!({
                            "name": field_key(&label, &taken),
                            "label": label,
                            "fields": [{ "name": "texte", "label": ui.now("cms-sub-default", &[]), "type": "string" }]
                        }));
                        write(blocks);
                    },
                )));
            };
            view! {
                <div class="cms-item-field" data-setting="blocks">
                    <span class="label">{ui.t("cms-field-blocks")}</span>
                    {move || types.get().into_iter().enumerate().map(|(i, b)| {
                        let label = label_of(&b);
                        let from_component = b.component.is_some() || comp;
                        let subs = Signal::derive(move || types.with(|t| t.get(i).map(|b| b.fields.clone()).unwrap_or_default()));
                        view! {
                            <div class="schema-block" data-block=b.name.clone()>
                                <div class="schema-block-head">
                                    <span class="schema-block-name">{icons::component()}{label}</span>
                                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("cms-block-remove") aria-label=ui.t("cms-block-remove")
                                        disabled=comp
                                        on:click=move |_| {
                                            let mut blocks = raw();
                                            if i < blocks.len() {
                                                blocks.remove(i);
                                                write(blocks);
                                            }
                                        }>{icons::trash()}</button>
                                </div>
                                <SubFields fields=subs locked=from_component
                                    on_change=Callback::new(move |defs: Vec<Value>| {
                                        let mut blocks = raw();
                                        if let Some(Value::Object(o)) = blocks.get_mut(i) {
                                            o.insert("fields".into(), Value::Array(defs));
                                            write(blocks);
                                        }
                                    })/>
                            </div>
                        }
                    }).collect_view()}
                    {(!comp).then(|| view! {
                        <button type="button" class="btn btn--small schema-block-add" on:click=add>{icons::plus()}{ui.t("cms-block-add")}</button>
                    })}
                </div>
            }
            .into_any()
        }
        _ => ().into_any(),
    }
}

/// « Nouveau champ » : choix du type, puis du libellé ; le nom dans les fichiers en est tiré.
#[component]
pub fn NewField(
    col: Memo<Option<CmsCollection>>,
    cols: Cols,
    apply: Callback<Command>,
    selected: RwSignal<Option<String>>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let open = RwSignal::new(false);
    let on_down = window_event_listener(ev::pointerdown, move |e: web_sys::PointerEvent| {
        let inside = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest(".new-field").ok().flatten())
            .is_some();
        if !inside {
            open.set(false);
        }
    });
    let on_key = window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" {
            open.set(false);
        }
    });
    on_cleanup(move || {
        on_down.remove();
        on_key.remove();
    });
    let pick = move |t: &'static FieldType| {
        open.set(false);
        let Some(c) = col.get_untracked() else {
            return;
        };
        let taken: Vec<String> = c.fields.iter().map(|f| f.name.clone()).collect();
        let others = cols.get_untracked().unwrap_or_default();
        ui.ask.set(Some(Ask::input(
            ui.now("cms-field-new-title", &[("type", ui.now(t.key, &[]))]),
            ui.now("cms-field-label", &[]),
            String::new(),
            ui.now("action-create", &[]),
            move |label| {
                let label = label.trim().to_string();
                if label.is_empty() {
                    return;
                }
                let key = field_key(&label, &taken);
                let mut def = json!({ "name": key, "label": label, "type": t.id });
                match t.id {
                    "select" => def["options"] = json!({ "values": [] }),
                    // Groupe de champs : un premier champ texte ; blocs : un type « Texte ».
                    "object" => {
                        def["fields"] = json!([{ "name": "texte", "label": ui.now("cms-sub-default", &[]), "type": "string" }])
                    }
                    "block" => {
                        def["list"] = json!(true);
                        def["blockKey"] = json!("type");
                        def["blocks"] = json!([{
                            "name": "texte",
                            "label": ui.now("cms-sub-default", &[]),
                            "fields": [{ "name": "body", "label": ui.now("cms-sub-default", &[]), "type": "rich-text" }]
                        }]);
                    }
                    "reference" => {
                        // Collection proposée : une autre que celle-ci.
                        if let Some(o) = others
                            .iter()
                            .find(|o| o.folder != c.folder)
                            .or_else(|| others.first())
                        {
                            def["options"] = ref_options(o);
                        }
                    }
                    _ => {}
                }
                selected.set(Some(key));
                apply.run(Command::AddCmsField {
                    folder: c.folder.clone(),
                    def,
                    index: None,
                });
            },
        )));
    };
    view! {
        <div class="new-field">
            <button class="btn btn--primary" aria-haspopup="menu" aria-expanded=move || open.get().to_string()
                on:click=move |_| open.update(|o| *o = !*o)>
                {icons::plus()}{ui.t("cms-field-new")}
            </button>
            <Show when=move || open.get()>
                <div class="menu new-field-menu" role="menu">
                    {TYPES.iter().filter(|t| t.create || t.id == "object" || t.id == "block").map(|t| {
                        let planned = !t.create;
                        view! {
                            <button class="menu-item" role="menuitem" data-type=t.id disabled=planned
                                title=move || if planned { ui.tx("planned-step", &[("step", "3".to_string())]) } else { String::new() }
                                on:click=move |_| pick(t)>
                                {(t.icon)()}<span class="menu-label">{ui.t(t.key)}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}

/// Définition `.pages.yml` d'un champ (inverse de la lecture par le cœur).
pub fn to_def(f: &CmsField) -> Value {
    let mut m = Map::new();
    m.insert("name".into(), Value::String(f.name.clone()));
    if let Some(c) = &f.component {
        m.insert("component".into(), Value::String(c.clone()));
        if let Some(l) = &f.label {
            m.insert("label".into(), Value::String(l.clone()));
        }
        return Value::Object(m);
    }
    if let Some(l) = &f.label {
        m.insert("label".into(), Value::String(l.clone()));
    }
    m.insert("type".into(), Value::String(yaml_type(f)));
    if let Some(d) = &f.description {
        m.insert("description".into(), Value::String(d.clone()));
    }
    if f.required {
        m.insert("required".into(), Value::Bool(true));
    }
    if f.hidden {
        m.insert("hidden".into(), Value::Bool(true));
    }
    if f.list {
        m.insert("list".into(), Value::Bool(true));
    }
    if let Some(d) = &f.default {
        m.insert("default".into(), d.clone());
    }
    if let Some(p) = &f.pattern {
        m.insert("pattern".into(), Value::String(p.clone()));
    }
    let mut options = f.options.as_object().cloned().unwrap_or_default();
    if f.kind.as_deref() == Some("block") {
        if let Some(b) = options.remove("blocks") {
            m.insert("blocks".into(), b);
        }
        if let Some(k) = options.remove("blockKey") {
            m.insert("blockKey".into(), k);
        }
    } else if !f.fields.is_empty() {
        m.insert(
            "fields".into(),
            Value::Array(f.fields.iter().map(to_def).collect()),
        );
    }
    if !options.is_empty() {
        m.insert("options".into(), Value::Object(options));
    }
    Value::Object(m)
}

/// Types proposés pour un champ d'un groupe ou d'un bloc.
const SUB_TYPES: [&str; 8] = [
    "string",
    "text",
    "rich-text",
    "image",
    "date",
    "boolean",
    "number",
    "file",
];

/// Champs d'un groupe ou d'un type de bloc : libellé, type, suppression, ajout ; `locked` :
/// définis par un composant de `.pages.yml` (lecture seule).
#[component]
fn SubFields(
    #[prop(into)] fields: Signal<Vec<CmsField>>,
    locked: bool,
    on_change: Callback<Vec<Value>>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let defs = move || {
        fields
            .get_untracked()
            .iter()
            .map(to_def)
            .collect::<Vec<Value>>()
    };
    let add = move |_| {
        let taken: Vec<String> = fields
            .get_untracked()
            .iter()
            .map(|f| f.name.clone())
            .collect();
        ui.ask.set(Some(Ask::input(
            ui.now("cms-sub-new", &[]),
            ui.now("cms-field-label", &[]),
            String::new(),
            ui.now("action-create", &[]),
            move |label| {
                let label = label.trim().to_string();
                if label.is_empty() {
                    return;
                }
                let mut d = defs();
                d.push(
                    json!({ "name": field_key(&label, &taken), "label": label, "type": "string" }),
                );
                on_change.run(d);
            },
        )));
    };
    view! {
        <div class="schema-subfields">
            {move || fields.get().into_iter().enumerate().map(|(i, f)| {
                let kind = yaml_type(&f);
                let simple = SUB_TYPES.contains(&kind.as_str());
                let label = label_of(&f);
                view! {
                    <div class="schema-sub" data-sub=f.name.clone()>
                        <input class="input schema-sub-label" prop:value=label disabled=locked
                            aria-label=ui.t("cms-field-label") title=f.name.clone()
                            on:change=move |e| {
                                let v = event_target_value(&e).trim().to_string();
                                if v.is_empty() { return; }
                                let mut d = defs();
                                if let Some(Value::Object(o)) = d.get_mut(i) {
                                    o.insert("label".into(), Value::String(v));
                                    on_change.run(d);
                                }
                            }/>
                        <div class="select schema-sub-type">
                            <select class="input" disabled={locked || !simple} aria-label=ui.t("cms-field-type")
                                on:change=move |e| {
                                    let v = event_target_value(&e);
                                    let mut d = defs();
                                    if let Some(Value::Object(o)) = d.get_mut(i) {
                                        o.insert("type".into(), Value::String(v));
                                        o.remove("options");
                                        on_change.run(d);
                                    }
                                }>
                                {(!simple).then(|| { let k = kind.clone(); view! { <option value=k.clone() selected=true>{k.clone()}</option> } })}
                                {SUB_TYPES.iter().map(|t| {
                                    let on = *t == kind;
                                    let key = type_of(t).key;
                                    view! { <option value=*t prop:selected=on>{ui.t(key)}</option> }
                                }).collect_view()}
                            </select>
                        </div>
                        <button type="button" class="icon-btn icon-btn--small" title=ui.t("cms-sub-remove") aria-label=ui.t("cms-sub-remove")
                            disabled=locked
                            on:click=move |_| {
                                let mut d = defs();
                                if i < d.len() {
                                    d.remove(i);
                                    on_change.run(d);
                                }
                            }>{icons::trash()}</button>
                    </div>
                }
            }).collect_view()}
            {(!locked).then(|| view! {
                <button type="button" class="btn btn--small schema-sub-add" on:click=add>{icons::plus()}{ui.t("cms-sub-add")}</button>
            })}
            {locked.then(|| view! { <p class="muted small" title=ui.t("cms-sub-locked")>{ui.t("cms-sub-locked-short")}</p> })}
        </div>
    }
}
