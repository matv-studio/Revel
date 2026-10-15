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

#[test]
fn textes_lies_ecrits_dans_leur_source() {
    use revel_edit::Command;
    use revel_edit::canvas::{Binding, Seg, Source};
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    let data = "# Données de l'accueil\nhero:\n  title: L'agence   # titre\n  year: 2026\n";
    let entry = "---\ntitle: 'Wattr'\ndate: 2026-01-02\n---\n\nCorps\n";
    {
        let mut p = revel_project::Project::open(&site).unwrap();
        p.site.data_dir = Some("donnees".into());
        p.site.settings.typography = Some("fr".into());
        p.site.collections.push(revel_model::Collection {
            id: "projets".into(),
            source: revel_model::CollectionSource::Folder,
            folder: "contenus/projets".into(),
            path: None,
            layout: None,
            props: Default::default(),
            filter: None,
            sort: None,
            fields: vec![],
        });
        p.save().unwrap();
        std::fs::create_dir_all(site.join("donnees/pages")).unwrap();
        std::fs::write(site.join("donnees/pages/accueil.yml"), data).unwrap();
        std::fs::create_dir_all(site.join("contenus/projets")).unwrap();
        std::fs::write(site.join("contenus/projets/wattr.md"), entry).unwrap();
    }
    c.edit_open(&site).unwrap();
    let stamp = c.edit_stamp(&site).unwrap();
    let k = |s: &str| Seg::Key(s.into());
    let title = Binding {
        source: Source::Data,
        path: vec![k("pages"), k("accueil"), k("hero"), k("title")],
    };
    // Typographie de la page retirée (apostrophe droite et espace dans la source).
    let st = c
        .edit_apply(
            &site,
            Command::SetBound {
                binding: title.clone(),
                value: "L’atelier\u{a0}!".into(),
            },
            false,
        )
        .unwrap();
    assert!(matches!(&st.applied, Some(Command::SetBound { value, .. }) if value == "L'atelier !"));
    let written = std::fs::read_to_string(site.join("donnees/pages/accueil.yml")).unwrap();
    assert_eq!(
        written,
        data.replace("title: L'agence", "title: L'atelier !")
    );
    assert_ne!(
        c.edit_stamp(&site).unwrap(),
        stamp,
        "fichier modifié : empreinte changée"
    );
    // En-tête d'un contenu (style entre guillemets simples gardé).
    c.edit_apply(
        &site,
        Command::SetBound {
            binding: Binding {
                source: Source::Entry {
                    folder: "contenus/projets".into(),
                    file: "wattr".into(),
                },
                path: vec![k("title")],
            },
            value: "Wattr, l'eau".into(),
        },
        false,
    )
    .unwrap();
    let md = std::fs::read_to_string(site.join("contenus/projets/wattr.md")).unwrap();
    assert_eq!(
        md,
        entry.replace("title: 'Wattr'", "title: 'Wattr, l''eau'")
    );
    // Variable de page (document du site).
    c.edit_apply(
        &site,
        Command::SetBound {
            binding: Binding {
                source: Source::Page {
                    page: "index".into(),
                },
                path: vec![k("title")],
            },
            value: "Bonjour".into(),
        },
        false,
    )
    .unwrap();
    assert_eq!(
        revel_project::Project::open(&site).unwrap().site.pages[0].props["title"],
        "Bonjour"
    );
    // Tout annuler : fichiers identiques octet pour octet.
    for _ in 0..3 {
        c.edit_undo(&site).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(site.join("donnees/pages/accueil.yml")).unwrap(),
        data
    );
    assert_eq!(
        std::fs::read_to_string(site.join("contenus/projets/wattr.md")).unwrap(),
        entry
    );
    assert_eq!(
        revel_project::Project::open(&site).unwrap().site.pages[0].props["title"],
        "Accueil"
    );
    // Rétablir puis annuler encore : même résultat.
    c.edit_redo(&site).unwrap();
    assert!(
        std::fs::read_to_string(site.join("donnees/pages/accueil.yml"))
            .unwrap()
            .contains("L'atelier !")
    );
    c.edit_undo(&site).unwrap();
    // Valeur calculée ou fichier absent : refusé, rien n'est écrit.
    let e = c.edit_apply(
        &site,
        Command::SetBound {
            binding: Binding {
                source: Source::Data,
                path: vec![k("absent"), k("x")],
            },
            value: "x".into(),
        },
        false,
    );
    assert!(matches!(
        e,
        Err(CoreError::Edit(revel_edit::EditError::Bound(_)))
    ));
}

#[test]
fn dossier_d_un_nouveau_site() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path();
    assert_eq!(
        site_folder(parent, "  Atelier Lumière & Cœur  "),
        parent.join("atelier-lumiere-coeur")
    );
    assert_eq!(site_folder(parent, "???"), parent.join("site"));
    // Dossier vide : réutilisé ; dossier occupé : suffixe.
    std::fs::create_dir(parent.join("oka")).unwrap();
    assert_eq!(site_folder(parent, "Oka"), parent.join("oka"));
    std::fs::write(parent.join("oka/notes.txt"), "x").unwrap();
    assert_eq!(site_folder(parent, "Oka"), parent.join("oka-2"));
    let c = core(parent);
    let s = c.create_site(parent, "Oka", "fr").unwrap();
    assert!(s.path.ends_with("oka-2"));
    assert!(c.cms(Path::new(&s.path)).unwrap().is_empty());
}

#[test]
fn contenus_modifies_crees_supprimes_et_annules() {
    use revel_edit::Command;
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    let entry = "---\n# Projet\ntitle: 'Wattr'\ntags:\n  - film\n---\n\nCorps\n";
    {
        let mut p = revel_project::Project::open(&site).unwrap();
        p.site.collections.push(revel_model::Collection {
            id: "projets".into(),
            source: revel_model::CollectionSource::Folder,
            folder: "contenus/projets".into(),
            path: Some("/projet/{{ title }}/".into()),
            layout: None,
            props: Default::default(),
            filter: None,
            sort: None,
            fields: vec![],
        });
        p.save().unwrap();
        std::fs::create_dir_all(site.join("contenus/projets")).unwrap();
        std::fs::write(site.join("contenus/projets/wattr.md"), entry).unwrap();
    }
    let read = || std::fs::read_to_string(site.join("contenus/projets/wattr.md")).unwrap();
    c.edit_open(&site).unwrap();
    let field = |key: &str, value: Option<serde_json::Value>| Command::SetEntryField {
        folder: "contenus/projets".into(),
        file: "wattr".into(),
        key: key.into(),
        value,
    };
    c.edit_apply(
        &site,
        field("title", Some(serde_json::json!("Wattr 2"))),
        false,
    )
    .unwrap();
    c.edit_apply(
        &site,
        field("tags", Some(serde_json::json!(["film", "ia"]))),
        false,
    )
    .unwrap();
    c.edit_apply(
        &site,
        field("featured", Some(serde_json::json!(true))),
        false,
    )
    .unwrap();
    c.edit_apply(
        &site,
        Command::SetEntryBody {
            folder: "contenus/projets".into(),
            file: "wattr".into(),
            body: "Nouveau corps".into(),
        },
        false,
    )
    .unwrap();
    assert_eq!(
        read(),
        "---\n# Projet\ntitle: 'Wattr 2'\ntags:\n  - film\n  - ia\nfeatured: true\n---\nNouveau corps\n"
    );
    // Nouveau contenu, visible dans la vue CMS avec l'adresse de sa page ; puis supprimé.
    c.edit_apply(
        &site,
        Command::CreateEntry {
            folder: "contenus/projets".into(),
            file: "oka".into(),
            data: serde_json::json!({ "title": "Oka" }),
            body: String::new(),
        },
        false,
    )
    .unwrap();
    let cms = c.cms(&site).unwrap();
    let oka = cms[0].entries.iter().find(|e| e.file == "oka").unwrap();
    assert_eq!(oka.url.as_deref(), Some("/projet/Oka/"));
    assert!(
        c.edit_apply(
            &site,
            Command::CreateEntry {
                folder: "contenus/projets".into(),
                file: "oka".into(),
                data: serde_json::json!({}),
                body: String::new(),
            },
            false,
        )
        .is_err()
    );
    c.edit_apply(
        &site,
        Command::WriteEntry {
            folder: "contenus/projets".into(),
            file: "wattr".into(),
            text: None,
        },
        false,
    )
    .unwrap();
    assert!(!site.join("contenus/projets/wattr.md").exists());
    // Tout annulé : fichiers identiques octet pour octet, nouveau contenu retiré.
    for _ in 0..6 {
        c.edit_undo(&site).unwrap();
    }
    assert_eq!(read(), entry);
    assert!(!site.join("contenus/projets/oka.md").exists());
    // Rétabli, puis chemins refusés.
    c.edit_redo(&site).unwrap();
    assert!(read().contains("title: 'Wattr 2'"));
    assert!(
        c.edit_apply(
            &site,
            Command::WriteEntry {
                folder: "contenus/projets".into(),
                file: "../../site".into(),
                text: Some("x".into()),
            },
            false,
        )
        .is_err()
    );
    assert!(
        c.edit_apply(
            &site,
            Command::WriteEntry {
                folder: "ailleurs".into(),
                file: "x".into(),
                text: Some("x".into()),
            },
            false,
        )
        .is_err()
    );
}

#[test]
fn schema_des_collections_lu_dans_pages_cms() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    {
        let mut p = revel_project::Project::open(&site).unwrap();
        // Deux collections d'Eleventy sur le même dossier : une seule dans la vue CMS.
        for id in ["toutesOffres", "offres"] {
            p.site.collections.push(revel_model::Collection {
                id: id.into(),
                source: revel_model::CollectionSource::Folder,
                folder: "contenus/offres".into(),
                path: None,
                layout: None,
                props: Default::default(),
                filter: None,
                sort: None,
                fields: vec![],
            });
        }
        p.save().unwrap();
        std::fs::create_dir_all(site.join("contenus/offres")).unwrap();
        std::fs::write(
            site.join("contenus/offres/stage.md"),
            "---\ntitle: Stage\nville: Lyon\n---\n",
        )
        .unwrap();
    }
    std::fs::write(
        site.join(".pages.yml"),
        "content:\n  - name: offres\n    label: Offres d'emploi\n    type: collection\n    path: contenus/offres/\n    view: { primary: title, sort: date, order: desc }\n    fields:\n      - { name: title, label: Titre, type: string, required: true }\n      - { name: date, type: date, options: { time: true } }\n",
    )
    .unwrap();
    let cms = c.cms(&site).unwrap();
    assert_eq!(cms.len(), 1);
    let o = &cms[0];
    assert_eq!(o.id, "offres");
    assert_eq!(o.schema.as_deref(), Some("offres"));
    assert_eq!(o.label.as_deref(), Some("Offres d'emploi"));
    assert_eq!(o.fields.len(), 2);
    assert!(o.fields[0].required);
    assert_eq!(o.fields[1].options["time"], true);
    assert!(o.view.as_ref().unwrap().descending);
    // La valeur hors du schéma reste dans le contenu (la fiche l'affiche à part).
    assert_eq!(o.entries[0].data["ville"], "Lyon");
}

#[test]
fn schema_modifie_dans_pages_cms_et_annule() {
    use revel_edit::Command;
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    {
        let mut p = revel_project::Project::open(&site).unwrap();
        p.site.collections.push(revel_model::Collection {
            id: "projets".into(),
            source: revel_model::CollectionSource::Folder,
            folder: "contenus/projets".into(),
            path: None,
            layout: None,
            props: Default::default(),
            filter: None,
            sort: None,
            fields: vec![],
        });
        p.save().unwrap();
        std::fs::create_dir_all(site.join("contenus/projets")).unwrap();
        std::fs::write(
            site.join("contenus/projets/oka.md"),
            "---\ntitle: Oka\nlayout: projet.njk\ndate: 2026-03-19T09:29:24\nville: Lyon\n---\n",
        )
        .unwrap();
    }
    c.edit_open(&site).unwrap();
    let config = || std::fs::read_to_string(site.join(".pages.yml")).ok();
    let folder = || "contenus/projets".to_string();
    // Sans `.pages.yml` : fichier créé, collection décrite d'après ses contenus.
    c.edit_apply(
        &site,
        Command::SetCmsField {
            folder: folder(),
            field: "title".into(),
            patch: serde_json::json!({ "label": "Titre du projet", "required": true }),
        },
        false,
    )
    .unwrap();
    let t = config().unwrap();
    assert!(t.contains("merge: true"), "{t}");
    let cms = c.cms(&site).unwrap();
    let p = &cms[0];
    assert_eq!(p.schema.as_deref(), Some("projets"));
    assert!(p.merge);
    assert_eq!(p.fields[0].label.as_deref(), Some("Titre du projet"));
    assert!(p.fields[0].required);
    let f = |n: &str| p.fields.iter().find(|f| f.name == n).unwrap().clone();
    assert!(f("layout").hidden);
    assert_eq!(f("date").options["format"], "yyyy-MM-dd'T'HH:mm:ss");
    // Champ retiré puis rajouté sans type (déduit des contenus), déplacé ; réglages.
    c.edit_apply(
        &site,
        Command::RemoveCmsField {
            folder: folder(),
            field: "ville".into(),
        },
        false,
    )
    .unwrap();
    assert!(
        c.cms(&site).unwrap()[0]
            .fields
            .iter()
            .all(|f| f.name != "ville")
    );
    c.edit_apply(
        &site,
        Command::AddCmsField {
            folder: folder(),
            def: serde_json::json!({ "name": "ville" }),
            index: Some(0),
        },
        false,
    )
    .unwrap();
    c.edit_apply(
        &site,
        Command::MoveCmsField {
            folder: folder(),
            field: "ville".into(),
            index: 3,
        },
        false,
    )
    .unwrap();
    c.edit_apply(
        &site,
        Command::SetCmsCollection {
            folder: folder(),
            patch: serde_json::json!({ "label": "Réalisations" }),
        },
        false,
    )
    .unwrap();
    let cms = c.cms(&site).unwrap();
    assert_eq!(cms[0].label.as_deref(), Some("Réalisations"));
    let names: Vec<&str> = cms[0].fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["title", "layout", "date", "ville"]);
    assert_eq!(cms[0].fields[3].kind.as_deref(), Some("string"));
    assert!(
        c.edit_apply(
            &site,
            Command::SetCmsField {
                folder: folder(),
                field: "title".into(),
                patch: serde_json::json!({ "name": "nom" })
            },
            false
        )
        .is_err()
    );
    // Tout annulé : plus de fichier ; rétabli : identique.
    let last = config();
    for _ in 0..5 {
        c.edit_undo(&site).unwrap();
    }
    assert_eq!(config(), None);
    for _ in 0..5 {
        c.edit_redo(&site).unwrap();
    }
    assert_eq!(config(), last);
    // Nouvelle collection : site et `.pages.yml` ; un contenu y est créé ; annulée, rétablie.
    let fields = serde_json::json!([
        { "name": "title", "label": "Titre", "type": "string", "required": true },
        { "name": "slug", "label": "Adresse", "type": "string" }
    ]);
    let create = Command::CreateCmsCollection {
        id: "offres".into(),
        label: "Offres d'emploi".into(),
        folder: "contenus/offres".into(),
        fields: fields.clone(),
    };
    c.edit_apply(&site, create, false).unwrap();
    let cms = c.cms(&site).unwrap();
    let o = cms.iter().find(|x| x.id == "offres").unwrap();
    assert_eq!(o.label.as_deref(), Some("Offres d'emploi"));
    assert_eq!(o.filename.as_deref(), Some("{fields.slug}.md"));
    assert!(o.fields[0].required);
    c.edit_apply(
        &site,
        Command::CreateEntry {
            folder: "contenus/offres".into(),
            file: "stage".into(),
            data: serde_json::json!({ "title": "Stage", "slug": "stage" }),
            body: String::new(),
        },
        false,
    )
    .unwrap();
    assert!(site.join("contenus/offres/stage.md").is_file());
    let with = config();
    c.edit_undo(&site).unwrap();
    c.edit_undo(&site).unwrap();
    assert_eq!(config(), last);
    assert!(c.cms(&site).unwrap().iter().all(|x| x.id != "offres"));
    assert!(!site.join("contenus/offres/stage.md").exists());
    c.edit_redo(&site).unwrap();
    assert_eq!(config(), with);
    // Noms refusés.
    for (id, folder) in [
        ("x y", "contenus/xy"),
        ("ok", "../ailleurs"),
        ("projets", "contenus/autre"),
        ("autre", "contenus/projets"),
    ] {
        let r = c.edit_apply(
            &site,
            Command::CreateCmsCollection {
                id: id.into(),
                label: "X".into(),
                folder: folder.into(),
                fields: fields.clone(),
            },
            false,
        );
        assert!(r.is_err(), "{id} {folder}");
    }
}

#[test]
fn apercu_sans_generation_et_export() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    // Aperçu : page rendue à la demande, sans repères ni script du canevas.
    let base = c.preview_open(&site, "production").unwrap();
    assert!(base.contains("revel-preview"));
    let r = c.preview_respond("/");
    assert_eq!(r.status, 200);
    let html = String::from_utf8(r.body).unwrap();
    assert!(html.contains("<h1>Accueil</h1>"), "{html}");
    assert!(!html.contains("data-rv") && !html.contains("rv:") && !html.contains("canvas.js"));
    assert_eq!(c.preview_respond(canvas::CANVAS_JS_URL).status, 404);
    // Export : génération puis copie dans un nouveau dossier.
    let exports = d.path().join("exports");
    let s = c
        .export(&site, "production", false, &exports, &|_| {})
        .unwrap();
    assert!(s.out_dir.ends_with("essai-production"), "{}", s.out_dir);
    let index = std::fs::read_to_string(Path::new(&s.out_dir).join("index.html")).unwrap();
    assert!(index.contains("<h1>Accueil</h1>"));
    // Deuxième export : dossier distinct.
    let s2 = c
        .export(&site, "production", false, &exports, &|_| {})
        .unwrap();
    assert!(s2.out_dir.ends_with("essai-production-2"));
}
