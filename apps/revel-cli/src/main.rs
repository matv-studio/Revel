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
        Command::Build { dir, out, env } => {
            let project =
                Project::open(&dir).with_context(|| format!("ouverture de {}", dir.display()))?;
            let started = std::time::Instant::now();
            let report = project.build(&out, &env, None)?;
            for w in &report.warnings {
                println!("! {w}");
            }
            println!(
                "{} fichiers générés, {} copiés, en {:.1} s -> {}",
                report.generated,
                report.copied,
                started.elapsed().as_secs_f32(),
                out.display()
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
