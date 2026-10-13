//! `revel` en ligne de commande : mêmes modules que l'application, sans interface.
//! Sert aux tests automatiques (CI) et au diagnostic.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use revel_deploy::{SiteNeeds, check, providers};
use revel_i18n::Localizer;
use revel_model::Site;
use revel_render::{HtmlRenderer, Renderer};
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
    /// Crée un document de site vide.
    New {
        name: String,
        #[arg(long, default_value = "site.revel.json")]
        out: PathBuf,
    },
    /// Génère le HTML d'un document de site dans un dossier.
    Build {
        site: PathBuf,
        #[arg(long, default_value = "_site")]
        out: PathBuf,
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
        Command::New { name, out } => {
            let site = Site::new(name, cli.lang.clone());
            std::fs::write(&out, site.to_json()?)
                .with_context(|| format!("écriture de {}", out.display()))?;
            println!("{}", out.display());
        }
        Command::Build { site, out } => {
            let text = std::fs::read_to_string(&site)
                .with_context(|| format!("lecture de {}", site.display()))?;
            let site = Site::from_json(&text)?;
            for file in HtmlRenderer.render(&site)? {
                let path = out.join(&file.path);
                if let Some(dir) = path.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                std::fs::write(&path, &file.bytes)?;
                println!("{}", path.display());
            }
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
