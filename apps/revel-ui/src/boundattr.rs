//! Inspecteur › Attributs : valeur reliée sans accolades. Valeur choisie dans une liste (champ du
//! contenu, propriété, information du site, nombre de contenus), texte écrit avant et après,
//! partie écrite seulement si la valeur est remplie (« Lire la vidéo : Titre », sinon « Lire la
//! vidéo ») ; propriétés de style reliées (`--dot` ← Couleur, comme une couleur de champ du CMS
//! liée à un style).

use crate::editor::Ed;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::attrbind::{Bound, StyleDecl, style_decls, write_style};
use revel_edit::{Command, Target};

/// Valeurs proposées à cet endroit : champs, propriétés, informations du site, environnement,
/// nombre de contenus de chaque collection.
fn options(ui: Ui, ed: Ed, at: &Target) -> Signal<Vec<(String, String)>> {
    let base = crate::blocks::condition_fields(ed, at);
    let cols = crate::blocks::collections(ed);
    Signal::derive(move || {
        let mut out: Vec<(String, String)> = base.map(|s| s.get()).unwrap_or_default();
        // Valeurs vraies ou fausses (premier de la liste) et variables de publication : pas des
        // valeurs écrites dans un attribut.
        out.retain(|(e, _)| !e.starts_with("loop.") && !e.starts_with("env."));
        cols.with(|cs| {
            for c in cs {
                let label = c
                    .label
                    .clone()
                    .unwrap_or_else(|| crate::site::capitalize(&c.id));
                out.push((
                    format!("collections.{}|length", c.id),
                    ui.now("bound-count", &[("collection", label)]),
                ));
            }
        });
        out
    })
}

/// Valeur reliée découpée : valeur (`video`), texte si vide (`or 'Vidéo'`), écriture
/// (`| youtube_id`).
#[derive(Clone, Default)]
struct Parts {
    head: String,
    fallback: Option<String>,
    tail: String,
}

impl Parts {
    fn of(v: &str) -> Parts {
        let v = v.trim();
        // Nombre de valeurs ou de contenus : la valeur entière.
        if v.ends_with("|length")
            || v.ends_with("| length")
            || v.contains('(') && !v.starts_with('(')
        {
            return Parts {
                head: v.to_string(),
                ..Default::default()
            };
        }
        let (h, tail) = match v.split_once('|') {
            Some((h, t)) => (h.trim(), format!(" |{t}")),
            None => (v, String::new()),
        };
        let inner = h
            .strip_prefix('(')
            .and_then(|x| x.strip_suffix(')'))
            .unwrap_or(h);
        match inner.split_once(" or ") {
            Some((a, b)) if revel_edit::canvas::literal(b.trim()).is_some() && !a.contains(' ') => {
                Parts {
                    head: a.trim().to_string(),
                    fallback: revel_edit::canvas::literal(b.trim()),
                    tail,
                }
            }
            _ => Parts {
                head: h.to_string(),
                fallback: None,
                tail,
            },
        }
    }
    fn write(&self) -> String {
        match &self.fallback {
            Some(f) if self.tail.is_empty() => {
                format!("{} or {}", self.head, revel_edit::canvas::quote(f))
            }
            Some(f) => format!(
                "({} or {}){}",
                self.head,
                revel_edit::canvas::quote(f),
                self.tail
            ),
            None => format!("{}{}", self.head, self.tail),
        }
    }
}

/// Liste des valeurs ; l'écriture et le texte si vide de la valeur actuelle sont gardés.
fn value_select(
    opts: Signal<Vec<(String, String)>>,
    current: String,
    on_change: impl Fn(String) + Send + Sync + 'static,
) -> AnyView {
    let parts = Parts::of(&current);
    let head = parts.head.clone();
    let parts = StoredValue::new(parts);
    view! {
        <div class="select">
            <select class="input" on:change=move |e| {
                let mut p = parts.get_value();
                p.head = event_target_value(&e);
                on_change(p.write())
            }>
                {move || {
                    let o = opts.get();
                    let known = o.iter().any(|(e, _)| *e == head);
                    let mut v: Vec<AnyView> = Vec::new();
                    if !known {
                        let cur = head.clone();
                        v.push(view! { <option value=cur.clone() selected=true>{cur.clone()}</option> }.into_any());
                    }
                    for (e, l) in o {
                        let sel = e == head;
                        v.push(view! { <option value=e selected=sel>{l}</option> }.into_any());
                    }
                    v
                }}
            </select>
        </div>
    }
    .into_any()
}

/// « Si vide » : texte écrit quand la valeur est vide (`label or 'Vidéo'`).
fn fallback_input(
    ui: Ui,
    value: String,
    on_change: impl Fn(String) + Send + Sync + 'static,
) -> AnyView {
    let parts = Parts::of(&value);
    let cur = parts.fallback.clone().unwrap_or_default();
    let parts = StoredValue::new(parts);
    view! {
        <label class="bound-text" title=ui.t("bound-fallback-help")>
            <span class="label">{ui.t("bound-fallback")}</span>
            <input class="input" prop:value=cur on:change=move |e| {
                let v = event_target_value(&e);
                let mut p = parts.get_value();
                p.fallback = (!v.is_empty()).then_some(v);
                on_change(p.write())
            }/>
        </label>
    }
    .into_any()
}

fn set_attr(ui: Ui, ed: Ed, at: &Target, name: &str, value: Option<String>) {
    ed.apply(
        ui,
        Command::SetAttr {
            at: at.clone(),
            name: name.to_string(),
            value,
        },
        false,
    );
}

/// Champ de texte court validé à la sortie.
fn text_input(
    label: String,
    value: String,
    on_commit: impl Fn(String) + Send + Sync + 'static,
) -> AnyView {
    view! {
        <label class="bound-text">
            <span class="label">{label}</span>
            <input class="input input--mono" prop:value=value on:change=move |e| on_commit(event_target_value(&e))/>
        </label>
    }
    .into_any()
}

/// Valeur reliée d'un attribut (`None` : texte écrit ou autre forme, modifiée comme texte).
pub fn attr_editor(ui: Ui, ed: Ed, at: Target, name: String, value: String) -> Option<AnyView> {
    if name == "style" {
        return style_editor(ui, ed, at, value);
    }
    if name == "class" {
        return None;
    }
    let b = Bound::parse(&value)?;
    let opts = options(ui, ed, &at);
    let at = StoredValue::new(at);
    let name = StoredValue::new(name);
    let cur = StoredValue::new(b.clone());
    let save = move |f: &dyn Fn(&mut Bound)| {
        let mut b = cur.get_value();
        f(&mut b);
        cur.set_value(b.clone());
        set_attr(ui, ed, &at.get_value(), &name.get_value(), Some(b.write()));
    };
    let only_if = b.only_if;
    Some(
        view! {
            <div class="bound-attr">
                <label class="field">
                    <span class="label">{ui.t("bound-value")}</span>
                    {value_select(opts, b.value.clone(), move |v| save(&|b| b.value = v.clone()))}
                </label>
                {fallback_input(ui, b.value.clone(), move |v| save(&|b| b.value = v.clone()))}
                <div class="bound-affix">
                    {text_input(ui.now("bound-before", &[]), b.before.clone(), move |v| save(&|b| b.before = v.clone()))}
                    {text_input(ui.now("bound-after", &[]), b.after.clone(), move |v| save(&|b| b.after = v.clone()))}
                </div>
                <label class="view-line" title=ui.t("bound-only-if-help")>
                    <span>{ui.t("bound-only-if")}</span>
                    <span class="switch">
                        <input type="checkbox" prop:checked=only_if
                            on:change=move |e| { let on = event_target_checked(&e); save(&|b| b.only_if = on) }/>
                        <span class="switch-track"></span>
                    </span>
                </label>
                {only_if.then(|| view! {
                    <div class="bound-affix">
                        {text_input(ui.now("bound-fixed", &[]), b.fixed.clone(), move |v| save(&|b| b.fixed = v.clone()))}
                        {text_input(ui.now("bound-tail", &[]), b.tail.clone(), move |v| save(&|b| b.tail = v.clone()))}
                    </div>
                })}
            </div>
        }
        .into_any(),
    )
}

/// Relie un attribut écrit à la première valeur proposée (le texte écrit reste devant).
pub fn link_button(ui: Ui, ed: Ed, at: Target, name: String, value: String) -> AnyView {
    let opts = options(ui, ed, &at);
    let at = StoredValue::new(at);
    let name = StoredValue::new(name);
    let value = StoredValue::new(value);
    view! {
        <button class="icon-btn" title=ui.t("bound-link") aria-label=ui.t("bound-link")
            disabled=move || opts.with(|o| o.is_empty())
            on:click=move |_| {
                let Some((first, _)) = opts.get_untracked().into_iter().next() else { return };
                let name = name.get_value();
                let v = if name == "style" {
                    let mut decls = style_decls(&value.get_value()).unwrap_or_default();
                    decls.push(StyleDecl {
                        prop: "--valeur".into(),
                        text: None,
                        bound: Some(Bound { value: first, ..Default::default() }),
                    });
                    write_style(&decls)
                } else {
                    Bound { before: value.get_value(), value: first, ..Default::default() }.write()
                };
                set_attr(ui, ed, &at.get_value(), &name, Some(v));
            }>{icons::link()}</button>
    }
    .into_any()
}

/// Délier : texte écrit à la place (le texte montré quand la valeur est vide, sinon le texte
/// écrit autour).
pub fn unlink_button(ui: Ui, ed: Ed, at: Target, name: String, value: String) -> Option<AnyView> {
    if name == "style" || name == "class" {
        return None;
    }
    let b = Bound::parse(&value)?;
    let text = b
        .when_empty()
        .unwrap_or_else(|| format!("{}{}", b.before, b.after));
    let at = StoredValue::new(at);
    let name = StoredValue::new(name);
    let text = StoredValue::new(text);
    Some(
        view! {
            <button class="icon-btn" title=ui.t("bound-unlink") aria-label=ui.t("bound-unlink")
                on:click=move |_| set_attr(ui, ed, &at.get_value(), &name.get_value(), Some(text.get_value()))>
                {icons::unlink()}
            </button>
        }
        .into_any(),
    )
}

/// Attribut `style` avec une propriété reliée : une ligne par propriété (nom, valeur écrite ou
/// reliée).
fn style_editor(ui: Ui, ed: Ed, at: Target, value: String) -> Option<AnyView> {
    let decls = style_decls(&value)?;
    if !decls.iter().any(|d| d.bound.is_some()) {
        return None;
    }
    let opts = options(ui, ed, &at);
    let at = StoredValue::new(at);
    let cur = StoredValue::new(decls.clone());
    let save = move |f: &dyn Fn(&mut Vec<StyleDecl>)| {
        let mut d = cur.get_value();
        f(&mut d);
        cur.set_value(d.clone());
        let v = write_style(&d);
        set_attr(
            ui,
            ed,
            &at.get_value(),
            "style",
            (!v.is_empty()).then_some(v),
        );
    };
    Some(
        view! {
            <div class="bound-attr bound-style">
                {decls.into_iter().enumerate().map(|(i, d)| {
                    let bound = d.bound.clone();
                    let value_part = match bound {
                        Some(b) => value_select(opts, b.value.clone(), move |v| save(&|ds| {
                            if let Some(b) = ds.get_mut(i).and_then(|d| d.bound.as_mut()) { b.value = v.clone(); }
                        })),
                        None => view! {
                            <input class="input input--mono" prop:value=d.text.clone().unwrap_or_default()
                                on:change=move |e| { let v = event_target_value(&e); save(&|ds| {
                                    if let Some(d) = ds.get_mut(i) { d.text = Some(v.clone()); d.bound = None; }
                                }) }/>
                        }.into_any(),
                    };
                    let is_bound = d.bound.is_some();
                    // Texte si vide d'une propriété reliée (`--ratio:{{ ratio or '16 / 9' }}`).
                    let fallback = d.bound.clone().map(|b| fallback_input(ui, b.value.clone(), move |v| save(&|ds| {
                        if let Some(b) = ds.get_mut(i).and_then(|d| d.bound.as_mut()) { b.value = v.clone(); }
                    })));
                    view! {
                        <div class="bound-decl">
                            <input class="input input--mono bound-prop" prop:value=d.prop.clone() aria-label=ui.t("bound-style-prop")
                                on:change=move |e| { let v = event_target_value(&e).trim().to_string(); save(&|ds| {
                                    if let Some(d) = ds.get_mut(i) && !v.is_empty() { d.prop = v.clone(); }
                                }) }/>
                            {value_part}
                            <button class="icon-btn icon-btn--small"
                                title=ui.t(if is_bound { "bound-unlink" } else { "bound-link" })
                                aria-label=ui.t(if is_bound { "bound-unlink" } else { "bound-link" })
                                on:click=move |_| {
                                    let first = opts.get_untracked().into_iter().next().map(|(e, _)| e);
                                    save(&|ds| {
                                        if let Some(d) = ds.get_mut(i) {
                                            if d.bound.is_some() {
                                                d.bound = None;
                                                d.text = Some(String::new());
                                            } else if let Some(f) = &first {
                                                d.bound = Some(Bound { value: f.clone(), ..Default::default() });
                                                d.text = None;
                                            }
                                        }
                                    })
                                }>{if is_bound { icons::unlink() } else { icons::link() }}</button>
                            <button class="icon-btn icon-btn--small" title=ui.t("bound-style-remove") aria-label=ui.t("bound-style-remove")
                                on:click=move |_| save(&|ds| { if i < ds.len() { ds.remove(i); } })>{icons::trash()}</button>
                        </div>
                        {fallback.map(|f| view! { <div class="bound-decl-fallback">{f}</div> })}
                    }
                }).collect_view()}
                <button class="btn btn--ghost btn--small" on:click=move |_| {
                    let first = opts.get_untracked().into_iter().next().map(|(e, _)| e).unwrap_or_default();
                    save(&|ds| ds.push(StyleDecl {
                        prop: "--valeur".into(),
                        text: None,
                        bound: Some(Bound { value: first.clone(), ..Default::default() }),
                    }))
                }>{icons::plus()}{ui.t("bound-style-add")}</button>
            </div>
        }
        .into_any(),
    )
}
