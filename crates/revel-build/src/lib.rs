//! `revel-build` : construction des sites faits avec un générateur, quand l'utilisateur
//! l'accepte (le code du projet et de ses dépendances s'exécute sur l'ordinateur).
//!
//! - [`recipe`] reconnaît l'outil (SvelteKit, Astro, Next.js, Nuxt, Hugo, Jekyll…), le moteur,
//!   le gestionnaire de paquets, les dossiers produits et le serveur d'aperçu.
//! - [`toolchain`] télécharge Node.js et Hugo depuis leurs publications officielles, vérifie
//!   leur empreinte et les range dans le dossier de Revel. Jekyll utilise le Ruby de
//!   l'ordinateur.
//! - [`run`] lance les commandes (sortie lue, interruption, délai).
//! - [`published`] trouve l'adresse du site publié dans les fichiers du projet.
//!
//! [`build`] enchaîne : outils, dépendances, construction ; le résultat est un dossier de pages
//! ([`Outcome::Static`]) ou, pour un site rendu par un serveur, ce serveur en marche
//! ([`Outcome::Served`]) dont les pages sont ensuite lues comme celles d'un site en ligne.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

pub mod published;
pub mod recipe;
pub mod run;
pub mod toolchain;

pub use recipe::{Manager, Missing, Note, Recipe, Runtime, Serve};
use run::{Cmd, RunError, Server};
use toolchain::{Installed, Kind, Net};

/// Étape d'une construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Téléchargement d'un outil.
    Download,
    /// Installation des dépendances.
    Install,
    /// Construction.
    Build,
    /// Démarrage du serveur d'aperçu.
    Serve,
}

impl Step {
    pub fn id(self) -> &'static str {
        match self {
            Step::Download => "download",
            Step::Install => "install",
            Step::Build => "build",
            Step::Serve => "serve",
        }
    }
}

/// Avancement d'une construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Step(Step, String),
    /// Octets téléchargés (sur le total, s'il est connu).
    Bytes(u64, Option<u64>),
    /// Ligne de la sortie d'une commande.
    Line(String),
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("téléchargement impossible : {0}")]
    Download(String),
    #[error("fichier altéré : {0}")]
    Integrity(String),
    #[error("archive illisible : {0}")]
    Archive(String),
    #[error("système non pris en charge")]
    Platform,
    #[error("outil manquant : {0:?}")]
    Missing(Missing),
    #[error("échec de l'étape {step:?} (code {code:?})")]
    Failed {
        step: Step,
        code: Option<i32>,
        log: String,
    },
    #[error("étape {step:?} trop longue")]
    Timeout { step: Step, log: String },
    #[error("aucune page produite")]
    NoOutput { log: String },
    #[error("construction interrompue")]
    Cancelled,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl BuildError {
    /// Fin du journal des commandes, quand l'échec vient d'une commande.
    pub fn log(&self) -> Option<&str> {
        match self {
            BuildError::Failed { log, .. }
            | BuildError::Timeout { log, .. }
            | BuildError::NoOutput { log } => Some(log),
            _ => None,
        }
    }
}

/// Résultat d'une construction.
pub enum Outcome {
    /// Dossier des pages produites.
    Static(PathBuf),
    /// Serveur d'aperçu en marche à `url` ; fichiers du navigateur à copier après la lecture des
    /// pages (dossier, préfixe d'adresse).
    Served {
        url: String,
        server: Server,
        client_dirs: Vec<(PathBuf, String)>,
    },
}

/// Contexte d'une construction.
pub struct Ctx<'a> {
    /// Dossier des outils téléchargés.
    pub tools: &'a Path,
    /// Dossier des caches (npm, Corepack, Hugo).
    pub cache: &'a Path,
    pub net: &'a dyn Net,
    pub cancel: &'a AtomicBool,
    pub on: &'a dyn Fn(Event),
}

/// Durées maximales des étapes.
const INSTALL_LIMIT: Duration = Duration::from_secs(25 * 60);
const BUILD_LIMIT: Duration = Duration::from_secs(25 * 60);
const SERVE_LIMIT: Duration = Duration::from_secs(120);

/// Journal des dernières lignes (affiché en cas d'échec).
struct Log {
    lines: VecDeque<String>,
}

impl Log {
    fn new() -> Self {
        Self {
            lines: VecDeque::new(),
        }
    }
    fn push(&mut self, l: &str) {
        if self.lines.len() >= 200 {
            self.lines.pop_front();
        }
        self.lines.push_back(l.to_string());
    }
    fn tail(&self) -> String {
        self.lines.iter().cloned().collect::<Vec<_>>().join("\n")
    }
}

/// Outil `kind` installé, sinon téléchargé.
pub fn ensure(kind: Kind, ctx: &Ctx) -> Result<Installed, BuildError> {
    if let Some(t) = toolchain::find(ctx.tools, kind) {
        return Ok(t);
    }
    let rel = toolchain::latest(kind, ctx.net)?;
    (ctx.on)(Event::Step(
        Step::Download,
        format!("{} {}", kind.label(), rel.version),
    ));
    toolchain::install(
        &rel,
        ctx.tools,
        ctx.net,
        &mut |got, total| (ctx.on)(Event::Bytes(got, total.or(rel.size))),
        ctx.cancel,
    )
}

/// Variables communes aux commandes : caches dans le dossier de Revel, pas de questions, pas de
/// mesure d'utilisation des outils.
fn tool_env(cmd: Cmd, ctx: &Ctx) -> Cmd {
    let cache = |s: &str| ctx.cache.join(s).display().to_string();
    cmd.env("CI", "1")
        .env("npm_config_cache", cache("npm"))
        .env("npm_config_update_notifier", "false")
        .env("npm_config_fund", "false")
        .env("npm_config_audit", "false")
        .env("npm_config_yes", "true")
        .env("COREPACK_HOME", cache("corepack"))
        .env("COREPACK_ENABLE_DOWNLOAD_PROMPT", "0")
        .env("YARN_ENABLE_TELEMETRY", "0")
        .env("HUGO_CACHEDIR", cache("hugo"))
        .env("ASTRO_TELEMETRY_DISABLED", "1")
        .env("NEXT_TELEMETRY_DISABLED", "1")
        .env("NUXT_TELEMETRY_DISABLED", "1")
        .env("GATSBY_TELEMETRY_DISABLED", "1")
        .env("DO_NOT_TRACK", "1")
}

/// Commande npm, pnpm ou Yarn du projet (`args` : `install`, `run build`…).
fn package_cmd(node: &Installed, manager: &Manager, dir: &Path, args: &[&str], ctx: &Ctx) -> Cmd {
    let base = Cmd::new(node.program(), dir);
    let cmd = match manager {
        Manager::Npm => base
            .args([node.npm_cli().display().to_string()])
            .args(args.iter().copied()),
        Manager::Yarn { release: Some(rel) } => {
            base.args([rel.as_str()]).args(args.iter().copied())
        }
        Manager::Pnpm | Manager::Yarn { release: None } => {
            // Corepack (paquet npm) lance la version de pnpm ou de Yarn du projet.
            let pm = if matches!(manager, Manager::Pnpm) {
                "pnpm"
            } else {
                "yarn"
            };
            base.args([
                node.npm_cli().display().to_string(),
                "exec".into(),
                "--yes".into(),
                "--package=corepack@latest".into(),
                "--".into(),
                "corepack".into(),
                pm.into(),
            ])
            .args(args.iter().copied())
        }
    };
    let mut cmd = tool_env(cmd, ctx);
    cmd.path.push(node.bin_dir());
    cmd
}

fn run_step(
    step: Step,
    cmd: &Cmd,
    log: &mut Log,
    ctx: &Ctx,
    limit: Duration,
) -> Result<(), BuildError> {
    let r = run::run(
        cmd,
        &mut |l| {
            log.push(l);
            (ctx.on)(Event::Line(l.to_string()));
        },
        ctx.cancel,
        limit,
    );
    match r {
        Ok(()) => Ok(()),
        Err(RunError::Cancelled) => Err(BuildError::Cancelled),
        Err(RunError::Timeout) => Err(BuildError::Timeout {
            step,
            log: log.tail(),
        }),
        Err(RunError::Failed(code)) => Err(BuildError::Failed {
            step,
            code,
            log: log.tail(),
        }),
        Err(RunError::Spawn(e)) => {
            log.push(&format!("{} : {e}", cmd.program.display()));
            Err(BuildError::Failed {
                step,
                code: None,
                log: log.tail(),
            })
        }
    }
}

/// Construit le projet du dossier `dir` selon `recipe`.
pub fn build(dir: &Path, recipe: &Recipe, ctx: &Ctx) -> Result<Outcome, BuildError> {
    if let Some(m) = recipe.missing {
        return Err(BuildError::Missing(m));
    }
    let mut log = Log::new();
    let node = if recipe.runtime == Runtime::Node || recipe.manager.is_some() {
        Some(ensure(Kind::Node, ctx)?)
    } else {
        None
    };
    // Dépendances.
    if let (Some(node), Some(manager)) = (&node, &recipe.manager) {
        (ctx.on)(Event::Step(Step::Install, manager.name().into()));
        let args: &[&str] = match manager {
            Manager::Npm => &["install", "--no-audit", "--no-fund"],
            _ => &["install"],
        };
        run_step(
            Step::Install,
            &package_cmd(node, manager, dir, args, ctx),
            &mut log,
            ctx,
            INSTALL_LIMIT,
        )?;
    }
    // Construction.
    let building = || (ctx.on)(Event::Step(Step::Build, recipe.tool.clone()));
    match recipe.runtime {
        Runtime::Node => {
            building();
            let (node, manager) = (
                node.as_ref().expect("Node.js"),
                recipe.manager.as_ref().unwrap_or(&Manager::Npm),
            );
            let mut args = vec!["run"];
            args.extend(recipe.build.iter().map(String::as_str));
            run_step(
                Step::Build,
                &package_cmd(node, manager, dir, &args, ctx),
                &mut log,
                ctx,
                BUILD_LIMIT,
            )?;
        }
        Runtime::Hugo => {
            let hugo = ensure(Kind::Hugo, ctx)?;
            building();
            let mut cmd = tool_env(
                Cmd::new(hugo.program(), dir).args(recipe.build.clone()),
                ctx,
            );
            cmd.path.push(hugo.bin_dir());
            if let Some(n) = &node {
                cmd.path.push(n.bin_dir());
            }
            run_step(Step::Build, &cmd, &mut log, ctx, BUILD_LIMIT)?;
        }
        Runtime::Ruby => {
            let gemfile = dir.join("Gemfile").is_file();
            if gemfile {
                let bundle = run::which("bundle").ok_or(BuildError::Missing(Missing::Ruby))?;
                (ctx.on)(Event::Step(Step::Install, "Bundler".into()));
                let cfg = tool_env(
                    Cmd::new(&bundle, dir).args([
                        "config",
                        "set",
                        "--local",
                        "path",
                        "vendor/bundle",
                    ]),
                    ctx,
                );
                run_step(Step::Install, &cfg, &mut log, ctx, INSTALL_LIMIT)?;
                let install = tool_env(Cmd::new(&bundle, dir).args(["install"]), ctx);
                run_step(Step::Install, &install, &mut log, ctx, INSTALL_LIMIT)?;
                building();
                let b = tool_env(
                    Cmd::new(&bundle, dir)
                        .args(["exec", "jekyll"])
                        .args(recipe.build.clone()),
                    ctx,
                );
                run_step(Step::Build, &b, &mut log, ctx, BUILD_LIMIT)?;
            } else {
                let jekyll = run::which("jekyll").ok_or(BuildError::Missing(Missing::Ruby))?;
                building();
                let b = tool_env(Cmd::new(jekyll, dir).args(recipe.build.clone()), ctx);
                run_step(Step::Build, &b, &mut log, ctx, BUILD_LIMIT)?;
            }
        }
    }
    if let Some(out) = recipe::find_output(dir, &recipe.outputs) {
        return Ok(Outcome::Static(out));
    }
    // Pas de pages écrites : rendu par un serveur.
    let (Some(node), Some(serve)) = (&node, &recipe.serve) else {
        return Err(BuildError::NoOutput { log: log.tail() });
    };
    let port = run::free_port()?;
    let cmd = match serve {
        Serve::Script { name, args } => {
            // npm transmet les arguments placés après `--` ; pnpm et Yarn, ceux qui suivent le
            // nom du script.
            let mut a: Vec<String> = vec!["run".into(), name.clone()];
            if matches!(recipe.manager, None | Some(Manager::Npm)) {
                a.push("--".into());
            }
            a.extend(args.iter().map(|x| x.replace("{port}", &port.to_string())));
            let refs: Vec<&str> = a.iter().map(String::as_str).collect();
            package_cmd(
                node,
                recipe.manager.as_ref().unwrap_or(&Manager::Npm),
                dir,
                &refs,
                ctx,
            )
        }
        Serve::Entry(entry) => {
            let mut c = tool_env(Cmd::new(node.program(), dir).args([entry.as_str()]), ctx);
            c.path.push(node.bin_dir());
            c
        }
    }
    .env("PORT", port.to_string())
    .env("HOST", "127.0.0.1")
    .env("NITRO_PORT", port.to_string())
    .env("NITRO_HOST", "127.0.0.1")
    .env("NODE_ENV", "production");
    (ctx.on)(Event::Step(Step::Serve, format!("127.0.0.1:{port}")));
    log.push(&cmd.display());
    let mut server = Server::spawn(&cmd)?;
    let url = format!("http://127.0.0.1:{port}/");
    let start = Instant::now();
    loop {
        for l in server.drain() {
            log.push(&l);
            (ctx.on)(Event::Line(l));
        }
        if ctx.cancel.load(std::sync::atomic::Ordering::SeqCst) {
            server.stop();
            return Err(BuildError::Cancelled);
        }
        if let Some(code) = server.exited() {
            return Err(BuildError::Failed {
                step: Step::Serve,
                code,
                log: log.tail(),
            });
        }
        if answers(port) {
            break;
        }
        if start.elapsed() > SERVE_LIMIT {
            server.stop();
            return Err(BuildError::Timeout {
                step: Step::Serve,
                log: log.tail(),
            });
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    let client_dirs = recipe
        .client_dirs
        .iter()
        .map(|(d, p)| (dir.join(d), p.clone()))
        .filter(|(d, _)| d.is_dir())
        .collect();
    Ok(Outcome::Served {
        url,
        server,
        client_dirs,
    })
}

/// Le serveur local répond par une page (requête HTTP minimale).
fn answers(port: u16) -> bool {
    use std::io::{Read, Write};
    let Ok(mut s) = std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(500),
    ) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    if s.write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut head = [0u8; 16];
    matches!(s.read(&mut head), Ok(n) if n >= 12 && head.starts_with(b"HTTP/1."))
}
