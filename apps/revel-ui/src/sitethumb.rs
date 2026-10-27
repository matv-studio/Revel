//! Vignette d'un site dans l'accueil : couleur choisie (teinte tirée du nom par défaut) ou image
//! de sa page d'accueil, prise dans le canevas (cadre caché) et reprise quand le site a changé.

use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::RecentSite;
use std::cell::Cell;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;

/// Couleurs proposées (comme les étiquettes de Webflow) ; une autre au sélecteur.
pub const SWATCHES: [&str; 10] = [
    "#ff5c35", "#f5a524", "#e5c100", "#3fb950", "#14b8a6", "#3b82f6", "#6366f1", "#a855f7",
    "#ec4899", "#71717a",
];

/// Teinte (0 à 359) d'une couleur `#rrggbb`.
pub fn hue_of(hex: &str) -> Option<u32> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let c = |i: usize| {
        u8::from_str_radix(&h[i..i + 2], 16)
            .ok()
            .map(|v| f64::from(v) / 255.0)
    };
    let (r, g, b) = (c(0)?, c(2)?, c(4)?);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    if d == 0.0 {
        return Some(0);
    }
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    Some((h * 60.0).round() as u32 % 360)
}

/// Teinte stable dérivée du nom (couleur de la vignette par défaut).
pub fn hue_of_name(name: &str) -> u32 {
    name.bytes()
        .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)))
        % 360
}

/// Variables de style de la carte d'un site (`--hue`, `--thumb` si une couleur est choisie).
pub fn style(r: &RecentSite) -> String {
    match r.color.as_deref() {
        Some(c) => format!("--hue:{};--thumb:{c}", hue_of(c).unwrap_or(0)),
        None => format!("--hue:{}", hue_of_name(&r.name)),
    }
}

/// Clé d'affichage d'une carte : refaite quand sa vignette change.
pub fn key(r: &RecentSite) -> String {
    format!(
        "{}|{:?}|{}|{}",
        r.path,
        r.color,
        r.homepage,
        r.image.as_ref().map_or(0, String::len)
    )
}

thread_local! {
    /// Une prise à la fois (le canevas ne sert qu'un site).
    static BUSY: Cell<bool> = const { Cell::new(false) };
}

/// Image de la page d'accueil de `path` (JPEG, adresse `data:`) : site ouvert dans le canevas,
/// page chargée dans un cadre caché de 1280 × 800, image demandée au canevas.
pub async fn capture(path: &str) -> Option<String> {
    api::edit_open(path).await.ok()?;
    let base = api::canvas_open(path, false).await.ok()?;
    let doc = document();
    let frame: web_sys::HtmlIFrameElement = doc.create_element("iframe").ok()?.dyn_into().ok()?;
    let _ = frame.set_attribute(
        "style",
        "position:fixed;left:0;top:0;width:1280px;height:800px;border:0;opacity:0;z-index:-1;pointer-events:none",
    );
    let _ = frame.set_attribute("aria-hidden", "true");
    let _ = frame.set_attribute("tabindex", "-1");
    let _ = frame.set_attribute("class", "site-shot-frame");
    let (load_tx, load_rx) = futures_channel::oneshot::channel::<()>();
    let load_tx = Cell::new(Some(load_tx));
    let on_load = Closure::<dyn FnMut()>::new(move || {
        if let Some(t) = load_tx.take() {
            let _ = t.send(());
        }
    });
    frame.set_onload(Some(on_load.as_ref().unchecked_ref()));
    frame.set_src(&format!("{base}?rv=thumb"));
    doc.body()?.append_child(&frame).ok()?;
    let done = |frame: &web_sys::HtmlIFrameElement| frame.remove();
    // Page chargée (20 s au plus), polices et images ensuite.
    let loaded = futures_select(load_rx, 20_000).await;
    drop(on_load);
    if !loaded {
        done(&frame);
        return None;
    }
    crate::sleep_ms(700).await;
    let Some(win) = frame.content_window() else {
        done(&frame);
        return None;
    };
    let (tx, rx) = futures_channel::oneshot::channel::<Option<String>>();
    let tx = Cell::new(Some(tx));
    // Cadre d'une autre origine : comparé tel quel (pas `instanceof Window`).
    let src = wasm_bindgen::JsValue::from(win.clone());
    let on_msg =
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
            if e.source().map(wasm_bindgen::JsValue::from).as_ref() != Some(&src) {
                return;
            }
            let d = e.data();
            let get = |k: &str| js_sys::Reflect::get(&d, &k.into()).ok();
            if get("type").and_then(|t| t.as_string()).as_deref() != Some("shot") {
                return;
            }
            if let Some(t) = tx.take() {
                let _ = t.send(get("data").and_then(|v| v.as_string()));
            }
        });
    let _ = window().add_event_listener_with_callback("message", on_msg.as_ref().unchecked_ref());
    let msg = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&msg, &"revel".into(), &1.into());
    let _ = js_sys::Reflect::set(&msg, &"type".into(), &"shot".into());
    let _ = js_sys::Reflect::set(&msg, &"width".into(), &640.into());
    let _ = win.post_message(&msg, "*");
    let data = futures_select_value(rx, 30_000).await.flatten();
    let _ =
        window().remove_event_listener_with_callback("message", on_msg.as_ref().unchecked_ref());
    drop(on_msg);
    done(&frame);
    data.filter(|d| d.starts_with("data:image/jpeg"))
}

/// Attend `rx` au plus `ms` millisecondes ; vrai s'il a répondu.
async fn futures_select(rx: futures_channel::oneshot::Receiver<()>, ms: i32) -> bool {
    futures_select_value(rx, ms).await.is_some()
}

async fn futures_select_value<T>(
    mut rx: futures_channel::oneshot::Receiver<T>,
    ms: i32,
) -> Option<T> {
    let mut waited = 0;
    loop {
        match rx.try_recv() {
            Ok(Some(v)) => return Some(v),
            Ok(None) => {}
            Err(_) => return None,
        }
        if waited >= ms {
            return None;
        }
        crate::sleep_ms(100).await;
        waited += 100;
    }
}

/// Reprend les images des pages d'accueil absentes ou dépassées (une à la fois), puis relit la
/// liste des sites.
pub fn refresh_stale(ui: Ui) {
    if BUSY.with(Cell::get) {
        return;
    }
    let todo: Vec<String> = ui.recents.with_untracked(|rs| {
        rs.iter()
            .filter(|r| r.homepage && r.image_stale)
            .map(|r| r.path.clone())
            .collect()
    });
    if todo.is_empty() {
        return;
    }
    BUSY.with(|b| b.set(true));
    spawn_local(async move {
        let mut any = false;
        for p in todo {
            // Site ouvert entre-temps : sa session n'est pas remplacée.
            if !matches!(ui.route.get_untracked(), crate::Route::Home) {
                break;
            }
            if let Some(data) = capture(&p).await
                && api::save_site_thumb(&p, &data).await.is_ok()
            {
                any = true;
            }
        }
        BUSY.with(|b| b.set(false));
        if any {
            ui.reload_recents();
        }
    });
}

/// Boîte « Vignette » d'un site : couleur (proposées, autre, d'après le nom) ou page d'accueil.
#[component]
pub fn ThumbDialog(site: RecentSite, close: Callback<()>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let path = StoredValue::new(site.path.clone());
    let color = RwSignal::new(site.color.clone());
    let homepage = RwSignal::new(site.homepage);
    let image = RwSignal::new(site.image.clone());
    let busy = RwSignal::new(false);
    let name = site.name.clone();
    let auto_hue = hue_of_name(&name);
    let save = move || {
        let p = path.get_value();
        let (c, h) = (color.get_untracked(), homepage.get_untracked());
        spawn_local(async move {
            match api::set_site_thumb(&p, c.as_deref(), h).await {
                Ok(()) => ui.reload_recents(),
                Err(e) => ui.fail(e),
            }
        });
    };
    let shoot = move || {
        if busy.get_untracked() || BUSY.with(Cell::get) {
            return;
        }
        busy.set(true);
        BUSY.with(|b| b.set(true));
        let p = path.get_value();
        spawn_local(async move {
            let data = capture(&p).await;
            BUSY.with(|b| b.set(false));
            match data {
                Some(d) => {
                    if let Err(e) = api::save_site_thumb(&p, &d).await {
                        ui.fail(e);
                    }
                    let _ = image.try_set(Some(d));
                    ui.reload_recents();
                }
                None => ui.fail(revel_protocol::Failure::new("error-site-thumb")),
            }
            let _ = busy.try_set(false);
        });
    };
    let set_mode = move |home: bool| {
        homepage.set(home);
        save();
        if home && image.get_untracked().is_none() {
            shoot();
        }
    };
    let pick = move |c: Option<String>| {
        color.set(c);
        homepage.set(false);
        save();
    };
    let preview_style = move || match color.get() {
        Some(c) => format!("--hue:{};--thumb:{c}", hue_of(&c).unwrap_or(0)),
        None => format!("--hue:{auto_hue}"),
    };
    let mono = crate::home::mono(&name);
    let on_key = window_event_listener(leptos::ev::keydown, move |e| {
        if e.key() == "Escape" {
            close.run(());
        }
    });
    on_cleanup(move || on_key.remove());
    view! {
        <div class="overlay" on:mousedown=move |e| if crate::dialog::backdrop_click(&e) { close.run(()) }>
            <div class="dialog dialog--form site-thumb-dialog" role="dialog" aria-modal="true" aria-labelledby="site-thumb-title">
                <div class="setup-head">
                    <h2 id="site-thumb-title">{ui.t("site-thumb")}</h2>
                    <button type="button" class="icon-btn" title=ui.t("action-close") aria-label=ui.t("action-close")
                        on:click=move |_| close.run(())>{icons::close()}</button>
                </div>
                <div class="site-card site-thumb-preview" style=preview_style>
                    <div class="site-thumb">
                        {move || match (homepage.get(), image.get()) {
                            (true, Some(src)) => view! { <img class="site-shot" src=src alt=""/> }.into_any(),
                            _ => view! { <span class="site-mono">{mono.clone()}</span> }.into_any(),
                        }}
                        <Show when=move || busy.get()>
                            <span class="site-shot-busy" role="status" aria-label=ui.t("site-thumb-taking")>{icons::sync()}</span>
                        </Show>
                    </div>
                </div>
                <div class="segmented site-thumb-mode" role="radiogroup" aria-label=ui.t("site-thumb")>
                    <button type="button" role="radio" data-mode="color" class:is-on=move || !homepage.get()
                        aria-checked=move || (!homepage.get()).to_string() on:click=move |_| set_mode(false)>
                        {icons::palette()}<span>{ui.t("site-thumb-color")}</span></button>
                    <button type="button" role="radio" data-mode="homepage" class:is-on=move || homepage.get()
                        aria-checked=move || homepage.get().to_string() on:click=move |_| set_mode(true)>
                        {icons::image()}<span>{ui.t("site-thumb-homepage")}</span></button>
                </div>
                <Show when=move || !homepage.get() fallback=move || view! {
                    <button type="button" class="btn btn--small site-thumb-retake" prop:disabled=move || busy.get()
                        title=ui.t("site-thumb-retake-help") on:click=move |_| shoot()>
                        {icons::sync()}<span>{ui.t("site-thumb-retake")}</span></button>
                }>
                    <div class="site-thumb-swatches" role="radiogroup" aria-label=ui.t("site-thumb-color")>
                        <button type="button" role="radio" class="site-swatch site-swatch--auto" data-color=""
                            style=format!("--hue:{auto_hue}")
                            class:is-on=move || color.get().is_none() aria-checked=move || color.get().is_none().to_string()
                            title=ui.t("site-thumb-auto") aria-label=ui.t("site-thumb-auto")
                            on:click=move |_| pick(None)></button>
                        {SWATCHES.iter().map(|c| {
                            let c = c.to_string();
                            let (c1, c2, c3) = (c.clone(), c.clone(), c.clone());
                            view! {
                                <button type="button" role="radio" class="site-swatch" data-color=c.clone()
                                    style=format!("background:{c}")
                                    class:is-on=move || color.get().as_deref() == Some(c1.as_str())
                                    aria-checked=move || (color.get().as_deref() == Some(c2.as_str())).to_string()
                                    aria-label=c.clone() title=c.clone()
                                    on:click=move |_| pick(Some(c3.clone()))></button>
                            }
                        }).collect_view()}
                        <label class="site-swatch site-swatch--custom" title=ui.t("site-thumb-custom")>
                            <input type="color" aria-label=ui.t("site-thumb-custom")
                                prop:value=move || color.get().unwrap_or_else(|| "#ff5c35".into())
                                on:change=move |e| pick(Some(event_target_value(&e)))/>
                            {icons::plus()}
                        </label>
                    </div>
                </Show>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teintes() {
        assert_eq!(hue_of("#ff0000"), Some(0));
        assert_eq!(hue_of("#00ff00"), Some(120));
        assert_eq!(hue_of("#0000ff"), Some(240));
        assert_eq!(hue_of("#ff5c35"), Some(12));
        assert_eq!(hue_of("bleu"), None);
    }
}
