//! Profil (D-106) : photo et nom de la personne qui travaille dans Revel, en haut à droite de la
//! barre supérieure et dans Réglages › Comptes.
//!
//! - nom : celui qui signe les versions (D-89) ; sans nom choisi, le nom du compte GitHub connecté ;
//! - photo : image choisie sur l'ordinateur (recadrée en carré, gardée dans les réglages) ; sans
//!   photo choisie, celle du compte GitHub connecté ; sinon les initiales.

use crate::{Route, Ui, icons};
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// Côté de la photo gardée (pixels).
const SIDE: u32 = 160;

/// Photo du compte GitHub connecté (adresse publique, redirigée vers l'image du compte).
fn github_picture(login: &str) -> String {
    format!("https://github.com/{login}.png?size=96")
}

/// Photo montrée : choisie, sinon celle du compte GitHub.
pub fn picture(ui: Ui) -> Option<String> {
    ui.settings
        .with(|s| s.avatar.clone())
        .or_else(|| ui.github.get().map(|l| github_picture(&l)))
}

/// Nom montré : choisi, sinon celui du compte GitHub.
pub fn name(ui: Ui) -> Option<String> {
    ui.settings
        .with(|s| s.author.clone())
        .filter(|n| !n.trim().is_empty())
        .or_else(|| ui.github.get())
}

/// Initiales d'un nom (« Anne Martin » → « AM »).
fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect()
}

/// Pastille de la photo (ou des initiales, ou d'une silhouette) ; `size` : `sm` ou `lg`.
pub fn avatar(ui: Ui, size: &'static str) -> impl IntoView {
    // Photo du compte GitHub injoignable (hors ligne) : initiales à la place.
    let broken = RwSignal::new(None::<String>);
    view! {
        <span class=format!("avatar avatar--{size}") aria-hidden="true">
            {move || match picture(ui).filter(|p| broken.with(|b| b.as_deref() != Some(p.as_str()))) {
                Some(src) => {
                    let s2 = src.clone();
                    view! { <img src=src alt="" draggable="false" on:error=move |_| broken.set(Some(s2.clone()))/> }.into_any()
                }
                None => match name(ui).map(|n| initials(&n)).filter(|i| !i.is_empty()) {
                    Some(i) => view! { <span class="avatar-initials">{i}</span> }.into_any(),
                    None => icons::user(),
                },
            }}
        </span>
    }
}

/// Image choisie recadrée en carré au centre, réduite à `SIDE` pixels (WebP, sinon PNG).
fn square(img: &web_sys::HtmlImageElement) -> Option<String> {
    let (w, h) = (
        f64::from(img.natural_width()),
        f64::from(img.natural_height()),
    );
    if w < 1.0 || h < 1.0 {
        return None;
    }
    let canvas: web_sys::HtmlCanvasElement =
        document().create_element("canvas").ok()?.dyn_into().ok()?;
    canvas.set_width(SIDE);
    canvas.set_height(SIDE);
    let ctx: web_sys::CanvasRenderingContext2d = canvas.get_context("2d").ok()??.dyn_into().ok()?;
    let side = w.min(h);
    ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
        img,
        (w - side) / 2.0,
        (h - side) / 2.0,
        side,
        side,
        0.0,
        0.0,
        f64::from(SIDE),
        f64::from(SIDE),
    )
    .ok()?;
    canvas.to_data_url_with_type("image/webp").ok()
}

/// Photo lue depuis un fichier de l'ordinateur, gardée dans les réglages.
fn use_file(ui: Ui, file: web_sys::File) {
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&file) else {
        return;
    };
    let Ok(img) = web_sys::HtmlImageElement::new() else {
        return;
    };
    let (img2, url2) = (img.clone(), url.clone());
    let done = Closure::once(move || {
        let _ = web_sys::Url::revoke_object_url(&url2);
        match square(&img2) {
            Some(data) => crate::settings::update(ui, &|s| s.avatar = Some(data.clone())),
            None => ui.fail(revel_protocol::Failure::new("error-profile-picture")),
        }
    });
    img.set_onload(Some(done.as_ref().unchecked_ref()));
    let failed =
        Closure::once(move || ui.fail(revel_protocol::Failure::new("error-profile-picture")));
    img.set_onerror(Some(failed.as_ref().unchecked_ref()));
    done.forget();
    failed.forget();
    img.set_src(&url);
}

/// Photo : pastille, « Changer la photo », retour à la photo GitHub ou retrait.
pub fn picture_controls(ui: Ui) -> impl IntoView {
    let input = NodeRef::<html::Input>::new();
    let custom = move || ui.settings.with(|s| s.avatar.is_some());
    view! {
        <div class="profile-picture">
            {avatar(ui, "lg")}
            <div class="profile-picture-actions">
                <button type="button" class="btn btn--small profile-pick"
                    on:click=move |_| if let Some(i) = input.get() { i.click() }>
                    {icons::camera()}{ui.t("profile-picture-change")}
                </button>
                <Show when=custom>
                    <button type="button" class="link-btn profile-picture-reset"
                        on:click=move |_| crate::settings::update(ui, &|s| s.avatar = None)>
                        {move || ui.tx(if ui.github.with(Option::is_some) { "profile-picture-github" } else { "profile-picture-remove" }, &[])}
                    </button>
                </Show>
            </div>
            <input type="file" accept="image/png,image/jpeg,image/webp,image/gif,image/avif" class="profile-file" hidden=true
                tabindex="-1" aria-hidden="true" node_ref=input
                on:change=move |e| {
                    let el = event_target::<web_sys::HtmlInputElement>(&e);
                    if let Some(f) = el.files().and_then(|l| l.get(0)) {
                        use_file(ui, f);
                    }
                    el.set_value("");
                }/>
        </div>
    }
}

/// Nom du profil (signe les versions) ; vide : nom du compte GitHub.
pub fn name_input(ui: Ui, id: &'static str) -> impl IntoView {
    view! {
        <input id=id class="input settings-author" spellcheck="false" autocomplete="name"
            placeholder=move || ui.github.get().unwrap_or_else(|| ui.tx("settings-author-placeholder", &[]))
            title=ui.t("settings-author-help")
            prop:value=move || ui.settings.with(|s| s.author.clone().unwrap_or_default())
            on:change=move |e| {
                let v = event_target_value(&e).trim().to_string();
                crate::settings::update(ui, &|s| s.author = (!v.is_empty()).then(|| v.clone()));
            }/>
    }
}

/// Profil en haut à droite : la photo ; fenêtre pour changer la photo, le nom et le compte GitHub
/// sans quitter ce qu'on fait.
#[component]
pub fn ProfileButton() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let open = RwSignal::new(false);
    let on_down =
        window_event_listener(leptos::ev::pointerdown, move |e: web_sys::PointerEvent| {
            let inside = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .and_then(|el| el.closest(".profile").ok().flatten())
                .is_some();
            // Boîte de dialogue ouverte depuis le profil (erreur, choix) : le profil reste ouvert.
            if !inside && ui.ask.with_untracked(Option::is_none) {
                open.set(false);
            }
        });
    let on_key = window_event_listener(leptos::ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" {
            open.set(false);
        }
    });
    on_cleanup(move || {
        on_down.remove();
        on_key.remove();
    });
    let shown = move || name(ui).unwrap_or_else(|| ui.tx("profile-title", &[]));
    view! {
        <div class="profile">
            <button class="profile-btn" aria-haspopup="dialog" aria-expanded=move || open.get().to_string()
                title=shown aria-label=ui.t("profile-title")
                on:click=move |_| open.update(|o| *o = !*o)>
                // Photo seule (le nom est dans la bulle d'aide et dans la fenêtre du profil).
                {avatar(ui, "sm")}
            </button>
            <Show when=move || open.get()>
                <div class="menu profile-pop" role="dialog" aria-label=ui.t("profile-title")>
                    {picture_controls(ui)}
                    <label class="field">
                        <span class="label" title=ui.t("settings-author-help")>{ui.t("settings-author")}</span>
                        {name_input(ui, "profile-name")}
                    </label>
                    <div class="field profile-github">
                        <span class="label" title=ui.t("settings-github-help")>"GitHub"</span>
                        <crate::account::GithubAccount/>
                    </div>
                    <div class="menu-sep"></div>
                    <button class="menu-item" role="menuitem" on:click=move |_| { open.set(false); ui.route.set(Route::Settings) }>
                        {icons::settings()}<span class="menu-label">{ui.t("nav-app-settings")}</span>
                    </button>
                </div>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::initials;

    #[test]
    fn initiales() {
        assert_eq!(initials("Anne Martin"), "AM");
        assert_eq!(initials("matv-studio"), "M");
        assert_eq!(initials("élise de la Tour"), "ÉD");
        assert_eq!(initials("  "), "");
    }
}
