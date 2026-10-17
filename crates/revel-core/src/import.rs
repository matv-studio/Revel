//! Import d'un site existant (étape 7) : examen d'un dossier, import Eleventy ou HTML statique
//! (7.1), copie d'un dépôt Git (7.2, D-80).

use crate::{Core, CoreError, Result, summarize};
use revel_import::SiteKind;
use revel_protocol::{
    CloneEvent, ClonedRepo, FigmaCanvas, FigmaFile, FigmaFrame, ImportEvent, ImportKind,
    ImportOptions, ImportProbe, ImportResult, ImportStats, WxrProbe,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

/// Jeton Figma dans le trousseau.
const FIGMA_TOKEN: &str = "figma:token";

/// Bilan d'un import pour l'interface.
fn stats(r: &revel_import::Report) -> ImportStats {
    ImportStats {
        tool: r.tool.clone(),
        pages: r.pages,
        layouts: r.layouts,
        style_rules: r.style_rules,
        animations: r.animations,
        forms: r.forms,
        collections: r.collections,
        entries: r.entries,
        files: r.files,
    }
}

/// Fichiers CSV du CMS de Webflow à la racine d'un dossier.
fn webflow_csv_in(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| {
                    e.path().is_file()
                        && e.file_name()
                            .to_string_lossy()
                            .to_ascii_lowercase()
                            .ends_with(".csv")
                })
                .filter(|e| {
                    std::fs::read_to_string(e.path())
                        .is_ok_and(|t| revel_import::webflow::is_webflow_csv(&t))
                })
                .map(|e| e.path().display().to_string())
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

impl Core {
    /// Nature du dossier `path` avant import.
    pub fn probe_site(&self, path: &Path) -> Result<ImportProbe> {
        if !path.is_dir() {
            return Err(CoreError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                path.display().to_string(),
            )));
        }
        let has_site = path.join(revel_project::SITE_FILE).exists();
        Ok(match revel_import::detect(path) {
            SiteKind::Eleventy => ImportProbe {
                kind: ImportKind::Eleventy,
                pages: 0,
                tool: None,
                outputs: Vec::new(),
                has_site,
                made_with: None,
                csv: Vec::new(),
            },
            SiteKind::Static { pages } => {
                let made_with = revel_import::html::origin(path).map(|t| t.name().to_string());
                let csv = if made_with.as_deref() == Some("Webflow") {
                    webflow_csv_in(path)
                } else {
                    Vec::new()
                };
                ImportProbe {
                    kind: ImportKind::Static,
                    pages,
                    tool: None,
                    outputs: Vec::new(),
                    has_site,
                    made_with,
                    csv,
                }
            }
            SiteKind::NeedsBuild { tool, outputs } => ImportProbe {
                kind: ImportKind::NeedsBuild,
                pages: 0,
                tool: Some(tool),
                outputs,
                has_site,
                made_with: None,
                csv: Vec::new(),
            },
            SiteKind::Unknown { outputs } => ImportProbe {
                kind: ImportKind::Unknown,
                pages: 0,
                tool: None,
                outputs,
                has_site,
                made_with: None,
                csv: Vec::new(),
            },
        })
    }

    /// Importe le site du dossier `path` (Eleventy ou HTML statique) : écrit `site.revel.json` à
    /// sa racine, sans modifier les autres fichiers (export Webflow : contenus du CMS ajoutés
    /// dans `content/`).
    pub fn import_site(
        &self,
        path: &Path,
        opts: &ImportOptions,
        on_event: impl Fn(ImportEvent),
    ) -> Result<ImportResult> {
        match revel_import::detect(path) {
            SiteKind::Eleventy => self.import_eleventy(path, opts.overwrite),
            SiteKind::Static { .. } => {
                if path.join(revel_project::SITE_FILE).exists() && !opts.overwrite {
                    return Err(CoreError::AlreadyImported);
                }
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Site".into());
                let (mut site, mut report) = revel_import::html::import(
                    path,
                    &revel_import::html::Options {
                        name: pretty_name(&name),
                        lang: opts.lang.clone(),
                        ..Default::default()
                    },
                )?;
                // Contenus du CMS de Webflow : API (jeton) ou fichiers CSV.
                if report.tool.as_deref() == Some("Webflow") {
                    self.import_cancel.store(false, Ordering::SeqCst);
                    let http = revel_import::crawl::Http::new();
                    let mut cols = Vec::new();
                    if let Some(token) = opts
                        .webflow_token
                        .as_deref()
                        .filter(|t| !t.trim().is_empty())
                    {
                        match revel_import::webflow::site_id(&site) {
                            Some(id) => {
                                cols = revel_import::webflow::from_api(&http, &id, token).map_err(
                                    |e| match e {
                                        revel_import::webflow::ApiError::Token => CoreError::WebflowToken,
                                        other => CoreError::Other(other.to_string()),
                                    },
                                )?;
                            }
                            None => report.warnings.push(
                                "Webflow : identifiant du site absent des pages, contenus du CMS non lus".into(),
                            ),
                        }
                    } else {
                        let mut files = opts.csv.clone();
                        for f in webflow_csv_in(path) {
                            if !files.contains(&f) {
                                files.push(f);
                            }
                        }
                        for f in files {
                            let p = Path::new(&f);
                            let text = std::fs::read_to_string(p)?;
                            let file = p
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            match revel_import::webflow::from_csv(&file, &text) {
                                Some(c) => cols.push(c),
                                None => report.warnings.push(format!(
                                    "{file} : fichier CSV du CMS de Webflow non reconnu"
                                )),
                            }
                        }
                        revel_import::webflow::link_csv_references(&mut cols);
                    }
                    if !cols.is_empty() {
                        revel_import::webflow::apply(
                            &mut site,
                            path,
                            cols,
                            &http,
                            Some(&self.import_cancel),
                            &mut |n| {
                                on_event(ImportEvent {
                                    pages: 0,
                                    files: n,
                                    bytes: 0,
                                })
                            },
                            &mut report,
                        )
                        .map_err(CoreError::Other)?;
                    }
                }
                let p = revel_project::Project::create(path, site)?;
                let summary = summarize(&p)?;
                self.touch_recent(&summary);
                self.clones
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .retain(|c| !path.starts_with(c));
                Ok(ImportResult {
                    site: summary,
                    stats: stats(&report),
                    warnings: report.warnings,
                })
            }
            SiteKind::NeedsBuild { tool, .. } => Err(CoreError::NeedsBuild(tool)),
            SiteKind::Unknown { .. } => Err(CoreError::NothingToImport),
        }
    }

    /// Interrompt l'import en cours (site lu en ligne, médias, Figma).
    pub fn import_cancel(&self) {
        self.import_cancel.store(true, Ordering::SeqCst);
    }

    /// Lit le site publié à l'adresse `url` (Framer, Webflow, tout site) dans un nouveau dossier
    /// de `parent`, puis l'importe comme un dossier de pages HTML (7.4) : styles écrits dans les
    /// pages repris dans le panneau Styles, moteur de l'outil retiré, animations reprises.
    pub fn import_url(
        &self,
        url: &str,
        parent: &Path,
        lang: &str,
        on_event: impl Fn(ImportEvent),
    ) -> Result<ImportResult> {
        self.import_url_with(
            url,
            parent,
            lang,
            &revel_import::crawl::Http::new(),
            on_event,
        )
    }

    /// [`Core::import_url`] avec un accès réseau donné (essais).
    pub fn import_url_with(
        &self,
        url: &str,
        parent: &Path,
        lang: &str,
        fetch: &dyn revel_import::crawl::Fetch,
        on_event: impl Fn(ImportEvent),
    ) -> Result<ImportResult> {
        use revel_import::crawl;
        let start = crawl::parse_start(url).ok_or(CoreError::BadSiteUrl)?;
        let name = crawl::site_name(&start);
        let dest = crate::site_folder(parent, &name);
        self.import_cancel.store(false, Ordering::SeqCst);
        let r = crawl::crawl(
            &start,
            &dest,
            fetch,
            &crawl::Options {
                cancel: Some(&self.import_cancel),
                ..Default::default()
            },
            &mut |p| {
                on_event(ImportEvent {
                    pages: p.pages,
                    files: p.files,
                    bytes: p.bytes,
                })
            },
        );
        let fail = |e: CoreError| {
            if dest.exists() {
                let _ = std::fs::remove_dir_all(&dest);
            }
            e
        };
        let crawled = match r {
            Ok(c) => c,
            Err(e) => {
                return Err(fail(match e {
                    crawl::CrawlError::BadUrl(_) => CoreError::BadSiteUrl,
                    crawl::CrawlError::Unreachable(d) => CoreError::SiteUnreachable(d),
                    crawl::CrawlError::NotHtml => CoreError::NotHtml,
                    crawl::CrawlError::Cancelled => CoreError::ImportCancelled,
                    crawl::CrawlError::Io(e) => CoreError::Io(e),
                }));
            }
        };
        let imported = revel_import::html::import(
            &dest,
            &revel_import::html::Options {
                name: pretty_name(&name),
                lang: lang.to_string(),
                merge_styles: true,
                ..Default::default()
            },
        );
        let (mut site, mut report) = match imported {
            Ok(x) => x,
            Err(e) => return Err(fail(e.into())),
        };
        if site.base_url.is_none() {
            site.base_url = Some(start.origin().ascii_serialization());
        }
        report.files = crawled.files;
        if report.tool.is_none() {
            report.tool = crawled.tool.clone();
        }
        let mut warnings = crawled.warnings;
        warnings.append(&mut report.warnings);
        report.warnings = warnings;
        let p = match revel_project::Project::create(&dest, site) {
            Ok(p) => p,
            Err(e) => return Err(fail(e.into())),
        };
        let summary = summarize(&p)?;
        self.touch_recent(&summary);
        Ok(ImportResult {
            site: summary,
            stats: stats(&report),
            warnings: report.warnings,
        })
    }

    /// Examine un fichier d'export WordPress (WXR) avant import.
    pub fn probe_wordpress(&self, file: &Path) -> Result<WxrProbe> {
        let text = std::fs::read_to_string(file)?;
        let w = revel_import::wordpress::parse(&text).map_err(CoreError::NotWxr)?;
        Ok(WxrProbe {
            title: w.title.clone(),
            url: (!w.url.is_empty()).then(|| w.url.clone()),
            pages: w.pages().count(),
            posts: w.posts().count(),
            media: w.attachments().count(),
        })
    }

    /// Importe un site WordPress depuis son fichier d'export (7.6) dans un nouveau dossier de
    /// `parent` : modèle Blog de Revel, articles en collection, pages, menu, images téléchargées
    /// depuis le site si `media`.
    pub fn import_wordpress(
        &self,
        file: &Path,
        parent: &Path,
        lang: &str,
        media: bool,
        on_event: impl Fn(ImportEvent),
    ) -> Result<ImportResult> {
        self.import_wordpress_with(
            file,
            parent,
            lang,
            media,
            &revel_import::crawl::Http::new(),
            on_event,
        )
    }

    /// [`Core::import_wordpress`] avec un accès réseau donné (essais).
    pub fn import_wordpress_with(
        &self,
        file: &Path,
        parent: &Path,
        lang: &str,
        media: bool,
        fetch: &dyn revel_import::crawl::Fetch,
        on_event: impl Fn(ImportEvent),
    ) -> Result<ImportResult> {
        let text = std::fs::read_to_string(file)?;
        let w = revel_import::wordpress::parse(&text).map_err(CoreError::NotWxr)?;
        let title = if w.title.trim().is_empty() {
            "WordPress".to_string()
        } else {
            w.title.trim().to_string()
        };
        let site_lang = w
            .language
            .split(['-', '_'])
            .next()
            .filter(|l| !l.is_empty())
            .map(str::to_ascii_lowercase)
            .unwrap_or_else(|| lang.to_string());
        let dest = crate::site_folder(parent, &title);
        if dest.join(revel_project::SITE_FILE).exists() {
            return Err(CoreError::AlreadyImported);
        }
        std::fs::create_dir_all(&dest)?;
        let fail = |e: CoreError| {
            let _ = std::fs::remove_dir_all(&dest);
            e
        };
        self.import_cancel.store(false, Ordering::SeqCst);
        let mut site = crate::starter::starter_from(
            &title,
            &site_lang,
            revel_protocol::SiteTemplate::Blog,
            true,
        );
        let fr = site_lang.starts_with("fr");
        let report = revel_import::wordpress::fill(
            &mut site,
            &dest,
            &w,
            fetch,
            &revel_import::wordpress::Options {
                media,
                fr,
                cancel: Some(&self.import_cancel),
            },
            &mut |n| {
                on_event(ImportEvent {
                    pages: 0,
                    files: n,
                    bytes: 0,
                })
            },
        );
        let report = match report {
            Ok(r) => r,
            Err(_) if self.import_cancel.load(Ordering::SeqCst) => {
                return Err(fail(CoreError::ImportCancelled));
            }
            Err(e) => return Err(fail(e.into())),
        };
        if let Err(e) = std::fs::write(
            dest.join(revel_project::pagesyml::CONFIG_FILE),
            revel_import::wordpress::pages_yml(fr),
        ) {
            return Err(fail(e.into()));
        }
        let _ = std::fs::create_dir_all(dest.join("media"));
        let p = match revel_project::Project::create(&dest, site) {
            Ok(p) => p,
            Err(e) => return Err(fail(e.into())),
        };
        let summary = summarize(&p)?;
        self.touch_recent(&summary);
        let mut st = stats(&report);
        st.layouts = p.site.layouts.len();
        st.style_rules = p
            .site
            .stylesheet
            .as_ref()
            .map(|s| {
                s.items
                    .iter()
                    .filter(|i| matches!(i, revel_model::StyleItem::Rule(_)))
                    .count()
            })
            .unwrap_or(0);
        Ok(ImportResult {
            site: summary,
            stats: st,
            warnings: report.warnings,
        })
    }

    /// Un jeton Figma est gardé dans le trousseau.
    pub fn figma_has_token(&self) -> bool {
        self.secrets.get(FIGMA_TOKEN).ok().flatten().is_some()
    }

    /// Oublie le jeton Figma.
    pub fn figma_forget_token(&self) -> Result<()> {
        self.secrets.delete(FIGMA_TOKEN).map_err(CoreError::Secret)
    }

    fn figma_token(&self, given: Option<&str>) -> Result<String> {
        match given.map(str::trim).filter(|t| !t.is_empty()) {
            Some(t) => Ok(t.to_string()),
            None => self
                .secrets
                .get(FIGMA_TOKEN)
                .ok()
                .flatten()
                .ok_or(CoreError::FigmaToken),
        }
    }

    /// Lit un fichier Figma (pages et cadres) avant import ; un jeton donné et accepté est gardé
    /// dans le trousseau.
    pub fn figma_read(&self, url: &str, token: Option<&str>) -> Result<FigmaFile> {
        self.figma_read_with(url, token, &revel_import::crawl::Http::new())
    }

    /// [`Core::figma_read`] avec un accès réseau donné (essais).
    pub fn figma_read_with(
        &self,
        url: &str,
        token: Option<&str>,
        fetch: &dyn revel_import::crawl::Fetch,
    ) -> Result<FigmaFile> {
        use revel_import::figma;
        let key = figma::file_key(url).ok_or(CoreError::FigmaUrl)?;
        let t = self.figma_token(token)?;
        let file = figma::fetch_file(fetch, &key, &t).map_err(figma_error)?;
        if token.is_some_and(|x| !x.trim().is_empty()) {
            self.secrets
                .set(FIGMA_TOKEN, &t)
                .map_err(CoreError::Secret)?;
        }
        Ok(FigmaFile {
            key,
            name: file["name"].as_str().unwrap_or("Figma").to_string(),
            canvases: figma::outline(&file)
                .into_iter()
                .map(|(id, name, frames)| FigmaCanvas {
                    id,
                    name,
                    frames: frames
                        .into_iter()
                        .map(|f| FigmaFrame {
                            id: f.id,
                            name: f.name,
                            width: f.width.round().max(0.0) as u32,
                            height: f.height.round().max(0.0) as u32,
                        })
                        .collect(),
                })
                .collect(),
        })
    }

    /// Importe les cadres `frames` du fichier Figma `url` dans un nouveau dossier de `parent`
    /// (7.5) ; polices utilisées ajoutées depuis Google Fonts quand elles y sont.
    pub fn import_figma(
        &self,
        url: &str,
        frames: &[String],
        parent: &Path,
        lang: &str,
        on_event: impl Fn(ImportEvent),
    ) -> Result<ImportResult> {
        self.import_figma_with(
            url,
            frames,
            parent,
            lang,
            &revel_import::crawl::Http::new(),
            true,
            on_event,
        )
    }

    /// [`Core::import_figma`] avec un accès réseau donné ; `fonts` : polices ajoutées.
    #[allow(clippy::too_many_arguments)]
    pub fn import_figma_with(
        &self,
        url: &str,
        frames: &[String],
        parent: &Path,
        lang: &str,
        fetch: &dyn revel_import::crawl::Fetch,
        fonts: bool,
        on_event: impl Fn(ImportEvent),
    ) -> Result<ImportResult> {
        use revel_import::figma;
        let key = figma::file_key(url).ok_or(CoreError::FigmaUrl)?;
        let token = self.figma_token(None)?;
        let file = figma::fetch_file(fetch, &key, &token).map_err(figma_error)?;
        let name = file["name"].as_str().unwrap_or("Figma").to_string();
        let dest = crate::site_folder(parent, &name);
        std::fs::create_dir_all(&dest)?;
        let fail = |e: CoreError| {
            let _ = std::fs::remove_dir_all(&dest);
            e
        };
        self.import_cancel.store(false, Ordering::SeqCst);
        let imported = figma::import(
            &file,
            &key,
            &token,
            fetch,
            &dest,
            &figma::Options {
                frames: frames.to_vec(),
                lang: lang.to_string(),
                cancel: Some(&self.import_cancel),
            },
            &mut |done, _total| {
                on_event(ImportEvent {
                    pages: frames.len(),
                    files: done,
                    bytes: 0,
                })
            },
        );
        let mut imp = match imported {
            Ok(i) => i,
            Err(e) => return Err(fail(figma_error(e))),
        };
        let _ = std::fs::create_dir_all(dest.join("media"));
        let p = match revel_project::Project::create(&dest, imp.site) {
            Ok(p) => p,
            Err(e) => return Err(fail(e.into())),
        };
        // Polices : copies locales depuis Google Fonts (aucune requête des visiteurs).
        if fonts && !imp.fonts.is_empty() && self.edit_open(&dest).is_ok() {
            for (family, variants) in &imp.fonts {
                let v: Vec<String> = variants.iter().map(|x| x.replace("italic", "i")).collect();
                if let Err(e) = self.fonts_add_google(&dest, family, &v) {
                    imp.report.warnings.push(format!(
                        "police « {family} » non ajoutée ({}) : à ajouter dans Styles › Polices",
                        e
                    ));
                }
            }
            self.edit_close(&dest);
        }
        let summary = summarize(&revel_project::Project::open(&dest).unwrap_or(p))?;
        self.touch_recent(&summary);
        Ok(ImportResult {
            site: summary,
            stats: stats(&imp.report),
            warnings: imp.report.warnings,
        })
    }

    /// Copie le dépôt `url` dans un nouveau dossier de `parent` (nom du dépôt, suffixe si déjà
    /// pris). Dépôt sur github.com : jeton du compte connecté s'il y en a un (dépôt privé).
    pub fn git_clone(
        &self,
        url: &str,
        parent: &Path,
        on_event: impl Fn(CloneEvent),
    ) -> Result<ClonedRepo> {
        let repo = revel_git::clone::parse_url(url).ok_or(CoreError::BadRepoUrl)?;
        let dest = crate::site_folder(parent, &repo.name);
        if dest.exists() {
            return Err(CoreError::AlreadyImported);
        }
        let token = if repo.github.is_some() {
            self.github_token().ok().map(|t| t.access_token)
        } else {
            None
        };
        self.clone_cancel.store(false, Ordering::SeqCst);
        let r = revel_git::clone::clone(&repo, &dest, token.as_deref(), &mut |p| {
            on_event(CloneEvent {
                received: p.received,
                total: p.total,
            });
            !self.clone_cancel.load(Ordering::SeqCst)
        });
        if let Err(e) = r {
            // Copie inachevée : dossier créé ici, retiré.
            if dest.exists() {
                let _ = std::fs::remove_dir_all(&dest);
            }
            if self.clone_cancel.load(Ordering::SeqCst) {
                return Err(CoreError::CloneCancelled);
            }
            return Err(e.into());
        }
        self.clones
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(dest.clone());
        let license = std::fs::read_dir(&dest).is_ok_and(|rd| {
            rd.flatten().any(|e| {
                let n = e.file_name().to_string_lossy().to_ascii_lowercase();
                n.starts_with("license") || n.starts_with("licence") || n.starts_with("copying")
            })
        });
        Ok(ClonedRepo {
            path: dest.display().to_string(),
            name: repo.name,
            subdir: repo.subdir,
            license,
        })
    }

    /// Interrompt la copie de dépôt en cours.
    pub fn git_clone_cancel(&self) {
        self.clone_cancel.store(true, Ordering::SeqCst);
    }

    /// Retire une copie de dépôt faite par [`Core::git_clone`] et pas importée (import
    /// abandonné) ; tout autre dossier est refusé.
    pub fn discard_clone(&self, path: &Path) -> Result<()> {
        let mut clones = self.clones.lock().unwrap_or_else(|e| e.into_inner());
        let Some(i) = clones.iter().position(|c| c == path) else {
            return Err(CoreError::NotAClone);
        };
        let dir: PathBuf = clones.remove(i);
        if dir.join(revel_project::SITE_FILE).exists() {
            return Err(CoreError::NotAClone);
        }
        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }
}

fn figma_error(e: revel_import::figma::FigmaError) -> CoreError {
    use revel_import::figma::FigmaError as F;
    match e {
        F::BadUrl => CoreError::FigmaUrl,
        F::Token => CoreError::FigmaToken,
        F::NotFound => CoreError::FigmaNotFound,
        F::Cancelled => CoreError::ImportCancelled,
        F::Api(d) | F::Io(d) => CoreError::Figma(d),
    }
}

/// Nom lisible d'un dossier (`mon-site_v2` → `Mon site v2`).
fn pretty_name(folder: &str) -> String {
    let s: String = folder
        .chars()
        .map(|c| if c == '-' || c == '_' { ' ' } else { c })
        .collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => "Site".into(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn nom_lisible() {
        assert_eq!(super::pretty_name("mon-site_v2"), "Mon site v2");
        assert_eq!(super::pretty_name(""), "Site");
    }
}
