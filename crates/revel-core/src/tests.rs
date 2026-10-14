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

#[test]
fn session_edition_enregistre_et_annule() {
    use revel_edit::{Branch, Command, Owner, Parent, Target};
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    let file = site.join(revel_project::SITE_FILE);
    assert!(matches!(
        c.edit_apply(&site, Command::Batch { commands: vec![] }, false),
        Err(CoreError::NotOpen)
    ));
    let opened = c.edit_open(&site).unwrap();
    assert_eq!(opened.state.revision, 0);
    // Les identifiants attribués à la lecture ne sont écrits qu'à la première modification.
    let h1 = revel_model::ids::node_id(&opened.site.pages[0].body[1])
        .unwrap()
        .to_string();
    let target = Target {
        owner: Owner::Page("index".into()),
        node: h1,
    };
    let st = c
        .edit_apply(
            &site,
            Command::SetAttr {
                at: target.clone(),
                name: "class".into(),
                value: Some("titre".into()),
            },
            false,
        )
        .unwrap();
    assert_eq!((st.revision, st.can_undo, st.can_redo), (1, true, false));
    let saved = std::fs::read_to_string(&file).unwrap();
    assert!(saved.contains("\"titre\""));
    let ids_written = saved.clone();
    // Insertion d'un texte, puis annulation complète : fichier identique à l'état d'après la 1re commande.
    c.edit_apply(
        &site,
        Command::Insert {
            parent: Parent::of(&target, Branch::Children),
            index: 0,
            node: revel_model::Node::Text {
                text: "Préfixe ".into(),
            },
        },
        false,
    )
    .unwrap();
    let st = c.edit_undo(&site).unwrap();
    assert!(matches!(st.applied, Some(Command::RemoveAt { .. })));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), ids_written);
    let st = c.edit_redo(&site).unwrap();
    assert!(st.applied.is_some() && !st.can_redo);
    // La génération lit le fichier enregistré.
    c.build(&site, "production", false, &|_| {}).unwrap();
    let html = std::fs::read_to_string(out_dir(&site, "production").join("index.html")).unwrap();
    assert!(
        html.contains(r#"<h1 class="titre">Préfixe Accueil</h1>"#),
        "{html}"
    );
    // Modification hors de Revel : la session relit le fichier.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let mut p = revel_project::Project::open(&site).unwrap();
    p.site.name = "Renommé".into();
    p.save().unwrap();
    let again = c.edit_open(&site).unwrap();
    assert_eq!(again.site.name, "Renommé");
    assert!(!again.state.can_undo);
}

#[test]
fn canevas_par_le_protocole() {
    use revel_edit::canvas::{Binding, Seg, Slot, Source};
    use revel_edit::{Command, Owner, Target};
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    // Feuille de style du modèle et fichier copié.
    {
        let mut p = revel_project::Project::open(&site).unwrap();
        p.site.stylesheet = Some(revel_model::Stylesheet {
            url: "/assets/site.css".into(),
            source: None,
            items: revel_style::parse("h1 { color: red }").unwrap(),
        });
        p.site.passthrough.push(revel_model::Passthrough {
            from: "public".into(),
            to: "/".into(),
        });
        p.save().unwrap();
        std::fs::create_dir_all(site.join("public/média")).unwrap();
        std::fs::write(site.join("public/média/a.txt"), "fichier").unwrap();
    }
    let opts = canvas::CanvasOptions::default();
    assert!(matches!(
        c.canvas_open(&site, opts),
        Err(CoreError::NotOpen)
    ));
    let opened = c.edit_open(&site).unwrap();
    let base = c.canvas_open(&site, opts).unwrap();
    assert!(base == "revel-canvas://localhost/" || base == "http://revel-canvas.localhost/");

    let page = c.canvas_respond("/");
    assert_eq!(page.status, 200);
    assert_eq!(page.content_type, "text/html; charset=utf-8");
    let html = String::from_utf8(page.body).unwrap();
    let h1 = revel_model::ids::node_id(&opened.site.pages[0].body[1])
        .unwrap()
        .to_string();
    assert!(
        html.contains(&format!(
            "<h1 data-rv=\"{h1}\"><!--rv:0-->Accueil<!--/rv:0--></h1>"
        )),
        "{html}"
    );
    assert!(html.contains("<script src=\"/__revel/canvas.js\"></script>"));
    // Repères : la valeur affichée vient de la variable « title » de la page.
    let start = html.find("id=\"rv-slots\">").unwrap() + "id=\"rv-slots\">".len();
    let end = start + html[start..].find("</script>").unwrap();
    let slots: Vec<Slot> = serde_json::from_str(&html[start..end]).unwrap();
    assert!(
        matches!(&slots[0], Slot::Expr { binding: Some(Binding { source: Source::Page { page }, path }), .. }
        if page == "index" && path == &vec![Seg::Key("title".into())])
    );

    let js = c.canvas_respond("/__revel/canvas.js");
    assert_eq!(js.content_type, "text/javascript; charset=utf-8");
    assert!(String::from_utf8(js.body).unwrap().contains("rv-slots"));
    assert_eq!(
        c.canvas_respond("/m%C3%A9dia/a.txt").body,
        b"fichier".to_vec(),
        "chemin décodé, fichier copié"
    );
    assert_eq!(c.canvas_respond("/absent/").status, 404);
    assert_eq!(c.canvas_respond("/../site.revel.json").status, 404);

    // La feuille de style suit le document en cours d'édition.
    let css = c.canvas_respond("/assets/site.css?rv=1");
    assert_eq!(String::from_utf8(css.body).unwrap(), "h1 { color: red; }\n");
    let rule = opened
        .site
        .style_rules()
        .next()
        .unwrap()
        .id
        .clone()
        .unwrap();
    c.edit_apply(
        &site,
        Command::SetDecl {
            rule,
            prop: "color".into(),
            value: Some("blue".into()),
            important: false,
        },
        false,
    )
    .unwrap();
    let css = c.canvas_respond("/assets/site.css");
    assert_eq!(
        String::from_utf8(css.body).unwrap(),
        "h1 { color: blue; }\n"
    );
    // Un texte modifié apparaît au rendu suivant.
    c.edit_apply(
        &site,
        Command::SetPageProp {
            page: "index".into(),
            key: "title".into(),
            value: Some(serde_json::json!("Bonjour")),
        },
        false,
    )
    .unwrap();
    let html = String::from_utf8(c.canvas_respond("/").body).unwrap();
    assert!(html.contains("<!--rv:0-->Bonjour<!--/rv:0-->"));
    let _ = Target {
        owner: Owner::Page("index".into()),
        node: h1,
    };
}
