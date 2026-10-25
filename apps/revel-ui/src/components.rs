//! Composants dans l'éditeur (3.6, D-34), comme ceux de Webflow et Framer : réglages d'une
//! utilisation (valeurs de ses propriétés, variante, ouvrir le composant, détacher), propriétés
//! du composant ouvert (ajouter, renommer, valeur par défaut, supprimer), variantes (survol,
//! appui, personnalisées) dans le canevas des composants, éléments du composant reliés à une
//! propriété (texte, image, lien, visibilité), création d'un composant à partir de la sélection.

use crate::editor::{Ed, Selection};
use crate::inspector::Field;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::canvas::{literal, quote};
use revel_edit::{Branch, Command, Owner, Parent, Target};
use revel_model::{Component, Node, PropDef, PropKind, Variant, VariantKind};
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) const KINDS: [PropKind; 5] = [
    PropKind::Text,
    PropKind::Image,
    PropKind::Link,
    PropKind::Switch,
    PropKind::List,
];

pub(crate) fn kind_key(k: PropKind) -> &'static str {
    match k {
        PropKind::Text => "prop-kind-text",
        PropKind::Image => "prop-kind-image",
        PropKind::Link => "prop-kind-link",
        PropKind::Switch => "prop-kind-switch",
        PropKind::List => "prop-kind-list",
    }
}

pub(crate) fn kind_icon(k: PropKind) -> AnyView {
    match k {
        PropKind::Text => icons::text(),
        PropKind::Image => icons::image(),
        PropKind::Link => icons::link(),
        PropKind::Switch => icons::eye(),
        PropKind::List => icons::list(),
    }
}

pub(crate) fn kind_icon_fn(k: PropKind) -> fn() -> AnyView {
    match k {
        PropKind::Text => icons::text,
        PropKind::Image => icons::image,
        PropKind::Link => icons::link,
        PropKind::Switch => icons::eye,
        PropKind::List => icons::list,
    }
}

/// Début du nom d'une nouvelle propriété (identifiant dans les expressions).
fn base_name(k: PropKind) -> &'static str {
    match k {
        PropKind::Text => "texte",
        PropKind::Image => "image",
        PropKind::Link => "lien",
        PropKind::Switch => "visible",
        PropKind::List => "liste",
    }
}

/// Valeur par défaut en texte.
pub(crate) fn default_text(p: &PropDef) -> String {
    match &p.default {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string(),
    }
}

pub(crate) fn default_bool(p: &PropDef) -> bool {
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

/// Propriétaire de propriétés : composant (valeur choisie à chaque utilisation) ou gabarit
/// (valeur choisie par chaque page qui l'utilise, dans ses réglages).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Holder {
    Component(String),
    Layout(String),
}

impl Holder {
    /// Propriétaire du nœud `at` : son composant ou son gabarit.
    pub fn of(at: &Target) -> Option<Holder> {
        match &at.owner {
            Owner::Component(c) => Some(Holder::Component(c.clone())),
            Owner::Layout(l) => Some(Holder::Layout(l.clone())),
            Owner::Page(_) => None,
        }
    }
    /// Propriétés que le propriétaire déclare.
    pub(crate) fn own(&self, site: &revel_model::Site) -> Option<Vec<PropDef>> {
        match self {
            Holder::Component(id) => site.component(id).map(|c| c.props.clone()),
            Holder::Layout(id) => site
                .layouts
                .iter()
                .find(|l| &l.id == id)
                .map(|l| l.fields.clone()),
        }
    }
    /// Propriétés lisibles dans le propriétaire : les siennes ; pour un gabarit, aussi celles
    /// des gabarits qui le contiennent.
    pub(crate) fn visible(&self, site: &revel_model::Site) -> Option<Vec<PropDef>> {
        match self {
            Holder::Component(_) => self.own(site),
            Holder::Layout(id) => {
                self.own(site)?;
                Some(revel_edit::editable::layout_fields(site, Some(id)))
            }
        }
    }
    pub(crate) fn set(&self, props: Vec<PropDef>) -> Command {
        match self {
            Holder::Component(id) => Command::SetComponentProps {
                id: id.clone(),
                props,
            },
            Holder::Layout(id) => Command::SetLayoutFields {
                id: id.clone(),
                fields: props,
            },
        }
    }
    /// Retrait d'une propriété (valeur par défaut écrite là où elle servait).
    pub(crate) fn remove(&self, site: &revel_model::Site, name: &str) -> Option<Command> {
        match self {
            Holder::Component(id) => revel_edit::make::remove_prop(site, id, name).ok(),
            Holder::Layout(id) => revel_edit::make::remove_layout_field(site, id, name).ok(),
        }
    }
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
    ed.select_and_reveal(&id);
}

/// Ouvre le composant d'une utilisation dans le canevas des composants : son premier élément est
/// sélectionné.
pub fn open_component(ed: Ed, use_at: &Target) {
    let found = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let Node::Use { component, .. } = revel_edit::address::node(d, use_at).ok()? else {
            return None;
        };
        Some((component.clone(), first_id(&d.component(component)?.body)))
    });
    if let Some((c, first)) = found {
        ed.open_comp(&c, first);
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
        crate::blocks::enclosing_list(d.as_ref()?, &at.get_value()).map(|(q, item)| (q.key(), item))
    });
    let cols = list.as_ref().map(|_| crate::blocks::collections(ed));
    // Propriétés de ce qui contient l'utilisation (composant ouvert, gabarit) : une valeur peut
    // en reprendre une (`titre`, `not masquer`).
    let parents: Vec<PropDef> = ed
        .doc
        .with_untracked(|d| {
            let d = d.as_ref()?;
            Holder::of(&at.get_value())?.visible(d)
        })
        .unwrap_or_default();
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
            let parent = arg.as_deref().and_then(|a| {
                let a = a.trim();
                let (neg, name) = match a.strip_prefix("not ") {
                    Some(x) => (true, x.trim()),
                    None => (false, a),
                };
                parents
                    .iter()
                    .find(|q| q.name == name && (!neg || q.kind == PropKind::Switch))
                    .map(|q| (q.clone(), neg))
            });
            prop_value_row(ui, ed, p.clone(), arg, set, bind, parent)
        })
        .collect_view();
    let has_props = !c.props.is_empty();
    // Variante choisie pour cette utilisation (variantes personnalisées du composant).
    let customs: Vec<Variant> = c
        .variants
        .iter()
        .filter(|v| v.kind == VariantKind::Custom)
        .cloned()
        .collect();
    let chosen = ed.doc.with_untracked(|d| {
        match revel_edit::address::node(d.as_ref()?, &at.get_value()).ok()? {
            Node::Use { variant, .. } => variant.clone(),
            _ => None,
        }
    });
    let variant_row = (!customs.is_empty()).then(|| {
        let chosen = chosen.clone().unwrap_or_default();
        view! {
            <label class="field instance-variant">
                <span class="label">{ui.t("variant-label")}</span>
                <span class="select">
                    <select class="input" on:change=move |e| {
                        let v = event_target_value(&e);
                        ed.apply(ui, Command::SetUseVariant {
                            at: at.get_value(),
                            variant: (!v.is_empty()).then_some(v),
                        }, false);
                    }>
                        <option value="" selected=chosen.is_empty()>{ui.t("variant-primary")}</option>
                        {customs.iter().map(|v| {
                            let on = chosen == v.id;
                            view! { <option value=v.id.clone() selected=on>{variant_label(ui, Some(v))}</option> }
                        }).collect_view()}
                    </select>
                </span>
            </label>
        }
    });
    // Emplacement du composant (D-118) : contenu du composant, ou propre à cette utilisation.
    let slot_row = revel_edit::make::slot_of(&c).is_some().then(|| {
        let own = ed.doc.with_untracked(|d| {
            match d.as_ref().and_then(|d| revel_edit::address::node(d, &at.get_value()).ok()) {
                Some(Node::Use { children, .. }) => !children.is_empty(),
                _ => false,
            }
        });
        let run = move |reset: bool| {
            let cmd = ed.doc.with_untracked(|d| {
                let d = d.as_ref()?;
                let at = at.get_value();
                if reset {
                    revel_edit::make::reset_slot(d, &at).ok()
                } else {
                    revel_edit::make::customize_slot(d, &at).ok()
                }
            });
            if let Some(cmd) = cmd {
                ed.apply(ui, cmd, false);
            }
        };
        view! {
            <div class="insp-group instance-slot" data-slot=if own { "own" } else { "default" }>
                <p class="panel-title" title=ui.t("slot-help")>{ui.t("slot-title")}</p>
                <div class="slot-row">
                    <span class="muted small">{ui.t(if own { "slot-own" } else { "slot-default" })}</span>
                    {if own {
                        view! {
                            <button class="btn btn--small" data-action="slot-reset" title=ui.t("slot-reset-help")
                                on:click=move |_| run(true)>{icons::undo()}<span>{ui.t("slot-reset")}</span></button>
                        }.into_any()
                    } else {
                        view! {
                            <button class="btn btn--small" data-action="slot-edit" title=ui.t("slot-edit-help")
                                on:click=move |_| run(false)>{icons::edit()}<span>{ui.t("slot-edit")}</span></button>
                        }.into_any()
                    }}
                </div>
            </div>
        }
    });
    view! {
        <div class="stack instance">
            // Composant utilisé (pastille verte) et ses actions sur une ligne ; le nom du calque est
            // dans la barre du haut (D-124).
            <div class="instance-actions">
                <span class="instance-head" title=ui.t("component-of")>
                    <span class="instance-icon">{icons::component()}</span>
                    <span class="instance-name">{crate::names::component_name(&c)}</span>
                </span>
                <button class="btn btn--small" on:click=move |_| open_component(ed, &at.get_value())>
                    {icons::edit()}{ui.t("component-edit")}</button>
                <button class="btn btn--small" title=ui.t("component-detach-help")
                    on:click=move |_| detach(ui, ed, at.get_value())>{icons::unlink()}{ui.t("component-detach")}</button>
            </div>
            {variant_row}
            {slot_row}
            {has_props.then(|| view! {
                <div class="insp-group">
                    <p class="panel-title">{ui.t("component-props")}</p>
                    {rows}
                </div>
            })}
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

/// Valeur d'une propriété reliée à un champ du contenu de la liste : champ, autre champ lu s'il
/// est vide, écriture d'une date (`(item.data.a or item.data.b) | date("%d.%m.%Y")`).
fn bound_field(arg: &str, item: &str) -> Option<(String, Option<String>, Option<String>)> {
    let prefix = format!("{item}.data.");
    let field = |e: &str| {
        e.trim()
            .strip_prefix(prefix.as_str())
            .filter(|f| revel_model::valid_prop_name(f))
            .map(str::to_string)
    };
    let a = arg.trim();
    let (head, format) = match a.rsplit_once('|') {
        Some((h, t)) => (h.trim(), Some(revel_edit::dates::format_of(t)?)),
        None => (a, None),
    };
    let inner = head
        .strip_prefix('(')
        .and_then(|h| h.strip_suffix(')'))
        .unwrap_or(head);
    match inner.split(" or ").collect::<Vec<_>>().as_slice() {
        [x] => Some((field(x)?, None, format)),
        [x, y] => Some((field(x)?, Some(field(y)?), format)),
        _ => None,
    }
}

/// Expression d'une propriété reliée (inverse de [`bound_field`]).
fn bound_expr(item: &str, field: &str, fallback: Option<&str>, format: Option<&str>) -> String {
    let head = match fallback {
        Some(fb) => format!("({item}.data.{field} or {item}.data.{fb})"),
        None => format!("{item}.data.{field}"),
    };
    match format {
        Some(f) => format!("{head} | {f}"),
        None => head,
    }
}

/// Valeur d'une propriété pour une utilisation : vide, la valeur par défaut s'applique.
fn prop_value_row(
    ui: Ui,
    ed: Ed,
    p: PropDef,
    arg: Option<String>,
    set: impl Fn(String, Option<String>) + Copy + Send + Sync + 'static,
    list: Option<ListBind>,
    parent: Option<(PropDef, bool)>,
) -> AnyView {
    let name = StoredValue::new(p.name.clone());
    let label = p.label().to_string();
    let overridden = arg.is_some();
    // Reprend une propriété du composant ouvert ou du gabarit (inversée pour `not x`).
    if let Some((q, neg)) = parent {
        let shown = if neg {
            ui.now(
                "component-prop-inverted",
                &[("label", q.label().to_string())],
            )
        } else {
            q.label().to_string()
        };
        let tip = format!("{{{{ {} }}}}", arg.clone().unwrap_or_default());
        return view! {
            <div class="view-line prop-row" data-prop=p.name.clone()>
                <span class="prop-label">{kind_icon(p.kind)}{label}</span>
                <span class="prop-chip prop-chip--parent" title=tip>
                    {icons::prop_dot()}<span>{shown}</span>
                    <button class="chip-x" title=ui.t("component-unbind") aria-label=ui.t("component-unbind")
                        on:click=move |_| set(name.get_value(), None)>{icons::close()}</button>
                </span>
            </div>
        }
        .into_any();
    }
    // Relié à un champ du contenu de la liste (`item.data.titre`), avec un autre champ lu s'il est
    // vide (`(item.data.a or item.data.b)`) et l'écriture d'une date (`| date("%d.%m.%Y")`).
    if let Some(((collection, item), cols)) = list.clone()
        && p.kind != PropKind::Switch
        && let Some((field, fallback, format)) = arg.as_deref().and_then(|a| bound_field(a, &item))
    {
        // Libellé d'un champ et sa nature (date : écriture choisie).
        let info = move |f: &str| {
            cols.with(|cs| {
                cs.iter()
                    .find(|c| c.id == collection)
                    .and_then(|c| c.fields.iter().find(|x| x.name == f))
                    .map(|x| {
                        (
                            x.label.clone().unwrap_or_else(|| x.name.clone()),
                            x.kind.clone(),
                        )
                    })
            })
            .unwrap_or_else(|| (f.to_string(), None))
        };
        let (main_label, main_kind) = info(&field);
        let shown = match &fallback {
            Some(fb) => ui.now(
                "component-field-fallback",
                &[("field", main_label), ("other", info(fb).0)],
            ),
            None => main_label,
        };
        let date = format.is_some() || main_kind.as_deref().is_some_and(|k| k.starts_with("date"));
        let tip = format!("{{{{ {} }}}}", arg.clone().unwrap_or_default());
        let item2 = item.clone();
        let (f2, fb2) = (field.clone(), fallback.clone());
        let cur_fmt = format.clone().unwrap_or_default();
        let lang = ed.lang().unwrap_or_else(|| {
            ed.doc
                .with_untracked(|d| d.as_ref().map(|d| d.default_locale.clone()))
                .unwrap_or_else(|| ui.lang())
        });
        return view! {
            <div class="prop-row prop-row--bound" data-prop=p.name.clone()>
                <div class="view-line">
                    <span class="prop-label">{kind_icon(p.kind)}{label}</span>
                    <span class="prop-chip prop-chip--field" title=tip>
                        {icons::database()}<span>{shown}</span>
                        <button class="chip-x" title=ui.t("component-unbind") aria-label=ui.t("component-unbind")
                            on:click=move |_| set(name.get_value(), None)>{icons::close()}</button>
                    </span>
                </div>
                {date.then(|| view! {
                    <label class="field" data-bound="block-bound-date">
                        <span class="label">{ui.t("block-bound-date")}</span>
                        <div class="select">
                            <select class="input" on:change=move |e| {
                                let v = event_target_value(&e);
                                let fmt = (!v.is_empty()).then_some(v);
                                set(name.get_value(), Some(bound_expr(&item2, &f2, fb2.as_deref(), fmt.as_deref())));
                            }>
                                <option value="" prop:selected=cur_fmt.is_empty()>{ui.t("block-bound-date-raw")}</option>
                                {revel_edit::dates::FORMATS.iter().map(|(f, _)| {
                                    let on = revel_edit::dates::format_of(&cur_fmt).as_deref() == Some(*f);
                                    view! { <option value=*f prop:selected=on>{revel_edit::dates::example(f, &lang)}</option> }
                                }).collect_view()}
                            </select>
                        </div>
                    </label>
                })}
            </div>
        }
        .into_any();
    }
    // Relié à l'adresse de la page du contenu (`item.url`).
    if let Some(((_, item), _)) = list.clone()
        && p.kind != PropKind::Switch
        && p.kind != PropKind::List
        && arg.as_deref().map(str::trim) == Some(format!("{item}.url").as_str())
    {
        return view! {
            <div class="view-line prop-row" data-prop=p.name.clone()>
                <span class="prop-label">{kind_icon(p.kind)}{label}</span>
                <span class="prop-chip prop-chip--field" title=format!("{{{{ {item}.url }}}}")>
                    {icons::link()}<span>{ui.t("component-field-url")}</span>
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
        // Adresse de la page du contenu, puis ses champs.
        let mut items: Vec<CtxItem> = Vec::new();
        if p.kind != PropKind::List {
            let expr = format!("{item}.url");
            items.push(
                CtxItem::new(ui.now("component-field-url", &[]), move || {
                    set(name.get_value(), Some(expr.clone()))
                })
                .icon(icons::link),
            );
        }
        let fields: Vec<CtxItem> = cols.with_untracked(|cs| {
            cs.iter()
                .find(|c| c.id == collection)
                .map(|c| {
                    c.fields
                        .iter()
                        // Propriété liste : champs à plusieurs valeurs ; sinon une valeur.
                        .filter(|f| {
                            !matches!(f.kind.as_deref(), Some("object" | "block" | "boolean"))
                                && f.list == (p.kind == PropKind::List)
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
        items.extend(fields);
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
    let expr = p.kind == PropKind::List || arg.as_deref().is_some_and(|a| literal(a).is_none());
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
pub fn props_section(ui: Ui, ed: Ed, holder: Holder) -> AnyView {
    let layout = matches!(holder, Holder::Layout(_));
    let holder = StoredValue::new(holder);
    let props = Memo::new(move |_| {
        ed.doc
            .with(|d| d.as_ref().and_then(|d| holder.with_value(|h| h.own(d))))
    });
    let write = move |props: Vec<PropDef>| ed.apply(ui, holder.with_value(|h| h.set(props)), false);
    let add = move |e: leptos::ev::MouseEvent| {
        use crate::ctxmenu::{CtxItem, open};
        let items = KINDS
            .into_iter()
            // Gabarit : valeurs réglées dans les réglages d'une page (pas de liste).
            .filter(|k| !(layout && *k == PropKind::List))
            .map(|k| {
                CtxItem::new(ui.now(kind_key(k), &[]), move || {
                    let Some(mut list) = props.get_untracked() else {
                        return;
                    };
                    let visible = ed
                        .doc
                        .with_untracked(|d| {
                            d.as_ref().and_then(|d| holder.with_value(|h| h.visible(d)))
                        })
                        .unwrap_or_else(|| list.clone());
                    list.push(new_prop(ui, &visible, k, None));
                    write(list);
                })
                .icon(kind_icon_fn(k))
            })
            .collect();
        open(ui, &e, items);
    };
    let (title, empty) = if layout {
        ("layout-props", "layout-props-empty")
    } else {
        ("component-props", "component-props-empty")
    };
    view! {
        <div class="insp-group component-props">
            <div class="component-props-head">
                <p class="panel-title" title=ui.t(empty)>{ui.t(title)}</p>
                <button class="icon-btn icon-btn--small" title=ui.t("component-prop-add") aria-label=ui.t("component-prop-add")
                    on:click=add>{icons::plus()}</button>
            </div>
            {move || props.get().map(|list| {
                if list.is_empty() {
                    return ().into_any();
                }
                list.iter().enumerate().map(|(i, p)| {
                    let kind = p.kind;
                    let pname = StoredValue::new(p.name.clone());
                    let set_prop = move |f: Box<dyn Fn(&mut PropDef)>| {
                        let Some(mut list) = props.get_untracked() else { return };
                        if let Some(p) = list.get_mut(i) {
                            f(p);
                        }
                        write(list);
                    };
                    let remove = move |_| {
                        let cmd = ed.doc.with_untracked(|d| {
                            holder.with_value(|h| h.remove(d.as_ref()?, &pname.get_value()))
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
pub(crate) fn new_prop(ui: Ui, props: &[PropDef], k: PropKind, default: Option<Value>) -> PropDef {
    let name = revel_edit::make::free_name(props, base_name(k));
    let base_label = ui.now(kind_key(k), &[]);
    let mut label = base_label.clone();
    let mut n = 2;
    while props.iter().any(|p| p.label() == label) {
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
    props: &[PropDef],
    at: &Target,
    b: Bind,
) -> Option<(Option<String>, String)> {
    let node = revel_edit::address::node(site, at).ok()?;
    let is_prop = |name: &str| props.iter().any(|p| p.name == name && p.kind == b.kind());
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
    holder: &Holder,
    at: &Target,
    b: Bind,
    name: &str,
    new: Option<Vec<PropDef>>,
) -> Option<Command> {
    let mut commands = Vec::new();
    if let Some(props) = new {
        commands.push(holder.set(props));
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
    props: &[PropDef],
    at: &Target,
    b: Bind,
    name: &str,
) -> Option<Command> {
    let dflt = props
        .iter()
        .find(|p| p.name == name)
        .map(default_text)
        .unwrap_or_default();
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

/// Éléments du composant ouvert (ou d'un gabarit) reliés, ou à relier, à ses propriétés : texte,
/// image, lien, visibilité de l'élément sélectionné. Dans un gabarit, seulement quand il a des
/// propriétés (créées dans les réglages d'une page) ou que l'élément en lit une.
pub fn bindings(ui: Ui, ed: Ed, holder: Holder, at: Target) -> AnyView {
    let holder = StoredValue::new(holder);
    let at = StoredValue::new(at);
    let rows = move || {
        ed.doc.with(|d| {
            let d = d.as_ref()?;
            let props = holder.with_value(|h| h.visible(d))?;
            let at = at.get_value();
            let v: Vec<(Bind, Option<String>, String, Option<String>)> =
                [Bind::Text, Bind::Src, Bind::Href, Bind::Visible]
                    .into_iter()
                    .filter_map(|b| {
                        let (linked, current) = bind_state(d, &props, &at, b)?;
                        let label = linked
                            .as_ref()
                            .and_then(|n| props.iter().find(|p| &p.name == n))
                            .map(|p| p.label().to_string());
                        Some((b, linked, current, label))
                    })
                    .collect();
            let layout = holder.with_value(|h| matches!(h, Holder::Layout(_)));
            (!layout || !props.is_empty() || v.iter().any(|(_, l, _, _)| l.is_some())).then_some(v)
        })
    };
    let open_menu = move |e: leptos::ev::MouseEvent, b: Bind, current: String| {
        use crate::ctxmenu::{CtxItem, open};
        let Some((own, visible)) = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            holder.with_value(|h| Some((h.own(d)?, h.visible(d)?)))
        }) else {
            return;
        };
        let mut items = Vec::new();
        let label = ui.now(kind_key(b.kind()), &[]);
        {
            let current = current.clone();
            let own = own.clone();
            let visible = visible.clone();
            items.push(
                CtxItem::new(
                    ui.now("component-bind-new", &[("label", label)]),
                    move || {
                        let default = match b {
                            Bind::Visible => Some(Value::Bool(true)),
                            _ => (!current.is_empty()).then(|| Value::String(current.clone())),
                        };
                        let p = new_prop(ui, &visible, b.kind(), default);
                        let mut props = own.clone();
                        let name = p.name.clone();
                        props.push(p);
                        let cmd = ed.doc.with_untracked(|d| {
                            holder.with_value(|h| {
                                bind_commands(
                                    d.as_ref()?,
                                    h,
                                    &at.get_value(),
                                    b,
                                    &name,
                                    Some(props),
                                )
                            })
                        });
                        if let Some(cmd) = cmd {
                            ed.apply(ui, cmd, false);
                        }
                    },
                )
                .icon(icons::plus),
            );
        }
        for p in visible.iter().filter(|p| p.kind == b.kind()) {
            let name = p.name.clone();
            items.push(
                CtxItem::new(p.label().to_string(), move || {
                    let cmd = ed.doc.with_untracked(|d| {
                        holder.with_value(|h| {
                            bind_commands(d.as_ref()?, h, &at.get_value(), b, &name, None)
                        })
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
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let props = holder.with_value(|h| h.visible(d))?;
            unbind_commands(d, &props, &at.get_value(), b, &name)
        });
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

/// Ouvre un composant depuis le panneau dans le canevas des composants (même s'il n'est utilisé
/// sur aucune page).
pub fn edit_component(ui: Ui, ed: Ed, id: &str) {
    let first = ed
        .doc
        .with_untracked(|d| first_id(&d.as_ref()?.component(id)?.body));
    match first {
        Some(f) => ed.open_comp(id, Some(f)),
        None => ui.fail(revel_protocol::Failure::new("component-empty")),
    }
}

// ------------------------------------------------------------------ variantes (comme Framer)

/// Nom d'une variante (`None` : principale).
pub fn variant_label(ui: Ui, v: Option<&Variant>) -> String {
    match v {
        None => ui.now("variant-primary", &[]),
        Some(v) => v
            .name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| match v.kind {
                VariantKind::Hover => ui.now("variant-hover", &[]),
                VariantKind::Pressed => ui.now("variant-pressed", &[]),
                VariantKind::Custom => crate::names::humanize(&v.id),
            }),
    }
}

/// Variante personnalisée prise au clic sur le composant (D-121) : interaction de l'élément
/// racine `click class:is-<variante> self` (un clic la prend, le suivant revient à la principale).
pub fn variant_on_click(c: &Component, vid: &str) -> bool {
    root_rules(c).iter().any(|r| is_click_rule(r, vid))
}

fn is_click_rule(r: &revel_edit::actions::Rule, vid: &str) -> bool {
    use revel_edit::actions::{Act, Trigger};
    r.trigger == Trigger::Click && r.target == "self" && r.act == Act::Class(format!("is-{vid}"))
}

fn root_rules(c: &Component) -> Vec<revel_edit::actions::Rule> {
    let Some(Node::Element { attrs, .. }) = c.root() else {
        return Vec::new();
    };
    attrs
        .iter()
        .find(|a| a.name == "data-revel-on")
        .and_then(|a| a.value.as_deref())
        .and_then(revel_edit::actions::parse)
        .unwrap_or_default()
}

/// Commande qui ajoute ou retire le passage à la variante au clic sur l'élément racine.
fn click_command(c: &Component, vid: &str, on: bool) -> Option<Command> {
    use revel_edit::actions::{Act, Rule, Trigger};
    let Some(Node::Element { id: Some(root), .. }) = c.root() else {
        return None;
    };
    let mut rules: Vec<Rule> = root_rules(c)
        .into_iter()
        .filter(|r| !is_click_rule(r, vid))
        .collect();
    if on {
        rules.push(Rule {
            trigger: Trigger::Click,
            act: Act::Class(format!("is-{vid}")),
            target: "self".into(),
        });
    }
    let at = Target {
        owner: Owner::Component(c.id.clone()),
        node: root.clone(),
    };
    Some(if rules.is_empty() {
        Command::RemoveAttr {
            at,
            name: "data-revel-on".into(),
        }
    } else {
        Command::SetAttr {
            at,
            name: "data-revel-on".into(),
            value: Some(revel_edit::actions::write(&rules)),
        }
    })
}

/// Passage à une variante au clic : ajouté ou retiré.
pub fn set_variant_click(ui: Ui, ed: Ed, comp_id: &str, vid: &str, on: bool) {
    let Some(c) = component(ed, comp_id) else {
        return;
    };
    if let Some(cmd) = click_command(&c, vid, on) {
        ed.apply(ui, cmd, false);
    }
}

/// Commande qui donne une classe à l'élément racine du composant s'il n'en a pas (les styles
/// des variantes s'écrivent à partir d'elle) : d'après le nom du composant.
fn root_class_command(d: &revel_model::Site, c: &Component) -> Option<Command> {
    if c.root_class().is_some() {
        return None;
    }
    let Some(Node::Element {
        id: Some(root),
        attrs,
        ..
    }) = c.root()
    else {
        return None;
    };
    let base = revel_model::slugify(&c.label());
    let base = if base.is_empty() {
        "composant".to_string()
    } else {
        base
    };
    let taken: std::collections::HashSet<String> = d
        .style_rules()
        .flat_map(|r| {
            r.selector
                .split(|ch: char| !(ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '.'))
                .flat_map(|p| p.split('.').skip(1).map(str::to_string).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        })
        .collect();
    let class = (1..)
        .map(|i| {
            if i == 1 {
                base.clone()
            } else {
                format!("{base}-{i}")
            }
        })
        .find(|c| !taken.contains(c))
        .unwrap_or(base);
    let cur = attrs
        .iter()
        .find(|a| a.name == "class")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    Some(Command::SetAttr {
        at: Target {
            owner: Owner::Component(c.id.clone()),
            node: root.clone(),
        },
        name: "class".into(),
        value: Some(format!("{class} {cur}").trim().to_string()),
    })
}

/// Ajoute une variante au composant ouvert et l'affiche ; geste déjà présent : affiché.
pub fn add_variant(ui: Ui, ed: Ed, comp_id: &str, kind: VariantKind, click: bool) {
    let Some(c) = component(ed, comp_id) else {
        return;
    };
    let fixed = match kind {
        VariantKind::Hover => Some("survol"),
        VariantKind::Pressed => Some("appui"),
        VariantKind::Custom => None,
    };
    if let Some(v) = c
        .variants
        .iter()
        .find(|v| v.kind == kind && kind != VariantKind::Custom)
    {
        ed.variant.set(v.id.clone());
        return;
    }
    let comp_id = comp_id.to_string();
    let create = move |name: Option<String>, id: String| {
        let Some(c) = component(ed, &comp_id) else {
            return;
        };
        let mut commands: Vec<Command> = ed
            .doc
            .with_untracked(|d| d.as_ref().and_then(|d| root_class_command(d, &c)))
            .into_iter()
            .collect();
        let mut variants = c.variants.clone();
        variants.push(Variant {
            id: id.clone(),
            name,
            kind,
        });
        commands.push(Command::SetComponentVariants {
            id: comp_id.clone(),
            variants,
        });
        if click && let Some(cmd) = click_command(&c, &id, true) {
            commands.push(cmd);
        }
        ed.apply(ui, Command::Batch { commands }, false);
        ed.variant.set(id);
    };
    match fixed {
        Some(id) => create(None, id.to_string()),
        None => {
            let taken: Vec<String> = c.variants.iter().map(|v| v.id.clone()).collect();
            let n = c
                .variants
                .iter()
                .filter(|v| v.kind == VariantKind::Custom)
                .count()
                + 1;
            ui.ask.set(Some(crate::dialog::Ask::input(
                ui.now("variant-new", &[]),
                ui.now("variant-name", &[]),
                ui.now("variant-default-name", &[("n", n.to_string())]),
                ui.now("variant-create", &[]),
                move |name: String| {
                    let base = revel_model::slugify(&name);
                    let base = if revel_model::valid_variant_id(&base) {
                        base
                    } else {
                        "variante".into()
                    };
                    let id = (1..)
                        .map(|i| {
                            if i == 1 {
                                base.clone()
                            } else {
                                format!("{base}-{i}")
                            }
                        })
                        .find(|x| !taken.contains(x) && !matches!(x.as_str(), "survol" | "appui"))
                        .unwrap_or(base);
                    create(Some(name.trim().to_string()).filter(|n| !n.is_empty()), id)
                },
            )));
        }
    }
}

/// Renomme une variante (nom affiché ; l'identifiant et la classe ne changent pas).
pub fn rename_variant(ui: Ui, ed: Ed, comp_id: &str, vid: &str) {
    let Some(c) = component(ed, comp_id) else {
        return;
    };
    let Some(v) = c.variant(vid).cloned() else {
        return;
    };
    let comp_id = comp_id.to_string();
    let vid = vid.to_string();
    ui.ask.set(Some(crate::dialog::Ask::input(
        ui.now("variant-rename", &[]),
        ui.now("variant-name", &[]),
        variant_label(ui, Some(&v)),
        ui.now("variant-rename", &[]),
        move |name: String| {
            let Some(c) = component(ed, &comp_id) else {
                return;
            };
            let variants = c
                .variants
                .iter()
                .map(|x| {
                    let mut x = x.clone();
                    if x.id == vid {
                        x.name = Some(name.trim().to_string()).filter(|n| !n.is_empty());
                    }
                    x
                })
                .collect();
            ed.apply(
                ui,
                Command::SetComponentVariants {
                    id: comp_id.clone(),
                    variants,
                },
                false,
            );
        },
    )));
}

/// Retire une variante : ses styles (variante personnalisée) et son choix dans les utilisations
/// avec elle ; les styles de survol et d'appui restent (ils peuvent venir du site).
pub fn remove_variant(ui: Ui, ed: Ed, comp_id: &str, vid: &str) {
    let Some(c) = component(ed, comp_id) else {
        return;
    };
    let Some(v) = c.variant(vid).cloned() else {
        return;
    };
    let comp_id = comp_id.to_string();
    let vid = vid.to_string();
    let name = variant_label(ui, Some(&v));
    ui.ask.set(Some(
        crate::dialog::Ask::confirm(
            ui.now("variant-remove-title", &[("name", name)]),
            None,
            ui.now("action-remove", &[]),
            move || {
                let Some(c) = component(ed, &comp_id) else {
                    return;
                };
                let mut commands = vec![Command::SetComponentVariants {
                    id: comp_id.clone(),
                    variants: c.variants.iter().filter(|x| x.id != vid).cloned().collect(),
                }];
                ed.doc.with_untracked(|d| {
                    let Some(d) = d.as_ref() else { return };
                    if v.kind == VariantKind::Custom
                        && let Some(root) = c.root_class()
                    {
                        let prefix = format!(".{root}{}", v.suffix());
                        for r in d.style_rules() {
                            let hit = r.selector == prefix
                                || r.selector.starts_with(&format!("{prefix} "))
                                || r.selector.starts_with(&format!("{prefix}:"));
                            if hit && let Some(id) = &r.id {
                                commands.push(Command::RemoveRule { rule: id.clone() });
                            }
                        }
                    }
                    for (kind, owner, body) in d.owners() {
                        revel_model::ids::walk(body, &mut |n| {
                            if let Node::Use {
                                id: Some(uid),
                                component,
                                variant: Some(x),
                                ..
                            } = n
                                && *component == comp_id
                                && *x == vid
                            {
                                let owner = match kind {
                                    revel_model::ids::OwnerKind::Page => {
                                        Owner::Page(owner.to_string())
                                    }
                                    revel_model::ids::OwnerKind::Layout => {
                                        Owner::Layout(owner.to_string())
                                    }
                                    revel_model::ids::OwnerKind::Component => {
                                        Owner::Component(owner.to_string())
                                    }
                                };
                                commands.push(Command::SetUseVariant {
                                    at: Target {
                                        owner,
                                        node: uid.clone(),
                                    },
                                    variant: None,
                                });
                            }
                        });
                    }
                });
                if ed.variant.get_untracked() == vid {
                    ed.variant.set(String::new());
                }
                ed.apply(ui, Command::Batch { commands }, false);
            },
        )
        .danger(),
    ));
}

/// Renomme un composant (nom affiché ; l'identifiant ne change pas).
pub fn rename(ui: Ui, ed: Ed, id: &str) {
    let Some(c) = component(ed, id) else { return };
    let id = id.to_string();
    ui.ask.set(Some(crate::dialog::Ask::input(
        ui.now("component-rename", &[]),
        ui.now("component-make-name", &[]),
        crate::names::component_name(&c),
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
            ui.now(
                "component-delete-title",
                &[("name", crate::names::component_name(&c))],
            ),
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

/// Transitions proposées pour les variantes de survol et d'appui : durée, clé du libellé.
const TRANSITIONS: &[(&str, &str)] = &[
    ("", "variant-transition-none"),
    ("0.15s", "variant-transition-fast"),
    ("0.3s", "variant-transition-smooth"),
    ("0.6s", "variant-transition-slow"),
];

/// Transition des variantes de survol et d'appui (comme les transitions de Framer) : règle
/// `.racine, .racine *` de la feuille du site (propriétés visuelles, durée, courbe douce).
#[component]
pub fn TransitionSelect(id: String) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let id = StoredValue::new(id);
    let selector = Memo::new(move |_| {
        ed.doc.with(|d| {
            let r = d.as_ref()?.component(&id.get_value())?.root_class()?;
            Some(format!(".{r}, .{r} *"))
        })
    });
    let rule = Memo::new(move |_| {
        let sel = selector.get()?;
        ed.doc
            .with(|d| crate::styles::find_rule(d.as_ref()?, &sel, &None))
    });
    let current = move || {
        rule.with(|r| {
            r.as_ref()
                .and_then(|r| revel_style::values::read(&r.decls, "transition-duration"))
                .and_then(|f| f.value)
        })
        .unwrap_or_default()
    };
    let change = move |v: String| {
        let Some(sel) = selector.get_untracked() else {
            return;
        };
        let on = !v.is_empty();
        let list = vec![
            (
                "transition-property".to_string(),
                on.then(|| {
                    "color, background-color, border-color, opacity, transform, box-shadow, filter"
                        .to_string()
                }),
            ),
            ("transition-duration".to_string(), on.then(|| v.clone())),
            (
                "transition-timing-function".to_string(),
                on.then(|| "cubic-bezier(.2, .8, .2, 1)".to_string()),
            ),
        ];
        if let Some(cmd) = crate::styles::style_command(rule.get_untracked(), sel, None, list) {
            ed.apply(ui, cmd, false);
        }
    };
    view! {
        <span class="select select--small comp-transition" title=ui.t("variant-transition-help")
            class:is-disabled=move || selector.with(Option::is_none)>
            <select class="input" aria-label=ui.t("variant-transition") prop:disabled=move || selector.with(Option::is_none)
                on:change=move |e| change(event_target_value(&e))>
                {TRANSITIONS.iter().map(|(v, k)| {
                    let v: &'static str = v;
                    view! { <option value=v selected=move || current() == v>{ui.t(k)}</option> }
                }).collect_view()}
            </select>
        </span>
    }
}
