//! Versions d'un site (3.10, D-25) : le dossier du site est un dépôt Git ordinaire. Une version
//! est un enregistrement (commit) de tous ses fichiers ; on compare une version au site actuel,
//! fichier par fichier, et on la restaure sans réécrire l'historique (nouvelle version qui
//! reprend ses fichiers, l'état précédent étant enregistré d'abord).
//!
//! Synchronisation avec le dépôt distant (`origin`, GitHub) : récupérer les versions faites
//! ailleurs (Pages CMS), en avance rapide ou par fusion sans conflit, puis envoyer celles du site
//! (avance rapide seulement : rien n'est écrasé en ligne).

use crate::GitError;
use crate::publish::{callbacks, proxy};
use git2::{
    BranchType, Delta, DiffOptions, FetchOptions, IndexAddOption, Oid, PushOptions, Repository,
    Signature, Sort, StatusOptions,
};
use std::path::Path;

/// Lignes ajoutées au `.gitignore` d'un site dont les versions sont activées par Revel.
pub const IGNORE: &str = ".revel/\n_site/\nnode_modules/\n.DS_Store\nThumbs.db\n";

/// État du dépôt d'un site.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Status {
    /// Branche courante (`main`).
    pub branch: String,
    /// Dernière version, `None` pour un dépôt sans version.
    pub head: Option<String>,
    /// Fichiers modifiés, ajoutés ou supprimés depuis la dernière version.
    pub changes: usize,
    /// Adresse du dépôt distant (`origin`).
    pub remote: Option<String>,
    /// Versions locales pas encore envoyées, versions en ligne pas encore récupérées (selon la
    /// dernière récupération).
    pub ahead: usize,
    pub behind: usize,
}

/// Version (enregistrement) du site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub id: String,
    pub message: String,
    pub author: String,
    pub email: String,
    /// Secondes depuis 1970.
    pub time: i64,
    /// Fichiers changés par rapport à la version précédente.
    pub files: usize,
    /// Version récupérée du dépôt distant mais faite ailleurs (Pages CMS, autre poste) : son
    /// auteur n'est pas celui des versions locales.
    pub merge: bool,
}

/// Nature du changement d'un fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// Fichier changé entre une version et le site actuel (ou la version précédente).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub change: Change,
}

/// Ligne d'une comparaison : `+` ajoutée, `-` retirée, ` ` inchangée (contexte).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: char,
    pub old: Option<u32>,
    pub new: Option<u32>,
    pub text: String,
}

/// Résultat d'une synchronisation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Versions récupérées du dépôt distant.
    pub pulled: usize,
    /// Versions envoyées.
    pub pushed: usize,
    /// Fusion nécessaire (versions faites des deux côtés), réussie.
    pub merged: bool,
    /// Fichiers en conflit : rien n'a été changé, à régler (garder l'une des deux versions).
    pub conflicts: Vec<String>,
}

/// Le dossier est un dépôt Git (racine).
pub fn is_repo(dir: &Path) -> bool {
    Repository::open(dir).is_ok_and(|r| {
        r.workdir()
            .is_some_and(|w| w.canonicalize().ok() == dir.canonicalize().ok())
    })
}

fn open(dir: &Path) -> Result<Repository, GitError> {
    Ok(Repository::open(dir)?)
}

/// Auteur des versions : celui réglé pour le dépôt, sinon celui donné.
fn signature<'a>(repo: &Repository, name: &str, email: &str) -> Result<Signature<'a>, GitError> {
    match repo.signature() {
        Ok(s) => Ok(s.to_owned()),
        Err(_) => Ok(Signature::now(name, email)?),
    }
}

fn head_commit(repo: &Repository) -> Option<git2::Commit<'_>> {
    repo.head().ok()?.peel_to_commit().ok()
}

fn branch_name(repo: &Repository) -> String {
    repo.head()
        .ok()
        .and_then(|h| h.shorthand().ok().map(str::to_string))
        .or_else(|| {
            // Dépôt sans version : branche indiquée par HEAD.
            repo.find_reference("HEAD")
                .ok()
                .and_then(|r| r.symbolic_target().ok().flatten().map(str::to_string))
                .map(|t| t.trim_start_matches("refs/heads/").to_string())
        })
        .unwrap_or_else(|| "main".into())
}

/// Fichiers changés depuis la dernière version (non ignorés).
fn change_count(repo: &Repository) -> Result<usize, GitError> {
    let mut o = StatusOptions::new();
    o.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);
    Ok(repo
        .statuses(Some(&mut o))?
        .iter()
        .filter(|s| !s.status().is_empty() && !s.status().is_ignored())
        .count())
}

/// État du dépôt d'un site.
pub fn status(dir: &Path) -> Result<Status, GitError> {
    let repo = open(dir)?;
    let branch = branch_name(&repo);
    let head = head_commit(&repo).map(|c| c.id().to_string());
    let remote = repo
        .find_remote("origin")
        .ok()
        .and_then(|r| r.url().ok().map(str::to_string));
    let (mut ahead, mut behind) = (0, 0);
    if let (Some(h), Ok(up)) = (
        head_commit(&repo).map(|c| c.id()),
        repo.find_reference(&format!("refs/remotes/origin/{branch}")),
    ) && let Some(u) = up.target()
    {
        (ahead, behind) = repo.graph_ahead_behind(h, u)?;
    }
    Ok(Status {
        branch,
        head,
        changes: change_count(&repo)?,
        remote,
        ahead,
        behind,
    })
}

/// Active les versions d'un site : dépôt créé, fichiers de travail de Revel ignorés, première
/// version enregistrée.
pub fn init(dir: &Path, name: &str, email: &str, message: &str) -> Result<Version, GitError> {
    if is_repo(dir) {
        return Err(GitError::Rejected("le site a déjà des versions".into()));
    }
    let mut o = git2::RepositoryInitOptions::new();
    o.initial_head("main");
    Repository::init_opts(dir, &o)?;
    let gi = dir.join(".gitignore");
    let old = std::fs::read_to_string(&gi).unwrap_or_default();
    let missing: String = IGNORE
        .lines()
        .filter(|l| !old.lines().any(|o| o.trim() == *l))
        .map(|l| format!("{l}\n"))
        .collect();
    if !missing.is_empty() {
        let sep = if old.is_empty() || old.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        std::fs::write(&gi, format!("{old}{sep}{missing}"))?;
    }
    save(dir, name, email, message)?
        .ok_or_else(|| GitError::Rejected("aucun fichier à enregistrer".into()))
}

/// Dossier de travail de Revel (`.revel/` : caches d'images, corbeille) ignoré par le dépôt :
/// ajouté au `.gitignore` s'il n'y est pas.
fn ensure_ignored(dir: &Path) -> Result<(), GitError> {
    let gi = dir.join(".gitignore");
    let old = std::fs::read_to_string(&gi).unwrap_or_default();
    if old
        .lines()
        .any(|l| matches!(l.trim(), ".revel" | ".revel/" | "/.revel" | "/.revel/"))
    {
        return Ok(());
    }
    let sep = if old.is_empty() || old.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    std::fs::write(&gi, format!("{old}{sep}.revel/\n"))?;
    Ok(())
}

/// Enregistre une version de tous les fichiers du site ; `None` : rien n'a changé.
pub fn save(
    dir: &Path,
    name: &str,
    email: &str,
    message: &str,
) -> Result<Option<Version>, GitError> {
    ensure_ignored(dir)?;
    let repo = open(dir)?;
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.update_all(["*"].iter(), None)?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let parent = head_commit(&repo);
    if parent.as_ref().is_some_and(|p| p.tree_id() == tree_id) {
        return Ok(None);
    }
    let tree = repo.find_tree(tree_id)?;
    let sig = signature(&repo, name, email)?;
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    let id = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;
    version(&repo, id).map(Some)
}

fn version(repo: &Repository, id: Oid) -> Result<Version, GitError> {
    let c = repo.find_commit(id)?;
    let tree = c.tree()?;
    let parent_tree = c.parent(0).ok().and_then(|p| p.tree().ok());
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
    let a = c.author();
    Ok(Version {
        id: id.to_string(),
        message: c.message().unwrap_or_default().trim().to_string(),
        author: a.name().unwrap_or_default().to_string(),
        email: a.email().unwrap_or_default().to_string(),
        time: c.time().seconds(),
        files: diff.deltas().len(),
        merge: c.parent_count() > 1,
    })
}

/// Versions du site, de la plus récente à la plus ancienne (au plus `limit`).
pub fn history(dir: &Path, limit: usize) -> Result<Vec<Version>, GitError> {
    let repo = open(dir)?;
    let Some(head) = head_commit(&repo) else {
        return Ok(Vec::new());
    };
    let mut walk = repo.revwalk()?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    walk.push(head.id())?;
    walk.take(limit).map(|id| version(&repo, id?)).collect()
}

fn change_of(d: Delta) -> Option<Change> {
    match d {
        Delta::Added | Delta::Untracked | Delta::Copied => Some(Change::Added),
        Delta::Modified | Delta::Typechange => Some(Change::Modified),
        Delta::Deleted => Some(Change::Deleted),
        Delta::Renamed => Some(Change::Renamed),
        _ => None,
    }
}

fn workdir_diff<'r>(
    repo: &'r Repository,
    id: Option<&str>,
    path: Option<&str>,
) -> Result<git2::Diff<'r>, GitError> {
    let tree = match id {
        Some(i) => Some(repo.find_commit(Oid::from_str(i)?)?.tree()?),
        None => head_commit(repo).map(|c| c.tree()).transpose()?,
    };
    let mut o = DiffOptions::new();
    o.include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
    if let Some(p) = path {
        o.pathspec(p).disable_pathspec_match(true);
    }
    Ok(repo.diff_tree_to_workdir_with_index(tree.as_ref(), Some(&mut o))?)
}

/// Fichiers qui diffèrent entre une version (`None` : la dernière) et le site actuel.
pub fn changes(dir: &Path, id: Option<&str>) -> Result<Vec<FileChange>, GitError> {
    let repo = open(dir)?;
    let diff = workdir_diff(&repo, id, None)?;
    let mut out: Vec<FileChange> = diff
        .deltas()
        .filter_map(|d| {
            let change = change_of(d.status())?;
            let file = if change == Change::Deleted {
                d.old_file()
            } else {
                d.new_file()
            };
            Some(FileChange {
                path: file.path()?.to_string_lossy().replace('\\', "/"),
                change,
            })
        })
        .collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Fichiers changés par une version (par rapport à la précédente).
pub fn version_changes(dir: &Path, id: &str) -> Result<Vec<FileChange>, GitError> {
    let repo = open(dir)?;
    let c = repo.find_commit(Oid::from_str(id)?)?;
    let tree = c.tree()?;
    let parent_tree = c.parent(0).ok().and_then(|p| p.tree().ok());
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
    let mut out: Vec<FileChange> = diff
        .deltas()
        .filter_map(|d| {
            let change = change_of(d.status())?;
            let file = if change == Change::Deleted {
                d.old_file()
            } else {
                d.new_file()
            };
            Some(FileChange {
                path: file.path()?.to_string_lossy().replace('\\', "/"),
                change,
            })
        })
        .collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Comparaison d'un fichier entre une version et le site actuel, ligne par ligne (3 lignes de
/// contexte) ; `None` : fichier binaire.
pub fn file_diff(dir: &Path, id: Option<&str>, path: &str) -> Result<Option<Vec<Line>>, GitError> {
    let repo = open(dir)?;
    let diff = workdir_diff(&repo, id, Some(path))?;
    diff_lines(&diff)
}

/// Lignes d'une comparaison (fichier binaire : `None`).
fn diff_lines(diff: &git2::Diff<'_>) -> Result<Option<Vec<Line>>, GitError> {
    let mut lines = Vec::new();
    let mut binary = false;
    diff.foreach(
        &mut |_, _| true,
        Some(&mut |_, _| {
            binary = true;
            true
        }),
        None,
        Some(&mut |_, _, l| {
            let kind = l.origin();
            if matches!(kind, '+' | '-' | ' ') {
                lines.push(Line {
                    kind,
                    old: l.old_lineno(),
                    new: l.new_lineno(),
                    text: String::from_utf8_lossy(l.content())
                        .trim_end_matches(['\n', '\r'])
                        .to_string(),
                });
            }
            true
        }),
    )?;
    Ok((!binary).then_some(lines))
}

/// Ce qu'une version a changé dans un fichier (par rapport à la version précédente).
pub fn version_file_diff(dir: &Path, id: &str, path: &str) -> Result<Option<Vec<Line>>, GitError> {
    let repo = open(dir)?;
    let c = repo.find_commit(Oid::from_str(id)?)?;
    let tree = c.tree()?;
    let parent_tree = c.parent(0).ok().and_then(|p| p.tree().ok());
    let mut o = DiffOptions::new();
    o.pathspec(path).disable_pathspec_match(true);
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut o))?;
    diff_lines(&diff)
}

/// Dépôt distant (`origin`) ajouté ou changé ; vide : retiré.
pub fn set_remote(dir: &Path, url: &str) -> Result<(), GitError> {
    let repo = open(dir)?;
    let url = url.trim();
    if url.is_empty() {
        if repo.find_remote("origin").is_ok() {
            repo.remote_delete("origin")?;
        }
        return Ok(());
    }
    if repo.find_remote("origin").is_ok() {
        repo.remote_set_url("origin", url)?;
    } else {
        repo.remote("origin", url)?;
    }
    Ok(())
}

/// Restaure les fichiers d'une version : l'état actuel est d'abord enregistré (s'il a changé),
/// puis les fichiers de la version remis en place et enregistrés comme nouvelle version.
pub fn restore(
    dir: &Path,
    id: &str,
    name: &str,
    email: &str,
    before: &str,
    message: &str,
) -> Result<Option<Version>, GitError> {
    save(dir, name, email, before)?;
    let repo = open(dir)?;
    let target = repo.find_commit(Oid::from_str(id)?)?;
    let mut co = git2::build::CheckoutBuilder::new();
    co.force().remove_untracked(true);
    repo.checkout_tree(target.as_object(), Some(&mut co))?;
    // L'index suit les fichiers restaurés ; HEAD reste sur la dernière version.
    save(dir, name, email, message)
}

/// Branche distante récupérée (`refs/remotes/origin/<branche>`).
fn fetch(repo: &Repository, branch: &str, token: Option<&str>) -> Result<Option<Oid>, GitError> {
    let mut remote = repo.find_remote("origin")?;
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks(token));
    fo.proxy_options(proxy());
    fo.download_tags(git2::AutotagOption::None);
    let spec = format!("+refs/heads/{branch}:refs/remotes/origin/{branch}");
    remote.fetch(&[spec.as_str()], Some(&mut fo), None)?;
    Ok(repo
        .find_reference(&format!("refs/remotes/origin/{branch}"))
        .ok()
        .and_then(|r| r.target()))
}

/// Récupère les versions en ligne (`origin`) puis envoie celles du site. Les changements en
/// cours sont d'abord enregistrés (`message`). Conflit : rien n'est changé, fichiers indiqués.
pub fn sync(
    dir: &Path,
    token: Option<&str>,
    name: &str,
    email: &str,
    message: &str,
    merge_message: &str,
) -> Result<SyncReport, GitError> {
    save(dir, name, email, message)?;
    let repo = open(dir)?;
    let branch = branch_name(&repo);
    let mut report = SyncReport::default();
    let remote_id = fetch(&repo, &branch, token)?;
    if let Some(theirs) = remote_id {
        let ours = head_commit(&repo).map(|c| c.id());
        let annotated = repo.find_annotated_commit(theirs)?;
        let (analysis, _) = repo.merge_analysis(&[&annotated])?;
        if analysis.is_up_to_date() {
            // Rien à récupérer.
        } else if analysis.is_fast_forward() || ours.is_none() {
            report.pulled = match ours {
                Some(o) => repo.graph_ahead_behind(theirs, o)?.0,
                None => 1,
            };
            let target = repo.find_commit(theirs)?;
            let mut co = git2::build::CheckoutBuilder::new();
            co.force();
            repo.checkout_tree(target.as_object(), Some(&mut co))?;
            let refname = format!("refs/heads/{branch}");
            match repo.find_reference(&refname) {
                Ok(mut r) => {
                    r.set_target(theirs, "revel: récupération")?;
                }
                Err(_) => {
                    repo.reference(&refname, theirs, true, "revel: récupération")?;
                    repo.set_head(&refname)?;
                }
            }
        } else if analysis.is_normal() {
            let ours_c = repo.find_commit(ours.expect("version locale"))?;
            let theirs_c = repo.find_commit(theirs)?;
            report.pulled = repo.graph_ahead_behind(theirs, ours_c.id())?.0;
            let mut idx = repo.merge_commits(&ours_c, &theirs_c, None)?;
            if idx.has_conflicts() {
                report.conflicts = idx
                    .conflicts()?
                    .filter_map(|c| c.ok())
                    .filter_map(|c| c.our.or(c.their).or(c.ancestor))
                    .map(|e| String::from_utf8_lossy(&e.path).to_string())
                    .collect();
                report.conflicts.sort();
                report.conflicts.dedup();
                return Ok(report);
            }
            let tree = repo.find_tree(idx.write_tree_to(&repo)?)?;
            let sig = signature(&repo, name, email)?;
            let id = repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                merge_message,
                &tree,
                &[&ours_c, &theirs_c],
            )?;
            let mut co = git2::build::CheckoutBuilder::new();
            co.force();
            repo.checkout_tree(repo.find_commit(id)?.as_object(), Some(&mut co))?;
            report.merged = true;
        }
    }
    // Envoi : seulement si le site a des versions que le dépôt distant n'a pas.
    if let Some(h) = head_commit(&repo).map(|c| c.id()) {
        let ahead = match repo
            .find_reference(&format!("refs/remotes/origin/{branch}"))
            .ok()
            .and_then(|r| r.target())
        {
            Some(u) => repo.graph_ahead_behind(h, u)?.0,
            None => repo.revwalk().map(|mut w| {
                let _ = w.push(h);
                w.count()
            })?,
        };
        if ahead > 0 {
            let mut remote = repo.find_remote("origin")?;
            let rejected = std::cell::RefCell::new(None::<String>);
            {
                let mut cb = callbacks(token);
                cb.push_update_reference(|_, status| {
                    if let Some(s) = status {
                        *rejected.borrow_mut() = Some(s.to_string());
                    }
                    Ok(())
                });
                let mut po = PushOptions::new();
                po.remote_callbacks(cb);
                po.proxy_options(proxy());
                let spec = format!("refs/heads/{branch}:refs/heads/{branch}");
                remote.push(&[spec.as_str()], Some(&mut po))?;
            }
            if let Some(r) = rejected.into_inner() {
                return Err(GitError::Rejected(r));
            }
            repo.reference(
                &format!("refs/remotes/origin/{branch}"),
                h,
                true,
                "revel: envoi",
            )?;
            report.pushed = ahead;
        }
    }
    Ok(report)
}

/// Règle un conflit en gardant, pour les fichiers en conflit, la version du site (`ours`) ou
/// celle en ligne : la version en ligne est fusionnée, les fichiers choisis écrits, puis le tout
/// enregistré et envoyé.
pub fn resolve(
    dir: &Path,
    token: Option<&str>,
    keep_ours: bool,
    name: &str,
    email: &str,
    message: &str,
) -> Result<SyncReport, GitError> {
    {
        let repo = open(dir)?;
        let branch = branch_name(&repo);
        let theirs = repo
            .find_branch(&format!("origin/{branch}"), BranchType::Remote)?
            .get()
            .target()
            .ok_or_else(|| GitError::Rejected("version en ligne absente".into()))?;
        let ours_c =
            head_commit(&repo).ok_or_else(|| GitError::Rejected("aucune version".into()))?;
        let theirs_c = repo.find_commit(theirs)?;
        let mut mo = git2::MergeOptions::new();
        mo.file_favor(if keep_ours {
            git2::FileFavor::Ours
        } else {
            git2::FileFavor::Theirs
        });
        let mut idx = repo.merge_commits(&ours_c, &theirs_c, Some(&mo))?;
        if idx.has_conflicts() {
            // Conflits de structure (fichier supprimé d'un côté) : version choisie entière.
            let pick = if keep_ours { &ours_c } else { &theirs_c };
            let conflicted: Vec<Vec<u8>> = idx
                .conflicts()?
                .filter_map(|c| c.ok())
                .filter_map(|c| c.our.or(c.their).or(c.ancestor))
                .map(|e| e.path)
                .collect();
            let pick_tree = pick.tree()?;
            for p in conflicted {
                let path = String::from_utf8_lossy(&p).to_string();
                idx.conflict_remove(Path::new(&path))?;
                if let Ok(entry) = pick_tree.get_path(Path::new(&path)) {
                    let blob = repo.find_blob(entry.id())?;
                    idx.add_frombuffer(
                        &git2::IndexEntry {
                            ctime: git2::IndexTime::new(0, 0),
                            mtime: git2::IndexTime::new(0, 0),
                            dev: 0,
                            ino: 0,
                            mode: entry.filemode() as u32,
                            uid: 0,
                            gid: 0,
                            file_size: blob.size() as u32,
                            id: entry.id(),
                            flags: 0,
                            flags_extended: 0,
                            path: p.clone(),
                        },
                        blob.content(),
                    )?;
                }
            }
        }
        let tree = repo.find_tree(idx.write_tree_to(&repo)?)?;
        let sig = signature(&repo, name, email)?;
        let id = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &[&ours_c, &theirs_c],
        )?;
        let mut co = git2::build::CheckoutBuilder::new();
        co.force();
        repo.checkout_tree(repo.find_commit(id)?.as_object(), Some(&mut co))?;
    }
    sync(dir, token, name, email, message, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: &str = "Revel";
    const E: &str = "revel@example.org";

    fn write(dir: &Path, f: &str, t: &str) {
        let p = dir.join(f);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, t).unwrap();
    }

    #[test]
    fn versions_comparees_et_restaurees() {
        let d = tempfile::tempdir().unwrap();
        let site = d.path();
        write(site, "site.revel.json", "{\"name\": \"A\"}\n");
        write(site, "src/index.md", "Bonjour\nle monde\n");
        write(site, ".revel/cache.bin", "x");
        assert!(!is_repo(site));
        let v1 = init(site, N, E, "Première version").unwrap();
        assert!(is_repo(site));
        assert_eq!(v1.files, 3); // .gitignore, site.revel.json, src/index.md (.revel ignoré)
        assert!(
            std::fs::read_to_string(site.join(".gitignore"))
                .unwrap()
                .contains(".revel/")
        );
        assert_eq!(status(site).unwrap().changes, 0);
        // Rien de changé : pas de version.
        assert!(save(site, N, E, "rien").unwrap().is_none());
        write(site, "src/index.md", "Bonjour\nla terre\n");
        write(site, "src/nouveau.md", "Nouveau\n");
        assert_eq!(status(site).unwrap().changes, 2);
        let ch = changes(site, None).unwrap();
        assert_eq!(
            ch.iter()
                .map(|c| (c.path.as_str(), c.change))
                .collect::<Vec<_>>(),
            vec![
                ("src/index.md", Change::Modified),
                ("src/nouveau.md", Change::Added)
            ]
        );
        let lines = file_diff(site, None, "src/index.md").unwrap().unwrap();
        assert!(lines.iter().any(|l| l.kind == '-' && l.text == "le monde"));
        assert!(lines.iter().any(|l| l.kind == '+' && l.text == "la terre"));
        let v2 = save(site, N, E, "Deuxième").unwrap().unwrap();
        assert_eq!(v2.files, 2);
        let h = history(site, 10).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].message, "Deuxième");
        assert_eq!(version_changes(site, &v2.id).unwrap().len(), 2);
        let vd = version_file_diff(site, &v2.id, "src/nouveau.md")
            .unwrap()
            .unwrap();
        assert_eq!(vd.len(), 1);
        assert_eq!((vd[0].kind, vd[0].text.as_str()), ('+', "Nouveau"));
        // Restaurer la première : l'état actuel enregistré d'abord, puis nouvelle version.
        write(site, "src/index.md", "Brouillon\n");
        let r = restore(site, &v1.id, N, E, "Avant restauration", "Restauration")
            .unwrap()
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(site.join("src/index.md")).unwrap(),
            "Bonjour\nle monde\n"
        );
        assert!(!site.join("src/nouveau.md").exists());
        assert!(site.join(".revel/cache.bin").exists());
        let h = history(site, 10).unwrap();
        assert_eq!(h.len(), 4);
        assert_eq!(h[0].id, r.id);
        assert_eq!(h[1].message, "Avant restauration");
        assert_eq!(status(site).unwrap().changes, 0);
    }

    #[test]
    fn synchronisation_avec_le_depot_distant() {
        let d = tempfile::tempdir().unwrap();
        let bare = d.path().join("distant.git");
        // Branche par défaut du dépôt distant : `main`, comme sur GitHub.
        Repository::init_bare(&bare)
            .unwrap()
            .set_head("refs/heads/main")
            .unwrap();
        // Site : versions activées, dépôt distant ajouté, premier envoi.
        let site = d.path().join("site");
        write(&site, "a.md", "un\ndeux\ntrois\n");
        init(&site, N, E, "Début").unwrap();
        open(&site)
            .unwrap()
            .remote("origin", bare.to_str().unwrap())
            .unwrap();
        let r = sync(&site, None, N, E, "Modifications", "Fusion").unwrap();
        assert_eq!(r.pushed, 1);
        // Pages CMS : autre copie, modifie un autre fichier et envoie.
        let cms = d.path().join("cms");
        let repo = Repository::clone(bare.to_str().unwrap(), &cms).unwrap();
        write(&cms, "b.md", "Pages CMS\n");
        drop(repo);
        let r2 = sync(
            &cms,
            None,
            "Pages CMS",
            "cms@example.org",
            "Contenu",
            "Fusion",
        )
        .unwrap();
        assert_eq!(r2.pushed, 1);
        // Site : modification locale + récupération (fusion sans conflit) + envoi.
        write(&site, "a.md", "un\ndeux\ntrois\nquatre\n");
        let r3 = sync(&site, None, N, E, "Modifications", "Fusion").unwrap();
        assert_eq!((r3.pulled, r3.merged, r3.pushed), (1, true, 2));
        assert_eq!(
            std::fs::read_to_string(site.join("b.md")).unwrap(),
            "Pages CMS\n"
        );
        assert_eq!(status(&site).unwrap().changes, 0);
        // Conflit : même ligne modifiée des deux côtés ; rien n'est changé.
        write(&cms, "c.md", "");
        sync(
            &cms,
            None,
            "Pages CMS",
            "cms@example.org",
            "Contenu",
            "Fusion",
        )
        .unwrap();
        write(&cms, "a.md", "UN\ndeux\ntrois\nquatre\n");
        sync(
            &cms,
            None,
            "Pages CMS",
            "cms@example.org",
            "Contenu 2",
            "Fusion",
        )
        .unwrap();
        write(&site, "a.md", "1\ndeux\ntrois\nquatre\n");
        let r4 = sync(&site, None, N, E, "Modifications", "Fusion").unwrap();
        assert_eq!(r4.conflicts, vec!["a.md".to_string()]);
        assert_eq!(
            std::fs::read_to_string(site.join("a.md")).unwrap(),
            "1\ndeux\ntrois\nquatre\n"
        );
        // Garder la version du site : fusion enregistrée et envoyée.
        let r5 = resolve(&site, None, true, N, E, "Conflit réglé").unwrap();
        assert!(r5.conflicts.is_empty() && r5.pushed > 0);
        assert_eq!(
            std::fs::read_to_string(site.join("a.md")).unwrap(),
            "1\ndeux\ntrois\nquatre\n"
        );
        assert!(site.join("c.md").exists());
        let s = status(&site).unwrap();
        assert_eq!((s.ahead, s.behind, s.changes), (0, 0, 0));
    }
}
