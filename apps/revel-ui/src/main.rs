//! Interface de Revel (Leptos) : coque (barre supérieure, barre latérale) et navigation.

mod account;
mod api;
mod assistants;
mod autotr;
mod blocks;
mod boundattr;
mod canvas;
mod classes;
mod cms;
mod cmsfields;
mod cmsschema;
mod components;
mod ctxmenu;
mod dialog;
mod dock;
mod domain;
mod editor;
mod files;
mod help;
mod home;
mod hovermedia;
mod icons;
mod importer;
mod insert;
mod inspector;
mod interactions;
mod items;
mod keys;
mod layers;
mod library;
mod locales;
mod names;
mod pageops;
mod pagesettings;
mod pagetree;
mod palette;
mod panels;
mod prefs;
mod profile;
mod publish;
mod publishbar;
mod readable;
mod richtext;
mod settings;
mod setup;
mod site;
mod sitecolors;
mod siteinfo;
mod sitemap;
mod sitesettings;
mod styles;
mod sync;
mod textfield;
mod variables;
mod versions;
mod visibility;

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

/// Préférence d'affichage de cet utilisateur (stockage du navigateur ; absente si indisponible).
pub fn pref_get(key: &str) -> Option<String> {
    window()
        .local_storage()
        .ok()
        .flatten()?
        .get_item(&format!("revel.{key}"))
        .ok()
        .flatten()
}

pub fn pref_set(key: &str, value: &str) {
    if let Some(s) = window().local_storage().ok().flatten() {
        let _ = s.set_item(&format!("revel.{key}"), value);
    }
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
    /// Élément à sélectionner dans la page demandée (plan du site : « Montrer dans l'éditeur »).
    pub goto_node: RwSignal<Option<String>>,
    /// Élément copié (Ctrl+C), collé avec de nouveaux identifiants (Ctrl+V), d'un site à l'autre.
    pub clipboard: RwSignal<Vec<revel_model::Node>>,
    /// Page affichée dans l'éditeur (l'aperçu s'ouvre sur elle).
    pub current_page: RwSignal<Option<String>>,
    /// Menu contextuel ouvert (clic droit).
    pub context: RwSignal<Option<ctxmenu::CtxMenu>>,
    /// Aperçu affiché dans l'espace de travail : adresse de base, page.
    pub preview: RwSignal<Option<(String, String)>>,
    /// Adresse de base du canevas du site ouvert (dossier du site, adresse) : vignettes des médias
    /// hors de l'éditeur.
    pub canvas_base: RwSignal<Option<(String, String)>>,
    /// Choix d'un média en cours (fenêtre « Choisir un média »).
    pub pick_media: RwSignal<Option<library::MediaPick>>,
    /// Médias du site changés (import) : listes à relire.
    pub media_rev: RwSignal<u32>,
    /// Langue de travail du site ouvert (3.8) : autre langue affichée et modifiée dans l'éditeur
    /// et le CMS ; `None` : langue principale.
    pub locale: RwSignal<Option<String>>,
    /// Contenu à ouvrir dans la vue CMS (collection, fichier), demandé par une autre vue.
    pub goto_entry: RwSignal<Option<(String, String)>>,
    /// Traduction automatique en cours (D-36).
    pub auto: RwSignal<Option<autotr::Job>>,
    /// Langue de traduction en téléchargement : code, avancement (0..1, `None` : préparation).
    pub pack_job: RwSignal<Option<(String, Option<f32>)>>,
    /// Publications de la dernière série (barre supérieure ou Paramètres › Publication).
    pub deploys: RwSignal<Vec<publish::Run>>,
    /// Hébergements ou publications changés : listes à relire.
    pub publish_rev: RwSignal<u32>,
    /// Panneau du bouton « Publier » ouvert.
    pub publish_panel: RwSignal<bool>,
    /// Fenêtre « Mettre en ligne » (première mise en ligne guidée) ouverte.
    pub setup: RwSignal<bool>,
    /// Fenêtre « Nom de domaine » ouverte pour un hébergement (identifiant).
    pub domain_for: RwSignal<Option<String>>,
    /// Version plus récente de Revel publiée (6.6), vérifiée au lancement.
    pub update: RwSignal<Option<revel_protocol::UpdateInfo>>,
    /// Dernier site ouvert et sa vue : gardés dans les réglages de Revel pour y revenir.
    pub last_site: RwSignal<Option<(SiteSummary, Tab)>>,
    /// Synchronisation automatique du travail à plusieurs (D-108).
    pub sync: RwSignal<sync::SyncState>,
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
        self.locale.set(None);
        self.route.set(Route::Site { site, tab });
        self.reload_recents();
    }
    /// Barre supérieure et liste des sites : nom, langue et adresse du site ouvert à jour
    /// (identité modifiée, annulée, version restaurée).
    pub fn site_identity(self, d: &revel_model::Site) {
        let same = self.route.with_untracked(|r| match r {
            Route::Site { site, .. } => {
                site.name == d.name
                    && site.locale == d.default_locale
                    && site.base_url == d.base_url
            }
            _ => true,
        });
        if same {
            return;
        }
        self.route.update(|r| {
            if let Route::Site { site, .. } = r {
                site.name = d.name.clone();
                site.locale = d.default_locale.clone();
                site.base_url = d.base_url.clone();
            }
        });
        self.reload_recents();
    }
    /// Aperçu du site ouvert, à la page affichée : dans l'espace de travail, ou dans une nouvelle
    /// fenêtre.
    pub fn open_preview(self, window: bool) {
        if let Route::Site { site, .. } = self.route.get_untracked() {
            let env = self.preview_environment(&site);
            spawn_local(async move {
                let page = self.current_page.get_untracked();
                if window {
                    if let Err(e) = api::preview(&site.path, &env, page.as_deref()).await {
                        self.fail(e);
                    }
                    return;
                }
                match api::preview_url(&site.path, &env).await {
                    Ok(base) => self
                        .preview
                        .set(Some((base, page.unwrap_or_else(|| "/".into())))),
                    Err(e) => self.fail(e),
                }
            });
        }
    }
    /// Change de vue dans le site ouvert.
    pub fn tab(self, tab: Tab) {
        if let Route::Site { site, .. } = self.route.get_untracked() {
            self.route.set(Route::Site { site, tab });
        }
    }
    /// Dossier du site ouvert.
    pub fn site_path(self) -> Option<String> {
        match self.route.get_untracked() {
            Route::Site { site, .. } => Some(site.path),
            _ => None,
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
    /// Plan du site (D-103).
    Map,
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
    Code,
    Menus,
    Versions,
    /// Informations du site (coordonnées, réseaux), partie de Général.
    SiteInfo,
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
    // Étendue de l'accent : seulement avec une couleur d'accent (`standard` : règles de base).
    let _ = match (s.accent.is_some(), s.accent_scope.as_str()) {
        (true, scope @ ("minimal" | "full")) => root.set_attribute("data-accent-scope", scope),
        _ => root.remove_attribute("data-accent-scope"),
    };
    if let Ok(el) = root.dyn_into::<web_sys::HtmlElement>() {
        let style = el.style();
        // Couleurs fonctionnelles choisies (D-125) : texte et fond assorti ; sinon celles du thème.
        for (key, text, bg, pct) in [
            ("comp", "--comp", "--comp-bg", 16),
            ("set", "--set-text", "--set-bg", 18),
            ("inherit", "--inherit-text", "--inherit-bg", 15),
            ("auto", "--auto-text", "--auto-bg", 15),
        ] {
            match s
                .ui_colors
                .get(key)
                .filter(|c| c.len() == 7 && c.starts_with('#'))
            {
                Some(c) => {
                    let _ = style.set_property(text, c);
                    let _ = style
                        .set_property(bg, &format!("color-mix(in srgb, {c} {pct}%, transparent)"));
                }
                None => {
                    let _ = style.remove_property(text);
                    let _ = style.remove_property(bg);
                }
            }
        }
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
        goto_node: RwSignal::new(None),
        clipboard: RwSignal::new(Vec::new()),
        current_page: RwSignal::new(None),
        context: RwSignal::new(None),
        preview: RwSignal::new(None),
        canvas_base: RwSignal::new(None),
        pick_media: RwSignal::new(None),
        media_rev: RwSignal::new(0),
        locale: RwSignal::new(None),
        goto_entry: RwSignal::new(None),
        auto: RwSignal::new(None),
        pack_job: RwSignal::new(None),
        deploys: RwSignal::new(Vec::new()),
        publish_rev: RwSignal::new(0),
        publish_panel: RwSignal::new(false),
        setup: RwSignal::new(false),
        domain_for: RwSignal::new(None),
        update: RwSignal::new(None),
        last_site: RwSignal::new(None),
        sync: RwSignal::new(sync::SyncState::default()),
    };
    provide_context(ui);
    // Site ouvert retenu (menu du site gardé dans les réglages de Revel) ; oublié une fois revenu
    // à l'accueil.
    Effect::new(move |_| match ui.route.get() {
        Route::Site { site, tab } => ui.last_site.set(Some((site, tab))),
        Route::Home => ui.last_site.set(None),
        Route::Settings => {}
    });
    // Mises à jour de Revel (6.6) : vérifiées une fois au lancement, sans bloquer l'ouverture.
    spawn_local(async move {
        if let Ok(c) = api::update_check().await {
            ui.update.set(c.available);
        }
    });
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

    // Menus déroulants (`details.bp-menu`) fermés par un clic ailleurs, Échap ou un clic dans le
    // canevas (la fenêtre perd alors le focus).
    let close_menus = |keep: Option<web_sys::Element>| {
        if let Ok(list) = document().query_selector_all("details.bp-menu[open]") {
            for i in 0..list.length() {
                if let Some(d) = list
                    .item(i)
                    .and_then(|n| n.dyn_into::<web_sys::Element>().ok())
                    && keep.as_ref().is_none_or(|t| !d.contains(Some(t)))
                {
                    let _ = d.remove_attribute("open");
                }
            }
        }
    };
    let on_down =
        window_event_listener(leptos::ev::pointerdown, move |e: web_sys::PointerEvent| {
            let t = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok());
            close_menus(t);
        });
    let on_esc = window_event_listener(leptos::ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" {
            close_menus(None);
        }
    });
    let on_blur = window_event_listener(leptos::ev::blur, move |_| close_menus(None));
    // Menu du navigateur au clic droit remplacé par les menus de Revel (gardé dans les champs de
    // saisie : copier, coller, orthographe).
    let on_context =
        window_event_listener(leptos::ev::contextmenu, move |e: web_sys::MouseEvent| {
            let in_field = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|el| {
                    matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA")
                        || el.closest("[contenteditable]").ok().flatten().is_some()
                });
            if !in_field {
                e.prevent_default();
            }
        });
    on_cleanup(move || {
        on_down.remove();
        on_esc.remove();
        on_blur.remove();
        on_context.remove();
    });

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
            tab: Tab::Editor | Tab::Cms | Tab::Map,
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
            <library::MediaPicker/>
            <dialog::Dialog/>
            <setup::SetupDialog/>
            <domain::DomainDialog/>
            <ctxmenu::ContextLayer/>
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
pub(crate) const VIEWS: &[(Tab, &str)] = &[
    (Tab::Editor, "view-editor"),
    (Tab::Cms, "view-cms"),
    (Tab::Locales, "view-locales"),
    (Tab::Map, "view-map"),
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
    // Aperçu : dans l'espace de travail ; dans une nouvelle fenêtre avec Maj+clic ou par le menu.
    let preview = move |window: bool| ui.open_preview(window);
    let preview_menu = RwSignal::new(false);
    let on_down =
        window_event_listener(leptos::ev::pointerdown, move |e: web_sys::PointerEvent| {
            let inside = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .and_then(|el| el.closest(".preview-split").ok().flatten())
                .is_some();
            if !inside {
                preview_menu.set(false);
            }
        });
    on_cleanup(move || on_down.remove());
    view! {
        <header class="topbar">
            <div class="topbar-start">
                <button class="logo" title=ui.t("nav-all-sites") aria-label=ui.t("nav-all-sites")
                    on:click=move |_| ui.route.set(Route::Home)>{icons::logo()}</button>
                <Show when=move || site_open() || (ui.route.with(|r| matches!(r, Route::Settings)) && ui.last_site.with(Option::is_some))
                    fallback=|| view! { <span class="topbar-title">"Revel"</span> }>
                    <SiteMenu/>
                </Show>
                // Réglages de Revel ouverts depuis un site : retour au site d'un clic (vue quittée).
                {move || matches!(ui.route.get(), Route::Settings).then(|| ui.last_site.get()).flatten().map(|(site, tab)| {
                    let name = site.name.clone();
                    view! {
                        <button class="btn btn--small topbar-back" title=site.path.clone()
                            on:click=move |_| ui.route.set(Route::Site { site: site.clone(), tab })>
                            {icons::arrow_left()}<span>{ui.tx("nav-back-to-site", &[("name", name)])}</span>
                        </button>
                    }
                })}
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
                // Version plus récente publiée : installée depuis les réglages.
                {move || ui.update.get().map(|u| view! {
                    <button class="btn btn--small update-pill" title=ui.t("update-pill-help")
                        on:click=move |_| ui.route.set(Route::Settings)>
                        {icons::download()}{ui.tx(if u.prerelease { "update-pill-pre" } else { "update-pill" }, &[("version", u.version.clone())])}
                    </button>
                })}
                <Show when=site_open>
                    <div class="split-btn preview-split" class:is-on=move || ui.preview.with(Option::is_some)>
                        <button class="btn split-main" title=ui.t("preview-help")
                            on:click=move |e: leptos::ev::MouseEvent| {
                                if ui.preview.with_untracked(Option::is_some) && !e.shift_key() {
                                    ui.preview.set(None);
                                } else {
                                    preview(e.shift_key());
                                }
                            }>{icons::play()}{ui.t("preview-open")}</button>
                        <button class="btn split-more" aria-haspopup="menu" aria-expanded=move || preview_menu.get().to_string()
                            title=ui.t("preview-options") aria-label=ui.t("preview-options")
                            on:click=move |_| preview_menu.update(|o| *o = !*o)>{icons::chevron_down()}</button>
                        <Show when=move || preview_menu.get()>
                            <div class="menu preview-menu" role="menu">
                                <button class="menu-item" role="menuitem" on:click=move |_| { preview_menu.set(false); preview(false); }>
                                    {icons::play()}<span class="menu-label">{ui.t("preview-here")}</span>
                                </button>
                                <button class="menu-item" role="menuitem" on:click=move |_| { preview_menu.set(false); preview(true); }>
                                    {icons::external()}<span class="menu-label">{ui.t("preview-window")}</span><kbd>{ui.t("key-preview-window")}</kbd>
                                </button>
                            </div>
                        </Show>
                    </div>
                    <publishbar::PublishButton/>
                    <sync::SyncButton/>
                </Show>
                <profile::ProfileButton/>
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
    // Site ouvert, ou dernier site ouvert (depuis les réglages de Revel).
    let current = move || match ui.route.get() {
        Route::Site { site, .. } => Some(site),
        _ => ui.last_site.get().map(|(s, _)| s),
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

/// Chemin affiché dans un champ de chemin (`.path-value`, écrit de droite à gauche pour garder la fin
/// visible) : marque de gauche à droite en tête, sinon `/` ou `\` du début passe à la fin.
pub(crate) fn ltr(path: &str) -> String {
    format!("\u{200E}{path}")
}

/// Attente (millisecondes) dans une tâche de l'interface.
pub async fn sleep_ms(ms: i32) {
    let p = js_sys::Promise::new(&mut |res, _| {
        let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(&res, ms);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(p).await;
}
