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
                            // Écrit en partie par des boucles ou des conditions (contenus, environnement
                            // de publication) : montré, pas modifiable ici (le texte seul les écraserait).
                            let computed = p.body.iter().any(|n| !matches!(n, revel_model::Node::Html { .. } | revel_model::Node::Text { .. }));
                            let help = if computed { "pages-file-computed" } else { "pages-file-help" };
                            view! {
                                <label class="field page-settings-file">
                                    <span class="label" title=ui.t(help)>
                                        {ui.t("pages-file")}
                                        {computed.then(|| view! { <span class="seo-warn">{icons::warning()}</span> })}
                                    </span>
                                    <textarea class="input input--mono" rows="14" spellcheck="false" prop:value=text disabled=computed
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
                                {
                                    let og = p.props.get("ogType").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                    let q3 = pid.clone();
                                    share_type(ui, Signal::derive(move || og.clone()), Callback::new(move |v: String| {
                                        ed.apply(ui, Command::SetPageProp { page: q3.clone(), key: "ogType".into(), value: (!v.is_empty()).then(|| json!(v)) }, false)
                                    }))
                                }
                                {
                                    // Date de publication : annoncée au partage d'un article.
                                    let date = p.props.get("date").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                    let q4 = pid.clone();
                                    view! {
                                        <Field label=ui.t("pages-date") value=Signal::derive(move || date.clone()) mono=true
                                            placeholder="2026-03-01".to_string()
                                            on_commit=move |v: String| {
                                                let v = v.trim().to_string();
                                                ed.apply(ui, Command::SetPageProp { page: q4.clone(), key: "date".into(), value: (!v.is_empty()).then(|| json!(v)) }, false)
                                            }/>
                                    }
                                }
                            }
                        }
                        {layout_props(ui, ed, StoredValue::new(pid.clone()))}
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

/// Réglages du modèle des pages d'une collection (gabarit) : classe du corps de page, thème et
/// type de partage, pour toutes les pages de la collection (une page peut les changer).
#[component]
pub fn TemplateSettings(layout: String, on_close: Callback<()>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let layout = StoredValue::new(layout);
    let props = Memo::new(move |_| {
        ed.doc.with(|d| {
            d.as_ref()?
                .layouts
                .iter()
                .find(|l| layout.with_value(|i| l.id == *i))
                .map(|l| l.props.clone())
        })
    });
    let text = move |k: &str| {
        props
            .get()
            .and_then(|p| p.get(k).and_then(|v| v.as_str()).map(str::to_string))
            .unwrap_or_default()
    };
    let set = move |key: &'static str, v: String| {
        let v = v.split_whitespace().collect::<Vec<_>>().join(" ");
        ed.apply(
            ui,
            Command::SetLayoutProp {
                layout: layout.get_value(),
                key: key.into(),
                value: (!v.is_empty()).then(|| json!(v)),
            },
            false,
        );
    };
    view! {
        <div class="page-settings">
            <header class="page-settings-head">
                <button class="icon-btn icon-btn--small" title=ui.t("action-back") aria-label=ui.t("action-back")
                    on:click=move |_| on_close.run(())>{icons::back()}</button>
                <span class="page-settings-title">{ui.t("template-settings")}</span>
            </header>
            <div class="page-settings-body">
                <p class="muted small">{ui.t("template-settings-help")}</p>
                <h3 class="page-settings-section">{ui.t("pages-look")}</h3>
                <Field label=ui.t("pages-body-class") value=Signal::derive(move || text("bodyClass")) mono=true
                    on_commit=move |v: String| set("bodyClass", v)/>
                <label class="field">
                    <span class="label" title=ui.t("pages-theme-help")>{ui.t("pages-theme")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| set("theme", event_target_value(&e))>
                            <option value="" prop:selected=move || text("theme").is_empty()>{ui.t("pages-theme-site")}</option>
                            <option value="light" prop:selected=move || text("theme") == "light">{ui.t("pages-theme-light")}</option>
                            <option value="dark" prop:selected=move || text("theme") == "dark">{ui.t("pages-theme-dark")}</option>
                        </select>
                    </div>
                </label>
                <h3 class="page-settings-section">{ui.t("pages-seo")}</h3>
                {share_type(ui, Signal::derive(move || text("ogType")), Callback::new(move |v: String| set("ogType", v)))}
            </div>
        </div>
    }
}

/// Type de contenu annoncé au partage (`og:type`) : page ou article.
fn share_type(ui: Ui, cur: Signal<String>, set: Callback<String>) -> impl IntoView {
    view! {
        <label class="field">
            <span class="label" title=ui.t("pages-og-type-help")>{ui.t("pages-og-type")}</span>
            <div class="select">
                <select class="input" on:change=move |e| set.run(event_target_value(&e))>
                    <option value="" prop:selected=move || cur.get().is_empty()>{ui.t("pages-og-type-auto")}</option>
                    <option value="website" prop:selected=move || cur.get() == "website">{ui.t("pages-og-type-website")}</option>
                    <option value="article" prop:selected=move || cur.get() == "article">{ui.t("pages-og-type-article")}</option>
                </select>
            </div>
        </label>
    }
}

/// Propriétés du gabarit de la page (D-96) : valeur choisie pour cette page (interrupteur, texte,
/// image, lien), comme les propriétés d'une utilisation de composant ; ajout (au gabarit le plus
/// extérieur, lisible par tous ceux qu'il contient), nom et retrait d'une propriété, pour toutes
/// les pages du gabarit. Les éléments du gabarit s'y relient depuis l'onglet Styles.
fn layout_props(ui: Ui, ed: Ed, page: StoredValue<String>) -> impl IntoView {
    use crate::components::{
        Holder, KINDS, default_bool, default_text, kind_icon, kind_icon_fn, kind_key, new_prop,
    };
    use revel_model::{PropDef, PropKind};
    // (gabarit de la page, gabarit le plus extérieur, (propriétaire, propriété)…, valeurs).
    type Rows = (
        String,
        String,
        Vec<(String, PropDef)>,
        std::collections::BTreeMap<String, Value>,
    );
    let data = Memo::new(move |_| -> Option<Rows> {
        ed.doc.with(|d| {
            let d = d.as_ref()?;
            let p = d.pages.iter().find(|p| page.with_value(|i| p.id == *i))?;
            let first = p.layout.clone()?;
            let mut rows: Vec<(String, PropDef)> = Vec::new();
            let mut seen: Vec<String> = Vec::new();
            let mut cur = Some(first.clone());
            while let Some(id) = cur {
                if seen.contains(&id) {
                    break;
                }
                let Some(l) = d.layouts.iter().find(|l| l.id == id) else {
                    break;
                };
                for f in &l.fields {
                    if !rows.iter().any(|(_, r)| r.name == f.name) {
                        rows.push((l.id.clone(), f.clone()));
                    }
                }
                seen.push(id);
                cur = l.parent.clone();
            }
            let outer = seen.last().cloned().unwrap_or_else(|| first.clone());
            Some((first, outer, rows, p.props.clone()))
        })
    });
    let set_value = move |name: String, v: Option<Value>| {
        ed.apply(
            ui,
            Command::SetPageProp {
                page: page.get_value(),
                key: name,
                value: v,
            },
            false,
        )
    };
    let add = move |e: leptos::ev::MouseEvent| {
        use crate::ctxmenu::{CtxItem, open};
        let Some((_, outer, rows, _)) = data.get_untracked() else {
            return;
        };
        let outer = StoredValue::new(outer);
        let visible: Vec<PropDef> = rows.iter().map(|(_, p)| p.clone()).collect();
        let items = KINDS
            .into_iter()
            .filter(|k| *k != PropKind::List)
            .map(|k| {
                let visible = visible.clone();
                CtxItem::new(ui.now(kind_key(k), &[]), move || {
                    let h = Holder::Layout(outer.get_value());
                    let Some(mut own) = ed.doc.with_untracked(|d| h.own(d.as_ref()?)) else {
                        return;
                    };
                    let mut p = new_prop(ui, &visible, k, None);
                    // Interrupteur : éteint tant que la page ne l'allume pas.
                    if k == PropKind::Switch {
                        p.default = None;
                    }
                    own.push(p);
                    ed.apply(ui, h.set(own), false);
                })
                .icon(kind_icon_fn(k))
            })
            .collect();
        open(ui, &e, items);
    };
    move || {
        let (_, _, rows, values) = data.get()?;
        let list = rows.into_iter().map(|(owner, f)| {
            let h = StoredValue::new(Holder::Layout(owner));
            let name = StoredValue::new(f.name.clone());
            let kind = f.kind;
            let rename = move |e: leptos::ev::Event| {
                let v = event_target_value(&e).trim().to_string();
                let Some(mut own) = ed.doc.with_untracked(|d| h.with_value(|h| h.own(d.as_ref()?))) else { return };
                if let Some(p) = own.iter_mut().find(|p| name.with_value(|n| &p.name == n)) {
                    p.label = (!v.is_empty()).then_some(v);
                }
                ed.apply(ui, h.with_value(|h| h.set(own)), false);
            };
            let remove = move |_| {
                let cmd = ed.doc.with_untracked(|d| h.with_value(|h| h.remove(d.as_ref()?, &name.get_value())));
                if let Some(cmd) = cmd {
                    ed.apply(ui, cmd, false);
                }
            };
            let cur = values.get(&f.name).cloned();
            let is_set = cur.is_some();
            let control = match kind {
                PropKind::Switch => {
                    let dflt = default_bool(&f);
                    let on = cur.as_ref().and_then(Value::as_bool).unwrap_or(dflt);
                    view! {
                        <span class="switch">
                            <input type="checkbox" prop:checked=on aria-label=f.label().to_string()
                                on:change=move |e| {
                                    let v = event_target_checked(&e);
                                    set_value(name.get_value(), (v != dflt).then_some(Value::Bool(v)));
                                }/>
                            <span class="switch-track"></span>
                        </span>
                    }.into_any()
                }
                _ => {
                    let text = cur.as_ref().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
                    let image = kind == PropKind::Image;
                    view! {
                        <Field label=String::new() value=text placeholder=default_text(&f) mono=kind == PropKind::Link
                            on_commit=move |v: String| {
                                let v = v.trim().to_string();
                                set_value(name.get_value(), (!v.is_empty()).then(|| json!(v)));
                            }>
                            {image.then(|| view! {
                                <button type="button" class="icon-btn icon-btn--small" title=ui.t("picker-choose-image") aria-label=ui.t("picker-choose-image")
                                    on:click=move |_| crate::library::pick_image(ui, crate::library::media_shelf(ed), move |url| set_value(name.get_value(), Some(json!(url))))>
                                    {icons::image()}</button>
                            })}
                        </Field>
                    }.into_any()
                }
            };
            let switch = kind == PropKind::Switch;
            // Interrupteur sur la ligne du nom ; texte, image, lien dessous.
            let (head_control, body_control) = if switch { (Some(control), None) } else { (None, Some(control)) };
            view! {
                <div class="layout-prop" class:layout-prop--switch=switch class:is-set=is_set data-prop=f.name.clone()>
                    <div class="prop-def-head">
                        <span class="prop-kind" title=ui.t(kind_key(kind))>{kind_icon(kind)}</span>
                        <input class="input prop-def-label" prop:value=f.label().to_string()
                            aria-label=ui.t("component-prop-label") title=format!("{{{{ {} }}}}", f.name)
                            on:change=rename/>
                        {head_control}
                        <button class="icon-btn icon-btn--small" title=ui.t("component-prop-remove") aria-label=ui.t("component-prop-remove")
                            on:click=remove>{icons::trash()}</button>
                    </div>
                    {body_control}
                </div>
            }
        }).collect_view();
        Some(view! {
            <div class="page-settings-section-head">
                <h3 class="page-settings-section" title=ui.t("layout-props-help")>{ui.t("layout-props")}</h3>
                <button class="icon-btn icon-btn--small" title=ui.t("component-prop-add") aria-label=ui.t("component-prop-add")
                    on:click=add>{icons::plus()}</button>
            </div>
            <div class="layout-props">{list}</div>
        })
    }
}
