//! Commandes de construction lancées dans le dossier du projet : sortie lue ligne à ligne,
//! interruption (avec tous les processus lancés par la commande), délai maximal ; serveur
//! d'aperçu gardé en marche puis arrêté.
//!
//! Les commandes reçoivent un environnement réduit : `PATH`, dossiers du système, langue et
//! réglages de proxy et de certificats (réseau d'entreprise), sans les jetons de Revel ni ceux
//! de l'utilisateur.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

/// Commande à lancer.
#[derive(Debug, Clone)]
pub struct Cmd {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    /// Variables ajoutées à l'environnement réduit.
    pub env: Vec<(String, String)>,
    /// Dossiers placés en tête du `PATH`.
    pub path: Vec<PathBuf>,
}

impl Cmd {
    pub fn new(program: impl Into<PathBuf>, cwd: &Path) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: cwd.to_path_buf(),
            env: Vec::new(),
            path: Vec::new(),
        }
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn env(mut self, k: &str, v: impl Into<String>) -> Self {
        self.env.push((k.to_string(), v.into()));
        self
    }

    /// Ligne affichée dans le journal (`$ node npm-cli.js install`).
    pub fn display(&self) -> String {
        let name = self
            .program
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let args: Vec<String> = self
            .args
            .iter()
            .map(|a| {
                let short = Path::new(a)
                    .file_name()
                    .filter(|_| a.contains(['/', '\\']))
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| a.clone());
                if short.is_empty() || short.contains(' ') {
                    format!("\"{short}\"")
                } else {
                    short
                }
            })
            .collect();
        format!("$ {name} {}", args.join(" "))
    }
}

/// Variables de l'environnement de Revel transmises aux commandes.
fn kept(name: &str) -> bool {
    let n = name.to_ascii_uppercase();
    const KEEP: &[&str] = &[
        "PATH",
        "HOME",
        "USER",
        "USERNAME",
        "USERPROFILE",
        "HOMEDRIVE",
        "HOMEPATH",
        "APPDATA",
        "LOCALAPPDATA",
        "PROGRAMDATA",
        "PROGRAMFILES",
        "PROGRAMFILES(X86)",
        "COMMONPROGRAMFILES",
        "SYSTEMROOT",
        "SYSTEMDRIVE",
        "WINDIR",
        "COMSPEC",
        "PATHEXT",
        "OS",
        "PROCESSOR_ARCHITECTURE",
        "NUMBER_OF_PROCESSORS",
        "TEMP",
        "TMP",
        "TMPDIR",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
        "SHELL",
        "TERM",
        "XDG_CACHE_HOME",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "NODE_EXTRA_CA_CERTS",
        "GEM_HOME",
        "GEM_PATH",
        "RBENV_ROOT",
        "RBENV_VERSION",
        "RUBY_ROOT",
        "GOPATH",
        "GOROOT",
        "GOMODCACHE",
    ];
    KEEP.contains(&n.as_str())
        || n.ends_with("_PROXY")
        || (n.starts_with("NPM_CONFIG_") && n.contains("PROXY"))
}

fn sep() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

fn build(cmd: &Cmd) -> Command {
    let mut c = Command::new(&cmd.program);
    c.args(&cmd.args)
        .current_dir(&cmd.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear();
    let mut path = String::new();
    for (k, v) in std::env::vars() {
        if kept(&k) {
            if k.eq_ignore_ascii_case("PATH") {
                path = v;
            } else {
                c.env(&k, &v);
            }
        }
    }
    let mut dirs: Vec<String> = cmd.path.iter().map(|p| p.display().to_string()).collect();
    if !path.is_empty() {
        dirs.push(path);
    }
    c.env("PATH", dirs.join(sep()));
    for (k, v) in &cmd.env {
        c.env(k, v);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Groupe de processus propre : tous arrêtés ensemble.
        c.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Pas de fenêtre de console pour les commandes lancées depuis l'application.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

/// Arrête `child` et tous les processus qu'il a lancés.
pub fn kill_tree(child: &mut Child) {
    let pid = child.id();
    #[cfg(unix)]
    if pid > 1 {
        let _ = Command::new("kill")
            // `--` : le groupe `-pid` ne peut pas être lu comme une option (`-1` : tous les
            // processus).
            .args(["-s", "TERM", "--", &format!("-{pid}")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let end = Instant::now() + Duration::from_secs(3);
        while Instant::now() < end {
            if matches!(child.try_wait(), Ok(Some(_))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = Command::new("kill")
            .args(["-s", "KILL", "--", &format!("-{pid}")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Lignes de la sortie standard et de la sortie d'erreur, mêlées dans l'ordre d'arrivée.
fn read_lines(child: &mut Child) -> Receiver<String> {
    let (tx, rx) = channel();
    if let Some(out) = child.stdout.take() {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(out).split(b'\n').map_while(Result::ok) {
                let _ = tx.send(clean_line(&line));
            }
        });
    }
    if let Some(err) = child.stderr.take() {
        std::thread::spawn(move || {
            for line in BufReader::new(err).split(b'\n').map_while(Result::ok) {
                let _ = tx.send(clean_line(&line));
            }
        });
    }
    rx
}

/// Ligne lisible : séquences de couleur du terminal retirées, retour chariot final retiré.
fn clean_line(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for d in chars.by_ref() {
                    if d.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        if c != '\r' {
            out.push(c);
        }
    }
    out
}

/// Fin d'une commande en échec.
#[derive(Debug)]
pub enum RunError {
    /// Code de sortie (absent : arrêtée par le système).
    Failed(Option<i32>),
    Timeout,
    Cancelled,
    Spawn(std::io::Error),
}

/// Lance `cmd` jusqu'à sa fin ; chaque ligne de sortie passe par `on_line`.
pub fn run(
    cmd: &Cmd,
    on_line: &mut dyn FnMut(&str),
    cancel: &AtomicBool,
    limit: Duration,
) -> Result<(), RunError> {
    on_line(&cmd.display());
    let mut child = build(cmd).spawn().map_err(RunError::Spawn)?;
    let rx = read_lines(&mut child);
    let start = Instant::now();
    loop {
        match rx.recv_timeout(Duration::from_millis(150)) {
            Ok(line) => on_line(&line),
            Err(RecvTimeoutError::Disconnected) => {
                // Sorties fermées : attente de la fin du processus.
                let status = loop {
                    if cancel.load(Ordering::SeqCst) {
                        kill_tree(&mut child);
                        return Err(RunError::Cancelled);
                    }
                    if let Ok(Some(s)) = child.try_wait() {
                        break s;
                    }
                    if start.elapsed() > limit {
                        kill_tree(&mut child);
                        return Err(RunError::Timeout);
                    }
                    std::thread::sleep(Duration::from_millis(100));
                };
                return if status.success() {
                    Ok(())
                } else {
                    Err(RunError::Failed(status.code()))
                };
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
        if cancel.load(Ordering::SeqCst) {
            kill_tree(&mut child);
            return Err(RunError::Cancelled);
        }
        if start.elapsed() > limit {
            kill_tree(&mut child);
            return Err(RunError::Timeout);
        }
    }
}

/// Serveur en marche ; arrêté quand il est abandonné.
pub struct Server {
    child: Child,
    lines: Receiver<String>,
}

impl Server {
    /// Lance `cmd` sans attendre sa fin.
    pub fn spawn(cmd: &Cmd) -> std::io::Result<Self> {
        let mut child = build(cmd).spawn()?;
        let lines = read_lines(&mut child);
        Ok(Self { child, lines })
    }

    /// Lignes de sortie arrivées depuis le dernier appel.
    pub fn drain(&self) -> Vec<String> {
        self.lines.try_iter().collect()
    }

    /// Le serveur s'est arrêté (code de sortie).
    pub fn exited(&mut self) -> Option<Option<i32>> {
        self.child.try_wait().ok().flatten().map(|s| s.code())
    }

    pub fn stop(mut self) {
        kill_tree(&mut self.child);
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            kill_tree(&mut self.child);
        }
    }
}

/// Port libre de cette machine (127.0.0.1).
pub fn free_port() -> std::io::Result<u16> {
    let l = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    Ok(l.local_addr()?.port())
}

/// Commande présente dans le `PATH` de l'ordinateur.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT".into())
            .split(';')
            .map(|e| e.to_ascii_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    std::env::split_paths(&path).find_map(|d| {
        exts.iter().find_map(|e| {
            let p = d.join(format!("{name}{e}"));
            p.is_file().then_some(p)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn couleurs_retirees() {
        assert_eq!(
            clean_line(b"\x1b[32m\xe2\x9c\x93 built\x1b[39m in 2s\r"),
            "✓ built in 2s"
        );
    }

    #[test]
    fn environnement_reduit() {
        assert!(kept("PATH") && kept("https_proxy") && kept("npm_config_https_proxy"));
        assert!(
            !kept("GITHUB_TOKEN") && !kept("REVEL_MCP_TOKEN") && !kept("AWS_SECRET_ACCESS_KEY")
        );
    }

    #[cfg(unix)]
    #[test]
    fn commande_sortie_et_interruption() {
        let d = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        let mut lines = Vec::new();
        let ok = Cmd::new("/bin/sh", d.path())
            .args([
                "-c",
                "echo un; echo deux >&2; echo \"$REVEL_ESSAI\"; exit 0",
            ])
            .env("REVEL_ESSAI", "trois");
        run(
            &ok,
            &mut |l| lines.push(l.to_string()),
            &cancel,
            Duration::from_secs(20),
        )
        .unwrap();
        assert!(lines.iter().any(|l| l == "un") && lines.iter().any(|l| l == "deux"));
        assert!(lines.iter().any(|l| l == "trois"));
        let fail = Cmd::new("/bin/sh", d.path()).args(["-c", "exit 3"]);
        assert!(matches!(
            run(&fail, &mut |_| {}, &cancel, Duration::from_secs(20)),
            Err(RunError::Failed(Some(3)))
        ));
        // Délai dépassé : la commande et ses enfants sont arrêtés.
        let long = Cmd::new("/bin/sh", d.path()).args(["-c", "sleep 30 & sleep 30"]);
        let t = Instant::now();
        assert!(matches!(
            run(&long, &mut |_| {}, &cancel, Duration::from_millis(500)),
            Err(RunError::Timeout)
        ));
        assert!(t.elapsed() < Duration::from_secs(10));
    }
}
