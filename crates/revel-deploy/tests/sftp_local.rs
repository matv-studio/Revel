//! Essai réel contre un serveur SFTP (ignoré sans configuration).
//!
//! Variables : `REVEL_SFTP_HOST`, `REVEL_SFTP_PORT`, `REVEL_SFTP_USER`, `REVEL_SFTP_PASSWORD`,
//! et `REVEL_SFTP_HOME` (dossier personnel de l'utilisateur, lisible localement, pour vérifier).

#![cfg(feature = "sftp")]

use revel_deploy::sftp::{Event, Options, SftpError, SftpTarget, deploy};
use std::path::PathBuf;

fn target() -> Option<(SftpTarget, PathBuf)> {
    let get = |k: &str| std::env::var(k).ok();
    Some((
        SftpTarget {
            host: get("REVEL_SFTP_HOST")?,
            port: get("REVEL_SFTP_PORT")?.parse().ok()?,
            user: get("REVEL_SFTP_USER")?,
            password: get("REVEL_SFTP_PASSWORD")?,
            remote_dir: "revel-essai/".into(),
            known_host_key: None,
        },
        PathBuf::from(get("REVEL_SFTP_HOME")?),
    ))
}

#[tokio::test(flavor = "multi_thread")]
async fn publication_incrementale() {
    let Some((mut t, home)) = target() else {
        eprintln!("REVEL_SFTP_* absent : essai ignoré");
        return;
    };
    let _ = std::fs::remove_dir_all(home.join("revel-essai"));
    let _ = std::fs::remove_file(home.join(".deploy/revel-essai.manifest"));
    let site = tempfile::tempdir().unwrap();
    let w = |p: &str, c: &[u8]| {
        let f = site.path().join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, c).unwrap();
    };
    w("a.txt", b"A");
    w("dossier/b.txt", b"B");
    let big: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
    w("img/grand.bin", &big);
    let events = std::sync::Mutex::new(Vec::new());
    let on = |e: Event| events.lock().unwrap().push(e);

    // 1. Premier envoi : tout part, clé reçue.
    let r = deploy(site.path(), &t, &Options::default(), &on)
        .await
        .unwrap();
    assert_eq!(r.plan.upload.len(), 3);
    assert!(r.server_key.contains("ssh-"), "{}", r.server_key);
    assert!(r.server_fingerprint.starts_with("SHA256:"));
    assert_eq!(
        std::fs::read(home.join("revel-essai/img/grand.bin")).unwrap(),
        big
    );
    let manifest = std::fs::read_to_string(home.join(".deploy/revel-essai.manifest")).unwrap();
    assert_eq!(manifest.lines().count(), 3);

    // 2. Modification, ajout, suppression ; clé désormais vérifiée.
    t.known_host_key = Some(r.server_key.clone());
    w("dossier/b.txt", b"B2");
    w("c.txt", b"C");
    std::fs::remove_file(site.path().join("a.txt")).unwrap();
    let r2 = deploy(site.path(), &t, &Options::default(), &on)
        .await
        .unwrap();
    assert_eq!(r2.plan.upload, vec!["c.txt", "dossier/b.txt"]);
    assert_eq!(r2.plan.delete, vec!["a.txt"]);
    assert_eq!(
        std::fs::read(home.join("revel-essai/dossier/b.txt")).unwrap(),
        b"B2"
    );
    assert!(!home.join("revel-essai/a.txt").exists());
    assert!(!home.join("revel-essai/dossier/.in.b.txt").exists());

    // 3. Rien à faire.
    let r3 = deploy(site.path(), &t, &Options::default(), &on)
        .await
        .unwrap();
    assert!(r3.plan.upload.is_empty() && r3.plan.delete.is_empty());

    // 4. Mauvaise clé : refus.
    t.known_host_key = Some(
        "hote ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIMnT0w0R7n0qVZ3WlCwuBQqUqbXHqzKqX6sT8y6mT2G"
            .into(),
    );
    let e = deploy(site.path(), &t, &Options::default(), &on)
        .await
        .unwrap_err();
    assert!(matches!(e, SftpError::HostKeyMismatch(_)), "{e}");

    // 5. Mauvais mot de passe.
    t.known_host_key = None;
    t.password = "faux".into();
    let e = deploy(site.path(), &t, &Options::default(), &on)
        .await
        .unwrap_err();
    assert!(matches!(e, SftpError::Auth), "{e}");
}

/// Connexion d'essai : dossiers du site trouvés (domaines, dossiers habituels, lien symbolique),
/// rien écrit ; serveur injoignable : délai dépassé.
#[tokio::test(flavor = "multi_thread")]
async fn connexion_d_essai() {
    use revel_deploy::sftp::probe;
    let Some((t, home)) = target() else {
        eprintln!("REVEL_SFTP_* absent : essai ignoré");
        return;
    };
    let p = probe(&t).await.unwrap();
    assert!(p.server_key.contains("ssh-"), "{}", p.server_key);
    assert!(p.fingerprint.starts_with("SHA256:"));
    // Dossiers attendus seulement s'ils existent chez l'utilisateur d'essai.
    for (dir, rel) in [
        (
            "domains/alunites.com/public_html",
            "domains/alunites.com/public_html/",
        ),
        ("www", "www/"),
        ("public_html", "public_html/"),
        ("html", "html/"),
    ] {
        assert_eq!(
            home.join(dir).is_dir(),
            p.folders.iter().any(|f| f == rel),
            "{rel} : {:?}",
            p.folders
        );
    }
    let first_domain = p.folders.iter().position(|f| f.starts_with("domains/"));
    let first_root = p
        .folders
        .iter()
        .position(|f| !f.contains('/') || f.matches('/').count() == 1);
    if let (Some(a), Some(b)) = (first_domain, first_root) {
        assert!(a < b, "{:?}", p.folders);
    }
    assert!(!home.join(".deploy/probe").exists());

    // Mauvais mot de passe.
    let mut bad = t.clone();
    bad.password = "faux".into();
    assert!(matches!(probe(&bad).await.unwrap_err(), SftpError::Auth));

    // Adresse non routable (pare-feu qui ignore les paquets) : délai dépassé.
    let mut lost = t.clone();
    lost.host = "10.255.255.1".into();
    let started = std::time::Instant::now();
    let e = probe(&lost).await.unwrap_err();
    assert!(
        matches!(e, SftpError::Timeout(_) | SftpError::Connect(..)),
        "{e}"
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
}
