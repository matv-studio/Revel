//! Paramètres d'une page fixe (3.4, D-32), à la place de la liste dans le panneau Pages (comme
//! Webflow) : nom, adresse (redirection de l'ancienne), brouillon, référencement (titre,
//! description, image de partage, visibilité par les moteurs de recherche) avec un aperçu du
//! résultat de recherche ; dupliquer, supprimer.
//!
//! Le référencement s'écrit là où la page le lit : variable `seo` de la page, ou fichier de
//! données quand `seo` en vient (site importé, aussi modifiable dans Pages CMS) ; une expression
//! calculée n'est pas modifiable ici.

use crate::editor::Ed;
use crate::inspector::Field;
use crate::{Ui, icons, pageops};
use leptos::prelude::*;
use revel_edit::Command;
use revel_edit::canvas::{Binding, Seg, Source};
use serde_json::{Value, json};

/// Écrit une valeur de référencement (`title`, `description`, `image`) ; vide : retirée.
fn write_seo(ui: Ui, ed: Ed, id: &str, data_path: &[String], key: &str, text: String) {
    let text = text.trim().to_string();
    let value = (!text.is_empty()).then(|| json!(text));
    let cmd = if data_path.is_empty() {
        let mut seo = ed
            .doc
            .with_untracked(|d| {
                d.as_ref()?
                    .pages
                    .iter()
                    .find(|p| p.id == id)?
                    .props
                    .get("seo")
                    .cloned()
            })
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        match value {
            Some(v) => {
                seo.insert(key.to_string(), v);
            }
            None => {
                seo.remove(key);
            }
        }
        Command::SetPageProp {
            page: id.to_string(),
            key: "seo".into(),
            value: (!seo.is_empty()).then_some(Value::Object(seo)),
        }
    } else {
        Command::SetDataValue {
            binding: Binding {
                source: Source::Data,
                path: data_path
                    .iter()
                    .map(|k| Seg::Key(k.clone()))
                    .chain([Seg::Key(key.to_string())])
                    .collect(),
            },
            value,
        }
    };
    ed.apply(ui, cmd, false);
}

#[component]
pub fn PageSettings(id: String, on_close: Callback<()>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let id = StoredValue::new(id);
    let page = Memo::new(move |_| {
        ed.doc.with(|d| {
            d.as_ref()?
                .pages
                .iter()
                .find(|p| id.with_value(|i| p.id == *i))
                .cloned()
        })
    });
    let info = Memo::new(move |_| {
        ed.pages.with(|ps| {
            ps.iter()
                .find(|p| id.with_value(|i| p.page.as_deref() == Some(i.as_str())))
                .cloned()
        })
    });
    // Page supprimée (annulation d'une création) : retour à la liste.
    Effect::new(move |_| {
        if ed.doc.with(Option::is_some) && page.with(Option::is_none) {
            on_close.run(());
        }
    });
    let redirect = RwSignal::new(true);
    let bad_path = RwSignal::new(false);
    let site_url = move || {
        ed.doc.with(|d| {
            d.as_ref()
                .and_then(|d| d.base_url.clone())
                .unwrap_or_default()
                .trim_end_matches('/')
                .to_string()
        })
    };
    view! {
        <div class="page-settings">
            <header class="page-settings-head">
                <button class="icon-btn icon-btn--small" title=ui.t("action-back") aria-label=ui.t("action-back")
                    on:click=move |_| on_close.run(())>{icons::back()}</button>
                <span class="page-settings-title">{ui.t("pages-settings")}</span>
            </header>
            {move || page.get().map(|p| {
                let lang = ed.lang();
                let home = p.path == "/";
                let pid = p.id.clone();
                let folder = pageops::folder_of(&p.path);
                let prefix = if folder.is_empty() { "/".to_string() } else { format!("/{folder}/") };
                let slug = pageops::slug_of(&p.path);
                let html_ext = p.path.ends_with(".html");
                let seo = info.get().and_then(|i| i.seo).unwrap_or_default();
                let data_path = StoredValue::new(seo.data_path.clone());
                let computed = seo.computed;
                let (p1, p2, p3, p4, p5, p6, p7) = (pid.clone(), pid.clone(), pid.clone(), pid.clone(), pid.clone(), pid.clone(), pid.clone());
                let draft = p.draft;
                let name = p.name.clone().unwrap_or_default();
                let derived = info.get().map(|i| crate::pagetree::title_of(&i)).unwrap_or_default();
                let shown_title = if seo.title.is_empty() { derived } else { seo.title.clone() };
                let shown_url = format!("{}{}", site_url(), p.path);
                let source_note = if computed {
                    Some(ui.now("pages-seo-computed", &[]))
                } else if !seo.data_path.is_empty() {
                    Some(ui.now("pages-seo-data", &[("file", seo.data_path[..seo.data_path.len().saturating_sub(1)].join("/"))]))
                } else {
                    None
                };
                view! {
                    <div class="page-settings-body">
                        <Field label=ui.t("pages-name") value=Signal::derive(move || name.clone())
                            placeholder=info.get().map(|i| super::pagetree::title_of(&crate::pagetree::EditorPageInfo { name: None, ..i })).unwrap_or_default()
                            on_commit=move |v: String| ed.apply(ui, Command::SetPageName { id: p1.clone(), name: Some(v) }, false)/>
                        <label class="field">
                            <span class="label">{ui.t("pages-path")}</span>
                            <span class="path-input" class:is-bad=move || bad_path.get()>
                                <span class="path-prefix">{prefix.clone()}</span>
                                <input class="input input--mono" prop:value=slug.clone() disabled=home
                                    title=move || if home { ui.now("pages-home-path", &[]) } else { String::new() }
                                    on:input=move |_| bad_path.set(false)
                                    on:change=move |e| {
                                        let s = revel_model::slugify(&event_target_value(&e));
                                        let path = if html_ext { format!("{prefix}{s}.html") } else { format!("{prefix}{s}/") };
                                        if s.is_empty() || !pageops::set_path(ui, ed, &p2, &path, redirect.get_untracked()) {
                                            bad_path.set(true);
                                        }
                                    }/>
                            </span>
                        </label>
                        {(!home && !draft).then(|| view! {
                            <label class="view-line">
                                <span title=ui.t("pages-redirect-help")>{ui.t("pages-redirect")}</span>
                                <span class="switch">
                                    <input type="checkbox" prop:checked=move || redirect.get()
                                        on:change=move |e| redirect.set(event_target_checked(&e))/>
                                    <span class="switch-track"></span>
                                </span>
                            </label>
                        })}
                        <label class="view-line" title=move || if home { ui.now("pages-home-draft", &[]) } else { ui.now("pages-draft-help", &[]) }>
                            <span>{ui.t("pages-draft")}</span>
                            <span class="switch">
                                <input type="checkbox" prop:checked=draft disabled=home
                                    on:change=move |e| pageops::set_draft(ui, ed, &p3, event_target_checked(&e))/>
                                <span class="switch-track"></span>
                            </span>
                        </label>
                        {(!p.path.ends_with('/') && !p.path.ends_with(".html") && !p.path.ends_with(".xml")).then(|| {
                            // Fichier du serveur (`.htaccess`, `robots.txt`, `.php`) : texte écrit tel quel.
                            let text: String = p.body.iter().map(|n| match n {
                                revel_model::Node::Html { html } => html.clone(),
                                revel_model::Node::Text { text } => text.clone(),
                                _ => String::new(),
                            }).collect();
                            let n = p.body.len();
                            let pid = pid.clone();
                            view! {
                                <label class="field page-settings-file">
                                    <span class="label" title=ui.t("pages-file-help")>{ui.t("pages-file")}</span>
                                    <textarea class="input input--mono" rows="14" spellcheck="false" prop:value=text
                                        on:change=move |e| {
                                            let v = event_target_value(&e);
                                            let root = revel_edit::Parent::root(revel_edit::Owner::Page(pid.clone()));
                                            let mut commands = vec![Command::Replace {
                                                parent: root.clone(),
                                                index: 0,
                                                node: revel_model::Node::Html { html: v },
                                            }];
                                            commands.extend((1..n).map(|i| Command::Replace {
                                                parent: root.clone(),
                                                index: i,
                                                node: revel_model::Node::Text { text: String::new() },
                                            }));
                                            ed.apply(ui, Command::Batch { commands }, false);
                                        }></textarea>
                                </label>
                            }
                        })}
                        <h3 class="page-settings-section">{ui.t("pages-look")}</h3>
                        {
                            let body_class = p.props.get("bodyClass").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                            let theme = p.props.get("theme").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                            let (q1, q2) = (pid.clone(), pid.clone());
                            view! {
                                <Field label=ui.t("pages-body-class") value=Signal::derive(move || body_class.clone()) mono=true
                                    on_commit=move |v: String| {
                                        let v = v.split_whitespace().collect::<Vec<_>>().join(" ");
                                        ed.apply(ui, Command::SetPageProp { page: q1.clone(), key: "bodyClass".into(), value: (!v.is_empty()).then(|| json!(v)) }, false)
                                    }/>
                                <label class="field">
                                    <span class="label" title=ui.t("pages-theme-help")>{ui.t("pages-theme")}</span>
                                    <div class="select">
                                        <select class="input" on:change=move |e| {
                                            let v = event_target_value(&e);
                                            ed.apply(ui, Command::SetPageProp { page: q2.clone(), key: "theme".into(), value: (!v.is_empty()).then(|| json!(v)) }, false)
                                        }>
                                            <option value="" prop:selected=theme.is_empty()>{ui.t("pages-theme-site")}</option>
                                            <option value="light" prop:selected=theme == "light">{ui.t("pages-theme-light")}</option>
                                            <option value="dark" prop:selected=theme == "dark">{ui.t("pages-theme-dark")}</option>
                                        </select>
                                    </div>
                                </label>
                            }
                        }
                        <h3 class="page-settings-section">{ui.t("pages-seo")}</h3>
                        {lang.clone().map(|code| {
                            // Autre langue : titre et description traduits (variable de la page) ; ceux
                            // d'un fichier de données se traduisent dans la vue Localisation.
                            let (t_src, d_src) = (seo.title.clone(), seo.description.clone());
                            let from_page = seo.data_path.is_empty() && !computed;
                            let (c1, c2) = (code.clone(), code.clone());
                            let (t2, d2) = (t_src.clone(), d_src.clone());
                            view! {
                                <fieldset class="page-settings-seo page-settings-locale" disabled=!from_page
                                    title=move || if from_page { ui.now("pages-seo-locale", &[]) } else { ui.now("pages-seo-locale-data", &[]) }>
                                    <Field label=ui.t("pages-seo-title") value=ed.translation(&code, &t_src).unwrap_or_default() placeholder=t_src.clone()
                                        on_commit=move |v: String| ed.apply(ui, Command::SetTranslation { locale: c1.clone(), source: t2.clone(), text: Some(v), machine: false }, false)/>
                                    <label class="field">
                                        <span class="label">{ui.t("pages-seo-description")}</span>
                                        <textarea class="input" rows="3" prop:value=ed.translation(&code, &d_src).unwrap_or_default() placeholder=d_src.clone()
                                            on:change=move |e| ed.apply(ui, Command::SetTranslation { locale: c2.clone(), source: d2.clone(), text: Some(event_target_value(&e)), machine: false }, false)></textarea>
                                    </label>
                                </fieldset>
                            }
                        })}
                        <fieldset class="page-settings-seo" disabled=computed class:is-hidden=lang.is_some()>
                            <Field label=ui.t("pages-seo-title") value=Signal::derive({ let t = seo.title.clone(); move || t.clone() })
                                on_commit=move |v: String| write_seo(ui, ed, &p4, &data_path.get_value(), "title", v)/>
                            <label class="field">
                                <span class="label">{ui.t("pages-seo-description")}</span>
                                <textarea class="input" rows="3" prop:value=seo.description.clone()
                                    on:change=move |e| write_seo(ui, ed, &p5, &data_path.get_value(), "description", event_target_value(&e))></textarea>
                            </label>
                            <Field label=ui.t("pages-seo-image") value=Signal::derive({ let t = seo.image.clone(); move || t.clone() })
                                placeholder="/media/partage.jpg"
                                on_commit={ let p = p6.clone(); move |v: String| write_seo(ui, ed, &p, &data_path.get_value(), "image", v) }>
                                <button type="button" class="icon-btn icon-btn--small" title=ui.t("picker-choose-image") aria-label=ui.t("picker-choose-image")
                                    on:click={ let p = p6.clone(); move |_| {
                                        let p = p.clone();
                                        crate::library::pick_image(ui, crate::library::media_shelf(ed), move |url| {
                                            if let Some(dp) = data_path.try_get_value() {
                                                write_seo(ui, ed, &p, &dp, "image", url)
                                            }
                                        })
                                    } }>{icons::image()}</button>
                            </Field>
                        </fieldset>
                        <label class="view-line" title=ui.t("pages-index-help")>
                            <span>{ui.t("pages-index")}</span>
                            <span class="switch">
                                <input type="checkbox" prop:checked=!seo.noindex
                                    on:change=move |e| ed.apply(ui, Command::SetPageProp {
                                        page: p7.clone(),
                                        key: "noindex".into(),
                                        value: (!event_target_checked(&e)).then_some(json!(true)),
                                    }, false)/>
                                <span class="switch-track"></span>
                            </span>
                        </label>
                        {source_note.map(|n| view! { <p class="page-settings-note muted">{n}</p> })}
                        <div class="serp" aria-label=ui.t("pages-serp")>
                            <span class="serp-url">{shown_url}</span>
                            <span class="serp-title">{shown_title}</span>
                            <span class="serp-desc">{seo.description.clone()}</span>
                        </div>
                        <crate::readable::PagePicks page=pid.clone()/>
                        <h3 class="page-settings-section" title=ui.t("page-code-help")>{ui.t("page-code")}</h3>
                        {
                            let (c1, c2) = (p.code.clone(), p.code.clone());
                            let (q1, q2) = (pid.clone(), pid.clone());
                            view! {
                                <div class="page-settings-code">
                                    {crate::sitesettings::code_field(ui, "code-head", "code-head-help", move || c1.head.clone(), move |v| {
                                        let mut c = page.get_untracked().map(|p| p.code).unwrap_or_default();
                                        c.head = v;
                                        ed.apply(ui, Command::SetCode { page: Some(q1.clone()), code: c }, false)
                                    })}
                                    {crate::sitesettings::code_field(ui, "code-body", "code-body-help", move || c2.body.clone(), move |v| {
                                        let mut c = page.get_untracked().map(|p| p.code).unwrap_or_default();
                                        c.body = v;
                                        ed.apply(ui, Command::SetCode { page: Some(q2.clone()), code: c }, false)
                                    })}
                                </div>
                            }
                        }
                    </div>
                    <footer class="panel-foot page-settings-foot">
                        <button class="btn btn--small" on:click={let i = pid.clone(); move |_| pageops::duplicate(ui, ed, &i)}>
                            {icons::copy()}{ui.t("pages-duplicate")}</button>
                        <span class="toolbar-spacer"></span>
                        <button class="btn btn--small btn--danger-ghost" disabled=home
                            title=move || if home { ui.now("pages-home-delete", &[]) } else { String::new() }
                            on:click={let i = pid.clone(); move |_| pageops::delete(ui, ed, &i)}>
                            {icons::trash()}{ui.t("pages-delete")}</button>
                    </footer>
                }
            })}
        </div>
    }
}
