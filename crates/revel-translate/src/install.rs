//! Téléchargement d'une langue du [`catalog`](crate::catalog) : fichiers d'origine téléchargés
//! depuis Hugging Face aux révisions figées, empreintes SHA-256 vérifiées, puis préparés
//! ([`prepare`](crate::prepare)) dans le dossier des modèles. Une seule fois : la traduction se
//! fait ensuite sans connexion.
//!
//! Fichiers d'origine gardés dans `<dossier>/.telechargement/` jusqu'à la fin (une reprise ne
//! retélécharge que ce qui manque), puis retirés.

use crate::TranslateError;
use crate::catalog::{Pack, RemoteFile, Source};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Avancement d'une installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// Octets téléchargés sur le total de la langue.
    Download { done: u64, total: u64 },
    /// Préparation des modèles (quelques secondes).
    Prepare,
}

/// Langue installée dans `root` (ses deux modèles prêts).
pub fn installed(pack: &Pack, root: &Path) -> bool {
    pack.models
        .iter()
        .all(|m| root.join(m.name).join("model.safetensors").is_file())
}

/// Retire une langue de `root`.
pub fn remove(pack: &Pack, root: &Path) -> Result<(), TranslateError> {
    for m in &pack.models {
        let d = root.join(m.name);
        if d.exists() {
            std::fs::remove_dir_all(&d).map_err(|e| io(&d, e))?;
        }
    }
    Ok(())
}

fn io(p: &Path, e: std::io::Error) -> TranslateError {
    TranslateError::Download(format!("{} : {e}", p.display()))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent(concat!("Revel/", env!("CARGO_PKG_VERSION")))
        .timeout_connect(Some(Duration::from_secs(30)))
        .timeout_recv_response(Some(Duration::from_secs(60)))
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

/// Adresse d'un fichier d'origine.
pub fn url(source: &Source, file: &RemoteFile) -> String {
    format!(
        "https://huggingface.co/Helsinki-NLP/{}/resolve/{}/{}",
        source.name, source.rev, file.name
    )
}

fn sha256_of(path: &Path) -> Option<String> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut h = Sha256::new();
    std::io::copy(&mut f, &mut h).ok()?;
    Some(hex(&h.finalize()))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Télécharge un fichier dans `dest` (empreinte vérifiée).
fn fetch(
    agent: &ureq::Agent,
    source: &Source,
    file: &RemoteFile,
    dest: &Path,
    cancel: &AtomicBool,
    on: &mut dyn FnMut(u64),
) -> Result<(), TranslateError> {
    let part = dest.with_extension("part");
    let resp = agent
        .get(&url(source, file))
        .call()
        .map_err(|e| TranslateError::Download(e.to_string()))?;
    let mut reader = resp.into_body().into_reader();
    let mut out = std::fs::File::create(&part).map_err(|e| io(&part, e))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut got = 0u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            drop(out);
            let _ = std::fs::remove_file(&part);
            return Err(TranslateError::Cancelled);
        }
        let n = reader
            .read(&mut buf)
            .map_err(|e| TranslateError::Download(e.to_string()))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        out.write_all(&buf[..n]).map_err(|e| io(&part, e))?;
        got += n as u64;
        on(n as u64);
    }
    out.flush().map_err(|e| io(&part, e))?;
    drop(out);
    let sum = hex(&h.finalize());
    if got != file.size || sum != file.sha256 {
        let _ = std::fs::remove_file(&part);
        return Err(TranslateError::Integrity(format!(
            "{}/{} ({got} octets, {sum})",
            source.name, file.name
        )));
    }
    std::fs::rename(&part, dest).map_err(|e| io(dest, e))
}

/// Télécharge et prépare une langue dans `root`. `cancel` : arrêt demandé (vérifié pendant le
/// téléchargement).
pub fn install(
    pack: &Pack,
    root: &Path,
    cancel: &AtomicBool,
    on: &mut dyn FnMut(Progress),
) -> Result<(), TranslateError> {
    let work: PathBuf = root.join(".telechargement").join(pack.code);
    let total = pack.download_size();
    let mut done = 0u64;
    let agent = agent();
    for m in &pack.models {
        let dir = work.join(m.name);
        std::fs::create_dir_all(&dir).map_err(|e| io(&dir, e))?;
        for f in m.files {
            let dest = dir.join(f.name);
            // Fichier déjà téléchargé et intact (reprise) : gardé.
            let ok = std::fs::metadata(&dest).is_ok_and(|md| md.len() == f.size)
                && sha256_of(&dest).as_deref() == Some(f.sha256);
            if !ok {
                fetch(&agent, m, f, &dest, cancel, &mut |n| {
                    done += n;
                    on(Progress::Download { done, total });
                })?;
            } else {
                done += f.size;
                on(Progress::Download { done, total });
            }
        }
    }
    on(Progress::Prepare);
    for m in &pack.models {
        let ready = root.join(m.name);
        let tmp = root.join(format!(".prepare-{}", m.name));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp).map_err(|e| io(&tmp, e))?;
        }
        let source = format!(
            "https://huggingface.co/Helsinki-NLP/{}/tree/{}",
            m.name, m.rev
        );
        crate::prepare::prepare(&work.join(m.name), &tmp, &source)?;
        if ready.exists() {
            std::fs::remove_dir_all(&ready).map_err(|e| io(&ready, e))?;
        }
        std::fs::rename(&tmp, &ready).map_err(|e| io(&ready, e))?;
    }
    let _ = std::fs::remove_dir_all(&work);
    let _ = std::fs::remove_dir(root.join(".telechargement"));
    Ok(())
}
