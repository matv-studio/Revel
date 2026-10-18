//! Affichage selon des conditions (« Conditional visibility » de Webflow) : une ligne par
//! clause (champ, test, valeur), toutes vraies pour que l'élément s'affiche. Les champs
//! proposés sont ceux de l'endroit : contenu d'une liste ou d'un modèle de collection, valeur
//! d'une liste tirée d'un champ, propriétés du composant ouvert.

use crate::Ui;
use leptos::prelude::*;
use revel_edit::visibility::{self, Clause, Lit, Test};

const TESTS: &[(&str, &str)] = &[
    ("set", "cond-set"),
    ("empty", "cond-empty"),
    ("equals", "cond-equals"),
    ("differs", "cond-differs"),
    ("contains", "cond-contains"),
    ("more", "cond-more"),
    ("less", "cond-less"),
];

/// Éditeur de condition. `fields` : (expression, libellé). `cond` : condition actuelle
/// (`None` : toujours). `optional` : la dernière clause peut être retirée (toujours affiché).
/// `on_change` reçoit la condition écrite, ou `None`.
pub fn condition_editor(
    ui: Ui,
    fields: Signal<Vec<(String, String)>>,
    cond: Option<String>,
    optional: bool,
    on_change: impl Fn(Option<String>) + Copy + Send + Sync + 'static,
) -> AnyView {
    let parsed = cond.as_deref().map(visibility::parse);
    // Condition écrite autrement : affichée telle quelle, remplaçable par des clauses.
    if let Some(None) = parsed {
        let raw = cond.clone().unwrap_or_default();
        return view! {
            <div class="cond-editor">
                <p class="cond-raw mono" title=ui.t("cond-raw-help")>{raw}</p>
                {move || fields.with(|fs| fs.first().map(|(e, _)| e.clone())).map(|f| view! {
                    <button type="button" class="btn btn--small" on:click=move |_| {
                        on_change(Some(visibility::to_expr(&[Clause { field: f.clone(), test: Test::Set }])))
                    }>{ui.t("cond-replace")}</button>
                })}
            </div>
        }
        .into_any();
    }
    let clauses = RwSignal::new(parsed.flatten().unwrap_or_default());
    let commit = move |list: Vec<Clause>| {
        clauses.set(list.clone());
        on_change((!list.is_empty()).then(|| visibility::to_expr(&list)));
    };
    let edit = move |i: usize, f: Box<dyn Fn(&mut Clause)>| {
        let mut list = clauses.get_untracked();
        if let Some(c) = list.get_mut(i) {
            f(c);
        }
        commit(list);
    };
    let row = move |i: usize, c: Clause| {
        let cur_field = c.field.clone();
        let sel_field = c.field.clone();
        let value = match &c.test {
            Test::Equals(l) | Test::Differs(l) | Test::Contains(l) => Some(l.text.clone()),
            _ => None,
        };
        let count = match &c.test {
            Test::MoreThan(n) | Test::LessThan(n) => Some(*n),
            _ => None,
        };
        let key = c.test.key();
        let removable = optional || clauses.with_untracked(|l| l.len() > 1);
        view! {
            <div class="cond-row">
                <div class="select">
                    <select class="input" aria-label=ui.t("cond-field") on:change=move |e| {
                        let v = event_target_value(&e);
                        edit(i, Box::new(move |c| c.field = v.clone()));
                    }>
                        {move || {
                            let known = fields.with(|fs| fs.iter().any(|(e, _)| *e == cur_field));
                            (!known).then(|| view! { <option value=cur_field.clone() selected=true>{cur_field.clone()}</option> })
                        }}
                        {move || fields.with(|fs| fs.iter().map(|(e, l)| {
                            let on = *e == sel_field;
                            view! { <option value=e.clone() selected=on>{l.clone()}</option> }
                        }).collect_view())}
                    </select>
                </div>
                <div class="select">
                    <select class="input" aria-label=ui.t("cond-test") on:change=move |e| {
                        let v = event_target_value(&e);
                        edit(i, Box::new(move |c| c.test = c.test.with_key(&v)));
                    }>
                        {TESTS.iter().map(|(k, label)| view! {
                            <option value=*k selected=*k == key>{ui.t(label)}</option>
                        }).collect_view()}
                    </select>
                </div>
                {value.map(|v| view! {
                    <input class="input" aria-label=ui.t("cond-value") prop:value=v
                        on:change=move |e| {
                            let t = event_target_value(&e);
                            edit(i, Box::new(move |c| {
                                let number = matches!(&c.test, Test::Equals(l) | Test::Differs(l) | Test::Contains(l) if l.number);
                                let lit = Lit { text: t.clone(), number: number && t.trim().parse::<f64>().is_ok() };
                                c.test = match &c.test {
                                    Test::Differs(_) => Test::Differs(lit),
                                    Test::Contains(_) => Test::Contains(lit),
                                    _ => Test::Equals(lit),
                                };
                            }));
                        }/>
                })}
                {count.map(|n| view! {
                    <input class="input input--narrow" type="number" min="0" aria-label=ui.t("cond-count")
                        prop:value=n.to_string()
                        on:change=move |e| {
                            let n: usize = event_target_value(&e).trim().parse().unwrap_or(0);
                            edit(i, Box::new(move |c| c.test = match c.test {
                                Test::LessThan(_) => Test::LessThan(n),
                                _ => Test::MoreThan(n),
                            }));
                        }/>
                })}
                {removable.then(|| view! {
                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("cond-remove") aria-label=ui.t("cond-remove")
                        on:click=move |_| {
                            let mut list = clauses.get_untracked();
                            if i < list.len() { list.remove(i); }
                            commit(list);
                        }>{crate::icons::close()}</button>
                })}
            </div>
        }
    };
    view! {
        <div class="cond-editor">
            {move || {
                let list = clauses.get();
                if list.is_empty() {
                    view! { <p class="muted small">{ui.t("cond-always")}</p> }.into_any()
                } else {
                    list.into_iter().enumerate().map(|(i, c)| row(i, c)).collect_view().into_any()
                }
            }}
            <button type="button" class="link-btn cond-add" on:click=move |_| {
                let Some(f) = fields.with_untracked(|fs| fs.first().map(|(e, _)| e.clone())) else { return };
                let mut list = clauses.get_untracked();
                list.push(Clause { field: f, test: Test::Set });
                commit(list);
            }>{ui.t("cond-add")}</button>
        </div>
    }
    .into_any()
}
