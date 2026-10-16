//! Revenir à la publication précédente (5.5).
//!
//! Chaque publication réussie garde une copie de ce qui a été mis en ligne, dans le dossier du
//! site (`.revel/published`) : fichiers rangés par empreinte (BLAKE3, une seule copie d'un même
//! contenu d'une version à l'autre) et, pour chaque hébergement, les deux dernières versions
//! (chemin → empreinte). Revenir en arrière republie la version précédente avec le même envoi
//! qu'une publication (seuls les fichiers qui diffèrent partent) ; la version remplacée devient à
//! son tour la précédente, ce qui permet d'annuler le retour.

use revel_project::Snapshot;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// Versions gardées par hébergement (la publiée et la précédente).
const KEEP: usize = 2;

/// Version publiée sur un hébergement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Version {
    /// Date (millisecondes depuis 1970).
    pub at: i64,
    /// Environnement généré.
    pub environment: String,
    /// Empreinte des pages (pages modifiées depuis, 5.1).
    pub snapshot: Snapshot,
    /// Fichier publié (chemin relatif, `/`) → empreinte du contenu.
    pub files: BTreeMap<String, String>,
}

/// Dossier des publications gardées d'un site.
pub(crate) fn store(site: &Path) -> PathBuf {
    site.join(revel_project::CACHE_DIR).join("published")
}

fn object(store: &Path, hash: &str) -> PathBuf {
    store.join("objects").join(&hash[..2]).join(&hash[2..])
}

fn versions_file(store: &Path, profile: &str) -> PathBuf {
    let safe: String = profile
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    store.join(format!("{safe}.json"))
}

/// Versions gardées d'un hébergement, de la plus ancienne à la plus récente.
pub(crate) fn versions(site: &Path, profile: &str) -> Vec<Version> {
    std::fs::read(versions_file(&store(site), profile))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            walk(&p, out)?;
        } else {
            out.push(p);
        }
    }
    Ok(())
}

/// Garde ce qui vient d'être publié depuis `dir` : fichiers absents de la réserve copiés, version
/// ajoutée (les plus anciennes au-delà de deux retirées, avec les fichiers qu'elles seules
/// utilisaient).
pub(crate) fn record(
    site: &Path,
    profile: &str,
    environment: &str,
    dir: &Path,
    snapshot: Snapshot,
    at: i64,
) -> std::io::Result<()> {
    let store = store(site);
    let mut paths = Vec::new();
    walk(dir, &mut paths)?;
    let mut files = BTreeMap::new();
    for p in paths {
        let bytes = std::fs::read(&p)?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let obj = object(&store, &hash);
        if !obj.is_file() {
            std::fs::create_dir_all(obj.parent().unwrap_or(&store))?;
            // Écriture sous un nom provisoire puis renommage : jamais d'objet partiel.
            let tmp = obj.with_extension("tmp");
            std::fs::write(&tmp, &bytes)?;
            std::fs::rename(&tmp, &obj)?;
        }
        let rel = p.strip_prefix(dir).unwrap_or(&p);
        files.insert(rel.to_string_lossy().replace('\\', "/"), hash);
    }
    let mut list = versions(site, profile);
    list.push(Version {
        at,
        environment: environment.to_string(),
        snapshot,
        files,
    });
    if list.len() > KEEP {
        list.drain(..list.len() - KEEP);
    }
    let file = versions_file(&store, profile);
    let tmp = file.with_extension("json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec(&list).map_err(std::io::Error::other)?,
    )?;
    std::fs::rename(&tmp, &file)?;
    collect_garbage(&store)
}

/// Retire de la réserve les fichiers qu'aucune version gardée n'utilise.
fn collect_garbage(store: &Path) -> std::io::Result<()> {
    let mut used = HashSet::new();
    for e in std::fs::read_dir(store)?.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "json")
            && let Ok(b) = std::fs::read(&p)
            && let Ok(list) = serde_json::from_slice::<Vec<Version>>(&b)
        {
            for v in list {
                used.extend(v.files.into_values());
            }
        }
    }
    let objects = store.join("objects");
    let Ok(dirs) = std::fs::read_dir(&objects) else {
        return Ok(());
    };
    for d in dirs.flatten() {
        let prefix = d.file_name().to_string_lossy().to_string();
        let Ok(rd) = std::fs::read_dir(d.path()) else {
            continue;
        };
        for f in rd.flatten() {
            let hash = format!("{prefix}{}", f.file_name().to_string_lossy());
            if !used.contains(&hash) {
                let _ = std::fs::remove_file(f.path());
            }
        }
        let _ = std::fs::remove_dir(d.path());
    }
    Ok(())
}

/// Reconstitue la version précédente d'un hébergement dans un dossier provisoire (liens vers la
/// réserve, copie à défaut) ; `None` s'il n'y en a pas.
pub(crate) fn restore_previous(
    site: &Path,
    profile: &str,
) -> std::io::Result<Option<(PathBuf, Version)>> {
    let list = versions(site, profile);
    if list.len() < 2 {
        return Ok(None);
    }
    let prev = list[list.len() - 2].clone();
    let store = store(site);
    let dir = store.join(format!(
        "restore-{}",
        versions_file(&store, profile)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    for (rel, hash) in &prev.files {
        if rel.split('/').any(|s| s == ".." || s.is_empty()) {
            continue;
        }
        let to = dir.join(rel);
        std::fs::create_dir_all(to.parent().unwrap_or(&dir))?;
        let from = object(&store, hash);
        if std::fs::hard_link(&from, &to).is_err() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(Some((dir, prev)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_gardees_et_restauration() {
        let d = tempfile::tempdir().unwrap();
        let site = d.path().join("site");
        let out = d.path().join("out");
        let w = |p: &str, c: &str| {
            let f = out.join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, c).unwrap();
        };
        let snap = |s: &str| Snapshot {
            pages: BTreeMap::from([("/".to_string(), s.to_string())]),
            assets: String::new(),
        };
        w("index.html", "v1");
        w("img/a.jpg", "image");
        record(&site, "h1", "production", &out, snap("1"), 1).unwrap();
        assert!(restore_previous(&site, "h1").unwrap().is_none());
        w("index.html", "v2");
        std::fs::remove_file(out.join("img/a.jpg")).unwrap();
        w("b.html", "nouvelle");
        record(&site, "h1", "production", &out, snap("2"), 2).unwrap();
        let (dir, v) = restore_previous(&site, "h1").unwrap().unwrap();
        assert_eq!(v.at, 1);
        assert_eq!(v.snapshot, snap("1"));
        assert_eq!(
            std::fs::read_to_string(dir.join("index.html")).unwrap(),
            "v1"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("img/a.jpg")).unwrap(),
            "image"
        );
        assert!(!dir.join("b.html").exists());
        // Troisième version : la première est oubliée, ses fichiers retirés de la réserve.
        w("index.html", "v3");
        record(&site, "h1", "production", &out, snap("3"), 3).unwrap();
        let objects = |site: &Path| {
            let mut n = 0;
            for d in std::fs::read_dir(store(site).join("objects"))
                .unwrap()
                .flatten()
            {
                n += std::fs::read_dir(d.path()).unwrap().count();
            }
            n
        };
        // v2 (index v2, b.html) et v3 (index v3, b.html) : trois contenus.
        assert_eq!(objects(&site), 3);
        let (dir, v) = restore_previous(&site, "h1").unwrap().unwrap();
        assert_eq!(v.at, 2);
        assert_eq!(
            std::fs::read_to_string(dir.join("index.html")).unwrap(),
            "v2"
        );
        assert_eq!(versions(&site, "h1").len(), 2);
    }
}
