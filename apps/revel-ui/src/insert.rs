//! Panneau « Ajouter » (étape 2.5) : éléments de base insérés dans l'élément sélectionné (s'il peut
//! contenir des enfants) ou juste après lui.

use crate::editor::Ed;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::make::Preset;
use revel_edit::{Branch, Command, Owner, Parent};
use revel_model::Node;

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Emplacement d'insertion : dans l'élément sélectionné, après lui, ou à la fin de la page.
pub fn insertion_point(ed: Ed) -> Option<(Parent, usize)> {
    let target = ed.selected_target();
    ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        match target {
            Some(t) => {
                let node = revel_edit::address::node(d, &t).ok()?;
                match node {
                    Node::Element { tag, children, .. }
                        if !VOID.contains(&tag.to_ascii_lowercase().as_str()) =>
                    {
                        Some((Parent::of(&t, Branch::Children), children.len()))
                    }
                    _ => {
                        let (p, i) = revel_edit::address::locate(d, &t).ok()?;
                        Some((p, i + 1))
                    }
                }
            }
            None => {
                // Sans sélection : à la fin du corps de la page affichée (page fixe).
                let url = ed.page.get_untracked();
                let id = ed.pages.with_untracked(|ps| {
                    ps.iter()
                        .find(|p| p.url == url)
                        .and_then(|p| p.page.clone())
                })?;
                let body = d.pages.iter().find(|p| p.id == id)?.body.len();
                Some((Parent::root(Owner::Page(id)), body))
            }
        }
    })
}

/// Élément proposé : préréglage, libellé, texte par défaut (clés de traduction), icône.
type Item = (Preset, &'static str, &'static str, fn() -> AnyView);

#[component]
pub fn Insert() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let items: [Item; 8] = [
        (Preset::Section, "insert-section", "", icons::slot),
        (Preset::Container, "insert-container", "", icons::box_),
        (
            Preset::Heading,
            "insert-heading",
            "insert-text-heading",
            icons::text,
        ),
        (
            Preset::Paragraph,
            "insert-paragraph",
            "insert-text-paragraph",
            icons::text,
        ),
        (Preset::Link, "insert-link", "insert-text-link", icons::link),
        (
            Preset::Button,
            "insert-button",
            "insert-text-button",
            icons::wrap,
        ),
        (
            Preset::Image,
            "insert-image",
            "insert-text-image",
            icons::image,
        ),
        (
            Preset::List,
            "insert-list",
            "insert-text-list",
            icons::repeat,
        ),
    ];
    let add = move |preset: Preset, text_key: &'static str| {
        let Some((parent, index)) = insertion_point(ed) else {
            ui.fail(revel_protocol::Failure::new("insert-no-place"));
            return;
        };
        let text = if text_key.is_empty() {
            String::new()
        } else {
            ui.now(text_key, &[])
        };
        ed.apply(
            ui,
            Command::InsertPreset {
                parent,
                index,
                preset,
                text,
            },
            false,
        );
    };
    let place = move || {
        if ed.selected.get().is_some() {
            ui.tx("insert-where-selection", &[])
        } else {
            ui.tx("insert-where-page", &[])
        }
    };
    view! {
        <div class="insert">
            <p class="muted insert-help">{place}</p>
            <div class="insert-grid">
                {items.into_iter().map(|(preset, key, text_key, icon)| view! {
                    <button class="insert-item" on:click=move |_| add(preset, text_key)>
                        {icon()}<span>{ui.t(key)}</span>
                    </button>
                }).collect_view()}
            </div>
        </div>
    }
}
