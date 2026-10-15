//! Réglages de l'élément sélectionné (étape 2.5) : balise, identifiant, classes, lien, image,
//! logique (condition, boucle), attributs. Chaque changement validé (sortie du champ, Entrée) est une
//! commande d'édition.

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
                <span class="label">{label}</span>
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

/// Panneau « Réglages » de l'élément sélectionné.
#[component]
pub fn ElementSettings() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    // Nœud sélectionné et son adresse.
    let current = Memo::new(move |_| {
        let target = ed.selected_target()?;
        let node = ed.doc.with(|d| {
            revel_edit::address::node(d.as_ref()?, &target)
                .ok()
                .cloned()
        })?;
        Some((target, node))
    });
    let apply = move |cmd: Command| ed.apply(ui, cmd, false);

    // Élément d'un composant ouvert : propriétés du composant, liaisons de l'élément.
    let in_component = move || {
        current.get().and_then(|(at, node)| {
            let c = crate::components::owner_component(&at)?;
            let bind = (!matches!(node, Node::Use { .. }))
                .then(|| crate::components::bindings(ui, ed, c.clone(), at.clone()));
            Some(view! {
                {bind}
                {crate::components::props_section(ui, ed, c)}
            })
        })
    };
    view! {
        <div class="element-settings panel-pad">
                {move || current.get().and_then(|(at, node)| crate::blocks::extra_settings(ui, ed, at, node))}
                {move || match current.get() {
                    None => view! { <p class="muted inspector-empty">{ui.t("inspector-none")}</p> }.into_any(),
                    Some((at, node)) => match node {
                        Node::Element { tag, attrs, .. } => element(ui, ed, apply, at, tag, attrs).into_any(),
                        Node::Image(img) => image(ui, ed, apply, at, img).into_any(),
                        Node::If { cond, .. } => {
                            let at2 = at.clone();
                            view! {
                                <div class="stack">
                                    <Field label=ui.t("inspector-condition") value=cond mono=true
                                        on_commit=move |v| apply(Command::SetLogic { at: at2.clone(), cond: Some(v), expr: None, item: None })/>
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
                        _ => view! { <p class="muted">{ui.t("inspector-none")}</p> }.into_any(),
                    },
                }}
                {in_component}
        </div>
    }
}

fn element(
    ui: Ui,
    ed: Ed,
    apply: impl Fn(Command) + Copy + Send + Sync + 'static,
    at: Target,
    tag: String,
    attrs: Vec<Attr>,
) -> impl IntoView {
    let set = {
        let at = at.clone();
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
    };
    let class_value = attr(&attrs, "class")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let dynamic_class = class_value.contains("{{") || class_value.contains("{%");
    let classes: Vec<String> = class_value.split_whitespace().map(str::to_string).collect();
    let id_value = attr(&attrs, "id")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let is_link = tag.eq_ignore_ascii_case("a");
    let href = attr(&attrs, "href")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let blank = attr(&attrs, "target").and_then(|a| a.value.as_deref()) == Some("_blank");
    let tag_at = at.clone();
    let tag_now = tag.clone();
    let others: Vec<Attr> = attrs
        .iter()
        .filter(|a| !matches!(a.name.as_str(), "id" | "class" | "href" | "target"))
        .cloned()
        .collect();
    let set_id = set.clone();
    let set_href = set.clone();
    let set_class_raw = set.clone();
    let class_list = classes.clone();
    let set_classes = {
        let set = set.clone();
        move |list: Vec<String>| set("class", list.join(" "))
    };
    let add_class = {
        let list = class_list.clone();
        let set_classes = set_classes.clone();
        move |v: String| {
            let mut l = list.clone();
            for c in v.split_whitespace() {
                if !l.iter().any(|x| x == c) {
                    l.push(c.to_string());
                }
            }
            set_classes(l);
        }
    };
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
    let new_attr_at = at.clone();
    let n_attrs = others.len();
    let lower = tag.to_ascii_lowercase();
    let is_img = lower == "img";
    let media_tag = matches!(lower.as_str(), "img" | "video" | "audio" | "source");
    let alt_empty = attr(&attrs, "alt")
        .and_then(|a| a.value.as_deref())
        .is_none_or(|v| v.trim().is_empty());
    view! {
        <div class="stack">
            <label class="field">
                <span class="label">{ui.t("inspector-tag")}</span>
                <input class="input input--mono" list="rv-tags" prop:value=tag.clone()
                    on:change=move |e| {
                        let v = event_target_value(&e).trim().to_ascii_lowercase();
                        if !v.is_empty() && v != tag_now {
                            apply(Command::SetTag { at: tag_at.clone(), tag: v });
                        }
                    }/>
                <datalist id="rv-tags">{TAGS.iter().map(|t| view! { <option value=*t></option> }).collect_view()}</datalist>
            </label>
            <Field label=ui.t("inspector-id") value=id_value mono=true on_commit=move |v| set_id("id", v)/>
            <div class="field">
                <span class="label">{ui.t("inspector-classes")}</span>
                {if dynamic_class {
                    // Classes calculées (expression) : modifiées comme du texte.
                    view! {
                        <input class="input input--mono" prop:value=class_value.clone()
                            on:change=move |e| set_class_raw("class", event_target_value(&e))/>
                    }.into_any()
                } else {
                    let add = add_class.clone();
                    view! {
                        <div class="classes">
                            {classes.iter().map(|c| {
                                let c2 = c.clone();
                                let list = class_list.clone();
                                let set_classes = set_classes.clone();
                                view! {
                                    <span class="chip chip--class">{c.clone()}
                                        <button class="chip-x" title=ui.t("inspector-class-remove") aria-label=ui.t("inspector-class-remove")
                                            on:click=move |_| set_classes(list.iter().filter(|x| **x != c2).cloned().collect())>{icons::close()}</button>
                                    </span>
                                }
                            }).collect_view()}
                            <input class="input input--mono classes-add" placeholder=ui.t("inspector-class-add")
                                on:change=move |e| {
                                    let v = event_target_value(&e);
                                    if !v.trim().is_empty() {
                                        add(v);
                                    }
                                }/>
                        </div>
                    }.into_any()
                }}
            </div>
            {is_link.then(|| view! {
                <div class="insp-group">
                    <p class="panel-title">{ui.t("inspector-link")}</p>
                    <Field label=ui.t("inspector-href") value=href.clone() placeholder="https://… /page/ #ancre"
                        on_commit=move |v| set_href("href", v)/>
                    <label class="row-switch">
                        <span class="switch">
                            <input type="checkbox" prop:checked=blank on:change=move |e| toggle_blank(event_target_checked(&e))/>
                            <span class="switch-track"></span>
                        </span>
                        <span>{ui.t("inspector-new-tab")}</span>
                    </label>
                </div>
            })}
            <details class="insp-group insp-attrs" open=n_attrs <= 4>
                <summary class="panel-title">{ui.t("inspector-attributes")}<span class="list-count">{n_attrs}</span></summary>
                {others.into_iter().map(|a| {
                    let name = a.name.clone();
                    let name_rm = a.name.clone();
                    let at_set = at.clone();
                    let at_rm = at.clone();
                    let conditional = a.when.clone();
                    let input = view! {
                        <input class="input input--mono" prop:value=a.value.clone().unwrap_or_default()
                            placeholder=ui.t("inspector-attr-boolean")
                            on:change=move |e| {
                                let v = event_target_value(&e);
                                apply(Command::SetAttr { at: at_set.clone(), name: name.clone(), value: (!v.is_empty()).then_some(v) });
                            }/>
                    };
                    // Source d'un média : choix parmi les médias du site (pas pour une valeur calculée).
                    let value_cell = if media_tag && a.name == "src" {
                        let dynamic = a.value.as_deref().is_some_and(|v| v.contains("{{") || v.contains("{%"));
                        let at_pick = at.clone();
                        let key = if is_img { "picker-choose-image" } else { "picker-choose-media" };
                        view! {
                            <span class="field-line">
                                {input}
                                <button class="icon-btn" title=ui.t(key) aria-label=ui.t(key) disabled=dynamic
                                    on:click=move |_| {
                                        let at = at_pick.clone();
                                        crate::library::pick_media(ui, is_img, crate::library::media_shelf(ed), move |url| {
                                            let mut commands = vec![Command::SetAttr { at: at.clone(), name: "src".into(), value: Some(url.clone()) }];
                                            if is_img && alt_empty
                                                && let Some(alt) = ed.doc.with_untracked(|d| d.as_ref().and_then(|d| d.library.alt.get(&url).cloned()))
                                            {
                                                commands.push(Command::SetAttr { at: at.clone(), name: "alt".into(), value: Some(alt) });
                                            }
                                            apply(if commands.len() == 1 { commands.remove(0) } else { Command::Batch { commands } });
                                        });
                                    }>{icons::image()}</button>
                            </span>
                        }.into_any()
                    } else {
                        input.into_any()
                    };
                    view! {
                        <div class="attr-row">
                            <span class="attr-name" title=conditional.clone().map(|w| ui.tx("inspector-attr-when", &[("cond", w)]))>{a.name.clone()}{conditional.is_some().then_some(" ?")}</span>
                            {value_cell}
                            <button class="icon-btn" title=ui.t("inspector-attr-remove") aria-label=ui.t("inspector-attr-remove")
                                on:click=move |_| apply(Command::RemoveAttr { at: at_rm.clone(), name: name_rm.clone() })>{icons::trash()}</button>
                        </div>
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
            </details>
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
            <Field label=Signal::derive(move || ui.tx(if alt_expr { "inspector-alt-expr" } else { "inspector-alt" }, &[]))
                value=alt mono=alt_expr
                on_commit=move |v| {
                    let mut i = img2.clone();
                    i.alt = Some(if alt_expr { v } else { quote(&v) });
                    apply(Command::SetImage { at: at2.clone(), image: i });
                }/>
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
