//! Publication en un clic (5.1) : destinations d'un site, date de la dernière publication, pages
//! modifiées depuis (empreintes de ce que la génération mettrait en ligne, comparées à celles de
//! la dernière publication).

use crate::state::PublishRecord;
use crate::{Core, Result, secret_key};
use revel_project::{Project, Snapshot};
use revel_protocol::{
    ChangeKind, DeployEvent, HostProbe, HostingProfile, HostingTarget, PageChange,
    PublishDestination,
};
use std::collections::BTreeMap;
use std::path::Path;

/// Pages modifiées entre la dernière publication et l'état actuel ; titres pris dans `titles`.
pub(crate) fn changes(
    published: &Snapshot,
    now: &Snapshot,
    titles: &BTreeMap<String, String>,
) -> Vec<PageChange> {
    let title = |url: &str| titles.get(url).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for (url, h) in &now.pages {
        match published.pages.get(url) {
            None => out.push(PageChange {
                url: url.clone(),
                title: title(url),
                kind: ChangeKind::Added,
            }),
            Some(old) if old != h => out.push(PageChange {
                url: url.clone(),
                title: title(url),
                kind: ChangeKind::Changed,
            }),
            Some(_) => {}
        }
    }
    for url in published.pages.keys() {
        if !now.pages.contains_key(url) {
            out.push(PageChange {
                url: url.clone(),
                title: title(url),
                kind: ChangeKind::Removed,
            });
        }
    }
    if published.assets != now.assets {
        out.push(PageChange {
            url: String::new(),
            title: String::new(),
            kind: ChangeKind::Files,
        });
    }
    out
}

/// Dossier du site proposé : celui du domaine du site, sinon le dossier habituel de l'hébergeur,
/// sinon le premier trouvé.
pub(crate) fn suggest_folder(
    folders: &[String],
    preset_root: Option<&str>,
    domain: Option<&str>,
) -> Option<String> {
    if let Some(d) = domain
        && let Some(f) = folders.iter().find(|f| f.split('/').any(|part| part == d))
    {
        return Some(f.clone());
    }
    if let Some(r) = preset_root.filter(|r| !r.is_empty())
        && folders.iter().any(|f| f == r)
    {
        return Some(r.to_string());
    }
    folders.first().cloned()
}

impl Core {
    /// Empreinte de l'état actuel du site pour un environnement, gardée tant que le site ne change
    /// pas (révision de la session d'édition et fichiers lus par le canevas).
    pub(crate) fn snapshot(&self, site: &Path, environment: &str) -> Result<Snapshot> {
        let stamp = self
            .edit_stamp(site)
            .ok()
            .map(|s| format!("{}:{}", s.revision, s.files));
        let key = (site.display().to_string(), environment.to_string());
        if let Some(stamp) = &stamp
            && let Some((s, snap)) = self
                .snapshots
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&key)
            && s == stamp
        {
            return Ok(snap.clone());
        }
        let snap = Project::open(site)?.publish_snapshot(environment)?;
        if let Some(stamp) = stamp {
            self.snapshots
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(key, (stamp, snap.clone()));
        }
        Ok(snap)
    }

    /// Adresse du site publié par un hébergement : réglée, sinon donnée par la dernière
    /// publication, sinon l'adresse publique du site pour la production.
    pub(crate) fn destination_url(
        &self,
        site: &Path,
        profile: &HostingProfile,
        record: Option<&PublishRecord>,
    ) -> Option<String> {
        profile
            .site_url
            .clone()
            .filter(|u| !u.trim().is_empty())
            .or_else(|| record.and_then(|r| r.url.clone()))
            .or_else(|| {
                (profile.environment == "production")
                    .then(|| Project::open(site).ok()?.site.base_url)
                    .flatten()
            })
    }

    /// Destinations du site, avec la dernière publication et les pages modifiées depuis.
    pub fn publish_state(&self, site: &Path) -> Result<Vec<PublishDestination>> {
        let key = site.display().to_string();
        let profiles = self.profiles(&key);
        let records = self.with_state(|s| s.published.get(&key).cloned().unwrap_or_default());
        let titles: BTreeMap<String, String> = Project::open(site)?
            .list_pages()?
            .into_iter()
            .map(|p| {
                let t = p.name.filter(|n| !n.trim().is_empty()).unwrap_or(p.title);
                (p.url, t)
            })
            .collect();
        let mut out = Vec::new();
        for p in profiles {
            let record = records.get(&p.id);
            let changes = match record {
                Some(r) => changes(&r.snapshot, &self.snapshot(site, &p.environment)?, &titles),
                None => Vec::new(),
            };
            let ready = match &p.target {
                HostingTarget::Sftp { .. } => secret_key(&p.target)
                    .and_then(|k| self.secrets.get(&k).ok().flatten())
                    .is_some(),
                HostingTarget::GithubPages { .. } => self.has_secret(&p.target),
            };
            let kept = crate::rollback::versions(site, &p.id);
            let previous_at = (kept.len() >= 2).then(|| kept[kept.len() - 2].at);
            out.push(PublishDestination {
                url: self.destination_url(site, &p, record),
                published_at: record.map(|r| r.at),
                previous_at,
                changes,
                ready,
                profile: p.id,
                name: p.name,
                environment: p.environment,
            });
        }
        Ok(out)
    }

    /// Connexion d'essai à un hébergement SFTP (5.3), avant de l'enregistrer : accès vérifiés,
    /// clé du serveur, dossiers où peut aller le site et dossier proposé. `password` : saisi
    /// (sinon celui du trousseau). Rien n'est écrit sur le serveur.
    pub async fn hosting_probe(
        &self,
        site: &Path,
        profile: &HostingProfile,
        password: Option<String>,
    ) -> Result<HostProbe> {
        let HostingTarget::Sftp {
            host,
            port,
            user,
            remote_dir,
            known_host_key,
        } = &profile.target
        else {
            return Err(crate::CoreError::Other("SFTP attendu".into()));
        };
        let password = match password.filter(|p| !p.is_empty()) {
            Some(p) => p,
            None => {
                let key = secret_key(&profile.target).unwrap_or_default();
                self.secrets
                    .get(&key)
                    .map_err(crate::CoreError::Secret)?
                    .ok_or(crate::CoreError::MissingSecret(key))?
            }
        };
        let target = revel_deploy::sftp::SftpTarget {
            host: host.trim().to_string(),
            port: *port,
            user: user.trim().to_string(),
            password,
            remote_dir: remote_dir.clone(),
            known_host_key: known_host_key.clone(),
        };
        let probe = revel_deploy::sftp::probe(&target).await?;
        let domain = Project::open(site)
            .ok()
            .and_then(|p| p.site.base_url)
            .map(|u| {
                let u = u
                    .split_once("://")
                    .map(|(_, r)| r)
                    .unwrap_or(&u)
                    .to_string();
                let h = u.split('/').next().unwrap_or_default().to_ascii_lowercase();
                h.strip_prefix("www.").unwrap_or(&h).to_string()
            })
            .filter(|d| !d.is_empty());
        let suggested = suggest_folder(
            &probe.folders,
            profile
                .preset
                .as_deref()
                .and_then(revel_protocol::hosting::preset)
                .map(|p| p.web_root),
            domain.as_deref(),
        );
        Ok(HostProbe {
            server_key: probe.server_key,
            fingerprint: probe.fingerprint,
            folders: probe.folders,
            suggested,
        })
    }

    /// Garde ce qui vient d'être publié (revenir en arrière, 5.5) ; un échec n'empêche pas la
    /// publication, qui a eu lieu.
    pub(crate) fn keep_version(
        &self,
        site: &Path,
        profile: &HostingProfile,
        out: &Path,
        snapshot: &Snapshot,
    ) {
        let at = chrono::Utc::now().timestamp_millis();
        let r = tokio::task::block_in_place(|| {
            crate::rollback::record(
                site,
                &profile.id,
                &profile.environment,
                out,
                snapshot.clone(),
                at,
            )
        });
        if let Err(e) = r {
            eprintln!("revel : publication non gardée ({}) : {e}", profile.id);
        }
    }

    /// Republie la version précédente d'un hébergement (5.5) ; la version remplacée devient la
    /// précédente (un second retour l'annule).
    pub async fn rollback(
        &self,
        site: &Path,
        profile_id: &str,
        on_event: &(dyn Fn(DeployEvent) + Send + Sync),
    ) -> Result<revel_protocol::DeploySummary> {
        let profile = self
            .profiles(&site.display().to_string())
            .into_iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| crate::CoreError::UnknownProfile(profile_id.to_string()))?;
        let restored =
            tokio::task::block_in_place(|| crate::rollback::restore_previous(site, profile_id))?;
        let Some((dir, prev)) = restored else {
            let e = crate::CoreError::NoPrevious;
            on_event(DeployEvent::Failed(e.failure()));
            return Err(e);
        };
        let r = self
            .upload(
                site,
                &profile,
                &dir,
                false,
                false,
                Some(prev.snapshot),
                on_event,
            )
            .await;
        let _ = std::fs::remove_dir_all(&dir);
        r
    }

    /// Publication réussie : date, adresse et empreinte de ce qui a été mis en ligne.
    pub(crate) fn record_publish(
        &self,
        site: &Path,
        profile: &str,
        url: Option<String>,
        snapshot: Snapshot,
    ) {
        let key = site.display().to_string();
        self.with_state(|s| {
            s.published.entry(key).or_default().insert(
                profile.to_string(),
                PublishRecord {
                    at: chrono::Utc::now().timestamp_millis(),
                    url,
                    snapshot,
                },
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_modifiees() {
        let snap = |pages: &[(&str, &str)], assets: &str| Snapshot {
            pages: pages
                .iter()
                .map(|(u, h)| (u.to_string(), h.to_string()))
                .collect(),
            assets: assets.into(),
        };
        let old = snap(&[("/", "a"), ("/agence/", "b"), ("/ancienne/", "c")], "x");
        let new = snap(&[("/", "a"), ("/agence/", "B"), ("/nouvelle/", "d")], "y");
        let titles = BTreeMap::from([("/agence/".to_string(), "Agence".to_string())]);
        let c = changes(&old, &new, &titles);
        let kinds: Vec<(&str, ChangeKind)> = c.iter().map(|c| (c.url.as_str(), c.kind)).collect();
        assert_eq!(
            kinds,
            [
                ("/agence/", ChangeKind::Changed),
                ("/nouvelle/", ChangeKind::Added),
                ("/ancienne/", ChangeKind::Removed),
                ("", ChangeKind::Files),
            ]
        );
        assert_eq!(c[0].title, "Agence");
        assert!(changes(&old, &old, &titles).is_empty());
    }

    #[test]
    fn dossier_propose() {
        let f = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let hostinger = f(&[
            "domains/alunites.com/public_html/",
            "domains/autre.fr/public_html/",
            "public_html/",
        ]);
        assert_eq!(
            suggest_folder(&hostinger, Some("public_html/"), Some("alunites.com")).as_deref(),
            Some("domains/alunites.com/public_html/")
        );
        assert_eq!(
            suggest_folder(&f(&["public_html/", "www/"]), Some("www/"), None).as_deref(),
            Some("www/")
        );
        assert_eq!(
            suggest_folder(&f(&["sites/a.fr/"]), Some(""), Some("b.fr")).as_deref(),
            Some("sites/a.fr/")
        );
        assert_eq!(suggest_folder(&[], Some("www/"), None), None);
    }
}
