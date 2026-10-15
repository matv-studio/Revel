//! Paramètres du site (3.9) : identité (nom, adresse, langue principale, typographie), code
//! ajouté aux pages, polices (Google Fonts téléchargées dans le site, fichiers importés, D-24).

use crate::dialog::Ask;
use crate::site::{DocSession, doc_session};
use crate::{Tab, Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::Command;
use revel_protocol::{FontFamily, FontUpload, GoogleFont, SiteSummary};

/// Identité du site : valeurs du document et commande qui les remplace toutes.
#[derive(Clone, PartialEq)]
struct Info {
    name: String,
    base_url: Option<String>,
    locale: String,
    typography: Option<String>,
}

fn info(s: DocSession) -> Option<Info> {
    s.doc.with(|d| {
        d.as_ref().map(|d| Info {
            name: d.name.clone(),
            base_url: d.base_url.clone(),
            locale: d.default_locale.clone(),
            typography: d.settings.typography.clone(),
        })
    })
}

/// Paramètres généraux : identité modifiable, structure, environnements.
#[component]
pub fn General(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let s = doc_session(ui, site.path.clone());
    // Résumé relu (structure du site).
    let fresh = RwSignal::new(site.clone());
    let path = StoredValue::new(site.path.clone());
    let refresh = move || {
        // Vue quittée avant la relecture : rien à faire.
        let Some(p) = path.try_get_value() else {
            return;
        };
        spawn_local(async move {
            if let Ok(sum) = api::open_site(&p).await {
                if !fresh.is_disposed() {
                    fresh.set(sum);
                }
                ui.reload_recents();
            }
        });
    };
    refresh();
    let set = move |f: &dyn Fn(&mut Info)| {
        let Some(mut v) = info(s) else { return };
        let before = v.clone();
        f(&mut v);
        if v == before {
            return;
        }
        s.apply.run(Command::SetSiteInfo {
            name: v.name,
            base_url: v.base_url,
            default_locale: v.locale,
            typography: v.typography,
        });
        // Résumé relu après l'écriture (liste des sites récents).
        set_timeout(refresh, std::time::Duration::from_millis(250));
    };
    let current = move || info(s);
    // Une valeur par champ : un champ en cours de saisie n'est pas remis à zéro quand un autre
    // est enregistré.
    let name = Memo::new(move |_| current().map(|i| i.name).unwrap_or_default());
    let url = Memo::new(move |_| current().and_then(|i| i.base_url).unwrap_or_default());
    let langs = move || {
        let taken: Vec<String> = s.doc.with(|d| {
            d.as_ref()
                .map(|d| d.locales.iter().map(|l| l.code.clone()).collect())
                .unwrap_or_default()
        });
        let cur = current().map(|i| i.locale).unwrap_or_default();
        let mut list: Vec<String> = vec![cur.clone()];
        list.extend(
            crate::locales::COMMON
                .iter()
                .map(|c| c.to_string())
                .filter(|c| *c != cur && !taken.contains(c)),
        );
        list
    };
    let n_cols = move || {
        fresh.with(|s| {
            s.collections
                .iter()
                .filter(|c| !c.folder.is_empty())
                .count()
        })
    };
    view! {
        <section class="page settings-page">
            <header class="page-head"><h1>{ui.t("settings-general")}</h1></header>
            <div class="section">
                <h2>{ui.t("settings-identity")}</h2>
                <div class="form-grid">
                    <label class="label" for="site-name">{ui.t("settings-site-name")}</label>
                    <input id="site-name" class="input" spellcheck="false"
                        prop:value=name
                        on:change=move |e| {
                            let v = event_target_value(&e);
                            if v.trim().is_empty() {
                                event_target::<web_sys::HtmlInputElement>(&e).set_value(&name.get_untracked());
                            } else {
                                set(&|i| i.name = v.trim().to_string())
                            }
                        }/>
                    <label class="label" for="site-url" title=ui.t("settings-site-url-help")>{ui.t("settings-site-url")}</label>
                    <input id="site-url" class="input input--mono" spellcheck="false" placeholder="https://exemple.com"
                        prop:value=url
                        on:change=move |e| {
                            let v = event_target_value(&e);
                            set(&|i| i.base_url = Some(v.trim().to_string()).filter(|u| !u.is_empty()));
                            // Adresse refusée : la valeur enregistrée revient.
                            let v = v.trim();
                            if !(v.is_empty() || v.starts_with("https://") || v.starts_with("http://")) {
                                event_target::<web_sys::HtmlInputElement>(&e).set_value(&url.get_untracked());
                            }
                        }/>
                    <label class="label" for="site-lang" title=ui.t("settings-site-lang-help")>{ui.t("settings-site-lang")}</label>
                    <select id="site-lang" class="input"
                        on:change=move |e| { let v = event_target_value(&e); set(&|i| i.locale = v.clone()) }>
                        {move || {
                            let cur = current().map(|i| i.locale).unwrap_or_default();
                            langs().into_iter().map(|c| view! {
                                <option value=c.clone() selected=c == cur>{format!("{} · {c}", revel_model::language_name(&c))}</option>
                            }).collect_view()
                        }}
                    </select>
                    <span class="label" title=ui.t("settings-typography-help")>{ui.t("settings-typography")}</span>
                    <span class="switch">
                        <input type="checkbox" aria-label=ui.t("settings-typography")
                            prop:checked=move || current().is_some_and(|i| i.typography.as_deref() == Some("fr"))
                            on:change=move |e| { let on = event_target_checked(&e); set(&|i| i.typography = on.then(|| "fr".to_string())) }/>
                        <span class="switch-track"></span>
                    </span>
                    <span class="label">{ui.t("settings-site-folder")}</span>
                    <span class="muted path">{move || fresh.with(|s| s.path.clone())}</span>
                </div>
            </div>
            <div class="section">
                <h2>{ui.t("overview-structure")}</h2>
                <dl class="facts">
                    <div><dt>{ui.t("site-pages")}</dt><dd>{move || fresh.with(|s| s.pages)}</dd></div>
                    <div><dt>{ui.t("site-layouts")}</dt><dd>{move || fresh.with(|s| s.layouts)}</dd></div>
                    <div><dt>{ui.t("site-components")}</dt><dd>{move || fresh.with(|s| s.components)}</dd></div>
                    <div>
                        <dt>{ui.t("site-collections")}</dt>
                        <dd><button class="link-btn" title=ui.t("view-cms") on:click=move |_| ui.tab(Tab::Cms)>{n_cols}</button></dd>
                    </div>
                </dl>
            </div>
            <div class="section">
                <h2>{ui.t("overview-environments")}</h2>
                <div class="chips">{move || fresh.with(|s| s.environments.clone()).into_iter().map(|e| view! { <span class="chip">{e}</span> }).collect_view()}</div>
            </div>
        </section>
    }
}

/// Champ de code (police à chasse fixe), écrit à la sortie du champ.
pub fn code_field(
    ui: Ui,
    key: &'static str,
    help: &'static str,
    value: impl Fn() -> String + Send + Sync + 'static,
    on: impl Fn(String) + 'static,
) -> impl IntoView {
    let tag = if key == "code-head" {
        "</head>"
    } else {
        "</body>"
    };
    view! {
        <label class="field code-field">
            <span class="label" title=ui.t(help)>{ui.t(key)}<code class="code-tag">{tag}</code></span>
            <textarea class="input input--mono code-input" rows="8" spellcheck="false" wrap="off"
                placeholder="<!-- … -->" prop:value=value
                on:change=move |e| on(event_target_value(&e))></textarea>
        </label>
    }
}

/// Code ajouté à toutes les pages (avant `</head>`, avant `</body>`).
#[component]
pub fn Code(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let s = doc_session(ui, site.path.clone());
    let code = move || {
        s.doc.with(|d| {
            d.as_ref()
                .map(|d| d.settings.code.clone())
                .unwrap_or_default()
        })
    };
    // Un champ en cours de saisie n'est pas remis à zéro quand l'autre est enregistré.
    let head = Memo::new(move |_| code().head);
    let body = Memo::new(move |_| code().body);
    let set = move |head: Option<String>, body: Option<String>| {
        let mut c = code();
        if let Some(h) = head {
            c.head = h;
        }
        if let Some(b) = body {
            c.body = b;
        }
        if c != code() {
            s.apply.run(Command::SetCode {
                page: None,
                code: c,
            });
        }
    };
    // Pages qui ont leur propre code (paramètres de la page).
    let pages = move || {
        s.doc.with(|d| {
            d.as_ref()
                .map(|d| {
                    d.pages
                        .iter()
                        .filter(|p| !p.code.is_empty())
                        .map(|p| p.name.clone().unwrap_or_else(|| p.path.clone()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    };
    view! {
        <section class="page settings-page">
            <header class="page-head"><h1 title=ui.t("code-help")>{ui.t("settings-code")}</h1></header>
            <div class="section">
                {code_field(ui, "code-head", "code-head-help", move || head.get(), move |v| set(Some(v), None))}
                {code_field(ui, "code-body", "code-body-help", move || body.get(), move |v| set(None, Some(v)))}
            </div>
            {move || {
                let p = pages();
                (!p.is_empty()).then(|| view! {
                    <div class="section">
                        <h2 title=ui.t("code-pages-help")>{ui.t("code-pages")}</h2>
                        <div class="chips">{p.into_iter().map(|n| view! { <span class="chip">{n}</span> }).collect_view()}</div>
                    </div>
                })
            }}
        </section>
    }
}

/// Graisse et style lus dans un nom de fichier (`Inter-SemiBoldItalic.woff2`).
fn guess_face(name: &str) -> (String, bool) {
    let stem = name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(name)
        .to_ascii_lowercase()
        .replace(['-', '_', ' '], "");
    let italic = stem.contains("italic") || stem.ends_with("it") || stem.contains("oblique");
    let table = [
        ("extralight", "200"),
        ("ultralight", "200"),
        ("semibold", "600"),
        ("demibold", "600"),
        ("extrabold", "800"),
        ("ultrabold", "800"),
        ("thin", "100"),
        ("hairline", "100"),
        ("light", "300"),
        ("medium", "500"),
        ("bold", "700"),
        ("black", "900"),
        ("heavy", "900"),
    ];
    let w = table
        .iter()
        .find(|(k, _)| stem.contains(k))
        .map(|(_, w)| *w)
        .unwrap_or("400");
    (w.to_string(), italic)
}

/// Nom de famille lu dans un nom de fichier (`BrandSans-Bold.woff2` -> « Brand Sans »).
fn guess_family(name: &str) -> String {
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
    let base = stem.split(['-', '_']).next().unwrap_or(stem);
    let mut out = String::new();
    let mut prev_lower = false;
    for c in base.chars() {
        if c.is_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_lowercase();
        out.push(c);
    }
    out.trim().to_string()
}

/// Libellé d'une graisse (`400` -> « 400 · Normal »).
fn weight_label(ui: Ui, w: &str) -> String {
    let key = format!("weight-{w}");
    let name = ui.now(&key, &[]);
    if name == key {
        w.to_string()
    } else {
        format!("{w} · {name}")
    }
}

/// Variante de Google Fonts lisible (`700i` -> « 700 italique »).
fn variant_label(ui: Ui, v: &str) -> String {
    let w = v.trim_end_matches('i');
    if v.ends_with('i') {
        format!("{w} {}", ui.now("fonts-italic", &[]))
    } else {
        w.to_string()
    }
}

/// Fichier choisi pour l'import, avec sa graisse et son style.
#[derive(Clone)]
struct Pending {
    name: String,
    data: String,
    weight: RwSignal<String>,
    italic: RwSignal<bool>,
}

/// Polices du site : familles (graisses, utilisation, taille), ajout depuis Google Fonts,
/// import de fichiers, retrait (annulable).
#[component]
pub fn Fonts(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let path = StoredValue::new(site.path.clone());
    let list = RwSignal::new(None::<Vec<FontFamily>>);
    let reload = move || {
        let Some(p) = path.try_get_value() else {
            return;
        };
        spawn_local(async move {
            // Session d'édition ouverte (historique partagé avec l'éditeur).
            let _ = api::edit_open(&p).await;
            match api::fonts(&p).await {
                Ok(l) => {
                    if !list.is_disposed() {
                        list.set(Some(l))
                    }
                }
                Err(e) => ui.fail(e),
            }
        })
    };
    reload();
    let busy = RwSignal::new(false);
    let done = move |r: Result<revel_edit::EditState, revel_protocol::Failure>| {
        // La vue a pu être quittée pendant le téléchargement.
        let _ = busy.try_set(false);
        ui.activity.set(None);
        if let Err(e) = r {
            ui.fail(e);
        }
        reload();
    };
    // Fenêtre Google Fonts.
    let google = RwSignal::new(false);
    let catalog = RwSignal::new(None::<Vec<GoogleFont>>);
    let query = RwSignal::new(String::new());
    let chosen = RwSignal::new(None::<GoogleFont>);
    let variants = RwSignal::new(Vec::<String>::new());
    let open_google = move || {
        query.set(String::new());
        chosen.set(None);
        google.set(true);
        if catalog.with_untracked(Option::is_none) {
            spawn_local(async move {
                match api::fonts_google().await {
                    Ok(c) => catalog.set(Some(c)),
                    Err(e) => {
                        google.set(false);
                        ui.fail(e)
                    }
                }
            });
        }
    };
    let pick = move |f: GoogleFont| {
        let mut v: Vec<String> = ["400", "700"]
            .iter()
            .filter(|w| f.variants.iter().any(|x| x == *w))
            .map(|w| w.to_string())
            .collect();
        if v.is_empty() {
            v = f.variants.iter().take(1).cloned().collect();
        }
        variants.set(v);
        chosen.set(Some(f));
    };
    let add_google = move || {
        let Some(f) = chosen.get_untracked() else {
            return;
        };
        let v = variants.get_untracked();
        if v.is_empty() {
            return;
        }
        google.set(false);
        busy.set(true);
        ui.activity.set(Some(0.3));
        let p = path.get_value();
        spawn_local(async move { done(api::fonts_add_google(&p, &f.family, &v).await) });
    };
    let results = move || {
        let q = query.get().trim().to_lowercase();
        catalog.with(|c| {
            c.as_ref()
                .map(|c| {
                    c.iter()
                        .filter(|f| q.is_empty() || f.family.to_lowercase().contains(&q))
                        .take(40)
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    };
    // Fenêtre d'import de fichiers.
    let pending = RwSignal::new(None::<(RwSignal<String>, Vec<Pending>)>);
    let import = move || {
        crate::files::pick(".woff2,.woff,.ttf,.otf", true, move |fs| {
            if fs.is_empty() {
                return;
            }
            spawn_local(async move {
                use base64::Engine as _;
                let mut out = Vec::new();
                for f in fs {
                    let Ok(bytes) = crate::files::read(&f).await else {
                        continue;
                    };
                    let name = f.name();
                    let (w, it) = guess_face(&name);
                    out.push(Pending {
                        data: base64::engine::general_purpose::STANDARD.encode(bytes),
                        weight: RwSignal::new(w),
                        italic: RwSignal::new(it),
                        name,
                    });
                }
                if let Some(first) = out.first() {
                    let fam = RwSignal::new(guess_family(&first.name));
                    pending.set(Some((fam, out)));
                }
            });
        });
    };
    let add_files = move || {
        let Some((fam, files)) = pending.get_untracked() else {
            return;
        };
        let family = fam.get_untracked().trim().to_string();
        if family.is_empty() {
            return;
        }
        let up: Vec<FontUpload> = files
            .iter()
            .map(|f| FontUpload {
                name: f.name.clone(),
                data: f.data.clone(),
                weight: f.weight.get_untracked(),
                italic: f.italic.get_untracked(),
            })
            .collect();
        pending.set(None);
        busy.set(true);
        ui.activity.set(Some(0.3));
        let p = path.get_value();
        spawn_local(async move { done(api::fonts_add_files(&p, &family, &up).await) });
    };
    let remove = move |family: String| {
        ui.ask.set(Some(
            Ask::confirm(
                ui.now("fonts-remove-title", &[("name", family.clone())]),
                None,
                ui.now("action-remove", &[]),
                move || {
                    let (p, f) = (path.get_value(), family.clone());
                    spawn_local(async move { done(api::fonts_remove(&p, &f).await) });
                },
            )
            .danger(),
        ));
    };
    let esc = window_event_listener(leptos::ev::keydown, move |e| {
        if e.key() == "Escape"
            && (google.get_untracked() || pending.with_untracked(Option::is_some))
        {
            e.prevent_default();
            google.set(false);
            pending.set(None);
        }
    });
    on_cleanup(move || esc.remove());
    let weights = [
        "100", "200", "300", "400", "500", "600", "700", "800", "900",
    ];
    view! {
        <section class="page settings-page fonts-page">
            <header class="page-head">
                <h1 title=ui.t("fonts-help")>{ui.t("settings-fonts")}</h1>
                <div class="toolbar">
                    <button class="btn" disabled=move || busy.get() on:click=move |_| import()>{icons::upload()}{ui.t("fonts-import")}</button>
                    <button class="btn btn--primary" disabled=move || busy.get() title=ui.t("fonts-google-help")
                        on:click=move |_| open_google()>{icons::plus()}{ui.t("fonts-google")}</button>
                </div>
            </header>
            <div class="section section--flush">
                <ul class="font-list">
                    {move || match list.get() {
                        None => ().into_any(),
                        Some(l) if l.is_empty() => view! { <li class="muted font-empty">{ui.t("fonts-none")}</li> }.into_any(),
                        Some(l) => l.into_iter().map(|f| {
                            let fam = f.family.clone();
                            view! {
                                <li class="font-row" data-family=f.family.clone()>
                                    <span class="font-name">{f.family.clone()}</span>
                                    <span class="font-faces">
                                        {f.faces.iter().map(|x| {
                                            // Police variable : plage de graisses (`100 900` -> `100–900`).
                                            let w = x.weight.split_whitespace().collect::<Vec<_>>().join("–");
                                            let label = if x.style == "normal" { w } else { format!("{w} {}", ui.now("fonts-italic", &[])) };
                                            let title = if x.weight.contains(' ') { ui.now("fonts-variable", &[]) } else { weight_label(ui, &x.weight) };
                                            view! { <span class="chip" title=title>{label}</span> }
                                        }).collect_view()}
                                    </span>
                                    <span class="muted font-uses" class:is-unused=f.uses == 0>
                                        {if f.uses == 0 { ui.now("fonts-unused", &[]) } else { ui.now("fonts-uses", &[("n", f.uses.to_string())]) }}
                                    </span>
                                    <span class="muted font-size">{if f.size > 0 { crate::settings::size(ui, f.size) } else { String::new() }}</span>
                                    <button class="icon-btn icon-btn--small" title=ui.t("fonts-remove") aria-label=ui.t("fonts-remove")
                                        disabled=move || busy.get() on:click=move |_| remove(fam.clone())>{icons::trash()}</button>
                                </li>
                            }
                        }).collect_view().into_any(),
                    }}
                    {move || busy.get().then(|| view! { <li class="font-row font-busy"><span class="meter-bar is-busy"><span class="meter-fill"></span></span></li> })}
                </ul>
            </div>
            {move || google.get().then(|| view! {
                <div class="overlay" on:mousedown=move |e| if e.target() == e.current_target() { google.set(false) }>
                    <form class="dialog fonts-dialog" role="dialog" aria-modal="true" aria-labelledby="gf-title"
                        on:submit=move |e| { e.prevent_default(); add_google() }>
                        <h2 id="gf-title">{ui.t("fonts-google")}</h2>
                        <label class="panel-search">
                            {icons::search()}
                            <input type="search" placeholder=ui.t("fonts-search") aria-label=ui.t("fonts-search") autofocus=true
                                prop:value=move || query.get() on:input=move |e| { query.set(event_target_value(&e)); chosen.set(None) }/>
                        </label>
                        {move || match chosen.get() {
                            None => view! {
                                <ul class="gf-list" role="listbox">
                                    {move || if catalog.with(Option::is_none) {
                                        view! { <li class="muted gf-wait">{ui.t("fonts-loading")}</li> }.into_any()
                                    } else {
                                        results().into_iter().map(|f| {
                                            let f2 = f.clone();
                                            view! {
                                                <li role="option" class="gf-item" data-family=f.family.clone() on:click=move |_| pick(f2.clone())>
                                                    <span class="gf-name">{f.family.clone()}</span>
                                                    <span class="muted gf-meta">{format!("{} · {}", f.category, f.variants.len())}</span>
                                                </li>
                                            }
                                        }).collect_view().into_any()
                                    }}
                                </ul>
                            }.into_any(),
                            Some(f) => view! {
                                <div class="gf-chosen">
                                    <div class="gf-head">
                                        <button type="button" class="icon-btn icon-btn--small" title=ui.t("action-back") aria-label=ui.t("action-back")
                                            on:click=move |_| chosen.set(None)>{icons::back()}</button>
                                        <span class="gf-name strong">{f.family.clone()}</span>
                                        <span class="muted gf-meta">{f.category.clone()}</span>
                                    </div>
                                    <div class="gf-variants" role="group" aria-label=ui.t("fonts-variants")>
                                        {f.variants.iter().map(|v| {
                                            let v2 = v.clone();
                                            let v3 = v.clone();
                                            view! {
                                                <button type="button" class="chip-toggle" title=weight_label(ui, v.trim_end_matches('i'))
                                                    class:is-on=move || variants.with(|l| l.contains(&v2))
                                                    on:click=move |_| variants.update(|l| if let Some(i) = l.iter().position(|x| *x == v3) { l.remove(i); } else { l.push(v3.clone()) })>
                                                    {variant_label(ui, v)}
                                                </button>
                                            }
                                        }).collect_view()}
                                    </div>
                                </div>
                            }.into_any(),
                        }}
                        <div class="dialog-actions">
                            <button type="button" class="btn" on:click=move |_| google.set(false)>{ui.t("action-cancel")}</button>
                            <button type="submit" class="btn btn--primary" title=ui.t("fonts-add-help")
                                disabled=move || chosen.with(Option::is_none) || variants.with(Vec::is_empty)>{ui.t("action-add")}</button>
                        </div>
                    </form>
                </div>
            })}
            {move || pending.get().map(|(fam, files)| view! {
                <div class="overlay" on:mousedown=move |e| if e.target() == e.current_target() { pending.set(None) }>
                    <form class="dialog fonts-dialog" role="dialog" aria-modal="true" aria-labelledby="fi-title"
                        on:submit=move |e| { e.prevent_default(); add_files() }>
                        <h2 id="fi-title">{ui.t("fonts-import")}</h2>
                        <label class="field">
                            <span class="label">{ui.t("fonts-family")}</span>
                            <input class="input" spellcheck="false" prop:value=move || fam.get() on:input=move |e| fam.set(event_target_value(&e))/>
                        </label>
                        <ul class="fi-list">
                            {files.into_iter().map(|f| view! {
                                <li class="fi-row">
                                    <span class="fi-name path" title=f.name.clone()>{f.name.clone()}</span>
                                    <select class="input input--small" aria-label=ui.t("styles-font-weight")
                                        on:change=move |e| f.weight.set(event_target_value(&e))>
                                        {weights.iter().map(|w| view! {
                                            <option value=*w selected=f.weight.get_untracked() == *w>{weight_label(ui, w)}</option>
                                        }).collect_view()}
                                    </select>
                                    <label class="fi-italic" title=ui.t("fonts-italic")>
                                        <span class="switch">
                                            <input type="checkbox" aria-label=ui.t("fonts-italic") prop:checked=move || f.italic.get()
                                                on:change=move |e| f.italic.set(event_target_checked(&e))/>
                                            <span class="switch-track"></span>
                                        </span>
                                        <span>{ui.t("fonts-italic")}</span>
                                    </label>
                                </li>
                            }).collect_view()}
                        </ul>
                        <div class="dialog-actions">
                            <button type="button" class="btn" on:click=move |_| pending.set(None)>{ui.t("action-cancel")}</button>
                            <button type="submit" class="btn btn--primary" disabled=move || fam.with(|f| f.trim().is_empty())>{ui.t("action-add")}</button>
                        </div>
                    </form>
                </div>
            })}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_de_fichiers() {
        assert_eq!(
            guess_face("Inter-SemiBoldItalic.woff2"),
            ("600".into(), true)
        );
        assert_eq!(guess_face("brand_sans-bold.ttf"), ("700".into(), false));
        assert_eq!(guess_face("Brand-ExtraLight.otf"), ("200".into(), false));
        assert_eq!(guess_face("Brand-Regular.woff"), ("400".into(), false));
        assert_eq!(guess_family("BrandSans-Bold.woff2"), "Brand Sans");
        assert_eq!(guess_family("inter_regular.woff2"), "inter");
    }
}
