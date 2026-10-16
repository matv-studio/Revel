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
        <div class="content-settings">
            {move || current.get().and_then(|(at, node)| crate::blocks::extra_settings(ui, ed, at, node))}
            {move || current.get().map(|(at, node)| match node {
                Node::Element { tag, attrs, .. } => element_content(ui, ed, apply, at, tag, attrs).into_any(),
                Node::Image(img) => image(ui, ed, apply, at, img).into_any(),
                Node::If { cond, .. } => view! {
                    <div class="stack">
                        <Field label=ui.t("inspector-condition") value=cond mono=true
                            on_commit=move |v| apply(Command::SetLogic { at: at.clone(), cond: Some(v), expr: None, item: None })/>
                    </div>
                }.into_any(),
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

/// Contenu d'un élément : lien (adresse, nouvel onglet), source d'un média et texte alternatif.
fn element_content(
    ui: Ui,
    ed: Ed,
    apply: impl Fn(Command) + Copy + Send + Sync + 'static,
    at: Target,
    tag: String,
    attrs: Vec<Attr>,
) -> impl IntoView {
    let set = setter(apply, at.clone());
    let lower = tag.to_ascii_lowercase();
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
    let media = matches!(lower.as_str(), "img" | "video" | "audio" | "source");
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
    view! {
        {is_link.then(|| view! {
            <div class="stack insp-content">
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
        {media.then(|| view! {
            <div class="stack insp-content">
                <Field label=ui.t("inspector-src") value=src.clone() mono=dynamic_src
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
    let media = matches!(lower.as_str(), "img" | "video" | "audio" | "source");
    let is_link = lower == "a";
    let others: Vec<Attr> = attrs
        .iter()
        .filter(|a| !matches!(a.name.as_str(), "id" | "class"))
        // Lien et média : réglés en tête de l'onglet.
        .filter(|a| !(is_link && matches!(a.name.as_str(), "href" | "target")))
        .filter(|a| !(media && matches!(a.name.as_str(), "src" | "alt")))
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
                    view! {
                        <div class="attr-row">
                            <span class="attr-name" title=conditional.clone().map(|w| ui.tx("inspector-attr-when", &[("cond", w)]))>{a.name.clone()}{conditional.is_some().then_some(" ?")}</span>
                            {input}
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
