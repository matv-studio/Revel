//! Import d'un site (étape 7) : dossier de l'ordinateur (pages HTML, export Webflow, projet
//! Eleventy) ou dépôt Git copié dans le dossier des projets (D-80). Le dossier est examiné avant
//! l'import : nature, nombre de pages, site à construire et ses dossiers produits, licence.

use crate::dialog::Ask;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{ClonedRepo, ImportKind, ImportProbe, ImportResult};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Folder,
    Git,
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Idle,
    /// Copie du dépôt (part reçue, inconnue au début).
    Cloning(Option<f32>),
    Importing,
}

/// `a` + `/` (ou `\`) + `b`, selon le séparateur du chemin.
fn join(a: &str, b: &str) -> String {
    let sep = if a.contains('\\') { '\\' } else { '/' };
    format!(
        "{}{sep}{}",
        a.trim_end_matches(['/', '\\']),
        b.replace('/', &sep.to_string())
    )
}

/// Nom du dépôt d'une adresse (nom du dossier créé) : `…/compte/site(.git)`,
/// `…/compte/site/tree/main/docs`, `git@github.com:compte/site.git`.
fn repo_name(url: &str) -> String {
    let u = url.trim().trim_end_matches('/');
    let u = u
        .rsplit_once(':')
        .map_or(u, |(a, b)| if a.starts_with("git@") { b } else { u });
    let segs: Vec<&str> = u.split('/').filter(|s| !s.is_empty()).collect();
    let name = match segs.iter().position(|s| *s == "tree") {
        Some(i) if i > 0 => segs[i - 1],
        _ => segs.last().copied().unwrap_or(""),
    };
    name.trim_end_matches(".git").to_string()
}

#[component]
pub fn ImportSite(close: Callback<()>, done: Callback<ImportResult>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let source = RwSignal::new(Source::Folder);
    let folder = RwSignal::new(String::new());
    let url = RwSignal::new(String::new());
    let parent = RwSignal::new(String::new());
    let target = RwSignal::new(String::new());
    // Dossier examiné et résultat de l'examen.
    let probed = RwSignal::new(String::new());
    let probe = RwSignal::new(None::<ImportProbe>);
    // Dépôt copié, pas encore importé (retiré si l'import est abandonné).
    let cloned = RwSignal::new(None::<ClonedRepo>);
    let phase = RwSignal::new(Phase::Idle);
    // Échec affiché dans la boîte (le message de l'application serait caché derrière elle).
    let error = RwSignal::new(None::<revel_protocol::Failure>);
    let url_input = NodeRef::<leptos::html::Input>::new();
    spawn_local(async move {
        if let Ok(d) = api::default_sites_dir().await {
            parent.set(d);
        }
    });
    // Dossier qui sera créé pour la copie du dépôt.
    Effect::new(move |_| {
        let (p, n) = (parent.get(), repo_name(&url.get()));
        if p.is_empty() || n.is_empty() {
            target.set(String::new());
            return;
        }
        spawn_local(async move {
            if let Ok(f) = api::site_folder(&p, &n).await {
                target.set(f);
            }
        });
    });
    Effect::new(move |_| {
        if source.get() == Source::Git
            && let Some(i) = url_input.get()
        {
            request_animation_frame(move || {
                let _ = i.focus();
            });
        }
    });
    let examine = move |path: String| {
        error.set(None);
        probe.set(None);
        probed.set(path.clone());
        spawn_local(async move {
            match api::probe_site(&path).await {
                Ok(p) => {
                    if probed.get_untracked() == path {
                        probe.set(Some(p));
                    }
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    // Abandon : copie de dépôt pas importée retirée.
    let discard = move || {
        if let Some(c) = cloned.get_untracked() {
            cloned.set(None);
            spawn_local(async move {
                let _ = api::discard_clone(&c.path).await;
            });
        }
    };
    let quit = move || {
        if let Phase::Cloning(_) = phase.get_untracked() {
            spawn_local(async {
                let _ = api::git_clone_cancel().await;
            });
            return;
        }
        if phase.get_untracked() == Phase::Importing {
            return;
        }
        discard();
        close.run(());
    };
    let run = move |path: String, overwrite: bool| {
        error.set(None);
        phase.set(Phase::Importing);
        ui.activity.set(Some(0.3));
        spawn_local(async move {
            let r = api::import_site(&path, overwrite, &ui.lang()).await;
            ui.activity.set(None);
            phase.set(Phase::Idle);
            match r {
                Ok(r) => {
                    cloned.set(None);
                    ui.reload_recents();
                    done.run(r);
                    close.run(());
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    // Import du dossier examiné ; site déjà importé : remplacement confirmé.
    let import_probed = move || {
        let path = probed.get_untracked();
        let has = probe.with_untracked(|p| p.as_ref().is_some_and(|p| p.has_site));
        if has {
            ui.ask.set(Some(
                Ask::confirm(
                    ui.now("import-overwrite-title", &[]),
                    Some(ui.now("import-overwrite", &[])),
                    ui.now("action-replace", &[]),
                    move || run(path.clone(), true),
                )
                .danger(),
            ));
        } else {
            run(path, false);
        }
    };
    let choose = move |_| {
        spawn_local(async move {
            match api::pick_folder().await {
                Ok(Some(p)) => {
                    folder.set(p.clone());
                    examine(p);
                }
                Ok(None) => {}
                Err(e) => error.set(Some(e)),
            }
        })
    };
    let change_parent = move |_| {
        spawn_local(async move {
            let start = parent.get_untracked();
            match api::pick_folder_at(Some(&start).filter(|s| !s.is_empty()).map(|s| s.as_str()))
                .await
            {
                Ok(Some(p)) => parent.set(p),
                Ok(None) => {}
                Err(e) => error.set(Some(e)),
            }
        })
    };
    // Copie du dépôt, examen, puis import direct s'il est importable.
    let clone_repo = move || {
        let (u, p) = (
            url.get_untracked().trim().to_string(),
            parent.get_untracked(),
        );
        if u.is_empty() || p.is_empty() {
            return;
        }
        discard();
        error.set(None);
        probe.set(None);
        phase.set(Phase::Cloning(None));
        spawn_local(async move {
            let r = api::git_clone(&u, &p, move |e| {
                let f = (e.total > 0).then(|| e.received as f32 / e.total as f32);
                if let Phase::Cloning(_) = phase.get_untracked() {
                    phase.set(Phase::Cloning(f));
                }
            })
            .await;
            phase.set(Phase::Idle);
            let c = match r {
                Ok(c) => c,
                Err(e) => {
                    if e.key != "error-git-clone-cancelled" {
                        error.set(Some(e));
                    }
                    return;
                }
            };
            let path = match &c.subdir {
                Some(sub) => join(&c.path, sub),
                None => c.path.clone(),
            };
            cloned.set(Some(c));
            probed.set(path.clone());
            match api::probe_site(&path).await {
                Ok(p) => {
                    let ready = matches!(p.kind, ImportKind::Static | ImportKind::Eleventy);
                    probe.set(Some(p));
                    if ready {
                        run(path, false);
                    }
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let submit = move || match source.get_untracked() {
        Source::Folder => import_probed(),
        Source::Git => {
            if cloned.with_untracked(Option::is_some)
                && probe.with_untracked(|p| {
                    p.as_ref().is_some_and(|p| {
                        matches!(p.kind, ImportKind::Static | ImportKind::Eleventy)
                    })
                })
            {
                import_probed()
            } else {
                clone_repo()
            }
        }
    };
    let can_submit = move || {
        if phase.get() != Phase::Idle {
            return false;
        }
        let ready = probe.with(|p| {
            p.as_ref()
                .is_some_and(|p| matches!(p.kind, ImportKind::Static | ImportKind::Eleventy))
        });
        match source.get() {
            Source::Folder => ready,
            Source::Git => {
                (cloned.with(Option::is_some) && ready)
                    || (cloned.with(Option::is_none)
                        && !repo_name(&url.get()).is_empty()
                        && parent.with(|p| !p.is_empty()))
            }
        }
    };
    let on_key = window_event_listener(leptos::ev::keydown, move |e| {
        if e.key() == "Escape" {
            quit();
        }
    });
    on_cleanup(move || on_key.remove());
    let switch = move |s: Source| {
        if source.get_untracked() == s || phase.get_untracked() != Phase::Idle {
            return;
        }
        discard();
        error.set(None);
        probe.set(None);
        probed.set(String::new());
        if s == Source::Folder && !folder.get_untracked().is_empty() {
            examine(folder.get_untracked());
        }
        source.set(s);
    };
    // Nature du dossier examiné : pastilles, puis dossiers produits importables.
    let status = move || {
        let p = probe.get()?;
        let license = cloned.with(|c| c.as_ref().map(|c| c.license));
        let (label, help, ok) = match p.kind {
            ImportKind::Static => (
                ui.tx("import-kind-static", &[("count", p.pages.to_string())]),
                ui.tx("import-kind-static-help", &[]),
                true,
            ),
            ImportKind::Eleventy => (ui.tx("import-kind-eleventy", &[]), String::new(), true),
            ImportKind::NeedsBuild => {
                let tool = p.tool.clone().unwrap_or_default();
                (
                    ui.tx("import-kind-build", &[("tool", tool.clone())]),
                    ui.tx("import-kind-build-help", &[("tool", tool)]),
                    false,
                )
            }
            ImportKind::Unknown => (
                ui.tx("import-kind-unknown", &[]),
                ui.tx("import-kind-unknown-help", &[]),
                false,
            ),
        };
        let base = probed.get();
        let outputs = if ok { Vec::new() } else { p.outputs.clone() };
        Some(view! {
            <div class="import-status">
                <div class="chips">
                    <span class="chip import-kind" class:chip--ok=ok class:chip--warn=!ok title=help>{label}</span>
                    {p.has_site.then(|| view! { <span class="chip">{ui.t("import-has-site")}</span> })}
                    {(license == Some(false)).then(|| view! {
                        <span class="chip chip--warn import-license" title=ui.t("import-no-license-help")>{ui.t("import-no-license")}</span>
                    })}
                </div>
                {(!outputs.is_empty()).then(|| view! {
                    <div class="import-outputs">
                        {outputs.into_iter().map(|dir| {
                            let path = join(&base, &dir);
                            view! {
                                <button type="button" class="btn btn--small import-output" title=ui.t("import-output-help")
                                    disabled=move || phase.get() != Phase::Idle
                                    on:click=move |_| run(path.clone(), false)>
                                    {icons::import()}{ui.tx("import-output", &[("dir", dir.clone())])}
                                </button>
                            }
                        }).collect_view()}
                    </div>
                })}
            </div>
        })
    };
    view! {
        <div class="overlay" on:mousedown=move |e| if e.target() == e.current_target() { quit() }>
            <form class="dialog dialog--form import-dialog" role="dialog" aria-modal="true" aria-labelledby="import-title"
                on:submit=move |e| { e.prevent_default(); if can_submit() { submit() } }>
                <h2 id="import-title">{ui.t("import-title")}</h2>
                <div class="field">
                    <span class="label">{ui.t("import-source")}</span>
                    <div class="segmented import-source" role="radiogroup" aria-label=ui.t("import-source")>
                        <button type="button" role="radio" data-source="folder" class:is-on=move || source.get() == Source::Folder
                            aria-checked=move || (source.get() == Source::Folder).to_string()
                            title=ui.t("import-source-folder-help")
                            on:click=move |_| switch(Source::Folder)>{icons::folder()}{ui.t("import-source-folder")}</button>
                        <button type="button" role="radio" data-source="git" class:is-on=move || source.get() == Source::Git
                            aria-checked=move || (source.get() == Source::Git).to_string()
                            title=ui.t("import-source-git-help")
                            on:click=move |_| switch(Source::Git)>{icons::branch()}{ui.t("import-source-git")}</button>
                    </div>
                </div>
                <Show when=move || source.get() == Source::Folder>
                    <div class="field">
                        <span class="label">{ui.t("import-folder")}</span>
                        <div class="path-field">
                            <span class="path-value import-folder" class:is-empty=move || folder.with(String::is_empty) title=move || folder.get()>
                                {move || folder.get()}
                            </span>
                            <button type="button" class="btn btn--small import-choose" on:click=choose>{ui.t("import-choose")}</button>
                        </div>
                    </div>
                </Show>
                <Show when=move || source.get() == Source::Git>
                    <label class="field">
                        <span class="label" title=ui.t("import-url-help")>{ui.t("import-url")}</span>
                        <input class="input import-url" type="text" inputmode="url" node_ref=url_input spellcheck="false" autocomplete="off"
                            placeholder=ui.t("import-url-placeholder") title=ui.t("import-url-help")
                            disabled=move || phase.get() != Phase::Idle || cloned.with(Option::is_some)
                            prop:value=move || url.get() on:input=move |e| url.set(event_target_value(&e))/>
                    </label>
                    <div class="field">
                        <span class="label">{ui.t("new-site-location")}</span>
                        <div class="path-field">
                            <span class="path-value" title=move || target.get()>
                                {move || if target.get().is_empty() { parent.get() } else { target.get() }}
                            </span>
                            <button type="button" class="btn btn--small" on:click=change_parent
                                disabled=move || phase.get() != Phase::Idle || cloned.with(Option::is_some)>{ui.t("new-site-change")}</button>
                        </div>
                    </div>
                </Show>
                {move || match phase.get() {
                    Phase::Cloning(f) => Some(view! {
                        <div class="import-progress">
                            <span class="muted">{ui.t("import-cloning")}</span>
                            <span class="meter-bar" class:is-busy=f.is_none()>
                                <span class="meter-fill" style:width=format!("{:.1}%", f.unwrap_or(1.0) * 100.0)></span>
                            </span>
                            <span class="pack-num">{f.map(|f| format!("{:.0} %", f * 100.0))}</span>
                        </div>
                    }.into_any()),
                    Phase::Importing => Some(view! {
                        <div class="import-progress">
                            <span class="muted">{ui.t("import-importing")}</span>
                            <span class="meter-bar is-busy"><span class="meter-fill" style:width="100%"></span></span>
                        </div>
                    }.into_any()),
                    Phase::Idle => None,
                }}
                {status}
                {move || error.get().map(|f| view! {
                    <div class="notice notice--error import-error" role="alert">{ui.failure(&f)}</div>
                })}
                <div class="dialog-actions">
                    <button type="button" class="btn import-cancel" on:click=move |_| quit()
                        disabled=move || phase.get() == Phase::Importing>{ui.t("action-cancel")}</button>
                    <button type="submit" class="btn btn--primary import-run" disabled=move || !can_submit()>
                        {icons::import()}{ui.t("import-run")}
                    </button>
                </div>
            </form>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nom_du_depot() {
        assert_eq!(repo_name("https://github.com/compte/site"), "site");
        assert_eq!(repo_name("https://github.com/compte/site.git/"), "site");
        assert_eq!(repo_name("github.com/compte/site/tree/main/docs"), "site");
        assert_eq!(repo_name("git@github.com:compte/site.git"), "site");
        assert_eq!(repo_name(""), "");
        assert_eq!(join("C:\\Projets\\site", "dist"), "C:\\Projets\\site\\dist");
        assert_eq!(join("/home/a/site/", "docs/web"), "/home/a/site/docs/web");
    }
}
