//! Interface de Revel (Leptos) : coque (barre supérieure, barre latérale) et navigation.

mod account;
mod api;
mod canvas;
mod cms;
mod dialog;
mod dock;
mod editor;
mod home;
mod icons;
mod insert;
mod inspector;
mod layers;
mod panels;
mod publish;
mod richtext;
mod settings;
mod site;
mod styles;

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
    /// Page à afficher dans l'éditeur (demandée par une autre vue, la vue CMS par exemple).
    pub goto_page: RwSignal<Option<String>>,
    /// Élément copié (Ctrl+C), collé avec de nouveaux identifiants (Ctrl+V), d'un site à l'autre.
    pub clipboard: RwSignal<Option<revel_model::Node>>,
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
    /// Ouvre un site dans l'éditeur.
    pub fn open(self, site: SiteSummary) {
        self.open_at(site, Tab::Editor);
    }
    pub fn open_at(self, site: SiteSummary, tab: Tab) {
        self.route.set(Route::Site { site, tab });
        self.reload_recents();
    }
    /// Change de vue dans le site ouvert.
    pub fn tab(self, tab: Tab) {
        if let Route::Site { site, .. } = self.route.get_untracked() {
            self.route.set(Route::Site { site, tab });
        }
    }
    /// Ouvre un site par son dossier (vue donnée).
    pub fn open_path(self, path: String, tab: Tab) {
        spawn_local(async move {
            match api::open_site(&path).await {
                Ok(s) => self.open_at(s, tab),
                Err(e) => self.fail(e),
            }
        })
    }
}

/// Vue d'un site ouvert (barre supérieure).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Editor,
    Cms,
    Locales,
    Settings(Section),
}

/// Rubrique des paramètres d'un site.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Section {
    #[default]
    General,
    Publish,
    Export,
    Seo,
    Redirects,
    Forms,
    Fonts,
    Code,
    Backups,
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
        goto_page: RwSignal::new(None),
        clipboard: RwSignal::new(None),
    };
    provide_context(ui);
    spawn_local(async move {
        if let Ok(s) = api::settings().await {
            ui.settings.set(s);
        }
    });
    ui.reload_recents();
    // Ouverture directe dans l'éditeur (`--open` ou `REVEL_OPEN` : essais de l'application).
    spawn_local(async move {
        if let Ok(Some(path)) = api::startup_site().await {
            match api::open_site(&path).await {
                Ok(site) => {
                    ui.route.set(Route::Site {
                        site,
                        tab: Tab::Editor,
                    });
                    ui.reload_recents();
                }
                Err(e) => ui.fail(e),
            }
        }
    });
    spawn_local(async move {
        if let Ok(a) = api::github_account().await {
            ui.github.set(a);
        }
    });
    Effect::new(move |_| ui.settings.with(apply_settings));

    // Page affichée : accueil, réglages, ou site ouvert (identifié par son dossier : changer de
    // vue dans le site ne recrée pas l'éditeur).
    let page = Memo::new(move |_| match ui.route.get() {
        Route::Home => Page::Home,
        Route::Settings => Page::Settings,
        Route::Site { site, .. } => Page::Site(site.path),
    });
    // Teinte du haut du plateau (raccord de l'onglet actif) : barre d'outils ou page.
    let stage = move || match ui.route.get() {
        Route::Site {
            tab: Tab::Editor | Tab::Cms,
            ..
        } => "tool",
        _ => "page",
    };
    view! {
        <div class="shell" data-stage=stage>
            <TopBar/>
            <main class="main">
                <Show when=move || ui.error.get().is_some()>
                    <div class="notice notice--error" role="alert">
                        <span>{move || ui.error.get().map(|f| ui.failure(&f))}</span>
                        <button class="icon-btn" title=ui.t("action-close") aria-label=ui.t("action-close") on:click=move |_| ui.error.set(None)>{icons::close()}</button>
                    </div>
                </Show>
                {move || match page.get() {
                    Page::Home => view! { <home::Home/> }.into_any(),
                    Page::Settings => view! { <settings::Settings/> }.into_any(),
                    Page::Site(_) => match ui.route.get_untracked() {
                        Route::Site { site, .. } => view! { <site::SiteView site=site/> }.into_any(),
                        _ => ().into_any(),
                    },
                }}
            </main>
            <dialog::Dialog/>
        </div>
    }
}

#[derive(Clone, PartialEq)]
enum Page {
    Home,
    Settings,
    Site(String),
}

/// Vues d'un site, dans l'ordre de la barre supérieure.
const VIEWS: &[(Tab, &str)] = &[
    (Tab::Editor, "view-editor"),
    (Tab::Cms, "view-cms"),
    (Tab::Locales, "view-locales"),
    (Tab::Settings(Section::General), "view-settings"),
];

#[component]
fn TopBar() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let tab = move || match ui.route.get() {
        Route::Site { tab, .. } => Some(tab),
        _ => None,
    };
    let site_open = move || tab().is_some();
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
            <div class="topbar-start">
                <button class="logo" title=ui.t("nav-all-sites") aria-label=ui.t("nav-all-sites")
                    on:click=move |_| ui.route.set(Route::Home)>{icons::logo()}</button>
                <Show when=site_open fallback=|| view! { <span class="topbar-title">"Revel"</span> }>
                    <SiteMenu/>
                </Show>
            </div>
            // Onglets raccordés au plateau : vues du site ouvert, ou accueil et réglages de Revel.
            <nav class="views" role="tablist" aria-label=ui.t("views")>
                {move || if site_open() {
                    VIEWS.iter().map(|(t, key)| {
                        let t = *t;
                        let on = move || match (tab(), t) {
                            (Some(Tab::Settings(_)), Tab::Settings(_)) => true,
                            (Some(a), b) => a == b,
                            _ => false,
                        };
                        view! {
                            <button class="view-tab" role="tab" class:is-on=on aria-selected=move || on().to_string()
                                data-view=*key on:click=move |_| if !on() { ui.tab(t) }>{ui.t(key)}</button>
                        }
                    }).collect_view().into_any()
                } else {
                    [(Route::Home, "home-title"), (Route::Settings, "nav-settings")].into_iter().map(|(r, key)| {
                        let r2 = r.clone();
                        let on = move || ui.route.get() == r2;
                        view! {
                            <button class="view-tab" role="tab" class:is-on=on.clone() aria-selected=move || on().to_string()
                                data-view=key on:click=move |_| ui.route.set(r.clone())>{ui.t(key)}</button>
                        }
                    }).collect_view().into_any()
                }}
            </nav>
            <div class="topbar-actions">
                <Show when=site_open>
                    <button class="btn" title=ui.t("preview-help") on:click=preview>{icons::play()}{ui.t("preview-open")}</button>
                    <button class="btn btn--primary" on:click=move |_| ui.tab(Tab::Settings(Section::Publish))>{ui.t("nav-publish")}</button>
                </Show>
            </div>
            <div class="activity" class:activity--on=move || ui.activity.get().is_some()>
                <div class="activity-bar" style:width=move || format!("{}%", (ui.activity.get().unwrap_or(0.0) * 100.0).max(4.0))></div>
            </div>
        </header>
    }
}

/// Nom du site ouvert ; menu : autres sites récents, tous les sites, réglages de Revel.
#[component]
fn SiteMenu() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let open = RwSignal::new(false);
    let current = move || match ui.route.get() {
        Route::Site { site, .. } => Some(site),
        _ => None,
    };
    let on_down =
        window_event_listener(leptos::ev::pointerdown, move |e: web_sys::PointerEvent| {
            let inside = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .and_then(|el| el.closest(".site-menu").ok().flatten())
                .is_some();
            if !inside {
                open.set(false);
            }
        });
    let on_key = window_event_listener(leptos::ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" {
            open.set(false);
        }
    });
    let on_blur = window_event_listener(leptos::ev::blur, move |_| open.set(false));
    on_cleanup(move || {
        on_down.remove();
        on_key.remove();
        on_blur.remove();
    });
    view! {
        <div class="site-menu">
            <button class="site-menu-btn" aria-haspopup="menu" aria-expanded=move || open.get().to_string()
                title=move || current().map(|s| s.path).unwrap_or_default()
                on:click=move |_| open.update(|o| *o = !*o)>
                <span class="site-menu-name">{move || current().map(|s| s.name).unwrap_or_default()}</span>
                {icons::chevron_down()}
            </button>
            <Show when=move || open.get()>
                <div class="menu site-menu-pop" role="menu">
                    {move || {
                        let here = current().map(|s| s.path).unwrap_or_default();
                        let others: Vec<_> = ui.recents.get().into_iter().filter(|r| r.path != here).take(6).collect();
                        (!others.is_empty()).then(|| view! {
                            <p class="menu-title">{ui.t("nav-recent")}</p>
                            {others.into_iter().map(|r| {
                                let p = r.path.clone();
                                view! {
                                    <button class="menu-item" role="menuitem" title=r.path.clone()
                                        on:click=move |_| { open.set(false); ui.open_path(p.clone(), Tab::Editor) }>
                                        <span class="menu-label">{r.name.clone()}</span>
                                    </button>
                                }
                            }).collect_view()}
                            <div class="menu-sep"></div>
                        })
                    }}
                    <button class="menu-item" role="menuitem" on:click=move |_| { open.set(false); ui.route.set(Route::Home) }>
                        {icons::grid()}<span class="menu-label">{ui.t("nav-all-sites")}</span>
                    </button>
                    <button class="menu-item" role="menuitem" on:click=move |_| { open.set(false); ui.route.set(Route::Settings) }>
                        {icons::settings()}<span class="menu-label">{ui.t("nav-app-settings")}</span>
                    </button>
                </div>
            </Show>
        </div>
    }
}
