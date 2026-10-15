//! Versions du site et synchronisation avec son dépôt GitHub (3.10, D-25). Le dossier du site est
//! un dépôt Git (activé par Revel s'il ne l'est pas) ; après une restauration ou une
//! récupération, la session d'édition est relue depuis le disque (l'éditeur, qui surveille les
//! fichiers, se recharge).

use crate::{Core, CoreError, Result};
use revel_git::versions as v;
use revel_protocol::{DiffLine, FileChange, SyncInfo, VersionInfo, VersionsStatus};
use std::path::Path;

fn info(x: v::Version) -> VersionInfo {
    VersionInfo {
        id: x.id,
        message: x.message,
        author: x.author,
        time: x.time,
        files: x.files,
        merge: x.merge,
    }
}

fn change(c: v::FileChange) -> FileChange {
    FileChange {
        path: c.path,
        change: match c.change {
            v::Change::Added => "added",
            v::Change::Modified => "modified",
            v::Change::Deleted => "deleted",
            v::Change::Renamed => "renamed",
        }
        .into(),
    }
}

fn lines(l: Vec<v::Line>) -> Vec<DiffLine> {
    l.into_iter()
        .map(|x| DiffLine {
            kind: x.kind,
            old: x.old,
            new: x.new,
            text: x.text,
        })
        .collect()
}

fn sync_info(r: v::SyncReport) -> SyncInfo {
    SyncInfo {
        pulled: r.pulled,
        pushed: r.pushed,
        merged: r.merged,
        conflicts: r.conflicts,
    }
}

impl Core {
    /// Auteur des versions faites par Revel (à défaut de celui réglé dans Git).
    fn author(&self) -> (String, String) {
        match self.github_account() {
            Some(login) => (login.clone(), format!("{login}@users.noreply.github.com")),
            None => ("Revel".into(), "revel@localhost".into()),
        }
    }

    /// Jeton GitHub pour un dépôt distant sur github.com (sinon aucun).
    fn remote_token(&self, dir: &Path) -> Result<Option<String>> {
        let remote = v::status(dir)?.remote.unwrap_or_default();
        if !remote.contains("github.com") || self.github_account().is_none() {
            return Ok(None);
        }
        Ok(Some(self.github_token()?.access_token))
    }

    /// État des versions ; `None` : versions pas encore activées.
    pub fn versions_status(&self, path: &Path) -> Result<Option<VersionsStatus>> {
        if !v::is_repo(path) {
            return Ok(None);
        }
        let s = v::status(path)?;
        Ok(Some(VersionsStatus {
            branch: s.branch,
            changes: s.changes,
            remote: s.remote,
            ahead: s.ahead,
            behind: s.behind,
            github: self.github_account().is_some(),
        }))
    }

    /// Active les versions du site (première version enregistrée).
    pub fn versions_init(&self, path: &Path, message: &str) -> Result<VersionInfo> {
        let (n, e) = self.author();
        Ok(info(v::init(path, &n, &e, message)?))
    }

    /// Versions, de la plus récente à la plus ancienne.
    pub fn versions_list(&self, path: &Path, limit: usize) -> Result<Vec<VersionInfo>> {
        Ok(v::history(path, limit)?.into_iter().map(info).collect())
    }

    /// Enregistre une version ; `None` : rien n'a changé.
    pub fn versions_save(&self, path: &Path, message: &str) -> Result<Option<VersionInfo>> {
        let (n, e) = self.author();
        Ok(v::save(path, &n, &e, message)?.map(info))
    }

    /// Fichiers changés : par une version (`of_version`), ou entre une version (ou la dernière)
    /// et le site actuel.
    pub fn versions_changes(
        &self,
        path: &Path,
        id: Option<&str>,
        of_version: bool,
    ) -> Result<Vec<FileChange>> {
        let list = match (id, of_version) {
            (Some(i), true) => v::version_changes(path, i)?,
            _ => v::changes(path, id)?,
        };
        Ok(list.into_iter().map(change).collect())
    }

    /// Comparaison d'un fichier entre une version (ou la dernière) et le site actuel ; `None` :
    /// fichier binaire.
    pub fn versions_diff(
        &self,
        path: &Path,
        id: Option<&str>,
        file: &str,
    ) -> Result<Option<Vec<DiffLine>>> {
        Ok(v::file_diff(path, id, file)?.map(lines))
    }

    /// Ce qu'une version a changé dans un fichier.
    pub fn versions_version_diff(
        &self,
        path: &Path,
        id: &str,
        file: &str,
    ) -> Result<Option<Vec<DiffLine>>> {
        Ok(v::version_file_diff(path, id, file)?.map(lines))
    }

    /// Dépôt distant du site (adresse HTTPS d'un dépôt GitHub) ; vide : retiré.
    pub fn versions_set_remote(&self, path: &Path, url: &str) -> Result<()> {
        Ok(v::set_remote(path, url)?)
    }

    /// Restaure une version (l'état actuel est enregistré d'abord) ; la session d'édition est
    /// relue.
    pub fn versions_restore(
        &self,
        path: &Path,
        id: &str,
        before: &str,
        message: &str,
    ) -> Result<Option<VersionInfo>> {
        let (n, e) = self.author();
        let r = v::restore(path, id, &n, &e, before, message)?;
        self.edit_reload(path)?;
        Ok(r.map(info))
    }

    /// Synchronise avec le dépôt distant : changements enregistrés, versions en ligne récupérées
    /// (Pages CMS), versions du site envoyées.
    pub fn versions_sync(&self, path: &Path, message: &str, merge: &str) -> Result<SyncInfo> {
        let status = v::status(path)?;
        if status.remote.is_none() {
            return Err(CoreError::Other("aucun dépôt distant".into()));
        }
        let token = self.remote_token(path)?;
        let (n, e) = self.author();
        let r = v::sync(path, token.as_deref(), &n, &e, message, merge)?;
        if r.pulled > 0 || r.merged {
            self.edit_reload(path)?;
        }
        Ok(sync_info(r))
    }

    /// Règle un conflit : version du site (`keep_ours`) ou version en ligne, puis envoi.
    pub fn versions_resolve(
        &self,
        path: &Path,
        keep_ours: bool,
        message: &str,
    ) -> Result<SyncInfo> {
        let token = self.remote_token(path)?;
        let (n, e) = self.author();
        let r = v::resolve(path, token.as_deref(), keep_ours, &n, &e, message)?;
        self.edit_reload(path)?;
        Ok(sync_info(r))
    }
}
