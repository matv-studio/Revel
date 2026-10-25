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

#[component]
pub fn SelectedTexts() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let items = Memo::new(move |_| {
        let sel = ed.selected.with(|s| s.as_ref().map(|s| s.id.clone()));
        ed.texts.with(|(id, items)| {
            if sel.is_some() && *id == sel {
                // Propriétés de texte de l'utilisation sélectionnée : déjà dans ses réglages
                // (« Propriétés »), pas répétées ici (D-124).
                items
                    .iter()
                    .filter(|t| {
                        !matches!(&t.slot, revel_edit::canvas::Slot::Expr {
                            binding: Some(revel_edit::canvas::Binding {
                                source: revel_edit::canvas::Source::Prop { node, .. },
                                ..
                            }),
                            ..
                        } if Some(node) == sel.as_ref())
                    })
                    .cloned()
                    .collect()
            } else {
                Vec::new()
            }
        })
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
        <Show when=move || items.with(|i| !i.is_empty())>
            <section class="text-fields" aria-label=ui.t("text-field")>
                <For each=move || items.get().into_iter().enumerate()
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
                                <textarea class="input text-field-input" rows=rows.to_string() spellcheck="true"
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
