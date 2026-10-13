//! Interface de Revel (Leptos) : coque (barre supérieure, barre latérale) et navigation.

mod account;
mod api;
mod dialog;
mod home;
mod icons;
mod publish;
mod settings;
mod site;

use dialog::Ask;
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_i18n::Localizer;
use revel_protocol::{AppSettings, Failure, RecentSite, SiteSummary};
use std::cell::RefCell;
use wasm_bindgen::{JsCast, JsValue};

thread_local! {
    static L: RefCell<Option<Localizer>> = const { RefCell::new(None) };
}

/// Traduction dans une langue donnée.
pub fn tr(lang: &str, key: &str, args: &[(&str, String)]) -> String {
    L.with(|l| {
        let mut l = l.borrow_mut();
        if l.as_ref().map(|x| x.lang() != lang).unwrap_or(true) {
            *l = Some(Localizer::new(lang));
        }
        let args: Vec<(&str, &str)> = args.iter().map(|(k, v)| (*k, v.as_str())).collect();
        l.as_ref()
            .map(|x| x.t(key, &args))
            .unwrap_or_else(|| key.to_string())
    })
}

/// Texte d'un échec dans une langue donnée.
pub fn tr_failure(lang: &str, f: &Failure) -> String {
    let args: Vec<(&str, String)> = f
        .args
        .iter()
        .map(|(k, v)| (k.as_str(), v.clone()))
        .collect();
    tr(lang, &f.key, &args)
}

/// Nombre passé en paramètre d'un texte traduit, arrondi à `decimals` chiffres après la
/// virgule. La mise en forme (milliers, virgule décimale) et les pluriels relèvent de
/// `revel-i18n`, selon la langue.
pub fn num(value: f64, decimals: i32) -> String {
    let p = 10f64.powi(decimals);
    ((value * p).round() / p).to_string()
}

/// Date RFC 3339 en date et heure locales, mises en forme selon la langue.
pub fn fmt_datetime(lang: &str, rfc3339: &str) -> String {
    let d = js_sys::Date::new(&JsValue::from_str(rfc3339));
    if d.get_time().is_nan() {
        return rfc3339.to_string();
    }
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"dateStyle".into(), &"medium".into());
    let _ = js_sys::Reflect::set(&opts, &"timeStyle".into(), &"short".into());
    d.to_locale_string(lang, &opts)
        .as_string()
        .unwrap_or_default()
}

/// Contexte partagé de l'interface.
#[derive(Clone, Copy)]
pub struct Ui {
    pub settings: RwSignal<AppSettings>,
    pub route: RwSignal<Route>,
    pub error: RwSignal<Option<Failure>>,
    pub recents: RwSignal<Vec<RecentSite>>,
    /// Activité en cours (génération, publication) : fraction 0..1, `None` si aucune.
    pub activity: RwSignal<Option<f32>>,
    /// Environnement choisi en dernier (génération), utilisé par l'aperçu de la barre supérieure.
    pub environment: RwSignal<Option<String>>,
    /// Boîte de dialogue ouverte (confirmation, saisie).
    pub ask: RwSignal<Option<Ask>>,
    /// Compte GitHub connecté (identifiant).
    pub github: RwSignal<Option<String>>,
}

impl Ui {
    /// Texte traduit, suivi réactivement de la langue.
    pub fn t(self, key: &'static str) -> impl Fn() -> String + Send + Sync + 'static {
        let s = self.settings;
        move || tr(&s.with(|s| s.lang.clone()), key, &[])
    }
    /// Texte traduit maintenant (dans un gestionnaire d'événement).
    pub fn now(self, key: &str, args: &[(&str, String)]) -> String {
        tr(&self.settings.with_untracked(|s| s.lang.clone()), key, args)
    }
    /// Texte traduit, réactif, avec paramètres.
    pub fn tx(self, key: &str, args: &[(&str, String)]) -> String {
        tr(&self.settings.with(|s| s.lang.clone()), key, args)
    }
    pub fn lang(self) -> String {
        self.settings.with(|s| s.lang.clone())
    }
    /// Texte d'un échec (réactif).
    pub fn failure(self, f: &Failure) -> String {
        tr_failure(&self.lang(), f)
    }
    pub fn fail(self, e: Failure) {
        self.error.set(Some(e));
    }
    /// Environnement de l'aperçu : dernier choisi s'il existe pour ce site, sinon le premier.
    pub fn preview_environment(self, site: &SiteSummary) -> String {
        self.environment
            .get_untracked()
            .filter(|e| site.environments.contains(e))
            .or_else(|| site.environments.first().cloned())
            .unwrap_or_else(|| "production".into())
    }
    pub fn reload_recents(self) {
        spawn_local(async move {
            match api::recent_sites().await {
                Ok(r) => self.recents.set(r),
                Err(e) => self.fail(e),
            }
        });
    }
    pub fn open(self, site: SiteSummary) {
        self.route.set(Route::Site {
            site,
            tab: Tab::Overview,
        });
        self.reload_recents();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Build,
    Publish,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    Home,
    Settings,
    Site { site: SiteSummary, tab: Tab },
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

/// Applique le thème, la langue et la couleur d'accent au document.
fn apply_settings(s: &AppSettings) {
    let Some(root) = document().document_element() else {
        return;
    };
    let _ = match s.theme.as_str() {
        "dark" | "light" => root.set_attribute("data-theme", &s.theme),
        _ => root.remove_attribute("data-theme"),
    };
    let _ = root.set_attribute("lang", &s.lang);
    if let Ok(el) = root.dyn_into::<web_sys::HtmlElement>() {
        let style = el.style();
        match &s.accent {
            Some(c) if c.len() == 7 && c.starts_with('#') => {
                let _ = style.set_property("--accent", c);
                let _ = style.set_property("--accent-ink", accent_ink(c));
            }
            _ => {
                let _ = style.remove_property("--accent");
                let _ = style.remove_property("--accent-ink");
            }
        }
    }
}

/// Texte lisible sur la couleur d'accent (noir ou blanc selon la luminance relative).
fn accent_ink(hex: &str) -> &'static str {
    let c = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0) as f32 / 255.0;
    let lin = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let l = 0.2126 * lin(c(1)) + 0.7152 * lin(c(3)) + 0.0722 * lin(c(5));
    if l > 0.4 { "#111111" } else { "#ffffff" }
}

#[component]
fn App() -> impl IntoView {
    let ui = Ui {
        settings: RwSignal::new(AppSettings::default()),
        route: RwSignal::new(Route::Home),
        error: RwSignal::new(None),
        recents: RwSignal::new(Vec::new()),
        activity: RwSignal::new(None),
        environment: RwSignal::new(None),
        ask: RwSignal::new(None),
        github: RwSignal::new(None),
    };
    provide_context(ui);
    spawn_local(async move {
        if let Ok(s) = api::settings().await {
            ui.settings.set(s);
        }
    });
    ui.reload_recents();
    spawn_local(async move {
        if let Ok(a) = api::github_account().await {
            ui.github.set(a);
        }
    });
    Effect::new(move |_| ui.settings.with(apply_settings));

    view! {
        <div class="shell">
            <TopBar/>
            <SideBar/>
            <main class="main">
                <Show when=move || ui.error.get().is_some()>
                    <div class="notice notice--error" role="alert">
                        <span>{move || ui.error.get().map(|f| ui.failure(&f))}</span>
                        <button class="icon-btn" title=ui.t("action-close") aria-label=ui.t("action-close") on:click=move |_| ui.error.set(None)>{icons::close()}</button>
                    </div>
                </Show>
                {move || match ui.route.get() {
                    Route::Home => view! { <home::Home/> }.into_any(),
                    Route::Settings => view! { <settings::Settings/> }.into_any(),
                    Route::Site { site, tab } => match tab {
                        Tab::Overview => view! { <site::Overview site=site/> }.into_any(),
                        Tab::Build => view! { <site::Build site=site/> }.into_any(),
                        Tab::Publish => view! { <publish::Publish site=site/> }.into_any(),
                    },
                }}
            </main>
            <dialog::Dialog/>
        </div>
    }
}

#[component]
fn TopBar() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let crumb = move || match ui.route.get() {
        Route::Home => String::new(),
        Route::Settings => ui.tx("nav-settings", &[]),
        Route::Site { site, .. } => site.name,
    };
    let site_open = move || matches!(ui.route.get(), Route::Site { .. });
    let go_publish = move |_| {
        if let Route::Site { site, .. } = ui.route.get_untracked() {
            ui.route.set(Route::Site {
                site,
                tab: Tab::Publish,
            });
        }
    };
    let preview = move |_| {
        if let Route::Site { site, .. } = ui.route.get_untracked() {
            let env = ui.preview_environment(&site);
            spawn_local(async move {
                if let Err(e) = api::preview(&site.path, &env).await {
                    ui.fail(e);
                }
            });
        }
    };
    view! {
        <header class="topbar">
            <button class="logo" aria-label="Revel" on:click=move |_| ui.route.set(Route::Home)>{icons::logo()}</button>
            <nav class="crumbs">
                <Show when=site_open>
                    <button class="crumb" on:click=move |_| ui.route.set(Route::Home)>{ui.t("nav-sites")}</button>
                    <span class="crumb-sep" aria-hidden="true">"/"</span>
                </Show>
                <span class="crumb crumb--current">{crumb}</span>
            </nav>
            <div class="topbar-actions">
                <Show when=site_open>
                    <button class="btn" on:click=preview>{icons::play()}{ui.t("preview-open")}</button>
                    <button class="btn btn--primary" on:click=go_publish>{ui.t("nav-publish")}</button>
                </Show>
            </div>
            <div class="activity" class:activity--on=move || ui.activity.get().is_some()>
                <div class="activity-bar" style:width=move || format!("{}%", (ui.activity.get().unwrap_or(0.0) * 100.0).max(4.0))></div>
            </div>
        </header>
    }
}

#[component]
fn SideBar() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let current_path = move || match ui.route.get() {
        Route::Site { site, .. } => Some(site.path),
        _ => None,
    };
    let current_tab = move || match ui.route.get() {
        Route::Site { tab, .. } => Some(tab),
        _ => None,
    };
    let open_path = move |path: String| {
        spawn_local(async move {
            match api::open_site(&path).await {
                Ok(s) => ui.open(s),
                Err(e) => ui.fail(e),
            }
        })
    };
    let tab_item = move |tab: Tab, key: &'static str, icon: fn() -> AnyView| {
        view! {
            <button class="nav-item nav-item--sub" class:is-active=move || current_tab() == Some(tab)
                on:click=move |_| {
                    if let Route::Site { site, .. } = ui.route.get_untracked() {
                        ui.route.set(Route::Site { site, tab });
                    }
                }>
                {icon()}<span>{ui.t(key)}</span>
            </button>
        }
    };
    view! {
        <aside class="sidebar">
            <div class="sidebar-section">
                <button class="nav-item" class:is-active=move || ui.route.get() == Route::Home
                    on:click=move |_| ui.route.set(Route::Home)>
                    {icons::grid()}<span>{ui.t("nav-all-sites")}</span>
                </button>
            </div>
            <div class="sidebar-section sidebar-sites">
                <p class="sidebar-title">{ui.t("nav-recent")}</p>
                <For each=move || ui.recents.get() key=|r| r.path.clone() let:r>
                    {
                        let path = r.path.clone();
                        let p_active = r.path.clone();
                        let p_show = r.path.clone();
                        view! {
                            <button class="nav-item"
                                class:is-active=move || current_path().as_deref() == Some(p_active.as_str()) && current_tab() == Some(Tab::Overview)
                                title=r.path.clone()
                                on:click=move |_| open_path(path.clone())>
                                {icons::site()}<span class="nav-label">{r.name.clone()}</span>
                            </button>
                            <Show when=move || current_path().as_deref() == Some(p_show.as_str())>
                                {tab_item(Tab::Build, "nav-build", icons::build)}
                                {tab_item(Tab::Publish, "nav-publish", icons::upload)}
                            </Show>
                        }
                    }
                </For>
            </div>
            <div class="sidebar-section sidebar-bottom">
                <button class="nav-item" class:is-active=move || ui.route.get() == Route::Settings
                    on:click=move |_| ui.route.set(Route::Settings)>
                    {icons::settings()}<span>{ui.t("nav-settings")}</span>
                </button>
            </div>
        </aside>
    }
}
