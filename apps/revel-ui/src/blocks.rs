//! Réglages des éléments de 3.7 : icône (choix du dessin), code HTML, vidéo (lecture), liste de
//! collection (collection, filtre, tri, nombre, comme la « Collection List » de Webflow) et
//! éléments d'une liste reliés aux champs du contenu (texte, image, lien).

use crate::editor::Ed;
use crate::{Ui, api};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::collection::Query;
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
            <p class="panel-title">{ui.t("block-embed")}</p>
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
            <p class="muted small">{ui.t("block-embed-help")}</p>
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

fn collection_list(ui: Ui, ed: Ed, each_at: Target, expr: String) -> AnyView {
    let Some(q) = Query::parse(&expr) else {
        return view! {
            <div class="insp-group">
                <p class="panel-title">{ui.t("block-list")}</p>
                <p class="muted small">{ui.t("block-list-computed")}</p>
            </div>
        }
        .into_any();
    };
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
    let has_filter = !filter_field.is_empty();
    let has_sort = !sort_field.is_empty();
    let (ff, sf) = (filter_field.clone(), sort_field.clone());
    view! {
        <div class="insp-group collection-list">
            <p class="panel-title">{ui.t("block-list")}</p>
            <label class="field">
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
                        <select class="input" on:change=move |e| {
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
            let q = Query::parse(expr)?;
            let item = item.trim();
            return revel_model::valid_prop_name(item).then(|| (q, item.to_string()));
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
}

/// Contenu auquel un élément peut se relier : collection, début des expressions de ses champs,
/// expression de l'adresse de sa page. Dans une liste de collection : l'élément de la liste
/// (`item.data.client`, `item.url`) ; dans le modèle d'une collection (page de collection de
/// Webflow) : le contenu affiché (`client`, `page.url`).
fn binding_context(d: &Site, at: &Target) -> Option<(String, String, String)> {
    if let Some((q, item)) = enclosing_list(d, at) {
        return Some((q.collection, format!("{item}.data."), format!("{item}.url")));
    }
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
        .map(|c| (c.id.clone(), String::new(), "page.url".into()))
}

/// Texte, image ou lien d'un élément relié à un champ du contenu (« Get text from » de
/// Webflow) : dans une liste de collection ou dans le modèle des pages d'une collection.
fn item_bindings(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    let (collection_id, prefix, url_expr) =
        ed.doc.with_untracked(|d| binding_context(d.as_ref()?, at))?;
    let is_field = |f: &str| !f.is_empty() && f.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-');
    let field_of = |e: &str| {
        let e = e.trim();
        // Texte riche (HTML) : affiché tel quel (`| safe`).
        let e = e.strip_suffix("| safe").map(str::trim_end).unwrap_or(e);
        e.strip_prefix(prefix.as_str())
            .filter(|f| is_field(f))
            .map(str::to_string)
    };
    let mut slots: Vec<(Slot, Option<String>)> = Vec::new();
    match node {
        Node::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            match children.as_slice() {
                // Bloc vide (texte riche du contenu à y placer) ou texte écrit.
                [] if !tag.eq_ignore_ascii_case("a") => slots.push((Slot::Text, None)),
                [Node::Text { .. }] => slots.push((Slot::Text, None)),
                [Node::Out { expr }] => slots.push((
                    Slot::Text,
                    Some(field_of(expr).unwrap_or_else(|| expr.clone())),
                )),
                _ => {}
            }
            if tag.eq_ignore_ascii_case("a") {
                let href = attr(attrs, "href")
                    .and_then(|a| a.value.clone())
                    .unwrap_or_default();
                let inner = href
                    .trim()
                    .strip_prefix("{{")
                    .and_then(|t| t.strip_suffix("}}"))
                    .map(str::trim)
                    .map(str::to_string);
                let cur = match inner {
                    Some(e) if e == url_expr => Some("@url".to_string()),
                    Some(e) => Some(field_of(&e).unwrap_or(e)),
                    None => None,
                };
                slots.push((Slot::Href, cur));
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
    let collection = StoredValue::new(collection_id);
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
                            Slot::Src => matches!(f.kind.as_deref(), Some("image" | "file") | None),
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
                        .collect()
                })
                .unwrap_or_default()
        })
    };
    let set = move |slot: Slot, v: String| {
        let at = at.get_value();
        let prefix = prefix.get_value();
        let rich = cols.with_untracked(|cs| {
            cs.iter()
                .find(|c| collection.with_value(|id| c.id == *id))
                .and_then(|c| c.fields.iter().find(|f| f.name == v))
                .is_some_and(|f| matches!(f.kind.as_deref(), Some("rich-text" | "rich_text" | "markdown")))
        });
        let expr = |f: &str| {
            if rich && slot == Slot::Text {
                format!("{prefix}{f} | safe")
            } else {
                format!("{prefix}{f}")
            }
        };
        let cmd = match slot {
            Slot::Text => {
                let node = if v.is_empty() {
                    Node::Text {
                        text: ui.now("block-bound-text", &[]),
                    }
                } else {
                    Node::Out { expr: expr(&v) }
                };
                let empty = ed.doc.with_untracked(|d| {
                    d.as_ref()
                        .and_then(|d| revel_edit::address::node(d, &at).ok())
                        .is_some_and(|n| matches!(n, Node::Element { children, .. } if children.is_empty()))
                });
                let parent = Parent::of(&at, Branch::Children);
                if empty {
                    Command::Insert { parent, index: 0, node }
                } else {
                    Command::Replace { parent, index: 0, node }
                }
            }
            Slot::Href => Command::SetAttr {
                at: at.clone(),
                name: "href".into(),
                value: Some(match v.as_str() {
                    "" => "#".into(),
                    "@url" => format!("{{{{ {} }}}}", url_expr.get_value()),
                    f => format!("{{{{ {} }}}}", expr(f)),
                }),
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
    Some(
        view! {
            <div class="insp-group">
                <p class="panel-title">{ui.t("block-bound")}</p>
                {slots.into_iter().map(|(slot, cur)| {
                    let key = match slot {
                        Slot::Text => "block-bound-text-label",
                        Slot::Src => "block-bound-image",
                        Slot::Href => "block-bound-link",
                    };
                    let cur = cur.unwrap_or_default();
                    let cur2 = cur.clone();
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
                    }
                }).collect_view()}
            </div>
        }
        .into_any(),
    )
}
