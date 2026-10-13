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
    },
    /// Importe un projet Eleventy : écrit `site.revel.json` à sa racine.
    ImportEleventy {
        dir: PathBuf,
        /// Affiche le rapport sans écrire le fichier.
        #[arg(long)]
        dry_run: bool,
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
        Command::New { dir, name } => {
            Project::create(&dir, Site::new(name, cli.lang.clone()))?;
            println!("{}", dir.join(revel_project::SITE_FILE).display());
        }
        Command::ImportEleventy { dir, dry_run } => {
            let profile = revel_import::eleventy::load_profile(&dir)?;
            let (site, report) = revel_import::eleventy::import(&dir, &profile)?;
            println!(
                "pages : {} · gabarits : {} · composants : {} · collections : {}",
                report.pages, report.layouts, report.components, report.collections
            );
            for w in &report.warnings {
                println!("! {w}");
            }
            if !dry_run {
                Project::create(&dir, site)?;
                println!("{}", dir.join(revel_project::SITE_FILE).display());
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
