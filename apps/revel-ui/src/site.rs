//! Site ouvert : vues (éditeur, CMS, localisation, paramètres) et rubriques des paramètres.

use crate::{Route, Section, Tab, Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{BuildEvent, BuildPhase, BuildSummary, Failure, SiteSummary};
use wasm_bindgen::JsCast;

/// Vues d'un site. L'éditeur, une fois ouvert, reste en place (caché) pendant que les autres
/// vues sont affichées : on y revient sans recharger le canevas.
#[component]
pub fn SiteView(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let tab = Memo::new(move |_| match ui.route.get() {
        Route::Site { tab, .. } => tab,
        _ => Tab::Editor,
    });
    // Sens du passage d'un espace à l'autre (ordre des onglets) : animation de la vue.
    let order = |t: Tab| match t {
        Tab::Editor => 0,
        Tab::Cms => 1,
        Tab::Locales => 2,
        Tab::Map => 3,
        Tab::Settings(_) => 4,
    };
    let dir = Memo::new(move |prev: Option<&(i32, i32, bool)>| {
        let now = order(tab.get());
        let before = prev.map_or(now, |p| p.0);
        (now, if now < before { -1 } else { 1 }, now != before)
    });
    // Rubrique des paramètres changée : sans animation (même espace).
    let quiet = move || !dir.with_untracked(|d| d.2);
    // Aperçu fermé quand on change d'espace.
    Effect::new(move |prev: Option<i32>| {
        let now = order(tab.get());
        if prev.is_some_and(|p| p != now) {
            ui.preview.set(None);
        }
        now
    });
    on_cleanup(move || ui.preview.set(None));
    let editor_seen = RwSignal::new(false);
    Effect::new(move |_| {
        if tab.get() == Tab::Editor {
            editor_seen.set(true);
        }
    });
    let s_editor = site.clone();
    let site = StoredValue::new(site);
    view! {
        <div class="site-view" style=move || format!("--view-dir:{}", dir.get().1)>
            <Show when=move || editor_seen.get()>
                <div class="view-pane view-pane--editor" class:is-hidden=move || tab.get() != Tab::Editor>
                    <crate::editor::Editor site=s_editor.clone()/>
                </div>
            </Show>
            <Show when=move || ui.preview.with(Option::is_some)>
                <PreviewPane site=site.get_value()/>
            </Show>
            {move || match tab.get() {
                Tab::Editor => ().into_any(),
                Tab::Cms => view! { <div class="view-pane"><crate::cms::CmsView site=site.get_value()/></div> }.into_any(),
                Tab::Locales => view! { <div class="view-pane view-pane--scroll"><crate::locales::Locales site=site.get_value()/></div> }.into_any(),
                Tab::Map => view! { <div class="view-pane"><crate::sitemap::SiteMap site=site.get_value()/></div> }.into_any(),
                Tab::Settings(sec) => view! { <div class="view-pane" class:is-quiet=quiet()><SiteSettings site=site.get_value() section=sec/></div> }.into_any(),
            }}
        </div>
    }
}

/// Largeur de l'aperçu (`None` : toute la place), libellé, icône.
type PreviewSize = (Option<u32>, &'static str, fn() -> AnyView);

/// Aperçu du site dans l'espace de travail : la page comme elle sera publiée (scripts actifs),
/// largeur d'écran au choix, rechargement, ouverture dans une fenêtre, fermeture (Échap).
#[component]
fn PreviewPane(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    // Largeur : `None` toute la place, sinon tablette ou mobile.
    let width = RwSignal::new(None::<u32>);
    let reload = RwSignal::new(0u32);
    let url = move || {
        ui.preview
            .get()
            .map(|(base, page)| format!("{base}{}", page.trim_start_matches('/')))
            .unwrap_or_default()
    };
    let page = move || ui.preview.get().map(|p| p.1).unwrap_or_default();
    let on_key = window_event_listener(leptos::ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" && ui.ask.with_untracked(Option::is_none) {
            ui.preview.set(None);
        }
    });
    on_cleanup(move || on_key.remove());
    let site = StoredValue::new(site);
    let in_window = move |_| {
        let s = site.get_value();
        let env = ui.preview_environment(&s);
        let p = page();
        leptos::task::spawn_local(async move {
            match api::preview(&s.path, &env, Some(&p)).await {
                Ok(_) => ui.preview.set(None),
                Err(e) => ui.fail(e),
            }
        });
    };
    let sizes: [PreviewSize; 3] = [
        (None, "editor-bp-desktop-short", icons::desktop),
        (Some(768), "preview-tablet", icons::tablet),
        (Some(390), "preview-mobile", icons::mobile),
    ];
    view! {
        <div class="preview-pane" role="region" aria-label=ui.t("preview-open")>
            <div class="preview-bar">
                <span class="preview-title">{icons::play()}<span>{ui.t("preview-open")}</span><span class="muted path">{page}</span></span>
                <div class="segmented" role="group">
                    {sizes.into_iter().map(|(w, key, icon)| view! {
                        <button class:is-on=move || width.get() == w title=move || match w {
                                Some(px) => format!("{} · {px} px", ui.tx(key, &[])),
                                None => ui.tx(key, &[]),
                            }
                            aria-label=ui.t(key) on:click=move |_| width.set(w)>{icon()}</button>
                    }).collect_view()}
                </div>
                <span class="toolbar-spacer"></span>
                <button class="icon-btn" title=ui.t("preview-reload") aria-label=ui.t("preview-reload")
                    on:click=move |_| reload.update(|n| *n += 1)>{icons::reset()}</button>
                <button class="icon-btn" title=move || format!("{} ({})", ui.tx("preview-window", &[]), ui.tx("key-preview-window", &[]))
                    aria-label=ui.t("preview-window") on:click=in_window>{icons::external()}</button>
                <button class="btn" title=ui.t("preview-close-help") on:click=move |_| ui.preview.set(None)>{icons::close()}{ui.t("preview-close")}</button>
            </div>
            <div class="preview-stage" class:is-device=move || width.get().is_some()>
                {move || {
                    let _ = reload.get();
                    view! {
                        <iframe class="preview-frame" title=ui.t("preview-open") src=url()
                            style=move || width.get().map(|w| format!("width:{w}px")).unwrap_or_default()></iframe>
                    }
                }}
            </div>
        </div>
    }
}

/// Rubriques des paramètres : (rubrique, libellé, étape prévue si pas encore disponible).
/// Cinq rubriques : les réglages proches sur une même page (polices dans Général).
pub(crate) const SECTIONS: &[(Section, &str, Option<u8>)] = &[
    (Section::General, "settings-general", None),
    (Section::Publish, "settings-publish", None),
    (Section::Menus, "settings-navigation", None),
    (Section::Code, "settings-code", None),
    (Section::Versions, "settings-versions", None),
];

/// Rubrique qui affiche une partie des réglages : général avec référencement et formulaires,
/// publication avec export, navigation (menus) avec redirections.
fn group_of(s: Section) -> Section {
    match s {
        Section::Seo | Section::Forms | Section::SiteInfo => Section::General,
        Section::Export => Section::Publish,
        Section::Redirects => Section::Menus,
        x => x,
    }
}

/// Parties de chaque rubrique : (clé, titre, présentation).
fn parts_of(group: Section) -> &'static [(&'static str, &'static str, &'static str)] {
    match group {
        Section::General => &[
            ("general", "settings-general", "settings-general-lead"),
            ("site-info", "siteinfo-title", "siteinfo-help"),
            ("fonts", "settings-fonts", "fonts-help"),
            ("seo", "settings-seo", "settings-seo-lead"),
            ("forms", "settings-forms", "forms-help"),
        ],
        Section::Publish => &[
            ("publish", "settings-publish", "settings-publish-lead"),
            ("export", "export-title", "export-lead"),
        ],
        Section::Code => &[("code", "settings-code", "code-help")],
        Section::Versions => &[("versions", "settings-versions", "versions-help")],
        _ => &[
            ("site-menus", "sitemenus-title", "sitemenus-help"),
            ("menus", "settings-menus", "menus-help"),
            ("redirects", "settings-redirects", "redirects-help"),
        ],
    }
}

/// Une partie des paramètres : titre et présentation à gauche, réglages à droite (D-104).
pub(crate) fn settings_part(
    ui: Ui,
    key: &'static str,
    title: &'static str,
    lead: &'static str,
    body: AnyView,
) -> AnyView {
    view! {
        <section class="settings-sub" data-sub=key id=format!("set-{key}")>
            <header class="settings-sub-head">
                <h2>{ui.t(title)}</h2>
                <p class="settings-sub-lead">{ui.t(lead)}</p>
            </header>
            <div class="settings-sub-body">{body}</div>
        </section>
    }
    .into_any()
}

/// Partie des paramètres montrée en haut de la zone qui défile (rubrique en cours d'un rail) ;
/// parties vides (rien à régler) retirées du rail.
pub(crate) fn scroll_spy(
    content: NodeRef<leptos::html::Div>,
    current: RwSignal<String>,
    shown: RwSignal<Vec<String>>,
) {
    let Some(el) = content.get_untracked() else {
        return;
    };
    let top = el.get_bounding_client_rect().top();
    let Ok(list) = el.query_selector_all(".settings-sub") else {
        return;
    };
    let mut on = None;
    let mut keys = Vec::new();
    // Bas de la zone atteint : la dernière partie est celle montrée.
    let end = el.scroll_top() + el.client_height() >= el.scroll_height() - 4;
    for i in 0..list.length() {
        let Some(n) = list
            .item(i)
            .and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            continue;
        };
        let empty = n
            .query_selector(".settings-sub-body")
            .ok()
            .flatten()
            .is_some_and(|b| {
                b.child_element_count() == 0 || b.text_content().is_none_or(|t| t.trim().is_empty())
            });
        if empty || n.offset_height() == 0 {
            continue;
        }
        let key = n.get_attribute("data-sub").unwrap_or_default();
        if on.is_none() || n.get_bounding_client_rect().top() - top <= 96.0 || end {
            on = Some(key.clone());
        }
        keys.push(key);
    }
    if shown.get_untracked() != keys {
        shown.set(keys);
    }
    if let Some(k) = on
        && current.get_untracked() != k
    {
        current.set(k);
    }
}

/// Va à une partie des paramètres (défilement doux, sauf mouvement réduit).
pub(crate) fn scroll_to_part(key: &str) {
    if let Ok(Some(el)) = document().query_selector(&format!(".settings-sub[data-sub=\"{key}\"]")) {
        let opts = web_sys::ScrollIntoViewOptions::new();
        let reduce = window()
            .match_media("(prefers-reduced-motion: reduce)")
            .ok()
            .flatten()
            .is_some_and(|m| m.matches());
        opts.set_behavior(if reduce {
            web_sys::ScrollBehavior::Instant
        } else {
            web_sys::ScrollBehavior::Smooth
        });
        opts.set_block(web_sys::ScrollLogicalPosition::Start);
        el.scroll_into_view_with_scroll_into_view_options(&opts);
    }
}

/// Paramètres du site : rubriques dans un rail à gauche ; la rubrique ouverte, raccordée à la
/// page comme un onglet, liste ses parties (D-104). Chaque partie : titre et présentation à
/// gauche, réglages à droite.
#[component]
fn SiteSettings(site: SiteSummary, section: Section) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let group = group_of(section);
    let part = |key: &'static str, v: AnyView| {
        let (_, title, lead) = parts_of(group)
            .iter()
            .find(|p| p.0 == key)
            .copied()
            .unwrap_or((key, "settings-general", "settings-general-lead"));
        settings_part(ui, key, title, lead, v)
    };
    let body = match group {
        Section::General => view! {
            {part("general", view! { <crate::sitesettings::General site=site.clone()/> }.into_any())}
            {part("site-info", view! { <crate::siteinfo::SiteInfo s=doc_session(ui, site.path.clone()) path=site.path.clone()/> }.into_any())}
            {part("fonts", view! { <crate::sitesettings::Fonts site=site.clone()/> }.into_any())}
            {part("seo", view! { <Seo site=site.clone()/> }.into_any())}
            {part("forms", view! { <Forms site=site.clone()/> }.into_any())}
        }
        .into_any(),
        Section::Code => part("code", view! { <crate::sitesettings::Code site=site.clone()/> }.into_any()),
        Section::Versions => part("versions", view! { <crate::versions::Versions site=site.clone()/> }.into_any()),
        Section::Publish => view! {
            {part("publish", view! { <crate::publish::Publish site=site.clone()/> }.into_any())}
            {part("export", view! { <Export site=site.clone()/> }.into_any())}
        }
        .into_any(),
        _ => view! {
            {part("site-menus", view! { <crate::siteinfo::SiteMenus s=doc_session(ui, site.path.clone()) path=site.path.clone()/> }.into_any())}
            {part("menus", view! { <crate::readable::Menus s=doc_session(ui, site.path.clone())/> }.into_any())}
            {part("redirects", view! { <Redirects site=site.clone()/> }.into_any())}
        }
        .into_any(),
    };
    // Partie demandée (lien vers les formulaires, l'export…) : montrée.
    let wanted = match section {
        Section::Seo => Some("seo"),
        Section::Forms => Some("forms"),
        Section::SiteInfo => Some("site-info"),
        Section::Export => Some("export"),
        Section::Redirects => Some("redirects"),
        _ => None,
    };
    let content = NodeRef::<leptos::html::Div>::new();
    let current = RwSignal::new(wanted.unwrap_or(parts_of(group)[0].0).to_string());
    let shown = RwSignal::new(
        parts_of(group)
            .iter()
            .map(|p| p.0.to_string())
            .collect::<Vec<_>>(),
    );
    let spy = move || scroll_spy(content, current, shown);
    // Parties chargées après l'ouverture : rail mis à jour.
    for ms in [150u64, 600, 1500] {
        set_timeout(
            move || {
                if !current.is_disposed() {
                    spy()
                }
            },
            std::time::Duration::from_millis(ms),
        );
    }
    if let Some(key) = wanted {
        request_animation_frame(move || {
            if let Ok(Some(el)) =
                document().query_selector(&format!(".settings-sub[data-sub=\"{key}\"]"))
            {
                el.scroll_into_view();
            }
        });
    }
    view! {
        <div class="settings-layout">
            <nav class="settings-nav" aria-label=ui.t("view-settings")>
                {SECTIONS.iter().map(|(sec, key, step)| {
                    let sec = *sec;
                    let on = group == sec;
                    let title = step.map(|n| ui.tx("planned-step", &[("step", n.to_string())]));
                    let parts = parts_of(sec);
                    view! {
                        <div class="settings-nav-group" class:is-on=on class:is-single={parts.len() < 2}>
                            <button class="settings-nav-item" class:is-on=on class:is-planned=step.is_some()
                                data-section=*key title=title aria-current=on.then_some("page")
                                on:click=move |_| if on { scroll_to_part(parts[0].0) } else { ui.tab(Tab::Settings(sec)) }>
                                <span>{ui.t(key)}</span>
                                {step.map(|n| view! { <span class="settings-nav-step">{n}</span> })}
                            </button>
                            {(on && parts.len() > 1).then(|| view! {
                                <ul class="settings-nav-parts">
                                    {parts.iter().map(|(k, title, _)| {
                                        let k: &'static str = k;
                                        view! {
                                            <li class:is-gone=move || shown.with(|s| !s.iter().any(|x| x == k))>
                                                <button class="settings-nav-part" data-part=k
                                                    class:is-on=move || current.with(|c| c == k)
                                                    on:click=move |_| { current.set(k.to_string()); scroll_to_part(k) }>
                                                    {ui.t(title)}
                                                </button>
                                            </li>
                                        }
                                    }).collect_view()}
                                </ul>
                            })}
                        </div>
                    }
                }).collect_view()}
            </nav>
            <div class="settings-content" node_ref=content on:scroll=move |_| spy()>{body}</div>
        </div>
    }
}

/// Première lettre en capitale (identifiants de collection affichés comme noms).
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// Document du site pour une rubrique des paramètres : relu à l'ouverture, modifié par des
/// commandes (même historique que l'éditeur ; l'éditeur relit le document modifié).
#[derive(Clone, Copy)]
pub(crate) struct DocSession {
    pub doc: RwSignal<Option<revel_model::Site>>,
    pub apply: Callback<revel_edit::Command>,
}

pub(crate) fn doc_session(ui: Ui, path: String) -> DocSession {
    let doc = RwSignal::new(None::<revel_model::Site>);
    let path = StoredValue::new(path);
    let reload = move || {
        let p = path.get_value();
        spawn_local(async move {
            if let Ok(o) = api::edit_open(&p).await
                && !doc.is_disposed()
            {
                ui.site_identity(&o.site);
                doc.set(Some(o.site));
            }
        });
    };
    reload();
    let apply = Callback::new(move |cmd: revel_edit::Command| {
        let p = path.get_value();
        spawn_local(async move {
            if let Err(e) = api::edit_apply(&p, &cmd, false).await {
                ui.fail(e);
            }
            if !doc.is_disposed() {
                reload();
            }
        });
    });
    DocSession { doc, apply }
}

/// Redirections permanentes (301) du site : ajout, modification, retrait. Les changements
/// d'adresse d'une page en ajoutent (paramètres de la page).
#[component]
fn Redirects(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let s = doc_session(ui, site.path.clone());
    let list = move || {
        s.doc
            .with(|d| d.as_ref().map(|d| d.redirects.clone()).unwrap_or_default())
    };
    let set = move |l: Vec<revel_model::Redirect>| {
        s.apply
            .run(revel_edit::Command::SetRedirects { redirects: l })
    };
    let from = RwSignal::new(String::new());
    let to = RwSignal::new(String::new());
    let add = move || {
        let (f, t) = (
            from.get_untracked().trim().to_string(),
            to.get_untracked().trim().to_string(),
        );
        if f.is_empty() || t.is_empty() {
            return;
        }
        let f = if f.starts_with('/') {
            f
        } else {
            format!("/{f}")
        };
        let mut l = list();
        l.retain(|r| r.from != f);
        l.push(revel_model::Redirect { from: f, to: t });
        set(l);
        from.set(String::new());
        to.set(String::new());
    };
    view! {
        <section class="page settings-page">
            <header class="page-head">
                <h1 title=ui.t("redirects-help")>{ui.t("settings-redirects")}</h1>
            </header>
            <div class="section section--flush">
                <table class="table table--compact redirects">
                    <thead><tr><th>{ui.t("redirects-from")}</th><th></th><th>{ui.t("redirects-to")}</th><th></th></tr></thead>
                    <tbody>
                        {move || list().into_iter().enumerate().map(|(i, r)| view! {
                            <tr>
                                <td class="path">{r.from.clone()}</td>
                                <td class="muted col-arrow">{icons::arrow_right()}</td>
                                <td class="path">{r.to.clone()}</td>
                                <td class="col-actions">
                                    <button class="icon-btn icon-btn--small" title=ui.t("redirects-remove") aria-label=ui.t("redirects-remove")
                                        on:click=move |_| { let mut l = list(); if i < l.len() { l.remove(i); } set(l) }>{icons::trash()}</button>
                                </td>
                            </tr>
                        }).collect_view()}
                        <tr class="redirect-new">
                            <td><input class="input input--mono" placeholder="/ancienne-adresse/" aria-label=ui.t("redirects-from")
                                prop:value=move || from.get() on:input=move |e| from.set(event_target_value(&e))
                                on:keydown=move |e: leptos::ev::KeyboardEvent| if e.key() == "Enter" { add() }/></td>
                            <td class="muted col-arrow">{icons::arrow_right()}</td>
                            <td><input class="input input--mono" placeholder="/nouvelle-adresse/" aria-label=ui.t("redirects-to")
                                prop:value=move || to.get() on:input=move |e| to.set(event_target_value(&e))
                                on:keydown=move |e: leptos::ev::KeyboardEvent| if e.key() == "Enter" { add() }/></td>
                            <td class="col-actions">
                                <button class="icon-btn icon-btn--small" title=ui.t("redirects-add") aria-label=ui.t("redirects-add")
                                    on:click=move |_| add()>{icons::plus()}</button>
                            </td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </section>
    }
}

/// Référencement du site : plan du site et robots.txt produits par Revel (sauf si le site a ses
/// propres pages à ces adresses).
#[component]
fn Seo(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let s = doc_session(ui, site.path.clone());
    let seo = move || {
        s.doc.with(|d| {
            d.as_ref()
                .map(|d| d.settings.seo.clone())
                .unwrap_or_default()
        })
    };
    let own = move |path: &str| {
        s.doc.with(|d| {
            d.as_ref()
                .is_some_and(|d| d.pages.iter().any(|p| p.path == path))
        })
    };
    let no_url = move || {
        s.doc.with(|d| {
            d.as_ref()
                .is_some_and(|d| d.base_url.as_deref().unwrap_or_default().is_empty())
        })
    };
    let set = move |f: &dyn Fn(&mut revel_model::SeoSettings)| {
        let mut v = seo();
        f(&mut v);
        s.apply.run(revel_edit::Command::SetSeoSettings { seo: v });
    };
    view! {
        <section class="page settings-page">
            <header class="page-head"><h1 title=ui.t("seo-pages-help")>{ui.t("settings-seo")}</h1></header>
            <crate::readable::SeoReadable s=s path=site.path.clone()/>
            <div class="section">
                <label class="view-line" title=move || ui.tx(if own("/sitemap.xml") { "seo-sitemap-own" } else if no_url() { "seo-sitemap-no-url" } else { "seo-sitemap-help" }, &[])>
                    <span>
                        {ui.t("seo-sitemap")}
                        {move || (own("/sitemap.xml") || no_url()).then(|| view! { <span class="seo-warn">{icons::warning()}</span> })}
                    </span>
                    <span class="switch">
                        <input type="checkbox" prop:checked=move || !seo().no_sitemap disabled=move || own("/sitemap.xml")
                            on:change=move |e| { let on = event_target_checked(&e); set(&|v| v.no_sitemap = !on) }/>
                        <span class="switch-track"></span>
                    </span>
                </label>
                <label class="view-line" title=ui.t("seo-sitemap-lastmod-help")>
                    <span>{ui.t("seo-sitemap-lastmod")}</span>
                    <span class="switch">
                        <input type="checkbox" prop:checked=move || seo().sitemap_lastmod
                            disabled=move || own("/sitemap.xml") || seo().no_sitemap
                            on:change=move |e| { let on = event_target_checked(&e); set(&|v| v.sitemap_lastmod = on) }/>
                        <span class="switch-track"></span>
                    </span>
                </label>
            </div>
            <div class="section">
                <h2 title=ui.t("seo-share-help")>{ui.t("seo-share")}</h2>
                <div class="form-grid">
                    {text_row(ui, "seo-site-title", "seo-site-title-help", "Agence Exemple", Signal::derive(move || seo().site_title.unwrap_or_default()),
                        Callback::new(move |v: String| set(&|x| x.site_title = some(&v))))}
                    {text_row(ui, "seo-description", "seo-description-help", "", Signal::derive(move || seo().description.unwrap_or_default()),
                        Callback::new(move |v: String| set(&|x| x.description = some(&v))))}
                    {text_row(ui, "seo-share-image", "seo-share-image-help", "/media/partage.jpg", Signal::derive(move || seo().share_image.unwrap_or_default()),
                        Callback::new(move |v: String| set(&|x| x.share_image = some(&v))))}
                    {text_row(ui, "seo-theme-color", "seo-theme-color-help", "#16171B", Signal::derive(move || seo().theme_color.unwrap_or_default()),
                        Callback::new(move |v: String| set(&|x| x.theme_color = some(&v))))}
                    {text_row(ui, "seo-favicon", "seo-favicon-help", "/media/favicon.svg", Signal::derive(move || seo().favicon.unwrap_or_default()),
                        Callback::new(move |v: String| set(&|x| x.favicon = some(&v))))}
                    {text_row(ui, "seo-favicon-png", "seo-favicon-png-help", "/media/favicon.png", Signal::derive(move || seo().favicon_png.unwrap_or_default()),
                        Callback::new(move |v: String| set(&|x| x.favicon_png = some(&v))))}
                    {text_row(ui, "seo-touch-icon", "seo-touch-icon-help", "/media/apple-touch-icon.png", Signal::derive(move || seo().touch_icon.unwrap_or_default()),
                        Callback::new(move |v: String| set(&|x| x.touch_icon = some(&v))))}
                </div>
            </div>
            <div class="section">
                <h2 title=ui.t("seo-org-help")>{ui.t("seo-org")}</h2>
                <div class="form-grid">
                    {org_rows(ui, Signal::derive(move || seo().organization.unwrap_or_default()),
                        Callback::new(move |o: revel_model::Organization| set(&|x| x.organization = (o != revel_model::Organization::default()).then(|| o.clone()))))}
                </div>
            </div>
            <div class="section">
                <label class="field">
                    <span class="label" title=move || ui.tx(if own("/robots.txt") { "seo-robots-own" } else { "seo-robots-help" }, &[])>
                        "robots.txt"
                        {move || own("/robots.txt").then(|| view! { <span class="seo-warn">{icons::warning()}</span> })}
                    </span>
                    <textarea class="input input--mono code-input robots-input" rows="6" disabled=move || own("/robots.txt")
                        placeholder="User-agent: *\nAllow: /"
                        prop:value=move || seo().robots.unwrap_or_default()
                        on:change=move |e| {
                            let t = event_target_value(&e);
                            set(&|v| v.robots = (!t.trim().is_empty()).then(|| t.clone()))
                        }></textarea>
                </label>
            </div>
        </section>
    }
}

/// Texte réglé (vide : retiré).
fn some(v: &str) -> Option<String> {
    let v = v.trim();
    (!v.is_empty()).then(|| v.to_string())
}

/// Ligne « libellé : champ » d'un réglage texte.
fn text_row(
    ui: Ui,
    label: &'static str,
    help: &'static str,
    placeholder: &'static str,
    value: Signal<String>,
    on_change: Callback<String>,
) -> impl IntoView {
    view! {
        <span class="label" title=ui.t(help)>{ui.t(label)}</span>
        <input class="input" spellcheck="false" placeholder=placeholder prop:value=move || value.get()
            on:change=move |e| on_change.run(event_target_value(&e))/>
    }
}

/// Fiche de l'organisation : nom, autre nom, logo, contact, adresse, réseaux (un lien par ligne).
fn org_rows(
    ui: Ui,
    org: Signal<revel_model::Organization>,
    set: Callback<revel_model::Organization>,
) -> impl IntoView {
    type Get = fn(&revel_model::Organization) -> String;
    type Put = fn(&mut revel_model::Organization, String);
    let fields: [(&'static str, &'static str, Get, Put); 10] = [
        (
            "seo-org-name",
            "Atelier Exemple",
            |o| o.name.clone(),
            |o, v| o.name = v,
        ),
        (
            "seo-org-alt",
            "",
            |o| o.alternate_name.clone(),
            |o, v| o.alternate_name = v,
        ),
        (
            "seo-org-logo",
            "/media/logo.svg",
            |o| o.logo.clone(),
            |o, v| o.logo = v,
        ),
        (
            "seo-org-email",
            "bonjour@exemple.fr",
            |o| o.email.clone(),
            |o, v| o.email = v,
        ),
        (
            "seo-org-phone",
            "+33 1 23 45 67 89",
            |o| o.telephone.clone(),
            |o, v| o.telephone = v,
        ),
        (
            "seo-org-founded",
            "2004",
            |o| o.founding_date.clone(),
            |o, v| o.founding_date = v,
        ),
        (
            "seo-org-street",
            "",
            |o| o.street.clone(),
            |o, v| o.street = v,
        ),
        (
            "seo-org-postcode",
            "",
            |o| o.postcode.clone(),
            |o, v| o.postcode = v,
        ),
        ("seo-org-city", "", |o| o.city.clone(), |o, v| o.city = v),
        (
            "seo-org-country",
            "FR",
            |o| o.country.clone(),
            |o, v| o.country = v,
        ),
    ];
    view! {
        {fields.into_iter().map(|(label, ph, get, put)| view! {
            <span class="label">{ui.t(label)}</span>
            <input class="input" spellcheck="false" placeholder=ph prop:value=move || get(&org.get())
                on:change=move |e| {
                    let mut o = org.get_untracked();
                    put(&mut o, event_target_value(&e).trim().to_string());
                    set.run(o);
                }/>
        }).collect_view()}
        <span class="label" title=ui.t("seo-org-social-help")>{ui.t("seo-org-social")}</span>
        <textarea class="input input--mono" rows="4" spellcheck="false" placeholder="https://www.linkedin.com/company/…"
            prop:value=move || org.get().same_as.join("\n")
            on:change=move |e| {
                let mut o = org.get_untracked();
                o.same_as = event_target_value(&e).lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect();
                set.run(o);
            }></textarea>
    }
}

/// Paramètres › Formulaires (5.8) : adresse qui reçoit les messages des formulaires du site,
/// objet, message après l'envoi ; hébergements qui les reçoivent (PHP) ou non.
#[component]
fn Forms(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let s = doc_session(ui, site.path.clone());
    let forms = move || {
        s.doc.with(|d| {
            d.as_ref()
                .map(|d| d.settings.forms.clone())
                .unwrap_or_default()
        })
    };
    let site_name = move || {
        s.doc
            .with(|d| d.as_ref().map(|d| d.name.clone()).unwrap_or_default())
    };
    let set = move |f: &dyn Fn(&mut revel_model::FormSettings)| {
        let mut v = forms();
        f(&mut v);
        s.apply
            .run(revel_edit::Command::SetFormSettings { forms: v });
    };
    let hosts = RwSignal::new(Vec::<revel_protocol::HostingProfile>::new());
    let path = site.path.clone();
    spawn_local(async move {
        if let Ok(l) = api::profiles(&path).await {
            hosts.set(l.into_iter().map(|(p, _)| p).collect());
        }
    });
    let email_ok = move || {
        let r = forms().recipient;
        let r = r.trim();
        r.is_empty() || (r.contains('@') && r.rsplit('@').next().is_some_and(|d| d.contains('.')))
    };
    view! {
        <section class="page settings-page">
            <header class="page-head"><h1 title=ui.t("forms-help")>{ui.t("settings-forms")}</h1></header>
            <div class="section stack">
                <label class="field">
                    <span class="label">{ui.t("forms-recipient")}</span>
                    <input class="input forms-recipient" type="email" autocomplete="email" spellcheck="false"
                        placeholder="bonjour@exemple.fr" class:is-invalid=move || !email_ok()
                        prop:value=move || forms().recipient
                        on:change=move |e| { let v = event_target_value(&e); set(&|f| f.recipient = v.trim().to_string()) }/>
                </label>
                <div class="form-cols">
                    <label class="field">
                        <span class="label">{ui.t("forms-subject")}</span>
                        <input class="input" type="text"
                            placeholder=move || ui.tx("forms-subject-default", &[("site", site_name())])
                            prop:value=move || forms().subject.unwrap_or_default()
                            on:change=move |e| { let v = event_target_value(&e); set(&|f| f.subject = (!v.trim().is_empty()).then(|| v.trim().to_string())) }/>
                    </label>
                    <label class="field">
                        <span class="label">{ui.t("forms-success")}</span>
                        <input class="input" type="text" placeholder=ui.t("forms-success-default")
                            prop:value=move || forms().success.unwrap_or_default()
                            on:change=move |e| { let v = event_target_value(&e); set(&|f| f.success = (!v.trim().is_empty()).then(|| v.trim().to_string())) }/>
                    </label>
                </div>
            </div>
            <Show when=move || !hosts.with(Vec::is_empty)>
                <div class="section">
                    <ul class="domain-lines forms-hosts">
                        {move || hosts.get().into_iter().map(|p| {
                            let github = matches!(p.target, revel_protocol::HostingTarget::GithubPages { .. });
                            let note = match p.preset.as_deref() {
                                Some("infomaniak") => Some("forms-infomaniak"),
                                Some("ovh") | Some("ionos") => Some("forms-from-domain"),
                                _ => None,
                            };
                            let text = if github {
                                ui.tx("forms-host-no", &[("name", p.name.clone())])
                            } else {
                                ui.tx("forms-host-yes", &[("name", p.name.clone())])
                            };
                            view! {
                                <li class="domain-line" class:is-ok=!github class:is-ko=github
                                    title=note.map(|k| ui.tx(k, &[])).unwrap_or_default()>
                                    {if github { icons::warning() } else { icons::check() }}
                                    <span>{text}</span>
                                    {note.map(|_| view! { <span class="forms-note">{icons::info()}</span> })}
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </div>
            </Show>
        </section>
    }
}

/// Export du site : fichiers finaux (pages, styles, images optimisées) copiés dans un nouveau
/// dossier choisi, pour héberger le site ailleurs ou l'archiver. L'aperçu et la publication n'en
/// ont pas besoin (D-26).
#[component]
pub fn Export(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let env = RwSignal::new(ui.preview_environment(&site));
    let images = RwSignal::new(true);
    let parent = RwSignal::new(String::new());
    let running = RwSignal::new(false);
    let progress = RwSignal::new(None::<(BuildPhase, usize, usize)>);
    let result = RwSignal::new(None::<Result<BuildSummary, Failure>>);
    let path = StoredValue::new(site.path.clone());
    // Dossier proposé : celui des sites (Documents/Revel).
    spawn_local(async move {
        if let Ok(d) = api::default_sites_dir().await
            && parent.get_untracked().is_empty()
        {
            parent.set(d);
        }
    });
    let change = move |_| {
        spawn_local(async move {
            let start = parent.get_untracked();
            match api::pick_folder_at(Some(start.as_str()).filter(|s| !s.is_empty())).await {
                Ok(Some(p)) => parent.set(p),
                Ok(None) => {}
                Err(e) => ui.fail(e),
            }
        })
    };
    // Fin de l'export. Les événements du canal peuvent arriver après la réponse de la commande :
    // l'état final est fixé par le premier signal de fin, la progression qui suit est ignorée.
    let finish = move |r: Result<BuildSummary, Failure>| {
        result.set(Some(r));
        progress.set(None);
        running.set(false);
        ui.activity.set(None);
    };
    let run = move |_| {
        running.set(true);
        result.set(None);
        ui.activity.set(Some(0.02));
        spawn_local(async move {
            let r = api::export(
                &path.get_value(),
                &env.get_untracked(),
                images.get_untracked(),
                &parent.get_untracked(),
                move |e| match e {
                    BuildEvent::Progress { .. } if result.with_untracked(|r| r.is_some()) => {}
                    BuildEvent::Progress { phase, done, total } => {
                        progress.set(Some((phase, done, total)));
                        let f = match phase {
                            BuildPhase::Pages => 0.05,
                            BuildPhase::Copies => 0.1,
                            BuildPhase::Images if total > 0 => {
                                0.1 + 0.85 * done as f32 / total as f32
                            }
                            BuildPhase::Images => 0.1,
                        };
                        ui.activity.set(Some(f));
                    }
                    BuildEvent::Finished(s) => finish(Ok(s)),
                    BuildEvent::Failed(f) => finish(Err(f)),
                },
            )
            .await;
            if let Err(e) = r
                && result.with_untracked(|r| r.is_none())
            {
                finish(Err(e));
            }
        });
    };
    let envs = site.environments.clone();
    let phase_text = move || {
        progress.get().map(|(phase, done, total)| match phase {
            BuildPhase::Pages => ui.now("build-phase-pages", &[]),
            BuildPhase::Copies => ui.now("build-phase-copies", &[]),
            BuildPhase::Images => ui.now(
                "build-phase-images",
                &[
                    ("done", crate::num(done as f64, 0)),
                    ("total", crate::num(total as f64, 0)),
                ],
            ),
        })
    };
    view! {
        <section class="page settings-page">
            <header class="page-head">
                <div>
                    <h1 title=ui.t("export-lead")>{ui.t("export-title")}</h1>
                </div>
            </header>
            <div class="section">
                <div class="form-grid">
                    <span class="label" title=ui.t("export-env-help")>{ui.t("build-env")}</span>
                    <div class="segmented" role="radiogroup">
                        {envs.into_iter().map(|e| {
                            let v = e.clone();
                            let v2 = e.clone();
                            view! {
                                <button role="radio" class:is-on=move || env.get() == v aria-checked=move || (env.get() == v2).to_string()
                                    on:click=move |_| env.set(e.clone())>{e.clone()}</button>
                            }
                        }).collect_view()}
                    </div>
                    <span class="label" title=ui.t("export-images-help")>{ui.t("build-images")}</span>
                    <label class="switch">
                        <input type="checkbox" prop:checked=move || images.get() on:change=move |ev| images.set(event_target_checked(&ev))/>
                        <span class="switch-track"></span>
                    </label>
                    <span class="label">{ui.t("export-where")}</span>
                    <div class="path-field">
                        <span class="path-value" title=move || parent.get()>{move || crate::ltr(&parent.get())}</span>
                        <button type="button" class="btn btn--small" on:click=change>{ui.t("new-site-change")}</button>
                    </div>
                </div>
                <div class="toolbar toolbar--start">
                    <button class="btn btn--primary" on:click=run disabled=move || running.get() || parent.with(String::is_empty)>
                        {icons::upload()}
                        {move || if running.get() { ui.tx("export-running", &[]) } else { ui.tx("export-run", &[]) }}
                    </button>
                    <span class="muted">{phase_text}</span>
                </div>
            </div>
            {move || result.get().map(|r| match r {
                Ok(s) => {
                    let n = s.warnings.len();
                    let dir = s.out_dir.clone();
                    let msg = ui.tx("build-done", &[
                        ("generated", crate::num(s.generated as f64, 0)),
                        ("copied", crate::num(s.copied as f64, 0)),
                        ("produced", crate::num(s.images_produced as f64, 0)),
                        ("cached", crate::num(s.images_cached as f64, 0)),
                        ("seconds", crate::num(s.seconds as f64, 1)),
                    ]);
                    view! {
                        <div class="section" aria-live="polite">
                            <div class="section-head">
                                <p class="status status--ok">{icons::check()}<span>{msg}</span></p>
                                <button class="btn" on:click=move |_| {
                                    let d = dir.clone();
                                    spawn_local(async move { if let Err(e) = api::reveal(&d).await { ui.fail(e) } });
                                }>{icons::folder()}{ui.t("export-reveal")}</button>
                            </div>
                            <p class="muted path">{s.out_dir.clone()}</p>
                            <Show when=move || { n > 0 }>
                                <h3>{ui.t("build-warnings")}</h3>
                                <ul class="list-plain">{s.warnings.iter().map(|w| view! { <li>{w.clone()}</li> }).collect_view()}</ul>
                            </Show>
                        </div>
                    }.into_any()
                }
                Err(f) => view! { <div class="notice notice--error" role="alert">{ui.tx("export-failed", &[("message", ui.failure(&f))])}</div> }.into_any(),
            })}
        </section>
    }
}
