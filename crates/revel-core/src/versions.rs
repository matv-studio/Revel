//! Versions du site et synchronisation avec son dépôt GitHub (3.10, D-25). Le dossier du site est
//! un dépôt Git (activé par Revel s'il ne l'est pas) ; après une restauration ou une
//! récupération, la session d'édition est relue depuis le disque (l'éditeur, qui surveille les
//! fichiers, se recharge).

use crate::{Core, CoreError, Result};
use revel_git::versions as v;
use revel_protocol::{
    DiffLine, FileChange, SyncConflictItem, SyncInfo, VersionInfo, VersionsStatus,
};
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

fn sync_info(r: v::SyncReport, dir: &Path) -> SyncInfo {
    // Document local : noms des pages, composants… des endroits à choisir.
    let doc: Option<serde_json::Value> =
        std::fs::read_to_string(dir.join(revel_project::SITE_FILE))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok());
    SyncInfo {
        pulled: r.pulled,
        pushed: r.pushed,
        merged: r.merged,
        conflicts: r.conflicts,
        items: r
            .items
            .into_iter()
            .map(|i| label_item(i, doc.as_ref()))
            .collect(),
    }
}

/// Où se trouve un endroit modifié des deux côtés : page, composant, styles… et élément.
fn label_item(i: v::ItemConflict, doc: Option<&serde_json::Value>) -> SyncConflictItem {
    let segs: Vec<&str> = i.id.split('/').filter(|s| !s.is_empty()).collect();
    let (area, name) = if i.file != revel_project::SITE_FILE {
        ("data", i.file.clone())
    } else {
        match segs.as_slice() {
            [
                list @ ("pages" | "layouts" | "components" | "collections"),
                id,
                ..,
            ] if id.starts_with('@') => {
                let id = &id[1..];
                let area = match *list {
                    "pages" => "page",
                    "layouts" => "layout",
                    "components" => "component",
                    _ => "collection",
                };
                // Page : son adresse ; autres : leur nom, sinon leur identifiant.
                let key = if *list == "pages" { "path" } else { "name" };
                let name = doc
                    .and_then(|d| d.get(*list)?.as_array()?.iter().find(|x| x["id"] == id))
                    .and_then(|x| x.get(key).and_then(|p| p.as_str()).map(str::to_string))
                    .unwrap_or_else(|| id.to_string());
                (area, name)
            }
            ["stylesheet", ..] => ("styles", String::new()),
            ["settings", ..] => ("settings", String::new()),
            _ => ("site", String::new()),
        }
    };
    // Élément : dernier nœud repéré par son `id` dans le chemin (balise et classe).
    let mut detail = String::new();
    if let Some(d) = doc {
        let mut cur = Some(d);
        let mut last_tag: Option<String> = None;
        for s in &segs {
            cur = cur.and_then(|c| {
                if let Some(id) = s.strip_prefix('@') {
                    c.as_array()?.iter().find(|x| x["id"] == id)
                } else if let Some(n) = s.strip_prefix('#') {
                    c.as_array()?.get(n.parse::<usize>().ok()?)
                } else {
                    c.get(*s)
                }
            });
            if s.starts_with('@')
                && let Some(t) = cur.and_then(|c| c.get("tag")).and_then(|t| t.as_str())
            {
                let class = cur
                    .and_then(|c| c.get("attrs"))
                    .and_then(|a| a.as_array())
                    .and_then(|a| a.iter().find(|x| x["name"] == "class"))
                    .and_then(|x| x["value"].as_str())
                    .and_then(|v| v.split_whitespace().next())
                    .map(|c| format!(".{c}"))
                    .unwrap_or_default();
                last_tag = Some(format!("<{t}>{class}"));
            }
        }
        detail = last_tag.unwrap_or_default();
    }
    SyncConflictItem {
        file: i.file,
        id: i.id,
        area: area.into(),
        name,
        detail,
        ours: i.ours,
        theirs: i.theirs,
    }
}

impl Core {
    /// Auteur des versions faites par Revel (à défaut de celui réglé dans Git).
    fn author(&self) -> (String, String) {
        // Nom réglé dans Revel (travail à plusieurs, D-89), sinon compte GitHub.
        let set = self.settings().author.filter(|a| !a.trim().is_empty());
        match (set, self.github_account()) {
            (Some(name), Some(login)) => (
                name.trim().to_string(),
                format!("{login}@users.noreply.github.com"),
            ),
            (Some(name), None) => {
                let local: String = name
                    .to_lowercase()
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .filter(|w| !w.is_empty())
                    .collect::<Vec<_>>()
                    .join(".");
                let local = if local.is_empty() {
                    "auteur".into()
                } else {
                    local
                };
                (name.trim().to_string(), format!("{local}@revel.local"))
            }
            (None, Some(login)) => (login.clone(), format!("{login}@users.noreply.github.com")),
            (None, None) => ("Revel".into(), "revel@localhost".into()),
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
        Ok(sync_info(r, path))
    }

    /// Règle les endroits modifiés des deux côtés un par un (`items` : `fichier#chemin` → garder
    /// la version locale), les autres selon `keep_ours`, puis envoie (D-89).
    pub fn versions_resolve_items(
        &self,
        path: &Path,
        keep_ours: bool,
        items: &[(String, bool)],
        message: &str,
    ) -> Result<SyncInfo> {
        let token = self.remote_token(path)?;
        let (n, e) = self.author();
        let map: std::collections::HashMap<String, bool> = items.iter().cloned().collect();
        let r = v::resolve_items(path, token.as_deref(), keep_ours, &map, &n, &e, message)?;
        self.edit_reload(path)?;
        Ok(sync_info(r, path))
    }

    /// Dépôt central dans un dossier partagé (NAS, réseau local) : `<dossier>/<site>.git` créé
    /// s'il n'existe pas, puis choisi comme dépôt distant du site (D-89). Renvoie son chemin.
    pub fn versions_set_shared(&self, path: &Path, folder: &Path) -> Result<String> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "site".into());
        // Dossier choisi : le dépôt central lui-même, ou le dossier où le créer.
        let central = if folder.extension().is_some_and(|e| e == "git") || v::is_bare(folder) {
            folder.to_path_buf()
        } else {
            folder.join(format!("{name}.git"))
        };
        let branch = v::status(path)?.branch;
        v::init_shared(&central, &branch)?;
        let url = central.display().to_string();
        v::set_remote(path, &url)?;
        Ok(url)
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
        Ok(sync_info(r, path))
    }
}
