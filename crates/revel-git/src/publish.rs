//! Publication d'un dossier (site généré) sur une branche d'un dépôt distant.
//!
//! Fonctionnement :
//! 1. un dépôt local « nu » sert de cache (`cache_dir`) : seule la branche publiée y est récupérée,
//!    de façon incrémentale ;
//! 2. l'arbre du dossier est construit directement dans la base d'objets (sans copie de travail) ;
//!    les fichiers déjà connus ne sont ni recompressés ni réécrits ;
//! 3. si l'arbre est identique à celui de la branche, rien n'est envoyé ;
//! 4. sinon un commit est créé au-dessus de la branche distante, puis envoyé (avance rapide
//!    uniquement : si la branche a changé entre-temps, l'envoi est refusé, rien n'est écrasé).

use crate::GitError;
use git2::{
    Cred, FetchOptions, ObjectType, Oid, ProxyOptions, PushOptions, RemoteCallbacks, Repository,
    Signature,
};
use std::cell::RefCell;
use std::path::Path;

/// Fichiers ajoutés à la racine de la branche publiée.
#[derive(Debug, Clone, Default)]
pub struct Extras {
    /// Fichier `.nojekyll` vide : GitHub Pages sert tel quel les fichiers et dossiers
    /// commençant par `_` (sinon ignorés par Jekyll).
    pub nojekyll: bool,
    /// Contenu du fichier `CNAME` (domaine personnalisé GitHub Pages).
    pub cname: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Target<'a> {
    /// Adresse du dépôt (`https://github.com/proprietaire/depot.git`, ou chemin local pour les essais).
    pub url: &'a str,
    pub branch: &'a str,
    /// Jeton d'accès (HTTPS) ; `None` pour un dépôt local.
    pub token: Option<&'a str>,
    /// Auteur du commit.
    pub author_name: &'a str,
    pub author_email: &'a str,
    pub message: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Récupération de la branche distante (objets reçus / attendus).
    Fetching { done: usize, total: usize },
    /// Lecture des fichiers du site.
    Preparing { done: usize, total: usize },
    /// Envoi (objets envoyés / à envoyer, octets).
    Pushing {
        done: usize,
        total: usize,
        bytes: usize,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Commit publié (ou commit existant si rien n'a changé).
    pub commit: String,
    pub added: usize,
    pub modified: usize,
    pub deleted: usize,
    pub total_files: usize,
    /// Taille des fichiers ajoutés ou modifiés (octets).
    pub bytes: u64,
    /// La branche n'existait pas encore.
    pub created: bool,
    /// Rien n'a changé : aucun envoi.
    pub up_to_date: bool,
    /// Simulation : rien n'a été envoyé.
    pub dry_run: bool,
}

fn callbacks<'a>(token: Option<&'a str>) -> RemoteCallbacks<'a> {
    let mut cb = RemoteCallbacks::new();
    let mut tried = false;
    cb.credentials(move |_url, _user, _allowed| {
        // libgit2 rappelle cette fonction tant que l'authentification échoue : un seul essai.
        match token {
            Some(t) if !tried => {
                tried = true;
                Cred::userpass_plaintext("x-access-token", t)
            }
            _ => Err(git2::Error::from_str("authentification refusée")),
        }
    });
    cb
}

fn proxy<'a>() -> ProxyOptions<'a> {
    let mut p = ProxyOptions::new();
    p.auto();
    p
}

/// Ouvre (ou crée) le dépôt cache.
fn cache_repo(cache_dir: &Path) -> Result<Repository, GitError> {
    if cache_dir.join("HEAD").is_file() {
        Ok(Repository::open_bare(cache_dir)?)
    } else {
        std::fs::create_dir_all(cache_dir)?;
        Ok(Repository::init_bare(cache_dir)?)
    }
}

/// Récupère la branche distante ; `None` si elle n'existe pas encore.
fn fetch_branch(
    repo: &Repository,
    target: &Target<'_>,
    on_event: &dyn Fn(Event),
) -> Result<Option<Oid>, GitError> {
    let tracking = format!("refs/revel/remote/{}", target.branch);
    let mut remote = repo.remote_anonymous(target.url)?;
    let mut cb = callbacks(target.token);
    cb.transfer_progress(|p| {
        on_event(Event::Fetching {
            done: p.received_objects(),
            total: p.total_objects(),
        });
        true
    });
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    fo.proxy_options(proxy());
    // Les étiquettes du dépôt ne sont pas utiles ici.
    fo.download_tags(git2::AutotagOption::None);
    // Ne pas supprimer la référence locale si la branche distante n'existe pas : on la retire
    // explicitement d'abord, pour savoir si elle a été récupérée.
    if let Ok(mut r) = repo.find_reference(&tracking) {
        r.delete()?;
    }
    let refspec = format!("+refs/heads/{0}:{tracking}", target.branch);
    remote.connect_auth(
        git2::Direction::Fetch,
        Some(callbacks(target.token)),
        Some(proxy()),
    )?;
    let exists = remote
        .list()?
        .iter()
        .any(|h| h.name() == format!("refs/heads/{}", target.branch));
    remote.disconnect()?;
    if !exists {
        return Ok(None);
    }
    remote.fetch(&[refspec.as_str()], Some(&mut fo), None)?;
    Ok(repo.find_reference(&tracking).ok().and_then(|r| r.target()))
}

/// Nombre de fichiers d'un dossier (pour la progression).
fn count_files(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| {
                    let p = e.path();
                    if p.is_dir() { count_files(&p) } else { 1 }
                })
                .sum()
        })
        .unwrap_or(0)
}

struct Builder<'r> {
    repo: &'r Repository,
    done: RefCell<usize>,
    total: usize,
}

impl Builder<'_> {
    /// Arbre Git d'un dossier ; les objets déjà présents ne sont pas réécrits.
    fn tree(&self, dir: &Path, on_event: &dyn Fn(Event)) -> Result<Oid, GitError> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        let mut tb = self.repo.treebuilder(None)?;
        let odb = self.repo.odb()?;
        for e in entries {
            let name = e.file_name();
            let Some(name) = name.to_str() else {
                return Err(GitError::FileName(e.path().display().to_string()));
            };
            if name == ".git" {
                continue;
            }
            let path = e.path();
            let ft = e.file_type()?;
            if ft.is_dir() {
                let oid = self.tree(&path, on_event)?;
                // Un dossier vide n'existe pas en Git.
                if self.repo.find_tree(oid)?.is_empty() {
                    continue;
                }
                tb.insert(name, oid, 0o040000)?;
            } else if ft.is_file() || ft.is_symlink() {
                let oid = Oid::hash_file(ObjectType::Blob, &path)?;
                if !odb.exists(oid) {
                    self.repo.blob_path(&path)?;
                }
                tb.insert(name, oid, 0o100644)?;
                let mut d = self.done.borrow_mut();
                *d += 1;
                if (*d).is_multiple_of(50) || *d == self.total {
                    on_event(Event::Preparing {
                        done: *d,
                        total: self.total,
                    });
                }
            }
        }
        Ok(tb.write()?)
    }
}

/// Ajoute `.nojekyll` et `CNAME` à la racine de l'arbre.
fn with_extras(repo: &Repository, root: Oid, extras: &Extras) -> Result<Oid, GitError> {
    let tree = repo.find_tree(root)?;
    let mut tb = repo.treebuilder(Some(&tree))?;
    if extras.nojekyll {
        tb.insert(".nojekyll", repo.blob(b"")?, 0o100644)?;
    }
    if let Some(c) = extras
        .cname
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
    {
        tb.insert("CNAME", repo.blob(format!("{c}\n").as_bytes())?, 0o100644)?;
    }
    Ok(tb.write()?)
}

/// Publie `dir` sur la branche `target.branch` de `target.url`.
pub fn publish_dir(
    dir: &Path,
    cache_dir: &Path,
    target: &Target<'_>,
    extras: &Extras,
    dry_run: bool,
    on_event: &dyn Fn(Event),
) -> Result<Report, GitError> {
    if !dir.is_dir() {
        return Err(GitError::MissingDir(dir.display().to_string()));
    }
    let repo = cache_repo(cache_dir)?;
    let parent = fetch_branch(&repo, target, on_event)?;
    let total = count_files(dir);
    let b = Builder {
        repo: &repo,
        done: RefCell::new(0),
        total,
    };
    let tree_id = with_extras(&repo, b.tree(dir, on_event)?, extras)?;
    let tree = repo.find_tree(tree_id)?;
    let parent_commit = parent.map(|p| repo.find_commit(p)).transpose()?;
    let parent_tree = parent_commit.as_ref().map(|c| c.tree()).transpose()?;

    let mut report = Report {
        created: parent.is_none(),
        dry_run,
        ..Default::default()
    };
    // Différences avec la branche publiée.
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
    for d in diff.deltas() {
        match d.status() {
            git2::Delta::Added => report.added += 1,
            git2::Delta::Deleted => report.deleted += 1,
            _ => report.modified += 1,
        }
        if d.status() != git2::Delta::Deleted
            && let Ok(blob) = repo.find_blob(d.new_file().id())
        {
            report.bytes += blob.size() as u64;
        }
    }
    let mut files = 0;
    tree.walk(git2::TreeWalkMode::PreOrder, |_, e| {
        if e.kind() == Some(ObjectType::Blob) {
            files += 1;
        }
        git2::TreeWalkResult::Ok
    })?;
    report.total_files = files;

    if parent_tree.as_ref().is_some_and(|t| t.id() == tree_id) {
        report.up_to_date = true;
        report.commit = parent.map(|p| p.to_string()).unwrap_or_default();
        return Ok(report);
    }
    if dry_run {
        return Ok(report);
    }

    let sig = Signature::now(target.author_name, target.author_email)?;
    let parents: Vec<&git2::Commit<'_>> = parent_commit.iter().collect();
    let commit = repo.commit(None, &sig, &sig, target.message, &tree, &parents)?;
    let local = format!("refs/revel/publish/{}", target.branch);
    repo.reference(&local, commit, true, "revel: publication")?;

    let rejected: RefCell<Option<String>> = RefCell::new(None);
    {
        let mut remote = repo.remote_anonymous(target.url)?;
        let mut cb = callbacks(target.token);
        cb.push_transfer_progress(|done, total, bytes| {
            on_event(Event::Pushing { done, total, bytes })
        });
        cb.push_update_reference(|_name, status| {
            if let Some(s) = status {
                *rejected.borrow_mut() = Some(s.to_string());
            }
            Ok(())
        });
        let mut po = PushOptions::new();
        po.remote_callbacks(cb);
        po.proxy_options(proxy());
        // Avance rapide uniquement (pas de « + ») : jamais d'écrasement de la branche distante.
        let spec = format!("{local}:refs/heads/{}", target.branch);
        remote.push(&[spec.as_str()], Some(&mut po))?;
    }
    if let Some(reason) = rejected.into_inner() {
        return Err(GitError::Rejected(reason));
    }
    report.commit = commit.to_string();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(url: &str) -> Target<'_> {
        Target {
            url,
            branch: "gh-pages",
            token: None,
            author_name: "Revel",
            author_email: "revel@example.invalid",
            message: "Publication",
        }
    }

    #[test]
    fn publication_incrementale_sur_une_branche() {
        let tmp = tempfile::tempdir().unwrap();
        let remote = tmp.path().join("distant.git");
        Repository::init_bare(&remote).unwrap();
        let url = remote.to_str().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(site.join("assets/_partiel")).unwrap();
        std::fs::write(site.join("index.html"), "<h1>Accueil</h1>").unwrap();
        std::fs::write(site.join("assets/_partiel/a.css"), "a{}").unwrap();
        std::fs::create_dir_all(site.join("vide")).unwrap();
        let cache = tmp.path().join("cache");
        let extras = Extras {
            nojekyll: true,
            cname: Some("www.exemple.fr".into()),
        };
        let none = |_: Event| {};

        // Simulation : rien n'est envoyé.
        let r = publish_dir(&site, &cache, &target(url), &extras, true, &none).unwrap();
        assert!(r.created && r.dry_run);
        assert_eq!(r.added, 4, "{r:?}"); // index.html, a.css, .nojekyll, CNAME
        let distant = Repository::open_bare(&remote).unwrap();
        assert!(distant.find_reference("refs/heads/gh-pages").is_err());

        // Première publication : branche créée.
        let r = publish_dir(&site, &cache, &target(url), &extras, false, &none).unwrap();
        assert!(r.created && !r.up_to_date);
        assert_eq!((r.added, r.total_files), (4, 4));
        let head = distant.find_reference("refs/heads/gh-pages").unwrap();
        let tree = head.peel_to_commit().unwrap().tree().unwrap();
        assert!(tree.get_name(".nojekyll").is_some());
        assert!(tree.get_name("vide").is_none(), "dossier vide ignoré");
        let cname = tree.get_name("CNAME").unwrap().to_object(&distant).unwrap();
        assert_eq!(cname.as_blob().unwrap().content(), b"www.exemple.fr\n");

        // Sans changement : rien n'est envoyé.
        let r = publish_dir(&site, &cache, &target(url), &extras, false, &none).unwrap();
        assert!(r.up_to_date && !r.created);

        // Un fichier modifié, un supprimé, un ajouté ; cache local effacé (reprise depuis le distant).
        std::fs::write(site.join("index.html"), "<h1>Accueil 2</h1>").unwrap();
        std::fs::remove_file(site.join("assets/_partiel/a.css")).unwrap();
        std::fs::write(site.join("404.html"), "introuvable").unwrap();
        std::fs::remove_dir_all(&cache).unwrap();
        let r = publish_dir(&site, &cache, &target(url), &extras, false, &none).unwrap();
        assert_eq!((r.added, r.modified, r.deleted), (1, 1, 1), "{r:?}");
        let c = distant
            .find_reference("refs/heads/gh-pages")
            .unwrap()
            .peel_to_commit()
            .unwrap();
        assert_eq!(c.parent_count(), 1, "historique conservé");
    }

    #[test]
    fn cache_en_retard_repris_depuis_le_distant() {
        let tmp = tempfile::tempdir().unwrap();
        let remote = tmp.path().join("distant.git");
        Repository::init_bare(&remote).unwrap();
        let url = remote.to_str().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(&site).unwrap();
        std::fs::write(site.join("index.html"), "1").unwrap();
        let none = |_: Event| {};
        let extras = Extras::default();
        publish_dir(
            &site,
            &tmp.path().join("a"),
            &target(url),
            &extras,
            false,
            &none,
        )
        .unwrap();
        // Un autre poste publie entre-temps.
        std::fs::write(site.join("index.html"), "2").unwrap();
        publish_dir(
            &site,
            &tmp.path().join("b"),
            &target(url),
            &extras,
            false,
            &none,
        )
        .unwrap();
        // Le premier cache est en retard, mais la branche est récupérée avant chaque envoi :
        // la publication repart du commit distant (avance rapide).
        std::fs::write(site.join("index.html"), "3").unwrap();
        let r = publish_dir(
            &site,
            &tmp.path().join("a"),
            &target(url),
            &extras,
            false,
            &none,
        )
        .unwrap();
        assert_eq!(r.modified, 1);
    }

    /// Récupération réelle en HTTPS (réseau requis) : `cargo test -p revel-git -- --ignored`.
    #[test]
    #[ignore]
    fn recuperation_https_github() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(&site).unwrap();
        std::fs::write(site.join("index.html"), "x").unwrap();
        let t = Target {
            url: "https://github.com/octocat/Hello-World.git",
            branch: "master",
            ..target("")
        };
        let r = publish_dir(
            &site,
            &tmp.path().join("c"),
            &t,
            &Extras::default(),
            true,
            &|_| {},
        )
        .unwrap();
        assert!(!r.created && r.dry_run, "{r:?}");
        assert!(r.deleted >= 1, "{r:?}"); // README de la branche distante
    }
}
