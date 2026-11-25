//! Champs d'un formulaire dans le panneau Styles (audit M7, D-199, maquette validée le
//! 24/11/2026) : liste des champs (nom, sorte, obligatoire, monter, descendre, retirer), « Ajouter
//! un champ » par sorte, valeurs d'une liste déroulante. Double-clic sur un nom : renommer.

use crate::editor::Ed;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::formfields::{self, Field, FieldKind};
use revel_edit::{Command, Target};

fn kind_label(ui: Ui, k: FieldKind) -> String {
    ui.now(&format!("form-field-{}", k.key()), &[])
}

fn kind_icon_fn(k: FieldKind) -> fn() -> AnyView {
    match k {
        FieldKind::Dropdown | FieldKind::Radio => icons::list_ordered,
        FieldKind::Checkbox => icons::check,
        FieldKind::Date => icons::clock,
        FieldKind::File => icons::upload,
        FieldKind::LongText | FieldKind::Email | FieldKind::Phone | FieldKind::Text => icons::text,
    }
}

fn kind_icon(k: FieldKind) -> AnyView {
    kind_icon_fn(k)()
}

thread_local! {
    /// Champ ajouté dont le nom est à saisir une fois le panneau redessiné.
    static PENDING_RENAME: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Liste des champs du formulaire `at`.
pub fn fields(ui: Ui, ed: Ed, at: &Target) -> AnyView {
    let at = StoredValue::new(at.clone());
    let list = Memo::new(move |_| {
        ed.doc.with(|d| {
            d.as_ref()
                .and_then(|d| revel_edit::address::node(d, &at.get_value()).ok())
                .map(formfields::fields)
                .unwrap_or_default()
        })
    });
    let target = move |id: &str| Target {
        owner: at.get_value().owner,
        node: id.to_string(),
    };
    // Le panneau est redessiné après chaque modification : le champ tout juste ajouté, à nommer,
    // est gardé le temps de ce redessin.
    let renaming = RwSignal::new(PENDING_RENAME.with(|p| p.borrow_mut().take()));
    // Champ déplié (valeurs d'une liste déroulante).
    let open = RwSignal::new(None::<String>);
    let rename = move |id: String, text: String| {
        renaming.set(None);
        let text = text.trim().to_string();
        if text.is_empty() {
            return;
        }
        let cmd = ed.doc.with_untracked(|d| {
            revel_edit::make::rename_link_item(d.as_ref()?, &target(&id), &text).ok()
        });
        if let Some(c) = cmd {
            ed.apply_keep(ui, c);
        }
    };
    // Champ `i` placé au rang `j` (poignée de glisser, D-200).
    let step = move |i: usize, j: usize| {
        let l = list.get_untracked();
        if i >= l.len() || j >= l.len() || i == j {
            return;
        }
        let id = l[i].id.clone();
        let place = ed
            .doc
            .with_untracked(|d| revel_edit::address::locate(d.as_ref()?, &target(&l[j].id)).ok());
        if let Some((to, index)) = place {
            ed.apply_keep(
                ui,
                Command::Move {
                    at: target(&id),
                    to,
                    index,
                },
            );
        }
    };
    let add = move |e: leptos::ev::MouseEvent| {
        use crate::ctxmenu::{CtxItem, open as show};
        let items = FieldKind::ALL
            .into_iter()
            .map(|k| {
                CtxItem::new(kind_label(ui, k), move || {
                    let options: Vec<String> = (1..=2)
                        .map(|n| ui.now("form-option", &[("n", n.to_string())]))
                        .collect();
                    let label = kind_label(ui, k);
                    let cmd = ed.doc.with_untracked(|d| {
                        formfields::add_field(d.as_ref()?, &at.get_value(), k, &label, &options)
                            .ok()
                    });
                    if let Some((c, id)) = cmd {
                        // Nom à donner tout de suite.
                        PENDING_RENAME.with(|p| *p.borrow_mut() = Some(id.clone()));
                        renaming.set(Some(id));
                        ed.apply_keep(ui, c);
                    }
                })
                .icon(kind_icon_fn(k))
            })
            .collect();
        show(ui, &e, items);
    };
    view! {
        <div class="insp-group form-fields" data-tool="form-fields">
            <div class="items-list items-list--fields" data-sort-list role="list" aria-label=ui.t("form-fields")>
                <div class="items-head">
                    <span class="label">{ui.t("form-fields")}</span>
                    <span class="muted items-count">{move || list.with(Vec::len)}</span>
                </div>
                <For each=move || list.get() key=|f| f.clone() let:f>
                    {
                        let f: StoredValue<Field> = StoredValue::new(f);
                        let id = f.with_value(|f| f.id.clone());
                        let control = f.with_value(|f| f.control.clone());
                        let kind = f.with_value(|f| f.kind);
                        let required = f.with_value(|f| f.required);
                        let (i3, i6) = (id.clone(), id.clone());
                        let on = { let id = id.clone(); move || open.with(|o| o.as_deref() == Some(id.as_str())) };
                        let c1 = control.clone();
                        view! {
                            <div class="items-row" role="listitem" class:is-on=on data-item=id.clone() data-kind=kind.key() data-sort-row>
                                {crate::sortable::grip(ui, Callback::new(move |(i, j)| step(i, j)))}
                                <button class="items-main" title=f.with_value(|f| f.name.clone())
                                    on:click={ let id = id.clone(); move |_| {
                                        // Le formulaire reste sélectionné (sa liste aussi) ; une liste
                                        // déroulante montre ses valeurs.
                                        open.update(|o| *o = if o.as_deref() == Some(id.as_str()) { None } else { Some(id.clone()) });
                                    } }>
                                    <span class="items-thumb">{kind_icon(kind)}</span>
                                    <span class="items-name" title=ui.t("items-rename-help")
                                        on:dblclick={ let id = id.clone(); move |e: leptos::ev::MouseEvent| { e.stop_propagation(); renaming.set(Some(id.clone())) } }>
                                        {f.with_value(|f| f.label.clone())}
                                        {required.then(|| view! { <span class="form-req-mark" title=ui.t("form-field-required")>" *"</span> })}
                                    </span>
                                    <span class="items-target">{kind_label(ui, kind)}</span>
                                </button>
                                {{
                                    let id = id.clone();
                                    move || (renaming.get().as_deref() == Some(id.as_str())).then(|| {
                                        let (a, b) = (id.clone(), id.clone());
                                        let field = NodeRef::<leptos::html::Input>::new();
                                        request_animation_frame(move || {
                                            if let Some(i) = field.try_get_untracked().flatten() {
                                                let _ = i.focus();
                                                i.select();
                                            }
                                        });
                                        view! {
                                            <input class="input items-rename" node_ref=field aria-label=ui.t("items-rename-help")
                                                prop:value=f.with_value(|f| f.label.clone())
                                                on:keydown=move |e: leptos::ev::KeyboardEvent| match e.key().as_str() {
                                                    "Enter" => rename(a.clone(), event_target_value(&e)),
                                                    "Escape" => { e.stop_propagation(); renaming.set(None) }
                                                    _ => {}
                                                }
                                                on:blur=move |e| if renaming.get_untracked().is_some() { rename(b.clone(), event_target_value(&e)) }/>
                                        }
                                    })
                                }}
                                <span class="items-actions">
                                    <button class="icon-btn icon-btn--small" title=ui.t("component-rename") aria-label=ui.t("component-rename")
                                        on:click={ let id = id.clone(); move |_| renaming.set(Some(id.clone())) }>{icons::edit()}</button>
                                    <button class="icon-btn icon-btn--small form-required" class:is-on=required
                                        title=ui.t("form-field-required") aria-label=ui.t("form-field-required") aria-pressed=required.to_string()
                                        on:click=move |_| ed.apply_keep(ui, formfields::set_required(&target(&c1), !required))>"*"</button>
                                    <button class="icon-btn icon-btn--small" title=ui.t("action-remove") aria-label=ui.t("action-remove")
                                        on:click=move |_| ed.apply(ui, Command::Remove { at: target(&i3) }, false)>{icons::trash()}</button>
                                </span>
                            </div>
                            {(kind == FieldKind::Dropdown).then(|| {
                                let control = control.clone();
                                let options = Signal::stored(f.with_value(|f| f.options.clone()));
                                view! {
                                    <Show when={ let id = i6.clone(); move || open.get().as_deref() == Some(id.as_str()) }>
                                        <div class="form-options" data-options=control.clone()>
                                            <span class="label">{ui.t("form-field-options")}</span>
                                            <crate::cmsfields::RowsEditor items=options set=Callback::new({ let control = control.clone(); move |v: Vec<String>| {
                                                let v: Vec<String> = v.iter().map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
                                                let cmd = ed.doc.with_untracked(|d| formfields::set_options(d.as_ref()?, &target(&control), &v).ok());
                                                if let Some(c) = cmd {
                                                    ed.apply_keep(ui, c);
                                                }
                                            } })/>
                                        </div>
                                    </Show>
                                }
                            })}
                        }
                    }
                </For>
                <button class="btn btn--small items-add" data-action="form-field-add" on:click=add>{icons::plus()}<span>{ui.t("form-field-add")}</span></button>
            </div>
        </div>
    }
    .into_any()
}
