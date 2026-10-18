//! `revel` en ligne de commande : mêmes modules que l'application, sans interface.
//! Sert aux tests automatiques (CI) et au diagnostic.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use revel_deploy::{SiteNeeds, check, providers};
use revel_i18n::Localizer;
use revel_model::Site;
use revel_project::Project;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "revel", version, about = "Revel : outils en ligne de commande")]
struct Cli {
    /// Langue des messages (en, fr).
    #[arg(long, global = true, default_value = "fr")]
    lang: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Affiche la version et les hébergeurs compilés.
    Info,
    /// Relevé de ce que l'interface ne crée ni ne règle dans un site (variables, expressions,
    /// conditions et listes écrites…) ; `--json` : liste complète.
    Audit {
        dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Reconstruit un site dans la forme de l'interface (données et calculs écrits, listes de
    /// collection et propriétés de composants gardées) ; `--dry-run` : relevé avant et après.
    Rebuild {
        dir: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Crée un projet vide.
    New {
        dir: PathBuf,
        #[arg(long, default_value = "Nouveau site")]
        name: String,
        /// Modèle : `vide`, `vitrine` ou `blog` (sans modèle : projet sans page).
        #[arg(long)]
        template: Option<String>,
        /// Fond clair (sombre par défaut).
        #[arg(long)]
        light: bool,
    },
    /// Importe un projet Eleventy : écrit `site.revel.json` à sa racine.
    ImportEleventy {
        dir: PathBuf,
        /// Affiche le rapport sans écrire le fichier.
        #[arg(long)]
        dry_run: bool,
        /// Garde le site tel qu'importé, sans le reconstruire dans la forme de l'interface.
        #[arg(long)]
        no_rebuild: bool,
    },
    /// Importe un site (Eleventy ou HTML statique, reconnu) : écrit `site.revel.json` à sa racine.
    Import {
        dir: PathBuf,
        /// Affiche le rapport sans écrire le fichier.
        #[arg(long)]
        dry_run: bool,
        /// Garde le site tel qu'importé, sans le reconstruire dans la forme de l'interface.
        #[arg(long)]
        no_rebuild: bool,
        /// Langue du site si ses pages n'en déclarent pas.
        #[arg(long, default_value = "en")]
        lang: String,
    },
    /// Lit un site publié (Framer, Webflow, tout site) et l'importe dans un nouveau dossier de
    /// `parent` (7.4) ; `--raw` : lecture seule (copie du site, sans import).
    ImportUrl {
        url: String,
        parent: PathBuf,
        #[arg(long, default_value = "en")]
        lang: String,
        #[arg(long)]
        raw: bool,
    },
    /// Construit un projet (SvelteKit, Astro, Next.js, Hugo, Jekyll…) puis importe le site
    /// produit dans un nouveau dossier de `parent` (D-88) : Node.js ou Hugo téléchargés au besoin
    /// dans `--tools` (par défaut le dossier de Revel en ligne de commande), dépendances
    /// installées. Le code du projet s'exécute sur cet ordinateur.
    ImportBuild {
        dir: PathBuf,
        parent: PathBuf,
        #[arg(long, default_value = "en")]
        lang: String,
        /// Dossier de réglages de Revel (outils, caches).
        #[arg(long)]
        config: Option<PathBuf>,
        /// Plan seulement (outils, gestionnaire), sans construire.
        #[arg(long)]
        plan: bool,
    },
    /// Importe un site WordPress depuis son fichier d'export (WXR) dans un nouveau dossier de
    /// `parent` (7.6).
    ImportWordpress {
        file: PathBuf,
        parent: PathBuf,
        #[arg(long, default_value = "en")]
        lang: String,
        /// Images gardées sur le site d'origine (pas de téléchargement).
        #[arg(long)]
        no_media: bool,
    },
    /// Importe un fichier Figma (7.5) : jeton dans la variable d'environnement `FIGMA_TOKEN`,
    /// cadres choisis (`--frames 1:2,1:3`, sinon ceux de la première page).
    ImportFigma {
        url: String,
        parent: PathBuf,
        #[arg(long, default_value = "en")]
        lang: String,
        #[arg(long)]
        frames: Option<String>,
    },
    /// Fidélité d'un import HTML statique : génère le site importé et compare chaque page produite
    /// à la page d'origine (structure lue comme un navigateur).
    ImportCheck {
        dir: PathBuf,
        /// Différences affichées par page.
        #[arg(long, default_value_t = 5)]
        limit: usize,
    },
    /// Applique des commandes d'édition (fichier JSON : une commande ou une liste) et enregistre.
    Apply {
        dir: PathBuf,
        /// Fichier de commandes (`-` : entrée standard).
        commands: PathBuf,
    },
    /// Forme canonique d'un fichier CSS (comparaison d'équivalence).
    CssCanon { file: PathBuf },
    /// Page rendue pour l'éditeur (HTML avec repères ; `--slots` : liste des repères en JSON).
    Canvas {
        dir: PathBuf,
        url: String,
        #[arg(long)]
        slots: bool,
        /// Exécuter les scripts du site.
        #[arg(long)]
        scripts: bool,
    },
    /// Sert le canevas de l'éditeur en HTTP local (127.0.0.1), comme le protocole de l'application :
    /// diagnostic et essais de bout en bout.
    CanvasServe {
        dir: PathBuf,
        #[arg(long, default_value_t = 0)]
        port: u16,
        /// Exécuter les scripts du site.
        #[arg(long)]
        scripts: bool,
        /// Images en `<img>` simple (comme `build --no-images`).
        #[arg(long)]
        no_images: bool,
        /// Expose aussi les commandes d'édition (`POST /__revel/invoke/…`) pour l'interface
        /// ouverte dans un navigateur (essais) : modifie le site.
        #[arg(long)]
        bridge: bool,
        /// Serveur MCP des assistants sur ce port (même cœur que le pont ; clé lue dans
        /// REVEL_MCP_TOKEN, sinon créée et affichée).
        #[arg(long)]
        mcp_port: Option<u16>,
    },
    /// Serveur MCP des assistants IA sur un site, sans l'application : entrée et sortie standard,
    /// ou HTTP local avec `--http PORT` (clé lue dans REVEL_MCP_TOKEN, sinon créée et affichée).
    Mcp {
        dir: PathBuf,
        #[arg(long)]
        http: Option<u16>,
    },
    /// Vérifie que chaque page du canevas, repères retirés, est identique à la page générée.
    CanvasCheck {
        dir: PathBuf,
        #[arg(long, default_value = "production")]
        env: String,
    },
    /// Génère un projet dans un dossier.
    Build {
        dir: PathBuf,
        #[arg(long, default_value = "_site")]
        out: PathBuf,
        /// Environnement (`production`, `staging`…).
        #[arg(long, default_value = "production")]
        env: String,
        /// Ne pas produire les variantes d'images.
        #[arg(long)]
        no_images: bool,
    },
    /// Publie un dossier par SFTP. Mot de passe lu dans la variable REVEL_SFTP_PASSWORD.
    DeploySftp {
        /// Dossier généré à publier.
        dir: PathBuf,
        #[arg(long)]
        host: String,
        #[arg(long, default_value_t = 22)]
        port: u16,
        #[arg(long)]
        user: String,
        /// Dossier distant, relatif au dossier personnel (ex. www/).
        #[arg(long)]
        remote_dir: String,
        /// Clé attendue du serveur (format known_hosts) ; absente : clé reçue affichée.
        #[arg(long)]
        known_host: Option<String>,
        /// Renvoyer tous les fichiers.
        #[arg(long)]
        full_sync: bool,
        /// Afficher seulement ce qui serait envoyé et supprimé.
        #[arg(long)]
        dry_run: bool,
    },
    /// Liste les avertissements d'un hébergeur pour des besoins donnés.
    CheckHost {
        host: String,
        #[arg(long, default_value_t = 0)]
        redirects: usize,
        #[arg(long, default_value_t = 0)]
        php: usize,
        #[arg(long, default_value_t = 0)]
        bytes: u64,
        #[arg(long, default_value_t = 0)]
        files: u64,
        #[arg(long)]
        htaccess: bool,
        #[arg(long)]
        private: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let l = Localizer::new(&cli.lang);
    match cli.command {
        Command::Info => {
            println!("{} — {}", l.t("app-title", &[]), l.t("app-tagline", &[]));
            println!(
                "{}",
                l.t("app-version", &[("version", env!("CARGO_PKG_VERSION"))])
            );
            for caps in providers::all() {
                println!("  - {}", caps.id);
            }
        }
        Command::Audit { dir, json } => {
            let p = Project::open(&dir)?;
            let issues = revel_edit::editable::audit(&p.site);
            if json {
                println!("{}", serde_json::to_string_pretty(&issues)?);
            } else {
                for (k, n) in revel_edit::editable::summary(&issues) {
                    println!("{k:?} : {n}");
                }
                for i in &issues {
                    println!(
                        "{} {:?} {} {}",
                        i.owner,
                        i.kind,
                        i.node.as_deref().unwrap_or("-"),
                        i.detail
                    );
                }
            }
        }
        Command::Rebuild { dir, dry_run } => {
            let mut p = Project::open(&dir)?;
            let before = revel_edit::editable::audit(&p.site).len();
            let (site, notes) = p.rebuilt().map_err(|e| anyhow::anyhow!("{e}"))?;
            let issues = revel_edit::editable::audit(&site);
            for n in &notes {
                println!("{n}");
            }
            for (k, n) in revel_edit::editable::summary(&issues) {
                println!("{k:?} : {n}");
            }
            println!("relevé : {before} -> {}", issues.len());
            if !dry_run {
                p.site = site;
                p.save()?;
            }
        }
        Command::New {
            dir,
            name,
            template,
            light,
        } => {
            match template.as_deref() {
                None => {
                    Project::create(&dir, Site::new(name, cli.lang.clone()))?;
                }
                Some(t) => {
                    let template = match t {
                        "vitrine" | "showcase" => revel_protocol::SiteTemplate::Showcase,
                        "blog" => revel_protocol::SiteTemplate::Blog,
                        "vide" | "blank" => revel_protocol::SiteTemplate::Blank,
                        other => anyhow::bail!("modèle inconnu : {other} (vide, vitrine, blog)"),
                    };
                    let config =
                        std::env::temp_dir().join(format!("revel-new-{}", std::process::id()));
                    let core = revel_core::Core::new(
                        &config,
                        Box::new(revel_core::secrets::Memory::default()),
                    );
                    core.new_site_from(&dir, &name, &cli.lang, template, light)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                }
            }
            println!("{}", dir.join(revel_project::SITE_FILE).display());
        }
        Command::ImportEleventy {
            dir,
            dry_run,
            no_rebuild,
        } => {
            let profile = revel_import::eleventy::load_profile(&dir)?;
            let (site, report) = revel_import::eleventy::import(&dir, &profile)?;
            println!(
                "pages : {} · gabarits : {} · composants : {} · collections : {} · règles de style : {}",
                report.pages,
                report.layouts,
                report.components,
                report.collections,
                report.style_rules
            );
            for w in &report.warnings {
                println!("! {w}");
            }
            if !dry_run {
                let mut p = Project::create(&dir, site)?;
                if !no_rebuild && let Some(w) = revel_core::rebuild_for_ui(&mut p) {
                    println!("! {w}");
                }
                println!("{}", dir.join(revel_project::SITE_FILE).display());
            }
        }
        Command::Import {
            dir,
            dry_run,
            no_rebuild,
            lang,
        } => match revel_import::detect(&dir) {
            revel_import::SiteKind::Eleventy => {
                anyhow::bail!("projet Eleventy : utiliser `revel import-eleventy`")
            }
            revel_import::SiteKind::Static { .. } => {
                let name = dir
                    .canonicalize()
                    .ok()
                    .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
                    .unwrap_or_else(|| "Site".into());
                let (site, report) = revel_import::html::import(
                    &dir,
                    &revel_import::html::Options {
                        name,
                        lang,
                        ..Default::default()
                    },
                )?;
                println!(
                    "pages : {} · gabarits : {} · règles de style : {} · fichiers copiés : {}",
                    report.pages,
                    report.layouts,
                    report.style_rules,
                    site.passthrough.len()
                );
                for w in &report.warnings {
                    println!("! {w}");
                }
                if !dry_run {
                    let mut p = Project::create(&dir, site)?;
                    if !no_rebuild && let Some(w) = revel_core::rebuild_for_ui(&mut p) {
                        println!("! {w}");
                    }
                    println!("{}", dir.join(revel_project::SITE_FILE).display());
                }
            }
            revel_import::SiteKind::NeedsBuild { tool, outputs } => anyhow::bail!(
                "site à construire avec {tool} (non importé) ; dossiers produits : {outputs:?}"
            ),
            revel_import::SiteKind::Unknown { outputs } => {
                anyhow::bail!("rien à importer ; dossiers de pages : {outputs:?}")
            }
        },
        Command::ImportUrl {
            url,
            parent,
            lang,
            raw,
        } => {
            std::fs::create_dir_all(&parent)?;
            if raw {
                let start = revel_import::crawl::parse_start(&url)
                    .ok_or_else(|| anyhow::anyhow!("adresse non reconnue"))?;
                let dest = parent.join(revel_import::crawl::site_name(&start));
                let r = revel_import::crawl::crawl(
                    &start,
                    &dest,
                    &revel_import::crawl::Http::new(),
                    &Default::default(),
                    &mut |p| {
                        eprint!(
                            "\r{} pages · {} fichiers · {} Ko   ",
                            p.pages,
                            p.files,
                            p.bytes / 1024
                        )
                    },
                )?;
                eprintln!();
                println!(
                    "{} : {} pages, {} fichiers, {} Ko, outil : {}",
                    dest.display(),
                    r.pages,
                    r.files,
                    r.bytes / 1024,
                    r.tool.as_deref().unwrap_or("?")
                );
                for w in &r.warnings {
                    println!("! {w}");
                }
                return Ok(());
            }
            let core = revel_core::Core::new(
                &std::env::temp_dir().join("revel-cli-config"),
                Box::new(revel_core::secrets::Memory::default()),
            );
            let r = core
                .import_url(&url, &parent, &lang, |e| {
                    eprint!(
                        "\r{} pages · {} fichiers · {} Ko   ",
                        e.pages,
                        e.files,
                        e.bytes / 1024
                    )
                })
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            eprintln!();
            print_import(&r);
        }
        Command::ImportBuild {
            dir,
            parent,
            lang,
            config,
            plan,
        } => {
            let core = revel_core::Core::new(
                &config.unwrap_or_else(|| std::env::temp_dir().join("revel-cli-config")),
                Box::new(revel_core::secrets::Memory::default()),
            );
            let p = core.build_plan(&dir).map_err(|e| anyhow::anyhow!("{e}"))?;
            println!(
                "{} · {} · {}{}",
                p.tool,
                p.tools
                    .iter()
                    .map(|t| format!(
                        "{} {}{}",
                        t.name,
                        t.version,
                        if t.installed {
                            " (présent)".to_string()
                        } else {
                            t.size
                                .map(|s| format!(" ({} Mo)", s / 1_000_000))
                                .unwrap_or_default()
                        }
                    ))
                    .collect::<Vec<_>>()
                    .join(", "),
                p.manager.as_deref().unwrap_or("-"),
                p.missing
                    .as_deref()
                    .map(|m| format!(" · manque : {m}"))
                    .unwrap_or_default()
            );
            if plan {
                return Ok(());
            }
            std::fs::create_dir_all(&parent)?;
            let r = core.build_import(&dir, &parent, &lang, |e| match &e.line {
                Some(l) => eprintln!("  {l}"),
                None if e.step == "download" && e.total > 0 => {
                    eprint!("\r  {} / {} Mo   ", e.done / 1_000_000, e.total / 1_000_000)
                }
                None => eprintln!("== {} {}", e.step, e.detail),
            });
            let r = r.map_err(|e| {
                let f = e.failure();
                if let Some(log) = f.args.iter().find(|(k, _)| k == "log").map(|(_, v)| v) {
                    eprintln!("{log}");
                }
                anyhow::anyhow!("{e}")
            })?;
            print_import(&r);
        }
        Command::ImportWordpress {
            file,
            parent,
            lang,
            no_media,
        } => {
            std::fs::create_dir_all(&parent)?;
            let core = revel_core::Core::new(
                &std::env::temp_dir().join("revel-cli-config"),
                Box::new(revel_core::secrets::Memory::default()),
            );
            let r = core
                .import_wordpress(&file, &parent, &lang, !no_media, |e| {
                    eprint!("\r{} fichiers   ", e.files)
                })
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            eprintln!();
            print_import(&r);
        }
        Command::ImportFigma {
            url,
            parent,
            lang,
            frames,
        } => {
            std::fs::create_dir_all(&parent)?;
            let token = std::env::var("FIGMA_TOKEN")
                .map_err(|_| anyhow::anyhow!("variable d'environnement FIGMA_TOKEN absente"))?;
            let core = revel_core::Core::new(
                &std::env::temp_dir().join("revel-cli-config"),
                Box::new(revel_core::secrets::Memory::default()),
            );
            let file = core
                .figma_read(&url, Some(&token))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let ids: Vec<String> = match frames {
                Some(f) => f.split(',').map(|s| s.trim().to_string()).collect(),
                None => file
                    .canvases
                    .first()
                    .map(|c| c.frames.iter().map(|f| f.id.clone()).collect())
                    .unwrap_or_default(),
            };
            let r = core
                .import_figma(&url, &ids, &parent, &lang, |e| {
                    eprint!("\r{} fichiers   ", e.files)
                })
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            eprintln!();
            print_import(&r);
        }
        Command::ImportCheck { dir, limit } => {
            let project =
                Project::open(&dir).with_context(|| format!("ouverture de {}", dir.display()))?;
            let out =
                std::env::temp_dir().join(format!("revel-import-check-{}", std::process::id()));
            project.build(
                &out,
                &revel_project::BuildOptions {
                    environment: "production".into(),
                    images: false,
                    progress: None,
                },
            )?;
            let mut same = 0;
            let pages = revel_import::html::scan(&dir);
            for rel in &pages {
                let produced_rel = {
                    let l = rel.to_ascii_lowercase();
                    if l.ends_with("index.htm") {
                        format!("{}l", rel)
                    } else {
                        rel.clone()
                    }
                };
                let original = std::fs::read_to_string(dir.join(rel)).unwrap_or_default();
                let produced = match std::fs::read_to_string(out.join(&produced_rel)) {
                    Ok(t) => t,
                    Err(_) => {
                        println!("✗ {rel} : page non produite");
                        continue;
                    }
                };
                let diffs = revel_import::html::compare(&original, &produced, rel, limit);
                if diffs.is_empty() {
                    same += 1;
                } else {
                    println!("✗ {rel}");
                    for d in diffs {
                        println!("    {d}");
                    }
                }
            }
            println!("{same}/{} pages équivalentes", pages.len());
            let _ = std::fs::remove_dir_all(&out);
        }
        Command::Apply { dir, commands } => {
            let text = if commands.as_os_str() == "-" {
                let mut t = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut t)?;
                t
            } else {
                std::fs::read_to_string(&commands)
                    .with_context(|| format!("lecture de {}", commands.display()))?
            };
            let value: serde_json::Value = serde_json::from_str(&text).context("JSON invalide")?;
            let list: Vec<revel_edit::Command> = match value {
                serde_json::Value::Array(_) => serde_json::from_value(value)?,
                v => vec![serde_json::from_value(v)?],
            };
            let mut project =
                Project::open(&dir).with_context(|| format!("ouverture de {}", dir.display()))?;
            let n = list.len();
            revel_edit::apply(
                &mut project.site,
                &revel_edit::Command::Batch { commands: list },
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            project.save()?;
            println!("{n} commande(s) appliquée(s)");
        }
        Command::CssCanon { file } => {
            let css = std::fs::read_to_string(&file)
                .with_context(|| format!("lecture de {}", file.display()))?;
            print!(
                "{}",
                revel_style::canonical(&css)
                    .map_err(|e| anyhow::anyhow!("{}: {e}", file.display()))?
            );
        }
        Command::Canvas {
            dir,
            url,
            slots,
            scripts,
        } => {
            let project =
                Project::open(&dir).with_context(|| format!("ouverture de {}", dir.display()))?;
            let hooks = std::sync::Arc::new(revel_project::ProjectAssets::new(&project));
            let page = project
                .render_editor(
                    &url,
                    hooks,
                    "production",
                    &revel_render::EditorOptions { scripts },
                )?
                .with_context(|| format!("aucune page HTML à l'adresse {url}"))?;
            if slots {
                println!("{}", serde_json::to_string_pretty(&page.slots)?);
            } else {
                print!("{}", page.html);
            }
        }
        Command::CanvasServe {
            dir,
            port,
            scripts,
            no_images,
            bridge,
            mcp_port,
        } => {
            let config = std::env::temp_dir().join(format!("revel-canvas-{}", std::process::id()));
            let core = std::sync::Arc::new(revel_core::Core::new(
                &config,
                Box::new(revel_core::secrets::Memory::default()),
            ));
            core.edit_open(&dir)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("ouverture de {}", dir.display()))?;
            core.canvas_open(
                &dir,
                revel_core::canvas::CanvasOptions {
                    scripts,
                    responsive_images: !no_images,
                },
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            let rt = tokio::runtime::Runtime::new()?;
            let _mcp = match mcp_port {
                Some(port) => Some(rt.block_on(start_mcp(core.clone(), port))?),
                None => None,
            };
            rt.block_on(revel_core::devserver::serve(
                core,
                dir,
                port,
                bridge,
                |base| println!("canevas : {base}"),
                async {
                    let _ = tokio::signal::ctrl_c().await;
                },
            ))?;
        }
        Command::Mcp { dir, http } => {
            let config = std::env::temp_dir().join(format!("revel-mcp-{}", std::process::id()));
            let core = std::sync::Arc::new(revel_core::Core::new(
                &config,
                Box::new(revel_core::secrets::Memory::default()),
            ));
            core.edit_open(&dir)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("ouverture de {}", dir.display()))?;
            let rt = tokio::runtime::Runtime::new()?;
            match http {
                // Entrée et sortie standard réservées au protocole : messages sur la sortie
                // d'erreur.
                None => {
                    let inv: std::sync::Arc<dyn revel_mcp::Invoke> =
                        std::sync::Arc::new(revel_mcp::direct::CoreInvoke(core));
                    rt.block_on(revel_mcp::relay::serve_stdio(inv))
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                }
                Some(port) => rt.block_on(async {
                    let server = start_mcp(core, port).await?;
                    let _ = tokio::signal::ctrl_c().await;
                    server.shutdown().await;
                    anyhow::Ok(())
                })?,
            }
        }
        Command::CanvasCheck { dir, env } => {
            let project =
                Project::open(&dir).with_context(|| format!("ouverture de {}", dir.display()))?;
            let (pages, diffs) = project.check_canvas(&env)?;
            for d in &diffs {
                eprintln!("{d}");
            }
            println!(
                "{pages} pages : {} identique(s) à la génération une fois les repères retirés, {} écart(s)",
                pages - diffs.len(),
                diffs.len()
            );
            if !diffs.is_empty() {
                std::process::exit(1);
            }
        }
        Command::Build {
            dir,
            out,
            env,
            no_images,
        } => {
            let project =
                Project::open(&dir).with_context(|| format!("ouverture de {}", dir.display()))?;
            let started = std::time::Instant::now();
            let last = std::sync::Mutex::new(std::time::Instant::now());
            let progress = |phase: revel_project::Phase, done: usize, total: usize| {
                if phase != revel_project::Phase::Images {
                    return;
                }
                if let Ok(mut l) = last.lock()
                    && (l.elapsed().as_secs() >= 5 || done == total)
                {
                    *l = std::time::Instant::now();
                    eprintln!("images : {done}/{total}");
                }
            };
            let report = project.build(
                &out,
                &revel_project::BuildOptions {
                    environment: env,
                    images: !no_images,
                    progress: Some(&progress),
                },
            )?;
            for w in &report.warnings {
                println!("! {w}");
            }
            println!(
                "{} fichiers générés, {} copiés ; images : {} produites, {} en cache, {} en échec ; {:.1} s -> {}",
                report.generated,
                report.copied,
                report.images.produced,
                report.images.from_cache,
                report.images.failed,
                started.elapsed().as_secs_f32(),
                out.display()
            );
        }
        Command::DeploySftp {
            dir,
            host,
            port,
            user,
            remote_dir,
            known_host,
            full_sync,
            dry_run,
        } => {
            let password = std::env::var("REVEL_SFTP_PASSWORD")
                .context("variable REVEL_SFTP_PASSWORD absente")?;
            let target = revel_deploy::sftp::SftpTarget {
                host,
                port,
                user,
                password,
                remote_dir,
                known_host_key: known_host,
            };
            let opts = revel_deploy::sftp::Options {
                full_sync,
                dry_run,
                parallel: 4,
            };
            let rt = tokio::runtime::Runtime::new()?;
            let report = rt.block_on(revel_deploy::sftp::deploy(&dir, &target, &opts, &|e| {
                use revel_deploy::sftp::Event;
                match e {
                    Event::Planned { upload, delete, bytes, total } => println!(
                        "Site : {total} fichiers. À envoyer : {upload} ({} Mo). À supprimer : {delete}.",
                        bytes / 1_048_576
                    ),
                    Event::Uploaded { done, total, .. } if done == total || done % 100 == 0 => {
                        eprintln!("envoi : {done}/{total}")
                    }
                    _ => {}
                }
            }))?;
            if target.known_host_key.is_none() {
                println!("Clé du serveur (à conserver) : {}", report.server_key);
            }
            println!(
                "Empreinte : {} ; remplacement : {:?}{}",
                report.server_fingerprint,
                report.replace,
                if report.dry_run {
                    " (aperçu, rien n'a été modifié)"
                } else {
                    ""
                }
            );
        }
        Command::CheckHost {
            host,
            redirects,
            php,
            bytes,
            files,
            htaccess,
            private,
        } => {
            let caps = providers::all()
                .into_iter()
                .find(|c| c.id == host)
                .with_context(|| format!("hébergeur inconnu : {host}"))?;
            let needs = SiteNeeds {
                redirects_301: redirects,
                php_files: php,
                custom_headers: htaccess,
                htaccess,
                total_bytes: bytes,
                largest_file_bytes: 0,
                file_count: files,
                wants_private_source: private,
            };
            for w in check(&caps, &needs) {
                let args: Vec<(&str, &str)> =
                    w.args.iter().map(|(k, v)| (*k, v.as_str())).collect();
                println!("[{:?}] {}", w.severity, l.t(w.key, &args));
            }
        }
    }
    Ok(())
}

/// Serveur MCP en HTTP local sur le cœur ; clé lue dans REVEL_MCP_TOKEN, sinon créée et affichée
/// (sortie d'erreur).
async fn start_mcp(
    core: std::sync::Arc<revel_core::Core>,
    port: u16,
) -> Result<revel_mcp::http::Running> {
    let token = std::env::var("REVEL_MCP_TOKEN")
        .ok()
        .filter(|t| !t.trim().is_empty());
    let shown = token.is_none();
    let token = token.unwrap_or_else(revel_mcp::config::new_token);
    let inv: std::sync::Arc<dyn revel_mcp::Invoke> =
        std::sync::Arc::new(revel_mcp::direct::CoreInvoke(core));
    let server = revel_mcp::http::start(inv, token.clone(), port).await?;
    eprintln!("MCP : http://127.0.0.1:{}/mcp", server.port);
    if shown {
        eprintln!("clé : {token}");
    }
    Ok(server)
}

/// Bilan d'un import.
fn print_import(r: &revel_protocol::ImportResult) {
    let s = &r.stats;
    println!(
        "{} · outil : {} · pages : {} · gabarits : {} · règles de style : {} · animations : {} · formulaires : {} · collections : {} ({} contenus) · fichiers : {}",
        r.site.path,
        s.tool.as_deref().unwrap_or("?"),
        s.pages,
        s.layouts,
        s.style_rules,
        s.animations,
        s.forms,
        s.collections,
        s.entries,
        s.files
    );
    for w in &r.warnings {
        println!("! {w}");
    }
}
