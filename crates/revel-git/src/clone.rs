//! Copie d'un dépôt distant dans un dossier (import depuis Git, 7.2, D-80) : libgit2, HTTPS
//! (jeton GitHub pour un dépôt privé), historique complet gardé.

use crate::GitError;
use crate::publish::{callbacks, proxy};
use git2::FetchOptions;
use git2::build::RepoBuilder;
use std::path::Path;

/// Adresse d'un dépôt donnée par l'utilisateur, ramenée à une adresse HTTPS clonable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoUrl {
    /// Adresse clonable (`https://github.com/compte/depot.git`).
    pub url: String,
    /// Nom du dépôt (`depot`), pour le nom du dossier.
    pub name: String,
    /// Branche nommée dans l'adresse (`/tree/<branche>/…`).
    pub branch: Option<String>,
    /// Sous-dossier nommé dans l'adresse (`/tree/<branche>/docs`).
    pub subdir: Option<String>,
    /// Compte et dépôt sur github.com.
    pub github: Option<(String, String)>,
}

/// Lit une adresse de dépôt : `https://github.com/compte/depot`, `github.com/compte/depot`,
/// `git@github.com:compte/depot.git`, adresse d'une branche ou d'un dossier sur GitHub
/// (`…/tree/main/docs`), tout dépôt HTTPS (`https://gitlab.com/…/depot.git`) ou `file://` (essais).
pub fn parse_url(input: &str) -> Option<RepoUrl> {
    let s = input.trim().trim_end_matches('/');
    if s.is_empty() || s.contains(char::is_whitespace) {
        return None;
    }
    // Forme SSH de GitHub : jointe en HTTPS (pas de SSH dans Revel).
    let s = match s.strip_prefix("git@github.com:") {
        Some(rest) => format!("https://github.com/{rest}"),
        None => s.to_string(),
    };
    let s = if s.starts_with("github.com/") {
        format!("https://{s}")
    } else {
        s
    };
    if let Some(path) = s.strip_prefix("file://") {
        let name = repo_name(path)?;
        return Some(RepoUrl {
            url: s.clone(),
            name,
            branch: None,
            subdir: None,
            github: None,
        });
    }
    let rest = s.strip_prefix("https://")?;
    let (host, path) = rest.split_once('/')?;
    if host.is_empty() || path.is_empty() {
        return None;
    }
    if host.eq_ignore_ascii_case("github.com") || host.eq_ignore_ascii_case("www.github.com") {
        let segs: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let [owner, repo, more @ ..] = segs.as_slice() else {
            return None;
        };
        let repo = repo.trim_end_matches(".git");
        if owner.is_empty() || repo.is_empty() {
            return None;
        }
        let (branch, subdir) = match more {
            ["tree", branch, sub @ ..] => (
                Some((*branch).to_string()),
                (!sub.is_empty()).then(|| sub.join("/")),
            ),
            _ => (None, None),
        };
        return Some(RepoUrl {
            url: format!("https://github.com/{owner}/{repo}.git"),
            name: repo.to_string(),
            branch,
            subdir,
            github: Some((owner.to_string(), repo.to_string())),
        });
    }
    Some(RepoUrl {
        url: s.clone(),
        name: repo_name(path)?,
        branch: None,
        subdir: None,
        github: None,
    })
}

fn repo_name(path: &str) -> Option<String> {
    let last = path.trim_end_matches('/').rsplit('/').next()?;
    let name = last.trim_end_matches(".git");
    (!name.is_empty()).then(|| name.to_string())
}

/// Avancement de la copie : objets reçus sur le total, octets reçus.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    pub received: usize,
    pub total: usize,
    pub bytes: usize,
}

/// Copie le dépôt `repo` dans `dest` (dossier absent ou vide). `progress` renvoie `false` pour
/// interrompre.
pub fn clone(
    repo: &RepoUrl,
    dest: &Path,
    token: Option<&str>,
    progress: &mut dyn FnMut(Progress) -> bool,
) -> Result<(), GitError> {
    let mut cb = callbacks(token);
    cb.transfer_progress(|s| {
        progress(Progress {
            received: s.received_objects(),
            total: s.total_objects(),
            bytes: s.received_bytes(),
        })
    });
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    fo.proxy_options(proxy());
    let mut b = RepoBuilder::new();
    b.fetch_options(fo);
    if let Some(branch) = &repo.branch {
        b.branch(branch);
    }
    b.clone(&repo.url, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresses_reconnues() {
        let g = parse_url("https://github.com/deltea/p3r-pause-menu").unwrap();
        assert_eq!(g.url, "https://github.com/deltea/p3r-pause-menu.git");
        assert_eq!(g.name, "p3r-pause-menu");
        assert_eq!(g.github, Some(("deltea".into(), "p3r-pause-menu".into())));
        let t = parse_url("github.com/compte/site/tree/main/docs/").unwrap();
        assert_eq!(t.url, "https://github.com/compte/site.git");
        assert_eq!(t.branch.as_deref(), Some("main"));
        assert_eq!(t.subdir.as_deref(), Some("docs"));
        let ssh = parse_url("git@github.com:compte/site.git").unwrap();
        assert_eq!(ssh.url, "https://github.com/compte/site.git");
        let gl = parse_url("https://gitlab.com/groupe/sous/projet.git").unwrap();
        assert_eq!(gl.url, "https://gitlab.com/groupe/sous/projet.git");
        assert_eq!(gl.name, "projet");
        assert!(gl.github.is_none());
        assert!(parse_url("http://exemple.com/depot.git").is_none());
        assert!(parse_url("pas une adresse").is_none());
        assert!(parse_url("https://github.com/seul").is_none());
    }

    /// Dépôt local avec une version, copié par son adresse `file://` (jamais de dépôt en ligne
    /// dans les essais).
    #[test]
    fn depot_copie_avec_son_historique() {
        let src = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("index.html"), "<p>Bonjour</p>").unwrap();
        crate::versions::init(
            src.path(),
            "Essai",
            "essai@example.invalid",
            "Première version",
        )
        .unwrap();
        // `file:///tmp/…` sous Unix, `file:///C:/…` sous Windows.
        let p = src.path().display().to_string().replace('\\', "/");
        let url = format!("file://{}{p}", if p.starts_with('/') { "" } else { "/" });
        let repo = parse_url(&url).unwrap();
        let dest = tempfile::tempdir().unwrap();
        let target = dest.path().join("copie");
        let mut calls = 0;
        clone(&repo, &target, None, &mut |_| {
            calls += 1;
            true
        })
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("index.html")).unwrap(),
            "<p>Bonjour</p>"
        );
        let history = crate::versions::history(&target, 10).unwrap();
        assert_eq!(history.len(), 1);
        assert!(calls > 0);
        // Interruption demandée : copie arrêtée.
        let stopped = dest.path().join("arretee");
        assert!(clone(&repo, &stopped, None, &mut |_| false).is_err());
    }
}
