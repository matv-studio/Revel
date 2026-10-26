//! Sites Revel du compte GitHub sur plusieurs ordinateurs (D-127) : dépôts du compte qui
//! contiennent un site (`site.revel.json` à la racine), copie présente sur cet ordinateur
//! retrouvée par l'adresse de son dépôt distant, téléchargement dans le dossier des sites, mise
//! à jour de toutes les copies d'un coup.

use crate::{Core, Result};
use revel_git::github::{self, Client};
use revel_git::versions as v;
use revel_protocol::{GithubSite, GithubSyncReport};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Nombre de dépôts examinés au plus (les plus récemment modifiés d'abord).
const MAX_REPOS: usize = 300;
/// Racines de dépôt lues en même temps.
const PARALLEL: usize = 8;

impl Core {
    /// Sites Revel des dépôts GitHub du compte connecté, du plus récemment modifié au plus
    /// ancien, avec le dossier de leur copie sur cet ordinateur (sites connus de Revel et
    /// sous-dossiers de `projects`, le dossier des nouveaux sites).
    pub fn github_sites(&self, projects: Option<&Path>) -> Result<Vec<GithubSite>> {
        let token = self.github_token()?;
        let client = Client::new(&token.access_token);
        let repos = client.list_repos(MAX_REPOS)?;
        let mut is_site = vec![false; repos.len()];
        std::thread::scope(|sc| {
            for (chunk, flags) in repos.chunks(PARALLEL).zip(is_site.chunks_mut(PARALLEL)) {
                let handles: Vec<_> = chunk
                    .iter()
                    .map(|r| {
                        let c = &client;
                        sc.spawn(move || {
                            c.root_names(&r.full_name)
                                .is_ok_and(|n| n.iter().any(|f| f == revel_project::SITE_FILE))
                        })
                    })
                    .collect();
                for (h, f) in handles.into_iter().zip(flags.iter_mut()) {
                    *f = h.join().unwrap_or(false);
                }
            }
        });
        let local = self.local_github_copies(projects);
        Ok(repos
            .into_iter()
            .zip(is_site)
            .filter(|(_, s)| *s)
            .map(|(r, _)| GithubSite {
                path: local
                    .get(&r.full_name.to_ascii_lowercase())
                    .map(|p| p.display().to_string()),
                repo: r.full_name,
                name: r.name,
                private: r.private,
                clone_url: r.clone_url,
                html_url: r.html_url,
                description: r.description.filter(|d| !d.trim().is_empty()),
                pushed_at: r.pushed_at,
                size_kb: r.size,
            })
            .collect())
    }

    /// Copies sur cet ordinateur des dépôts GitHub (`proprietaire/nom` en minuscules → dossier).
    pub(crate) fn local_github_copies(&self, projects: Option<&Path>) -> BTreeMap<String, PathBuf> {
        let mut dirs: Vec<PathBuf> = self
            .recent_sites()
            .into_iter()
            .map(|r| PathBuf::from(r.path))
            .collect();
        if let Some(p) = projects
            && let Ok(rd) = std::fs::read_dir(p)
        {
            for e in rd.flatten() {
                let d = e.path();
                if d.join(revel_project::SITE_FILE).is_file() && !dirs.contains(&d) {
                    dirs.push(d);
                }
            }
        }
        let mut out = BTreeMap::new();
        for d in dirs {
            if !v::is_repo(&d) {
                continue;
            }
            if let Some(repo) = v::status(&d)
                .ok()
                .and_then(|s| s.remote)
                .and_then(|u| github::repo_of_url(&u))
            {
                out.entry(repo).or_insert(d);
            }
        }
        out
    }

    /// Met à jour toutes les copies de sites GitHub de cet ordinateur : versions reçues (sans
    /// toucher aux changements en cours), versions enregistrées envoyées. Un site modifié des
    /// deux côtés au même endroit, ou dont les versions divergent pendant des changements en
    /// cours, est laissé tel quel (`waiting`) : à régler en l'ouvrant.
    pub fn github_sync_all(&self, projects: Option<&Path>, merge: &str) -> GithubSyncReport {
        let mut report = GithubSyncReport::default();
        for (_, dir) in self.local_github_copies(projects) {
            let name = revel_project::Project::open(&dir)
                .ok()
                .map(|p| p.site.name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| {
                    dir.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                });
            match self.versions_receive(&dir, merge) {
                Ok(r) => {
                    if r.waiting || !r.conflicts.is_empty() {
                        report.waiting.push(name.clone());
                    }
                    if r.pulled > 0 || r.merged {
                        report.updated.push(name.clone());
                    }
                    if r.pushed > 0 {
                        report.sent.push(name);
                    }
                }
                Err(e) => report.failed.push((name, e.to_string())),
            }
        }
        report
    }
}
