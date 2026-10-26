//! Boîte de dialogue de l'application : confirmation ou saisie d'une valeur.
//!
//! Remplace les fenêtres natives `confirm()` / `prompt()`, dont le rendu et la disponibilité
//! varient selon le système (WebView2, WKWebView, WebKitGTK). Une seule boîte à la fois,
//! ouverte par `Ui::ask`.

use crate::Ui;
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use std::sync::Arc;

/// Option d'un choix : valeur, libellé, valeur proposée pour le champ.
pub type ChoiceOpt = (String, String, String);

/// Demande affichée à l'utilisateur. Les textes sont déjà traduits.
#[derive(Clone)]
pub struct Ask {
    pub title: String,
    pub body: Option<String>,
    /// Champ de saisie : libellé et valeur initiale.
    pub input: Option<(String, String)>,
    /// Libellé du bouton de confirmation (verbe d'action).
    pub confirm: String,
    /// Action destructrice : bouton de confirmation en rouge.
    pub danger: bool,
    /// Choix au-dessus du champ (dossier d'une nouvelle collection) : libellé, options (valeur,
    /// libellé, valeur proposée pour le champ), choix retenu.
    pub choice: Option<(String, Vec<ChoiceOpt>, RwSignal<String>)>,
    /// Appelé avec la valeur saisie (vide sans champ) quand l'utilisateur confirme. Fonction
    /// simple (pas un `Callback` réactif) : la vue qui a ouvert la boîte peut avoir été
    /// redessinée entre-temps sans que l'action soit perdue.
    pub on_confirm: Arc<dyn Fn(String) + Send + Sync>,
}

impl Ask {
    pub fn confirm(
        title: String,
        body: Option<String>,
        confirm: String,
        on_confirm: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self {
            title,
            body,
            input: None,
            confirm,
            danger: false,
            choice: None,
            on_confirm: Arc::new(move |_: String| on_confirm()),
        }
    }

    /// Ajoute un choix au-dessus du champ ; choisir une option propose sa valeur dans le champ.
    pub fn with_choice(
        mut self,
        label: String,
        options: Vec<ChoiceOpt>,
        value: RwSignal<String>,
    ) -> Self {
        self.choice = Some((label, options, value));
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    pub fn input(
        title: String,
        label: String,
        initial: String,
        confirm: String,
        on_confirm: impl Fn(String) + Send + Sync + 'static,
    ) -> Self {
        Self {
            title,
            body: None,
            input: Some((label, initial)),
            confirm,
            danger: false,
            choice: None,
            on_confirm: Arc::new(on_confirm),
        }
    }
}

#[component]
pub fn Dialog() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let value = RwSignal::new(String::new());
    let input_ref = NodeRef::<html::Input>::new();
    let confirm_ref = NodeRef::<html::Button>::new();
    let cancel_ref = NodeRef::<html::Button>::new();

    // À l'ouverture : valeur initiale, puis focus sur le champ, sinon sur le bouton de
    // confirmation — ou sur « Annuler » pour une action destructrice (Entrée ne supprime pas).
    Effect::new(move |_| {
        if let Some(a) = ui.ask.get() {
            value.set(a.input.as_ref().map(|(_, v)| v.clone()).unwrap_or_default());
            request_animation_frame(move || {
                let button = if a.danger { cancel_ref } else { confirm_ref };
                if let Some(i) = input_ref.get_untracked() {
                    let _ = i.focus();
                    i.select();
                } else if let Some(b) = button.get_untracked() {
                    let _ = b.focus();
                }
            });
        }
    });

    let close = move || ui.ask.set(None);
    let submit = move || {
        let Some(a) = ui.ask.get_untracked() else {
            return;
        };
        let v = value.get_untracked().trim().to_string();
        if a.input.is_some() && v.is_empty() {
            return;
        }
        ui.ask.set(None);
        (a.on_confirm)(v);
    };
    let _esc = window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" && ui.ask.with_untracked(|a| a.is_some()) {
            e.prevent_default();
            close();
        }
    });

    move || {
        ui.ask.get().map(|a| {
            let has_input = a.input.is_some();
            let label = a.input.as_ref().map(|(l, _)| l.clone()).unwrap_or_default();
            view! {
                <div class="overlay" on:input=move |e| mark_dirty(&e) on:mousedown=move |e| {
                    // Clic sur le fond (hors de la boîte) : fermeture, sauf après une saisie.
                    if backdrop_click(&e) { close() }
                }>
                    <form class="dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title"
                        on:submit=move |e| { e.prevent_default(); submit() }>
                        <h2 id="dialog-title">{a.title.clone()}</h2>
                        {a.body.clone().map(|b| view! { <p class="muted">{b}</p> })}
                        {a.choice.clone().map(|(l, options, chosen)| {
                            let opts = options.clone();
                            view! {
                                <label class="field">
                                    <span class="label">{l}</span>
                                    <div class="select">
                                        <select class="input" on:change=move |e| {
                                            let v = event_target_value(&e);
                                            // Valeur proposée : remplace le champ s'il est vide ou
                                            // s'il porte la proposition d'un autre choix.
                                            let cur = value.get_untracked();
                                            let proposed = |x: &str| opts.iter().any(|(_, _, n)| n == x);
                                            if cur.trim().is_empty() || proposed(&cur) {
                                                let n = opts.iter().find(|(o, _, _)| *o == v).map(|(_, _, n)| n.clone()).unwrap_or_default();
                                                value.set(n);
                                            }
                                            chosen.set(v);
                                        }>
                                            {options.into_iter().map(|(v, l, _)| {
                                                let on = chosen.get_untracked() == v;
                                                view! { <option value=v selected=on>{l}</option> }
                                            }).collect_view()}
                                        </select>
                                    </div>
                                </label>
                            }
                        })}
                        <Show when=move || has_input>
                            <label class="field">
                                <span class="label">{label.clone()}</span>
                                <input class="input" type="text" node_ref=input_ref spellcheck="false" autocomplete="off"
                                    prop:value=move || value.get()
                                    on:input=move |e| value.set(event_target_value(&e))/>
                            </label>
                        </Show>
                        <div class="dialog-actions">
                            <button type="button" class="btn" node_ref=cancel_ref on:click=move |_| close()>{ui.t("action-cancel")}</button>
                            <button type="submit" node_ref=confirm_ref class="btn"
                                class:btn--primary=!a.danger class:btn--danger=a.danger
                                disabled=move || has_input && value.with(|v| v.trim().is_empty())>{a.confirm.clone()}</button>
                        </div>
                    </form>
                </div>
            }
        })
    }
}

/// Clic sur le fond d'une fenêtre : vrai seulement si rien n'y a été saisi (le fond garde la
/// marque `data-dirty` posée par [`mark_dirty`]) ; sinon la fenêtre reste ouverte (Échap ou
/// Annuler la ferment) pour ne pas perdre la saisie.
pub fn backdrop_click(e: &web_sys::MouseEvent) -> bool {
    use wasm_bindgen::JsCast;
    if e.target() != e.current_target() {
        return false;
    }
    let dirty = e
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .is_some_and(|el| el.has_attribute("data-dirty"));
    if dirty {
        // Signe que la fenêtre reste ouverte : la boîte tremble un instant.
        if let Some(el) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.first_element_child())
        {
            let _ = el.class_list().remove_1("is-nudged");
            let _ = el.get_bounding_client_rect();
            let _ = el.class_list().add_1("is-nudged");
        }
    }
    !dirty
}

/// Saisie dans une fenêtre (événement `input` remonté jusqu'au fond) : fond marqué.
pub fn mark_dirty(e: &web_sys::Event) {
    use wasm_bindgen::JsCast;
    if let Some(el) = e.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
        let _ = el.set_attribute("data-dirty", "");
    }
}
