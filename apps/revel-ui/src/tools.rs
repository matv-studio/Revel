//! Panneaux des outils fermés (D-176) : onglets, accordéon, barre de navigation, et textes des
//! parties d'un outil (réglés ici avec leur nom, à la place du champ « Texte » générique).

use crate::editor::Ed;
use crate::toolparts::{Part, PartsList, select_row, switch_row, text_row};
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::{Command, Target};
use revel_model::{Attr, Node, PART_ATTR, Site};

fn attr_of(attrs: &[Attr], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.clone().unwrap_or_default())
}

fn has(attrs: &[Attr], name: &str) -> bool {
    attrs.iter().any(|a| a.name == name)
}

/// Outil fermé : ses textes se règlent dans son panneau, pas dans le champ « Texte ».
pub fn is_closed_tool(n: &Node) -> bool {
    matches!(n, Node::Element { attrs, .. }
        if has(attrs, revel_model::WIDGET_ATTR) && has(attrs, revel_model::BASE_ATTR))
}

fn first_plain(n: &Node) -> Option<String> {
    match n {
        Node::Text { text } => {
            let t = text.trim();
            (!t.is_empty()).then(|| t.to_string())
        }
        Node::Element { children, .. } => children.iter().find_map(first_plain),
        _ => None,
    }
}

/// Premier descendant qui porte la partie `kind` : identifiant.
fn part_id(n: &Node, kind: &str) -> Option<String> {
    let Node::Element {
        id,
        attrs,
        children,
        ..
    } = n
    else {
        return None;
    };
    if attr_of(attrs, PART_ATTR).as_deref() == Some(kind) {
        return id.clone();
    }
    children.iter().find_map(|c| part_id(c, kind))
}

/// Première icône (`data-icon`) dans un élément : identifiant et nom.
fn icon_of(n: &Node) -> Option<(String, String)> {
    let Node::Element {
        id,
        attrs,
        children,
        ..
    } = n
    else {
        return None;
    };
    if let (Some(id), Some(name)) = (id, attr_of(attrs, "data-icon")) {
        return Some((id.clone(), name));
    }
    children.iter().find_map(icon_of)
}

fn icon_button(ui: Ui, name: &str, key: &'static str, open: RwSignal<bool>) -> AnyView {
    let path = revel_edit::icons::icon(name).unwrap_or_default();
    view! {
        <button type="button" class="icon-btn carousel-icon tool-icon" class:is-on=move || open.get()
            title=ui.t(key) aria-label=ui.t(key) on:click=move |_| open.update(|o| *o = !*o)>
            <svg class="icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor"
                stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" inner_html=path></svg>
        </button>
    }
    .into_any()
}

// ------------------------------------------------------------------ onglets

/// Onglets listés (renommer, ordonner, retirer, ajouter) et onglet ouvert au départ.
pub fn tabs(ui: Ui, ed: Ed, at: Target, options: String) -> AnyView {
    let at = StoredValue::new(at);
    let options = StoredValue::new(options);
    let parts = Memo::new(move |_| {
        let at = at.get_value();
        ed.doc.with(|d| {
            let d = d.as_ref()?;
            let m = revel_edit::tabs::map(d, &at);
            if m.repeated {
                return None;
            }
            Some(
                m.tabs
                    .iter()
                    .enumerate()
                    .filter_map(|(i, t)| {
                        let n = revel_edit::address::node(d, t).ok()?;
                        let tag = match n {
                            Node::Element { tag, .. } => tag.clone(),
                            _ => String::new(),
                        };
                        Some(Part {
                            id: t.node.clone(),
                            tag,
                            label: t.node.clone(),
                            name: first_plain(n).unwrap_or_else(|| {
                                ui.now("items-tab", &[("n", (i + 1).to_string())])
                            }),
                            extra: None,
                        })
                    })
                    .collect::<Vec<_>>(),
            )
        })
    });
    let run = move |f: &dyn Fn(&Site, &Target) -> Option<Command>, keep: bool| {
        let cmd = ed
            .doc
            .with_untracked(|d| d.as_ref().and_then(|d| f(d, &at.get_value())));
        if let Some(c) = cmd {
            if keep {
                ed.apply_keep(ui, c);
            } else {
                ed.apply(ui, c, false);
            }
        }
    };
    let on_add = Callback::new(move |_| run(&|d, at| revel_edit::tabs::add_tab(d, at), true));
    let on_remove =
        Callback::new(move |i: usize| run(&|d, at| revel_edit::tabs::remove_tab(d, at, i), false));
    let on_move = Callback::new(move |(i, dir): (usize, i32)| {
        run(&|d, at| revel_edit::tabs::move_tab(d, at, i, dir), true)
    });
    let start = move || {
        options.with_value(|o| {
            o.split_whitespace()
                .find_map(|x| x.strip_prefix("start="))
                .unwrap_or("1")
                .to_string()
        })
    };
    let set_start = move |v: String| {
        let mut list: Vec<String> = options.with_value(|o| {
            o.split_whitespace()
                .filter(|x| !x.starts_with("start="))
                .map(str::to_string)
                .collect()
        });
        if v != "1" {
            list.push(format!("start={v}"));
        }
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: "data-revel-tabs".into(),
                value: Some(list.join(" ")),
            },
            false,
        );
    };
    (move || {
        parts.get().map(|list| {
            let names: Vec<(String, String)> = list
                .iter()
                .enumerate()
                .map(|(i, p)| ((i + 1).to_string(), p.name.clone()))
                .collect();
            let list = Signal::derive(move || parts.get().unwrap_or_default());
            view! {
                <PartsList owner=at.get_value().owner title="items-tabs" add_key="items-add-tab" parts=list
                    icon=icons::tabs on_add=on_add on_remove=on_remove on_move=on_move min=2/>
                {select_row(ui, "tabs-start", names, start(), set_start)}
            }
        })
    })
    .into_any()
}

// ------------------------------------------------------------------ accordéon

/// Volets d'un accordéon (`data-revel-accordion`) : identifiant, ouvert, nom d'exclusivité,
/// élément du nom, icône.
struct Row {
    id: String,
    open: bool,
    group: bool,
    label: String,
    name: String,
    icon: Option<(String, String)>,
}

fn rows_of(n: &Node) -> Vec<Row> {
    let Node::Element { children, .. } = n else {
        return Vec::new();
    };
    children
        .iter()
        .filter_map(|c| match c {
            Node::Element {
                id: Some(id),
                tag,
                attrs,
                ..
            } if tag == "details" => {
                let label = part_id(c, "name")
                    .or_else(|| part_id(c, "summary"))
                    .unwrap_or_else(|| id.clone());
                Some(Row {
                    id: id.clone(),
                    open: has(attrs, "open"),
                    group: has(attrs, "name"),
                    label,
                    name: first_plain(c).unwrap_or_default(),
                    icon: icon_of(c),
                })
            }
            _ => None,
        })
        .collect()
}

/// Accordéon : volets listés, volet ouvert au départ, un seul ouvert à la fois, icône.
pub fn accordion(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    let Node::Element { attrs, .. } = node else {
        return None;
    };
    if !has(attrs, "data-revel-accordion") {
        return None;
    }
    let root = StoredValue::new(at.clone());
    let owner = at.owner.clone();
    let target = {
        let owner = owner.clone();
        move |id: &str| Target {
            owner: owner.clone(),
            node: id.to_string(),
        }
    };
    let rows = rows_of(node);
    let opened: Vec<(String, bool)> = rows.iter().map(|r| (r.id.clone(), r.open)).collect();
    let grouped: Vec<(String, bool)> = rows.iter().map(|r| (r.id.clone(), r.group)).collect();
    let single = !rows.is_empty() && rows.iter().all(|r| r.group);
    let start = rows
        .iter()
        .position(|r| r.open)
        .map(|i| (i + 1).to_string())
        .unwrap_or_else(|| "0".into());
    let mut starts = vec![("0".to_string(), ui.now("accordion-start-none", &[]))];
    starts.extend(
        rows.iter()
            .enumerate()
            .map(|(i, r)| ((i + 1).to_string(), r.name.clone())),
    );
    let icons_of: Vec<(String, String)> = rows.iter().filter_map(|r| r.icon.clone()).collect();
    let icon_side = attr_of(attrs, "data-revel-icon").unwrap_or_else(|| "end".into());
    // Lignes relues dans le document : la liste suit les modifications.
    let parts = {
        Signal::derive(move || {
            ed.doc.with(|d| {
                d.as_ref()
                    .and_then(|d| revel_edit::address::node(d, &root.get_value()).ok())
                    .map(|n| {
                        rows_of(n)
                            .into_iter()
                            .map(|r| Part {
                                id: r.id,
                                tag: "details".into(),
                                label: r.label,
                                name: r.name,
                                extra: None,
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            })
        })
    };
    let set_start = {
        let target = target.clone();
        move |v: String| {
            let pick = v.parse::<usize>().unwrap_or(0);
            // Attribut posé sur le volet choisi, retiré des volets qui le portent.
            let commands: Vec<Command> = opened
                .iter()
                .enumerate()
                .filter_map(|(i, (id, open))| match (i + 1 == pick, *open) {
                    (true, false) => Some(Command::SetAttr {
                        at: target(id),
                        name: "open".into(),
                        value: None,
                    }),
                    (false, true) => Some(Command::RemoveAttr {
                        at: target(id),
                        name: "open".into(),
                    }),
                    _ => None,
                })
                .collect();
            if !commands.is_empty() {
                ed.apply(ui, Command::Batch { commands }, false);
            }
        }
    };
    let set_single = {
        let target = target.clone();
        move |on: bool| {
            let name = format!("accordeon-{}", root.get_value().node);
            let commands: Vec<Command> = grouped
                .iter()
                .filter_map(|(id, has)| match (on, *has) {
                    (true, _) => Some(Command::SetAttr {
                        at: target(id),
                        name: "name".into(),
                        value: Some(name.clone()),
                    }),
                    (false, true) => Some(Command::RemoveAttr {
                        at: target(id),
                        name: "name".into(),
                    }),
                    _ => None,
                })
                .collect();
            if !commands.is_empty() {
                ed.apply(ui, Command::Batch { commands }, false);
            }
        }
    };
    let picker = RwSignal::new(false);
    let icon_row = icons_of.first().cloned().map(|(first, name)| {
        let also: Vec<Target> = icons_of.iter().skip(1).map(|(id, _)| target(id)).collect();
        let (first_at, name2) = (target(&first), name.clone());
        let sides = vec![
            ("end".to_string(), ui.now("accordion-icon-end", &[])),
            ("start".to_string(), ui.now("accordion-icon-start", &[])),
        ];
        // Le « plus » tourne d'un huitième de tour (croix), les autres d'un demi-tour.
        view! {
            <div class="style-row" data-row="accordion-icon">
                <span class="style-label" title=ui.t("accordion-icon-help")><span class="style-label-text">{ui.t("accordion-icon")}</span></span>
                <div class="style-control carousel-icons">{icon_button(ui, &name, "accordion-icon", picker)}</div>
            </div>
            {move || picker.get().then(|| view! { <crate::blocks::IconPicker at=first_at.clone() current=name2.clone() also=also.clone()/> })}
            {select_row(ui, "accordion-icon-side", sides, icon_side.clone(), move |v| {
                let at = root.get_value();
                ed.apply(ui, if v == "start" {
                    Command::SetAttr { at, name: "data-revel-icon".into(), value: Some(v) }
                } else {
                    Command::RemoveAttr { at, name: "data-revel-icon".into() }
                }, false)
            })}
        }
    });
    Some(
        view! {
            <div class="insp-group accordion-settings" data-tool="accordion">
                <p class="panel-title tool-head">{icons::accordion()}<span>{ui.t("insert-accordion")}</span></p>
                <PartsList owner=owner title="accordion-rows" add_key="accordion-add" parts=parts icon=icons::accordion/>
                {select_row(ui, "accordion-start", starts, start, set_start)}
                {switch_row(ui, "accordion-single", single, set_single)}
                {icon_row}
            </div>
        }
        .into_any(),
    )
}

// ------------------------------------------------------------------ barre de navigation

thread_local! {
    /// Barre de navigation dont le menu est montré ouvert dans le canevas (session en cours).
    static MENU_SHOWN: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Barre de navigation : marque (texte ou logo), bouton du menu sur téléphone (icône ou texte).
pub fn navbar(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    let Node::Element {
        attrs, children, ..
    } = node
    else {
        return None;
    };
    if attr_of(attrs, revel_model::BASE_ATTR).as_deref() != Some("navbar") {
        return None;
    }
    let owner = at.owner.clone();
    let target = {
        let owner = owner.clone();
        move |id: &str| Target {
            owner: owner.clone(),
            node: id.to_string(),
        }
    };
    // Marque : premier lien enfant direct de la barre.
    let brand = children.iter().find_map(|c| match c {
        Node::Element {
            id: Some(id), tag, ..
        } if tag == "a" => Some((id.clone(), c)),
        _ => None,
    });
    let brand_row = brand.map(|(id, n)| {
        let logo = revel_edit::make::first_image(n).is_some();
        let has_text = first_plain(n).is_some();
        let id = StoredValue::new(id);
        let owner = StoredValue::new(owner.clone());
        // Logo : image choisie dans les médias, à la place du texte de la marque.
        let pick = move |_| {
            crate::library::pick_image(ui, crate::library::media_shelf(ed), move |url: String| {
                let cmd = ed.doc.with_untracked(|d| {
                    let d = d.as_ref()?;
                    let at = Target {
                        owner: owner.get_value(),
                        node: id.get_value(),
                    };
                    let Node::Element { children, .. } = revel_edit::address::node(d, &at).ok()?
                    else {
                        return None;
                    };
                    let parent = revel_edit::Parent::of(&at, revel_edit::Branch::Children);
                    let index = children
                        .iter()
                        .position(|c| matches!(c, Node::Image(_)) || matches!(c, Node::Text { text } if !text.trim().is_empty()))?;
                    let alt = first_plain(revel_edit::address::node(d, &at).ok()?).unwrap_or_default();
                    let image = match &children[index] {
                        Node::Image(i) => revel_model::ImageNode {
                            src: revel_edit::canvas::quote(&url),
                            ..i.clone()
                        },
                        _ => revel_model::ImageNode {
                            id: Some(revel_model::ids::fresh(&d.node_ids())),
                            src: revel_edit::canvas::quote(&url),
                            alt: Some(revel_edit::canvas::quote(&alt)),
                            ..Default::default()
                        },
                    };
                    Some(Command::Replace {
                        parent,
                        index,
                        node: Node::Image(image),
                    })
                });
                if let Some(c) = cmd {
                    ed.apply(ui, c, false);
                }
            });
        };
        view! {
            {has_text.then(|| text_row(ui, ed, owner.get_value(), id.get_value(), "navbar-brand", None))}
            <div class="style-row" data-row="navbar-logo">
                <span class="style-label" title=ui.t("navbar-logo-help")><span class="style-label-text">{ui.t("navbar-logo")}</span></span>
                <div class="style-control">
                    <button type="button" class="btn btn--small navbar-logo-pick" on:click=pick>
                        {icons::image()}<span>{ui.t(if logo { "items-replace" } else { "navbar-logo-choose" })}</span>
                    </button>
                </div>
            </div>
        }
    });
    // Bouton du menu (téléphone) : icône remplaçable, sinon son texte.
    let toggle = children.iter().find(|c| {
        matches!(c, Node::Element { attrs, .. } if attr_of(attrs, PART_ATTR).as_deref() == Some("toggle"))
    });
    let picker = RwSignal::new(false);
    let toggle_row = toggle.map(|t| match icon_of(t) {
        Some((id, name)) => {
            let (at, name2) = (target(&id), name.clone());
            view! {
                <div class="style-row" data-row="navbar-toggle">
                    <span class="style-label" title=ui.t("navbar-toggle-help")><span class="style-label-text">{ui.t("navbar-toggle")}</span></span>
                    <div class="style-control carousel-icons">{icon_button(ui, &name, "navbar-toggle", picker)}</div>
                </div>
                {move || picker.get().then(|| view! { <crate::blocks::IconPicker at=at.clone() current=name2.clone()/> })}
            }
            .into_any()
        }
        None => match t {
            Node::Element { id: Some(id), .. } => {
                text_row(ui, ed, at.owner.clone(), id.clone(), "navbar-toggle", Some("navbar-toggle-help"))
            }
            _ => ().into_any(),
        },
    });
    // Largeur d'écran sous laquelle le menu se replie derrière son bouton.
    let collapse = attr_of(attrs, "data-revel-collapse").unwrap_or_else(|| "phone".into());
    let widths = ["tablet", "phone", "small", "always", "never"]
        .into_iter()
        .map(|w| (w.to_string(), ui.now(&format!("navbar-collapse-{w}"), &[])))
        .collect();
    let root = StoredValue::new(at.clone());
    let collapse_row = select_row(ui, "navbar-collapse", widths, collapse, move |v| {
        let at = root.get_value();
        ed.apply(
            ui,
            if v == "phone" {
                Command::RemoveAttr {
                    at,
                    name: "data-revel-collapse".into(),
                }
            } else {
                Command::SetAttr {
                    at,
                    name: "data-revel-collapse".into(),
                    value: Some(v),
                }
            },
            false,
        )
    });
    // Menu montré ouvert dans le canevas (comme « Open menu » de Webflow) : le temps du réglage,
    // sans rien écrire dans le site.
    let id = at.node.clone();
    let shown = MENU_SHOWN.with(|m| m.borrow().as_deref() == Some(id.as_str()));
    let preview = {
        let id = id.clone();
        move |on: bool| {
            ed.post(
                serde_json::json!({ "type": "tool-preview", "id": id, "cls": "is-open", "on": on }),
            );
        }
    };
    if shown {
        preview(true);
    }
    {
        let preview = preview.clone();
        on_cleanup(move || preview(false));
    }
    let open_row = {
        let id = id.clone();
        switch_row(ui, "navbar-open", shown, move |on| {
            MENU_SHOWN.with(|m| *m.borrow_mut() = on.then(|| id.clone()));
            preview(on);
        })
    };
    Some(
        view! {
            <div class="insp-group navbar-settings" data-tool="navbar">
                <p class="panel-title tool-head">{icons::navbar()}<span>{ui.t("insert-navbar")}</span></p>
                {brand_row}
                {collapse_row}
                {toggle_row}
                {open_row}
            </div>
        }
        .into_any(),
    )
}

// ------------------------------------------------------------------ textes des parties

/// Parties d'un outil qui portent un texte écrit (hors contenus accueillis) : identifiant,
/// nature de la partie, balise.
fn text_parts(n: &Node, root: bool, out: &mut Vec<(String, String, String, String)>) {
    let Node::Element {
        id,
        tag,
        attrs,
        children,
    } = n
    else {
        return;
    };
    let part = attr_of(attrs, PART_ATTR);
    // Contenu accueilli (calque ordinaire) : ses textes se règlent sur lui.
    if !root && part.is_none() {
        return;
    }
    if let (Some(id), Some(part)) = (id, &part)
        && children
            .iter()
            .any(|c| matches!(c, Node::Text { text } if !text.trim().is_empty()))
    {
        out.push((
            id.clone(),
            part.clone(),
            tag.to_ascii_lowercase(),
            attr_of(attrs, "data-revel-consent-choice").unwrap_or_default(),
        ));
    }
    for c in children {
        text_parts(c, false, out);
    }
}

/// Textes des parties d'un outil fermé, nommés d'après la partie (bouton, libellé, titre…).
/// Les outils qui règlent leurs textes dans leur propre liste (onglets, filtres, accordéon,
/// carrousel, sommaire, barre de navigation) n'en montrent pas ici.
pub fn texts(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    if !is_closed_tool(node) {
        return None;
    }
    let Node::Element { attrs, .. } = node else {
        return None;
    };
    let base = attr_of(attrs, revel_model::BASE_ATTR).unwrap_or_default();
    if matches!(
        base.as_str(),
        "tabs" | "filters" | "accordion-group" | "carousel" | "slideshow" | "toc" | "navbar"
    ) {
        return None;
    }
    let mut parts = Vec::new();
    text_parts(node, true, &mut parts);
    if parts.is_empty() {
        return None;
    }
    let rows = parts
        .into_iter()
        .map(|(id, part, tag, choice)| {
            let key = match (part.as_str(), tag.as_str()) {
                _ if choice == "accept" => "tooltext-accept",
                _ if choice == "refuse" => "tooltext-refuse",
                ("label", _) | (_, "label") => "tooltext-label",
                ("title", _) | ("summary", _) => "tooltext-title",
                ("state", _) => "tooltext-state",
                (_, "button") => "tooltext-button",
                (_, "a") => "tooltext-link",
                _ => "tooltext-text",
            };
            text_row(ui, ed, at.owner.clone(), id, key, None)
        })
        .collect_view();
    Some(
        view! {
            <div class="insp-group tool-texts">
                <p class="panel-title">{ui.t("tooltext-title-group")}</p>
                {rows}
            </div>
        }
        .into_any(),
    )
}

// ------------------------------------------------------------------ sélecteur de langue

/// Lien modèle du sélecteur de langue : identifiant du lien, rang et expression de son texte.
fn locale_link(n: &Node) -> Option<(String, usize, String)> {
    match n {
        Node::Each { expr, children, .. } if expr.trim() == "locales" => {
            children.iter().find_map(|c| match c {
                Node::Element {
                    id: Some(id),
                    children,
                    ..
                } => children.iter().enumerate().find_map(|(i, x)| match x {
                    Node::Out { expr } => Some((id.clone(), i, expr.clone())),
                    _ => None,
                }),
                _ => None,
            })
        }
        Node::Element { children, .. } => children.iter().find_map(locale_link),
        _ => None,
    }
}

/// Sélecteur de langue : nom de la langue (« Français ») ou son code (« fr »).
pub fn locales(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    let Node::Element { attrs, .. } = node else {
        return None;
    };
    if attr_of(attrs, revel_model::BASE_ATTR).as_deref() != Some("locales") {
        return None;
    }
    let (link, index, expr) = locale_link(node)?;
    let item = expr.split('.').next().unwrap_or("l").trim().to_string();
    let current = if expr.trim().ends_with(".code") {
        "code"
    } else {
        "name"
    };
    let options = vec![
        ("name".to_string(), ui.now("locales-show-name", &[])),
        ("code".to_string(), ui.now("locales-show-code", &[])),
    ];
    let parent = revel_edit::Parent {
        owner: at.owner.clone(),
        node: Some(link),
        branch: revel_edit::Branch::Children,
    };
    Some(
        view! {
            <div class="insp-group locales-settings" data-tool="locales">
                <p class="panel-title tool-head">{icons::globe()}<span>{ui.t("insert-locale-switch")}</span></p>
                {select_row(ui, "locales-show", options, current.to_string(), move |v| {
                    ed.apply(ui, Command::Replace {
                        parent: parent.clone(),
                        index,
                        node: Node::Out { expr: format!("{item}.{v}") },
                    }, false)
                })}
            </div>
        }
        .into_any(),
    )
}

// ------------------------------------------------------------------ lecteur vidéo

/// Premier dessin `svg` dans un élément : identifiant et nom de l'icône (vide : dessin d'origine).
fn svg_of(n: &Node) -> Option<(String, String)> {
    let Node::Element {
        id,
        tag,
        attrs,
        children,
    } = n
    else {
        return None;
    };
    if tag == "svg"
        && let Some(id) = id
    {
        return Some((id.clone(), attr_of(attrs, "data-icon").unwrap_or_default()));
    }
    children.iter().find_map(svg_of)
}

/// Élément qui porte l'attribut `name` : identifiant et valeur.
fn attr_holder(n: &Node, name: &str) -> Option<(String, String)> {
    let Node::Element {
        id,
        attrs,
        children,
        ..
    } = n
    else {
        return None;
    };
    if let (Some(id), Some(v)) = (id, attr_of(attrs, name)) {
        return Some((id.clone(), v));
    }
    children.iter().find_map(|c| attr_holder(c, name))
}

/// Lecteur vidéo : icône du bouton de lecture, nom du bouton (la couverture est son image).
pub fn video_player(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    let Node::Element { attrs, .. } = node else {
        return None;
    };
    if attr_of(attrs, revel_model::BASE_ATTR).as_deref() != Some("video-player") {
        return None;
    }
    let owner = StoredValue::new(at.owner.clone());
    let target = move |id: &str| Target {
        owner: owner.get_value(),
        node: id.to_string(),
    };
    let picker = RwSignal::new(false);
    let icon_row = svg_of(node).map(|(id, name)| {
        let shown = if name.is_empty() { "lecture".to_string() } else { name.clone() };
        let at = target(&id);
        view! {
            <div class="style-row" data-row="player-icon">
                <span class="style-label"><span class="style-label-text">{ui.t("accordion-icon")}</span></span>
                <div class="style-control carousel-icons">
                    <button type="button" class="icon-btn carousel-icon tool-icon" class:is-on=move || picker.get()
                        title=ui.t("accordion-icon") aria-label=ui.t("accordion-icon") data-icon=shown
                        on:click=move |_| picker.update(|o| *o = !*o)>{icons::play()}</button>
                </div>
            </div>
            {move || picker.get().then(|| view! { <crate::blocks::IconPicker at=at.clone() current=name.clone()/> })}
        }
    });
    let label_row = attr_holder(node, "aria-label").map(|(id, value)| {
        view! {
            <div class="style-row" data-row="player-label">
                <span class="style-label" title=ui.t("player-label-help")><span class="style-label-text">{ui.t("tooltext-label")}</span></span>
                <div class="style-control">
                    <input class="input" type="text" prop:value=value aria-label=ui.t("tooltext-label")
                        on:change=move |e| ed.apply(ui, Command::SetAttr {
                            at: target(&id), name: "aria-label".into(), value: Some(event_target_value(&e).trim().to_string()),
                        }, false)/>
                </div>
            </div>
        }
    });
    Some(
        view! {
            <div class="insp-group player-settings" data-tool="video-player">
                {icon_row}
                {label_row}
            </div>
        }
        .into_any(),
    )
}

// ------------------------------------------------------------------ blocs qui portent un outil

/// Premier descendant qui vérifie `test` (pas l'élément lui-même) : identifiant et nœud.
pub fn inner_tool<'a>(
    n: &'a Node,
    test: &dyn Fn(&str, &[Attr]) -> bool,
) -> Option<(String, &'a Node)> {
    let Node::Element { children, .. } = n else {
        return None;
    };
    for c in children {
        if let Node::Element {
            id: Some(id),
            tag,
            attrs,
            ..
        } = c
            && test(tag, attrs)
        {
            return Some((id.clone(), c));
        }
        if let Some(found) = inner_tool(c, test) {
            return Some(found);
        }
    }
    None
}
