use super::*;
use revel_model::{Node, Page, build::*};

fn core(dir: &Path) -> Core {
    Core::new(&dir.join("config"), Box::new(secrets::Memory::default()))
}

fn sample_site(dir: &Path, c: &Core) -> PathBuf {
    let site = dir.join("site");
    c.new_site(&site, "Essai", "fr").unwrap();
    let mut p = revel_project::Project::open(&site).unwrap();
    // Site vide (sans le site de départ) : les essais construisent ce qu'ils vérifient.
    let name = p.site.name.clone();
    p.site = revel_model::Site::new(name, "fr");
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
        code: Default::default(),
        name: None,
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
    // La page et le robots.txt produit par Revel (pas d'adresse du site : pas de plan du site).
    assert_eq!(b.generated, 2);
    assert!(out_dir(&site, "production").join("robots.txt").is_file());
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
                sync_versions: true,
                site_url: None,
                preset: None,
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
                sync_versions: true,
                site_url: None,
                preset: None,
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
        .deploy(&site, &p.id, false, false, false, &|e| {
            events.lock().unwrap().push(e)
        })
        .await
        .unwrap();
    // Page d'accueil et robots.txt (produit quand le site n'en a pas).
    assert_eq!(s.uploaded, 2);
    assert!(s.key_recorded);
    assert!(Path::new(&home).join("revel-coeur/robots.txt").is_file());
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
    let s2 = c
        .deploy(&site, &p.id, false, false, false, &|_| {})
        .await
        .unwrap();
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
fn canevas_d_un_composant_seul() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    {
        let mut p = revel_project::Project::open(&site).unwrap();
        let mut carte = revel_model::Component::new(
            "carte",
            vec![revel_model::build::el(
                "div",
                &[("class", "carte")],
                vec![revel_model::build::text("Titre")],
            )],
        );
        carte.variants = vec![revel_model::Variant {
            id: "sombre".into(),
            name: None,
            kind: revel_model::VariantKind::Custom,
        }];
        p.site.components.push(carte);
        p.site.ensure_ids();
        p.save().unwrap();
    }
    c.edit_open(&site).unwrap();
    c.canvas_open(&site, canvas::CanvasOptions::default())
        .unwrap();
    let page = c.canvas_respond("/__revel/component/?component=carte&variant=sombre&page=%2F");
    assert_eq!(page.status, 200);
    let html = String::from_utf8(page.body).unwrap();
    assert!(html.contains("class=\"carte is-sombre\""), "{html}");
    assert!(html.contains("data-rv="), "{html}");
    assert!(html.contains("<script src=\"/__revel/canvas.js\"></script>"));
    // Utilisation repérée comme un composant (sélection de ses éléments dans le canevas).
    assert!(html.contains("\"component\":\"carte\""), "{html}");
    assert_eq!(
        c.canvas_respond("/__revel/component/?component=absent")
            .status,
        404
    );
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
    // Balise des repères : `id="rv-slots"` puis, le cas échéant, le numéro de révision (`data-rev`).
    let tag = html.find("id=\"rv-slots\"").unwrap();
    let start = tag + html[tag..].find('>').unwrap() + 1;
    let end = start + html[start..].find("</script>").unwrap();
    let slots: Vec<Slot> = serde_json::from_str(&html[start..end]).unwrap();
    assert!(
        matches!(&slots[0], Slot::Expr { binding: Some(Binding { source: Source::Page { page }, path }), .. }
        if page == "index" && path == &vec![Seg::Key("title".into())])
    );

    let js = c.canvas_respond("/__revel/canvas.js");
    assert_eq!(js.content_type, "text/javascript; charset=utf-8");
    assert!(String::from_utf8(js.body).unwrap().contains("rv-slots"));
    // Script des animations (aperçu dans le canevas, 6.1).
    let anim = c.canvas_respond("/assets/revel-animations.js");
    assert_eq!(anim.content_type, "text/javascript; charset=utf-8");
    assert!(
        String::from_utf8(anim.body)
            .unwrap()
            .contains("RevelAnimations")
    );
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
    let after = c.edit_stamp(&site).unwrap();
    assert_ne!(
        after.files, stamp.files,
        "fichier modifié : empreinte changée"
    );
    assert_eq!(after.revision, stamp.revision + 1, "révision suivante");
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
    // Adresse remplacée dans le fichier (plan du site, D-107) : mise en forme gardée, annulable.
    std::fs::write(
        site.join("contenus/projets/wattr.md"),
        "---\ntitle: Wattr\nvideo: https://vimeo.com/1\nblocks:\n  - body: |-\n      <p>Voir https://vimeo.com/1.</p>\n---\nAussi https://vimeo.com/12\n",
    )
    .unwrap();
    let before = read();
    c.edit_apply(
        &site,
        Command::ReplaceInEntry {
            folder: "contenus/projets".into(),
            file: "wattr".into(),
            find: "https://vimeo.com/1".into(),
            replace: "https://vimeo.com/2".into(),
        },
        false,
    )
    .unwrap();
    assert_eq!(
        read(),
        "---\ntitle: Wattr\nvideo: https://vimeo.com/2\nblocks:\n  - body: |-\n      <p>Voir https://vimeo.com/2.</p>\n---\nAussi https://vimeo.com/12\n"
    );
    c.edit_undo(&site).unwrap();
    assert_eq!(read(), before);
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
    // Collection créée avec ses pages (D-119) : une seule étape.
    let opened = c.edit_open(&site).unwrap().site;
    let layout = revel_edit::make::collection_template(&opened, "annonces", "title");
    let lid = layout.id.clone();
    c.edit_apply(
        &site,
        Command::Batch {
            commands: vec![
                Command::CreateCmsCollection {
                    id: "annonces".into(),
                    label: "Annonces".into(),
                    folder: "contenus/annonces".into(),
                    fields: fields.clone(),
                },
                Command::AddLayout {
                    layout,
                    index: None,
                },
                Command::SetCollectionPages {
                    id: "annonces".into(),
                    path: Some("/annonces/{{ slug }}/".into()),
                    layout: Some(lid),
                },
            ],
        },
        false,
    )
    .unwrap();
    c.edit_undo(&site).unwrap();
    assert_eq!(config(), with);
    // Dossier qui a déjà des contenus (site existant repris) : leurs champs décrits en plus.
    std::fs::create_dir_all(site.join("contenus/clients")).unwrap();
    std::fs::write(
        site.join("contenus/clients/volvo.md"),
        "---\ntitle: Volvo\nslug: volvo\nlogo: /media/volvo.svg\nordre: 3\n---\n",
    )
    .unwrap();
    c.edit_apply(
        &site,
        Command::CreateCmsCollection {
            id: "clients".into(),
            label: "Clients".into(),
            folder: "contenus/clients".into(),
            fields: fields.clone(),
        },
        false,
    )
    .unwrap();
    let cl = c
        .cms(&site)
        .unwrap()
        .into_iter()
        .find(|x| x.id == "clients")
        .unwrap();
    let names: Vec<&str> = cl.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["title", "slug", "logo", "ordre"]);
    assert_eq!(cl.fields[2].kind.as_deref(), Some("image"));
    assert_eq!(cl.fields[0].label.as_deref(), Some("Titre"));
    c.edit_undo(&site).unwrap();
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

#[test]
fn expression_invalide_refusee() {
    use revel_edit::{Command, Owner, Parent};
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    c.edit_open(&site).unwrap();
    let file = site.join(revel_project::SITE_FILE);
    let root = Parent::root(Owner::Page("index".into()));
    let set = |expr: &str| revel_model::Node::Set {
        name: "x".into(),
        expr: expr.into(),
    };
    let replace = |expr: &str| Command::Replace {
        parent: root.clone(),
        index: 0,
        node: set(expr),
    };
    c.edit_apply(
        &site,
        Command::Insert {
            parent: root.clone(),
            index: 0,
            node: set("1"),
        },
        false,
    )
    .unwrap();
    let saved = std::fs::read_to_string(&file).unwrap();
    // Syntaxe fausse : refusée, rien n'est écrit.
    let err = c.edit_apply(&site, replace("a |"), false).unwrap_err();
    assert!(
        matches!(&err, CoreError::Edit(revel_edit::EditError::InvalidExpr(m)) if m.starts_with("a |")),
        "{err:?}"
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), saved);
    // Filtre inconnu du moteur : accepté (résolu au rendu).
    c.edit_apply(&site, replace("site.title | inconnu"), false)
        .unwrap();
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("site.title | inconnu")
    );
    c.edit_undo(&site).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), saved);
}

#[test]
fn medias_du_site() {
    use revel_protocol::MediaKind;
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    let mut p = revel_project::Project::open(&site).unwrap();
    p.site.passthrough = vec![
        revel_model::Passthrough {
            from: "src/media".into(),
            to: "media".into(),
        },
        revel_model::Passthrough {
            from: "src/static".into(),
            to: "".into(),
        },
    ];
    p.save().unwrap();
    for (f, n) in [
        ("src/media/2024/01/photo.JPG", 3),
        ("src/media/2024/plaquette.pdf", 5),
        ("src/media/.cache/x.png", 1),
        ("src/media/police.woff2", 2),
        ("src/static/favicon.png", 1),
        ("src/static/contact.php", 1),
        ("src/static/robots.txt", 1),
    ] {
        let path = site.join(f);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, vec![0u8; n]).unwrap();
    }
    c.edit_open(&site).unwrap();
    let m = c.edit_media(&site).unwrap();
    let urls: Vec<(&str, MediaKind, u64)> =
        m.iter().map(|f| (f.url.as_str(), f.kind, f.size)).collect();
    assert_eq!(
        urls,
        vec![
            ("/favicon.png", MediaKind::Image, 1),
            ("/media/2024/01/photo.JPG", MediaKind::Image, 3),
            ("/media/2024/plaquette.pdf", MediaKind::Document, 5),
            ("/media/police.woff2", MediaKind::Font, 2),
        ]
    );
    assert!(m.iter().all(|f| f.modified.is_some()));
}

#[test]
fn design_propre_a_un_contenu() {
    use revel_edit::Command;
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    let entry = "---\ntitle: Wattr\n---\n\nCorps\n";
    {
        let mut p = revel_project::Project::open(&site).unwrap();
        p.site.layouts.push(revel_model::Layout {
            fields: Vec::new(),
            id: "projet".into(),
            parent: None,
            props: Default::default(),
            body: vec![
                Node::Html {
                    html: "<!doctype html>".into(),
                },
                el("article", &[("class", "modele")], vec![out("title")]),
            ],
        });
        p.site.collections.push(revel_model::Collection {
            id: "projets".into(),
            source: revel_model::CollectionSource::Folder,
            folder: "contenus/projets".into(),
            path: Some("/projet/{{ fileSlug }}/".into()),
            layout: Some("projet".into()),
            props: Default::default(),
            filter: None,
            sort: None,
            fields: vec![],
        });
        p.save().unwrap();
        std::fs::create_dir_all(site.join("contenus/projets")).unwrap();
        std::fs::write(site.join("contenus/projets/wattr.md"), entry).unwrap();
    }
    let opened = c.edit_open(&site).unwrap();
    let id = revel_edit::make::own_layout_id(&opened.site, "projet", "wattr");
    let mut copy = revel_edit::make::layout_copy(&opened.site, "projet", &id).unwrap();
    // Le design propre se modifie sans toucher au modèle.
    if let Node::Element { attrs, .. } = &mut copy.body[1] {
        attrs[0].value = Some("propre".into());
    }
    let field = |v: Option<serde_json::Value>| Command::SetEntryField {
        folder: "contenus/projets".into(),
        file: "wattr".into(),
        key: "layout".into(),
        value: v,
    };
    c.edit_apply(
        &site,
        Command::Batch {
            commands: vec![
                Command::AddLayout {
                    layout: copy,
                    index: None,
                },
                field(Some(serde_json::json!(id))),
            ],
        },
        false,
    )
    .unwrap();
    let file = site.join("contenus/projets/wattr.md");
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("layout: projet--wattr")
    );
    let pages = c.edit_pages(&site).unwrap();
    let p = pages
        .iter()
        .find(|p| p.file.as_deref() == Some("wattr"))
        .unwrap();
    assert_eq!(p.layout.as_deref(), Some("projet--wattr"));
    c.build(&site, "production", false, &|_| {}).unwrap();
    let out = revel_render::output_path(&p.url).unwrap();
    let html = std::fs::read_to_string(out_dir(&site, "production").join(&out))
        .unwrap_or_else(|e| panic!("{out} : {e}"));
    assert!(
        html.contains(r#"<article class="propre">Wattr</article>"#),
        "{html}"
    );
    // Échec au milieu d'une suite : rien n'est gardé (gabarit retiré, fichier intact).
    let bad = Command::Batch {
        commands: vec![
            Command::AddLayout {
                layout: revel_model::Layout {
                    fields: Vec::new(),
                    id: "autre".into(),
                    parent: None,
                    props: Default::default(),
                    body: vec![],
                },
                index: None,
            },
            Command::SetEntryField {
                folder: "contenus/absent".into(),
                file: "x".into(),
                key: "layout".into(),
                value: Some(serde_json::json!("autre")),
            },
        ],
    };
    assert!(c.edit_apply(&site, bad, false).is_err());
    assert!(c.edit_open(&site).unwrap().site.layout("autre").is_none());
    // Une seule annulation : modèle rétabli, fichiers identiques.
    c.edit_undo(&site).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), entry);
    assert_eq!(c.edit_open(&site).unwrap().site, opened.site);
    // Gabarit introuvable : celui de la collection, avec un avertissement.
    std::fs::write(
        &file,
        entry.replace("title: Wattr", "title: Wattr\nlayout: disparu"),
    )
    .unwrap();
    let r = c.build(&site, "production", false, &|_| {}).unwrap();
    assert!(
        r.warnings.iter().any(|w| w.contains("disparu")),
        "{:?}",
        r.warnings
    );
}

#[test]
fn pages_et_referencement() {
    use revel_edit::Command;
    use revel_edit::canvas::{Binding, Seg, Source};
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    let data = "# Agence\nseo:\n  title: L'agence # titre\n";
    {
        let mut p = revel_project::Project::open(&site).unwrap();
        p.site.data_dir = Some("donnees".into());
        p.site.pages.push(Page {
            id: "agence".into(),
            path: "/agence/".into(),
            layout: None,
            props: Default::default(),
            computed: [("seo".to_string(), "pages.agence.seo".to_string())].into(),
            body: vec![el("p", &[], vec![out("seo.title")])],
            draft: false,
            code: Default::default(),
            name: None,
        });
        p.save().unwrap();
        std::fs::create_dir_all(site.join("donnees/pages")).unwrap();
        std::fs::write(site.join("donnees/pages/agence.yml"), data).unwrap();
    }
    let opened = c.edit_open(&site).unwrap();
    let pages = c.edit_pages(&site).unwrap();
    let agence = pages.iter().find(|p| p.url == "/agence/").unwrap();
    let seo = agence.seo.clone().unwrap();
    assert_eq!(seo.title, "L'agence");
    assert_eq!(seo.data_path, vec!["pages", "agence", "seo"]);
    let key = |k: &str| Binding {
        source: Source::Data,
        path: seo
            .data_path
            .iter()
            .map(|s| Seg::Key(s.clone()))
            .chain([Seg::Key(k.into())])
            .collect(),
    };
    // Titre réécrit sur place, image ajoutée, en une étape.
    c.edit_apply(
        &site,
        Command::Batch {
            commands: vec![
                Command::SetDataValue {
                    binding: key("title"),
                    value: Some(serde_json::json!("Notre agence")),
                },
                Command::SetDataValue {
                    binding: key("image"),
                    value: Some(serde_json::json!("/media/agence.jpg")),
                },
            ],
        },
        false,
    )
    .unwrap();
    let file = site.join("donnees/pages/agence.yml");
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "# Agence\nseo:\n  title: Notre agence # titre\n  image: /media/agence.jpg\n"
    );
    // Nouvelle page, adresse changée avec redirection, brouillon.
    let mut np = opened.site.pages[0].clone();
    np.id = "offres".into();
    np.path = "/offres/".into();
    np.body = vec![];
    c.edit_apply(
        &site,
        Command::AddPage {
            page: np.clone(),
            index: None,
        },
        false,
    )
    .unwrap();
    assert!(matches!(
        c.edit_apply(
            &site,
            Command::AddPage {
                page: np,
                index: None
            },
            false
        ),
        Err(CoreError::Edit(revel_edit::EditError::DuplicateId(_)))
    ));
    assert!(matches!(
        c.edit_apply(
            &site,
            Command::SetPagePath {
                id: "offres".into(),
                path: "/agence/".into()
            },
            false
        ),
        Err(CoreError::Edit(revel_edit::EditError::PathTaken(_)))
    ));
    assert!(
        c.edit_apply(
            &site,
            Command::SetPagePath {
                id: "offres".into(),
                path: "/Offres été/".into()
            },
            false
        )
        .is_err()
    );
    c.edit_apply(
        &site,
        Command::Batch {
            commands: vec![
                Command::SetPagePath {
                    id: "offres".into(),
                    path: "/emplois/".into(),
                },
                Command::SetRedirects {
                    redirects: vec![revel_model::Redirect {
                        from: "/offres/".into(),
                        to: "/emplois/".into(),
                    }],
                },
                Command::SetPageDraft {
                    id: "offres".into(),
                    draft: true,
                },
            ],
        },
        false,
    )
    .unwrap();
    let pages = c.edit_pages(&site).unwrap();
    assert!(pages.iter().find(|p| p.url == "/emplois/").unwrap().draft);
    c.build(&site, "production", false, &|_| {}).unwrap();
    assert!(
        !out_dir(&site, "production").join("emplois").exists(),
        "brouillon non publié"
    );
    // Tout annulé : fichier de données et document d'origine.
    for _ in 0..3 {
        c.edit_undo(&site).unwrap();
    }
    assert_eq!(std::fs::read_to_string(&file).unwrap(), data);
    assert_eq!(c.edit_open(&site).unwrap().site, opened.site);
}

#[test]
fn nouveau_site_pret_a_modifier() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = d.path().join("atelier");
    let summary = c.new_site(&site, "Atelier Lumière", "fr").unwrap();
    assert_eq!(summary.pages, 1);
    assert!(site.join("media").is_dir());
    c.edit_open(&site).unwrap();
    let pages = c.edit_pages(&site).unwrap();
    assert_eq!(pages[0].url, "/");
    assert_eq!(pages[0].name.as_deref(), Some("Accueil"));
    // Réglages lisibles : données écrites dans le document, sans dossier de données (exemple
    // du titre de l'onglet dans Paramètres › Référencement).
    let rd = c
        .readable_data(&site, &[vec!["site".into(), "title".into()]], &[])
        .unwrap();
    assert_eq!(rd.values[0], Some(serde_json::json!("Atelier Lumière")));
    c.build(&site, "production", false, &|_| {}).unwrap();
    let out = out_dir(&site, "production");
    let html = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(html.contains("<title>Atelier Lumière</title>"), "{html}");
    assert!(
        html.contains(r#"<h1 class="titre">Atelier Lumière</h1>"#),
        "{html}"
    );
    assert!(out.join("assets/site.css").is_file());
    assert!(out.join("robots.txt").is_file());
}

#[test]
fn animations_lottie_importees() {
    use std::io::Write;
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = d.path().join("atelier");
    c.new_site(&site, "Atelier", "fr").unwrap();
    c.edit_open(&site).unwrap();
    let json = br#"{"v":"5.7.4","fr":30,"ip":0,"op":60,"w":200,"h":100,"nm":"x","layers":[]}"#;
    // Fichier `.json` de Bodymovin : animation ; autre JSON : refusé.
    let m = c.edit_import_media(&site, "Logo anime.json", json).unwrap();
    assert_eq!(m.url, "/media/logo-anime.json");
    assert_eq!(m.kind, revel_protocol::MediaKind::Animation);
    assert!(
        c.edit_import_media(&site, "donnees.json", b"{\"a\":1}")
            .is_err()
    );
    // Archive `.lottie` de LottieFiles : animation extraite en `.json`.
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let o = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        z.start_file("manifest.json", o).unwrap();
        z.write_all(br#"{"animations":[{"id":"a"}]}"#).unwrap();
        z.start_file("animations/a.json", o).unwrap();
        z.write_all(json).unwrap();
        z.finish().unwrap();
    }
    let m2 = c
        .edit_import_media(&site, "Bouton.lottie", buf.get_ref())
        .unwrap();
    assert_eq!(m2.url, "/media/bouton.json");
    assert_eq!(std::fs::read(site.join("media/bouton.json")).unwrap(), json);
    let list = c.edit_media(&site).unwrap();
    assert_eq!(
        list.iter()
            .filter(|f| f.kind == revel_protocol::MediaKind::Animation)
            .count(),
        2
    );
    // Lecteur servi au canevas.
    c.canvas_open(&site, Default::default()).unwrap();
    let js = c.canvas_respond("/assets/revel-lottie.js");
    assert_eq!(js.content_type, "text/javascript; charset=utf-8");
    assert!(
        String::from_utf8(js.body)
            .unwrap()
            .contains("window.RevelLottie")
    );
}

#[test]
fn medias_importes_et_remplaces() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = d.path().join("atelier");
    c.new_site(&site, "Atelier", "fr").unwrap();
    c.edit_open(&site).unwrap();
    let m = c.edit_import_media(&site, "Photo Été.JPG", b"abc").unwrap();
    assert_eq!(m.url, "/media/photo-ete.jpg");
    assert_eq!((m.kind, m.size), (revel_protocol::MediaKind::Image, 3));
    let m2 = c.edit_import_media(&site, "photo-ete.jpg", b"x").unwrap();
    assert_eq!(m2.url, "/media/photo-ete-2.jpg");
    assert!(c.edit_import_media(&site, "script.php", b"<?php").is_err());
    assert_eq!(c.edit_media(&site).unwrap().len(), 2);
    // Remplacement : même adresse, ancien fichier gardé, format identique exigé.
    let r = c
        .edit_replace_media(&site, "/media/photo-ete.jpg", "nouvelle.jpg", b"abcdef")
        .unwrap();
    assert_eq!((r.url.as_str(), r.size), ("/media/photo-ete.jpg", 6));
    assert_eq!(
        std::fs::read(site.join("media/photo-ete.jpg")).unwrap(),
        b"abcdef"
    );
    assert_eq!(
        std::fs::read_dir(site.join(".revel/corbeille"))
            .unwrap()
            .count(),
        1
    );
    assert!(
        c.edit_replace_media(&site, "/media/photo-ete.jpg", "autre.png", b"z")
            .is_err()
    );
    // Site sans dossier copié : `media/` créé et déclaré.
    let site2 = sample_site(d.path(), &c);
    c.edit_open(&site2).unwrap();
    let m3 = c.edit_import_media(&site2, "logo.svg", b"<svg/>").unwrap();
    assert_eq!(m3.url, "/media/logo.svg");
    assert!(
        c.edit_open(&site2)
            .unwrap()
            .site
            .passthrough
            .iter()
            .any(|p| p.from == "media")
    );
}

#[test]
fn modeles_de_site() {
    use revel_protocol::SiteTemplate;
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    // Vitrine : trois pages faites de blocs, menu dans l'en-tête.
    let v = d.path().join("vitrine");
    let s = c
        .new_site_from(&v, "Atelier", "fr", SiteTemplate::Showcase, false)
        .unwrap();
    assert_eq!(s.pages, 3);
    c.build(&v, "production", false, &|_| {}).unwrap();
    let out = out_dir(&v, "production");
    let home = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(
        home.contains(r#"<a href="/a-propos/">À propos</a>"#),
        "{home}"
    );
    assert!(
        home.contains(r#"aria-current="page">Accueil</a>"#),
        "{home}"
    );
    assert!(home.contains("bloc-avantages") && home.contains("bloc-appel"));
    let contact = std::fs::read_to_string(out.join("contact/index.html")).unwrap();
    assert!(contact.contains("Questions fréquentes"));
    let css = std::fs::read_to_string(out.join("assets/site.css")).unwrap();
    assert!(css.contains(".bloc-hero .bloc-titre") && css.contains(".entete__nav"));
    // Blog : collection d'articles (fichiers et .pages.yml), liste des trois plus récents.
    let b = d.path().join("blog");
    c.new_site_from(&b, "Carnet", "fr", SiteTemplate::Blog, false)
        .unwrap();
    assert!(b.join(".pages.yml").is_file());
    assert!(b.join("content/articles/bienvenue.md").is_file());
    c.build(&b, "production", false, &|_| {}).unwrap();
    let out = out_dir(&b, "production");
    let home = std::fs::read_to_string(out.join("index.html")).unwrap();
    let i1 = home.find("Un deuxième article").unwrap();
    let i2 = home.find("Bienvenue sur le blog").unwrap();
    assert!(i1 < i2, "plus récent d'abord");
    let art = std::fs::read_to_string(out.join("articles/bienvenue/index.html")).unwrap();
    assert!(
        art.contains("<p class=\"article__date\">05/01/2026</p>"),
        "{art}"
    );
    assert!(art.contains("Écrivez ici votre premier article"));
    let cols = c.cms(&b).unwrap();
    let a = cols.iter().find(|x| x.id == "articles").unwrap();
    assert_eq!(a.entries.len(), 2);
    assert!(a.fields.iter().any(|f| f.name == "description"));
}

const PAGES_YML: &str = "# Contenus\ncontent:\n  - name: projets\n    label: Projets\n    type: collection\n    path: contenus/projets\n    fields:\n      - name: title\n        type: string\n  - name: accueil\n    label: Accueil\n    type: file\n    path: donnees/pages/accueil.yml\n    fields:\n      - name: hero\n        type: object\n        fields:\n          - name: title\n            type: string\n";

#[test]
fn versions_du_site() {
    use revel_edit::Command;
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    c.edit_open(&site).unwrap();
    assert!(c.versions_status(&site).unwrap().is_none());
    let v1 = c.versions_init(&site, "Première version").unwrap();
    assert!(
        std::fs::read_to_string(site.join(".gitignore"))
            .unwrap()
            .contains(".revel/")
    );
    let s = c.versions_status(&site).unwrap().unwrap();
    assert_eq!((s.changes, s.remote.as_deref()), (0, None));
    // Modification dans l'éditeur : un fichier changé, comparé à la dernière version.
    let info = |name: &str| Command::SetSiteInfo {
        name: name.into(),
        base_url: None,
        default_locale: "fr".into(),
        typography: None,
    };
    c.edit_apply(&site, info("Nouveau nom"), false).unwrap();
    assert_eq!(c.versions_status(&site).unwrap().unwrap().changes, 1);
    let ch = c.versions_changes(&site, None, false).unwrap();
    assert_eq!(
        (ch[0].path.as_str(), ch[0].change.as_str()),
        ("site.revel.json", "modified")
    );
    let diff = c
        .versions_diff(&site, None, "site.revel.json")
        .unwrap()
        .unwrap();
    assert!(
        diff.iter()
            .any(|l| l.kind == '+' && l.text.contains("Nouveau nom"))
    );
    let v2 = c.versions_save(&site, "Renommé").unwrap().unwrap();
    assert_eq!(c.versions_list(&site, 10).unwrap().len(), 2);
    assert_eq!(
        c.versions_changes(&site, Some(&v2.id), true).unwrap().len(),
        1
    );
    // Restauration : session relue depuis le disque (ancien nom), l'éditeur continue.
    c.versions_restore(&site, &v1.id, "Avant restauration", "Restauration")
        .unwrap();
    assert_eq!(c.edit_site(&site).unwrap().name, "Essai");
    c.edit_apply(&site, info("Encore"), false).unwrap();
    assert_eq!(c.versions_list(&site, 10).unwrap().len(), 3);
    assert!(c.versions_sync(&site, "m", "f").is_err());
}

#[test]
fn synchronisation_automatique() {
    use revel_edit::Command;
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    c.edit_open(&site).unwrap();
    c.versions_init(&site, "Début").unwrap();
    let shared = d.path().join("partage");
    std::fs::create_dir_all(&shared).unwrap();
    c.versions_set_shared(&site, &shared).unwrap();
    c.versions_sync(&site, "m", "f").unwrap();
    assert!(!c.versions_incoming(&site).unwrap());
    // Équipier : copie du dépôt commun, nom du site changé, version envoyée.
    let remote = c.versions_status(&site).unwrap().unwrap().remote.unwrap();
    let mate = d.path().join("equipier");
    std::process::Command::new("git")
        .args(["clone", "-q", &remote, mate.to_str().unwrap()])
        .status()
        .unwrap();
    let path = mate.join(revel_project::SITE_FILE);
    let mut doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    doc["name"] = "Nom de Bruno".into();
    std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap() + "\n").unwrap();
    revel_git::versions::sync(&mate, None, "Bruno", "bruno@local", "Nom", "f").unwrap();
    // Ici : une modification en cours ailleurs (titre d'une page), annulable.
    let page = c.edit_site(&site).unwrap().pages[0].id.clone();
    c.edit_apply(
        &site,
        Command::SetPageName {
            id: page.clone(),
            name: Some("Accueil d'Anne".into()),
        },
        false,
    )
    .unwrap();
    assert!(c.versions_incoming(&site).unwrap());
    let before = c.versions_list(&site, 50).unwrap().len();
    let r = c.versions_receive(&site, "Fusion").unwrap();
    assert_eq!(
        (r.pulled, r.conflicts.len(), r.waiting),
        (1, 0, false),
        "{r:?}"
    );
    // Reçu sans version de plus que celle de Bruno ; changement en cours gardé.
    assert_eq!(c.versions_list(&site, 50).unwrap().len(), before + 1);
    let s = c.edit_site(&site).unwrap();
    assert_eq!(s.name, "Nom de Bruno");
    assert_eq!(s.pages[0].name.as_deref(), Some("Accueil d'Anne"));
    assert_eq!(c.versions_status(&site).unwrap().unwrap().changes, 1);
    // Ctrl+Z annule sa propre modification, celle de Bruno reste.
    c.edit_undo(&site).unwrap();
    let s = c.edit_site(&site).unwrap();
    assert_eq!(
        (s.name.as_str(), s.pages[0].name.as_deref()),
        ("Nom de Bruno", None)
    );
    // Même endroit des deux côtés : rien n'est changé, endroit à vérifier.
    c.edit_apply(
        &site,
        Command::SetPageName {
            id: page.clone(),
            name: Some("A".into()),
        },
        false,
    )
    .unwrap();
    let mut doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    revel_git::versions::receive(&mate, None).unwrap();
    doc = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap_or(doc);
    doc["pages"][0]["name"] = "B".into();
    std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap() + "\n").unwrap();
    revel_git::versions::sync(&mate, None, "Bruno", "bruno@local", "Titre", "f").unwrap();
    let r = c.versions_receive(&site, "Fusion").unwrap();
    assert_eq!(
        (r.pulled, r.conflicts.len(), r.items.len()),
        (0, 1, 1),
        "{r:?}"
    );
    assert_eq!(
        c.edit_site(&site).unwrap().pages[0].name.as_deref(),
        Some("A")
    );
}

#[test]
fn polices_importees_retirees_annulees() {
    use base64::Engine as _;
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    {
        let mut p = revel_project::Project::open(&site).unwrap();
        p.site.stylesheet = Some(revel_model::Stylesheet {
            url: "/css/site.css".into(),
            source: None,
            items: revel_style::parse(
                "@import url(\"/x.css\");\nh1 { font-family: \"Brand Sans\", sans-serif; }",
            )
            .unwrap(),
        });
        p.save().unwrap();
    }
    c.edit_open(&site).unwrap();
    let sheet0 = c.edit_site(&site).unwrap().stylesheet;
    let woff2 = |n: u8| {
        base64::engine::general_purpose::STANDARD.encode([b'w', b'O', b'F', b'2', n, n, n, n])
    };
    let up = |name: &str, weight: &str, italic: bool, n: u8| revel_protocol::FontUpload {
        name: name.into(),
        data: woff2(n),
        weight: weight.into(),
        italic,
    };
    // Fichier refusé (contenu qui n'est pas une police).
    let bad = revel_protocol::FontUpload {
        name: "x.woff2".into(),
        data: base64::engine::general_purpose::STANDARD.encode(b"<html>"),
        weight: "400".into(),
        italic: false,
    };
    assert!(c.fonts_add_files(&site, "Brand Sans", &[bad]).is_err());
    c.fonts_add_files(
        &site,
        "Brand Sans",
        &[
            up("BrandSans-Regular.woff2", "400", false, 1),
            up("BrandSans-BoldItalic.woff2", "700", true, 2),
        ],
    )
    .unwrap();
    let fonts = c.fonts(&site).unwrap();
    assert_eq!(fonts.len(), 1);
    assert_eq!(fonts[0].family, "Brand Sans");
    assert_eq!(fonts[0].uses, 1);
    assert_eq!(fonts[0].size, 16);
    assert_eq!(
        fonts[0]
            .faces
            .iter()
            .map(|f| (f.weight.as_str(), f.style.as_str()))
            .collect::<Vec<_>>(),
        vec![("400", "normal"), ("700", "italic")]
    );
    let file = site.join("media/fonts/brand-sans/brandsans-regular.woff2");
    assert!(file.is_file());
    // Règles après l'`@import` (sinon il ne serait plus lu).
    let sheet = c.edit_site(&site).unwrap().stylesheet.unwrap();
    assert!(
        matches!(&sheet.items[0], revel_model::StyleItem::Raw { css } if css.starts_with("@import"))
    );
    assert!(
        matches!(&sheet.items[1], revel_model::StyleItem::Raw { css } if css.contains("/media/fonts/brand-sans/brandsans-regular.woff2"))
    );
    // Retrait : règles retirées, fichiers rangés ; annuler remet tout.
    c.fonts_remove(&site, "brand sans").unwrap();
    assert!(c.fonts(&site).unwrap().is_empty());
    assert!(!file.exists());
    c.edit_undo(&site).unwrap();
    assert!(file.is_file());
    assert_eq!(c.fonts(&site).unwrap()[0].faces.len(), 2);
    // Annuler l'ajout : fichiers sortis du site, document identique.
    c.edit_undo(&site).unwrap();
    assert!(!file.exists());
    assert!(c.fonts(&site).unwrap().is_empty());
    assert_eq!(c.edit_site(&site).unwrap().stylesheet, sheet0);
}

#[test]
fn traduction_automatique_lots_et_relecture() {
    use revel_edit::Command;
    use revel_edit::canvas::{Binding, Seg, Source};
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    let data = "hero:\n  title: L'agence\n";
    {
        let mut p = revel_project::Project::open(&site).unwrap();
        p.site.data_dir = Some("donnees".into());
        p.save().unwrap();
        std::fs::create_dir_all(site.join("donnees/pages")).unwrap();
        std::fs::write(site.join("donnees/pages/accueil.yml"), data).unwrap();
    }
    c.edit_open(&site).unwrap();
    c.edit_apply(
        &site,
        Command::AddLocale {
            locale: revel_model::Locale::new("en"),
            index: None,
        },
        false,
    )
    .unwrap();
    let before = std::fs::read_to_string(site.join(revel_project::SITE_FILE)).unwrap();
    let k = |s: &str| Seg::Key(s.into());
    let path = vec![k("pages"), k("accueil"), k("hero"), k("title")];
    let binding = Binding {
        source: Source::LocaleData {
            locale: "en".into(),
        },
        path: path.clone(),
    };
    let key = revel_edit::locale::data_key(&path);
    let marks = || {
        c.edit_site(&site).unwrap().locales[0]
            .machine
            .iter()
            .cloned()
            .collect::<Vec<_>>()
    };
    // Deux lots d'une même opération : une seule étape d'annulation.
    c.edit_apply_grouped(
        &site,
        Command::Batch {
            commands: vec![
                Command::SetDataValue {
                    binding: binding.clone(),
                    value: Some(serde_json::json!("The agency")),
                },
                Command::MarkMachine {
                    locale: "en".into(),
                    keys: vec![key.clone()],
                    on: true,
                },
            ],
        },
        "auto-1",
    )
    .unwrap();
    c.edit_apply_grouped(
        &site,
        Command::SetTranslation {
            locale: "en".into(),
            source: "Bonjour".into(),
            text: Some("Hello".into()),
            machine: true,
        },
        "auto-1",
    )
    .unwrap();
    assert_eq!(marks(), vec![key.clone(), "text:Bonjour".to_string()]);
    let en = site.join("donnees/en/pages/accueil.yml");
    assert!(std::fs::read_to_string(&en).unwrap().contains("The agency"));
    // Valeur écrite à la main : marque retirée ; annuler la rétablit.
    c.edit_apply(
        &site,
        Command::SetBound {
            binding: binding.clone(),
            value: "Our agency".into(),
        },
        false,
    )
    .unwrap();
    assert_eq!(marks(), vec!["text:Bonjour".to_string()]);
    c.edit_undo(&site).unwrap();
    assert_eq!(marks(), vec![key.clone(), "text:Bonjour".to_string()]);
    // Une annulation retire toute l'opération (fichier de la langue compris).
    c.edit_undo(&site).unwrap();
    assert!(!en.exists());
    assert_eq!(
        std::fs::read_to_string(site.join(revel_project::SITE_FILE)).unwrap(),
        before
    );
    // Sans modèles : traduction indisponible, erreur explicite.
    if std::env::var_os("REVEL_MODELS").is_none() {
        assert!(!c.translate_can("fr", "en"));
        let e = c
            .translate(
                "fr",
                "en",
                &[revel_protocol::TranslateItem {
                    text: "Bonjour".into(),
                    format: None,
                }],
                &[],
            )
            .unwrap_err();
        assert_eq!(e.failure().key, "error-translate-pair");
    }
}

#[test]
fn langues_fichiers_et_generation() {
    use revel_edit::Command;
    use revel_edit::canvas::{Binding, Seg, Source};
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    let data = "hero:\n  title: L'agence   # titre\n  lien: /contact/\n";
    let entry = "---\ntitle: 'Wattr'\n---\n\nCorps\n";
    {
        let mut p = revel_project::Project::open(&site).unwrap();
        p.site.base_url = Some("https://exemple.fr".into());
        p.site.data_dir = Some("donnees".into());
        p.site.settings.typography = Some("fr".into());
        p.site.collections.push(revel_model::Collection {
            id: "projets".into(),
            source: revel_model::CollectionSource::Folder,
            folder: "contenus/projets".into(),
            path: Some("/projets/{{ title | lower }}/".into()),
            layout: None,
            props: Default::default(),
            filter: None,
            sort: None,
            fields: vec![],
        });
        p.site.pages[0].body.insert(1, el("head", &[], vec![]));
        p.site.pages[0]
            .body
            .push(el("p", &[], vec![out("pages.accueil.hero.title")]));
        p.save().unwrap();
        std::fs::create_dir_all(site.join("donnees/pages")).unwrap();
        std::fs::write(site.join("donnees/pages/accueil.yml"), data).unwrap();
        std::fs::create_dir_all(site.join("contenus/projets")).unwrap();
        std::fs::write(site.join("contenus/projets/wattr.md"), entry).unwrap();
        std::fs::write(site.join(".pages.yml"), PAGES_YML).unwrap();
    }
    c.edit_open(&site).unwrap();
    c.edit_apply(
        &site,
        Command::AddLocale {
            locale: revel_model::Locale::new("en"),
            index: None,
        },
        false,
    )
    .unwrap();
    let k = |s: &str| Seg::Key(s.into());
    let title = Binding {
        source: Source::LocaleData {
            locale: "en".into(),
        },
        path: vec![k("pages"), k("accueil"), k("hero"), k("title")],
    };
    // Texte à traduire : la langue n'a pas encore de fichier.
    let texts = c.edit_locale_data(&site, "en").unwrap();
    assert_eq!(texts.len(), 1, "{texts:?}");
    assert_eq!(texts[0].source, "L'agence");
    assert_eq!(texts[0].text, None);
    // Écriture dans la langue : fichier copié puis modifié (typographie française non retirée
    // en anglais), annuler le retire avec son dossier.
    c.edit_apply(
        &site,
        Command::SetBound {
            binding: title.clone(),
            value: "The agency’s".into(),
        },
        false,
    )
    .unwrap();
    let en = site.join("donnees/en/pages/accueil.yml");
    assert_eq!(
        std::fs::read_to_string(&en).unwrap(),
        data.replace("title: L'agence", "title: The agency’s")
    );
    // Pages CMS : entrée du fichier de la langue ajoutée (copie de celle de la langue principale).
    let cfg = || std::fs::read_to_string(site.join(".pages.yml")).unwrap();
    assert!(
        cfg().contains("  - name: accueil-en\n    label: Accueil (EN)\n    type: file\n    path: donnees/en/pages/accueil.yml\n"),
        "{}",
        cfg()
    );
    assert_eq!(
        std::fs::read_to_string(site.join("donnees/pages/accueil.yml")).unwrap(),
        data
    );
    assert_eq!(
        c.edit_locale_data(&site, "en").unwrap()[0].text.as_deref(),
        Some("The agency’s")
    );
    // Contenu d'une langue : copié depuis la langue principale.
    c.edit_apply(
        &site,
        Command::SetEntryField {
            folder: "contenus/en/projets".into(),
            file: "wattr".into(),
            key: "title".into(),
            value: Some(serde_json::json!("Wattr EN")),
        },
        false,
    )
    .unwrap();
    let md = site.join("contenus/en/projets/wattr.md");
    assert!(std::fs::read_to_string(&md).unwrap().contains("Wattr EN"));
    assert!(cfg().contains("  - name: projets-en\n    label: Projets (EN)\n    type: collection\n    path: contenus/en/projets\n"), "{}", cfg());
    let cms = c.cms(&site).unwrap();
    assert_eq!(cms[0].entries.len(), 1);
    assert_eq!(cms[0].locales["en"][0].data["title"], "Wattr EN");
    assert_eq!(
        cms[0].locales["en"][0].url.as_deref(),
        Some("/en/projets/wattr/")
    );

    // Génération : pages de la langue préfixées, fichiers de la langue lus, `hreflang`.
    let out = d.path().join("sortie");
    let p = revel_project::Project::open(&site).unwrap();
    p.build(&out, &revel_project::BuildOptions::default())
        .unwrap();
    let home = std::fs::read_to_string(out.join("en/index.html")).unwrap();
    assert!(home.contains("<p>The agency’s</p>"), "{home}");
    assert!(
        home.contains(r#"hreflang="en" href="https://exemple.fr/en/""#),
        "{home}"
    );
    assert!(std::fs::read_to_string(out.join("en/projets/wattr/index.html")).is_ok());
    let fr = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(fr.contains("<p>L&#39;agence</p>"), "{fr}");
    let sitemap = std::fs::read_to_string(out.join("sitemap.xml")).unwrap();
    assert!(
        sitemap.contains("https://exemple.fr/en/projets/wattr/"),
        "{sitemap}"
    );
    // Le dossier de la langue n'est pas une donnée de la langue principale.
    assert!(!p.load_data().unwrap().contains_key("en"));

    // Tout annuler : fichiers de la langue retirés, langue retirée.
    for _ in 0..3 {
        c.edit_undo(&site).unwrap();
    }
    assert!(!md.exists() && !site.join("contenus/en").exists());
    assert!(!en.exists() && !site.join("donnees/en").exists());
    assert_eq!(cfg(), PAGES_YML);
    assert!(
        revel_project::Project::open(&site)
            .unwrap()
            .site
            .locales
            .is_empty()
    );
}

#[test]
fn pages_modifiees_depuis_la_publication() {
    use revel_edit::Command;
    use revel_protocol::{ChangeKind, HostingTarget, SiteTemplate};
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let v = d.path().join("vitrine");
    c.new_site_from(&v, "Atelier", "fr", SiteTemplate::Showcase, false)
        .unwrap();
    c.edit_open(&v).unwrap();
    let key = v.display().to_string();
    let p = c
        .save_profile(
            &key,
            HostingProfile {
                sync_versions: false,
                site_url: Some("https://atelier.example".into()),
                preset: None,
                id: String::new(),
                name: "Hébergement".into(),
                environment: "production".into(),
                target: HostingTarget::Sftp {
                    host: "ftp.example".into(),
                    port: 22,
                    user: "atelier".into(),
                    remote_dir: "www/".into(),
                    known_host_key: None,
                },
            },
            Some("secret".into()),
        )
        .unwrap();
    // Jamais publié : aucune page signalée, adresse réglée, accès prêts.
    let st = c.publish_state(&v).unwrap();
    assert_eq!(st.len(), 1);
    assert!(st[0].published_at.is_none() && st[0].changes.is_empty() && st[0].ready);
    assert_eq!(st[0].url.as_deref(), Some("https://atelier.example"));
    // Publié : à jour.
    let snap = c.snapshot(&v, "production").unwrap();
    assert!(snap.pages.contains_key("/") && snap.pages.contains_key("/contact/"));
    c.record_publish(&v, &p.id, None, snap);
    let st = c.publish_state(&v).unwrap();
    assert!(
        st[0].published_at.is_some() && st[0].changes.is_empty(),
        "{st:?}"
    );
    // Page ajoutée, nom du site changé (toutes les pages) : signalés ; annulés : à jour.
    let page = revel_model::Page {
        id: "tarifs".into(),
        path: "/tarifs/".into(),
        body: Vec::new(),
        ..c.edit_open(&v).unwrap().site.pages[0].clone()
    };
    c.edit_apply(&v, Command::AddPage { page, index: None }, false)
        .unwrap();
    let st = c.publish_state(&v).unwrap();
    let added: Vec<_> = st[0]
        .changes
        .iter()
        .filter(|ch| ch.kind == ChangeKind::Added)
        .map(|ch| ch.url.as_str())
        .collect();
    assert_eq!(added, ["/tarifs/"], "{:?}", st[0].changes);
    c.edit_undo(&v).unwrap();
    assert!(c.publish_state(&v).unwrap()[0].changes.is_empty());
}

/// Connexion d'essai (5.3) et contrôles avant publication (5.4) contre le serveur d'essai
/// (variables `REVEL_SFTP_*`) : dossier proposé, accès refusés, publication arrêtée par une page
/// sans titre puis menée « quand même ».
#[tokio::test(flavor = "multi_thread")]
async fn connexion_d_essai_et_controles() {
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
    let _ = std::fs::remove_dir_all(Path::new(&home).join("revel-controles"));
    let _ = std::fs::remove_file(Path::new(&home).join(".deploy/revel-controles.manifest"));
    let mut profile = HostingProfile {
        id: String::new(),
        name: "OVHcloud".into(),
        environment: "production".into(),
        target: HostingTarget::Sftp {
            host,
            port: port.parse().unwrap(),
            user,
            remote_dir: String::new(),
            known_host_key: None,
        },
        sync_versions: false,
        site_url: None,
        preset: Some("ovh".into()),
    };
    let probe = c
        .hosting_probe(&site, &profile, Some(password.clone()))
        .await
        .unwrap();
    assert!(probe.server_key.contains(" ssh-"), "{}", probe.server_key);
    let suggested = probe.suggested.clone().unwrap();
    assert!(probe.folders.contains(&suggested), "{probe:?}");
    // Site sans adresse publique : dossier habituel de l'hébergeur s'il existe.
    if probe.folders.iter().any(|f| f == "www/") {
        assert_eq!(suggested, "www/");
    }
    let refused = c
        .hosting_probe(&site, &profile, Some("faux".into()))
        .await
        .unwrap_err();
    assert_eq!(refused.failure().key, "error-sftp-auth");

    if let HostingTarget::Sftp {
        remote_dir,
        known_host_key,
        ..
    } = &mut profile.target
    {
        *remote_dir = "revel-controles/".into();
        *known_host_key = Some(probe.server_key.clone());
    }
    let p = c.save_profile(&key, profile, Some(password)).unwrap();
    // Page sans titre : publication arrêtée avant toute connexion.
    let events = Mutex::new(Vec::new());
    let e = c
        .deploy(&site, &p.id, false, false, true, &|e| {
            events.lock().unwrap().push(e)
        })
        .await
        .unwrap_err();
    assert_eq!(e.failure().key, "error-publish-checks");
    assert!(events.lock().unwrap().iter().any(|e| matches!(
        e,
        DeployEvent::Checks { checks } if checks.iter().any(|c| c.kind == revel_protocol::CheckKind::NoTitle && c.page == "/")
    )));
    assert!(!Path::new(&home).join("revel-controles").exists());
    // Vérification : contrôles signalés, comparaison menée à terme.
    let s = c
        .deploy(&site, &p.id, true, false, true, &|_| {})
        .await
        .unwrap();
    assert!(s.dry_run && s.uploaded > 0);
    // Publier quand même.
    let s = c
        .deploy(&site, &p.id, false, false, false, &|_| {})
        .await
        .unwrap();
    assert!(!s.dry_run && !s.key_recorded);
    assert!(
        Path::new(&home)
            .join("revel-controles/index.html")
            .is_file()
    );
}

/// Revenir à la publication précédente (5.5) contre le serveur d'essai : deux publications,
/// retour à la première, puis retour annulé.
#[tokio::test(flavor = "multi_thread")]
async fn revenir_a_la_publication_precedente() {
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
    let remote = Path::new(&home).join("revel-retour");
    let _ = std::fs::remove_dir_all(&remote);
    let _ = std::fs::remove_file(Path::new(&home).join(".deploy/revel-retour.manifest"));
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
                    remote_dir: "revel-retour/".into(),
                    known_host_key: None,
                },
                sync_versions: false,
                site_url: None,
                preset: None,
            },
            Some(password),
        )
        .unwrap();
    let set_title = |t: &str| {
        let mut pr = revel_project::Project::open(&site).unwrap();
        pr.site.pages[0]
            .props
            .insert("title".into(), serde_json::json!(t));
        pr.save().unwrap();
    };
    let home_page = || std::fs::read_to_string(remote.join("index.html")).unwrap();
    // Aucune version précédente : refus.
    let e = c.rollback(&site, &p.id, &|_| {}).await.unwrap_err();
    assert_eq!(e.failure().key, "error-no-previous");
    c.deploy(&site, &p.id, false, false, false, &|_| {})
        .await
        .unwrap();
    assert!(home_page().contains("<h1>Accueil</h1>"));
    set_title("Bienvenue");
    c.deploy(&site, &p.id, false, false, false, &|_| {})
        .await
        .unwrap();
    assert!(home_page().contains("<h1>Bienvenue</h1>"));
    let state = c.publish_state(&site).unwrap();
    assert!(state[0].previous_at.is_some() && state[0].changes.is_empty());
    // Retour : première version en ligne ; la page d'accueil diffère désormais du site.
    let s = c.rollback(&site, &p.id, &|_| {}).await.unwrap();
    assert_eq!(s.uploaded, 1, "{s:?}");
    assert!(home_page().contains("<h1>Accueil</h1>"));
    let state = c.publish_state(&site).unwrap();
    assert_eq!(state[0].changes.len(), 1, "{:?}", state[0].changes);
    // Second retour : annule le premier.
    c.rollback(&site, &p.id, &|_| {}).await.unwrap();
    assert!(home_page().contains("<h1>Bienvenue</h1>"));
    assert!(c.publish_state(&site).unwrap()[0].changes.is_empty());
}

#[test]
fn dossier_des_projets_utilisable() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("Projects");
    assert!(crate::usable_dir(&dir));
    assert!(dir.is_dir());
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    // Un fichier à la place du dossier : inutilisable.
    let f = d.path().join("fichier");
    std::fs::write(&f, "x").unwrap();
    assert!(!crate::usable_dir(&f));
}

#[test]
fn suppression_d_un_site_refusee_hors_site() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let other = d.path().join("documents");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(other.join("note.txt"), "garder").unwrap();
    // Dossier sans site Revel : refusé, rien n'est touché.
    assert!(matches!(
        c.delete_site(&other.display().to_string()),
        Err(CoreError::NotASite)
    ));
    assert!(other.join("note.txt").is_file());
}

/// Mise à la corbeille réelle (dépend de la corbeille du système) : à lancer à la main avec
/// `cargo test -p revel-core -- --ignored corbeille`.
#[test]
#[ignore]
fn site_mis_a_la_corbeille() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = sample_site(d.path(), &c);
    let path = site.display().to_string();
    c.edit_open(&site).unwrap();
    c.delete_site(&path).unwrap();
    assert!(!site.exists());
    assert!(c.recent_sites().iter().all(|r| r.path != path));
}

/// Import d'un dépôt Git de pages HTML (7.2) : copie par son adresse `file://` (jamais de dépôt en
/// ligne dans les essais), examen, import, ouverture dans l'éditeur, page produite équivalente.
#[test]
fn depot_git_de_pages_html_importe() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let src = d.path().join("source");
    std::fs::create_dir_all(src.join("css")).unwrap();
    let page = |title: &str, main: &str| {
        format!(
            "<!DOCTYPE html><html lang=\"fr\"><head><meta charset=\"utf-8\"><title>{title}</title><link rel=\"stylesheet\" href=\"/css/site.css\"></head><body><header class=\"tete\"><a href=\"/\">Atelier</a></header><main>{main}</main><footer class=\"pied\">©</footer></body></html>"
        )
    };
    std::fs::write(src.join("index.html"), page("Accueil", "<h1>Bonjour</h1>")).unwrap();
    std::fs::write(
        src.join("contact.html"),
        page("Contact", "<h1>Contact</h1>"),
    )
    .unwrap();
    std::fs::write(src.join("css/site.css"), "h1 { color: red; }\n").unwrap();
    revel_git::versions::init(&src, "Essai", "essai@example.invalid", "Pages").unwrap();
    let p = src.display().to_string().replace('\\', "/");
    let url = format!("file://{}{p}", if p.starts_with('/') { "" } else { "/" });

    let parent = d.path().join("Projets");
    std::fs::create_dir_all(&parent).unwrap();
    let events = std::cell::Cell::new(0);
    let cloned = c
        .git_clone(&url, &parent, |_| events.set(events.get() + 1))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(cloned.name, "source");
    assert!(!cloned.license);
    assert!(events.get() > 0);
    let path = PathBuf::from(&cloned.path);
    let probe = c.probe_site(&path).unwrap();
    assert_eq!(probe.kind, revel_protocol::ImportKind::Static);
    assert_eq!(probe.pages, 2);
    // Adresse non reconnue, copie inconnue : refusées.
    assert!(matches!(
        c.git_clone("pas une adresse", &parent, |_| {}),
        Err(CoreError::BadRepoUrl)
    ));
    assert!(matches!(c.discard_clone(&src), Err(CoreError::NotAClone)));

    let r = c
        .import_site(
            &path,
            &revel_protocol::ImportOptions {
                lang: "en".into(),
                ..Default::default()
            },
            |_| {},
        )
        .unwrap();
    assert_eq!(r.site.name, "Source");
    assert!(path.join(".git").is_dir(), "historique gardé");
    // Une copie importée n'est plus retirable.
    assert!(matches!(c.discard_clone(&path), Err(CoreError::NotAClone)));
    let doc = c.edit_open(&path).unwrap();
    assert_eq!(doc.site.pages.len(), 2);
    assert_eq!(doc.site.layouts.len(), 2);
    let out = d.path().join("out");
    revel_project::Project::open(&path)
        .unwrap()
        .build(
            &out,
            &revel_project::BuildOptions {
                environment: "production".into(),
                images: false,
                progress: None,
            },
        )
        .unwrap();
    for f in ["index.html", "contact.html"] {
        let a = std::fs::read_to_string(src.join(f)).unwrap();
        let b = std::fs::read_to_string(out.join(f)).unwrap();
        let diffs = revel_import::html::compare(&a, &b, f, 5);
        assert!(diffs.is_empty(), "{f} : {diffs:?}\n{b}");
    }
    let css = std::fs::read_to_string(out.join("css/site.css")).unwrap();
    assert!(css.contains("color: red"), "{css}");
}

/// Copie abandonnée : retirée ; site à construire : signalé, pas importé.
#[test]
fn copie_abandonnee_et_site_a_construire() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let src = d.path().join("appli");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("package.json"),
        r#"{"devDependencies":{"@sveltejs/kit":"2"}}"#,
    )
    .unwrap();
    std::fs::write(src.join("README.md"), "# Appli").unwrap();
    revel_git::versions::init(&src, "Essai", "essai@example.invalid", "Appli").unwrap();
    let p = src.display().to_string().replace('\\', "/");
    let url = format!("file://{}{p}", if p.starts_with('/') { "" } else { "/" });
    let cloned = c.git_clone(&url, d.path(), |_| {}).unwrap();
    let path = PathBuf::from(&cloned.path);
    let probe = c.probe_site(&path).unwrap();
    assert_eq!(probe.kind, revel_protocol::ImportKind::NeedsBuild);
    assert_eq!(probe.tool.as_deref(), Some("SvelteKit"));
    assert!(matches!(
        c.import_site(
            &path,
            &revel_protocol::ImportOptions {
                lang: "fr".into(),
                ..Default::default()
            },
            |_| {}
        ),
        Err(CoreError::NeedsBuild(t)) if t == "SvelteKit"
    ));
    c.discard_clone(&path).unwrap();
    assert!(!path.exists());
    assert!(src.join("package.json").is_file(), "dépôt d'origine intact");
}

/// Accès réseau simulé : adresse → (type, contenu).
struct Net(std::collections::HashMap<String, (&'static str, Vec<u8>)>);

impl revel_import::crawl::Fetch for Net {
    fn get(
        &self,
        url: &revel_import::crawl::Url,
    ) -> std::result::Result<revel_import::crawl::Fetched, String> {
        let (ct, body) = self
            .0
            .get(url.as_str())
            .cloned()
            .unwrap_or(("text/plain", Vec::new()));
        Ok(revel_import::crawl::Fetched {
            status: if body.is_empty() { 404 } else { 200 },
            url: url.clone(),
            content_type: ct.into(),
            body,
        })
    }
}

fn net(entries: &[(&str, &'static str, &[u8])]) -> Net {
    Net(entries
        .iter()
        .map(|(u, ct, b)| (u.to_string(), (*ct, b.to_vec())))
        .collect())
}

fn build_to(path: &Path, out: &Path) -> revel_project::BuildReport {
    revel_project::Project::open(path)
        .unwrap()
        .build(
            out,
            &revel_project::BuildOptions {
                environment: "production".into(),
                images: false,
                progress: None,
            },
        )
        .unwrap()
}

/// Import depuis un lien (7.4) : site publié par Framer lu, importé, généré.
#[test]
fn import_depuis_un_lien() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let page = |title: &str, body: &str, css: &str| {
        format!(
            r#"<!doctype html><html lang="fr"><head><meta charset="utf-8"><meta name="generator" content="Framer 1"><title>{title}</title><link rel="preconnect" href="https://fonts.gstatic.com"><style data-framer-css-ssr-minified>body{{margin:0}}.framer-t{{font-size:40px}}{css}</style><script type="framer/appear" id="__framer__appearAnimationsContent">{{"a":{{"default":{{"initial":{{"opacity":0.001,"x":-40}},"animate":{{"opacity":1,"x":0,"transition":{{"type":"tween","delay":0,"duration":0.5}}}}}}}}}}</script></head><body><div id="main"><nav><a href="./">Accueil</a><a href="./a-propos">À propos</a></nav>{body}</div><script type="module" data-framer-bundle="main" src="https://framerusercontent.com/sites/1/script_main.mjs"></script></body></html>"#
        )
    };
    let home = page(
        "Studio",
        r#"<h1 class="framer-t" data-framer-appear-id="a" style="opacity:0.001;transform:translateX(-40px)">Studio</h1><img src="https://framerusercontent.com/images/abc.png?scale-down-to=512" alt="">"#,
        ".framer-home{padding:10px}",
    );
    let about = page("À propos", "<p>Équipe</p>", ".framer-about{padding:20px}");
    let png = [0x89u8, b'P', b'N', b'G', 1, 2, 3];
    let fetch = net(&[
        (
            "https://studio.framer.website/",
            "text/html",
            home.as_bytes(),
        ),
        (
            "https://studio.framer.website/a-propos",
            "text/html",
            about.as_bytes(),
        ),
        (
            "https://framerusercontent.com/images/abc.png?scale-down-to=512",
            "image/png",
            &png,
        ),
    ]);
    let parent = d.path().join("projets");
    std::fs::create_dir_all(&parent).unwrap();
    let pages = std::cell::Cell::new(0);
    let r = c
        .import_url_with("studio.framer.website", &parent, "fr", &fetch, |e| {
            pages.set(e.pages)
        })
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(pages.get(), 2);
    assert_eq!(r.stats.tool.as_deref(), Some("Framer"));
    assert_eq!(r.stats.pages, 2);
    assert_eq!(r.stats.animations, 1);
    assert!(r.stats.style_rules >= 4, "{:?}", r.stats);
    assert!(r.stats.files >= 1);
    let path = PathBuf::from(&r.site.path);
    assert!(path.starts_with(&parent));
    let out = d.path().join("out");
    build_to(&path, &out);
    let html = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(
        html.contains("data-revel-appear=\"left dur=0.5 delay=0 dist=40\""),
        "{html}"
    );
    assert!(html.contains("/assets/revel-animations.js"));
    assert!(
        !html.contains("framerusercontent.com/sites"),
        "moteur retiré : {html}"
    );
    assert!(
        !html.contains("https://framerusercontent.com/images"),
        "image locale : {html}"
    );
    assert!(!html.contains("preconnect"), "{html}");
    assert!(out.join("a-propos/index.html").is_file());
    let css = std::fs::read_to_string(out.join("assets/styles.css")).unwrap();
    assert!(
        css.contains(".framer-about") && css.contains(".framer-home"),
        "{css}"
    );
    // Site injoignable : dossier retiré.
    let before = std::fs::read_dir(&parent).unwrap().count();
    assert!(matches!(
        c.import_url_with("absent.example", &parent, "fr", &fetch, |_| {}),
        Err(CoreError::SiteUnreachable(_))
    ));
    assert_eq!(std::fs::read_dir(&parent).unwrap().count(), before);
    assert!(matches!(
        c.import_url_with("pas une adresse", &parent, "fr", &fetch, |_| {}),
        Err(CoreError::BadSiteUrl)
    ));
}

/// Import WordPress (7.6) : articles, pages, menu, images, site généré.
#[test]
fn import_wordpress() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let wxr = r#"<?xml version="1.0" encoding="UTF-8" ?>
<rss version="2.0" xmlns:excerpt="http://wordpress.org/export/1.2/excerpt/" xmlns:content="http://purl.org/rss/1.0/modules/content/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:wp="http://wordpress.org/export/1.2/">
<channel>
<title>Atelier Lumen</title><link>https://atelier.example</link><description>Céramiques</description><language>fr-FR</language>
<wp:base_site_url>https://atelier.example</wp:base_site_url><wp:base_blog_url>https://atelier.example</wp:base_blog_url>
<item><title>Bols d'hiver</title><link>https://atelier.example/2024/01/bols-d-hiver/</link><dc:creator><![CDATA[marie]]></dc:creator>
<content:encoded><![CDATA[<!-- wp:paragraph --><p>Une série de <a href="https://atelier.example/a-propos/">bols</a>.</p><!-- /wp:paragraph -->
<figure><img src="https://atelier.example/wp-content/uploads/2024/01/bol.jpg" alt=""/></figure>]]></content:encoded>
<excerpt:encoded><![CDATA[]]></excerpt:encoded><wp:post_id>10</wp:post_id><wp:post_date><![CDATA[2024-01-15 09:30:00]]></wp:post_date>
<wp:post_name><![CDATA[bols-d-hiver]]></wp:post_name><wp:status><![CDATA[publish]]></wp:status><wp:post_type><![CDATA[post]]></wp:post_type>
<category domain="category" nicename="ceramique"><![CDATA[Céramique]]></category></item>
<item><title>À propos</title><link>https://atelier.example/a-propos/</link>
<content:encoded><![CDATA[<p>L'atelier {{ Lumen }}.</p>]]></content:encoded><wp:post_id>12</wp:post_id>
<wp:post_name><![CDATA[a-propos]]></wp:post_name><wp:status><![CDATA[publish]]></wp:status><wp:post_type><![CDATA[page]]></wp:post_type></item>
</channel></rss>"#;
    let file = d.path().join("export.xml");
    std::fs::write(&file, wxr).unwrap();
    let probe = c.probe_wordpress(&file).unwrap();
    assert_eq!((probe.pages, probe.posts), (1, 1));
    let jpg = [0xffu8, 0xd8, 0xff, 0xe0, 1];
    let fetch = net(&[(
        "https://atelier.example/wp-content/uploads/2024/01/bol.jpg",
        "image/jpeg",
        &jpg,
    )]);
    let r = c
        .import_wordpress_with(&file, d.path(), "en", true, &fetch, |_| {})
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.site.name, "Atelier Lumen");
    assert_eq!(r.stats.entries, 1);
    assert_eq!(r.stats.files, 1);
    let path = PathBuf::from(&r.site.path);
    let out = d.path().join("out");
    let report = build_to(&path, &out);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let post = std::fs::read_to_string(out.join("2024/01/bols-d-hiver/index.html"))
        .unwrap_or_else(|_| panic!("article à son adresse d'origine"));
    assert!(post.contains("<a href=\"/a-propos/\">bols</a>"), "{post}");
    assert!(
        post.contains("src=\"/wp-content/uploads/2024/01/bol.jpg\""),
        "{post}"
    );
    assert!(
        !post.contains("article__resume"),
        "résumé non répété sous le titre : {post}"
    );
    assert!(out.join("wp-content/uploads/2024/01/bol.jpg").is_file());
    let about = std::fs::read_to_string(out.join("a-propos/index.html")).unwrap();
    assert!(
        about.contains("L'atelier {{ Lumen }}.") || about.contains("L&#39;atelier {{ Lumen }}."),
        "{about}"
    );
    let home = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(
        home.contains("Atelier Lumen") && home.contains("Bols d"),
        "{home}"
    );
    assert!(home.contains("href=\"/a-propos/\""), "menu : {home}");
    // Fichier qui n'est pas un export WordPress.
    std::fs::write(d.path().join("x.xml"), "<rss></rss>").unwrap();
    assert!(matches!(
        c.probe_wordpress(&d.path().join("x.xml")),
        Err(CoreError::NotWxr(_))
    ));
}

/// Import Figma (7.5) : jeton gardé, cadres lus, site généré.
#[test]
fn import_figma() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let file = serde_json::json!({
        "name": "Maquette",
        "components": {},
        "document": { "id": "0:0", "type": "DOCUMENT", "children": [
            { "id": "0:1", "name": "Page 1", "type": "CANVAS", "children": [
                { "id": "1:1", "name": "Desktop", "type": "FRAME",
                  "absoluteBoundingBox": { "x": 0, "y": 0, "width": 1440, "height": 900 },
                  "layoutMode": "VERTICAL", "itemSpacing": 16, "paddingTop": 64, "paddingLeft": 64, "paddingRight": 64, "paddingBottom": 64,
                  "children": [
                    { "id": "1:2", "name": "Titre", "type": "TEXT", "characters": "Bonjour",
                      "absoluteBoundingBox": { "x": 64, "y": 64, "width": 400, "height": 60 },
                      "layoutSizingHorizontal": "HUG", "layoutSizingVertical": "HUG",
                      "style": { "fontFamily": "Inter", "fontWeight": 700, "fontSize": 48 } }
                  ] }
            ] }
        ] }
    });
    let body = serde_json::to_vec(&file).unwrap();
    let fetch = net(&[(
        "https://api.figma.com/v1/files/AbCdEf1234567890",
        "application/json",
        &body,
    )]);
    assert!(!c.figma_has_token());
    assert!(matches!(
        c.figma_read_with(
            "https://www.figma.com/design/AbCdEf1234567890/Maquette",
            None,
            &fetch
        ),
        Err(CoreError::FigmaToken)
    ));
    let f = c
        .figma_read_with(
            "https://www.figma.com/design/AbCdEf1234567890/Maquette",
            Some("figd_essai"),
            &fetch,
        )
        .unwrap();
    assert!(c.figma_has_token(), "jeton gardé");
    assert_eq!(f.canvases[0].frames[0].width, 1440);
    let r = c
        .import_figma_with(
            "https://www.figma.com/design/AbCdEf1234567890/Maquette",
            &["1:1".to_string()],
            d.path(),
            "fr",
            &fetch,
            false,
            |_| {},
        )
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.stats.tool.as_deref(), Some("Figma"));
    let path = PathBuf::from(&r.site.path);
    let out = d.path().join("out");
    build_to(&path, &out);
    let html = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(html.contains("<h1 class=\"titre\">Bonjour</h1>"), "{html}");
    assert!(html.contains("<title>Desktop</title>"), "{html}");
    c.figma_forget_token().unwrap();
    assert!(!c.figma_has_token());
}

/// Export Webflow avec les contenus du CMS en CSV (7.3) : pages des contenus générées.
#[test]
fn export_webflow_et_cms() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = d.path().join("export");
    let page = |title: &str, main: &str| {
        format!(
            r#"<!DOCTYPE html><html data-wf-page="p" data-wf-site="65f0"><head><meta charset="utf-8"><title>{title}</title><meta content="Webflow" name="generator"><link href="css/site.webflow.css" rel="stylesheet" type="text/css"></head><body><div class="nav">Atelier</div>{main}</body></html>"#
        )
    };
    std::fs::create_dir_all(site.join("css")).unwrap();
    std::fs::write(site.join("css/site.webflow.css"), ".nav{color:red}").unwrap();
    std::fs::write(
        site.join("index.html"),
        page("Accueil", "<h1>Bienvenue</h1>"),
    )
    .unwrap();
    std::fs::write(
        site.join("detail_post.html"),
        page(
            "Article",
            r#"<h1 class="t w-dyn-bind-empty"></h1><div class="w-richtext w-dyn-bind-empty"></div>"#,
        ),
    )
    .unwrap();
    std::fs::write(
        site.join("Atelier - Posts - 1.csv"),
        "Name,Slug,Collection ID,Locale ID,Item ID,Archived,Draft,Created On,Updated On,Published On,Post Body\nBols,bols,c,l,i,false,false,x,x,x,<p>Texte du bol.</p>\n",
    )
    .unwrap();
    let probe = c.probe_site(&site).unwrap();
    assert_eq!(probe.made_with.as_deref(), Some("Webflow"));
    assert_eq!(probe.csv.len(), 1);
    let r = c
        .import_site(
            &site,
            &revel_protocol::ImportOptions {
                lang: "fr".into(),
                ..Default::default()
            },
            |_| {},
        )
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.stats.collections, 1);
    assert_eq!(r.stats.entries, 1);
    let out = d.path().join("out");
    build_to(&site, &out);
    let entry = std::fs::read_to_string(out.join("post/bols/index.html")).unwrap();
    assert!(entry.contains("<h1 class=\"t\">Bols</h1>"), "{entry}");
    assert!(entry.contains("<p>Texte du bol.</p>"), "{entry}");
    assert!(
        entry.contains("<div class=\"nav\">Atelier</div>"),
        "gabarit du site : {entry}"
    );
    assert!(!out.join("detail_post.html").exists());
}

/// Copies de sites GitHub sur cet ordinateur (D-127) : site connu et site rangé dans le dossier
/// des sites, retrouvés par l'adresse de leur dépôt distant (sans réseau).
#[test]
fn copies_des_sites_github() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let known = sample_site(d.path(), &c);
    c.versions_init(&known, "Début").unwrap();
    c.versions_set_remote(&known, "https://github.com/Compte/Vitrine.git")
        .unwrap();
    let projects = d.path().join("Projects");
    let other = projects.join("blog");
    c.new_site(&other, "Blog", "fr").unwrap();
    c.forget_site(&other.display().to_string());
    c.versions_init(&other, "Début").unwrap();
    c.versions_set_remote(&other, "git@github.com:compte/blog.git")
        .unwrap();
    let elsewhere = d.path().join("ailleurs");
    c.new_site(&elsewhere, "Ailleurs", "fr").unwrap();
    let found = c.local_github_copies(Some(&projects));
    assert_eq!(found.get("compte/vitrine"), Some(&known));
    assert_eq!(found.get("compte/blog"), Some(&other));
    assert_eq!(found.len(), 2);
}

/// Images en ligne d'un modèle (D-128) : trouvées dans les pages, composants, informations et
/// contenus du CMS ; remplacées en une étape d'annulation (sans réseau : adresses déjà copiées).
#[test]
fn images_en_ligne_remplacees() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = d.path().join("blog");
    c.new_site_template(&site, "Carnet", "fr", "blog", "auto")
        .unwrap();
    c.edit_open(&site).unwrap();
    let before = c.media_online(&site).unwrap();
    assert!(before.len() > 3, "{before:?}");
    let doc = c.edit_site(&site).unwrap();
    let entries_text = |s: &std::path::Path| {
        let mut all = String::new();
        for e in walkdir_md(&s.join("content")) {
            all.push_str(&std::fs::read_to_string(e).unwrap());
        }
        all
    };
    assert!(entries_text(&site).contains("images.unsplash.com"));
    let map: std::collections::BTreeMap<String, String> = before
        .iter()
        .enumerate()
        .map(|(i, u)| (u.clone(), format!("/media/photos/p{i}.jpg")))
        .collect();
    let list = crate::localize::commands(&site, &doc, &map);
    c.edit_apply(&site, revel_edit::Command::Batch { commands: list }, false)
        .unwrap();
    assert!(c.media_online(&site).unwrap().is_empty());
    assert!(!entries_text(&site).contains("images.unsplash.com"));
    c.edit_undo(&site).unwrap();
    assert_eq!(c.media_online(&site).unwrap(), before);
    assert!(entries_text(&site).contains("images.unsplash.com"));
}

fn walkdir_md(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walkdir_md(&p));
            } else if p.extension().is_some_and(|x| x == "md") {
                out.push(p);
            }
        }
    }
    out
}

/// Modèle partagé (D-132) : fichier léger (images en attente en ligne, icônes et polices
/// gardées, vidéo retirée, code laissé de côté), puis nouveau site d'après ce fichier.
#[test]
fn modele_partage_fichier_leger() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let site = d.path().join("blog");
    c.new_site_template(&site, "Carnet", "fr", "blog", "auto")
        .unwrap();
    c.edit_open(&site).unwrap();
    let online = c.media_online(&site).unwrap();
    let doc = c.edit_site(&site).unwrap();
    let map: std::collections::BTreeMap<String, String> = online
        .iter()
        .enumerate()
        .map(|(i, u)| (u.clone(), format!("/media/photos/p{i}.png")))
        .collect();
    let list = crate::localize::commands(&site, &doc, &map);
    c.edit_apply(&site, revel_edit::Command::Batch { commands: list }, false)
        .unwrap();
    // PNG de 1 × 1 pixel.
    let png = base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==")
        .unwrap();
    std::fs::create_dir_all(site.join("media/photos")).unwrap();
    for i in 0..online.len() {
        std::fs::write(site.join(format!("media/photos/p{i}.png")), &png).unwrap();
    }
    std::fs::create_dir_all(site.join("media/icons")).unwrap();
    std::fs::write(
        site.join("media/icons/star.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
    )
    .unwrap();
    std::fs::write(site.join("media/clip.mp4"), b"video").unwrap();
    std::fs::write(site.join("media/tracker.js"), b"alert(1)").unwrap();
    let pages = c.edit_site(&site).unwrap().pages.len();
    let share = revel_protocol::TemplateShare {
        name: "Mon carnet".into(),
        category: "editorial".into(),
        author: "Atelier".into(),
        license: "CC-BY-4.0".into(),
        preview: Some("media/photos/p0.png".into()),
        ..Default::default()
    };
    let dest = d.path().join("carnet.revel-template");
    let saved = c.template_save(&site, &share, &dest).unwrap();
    assert_eq!(saved.placeholders, online.len(), "{saved:?}");
    assert_eq!((saved.missing, saved.skipped), (1, 1), "{saved:?}");
    let info = c.template_file_info(&dest).unwrap();
    assert_eq!((info.name.as_str(), info.pages), ("Mon carnet", pages));
    assert!(
        info.preview
            .as_deref()
            .is_some_and(|p| p.starts_with("data:image/png;base64,"))
    );
    let (s, _) = c
        .create_site_shared(d.path(), "Nouveau", &format!("file:{}", dest.display()))
        .unwrap();
    let new = std::path::PathBuf::from(&s.path);
    assert_eq!(s.name, "Nouveau");
    assert!(new.join("media/icons/star.svg").exists());
    assert!(!new.join("media/photos/p0.png").exists());
    assert!(!new.join("media/tracker.js").exists() && !new.join("media/clip.mp4").exists());
    let doc = std::fs::read_to_string(new.join(revel_project::SITE_FILE)).unwrap();
    let mut entries = String::new();
    for e in walkdir_md(&new.join("content")) {
        entries.push_str(&std::fs::read_to_string(e).unwrap());
    }
    assert!(format!("{doc}{entries}").contains("https://placehold.co/10x10.webp"));
    assert!(!doc.contains("/media/photos/p0.png") && !entries.contains("media/photos/p"));
    assert_eq!(
        revel_project::Project::open(&new).unwrap().site.pages.len(),
        pages
    );
    // Images d'attente : pas proposées à la copie dans le site.
    c.edit_open(&new).unwrap();
    assert!(
        c.media_online(&new)
            .unwrap()
            .iter()
            .all(|u| !u.contains("placehold.co"))
    );
}

/// Serveur local qui imite l'API de GitHub (objets Git d'un dépôt) et ses fichiers bruts, pour
/// l'envoi d'un modèle dans un catalogue (D-132).
fn faux_github() -> (
    String,
    std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<String, Vec<u8>>>>,
) {
    use base64::Engine;
    use std::collections::BTreeMap;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let files: Arc<Mutex<BTreeMap<String, Vec<u8>>>> = Arc::default();
    let shared = files.clone();
    std::thread::spawn(move || {
        let mut blobs: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        let mut trees: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        let mut commits: BTreeMap<String, String> = BTreeMap::new();
        let mut head = "c0".to_string();
        commits.insert("c0".into(), "t0".into());
        trees.insert("t0".into(), Vec::new());
        let mut n = 0;
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut r = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            if r.read_line(&mut line).is_err() || line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let (method, path) = (
                parts.next().unwrap_or("").to_string(),
                parts.next().unwrap_or("").to_string(),
            );
            let mut len = 0usize;
            loop {
                let mut h = String::new();
                r.read_line(&mut h).unwrap();
                if h == "\r\n" || h.is_empty() {
                    break;
                }
                if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0u8; len];
            r.read_exact(&mut body).unwrap();
            let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
            n += 1;
            let repo = "/api/repos/tester/cat";
            let (status, out): (u16, Vec<u8>) = if path == "/api/user" {
                (200, br#"{"login":"tester","id":1}"#.to_vec())
            } else if path == repo {
                (200, br#"{"full_name":"tester/cat","private":false,"html_url":"x","clone_url":"x","default_branch":"main","permissions":{"push":true}}"#.to_vec())
            } else if path == format!("{repo}/git/ref/heads/main") {
                (
                    200,
                    serde_json::json!({ "object": { "sha": head } })
                        .to_string()
                        .into_bytes(),
                )
            } else if let Some(sha) = path.strip_prefix(&format!("{repo}/git/commits/")) {
                (
                    200,
                    serde_json::json!({ "tree": { "sha": commits[sha] } })
                        .to_string()
                        .into_bytes(),
                )
            } else if path == format!("{repo}/git/blobs") {
                let b = base64::engine::general_purpose::STANDARD
                    .decode(json["content"].as_str().unwrap())
                    .unwrap();
                let sha = format!("b{n}");
                blobs.insert(sha.clone(), b);
                (
                    201,
                    serde_json::json!({ "sha": sha }).to_string().into_bytes(),
                )
            } else if path == format!("{repo}/git/trees") {
                let mut entries = trees[json["base_tree"].as_str().unwrap()].clone();
                for e in json["tree"].as_array().unwrap() {
                    let p = e["path"].as_str().unwrap().to_string();
                    entries.retain(|(q, _)| *q != p);
                    entries.push((p, e["sha"].as_str().unwrap().to_string()));
                }
                let sha = format!("t{n}");
                trees.insert(sha.clone(), entries);
                (
                    201,
                    serde_json::json!({ "sha": sha }).to_string().into_bytes(),
                )
            } else if path == format!("{repo}/git/commits") && method == "POST" {
                assert_eq!(json["parents"][0].as_str(), Some(head.as_str()));
                let sha = format!("c{n}");
                commits.insert(sha.clone(), json["tree"].as_str().unwrap().to_string());
                (
                    201,
                    serde_json::json!({ "sha": sha }).to_string().into_bytes(),
                )
            } else if path == format!("{repo}/git/refs/heads/main") && method == "PATCH" {
                head = json["sha"].as_str().unwrap().to_string();
                let mut f = shared.lock().unwrap();
                f.clear();
                for (p, b) in &trees[&commits[&head]] {
                    f.insert(p.clone(), blobs[b].clone());
                }
                (200, b"{}".to_vec())
            } else if path == format!("{repo}/contents/catalog.json") {
                match shared.lock().unwrap().get("catalog.json") {
                    Some(b) => (
                        200,
                        serde_json::json!({ "content": base64::engine::general_purpose::STANDARD.encode(b) })
                            .to_string()
                            .into_bytes(),
                    ),
                    None => (404, b"{}".to_vec()),
                }
            } else if let Some(p) = path.strip_prefix("/raw/tester/cat/HEAD/") {
                let p = p.replace("%20", " ");
                match shared.lock().unwrap().get(&p) {
                    Some(b) => (200, b.clone()),
                    None => (404, b"404".to_vec()),
                }
            } else {
                (
                    404,
                    format!("{{\"message\":\"{method} {path}\"}}").into_bytes(),
                )
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                out.len()
            );
            let _ = stream.write_all(&out);
        }
    });
    (base, files)
}

/// Catalogue GitHub (D-132) : modèle complet envoyé (images comprises), listé, repris.
#[test]
fn modele_partage_catalogue() {
    let d = tempfile::tempdir().unwrap();
    let c = core(d.path());
    let (base, files) = faux_github();
    revel_git::github::override_api(Some(format!("{base}/api")));
    crate::sharing::override_raw(Some(format!("{base}/raw")));
    c.store_github_token(&revel_git::github::Token {
        access_token: "essai".into(),
        refresh_token: None,
        expires_at: None,
        refresh_expires_at: None,
        scope: String::new(),
    })
    .unwrap();
    let site = d.path().join("vitrine");
    c.new_site_template(&site, "Vitrine", "fr", "showcase", "auto")
        .unwrap();
    std::fs::create_dir_all(site.join("media/photos")).unwrap();
    std::fs::write(site.join("media/photos/a b.jpg"), b"jpeg").unwrap();
    std::fs::write(site.join("media/clip.mp4"), b"video").unwrap();
    std::fs::write(site.join("build.js"), b"x").unwrap();
    let share = revel_protocol::TemplateShare {
        name: "Vitrine claire".into(),
        category: "business".into(),
        license: "CC0-1.0".into(),
        ..Default::default()
    };
    let r = c.template_publish(&site, "tester/cat", &share).unwrap();
    assert!(
        !r.pull_request && r.url.contains("templates/vitrine-claire"),
        "{r:?}"
    );
    {
        let f = files.lock().unwrap();
        assert!(f.contains_key("catalog.json"));
        assert!(f.contains_key("templates/vitrine-claire/site/media/photos/a b.jpg"));
        assert!(f.contains_key("templates/vitrine-claire/site/media/clip.mp4"));
        assert!(!f.keys().any(|k| k.ends_with("build.js")));
    }
    let list = c.template_catalog("tester/cat").unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "catalog:tester/cat#vitrine-claire");
    assert_eq!(list[0].license, "CC0-1.0");
    let (s, _) = c
        .create_site_shared(d.path(), "Reprise", &list[0].id)
        .unwrap();
    let new = std::path::PathBuf::from(&s.path);
    assert_eq!(
        std::fs::read(new.join("media/photos/a b.jpg")).unwrap(),
        b"jpeg"
    );
    assert!(new.join("media/clip.mp4").exists() && !new.join("build.js").exists());
    // Deuxième envoi du même modèle : remplacé dans l'index, pas en double.
    c.template_publish(&site, "tester/cat", &share).unwrap();
    assert_eq!(c.template_catalog("tester/cat").unwrap().len(), 1);
    let other = revel_protocol::TemplateShare {
        name: "Vitrine sombre".into(),
        ..share.clone()
    };
    c.template_publish(&site, "tester/cat", &other).unwrap();
    assert_eq!(c.template_catalog("tester/cat").unwrap().len(), 2);
    revel_git::github::override_api(None);
    crate::sharing::override_raw(None);
}
