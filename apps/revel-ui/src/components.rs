//! Composants dans l'éditeur (3.6, D-34), comme ceux de Webflow : réglages d'une utilisation
//! (valeurs de ses propriétés, ouvrir le composant, détacher), propriétés du composant ouvert
//! (ajouter, renommer, valeur par défaut, supprimer), éléments du composant reliés à une
//! propriété (texte, image, lien, visibilité), création d'un composant à partir de la sélection.

use crate::editor::{Ed, Selection};
use crate::inspector::Field;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::canvas::{literal, quote};
use revel_edit::{Branch, Command, Owner, Parent, Target};
use revel_model::{Component, Node, PropDef, PropKind};
use serde_json::Value;
use std::collections::BTreeMap;

const KINDS: [PropKind; 4] = [
    PropKind::Text,
    PropKind::Image,
    PropKind::Link,
    PropKind::Switch,
];

fn kind_key(k: PropKind) -> &'static str {
    match k {
        PropKind::Text => "prop-kind-text",
        PropKind::Image => "prop-kind-image",
        PropKind::Link => "prop-kind-link",
        PropKind::Switch => "prop-kind-switch",
    }
}

fn kind_icon(k: PropKind) -> AnyView {
    match k {
        PropKind::Text => icons::text(),
        PropKind::Image => icons::image(),
        PropKind::Link => icons::link(),
        PropKind::Switch => icons::eye(),
    }
}

fn kind_icon_fn(k: PropKind) -> fn() -> AnyView {
    match k {
        PropKind::Text => icons::text,
        PropKind::Image => icons::image,
        PropKind::Link => icons::link,
        PropKind::Switch => icons::eye,
    }
}

/// Début du nom d'une nouvelle propriété (identifiant dans les expressions).
fn base_name(k: PropKind) -> &'static str {
    match k {
        PropKind::Text => "texte",
        PropKind::Image => "image",
        PropKind::Link => "lien",
        PropKind::Switch => "visible",
    }
}

/// Valeur par défaut en texte.
fn default_text(p: &PropDef) -> String {
    match &p.default {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string(),
    }
}

fn default_bool(p: &PropDef) -> bool {
    match &p.default {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s == "true",
        _ => false,
    }
}

fn component(ed: Ed, id: &str) -> Option<Component> {
    ed.doc
        .with_untracked(|d| d.as_ref()?.component(id).cloned())
}

/// Premier nœud repéré (avec identifiant) d'une liste.
fn first_id(nodes: &[Node]) -> Option<(String, String)> {
    let mut found = None;
    revel_model::ids::walk(nodes, &mut |n| {
        if found.is_none()
            && let Some(id) = revel_model::ids::node_id(n)
        {
            let tag = match n {
                Node::Element { tag, .. } => tag.clone(),
                Node::Image(_) => "img".into(),
                _ => String::new(),
            };
            found = Some((id.to_string(), tag));
        }
    });
    found
}

/// Sélectionne un nœud et le montre dans le canevas.
fn select(ed: Ed, id: String, tag: String, label: String) {
    ed.selected.set(Some(Selection {
        id: id.clone(),
        tag,
        label,
    }));
    ed.post(serde_json::json!({ "type": "select", "id": id, "reveal": true }));
}

/// Ouvre le composant d'une utilisation (modification isolée) : son premier élément est
/// sélectionné.
pub fn open_component(ed: Ed, use_at: &Target) {
    let first = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let Node::Use { component, .. } = revel_edit::address::node(d, use_at).ok()? else {
            return None;
        };
        first_id(&d.component(component)?.body)
    });
    if let Some((id, tag)) = first {
        select(ed, id, tag.clone(), tag);
    }
}

/// Utilisation du composant `id` (la première trouvée), avec son propriétaire.
fn find_use(ed: Ed, id: &str) -> Option<String> {
    ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let mut found = None;
        for (_, _, body) in d.owners() {
            revel_model::ids::walk(body, &mut |n| {
                if found.is_none()
                    && let Node::Use {
                        id: Some(u),
                        component,
                        ..
                    } = n
                    && component == id
                {
                    found = Some(u.clone());
                }
            });
        }
        found
    })
}

/// Crée un composant à partir de l'élément sélectionné (nom demandé), puis sélectionne son
/// utilisation.
pub fn make_from_selection(ui: Ui, ed: Ed) {
    let Some(at) = untrack(|| ed.selected_target()) else {
        ui.fail(revel_protocol::Failure::new("component-make-none"));
        return;
    };
    let suggestion = ed
        .doc
        .with_untracked(|d| {
            let n = revel_edit::address::node(d.as_ref()?, &at).ok()?;
            Some(match n {
                Node::Element { tag, attrs, .. } => attrs
                    .iter()
                    .find(|a| a.name == "class")
                    .and_then(|a| a.value.as_deref())
                    .and_then(|c| c.split_whitespace().next())
                    .map(|c| c.replace(['_', '-'], " "))
                    .unwrap_or_else(|| tag.clone()),
                _ => String::new(),
            })
        })
        .unwrap_or_default();
    let suggestion = crate::site::capitalize(suggestion.trim());
    ui.ask.set(Some(crate::dialog::Ask::input(
        ui.now("component-make", &[]),
        ui.now("component-make-name", &[]),
        suggestion,
        ui.now("component-make-confirm", &[]),
        move |name| {
            let Some(id) = ed.doc.with_untracked(|d| {
                d.as_ref()
                    .map(|d| revel_edit::make::free_component_id(d, &name))
            }) else {
                return;
            };
            ed.apply(
                ui,
                Command::MakeComponent {
                    at: at.clone(),
                    id: id.clone(),
                    name: Some(name.trim().to_string()),
                },
                false,
            );
            if let Some(u) = find_use(ed, &id) {
                select(ed, u, String::new(), name.clone());
            }
        },
    )));
}

/// Détache une utilisation : copie modifiable séparément.
pub fn detach(ui: Ui, ed: Ed, at: Target) {
    ed.apply(ui, Command::Detach { at }, false);
}

// ------------------------------------------------------------------ utilisation sélectionnée

/// Réglages d'une utilisation de composant : nom, ouvrir, détacher, valeurs des propriétés.
pub fn instance_settings(
    ui: Ui,
    ed: Ed,
    at: Target,
    component_id: String,
    args: BTreeMap<String, String>,
) -> AnyView {
    let Some(c) = component(ed, &component_id) else {
        return view! { <p class="muted">{ui.tx("component-missing", &[("name", component_id)])}</p> }
            .into_any();
    };
    let at = StoredValue::new(at);
    let set = move |name: String, value: Option<String>| {
        ed.apply(
            ui,
            Command::SetUseArg {
                at: at.get_value(),
                name,
                value,
            },
            false,
        )
    };
    // Utilisation dans une liste de collection : propriétés reliables aux champs du contenu.
    let list = ed.doc.with_untracked(|d| {
        crate::blocks::enclosing_list(d.as_ref()?, &at.get_value())
            .map(|(q, item)| (q.collection, item))
    });
    let cols = list.as_ref().map(|_| crate::blocks::collections(ed));
    let lang = ed.lang();
    let rows = c
        .props
        .iter()
        .map(|p| {
            let arg = args.get(&p.name).cloned();
            // Autre langue : texte de la propriété traduit (valeur de l'utilisation, sinon
            // valeur par défaut).
            if let Some(code) = lang.clone()
                && p.kind == PropKind::Text
                && let Some(src) = match &arg {
                    Some(a) => literal(a),
                    None => p.default.as_ref().and_then(|v| v.as_str()).map(str::to_string),
                }
                .filter(|t| revel_edit::locale::translatable(t))
            {
                let name = p.name.clone();
                return view! {
                    <div class="prop-row" data-prop=p.name.clone()>
                        <Field label=p.label().to_string() value=ed.translation(&code, &src).unwrap_or_default()
                            placeholder=src.clone()
                            on_commit=move |v: String| ed.apply(ui, Command::Translate {
                                locale: code.clone(),
                                place: revel_edit::locale::TextPlace::Arg { at: at.get_value(), name: name.clone() },
                                text: v,
                            }, false)/>
                    </div>
                }
                .into_any();
            }
            let bind = list.clone().zip(cols);
            prop_value_row(ui, ed, p.clone(), arg, set, bind)
        })
        .collect_view();
    let has_props = !c.props.is_empty();
    view! {
        <div class="stack instance">
            <div class="instance-head">
                <span class="instance-icon">{icons::component()}</span>
                <span class="instance-name">{c.label()}</span>
            </div>
            <div class="instance-actions">
                <button class="btn btn--small" on:click=move |_| open_component(ed, &at.get_value())>
                    {icons::edit()}{ui.t("component-edit")}</button>
                <button class="btn btn--small" title=ui.t("component-detach-help")
                    on:click=move |_| detach(ui, ed, at.get_value())>{icons::unlink()}{ui.t("component-detach")}</button>
            </div>
            <div class="insp-group">
                <p class="panel-title">{ui.t("component-props")}</p>
                {if has_props {
                    rows.into_any()
                } else {
                    view! { <p class="muted small">{ui.t("component-no-props")}</p> }.into_any()
                }}
            </div>
        </div>
    }
    .into_any()
}

/// Liste de collection qui contient une utilisation : (collection, variable d'élément) et
/// collections du site (champs).
type ListBind = (
    (String, String),
    RwSignal<Vec<revel_protocol::CmsCollection>>,
);

/// Valeur d'une propriété pour une utilisation : vide, la valeur par défaut s'applique.
fn prop_value_row(
    ui: Ui,
    ed: Ed,
    p: PropDef,
    arg: Option<String>,
    set: impl Fn(String, Option<String>) + Copy + Send + Sync + 'static,
    list: Option<ListBind>,
) -> AnyView {
    let name = StoredValue::new(p.name.clone());
    let label = p.label().to_string();
    let overridden = arg.is_some();
    // Relié à un champ du contenu de la liste (`item.data.titre`).
    if let Some(((collection, item), cols)) = list.clone()
        && p.kind != PropKind::Switch
        && let Some(field) = arg
            .as_deref()
            .and_then(|a| a.trim().strip_prefix(&format!("{item}.data.")))
            .filter(|f| revel_model::valid_prop_name(f))
    {
        let field = field.to_string();
        let shown = cols.with_untracked(|cs| {
            cs.iter()
                .find(|c| c.id == collection)
                .and_then(|c| c.fields.iter().find(|f| f.name == field))
                .and_then(|f| f.label.clone())
        });
        let shown = Signal::derive({
            let field = field.clone();
            let collection = collection.clone();
            move || {
                cols.with(|cs| {
                    cs.iter()
                        .find(|c| c.id == collection)
                        .and_then(|c| c.fields.iter().find(|f| f.name == field))
                        .and_then(|f| f.label.clone())
                })
                .or_else(|| shown.clone())
                .unwrap_or_else(|| field.clone())
            }
        });
        return view! {
            <div class="view-line prop-row" data-prop=p.name.clone()>
                <span class="prop-label">{kind_icon(p.kind)}{label}</span>
                <span class="prop-chip prop-chip--field" title=format!("{{{{ {item}.data.{field} }}}}")>
                    {icons::database()}<span>{shown}</span>
                    <button class="chip-x" title=ui.t("component-unbind") aria-label=ui.t("component-unbind")
                        on:click=move |_| set(name.get_value(), None)>{icons::close()}</button>
                </span>
            </div>
        }
        .into_any();
    }
    let can_bind = list.is_some() && p.kind != PropKind::Switch;
    let field_menu = move |e: leptos::ev::MouseEvent| {
        use crate::ctxmenu::{CtxItem, open};
        let Some(((collection, item), cols)) = list.clone() else {
            return;
        };
        let items: Vec<CtxItem> = cols.with_untracked(|cs| {
            cs.iter()
                .find(|c| c.id == collection)
                .map(|c| {
                    c.fields
                        .iter()
                        .filter(|f| {
                            !matches!(f.kind.as_deref(), Some("object" | "block" | "boolean"))
                        })
                        .map(|f| {
                            let expr = format!("{item}.data.{}", f.name);
                            CtxItem::new(
                                f.label.clone().unwrap_or_else(|| f.name.clone()),
                                move || set(name.get_value(), Some(expr.clone())),
                            )
                            .icon(icons::database)
                        })
                        .collect()
                })
                .unwrap_or_default()
        });
        open(ui, &e, items);
    };

    let reset = move || {
        overridden.then(|| {
            view! {
                <button class="icon-btn icon-btn--small" title=ui.t("component-prop-reset") aria-label=ui.t("component-prop-reset")
                    on:click=move |_| set(name.get_value(), None)>{icons::reset()}</button>
            }
        })
    };
    if p.kind == PropKind::Switch {
        let on = match arg.as_deref().map(str::trim) {
            Some("true") => true,
            Some("false") => false,
            _ => default_bool(&p),
        };
        let dflt = default_bool(&p);
        return view! {
            <label class="view-line prop-row" data-prop=p.name.clone()>
                <span class="prop-label">{kind_icon(p.kind)}{label}</span>
                {reset()}
                <span class="switch">
                    <input type="checkbox" prop:checked=on
                        on:change=move |e| {
                            let v = event_target_checked(&e);
                            set(name.get_value(), (v != dflt).then(|| v.to_string()));
                        }/>
                    <span class="switch-track"></span>
                </span>
            </label>
        }
        .into_any();
    }
    // Valeur calculée (expression) : modifiée telle quelle.
    let expr = arg.as_deref().is_some_and(|a| literal(a).is_none());
    let shown = match &arg {
        Some(a) => literal(a).unwrap_or_else(|| a.clone()),
        None => String::new(),
    };
    let field_label = if expr {
        ui.now("component-prop-computed", &[("label", label.clone())])
    } else {
        label.clone()
    };
    let placeholder = default_text(&p);
    let image = p.kind == PropKind::Image;
    view! {
        <div class="prop-row" data-prop=p.name.clone()>
            <Field label=field_label value=shown mono=expr placeholder=placeholder
                on_commit=move |v: String| {
                    let v2 = v.trim().to_string();
                    set(name.get_value(), if v2.is_empty() { None } else if expr { Some(v2) } else { Some(quote(&v)) })
                }>
                {image.then(|| view! {
                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("picker-choose-image") aria-label=ui.t("picker-choose-image")
                        on:click=move |_| crate::library::pick_image(ui, crate::library::media_shelf(ed), move |url| set(name.get_value(), Some(quote(&url))))>
                        {icons::image()}</button>
                })}
                {can_bind.then(|| view! {
                    <button type="button" class="icon-btn icon-btn--small bind-field" title=ui.t("component-bind-field") aria-label=ui.t("component-bind-field")
                        on:click=field_menu>{icons::database()}</button>
                })}
                {reset()}
            </Field>
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ composant ouvert

/// Propriétés du composant ouvert : nom affiché, valeur par défaut, suppression ; ajout par type.
pub fn props_section(ui: Ui, ed: Ed, component_id: String) -> AnyView {
    let id = StoredValue::new(component_id);
    let comp = Memo::new(move |_| {
        ed.doc.with(|d| {
            d.as_ref()
                .and_then(|d| id.with_value(|i| d.component(i).cloned()))
        })
    });
    let write = move |props: Vec<PropDef>| {
        ed.apply(
            ui,
            Command::SetComponentProps {
                id: id.get_value(),
                props,
            },
            false,
        )
    };
    let add = move |e: leptos::ev::MouseEvent| {
        use crate::ctxmenu::{CtxItem, open};
        let items = KINDS
            .into_iter()
            .map(|k| {
                CtxItem::new(ui.now(kind_key(k), &[]), move || {
                    let Some(c) = comp.get_untracked() else {
                        return;
                    };
                    let mut props = c.props.clone();
                    props.push(new_prop(ui, &c, k, None));
                    write(props);
                })
                .icon(kind_icon_fn(k))
            })
            .collect();
        open(ui, &e, items);
    };
    view! {
        <div class="insp-group component-props">
            <div class="component-props-head">
                <p class="panel-title">{ui.t("component-props")}</p>
                <button class="icon-btn icon-btn--small" title=ui.t("component-prop-add") aria-label=ui.t("component-prop-add")
                    on:click=add>{icons::plus()}</button>
            </div>
            {move || comp.get().map(|c| {
                if c.props.is_empty() {
                    return view! { <p class="muted small">{ui.t("component-props-empty")}</p> }.into_any();
                }
                c.props.iter().enumerate().map(|(i, p)| {
                    let kind = p.kind;
                    let pname = StoredValue::new(p.name.clone());
                    let set_prop = move |f: Box<dyn Fn(&mut PropDef)>| {
                        let Some(c) = comp.get_untracked() else { return };
                        let mut props = c.props.clone();
                        if let Some(p) = props.get_mut(i) {
                            f(p);
                        }
                        write(props);
                    };
                    let remove = move |_| {
                        let cmd = ed.doc.with_untracked(|d| {
                            revel_edit::make::remove_prop(d.as_ref()?, &id.get_value(), &pname.get_value()).ok()
                        });
                        if let Some(cmd) = cmd {
                            ed.apply(ui, cmd, false);
                        }
                    };
                    let default_view = if kind == PropKind::Switch {
                        let on = default_bool(p);
                        view! {
                            <label class="view-line">
                                <span class="muted">{ui.t("component-prop-default")}</span>
                                <span class="switch">
                                    <input type="checkbox" prop:checked=on
                                        on:change=move |e| {
                                            let v = event_target_checked(&e);
                                            set_prop(Box::new(move |p| p.default = Some(Value::Bool(v))));
                                        }/>
                                    <span class="switch-track"></span>
                                </span>
                            </label>
                        }.into_any()
                    } else {
                        view! {
                            <Field label=ui.t("component-prop-default") value=default_text(p)
                                on_commit=move |v: String| set_prop(Box::new(move |p| {
                                    p.default = (!v.is_empty()).then(|| Value::String(v.clone()));
                                }))/>
                        }.into_any()
                    };
                    view! {
                        <div class="prop-def" data-prop=p.name.clone()>
                            <div class="prop-def-head">
                                <span class="prop-kind" title=ui.t(kind_key(kind))>{kind_icon(kind)}</span>
                                <input class="input prop-def-label" prop:value=p.label().to_string()
                                    aria-label=ui.t("component-prop-label") title=format!("{{{{ {} }}}}", p.name)
                                    on:change=move |e| {
                                        let v = event_target_value(&e).trim().to_string();
                                        set_prop(Box::new(move |p| p.label = (!v.is_empty()).then(|| v.clone())));
                                    }/>
                                <button class="icon-btn icon-btn--small" title=ui.t("component-prop-remove") aria-label=ui.t("component-prop-remove")
                                    on:click=remove>{icons::trash()}</button>
                            </div>
                            {default_view}
                        </div>
                    }.into_any()
                }).collect_view().into_any()
            })}
        </div>
    }
    .into_any()
}

/// Nouvelle propriété de type `k` (nom libre, libellé numéroté, valeur par défaut donnée).
fn new_prop(ui: Ui, c: &Component, k: PropKind, default: Option<Value>) -> PropDef {
    let name = revel_edit::make::free_prop_name(c, base_name(k));
    let base_label = ui.now(kind_key(k), &[]);
    let mut label = base_label.clone();
    let mut n = 2;
    while c.props.iter().any(|p| p.label() == label) {
        label = format!("{base_label} {n}");
        n += 1;
    }
    PropDef {
        name,
        label: Some(label),
        kind: k,
        default: default.or_else(|| (k == PropKind::Switch).then_some(Value::Bool(true))),
    }
}

/// Ce qu'un élément du composant peut relier à une propriété.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bind {
    Text,
    Src,
    Href,
    Visible,
}

impl Bind {
    fn kind(self) -> PropKind {
        match self {
            Bind::Text => PropKind::Text,
            Bind::Src => PropKind::Image,
            Bind::Href => PropKind::Link,
            Bind::Visible => PropKind::Switch,
        }
    }
    fn key(self) -> &'static str {
        match self {
            Bind::Text => "component-bind-text",
            Bind::Src => "component-bind-image",
            Bind::Href => "component-bind-link",
            Bind::Visible => "component-bind-visibility",
        }
    }
}

/// `{{ name }}` seul.
fn only_var(text: &str) -> Option<String> {
    let t = text.trim().strip_prefix("{{")?.strip_suffix("}}")?.trim();
    revel_model::valid_prop_name(t).then(|| t.to_string())
}

/// État d'une liaison pour le nœud `at` : `Some(Some(prop))` reliée, `Some(None)` possible
/// (valeur actuelle écrite), `None` sans objet.
fn bind_state(
    site: &revel_model::Site,
    c: &Component,
    at: &Target,
    b: Bind,
) -> Option<(Option<String>, String)> {
    let node = revel_edit::address::node(site, at).ok()?;
    let is_prop = |name: &str| c.prop(name).is_some_and(|p| p.kind == b.kind());
    match (b, node) {
        (Bind::Text, Node::Element { children, .. }) => match children.as_slice() {
            [Node::Text { text }] => Some((None, text.clone())),
            [Node::Out { expr }] if is_prop(expr.trim()) => {
                Some((Some(expr.trim().to_string()), String::new()))
            }
            _ => None,
        },
        (Bind::Src, Node::Image(img)) => {
            if is_prop(img.src.trim()) {
                Some((Some(img.src.trim().to_string()), String::new()))
            } else {
                literal(&img.src).map(|t| (None, t))
            }
        }
        (Bind::Href, Node::Element { tag, attrs, .. }) if tag.eq_ignore_ascii_case("a") => {
            let v = attrs
                .iter()
                .find(|a| a.name == "href")
                .and_then(|a| a.value.clone())
                .unwrap_or_default();
            match only_var(&v) {
                Some(n) if is_prop(&n) => Some((Some(n), String::new())),
                _ if !v.contains("{{") && !v.contains("{%") => Some((None, v)),
                _ => None,
            }
        }
        (Bind::Visible, _) => {
            let (parent, _) = revel_edit::address::locate(site, at).ok()?;
            let wrapper = parent.node.as_ref().and_then(|id| {
                revel_edit::address::node(
                    site,
                    &Target {
                        owner: at.owner.clone(),
                        node: id.clone(),
                    },
                )
                .ok()
            });
            match wrapper {
                Some(Node::If {
                    cond,
                    then,
                    otherwise,
                    ..
                }) if parent.branch == Branch::Then
                    && then.len() == 1
                    && otherwise.is_empty()
                    && is_prop(cond.trim()) =>
                {
                    Some((Some(cond.trim().to_string()), String::new()))
                }
                _ => Some((None, String::new())),
            }
        }
        _ => None,
    }
}

/// Commandes qui relient le nœud `at` à la propriété `name` (`new` : propriétés du composant
/// avec la nouvelle).
fn bind_commands(
    site: &revel_model::Site,
    c: &Component,
    at: &Target,
    b: Bind,
    name: &str,
    new: Option<Vec<PropDef>>,
) -> Option<Command> {
    let mut commands = Vec::new();
    if let Some(props) = new {
        commands.push(Command::SetComponentProps {
            id: c.id.clone(),
            props,
        });
    }
    match (b, revel_edit::address::node(site, at).ok()?) {
        (Bind::Text, _) => commands.push(Command::Replace {
            parent: Parent::of(at, Branch::Children),
            index: 0,
            node: Node::Out {
                expr: name.to_string(),
            },
        }),
        (Bind::Src, Node::Image(img)) => {
            let mut image = img.clone();
            image.src = name.to_string();
            commands.push(Command::SetImage {
                at: at.clone(),
                image,
            });
        }
        (Bind::Href, _) => commands.push(Command::SetAttr {
            at: at.clone(),
            name: "href".into(),
            value: Some(format!("{{{{ {name} }}}}")),
        }),
        (Bind::Visible, _) => {
            let (parent, index) = revel_edit::address::locate(site, at).ok()?;
            let if_id = revel_model::ids::fresh(&site.node_ids());
            commands.push(Command::Insert {
                parent,
                index,
                node: Node::If {
                    id: Some(if_id.clone()),
                    cond: name.to_string(),
                    then: Vec::new(),
                    otherwise: Vec::new(),
                },
            });
            commands.push(Command::Move {
                at: at.clone(),
                to: Parent {
                    owner: at.owner.clone(),
                    node: Some(if_id),
                    branch: Branch::Then,
                },
                index: 0,
            });
        }
        _ => return None,
    }
    Some(Command::Batch { commands })
}

/// Commandes qui délient le nœud `at` de sa propriété (sa valeur par défaut est écrite à la
/// place).
fn unbind_commands(
    site: &revel_model::Site,
    c: &Component,
    at: &Target,
    b: Bind,
    name: &str,
) -> Option<Command> {
    let dflt = c.prop(name).map(default_text).unwrap_or_default();
    Some(match (b, revel_edit::address::node(site, at).ok()?) {
        (Bind::Text, _) => Command::Replace {
            parent: Parent::of(at, Branch::Children),
            index: 0,
            node: Node::Text { text: dflt },
        },
        (Bind::Src, Node::Image(img)) => {
            let mut image = img.clone();
            image.src = quote(&dflt);
            Command::SetImage {
                at: at.clone(),
                image,
            }
        }
        (Bind::Href, _) => Command::SetAttr {
            at: at.clone(),
            name: "href".into(),
            value: Some(dflt),
        },
        (Bind::Visible, _) => {
            let (parent, _) = revel_edit::address::locate(site, at).ok()?;
            let if_at = Target {
                owner: at.owner.clone(),
                node: parent.node?,
            };
            let (if_parent, if_index) = revel_edit::address::locate(site, &if_at).ok()?;
            Command::Batch {
                commands: vec![
                    Command::Move {
                        at: at.clone(),
                        to: if_parent,
                        index: if_index,
                    },
                    Command::Remove { at: if_at },
                ],
            }
        }
        _ => return None,
    })
}

/// Éléments du composant ouvert reliés (ou à relier) à ses propriétés : texte, image, lien,
/// visibilité de l'élément sélectionné.
pub fn bindings(ui: Ui, ed: Ed, component_id: String, at: Target) -> AnyView {
    let id = StoredValue::new(component_id);
    let at = StoredValue::new(at);
    let rows = move || {
        ed.doc.with(|d| {
            let d = d.as_ref()?;
            let c = id.with_value(|i| d.component(i).cloned())?;
            let at = at.get_value();
            let v: Vec<(Bind, Option<String>, String, Option<String>)> =
                [Bind::Text, Bind::Src, Bind::Href, Bind::Visible]
                    .into_iter()
                    .filter_map(|b| {
                        let (linked, current) = bind_state(d, &c, &at, b)?;
                        let label = linked
                            .as_ref()
                            .and_then(|n| c.prop(n))
                            .map(|p| p.label().to_string());
                        Some((b, linked, current, label))
                    })
                    .collect();
            Some(v)
        })
    };
    let open_menu = move |e: leptos::ev::MouseEvent, b: Bind, current: String| {
        use crate::ctxmenu::{CtxItem, open};
        let Some(c) = component(ed, &id.get_value()) else {
            return;
        };
        let mut items = Vec::new();
        let label = ui.now(kind_key(b.kind()), &[]);
        {
            let c = c.clone();
            let current = current.clone();
            items.push(
                CtxItem::new(
                    ui.now("component-bind-new", &[("label", label)]),
                    move || {
                        let default = match b {
                            Bind::Visible => Some(Value::Bool(true)),
                            _ => (!current.is_empty()).then(|| Value::String(current.clone())),
                        };
                        let p = new_prop(ui, &c, b.kind(), default);
                        let mut props = c.props.clone();
                        let name = p.name.clone();
                        props.push(p);
                        let cmd = ed.doc.with_untracked(|d| {
                            bind_commands(d.as_ref()?, &c, &at.get_value(), b, &name, Some(props))
                        });
                        if let Some(cmd) = cmd {
                            ed.apply(ui, cmd, false);
                        }
                    },
                )
                .icon(icons::plus),
            );
        }
        for p in c.props.iter().filter(|p| p.kind == b.kind()) {
            let c = c.clone();
            let name = p.name.clone();
            items.push(
                CtxItem::new(p.label().to_string(), move || {
                    let cmd = ed.doc.with_untracked(|d| {
                        bind_commands(d.as_ref()?, &c, &at.get_value(), b, &name, None)
                    });
                    if let Some(cmd) = cmd {
                        ed.apply(ui, cmd, false);
                    }
                })
                .icon(icons::component),
            );
        }
        open(ui, &e, items);
    };
    let unbind = move |b: Bind, name: String| {
        let Some(c) = component(ed, &id.get_value()) else {
            return;
        };
        let cmd = ed
            .doc
            .with_untracked(|d| unbind_commands(d.as_ref()?, &c, &at.get_value(), b, &name));
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    view! {
        {move || rows().filter(|v| !v.is_empty()).map(|v| view! {
            <div class="insp-group component-bindings">
                <p class="panel-title">{ui.t("component-bindings")}</p>
                {v.into_iter().map(|(b, linked, current, label)| {
                    let cur = current.clone();
                    view! {
                        <div class="view-line bind-row" data-bind=b.key()>
                            <span class="muted">{ui.t(b.key())}</span>
                            {match linked {
                                Some(name) => {
                                    let tip = format!("{{{{ {name} }}}}");
                                    let shown = label.unwrap_or_else(|| name.clone());
                                    view! {
                                    <span class="prop-chip" title=tip>
                                        {icons::component()}<span>{shown}</span>
                                        <button class="chip-x" title=ui.t("component-unbind") aria-label=ui.t("component-unbind")
                                            on:click=move |_| unbind(b, name.clone())>{icons::close()}</button>
                                    </span>
                                    }.into_any()
                                }
                                None => view! {
                                    <button class="icon-btn icon-btn--small bind-btn" title=ui.t("component-bind") aria-label=ui.t("component-bind")
                                        on:click=move |e| open_menu(e, b, cur.clone())>{icons::prop_dot()}</button>
                                }.into_any(),
                            }}
                        </div>
                    }
                }).collect_view()}
            </div>
        })}
    }
    .into_any()
}

/// Composant du nœud sélectionné, quand il en fait partie.
pub fn owner_component(at: &Target) -> Option<String> {
    match &at.owner {
        Owner::Component(c) => Some(c.clone()),
        _ => None,
    }
}

/// Composants affichés par une page : ceux de son contenu, de ses gabarits, et ceux qu'ils
/// utilisent.
fn shown_by(
    d: &revel_model::Site,
    info: &revel_protocol::EditorPageInfo,
) -> std::collections::HashSet<String> {
    let mut todo: Vec<String> = Vec::new();
    let push_uses = |nodes: &[Node], todo: &mut Vec<String>| {
        revel_model::ids::walk(nodes, &mut |n| {
            if let Node::Use { component, .. } = n {
                todo.push(component.clone());
            }
        });
    };
    if let Some(p) = info
        .page
        .as_ref()
        .and_then(|id| d.pages.iter().find(|p| &p.id == id))
    {
        push_uses(&p.body, &mut todo);
    }
    let mut layout = info.layout.clone();
    let mut guard = 0;
    while let Some(l) = layout.as_deref().and_then(|l| d.layout(l)) {
        push_uses(&l.body, &mut todo);
        layout = l.parent.clone();
        guard += 1;
        if guard > 32 {
            break;
        }
    }
    let mut seen = std::collections::HashSet::new();
    while let Some(c) = todo.pop() {
        if seen.insert(c.clone())
            && let Some(comp) = d.component(&c)
        {
            push_uses(&comp.body, &mut todo);
        }
    }
    seen
}

/// Ouvre un composant depuis le panneau : sur la page affichée s'il y est utilisé, sinon sur la
/// première page qui l'utilise.
pub fn edit_component(ui: Ui, ed: Ed, id: &str) {
    let found = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let first = first_id(&d.component(id)?.body)?;
        let cur = ed.page.get_untracked();
        let page = ed.pages.with_untracked(|ps| {
            let here = ps
                .iter()
                .find(|p| p.url == cur)
                .filter(|p| shown_by(d, p).contains(id));
            here.or_else(|| ps.iter().find(|p| shown_by(d, p).contains(id)))
                .map(|p| p.url.clone())
        });
        Some((first, page))
    });
    match found {
        Some(((node, tag), Some(url))) => {
            if ed.page.get_untracked() != url {
                ed.page.set(url);
                ed.scroll.set((0.0, 0.0));
            }
            select(ed, node, tag.clone(), tag);
        }
        Some((_, None)) => ui.fail(revel_protocol::Failure::new("component-unused")),
        None => ui.fail(revel_protocol::Failure::new("component-empty")),
    }
}

/// Renomme un composant (nom affiché ; l'identifiant ne change pas).
pub fn rename(ui: Ui, ed: Ed, id: &str) {
    let Some(c) = component(ed, id) else { return };
    let id = id.to_string();
    ui.ask.set(Some(crate::dialog::Ask::input(
        ui.now("component-rename", &[]),
        ui.now("component-make-name", &[]),
        c.label(),
        ui.now("component-rename", &[]),
        move |name| {
            ed.apply(
                ui,
                Command::SetComponentName {
                    id: id.clone(),
                    name: Some(name),
                },
                false,
            )
        },
    )));
}

/// Supprime un composant inutilisé (confirmation), retiré aussi de son dossier.
pub fn delete(ui: Ui, ed: Ed, id: &str) {
    let Some(c) = component(ed, id) else { return };
    let id = id.to_string();
    ui.ask.set(Some(
        crate::dialog::Ask::confirm(
            ui.now("component-delete-title", &[("name", c.label())]),
            None,
            ui.now("component-delete", &[]),
            move || {
                let mut commands = vec![Command::RemoveComponent { id: id.clone() }];
                if let Some(shelf) = ed.doc.with_untracked(|d| {
                    let s = &d.as_ref()?.library.components;
                    s.items.contains_key(&id).then(|| {
                        let mut s = s.clone();
                        s.items.remove(&id);
                        s
                    })
                }) {
                    commands.push(Command::SetShelf {
                        kind: revel_edit::ShelfKind::Components,
                        shelf,
                    });
                }
                ed.apply(ui, Command::Batch { commands }, false);
            },
        )
        .danger(),
    ));
}
