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
                        <h3 class="page-settings-section">{ui.t("pages-seo")}</h3>
                        <fieldset class="page-settings-seo" disabled=computed>
                            <Field label=ui.t("pages-seo-title") value=Signal::derive({ let t = seo.title.clone(); move || t.clone() })
                                on_commit=move |v: String| write_seo(ui, ed, &p4, &data_path.get_value(), "title", v)/>
                            <label class="field">
                                <span class="label">{ui.t("pages-seo-description")}</span>
                                <textarea class="input" rows="3" prop:value=seo.description.clone()
                                    on:change=move |e| write_seo(ui, ed, &p5, &data_path.get_value(), "description", event_target_value(&e))></textarea>
                            </label>
                            <Field label=ui.t("pages-seo-image") value=Signal::derive({ let t = seo.image.clone(); move || t.clone() })
                                placeholder="/media/partage.jpg"
                                on_commit=move |v: String| write_seo(ui, ed, &p6, &data_path.get_value(), "image", v)/>
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
