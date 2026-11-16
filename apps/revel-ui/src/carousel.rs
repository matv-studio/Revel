//! Réglages du carrousel (D-174), en tête de l'onglet Styles : fonction (images, contenu libre,
//! mosaïque), images (choisies une à une ou tirées d'un champ du contenu), affichage, commandes
//! (flèches et leurs icônes, compteur, points, titre : montrés ou cachés), défilement. Tout se
//! règle ici ; les calques ne montrent que les images ou les diapositives.

use crate::editor::Ed;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::make::CarouselMode;
use revel_edit::{Command, Target};
use revel_model::{Attr, Node};

fn attr_of(attrs: &[Attr], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.clone().unwrap_or_default())
}

/// Valeur d'une propriété du style écrit sur l'élément (`--rv-show: 1.3`).
fn style_get(style: &str, name: &str) -> Option<String> {
    style.split(';').find_map(|d| {
        let (k, v) = d.split_once(':')?;
        (k.trim() == name).then(|| v.trim().to_string())
    })
}

/// Style écrit avec la propriété réglée (ou retirée).
fn style_set(style: &str, name: &str, value: Option<&str>) -> String {
    let mut decls: Vec<String> = style
        .split(';')
        .filter_map(|d| {
            let (k, v) = d.split_once(':')?;
            (k.trim() != name).then(|| format!("{}: {}", k.trim(), v.trim()))
        })
        .collect();
    if let Some(v) = value.filter(|v| !v.trim().is_empty()) {
        decls.push(format!("{name}: {}", v.trim()));
    }
    decls.join("; ")
}

/// Partie du carrousel : identifiant, cachée ou non, icône qu'elle contient (identifiant, nom).
#[derive(Clone, Default)]
struct Part {
    id: String,
    hidden: bool,
    icon: Option<(String, String)>,
}

fn part(n: &Node, name: &str) -> Option<Part> {
    let Node::Element {
        id,
        attrs,
        children,
        ..
    } = n
    else {
        return None;
    };
    if let Some(id) = id
        && attrs.iter().any(|a| a.name == name)
    {
        let icon = children.iter().find_map(|c| match c {
            Node::Element {
                id: Some(i), attrs, ..
            } => attr_of(attrs, "data-icon").map(|n| (i.clone(), n)),
            _ => None,
        });
        return Some(Part {
            id: id.clone(),
            hidden: attrs.iter().any(|a| a.name == "hidden"),
            icon,
        });
    }
    children.iter().find_map(|c| part(c, name))
}

/// Partie désignée par sa nature (`data-revel-part="title"`).
fn named_part(n: &Node, kind: &str) -> Option<Part> {
    let Node::Element {
        id,
        attrs,
        children,
        ..
    } = n
    else {
        return None;
    };
    if let Some(id) = id
        && attr_of(attrs, revel_model::PART_ATTR).as_deref() == Some(kind)
    {
        return Some(Part {
            id: id.clone(),
            hidden: attrs.iter().any(|a| a.name == "hidden"),
            icon: None,
        });
    }
    children.iter().find_map(|c| named_part(c, kind))
}

fn switch_row(ui: Ui, key: &'static str, on: bool, set: impl Fn(bool) + 'static) -> AnyView {
    view! {
        <label class="view-line" data-row=key>
            <span>{ui.t(key)}</span>
            <span class="switch">
                <input type="checkbox" prop:checked=on on:change=move |e| set(event_target_checked(&e))/>
                <span class="switch-track"></span>
            </span>
        </label>
    }
    .into_any()
}

fn number_row(
    ui: Ui,
    key: &'static str,
    value: String,
    suffix: &'static str,
    placeholder: &'static str,
    set: impl Fn(String) + 'static,
) -> AnyView {
    view! {
        <div class="style-row" data-row=key>
            <span class="style-label" title=ui.tx(&format!("{key}-help"), &[])><span class="style-label-text">{ui.t(key)}</span></span>
            <div class="style-control">
                <div class="nbox" class:has-units=!suffix.is_empty()>
                    <input class="nbox-input" type="text" inputmode="decimal" spellcheck="false"
                        prop:value=value placeholder=placeholder aria-label=ui.t(key)
                        on:change=move |e| set(event_target_value(&e).trim().replace(',', "."))/>
                    {(!suffix.is_empty()).then(|| view! { <span class="nbox-suffix" aria-hidden="true">{suffix}</span> })}
                </div>
            </div>
        </div>
    }
    .into_any()
}

fn select_row(
    ui: Ui,
    key: &'static str,
    options: Vec<(String, String)>,
    current: String,
    set: impl Fn(String) + 'static,
) -> AnyView {
    view! {
        <div class="style-row" data-row=key>
            <span class="style-label" title=ui.tx(&format!("{key}-help"), &[])><span class="style-label-text">{ui.t(key)}</span></span>
            <div class="style-control">
                <div class="select">
                    <select class="input" aria-label=ui.t(key) on:change=move |e| set(event_target_value(&e))>
                        {options.into_iter().map(|(v, l)| {
                            let on = v == current;
                            view! { <option value=v selected=on>{l}</option> }
                        }).collect_view()}
                    </select>
                </div>
            </div>
        </div>
    }
    .into_any()
}

const RATIOS: &[(&str, &str)] = &[
    ("auto", ""),
    ("1", "1:1"),
    ("4 / 3", "4:3"),
    ("3 / 2", "3:2"),
    ("16 / 9", "16:9"),
    ("3 / 4", "3:4"),
];

/// Réglages du carrousel sélectionné (élément `data-revel-slider`).
pub fn settings(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    let Node::Element { attrs, .. } = node else {
        return None;
    };
    let options = attr_of(attrs, "data-revel-slider")?;
    let style = attr_of(attrs, "style").unwrap_or_default();
    let mode = revel_edit::make::carousel_mode(node);
    let (track_id, _) = revel_edit::make::carousel_track(node)?;
    let track = Target {
        owner: at.owner.clone(),
        node: track_id.to_string(),
    };
    let field = revel_edit::make::carousel_field(node).map(str::to_string);
    let root = StoredValue::new(at.clone());
    let owner = StoredValue::new(at.owner.clone());
    let target = move |id: &str| Target {
        owner: owner.get_value(),
        node: id.to_string(),
    };

    // ---- options de `data-revel-slider` (`autoplay=5 loop drag=off more=6`)
    let opt = |name: &str| {
        options
            .split_whitespace()
            .find_map(|o| o.strip_prefix(&format!("{name}=")).map(str::to_string))
    };
    let flag = |name: &str| options.split_whitespace().any(|o| o == name);
    let set_opt = move |name: &'static str, value: Option<String>| {
        let cur = ed.doc.with_untracked(|d| {
            d.as_ref()
                .and_then(|d| revel_edit::address::node(d, &root.get_value()).ok())
                .and_then(|n| match n {
                    Node::Element { attrs, .. } => attr_of(attrs, "data-revel-slider"),
                    _ => None,
                })
                .unwrap_or_default()
        });
        let mut parts: Vec<String> = cur
            .split_whitespace()
            .filter(|o| *o != name && !o.starts_with(&format!("{name}=")))
            .map(str::to_string)
            .collect();
        match value.as_deref() {
            Some("") => parts.push(name.to_string()),
            Some(v) => parts.push(format!("{name}={v}")),
            None => {}
        }
        ed.apply(
            ui,
            Command::SetAttr {
                at: root.get_value(),
                name: "data-revel-slider".into(),
                value: Some(parts.join(" ")),
            },
            false,
        );
    };
    // ---- propriétés de mise en page écrites sur l'élément
    let set_var = move |name: &'static str, value: Option<String>| {
        let cur = ed.doc.with_untracked(|d| {
            d.as_ref()
                .and_then(|d| revel_edit::address::node(d, &root.get_value()).ok())
                .and_then(|n| match n {
                    Node::Element { attrs, .. } => attr_of(attrs, "style"),
                    _ => None,
                })
                .unwrap_or_default()
        });
        let new = style_set(&cur, name, value.as_deref());
        let cmd = if new.is_empty() {
            Command::RemoveAttr {
                at: root.get_value(),
                name: "style".into(),
            }
        } else {
            Command::SetAttr {
                at: root.get_value(),
                name: "style".into(),
                value: Some(new),
            }
        };
        ed.apply(ui, cmd, false);
    };
    let number = |v: String| v.parse::<f64>().ok().filter(|n| *n > 0.0);
    // ---- parties montrées ou cachées
    let show = move |ids: Vec<String>, on: bool| {
        let commands = ids
            .into_iter()
            .map(|id| {
                if on {
                    Command::RemoveAttr {
                        at: target(&id),
                        name: "hidden".into(),
                    }
                } else {
                    Command::SetAttr {
                        at: target(&id),
                        name: "hidden".into(),
                        value: None,
                    }
                }
            })
            .collect();
        ed.apply(ui, Command::Batch { commands }, false);
    };
    let prev = part(node, "data-revel-prev");
    let next = part(node, "data-revel-next");
    let count = part(node, "data-revel-count");
    let dots = part(node, "data-revel-dots");
    let title = named_part(node, "title");

    // ---- fonction
    let set_mode = move |m: CarouselMode| {
        let cmd = ed.doc.with_untracked(|d| {
            revel_edit::make::carousel_set_mode(d.as_ref()?, &root.get_value(), m)
        });
        if let Some(c) = cmd {
            ed.apply(ui, c, false);
        }
    };
    let modes = [
        (CarouselMode::Images, "carousel-images"),
        (CarouselMode::Content, "carousel-content"),
        (CarouselMode::Mosaic, "carousel-mosaic"),
    ]
    .into_iter()
    .map(|(m, key)| {
        let on = m == mode;
        view! {
            <button type="button" role="radio" class:is-on=on aria-checked=on.to_string() data-mode=key
                title=ui.tx(&format!("{key}-help"), &[])
                on:click=move |_| if !on { set_mode(m) }>{ui.t(key)}</button>
        }
    })
    .collect_view();

    // ---- images : choisies une à une, ou champ du contenu
    let images = (mode != CarouselMode::Content).then(|| {
        let mut sources = vec![("chosen".to_string(), ui.now("carousel-source-chosen", &[]))];
        let mut fields = crate::blocks::image_list_fields(ed, &root.get_value());
        if let Some(f) = &field
            && !fields.iter().any(|(e, _)| e == f)
        {
            let name = f.rsplit('.').next().unwrap_or(f);
            fields.push((f.clone(), crate::names::humanize(name)));
        }
        sources.extend(
            fields
                .into_iter()
                .map(|(e, l)| (format!("field:{e}"), ui.now("carousel-source-field", &[("field", l)]))),
        );
        let current = field
            .as_ref()
            .map(|f| format!("field:{f}"))
            .unwrap_or_else(|| "chosen".into());
        let choose = move |v: String| {
            let expr = v.strip_prefix("field:").map(str::to_string);
            let cmd = ed.doc.with_untracked(|d| {
                revel_edit::make::carousel_set_field(d.as_ref()?, &root.get_value(), expr.as_deref())
            });
            if let Some(c) = cmd {
                ed.apply(ui, c, false);
            }
        };
        let list = field
            .is_none()
            .then(|| view! { <crate::items::ItemsList at=track.clone() grid=true/> });
        view! {
            {(sources.len() > 1).then(|| select_row(ui, "carousel-source", sources, current, choose))}
            {list}
        }
    });
    let slides = (mode == CarouselMode::Content)
        .then(|| view! { <crate::items::ItemsList at=track.clone()/> });

    // ---- affichage
    let ratio = style_get(&style, "--rv-ratio").unwrap_or_default();
    let ratios: Vec<(String, String)> = RATIOS
        .iter()
        .map(|(v, l)| {
            (
                v.to_string(),
                if l.is_empty() {
                    ui.now("carousel-ratio-free", &[])
                } else {
                    l.to_string()
                },
            )
        })
        .collect();
    let ratio_row = (mode != CarouselMode::Content).then(|| {
        let cur = if RATIOS.iter().any(|(v, _)| *v == ratio) {
            ratio.clone()
        } else if mode == CarouselMode::Mosaic {
            "1".to_string()
        } else {
            "4 / 3".to_string()
        };
        select_row(ui, "carousel-ratio", ratios, cur, move |v| {
            set_var("--rv-ratio", Some(v))
        })
    });
    let gap = style_get(&style, "--rv-gap")
        .map(|g| g.trim_end_matches("px").to_string())
        .unwrap_or_default();
    let gap_row = number_row(ui, "carousel-gap", gap, "px", "12", move |v| {
        set_var("--rv-gap", v.parse::<f64>().ok().map(|n| format!("{n}px")))
    });
    let radius = style_get(&style, "--rv-radius")
        .map(|g| g.trim_end_matches("px").to_string())
        .unwrap_or_default();
    let radius_row = number_row(ui, "carousel-radius", radius, "px", "0", move |v| {
        set_var(
            "--rv-radius",
            v.parse::<f64>()
                .ok()
                .filter(|n| *n > 0.0)
                .map(|n| format!("{n}px")),
        )
    });
    let layout = if mode == CarouselMode::Mosaic {
        view! {
            {number_row(ui, "carousel-cols", style_get(&style, "--rv-cols").unwrap_or_default(), "", "3",
                move |v| set_var("--rv-cols", number(v).map(|n| (n.round() as u32).max(1).to_string())))}
            {number_row(ui, "carousel-more", opt("more").unwrap_or_default(), "", "6",
                move |v| set_opt("more", number(v).map(|n| (n.round() as u32).max(2).to_string())))}
        }
        .into_any()
    } else {
        number_row(
            ui,
            "carousel-show",
            style_get(&style, "--rv-show").unwrap_or_default(),
            "",
            "1",
            move |v| set_var("--rv-show", number(v).map(|n| n.to_string())),
        )
    };

    // ---- commandes
    let picker = RwSignal::new(None::<usize>);
    let arrows: Vec<Part> = [prev.clone(), next.clone()].into_iter().flatten().collect();
    let arrows_on = arrows.iter().any(|p| !p.hidden);
    let arrow_ids: Vec<String> = arrows.iter().map(|p| p.id.clone()).collect();
    let icons_row = (arrows_on && arrows.iter().any(|p| p.icon.is_some())).then(|| {
        let buttons = arrows
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let (_, name) = p.icon.clone()?;
                let path = revel_edit::icons::icon(&name).unwrap_or_default();
                let key = if i == 0 { "carousel-icon-prev" } else { "carousel-icon-next" };
                Some(view! {
                    <button type="button" class="icon-btn carousel-icon" class:is-on=move || picker.get() == Some(i)
                        title=ui.t(key) aria-label=ui.t(key) data-arrow=i.to_string()
                        on:click=move |_| picker.update(|p| *p = if *p == Some(i) { None } else { Some(i) })>
                        <svg class="icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor"
                            stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" inner_html=path></svg>
                    </button>
                })
            })
            .collect_view();
        let svgs: Vec<Option<(String, String)>> = arrows.iter().map(|p| p.icon.clone()).collect();
        let svgs = StoredValue::new(svgs);
        view! {
            <div class="style-row" data-row="carousel-icons">
                <span class="style-label"><span class="style-label-text">{ui.t("carousel-icons")}</span></span>
                <div class="style-control carousel-icons">{buttons}</div>
            </div>
            {move || picker.get().and_then(|i| svgs.with_value(|s| s.get(i).cloned().flatten())).map(|(id, name)| {
                view! { <crate::blocks::IconPicker at=target(&id) current=name/> }
            })}
        }
    });
    let position = attr_of(attrs, "data-revel-controls").unwrap_or_else(|| "bottom".into());
    let positions = ["bottom", "top", "sides"]
        .into_iter()
        .map(|p| {
            (
                p.to_string(),
                ui.now(&format!("carousel-position-{p}"), &[]),
            )
        })
        .collect();
    let controls = (mode != CarouselMode::Mosaic).then(|| {
        let toggle = |p: Option<Part>, key: &'static str| {
            p.map(|p| {
                let id = p.id.clone();
                switch_row(ui, key, !p.hidden, move |on| show(vec![id.clone()], on))
            })
        };
        view! {
            <div class="insp-group carousel-controls">
                <p class="panel-title">{ui.t("carousel-controls")}</p>
                {(!arrow_ids.is_empty()).then(|| {
                    let ids = arrow_ids.clone();
                    switch_row(ui, "carousel-arrows", arrows_on, move |on| show(ids.clone(), on))
                })}
                {icons_row}
                {toggle(count.clone(), "carousel-count")}
                {toggle(dots.clone(), "carousel-dots")}
                {toggle(title.clone(), "carousel-title")}
                {title.as_ref().filter(|t| !t.hidden).map(|_| {
                    // Texte du titre : celui que le canevas envoie pour la sélection.
                    view! {
                        <div class="style-row style-row--stack" data-row="carousel-title-text">
                            {move || {
                                let item = ed.texts.with(|(_, items)| items.first().cloned());
                                let value = item.as_ref().map(|i| i.text.clone()).unwrap_or_default();
                                view! {
                                    <input class="input" type="text" prop:value=value aria-label=ui.t("carousel-title")
                                        on:change=move |e| {
                                            let v = event_target_value(&e);
                                            if let Some(cmd) = item.as_ref().and_then(|i| ed.text_command(&i.slot, v)) {
                                                ed.apply(ui, cmd, false);
                                            }
                                        }/>
                                }
                            }}
                        </div>
                    }
                })}
                {select_row(ui, "carousel-position", positions, position, move |v| {
                    ed.apply(ui, Command::SetAttr { at: root.get_value(), name: "data-revel-controls".into(), value: Some(v) }, false)
                })}
            </div>
        }
    });

    // ---- défilement
    let scrolling = (mode != CarouselMode::Mosaic).then(|| {
        let drag = opt("drag").as_deref() != Some("off");
        view! {
            <div class="insp-group slider-settings">
                <p class="panel-title">{ui.t("carousel-scroll")}</p>
                {switch_row(ui, "carousel-drag", drag, move |on| set_opt("drag", (!on).then(|| "off".to_string())))}
                {number_row(ui, "slider-autoplay", opt("autoplay").unwrap_or_default(), "s", "", move |v| {
                    set_opt("autoplay", number(v.trim_end_matches('s').trim().to_string()).map(|n| n.to_string()))
                })}
                {switch_row(ui, "slider-loop", flag("loop"), move |on| set_opt("loop", on.then(String::new)))}
            </div>
        }
    });

    let mode_name = match mode {
        CarouselMode::Images => "images",
        CarouselMode::Content => "content",
        CarouselMode::Mosaic => "mosaic",
    };
    Some(
        view! {
            <div class="insp-group carousel-settings" data-carousel=mode_name>
                <p class="panel-title carousel-head">{icons::carousel()}<span>{ui.t("insert-carousel")}</span></p>
                <div class="segmented segmented--small carousel-mode" role="radiogroup" aria-label=ui.t("fn-title")>{modes}</div>
                {images}
                {slides}
            </div>
            <div class="insp-group carousel-layout">
                <p class="panel-title">{ui.t("carousel-layout")}</p>
                {layout}
                {gap_row}
                {ratio_row}
                {radius_row}
            </div>
            {controls}
            {scrolling}
        }
        .into_any(),
    )
}
