//! Modèles partagés (D-132) : enregistrer le site ouvert comme modèle, en fichier léger (icônes
//! et polices comprises, images en attente en ligne) ou dans un catalogue GitHub (tous les
//! fichiers) ; réglages du catalogue (dépôt, création sur le compte GitHub connecté).

use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{SiteSummary, TemplateShare};

/// Licences proposées : (identifiant SPDX, nom affiché).
const LICENSES: &[(&str, &str)] = &[
    ("CC-BY-4.0", "CC BY 4.0"),
    ("CC0-1.0", "CC0"),
    ("MIT", "MIT"),
    ("", ""),
];

/// Résultat affiché sous les actions.
#[derive(Clone)]
enum Done {
    Saved(String),
    Published { text: String, url: String },
}

/// Paramètres du site › Publication › Template : enregistrer le site comme template (fichier
/// léger ou catalogue GitHub).
#[component]
pub fn SaveTemplate(site: SiteSummary) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let name = RwSignal::new(site.name.clone());
    let category = RwSignal::new("business".to_string());
    let description = RwSignal::new(String::new());
    let author = RwSignal::new(
        ui.settings
            .with_untracked(|s| s.author.clone())
            .or_else(|| ui.github.get_untracked())
            .unwrap_or_default(),
    );
    let license = RwSignal::new("CC-BY-4.0".to_string());
    let preview = RwSignal::new(None::<String>);
    let busy = RwSignal::new(None::<&'static str>);
    let done = RwSignal::new(None::<Done>);
    let path = StoredValue::new(site.path.clone());
    let share = move || TemplateShare {
        id: String::new(),
        name: name.get_untracked().trim().to_string(),
        category: category.get_untracked(),
        description: description.get_untracked(),
        author: author.get_untracked(),
        license: license.get_untracked(),
        preview: preview.get_untracked(),
    };
    let catalog = move || ui.settings.with(|s| s.template_catalog.clone());
    let save_file = move |_| {
        let sh = share();
        let file = format!("{}.revel-template", slug(&sh.name));
        busy.set(Some("file"));
        spawn_local(async move {
            let r = async {
                let Some(dest) =
                    api::pick_save(&ui.tx("template-file-kind", &[]), "revel-template", &file)
                        .await?
                else {
                    return Ok(None);
                };
                api::template_save(&path.get_value(), &sh, &dest)
                    .await
                    .map(Some)
            }
            .await;
            busy.set(None);
            match r {
                Ok(Some(s)) => done.set(Some(Done::Saved(ui.tx(
                    "template-saved",
                    &[
                        ("size", crate::settings::size(ui, s.size)),
                        ("images", s.placeholders.to_string()),
                        ("missing", s.missing.to_string()),
                    ],
                )))),
                Ok(None) => {}
                Err(e) => ui.fail(e),
            }
        });
    };
    let publish = move |_| {
        let sh = share();
        let repo = catalog();
        busy.set(Some("catalog"));
        spawn_local(async move {
            let r = api::template_publish(&path.get_value(), &repo, &sh).await;
            busy.set(None);
            match r {
                Ok(p) => done.set(Some(Done::Published {
                    text: ui.tx(
                        if p.pull_request {
                            "template-published-pr"
                        } else {
                            "template-published"
                        },
                        &[
                            ("files", p.files.to_string()),
                            ("size", crate::settings::size(ui, p.size)),
                            ("large", p.too_large.len().to_string()),
                        ],
                    ),
                    url: p.url,
                })),
                Err(e) => ui.fail(e),
            }
        });
    };
    let pick_preview = move |_| {
        spawn_local(async move {
            match api::pick_files(
                &ui.tx("template-preview", &[]),
                &["jpg", "jpeg", "png", "webp", "gif"],
                false,
            )
            .await
            {
                Ok(f) => {
                    if let Some(p) = f.into_iter().next() {
                        preview.set(Some(p));
                    }
                }
                Err(e) => ui.fail(e),
            }
        });
    };

    let file_name = |p: &str| p.rsplit(['/', '\\']).next().unwrap_or(p).to_string();
    let invalid = move || name.with(|n| n.trim().is_empty());
    view! {
        <div class="section save-template">
                <div class="form-grid save-template__grid">
                    <label class="label" for="template-name">{ui.t("template-name")}</label>
                    <input id="template-name" class="input" type="text" spellcheck="false" autocomplete="off"
                        prop:value=move || name.get() on:input=move |e| name.set(event_target_value(&e))/>
                    <label class="label" for="template-category">{ui.t("template-category")}</label>
                    <div class="select">
                        <select id="template-category" class="input" on:change=move |e| category.set(event_target_value(&e))>
                            {crate::home::TEMPLATE_CATS.iter().map(|c| {
                                let c: &'static str = c;
                                view! { <option value=c selected=move || category.get() == c>{ui.t(crate::home::cat_key(c))}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                    <label class="label" for="template-description">{ui.t("template-description")}</label>
                    <textarea id="template-description" class="input" rows="3"
                        prop:value=move || description.get() on:input=move |e| description.set(event_target_value(&e))></textarea>
                    <label class="label" for="template-author">{ui.t("template-author")}</label>
                    <input id="template-author" class="input" type="text" autocomplete="off"
                        prop:value=move || author.get() on:input=move |e| author.set(event_target_value(&e))/>
                    <label class="label" for="template-license" title=ui.t("template-license-help")>{ui.t("template-license")}</label>
                    <div class="select">
                        <select id="template-license" class="input" on:change=move |e| license.set(event_target_value(&e))>
                            {LICENSES.iter().map(|(id, label)| {
                                let (id, label): (&'static str, &'static str) = (id, label);
                                let text = if id.is_empty() { ui.tx("template-license-none", &[]) } else { label.to_string() };
                                view! { <option value=id selected=move || license.get() == id>{text}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                    <span class="label" title=ui.t("template-preview-help")>{ui.t("template-preview")}</span>
                    <div class="path-field save-template__preview">
                        <span class="path-value" class:is-empty=move || preview.with(Option::is_none)
                            title=move || preview.get().unwrap_or_default()>
                            {move || preview.get().map(|p| file_name(&p)).unwrap_or_else(|| ui.tx("template-preview-none", &[]))}
                        </span>
                        <Show when=move || preview.with(Option::is_some)>
                            <button type="button" class="icon-btn icon-btn--small" title=ui.t("action-remove")
                                aria-label=ui.t("action-remove") on:click=move |_| preview.set(None)>{icons::close()}</button>
                        </Show>
                        <button type="button" class="btn btn--small" on:click=pick_preview>{ui.t("import-choose")}</button>
                    </div>
                </div>
                <div class="save-template__targets">
                    <div class="save-template__target" data-target="file">
                        <span class="save-template__icon">{icons::download()}</span>
                        <span class="save-template__text">
                            <span class="save-template__name">{ui.t("template-target-file")}</span>
                            <span class="muted">{ui.t("template-target-file-what")}</span>
                        </span>
                        <button type="button" class="btn save-template-file" title=ui.t("template-target-file-help")
                            disabled=move || invalid() || busy.get().is_some() on:click=save_file>
                            {move || if busy.get() == Some("file") { ui.t("template-saving") } else { ui.t("template-save-file") }}
                        </button>
                    </div>
                    <div class="save-template__target" data-target="catalog">
                        <span class="save-template__icon">{icons::cloud()}</span>
                        <span class="save-template__text">
                            <span class="save-template__name">{ui.t("template-target-catalog")}</span>
                            <span class="muted mono" title=ui.t("template-catalog-help")>{catalog}</span>
                        </span>
                        <button type="button" class="btn save-template-publish"
                            title=move || if ui.github.with(Option::is_none) { ui.t("template-needs-github") } else { ui.t("template-target-catalog-help") }
                            disabled=move || invalid() || busy.get().is_some() || ui.github.with(Option::is_none)
                            on:click=publish>
                            {move || if busy.get() == Some("catalog") { ui.t("template-sending") } else { ui.t("template-send") }}
                        </button>
                    </div>
                </div>
                {move || done.get().map(|d| match d {
                    Done::Saved(t) => view! { <p class="save-template__done" role="status">{icons::check()}{t}</p> }.into_any(),
                    Done::Published { text, url } => view! {
                        <p class="save-template__done" role="status">{icons::check()}{text}
                            <button type="button" class="btn btn--small" on:click=move |_| {
                                let u = url.clone();
                                spawn_local(async move { let _ = api::open_external(&u).await; });
                            }>{icons::external()}{ui.t("action-open")}</button>
                        </p>
                    }.into_any(),
                })}
        </div>
    }
}

/// Identifiant de fichier tiré d'un nom (comme le cœur).
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        let c = match c {
            'à' | 'â' | 'ä' | 'á' | 'À' | 'Â' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' => 'e',
            'î' | 'ï' | 'Î' => 'i',
            'ô' | 'ö' | 'Ô' => 'o',
            'ù' | 'û' | 'ü' | 'Ù' | 'Û' => 'u',
            'ç' | 'Ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let s = out.trim_end_matches('-').to_string();
    if s.is_empty() { "modele".into() } else { s }
}

/// Réglages de Revel › Modèles : catalogue GitHub lu par la galerie (dépôt `propriétaire/nom`),
/// nombre de modèles qu'il propose, création d'un catalogue sur le compte connecté.
#[component]
pub fn TemplatesCard() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let repo = RwSignal::new(ui.settings.with_untracked(|s| s.template_catalog.clone()));
    // Nombre de modèles du catalogue ; `Err` : catalogue introuvable.
    let state = RwSignal::new(None::<Result<usize, revel_protocol::Failure>>);
    let check = move || {
        let r = repo.get_untracked();
        state.set(None);
        spawn_local(async move {
            let v = api::template_catalog(&r).await.map(|l| l.len());
            if !state.is_disposed() && repo.get_untracked() == r {
                state.set(Some(v));
            }
        });
    };
    check();
    let creating = RwSignal::new(false);
    let commit = move || {
        let v = repo.get_untracked().trim().to_string();
        crate::settings::update(ui, &|s| s.template_catalog = v.clone());
        check();
    };
    let create = move |_| {
        creating.set(true);
        spawn_local(async move {
            match api::template_catalog_create("revel-templates").await {
                Ok(full) => {
                    repo.set(full.clone());
                    crate::settings::update(ui, &|s| s.template_catalog = full.clone());
                    check();
                }
                Err(e) => ui.fail(e),
            }
            creating.set(false);
        });
    };
    view! {
        <section class="section settings-card" data-section="templates">
            <h2>{ui.t("settings-templates")}</h2>
            <div class="form-grid">
                <label class="label" for="template-catalog" title=ui.t("template-catalog-help")>{ui.t("template-catalog")}</label>
                <div class="catalog-field">
                    <input id="template-catalog" class="input mono" type="text" spellcheck="false" autocomplete="off"
                        placeholder="propriétaire/nom"
                        prop:value=move || repo.get() on:input=move |e| repo.set(event_target_value(&e))
                        on:change=move |_| commit()
                        on:keydown=move |e| if e.key() == "Enter" { commit() }/>
                    <span class="catalog-state" role="status">
                        {move || match state.get() {
                            None => view! { <span class="muted">"…"</span> }.into_any(),
                            Some(Ok(n)) => view! {
                                <span class="catalog-ok">{icons::check()}{ui.tx("template-catalog-count", &[("n", n.to_string())])}</span>
                            }.into_any(),
                            Some(Err(_)) => view! {
                                <span class="catalog-missing" title=ui.t("template-catalog-missing-help")>{icons::warning()}{ui.t("template-catalog-missing")}</span>
                            }.into_any(),
                        }}
                    </span>
                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("template-catalog-open")
                        aria-label=ui.t("template-catalog-open")
                        on:click=move |_| {
                            let u = format!("https://github.com/{}", repo.get_untracked().trim());
                            spawn_local(async move { let _ = api::open_external(&u).await; });
                        }>{icons::external()}</button>
                </div>
                <span class="label" title=ui.t("template-catalog-create-help")>{ui.t("template-catalog-own")}</span>
                <div>
                    <button type="button" class="btn btn--small catalog-create"
                        title=move || if ui.github.with(Option::is_none) { ui.t("template-needs-github") } else { ui.t("template-catalog-create-help") }
                        disabled=move || creating.get() || ui.github.with(Option::is_none)
                        on:click=create>
                        {icons::plus()}{move || if creating.get() { ui.t("template-catalog-creating") } else { ui.t("template-catalog-create") }}
                    </button>
                </div>
            </div>
        </section>
    }
}
