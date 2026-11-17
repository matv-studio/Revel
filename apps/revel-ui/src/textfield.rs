//! Champ « Texte » en tête du panneau de styles : textes de l'élément sélectionné, modifiables
//! sans double-clic dans la page (texte écrit, texte venu des données du site ou d'un contenu,
//! propriété de texte d'un composant, texte riche). Le canevas envoie les textes de la sélection ;
//! chaque modification passe par la même commande qu'un texte modifié sur place.

use crate::editor::{Ed, TextItem};
use crate::richtext::{Format, RichText};
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::canvas::{Slot, Source};

/// Le texte vient d'ailleurs que de la page : données du site, contenu, propriété de composant.
fn bound_help(item: &TextItem) -> Option<&'static str> {
    match &item.slot {
        Slot::Expr {
            binding: Some(b), ..
        } => Some(match b.source {
            Source::Prop { .. } => "text-field-prop",
            Source::Entry { .. } | Source::EntryItem { .. } => "text-field-entry",
            _ => "text-field-data",
        }),
        _ => None,
    }
}

/// Textes de la sélection montrés dans le panneau (sans les propriétés de l'utilisation, ni
/// les textes d'un outil fermé).
fn selection_texts(ed: Ed) -> Memo<Vec<TextItem>> {
    Memo::new(move |_| {
        let sel = ed.selected.with(|s| s.as_ref().map(|s| s.id.clone()));
        // Outil fermé : ses textes se règlent dans son panneau, avec leur nom (D-174, D-176).
        let tool = sel.as_ref().is_some_and(|id| {
            ed.doc.with(|d| {
                d.as_ref()
                    .and_then(|d| {
                        let at = crate::editor::target_of(d, id)?;
                        revel_edit::address::node(d, &at)
                            .ok()
                            .map(crate::tools::is_closed_tool)
                    })
                    .unwrap_or(false)
            })
        });
        ed.texts.with(|(id, items)| {
            if sel.is_some() && *id == sel && !tool {
                // Propriétés de texte de l'utilisation sélectionnée : déjà dans ses réglages
                // (« Propriétés »), pas répétées ici (D-124).
                items
                    .iter()
                    .filter(|t| {
                        // Sauf un champ du contenu remplacé sur l'utilisation (`p or champ`) :
                        // dans « Champs CMS » (D-179).
                        !matches!(&t.slot, revel_edit::canvas::Slot::Expr {
                            expr,
                            binding: Some(revel_edit::canvas::Binding {
                                source: revel_edit::canvas::Source::Prop { node, name, .. },
                                ..
                            }),
                            ..
                        } if Some(node) == sel.as_ref()
                            && revel_edit::overrides::split(expr).is_none_or(|(p, _)| p != name))
                    })
                    .cloned()
                    .collect()
            } else {
                Vec::new()
            }
        })
    })
}

/// Utilisation sélectionnée d'un composant relié à un contenu (D-179) : cible, composant, valeurs.
type UseCtx = Option<(
    revel_edit::Target,
    revel_model::Component,
    std::collections::BTreeMap<String, String>,
)>;

fn use_in(d: &revel_model::Site, sel: &str) -> UseCtx {
    let at = crate::editor::target_of(d, sel)?;
    let revel_model::Node::Use {
        component, args, ..
    } = revel_edit::address::node(d, &at).ok()?
    else {
        return None;
    };
    let c = d.components.iter().find(|c| &c.id == component)?;
    c.props
        .iter()
        .any(|p| p.kind == revel_model::PropKind::Entry)
        .then(|| (at, c.clone(), args.clone()))
}

fn use_ctx(ed: Ed) -> Memo<UseCtx> {
    Memo::new(move |_| {
        let sel = ed.selected.with(|s| s.as_ref().map(|s| s.id.clone()))?;
        ed.doc.with(|d| use_in(d.as_ref()?, &sel))
    })
}

/// La même, lue une fois (hors du panneau).
fn use_now(ed: Ed) -> UseCtx {
    let sel = ed
        .selected
        .with_untracked(|s| s.as_ref().map(|s| s.id.clone()))?;
    ed.doc.with_untracked(|d| use_in(d.as_ref()?, &sel))
}

/// Texte lu dans le contenu choisi par l'utilisation (ou remplacé sur elle).
/// Même emplacement dans le composant (sortie dans une boucle).
fn same_place(a: &Slot, b: &Slot) -> bool {
    matches!((a, b), (Slot::Expr { parent: p1, index: i1, .. }, Slot::Expr { parent: p2, index: i2, .. }) if p1 == p2 && i1 == i2)
}

fn is_cms(u: &UseCtx, item: &TextItem) -> bool {
    let Slot::Expr {
        parent,
        expr,
        binding,
        ..
    } = &item.slot
    else {
        return false;
    };
    // Autre contenu lu dans le composant (marque d'un projet…) : du CMS aussi.
    let entry = binding.as_ref().is_some_and(|b| {
        matches!(
            b.source,
            revel_edit::canvas::Source::Entry { .. } | revel_edit::canvas::Source::EntryItem { .. }
        )
    });
    // Texte lu dans un contenu, où qu'il soit (carte d'une liste, page d'une collection) : champ
    // CMS en lecture seule, jamais modifié depuis le panneau Styles (D-182, D-184).
    if entry {
        return true;
    }
    u.as_ref().is_some_and(|(_, c, _)| {
        matches!(&parent.owner, revel_edit::Owner::Component(id) if *id == c.id)
            && (entry
                || revel_edit::overrides::reads_entry(c, expr)
                || revel_edit::overrides::split(expr).is_some_and(|(p, e)| {
                    revel_edit::overrides::override_props(c).contains(p)
                        && revel_edit::overrides::reads_entry(c, e)
                }))
    })
}

#[component]
pub fn SelectedTexts() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let items = selection_texts(ed);
    let use_ctx = use_ctx(ed);
    // Textes lus dans le contenu de l'utilisation : sous « Champs CMS » (D-179).
    let free = Memo::new(move |_| {
        let u = use_ctx.get();
        items
            .get()
            .into_iter()
            .filter(|i| !is_cms(&u, i))
            .collect::<Vec<_>>()
    });
    let commit = move |item: &TextItem, v: String| {
        if v == item.text {
            return;
        }
        // Retours à la ligne (Maj+Entrée) : balises `br` dans le texte, comme sur le canevas.
        let cmd = if v.trim_end().contains('\n') && !item.rich {
            let html = v
                .trim_end()
                .split('\n')
                .map(|l| {
                    l.replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;")
                })
                .collect::<Vec<_>>()
                .join("<br>");
            ed.text_html_command(&item.slot, &html)
        } else {
            ed.text_command(&item.slot, v)
        };
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    view! {
        <Show when=move || free.with(|i| !i.is_empty())>
            <section class="text-fields" aria-label=ui.t("text-field")>
                <For each=move || free.get().into_iter().enumerate()
                    key=|(i, it)| (*i, it.text.clone(), it.rich)
                    let:entry>
                    {
                        let (_, item) = entry;
                        let item = StoredValue::new(item);
                        // Valeur déjà envoyée (Entrée puis sortie du champ) : une seule fois.
                        let sent = StoredValue::new(None::<String>);
                        let commit = move |i: &TextItem, v: String| {
                            if sent.with_value(|s| s.as_deref() == Some(v.as_str())) {
                                return;
                            }
                            sent.set_value(Some(v.clone()));
                            commit(i, v)
                        };
                        let help = item.with_value(bound_help);
                        let rich = item.with_value(|i| i.rich);
                        let host = item.with_value(|i| i.host.clone());
                        let text = item.with_value(|i| i.text.clone());
                        let lines = text.lines().count().max(1);
                        let rows = (lines + text.chars().count() / 34).clamp(1, 8);
                        let field = if let Some(host) = host {
                            // Texte mis en forme : tout l'élément d'un bloc (D-72).
                            let on_commit = Callback::new(move |v: String| {
                                if sent.with_value(|s| s.as_deref() == Some(v.as_str())) {
                                    return;
                                }
                                sent.set_value(Some(v.clone()));
                                let cmd = item.with_value(|i| ed.rich_text_command(&i.slot, &host, &v));
                                if let Some((cmd, _)) = cmd {
                                    ed.apply(ui, cmd, false);
                                }
                            });
                            view! { <InlineText html=text.clone() on_commit/> }.into_any()
                        } else if rich {
                            view! {
                                <RichText value=Signal::derive(move || item.with_value(|i| i.text.clone())) format=Format::Html
                                    on_commit=Callback::new(move |v: String| item.with_value(|i| commit(i, v)))/>
                            }.into_any()
                        } else {
                            view! {
                                <textarea class="input text-field-input" aria-label=ui.t("text-field") rows=rows.to_string() spellcheck="true"
                                    prop:value=text.clone()
                                    on:change=move |e| { let v = event_target_value(&e); item.with_value(|i| commit(i, v)) }
                                    on:keydown=move |e: leptos::ev::KeyboardEvent| {
                                        // Entrée valide (comme dans la page) ; Maj+Entrée : retour à la ligne.
                                        if e.key() == "Enter" && !e.shift_key() {
                                            e.prevent_default();
                                            let v = event_target_value(&e);
                                            item.with_value(|i| commit(i, v));
                                        }
                                    }></textarea>
                            }.into_any()
                        };
                        view! {
                            <div class="text-field">
                                <span class="text-field-head">
                                    <span class="style-label" title=ui.t("text-field-help")>
                                        <span class="style-label-text">{ui.t("text-field")}</span>
                                    </span>
                                    {help.map(|h| view! { <span class="text-field-bound" title=ui.t(h)>{icons::database()}</span> })}
                                </span>
                                {field}
                            </div>
                        }
                    }
                </For>
            </section>
        </Show>
    }
}

thread_local! {
    /// Collections du CMS déjà lues (libellés des champs), pour le menu du double-clic.
    static CMS_COLS: std::cell::RefCell<Vec<revel_protocol::CmsCollection>> = const { std::cell::RefCell::new(Vec::new()) };
    /// « Champs CMS » dépliés : gardé quand le panneau se refait (remplacement, autre contenu,
    /// autre utilisation).
    static CMS_OPEN: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// Champs du contenu d'une utilisation de composant relié au CMS (D-179) : repliés, grisés ;
/// chacun se remplace sur cette utilisation (icône crayon) et se rend au contenu.
#[component]
pub fn CmsFields() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let items = selection_texts(ed);
    let use_ctx = use_ctx(ed);
    // Un champ par expression : sortie répétée (numéro…) montrée une fois, éléments d'une liste
    // (mots-clés) réunis.
    let cms = Memo::new(move |_| {
        let u = use_ctx.get();
        let mut out: Vec<TextItem> = Vec::new();
        for i in items.get().into_iter().filter(|i| is_cms(&u, i)) {
            let key = |t: &TextItem| match &t.slot {
                Slot::Expr { expr, .. } => expr.clone(),
                _ => String::new(),
            };
            match out.iter_mut().find(|o| key(o) == key(&i)) {
                Some(o) if same_place(&o.slot, &i.slot) => {
                    o.text.push_str(", ");
                    o.text.push_str(&i.text);
                }
                Some(_) => {}
                None => out.push(i),
            }
        }
        out
    });
    // Remplacement d'un champ du contenu sur cette utilisation (`None` : rendu au contenu).
    let set_override = move |item: &TextItem, v: Option<String>| {
        let Slot::Expr { parent, index, .. } = &item.slot else {
            return;
        };
        let Some(at) = use_ctx.with_untracked(|u| u.as_ref().map(|(at, _, _)| at.clone())) else {
            return;
        };
        let cmd = ed.doc.with_untracked(|d| {
            revel_edit::overrides::set_override(d.as_ref()?, parent, *index, &at, v.as_deref()).ok()
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let editing = RwSignal::new(None::<usize>);
    let cols = crate::blocks::collections(ed);
    Effect::new(move |_| cols.with(|c| CMS_COLS.with(|k| *k.borrow_mut() = c.clone())));
    let cms_view = move || {
        let list = cms.get();
        // Collection du premier champ : les champs d'un autre contenu portent le nom de la leur.
        let main = list.iter().find_map(|i| entry_of(&i.slot)).map(|(f, _)| f);
        (!list.is_empty()).then(|| {
            let n = list.len();
            // Contenu lu par ces champs (le premier) : ouvert dans le CMS.
            let entry = list.iter().find_map(|i| entry_of(&i.slot));
            view! {
                <details class="cms-fields" prop:open=CMS_OPEN.get()
                    on:toggle=move |e| {
                        use wasm_bindgen::JsCast;
                        if let Some(d) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlDetailsElement>().ok()) {
                            CMS_OPEN.set(d.open());
                        }
                    }>
                    <summary class="cms-fields-head" title=ui.t("use-cms-fields-help")>
                        {icons::database()}
                        <span>{ui.t("use-cms-fields")}</span>
                        <span class="muted">{n}</span>
                    </summary>
                    {list.into_iter().enumerate().map(|(i, item)| {
                        let expr = match &item.slot { Slot::Expr { expr, .. } => expr.clone(), _ => String::new() };
                        let over = revel_edit::overrides::split(&expr)
                            .map(|(p, _)| p.to_string())
                            .filter(|p| use_ctx.with(|u| u.as_ref().is_some_and(|(_, c, args)| {
                                revel_edit::overrides::override_props(c).contains(p) && args.contains_key(p)
                            })));
                        let overridden = over.is_some();
                        // Remplaçable : texte lu dans le contenu choisi par l'utilisation.
                        let can = overridden || use_ctx.with(|u| u.as_ref().is_some_and(|(_, c, _)| {
                            revel_edit::overrides::reads_entry(c, &expr)
                                // Déjà remplaçable (remplacé ailleurs, ou rendu au contenu ici).
                                || revel_edit::overrides::split(&expr).is_some_and(|(p, e)| {
                                    revel_edit::overrides::override_props(c).contains(p) && revel_edit::overrides::reads_entry(c, e)
                                })
                        }));
                        let label = match entry_of(&item.slot) {
                            Some((folder, _)) => {
                                let (c, f) = cols.with(|cs| crate::blocks::cms_names(cs, &folder, &expr));
                                if main.as_deref() == Some(folder.as_str()) { f } else { format!("{c} › {f}") }
                            }
                            None => revel_edit::overrides::field_label(&expr),
                        };
                        let (label1, label2) = (label.clone(), label.clone());
                        let item = StoredValue::new(item);
                        let asked = StoredValue::new(expr.clone());
                        let open = move || {
                            overridden
                                || editing.get() == Some(i)
                                || (can && ed.cms_edit.with(|e| asked.with_value(|a| e.as_deref() == Some(a.as_str()))))
                        };
                        let text = item.with_value(|i| i.text.clone());
                        view! {
                            <div class="text-field cms-field" class:is-overridden=overridden data-field=label1>
                                <span class="text-field-head">
                                    <span class="style-label"><span class="style-label-text">{label2}</span></span>
                                    {if overridden {
                                        view! {
                                            <button type="button" class="icon-btn icon-btn--small cms-field-reset"
                                                title=ui.t("cms-field-reset") aria-label=ui.t("cms-field-reset")
                                                on:click=move |_| { editing.set(None); item.with_value(|it| set_override(it, None)) }>{icons::reset()}</button>
                                        }.into_any()
                                    } else if !can {
                                        ().into_any()
                                    } else {
                                        view! {
                                            <button type="button" class="icon-btn icon-btn--small cms-field-override"
                                                class:is-on=move || editing.get() == Some(i)
                                                title=ui.t("cms-field-override") aria-label=ui.t("cms-field-override")
                                                on:click=move |_| editing.update(|e| *e = if *e == Some(i) { None } else { Some(i) })>{icons::edit()}</button>
                                        }.into_any()
                                    }}
                                </span>
                                <textarea class="input text-field-input" rows="1" spellcheck="true"
                                    aria-label=label
                                    prop:readOnly=move || !open()
                                    class:is-readonly=move || !open()
                                    prop:value=text
                                    on:change=move |e| {
                                        // Champ déjà remplacé par Entrée (panneau refait) : rien.
                                        if editing.is_disposed() || item.is_disposed() || use_ctx.try_with_untracked(|_| ()).is_none() {
                                            return;
                                        }
                                        let v = event_target_value(&e);
                                        editing.set(None);
                                        ed.cms_edit.set(None);
                                        item.with_value(|it| if v != it.text { set_override(it, Some(v.clone())) });
                                    }
                                    on:keydown=move |e: leptos::ev::KeyboardEvent| {
                                        if e.key() == "Enter" && !e.shift_key() {
                                            e.prevent_default();
                                            let v = event_target_value(&e);
                                            editing.set(None);
                                            ed.cms_edit.set(None);
                                            item.with_value(|it| if v != it.text { set_override(it, Some(v.clone())) });
                                        }
                                    }></textarea>
                            </div>
                        }
                    }).collect_view()}
                    {entry.map(|(folder, file)| view! {
                        <button type="button" class="btn btn--small cms-fields-open" data-action="cms-open-entry"
                            on:click=move |_| open_entry(ui, &folder, &file)>
                            {icons::database()}<span>{ui.t("cms-open-entry")}</span>
                        </button>
                    })}
                </details>
            }
        })
    };
    view! { {cms_view} }
}

/// Contenu du CMS (dossier de la collection, fichier) lu par un emplacement de texte.
fn entry_of(slot: &Slot) -> Option<(String, String)> {
    let Slot::Expr {
        binding: Some(b), ..
    } = slot
    else {
        return None;
    };
    match &b.source {
        Source::Entry { folder, file } | Source::EntryItem { folder, file } => {
            Some((folder.clone(), file.clone()))
        }
        _ => None,
    }
}

/// Ouvre un contenu dans l'onglet CMS.
fn open_entry(ui: Ui, folder: &str, file: &str) {
    ui.goto_entry
        .set(Some((folder.to_string(), file.to_string())));
    ui.tab(crate::Tab::Cms);
}

/// Double-clic dans le canevas sur un texte lu dans un contenu du CMS (D-182, D-184) : menu des
/// actions à la place de la saisie sur place. `pos` : position dans la fenêtre ; `component`,
/// `node` : composant de l'utilisation pointée et son élément sous le pointeur.
pub fn cms_text_menu(
    ui: Ui,
    ed: Ed,
    dock: crate::dock::Dock,
    slot: &Slot,
    pos: (f64, f64),
    component: Option<String>,
    node: Option<(String, String)>,
) {
    use crate::ctxmenu::CtxItem;
    let Slot::Expr { expr, .. } = slot else {
        return;
    };
    let Some((folder, file)) = entry_of(slot) else {
        return;
    };
    let (collection, field) =
        CMS_COLS.with(|k| crate::blocks::cms_names(&k.borrow(), &folder, expr));
    let head = format!("{collection} › {field}");
    let mut items = vec![
        CtxItem::new(head, || {})
            .icon(icons::database)
            .disabled(true),
    ];
    // Remplaçable : utilisation sélectionnée d'un composant qui lit ce champ dans son contenu.
    let can = {
        let u = use_now(ed);
        u.as_ref().is_some_and(|(_, c, _)| {
            component.as_deref() == Some(c.id.as_str())
                && (revel_edit::overrides::reads_entry(c, expr)
                    || revel_edit::overrides::split(expr).is_some_and(|(p, e)| {
                        revel_edit::overrides::override_props(c).contains(p)
                            && revel_edit::overrides::reads_entry(c, e)
                    }))
        })
    };
    if can {
        let expr = expr.clone();
        items.push(
            CtxItem::new(ui.now("cms-field-override", &[]), move || {
                dock.layout.update(|l| l.open(crate::dock::PanelId::Styles));
                CMS_OPEN.set(true);
                ed.cms_edit.set(Some(expr.clone()));
            })
            .icon(icons::edit)
            .sep(),
        );
    }
    let mut open = CtxItem::new(ui.now("cms-open-entry", &[]), move || {
        open_entry(ui, &folder, &file)
    })
    .icon(icons::database);
    if !can {
        open = open.sep();
    }
    items.push(open);
    if let Some(c) = component {
        items.push(
            CtxItem::new(ui.now("component-edit", &[]), move || {
                ed.open_comp(&c, node.clone())
            })
            .icon(icons::edit),
        );
    }
    crate::ctxmenu::open_at(ui, pos.0, pos.1, items, true);
}

/// Texte mis en forme d'un élément (D-72), modifié d'un bloc dans le panneau : gras (Ctrl+B),
/// italique (Ctrl+I), retour à la ligne (Maj+Entrée) ; Entrée ou sortie du champ valide.
#[component]
fn InlineText(html: String, on_commit: Callback<String>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let node = NodeRef::<leptos::html::Div>::new();
    Effect::new(move |_| {
        if let Some(d) = node.get() {
            d.set_inner_html(&html);
        }
    });
    let read = move || {
        node.get_untracked()
            .map(|d| d.inner_html())
            .unwrap_or_default()
    };
    view! {
        <div class="input text-field-input text-field-rich" contenteditable="true" spellcheck="true"
            role="textbox" aria-label=ui.t("text-field") node_ref=node
            on:keydown=move |e: leptos::ev::KeyboardEvent| {
                if e.key() == "Enter" && !e.shift_key() {
                    e.prevent_default();
                    on_commit.run(read());
                }
            }
            on:blur=move |_| on_commit.run(read())></div>
    }
}
