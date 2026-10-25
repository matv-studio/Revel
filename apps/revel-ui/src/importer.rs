//! Import d'un site (étape 7), une boîte pour toutes les sources :
//!
//! - **Dossier** de l'ordinateur (pages HTML, export Webflow décompressé et contenus de son CMS,
//!   projet Eleventy), examiné avant l'import (nature, nombre de pages, outil d'origine) ;
//! - **Lien** : site publié lu à son adresse (Framer, Webflow, tout site en ligne) ;
//! - **Dépôt Git** copié dans le dossier des projets (D-80) ;
//! - **Figma** : fichier lu par l'API (jeton gardé dans le trousseau), cadres choisis ;
//! - **WordPress** : fichier d'export, examiné avant l'import.
//!
//! L'import fini, la boîte montre le bilan (ce qui a été repris, points à vérifier) et ouvre le
//! site dans l'éditeur.

use crate::dialog::Ask;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{
    BuildPlan, BuildProgress, ClonedRepo, Failure, FigmaFile, ImportEvent, ImportKind,
    ImportOptions, ImportProbe, ImportResult, WxrProbe,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Folder,
    Link,
    Git,
    Figma,
    WordPress,
}

impl Source {
    fn id(self) -> &'static str {
        match self {
            Source::Folder => "folder",
            Source::Link => "link",
            Source::Git => "git",
            Source::Figma => "figma",
            Source::WordPress => "wordpress",
        }
    }
}

/// Source proposée : libellé, aide, icône.
type Tile = (Source, &'static str, &'static str, fn() -> AnyView);

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Idle,
    /// Copie du dépôt (part reçue, inconnue au début).
    Cloning(Option<f32>),
    /// Lecture d'un fichier Figma.
    Reading,
    /// Import ; avancement des imports lus en ligne (pages, fichiers).
    Importing(Option<ImportEvent>),
    /// Construction du site par Revel (D-88) ; avancement dans `build_prog`.
    Building,
}

/// Contenus du CMS de Webflow : fichiers CSV ou API.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cms {
    Csv,
    Api,
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
    let u = url.trim().trim_end_matches(['/', '\\']);
    let u = u
        .rsplit_once(':')
        .map_or(u, |(a, b)| if a.starts_with("git@") { b } else { u });
    // Dossier partagé (`\\nas\equipe\site.git`, `Z:\site.git`) : séparateurs des deux sortes.
    let segs: Vec<&str> = u.split(['/', '\\']).filter(|s| !s.is_empty()).collect();
    let name = match segs.iter().position(|s| *s == "tree") {
        Some(i) if i > 0 => segs[i - 1],
        _ => segs.last().copied().unwrap_or(""),
    };
    name.trim_end_matches(".git").to_string()
}

/// Nom du dossier d'un site lu en ligne : son domaine, sans `www.`.
fn host_name(url: &str) -> String {
    let u = url.trim();
    let u = u.split_once("://").map_or(u, |(_, r)| r);
    let host = u.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    if !host.contains('.') || host.contains(' ') {
        return String::new();
    }
    host.trim_start_matches("www.").to_ascii_lowercase()
}

/// Nom d'un fichier dans un chemin.
fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

/// Taille lisible : `1,2 Mo`, `340 Ko`.
fn size(bytes: u64, fr: bool) -> String {
    let (v, unit) = if bytes >= 1024 * 1024 {
        (bytes as f64 / 1024.0 / 1024.0, if fr { "Mo" } else { "MB" })
    } else {
        (bytes as f64 / 1024.0, if fr { "Ko" } else { "KB" })
    };
    let s = if v >= 10.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    };
    format!("{} {unit}", if fr { s.replace('.', ",") } else { s })
}

#[component]
pub fn ImportSite(
    close: Callback<()>,
    done: Callback<ImportResult>,
    /// Dossier à examiner d'emblée (dossier déposé sur l'accueil, D-122).
    #[prop(optional)]
    start: Option<String>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let source = RwSignal::new(Source::Folder);
    let parent = RwSignal::new(String::new());
    let target = RwSignal::new(String::new());
    let phase = RwSignal::new(Phase::Idle);
    // Échec affiché dans la boîte (le message de l'application serait caché derrière elle).
    let error = RwSignal::new(None::<revel_protocol::Failure>);
    let result = RwSignal::new(None::<ImportResult>);
    // Dossier (et dépôt copié) : chemin examiné et résultat de l'examen.
    let folder = RwSignal::new(String::new());
    let probed = RwSignal::new(String::new());
    let probe = RwSignal::new(None::<ImportProbe>);
    let cms = RwSignal::new(Cms::Csv);
    let csv = RwSignal::new(Vec::<String>::new());
    let wf_token = RwSignal::new(String::new());
    // Lien.
    let link = RwSignal::new(String::new());
    // Dépôt Git copié, pas encore importé (retiré si l'import est abandonné).
    let url = RwSignal::new(String::new());
    let cloned = RwSignal::new(None::<ClonedRepo>);
    // Figma.
    let fig_url = RwSignal::new(String::new());
    let fig_token = RwSignal::new(String::new());
    let fig_saved = RwSignal::new(false);
    let fig_file = RwSignal::new(None::<FigmaFile>);
    let fig_sel = RwSignal::new(Vec::<String>::new());
    // WordPress.
    let wxr = RwSignal::new(String::new());
    let wxr_probe = RwSignal::new(None::<WxrProbe>);
    let wp_media = RwSignal::new(true);
    // Site à construire (D-88) : plan de construction, sites publiés trouvés, confirmation,
    // avancement et journal.
    let plan = RwSignal::new(None::<Result<BuildPlan, Failure>>);
    let published = RwSignal::new(Vec::<String>::new());
    let build_open = RwSignal::new(false);
    let build_prog = RwSignal::new(None::<BuildProgress>);
    let build_log = RwSignal::new(Vec::<String>::new());
    let first_input = NodeRef::<leptos::html::Input>::new();
    spawn_local(async move {
        if let Ok(d) = api::default_sites_dir().await {
            parent.set(d);
        }
        if let Ok(t) = api::figma_has_token().await {
            fig_saved.set(t);
        }
    });
    let busy = move || phase.get() != Phase::Idle;
    // Dossier qui sera créé (lien, dépôt, Figma, WordPress).
    Effect::new(move |_| {
        let p = parent.get();
        let name = match source.get() {
            Source::Git => repo_name(&url.get()),
            Source::Link => host_name(&link.get()),
            Source::Figma => {
                fig_file.with(|f| f.as_ref().map(|f| f.name.clone()).unwrap_or_default())
            }
            Source::WordPress => {
                wxr_probe.with(|w| w.as_ref().map(|w| w.title.clone()).unwrap_or_default())
            }
            Source::Folder => String::new(),
        };
        if p.is_empty() || name.trim().is_empty() {
            target.set(String::new());
            return;
        }
        spawn_local(async move {
            if let Ok(f) = api::site_folder(&p, &name).await {
                target.set(f);
            }
        });
    });
    // Champ d'adresse prêt à la saisie quand la source en a un.
    Effect::new(move |_| {
        let s = source.get();
        if matches!(s, Source::Git | Source::Link | Source::Figma)
            && let Some(i) = first_input.get()
        {
            request_animation_frame(move || {
                let _ = i.focus();
            });
        }
    });
    // Après l'examen d'un site à construire : plan de construction et adresse du site publié.
    let after_probe = move |path: String, p: &ImportProbe| {
        plan.set(None);
        published.set(Vec::new());
        build_open.set(false);
        if p.kind == ImportKind::NeedsBuild {
            let path = path.clone();
            spawn_local(async move {
                let r = api::project_build_plan(&path).await;
                if probed.get_untracked() == path {
                    plan.set(Some(r));
                }
            });
        }
        if matches!(p.kind, ImportKind::NeedsBuild | ImportKind::Unknown) {
            let repo = cloned
                .with_untracked(|c| c.is_some())
                .then(|| url.get_untracked().trim().to_string());
            spawn_local(async move {
                if let Ok(list) = api::published_sites(&path, repo.as_deref()).await
                    && probed.get_untracked() == path
                {
                    published.set(list);
                }
            });
        }
    };
    let examine = move |path: String| {
        error.set(None);
        probe.set(None);
        probed.set(path.clone());
        spawn_local(async move {
            match api::probe_site(&path).await {
                Ok(p) => {
                    if probed.get_untracked() == path {
                        csv.set(p.csv.clone());
                        after_probe(path, &p);
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
        match phase.get_untracked() {
            Phase::Cloning(_) => {
                spawn_local(async {
                    let _ = api::git_clone_cancel().await;
                });
                return;
            }
            Phase::Importing(_) | Phase::Reading | Phase::Building => {
                spawn_local(async {
                    let _ = api::import_cancel().await;
                });
                return;
            }
            Phase::Idle => {}
        }
        discard();
        close.run(());
    };
    // Fin d'un import : bilan affiché dans la boîte.
    let finish = move |r: Result<ImportResult, revel_protocol::Failure>| {
        ui.activity.set(None);
        phase.set(Phase::Idle);
        match r {
            Ok(r) => {
                cloned.set(None);
                ui.reload_recents();
                done.run(r.clone());
                result.set(Some(r));
            }
            Err(e) => {
                if e.key != "error-import-cancelled" {
                    error.set(Some(e));
                }
            }
        }
    };
    let progress = move |e: ImportEvent| {
        if let Phase::Importing(_) = phase.get_untracked() {
            phase.set(Phase::Importing(Some(e)));
        }
    };
    let run = move |path: String, overwrite: bool| {
        error.set(None);
        phase.set(Phase::Importing(None));
        ui.activity.set(Some(0.3));
        let webflow = probe.with_untracked(|p| {
            p.as_ref()
                .is_some_and(|p| p.made_with.as_deref() == Some("Webflow"))
        });
        let options = ImportOptions {
            overwrite,
            lang: ui.lang(),
            csv: if webflow && cms.get_untracked() == Cms::Csv {
                csv.get_untracked()
            } else {
                Vec::new()
            },
            webflow_token: (webflow && cms.get_untracked() == Cms::Api)
                .then(|| wf_token.get_untracked().trim().to_string())
                .filter(|t| !t.is_empty()),
        };
        spawn_local(async move {
            finish(api::import_site(&path, &options, progress).await);
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
    if let Some(p) = start.filter(|p| !p.is_empty()) {
        folder.set(p.clone());
        examine(p);
    }
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
    let add_csv = move |_| {
        spawn_local(async move {
            match api::pick_files("CSV", &["csv"], true).await {
                Ok(files) => csv.update(|l| {
                    for f in files {
                        if !l.contains(&f) {
                            l.push(f);
                        }
                    }
                }),
                Err(e) => error.set(Some(e)),
            }
        })
    };
    let choose_wxr = move |_| {
        spawn_local(async move {
            match api::pick_files("WordPress (WXR)", &["xml"], false).await {
                Ok(files) => {
                    let Some(f) = files.into_iter().next() else {
                        return;
                    };
                    wxr.set(f.clone());
                    wxr_probe.set(None);
                    error.set(None);
                    match api::probe_wordpress(&f).await {
                        Ok(p) => wxr_probe.set(Some(p)),
                        Err(e) => error.set(Some(e)),
                    }
                }
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
                    after_probe(path.clone(), &p);
                    probe.set(Some(p));
                    if ready {
                        run(path, false);
                    }
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let import_link = move || {
        let (u, p) = (
            link.get_untracked().trim().to_string(),
            parent.get_untracked(),
        );
        error.set(None);
        phase.set(Phase::Importing(None));
        ui.activity.set(Some(0.3));
        spawn_local(async move {
            finish(api::import_url(&u, &p, &ui.lang(), progress).await);
        });
    };
    // Construction par Revel, une fois acceptée (D-88).
    let start_build = move || {
        let (path, p) = (probed.get_untracked(), parent.get_untracked());
        error.set(None);
        build_open.set(false);
        build_log.set(Vec::new());
        build_prog.set(None);
        phase.set(Phase::Building);
        ui.activity.set(Some(0.3));
        spawn_local(async move {
            let r = api::project_build_import(&path, &p, &ui.lang(), move |e: BuildProgress| {
                if let Some(l) = e.line.clone() {
                    build_log.update(|v| {
                        v.push(l);
                        if v.len() > 300 {
                            v.remove(0);
                        }
                    });
                }
                build_prog.update(|cur| match cur {
                    // Ligne du journal : étape et avancement gardés.
                    Some(c) if e.line.is_some() => c.line = e.line.clone(),
                    Some(c) if e.step == c.step && e.detail.is_empty() => {
                        c.done = e.done;
                        c.total = e.total;
                    }
                    _ => *cur = Some(e),
                });
            })
            .await;
            finish(r);
        });
    };
    // Site publié trouvé : importé depuis son adresse (copie du dépôt retirée).
    let import_published = move |u: String| {
        discard();
        probe.set(None);
        probed.set(String::new());
        link.set(u);
        source.set(Source::Link);
        import_link();
    };
    let read_figma = move || {
        let u = fig_url.get_untracked().trim().to_string();
        let t = fig_token.get_untracked().trim().to_string();
        error.set(None);
        phase.set(Phase::Reading);
        spawn_local(async move {
            let r = api::figma_read(&u, Some(t.as_str()).filter(|t| !t.is_empty())).await;
            phase.set(Phase::Idle);
            match r {
                Ok(f) => {
                    // Cadres de la première page choisis.
                    fig_sel.set(
                        f.canvases
                            .first()
                            .map(|c| c.frames.iter().map(|f| f.id.clone()).collect())
                            .unwrap_or_default(),
                    );
                    fig_file.set(Some(f));
                    fig_token.set(String::new());
                    fig_saved.set(true);
                }
                Err(e) => {
                    if e.key == "error-figma-token" {
                        fig_saved.set(false);
                    }
                    error.set(Some(e));
                }
            }
        });
    };
    let import_figma = move || {
        let u = fig_url.get_untracked().trim().to_string();
        // Ordre du fichier (le premier cadre choisi devient l'accueil).
        let chosen = fig_sel.get_untracked();
        let frames: Vec<String> = fig_file.with_untracked(|f| {
            f.as_ref()
                .map(|f| {
                    f.canvases
                        .iter()
                        .flat_map(|c| c.frames.iter())
                        .filter(|fr| chosen.contains(&fr.id))
                        .map(|fr| fr.id.clone())
                        .collect()
                })
                .unwrap_or_default()
        });
        let p = parent.get_untracked();
        error.set(None);
        phase.set(Phase::Importing(None));
        ui.activity.set(Some(0.3));
        spawn_local(async move {
            finish(api::import_figma(&u, &frames, &p, &ui.lang(), progress).await);
        });
    };
    let import_wp = move || {
        let (f, p) = (wxr.get_untracked(), parent.get_untracked());
        let media = wp_media.get_untracked();
        error.set(None);
        phase.set(Phase::Importing(None));
        ui.activity.set(Some(0.3));
        spawn_local(async move {
            finish(api::import_wordpress(&f, &p, &ui.lang(), media, progress).await);
        });
    };
    let probe_ready = move || {
        probe.with(|p| {
            p.as_ref()
                .is_some_and(|p| matches!(p.kind, ImportKind::Static | ImportKind::Eleventy))
        })
    };
    let submit = move || match source.get_untracked() {
        Source::Folder => import_probed(),
        Source::Link => import_link(),
        Source::Git => {
            if cloned.with_untracked(Option::is_some) && probe_ready() {
                import_probed()
            } else {
                clone_repo()
            }
        }
        Source::Figma => {
            if fig_file.with_untracked(Option::is_some) {
                import_figma()
            } else {
                read_figma()
            }
        }
        Source::WordPress => import_wp(),
    };
    let can_submit = move || {
        if busy() || result.with(Option::is_some) {
            return false;
        }
        let has_parent = parent.with(|p| !p.is_empty());
        match source.get() {
            Source::Folder => {
                probe_ready()
                    && !(cms.get() == Cms::Api
                        && probe.with(|p| {
                            p.as_ref()
                                .is_some_and(|p| p.made_with.as_deref() == Some("Webflow"))
                        })
                        && wf_token.with(|t| t.trim().is_empty()))
            }
            Source::Link => has_parent && !host_name(&link.get()).is_empty(),
            Source::Git => {
                (cloned.with(Option::is_some) && probe_ready())
                    || (cloned.with(Option::is_none)
                        && !repo_name(&url.get()).is_empty()
                        && has_parent)
            }
            Source::Figma => {
                if fig_file.with(Option::is_some) {
                    has_parent && fig_sel.with(|s| !s.is_empty())
                } else {
                    fig_url.with(|u| !u.trim().is_empty())
                        && (fig_saved.get() || fig_token.with(|t| !t.trim().is_empty()))
                }
            }
            Source::WordPress => has_parent && wxr_probe.with(Option::is_some),
        }
    };
    let on_key = window_event_listener(leptos::ev::keydown, move |e| {
        if e.key() == "Escape" {
            quit();
        }
    });
    on_cleanup(move || on_key.remove());
    let switch = move |s: Source| {
        if source.get_untracked() == s || busy() {
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
    // Accord avant la construction : outils téléchargés, dépendances, code exécuté (D-88).
    let build_panel = move || {
        if !build_open.get() || busy() {
            return None;
        }
        let p = plan.get()?.ok()?;
        Some(view! {
            <div class="import-build-panel">
                <div class="import-build-head">
                    <span class="label">{ui.tx("import-build-title", &[("tool", p.tool.clone())])}</span>
                    <span class="chip chip--warn import-build-risk" title=ui.t("import-build-risk-help")>
                        {icons::warning()}{ui.t("import-build-risk")}
                    </span>
                </div>
                <ul class="import-build-list">
                    {p.tools.iter().map(|t| {
                        let state = if t.system {
                            ui.tx("import-build-system", &[])
                        } else if t.installed {
                            ui.tx("import-build-ready", &[])
                        } else {
                            t.size
                                .map(|n| crate::settings::size(ui, n))
                                .unwrap_or_else(|| ui.tx("import-build-download", &[]))
                        };
                        view! {
                            <li class="import-build-item" data-tool=t.name.clone()>
                                <span>{format!("{} {}", t.name, t.version).trim().to_string()}</span>
                                <span class="muted">{state}</span>
                            </li>
                        }
                    }).collect_view()}
                    {p.manager.clone().map(|m| view! {
                        <li class="import-build-item" data-tool="deps">
                            <span>{ui.tx("import-build-deps", &[("manager", m)])}</span>
                            <span class="muted">{ui.t("import-build-download")}</span>
                        </li>
                    })}
                    {p.lock_ignored.clone().map(|l| view! {
                        <li class="import-build-item import-build-note" title=ui.t("import-build-lock-help")>
                            <span>{ui.tx("import-build-lock", &[("file", l)])}</span>
                            {icons::info()}
                        </li>
                    })}
                </ul>
                <div class="import-build-actions">
                    <button type="button" class="btn btn--small import-build-cancel" on:click=move |_| build_open.set(false)>{ui.t("action-cancel")}</button>
                    <button type="button" class="btn btn--small btn--primary import-build-run" on:click=move |_| start_build()>
                        {icons::bolt()}{ui.t("import-build-run")}
                    </button>
                </div>
            </div>
        })
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
        let made_with = p.made_with.clone();
        let needs_build = p.kind == ImportKind::NeedsBuild;
        Some(view! {
            <div class="import-status">
                <div class="chips">
                    <span class="chip import-kind" class:chip--ok=ok class:chip--warn=!ok title=help>{label}</span>
                    {made_with.map(|t| view! { <span class="chip import-made-with">{ui.tx("import-made-with", &[("tool", t)])}</span> })}
                    {p.has_site.then(|| view! { <span class="chip">{ui.t("import-has-site")}</span> })}
                    {(license == Some(false)).then(|| view! {
                        <span class="chip chip--warn import-license" title=ui.t("import-no-license-help")>{ui.t("import-no-license")}</span>
                    })}
                </div>
                {(!ok).then(|| view! {
                    <div class="import-outputs import-actions">
                        // Construction par Revel : possible quand le plan est lu et que rien ne manque.
                        {needs_build.then(|| view! {
                            {move || {
                                let (enabled, title) = match plan.get() {
                                    None => (false, ui.tx("import-build-checking", &[])),
                                    Some(Err(e)) => (false, ui.failure(&e)),
                                    Some(Ok(p)) => match p.missing.as_deref() {
                                        Some(m) => (false, ui.tx(&format!("import-build-missing-{m}"), &[])),
                                        None => (true, ui.tx("import-build-help", &[])),
                                    },
                                };
                                view! {
                                    <button type="button" class="btn btn--small import-build" title=title
                                        class:is-on=move || build_open.get()
                                        disabled=move || busy() || !enabled
                                        on:click=move |_| build_open.update(|o| *o = !*o)>
                                        {icons::bolt()}{ui.t("import-build")}
                                    </button>
                                }
                            }}
                        })}
                        // Site publié trouvé (README, configuration, page du dépôt).
                        {move || published.get().into_iter().map(|u| {
                            let host = host_name(&u);
                            let go = u.clone();
                            view! {
                                <button type="button" class="btn btn--small import-published" title=format!("{} · {}", ui.tx("import-published-help", &[]), u)
                                    disabled=busy on:click=move |_| import_published(go.clone())>
                                    {icons::globe()}{ui.tx("import-published", &[("host", host)])}
                                </button>
                            }
                        }).collect_view()}
                        {outputs.into_iter().map(|dir| {
                            let path = join(&base, &dir);
                            view! {
                                <button type="button" class="btn btn--small import-output" title=ui.t("import-output-help")
                                    disabled=busy
                                    on:click=move |_| run(path.clone(), false)>
                                    {icons::import()}{ui.tx("import-output", &[("dir", dir.clone())])}
                                </button>
                            }
                        }).collect_view()}
                    </div>
                })}
                {build_panel}
            </div>
        })
    };
    // Contenus du CMS d'un export Webflow : fichiers CSV ou API.
    let cms_field = move || {
        let webflow = probe.with(|p| {
            p.as_ref()
                .is_some_and(|p| p.made_with.as_deref() == Some("Webflow"))
        });
        (webflow && source.get() == Source::Folder).then(|| view! {
            <div class="field import-cms">
                <div class="import-cms-head">
                    <span class="label">{ui.t("import-cms")}</span>
                    <div class="segmented segmented--small import-cms-mode" role="radiogroup" aria-label=ui.t("import-cms")>
                        <button type="button" role="radio" data-mode="csv" class:is-on=move || cms.get() == Cms::Csv
                            aria-checked=move || (cms.get() == Cms::Csv).to_string() title=ui.t("import-cms-csv-help")
                            on:click=move |_| cms.set(Cms::Csv)>{ui.t("import-cms-csv")}</button>
                        <button type="button" role="radio" data-mode="api" class:is-on=move || cms.get() == Cms::Api
                            aria-checked=move || (cms.get() == Cms::Api).to_string() title=ui.t("import-cms-api-help")
                            on:click=move |_| cms.set(Cms::Api)>{ui.t("import-cms-api")}</button>
                    </div>
                </div>
                <Show when=move || cms.get() == Cms::Csv>
                    <div class="import-files">
                        {move || {
                            let list = csv.get();
                            if list.is_empty() {
                                view! { <span class="muted import-files-none">{ui.t("import-cms-none")}</span> }.into_any()
                            } else {
                                list.into_iter().map(|f| {
                                    let name = file_name(&f);
                                    let full = f.clone();
                                    view! {
                                        <span class="chip import-file" title=f.clone()>
                                            {icons::file()}<span class="import-file-name">{name}</span>
                                            <button type="button" class="chip-remove" aria-label=ui.t("action-remove")
                                                on:click=move |_| csv.update(|l| l.retain(|x| *x != full))>{icons::close()}</button>
                                        </span>
                                    }
                                }).collect_view().into_any()
                            }
                        }}
                        <button type="button" class="btn btn--small import-cms-add" on:click=add_csv disabled=busy>{icons::plus()}{ui.t("import-cms-add")}</button>
                    </div>
                </Show>
                <Show when=move || cms.get() == Cms::Api>
                    <input class="input import-wf-token" type="password" autocomplete="off" spellcheck="false"
                        placeholder=ui.t("import-cms-token") aria-label=ui.t("import-cms-token") title=ui.t("import-cms-api-help")
                        prop:value=move || wf_token.get() on:input=move |e| wf_token.set(event_target_value(&e))/>
                </Show>
            </div>
        })
    };
    // Emplacement du dossier créé.
    let location = move || {
        view! {
            <div class="field">
                <span class="label">{ui.t("new-site-location")}</span>
                <div class="path-field">
                    <span class="path-value import-target" title=move || target.get()>
                        {move || crate::ltr(&if target.get().is_empty() { parent.get() } else { target.get() })}
                    </span>
                    <button type="button" class="btn btn--small" on:click=change_parent
                        disabled=move || busy() || cloned.with(Option::is_some)>{ui.t("new-site-change")}</button>
                </div>
            </div>
        }
    };
    // Cadres du fichier Figma lu, par page du fichier.
    let frames = move || {
        let f = fig_file.get()?;
        let all: Vec<String> = f
            .canvases
            .iter()
            .flat_map(|c| c.frames.iter().map(|x| x.id.clone()))
            .collect();
        let all2 = all.clone();
        Some(view! {
            <div class="field import-figma-frames">
                <div class="import-cms-head">
                    <span class="label" title=ui.t("import-figma-frames-help")>{ui.t("import-figma-frames")}</span>
                    <span class="import-figma-count muted">{move || format!("{} / {}", fig_sel.with(Vec::len), all.len())}</span>
                    <button type="button" class="btn btn--small import-figma-all" on:click=move |_| fig_sel.set(all2.clone())>{ui.t("import-figma-all")}</button>
                    <button type="button" class="btn btn--small import-figma-none" on:click=move |_| fig_sel.set(Vec::new())>{ui.t("import-figma-none")}</button>
                </div>
                <div class="import-frames">
                    {f.canvases.into_iter().filter(|c| !c.frames.is_empty()).map(|c| view! {
                        <div class="import-canvas">
                            <span class="import-canvas-name">{c.name}</span>
                            {c.frames.into_iter().map(|fr| {
                                let id = fr.id.clone();
                                let id2 = fr.id.clone();
                                view! {
                                    <label class="import-frame">
                                        <input type="checkbox" prop:checked=move || fig_sel.with(|s| s.contains(&id))
                                            on:change=move |e| {
                                                let on = event_target_checked(&e);
                                                fig_sel.update(|s| {
                                                    s.retain(|x| *x != id2);
                                                    if on { s.push(id2.clone()); }
                                                });
                                            }/>
                                        <span class="import-frame-name">{fr.name}</span>
                                        <span class="import-frame-size">{format!("{} × {}", fr.width, fr.height)}</span>
                                    </label>
                                }
                            }).collect_view()}
                        </div>
                    }).collect_view()}
                </div>
            </div>
        })
    };
    // Bilan d'un import réussi.
    let summary = move || {
        let r = result.get()?;
        let s = r.stats.clone();
        let site = r.site.clone();
        let stats: Vec<(&'static str, usize)> = [
            ("import-stat-pages", s.pages),
            ("import-stat-layouts", s.layouts),
            ("import-stat-styles", s.style_rules),
            ("import-stat-animations", s.animations),
            ("import-stat-forms", s.forms),
            ("import-stat-entries", s.entries),
            ("import-stat-files", s.files),
        ]
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .collect();
        let n = r.warnings.len();
        Some(view! {
            <div class="import-summary">
                <div class="import-summary-head">
                    <span class="import-summary-mark">{icons::check()}</span>
                    <div class="import-summary-title">
                        <h2 id="import-title">{ui.t("import-done-title")}</h2>
                        <span class="import-summary-name" title=site.path.clone()>{site.name.clone()}</span>
                    </div>
                    {s.tool.clone().map(|t| view! { <span class="chip import-made-with">{ui.tx("import-made-with", &[("tool", t)])}</span> })}
                </div>
                <div class="import-stats">
                    {stats.into_iter().map(|(k, v)| view! {
                        <div class="import-stat" data-stat=k>
                            <span class="import-stat-value">{v.to_string()}</span>
                            <span class="import-stat-label">{ui.t(k)}</span>
                        </div>
                    }).collect_view()}
                </div>
                {(n > 0).then(|| view! {
                    <details class="check-group import-warnings">
                        <summary>
                            <span class="check-chevron">{icons::chevron_right()}</span>
                            {icons::warning()}
                            <span>{ui.t("import-warnings")}</span>
                            <span class="chip chip--warn">{n.to_string()}</span>
                        </summary>
                        <ul class="check-list">
                            {r.warnings.iter().map(|w| view! { <li>{w.clone()}</li> }).collect_view()}
                        </ul>
                    </details>
                })}
                <div class="dialog-actions">
                    <button type="button" class="btn import-close" on:click=move |_| close.run(())>{ui.t("action-close")}</button>
                    <button type="button" class="btn btn--primary import-open" on:click=move |_| {
                        close.run(());
                        ui.open(site.clone());
                    }>{icons::edit()}{ui.t("import-open")}</button>
                </div>
            </div>
        })
    };
    let fr = move || ui.lang().starts_with("fr");
    let tiles: [Tile; 5] = [
        (
            Source::Folder,
            "import-source-folder",
            "import-source-folder-help",
            icons::folder,
        ),
        (
            Source::Link,
            "import-source-link",
            "import-source-link-help",
            icons::globe,
        ),
        (
            Source::Git,
            "import-source-git",
            "import-source-git-help",
            icons::branch,
        ),
        (
            Source::Figma,
            "import-source-figma",
            "import-source-figma-help",
            icons::layout,
        ),
        (
            Source::WordPress,
            "import-source-wordpress",
            "import-source-wordpress-help",
            icons::file,
        ),
    ];
    view! {
        <div class="overlay" on:mousedown=move |e| if e.target() == e.current_target() && !busy() { quit() }>
            <form class="dialog dialog--form import-dialog" role="dialog" aria-modal="true" aria-labelledby="import-title"
                on:submit=move |e| { e.prevent_default(); if can_submit() { submit() } }>
                <Show when=move || result.with(Option::is_some) fallback=move || view! {
                    <h2 id="import-title">{ui.t("import-title")}</h2>
                    <div class="import-source import-sources" role="radiogroup" aria-label=ui.t("import-source")>
                        {tiles.into_iter().map(|(s, key, help, icon)| view! {
                            <button type="button" class="template-choice import-tile" role="radio" data-source=s.id()
                                class:is-on=move || source.get() == s
                                aria-checked=move || (source.get() == s).to_string()
                                title=ui.t(help) disabled=move || busy() && source.get() != s
                                on:click=move |_| switch(s)>{icon()}<span>{ui.t(key)}</span></button>
                        }).collect_view()}
                    </div>
                    {move || match source.get() {
                        Source::Folder => view! {
                            <div class="field">
                                <span class="label">{ui.t("import-folder")}</span>
                                <div class="path-field">
                                    <span class="path-value import-folder" class:is-empty=move || folder.with(String::is_empty) title=move || folder.get()>
                                        {move || crate::ltr(&folder.get())}
                                    </span>
                                    <button type="button" class="btn btn--small import-choose" on:click=choose disabled=busy>{ui.t("import-choose")}</button>
                                </div>
                            </div>
                        }.into_any(),
                        Source::Link => view! {
                            <label class="field">
                                <span class="label" title=ui.t("import-link-help")>{ui.t("import-link")}</span>
                                <input class="input import-link" type="text" inputmode="url" node_ref=first_input spellcheck="false" autocomplete="off"
                                    placeholder=ui.t("import-link-placeholder") title=ui.t("import-link-help")
                                    disabled=busy prop:value=move || link.get() on:input=move |e| link.set(event_target_value(&e))/>
                            </label>
                            {location}
                        }.into_any(),
                        Source::Git => view! {
                            <div class="field">
                                <label class="label" for="import-url" title=ui.t("import-url-help")>{ui.t("import-url")}</label>
                                <span class="import-url-row">
                                    <input id="import-url" class="input import-url" type="text" inputmode="url" node_ref=first_input spellcheck="false" autocomplete="off"
                                        placeholder=ui.t("import-url-placeholder") title=ui.t("import-url-help")
                                        disabled=move || busy() || cloned.with(Option::is_some)
                                        prop:value=move || url.get() on:input=move |e| url.set(event_target_value(&e))/>
                                    // Dépôt de l'équipe dans un dossier partagé (NAS, réseau local, D-89).
                                    <button type="button" class="btn btn--small import-git-folder" title=ui.t("import-git-folder-help")
                                        disabled=move || busy() || cloned.with(Option::is_some)
                                        on:click=move |_| spawn_local(async move {
                                            match api::pick_folder().await {
                                                Ok(Some(d)) => { let _ = url.try_set(d); }
                                                Ok(None) => {}
                                                Err(e) => ui.fail(e),
                                            }
                                        })>{ui.t("import-git-folder")}</button>
                                </span>
                            </div>
                            {location}
                        }.into_any(),
                        Source::Figma => view! {
                            <label class="field">
                                <span class="label" title=ui.t("import-figma-link-help")>{ui.t("import-figma-link")}</span>
                                <input class="input import-figma-url" type="text" inputmode="url" node_ref=first_input spellcheck="false" autocomplete="off"
                                    placeholder=ui.t("import-figma-link-placeholder") title=ui.t("import-figma-link-help")
                                    disabled=move || busy() || fig_file.with(Option::is_some)
                                    prop:value=move || fig_url.get() on:input=move |e| fig_url.set(event_target_value(&e))/>
                            </label>
                            {move || if fig_saved.get() {
                                view! {
                                    <div class="field">
                                        <span class="label">{ui.t("import-figma-token")}</span>
                                        <div class="chips import-figma-saved">
                                            <span class="chip chip--ok">{icons::check()}{ui.t("import-figma-token-saved")}</span>
                                            <button type="button" class="btn btn--small import-figma-forget" title=ui.t("import-figma-forget-help")
                                                disabled=busy
                                                on:click=move |_| spawn_local(async move {
                                                    match api::figma_forget_token().await {
                                                        Ok(()) => { fig_saved.set(false); fig_file.set(None); }
                                                        Err(e) => error.set(Some(e)),
                                                    }
                                                })>{ui.t("import-figma-forget")}</button>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <label class="field">
                                        <span class="label" title=ui.t("import-figma-token-help")>{ui.t("import-figma-token")}{icons::info()}</span>
                                        <input class="input import-figma-token" type="password" autocomplete="off" spellcheck="false"
                                            title=ui.t("import-figma-token-help") disabled=busy
                                            prop:value=move || fig_token.get() on:input=move |e| fig_token.set(event_target_value(&e))/>
                                    </label>
                                }.into_any()
                            }}
                            {frames}
                            <Show when=move || fig_file.with(Option::is_some)>{location}</Show>
                        }.into_any(),
                        Source::WordPress => view! {
                            <div class="field">
                                <span class="label" title=ui.t("import-wp-file-help")>{ui.t("import-wp-file")}</span>
                                <div class="path-field">
                                    <span class="path-value import-wxr" class:is-empty=move || wxr.with(String::is_empty) title=move || wxr.get()>
                                        {move || crate::ltr(&wxr.get())}
                                    </span>
                                    <button type="button" class="btn btn--small import-choose-wxr" on:click=choose_wxr disabled=busy>{ui.t("import-choose")}</button>
                                </div>
                            </div>
                            {move || wxr_probe.get().map(|p| view! {
                                <div class="chips import-wp-probe">
                                    <span class="chip chip--ok" title=p.url.clone().unwrap_or_default()>{p.title.clone()}</span>
                                    <span class="chip">{ui.tx("import-wp-counts", &[("pages", p.pages.to_string()), ("posts", p.posts.to_string()), ("media", p.media.to_string())])}</span>
                                </div>
                            })}
                            <label class="field is-inline import-wp-media-row">
                                <span class="label" title=ui.t("import-wp-media-help")>{ui.t("import-wp-media")}</span>
                                <span class="switch">
                                    <input type="checkbox" class="import-wp-media" prop:checked=move || wp_media.get() disabled=busy
                                        on:change=move |e| wp_media.set(event_target_checked(&e))/>
                                    <span class="switch-track"></span>
                                </span>
                            </label>
                            {location}
                        }.into_any(),
                    }}
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
                        Phase::Reading => Some(view! {
                            <div class="import-progress">
                                <span class="muted">{ui.t("import-figma-reading")}</span>
                                <span class="meter-bar is-busy"><span class="meter-fill" style:width="100%"></span></span>
                            </div>
                        }.into_any()),
                        Phase::Importing(e) => {
                            let label = match (source.get(), e) {
                                (Source::Link, Some(e)) => format!(
                                    "{} {}",
                                    ui.now("import-reading", &[]),
                                    ui.now("import-progress", &[("pages", e.pages.to_string()), ("files", e.files.to_string()), ("size", size(e.bytes, fr()))]),
                                ),
                                (Source::Link, None) => ui.now("import-reading", &[]),
                                (_, Some(e)) if e.files > 0 => ui.now("import-downloading", &[("files", e.files.to_string())]),
                                _ => ui.now("import-importing", &[]),
                            };
                            Some(view! {
                                <div class="import-progress">
                                    <span class="muted import-progress-label">{label}</span>
                                    <span class="meter-bar is-busy"><span class="meter-fill" style:width="100%"></span></span>
                                </div>
                            }.into_any())
                        }
                        Phase::Building => {
                            let e = build_prog.get().unwrap_or_default();
                            let mb = |b: u64| crate::settings::size(ui, b);
                            let (label, frac) = match e.step.as_str() {
                                "download" if e.total > 0 => (
                                    ui.now("import-build-step-download", &[("tool", e.detail.clone()), ("done", mb(e.done)), ("total", mb(e.total))]),
                                    Some(e.done as f32 / e.total as f32),
                                ),
                                "download" => (ui.now("import-build-step-download-start", &[("tool", e.detail.clone())]), None),
                                "install" => (ui.now("import-build-step-install", &[("manager", e.detail.clone())]), None),
                                "build" => (ui.now("import-build-step-build", &[("tool", e.detail.clone())]), None),
                                "serve" => (ui.now("import-build-step-serve", &[]), None),
                                "read" => (ui.now("import-build-step-read", &[("pages", e.done.to_string()), ("files", e.total.to_string())]), None),
                                _ => (ui.now("import-build-step-start", &[]), None),
                            };
                            Some(view! {
                                <div class="import-progress import-build-progress" data-step=e.step.clone()>
                                    <span class="muted import-progress-label">{label}</span>
                                    <span class="meter-bar" class:is-busy=frac.is_none()>
                                        <span class="meter-fill" style:width=format!("{:.1}%", frac.unwrap_or(1.0) * 100.0)></span>
                                    </span>
                                    <span class="import-build-line" title=e.line.clone().unwrap_or_default()>{e.line.clone().unwrap_or_default()}</span>
                                    <details class="import-build-log">
                                        <summary>{ui.t("import-build-log")}</summary>
                                        <pre>{move || build_log.get().join("\n")}</pre>
                                    </details>
                                </div>
                            }.into_any())
                        }
                        Phase::Idle => None,
                    }}
                    {status}
                    {cms_field}
                    {move || error.get().map(|f| {
                        let log = f.args.iter().find(|(k, _)| k == "log").map(|(_, v)| v.clone()).filter(|l| !l.trim().is_empty());
                        view! {
                            <div class="notice notice--error import-error" role="alert">
                                <span>{ui.failure(&f)}</span>
                                {log.map(|l| {
                                    let copy = l.clone();
                                    view! {
                                        <details class="import-build-log">
                                            <summary>{ui.t("import-build-log")}</summary>
                                            <pre>{l}</pre>
                                            <button type="button" class="btn btn--small import-log-copy"
                                                on:click=move |_| api::copy_text(&copy)>{icons::copy()}{ui.t("action-copy")}</button>
                                        </details>
                                    }
                                })}
                            </div>
                        }
                    })}
                    <div class="dialog-actions">
                        <button type="button" class="btn import-cancel" on:click=move |_| quit()>{ui.t("action-cancel")}</button>
                        <button type="submit" class="btn btn--primary import-run" disabled=move || !can_submit()>
                            {move || if source.get() == Source::Figma && fig_file.with(Option::is_none) {
                                view! { {icons::search()}{ui.t("import-figma-read")} }.into_any()
                            } else {
                                view! { {icons::import()}{ui.t("import-run")} }.into_any()
                            }}
                        </button>
                    </div>
                }>
                    {summary}
                </Show>
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
        assert_eq!(repo_name(r"\\nas\Equipe\Sites\vitrine.git\"), "vitrine");
        assert_eq!(repo_name(r"Z:\Sites\vitrine.git"), "vitrine");
        assert_eq!(repo_name(""), "");
        assert_eq!(join("C:\\Projets\\site", "dist"), "C:\\Projets\\site\\dist");
        assert_eq!(join("/home/a/site/", "docs/web"), "/home/a/site/docs/web");
    }

    #[test]
    fn nom_du_site_en_ligne() {
        assert_eq!(
            host_name("https://www.Studio.framer.website/a"),
            "studio.framer.website"
        );
        assert_eq!(host_name("exemple.com"), "exemple.com");
        assert_eq!(host_name("pas une adresse"), "");
        assert_eq!(host_name(""), "");
        assert_eq!(size(1_500_000, true), "1,4 Mo");
        assert_eq!(size(340_000, false), "332 KB");
        assert_eq!(file_name("C:\\a\\b - Posts.csv"), "b - Posts.csv");
    }
}
