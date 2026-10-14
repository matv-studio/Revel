//! Sessions d'édition : un site ouvert dans l'éditeur, son historique, son enregistrement.
//!
//! Chaque modification passe par une commande (`revel-edit`), est appliquée au document en mémoire,
//! puis enregistrée aussitôt dans `site.revel.json` (écriture atomique). L'historique vit le temps
//! de la session.

use crate::{Core, CoreError, Result};
use revel_edit::{Command, EditState, History, Opened};
use revel_project::Project;
use std::path::Path;
use std::time::SystemTime;

pub(crate) struct Session {
    project: Project,
    history: History,
    revision: u64,
    /// Date de modification de `site.revel.json` après le dernier enregistrement de Revel.
    saved_mtime: Option<SystemTime>,
}

fn mtime(dir: &Path) -> Option<SystemTime> {
    std::fs::metadata(dir.join(revel_project::SITE_FILE))
        .and_then(|m| m.modified())
        .ok()
}

impl Session {
    fn state(&self, applied: Option<Command>) -> EditState {
        EditState {
            revision: self.revision,
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
            applied,
        }
    }

    fn save(&mut self) -> Result<()> {
        self.project.save()?;
        self.saved_mtime = mtime(&self.project.dir);
        Ok(())
    }
}

/// Exécutant des commandes d'une session : `SetBound` écrit la source de la valeur (fichier de
/// données, en-tête d'un contenu) ou la variable de page ; le reste passe par `revel_edit::apply`.
fn exec(
    project: &Project,
    site: &mut revel_model::Site,
    c: &Command,
) -> std::result::Result<Command, revel_edit::EditError> {
    match c {
        Command::SetBound { binding, value } => {
            let old = project
                .set_bound(site, binding, value)
                .map_err(revel_edit::EditError::Bound)?;
            Ok(Command::SetBound {
                binding: binding.clone(),
                value: old,
            })
        }
        other => revel_edit::apply(site, other),
    }
}

/// Empreinte des fichiers lus par le canevas (document, données, contenus) : change quand l'un
/// d'eux est modifié, ajouté ou retiré.
fn files_stamp(project: &Project) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Some(d) = &project.site.data_dir {
        dirs.push(project.dir.join(d));
    }
    for c in &project.site.collections {
        if !c.folder.is_empty() {
            dirs.push(project.dir.join(&c.folder));
        }
    }
    dirs.sort();
    dirs.dedup();
    let mut files = vec![project.dir.join(revel_project::SITE_FILE)];
    fn walk(d: &Path, out: &mut Vec<std::path::PathBuf>, depth: usize) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() && depth < 4 {
                walk(&p, out, depth + 1);
            } else {
                out.push(p);
            }
        }
    }
    for d in &dirs {
        walk(d, &mut files, 0);
    }
    files.sort();
    for f in files {
        f.hash(&mut h);
        if let Ok(m) = std::fs::metadata(&f) {
            m.len().hash(&mut h);
            if let Ok(t) = m.modified() {
                t.hash(&mut h);
            }
        }
    }
    format!("{:016x}", h.finish())
}

fn key(path: &Path) -> String {
    path.display().to_string()
}

impl Core {
    /// Projet d'un site ouvert pour l'édition (lecture).
    pub(crate) fn with_project<T>(
        &self,
        path: &Path,
        f: impl FnOnce(&Project) -> Result<T>,
    ) -> Result<T> {
        self.with_session(path, |s| f(&s.project))
    }

    fn with_session<T>(&self, path: &Path, f: impl FnOnce(&mut Session) -> Result<T>) -> Result<T> {
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let s = map.get_mut(&key(path)).ok_or(CoreError::NotOpen)?;
        f(s)
    }

    /// Ouvre un site pour l'édition (ou reprend la session en cours). Si le fichier a été modifié
    /// hors de Revel depuis le dernier enregistrement, il est relu et l'historique repart de zéro.
    pub fn edit_open(&self, path: &Path) -> Result<Opened> {
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let k = key(path);
        let stale = map
            .get(&k)
            .is_some_and(|s| s.saved_mtime.is_some() && s.saved_mtime != mtime(path));
        if stale || !map.contains_key(&k) {
            let project = Project::open(path)?;
            map.insert(
                k.clone(),
                Session {
                    project,
                    history: History::default(),
                    revision: 0,
                    saved_mtime: mtime(path),
                },
            );
        }
        let s = map.get(&k).ok_or(CoreError::NotOpen)?;
        Ok(Opened {
            site: s.project.site.clone(),
            state: s.state(None),
        })
    }

    /// Applique une commande, l'enregistre dans l'historique et sur disque.
    /// `coalesce` : saisie continue dans un même champ (une seule étape d'annulation).
    pub fn edit_apply(&self, path: &Path, cmd: Command, coalesce: bool) -> Result<EditState> {
        self.with_session(path, |s| {
            // Texte lié saisi dans le canevas : sans la typographie de la page si la source ne
            // l'utilisait pas.
            let cmd = match cmd {
                Command::SetBound { binding, value }
                    if s.project.site.settings.typography.as_deref() == Some("fr") =>
                {
                    let old = s.project.bound_value(&binding).unwrap_or_default();
                    Command::SetBound {
                        value: revel_edit::canvas::untypo(&value, &old),
                        binding,
                    }
                }
                other => other,
            };
            let project = &s.project;
            let mut site = project.site.clone();
            let applied = s
                .history
                .apply_with(&mut site, cmd, coalesce, &mut |site, c| {
                    exec(project, site, c)
                })
                .map_err(CoreError::Edit)?;
            s.project.site = site;
            s.revision += 1;
            s.save()?;
            Ok(s.state(Some(applied)))
        })
    }

    /// Empreinte des fichiers du site lus par le canevas : l'interface la compare régulièrement
    /// pour recharger la page quand un fichier a été modifié hors de Revel (Pages CMS…).
    pub fn edit_stamp(&self, path: &Path) -> Result<String> {
        self.with_project(path, |p| Ok(files_stamp(p)))
    }

    /// Termine une saisie continue (la prochaine commande ouvre une nouvelle étape).
    pub fn edit_seal(&self, path: &Path) -> Result<()> {
        self.with_session(path, |s| {
            s.history.seal();
            Ok(())
        })
    }

    pub fn edit_undo(&self, path: &Path) -> Result<EditState> {
        self.with_session(path, |s| {
            let project = &s.project;
            let mut site = project.site.clone();
            let applied = s
                .history
                .undo_with(&mut site, &mut |site, c| exec(project, site, c))
                .map_err(CoreError::Edit)?;
            s.project.site = site;
            if applied.is_some() {
                s.revision += 1;
                s.save()?;
            }
            Ok(s.state(applied))
        })
    }

    pub fn edit_redo(&self, path: &Path) -> Result<EditState> {
        self.with_session(path, |s| {
            let project = &s.project;
            let mut site = project.site.clone();
            let applied = s
                .history
                .redo_with(&mut site, &mut |site, c| exec(project, site, c))
                .map_err(CoreError::Edit)?;
            s.project.site = site;
            if applied.is_some() {
                s.revision += 1;
                s.save()?;
            }
            Ok(s.state(applied))
        })
    }

    /// Document en cours d'édition (copie).
    pub fn edit_site(&self, path: &Path) -> Result<revel_model::Site> {
        self.with_session(path, |s| Ok(s.project.site.clone()))
    }

    pub fn edit_close(&self, path: &Path) {
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(&key(path));
    }
}
