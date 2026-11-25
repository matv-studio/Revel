//! Éléments construits fermés (D-159) : un seul calque, réglés depuis leurs réglages ;
//! « Décomposer » montre leurs parties (calques ordinaires) pour les modifier une à une.

use crate::Ui;
use crate::editor::Ed;
use leptos::prelude::*;
use revel_edit::{Command, Target};

/// L'élément désigné est un élément construit fermé.
pub fn is_widget_at(ed: Ed, at: &Target) -> bool {
    ed.doc.with_untracked(|d| {
        d.as_ref()
            .and_then(|d| revel_edit::address::node(d, at).ok())
            .is_some_and(revel_model::is_widget)
    })
}

/// Décompose l'élément : ses parties deviennent des calques ordinaires (annulable).
pub fn decompose(ui: Ui, ed: Ed, at: Target) {
    ed.apply(
        ui,
        Command::RemoveAttr {
            at,
            name: revel_model::WIDGET_ATTR.into(),
        },
        false,
    );
}

/// Premier descendant (élément) qui porte l'attribut `name` : (identifiant, valeur).
fn find_attr(n: &revel_model::Node, name: &str) -> Option<(String, String)> {
    use revel_model::Node;
    match n {
        Node::Element {
            id,
            attrs,
            children,
            ..
        } => {
            if let (Some(id), Some(a)) = (id, attrs.iter().find(|a| a.name == name)) {
                return Some((id.clone(), a.value.clone().unwrap_or_default()));
            }
            children.iter().find_map(|c| find_attr(c, name))
        }
        Node::Each { children, .. } => children.iter().find_map(|c| find_attr(c, name)),
        _ => None,
    }
}

/// Interrupteur fermé : ce qu'il commande (thème, musique, sons) et le texte de son état.
pub fn switch_settings(ui: Ui, ed: Ed, at: &Target, node: &revel_model::Node) -> Option<AnyView> {
    use revel_model::Node;
    let Node::Element { attrs, .. } = node else {
        return None;
    };
    let cur = attrs
        .iter()
        .find(|a| a.name == "data-revel-switch")?
        .value
        .clone()
        .unwrap_or_default();
    let state = find_attr(node, "data-revel-switch-text");
    let owner = at.owner.clone();
    let at = StoredValue::new(at.clone());
    let state_at = StoredValue::new(state.as_ref().map(|(id, _)| Target {
        owner: owner.clone(),
        node: id.clone(),
    }));
    let texts = state.map(|(_, v)| v).unwrap_or_default();
    let (on, off) = texts
        .split_once('|')
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .unwrap_or((texts.clone(), String::new()));
    // Textes par défaut d'une commande (état allumé, éteint).
    let defaults = move |cmd: &str| -> (String, String) {
        let t = ui.now(
            if cmd == "theme" {
                "insert-text-theme-switch"
            } else {
                "insert-text-switch"
            },
            &[],
        );
        let mut p = t.split('|').skip(1);
        (
            p.next().unwrap_or_default().to_string(),
            p.next().unwrap_or_default().to_string(),
        )
    };
    let texts_s = StoredValue::new((on.clone(), off.clone()));
    let write_text = move |a: String, b: String| {
        if let Some(t) = state_at.get_value() {
            ed.apply(
                ui,
                Command::SetAttr {
                    at: t,
                    name: "data-revel-switch-text".into(),
                    value: Some(format!("{a}|{b}")),
                },
                false,
            );
        }
    };
    let choose = move |cmd: &'static str| {
        let prev = cur_of(ed, &at.get_value());
        let mut cmds = vec![Command::SetAttr {
            at: at.get_value(),
            name: "data-revel-switch".into(),
            value: Some(cmd.into()),
        }];
        // Textes de l'état encore ceux par défaut : ceux de la nouvelle commande.
        let (a, b) = texts_s.get_value();
        let was = defaults(if prev == "theme" { "theme" } else { "music" });
        let now = defaults(cmd);
        if (a.clone(), b.clone()) == was
            && was != now
            && let Some(t) = state_at.get_value()
        {
            cmds.push(Command::SetAttr {
                at: t,
                name: "data-revel-switch-text".into(),
                value: Some(format!("{}|{}", now.0, now.1)),
            });
        }
        ed.apply(ui, Command::Batch { commands: cmds }, false);
    };
    let field = move |key: &'static str, value: String, first: bool| {
        view! {
            <label class="field">
                <span class="label">{ui.t(key)}</span>
                <input class="input" prop:value=value on:change=move |e| {
                    let v = event_target_value(&e);
                    let (a, b) = texts_s.get_value();
                    if first { write_text(v, b) } else { write_text(a, v) }
                }/>
            </label>
        }
    };
    Some(
        view! {
            <div class="insp-group" data-role="switch">
                <p class="panel-title" title=ui.t("switch-role-help")>{ui.t("switch-cmd")}</p>
                <div class="seg seg--small" role="radiogroup" aria-label=ui.t("switch-cmd")>
                    {[("theme", "switch-cmd-theme"), ("music", "switch-music"), ("sounds", "switch-sounds")].into_iter().map(|(v, k)| {
                        let on = cur == v;
                        view! {
                            <button role="radio" data-cmd=v class:is-on=on aria-checked=on.to_string()
                                on:click=move |_| if !on { choose(v) }>
                                <span>{ui.t(k)}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
                {state_at.with_value(Option::is_some).then(|| view! {
                    <div class="gap-pair">
                        {field("switch-text-on", on.clone(), true)}
                        {field("switch-text-off", off.clone(), false)}
                    </div>
                })}
            </div>
        }
        .into_any(),
    )
}

/// Commande actuelle de l'interrupteur désigné.
fn cur_of(ed: Ed, at: &Target) -> String {
    ed.doc.with_untracked(|d| {
        d.as_ref()
            .and_then(|d| revel_edit::address::node(d, at).ok())
            .and_then(|n| match n {
                revel_model::Node::Element { attrs, .. } => attrs
                    .iter()
                    .find(|a| a.name == "data-revel-switch")
                    .and_then(|a| a.value.clone()),
                _ => None,
            })
            .unwrap_or_default()
    })
}

/// Images des parties d'un élément fermé (couverture d'un lecteur vidéo) : réglées depuis
/// l'élément, comme une image ordinaire.
pub fn images_settings(ui: Ui, ed: Ed, at: &Target, node: &revel_model::Node) -> Option<AnyView> {
    use revel_model::Node;
    fn collect(n: &Node, top: bool, out: &mut Vec<revel_model::ImageNode>) {
        match n {
            Node::Image(img) => out.push(img.clone()),
            Node::Element { children, .. } if top || revel_model::is_part(n) => {
                for c in children {
                    collect(c, false, out);
                }
            }
            _ => {}
        }
    }
    let mut imgs = Vec::new();
    collect(node, true, &mut imgs);
    if imgs.is_empty() {
        return None;
    }
    let apply = move |cmd: Command| ed.apply(ui, cmd, false);
    let one = imgs.len() == 1;
    Some(
        imgs.into_iter()
            .enumerate()
            .filter_map(|(i, img)| {
                let id = img.id.clone()?;
                let t = Target {
                    owner: at.owner.clone(),
                    node: id,
                };
                let title = if one {
                    ui.now("widget-image", &[])
                } else {
                    format!("{} {}", ui.now("widget-image", &[]), i + 1)
                };
                Some(view! {
                    <div class="insp-group" data-role="widget-image">
                        <p class="panel-title">{title}</p>
                        {crate::inspector::image(ui, ed, apply, t, img)}
                    </div>
                })
            })
            .collect_view()
            .into_any(),
    )
}

/// Libellé d'une valeur d'un champ « choix » d'après ses options (`.pages.yml` : `values` en
/// textes ou en `{ value, label }`).
fn choice_label(options: &serde_json::Value, value: &str) -> Option<String> {
    let values = options.get("values").or(Some(options))?.as_array()?;
    values.iter().find_map(|v| match v {
        serde_json::Value::Object(m)
            if m.get("value")
                .or_else(|| m.get("name"))
                .and_then(|x| x.as_str())
                == Some(value) =>
        {
            m.get("label").and_then(|x| x.as_str()).map(str::to_string)
        }
        _ => None,
    })
}

/// Champs d'une collection qui peuvent donner des boutons de filtre : choix, texte court, liste
/// de textes.
fn filter_fields(c: &revel_protocol::CmsCollection) -> Vec<(String, String)> {
    c.fields
        .iter()
        .filter(|f| {
            !f.hidden
                && matches!(
                    f.kind.as_deref(),
                    Some("select") | Some("string") | Some("list") | None
                )
                && !(f.kind.is_none() && !f.list)
        })
        .map(|f| {
            (
                f.name.clone(),
                f.label.clone().unwrap_or_else(|| f.name.clone()),
            )
        })
        .collect()
}

/// Boutons qu'un champ donne (clé, libellé), d'après les contenus de la collection.
fn field_buttons(c: &revel_protocol::CmsCollection, field: &str) -> Vec<(String, String)> {
    let options = c
        .fields
        .iter()
        .find(|f| f.name == field)
        .map(|f| f.options.clone())
        .unwrap_or_default();
    revel_model::filter_values(c.entries.iter().flat_map(|e| {
        let v = e.data.get(field).cloned().unwrap_or_default();
        revel_model::field_strings(&v).into_iter().map(|s| {
            let l = choice_label(&options, &s);
            (s, l)
        })
    }))
}

/// Filtres de liste fermés (D-202) : liste de la page et champ dont viennent les boutons ;
/// bouton « Tout » et son libellé ; plusieurs valeurs à la fois. Sans champ : boutons écrits à
/// la main (texte, valeur), comme avant.
pub fn filters_settings(ui: Ui, ed: Ed, at: &Target, node: &revel_model::Node) -> Option<AnyView> {
    use revel_edit::listfilters;
    let st = ed
        .doc
        .with_untracked(|d| d.as_ref().and_then(|d| listfilters::state(d, at).ok()))?;
    let lists = ed.doc.with_untracked(|d| {
        d.as_ref()
            .map(|d| listfilters::lists(d, &at.owner))
            .unwrap_or_default()
    });
    let cols = crate::blocks::collections(ed);
    let root = StoredValue::new(at.clone());
    // Liste choisie : celle reliée, sinon la seule de la page.
    let chosen = RwSignal::new(
        st.list
            .clone()
            .or_else(|| (lists.len() == 1).then(|| lists[0].each.clone())),
    );
    let lists = StoredValue::new(lists);
    let field = st.field.clone();
    let collection_of = move |each: &str| {
        lists.with_value(|l| {
            l.iter()
                .find(|x| x.each == each)
                .map(|x| x.collection.clone())
        })
    };
    let bind = move |list: String, f: String| {
        let all = Some(ui.now("filters-all", &[]));
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let keep = listfilters::state(d, &root.get_value()).ok()?.all;
            listfilters::bind(
                d,
                &root.get_value(),
                &list,
                &f,
                keep.as_deref().or(all.as_deref()),
            )
            .ok()
        });
        if let Some(cmd) = cmd {
            ed.apply_keep(ui, cmd);
        }
    };
    let list_options = move || {
        let mut out = vec![(String::new(), ui.now("filters-list-pick", &[]))];
        lists.with_value(|l| {
            cols.with(|cs| {
                for (i, x) in l.iter().enumerate() {
                    let name = cs
                        .iter()
                        .find(|c| c.id == x.collection)
                        .map(crate::blocks::label_of)
                        .unwrap_or_else(|| x.collection.clone());
                    let same = l.iter().filter(|y| y.collection == x.collection).count() > 1;
                    let n = l[..i]
                        .iter()
                        .filter(|y| y.collection == x.collection)
                        .count()
                        + 1;
                    let name = if same { format!("{name} {n}") } else { name };
                    out.push((
                        x.each.clone(),
                        ui.now("filters-list-name", &[("name", name)]),
                    ));
                }
            })
        });
        out
    };
    let f0 = field.clone();
    let list_row = move || {
        let cur = chosen.get().unwrap_or_default();
        let f = f0.clone();
        crate::toolparts::select_row(ui, "filters-list", list_options(), cur, move |v| {
            let v = (!v.is_empty()).then_some(v);
            chosen.set(v.clone());
            // Même champ dans la nouvelle liste : relié aussitôt.
            if let (Some(l), Some(f)) = (v, f.clone())
                && let Some(c) = collection_of(&l)
                && cols.with_untracked(|cs| {
                    cs.iter()
                        .any(|x| x.id == c && filter_fields(x).iter().any(|(n, _)| *n == f))
                })
            {
                bind(l, f);
            }
        })
    };
    let f1 = field.clone();
    let field_row = move || {
        let list = chosen.get()?;
        let c = collection_of(&list)?;
        let mut opts = vec![(String::new(), ui.now("filters-field-pick", &[]))];
        cols.with(|cs| {
            if let Some(col) = cs.iter().find(|x| x.id == c) {
                opts.extend(filter_fields(col));
            }
        });
        let cur = f1.clone().unwrap_or_default();
        Some(crate::toolparts::select_row(
            ui,
            "filters-field",
            opts,
            cur,
            move |v| {
                if !v.is_empty() {
                    bind(list.clone(), v);
                }
            },
        ))
    };
    let bound = field.is_some() && st.list.is_some();
    let detail = if bound {
        let f = field.clone().unwrap_or_default();
        let list = st.list.clone().unwrap_or_default();
        let all = st.all.clone();
        let has_all = all.is_some();
        let list = StoredValue::new(list);
        let f = StoredValue::new(f);
        let buttons = move || {
            let c = collection_of(&list.get_value()).unwrap_or_default();
            let f = f.get_value();
            cols.with(|cs| {
                cs.iter()
                    .find(|x| x.id == c)
                    .map(|x| field_buttons(x, &f))
                    .unwrap_or_default()
            })
        };
        let a2 = all.clone();
        let set_all = move |label: Option<String>| {
            let cmd = ed.doc.with_untracked(|d| {
                d.as_ref()
                    .and_then(|d| listfilters::set_all(d, &root.get_value(), label.as_deref()).ok())
            });
            if let Some(cmd) = cmd {
                ed.apply_keep(ui, cmd);
            }
        };
        view! {
            <div class="style-row filter-buttons-head" data-row="filters-buttons">
                <span class="style-label"><span class="style-label-text">{ui.t("filters-buttons")}</span></span>
                <span class="filter-buttons-note">{move || ui.now("filters-from-field", &[("n", buttons().len().to_string())])}</span>
            </div>
            <div class="filter-chips" data-list="filters-values">
                {a2.map(|l| view! { <span class="chip filter-chip is-on">{l}</span> })}
                {move || buttons().into_iter().map(|(k, l)| view! { <span class="chip filter-chip" data-value=k>{l}</span> }).collect_view()}
            </div>
            {crate::toolparts::switch_row(ui, "filters-all-button", has_all, move |on| {
                set_all(on.then(|| ui.now("filters-all", &[])))
            })}
            {all.map(|l| view! {
                <div class="style-row" data-row="filters-all-label">
                    <span class="style-label"><span class="style-label-text">{ui.t("filters-all-label")}</span></span>
                    <div class="style-control">
                        <input class="input" type="text" prop:value=l aria-label=ui.t("filters-all-label")
                            on:change=move |e| {
                                let v = event_target_value(&e).trim().to_string();
                                if !v.is_empty() { set_all(Some(v)) }
                            }/>
                    </div>
                </div>
            })}
        }
        .into_any()
    } else {
        written_buttons(ui, ed, at, node)?
    };
    let multi = st.multi;
    Some(
        view! {
            <div class="insp-group" data-role="filters">
                <p class="panel-title tool-head" title=ui.t("filters-role-help")>{crate::icons::filter()}<span>{ui.t("filters-role")}</span></p>
                {list_row}
                {field_row}
                {detail}
                {crate::toolparts::switch_row(ui, "filters-multi", multi, move |on| {
                    ed.apply_keep(ui, listfilters::set_multi(&root.get_value(), on))
                })}
            </div>
        }
        .into_any(),
    )
}

/// Boutons écrits à la main : chaque bouton (texte et valeur cherchée dans les étiquettes des
/// éléments filtrés), ajouter, retirer. Le premier bouton sans valeur montre tout.
fn written_buttons(ui: Ui, ed: Ed, at: &Target, node: &revel_model::Node) -> Option<AnyView> {
    use revel_model::{Attr, Node};
    let Node::Element {
        attrs, children, ..
    } = node
    else {
        return None;
    };
    let param = attrs
        .iter()
        .find(|a| a.name == "data-revel-filters")?
        .value
        .clone()
        .unwrap_or_default();
    let owner = at.owner.clone();
    let root = StoredValue::new(at.clone());
    let template = StoredValue::new(children.iter().rev().find_map(|c| match c {
        Node::Element { attrs, .. } if attrs.iter().any(|a| a.name == "data-revel-filter") => {
            Some(c.clone())
        }
        _ => None,
    }));
    let count = children.len();
    let add = move || {
        let Some(mut n) = template.get_value() else {
            return;
        };
        let n_btn = buttons_len(ed, &root.get_value());
        let label = format!("{} {}", ui.now("filters-option", &[]), n_btn);
        if let Node::Element {
            id,
            attrs,
            children,
            ..
        } = &mut n
        {
            *id = None;
            for a in attrs.iter_mut() {
                if a.name == "data-revel-filter" {
                    *a = Attr {
                        name: a.name.clone(),
                        value: Some(format!("option-{n_btn}")),
                        when: None,
                    };
                }
            }
            *children = vec![Node::Text { text: label }];
        }
        let at = root.get_value();
        let index = ed.doc.with_untracked(|d| {
            match d
                .as_ref()
                .and_then(|d| revel_edit::address::node(d, &at).ok())
            {
                Some(Node::Element { children, .. }) => children.len(),
                _ => count,
            }
        });
        ed.apply(
            ui,
            Command::Insert {
                parent: revel_edit::Parent::of(&at, revel_edit::Branch::Children),
                index,
                node: n,
            },
            false,
        );
    };
    let _ = owner;
    let set_param = move |v: String| {
        ed.apply(
            ui,
            Command::SetAttr {
                at: root.get_value(),
                name: "data-revel-filters".into(),
                value: Some(v.trim().to_string()),
            },
            false,
        );
    };
    // Boutons relus dans le document : la liste suit les modifications.
    let parts = Signal::derive(move || {
        ed.doc.with(|d| {
            match d
                .as_ref()
                .and_then(|d| revel_edit::address::node(d, &root.get_value()).ok())
            {
                Some(Node::Element { children, .. }) => children
                    .iter()
                    .filter_map(|c| match c {
                        Node::Element {
                            id: Some(id),
                            tag,
                            attrs,
                            children,
                        } => {
                            let v = attrs.iter().find(|a| a.name == "data-revel-filter")?;
                            let text = children
                                .iter()
                                .filter_map(|k| match k {
                                    Node::Text { text } => Some(text.as_str()),
                                    _ => None,
                                })
                                .collect::<String>();
                            Some(crate::toolparts::Part {
                                id: id.clone(),
                                tag: tag.clone(),
                                label: id.clone(),
                                name: text.trim().to_string(),
                                extra: Some(v.value.clone().unwrap_or_default()),
                            })
                        }
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            }
        })
    });
    Some(
        view! {
                <crate::toolparts::PartsList owner=owner.clone() title="filters-buttons" add_key="filters-add"
                    parts=parts icon=crate::icons::filter extra=("data-revel-filter", "filters-value")
                    on_add=Callback::new(move |_| add())/>
                <label class="field">
                    <span class="label" title=ui.t("filters-param-help")>{ui.t("filters-param")}</span>
                    <input class="input input--mono" spellcheck="false" prop:value=param
                        on:change=move |e| set_param(event_target_value(&e))/>
                </label>
        }
        .into_any(),
    )
}

/// Nombre de boutons d'un groupe de filtres.
fn buttons_len(ed: Ed, at: &Target) -> usize {
    ed.doc.with_untracked(|d| {
        match d.as_ref().and_then(|d| revel_edit::address::node(d, at).ok()) {
            Some(revel_model::Node::Element { children, .. }) => children
                .iter()
                .filter(|c| matches!(c, revel_model::Node::Element { attrs, .. } if attrs.iter().any(|a| a.name == "data-revel-filter")))
                .count(),
            _ => 0,
        }
    })
}
