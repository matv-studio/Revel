//! Réglages de l'élément sélectionné (étape 2.5), dans l'onglet Styles : en tête, son contenu
//! (lien, image, composant, condition, liste) ; en bas, rubrique « Paramètres avancés » (balise,
//! identifiant, attributs). Chaque changement validé (sortie du champ, Entrée) est une commande
//! d'édition.

use crate::editor::Ed;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::{Command, Target};
use revel_model::{Attr, ImageNode, Node};

/// Chaîne littérale d'une expression (`"/media/a.jpg"` -> `/media/a.jpg`) ; `None` si c'est une
/// vraie expression (variable, filtre…).
pub fn literal(expr: &str) -> Option<String> {
    let e = expr.trim();
    let q = e.chars().next()?;
    if (q != '"' && q != '\'') || e.len() < 2 || !e.ends_with(q) {
        return None;
    }
    let inner = &e[1..e.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push(chars.next()?),
            c if c == q => return None,
            c => out.push(c),
        }
    }
    Some(out)
}

/// Expression littérale d'un texte.
pub fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

const TAGS: &[&str] = &[
    "div",
    "section",
    "article",
    "aside",
    "header",
    "footer",
    "main",
    "nav",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "p",
    "span",
    "a",
    "ul",
    "ol",
    "li",
    "figure",
    "figcaption",
    "blockquote",
    "strong",
    "em",
    "button",
];

fn attr<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a Attr> {
    attrs.iter().find(|a| a.name == name)
}

/// Champ texte validé à la sortie ou par Entrée.
#[component]
pub(crate) fn Field(
    #[prop(into)] label: Signal<String>,
    #[prop(into)] value: Signal<String>,
    on_commit: impl Fn(String) + Send + Sync + Clone + 'static,
    #[prop(optional)] mono: bool,
    #[prop(optional, into)] placeholder: MaybeProp<String>,
    /// Boutons à droite du champ (choisir un média…).
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    // Valeur validée une seule fois (Entrée puis sortie du champ : une seule modification).
    let last = StoredValue::new(None::<String>);
    let commit = move |v: String| {
        if v == value.get_untracked() || last.with_value(|l| l.as_deref() == Some(v.as_str())) {
            return;
        }
        last.set_value(Some(v.clone()));
        on_commit(v);
    };
    let c2 = commit.clone();
    let input = view! {
        <input class="input" class:input--mono=mono prop:value=value placeholder=placeholder
            on:change=move |e| c2(event_target_value(&e))
            on:keydown=move |e: leptos::ev::KeyboardEvent| {
                if e.key() == "Enter" {
                    commit(event_target_value(&e));
                }
            }/>
    };
    match children {
        None => view! {
            <label class="field">
                {move || label.with(|l| (!l.is_empty()).then(|| view! { <span class="label">{l.clone()}</span> }))}
                {input}
            </label>
        }
        .into_any(),
        Some(extra) => view! {
            <div class="field">
                <span class="label">{label}</span>
                <span class="field-line">{input}{extra()}</span>
            </div>
        }
        .into_any(),
    }
}

/// Nœud sélectionné et son adresse.
fn current(ed: Ed) -> Memo<Option<(Target, Node)>> {
    Memo::new(move |_| {
        let target = ed.selected_target()?;
        let node = ed.doc.with(|d| {
            revel_edit::address::node(d.as_ref()?, &target)
                .ok()
                .cloned()
        })?;
        Some((target, node))
    })
}

/// Contenu de l'élément sélectionné, en tête de l'onglet Styles : lien, image ou média, utilisation
/// de composant (propriétés, variante), condition, liste, réglages propres à un bloc ; dans un
/// composant ouvert, ses propriétés et les liaisons de l'élément.
#[component]
pub fn ContentSettings() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let current = current(ed);
    let apply = move |cmd: Command| ed.apply(ui, cmd, false);
    let in_component = move || {
        current.get().and_then(|(at, node)| {
            let h = crate::components::Holder::of(&at)?;
            let bind = (!matches!(node, Node::Use { .. }))
                .then(|| crate::components::bindings(ui, ed, h.clone(), at.clone()));
            // Propriétés du composant ouvert ; celles d'un gabarit se règlent avec la page.
            let section = matches!(h, crate::components::Holder::Component(_))
                .then(|| crate::components::props_section(ui, ed, h));
            Some(view! {
                {bind}
                {section}
            })
        })
    };
    view! {
        <div class="content-settings">
            {move || current.get().and_then(|(at, node)| crate::blocks::extra_settings(ui, ed, at, node))}
            {move || current.get().map(|(at, node)| match node {
                Node::Element { tag, attrs, children, .. } => {
                    let anchors = has_section_links(&children);
                    element_content(ui, ed, apply, at, tag, attrs, anchors).into_any()
                }
                Node::Image(img) => image(ui, ed, apply, at, img).into_any(),
                Node::If { cond, .. } => {
                    // Champ du contenu (liste, modèle) : condition choisie dans une liste ;
                    // l'expression reste modifiable dessous.
                    let readable = crate::blocks::readable_condition(ui, ed, at.clone(), cond.clone());
                    view! {
                        <div class="stack">
                            {readable}
                            <Field label=ui.t("inspector-condition") value=cond mono=true
                                on_commit=move |v| apply(Command::SetLogic { at: at.clone(), cond: Some(v), expr: None, item: None })/>
                        </div>
                    }.into_any()
                }
                Node::Each { expr, item, .. } => {
                    let (a1, a2) = (at.clone(), at.clone());
                    view! {
                        <div class="stack">
                            <Field label=ui.t("inspector-each-list") value=expr mono=true
                                on_commit=move |v| apply(Command::SetLogic { at: a1.clone(), cond: None, expr: Some(v), item: None })/>
                            <Field label=ui.t("inspector-each-item") value=item mono=true
                                on_commit=move |v| apply(Command::SetLogic { at: a2.clone(), cond: None, expr: None, item: Some(v) })/>
                        </div>
                    }.into_any()
                }
                Node::Use { component, args, .. } => crate::components::instance_settings(ui, ed, at, component, args),
                _ => ().into_any(),
            })}
            {in_component}
        </div>
    }
}

/// Paramètres avancés de l'élément (balise, identifiant, attributs), rubrique repliée en bas de
/// l'onglet Styles.
#[component]
pub fn AdvancedSettings() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let current = current(ed);
    let apply = move |cmd: Command| ed.apply(ui, cmd, false);
    view! {
        {move || current.get().and_then(|(at, node)| match node {
            Node::Element { tag, attrs, .. } => Some(view! {
                <details class="style-section advanced-settings">
                    <summary><span>{ui.t("inspector-advanced")}</span></summary>
                    <div class="style-section-body">{element_advanced(ui, ed, apply, at, tag, attrs)}</div>
                </details>
            }),
            _ => None,
        })}
    }
}

/// Modifie ou retire un attribut (valeur vide : retiré).
fn setter(
    apply: impl Fn(Command) + Copy + Send + Sync + 'static,
    at: Target,
) -> impl Fn(&str, String) + Clone + Send + Sync + 'static {
    move |name: &str, v: String| {
        let at = at.clone();
        if v.trim().is_empty() {
            apply(Command::RemoveAttr {
                at,
                name: name.to_string(),
            });
        } else {
            apply(Command::SetAttr {
                at,
                name: name.to_string(),
                value: Some(v),
            });
        }
    }
}

/// Liens vers une section de la page (`#id`) parmi les descendants.
fn has_section_links(nodes: &[Node]) -> bool {
    nodes.iter().any(|n| match n {
        Node::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            (tag == "a"
                && attr(attrs, "href")
                    .and_then(|a| a.value.as_deref())
                    .and_then(|h| h.split_once('#'))
                    .is_some_and(|(p, id)| matches!(p, "" | "./" | "/" | ".") && !id.is_empty()))
                || has_section_links(children)
        }
        Node::If {
            then, otherwise, ..
        } => has_section_links(then) || has_section_links(otherwise),
        Node::Each { children, .. } => has_section_links(children),
        _ => false,
    })
}

/// Contenu d'un élément : lien (adresse, nouvel onglet), source d'un média et texte alternatif.
fn element_content(
    ui: Ui,
    ed: Ed,
    apply: impl Fn(Command) + Copy + Send + Sync + 'static,
    at: Target,
    tag: String,
    attrs: Vec<Attr>,
    anchors: bool,
) -> impl IntoView {
    let set = setter(apply, at.clone());
    let lower = tag.to_ascii_lowercase();
    // Navigation vers les sections de la page : lien de la section affichée marqué « actif ».
    let spy = attr(&attrs, "data-revel-spy").is_some();
    let set_spy = set.clone();
    let is_link = lower == "a";
    let href = attr(&attrs, "href")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let blank = attr(&attrs, "target").and_then(|a| a.value.as_deref()) == Some("_blank");
    let blank_at = at.clone();
    let toggle_blank = move |on: bool| {
        let at = blank_at.clone();
        let cmd = if on {
            Command::Batch {
                commands: vec![
                    Command::SetAttr {
                        at: at.clone(),
                        name: "target".into(),
                        value: Some("_blank".into()),
                    },
                    Command::SetAttr {
                        at,
                        name: "rel".into(),
                        value: Some("noopener".into()),
                    },
                ],
            }
        } else {
            Command::RemoveAttr {
                at,
                name: "target".into(),
            }
        };
        apply(cmd);
    };
    let set_href = set.clone();
    // Lien marqué « page actuelle » aussi sur les pages dont l'adresse commence par la sienne.
    let section =
        attr(&attrs, "data-revel-current").and_then(|a| a.value.as_deref()) == Some("section");
    let set_section = set.clone();
    // Barre de progression : défilement suivi (colonne qui la contient ou page).
    let progress = attr(&attrs, "data-revel-progress").map(|a| {
        if a.value.as_deref() == Some("page") {
            "page"
        } else {
            "parent"
        }
    });
    let set_progress = set.clone();
    let media = matches!(lower.as_str(), "img" | "video" | "audio" | "source");
    // Média au survol : son adresse est celle de l'élément survolé (réglée plus haut).
    let hover_display = attr(&attrs, revel_edit::hover::DISPLAY).is_some();
    let is_img = lower == "img";
    let src = attr(&attrs, "src")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let dynamic_src = src.contains("{{") || src.contains("{%");
    let alt = attr(&attrs, "alt")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let alt_empty = alt.trim().is_empty();
    let (set_src, set_alt) = (set.clone(), set.clone());
    let at_pick = at.clone();
    let pick = move || {
        let at = at_pick.clone();
        crate::library::pick_media(ui, is_img, crate::library::media_shelf(ed), move |url| {
            let mut commands = vec![Command::SetAttr {
                at: at.clone(),
                name: "src".into(),
                value: Some(url.clone()),
            }];
            if is_img
                && alt_empty
                && let Some(a) = ed
                    .doc
                    .with_untracked(|d| d.as_ref().and_then(|d| d.library.alt.get(&url).cloned()))
            {
                commands.push(Command::SetAttr {
                    at: at.clone(),
                    name: "alt".into(),
                    value: Some(a),
                });
            }
            apply(if commands.len() == 1 {
                commands.remove(0)
            } else {
                Command::Batch { commands }
            });
        });
    };
    let key = if is_img {
        "picker-choose-image"
    } else {
        "picker-choose-media"
    };
    let level = (lower.len() == 2 && lower.starts_with('h'))
        .then(|| lower[1..].parse::<u8>().ok())
        .flatten()
        .filter(|l| (1..=6).contains(l));
    let level_at = at.clone();
    view! {
        // Niveau d'un titre (comme les boutons H1 à H6 de Webflow).
        {level.map(|cur| view! {
            <div class="stack insp-content">
                <div class="field">
                    <span class="label">{ui.t("inspector-heading-level")}</span>
                    <div class="seg seg--small" role="radiogroup" aria-label=ui.t("inspector-heading-level")>
                        {(1..=6u8).map(|l| {
                            let at = level_at.clone();
                            view! {
                                <button role="radio" class:is-on=l == cur aria-checked=(l == cur).to_string()
                                    on:click=move |_| if l != cur { apply(Command::SetTag { at: at.clone(), tag: format!("h{l}") }) }>
                                    <span>{format!("H{l}")}</span>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                </div>
            </div>
        })}
        {progress.map(|cur| view! {
            <div class="stack insp-content">
                <div class="field">
                    <span class="label" title=ui.t("inspector-progress-help")>{ui.t("inspector-progress")}</span>
                    <div class="seg seg--small" role="radiogroup" aria-label=ui.t("inspector-progress")>
                        {[("parent", "inspector-progress-parent"), ("page", "inspector-progress-page")].into_iter().map(|(v, k)| {
                            let set = set_progress.clone();
                            view! {
                                <button role="radio" class:is-on=v == cur aria-checked=(v == cur).to_string()
                                    on:click=move |_| if v != cur { set("data-revel-progress", v.to_string()) }>
                                    <span>{ui.t(k)}</span>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                </div>
            </div>
        })}
        {is_link.then(|| view! {
            <div class="stack insp-content">
                // Adresse reliée (contenu, information du site, menu) : réglée plus haut.
                {(!href.contains("{{")).then(|| view! {
                    <LinkTarget ed href=href.clone() on_commit=move |v: String| set_href("href", v)/>
                })}
                <label class="row-switch">
                    <span class="switch">
                        <input type="checkbox" id="insp-new-tab" prop:checked=blank on:change=move |e| toggle_blank(event_target_checked(&e))/>
                        <span class="switch-track"></span>
                    </span>
                    <span>{ui.t("inspector-new-tab")}</span>
                </label>
                <label class="row-switch" title=ui.t("inspector-current-section-help")>
                    <span class="switch">
                        <input type="checkbox" prop:checked=section on:change=move |e| {
                            set_section("data-revel-current", if event_target_checked(&e) { "section".into() } else { String::new() })
                        }/>
                        <span class="switch-track"></span>
                    </span>
                    <span>{ui.t("inspector-current-section")}</span>
                </label>
            </div>
        })}
        // Élément d'un composant : emplacement du contenu propre à chaque utilisation (D-118).
        {matches!(at.owner, revel_edit::Owner::Component(_)).then(|| {
            let slot = attr(&attrs, revel_model::SLOT_ATTR).is_some();
            let slot_at = at.clone();
            view! {
                <div class="stack insp-content">
                    <label class="row-switch" title=ui.t("inspector-slot-help")>
                        <span class="switch">
                            <input type="checkbox" id="insp-slot" prop:checked=slot on:change=move |e| {
                                let on = event_target_checked(&e);
                                let cmd = ed.doc.with_untracked(|d| revel_edit::make::set_slot(d.as_ref()?, &slot_at, on).ok());
                                if let Some(c) = cmd { apply(c) }
                            }/>
                            <span class="switch-track"></span>
                        </span>
                        <span>{ui.t("inspector-slot")}</span>
                    </label>
                </div>
            }
        })}
        {(anchors || spy).then(|| view! {
            <div class="stack insp-content">
                <label class="row-switch" title=ui.t("inspector-spy-help")>
                    <span class="switch">
                        <input type="checkbox" id="insp-spy" prop:checked=spy on:change=move |e| {
                            set_spy("data-revel-spy", if event_target_checked(&e) { "section".into() } else { String::new() })
                        }/>
                        <span class="switch-track"></span>
                    </span>
                    <span>{ui.t("inspector-spy")}</span>
                </label>
            </div>
        })}
        {(media && !hover_display).then(|| view! {
            <div class="stack insp-content">
                <Field label=ui.t(match lower.as_str() { "video" => "inspector-src-video", "audio" => "inspector-src-audio", _ => "inspector-src" }) value=src.clone() mono=dynamic_src
                    on_commit=move |v| set_src("src", v)>
                    <button type="button" class="icon-btn icon-btn--small" title=ui.t(key) aria-label=ui.t(key)
                        disabled=dynamic_src on:click=move |_| pick()>{icons::image()}</button>
                </Field>
                {is_img.then(|| view! {
                    <Field label=ui.t("inspector-alt") value=alt.clone() on_commit=move |v| set_alt("alt", v)/>
                })}
            </div>
        })}
    }
}

/// Destination d'un lien : nature de l'adresse écrite (`mailto:`, `tel:`, page du site, autre).
#[derive(Clone, Copy, PartialEq)]
enum LinkKind {
    Page,
    Url,
    Email,
    Phone,
}

fn link_kind(href: &str, pages: &[String]) -> LinkKind {
    if href.starts_with("mailto:") {
        LinkKind::Email
    } else if href.starts_with("tel:") {
        LinkKind::Phone
    } else if href.is_empty() || href == "#" || pages.iter().any(|p| p == href) {
        LinkKind::Page
    } else {
        LinkKind::Url
    }
}

/// Destination d'un lien, comme dans Webflow : une page du site choisie dans la liste, une
/// adresse, une adresse e-mail ou un numéro de téléphone (le préfixe s'écrit seul).
#[component]
fn LinkTarget(
    ed: Ed,
    href: String,
    on_commit: impl Fn(String) + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    // Pages fixes du site (adresse, nom).
    let pages: Vec<(String, String)> = ed.pages.with_untracked(|ps| {
        ps.iter()
            .filter(|p| {
                p.page.is_some()
                    && !p.url.ends_with(".xml")
                    && !p.url.ends_with(".txt")
                    && !p.url.contains(".ht")
            })
            .map(|p| (p.url.clone(), crate::pagetree::title_of(p)))
            .collect()
    });
    let urls: Vec<String> = pages.iter().map(|(u, _)| u.clone()).collect();
    let kind = RwSignal::new(link_kind(&href, &urls));
    let href = StoredValue::new(href);
    let commit = StoredValue::new(on_commit);
    let run = move |v: String| commit.with_value(|f| f(v));
    let tabs = [
        (LinkKind::Page, "link-page"),
        (LinkKind::Url, "link-url"),
        (LinkKind::Email, "link-email"),
        (LinkKind::Phone, "link-phone"),
    ];
    let value_of = move |k: LinkKind| {
        let h = href.get_value();
        match k {
            LinkKind::Email => h.strip_prefix("mailto:").unwrap_or_default().to_string(),
            LinkKind::Phone => h.strip_prefix("tel:").unwrap_or_default().to_string(),
            _ if link_kind(&h, &[]) == LinkKind::Url || k == LinkKind::Url => {
                if h == "#" {
                    String::new()
                } else {
                    h
                }
            }
            _ => h,
        }
    };
    view! {
        <div class="field link-target">
            <span class="label">{ui.t("inspector-href")}</span>
            <div class="seg seg--small" role="radiogroup" aria-label=ui.t("inspector-href")>
                {tabs.into_iter().map(|(k, key)| view! {
                    <button role="radio" class:is-on=move || kind.get() == k aria-checked=move || (kind.get() == k).to_string()
                        on:click=move |_| kind.set(k)><span>{ui.t(key)}</span></button>
                }).collect_view()}
            </div>
            {move || match kind.get() {
                LinkKind::Page => {
                    let cur = href.get_value();
                    let known = pages.iter().any(|(u, _)| *u == cur);
                    view! {
                        <div class="select">
                            <select class="input" aria-label=ui.t("link-page") on:change=move |e| run(event_target_value(&e))>
                                <option value="" hidden=true selected=!known>{ui.t("link-page-choose")}</option>
                                {pages.iter().map(|(u, name)| view! {
                                    <option value=u.clone() selected=*u == cur>{name.clone()}</option>
                                }).collect_view()}
                            </select>
                        </div>
                    }.into_any()
                }
                k => {
                    let (ph, prefix) = match k {
                        LinkKind::Email => ("nom@exemple.fr", "mailto:"),
                        LinkKind::Phone => ("+33 4 72 27 03 74", "tel:"),
                        _ => ("https://…", ""),
                    };
                    view! {
                        <Field label=String::new() value=value_of(k) placeholder=ph
                            on_commit=move |v: String| {
                                let v = v.trim().to_string();
                                if v.is_empty() { run(String::new()); return; }
                                let v = match k {
                                    LinkKind::Phone => v.chars().filter(|c| !c.is_whitespace() && *c != '.').collect::<String>().replace("(0)", ""),
                                    _ => v,
                                };
                                run(format!("{prefix}{v}"))
                            }/>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Paramètres avancés d'un élément : balise, identifiant, classes calculées, autres attributs.
fn element_advanced(
    ui: Ui,
    ed: Ed,
    apply: impl Fn(Command) + Copy + Send + Sync + 'static,
    at: Target,
    tag: String,
    attrs: Vec<Attr>,
) -> impl IntoView {
    let set = setter(apply, at.clone());
    let class_value = attr(&attrs, "class")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let dynamic_class = class_value.contains("{{") || class_value.contains("{%");
    let id_value = attr(&attrs, "id")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let tag_at = at.clone();
    let tag_now = tag.clone();
    let lower = tag.to_ascii_lowercase();
    let had_link_attrs: Vec<&'static str> = ["href", "target", "rel"]
        .into_iter()
        .filter(|n| attrs.iter().any(|a| a.name == *n))
        .collect();
    let media = matches!(lower.as_str(), "img" | "video" | "audio" | "source");
    let is_link = lower == "a";
    let others: Vec<Attr> = attrs
        .iter()
        .filter(|a| !matches!(a.name.as_str(), "id" | "class"))
        // Lien et média : réglés en tête de l'onglet.
        .filter(|a| !(is_link && matches!(a.name.as_str(), "href" | "target")))
        .filter(|a| !(media && matches!(a.name.as_str(), "src" | "alt")))
        // Réglages de Revel qui ont leur propre contrôle (onglets, carrousel, lecteur, filtres,
        // consentement, interrupteurs, formulaire, média au survol, lien actif, interactions) :
        // montrés là, pas comme attributs bruts.
        .filter(|a| !MANAGED.contains(&a.name.as_str()))
        // Animations : réglées dans le panneau Interactions.
        .filter(|a| {
            !crate::interactions::FAMILIES
                .iter()
                .any(|f| a.name == f.attr())
        })
        .cloned()
        .collect();
    let set_id = set.clone();
    let set_class_raw = set.clone();
    let new_attr_at = at.clone();
    let lang = ed.lang();
    view! {
        <div class="stack">
            // Nature de l'élément par son nom (« Navigation », « Pied de page »), la balise en
            // info-bulle ; une balise hors de la liste reste proposée telle quelle.
            <label class="field">
                <span class="label">{ui.t("inspector-tag")}</span>
                <div class="select">
                    <select class="input" title=format!("<{lower}>")
                        on:change=move |e| {
                            let v = event_target_value(&e);
                            if !v.is_empty() && v != tag_now {
                                // Un lien qui devient autre chose perd son adresse et sa cible.
                                let mut commands = vec![Command::SetTag { at: tag_at.clone(), tag: v.clone() }];
                                if tag_now.eq_ignore_ascii_case("a") && v != "a" {
                                    for name in ["href", "target", "rel"] {
                                        if had_link_attrs.contains(&name) {
                                            commands.push(Command::RemoveAttr { at: tag_at.clone(), name: name.into() });
                                        }
                                    }
                                }
                                apply(if commands.len() == 1 { commands.remove(0) } else { Command::Batch { commands } });
                            }
                        }>
                        {(!TAGS.contains(&lower.as_str())).then(|| view! { <option value=lower.clone() selected=true>{format!("<{lower}>")}</option> })}
                        {TAGS.iter().map(|t| view! {
                            <option value=*t selected=*t == lower>{crate::names::type_name(ui, t)}</option>
                        }).collect_view()}
                    </select>
                </div>
            </label>
            <Field label=ui.t("inspector-id") value=id_value mono=true on_commit=move |v| set_id("id", v)/>
            {dynamic_class.then(|| view! {
                // Classes calculées (expression) : modifiées comme du texte.
                <label class="field">
                    <span class="label">{ui.t("inspector-classes")}</span>
                    <input class="input input--mono" prop:value=class_value.clone()
                        on:change=move |e| set_class_raw("class", event_target_value(&e))/>
                </label>
            })}
            <div class="insp-group insp-attrs">
                <p class="panel-title">{ui.t("inspector-attributes")}</p>
                {others.into_iter().map(|a| {
                    let name = a.name.clone();
                    let name_rm = a.name.clone();
                    let at_set = at.clone();
                    let at_rm = at.clone();
                    let conditional = a.when.clone();
                    // Autre langue : texte lu par les visiteurs (`alt`, `title`…) traduit.
                    let source = a.value.clone().unwrap_or_default();
                    let translate = lang.clone().filter(|_| {
                        revel_edit::locale::TEXT_ATTRS.contains(&a.name.as_str())
                            && revel_edit::locale::translatable(&source)
                    });
                    let input = match translate {
                        Some(code) => {
                            let at_tr = at.clone();
                            let n_tr = a.name.clone();
                            view! {
                                <input class="input input--locale" prop:value=ed.translation(&code, &source).unwrap_or_default()
                                    placeholder=source.clone() title=ui.t("inspector-locale-help")
                                    on:change=move |e| {
                                        apply(Command::Translate {
                                            locale: code.clone(),
                                            place: revel_edit::locale::TextPlace::Attr { at: at_tr.clone(), name: n_tr.clone() },
                                            text: event_target_value(&e),
                                        });
                                    }/>
                            }.into_any()
                        }
                        None => view! {
                            <input class="input input--mono" prop:value=a.value.clone().unwrap_or_default()
                                placeholder=ui.t("inspector-attr-boolean")
                                on:change=move |e| {
                                    let v = event_target_value(&e);
                                    apply(Command::SetAttr { at: at_set.clone(), name: name.clone(), value: (!v.is_empty()).then_some(v) });
                                }/>
                        }.into_any(),
                    };
                    // Valeur reliée (champ, propriété, information du site) : réglée sans accolades.
                    let value = a.value.clone().unwrap_or_default();
                    let bound = (conditional.is_none())
                        .then(|| crate::boundattr::attr_editor(ui, ed, at.clone(), a.name.clone(), value.clone()))
                        .flatten();
                    let link = if bound.is_some() {
                        crate::boundattr::unlink_button(ui, ed, at.clone(), a.name.clone(), value.clone())
                    } else if conditional.is_none() && a.value.is_some() && !value.contains("{%") && !matches!(a.name.as_str(), "class" | "id") {
                        Some(crate::boundattr::link_button(ui, ed, at.clone(), a.name.clone(), value.clone()))
                    } else {
                        None
                    };
                    let has_bound = bound.is_some();
                    view! {
                        <div class="attr-row" class:attr-row--bound=has_bound>
                            <span class="attr-name" title=conditional.clone().map(|w| ui.tx("inspector-attr-when", &[("cond", w)]))>{a.name.clone()}{conditional.is_some().then_some(" ?")}</span>
                            {(!has_bound).then_some(input)}
                            <span class="attr-actions">
                                {link}
                                <button class="icon-btn" title=ui.t("inspector-attr-remove") aria-label=ui.t("inspector-attr-remove")
                                    on:click=move |_| apply(Command::RemoveAttr { at: at_rm.clone(), name: name_rm.clone() })>{icons::trash()}</button>
                            </span>
                        </div>
                        {bound}
                    }
                }).collect_view()}
                <input class="input input--mono" placeholder=ui.t("inspector-attr-add")
                    on:change=move |e| {
                        // `nom` ou `nom=valeur`.
                        let v = event_target_value(&e);
                        let (n, val) = match v.split_once('=') {
                            Some((n, val)) => (n.trim().to_string(), Some(val.trim().trim_matches('"').to_string())),
                            None => (v.trim().to_string(), None),
                        };
                        if !n.is_empty() {
                            apply(Command::SetAttr { at: new_attr_at.clone(), name: n, value: val });
                        }
                    }/>
            </div>
        </div>
    }
}

fn image(
    ui: Ui,
    ed: Ed,
    apply: impl Fn(Command) + Copy + Send + Sync + 'static,
    at: Target,
    img: ImageNode,
) -> impl IntoView {
    // Valeur affichée : texte littéral, sinon l'expression.
    let show = |e: &str| literal(e).unwrap_or_else(|| e.to_string());
    let src_expr = literal(&img.src).is_none();
    let alt_expr = img.alt.as_deref().is_some_and(|a| literal(a).is_none());
    let src = show(&img.src);
    let alt = img.alt.as_deref().map(show).unwrap_or_default();
    let (img1, img2, img3) = (img.clone(), img.clone(), img.clone());
    let lang = ed.lang();
    let (at1, at2, at3) = (at.clone(), at.clone(), at.clone());
    // Image choisie parmi les médias : texte alternatif par défaut du média si l'image n'en a pas.
    let choose = move || {
        let (img, at) = (img3.clone(), at3.clone());
        crate::library::pick_image(ui, crate::library::media_shelf(ed), move |url| {
            let mut i = img.clone();
            i.src = quote(&url);
            let empty_alt = i
                .alt
                .as_deref()
                .is_none_or(|a| literal(a).is_some_and(|t| t.trim().is_empty()));
            if empty_alt
                && let Some(a) = ed
                    .doc
                    .with_untracked(|d| d.as_ref().and_then(|d| d.library.alt.get(&url).cloned()))
            {
                i.alt = Some(quote(&a));
            }
            apply(Command::SetImage {
                at: at.clone(),
                image: i,
            });
        });
    };
    view! {
        <div class="stack">
            <Field label=Signal::derive(move || ui.tx(if src_expr { "inspector-src-expr" } else { "inspector-src" }, &[]))
                value=src mono=src_expr
                on_commit=move |v| {
                    let mut i = img1.clone();
                    i.src = if src_expr { v } else { quote(&v) };
                    apply(Command::SetImage { at: at1.clone(), image: i });
                }>
                <button type="button" class="icon-btn icon-btn--small" title=ui.t("picker-choose-image") aria-label=ui.t("picker-choose-image")
                    disabled=src_expr on:click=move |_| choose()>{icons::image()}</button>
            </Field>
            {match lang.filter(|_| !alt_expr && revel_edit::locale::translatable(&alt)) {
                // Autre langue : texte alternatif traduit.
                Some(code) => {
                    let value = ed.translation(&code, &alt).unwrap_or_default();
                    view! {
                        <Field label=ui.t("inspector-alt") value=value placeholder=alt.clone()
                            on_commit=move |v| apply(Command::Translate {
                                locale: code.clone(),
                                place: revel_edit::locale::TextPlace::Attr { at: at2.clone(), name: "alt".into() },
                                text: v,
                            })/>
                    }.into_any()
                }
                None => view! {
                    <Field label=Signal::derive(move || ui.tx(if alt_expr { "inspector-alt-expr" } else { "inspector-alt" }, &[]))
                        value=alt mono=alt_expr
                        on_commit=move |v| {
                            let mut i = img2.clone();
                            i.alt = Some(if alt_expr { v } else { quote(&v) });
                            apply(Command::SetImage { at: at2.clone(), image: i });
                        }/>
                }.into_any(),
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn litteraux() {
        assert_eq!(literal("\"/a b.jpg\""), Some("/a b.jpg".into()));
        assert_eq!(literal("'l\\'agence'"), Some("l'agence".into()));
        assert_eq!(literal("item.src"), None);
        assert_eq!(literal("\"a\" ~ b ~ \"c\""), None);
        for t in ["simple", "avec \"guillemets\"", "barre \\ oblique"] {
            assert_eq!(literal(&quote(t)).as_deref(), Some(t));
        }
    }
}

/// Attributs de Revel réglés par leurs propres contrôles.
const MANAGED: &[&str] = &[
    "data-revel-tabs",
    "data-revel-tab",
    "data-revel-panel",
    "data-revel-slider",
    "data-revel-track",
    "data-revel-prev",
    "data-revel-next",
    "data-revel-count",
    "data-revel-dots",
    "data-revel-video",
    "data-revel-play",
    "data-revel-filters",
    "data-revel-filter",
    "data-revel-filter-empty",
    "data-revel-tags",
    "data-revel-consent",
    "data-revel-consent-choice",
    "data-revel-consent-open",
    "data-revel-switch",
    "data-revel-switch-text",
    "data-revel-form",
    "data-revel-form-status",
    "data-revel-hover-media",
    "data-revel-media-for",
    "data-revel-media",
    "data-revel-media-poster",
    "data-revel-current",
    "data-revel-progress",
    "data-revel-masonry",
    "data-revel-spy",
    "data-revel-slot",
    "data-revel-more",
    "data-revel-on",
    "data-revel-name",
    "data-revel-hidden",
];
