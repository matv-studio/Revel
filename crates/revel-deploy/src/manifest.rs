//! Manifeste de publication : empreinte SHA-1 de chaque fichier publié.
//!
//! Format identique à `sha1sum` (`<empreinte>  <chemin>`, trié par chemin, octet par octet),
//! le même que le script `deploy-sftp.sh` du site Alunites : un serveur déjà publié par ce
//! script est repris tel quel (seuls les fichiers modifiés sont renvoyés).

use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::path::Path;

/// Chemin relatif (séparateur `/`) -> empreinte hexadécimale.
pub type Manifest = BTreeMap<String, String>;

/// Fichier du manifeste sur le serveur, relatif au dossier personnel :
/// `www/` -> `.deploy/www.manifest`, `a/b/` -> `.deploy/a_b.manifest`.
pub fn remote_path(remote_dir: &str) -> String {
    format!(
        ".deploy/{}.manifest",
        normalize_dir(remote_dir).replace('/', "_")
    )
}

/// `./www/` -> `www`.
pub fn normalize_dir(d: &str) -> String {
    d.trim_start_matches("./").trim_end_matches('/').to_string()
}

/// Empreintes de tous les fichiers de `dir`.
pub fn compute(dir: &Path) -> std::io::Result<Manifest> {
    let mut out = Manifest::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d)? {
            let e = e?;
            let p = e.path();
            if e.file_type()?.is_dir() {
                stack.push(p);
                continue;
            }
            let rel = p
                .strip_prefix(dir)
                .unwrap_or(&p)
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            out.insert(rel, sha1_file(&p)?);
        }
    }
    Ok(out)
}

pub fn sha1_file(p: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(p)?;
    let mut h = Sha1::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn parse(text: &str) -> Manifest {
    text.lines()
        .filter_map(|l| {
            let (hash, path) = l.split_once("  ")?;
            (hash.len() == 40).then(|| (path.to_string(), hash.to_string()))
        })
        .collect()
}

pub fn format(m: &Manifest) -> String {
    // BTreeMap<String> : ordre octet par octet, comme `sort` avec LC_ALL=C.
    m.iter().map(|(p, h)| format!("{h}  {p}\n")).collect()
}

/// Fichiers à envoyer (nouveaux ou modifiés) et à supprimer (publiés puis retirés).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Plan {
    pub upload: Vec<String>,
    pub delete: Vec<String>,
}

pub fn diff(old: &Manifest, new: &Manifest) -> Plan {
    Plan {
        upload: new
            .iter()
            .filter(|(p, h)| old.get(*p) != Some(*h))
            .map(|(p, _)| p.clone())
            .collect(),
        delete: old
            .keys()
            .filter(|p| !new.contains_key(*p))
            .cloned()
            .collect(),
    }
}

/// Manifeste provisoire écrit avant l'envoi : l'ancien, sans les fichiers à renvoyer.
/// Si l'envoi est interrompu, ils seront renvoyés la fois suivante.
pub fn stable(old: &Manifest, plan: &Plan) -> Manifest {
    old.iter()
        .filter(|(p, _)| !plan.upload.contains(p))
        .map(|(p, h)| (p.clone(), h.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chemins_distants() {
        assert_eq!(remote_path("www/"), ".deploy/www.manifest");
        assert_eq!(
            remote_path("./recette-v4k8/"),
            ".deploy/recette-v4k8.manifest"
        );
        assert_eq!(remote_path("a/b"), ".deploy/a_b.manifest");
    }

    #[test]
    fn format_sha1sum_et_diff() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a")).unwrap();
        std::fs::write(dir.path().join("a/x.txt"), "abc").unwrap();
        std::fs::write(dir.path().join("b.txt"), "").unwrap();
        let m = compute(dir.path()).unwrap();
        let text = format(&m);
        // Empreintes connues de « abc » et du fichier vide.
        assert_eq!(
            text,
            "a9993e364706816aba3e25717850c26c9cd0d89d  a/x.txt\nda39a3ee5e6b4b0d3255bfef95601890afd80709  b.txt\n"
        );
        assert_eq!(parse(&text), m);
        let mut old = m.clone();
        old.insert("old.txt".into(), "0".repeat(40));
        old.insert("b.txt".into(), "1".repeat(40));
        let plan = diff(&old, &m);
        assert_eq!(plan.upload, vec!["b.txt"]);
        assert_eq!(plan.delete, vec!["old.txt"]);
        let s = stable(&old, &plan);
        assert!(!s.contains_key("b.txt") && s.contains_key("old.txt"));
    }
}
