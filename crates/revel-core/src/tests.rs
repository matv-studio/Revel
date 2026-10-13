use super::*;
use revel_model::{Node, Page, build::*};

fn core(dir: &Path) -> Core {
    Core::new(&dir.join("config"), Box::new(secrets::Memory::default()))
}

fn sample_site(dir: &Path, c: &Core) -> PathBuf {
    let site = dir.join("site");
    c.new_site(&site, "Essai", "fr").unwrap();
    let mut p = revel_project::Project::open(&site).unwrap();
    p.site.pages.push(Page {
        id: "index".into(),
        path: "/".into(),
        layout: None,
        props: [("title".to_string(), serde_json::json!("Accueil"))].into(),
        computed: Default::default(),
        body: vec![
            Node::Html {
                html: "<!doctype html>".into(),
            },
            el("h1", &[], vec![out("title")]),
        ],
        draft: false,
    });
    p.save().unwrap();
    site
}

#[test]
fn recents_generation_et_hebergements() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    assert_eq!(c.recent_sites().len(), 1);
    let summary = c.open_site(&site).unwrap();
    assert_eq!(summary.pages, 1);
    assert_eq!(summary.environments, vec!["production"]);

    let events = Mutex::new(Vec::new());
    let b = c
        .build(&site, "production", true, &|e| {
            events.lock().unwrap().push(e)
        })
        .unwrap();
    assert_eq!(b.generated, 1);
    let html = std::fs::read_to_string(out_dir(&site, "production").join("index.html")).unwrap();
    assert!(html.contains("<h1>Accueil</h1>"));
    assert!(matches!(
        events.lock().unwrap().last(),
        Some(BuildEvent::Finished(_))
    ));

    let key = site.display().to_string();
    let target = HostingTarget::Sftp {
        host: "h".into(),
        port: 22,
        user: "u".into(),
        remote_dir: "www/".into(),
        known_host_key: None,
    };
    let p = c
        .save_profile(
            &key,
            HostingProfile {
                id: String::new(),
                name: "OVH".into(),
                environment: "production".into(),
                target: target.clone(),
            },
            Some("secret".into()),
        )
        .unwrap();
    assert!(!p.id.is_empty());
    assert!(c.has_secret(&target));
    // État relu depuis le disque : hébergement présent, aucun secret dans le fichier.
    let c2 = Core::new(
        &d.path().join("config"),
        Box::new(secrets::Memory::default()),
    );
    assert_eq!(c2.profiles(&key).len(), 1);
    let state = std::fs::read_to_string(d.path().join("config/revel-state.json")).unwrap();
    assert!(!state.contains("secret"));
    c.delete_profile(&key, &p.id).unwrap();
    assert!(!c.has_secret(&target));
}

#[tokio::test]
async fn apercu() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("a")).unwrap();
    std::fs::write(d.path().join("a/index.html"), "page A").unwrap();
    std::fs::write(d.path().join("404.html"), "absente").unwrap();
    let mut p = preview::start(d.path().to_path_buf()).await.unwrap();
    let get = |path: &str| {
        let url = format!("{}{path}", p.url);
        async move {
            let addr = url
                .trim_start_matches("http://")
                .split('/')
                .next()
                .unwrap()
                .to_string();
            let path = format!("/{}", url.splitn(4, '/').nth(3).unwrap_or(""));
            let mut s = tokio::net::TcpStream::connect(&addr).await.unwrap();
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            s.write_all(
                format!("GET {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").as_bytes(),
            )
            .await
            .unwrap();
            let mut buf = String::new();
            s.read_to_string(&mut buf).await.unwrap();
            buf
        }
    };
    let a = get("a/").await;
    assert!(
        a.starts_with("HTTP/1.1 200") && a.ends_with("page A"),
        "{a}"
    );
    let n = get("absent").await;
    assert!(
        n.starts_with("HTTP/1.1 404") && n.ends_with("absente"),
        "{n}"
    );
    p.stop();
}

/// Publication complète par le cœur (génération + avertissements + SFTP), si un serveur
/// d'essai est configuré (variables `REVEL_SFTP_*`, voir revel-deploy/tests/sftp_local.rs).
#[tokio::test(flavor = "multi_thread")]
async fn publication_par_le_coeur() {
    let get = |k: &str| std::env::var(k).ok();
    let (Some(host), Some(port), Some(user), Some(password), Some(home)) = (
        get("REVEL_SFTP_HOST"),
        get("REVEL_SFTP_PORT"),
        get("REVEL_SFTP_USER"),
        get("REVEL_SFTP_PASSWORD"),
        get("REVEL_SFTP_HOME"),
    ) else {
        eprintln!("REVEL_SFTP_* absent : essai ignoré");
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    let key = site.display().to_string();
    let _ = std::fs::remove_dir_all(Path::new(&home).join("revel-coeur"));
    let _ = std::fs::remove_file(Path::new(&home).join(".deploy/revel-coeur.manifest"));
    let p = c
        .save_profile(
            &key,
            HostingProfile {
                id: String::new(),
                name: "Essai".into(),
                environment: "production".into(),
                target: HostingTarget::Sftp {
                    host,
                    port: port.parse().unwrap(),
                    user,
                    remote_dir: "revel-coeur/".into(),
                    known_host_key: None,
                },
            },
            Some(password),
        )
        .unwrap();
    let events = Mutex::new(Vec::new());
    let s = c
        .deploy(&site, &p.id, false, false, &|e| {
            events.lock().unwrap().push(e)
        })
        .await
        .unwrap();
    assert_eq!(s.uploaded, 1);
    assert!(s.key_recorded);
    let html = std::fs::read_to_string(Path::new(&home).join("revel-coeur/index.html")).unwrap();
    assert!(html.contains("<h1>Accueil</h1>"));
    // Clé enregistrée : la deuxième publication la vérifie ; rien à envoyer.
    let p2 = &c.profiles(&key)[0];
    assert!(matches!(
        &p2.target,
        HostingTarget::Sftp {
            known_host_key: Some(_),
            ..
        }
    ));
    let s2 = c.deploy(&site, &p.id, false, false, &|_| {}).await.unwrap();
    assert_eq!((s2.uploaded, s2.key_recorded), (0, false));
    assert!(
        events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, DeployEvent::Warnings { .. }))
    );
}
