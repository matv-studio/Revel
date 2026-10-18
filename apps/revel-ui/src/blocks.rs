//! Réglages des éléments de 3.7 : icône (choix du dessin), code HTML, vidéo (lecture), liste de
//! collection (collection, filtre, tri, nombre, comme la « Collection List » de Webflow) et
//! éléments d'une liste reliés aux champs du contenu (texte, image, lien).

use crate::editor::Ed;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::collection::{FieldList, Query};
use revel_edit::{Branch, Command, Parent, Target};
use revel_model::{Attr, Node, Site};
use revel_protocol::CmsCollection;

fn attr<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a Attr> {
    attrs.iter().find(|a| a.name == name)
}

/// Collections du site (schéma de Pages CMS), relues à la demande.
pub(crate) fn collections(ed: Ed) -> RwSignal<Vec<CmsCollection>> {
    let cols = RwSignal::new(Vec::new());
    let path = ed.path.get_value();
    spawn_local(async move {
        if let Ok(c) = api::cms_collections(&path).await {
            let _ = cols.try_set(c);
        }
    });
    cols
}

fn label_of(c: &CmsCollection) -> String {
    c.label
        .clone()
        .unwrap_or_else(|| crate::site::capitalize(&c.id))
}

/// Réglages propres à l'élément sélectionné (en plus des réglages communs).
pub fn extra_settings(ui: Ui, ed: Ed, at: Target, node: Node) -> Option<AnyView> {
    let mut parts: Vec<AnyView> = Vec::new();
    if let Node::Element {
        tag,
        attrs,
        children,
        ..
    } = &node
    {
        let lower = tag.to_ascii_lowercase();
        if lower == "svg"
            && let Some(name) = attr(attrs, "data-icon").and_then(|a| a.value.clone())
        {
            parts.push(icon_picker(ui, ed, at.clone(), name));
        }
        if let [Node::Html { html }] = children.as_slice()
            && lower != "svg"
        {
            parts.push(embed_code(ui, ed, at.clone(), html.clone()));
        }
        if lower == "video" {
            parts.push(video_options(ui, ed, at.clone(), attrs.clone()));
        }
        if attr(attrs, "data-lottie").is_some() {
            parts.push(lottie_options(ui, ed, at.clone(), attrs.clone()));
        }
        if let Some(v) = attr(attrs, "data-revel-video") {
            parts.push(video_player_options(
                ui,
                ed,
                at.clone(),
                v.value.clone().unwrap_or_default(),
            ));
        }
        // Défilement (animation en boucle « ticker »), carrousel et diaporama.
        if attr(attrs, "data-revel-loop")
            .and_then(|a| a.value.as_deref())
            .is_some_and(|v| v.split_whitespace().next() == Some("ticker"))
        {
            let at = at.clone();
            parts.push(view! { <crate::interactions::TickerSettings at=at/> }.into_any());
        }
        if let Some(v) = attr(attrs, "data-revel-tabs") {
            parts.push(tabs_options(
                ui,
                ed,
                at.clone(),
                v.value.clone().unwrap_or_default(),
            ));
        }
        // Filtres de liste : groupe, bouton, élément filtré, message sans résultat.
        if let Some(v) = filter_settings(ui, ed, &at, attrs) {
            parts.push(v);
        }
        // Consentement : bandeau (identifiant de mesure), boutons, lien qui le rouvre.
        if let Some(v) = consent_settings(ui, ed, &at, &lower, attrs) {
            parts.push(v);
        }
        // Interrupteur des sons ou de la musique, et texte de son état.
        if let Some(v) = switch_settings(ui, ed, &at, &lower, attrs) {
            parts.push(v);
        }
        // Élément dans des onglets : onglet (bouton ou lien) ou panneau.
        if attr(attrs, "data-revel-tabs").is_none()
            && ed
                .doc
                .with_untracked(|d| d.as_ref().is_some_and(|d| inside_tabs(d, &at)))
        {
            let role = if attr(attrs, "data-revel-tab").is_some() {
                "tab"
            } else if attr(attrs, "data-revel-panel").is_some() {
                "panel"
            } else {
                ""
            };
            parts.push(tab_role(ui, ed, at.clone(), role));
        }
        if let Some(v) = attr(attrs, "data-revel-slider") {
            parts.push(slider_options(
                ui,
                ed,
                at.clone(),
                v.value.clone().unwrap_or_default(),
            ));
        }
        // Liste de collection : son conteneur (une seule boucle dedans).
        let loops: Vec<&Node> = children
            .iter()
            .filter(|c| matches!(c, Node::Each { .. }))
            .collect();
        if let [
            Node::Each {
                id: Some(id), expr, ..
            },
        ] = loops.as_slice()
        {
            parts.push(collection_list(
                ui,
                ed,
                Target {
                    owner: at.owner.clone(),
                    node: id.clone(),
                },
                expr.clone(),
            ));
        }
    }
    if let Node::Each {
        expr, id: Some(_), ..
    } = &node
    {
        parts.push(collection_list(ui, ed, at.clone(), expr.clone()));
    }
    // Utilisation d'un composant dans une liste : élément filtré par les filtres de la page.
    if let Node::Use { attrs, .. } = &node
        && let Some(v) = filter_settings(ui, ed, &at, attrs)
    {
        parts.push(v);
    }
    if let Some(b) = item_bindings(ui, ed, &at, &node) {
        parts.push(b);
    }
    (!parts.is_empty()).then(|| parts.into_iter().collect_view().into_any())
}

// ------------------------------------------------------------------ icône

fn icon_picker(ui: Ui, ed: Ed, at: Target, current: String) -> AnyView {
    let at = StoredValue::new(at);
    let choose = move |name: &'static str, path: &'static str| {
        let at = at.get_value();
        ed.apply(
            ui,
            Command::Batch {
                commands: vec![
                    Command::SetAttr {
                        at: at.clone(),
                        name: "data-icon".into(),
                        value: Some(name.into()),
                    },
                    Command::Replace {
                        parent: Parent::of(&at, Branch::Children),
                        index: 0,
                        node: Node::Html { html: path.into() },
                    },
                ],
            },
            false,
        );
    };
    view! {
        <div class="insp-group">
            <p class="panel-title">{ui.t("block-icon")}</p>
            <div class="icon-grid" role="listbox" aria-label=ui.t("block-icon")>
                {revel_edit::icons::ICONS.iter().map(|(name, path)| {
                    let on = *name == current;
                    view! {
                        <button type="button" class="icon-choice" class:is-on=on title=*name aria-label=*name data-icon=*name
                            on:click=move |_| choose(name, path)>
                            <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2"
                                stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" inner_html=*path></svg>
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ code HTML

fn embed_code(ui: Ui, ed: Ed, at: Target, html: String) -> AnyView {
    let at = StoredValue::new(at);
    view! {
        <div class="insp-group">
            <p class="panel-title" title=ui.t("block-embed-help")>{ui.t("block-embed")}</p>
            <textarea class="input embed-code" rows="7" spellcheck="false" prop:value=html
                aria-label=ui.t("block-embed")
                on:change=move |e| {
                    let at = at.get_value();
                    ed.apply(ui, Command::Replace {
                        parent: Parent::of(&at, Branch::Children),
                        index: 0,
                        node: Node::Html { html: event_target_value(&e) },
                    }, false);
                }></textarea>
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ vidéo

fn video_options(ui: Ui, ed: Ed, at: Target, attrs: Vec<Attr>) -> AnyView {
    let has = |n: &str| attr(&attrs, n).is_some();
    let at = StoredValue::new(at);
    // Attribut booléen ajouté ou retiré (lecture automatique : sans le son, sinon refusée).
    let toggle = move |names: &'static [&'static str], on: bool| {
        let at = at.get_value();
        let present = ed.doc.with_untracked(|d| {
            d.as_ref()
                .and_then(|d| revel_edit::address::node(d, &at).ok())
                .map(|n| match n {
                    Node::Element { attrs, .. } => {
                        attrs.iter().map(|a| a.name.clone()).collect::<Vec<_>>()
                    }
                    _ => Vec::new(),
                })
                .unwrap_or_default()
        });
        let commands: Vec<Command> = names
            .iter()
            .filter(|n| present.iter().any(|p| p == *n) != on)
            .map(|n| {
                if on {
                    Command::SetAttr {
                        at: at.clone(),
                        name: n.to_string(),
                        value: None,
                    }
                } else {
                    Command::RemoveAttr {
                        at: at.clone(),
                        name: n.to_string(),
                    }
                }
            })
            .collect();
        if !commands.is_empty() {
            ed.apply(ui, Command::Batch { commands }, false);
        }
    };
    let row = move |key: &'static str, names: &'static [&'static str], on: bool| {
        view! {
            <label class="view-line">
                <span>{ui.t(key)}</span>
                <span class="switch">
                    <input type="checkbox" prop:checked=on on:change=move |e| toggle(names, event_target_checked(&e))/>
                    <span class="switch-track"></span>
                </span>
            </label>
        }
    };
    view! {
        <div class="insp-group">
            <p class="panel-title">{ui.t("block-video")}</p>
            {row("block-video-controls", &["controls"], has("controls"))}
            {row("block-video-autoplay", &["autoplay", "muted", "playsinline"], has("autoplay"))}
            {row("block-video-loop", &["loop"], has("loop"))}
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ lecteur vidéo

/// Lecteur vidéo : adresse YouTube, Vimeo ou fichier du site (`data-revel-video`), lancée à
/// l'appui sur la couverture (image et bouton : calques ordinaires).
fn video_player_options(ui: Ui, ed: Ed, at: Target, url: String) -> AnyView {
    let at = StoredValue::new(at);
    let set = move |v: String| {
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: "data-revel-video".into(),
                value: Some(v.trim().to_string()),
            },
            false,
        );
    };
    let pick = move || crate::library::pick_media(ui, false, crate::library::media_shelf(ed), set);
    let kind = if url.contains("youtu") {
        "YouTube"
    } else if url.contains("vimeo.com") {
        "Vimeo"
    } else if url.is_empty() {
        ""
    } else {
        "file"
    };
    view! {
        <div class="insp-group">
            <p class="panel-title">{ui.t("block-video-player")}</p>
            <div class="field">
                <span class="label" title=ui.t("block-video-player-help")>{ui.t("block-video-player-url")}</span>
                <span class="field-line">
                    <input class="input input--mono" spellcheck="false" prop:value=url.clone()
                        placeholder="https://www.youtube.com/watch?v=…"
                        on:change=move |e| set(event_target_value(&e))/>
                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("block-video-player-file")
                        aria-label=ui.t("block-video-player-file") on:click=move |_| pick()>{crate::icons::play()}</button>
                </span>
            </div>
            {(!kind.is_empty()).then(|| view! {
                <p class="muted small">{if kind == "file" { ui.tx("block-video-player-file", &[]) } else { kind.to_string() }}</p>
            })}
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ filtres de liste

/// Le site contient un élément qui porte l'attribut `name` (groupe de filtres, bandeau).
fn site_has_attr(d: &Site, name: &str) -> bool {
    let mut found = false;
    let mut look = |nodes: &[Node]| {
        revel_model::ids::walk(nodes, &mut |n| {
            if let Node::Element { attrs, .. } = n
                && attrs.iter().any(|a| a.name == name)
            {
                found = true;
            }
        });
    };
    for p in &d.pages {
        look(&p.body);
    }
    for l in &d.layouts {
        look(&l.body);
    }
    for c in &d.components {
        look(&c.body);
    }
    found
}

/// Rôle d'un élément dans les filtres de liste (« Filtres » de Finsweet pour Webflow) : groupe
/// (paramètre de l'adresse), bouton (valeur écrite ou champ), élément filtré (champ liste du
/// contenu), message sans résultat. Proposé quand l'élément a un rôle ou que le site a un groupe.
fn filter_settings(ui: Ui, ed: Ed, at: &Target, attrs: &[Attr]) -> Option<AnyView> {
    let value = |n: &str| attr(attrs, n).map(|a| a.value.clone().unwrap_or_default());
    let role = if attr(attrs, "data-revel-filters").is_some() {
        "group"
    } else if attr(attrs, "data-revel-filter").is_some() {
        "button"
    } else if attr(attrs, "data-revel-tags").is_some() {
        "item"
    } else if attr(attrs, "data-revel-filter-empty").is_some() {
        "empty"
    } else {
        ""
    };
    let (has_group, list, in_list) = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        Some((
            site_has_attr(d, "data-revel-filters"),
            binding_context(d, at),
            enclosing_list(d, at).is_some(),
        ))
    })?;
    // Sans rôle : proposé aux éléments d'une liste quand la page a des filtres (les boutons et le
    // groupe viennent de l'élément « Filtres de liste » du panneau Ajouter).
    if role.is_empty() && !(has_group && in_list) {
        return None;
    }
    let names = [
        ("group", "data-revel-filters"),
        ("button", "data-revel-filter"),
        ("item", "data-revel-tags"),
        ("empty", "data-revel-filter-empty"),
    ];
    let at = StoredValue::new(at.clone());
    let set_role = move |v: String| {
        let at = at.get_value();
        let mut commands = Vec::new();
        for (r, name) in names {
            if r == role && v != r {
                commands.push(Command::RemoveAttr {
                    at: at.clone(),
                    name: name.into(),
                });
            }
            if r == v && role != r {
                commands.push(Command::SetAttr {
                    at: at.clone(),
                    name: name.into(),
                    value: match r {
                        "group" => Some("filtre".into()),
                        "button" | "item" => Some(String::new()),
                        _ => None,
                    },
                });
            }
        }
        if !commands.is_empty() {
            ed.apply(ui, Command::Batch { commands }, false);
        }
    };
    let set_value = move |name: &'static str, v: String| {
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: name.into(),
                value: Some(v),
            },
            false,
        );
    };
    // Champs du contenu de la liste qui entoure l'élément (valeur d'un bouton, étiquettes).
    let cols = collections(ed);
    let fields = move |many: bool| -> Vec<(String, String)> {
        let Some((col, prefix, _)) = list.clone() else {
            return Vec::new();
        };
        cols.with(|cs| {
            cs.iter()
                .find(|c| c.id == col)
                .map(|c| {
                    c.fields
                        .iter()
                        .filter(|f| f.list == many)
                        .map(|f| {
                            (
                                format!("{prefix}{}", f.name),
                                f.label.clone().unwrap_or_else(|| f.name.clone()),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
    };
    let inner = |v: &str| {
        v.trim()
            .strip_prefix("{{")
            .and_then(|x| x.strip_suffix("}}"))
            .map(|x| x.trim().to_string())
    };
    let detail = match role {
        "group" => {
            let param = value("data-revel-filters").unwrap_or_default();
            Some(view! {
                <label class="field" title=ui.t("filters-param-help")>
                    <span class="label">{ui.t("filters-param")}</span>
                    <input class="input input--mono" spellcheck="false" prop:value=param
                        on:change=move |e| set_value("data-revel-filters", event_target_value(&e).trim().to_string())/>
                </label>
            }.into_any())
        }
        "button" => {
            let v = value("data-revel-filter").unwrap_or_default();
            let bound = inner(&v);
            let written = if bound.is_some() {
                String::new()
            } else {
                v.clone()
            };
            let cur = bound.clone().unwrap_or_default();
            Some(view! {
                <label class="field">
                    <span class="label" title=ui.t("filters-value-help")>{ui.t("filters-value")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| {
                            let f = event_target_value(&e);
                            set_value("data-revel-filter", if f.is_empty() { String::new() } else { format!("{{{{ {f} }}}}") });
                        }>
                            <option value="" prop:selected=cur.is_empty()>{ui.t("filters-value-written")}</option>
                            {move || fields(false).into_iter().map(|(e, l)| {
                                let on = e == cur;
                                view! { <option value=e prop:selected=on>{l}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                </label>
                {bound.is_none().then(|| view! {
                    <input class="input" prop:value=written placeholder=ui.t("filters-value-all")
                        aria-label=ui.t("filters-value")
                        on:change=move |e| set_value("data-revel-filter", event_target_value(&e).trim().to_string())/>
                })}
            }.into_any())
        }
        "item" => {
            let v = value("data-revel-tags").unwrap_or_default();
            let cur = inner(&v)
                .and_then(|e| {
                    let (h, t) = e.split_once('|')?;
                    t.trim().starts_with("join").then(|| h.trim().to_string())
                })
                .unwrap_or_default();
            Some(view! {
                <label class="field">
                    <span class="label" title=ui.t("filters-tags-help")>{ui.t("filters-tags")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| {
                            let f = event_target_value(&e);
                            set_value("data-revel-tags", if f.is_empty() { String::new() } else { format!("{{{{ {f} | join(' ') }}}}") });
                        }>
                            <option value="" prop:selected=cur.is_empty()>"—"</option>
                            {move || fields(true).into_iter().map(|(e, l)| {
                                let on = e == cur;
                                view! { <option value=e prop:selected=on>{l}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                </label>
            }.into_any())
        }
        _ => None,
    };
    Some(
        view! {
            <div class="insp-group">
                <label class="field" title=ui.t("filters-role-help")>
                    <span class="label">{ui.t("filters-role")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| set_role(event_target_value(&e))>
                            <option value="" prop:selected=role.is_empty()>{ui.t("filters-role-none")}</option>
                            <option value="group" prop:selected={role == "group"}>{ui.t("filters-role-group")}</option>
                            <option value="button" prop:selected={role == "button"}>{ui.t("filters-role-button")}</option>
                            <option value="item" prop:selected={role == "item"}>{ui.t("filters-role-item")}</option>
                            <option value="empty" prop:selected={role == "empty"}>{ui.t("filters-role-empty")}</option>
                        </select>
                    </div>
                </label>
                {detail}
            </div>
        }
        .into_any(),
    )
}

// ------------------------------------------------------------------ consentement

/// Rôle d'un élément dans le consentement aux cookies : bandeau (affiché tant que le visiteur
/// n'a pas choisi ; identifiant Google Analytics 4 chargé après accord), bouton Accepter,
/// bouton Refuser, lien qui rouvre le bandeau.
fn consent_settings(ui: Ui, ed: Ed, at: &Target, tag: &str, attrs: &[Attr]) -> Option<AnyView> {
    let choice = attr(attrs, "data-revel-consent-choice").and_then(|a| a.value.clone());
    let role = if attr(attrs, "data-revel-consent").is_some() {
        "banner"
    } else if choice.as_deref() == Some("accept") {
        "accept"
    } else if choice.as_deref() == Some("refuse") {
        "refuse"
    } else if attr(attrs, "data-revel-consent-open").is_some() {
        "open"
    } else {
        ""
    };
    let has_banner = ed.doc.with_untracked(|d| {
        d.as_ref()
            .is_some_and(|d| site_has_attr(d, "data-revel-consent"))
    });
    // Sans rôle : proposé aux boutons et aux liens quand le site a un bandeau (le bandeau vient
    // de l'élément « Consentement » du panneau Ajouter).
    if role.is_empty() && !(has_banner && matches!(tag, "a" | "button")) {
        return None;
    }
    let ga = attr(attrs, "data-revel-consent")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let at = StoredValue::new(at.clone());
    let set_role = move |v: String| {
        let at = at.get_value();
        let mut commands = Vec::new();
        let current_attr = match role {
            "banner" => Some("data-revel-consent"),
            "accept" | "refuse" => Some("data-revel-consent-choice"),
            "open" => Some("data-revel-consent-open"),
            _ => None,
        };
        if let Some(n) = current_attr {
            commands.push(Command::RemoveAttr {
                at: at.clone(),
                name: n.into(),
            });
        }
        let (name, value) = match v.as_str() {
            "banner" => ("data-revel-consent", Some(String::new())),
            "accept" => ("data-revel-consent-choice", Some("accept".to_string())),
            "refuse" => ("data-revel-consent-choice", Some("refuse".to_string())),
            "open" => ("data-revel-consent-open", None),
            _ => ("", None),
        };
        if !name.is_empty() {
            commands.push(Command::SetAttr {
                at: at.clone(),
                name: name.into(),
                value,
            });
        }
        if !commands.is_empty() {
            ed.apply(ui, Command::Batch { commands }, false);
        }
    };
    view! {
        <div class="insp-group">
            <label class="field" title=ui.t("consent-role-help")>
                <span class="label">{ui.t("consent-role")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| set_role(event_target_value(&e))>
                        <option value="" prop:selected=role.is_empty()>{ui.t("consent-role-none")}</option>
                        <option value="banner" prop:selected={role == "banner"}>{ui.t("consent-role-banner")}</option>
                        <option value="accept" prop:selected={role == "accept"}>{ui.t("consent-role-accept")}</option>
                        <option value="refuse" prop:selected={role == "refuse"}>{ui.t("consent-role-refuse")}</option>
                        <option value="open" prop:selected={role == "open"}>{ui.t("consent-role-open")}</option>
                    </select>
                </div>
            </label>
            {(role == "banner").then(|| view! {
                <label class="field" title=ui.t("consent-ga-help")>
                    <span class="label">{ui.t("consent-ga")}</span>
                    <input class="input input--mono" spellcheck="false" placeholder="G-XXXXXXXXXX" prop:value=ga
                        on:change=move |e| {
                            ed.apply(ui, Command::SetAttr {
                                at: at.get_value(),
                                name: "data-revel-consent".into(),
                                value: Some(event_target_value(&e).trim().to_string()),
                            }, false);
                        }/>
                </label>
            })}
        </div>
    }
    .into_any()
    .into()
}

/// Le site joue des sons ou de la musique (actions, onglets).
fn site_has_sound(d: &Site) -> bool {
    let mut found = false;
    let mut look = |nodes: &[Node]| {
        revel_model::ids::walk(nodes, &mut |n| {
            if let Node::Element { attrs, .. } | Node::Use { attrs, .. } = n
                && attrs.iter().any(|a| {
                    let v = a.value.as_deref().unwrap_or_default();
                    (a.name == "data-revel-on" && (v.contains("sound:") || v.contains("music:")))
                        || (a.name == "data-revel-tabs" && v.contains("sound="))
                })
            {
                found = true;
            }
        });
    };
    for p in &d.pages {
        look(&p.body);
    }
    for l in &d.layouts {
        look(&l.body);
    }
    for c in &d.components {
        look(&c.body);
    }
    found
}

/// Interrupteur (`data-revel-switch` : musique ou sons, allumé par défaut, choix gardé par le
/// navigateur) et texte de son état (`data-revel-switch-text` : « On | Off »). Proposé aux
/// boutons quand le site joue des sons, et à ce qui est dans un interrupteur.
fn switch_settings(ui: Ui, ed: Ed, at: &Target, tag: &str, attrs: &[Attr]) -> Option<AnyView> {
    let channel = attr(attrs, "data-revel-switch").map(|a| a.value.clone().unwrap_or_default());
    let text = attr(attrs, "data-revel-switch-text").map(|a| a.value.clone().unwrap_or_default());
    let (sound, inside) = ed.doc.with_untracked(|d| {
        d.as_ref().map_or((false, false), |d| {
            (site_has_sound(d), inside_attr(d, at, "data-revel-switch"))
        })
    });
    let offer_switch = channel.is_some() || (sound && matches!(tag, "button" | "a"));
    let offer_text = text.is_some() || inside;
    if !offer_switch && !offer_text {
        return None;
    }
    let at = StoredValue::new(at.clone());
    let set = move |name: &'static str, v: Option<String>| {
        let at = at.get_value();
        ed.apply(
            ui,
            match v {
                Some(v) => Command::SetAttr {
                    at,
                    name: name.into(),
                    value: Some(v),
                },
                None => Command::RemoveAttr {
                    at,
                    name: name.into(),
                },
            },
            false,
        );
    };
    let cur = channel.clone().unwrap_or_default();
    let parts: Vec<String> = text
        .clone()
        .unwrap_or_else(|| "On|Off".into())
        .split('|')
        .map(str::to_string)
        .collect();
    let (on_text, off_text) = (
        parts.first().cloned().unwrap_or_default(),
        parts.get(1).cloned().unwrap_or_default(),
    );
    let texts = StoredValue::new((on_text.clone(), off_text.clone()));
    let write_text = move |on: Option<String>, off: Option<String>| {
        let (a, b) = texts.get_value();
        let v = format!("{}|{}", on.unwrap_or(a), off.unwrap_or(b));
        set("data-revel-switch-text", Some(v));
    };
    Some(
        view! {
            <div class="insp-group">
                {offer_switch.then(|| view! {
                    <label class="field" title=ui.t("switch-role-help")>
                        <span class="label">{ui.t("switch-role")}</span>
                        <div class="select">
                            <select class="input" on:change=move |e| {
                                let v = event_target_value(&e);
                                set("data-revel-switch", (!v.is_empty()).then_some(v));
                            }>
                                <option value="" prop:selected=cur.is_empty()>{ui.t("switch-none")}</option>
                                <option value="music" prop:selected={cur == "music"}>{ui.t("switch-music")}</option>
                                <option value="sounds" prop:selected={cur == "sounds"}>{ui.t("switch-sounds")}</option>
                            </select>
                        </div>
                    </label>
                })}
                {offer_text.then(|| match text.is_some() {
                    true => view! {
                        <label class="field">
                            <span class="label">{ui.t("switch-text-on")}</span>
                            <input class="input" prop:value=on_text.clone()
                                on:change=move |e| write_text(Some(event_target_value(&e)), None)/>
                        </label>
                        <label class="field">
                            <span class="label">{ui.t("switch-text-off")}</span>
                            <input class="input" prop:value=off_text.clone()
                                on:change=move |e| write_text(None, Some(event_target_value(&e)))/>
                        </label>
                        <button class="link-btn" on:click=move |_| set("data-revel-switch-text", None)>{ui.t("switch-text-remove")}</button>
                    }.into_any(),
                    false => view! {
                        <button class="link-btn" title=ui.t("switch-text-help")
                            on:click=move |_| set("data-revel-switch-text", Some("On|Off".into()))>{ui.t("switch-text-add")}</button>
                    }.into_any(),
                })}
            </div>
        }
        .into_any(),
    )
}

// ------------------------------------------------------------------ onglets

/// Un parent de `at` porte `data-revel-tabs`.
fn inside_tabs(d: &Site, at: &Target) -> bool {
    inside_attr(d, at, "data-revel-tabs")
}

/// Un parent de `at` porte l'attribut `name`.
fn inside_attr(d: &Site, at: &Target, name: &str) -> bool {
    let mut cur = at.clone();
    for _ in 0..64 {
        let Ok((parent, _)) = revel_edit::address::locate(d, &cur) else {
            return false;
        };
        let Some(id) = parent.node else {
            return false;
        };
        let t = Target {
            owner: at.owner.clone(),
            node: id,
        };
        if let Ok(Node::Element { attrs, .. }) = revel_edit::address::node(d, &t)
            && attr(attrs, name).is_some()
        {
            return true;
        }
        cur = t;
    }
    false
}

/// Onglets : changement au survol (sinon à l'appui), touches du clavier dans toute la page
/// (flèches, W/A/S/D), son joué à chaque changement.
fn tabs_options(ui: Ui, ed: Ed, at: Target, value: String) -> AnyView {
    let at = StoredValue::new(at);
    let opts = StoredValue::new(value.clone());
    let has = |k: &str| value.split_whitespace().any(|o| o == k);
    let (hover, keys) = (has("hover"), has("keys"));
    let sound = value
        .split_whitespace()
        .find_map(|o| o.strip_prefix("sound="))
        .map(|u| u.replace("%20", " "));
    // Réglage `key` (drapeau, ou `key=valeur`) remplacé ou retiré.
    let put = move |key: &'static str, v: Option<String>| {
        let mut list: Vec<String> = opts
            .get_value()
            .split_whitespace()
            .filter(|o| *o != key && !o.starts_with(&format!("{key}=")))
            .map(str::to_string)
            .collect();
        if let Some(v) = v {
            list.push(if v.is_empty() {
                key.to_string()
            } else {
                format!("{key}={}", revel_edit::actions::url_token(&v))
            });
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
    let flag = move |key: &'static str, title: &'static str, help: &'static str, on: bool| {
        view! {
            <label class="view-line" title=ui.t(help)>
                <span>{ui.t(title)}</span>
                <span class="switch">
                    <input type="checkbox" prop:checked=on
                        on:change=move |e| put(key, event_target_checked(&e).then(String::new))/>
                    <span class="switch-track"></span>
                </span>
            </label>
        }
    };
    // Nom du fichier, sans le volume (`~50`).
    let sound_name = sound.as_deref().map(|u| {
        let u = u.rsplit_once('~').map_or(u, |(a, _)| a);
        u.rsplit('/').next().unwrap_or(u).to_string()
    });
    view! {
        <div class="insp-group">
            <p class="panel-title">{ui.t("tabs-title")}</p>
            {flag("hover", "tabs-hover", "tabs-hover-help", hover)}
            {flag("keys", "tabs-keys", "tabs-keys-help", keys)}
            <div class="view-line" title=ui.t("tabs-sound-help")>
                <span>{ui.t("tabs-sound")}</span>
                <span class="act-sound-name">{sound_name.map(|n| view! { {icons::audio()}<span>{n}</span> })}</span>
                <button class="icon-btn icon-btn--small" title=ui.t("act-sound-add") aria-label=ui.t("act-sound-add")
                    on:click=move |_| crate::library::pick_audio(ui, crate::library::media_shelf(ed), move |url| put("sound", Some(url)))>
                    {icons::audio()}</button>
                {sound.is_some().then(|| view! {
                    <button class="icon-btn icon-btn--small" title=ui.t("act-sound-remove") aria-label=ui.t("act-sound-remove")
                        on:click=move |_| put("sound", None)>{icons::close()}</button>
                })}
            </div>
        </div>
    }
    .into_any()
}

/// Rôle d'un élément dans des onglets : onglet, panneau ou aucun.
fn tab_role(ui: Ui, ed: Ed, at: Target, role: &'static str) -> AnyView {
    let at = StoredValue::new(at);
    let set = move |v: String| {
        let at = at.get_value();
        let mut commands = Vec::new();
        for (name, key) in [("data-revel-tab", "tab"), ("data-revel-panel", "panel")] {
            if v == key && role != key {
                commands.push(Command::SetAttr {
                    at: at.clone(),
                    name: name.into(),
                    value: None,
                });
            } else if v != key && role == key {
                commands.push(Command::RemoveAttr {
                    at: at.clone(),
                    name: name.into(),
                });
            }
        }
        if commands.is_empty() {
            return;
        }
        ed.apply(ui, Command::Batch { commands }, false);
    };
    view! {
        <div class="insp-group">
            <label class="field" title=ui.t("tabs-role-help")>
                <span class="label">{ui.t("tabs-role")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| set(event_target_value(&e))>
                        <option value="" prop:selected=role.is_empty()>{ui.t("tabs-role-none")}</option>
                        <option value="tab" prop:selected={role == "tab"}>{ui.t("tabs-role-tab")}</option>
                        <option value="panel" prop:selected={role == "panel"}>{ui.t("tabs-role-panel")}</option>
                    </select>
                </div>
            </label>
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ carrousel, diaporama

/// Carrousel et diaporama : lecture automatique (secondes, vide : à la main) et boucle, écrits
/// dans `data-revel-slider` (`autoplay=5 loop`).
fn slider_options(ui: Ui, ed: Ed, at: Target, value: String) -> AnyView {
    let at = StoredValue::new(at);
    let opts: Vec<String> = value.split_whitespace().map(str::to_string).collect();
    let autoplay = opts
        .iter()
        .find_map(|o| o.strip_prefix("autoplay="))
        .unwrap_or_default()
        .to_string();
    let looped = opts.iter().any(|o| o == "loop");
    let write = move |autoplay: Option<String>, looped: Option<bool>| {
        let cur = ed.doc.with_untracked(|d| {
            d.as_ref()
                .and_then(|d| revel_edit::address::node(d, &at.get_value()).ok())
                .and_then(|n| match n {
                    Node::Element { attrs, .. } => {
                        attr(attrs, "data-revel-slider").and_then(|a| a.value.clone())
                    }
                    _ => None,
                })
                .unwrap_or_default()
        });
        let mut parts: Vec<String> = cur
            .split_whitespace()
            .filter(|o| {
                !(autoplay.is_some() && o.starts_with("autoplay="))
                    && !(looped.is_some() && *o == "loop")
            })
            .map(str::to_string)
            .collect();
        if let Some(a) = autoplay.filter(|a| a.parse::<f64>().is_ok_and(|n| n > 0.0)) {
            parts.push(format!("autoplay={a}"));
        }
        if looped == Some(true) {
            parts.push("loop".into());
        }
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: "data-revel-slider".into(),
                value: Some(parts.join(" ")),
            },
            false,
        );
    };
    view! {
        <div class="insp-group slider-settings">
            <p class="panel-title">{ui.t("slider-title")}</p>
            <div class="style-row">
                <span class="style-label" title=ui.t("slider-autoplay-help")><span class="style-label-text">{ui.t("slider-autoplay")}</span></span>
                <div class="style-control">
                    <div class="nbox has-units">
                        <input class="nbox-input" type="text" inputmode="decimal" spellcheck="false"
                            prop:value=autoplay aria-label=ui.t("slider-autoplay")
                            on:change=move |e| {
                                let v = event_target_value(&e).trim().replace(',', ".");
                                write(Some(v.trim_end_matches('s').trim().to_string()), None);
                            }/>
                        <span class="nbox-suffix" aria-hidden="true">"s"</span>
                    </div>
                </div>
            </div>
            <label class="view-line">
                <span>{ui.t("slider-loop")}</span>
                <span class="switch">
                    <input type="checkbox" prop:checked=looped on:change=move |e| write(None, Some(event_target_checked(&e)))/>
                    <span class="switch-track"></span>
                </span>
            </label>
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ animation Lottie

const LOTTIE_PLAY: &[(&str, &str)] = &[
    ("auto", "lottie-play-auto"),
    ("hover", "lottie-play-hover"),
    ("click", "lottie-play-click"),
    ("scroll", "lottie-play-scroll"),
];
const LOTTIE_SPEED: &[&str] = &["0.5", "0.75", "1", "1.5", "2"];

/// Animation Lottie (6.2) : fichier, lecture (à l'écran, au survol, à l'appui, au défilement),
/// boucle, vitesse.
fn lottie_options(ui: Ui, ed: Ed, at: Target, attrs: Vec<Attr>) -> AnyView {
    let value = |n: &str| attr(&attrs, n).and_then(|a| a.value.clone());
    let src = value("data-lottie").unwrap_or_default();
    let play = value("data-lottie-play").unwrap_or_else(|| "auto".into());
    let looped = attr(&attrs, "data-lottie-loop").is_some();
    let speed = value("data-lottie-speed").unwrap_or_else(|| "1".into());
    let at = StoredValue::new(at);
    let set = move |name: &'static str, v: Option<String>| {
        let at = at.get_value();
        let cmd = match v {
            Some(v) => Command::SetAttr {
                at,
                name: name.into(),
                value: Some(v),
            },
            None => Command::RemoveAttr {
                at,
                name: name.into(),
            },
        };
        ed.apply(ui, cmd, false);
    };
    let pick = move || {
        crate::library::pick_animation(ui, crate::library::media_shelf(ed), move |url| {
            set("data-lottie", Some(url))
        })
    };
    let file = src.rsplit('/').next().unwrap_or_default().to_string();
    view! {
        <div class="insp-group">
            <p class="panel-title">{ui.t("block-lottie")}</p>
            <div class="field">
                <span class="label">{ui.t("block-lottie-file")}</span>
                <span class="field-line">
                    <input class="input input--mono" readonly=true prop:value=file.clone()
                        placeholder=ui.t("block-lottie-none") title=src.clone() on:click=move |_| pick()/>
                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("block-lottie-choose")
                        aria-label=ui.t("block-lottie-choose") on:click=move |_| pick()>{crate::icons::sparkle()}</button>
                </span>
            </div>
            <label class="field">
                <span class="label">{ui.t("block-lottie-play")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| set("data-lottie-play", Some(event_target_value(&e)))>
                        {LOTTIE_PLAY.iter().map(|(v, k)| view! { <option value=*v selected=*v == play title=ui.tx(&format!("{k}-help"), &[])>{ui.t(k)}</option> }).collect_view()}
                    </select>
                </div>
            </label>
            <label class="field">
                <span class="label">{ui.t("block-lottie-speed")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| {
                        let v = event_target_value(&e);
                        set("data-lottie-speed", (v != "1").then_some(v));
                    }>
                        {LOTTIE_SPEED.iter().map(|v| view! { <option value=*v selected=*v == speed>{format!("{}×", v.replace('.', ","))}</option> }).collect_view()}
                    </select>
                </div>
            </label>
            <label class="view-line">
                <span>{ui.t("block-lottie-loop")}</span>
                <span class="switch">
                    <input type="checkbox" prop:checked=looped on:change=move |e| {
                        set("data-lottie-loop", event_target_checked(&e).then(String::new))
                    }/>
                    <span class="switch-track"></span>
                </span>
            </label>
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ liste de collection

/// Champs proposés pour filtrer ou trier (chemin dans le contenu, libellé).
fn fields_of(ui: Ui, c: Option<&CmsCollection>) -> Vec<(String, String)> {
    let mut v = vec![("date".to_string(), ui.now("block-list-date", &[]))];
    if let Some(c) = c {
        for f in &c.fields {
            if matches!(
                f.kind.as_deref(),
                Some("object" | "block" | "rich_text" | "code" | "image" | "file")
            ) {
                continue;
            }
            v.push((
                format!("data.{}", f.name),
                f.label.clone().unwrap_or_else(|| f.name.clone()),
            ));
        }
    }
    v
}

/// Champs à plusieurs valeurs du contenu qui entoure une liste (liste dans une liste) : nom,
/// libellé, collection désignée (référence).
fn list_fields(c: Option<&CmsCollection>) -> Vec<(String, String, Option<String>)> {
    c.map(|c| {
        c.fields
            .iter()
            .filter(|f| f.list && !matches!(f.kind.as_deref(), Some("object" | "block")))
            .map(|f| {
                let target = (f.kind.as_deref() == Some("reference"))
                    .then(|| {
                        f.options
                            .get("collection")
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                    })
                    .flatten();
                (
                    f.name.clone(),
                    f.label.clone().unwrap_or_else(|| f.name.clone()),
                    target,
                )
            })
            .collect()
    })
    .unwrap_or_default()
}

/// Source d'une liste : une collection, ou un champ du contenu qui l'entoure.
fn list_source(ui: Ui, ed: Ed, each_at: Target, expr: String) -> Option<AnyView> {
    let outer = ed
        .doc
        .with_untracked(|d| binding_context(d.as_ref()?, &each_at));
    let field = FieldList::parse(&expr);
    if outer.is_none() && field.is_none() {
        return None;
    }
    let cols = collections(ed);
    let at = StoredValue::new(each_at);
    let (outer_col, prefix) = outer.map(|(c, p, _)| (c, p)).unwrap_or_default();
    let outer_col = StoredValue::new(outer_col);
    let prefix = StoredValue::new(prefix);
    // Valeur du choix : `col:<id>` ou `field:<nom>`.
    let cur = match (&field, Query::parse(&expr)) {
        (Some(FieldList::Values { field }), _) | (Some(FieldList::Refs { field, .. }), _) => {
            let p = prefix.get_value();
            format!("field:{}", field.strip_prefix(p.as_str()).unwrap_or(field))
        }
        (None, Some(q)) => format!("col:{}", q.collection),
        _ => String::new(),
    };
    let choose = move |v: String| {
        let p = prefix.get_value();
        // Dans une liste : nom de l'élément différent de celui de la liste qui l'entoure.
        let mut item = None;
        let new = if let Some(id) = v.strip_prefix("col:") {
            Query {
                collection: id.to_string(),
                ..Default::default()
            }
            .to_expr()
        } else if let Some(name) = v.strip_prefix("field:") {
            let target = cols.with_untracked(|cs| {
                list_fields(cs.iter().find(|c| outer_col.with_value(|o| c.id == *o)))
                    .into_iter()
                    .find(|(n, _, _)| n == name)
                    .and_then(|(_, _, t)| t)
            });
            let nested = !p.is_empty();
            match target {
                Some(collection) => {
                    item = nested.then(|| "ref".to_string());
                    FieldList::Refs {
                        collection,
                        field: format!("{p}{name}"),
                    }
                    .to_expr()
                }
                None => {
                    item = nested.then(|| "valeur".to_string());
                    FieldList::Values {
                        field: format!("{p}{name}"),
                    }
                    .to_expr()
                }
            }
        } else {
            return;
        };
        ed.apply(
            ui,
            Command::SetLogic {
                at: at.get_value(),
                cond: None,
                expr: Some(new),
                item,
            },
            false,
        );
    };
    Some(
        view! {
            <label class="field">
                <span class="label">{ui.t("block-list-source")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| choose(event_target_value(&e))>
                        {move || {
                            let mut opts: Vec<(String, String)> = cols.with(|cs| {
                                let mut v: Vec<(String, String)> = list_fields(cs.iter().find(|c| outer_col.with_value(|o| c.id == *o)))
                                    .into_iter()
                                    .map(|(n, l, _)| (format!("field:{n}"), ui.now("block-list-of-field", &[("field", l)])))
                                    .collect();
                                v.extend(cs.iter().map(|c| (format!("col:{}", c.id), label_of(c))));
                                v
                            });
                            if !cur.is_empty() && !opts.iter().any(|(v, _)| *v == cur) {
                                let name = cur.split_once(':').map(|(_, n)| n.to_string()).unwrap_or_default();
                                opts.insert(0, (cur.clone(), name));
                            }
                            opts.into_iter().map(|(v, l)| {
                                let on = v == cur;
                                view! { <option value=v prop:selected=on>{l}</option> }
                            }).collect_view()
                        }}
                    </select>
                </div>
            </label>
        }
        .into_any(),
    )
}

/// Classes d'un élément relié : selon des conditions, ou tirée de la valeur d'un champ.
fn class_editors(
    ui: Ui,
    ed: Ed,
    at: StoredValue<Target>,
    node: &Node,
    fields: Signal<Vec<(String, String)>>,
) -> Option<AnyView> {
    let Node::Element { attrs, .. } = node else {
        return None;
    };
    let v = attr(attrs, "class")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    if let Some((base, prefix, field, default)) = revel_edit::style::value_class(&v) {
        return Some(value_class_editor(
            ui, ed, at, base, prefix, field, default, fields,
        ));
    }
    let (base, conds) = revel_edit::style::cond_classes(&v)?;
    Some(cond_classes_editor(ui, ed, at, base, conds, fields))
}

/// Classe tirée d'un champ : début écrit autour, préfixe, champ, valeur par défaut.
type ValueClass = (String, String, String, Option<String>);

/// Classe tirée de la valeur d'un champ (`block--{{ b.style or 'plein' }}`) : champ, début de
/// la classe, valeur quand le champ est vide.
#[allow(clippy::too_many_arguments)]
fn value_class_editor(
    ui: Ui,
    ed: Ed,
    at: StoredValue<Target>,
    base: String,
    prefix: String,
    field: String,
    default: Option<String>,
    fields: Signal<Vec<(String, String)>>,
) -> AnyView {
    let state = StoredValue::new((base, prefix.clone(), field.clone(), default.clone()));
    let write = move |f: Box<dyn Fn(&mut ValueClass)>| {
        let mut st = state.get_value();
        f(&mut st);
        state.set_value(st.clone());
        let (b, p, fi, d) = st;
        let value = if fi.is_empty() {
            b
        } else {
            revel_edit::style::value_class_expr(&b, &p, &fi, d.as_deref())
        };
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: "class".into(),
                value: Some(value),
            },
            false,
        );
    };
    let cur = field.clone();
    view! {
        <div class="field" data-bound="block-bound-value-class">
            <span class="label" title=ui.t("block-bound-value-class-help")>{ui.t("block-bound-value-class")}</span>
            <div class="select">
                <select class="input" on:change=move |e| {
                    let v = event_target_value(&e);
                    write(Box::new(move |s| s.2 = v.clone()));
                }>
                    <option value="">{ui.t("block-bound-value-class-none")}</option>
                    {move || {
                        let known = fields.with(|fs| fs.iter().any(|(e, _)| *e == cur));
                        let extra = (!known).then(|| view! { <option value=cur.clone() selected=true>{cur.clone()}</option> });
                        view! {
                            {extra}
                            {fields.with(|fs| fs.iter().map(|(e, l)| {
                                let on = *e == cur;
                                view! { <option value=e.clone() selected=on>{l.clone()}</option> }
                            }).collect_view())}
                        }
                    }}
                </select>
            </div>
            <div class="field-pair">
                <label class="field">
                    <span class="label">{ui.t("block-bound-value-class-prefix")}</span>
                    <input class="input input--mono" spellcheck="false" prop:value=prefix
                        on:change=move |e| { let v = event_target_value(&e).trim().to_string(); write(Box::new(move |s| s.1 = v.clone())) }/>
                </label>
                <label class="field">
                    <span class="label">{ui.t("block-bound-value-class-default")}</span>
                    <input class="input input--mono" spellcheck="false" prop:value=default.unwrap_or_default()
                        on:change=move |e| {
                            let v = event_target_value(&e).trim().to_string();
                            write(Box::new(move |s| s.3 = (!v.is_empty()).then(|| v.clone())))
                        }/>
                </label>
            </div>
        </div>
    }
    .into_any()
}

/// Classes ajoutées selon des conditions (`carte{% if item.data.une %} carte--une{% endif %}`) :
/// une ligne par classe, avec ses conditions (champ rempli, égal à…).
fn cond_classes_editor(
    ui: Ui,
    ed: Ed,
    at: StoredValue<Target>,
    base: String,
    conds: Vec<(String, String)>,
    fields: Signal<Vec<(String, String)>>,
) -> AnyView {
    let rows = StoredValue::new(conds.clone());
    let base = StoredValue::new(base);
    let write = move |list: Vec<(String, String)>| {
        let base = base.get_value();
        rows.set_value(list.clone());
        let mut value = base.clone();
        for (c, cls) in list
            .iter()
            .filter(|(c, cls)| !c.trim().is_empty() && !cls.trim().is_empty())
        {
            value.push_str(&format!("{{% if {c} %}} {}{{% endif %}}", cls.trim()));
        }
        let cmd = if value.trim().is_empty() {
            Command::RemoveAttr {
                at: at.get_value(),
                name: "class".into(),
            }
        } else {
            Command::SetAttr {
                at: at.get_value(),
                name: "class".into(),
                value: Some(value.trim_start().to_string()),
            }
        };
        ed.apply(ui, cmd, false);
    };
    let first_class = conds_base_first(&rows);
    let no_rows = rows.with_value(|r| r.is_empty());
    view! {
        <div class="field" data-bound="block-bound-class">
            <span class="label" title=ui.t("block-bound-class-help")>{ui.t("block-bound-class")}</span>
            {conds.into_iter().enumerate().map(|(i, (c, cls))| {
                let set_cond = move |nc: Option<String>| {
                    if let Some(nc) = nc {
                        let mut list = rows.get_value();
                        if let Some(r) = list.get_mut(i) { r.0 = nc; }
                        write(list);
                    }
                };
                view! {
                    <div class="cond-class">
                        <div class="field-line">
                            <input class="input input--mono" prop:value=cls spellcheck="false" aria-label=ui.t("block-bound-class-name")
                                on:change=move |e| {
                                    let v = event_target_value(&e).trim().trim_start_matches('.').to_string();
                                    let mut list = rows.get_value();
                                    if let Some(r) = list.get_mut(i) { r.1 = v; }
                                    write(list);
                                }/>
                            <button type="button" class="icon-btn icon-btn--small" title=ui.t("cond-class-remove") aria-label=ui.t("cond-class-remove")
                                on:click=move |_| {
                                    let mut list = rows.get_value();
                                    if i < list.len() { list.remove(i); }
                                    write(list);
                                }>{icons::close()}</button>
                        </div>
                        {crate::visibility::condition_editor(ui, fields, Some(c), false, set_cond)}
                    </div>
                }
            }).collect_view()}
            <button type="button" class="link-btn cond-add" on:click=move |_| {
                let Some(f) = fields.with_untracked(|fs| fs.first().map(|(e, _)| e.clone())) else { return };
                let mut list = rows.get_value();
                list.push((f, format!("{}--option", first_class.clone())));
                write(list);
            }>{ui.t("cond-class-add")}</button>
            {no_rows.then(|| view! {
                <button type="button" class="link-btn cond-add" on:click=move |_| {
                    let Some(f) = fields.with_untracked(|fs| fs.first().map(|(e, _)| e.clone())) else { return };
                    let b = base.get_value();
                    let first = b.split_whitespace().next().unwrap_or("classe").to_string();
                    ed.apply(ui, Command::SetAttr {
                        at: at.get_value(),
                        name: "class".into(),
                        value: Some(revel_edit::style::value_class_expr(&b, &format!("{first}--"), &f, None)),
                    }, false);
                }>{ui.t("block-bound-value-class-add")}</button>
            })}
        </div>
    }
    .into_any()
}

fn conds_base_first(rows: &StoredValue<Vec<(String, String)>>) -> String {
    rows.with_value(|r| {
        r.first()
            .and_then(|(_, c)| c.split("--").next().map(str::to_string))
    })
    .unwrap_or_else(|| "classe".into())
}

/// Contenus choisis à la main, dans l'ordre : monter, descendre, retirer, ajouter (titres des
/// contenus de la collection).
fn picks_editor(
    ui: Ui,
    entries: RwSignal<Vec<revel_protocol::PickEntry>>,
    list: Vec<String>,
    set: impl Fn(Vec<String>) + Copy + Send + Sync + 'static,
) -> AnyView {
    let picked = StoredValue::new(list.clone());
    let n = list.len();
    view! {
        <ul class="pick-list list-picks">
            {list.into_iter().enumerate().map(|(i, slug)| {
                let title = entries.with_untracked(|es| es.iter().find(|e| e.slug == slug).map(|e| e.title.clone()));
                let missing = title.is_none();
                let shift = move |d: isize| {
                    let mut all = picked.get_value();
                    let j = i as isize + d;
                    if j >= 0 && (j as usize) < all.len() {
                        all.swap(i, j as usize);
                        set(all);
                    }
                };
                let slug2 = slug.clone();
                view! {
                    <li class="pick-item" class:is-missing=missing title=slug.clone()>
                        <span class="pick-title">{move || entries.with(|es| es.iter().find(|e| e.slug == slug2).map(|e| e.title.clone())).unwrap_or_else(|| slug2.clone())}</span>
                        <button class="icon-btn icon-btn--small" title=ui.t("layers-up") aria-label=ui.t("layers-up")
                            disabled=i == 0 on:click=move |_| shift(-1)>{icons::arrow_up()}</button>
                        <button class="icon-btn icon-btn--small" title=ui.t("layers-down") aria-label=ui.t("layers-down")
                            disabled=i + 1 == n on:click=move |_| shift(1)>{icons::arrow_down()}</button>
                        <button class="icon-btn icon-btn--small" title=ui.t("action-remove") aria-label=ui.t("action-remove")
                            on:click=move |_| { let mut all = picked.get_value(); all.remove(i); set(all); }>{icons::close()}</button>
                    </li>
                }
            }).collect_view()}
        </ul>
        <div class="select pick-add">
            <select class="input" aria-label=ui.t("picks-add") on:change=move |e| {
                let v = event_target_value(&e);
                if !v.is_empty() {
                    let mut all = picked.get_value();
                    all.push(v);
                    set(all);
                }
            }>
                <option value="" selected=true>{ui.t("picks-add")}</option>
                {move || entries.with(|es| es.iter()
                    .filter(|e| !picked.with_value(|p| p.contains(&e.slug)))
                    .map(|e| view! { <option value=e.slug.clone()>{e.title.clone()}</option> })
                    .collect_view())}
            </select>
        </div>
    }
    .into_any()
}

fn collection_list(ui: Ui, ed: Ed, each_at: Target, expr: String) -> AnyView {
    let source = list_source(ui, ed, each_at.clone(), expr.clone());
    let Some(q) = Query::parse(&expr) else {
        // Liste tirée d'un champ : sa source seulement.
        if let Some(src) = source {
            return view! {
                <div class="insp-group collection-list">
                    <p class="panel-title">{ui.t("block-list")}</p>
                    {src}
                </div>
            }
            .into_any();
        }
        return view! {
            <div class="insp-group">
                <p class="panel-title">{ui.t("block-list")}</p>
                <p class="muted small" title=ui.t("block-list-computed")>{ui.t("block-list-computed-short")}</p>
            </div>
        }
        .into_any();
    };
    let nested = source.is_some();
    let cols = collections(ed);
    let at = StoredValue::new(each_at);

    let q = StoredValue::new(q);
    let write = move |f: Box<dyn Fn(&mut Query)>| {
        let mut nq = q.get_value();
        f(&mut nq);
        ed.apply(
            ui,
            Command::SetLogic {
                at: at.get_value(),
                cond: None,
                expr: Some(nq.to_expr()),
                item: None,
            },
            false,
        );
    };
    let current =
        move || cols.with(|cs| q.with_value(|q| cs.iter().find(|c| c.id == q.collection).cloned()));
    let fields = move || fields_of(ui, current().as_ref());
    let filter_field = q.with_value(|q| q.filter.as_ref().map(|f| f.0.clone()).unwrap_or_default());
    let filter_value = q.with_value(|q| {
        q.filter
            .as_ref()
            .and_then(|f| f.1.clone())
            .map(|v| revel_edit::canvas::literal(&v).unwrap_or(v))
            .unwrap_or_default()
    });
    let sort_field = q.with_value(|q| q.sort.as_ref().map(|s| s.0.clone()).unwrap_or_default());
    let desc = q.with_value(|q| q.sort.as_ref().is_some_and(|s| s.1));
    let limit = q.with_value(|q| q.limit.map(|n| n.to_string()).unwrap_or_default());
    let exclude = q.with_value(|q| q.exclude_current);
    // Sur la page d'un contenu (modèle de collection) : liste sans le contenu affiché.
    let on_template = exclude
        || ed.doc.with_untracked(|d| {
            d.as_ref()
                .and_then(|d| template_collection(d, &at.get_value()))
                .is_some()
        });
    let has_filter = !filter_field.is_empty();
    let has_sort = !sort_field.is_empty();
    let (ff, sf) = (filter_field.clone(), sort_field.clone());
    // Contenus choisis à la main (ordre affiché) : contenus de la collection proposés par titre.
    let picked = q.with_value(|q| q.pick.clone());
    let picking = RwSignal::new(!picked.is_empty());
    let entries = RwSignal::new(Vec::<revel_protocol::PickEntry>::new());
    {
        let col = q.with_value(|q| q.collection.clone());
        let path = ed.path.get_value();
        spawn_local(async move {
            if let Ok(d) = crate::api::readable_data(&path, &[], &[col]).await {
                let _ = entries.try_set(d.entries);
            }
        });
    }
    let set_pick = move |v: Vec<String>| write(Box::new(move |q| q.pick = v.clone()));
    let pick_view = move || {
        picking
            .get()
            .then(|| picks_editor(ui, entries, picked.clone(), set_pick))
    };
    // Contenus mis en premier : champ du contenu affiché (liste de contenus reliés) ou
    // contenus choisis, puis les autres.
    let first = q.with_value(|q| q.first.clone());
    let first_picks = first
        .as_deref()
        .and_then(revel_edit::collection::first_picks);
    let first_value = match (&first, &first_picks) {
        (None, _) => String::new(),
        (Some(_), Some(_)) => "pick".to_string(),
        (Some(f), None) => format!("field:{f}"),
    };
    let first_fields: Vec<(String, String)> = ed.doc.with_untracked(|d| {
        d.as_ref()
            .and_then(|d| template_collection(d, &at.get_value()))
            .map(|col| {
                cols.with_untracked(|cs| {
                    list_fields(cs.iter().find(|c| c.id == col))
                        .into_iter()
                        .map(|(n, l, _)| (n, l))
                        .collect()
                })
            })
            .unwrap_or_default()
    });
    let first_fields = StoredValue::new(first_fields);
    let set_first = move |v: Vec<String>| {
        write(Box::new(move |q| {
            q.first = Some(revel_edit::collection::first_expr(&v))
        }))
    };
    let first_view = first_picks.map(|list| picks_editor(ui, entries, list, set_first));
    // Filtre « contient » (champ liste, comme « Projets dont les expertises contiennent la page
    // affichée ») : sur la page d'un contenu, la page affichée ; sinon une valeur écrite.
    let contains = q.with_value(|q| q.contains.clone());
    let contains_field = contains.as_ref().map(|c| c.0.clone()).unwrap_or_default();
    let contains_value = contains.map(|c| c.1).unwrap_or_default();
    view! {
        <div class="insp-group collection-list">
            <p class="panel-title">{ui.t("block-list")}</p>
            {source}
            <label class="field" class:is-hidden=nested>
                <span class="label">{ui.t("block-list-collection")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| {
                        let v = event_target_value(&e);
                        write(Box::new(move |q| {
                            q.collection = v.clone();
                            q.filter = None;
                            q.sort = q.sort.take().filter(|s| s.0 == "date");
                        }))
                    }>
                        {move || {
                            let cur = q.with_value(|q| q.collection.clone());
                            let mut list: Vec<(String, String)> = cols.with(|cs| cs.iter().map(|c| (c.id.clone(), label_of(c))).collect());
                            if !list.iter().any(|(id, _)| *id == cur) {
                                list.insert(0, (cur.clone(), cur.clone()));
                            }
                            list.into_iter().map(|(id, l)| {
                                let on = id == cur;
                                view! { <option value=id prop:selected=on>{l}</option> }
                            }).collect_view()
                        }}
                    </select>
                </div>
            </label>
            <label class="field" class:is-hidden=nested>
                <span class="label">{ui.t("block-list-items")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| {
                        let on = event_target_value(&e) == "pick";
                        picking.set(on);
                        if !on {
                            set_pick(Vec::new());
                        }
                    }>
                        <option value="all" prop:selected=move || !picking.get()>{ui.t("block-list-items-all")}</option>
                        <option value="pick" prop:selected=move || picking.get()>{ui.t("block-list-items-pick")}</option>
                    </select>
                </div>
            </label>
            {pick_view}
            <label class="field" class:is-hidden=nested title=ui.t("block-list-first-help")>
                <span class="label">{ui.t("block-list-first")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| {
                        let v = event_target_value(&e);
                        write(Box::new(move |q| {
                            q.first = if v.is_empty() {
                                None
                            } else if v == "pick" {
                                Some("[]".to_string())
                            } else {
                                v.strip_prefix("field:").map(str::to_string)
                            };
                        }))
                    }>
                        <option value="" prop:selected=first_value.is_empty()>{ui.t("block-list-first-none")}</option>
                        <option value="pick" prop:selected={first_value == "pick"}>{ui.t("block-list-first-pick")}</option>
                        {first_fields.get_value().into_iter().map(|(n, l)| {
                            let v = format!("field:{n}");
                            let on = v == first_value;
                            view! { <option value=v prop:selected=on>{ui.tx("block-list-first-field", &[("field", l)])}</option> }
                        }).collect_view()}
                        {(first_value.starts_with("field:") && !first_fields.with_value(|f| f.iter().any(|(n, _)| format!("field:{n}") == first_value))).then(|| {
                            let v = first_value.clone();
                            view! { <option value=v.clone() prop:selected=true>{v[6..].to_string()}</option> }
                        })}
                    </select>
                </div>
            </label>
            {first_view}
            {(!first_value.is_empty()).then(|| {
                let full = q.with_value(|q| q.first_full);
                view! {
                    <label class="view-line" title=ui.t("block-list-first-full-help")>
                        <span>{ui.t("block-list-first-full")}</span>
                        <span class="switch">
                            <input type="checkbox" prop:checked=full on:change=move |e| {
                                let on = event_target_checked(&e);
                                write(Box::new(move |q| q.first_full = on))
                            }/>
                            <span class="switch-track"></span>
                        </span>
                    </label>
                }
            })}
            <label class="field">
                <span class="label">{ui.t("block-list-contains")}</span>
                <div class="select">
                    <select class="input" title=ui.t("block-list-contains-help") on:change=move |e| {
                        let v = event_target_value(&e);
                        write(Box::new(move |q| {
                            q.contains = (!v.is_empty()).then(|| (v.clone(), if on_template { "page.fileSlug".into() } else { "\"\"".into() }))
                        }))
                    }>
                        <option value="" prop:selected=contains_field.is_empty()>{ui.t("block-list-contains-none")}</option>
                        {move || fields().into_iter().filter(|(p, _)| p != "date").map(|(p, l)| {
                            let on = p == contains_field;
                            view! { <option value=p prop:selected=on>{l}</option> }
                        }).collect_view()}
                    </select>
                </div>
            </label>
            {(!contains_value.is_empty()).then(|| {
                let shown = if contains_value == "page.fileSlug" {
                    String::new()
                } else {
                    revel_edit::canvas::literal(&contains_value).unwrap_or(contains_value.clone())
                };
                view! {
                    <label class="field">
                        <span class="label">{ui.t("block-list-contains-value")}</span>
                        <input class="input" prop:value=shown placeholder=ui.t("block-list-contains-current")
                            on:change=move |e| {
                                let v = event_target_value(&e).trim().to_string();
                                write(Box::new(move |q| {
                                    if let Some(c) = q.contains.as_mut() {
                                        c.1 = if v.is_empty() { "page.fileSlug".into() } else { revel_edit::canvas::quote(&v) };
                                    }
                                }))
                            }/>
                    </label>
                }
            })}
            <label class="field">
                <span class="label">{ui.t("block-list-filter")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| {
                        let v = event_target_value(&e);
                        write(Box::new(move |q| q.filter = (!v.is_empty()).then(|| (v.clone(), None))))
                    }>
                        <option value="" prop:selected=!has_filter>{ui.t("block-list-all")}</option>
                        {move || fields().into_iter().filter(|(p, _)| p != "date").map(|(p, l)| {
                            let on = p == ff;
                            view! { <option value=p prop:selected=on>{l}</option> }
                        }).collect_view()}
                    </select>
                </div>
            </label>
            {has_filter.then(|| view! {
                <label class="field">
                    <span class="label">{ui.t("block-list-filter-value")}</span>
                    <input class="input" prop:value=filter_value.clone() placeholder=ui.t("block-list-filter-any")
                        on:change=move |e| {
                            let v = event_target_value(&e).trim().to_string();
                            write(Box::new(move |q| {
                                if let Some(f) = q.filter.as_mut() {
                                    f.1 = if v.is_empty() {
                                        None
                                    } else if matches!(v.as_str(), "true" | "false") || v.parse::<f64>().is_ok() {
                                        Some(v.clone())
                                    } else {
                                        Some(revel_edit::canvas::quote(&v))
                                    };
                                }
                            }))
                        }/>
                </label>
            })}
            <div class="field-pair">
                <label class="field">
                    <span class="label">{ui.t("block-list-sort")}</span>
                    <div class="select">
                        <select class="input" data-list="sort" on:change=move |e| {
                            let v = event_target_value(&e);
                            write(Box::new(move |q| {
                                let desc = q.sort.as_ref().map(|s| s.1).unwrap_or(v == "date");
                                q.sort = (!v.is_empty()).then(|| (v.clone(), desc));
                            }))
                        }>
                            <option value="" prop:selected=!has_sort>{ui.t("block-list-sort-default")}</option>
                            {move || fields().into_iter().map(|(p, l)| {
                                let on = p == sf;
                                view! { <option value=p prop:selected=on>{l}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                </label>
                <label class="field">
                    <span class="label">{ui.t("block-list-order")}</span>
                    <div class="select">
                        <select class="input" disabled=!has_sort on:change=move |e| {
                            let d = event_target_value(&e) == "desc";
                            write(Box::new(move |q| { if let Some(s) = q.sort.as_mut() { s.1 = d; } }))
                        }>
                            <option value="asc" prop:selected=!desc>{ui.t("block-list-asc")}</option>
                            <option value="desc" prop:selected=desc>{ui.t("block-list-desc")}</option>
                        </select>
                    </div>
                </label>
            </div>
            <label class="field">
                <span class="label">{ui.t("block-list-limit")}</span>
                <input class="input" type="number" min="1" step="1" prop:value=limit placeholder=ui.t("block-list-limit-all")
                    on:change=move |e| {
                        let n = event_target_value(&e).trim().parse::<usize>().ok().filter(|n| *n > 0);
                        write(Box::new(move |q| q.limit = n))
                    }/>
            </label>
            {on_template.then(|| view! {
                <label class="view-line" title=ui.t("block-list-exclude-help")>
                    <span>{ui.t("block-list-exclude")}</span>
                    <span class="switch">
                        <input type="checkbox" prop:checked=exclude on:change=move |e| {
                            let on = event_target_checked(&e);
                            write(Box::new(move |q| q.exclude_current = on))
                        }/>
                        <span class="switch-track"></span>
                    </span>
                </label>
            })}
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ éléments reliés au contenu

/// Boucle de collection la plus proche qui contient le nœud : sa cible, ses réglages, le nom de
/// sa variable d'élément.
pub(crate) fn enclosing_list(d: &Site, at: &Target) -> Option<(Query, String)> {
    let mut cur = at.clone();
    for _ in 0..64 {
        let (parent, _) = revel_edit::address::locate(d, &cur).ok()?;
        let id = parent.node?;
        let t = Target {
            owner: at.owner.clone(),
            node: id,
        };
        if let Ok(Node::Each { expr, item, .. }) = revel_edit::address::node(d, &t)
            && parent.branch == Branch::Children
        {
            // Liste de collection, ou contenus désignés par un champ (liste dans une liste).
            let q = Query::parse(expr).or_else(|| match FieldList::parse(expr)? {
                FieldList::Refs { collection, .. } => Some(Query {
                    collection,
                    ..Default::default()
                }),
                FieldList::Values { .. } => None,
            })?;
            let item = item.trim();
            return revel_model::valid_prop_name(item).then(|| (q, item.to_string()));
        }
        cur = t;
    }
    None
}

/// Liste des valeurs d'un champ (`item.data.tags`) la plus proche qui contient le nœud : nom de
/// sa variable (chaque valeur).
fn enclosing_values(d: &Site, at: &Target) -> Option<String> {
    enclosing_values_at(d, at).map(|(v, _, _)| v)
}

/// Champ du contenu relié à une propriété liste d'un composant, d'après ses utilisations dans
/// les pages et les gabarits : (collection, nom du champ). Sert à connaître les sous-champs
/// d'une liste parcourue dans le composant (blocs d'un projet).
pub(crate) fn list_prop_source(d: &Site, cid: &str, prop: &str) -> Option<(String, String)> {
    let mut found: Option<(String, String)> = None;
    let owners = d
        .layouts
        .iter()
        .map(|l| (revel_edit::Owner::Layout(l.id.clone()), &l.body))
        .chain(
            d.pages
                .iter()
                .map(|p| (revel_edit::Owner::Page(p.id.clone()), &p.body)),
        );
    for (owner, body) in owners {
        if found.is_some() {
            break;
        }
        revel_model::ids::walk(body, &mut |n| {
            if found.is_some() {
                return;
            }
            if let Node::Use {
                id: Some(id),
                component,
                args,
                ..
            } = n
                && component == cid
                && let Some(e) = args.get(prop)
            {
                let t = Target {
                    owner: owner.clone(),
                    node: id.clone(),
                };
                if let Some((col, prefix, _)) = binding_context(d, &t)
                    && let Some(f) = e.trim().strip_prefix(prefix.as_str())
                    && revel_model::valid_prop_name(f)
                {
                    found = Some((col, f.to_string()));
                }
            }
        });
    }
    found
}

/// Liste des valeurs d'un champ qui contient le nœud : variable, champ (`activities`,
/// `item.data.tags`) et boucle.
fn enclosing_values_at(d: &Site, at: &Target) -> Option<(String, String, Target)> {
    let mut cur = at.clone();
    for _ in 0..64 {
        let (parent, _) = revel_edit::address::locate(d, &cur).ok()?;
        let id = parent.node?;
        let t = Target {
            owner: at.owner.clone(),
            node: id,
        };
        if let Ok(Node::Each { expr, item, .. }) = revel_edit::address::node(d, &t)
            && parent.branch == Branch::Children
        {
            let item = item.trim().to_string();
            return match FieldList::parse(expr) {
                Some(FieldList::Values { field }) if revel_model::valid_prop_name(&item) => {
                    Some((item, field, t))
                }
                _ => None,
            };
        }
        cur = t;
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    Text,
    Src,
    Href,
    /// Image de fond d'un bloc (`style="background-image: url(…)"`).
    Bg,
}

/// Texte d'un élément relié à un champ : texte écrit avant et après, champ, écriture d'une date,
/// texte riche (`| safe`), expression non reconnue.
#[derive(Clone, Default, PartialEq)]
struct TextBind {
    before: String,
    after: String,
    field: String,
    /// Champ lu quand le premier est vide (`(a or b)`).
    fallback: Option<String>,
    format: Option<String>,
    safe: bool,
    /// Nombre de valeurs d'un champ liste (`x | length`).
    count: bool,
    /// Mots entre astérisques mis en valeur : classe (`x | emphasis("accent")`).
    emphasis: Option<String>,
    raw: Option<String>,
}

/// Lit le contenu d'un élément : vide, texte écrit, ou `[texte] {{ champ | filtre }} [texte]`.
fn text_bind(children: &[Node], field_of: &dyn Fn(&str) -> Option<String>) -> Option<TextBind> {
    let (before, out, after) = match children {
        [] | [Node::Text { .. }] => return Some(TextBind::default()),
        [Node::Out { expr }] => ("", expr, ""),
        [Node::Text { text }, Node::Out { expr }] => (text.as_str(), expr, ""),
        [Node::Out { expr }, Node::Text { text }] => ("", expr, text.as_str()),
        [
            Node::Text { text: a },
            Node::Out { expr },
            Node::Text { text: b },
        ] => (a.as_str(), expr, b.as_str()),
        _ => return None,
    };
    let mut b = TextBind {
        before: before.to_string(),
        after: after.to_string(),
        ..Default::default()
    };
    let mut b_fallback = None;
    let (head, tail) = match out.split_once('|') {
        Some((h, t)) => (h.trim(), Some(t.trim())),
        None => (out.trim(), None),
    };
    // Champ, sinon un autre : `(a or b)`.
    let inner = head
        .strip_prefix('(')
        .and_then(|h| h.strip_suffix(')'))
        .unwrap_or(head);
    let head = match inner
        .split(" or ")
        .map(str::trim)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [a, b] if field_of(a).is_some() && field_of(b).is_some() => {
            b_fallback = field_of(b);
            *a
        }
        _ => head,
    };
    b.fallback = b_fallback;
    match (field_of(head), tail) {
        (Some(f), None) => b.field = f,
        (Some(f), Some("safe")) => {
            b.field = f;
            b.safe = true;
        }
        (Some(f), Some("length")) => {
            b.field = f;
            b.count = true;
        }
        (Some(f), Some(t)) if t.starts_with("emphasis(") && t.ends_with(')') => {
            b.field = f;
            let arg = t["emphasis(".len()..t.len() - 1].trim();
            b.emphasis = Some(revel_edit::canvas::literal(arg).unwrap_or_else(|| "accent".into()));
        }
        (Some(f), Some(t)) if revel_edit::dates::format_of(t).is_some() => {
            b.field = f;
            b.format = revel_edit::dates::format_of(t);
        }
        _ => b.raw = Some(out.trim().to_string()),
    }
    Some(b)
}

/// Contenu d'un élément relié : texte avant, valeur du champ, texte après.
fn text_children(b: &TextBind, prefix: &str) -> Vec<Node> {
    let mut v = Vec::new();
    if !b.before.is_empty() {
        v.push(Node::Text {
            text: b.before.clone(),
        });
    }
    let mut expr = match &b.fallback {
        Some(fb) => format!("({prefix}{} or {prefix}{fb})", b.field),
        None => format!("{prefix}{}", b.field),
    };
    if b.count {
        expr.push_str(" | length");
    } else if let Some(c) = &b.emphasis {
        expr.push_str(&format!(" | emphasis({})", revel_edit::canvas::quote(c)));
    } else if let Some(f) = &b.format {
        expr.push_str(" | ");
        expr.push_str(f);
    } else if b.safe {
        expr.push_str(" | safe");
    }
    v.push(Node::Out { expr });
    if !b.after.is_empty() {
        v.push(Node::Text {
            text: b.after.clone(),
        });
    }
    v
}

/// Image de fond reliée : `background-image: url('{{ expr }}')` (seule déclaration du style).
fn bg_expr(style: &str) -> Option<String> {
    let v = style.trim().trim_end_matches(';').trim();
    let inner = v
        .strip_prefix("background-image:")?
        .trim()
        .strip_prefix("url(")?
        .strip_suffix(')')?
        .trim()
        .trim_matches(|c| c == '\'' || c == '"');
    let e = inner.strip_prefix("{{")?.strip_suffix("}}")?.trim();
    (!e.is_empty()).then(|| e.to_string())
}

/// Condition d'affichage d'un élément relié : enveloppe `If` qui ne contient que lui.
fn visibility_of(d: &Site, at: &Target) -> Option<(Target, String)> {
    let (parent, _) = revel_edit::address::locate(d, at).ok()?;
    let id = parent.node.clone()?;
    if parent.branch != Branch::Then {
        return None;
    }
    let t = Target {
        owner: at.owner.clone(),
        node: id,
    };
    match revel_edit::address::node(d, &t).ok()? {
        Node::If {
            cond,
            then,
            otherwise,
            ..
        } if then.len() == 1 && otherwise.is_empty() => Some((t, cond.clone())),
        _ => None,
    }
}

/// Contenu auquel un élément peut se relier : collection, début des expressions de ses champs,
/// expression de l'adresse de sa page. Dans une liste de collection : l'élément de la liste
/// (`item.data.client`, `item.url`) ; dans le modèle d'une collection (page de collection de
/// Webflow) : le contenu affiché (`client`, `page.url`).
fn binding_context(d: &Site, at: &Target) -> Option<(String, String, String)> {
    if let Some((q, item)) = enclosing_list(d, at) {
        return Some((q.collection, format!("{item}.data."), format!("{item}.url")));
    }
    template_collection(d, at).map(|c| (c, String::new(), "page.url".into()))
}

/// Collection dont le modèle de page est le gabarit propriétaire de `at`.
fn template_collection(d: &Site, at: &Target) -> Option<String> {
    let revel_edit::Owner::Layout(lid) = &at.owner else {
        return None;
    };
    d.collections
        .iter()
        .find(|c| {
            c.layout.as_deref().is_some_and(|t| {
                t == lid || lid.strip_prefix(t).is_some_and(|r| r.starts_with("--"))
            })
        })
        .map(|c| c.id.clone())
}

/// Texte, image ou lien d'un élément relié à un champ du contenu (« Get text from » de
/// Webflow) : dans une liste de collection ou dans le modèle des pages d'une collection. Date :
/// écriture choisie ; texte : écrit avant et après la valeur ; affichage selon un champ rempli
/// ou vide (« Conditional visibility »).
fn item_bindings(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    if let Some(var) = ed.doc.with_untracked(|d| enclosing_values(d.as_ref()?, at)) {
        let main = value_binding(ui, ed, at, node, var);
        let fields = condition_fields(ed, at).unwrap_or_else(|| Signal::derive(Vec::new));
        let classes = class_editors(ui, ed, StoredValue::new(at.clone()), node, fields);
        return match (main, classes) {
            (None, None) => None,
            (m, c) => Some(
                view! { {m} {c.map(|c| view! { <div class="insp-group">{c}</div> })} }.into_any(),
            ),
        };
    }
    let (collection_id, prefix, url_expr) = ed
        .doc
        .with_untracked(|d| binding_context(d.as_ref()?, at))?;
    let is_field = |f: &str| {
        !f.is_empty()
            && f.chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    };
    let pre = prefix.clone();
    let field_of = move |e: &str| {
        let e = e.trim();
        // Texte riche (HTML) : affiché tel quel (`| safe`).
        let e = e.strip_suffix("| safe").map(str::trim_end).unwrap_or(e);
        e.strip_prefix(pre.as_str())
            .filter(|f| is_field(f))
            .map(str::to_string)
    };
    let mut slots: Vec<(Slot, Option<String>)> = Vec::new();
    let mut text: Option<TextBind> = None;
    let mut link_affix: Option<(String, String)> = None;
    match node {
        Node::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            let is_link = tag.eq_ignore_ascii_case("a");
            if let Some(b) = text_bind(children, &field_of)
                && !(is_link && children.is_empty())
            {
                let cur = b.raw.clone().or_else(|| {
                    (!b.field.is_empty()).then(|| {
                        if b.count {
                            format!("count:{}", b.field)
                        } else {
                            b.field.clone()
                        }
                    })
                });
                slots.push((Slot::Text, cur));
                text = Some(b);
            }
            if is_link {
                let href = attr(attrs, "href")
                    .and_then(|a| a.value.clone())
                    .unwrap_or_default();
                // Adresse : `avant{{ champ }}après` (texte écrit autour du champ).
                let (inner, affix) = match (href.find("{{"), href.rfind("}}")) {
                    (Some(i), Some(j))
                        if i < j && !href[..i].contains('{') && !href[j + 2..].contains('{') =>
                    {
                        (
                            Some(href[i + 2..j].trim().to_string()),
                            (href[..i].to_string(), href[j + 2..].to_string()),
                        )
                    }
                    _ => (None, (String::new(), String::new())),
                };
                link_affix = Some(affix);
                let cur = match inner {
                    Some(e) if e == url_expr => Some("@url".to_string()),
                    Some(e) => Some(field_of(&e).unwrap_or(e)),
                    None => None,
                };
                slots.push((Slot::Href, cur));
            }
            // Bloc (pas un texte) : image de fond reliée.
            let style = attr(attrs, "style").and_then(|a| a.value.clone());
            let bound_bg = style.as_deref().and_then(bg_expr);
            if text.is_none() && (style.is_none() || bound_bg.is_some()) {
                slots.push((Slot::Bg, bound_bg.map(|e| field_of(&e).unwrap_or(e))));
            }
        }
        Node::Image(img) => {
            let cur = revel_edit::canvas::literal(&img.src)
                .is_none()
                .then(|| field_of(&img.src).unwrap_or_else(|| img.src.clone()));
            slots.push((Slot::Src, cur));
        }
        _ => {}
    }
    if slots.is_empty() {
        return None;
    }
    let cols = collections(ed);
    let at = StoredValue::new(at.clone());
    let prefix = StoredValue::new(prefix.clone());
    let url_expr = StoredValue::new(url_expr);
    let link_affix = StoredValue::new(link_affix.unwrap_or_default());
    let collection = StoredValue::new(collection_id);
    let text = StoredValue::new(text);
    let site_lang = ed.doc.with_untracked(|d| {
        d.as_ref()
            .map(|d| d.default_locale.clone())
            .unwrap_or_else(|| "fr".into())
    });
    // Lecture non suivie : appelée pendant la construction du panneau (une lecture suivie le
    // reconstruirait à l'arrivée des collections, avec une nouvelle liste vide).
    let field_kind = move |name: &str| -> Option<String> {
        cols.with_untracked(|cs| {
            cs.iter()
                .find(|c| collection.with_value(|id| c.id == *id))
                .and_then(|c| c.fields.iter().find(|f| f.name == name))
                .and_then(|f| f.kind.clone())
        })
    };
    let fields = move |slot: Slot| -> Vec<(String, String)> {
        cols.with(|cs| {
            cs.iter()
                .find(|c| collection.with_value(|id| c.id == *id))
                .map(|c| {
                    c.fields
                        .iter()
                        // Champ à plusieurs valeurs : pas un texte ni une image seule.
                        .filter(|f| !f.list)
                        .filter(|f| match slot {
                            Slot::Src | Slot::Bg => {
                                matches!(f.kind.as_deref(), Some("image" | "file") | None)
                            }
                            _ => !matches!(
                                f.kind.as_deref(),
                                Some("object" | "block" | "image" | "file" | "boolean")
                            ),
                        })
                        .map(|f| {
                            (
                                f.name.clone(),
                                f.label.clone().unwrap_or_else(|| f.name.clone()),
                            )
                        })
                        // Texte : aussi le nombre de valeurs d'un champ liste.
                        .chain(
                            c.fields
                                .iter()
                                .filter(|f| f.list && slot == Slot::Text)
                                .map(|f| {
                                    (
                                        format!("count:{}", f.name),
                                        ui.now(
                                            "block-bound-count",
                                            &[(
                                                "field",
                                                f.label.clone().unwrap_or_else(|| f.name.clone()),
                                            )],
                                        ),
                                    )
                                }),
                        )
                        .collect()
                })
                .unwrap_or_default()
        })
    };
    // Élément entier remplacé par le même avec un autre contenu (même identifiant).
    let set_children = move |kids: Vec<Node>| {
        let at = at.get_value();
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let Ok(Node::Element { id, tag, attrs, .. }) = revel_edit::address::node(d, &at) else {
                return None;
            };
            let (parent, index) = revel_edit::address::locate(d, &at).ok()?;
            Some(Command::Replace {
                parent,
                index,
                node: Node::Element {
                    id: id.clone(),
                    tag: tag.clone(),
                    attrs: attrs.clone(),
                    children: kids,
                },
            })
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let set_text = move |f: Box<dyn Fn(&mut TextBind)>| {
        let mut b = text.get_value().unwrap_or_default();
        f(&mut b);
        if b.field.is_empty() {
            set_children(vec![Node::Text {
                text: ui.now("block-bound-text", &[]),
            }]);
        } else {
            set_children(text_children(&b, &prefix.get_value()));
        }
    };
    let set = move |slot: Slot, v: String| {
        let at = at.get_value();
        let prefix = prefix.get_value();
        let expr = |f: &str| format!("{prefix}{f}");
        let cmd = match slot {
            Slot::Text => {
                let kind = field_kind(&v);
                let rich = matches!(
                    kind.as_deref(),
                    Some("rich-text" | "rich_text" | "markdown")
                );
                let date = matches!(kind.as_deref(), Some("date" | "datetime"));
                let (field, count) = match v.strip_prefix("count:") {
                    Some(f) => (f.to_string(), true),
                    None => (v.clone(), false),
                };
                set_text(Box::new(move |b| {
                    b.field = field.clone();
                    b.raw = None;
                    b.count = count;
                    b.safe = rich && !count;
                    b.format =
                        (date && !count).then(|| revel_edit::dates::FORMATS[0].0.to_string());
                }));
                return;
            }
            Slot::Href => {
                let (before, after) = link_affix.get_value();
                Command::SetAttr {
                    at: at.clone(),
                    name: "href".into(),
                    value: Some(match v.as_str() {
                        "" => "#".into(),
                        "@url" => format!("{{{{ {} }}}}", url_expr.get_value()),
                        f => format!("{before}{{{{ {} }}}}{after}", expr(f)),
                    }),
                }
            }
            Slot::Bg if v.is_empty() => Command::RemoveAttr {
                at: at.clone(),
                name: "style".into(),
            },
            Slot::Bg => Command::SetAttr {
                at: at.clone(),
                name: "style".into(),
                value: Some(format!("background-image: url('{{{{ {} }}}}')", expr(&v))),
            },
            Slot::Src => {
                let Some(Node::Image(mut img)) = ed.doc.with_untracked(|d| {
                    d.as_ref()
                        .and_then(|d| revel_edit::address::node(d, &at).ok().cloned())
                }) else {
                    return;
                };
                img.src = if v.is_empty() {
                    "\"\"".into()
                } else {
                    expr(&v)
                };
                Command::SetImage { at, image: img }
            }
        };
        ed.apply(ui, cmd, false);
    };
    // Affichage : toujours, ou selon un champ rempli / vide (enveloppe `If`).
    let vis = ed
        .doc
        .with_untracked(|d| d.as_ref().and_then(|d| visibility_of(d, &at.get_value())));
    let vis_cond = vis.as_ref().map(|(_, c)| c.clone());
    let vis_target = StoredValue::new(vis.map(|(t, _)| t));
    let set_vis = move |cond: Option<String>| {
        let at = at.get_value();
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            match (vis_target.get_value(), cond) {
                // Condition changée.
                (Some(t), Some(c)) => Some(Command::SetLogic {
                    at: t,
                    cond: Some(c),
                    expr: None,
                    item: None,
                }),
                // Toujours affiché : l'enveloppe retirée.
                (Some(t), None) => {
                    let node = revel_edit::address::node(d, &at).ok()?.clone();
                    let (parent, index) = revel_edit::address::locate(d, &t).ok()?;
                    Some(Command::Replace {
                        parent,
                        index,
                        node,
                    })
                }
                // Nouvelle condition : l'élément enveloppé.
                (None, Some(c)) => {
                    let node = revel_edit::address::node(d, &at).ok()?.clone();
                    let (parent, index) = revel_edit::address::locate(d, &at).ok()?;
                    let id = revel_model::ids::fresh(&d.node_ids());
                    Some(Command::Replace {
                        parent,
                        index,
                        node: Node::If {
                            id: Some(id),
                            cond: c,
                            then: vec![node],
                            otherwise: Vec::new(),
                        },
                    })
                }
                (None, None) => None,
            }
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let cond_fields =
        condition_fields(ed, &at.get_value()).unwrap_or_else(|| Signal::derive(Vec::new));
    let text_extra = text
        .with_value(|t| t.clone())
        .filter(|t| !t.field.is_empty() && t.raw.is_none());
    // Classe ajoutée quand un champ est rempli (`person__photo--swap` si « Photo au survol »),
    // ou tirée de la valeur d'un champ.
    let cond_class_view = class_editors(ui, ed, at, node, cond_fields);
    Some(
        view! {
            <div class="insp-group">
                <p class="panel-title">{ui.t("block-bound")}</p>
                {slots.into_iter().map(|(slot, cur)| {
                    let key = match slot {
                        Slot::Text => "block-bound-text-label",
                        Slot::Src => "block-bound-image",
                        Slot::Href => "block-bound-link",
                        Slot::Bg => "block-bound-bg",
                    };
                    let cur = cur.unwrap_or_default();
                    let cur2 = cur.clone();
                    let cur3 = cur.clone();
                    view! {
                        <label class="field" data-bound=key>
                            <span class="label">{ui.t(key)}</span>
                            <div class="select">
                                <select class="input" on:change=move |e| set(slot, event_target_value(&e))>
                                    <option value="" prop:selected=cur.is_empty()>{ui.t(if slot == Slot::Text { "block-bound-written" } else { "block-bound-none" })}</option>
                                    {(slot == Slot::Href).then(|| view! {
                                        <option value="@url" prop:selected={cur == "@url"}>{ui.t("block-bound-page")}</option>
                                    })}
                                    {move || {
                                        let list = fields(slot);
                                        let known = cur2.is_empty() || cur2 == "@url" || list.iter().any(|(n, _)| *n == cur2);
                                        let extra = (!known).then(|| {
                                            let c = cur2.clone();
                                            view! { <option value=c.clone() prop:selected=true>{c.clone()}</option> }
                                        });
                                        view! {
                                            {extra}
                                            {list.into_iter().map(|(n, l)| {
                                                let on = n == cur2;
                                                view! { <option value=n prop:selected=on>{l}</option> }
                                            }).collect_view()}
                                        }
                                    }}
                                </select>
                            </div>
                        </label>
                        {(slot == Slot::Href && !cur.is_empty() && cur != "@url").then(|| {
                            let (before, after) = link_affix.get_value();
                            let write_affix = move |b: Option<String>, a: Option<String>| {
                                let (b0, a0) = link_affix.get_value();
                                let (b1, a1) = (b.unwrap_or(b0), a.unwrap_or(a0));
                                link_affix.set_value((b1, a1));
                                set(Slot::Href, cur3.clone());
                            };
                            let w2 = write_affix.clone();
                            view! {
                                <div class="field-pair">
                                    <label class="field">
                                        <span class="label">{ui.t("block-bound-before")}</span>
                                        <input class="input input--mono" spellcheck="false" prop:value=before
                                            on:change=move |e| write_affix(Some(event_target_value(&e)), None)/>
                                    </label>
                                    <label class="field">
                                        <span class="label">{ui.t("block-bound-after")}</span>
                                        <input class="input input--mono" spellcheck="false" prop:value=after
                                            on:change=move |e| w2(None, Some(event_target_value(&e)))/>
                                    </label>
                                </div>
                            }
                        })}
                    }
                }).collect_view()}
                {text_extra.map(|b| {
                    let has_format = b.format.is_some();
                    let field = b.field.clone();
                    // Champ date : suivi (les collections arrivent après la construction).
                    let is_date = move || has_format || cols.with(|cs| {
                        cs.iter()
                            .find(|c| collection.with_value(|id| c.id == *id))
                            .and_then(|c| c.fields.iter().find(|f| f.name == field))
                            .is_some_and(|f| matches!(f.kind.as_deref(), Some("date" | "datetime")))
                    });
                    let cur_fmt = b.format.clone().unwrap_or_default();
                    let lang = site_lang.clone();
                    let fb = b.fallback.clone().unwrap_or_default();
                    let main = b.field.clone();
                    view! {
                        <label class="field" data-bound="block-bound-fallback">
                            <span class="label" title=ui.t("block-bound-fallback-help")>{ui.t("block-bound-fallback")}</span>
                            <div class="select">
                                <select class="input" on:change=move |e| {
                                    let v = event_target_value(&e);
                                    set_text(Box::new(move |b| b.fallback = (!v.is_empty()).then(|| v.clone())));
                                }>
                                    <option value="" prop:selected=fb.is_empty()>"—"</option>
                                    {move || fields(Slot::Text).into_iter().filter(|(n, _)| *n != main).map(|(n, l)| {
                                        let on = n == fb;
                                        view! { <option value=n prop:selected=on>{l}</option> }
                                    }).collect_view()}
                                </select>
                            </div>
                        </label>
                        {move || is_date().then(|| {
                            let cur_fmt = cur_fmt.clone();
                            let lang = lang.clone();
                            view! {
                            <label class="field" data-bound="block-bound-date">
                                <span class="label">{ui.t("block-bound-date")}</span>
                                <div class="select">
                                    <select class="input" on:change=move |e| {
                                        let v = event_target_value(&e);
                                        set_text(Box::new(move |b| b.format = (!v.is_empty()).then(|| v.clone())));
                                    }>
                                        <option value="" prop:selected=cur_fmt.is_empty()>{ui.t("block-bound-date-raw")}</option>
                                        {revel_edit::dates::FORMATS.iter().map(|(f, _)| {
                                            let on = *f == cur_fmt;
                                            view! { <option value=*f prop:selected=on>{revel_edit::dates::example(f, &lang)}</option> }
                                        }).collect_view()}
                                    </select>
                                </div>
                            </label>
                        }})}
                        // Mots entre *astérisques* mis en valeur (classe réglable).
                        {(b.format.is_none() && !b.safe).then(|| {
                            let on = b.emphasis.is_some();
                            let cls = b.emphasis.clone().unwrap_or_else(|| "accent".into());
                            view! {
                                <label class="view-line" title=ui.t("block-bound-emphasis-help")>
                                    <span>{ui.t("block-bound-emphasis")}</span>
                                    <span class="switch">
                                        <input type="checkbox" prop:checked=on on:change=move |e| {
                                            let on = event_target_checked(&e);
                                            set_text(Box::new(move |b| b.emphasis = on.then(|| "accent".to_string())));
                                        }/>
                                        <span class="switch-track"></span>
                                    </span>
                                </label>
                                {on.then(|| view! {
                                    <label class="field">
                                        <span class="label">{ui.t("block-bound-emphasis-class")}</span>
                                        <input class="input input--mono" prop:value=cls spellcheck="false" on:change=move |e| {
                                            let v = event_target_value(&e).trim().trim_start_matches('.').to_string();
                                            set_text(Box::new(move |b| b.emphasis = Some(if v.is_empty() { "accent".into() } else { v.clone() })));
                                        }/>
                                    </label>
                                })}
                            }
                        })}
                        <div class="field-pair">
                            <label class="field">
                                <span class="label">{ui.t("block-bound-before")}</span>
                                <input class="input" prop:value=b.before.clone() on:change=move |e| {
                                    let v = event_target_value(&e);
                                    set_text(Box::new(move |b| b.before = v.clone()));
                                }/>
                            </label>
                            <label class="field">
                                <span class="label">{ui.t("block-bound-after")}</span>
                                <input class="input" prop:value=b.after.clone() on:change=move |e| {
                                    let v = event_target_value(&e);
                                    set_text(Box::new(move |b| b.after = v.clone()));
                                }/>
                            </label>
                        </div>
                    }
                })}
                {cond_class_view}
                <div class="field" data-bound="block-bound-visibility">
                    <span class="label">{ui.t("block-bound-visibility")}</span>
                    {crate::visibility::condition_editor(ui, cond_fields, vis_cond, true, set_vis)}
                </div>
            </div>
        }
        .into_any(),
    )
}

/// Texte d'un élément dans une liste des valeurs d'un champ (`{% for tag in item.data.tags %}`) :
/// écrit, ou la valeur.
fn value_binding(ui: Ui, ed: Ed, at: &Target, node: &Node, var: String) -> Option<AnyView> {
    let Node::Element { children, .. } = node else {
        return None;
    };
    // Sous-champs d'une liste d'objets (« Activités » : titre, texte).
    let subs: Vec<(String, String, bool)> = ed
        .doc
        .with_untracked(|d| {
            let d = d.as_ref()?;
            let (_, field, each) = enclosing_values_at(d, at)?;
            match binding_context(d, &each) {
                Some((col, prefix, _)) => {
                    let name = field
                        .strip_prefix(prefix.as_str())
                        .unwrap_or(&field)
                        .to_string();
                    Some((col, name))
                }
                // Liste d'une propriété du composant : champ relié à ses utilisations.
                None => match &at.owner {
                    revel_edit::Owner::Component(cid) => list_prop_source(d, cid, &field),
                    _ => None,
                },
            }
        })
        .and_then(|(col, name)| {
            collections(ed).with_untracked(|cs| {
                let f = cs
                    .iter()
                    .find(|c| c.id == col)?
                    .fields
                    .iter()
                    .find(|f| f.name == name)?
                    .clone();
                Some(
                    f.fields
                        .iter()
                        .filter(|x| !x.list)
                        .map(|x| {
                            (
                                x.name.clone(),
                                x.label.clone().unwrap_or_else(|| x.name.clone()),
                                matches!(
                                    x.kind.as_deref(),
                                    Some("rich-text" | "rich_text" | "markdown")
                                ),
                            )
                        })
                        .collect(),
                )
            })
        })
        .unwrap_or_default();
    // Valeur reliée : la valeur, ou un sous-champ (`a.title`, `a.body | safe`).
    let cur: Option<String> = match children.as_slice() {
        [Node::Out { expr }] => {
            let e = expr.trim().trim_end_matches("| safe").trim();
            if e == var {
                Some(String::new())
            } else {
                e.strip_prefix(&format!("{var}.")).map(str::to_string)
            }
        }
        [] | [Node::Text { .. }] => None,
        _ => return None,
    };
    let bound = cur.as_deref() == Some("");
    let at = StoredValue::new(at.clone());
    let rich = StoredValue::new(subs.clone());
    let set = move |choice: String| {
        let at = at.get_value();
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let Ok(Node::Element { id, tag, attrs, .. }) = revel_edit::address::node(d, &at) else {
                return None;
            };
            let (parent, index) = revel_edit::address::locate(d, &at).ok()?;
            let kid = match choice.as_str() {
                "" => Node::Text {
                    text: ui.now("block-bound-text", &[]),
                },
                "value" => Node::Out { expr: var.clone() },
                sub => {
                    // Texte riche d'un objet : écrit tel quel (HTML).
                    let long = rich.with_value(|r| r.iter().any(|(n, _, rich)| n == sub && *rich));
                    Node::Out {
                        expr: if long {
                            format!("{var}.{sub} | safe")
                        } else {
                            format!("{var}.{sub}")
                        },
                    }
                }
            };
            Some(Command::Replace {
                parent,
                index,
                node: Node::Element {
                    id: id.clone(),
                    tag: tag.clone(),
                    attrs: attrs.clone(),
                    children: vec![kid],
                },
            })
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    Some(
        view! {
            <div class="insp-group">
                <p class="panel-title">{ui.t("block-bound")}</p>
                <label class="field" data-bound="block-bound-text-label">
                    <span class="label">{ui.t("block-bound-text-label")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| set(event_target_value(&e))>
                            <option value="" prop:selected=cur.is_none()>{ui.t("block-bound-written")}</option>
                            {subs.is_empty().then(|| view! {
                                <option value="value" prop:selected=bound>{ui.t("block-bound-value")}</option>
                            })}
                            {subs.iter().map(|(n, l, _)| {
                                let on = cur.as_deref() == Some(n.as_str());
                                view! { <option value=n.clone() prop:selected=on>{l.clone()}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                </label>
            </div>
        }
        .into_any(),
    )
}

/// Champs qu'une condition peut tester à l'endroit de `at` : valeur d'une liste tirée d'un
/// champ (et ses sous-champs), champs du contenu (liste, modèle de collection), propriétés du
/// composant ouvert. (expression, libellé).
pub fn condition_fields(ed: Ed, at: &Target) -> Option<Signal<Vec<(String, String)>>> {
    // Endroit : liste de valeurs (variable, collection, champ), contenu relié, propriétés.
    let (values, content, props) = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let values = enclosing_values_at(d, at).map(|(var, field, each)| {
            let col = binding_context(d, &each)
                .map(|(col, prefix, _)| {
                    let name = field
                        .strip_prefix(prefix.as_str())
                        .unwrap_or(&field)
                        .to_string();
                    (col, name)
                })
                .or_else(|| match &at.owner {
                    revel_edit::Owner::Component(cid) => list_prop_source(d, cid, &field),
                    _ => None,
                });
            (var, col)
        });
        let content = binding_context(d, at).map(|(col, prefix, _)| (col, prefix));
        let props: Vec<(String, String)> = match &at.owner {
            revel_edit::Owner::Component(cid) => d
                .components
                .iter()
                .find(|c| c.id == *cid)
                .map(|c| {
                    c.props
                        .iter()
                        .map(|p| {
                            (
                                p.name.clone(),
                                p.label.clone().unwrap_or_else(|| p.name.clone()),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        Some((values, content, props))
    })?;
    if values.is_none() && content.is_none() && props.is_empty() {
        return None;
    }
    let cols = collections(ed);
    let label = |f: &revel_protocol::CmsField| f.label.clone().unwrap_or_else(|| f.name.clone());
    Some(Signal::derive(move || {
        let mut out: Vec<(String, String)> = Vec::new();
        cols.with(|cs| {
            if let Some((var, col)) = &values {
                out.push((var.clone(), var.clone()));
                if let Some((col, name)) = col
                    && let Some(f) = cs
                        .iter()
                        .find(|c| c.id == *col)
                        .and_then(|c| c.fields.iter().find(|f| f.name == *name))
                {
                    for sub in &f.fields {
                        out.push((
                            format!("{var}.{}", sub.name),
                            format!("{var} › {}", label(sub)),
                        ));
                    }
                }
            }
            if let Some((col, prefix)) = &content
                && let Some(c) = cs.iter().find(|c| c.id == *col)
            {
                for f in &c.fields {
                    out.push((format!("{prefix}{}", f.name), label(f)));
                }
            }
        });
        out.extend(props.iter().cloned());
        out
    }))
}

/// Condition d'un bloc « Si » : clauses sur les champs de l'endroit (rempli, vide, égal à…) ;
/// l'expression reste modifiable dessous.
pub fn readable_condition(ui: Ui, ed: Ed, at: Target, cond: String) -> Option<AnyView> {
    let fields = condition_fields(ed, &at)?;
    let at = StoredValue::new(at);
    let set = move |c: Option<String>| {
        if let Some(c) = c {
            ed.apply(
                ui,
                Command::SetLogic {
                    at: at.get_value(),
                    cond: Some(c),
                    expr: None,
                    item: None,
                },
                false,
            );
        }
    };
    Some(
        view! {
            <div class="field" data-bound="block-bound-visibility">
                <span class="label">{ui.t("block-bound-visibility")}</span>
                {crate::visibility::condition_editor(ui, fields, Some(cond), false, set)}
            </div>
        }
        .into_any(),
    )
}
