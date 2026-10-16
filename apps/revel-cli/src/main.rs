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
        Command::ImportEleventy { dir, dry_run } => {
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
                Project::create(&dir, site)?;
                println!("{}", dir.join(revel_project::SITE_FILE).display());
            }
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
