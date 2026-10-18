//! Construction d'un site par Revel (D-88), quand l'utilisateur l'accepte : le projet est copié
//! dans un dossier de travail, ses outils téléchargés (Node.js, Hugo), ses dépendances
//! installées, puis il est construit. Les pages produites, ou celles du serveur d'aperçu pour un
//! site rendu par un serveur, deviennent un site Revel ; le dossier de travail est retiré.
//!
//! Aussi : adresse du site publié d'un projet (pour l'importer depuis son lien), outils de
//! construction téléchargés (Réglages).

use crate::{Core, CoreError, Result, summarize};
use revel_build::toolchain::{self, Kind, Net};
use revel_build::{Event, Missing, Outcome, Recipe, Runtime};
use revel_protocol::{
    BuildPlan, BuildProgress, BuildToolInfo, BuildTools, ImportResult, InstalledBuildTool,
};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::Ordering;

/// Dossiers jamais copiés dans le dossier de travail (dépendances et résultats d'une
/// construction précédente, refaits).
const SKIP: &[&str] = &[
    "node_modules",
    ".svelte-kit",
    ".next",
    ".nuxt",
    ".output",
    ".astro",
    ".vercel",
    ".netlify",
    ".turbo",
    ".parcel-cache",
    ".cache",
    "vendor",
    ".jekyll-cache",
    "resources",
];

/// Copie `src` dans `dst` (créé), sans les dossiers de [`SKIP`] à la racine.
fn copy_tree(src: &Path, dst: &Path, skip_root: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let name = e.file_name();
        if skip_root && SKIP.iter().any(|s| name.to_string_lossy() == *s) {
            continue;
        }
        let from = e.path();
        let to = dst.join(&name);
        let t = e.file_type()?;
        if t.is_dir() {
            copy_tree(&from, &to, false)?;
        } else if t.is_file() {
            std::fs::copy(&from, &to)?;
        } else if t.is_symlink()
            && let Ok(target) = std::fs::canonicalize(&from)
        {
            // Lien : contenu copié s'il pointe dans le projet.
            if target.starts_with(src) {
                if target.is_dir() {
                    copy_tree(&target, &to, false)?;
                } else {
                    std::fs::copy(&target, &to)?;
                }
            }
        }
    }
    Ok(())
}

/// Copie les fichiers de `src` absents de `dst`.
fn merge_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            merge_tree(&e.path(), &to)?;
        } else if !to.exists() {
            std::fs::copy(e.path(), &to)?;
        }
    }
    Ok(())
}

/// Retire un dossier, y compris les fichiers en lecture seule (objets Git sous Windows).
pub(crate) fn remove_tree(dir: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => {
            let mut stack = vec![dir.to_path_buf()];
            while let Some(d) = stack.pop() {
                let Ok(rd) = std::fs::read_dir(&d) else {
                    continue;
                };
                for e in rd.flatten() {
                    let p = e.path();
                    if let Ok(m) = std::fs::symlink_metadata(&p) {
                        let mut perm = m.permissions();
                        if perm.readonly() {
                            #[allow(clippy::permissions_set_readonly_false)]
                            perm.set_readonly(false);
                            let _ = std::fs::set_permissions(&p, perm);
                        }
                        if m.is_dir() {
                            stack.push(p);
                        }
                    }
                }
            }
            std::fs::remove_dir_all(dir)
        }
    }
}

/// Nombre de fichiers d'un dossier (sans `site.revel.json`).
fn count_files(dir: &Path) -> usize {
    let mut n = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            match e.file_type() {
                Ok(t) if t.is_dir() => stack.push(e.path()),
                Ok(t) if t.is_file() && e.file_name() != revel_project::SITE_FILE => n += 1,
                _ => {}
            }
        }
    }
    n
}

fn missing_key(m: Missing) -> &'static str {
    match m {
        Missing::Ruby => "ruby",
        Missing::Go => "go",
        Missing::BuildScript => "build-script",
    }
}

fn has_cmd(c: &str) -> bool {
    revel_build::run::which(c).is_some()
}

impl Core {
    /// Dossier des outils de construction téléchargés.
    fn tools_dir(&self) -> PathBuf {
        self.config_dir().join("tools")
    }

    fn recipe(path: &Path) -> Result<Recipe> {
        revel_build::recipe::detect(path, &has_cmd).ok_or(CoreError::NothingToImport)
    }

    /// Ce que la construction du projet `path` utilisera et téléchargera.
    pub fn build_plan(&self, path: &Path) -> Result<BuildPlan> {
        self.build_plan_with(path, &toolchain::Http::new())
    }

    /// [`Core::build_plan`] avec un accès réseau donné (essais).
    pub fn build_plan_with(&self, path: &Path, net: &dyn Net) -> Result<BuildPlan> {
        let recipe = Self::recipe(path)?;
        let tools_dir = self.tools_dir();
        let mut kinds = Vec::new();
        if recipe.runtime == Runtime::Node || recipe.manager.is_some() {
            kinds.push(Kind::Node);
        }
        if recipe.runtime == Runtime::Hugo {
            kinds.push(Kind::Hugo);
        }
        let mut tools = Vec::new();
        for k in kinds {
            if let Some(t) = toolchain::find(&tools_dir, k) {
                tools.push(BuildToolInfo {
                    name: k.label().into(),
                    version: t.version,
                    size: None,
                    installed: true,
                    system: false,
                });
                continue;
            }
            let rel = toolchain::latest(k, net).map_err(CoreError::Build)?;
            let size = rel.size.or_else(|| net.size(&rel.url));
            tools.push(BuildToolInfo {
                name: k.label().into(),
                version: rel.version,
                size,
                installed: false,
                system: false,
            });
        }
        if recipe.runtime == Runtime::Ruby && recipe.missing.is_none() {
            tools.push(BuildToolInfo {
                name: "Ruby".into(),
                system: true,
                installed: true,
                ..Default::default()
            });
        }
        Ok(BuildPlan {
            tool: recipe.tool.clone(),
            tools,
            manager: recipe.manager.as_ref().map(|m| m.name().to_string()),
            missing: recipe.missing.map(|m| missing_key(m).to_string()),
            lock_ignored: recipe
                .notes
                .iter()
                .map(|n| match n {
                    revel_build::Note::LockIgnored(l) => l.clone(),
                })
                .next(),
        })
    }

    /// Construit le projet `path` puis importe le site produit. Copie de dépôt faite par Revel :
    /// le site prend sa place ; autre dossier : nouveau dossier dans `parent` (le projet n'est
    /// pas modifié).
    pub fn build_import(
        &self,
        path: &Path,
        parent: &Path,
        lang: &str,
        on_event: impl Fn(BuildProgress),
    ) -> Result<ImportResult> {
        self.build_import_with(
            path,
            parent,
            lang,
            &toolchain::Http::new(),
            &revel_import::crawl::Http::new(),
            on_event,
        )
    }

    /// [`Core::build_import`] avec des accès réseau donnés (essais).
    pub fn build_import_with(
        &self,
        path: &Path,
        parent: &Path,
        lang: &str,
        net: &dyn Net,
        fetch: &dyn revel_import::crawl::Fetch,
        on_event: impl Fn(BuildProgress),
    ) -> Result<ImportResult> {
        let recipe = Self::recipe(path)?;
        if let Some(m) = recipe.missing {
            return Err(CoreError::Build(revel_build::BuildError::Missing(m)));
        }
        // Copie de dépôt faite par Revel (le projet peut être un sous-dossier de la copie).
        let clone = self
            .clones
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|c| path.starts_with(c))
            .cloned();
        let name = clone
            .as_deref()
            .unwrap_or(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "site".into());
        let dest = match &clone {
            Some(c) => c.clone(),
            None => crate::site_folder(parent, &name),
        };
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let work = self
            .tools_dir()
            .join("work")
            .join(format!("{name}-{stamp}"));
        let staged = dest
            .parent()
            .unwrap_or(parent)
            .join(format!(".{name}-revel-{stamp}"));
        let cleanup = |also_staged: bool| {
            let _ = remove_tree(&work);
            if also_staged {
                let _ = remove_tree(&staged);
            }
        };
        self.import_cancel.store(false, Ordering::SeqCst);
        if let Err(e) = copy_tree(path, &work, true) {
            cleanup(true);
            return Err(e.into());
        }
        let tools = self.tools_dir();
        let cache = tools.join("cache");
        let step = Mutex::new(String::new());
        let on = |e: Event| match e {
            Event::Step(s, detail) => {
                *step.lock().unwrap_or_else(|e| e.into_inner()) = s.id().to_string();
                on_event(BuildProgress {
                    step: s.id().into(),
                    detail,
                    ..Default::default()
                })
            }
            Event::Bytes(done, total) => on_event(BuildProgress {
                step: "download".into(),
                done,
                total: total.unwrap_or(0),
                ..Default::default()
            }),
            Event::Line(l) => on_event(BuildProgress {
                step: step.lock().unwrap_or_else(|e| e.into_inner()).clone(),
                line: Some(l),
                ..Default::default()
            }),
        };
        let ctx = revel_build::Ctx {
            tools: &tools,
            cache: &cache,
            net,
            cancel: &self.import_cancel,
            on: &on,
        };
        let outcome = match revel_build::build(&work, &recipe, &ctx) {
            Ok(o) => o,
            Err(e) => {
                cleanup(true);
                return Err(match e {
                    revel_build::BuildError::Cancelled => CoreError::ImportCancelled,
                    e => CoreError::Build(e),
                });
            }
        };
        let mut warnings: Vec<String> = Vec::new();
        let mut runtime_kept = None;
        let served = matches!(outcome, Outcome::Served { .. });
        match outcome {
            Outcome::Static(out) => {
                if let Err(e) = copy_tree(&out, &staged, false) {
                    cleanup(true);
                    return Err(e.into());
                }
            }
            Outcome::Served {
                url,
                server,
                client_dirs,
            } => {
                on_event(BuildProgress {
                    step: "read".into(),
                    ..Default::default()
                });
                let start = revel_import::crawl::parse_start(&url).ok_or(CoreError::BadSiteUrl)?;
                let r = revel_import::crawl::crawl(
                    &start,
                    &staged,
                    fetch,
                    &revel_import::crawl::Options {
                        cancel: Some(&self.import_cancel),
                        ..Default::default()
                    },
                    &mut |p| {
                        on_event(BuildProgress {
                            step: "read".into(),
                            done: p.pages as u64,
                            total: p.files as u64,
                            ..Default::default()
                        })
                    },
                );
                server.stop();
                let crawled = match r {
                    Ok(c) => c,
                    Err(e) => {
                        cleanup(true);
                        return Err(match e {
                            revel_import::crawl::CrawlError::Cancelled => {
                                CoreError::ImportCancelled
                            }
                            revel_import::crawl::CrawlError::Io(e) => CoreError::Io(e),
                            other => CoreError::SiteUnreachable(other.to_string()),
                        });
                    }
                };
                // Fichiers du navigateur produits par la construction : tous copiés (ceux qu'un
                // script charge à la demande ne sont pas tous cités dans les pages).
                for (dir, prefix) in client_dirs {
                    let to = if prefix.is_empty() {
                        staged.clone()
                    } else {
                        staged.join(prefix)
                    };
                    if let Err(e) = merge_tree(&dir, &to) {
                        cleanup(true);
                        return Err(e.into());
                    }
                }
                runtime_kept = crawled.runtime.clone();
                warnings.extend(
                    crawled
                        .warnings
                        .into_iter()
                        .filter(|w| !w.contains(" : fichier introuvable")),
                );
            }
        }
        cleanup(false);
        // Le site prend la place de la copie du dépôt (sources et dépendances retirées).
        if let Some(c) = &clone {
            if let Err(e) = remove_tree(c) {
                let _ = remove_tree(&staged);
                return Err(e.into());
            }
            self.clones
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .retain(|x| x != c);
        }
        if let Err(e) = std::fs::rename(&staged, &dest) {
            let _ = remove_tree(&staged);
            return Err(e.into());
        }
        let imported = revel_import::html::import(
            &dest,
            &revel_import::html::Options {
                name: crate::import::pretty_name(&name),
                lang: lang.to_string(),
                merge_styles: true,
                ..Default::default()
            },
        );
        let (site, mut report) = match imported {
            Ok(x) => x,
            Err(e) => {
                let _ = remove_tree(&dest);
                return Err(e.into());
            }
        };
        let used: Vec<String> = toolchain::installed(&tools)
            .into_iter()
            .filter(|t| {
                (t.kind == Kind::Node
                    && (recipe.runtime == Runtime::Node || recipe.manager.is_some()))
                    || (t.kind == Kind::Hugo && recipe.runtime == Runtime::Hugo)
            })
            .map(|t| format!("{} {}", t.kind.label(), t.version))
            .collect();
        let mut first = vec![format!(
            "site construit par Revel ({}{}{})",
            recipe.tool,
            if used.is_empty() {
                String::new()
            } else {
                format!(", {}", used.join(", "))
            },
            recipe
                .manager
                .as_ref()
                .map(|m| format!(", {}", m.name()))
                .unwrap_or_default()
        )];
        for n in &recipe.notes {
            match n {
                revel_build::Note::LockIgnored(l) => first.push(format!(
                    "{l} ignoré : versions des dépendances choisies par npm, elles peuvent différer de celles de l'auteur"
                )),
            }
        }
        if served && runtime_kept.is_none() {
            first.push("pages lues sur le serveur d'aperçu du site construit".into());
        }
        first.append(&mut warnings);
        first.append(&mut report.warnings);
        report.warnings = first;
        report.tool = Some(recipe.tool.clone());
        // Fichiers du site (pages comprises), copiés ou lus.
        report.files = count_files(&dest);
        let p = revel_project::Project::create(&dest, site)?;
        let summary = summarize(&p)?;
        self.touch_recent(&summary);
        Ok(ImportResult {
            site: summary,
            stats: crate::import::stats(&report),
            warnings: report.warnings,
        })
    }

    /// Adresses du site publié d'un projet, vérifiées en ligne (page HTML qui répond) ; dépôt
    /// GitHub : adresse indiquée sur sa page et site GitHub Pages essayés aussi.
    pub fn published_sites(&self, path: &Path, repo_url: Option<&str>) -> Vec<String> {
        self.published_sites_with(path, repo_url, &revel_import::crawl::Http::new())
    }

    /// [`Core::published_sites`] avec un accès réseau donné (essais).
    pub fn published_sites_with(
        &self,
        path: &Path,
        repo_url: Option<&str>,
        fetch: &dyn revel_import::crawl::Fetch,
    ) -> Vec<String> {
        let mut candidates = revel_build::published::candidates(path);
        let gh = repo_url
            .and_then(revel_git::clone::parse_url)
            .and_then(|r| r.github);
        if let Some((owner, repo)) = &gh {
            if let Ok(v) = revel_import::crawl::get_json(
                fetch,
                &format!("https://api.github.com/repos/{owner}/{repo}"),
                &[("Accept", "application/vnd.github+json")],
            ) && let Some(h) = v.get("homepage").and_then(|h| h.as_str())
                && h.starts_with("http")
            {
                candidates.insert(0, h.to_string());
            }
            candidates.push(revel_build::published::github_pages(owner, repo));
        }
        let mut out: Vec<String> = Vec::new();
        for c in candidates {
            if out.len() >= 2
                || out
                    .iter()
                    .any(|o| o.trim_end_matches('/') == c.trim_end_matches('/'))
            {
                continue;
            }
            let Ok(u) = revel_import::crawl::Url::parse(&c) else {
                continue;
            };
            if let Ok(r) = fetch.get(&u)
                && r.status < 400
                && (r.content_type.contains("html") || r.body.starts_with(b"<"))
            {
                out.push(c);
            }
        }
        out
    }

    /// Outils de construction téléchargés et taille des caches.
    pub fn build_tools(&self) -> BuildTools {
        let tools = self.tools_dir();
        BuildTools {
            tools: toolchain::installed(&tools)
                .into_iter()
                .map(|t| InstalledBuildTool {
                    id: t
                        .dir
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    name: t.kind.label().into(),
                    version: t.version.clone(),
                    size: toolchain::dir_size(&t.dir),
                })
                .collect(),
            cache: toolchain::dir_size(&tools.join("cache")),
        }
    }

    /// Retire un outil téléchargé (`id` : nom de son dossier).
    pub fn build_tool_remove(&self, id: &str) -> Result<()> {
        let dir = self.tools_dir().join(id);
        if id.contains(['/', '\\'])
            || id.starts_with('.')
            || !toolchain::installed(&self.tools_dir())
                .iter()
                .any(|t| t.dir == dir)
        {
            return Err(CoreError::Other(format!("outil inconnu : {id}")));
        }
        remove_tree(&dir)?;
        Ok(())
    }

    /// Vide les caches de construction (paquets téléchargés).
    pub fn build_cache_clear(&self) -> Result<()> {
        remove_tree(&self.tools_dir().join("cache"))?;
        remove_tree(&self.tools_dir().join("work"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use revel_build::BuildError;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicBool;

    struct NoNet;

    impl Net for NoNet {
        fn text(&self, url: &str) -> std::result::Result<String, String> {
            assert_eq!(url, "https://nodejs.org/dist/index.json");
            Ok(r#"[{"version":"v24.22.0","lts":"Krypton","files":["linux-x64","linux-arm64","osx-arm64-tar","osx-x64-tar","win-x64-zip","win-arm64-zip"]}]"#.into())
        }
        fn size(&self, _url: &str) -> Option<u64> {
            Some(58_256_361)
        }
        fn download(
            &self,
            _url: &str,
            _to: &Path,
            _p: &mut dyn FnMut(u64, Option<u64>),
            _c: &AtomicBool,
        ) -> std::result::Result<String, BuildError> {
            panic!("aucun téléchargement pendant le plan");
        }
    }

    struct Pages(HashMap<String, u16>);

    impl revel_import::crawl::Fetch for Pages {
        fn get(
            &self,
            url: &revel_import::crawl::Url,
        ) -> std::result::Result<revel_import::crawl::Fetched, String> {
            let status = self.0.get(url.as_str()).copied().unwrap_or(404);
            Ok(revel_import::crawl::Fetched {
                status,
                url: url.clone(),
                content_type: if url.host_str() == Some("api.github.com") {
                    "application/json".into()
                } else {
                    "text/html".into()
                },
                body: if url.host_str() == Some("api.github.com") {
                    br#"{"homepage":"https://menu.example.org"}"#.to_vec()
                } else {
                    b"<html></html>".to_vec()
                },
            })
        }
    }

    #[test]
    fn plan_et_sites_publies() {
        let d = tempfile::tempdir().unwrap();
        let core = Core::new(
            &d.path().join("config"),
            Box::new(crate::secrets::Memory::default()),
        );
        let p = d.path().join("projet");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(
            p.join("package.json"),
            r#"{"scripts":{"build":"vite build","preview":"vite preview"},"devDependencies":{"@sveltejs/kit":"2"}}"#,
        )
        .unwrap();
        std::fs::write(p.join("bun.lock"), "{}").unwrap();
        std::fs::write(
            p.join("README.md"),
            "Démo : [voir ici](https://p3r.deltea.space)",
        )
        .unwrap();
        let plan = core.build_plan_with(&p, &NoNet).unwrap();
        assert_eq!(plan.tool, "SvelteKit");
        assert_eq!(plan.manager.as_deref(), Some("npm"));
        assert_eq!(plan.lock_ignored.as_deref(), Some("bun.lock"));
        assert_eq!(plan.tools.len(), 1);
        assert_eq!(
            (
                plan.tools[0].name.as_str(),
                plan.tools[0].version.as_str(),
                plan.tools[0].size,
                plan.tools[0].installed
            ),
            ("Node.js", "24.22.0", Some(58_256_361), false)
        );
        // Adresses vérifiées : page du dépôt (champ « homepage »), README ; GitHub Pages absent.
        let pages = Pages(
            [
                (
                    "https://api.github.com/repos/deltea/p3r-pause-menu".to_string(),
                    200,
                ),
                ("https://menu.example.org/".to_string(), 200),
                ("https://p3r.deltea.space/".to_string(), 200),
            ]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            core.published_sites_with(&p, Some("https://github.com/deltea/p3r-pause-menu"), &pages),
            vec!["https://menu.example.org", "https://p3r.deltea.space/"]
        );
        assert!(core.build_tools().tools.is_empty());
        assert!(core.build_tool_remove("../x").is_err());
    }
}
