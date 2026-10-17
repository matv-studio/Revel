//! Aide intégrée (6.8) : « Premiers pas », guide écrit en Markdown (`help/fr.md`, `help/en.md`),
//! une rubrique à la fois, sommaire à gauche. Ouvert depuis le menu « ? » de l'éditeur.

use crate::Ui;
use leptos::prelude::*;

const FR: &str = include_str!("../help/fr.md");
const EN: &str = include_str!("../help/en.md");

/// Rubriques du guide : titre (`## …`), Markdown. La première contient l'introduction.
pub fn sections(lang: &str) -> Vec<(String, String)> {
    let src = if lang.starts_with("fr") { FR } else { EN };
    let mut out: Vec<(String, String)> = Vec::new();
    for line in src.lines() {
        if let Some(t) = line.strip_prefix("# ") {
            out.push((t.trim().to_string(), String::new()));
        } else if let Some(t) = line.strip_prefix("## ") {
            out.push((t.trim().to_string(), String::new()));
        } else if let Some(last) = out.last_mut() {
            last.1.push_str(line);
            last.1.push('\n');
        }
    }
    out
}

fn html(md: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(md, Options::ENABLE_TABLES));
    out
}

/// Fenêtre de l'aide.
#[component]
pub fn Help(open: RwSignal<bool>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let current = RwSignal::new(0usize);
    let list = Memo::new(move |_| sections(&ui.lang()));
    // Échap ferme l'aide ; écouteur retiré avec la fenêtre (sinon il survit à l'éditeur).
    let esc = window_event_listener(leptos::ev::keydown, move |e| {
        if e.key() == "Escape" && open.try_get_untracked() == Some(true) {
            e.prevent_default();
            open.set(false);
        }
    });
    on_cleanup(move || esc.remove());
    view! {
        <Show when=move || open.get()>
            <div class="overlay" on:mousedown=move |e| { if e.target() == e.current_target() { open.set(false) } }>
                <div class="dialog help-dialog" role="dialog" aria-modal="true" aria-label=ui.t("help-title")>
                    <nav class="help-nav" aria-label=ui.t("help-title")>
                        {move || list.get().into_iter().enumerate().map(|(i, (t, _))| view! {
                            <button class="help-nav-item" class:is-on=move || current.get() == i on:click=move |_| current.set(i)>{t}</button>
                        }).collect_view()}
                    </nav>
                    <article class="help-body">
                        <button class="icon-btn help-close" title=ui.t("action-close") aria-label=ui.t("action-close")
                            on:click=move |_| open.set(false)>{crate::icons::close()}</button>
                        {move || list.with(|l| l.get(current.get()).map(|(t, md)| view! {
                            <h2>{t.clone()}</h2>
                            <div class="help-text" inner_html=html(md)></div>
                        }))}
                    </article>
                </div>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rubriques_du_guide() {
        for lang in ["fr", "en"] {
            let s = sections(lang);
            assert!(s.len() >= 10, "{lang}");
            assert!(
                s.iter()
                    .all(|(t, md)| !t.is_empty() && !md.trim().is_empty()),
                "{lang}"
            );
        }
        // Mêmes rubriques dans les deux langues.
        assert_eq!(sections("fr").len(), sections("en").len());
    }
}
