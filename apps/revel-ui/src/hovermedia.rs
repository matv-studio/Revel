//! Réglages du « Média au survol » ([`revel_edit::hover`]) : une vidéo ou une image qui montre
//! le média de l'élément survolé. Les deux côtés se règlent et se montrent l'un l'autre :
//! - le média : « Change avec l'élément survolé », éléments survolés (contenus d'une liste du
//!   CMS, ou éléments choisis un à un), champ qui donne la vidéo et l'image d'attente, liste des
//!   éléments qui le changent (chacun sélectionnable) ;
//! - l'élément survolé : média qu'il change (sélectionnable), son média (champ du contenu dans une
//!   liste, adresse sinon), retrait ;
//! - tout autre élément d'une page qui a un média au survol : le relier en un choix.

use crate::editor::{Ed, Selection};
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::collection::{FieldList, Query};
use revel_edit::hover::{self, DISPLAY, FOR, POSTER, SRC};
use revel_edit::{Command, Owner, Target};
use revel_model::{Attr, Node, Site};
use revel_protocol::CmsCollection;

fn attr<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a Attr> {
    attrs.iter().find(|a| a.name == name)
}

/// Nom d'un propriétaire (page, gabarit, composant) tel que les panneaux le montrent.
fn owner_name(d: &Site, o: &Owner) -> String {
    match o {
        Owner::Page(id) => d
            .pages
            .iter()
            .find(|p| &p.id == id)
            .map(|p| {
                p.name
                    .clone()
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| p.path.clone())
            })
            .unwrap_or_else(|| id.clone()),
        Owner::Layout(id) => id.rsplit('/').next().unwrap_or(id).to_string(),
        Owner::Component(id) => d
            .components
            .iter()
            .find(|c| &c.id == id)
            .map(crate::names::component_name)
            .unwrap_or_else(|| id.clone()),
    }
}

/// Nom d'un nœud (comme dans les calques), suivi de son propriétaire s'il n'est pas `here`.
pub(crate) fn label_of(ui: Ui, d: &Site, t: &Target, here: &Owner) -> String {
    let base = revel_edit::address::node(d, t)
        .ok()
        .map(|n| crate::editor::node_label(ui, n))
        .unwrap_or_default();
    if t.owner == *here {
        base
    } else {
        format!("{base} · {}", owner_name(d, &t.owner))
    }
}

/// Sélectionne un nœud et le montre dans le canevas.
fn select(ui: Ui, ed: Ed, t: &Target) {
    let found = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let n = revel_edit::address::node(d, t).ok()?;
        let tag = match n {
            Node::Element { tag, .. } => tag.clone(),
            Node::Image(_) => "img".into(),
            _ => String::new(),
        };
        Some((tag, crate::editor::node_label(ui, n)))
    });
    let Some((tag, label)) = found else { return };
    ed.selected.set(Some(Selection {
        id: t.node.clone(),
        tag,
        label,
    }));
    ed.select_and_reveal(&t.node);
}

/// Bouton « Sélectionner » d'un nœud.
pub(crate) fn select_btn(ui: Ui, ed: Ed, t: Target) -> AnyView {
    let t = StoredValue::new(t);
    view! {
        <button type="button" class="icon-btn icon-btn--small" title=ui.t("hover-select") aria-label=ui.t("hover-select")
            on:click=move |_| select(ui, ed, &t.get_value())>{icons::arrow_right()}</button>
    }
    .into_any()
}

/// Collection lue par une boucle (liste de collection, ou contenus désignés par un champ) :
/// identifiant tel que [`crate::blocks::collections`] le donne (`projets+actualites`).
fn each_collection(d: &Site, each: &Target) -> Option<String> {
    let Ok(Node::Each { expr, .. }) = revel_edit::address::node(d, each) else {
        return None;
    };
    Query::parse(expr)
        .map(|q| q.key())
        .or_else(|| match FieldList::parse(expr)? {
            FieldList::Refs { collection, .. } => Some(collection),
            FieldList::Values { .. } => None,
        })
}

/// Champs d'une collection qui peuvent donner le média : vidéo (adresse, fichier) ou image.
fn media_fields(c: Option<&CmsCollection>, image: bool) -> Vec<(String, String)> {
    c.map(|c| {
        c.fields
            .iter()
            .filter(|f| !f.list)
            .filter(|f| {
                if image {
                    matches!(f.kind.as_deref(), Some("image" | "file"))
                } else {
                    matches!(f.kind.as_deref(), None | Some("string" | "text" | "file"))
                }
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
}

/// Champ proposé d'office : celui dont le nom ou le libellé contient un des mots, sinon le premier.
fn guess(fields: &[(String, String)], words: &[&str]) -> Option<String> {
    fields
        .iter()
        .find(|(n, l)| {
            let (n, l) = (n.to_lowercase(), l.to_lowercase());
            words.iter().any(|w| n.contains(w) || l.contains(w))
        })
        .or_else(|| fields.first())
        .map(|(n, _)| n.clone())
}

const VIDEO_WORDS: &[&str] = &["video", "vidéo", "film", "clip"];
const POSTER_WORDS: &[&str] = &[
    "cover",
    "poster",
    "couverture",
    "vignette",
    "miniature",
    "image",
];

/// Listes de collection du propriétaire `owner` : (adresse de la boucle, nom affiché).
fn lists_of(ui: Ui, d: &Site, owner: &Owner) -> Vec<(Target, String)> {
    let mut out = Vec::new();
    for (kind, id, body) in d.owners() {
        let o = match kind {
            revel_model::ids::OwnerKind::Page => Owner::Page(id.to_string()),
            revel_model::ids::OwnerKind::Layout => Owner::Layout(id.to_string()),
            revel_model::ids::OwnerKind::Component => Owner::Component(id.to_string()),
        };
        if o != *owner {
            continue;
        }
        revel_model::ids::walk(body, &mut |n| {
            if let Node::Each {
                id: Some(nid),
                expr,
                ..
            } = n
                && (Query::parse(expr).is_some()
                    || matches!(FieldList::parse(expr), Some(FieldList::Refs { .. })))
            {
                let t = Target {
                    owner: o.clone(),
                    node: nid.clone(),
                };
                // Nom de la liste : l'élément qui la contient (« Hero list »).
                let name = revel_edit::address::locate(d, &t)
                    .ok()
                    .and_then(|(p, _)| p.node)
                    .and_then(|pid| {
                        revel_edit::address::node(
                            d,
                            &Target {
                                owner: o.clone(),
                                node: pid,
                            },
                        )
                        .ok()
                        .map(|n| crate::editor::node_label(ui, n))
                    })
                    .unwrap_or_else(|| ui.now("block-list", &[]));
                // Collection lue (« Projets ») : deux listes du même nom se distinguent.
                let key = each_collection(d, &t).unwrap_or_default();
                out.push((t, format!("{name}\u{1}{key}")));
            }
        });
    }
    out
}

/// Nom d'une liste proposée : « Grid 3 — Projets ».
fn list_label(raw: &str, cols: &[CmsCollection]) -> String {
    let (name, key) = raw.split_once('\u{1}').unwrap_or((raw, ""));
    let label = key
        .split('+')
        .map(|k| {
            cols.iter()
                .find(|c| c.id == k)
                .and_then(|c| c.label.clone())
                .unwrap_or_else(|| k.to_string())
        })
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" + ");
    if label.is_empty() {
        name.to_string()
    } else {
        format!("{name} — {label}")
    }
}

/// Choix des champs du contenu qui donnent le média (et l'image d'attente d'une vidéo), pour
/// l'élément survolé `trigger` d'une liste dont l'élément s'appelle `item`.
#[allow(clippy::too_many_arguments)]
fn field_selects(
    ui: Ui,
    ed: Ed,
    display: Target,
    trigger: Target,
    item: String,
    collection: String,
    image: bool,
    cur_src: Option<String>,
    cur_poster: Option<String>,
) -> AnyView {
    let cols = crate::blocks::collections(ed);
    let display = StoredValue::new(display);
    let trigger = StoredValue::new(trigger);
    let item = StoredValue::new(item);
    let collection = StoredValue::new(collection);
    let write = move |src: Option<String>, poster: Option<String>| {
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let it = item.get_value();
            let src = src.map(|f| hover::field_value(&it, &f));
            let poster = poster.map(|f| {
                if f.is_empty() {
                    String::new()
                } else {
                    hover::field_value(&it, &f)
                }
            });
            hover::connect(
                d,
                &display.get_value(),
                &trigger.get_value(),
                src.as_deref(),
                poster.as_deref(),
                false,
            )
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let of = move |image: bool| {
        cols.with(|cs| {
            media_fields(
                cs.iter().find(|c| collection.with_value(|id| c.id == *id)),
                image,
            )
        })
    };
    let src_now = cur_src.clone().unwrap_or_default();
    let poster_now = cur_poster.clone().unwrap_or_default();
    view! {
        <label class="field" data-hover="src-field" title=ui.t(if image { "hover-image-field-help" } else { "hover-video-field-help" })>
            <span class="label">{ui.t(if image { "hover-image-field" } else { "hover-video-field" })}</span>
            <div class="select">
                <select class="input" on:change=move |e| {
                    let v = event_target_value(&e);
                    if !v.is_empty() { write(Some(v), None); }
                }>
                    {move || {
                        let mut opts = of(image);
                        if !src_now.is_empty() && !opts.iter().any(|(n, _)| *n == src_now) {
                            opts.insert(0, (src_now.clone(), src_now.clone()));
                        }
                        let none = src_now.is_empty().then(|| view! { <option value="" selected=true>{ui.t("hover-field-choose")}</option> });
                        view! {
                            {none}
                            {opts.into_iter().map(|(n, l)| {
                                let on = n == src_now;
                                view! { <option value=n prop:selected=on>{l}</option> }
                            }).collect_view()}
                        }
                    }}
                </select>
            </div>
        </label>
        {(!image).then(|| view! {
            <label class="field" data-hover="poster-field" title=ui.t("hover-poster-help")>
                <span class="label">{ui.t("hover-poster-field")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| write(None, Some(event_target_value(&e)))>
                        <option value="" prop:selected=poster_now.is_empty()>{ui.t("hover-none")}</option>
                        {move || {
                            let pn = cur_poster.clone().unwrap_or_default();
                            of(true).into_iter().map(|(n, l)| {
                                let on = n == pn;
                                view! { <option value=n prop:selected=on>{l}</option> }
                            }).collect_view()
                        }}
                    </select>
                </div>
            </label>
        })}
    }
    .into_any()
}

/// Adresse du média d'un élément survolé hors liste (vidéo ou image, image d'attente).
fn address_fields(ui: Ui, ed: Ed, trigger: Target, attrs: &[Attr], image: bool) -> AnyView {
    let src = attr(attrs, SRC)
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let poster = attr(attrs, POSTER)
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let at = StoredValue::new(trigger);
    let set = move |name: &'static str, v: String| {
        let at = at.get_value();
        let cmd = if v.trim().is_empty() && name == POSTER {
            Command::RemoveAttr {
                at,
                name: name.into(),
            }
        } else {
            Command::SetAttr {
                at,
                name: name.into(),
                value: Some(v.trim().to_string()),
            }
        };
        ed.apply(ui, cmd, false);
    };
    let pick = move |name: &'static str, images: bool| {
        crate::library::pick_media(ui, images, crate::library::media_shelf(ed), move |url| {
            set(name, url)
        });
    };
    let (src_key, src_pick) = if image {
        ("hover-image-src", "picker-choose-image")
    } else {
        ("hover-video-src", "picker-choose-media")
    };
    view! {
        <crate::inspector::Field label=ui.t(src_key) value=src on_commit=move |v| set(SRC, v)>
            <button type="button" class="icon-btn icon-btn--small" title=ui.t(src_pick) aria-label=ui.t(src_pick)
                on:click=move |_| pick(SRC, image)>{if image { icons::image() } else { icons::video() }}</button>
        </crate::inspector::Field>
        {(!image).then(|| view! {
            <crate::inspector::Field label=ui.t("hover-poster-src") value=poster on_commit=move |v| set(POSTER, v)>
                <button type="button" class="icon-btn icon-btn--small" title=ui.t("picker-choose-image") aria-label=ui.t("picker-choose-image")
                    on:click=move |_| pick(POSTER, true)>{icons::image()}</button>
            </crate::inspector::Field>
        })}
    }
    .into_any()
}

/// Réglages du média au survol : vidéo ou image (`data-revel-hover-media`).
pub fn display_settings(ui: Ui, ed: Ed, at: &Target, tag: &str, attrs: &[Attr]) -> Option<AnyView> {
    if !matches!(tag, "video" | "img") {
        return None;
    }
    let image = tag == "img";
    let name = attr(attrs, DISPLAY).map(|a| a.value.clone().unwrap_or_default());
    let on = name.is_some();
    let name = name.unwrap_or_default();
    let display = StoredValue::new(at.clone());
    let toggle = move |want: bool| {
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let at = display.get_value();
            if want {
                Some(Command::SetAttr {
                    at: at.clone(),
                    name: DISPLAY.into(),
                    value: Some(hover::name_for(d, &at)),
                })
            } else {
                hover::disconnect(d, &at)
            }
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let body = on.then(|| {
        ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let here = at.owner.clone();
            let triggers = hover::triggers(d, &name);
            let driving = hover::driving_list(d, &name);
            let lists = lists_of(ui, d, &here);
            let mode = RwSignal::new(match (&driving, triggers.is_empty()) {
                (Some((each, _)), _) => format!("list:{}", each.node),
                (None, false) => "manual".to_string(),
                (None, true) => String::new(),
            });
            let lists_store = StoredValue::new(lists.clone());
            let cols_sig = crate::blocks::collections(ed);
            let choose = move |v: String| {
                mode.set(v.clone());
                let cmd = ed.doc.with_untracked(|d| {
                    let d = d.as_ref()?;
                    let disp = display.get_value();
                    let name = hover::attr_of(d, &disp, DISPLAY).unwrap_or_default();
                    if let Some(id) = v.strip_prefix("list:") {
                        let each = Target {
                            owner: disp.owner.clone(),
                            node: id.to_string(),
                        };
                        let key = each_collection(d, &each)?;
                        // Champs des collections (relues à l'ouverture du panneau) : choix
                        // d'office par nom.
                        let fields_now = cols_sig.with_untracked(|cs| {
                            let c = cs.iter().find(|c| c.id == key);
                            (media_fields(c, image), media_fields(c, true))
                        });
                        let src = guess(&fields_now.0, VIDEO_WORDS)
                            .unwrap_or_else(|| if image { "cover" } else { "video" }.into());
                        let poster = (!image)
                            .then(|| guess(&fields_now.1, POSTER_WORDS))
                            .flatten();
                        hover::connect_list(d, &disp, &each, &src, poster.as_deref())
                    } else {
                        // Aucun ou un à un : plus aucun élément ne le change (les éléments
                        // choisis se relient depuis leurs propres réglages).
                        let mut out = Vec::new();
                        for t in hover::triggers(d, &name) {
                            hover::unlink_trigger(d, &t, &mut out);
                        }
                        (!out.is_empty()).then_some(Command::Batch { commands: out })
                    }
                });
                if let Some(cmd) = cmd {
                    ed.apply(ui, cmd, false);
                }
            };
            // Champs de la liste qui le pilote (élément survolé de son modèle).
            let fields_view = driving.as_ref().and_then(|(each, item)| {
                let key = each_collection(d, each)?;
                let trig = triggers
                    .iter()
                    .find(|t| hover::enclosing_each(d, t).is_some_and(|(e, _)| e == *each))?
                    .clone();
                let cur_src = hover::attr_of(d, &trig, SRC).and_then(|v| hover::field_of(&v, item));
                let cur_poster =
                    hover::attr_of(d, &trig, POSTER).and_then(|v| hover::field_of(&v, item));
                Some(field_selects(
                    ui,
                    ed,
                    display.get_value(),
                    trig,
                    item.clone(),
                    key,
                    image,
                    cur_src,
                    cur_poster,
                ))
            });
            // Éléments qui le changent, chacun sélectionnable.
            let rows: Vec<(Target, String)> = triggers
                .iter()
                .map(|t| {
                    let base = label_of(ui, d, t, &here);
                    let l = match hover::enclosing_each(d, t) {
                        Some((each, _)) => {
                            let list = lists
                                .iter()
                                .find(|(e, _)| *e == each)
                                .map(|(_, n)| n.split('\u{1}').next().unwrap_or_default().to_string())
                                .unwrap_or_default();
                            ui.now("hover-item-in-list", &[("item", base), ("list", list)])
                        }
                        None => base,
                    };
                    (t.clone(), l)
                })
                .collect();
            let count = rows.len();
            Some(view! {
                <label class="field" data-hover="source" title=ui.t("hover-source-help")>
                    <span class="label">{ui.t("hover-source")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| choose(event_target_value(&e))>
                            <option value="" prop:selected=move || mode.get().is_empty()>{ui.t("hover-none")}</option>
                            {move || {
                                let cs = cols_sig;
                                lists_store.get_value().into_iter().map(|(t, n)| {
                                    let v = format!("list:{}", t.node);
                                    let v2 = v.clone();
                                    let name = cs.with(|c| list_label(&n, c));
                                    view! { <option value=v prop:selected=move || mode.get() == v2>{ui.now("hover-source-list", &[("list", name)])}</option> }
                                }).collect_view()
                            }}
                            <option value="manual" prop:selected=move || mode.get() == "manual">{ui.t("hover-source-manual")}</option>
                        </select>
                    </div>
                </label>
                {fields_view}
                {move || (mode.get() == "manual" && count == 0).then(|| view! {
                    <p class="muted small" data-hover="manual-help">{ui.t("hover-source-manual-help")}</p>
                })}
                {(count > 0).then(|| view! {
                    <div class="field hover-links" data-hover="items">
                        <span class="label">{ui.t("hover-items")}</span>
                        <ul class="hover-list">
                            {rows.into_iter().map(|(t, l)| view! {
                                <li class="hover-row"><span class="hover-row-name">{l}</span>{select_btn(ui, ed, t)}</li>
                            }).collect_view()}
                        </ul>
                    </div>
                })}
            })
        })
    });
    Some(
        view! {
            <div class="insp-group hover-media" data-hover="display">
                <p class="panel-title" title=ui.t("hover-help")>{ui.t("hover-title")}</p>
                <label class="view-line" title=ui.t("hover-on-help")>
                    <span>{ui.t(if image { "hover-on-image" } else { "hover-on" })}</span>
                    <span class="switch">
                        <input type="checkbox" prop:checked=on data-hover="on"
                            on:change=move |e| toggle(event_target_checked(&e))/>
                        <span class="switch-track"></span>
                    </span>
                </label>
                {body.flatten()}
            </div>
        }
        .into_any(),
    )
}

/// Réglages d'un élément survolé (`data-revel-media-for`), ou relier un élément au média au
/// survol de sa page.
pub fn trigger_settings(ui: Ui, ed: Ed, at: &Target, tag: &str, attrs: &[Attr]) -> Option<AnyView> {
    if attr(attrs, DISPLAY).is_some() {
        return None;
    }
    let linked = attr(attrs, FOR).map(|a| a.value.clone().unwrap_or_default());
    ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let here = at.owner.clone();
        // Médias au survol que cet élément peut changer : ceux de son arbre, et ceux des pages
        // qui utilisent ce gabarit ou du gabarit de cette page.
        let near = |o: &Owner| -> bool {
            if *o == here {
                return true;
            }
            match (&here, o) {
                (Owner::Page(p), Owner::Layout(l)) => d
                    .pages
                    .iter()
                    .any(|x| &x.id == p && x.layout.as_deref() == Some(l.as_str())),
                (Owner::Layout(l), Owner::Page(p)) => d
                    .pages
                    .iter()
                    .any(|x| &x.id == p && x.layout.as_deref() == Some(l.as_str())),
                _ => false,
            }
        };
        let displays: Vec<hover::Display> = hover::displays(d)
            .into_iter()
            .filter(|x| near(&x.at.owner) || linked.as_deref() == Some(x.name.as_str()))
            .collect();
        if displays.is_empty() {
            return None;
        }
        // Élément ordinaire : relié seulement s'il peut être survolé (pas un texte en ligne).
        if linked.is_none()
            && !matches!(
                tag,
                "a" | "li" | "button" | "div" | "article" | "figure" | "section"
            )
        {
            return None;
        }
        let trigger = StoredValue::new(at.clone());
        let in_list = hover::enclosing_each(d, at);
        let cols_sig = crate::blocks::collections(ed);
        let options: Vec<(String, String, Target)> = displays
            .iter()
            .map(|x| (x.name.clone(), label_of(ui, d, &x.at, &here), x.at.clone()))
            .collect();
        let options = StoredValue::new(options);
        let in_list_store = StoredValue::new(in_list.clone());
        let image_of = move |name: &str| {
            options.with_value(|o| {
                o.iter().find(|(n, _, _)| n == name).is_some_and(|(_, _, t)| {
                    ed.doc.with_untracked(|d| {
                        d.as_ref()
                            .and_then(|d| revel_edit::address::node(d, t).ok())
                            .is_some_and(|n| {
                                matches!(n, Node::Element { tag, .. } if tag.eq_ignore_ascii_case("img"))
                            })
                    })
                })
            })
        };
        // Média choisi (par son adresse dans la liste des options), ou aucun.
        let choose = move |v: String| {
            let cmd = ed.doc.with_untracked(|d| {
                let d = d.as_ref()?;
                let t = trigger.get_value();
                if v.is_empty() {
                    let mut out = Vec::new();
                    hover::unlink_trigger(d, &t, &mut out);
                    return (!out.is_empty()).then_some(Command::Batch { commands: out });
                }
                let i: usize = v.parse().ok()?;
                let (_, _, disp) = options.with_value(|o| o.get(i).cloned())?;
                let disp_name = hover::attr_of(d, &disp, DISPLAY).unwrap_or_default();
                let image = image_of(&disp_name);
                match in_list_store.get_value() {
                    // Dans une liste : relié aux champs du contenu (choix d'office par nom),
                    // seul élément survolé de la liste.
                    Some((each, item)) => {
                        let key = each_collection(d, &each);
                        let (srcs, imgs) = cols_sig.with_untracked(|cs| {
                            let c = key.as_ref().and_then(|k| cs.iter().find(|c| &c.id == k));
                            (media_fields(c, image), media_fields(c, true))
                        });
                        let src = guess(&srcs, VIDEO_WORDS)
                            .unwrap_or_else(|| if image { "cover" } else { "video" }.into());
                        let poster = (!image).then(|| guess(&imgs, POSTER_WORDS)).flatten();
                        let mut out = Vec::new();
                        for other in hover::triggers(d, &disp_name) {
                            if other != t
                                && hover::enclosing_each(d, &other).is_some_and(|(e, _)| e == each)
                            {
                                hover::unlink_trigger(d, &other, &mut out);
                            }
                        }
                        let c = hover::connect(
                            d,
                            &disp,
                            &t,
                            Some(&hover::field_value(&item, &src)),
                            Some(
                                &poster
                                    .map(|p| hover::field_value(&item, &p))
                                    .unwrap_or_default(),
                            ),
                            false,
                        )?;
                        out.push(c);
                        Some(Command::Batch { commands: out })
                    }
                    None => hover::connect(d, &disp, &t, None, None, false),
                }
            });
            if let Some(cmd) = cmd {
                ed.apply(ui, cmd, false);
            }
        };
        let cur_index = linked.as_deref().and_then(|n| {
            options.with_value(|o| o.iter().position(|(name, _, _)| name == n))
        });
        let cur_display = cur_index.and_then(|i| options.with_value(|o| o.get(i).map(|x| x.2.clone())));
        let media_view = linked.as_ref().map(|name| {
            let image = image_of(name);
            match &in_list {
                Some((each, item)) => {
                    let key = each_collection(d, each).unwrap_or_default();
                    let cur_src =
                        hover::attr_of(d, at, SRC).and_then(|v| hover::field_of(&v, item));
                    let cur_poster =
                        hover::attr_of(d, at, POSTER).and_then(|v| hover::field_of(&v, item));
                    match cur_display.clone() {
                        Some(disp) => field_selects(
                            ui,
                            ed,
                            disp,
                            at.clone(),
                            item.clone(),
                            key,
                            image,
                            cur_src,
                            cur_poster,
                        ),
                        None => ().into_any(),
                    }
                }
                None => address_fields(ui, ed, at.clone(), attrs, image),
            }
        });
        let missing = linked.is_some() && cur_index.is_none();
        Some(
            view! {
                <div class="insp-group hover-media" data-hover="trigger">
                    <p class="panel-title" title=ui.t("hover-help")>{ui.t("hover-title")}</p>
                    <div class="field" data-hover="shown-in" title=ui.t("hover-shown-in-help")>
                        <span class="label">{ui.t("hover-shown-in")}</span>
                        <span class="field-line">
                            <div class="select">
                                <select class="input" on:change=move |e| choose(event_target_value(&e))>
                                    <option value="" prop:selected=cur_index.is_none()>{ui.t("hover-none")}</option>
                                    {options.get_value().into_iter().enumerate().map(|(i, (_, l, _))| {
                                        view! { <option value=i.to_string() prop:selected=cur_index == Some(i)>{l}</option> }
                                    }).collect_view()}
                                </select>
                            </div>
                            {cur_display.map(|t| select_btn(ui, ed, t))}
                        </span>
                    </div>
                    {missing.then(|| view! { <p class="muted small">{ui.t("hover-missing")}</p> })}
                    {media_view}
                    {(linked.is_some() && in_list.is_some()).then(|| view! {
                        <p class="muted small" data-hover="list-note">{ui.t("hover-list-note")}</p>
                    })}
                </div>
            }
            .into_any(),
        )
    })
}
