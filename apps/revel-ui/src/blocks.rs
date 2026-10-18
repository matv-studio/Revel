//! Réglages des éléments de 3.7 : icône (choix du dessin), code HTML, vidéo (lecture), liste de
//! collection (collection, filtre, tri, nombre, comme la « Collection List » de Webflow) et
//! éléments d'une liste reliés aux champs du contenu (texte, image, lien).

use crate::editor::Ed;
use crate::{Ui, api};
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
        // Défilement (animation en boucle « ticker »), carrousel et diaporama.
        if attr(attrs, "data-revel-loop")
            .and_then(|a| a.value.as_deref())
            .is_some_and(|v| v.split_whitespace().next() == Some("ticker"))
        {
            let at = at.clone();
            parts.push(view! { <crate::interactions::TickerSettings at=at/> }.into_any());
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
                    Node::Element { attrs, .. } => attr(attrs, "data-revel-slider")
                        .and_then(|a| a.value.clone()),
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
            let item = item.trim();
            return matches!(FieldList::parse(expr), Some(FieldList::Values { .. }))
                .then(|| item.to_string())
                .filter(|i| revel_model::valid_prop_name(i));
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
    format: Option<String>,
    safe: bool,
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
    let (head, tail) = match out.split_once('|') {
        Some((h, t)) => (h.trim(), Some(t.trim())),
        None => (out.trim(), None),
    };
    match (field_of(head), tail) {
        (Some(f), None) => b.field = f,
        (Some(f), Some("safe")) => {
            b.field = f;
            b.safe = true;
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
    let mut expr = format!("{prefix}{}", b.field);
    if let Some(f) = &b.format {
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
        return value_binding(ui, ed, at, node, var);
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
                let cur = b
                    .raw
                    .clone()
                    .or_else(|| (!b.field.is_empty()).then(|| b.field.clone()));
                slots.push((Slot::Text, cur));
                text = Some(b);
            }
            if is_link {
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
                        .collect()
                })
                .unwrap_or_default()
        })
    };
    // Tous les champs (affichage selon un champ rempli).
    let all_fields = move || -> Vec<(String, String)> {
        cols.with(|cs| {
            cs.iter()
                .find(|c| collection.with_value(|id| c.id == *id))
                .map(|c| {
                    c.fields
                        .iter()
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
                set_text(Box::new(move |b| {
                    b.field = v.clone();
                    b.raw = None;
                    b.safe = rich;
                    b.format = date.then(|| revel_edit::dates::FORMATS[0].0.to_string());
                }));
                return;
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
    let vis_value = vis.as_ref().map(|(_, cond)| {
        let p = prefix.get_value();
        let c = cond.trim();
        match c.strip_prefix("not ") {
            Some(f) => f
                .trim()
                .strip_prefix(p.as_str())
                .filter(|f| is_field(f))
                .map(|f| format!("empty:{f}")),
            None => c
                .strip_prefix(p.as_str())
                .filter(|f| is_field(f))
                .map(|f| format!("set:{f}")),
        }
        .unwrap_or_else(|| format!("raw:{c}"))
    });
    let vis_target = StoredValue::new(vis.map(|(t, _)| t));
    let set_vis = move |v: String| {
        let at = at.get_value();
        let p = prefix.get_value();
        let cond = if let Some(f) = v.strip_prefix("set:") {
            Some(format!("{p}{f}"))
        } else {
            v.strip_prefix("empty:").map(|f| format!("not {p}{f}"))
        };
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
    let text_extra = text
        .with_value(|t| t.clone())
        .filter(|t| !t.field.is_empty() && t.raw.is_none());
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
                    view! {
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
                <label class="field" data-bound="block-bound-visibility">
                    <span class="label">{ui.t("block-bound-visibility")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| set_vis(event_target_value(&e))>
                            <option value="" prop:selected=vis_value.is_none()>{ui.t("block-bound-always")}</option>
                            {vis_value.clone().filter(|v| v.starts_with("raw:")).map(|v| {
                                let c = v[4..].to_string();
                                view! { <option value=v.clone() prop:selected=true>{c}</option> }
                            })}
                            {move || {
                                let cur = vis_value.clone().unwrap_or_default();
                                all_fields().into_iter().flat_map(|(n, l)| {
                                    let set_v = format!("set:{n}");
                                    let empty_v = format!("empty:{n}");
                                    let (on1, on2) = (cur == set_v, cur == empty_v);
                                    [
                                        view! { <option value=set_v prop:selected=on1>{ui.tx("block-bound-if-set", &[("field", l.clone())])}</option> },
                                        view! { <option value=empty_v prop:selected=on2>{ui.tx("block-bound-if-empty", &[("field", l.clone())])}</option> },
                                    ]
                                }).collect_view()
                            }}
                        </select>
                    </div>
                </label>
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
    let bound = match children.as_slice() {
        [Node::Out { expr }] => expr.trim() == var,
        [] | [Node::Text { .. }] => false,
        _ => return None,
    };
    let at = StoredValue::new(at.clone());
    let set = move |on: bool| {
        let at = at.get_value();
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let Ok(Node::Element { id, tag, attrs, .. }) = revel_edit::address::node(d, &at) else {
                return None;
            };
            let (parent, index) = revel_edit::address::locate(d, &at).ok()?;
            let kid = if on {
                Node::Out { expr: var.clone() }
            } else {
                Node::Text {
                    text: ui.now("block-bound-text", &[]),
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
                        <select class="input" on:change=move |e| set(event_target_value(&e) == "value")>
                            <option value="" prop:selected=!bound>{ui.t("block-bound-written")}</option>
                            <option value="value" prop:selected=bound>{ui.t("block-bound-value")}</option>
                        </select>
                    </div>
                </label>
            </div>
        }
        .into_any(),
    )
}

/// Condition d'affichage choisie dans une liste (« Si « Client » est rempli ») quand elle porte
/// sur un champ du contenu ; `None` hors d'une liste de collection ou d'un modèle.
pub fn readable_condition(ui: Ui, ed: Ed, at: Target, cond: String) -> Option<AnyView> {
    let (collection_id, prefix, _) = ed
        .doc
        .with_untracked(|d| binding_context(d.as_ref()?, &at))?;
    let is_field = |f: &str| {
        !f.is_empty()
            && f.chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    };
    let c = cond.trim();
    let cur = match c.strip_prefix("not ") {
        Some(f) => f
            .trim()
            .strip_prefix(prefix.as_str())
            .filter(|f| is_field(f))
            .map(|f| format!("empty:{f}")),
        None => c
            .strip_prefix(prefix.as_str())
            .filter(|f| is_field(f))
            .map(|f| format!("set:{f}")),
    };
    let cols = collections(ed);
    let collection = StoredValue::new(collection_id);
    let at = StoredValue::new(at);
    let set = move |v: String| {
        let cond = if let Some(f) = v.strip_prefix("set:") {
            format!("{prefix}{f}")
        } else if let Some(f) = v.strip_prefix("empty:") {
            format!("not {prefix}{f}")
        } else {
            return;
        };
        ed.apply(
            ui,
            Command::SetLogic {
                at: at.get_value(),
                cond: Some(cond),
                expr: None,
                item: None,
            },
            false,
        );
    };
    let raw = cur.is_none().then(|| c.to_string());
    let cur = cur.unwrap_or_default();
    Some(
        view! {
            <label class="field" data-bound="block-bound-visibility">
                <span class="label">{ui.t("block-bound-visibility")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| set(event_target_value(&e))>
                        {raw.map(|r| view! { <option value="" selected=true>{r}</option> })}
                        {move || {
                            let cur = cur.clone();
                            cols.with(|cs| {
                                cs.iter()
                                    .find(|c| collection.with_value(|id| c.id == *id))
                                    .map(|c| c.fields.iter().map(|f| (f.name.clone(), f.label.clone().unwrap_or_else(|| f.name.clone()))).collect::<Vec<_>>())
                                    .unwrap_or_default()
                            })
                            .into_iter()
                            .flat_map(|(n, l)| {
                                let (a, b) = (format!("set:{n}"), format!("empty:{n}"));
                                let (on1, on2) = (cur == a, cur == b);
                                [
                                    view! { <option value=a prop:selected=on1>{ui.tx("block-bound-if-set", &[("field", l.clone())])}</option> },
                                    view! { <option value=b prop:selected=on2>{ui.tx("block-bound-if-empty", &[("field", l.clone())])}</option> },
                                ]
                            })
                            .collect_view()
                        }}
                    </select>
                </div>
            </label>
        }
        .into_any(),
    )
}
