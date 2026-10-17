//! Panneau de styles (étapes 2.6, 2.8 ; D-18, D-20) : styles détenus par Revel, modèle Webflow,
//! contrôles visuels et noms simples.
//!
//! On règle une **classe** de l'élément (ou leur combinaison), éventuellement un **état**
//! (survol…), au **point de rupture** affiché. La règle correspondante est modifiée ; si elle
//! n'existe pas, la première valeur la crée à sa place dans la cascade. Chaque valeur indique son
//! origine (réglée ici, héritée d'un écran plus large ou de l'état de base) ; la valeur effective
//! de l'élément (calculée dans le canevas) sert d'indication.

mod background;
mod color;
mod controls;
mod ctx;
mod fx;
mod layout;
mod sections;

pub use ctx::watched;
pub(crate) use ctx::{bp_name, find_rule, style_command};

use crate::editor::Ed;
use crate::{Ui, icons};
use ctx::Sx;
use layout::{ChildSection, LayoutSection};
use leptos::ev;
use leptos::prelude::*;
use revel_edit::Command;
use revel_model::Node;
use sections::*;
use wasm_bindgen::JsCast;

/// États proposés (pseudo-classes ajoutées au sélecteur).
const STATES: &[(&str, &str)] = &[
    ("", "styles-state-none"),
    (":hover", "styles-state-hover"),
    (":active", "styles-state-active"),
    (":focus-visible", "styles-state-focus"),
];

/// Classes littérales de l'élément ou de l'image (pas d'expression). Les champs d'une image
/// sont des expressions : sa classe est un texte entre guillemets (`"carte__image"`).
pub(crate) fn classes_of(n: &Node) -> Vec<String> {
    let value = match n {
        Node::Element { attrs, .. } => attrs
            .iter()
            .find(|a| a.name == "class")
            .and_then(|a| a.value.clone()),
        Node::Image(img) => img.class.as_deref().and_then(revel_edit::canvas::literal),
        _ => None,
    };
    value
        .filter(|v| !v.contains("{{") && !v.contains("{%"))
        .map(|v| v.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

/// Commande qui remplace les classes d'un élément ou d'une image (`None` : plus de classe).
pub(crate) fn class_command(at: revel_edit::Target, n: &Node, list: &[String]) -> Option<Command> {
    let value = (!list.is_empty()).then(|| list.join(" "));
    match n {
        Node::Element { .. } => Some(Command::SetAttr {
            at,
            name: "class".into(),
            value,
        }),
        Node::Image(img) => {
            let mut img = img.clone();
            img.class = value.map(|v| revel_edit::canvas::quote(&v));
            Some(Command::SetImage { at, image: img })
        }
        _ => None,
    }
}

/// Classe à créer pour un élément qui n'en a pas (comme Webflow : d'après sa nature,
/// numérotée si elle existe déjà dans la feuille de style ou la page).
pub(crate) fn new_class(d: &revel_model::Site, n: &Node) -> String {
    let base = match n {
        Node::Image(_) => "image",
        Node::Element { tag, .. } => match tag.to_ascii_lowercase().as_str() {
            "div" => "div-block",
            "img" | "picture" => "image",
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
            "p" => "paragraph",
            "a" => "link",
            "ul" | "ol" => "list",
            "li" => "list-item",
            "span" => "text-span",
            "strong" | "b" => "bold-text",
            "em" | "i" => "italic-text",
            "blockquote" => "block-quote",
            "main" => "main",
            "article" => "article",
            "aside" => "aside",
            "label" => "label",
            "section" => "section",
            "button" => "button",
            "nav" => "nav",
            "header" => "header",
            "footer" => "footer",
            "figure" => "figure",
            "video" => "video",
            "form" => "form",
            "input" => "input",
            _ => "block",
        },
        _ => "block",
    };
    let mut taken = std::collections::HashSet::new();
    for r in d.style_rules() {
        for part in r
            .selector
            .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_' || c == '.'))
        {
            for c in part.split('.').skip(1) {
                taken.insert(c.to_string());
            }
        }
    }
    for (_, _, body) in d.owners() {
        revel_model::ids::walk(body, &mut |n| {
            for c in classes_of(n) {
                taken.insert(c);
            }
        });
    }
    (1..)
        .map(|i| {
            if i == 1 {
                base.to_string()
            } else {
                format!("{base}-{i}")
            }
        })
        .find(|c| !taken.contains(c))
        .unwrap_or_else(|| base.to_string())
}

/// Balises qu'on peut régler toutes ensemble (sélecteur de balise, comme « All Paragraphs »
/// et « Body (All Pages) » de Webflow), avec le libellé du choix.
const TAGS: &[(&str, &str)] = &[
    ("body", "styles-tag-body"),
    ("p", "styles-tag-p"),
    ("h1", "styles-tag-h1"),
    ("h2", "styles-tag-h2"),
    ("h3", "styles-tag-h3"),
    ("h4", "styles-tag-h4"),
    ("h5", "styles-tag-h5"),
    ("h6", "styles-tag-h6"),
    ("a", "styles-tag-a"),
    ("img", "styles-tag-img"),
    ("ul", "styles-tag-ul"),
    ("ol", "styles-tag-ol"),
    ("li", "styles-tag-li"),
    ("blockquote", "styles-tag-blockquote"),
    ("strong", "styles-tag-strong"),
    ("em", "styles-tag-em"),
    ("figure", "styles-tag-figure"),
    ("section", "styles-tag-section"),
    ("button", "styles-tag-button"),
    ("video", "styles-tag-video"),
];

/// Sélecteur de balise proposé pour l'élément (`p`, `body`…) et la clé de son libellé.
pub(crate) fn tag_selector(n: &Node) -> Option<(&'static str, &'static str)> {
    let tag = match n {
        Node::Element { tag, .. } => tag.to_ascii_lowercase(),
        Node::Image(_) => "img".into(),
        _ => return None,
    };
    TAGS.iter().find(|(t, _)| *t == tag).copied()
}

/// Sélecteurs proposés : chaque classe, puis la combinaison de toutes (classe combinée).
pub(crate) fn candidates(classes: &[String]) -> Vec<String> {
    let mut v: Vec<String> = classes.iter().map(|c| format!(".{c}")).collect();
    if classes.len() > 1 {
        v.push(classes.iter().map(|c| format!(".{c}")).collect());
    }
    v
}

#[component]
pub fn Styles() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let sx = Sx::new(ui, ed);
    let selector = sx.selector;
    let state = sx.state;

    let node = Memo::new(move |_| {
        let t = ed.selected_target()?;
        ed.doc
            .with(|d| revel_edit::address::node(d.as_ref()?, &t).ok().cloned())
    });
    let classes = Memo::new(move |_| node.get().map(|n| classes_of(&n)).unwrap_or_default());
    let is_use = Memo::new(move |_| matches!(node.get(), Some(Node::Use { .. })));
    // Choix « toutes les balises » : proposé pour un élément sans classe.
    let tag_sel = Memo::new(move |_| {
        node.get()
            .filter(|n| classes_of(n).is_empty())
            .and_then(|n| tag_selector(&n))
    });
    // Classe par défaut : la combinaison (gardée tant qu'elle reste proposée). Sans classe : le
    // corps de page se règle pour toutes les pages ; un autre élément reçoit une classe à la
    // première valeur réglée (comme Webflow), sauf choix « toutes les balises ».
    Effect::new(move |_| {
        let c = candidates(&classes.get());
        let tag = tag_sel.get();
        let cur = selector.get_untracked();
        if c.contains(&cur) || (cur.is_empty() && c.is_empty()) {
            if cur.is_empty() && tag.is_some_and(|(t, _)| t == "body") {
                selector.set("body".into());
            }
            return;
        }
        if tag.is_some_and(|(t, _)| t == cur) {
            return;
        }
        // Plusieurs classes : leur combinaison, comme Webflow (la classe seule se choisit dans
        // les pastilles).
        selector.set(match (c.last(), tag) {
            (Some(all), _) => all.clone(),
            (None, Some(("body", _))) => "body".into(),
            _ => String::new(),
        });
    });
    // Nombre d'éléments de la page qui portent la classe (demandé au canevas).
    Effect::new(move |_| {
        let sel = selector.get();
        let _ = ed.load.get();
        ed.count.set(None);
        if !sel.is_empty() {
            ed.post(serde_json::json!({ "type": "count", "selector": sel }));
        }
    });
    // Nom court du point de rupture (« Ordinateur », « ≤ 900 px »), le nom complet en info-bulle.
    let bp_label = move || {
        let id = ed.breakpoint.get();
        ed.doc.with(|d| {
            let b = d.as_ref()?.breakpoints.iter().find(|b| b.id == id)?.clone();
            Some(match (b.max_width, b.min_width) {
                (Some(w), _) => format!("≤ {w} px"),
                (_, Some(w)) => format!("≥ {w} px"),
                _ => ui.tx("editor-bp-desktop-short", &[]),
            })
        })
    };
    let bp_full = move || {
        let id = ed.breakpoint.get();
        ed.doc.with(|d| {
            let b = d.as_ref()?.breakpoints.iter().find(|b| b.id == id)?.clone();
            Some(bp_name(ui, &b))
        })
    };

    // Ce qui est affiché : rien de sélectionné (0), pas un élément (1), pas de feuille de style (2),
    // panneau (3). Le panneau reste en place d'un élément à l'autre (sections ouvertes gardées).
    let kind = Memo::new(move |_| match node.get() {
        None => 0u8,
        Some(n) if !matches!(n, Node::Element { .. } | Node::Image(_)) => 1,
        Some(_) => 3,
    });

    // Menus : fermés par un clic ailleurs ou par Échap.
    let on_down = window_event_listener(ev::pointerdown, move |e: web_sys::PointerEvent| {
        if sx.menu.get_untracked().is_none() {
            return;
        }
        let inside = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest(".pop, .pop-trigger").ok().flatten())
            .is_some();
        if !inside {
            sx.menu.set(None);
        }
    });
    let on_key = window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" && sx.menu.get_untracked().is_some() {
            sx.menu.set(None);
        }
    });
    let on_blur = window_event_listener(ev::blur, move |_| sx.menu.set(None));
    on_cleanup(move || {
        on_down.remove();
        on_key.remove();
        on_blur.remove();
        sx.clear_preview();
    });

    // Ajout d'une classe à l'élément.
    let add_class = move |name: String| {
        let name = name.trim().trim_start_matches('.').to_string();
        if name.is_empty() || name.contains(char::is_whitespace) {
            return;
        }
        let (Some(at), Some(n)) = (ed.selected_target(), node.get_untracked()) else {
            return;
        };
        let mut list = classes.get_untracked();
        if !list.contains(&name) {
            list.push(name.clone());
        }
        // Classe ajoutée à côté d'autres : la combinaison est réglée (classe combinée de
        // Webflow), pour ne pas modifier tous les éléments qui portent la nouvelle classe seule.
        selector.set(
            candidates(&list)
                .pop()
                .unwrap_or_else(|| format!(".{name}")),
        );
        if let Some(cmd) = class_command(at, &n, &list) {
            ed.apply(ui, cmd, false);
        }
    };

    // Retrait d'une classe de l'élément.
    let remove_class = move |name: String| {
        let (Some(at), Some(n)) = (ed.selected_target(), node.get_untracked()) else {
            return;
        };
        let list: Vec<String> = classes
            .get_untracked()
            .into_iter()
            .filter(|c| *c != name)
            .collect();
        if let Some(cmd) = class_command(at, &n, &list) {
            ed.apply(ui, cmd, false);
        }
    };

    view! {
        <div class="styles">
            // Contenu de l'élément (lien, image, composant, condition, liste) en tête, avec ses
            // textes (ceux d'une utilisation de composant après son nom) ; ses paramètres avancés
            // en bas (onglet Réglages fondu dans Styles).
            {move || if is_use.get() {
                view! { <crate::inspector::ContentSettings/><crate::textfield::SelectedTexts/> }.into_any()
            } else {
                view! { <crate::textfield::SelectedTexts/><crate::inspector::ContentSettings/> }.into_any()
            }}
            {move || match kind.get() {
                0 => view! { <p class="muted inspector-empty">{ui.t("inspector-none")}</p> }.into_any(),
                1 => ().into_any(),
                _ => view! {
                    <div class="style-target">
                        // Classes de l'élément (pastilles, comme le champ de sélecteur de Webflow) :
                        // clic = classe réglée ; × = retirée de l'élément ; champ = nouvelle classe.
                        <div class="selector-box" role="listbox" aria-label=ui.t("styles-selector")>
                            {move || {
                                let list = classes.get();
                                let mut chips: Vec<(String, String, Option<String>)> = list.iter()
                                    .map(|c| (format!(".{c}"), c.clone(), Some(c.clone()))).collect();
                                if list.len() > 1 {
                                    chips.push((candidates(&list).pop().unwrap_or_default(), list.join(" + "), None));
                                }
                                chips.into_iter().map(|(sel, label, class)| {
                                    let s2 = sel.clone();
                                    let s3 = sel.clone();
                                    let combo = class.is_none();
                                    let title = if combo { ui.tx("styles-combo", &[]) } else { sel.clone() };
                                    view! {
                                        <span class="sel-chip" class:is-on=move || selector.get() == s2 class:is-combo=combo
                                            role="option" aria-selected=move || (selector.get() == s3).to_string()
                                            title=title>
                                            <button class="sel-chip-name" on:click=move |_| selector.set(sel.clone())>{label}</button>
                                            {class.map(|c| view! {
                                                <button class="sel-chip-x" title=ui.t("styles-class-remove") aria-label=ui.t("styles-class-remove")
                                                    on:click=move |_| remove_class(c.clone())>{icons::close()}</button>
                                            })}
                                        </span>
                                    }
                                }).collect_view()
                            }}
                            // Toutes les balises de ce type (élément sans classe).
                            {move || tag_sel.get().map(|(t, key)| view! {
                                <span class="sel-chip is-tag" class:is-on=move || selector.get() == t
                                    role="option" aria-selected=move || (selector.get() == t).to_string()
                                    title=ui.t("styles-tag-help")>
                                    <button class="sel-chip-name" on:click=move |_| {
                                        let on = selector.get_untracked() == t;
                                        selector.set(if on { String::new() } else { t.to_string() });
                                    }>{icons::tag()}{ui.t(key)}</button>
                                </span>
                            })}
                            <input class="selector-add" placeholder=move || if classes.get().is_empty() { ui.tx("styles-class-first", &[]) } else { ui.tx("styles-class-new", &[]) }
                                aria-label=ui.t("styles-class-new")
                                on:change=move |e| {
                                    add_class(event_target_value(&e));
                                    if let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) {
                                        t.set_value("");
                                    }
                                }/>
                        </div>
                        <div class="target-meta">
                            <div class="select select--state" class:is-active=move || !state.get().is_empty() title=ui.t("styles-state")>
                                <select class="input" aria-label=ui.t("styles-state") on:change=move |e| state.set(event_target_value(&e))>
                                    {STATES.iter().map(|(s, k)| view! {
                                        <option value=*s selected=move || state.get() == *s>{ui.t(k)}</option>
                                    }).collect_view()}
                                </select>
                            </div>
                            <span class="target-where muted" title=move || format!("{} — {}", sx.full_selector(), bp_full().unwrap_or_default())>
                                {move || bp_label().unwrap_or_default()}
                                {move || ed.count.get().filter(|(s, _)| *s == selector.get()).map(|(_, n)| format!(" · {}", ui.tx("styles-count", &[("count", n.to_string())])))}
                            </span>
                            // Variante du composant ouvert : styles écrits pour elle.
                            {move || sx.scope.get().map(|(_, _, _, name)| view! {
                                <span class="chip chip--variant" title=ui.tx("variant-scope", &[("name", name.clone())])>{icons::component()}{name.clone()}</span>
                            })}
                            {move || (sx.rule.get().is_none() && !selector.get().is_empty()).then(|| view! {
                                <span class="chip chip--new" title=ui.t("styles-new-rule")>{ui.t("styles-new")}</span>
                            })}
                        </div>
                    </div>
                    // Sans classe ni balise choisie : la première valeur crée la classe.
                    <LayoutSection sx/>
                    <ChildSection sx/>
                    <SpacingSection sx/>
                    <SizeSection sx/>
                    <PositionSection sx/>
                    <TextSection sx/>
                    <BackgroundSection sx/>
                    <BorderSection sx/>
                    <EffectsSection sx/>
                    <CodeSection sx/>
                    <crate::inspector::AdvancedSettings/>
                }.into_any(),
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classe_d_une_image_ecrite_en_expression() {
        let img = Node::Image(revel_model::ImageNode {
            id: Some("i".into()),
            src: "\"/media/a.jpg\"".into(),
            class: Some("\"carte__image grande\"".into()),
            ..Default::default()
        });
        assert_eq!(classes_of(&img), vec!["carte__image", "grande"]);
        let at = revel_edit::Target {
            owner: revel_edit::Owner::Page("p".into()),
            node: "i".into(),
        };
        let Some(Command::SetImage { image, .. }) =
            class_command(at, &img, &["hero__img".to_string()])
        else {
            panic!("commande attendue");
        };
        assert_eq!(image.class.as_deref(), Some("\"hero__img\""));
        // Classe calculée : pas de pastille.
        let dynamic = Node::Image(revel_model::ImageNode {
            class: Some("cls".into()),
            ..Default::default()
        });
        assert!(classes_of(&dynamic).is_empty());
    }
}
