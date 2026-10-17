//! Import d'un site existant (étape 7) : examen d'un dossier, import Eleventy ou HTML statique
//! (7.1), copie d'un dépôt Git (7.2, D-80).

use crate::{Core, CoreError, Result, summarize};
use revel_import::SiteKind;
use revel_protocol::{CloneEvent, ClonedRepo, ImportKind, ImportProbe, ImportResult};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

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
            },
            SiteKind::Static { pages } => ImportProbe {
                kind: ImportKind::Static,
                pages,
                tool: None,
                outputs: Vec::new(),
                has_site,
            },
            SiteKind::NeedsBuild { tool, outputs } => ImportProbe {
                kind: ImportKind::NeedsBuild,
                pages: 0,
                tool: Some(tool),
                outputs,
                has_site,
            },
            SiteKind::Unknown { outputs } => ImportProbe {
                kind: ImportKind::Unknown,
                pages: 0,
                tool: None,
                outputs,
                has_site,
            },
        })
    }

    /// Importe le site du dossier `path` (Eleventy ou HTML statique) : écrit `site.revel.json` à
    /// sa racine, sans modifier les autres fichiers. `lang` : langue du site si ses pages n'en
    /// déclarent pas.
    pub fn import_site(&self, path: &Path, overwrite: bool, lang: &str) -> Result<ImportResult> {
        match revel_import::detect(path) {
            SiteKind::Eleventy => self.import_eleventy(path, overwrite),
            SiteKind::Static { .. } => {
                if path.join(revel_project::SITE_FILE).exists() && !overwrite {
                    return Err(CoreError::AlreadyImported);
                }
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Site".into());
                let (site, report) = revel_import::html::import(
                    path,
                    &revel_import::html::Options {
                        name: pretty_name(&name),
                        lang: lang.to_string(),
                    },
                )?;
                let p = revel_project::Project::create(path, site)?;
                let summary = summarize(&p)?;
                self.touch_recent(&summary);
                self.clones
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .retain(|c| !path.starts_with(c));
                Ok(ImportResult {
                    site: summary,
                    warnings: report.warnings,
                })
            }
            SiteKind::NeedsBuild { tool, .. } => Err(CoreError::NeedsBuild(tool)),
            SiteKind::Unknown { .. } => Err(CoreError::NothingToImport),
        }
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
