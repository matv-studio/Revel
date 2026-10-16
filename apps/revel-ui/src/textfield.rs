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
                items.clone()
            } else {
                Vec::new()
            }
        })
    });
    let commit = move |item: &TextItem, v: String| {
        if v == item.text {
            return;
        }
        if let Some(cmd) = ed.text_command(&item.slot, v) {
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
                        let help = item.with_value(bound_help);
                        let rich = item.with_value(|i| i.rich);
                        let text = item.with_value(|i| i.text.clone());
                        let lines = text.lines().count().max(1);
                        let rows = (lines + text.chars().count() / 34).clamp(1, 8);
                        let field = if rich {
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
