//! Variables : affectations `{% set nom = expression %}` de la page affichée, de ses gabarits et
//! des composants qu'elle utilise, rangées par propriétaire, avec leur valeur calculée au rendu
//! du canevas. L'expression se modifie sur place (`Replace` du nœud) ; sa syntaxe est vérifiée par
//! le cœur avant l'application.

use crate::editor::Ed;
use crate::layers::{self, Kind, VarRow};
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::canvas::{Shown, VarValue};
use revel_edit::{Command, Owner, Parent};

/// Texte affiché pour une valeur ; `true` : texte d'état (atténué), pas une valeur.
fn shown_text(ui: Ui, v: Option<&VarValue>) -> (String, bool) {
    let Some(v) = v else {
        return ("—".into(), true);
    };
    match &v.value {
        Shown::Empty => (ui.now("variables-value-empty", &[]), true),
        Shown::Text { text, html: false } | Shown::Literal { text } => (text.clone(), false),
        Shown::Text { text, html: true } => {
            let plain = strip_tags(text);
            if plain.trim().is_empty() {
                ("HTML".into(), true)
            } else {
                (plain, false)
            }
        }
        Shown::List { len } => (
            ui.now("variables-value-list", &[("count", len.to_string())]),
            true,
        ),
        Shown::Object { label: Some(l), .. } => (format!("« {l} »"), false),
        Shown::Object { len, label: None } => (
            ui.now("variables-value-object", &[("count", len.to_string())]),
            true,
        ),
    }
}

/// Texte d'un fragment HTML, balises retirées, espaces réduits.
fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in html.chars() {
        match c {
            '<' => tag = true,
            '>' if tag => {
                tag = false;
                out.push(' ');
            }
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Nom court d'un propriétaire (`layouts/base` -> `base`).
fn short(id: &str) -> String {
    id.rsplit('/').next().unwrap_or(id).to_string()
}

/// Panneau Variables.
#[component]
pub fn Variables() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let q = RwSignal::new(String::new());
    // Affectation dont l'expression est ouverte ; groupes repliés.
    let editing = RwSignal::new(None::<(Parent, usize)>);
    let folded = RwSignal::new(Vec::<Owner>::new());

    let all = Memo::new(move |_| {
        let url = ed.page.get();
        ed.doc.with(|d| {
            let Some(d) = d else { return Vec::new() };
            ed.pages.with(|ps| {
                ps.iter()
                    .find(|p| p.url == url)
                    .map(|p| layers::variables(d, p))
                    .unwrap_or_default()
            })
        })
    });
    let groups = Memo::new(move |_| {
        let q = q.get().trim().to_lowercase();
        all.with(|vs| {
            let mut out: Vec<(Owner, Vec<VarRow>)> = Vec::new();
            for v in vs.iter().filter(|v| {
                q.is_empty()
                    || v.name.to_lowercase().contains(&q)
                    || v.expr
                        .as_deref()
                        .is_some_and(|e| e.to_lowercase().contains(&q))
            }) {
                match out.iter_mut().find(|(o, _)| *o == v.owner) {
                    Some(g) => g.1.push(v.clone()),
                    None => out.push((v.owner.clone(), vec![v.clone()])),
                }
            }
            out
        })
    });
    let owner_head = move |o: &Owner| -> (AnyView, String, String) {
        match o {
            Owner::Layout(id) => (
                icons::layout(),
                short(id),
                ui.now("variables-owner-layout", &[("name", id.clone())]),
            ),
            Owner::Component(id) => (
                icons::component(),
                short(id),
                ui.now("variables-owner-component", &[("name", id.clone())]),
            ),
            Owner::Page(id) => {
                let title = ed.pages.with_untracked(|ps| {
                    ps.iter()
                        .find(|p| p.page.as_deref() == Some(id.as_str()))
                        .map(|p| p.title.clone())
                        .filter(|t| !t.is_empty())
                });
                (
                    icons::page(),
                    title.unwrap_or_else(|| id.clone()),
                    ui.now("variables-owner-page", &[("name", id.clone())]),
                )
            }
        }
    };
    let commit = move |v: &VarRow, text: String| {
        let expr = text.trim().to_string();
        if expr.is_empty() || Some(expr.as_str()) == v.expr.as_deref().map(str::trim) {
            return;
        }
        ed.apply(
            ui,
            Command::Replace {
                parent: v.parent.clone(),
                index: v.index,
                node: revel_model::Node::Set {
                    name: v.name.clone(),
                    expr,
                },
            },
            false,
        );
    };
    let row = move |v: VarRow| {
        let key = (v.parent.clone(), v.index);
        let k_open = key.clone();
        let is_open = Memo::new(move |_| editing.get().as_ref() == Some(&k_open));
        let (p2, i2) = (v.parent.clone(), v.index);
        let value = Memo::new(move |_| {
            ed.vars
                .with(|xs| xs.iter().find(|x| x.parent == p2 && x.index == i2).cloned())
        });
        let editable = v.expr.is_some();
        let within = v.within.clone();
        let v = StoredValue::new(v);
        let k_toggle = key.clone();
        let toggle = move |_| {
            if !editable {
                return;
            }
            editing.update(|e| {
                *e = if e.as_ref() == Some(&k_toggle) {
                    None
                } else {
                    Some(k_toggle.clone())
                }
            })
        };
        let k_menu = key.clone();
        let menu = move |e: leptos::ev::MouseEvent| {
            use crate::ctxmenu::{CtxItem, open};
            let name = v.with_value(|v| v.name.clone());
            let k = k_menu.clone();
            let items = vec![
                CtxItem::new(ui.now("variables-edit", &[]), move || {
                    editing.set(Some(k.clone()))
                })
                .icon(icons::edit)
                .disabled(!editable),
                CtxItem::new(ui.now("variables-copy-name", &[]), move || {
                    crate::api::copy_text(&name)
                })
                .icon(icons::copy),
            ];
            open(ui, &e, items);
        };
        let tip = move || {
            let value = value.get();
            let mut t = v.with_value(|v| match &v.expr {
                Some(e) => format!("{} = {e}", v.name),
                None => ui.now("variables-block", &[("name", v.name.clone())]),
            });
            if value.is_none() {
                t.push_str(&format!("\n{}", ui.now("variables-not-run", &[])));
            }
            t
        };
        view! {
            <div class="var" class:is-open=move || is_open.get() on:contextmenu=menu
                data-var=v.with_value(|v| v.name.clone())>
                <button class="var-head" title=tip on:click=toggle aria-expanded=move || is_open.get().to_string()>
                    {within.map(|k| {
                        let label = layers::label(ui, &k);
                        let icon = match k {
                            Kind::Each { .. } => icons::repeat(),
                            _ => icons::branch(),
                        };
                        view! { <span class="var-within" title=label>{icon}</span> }
                    })}
                    <span class="var-name">{v.with_value(|v| v.name.clone())}</span>
                    {move || {
                        let x = value.get();
                        let (text, muted) = shown_text(ui, x.as_ref());
                        let varies = x.as_ref().is_some_and(|x| x.varies);
                        view! {
                            <span class="var-value" class:is-muted=muted>{text}</span>
                            {varies.then(|| view! {
                                <span class="var-varies" title=ui.t("variables-varies")>{icons::repeat()}</span>
                            })}
                        }
                    }}
                </button>
                <Show when=move || is_open.get()>
                    {move || {
                        let original = v.with_value(|v| v.expr.clone().unwrap_or_default());
                        let draft = RwSignal::new(original.clone());
                        let rows = original.lines().count().clamp(1, 8);
                        let done = move || editing.set(None);
                        view! {
                            <div class="var-edit">
                                <textarea class="var-expr" spellcheck="false" rows=rows
                                    aria-label=ui.t("variables-expression")
                                    prop:value=move || draft.get()
                                    on:input=move |e| draft.set(event_target_value(&e))
                                    on:keydown=move |e: leptos::ev::KeyboardEvent| {
                                        e.stop_propagation();
                                        // Échap : rien n'est appliqué ; Entrée : appliqué en quittant
                                        // le champ (une seule fois, par `blur`).
                                        if e.key() == "Escape" {
                                            e.prevent_default();
                                            draft.set(original.clone());
                                            done();
                                        } else if e.key() == "Enter" && !e.shift_key() {
                                            e.prevent_default();
                                            if let Some(t) = e.target().and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok()) {
                                                let _ = t.blur();
                                            }
                                            done();
                                        }
                                    }
                                    on:blur=move |_| {
                                        // Champ retiré par Échap : rien à appliquer.
                                        if let Some(d) = draft.try_get_untracked() {
                                            v.with_value(|v| commit(v, d));
                                        }
                                    }></textarea>
                            </div>
                        }
                    }}
                </Show>
            </div>
        }
    };
    // Expression ouverte : champ sélectionné.
    Effect::new(move |_| {
        if editing.get().is_some() {
            request_animation_frame(|| {
                if let Ok(Some(el)) = document().query_selector(".var.is-open .var-expr")
                    && let Ok(t) =
                        wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlTextAreaElement>(el)
                {
                    let _ = t.focus();
                    let n = t.value().len() as u32;
                    let _ = t.set_selection_range(n, n);
                }
            });
        }
    });
    // Autre page : expression refermée.
    Effect::new(move |_| {
        ed.page.track();
        editing.set(None);
    });
    view! {
        <div class="panel-list variables">
            <crate::panels::Search value=q key="variables-search"/>
            <div class="list-scroll">
                {move || {
                    let gs = groups.get();
                    if gs.is_empty() {
                        let key = if q.with(|q| q.trim().is_empty()) { "variables-none" } else { "variables-no-match" };
                        return view! { <p class="panel-empty muted">{ui.t(key)}</p> }.into_any();
                    }
                    gs.into_iter().map(|(owner, vars)| {
                        let (icon, name, title) = owner_head(&owner);
                        let n = vars.len();
                        let (o2, o3) = (owner.clone(), owner.clone());
                        let is_folded = Memo::new(move |_| folded.with(|f| f.contains(&o2)));
                        view! {
                            <button class="list-row list-folder var-owner" title=title
                                on:click=move |_| folded.update(|f| {
                                    if let Some(i) = f.iter().position(|x| *x == o3) { f.remove(i); } else { f.push(o3.clone()); }
                                })>
                                <span class="list-chevron">{move || if is_folded.get() { icons::chevron_right() } else { icons::chevron_down() }}</span>
                                <span class="list-icon">{icon}</span>
                                <span class="list-label">{name}</span>
                                <span class="list-count">{n}</span>
                            </button>
                            <Show when=move || !is_folded.get()>
                                {vars.clone().into_iter().map(row).collect_view()}
                            </Show>
                        }
                    }).collect_view().into_any()
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texte_d_un_fragment() {
        assert_eq!(
            strip_tags("<time datetime=\"x\">12 mars</time> <b>2026</b>"),
            "12 mars 2026"
        );
        assert_eq!(short("layouts/base"), "base");
    }
}
