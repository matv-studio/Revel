//! Publication par SFTP (port 22, transfert chiffré).
//!
//! Même fonctionnement que le script `deploy-sftp.sh` du site Alunites :
//! 1. connexion, vérification de la clé du serveur (ou première connexion : clé retournée
//!    pour être enregistrée) ;
//! 2. lecture du manifeste précédent (`~/.deploy/<dossier>.manifest`) ;
//! 3. manifeste provisoire (sans les fichiers à renvoyer) écrit avant toute modification ;
//! 4. envoi des fichiers modifiés, en remplacement atomique quand le serveur l'accepte
//!    (nom temporaire puis renommage) ;
//! 5. suppression des fichiers retirés du site ;
//! 6. manifeste complet écrit en dernier.

use crate::manifest::{self, Manifest, Plan};
use russh::keys::{HashAlg, PublicKey, PublicKeyOrCertificate};
use russh_sftp::client::RawSftpSession;
use russh_sftp::protocol::{FileAttributes, OpenFlags, Packet, StatusCode};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Paramètres de connexion. Le mot de passe est fourni par l'utilisateur (trousseau du système).
#[derive(Clone)]
pub struct SftpTarget {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    /// Dossier racine du site, relatif au dossier personnel (ex. `www/`).
    pub remote_dir: String,
    /// Clé attendue du serveur, au format `known_hosts` (`hôte type clé`) ou `type clé`.
    /// `None` : première connexion, la clé reçue est acceptée puis renvoyée dans le rapport.
    pub known_host_key: Option<String>,
}

impl std::fmt::Debug for SftpTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SftpTarget")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("user", &self.user)
            .field("password", &"***")
            .field("remote_dir", &self.remote_dir)
            .finish()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Options {
    /// Renvoyer tous les fichiers (ignorer le manifeste).
    pub full_sync: bool,
    /// Calculer seulement ce qui serait envoyé et supprimé.
    pub dry_run: bool,
    /// Envois simultanés (défaut 4).
    pub parallel: usize,
}

/// Remplacement des fichiers existants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Replace {
    /// Extension `posix-rename@openssh.com` : renommage par-dessus le fichier existant.
    PosixRename,
    /// Renommage SFTP standard accepté par-dessus un fichier existant.
    Rename,
    /// Écriture directe (le serveur refuse le renommage par-dessus un fichier).
    Direct,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Connected {
        server_key: String,
    },
    Planned {
        upload: usize,
        delete: usize,
        bytes: u64,
        total: usize,
    },
    Uploaded {
        done: usize,
        total: usize,
        path: String,
    },
    Deleted {
        done: usize,
        total: usize,
    },
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub plan: Plan,
    pub upload_bytes: u64,
    pub total_files: usize,
    /// Clé du serveur, au format `known_hosts`, à conserver pour les connexions suivantes.
    pub server_key: String,
    /// Empreinte SHA-256 de la clé (à comparer à celle annoncée par l'hébergeur).
    pub server_fingerprint: String,
    pub replace: Replace,
    pub dry_run: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SftpError {
    #[error("connexion à {0} impossible : {1}")]
    Connect(String, String),
    /// Pas de réponse du serveur dans le délai (adresse, port, pare-feu de l'hébergeur).
    #[error("pas de réponse de {0}")]
    Timeout(String),
    #[error("la clé du serveur ne correspond pas à la clé enregistrée (reçue : {0})")]
    HostKeyMismatch(String),
    #[error("identifiant ou mot de passe refusé")]
    Auth,
    #[error("SFTP : {0}")]
    Sftp(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("dossier distant invalide : « {0} » (attendu : chemin relatif non vide, ex. www/)")]
    RemoteDir(String),
    #[error("nom de fichier non pris en charge : {0}")]
    FileName(String),
}

fn sftp_err(e: impl std::fmt::Display) -> SftpError {
    SftpError::Sftp(e.to_string())
}

struct Client {
    expected: Option<String>,
    received: Arc<Mutex<Option<PublicKey>>>,
}

impl russh::client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = key else {
            return Ok(false);
        };
        if let Ok(mut r) = self.received.lock() {
            *r = Some(key.clone());
        }
        Ok(match &self.expected {
            None => true,
            Some(line) => key_matches(line, key),
        })
    }
}

/// La ligne `known_hosts` (ou `type clé`) contient-elle cette clé ?
fn key_matches(line: &str, key: &PublicKey) -> bool {
    let Ok(ours) = key.to_openssh() else {
        return false;
    };
    let mut ours_parts = ours.split_whitespace();
    let (Some(kt), Some(kb)) = (ours_parts.next(), ours_parts.next()) else {
        return false;
    };
    line.lines().any(|l| {
        let parts: Vec<&str> = l.split_whitespace().collect();
        parts.windows(2).any(|w| w[0] == kt && w[1] == kb)
    })
}

/// Délai de connexion au serveur (serveur injoignable, adresse IP non autorisée par le pare-feu).
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// Connexion et authentification ; clé du serveur (ligne `known_hosts`) et son empreinte.
async fn connect(
    target: &SftpTarget,
) -> Result<(russh::client::Handle<Client>, String, String), SftpError> {
    let received = Arc::new(Mutex::new(None));
    let handler = Client {
        expected: target.known_host_key.clone(),
        received: received.clone(),
    };
    let config = Arc::new(russh::client::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(60)),
        keepalive_interval: Some(std::time::Duration::from_secs(20)),
        ..Default::default()
    });
    let addr = format!("{}:{}", target.host, target.port);
    let connected = tokio::time::timeout(
        CONNECT_TIMEOUT,
        russh::client::connect(config, (target.host.as_str(), target.port), handler),
    )
    .await;
    let server_key = received.lock().ok().and_then(|k| k.clone());
    let (key_line, fingerprint) = match &server_key {
        Some(k) => (
            format!("{} {}", target.host, k.to_openssh().unwrap_or_default()),
            k.fingerprint(HashAlg::Sha256).to_string(),
        ),
        None => (String::new(), String::new()),
    };
    let session = match connected {
        Err(_) => return Err(SftpError::Timeout(addr)),
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            if target.known_host_key.is_some() && server_key.is_some() {
                return Err(SftpError::HostKeyMismatch(key_line));
            }
            return Err(SftpError::Connect(addr, e.to_string()));
        }
    };
    let mut session = session;
    let auth = session
        .authenticate_password(target.user.clone(), target.password.clone())
        .await
        .map_err(|e| SftpError::Connect(addr.clone(), e.to_string()))?;
    if !auth.success() {
        return Err(SftpError::Auth);
    }
    Ok((session, key_line, fingerprint))
}

/// Ouvre le sous-système SFTP d'une session authentifiée.
async fn open_sftp(
    session: &russh::client::Handle<Client>,
) -> Result<Arc<RawSftpSession>, SftpError> {
    let channel = session.channel_open_session().await.map_err(sftp_err)?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(sftp_err)?;
    let sftp = Arc::new(RawSftpSession::new(channel.into_stream()));
    sftp.set_timeout(60);
    Ok(sftp)
}

/// Connexion d'essai (5.3) : clé du serveur et dossiers où peut aller le site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// Clé du serveur, au format `known_hosts`.
    pub server_key: String,
    pub fingerprint: String,
    /// Dossiers trouvés, relatifs au dossier personnel, terminés par `/`, du plus probable au
    /// moins probable.
    pub folders: Vec<String>,
}

/// Dossiers habituels d'un site dans le dossier personnel (hébergements mutualisés).
const WEB_ROOTS: &[&str] = &["www", "public_html", "htdocs", "httpdocs", "html", "web"];

/// Se connecte, s'authentifie et cherche les dossiers du site (`www/`, `public_html/`,
/// `domains/<domaine>/public_html/`, `sites/<domaine>/`…), sans rien écrire.
pub async fn probe(target: &SftpTarget) -> Result<Probe, SftpError> {
    let (session, server_key, fingerprint) = connect(target).await?;
    let sftp = open_sftp(&session).await?;
    sftp.init().await.map_err(sftp_err)?;
    let is_dir = |p: String| {
        let sftp = sftp.clone();
        async move {
            sftp.stat(p)
                .await
                .is_ok_and(|a| a.attrs.file_type().is_dir())
        }
    };
    let list = |p: &str| {
        let sftp = sftp.clone();
        let p = p.to_string();
        async move {
            let mut names = Vec::new();
            let Ok(h) = sftp.opendir(p).await else {
                return names;
            };
            while let Ok(n) = sftp.readdir(h.handle.clone()).await {
                if n.files.is_empty() {
                    break;
                }
                names.extend(
                    n.files
                        .into_iter()
                        .map(|f| f.filename)
                        .filter(|f| !f.starts_with('.')),
                );
            }
            let _ = sftp.close(h.handle).await;
            names.sort();
            names
        }
    };
    let mut folders = Vec::new();
    // Un dossier par domaine (Hostinger, Infomaniak) d'abord : plus précis.
    for d in list("domains").await {
        let p = format!("domains/{d}/public_html");
        if is_dir(p.clone()).await {
            folders.push(format!("{p}/"));
        }
    }
    for d in list("sites").await {
        let p = format!("sites/{d}");
        if is_dir(p.clone()).await {
            folders.push(format!("{p}/"));
        }
    }
    for d in WEB_ROOTS {
        if is_dir((*d).to_string()).await {
            folders.push(format!("{d}/"));
        }
    }
    let _ = session
        .disconnect(russh::Disconnect::ByApplication, "", "fr")
        .await;
    Ok(Probe {
        server_key,
        fingerprint,
        folders,
    })
}

/// Publie `local_dir` sur le serveur.
pub async fn deploy(
    local_dir: &Path,
    target: &SftpTarget,
    opts: &Options,
    on_event: &(dyn Fn(Event) + Send + Sync),
) -> Result<Report, SftpError> {
    let remote_dir = manifest::normalize_dir(&target.remote_dir);
    if remote_dir.is_empty()
        || remote_dir == "."
        || remote_dir.starts_with('/')
        || remote_dir.split('/').any(|s| s == "..")
    {
        return Err(SftpError::RemoteDir(target.remote_dir.clone()));
    }
    let manifest_path = manifest::remote_path(&remote_dir);

    // Manifeste local (avant connexion : rien n'est ouvert si le dossier est invalide).
    let new = {
        let dir = local_dir.to_path_buf();
        tokio::task::spawn_blocking(move || manifest::compute(&dir))
            .await
            .map_err(sftp_err)??
    };
    if let Some(bad) = new
        .keys()
        .find(|p| p.contains('"') || p.contains('\\') || p.contains('\n'))
    {
        return Err(SftpError::FileName(bad.clone()));
    }

    // Connexion.
    let (session, key_line, fingerprint) = connect(target).await?;
    on_event(Event::Connected {
        server_key: key_line.clone(),
    });
    let sftp = open_sftp(&session).await?;
    let version = sftp.init().await.map_err(sftp_err)?;
    let posix_rename = version.extensions.contains_key("posix-rename@openssh.com");
    let r = Remote {
        sftp: sftp.clone(),
        made: Mutex::new(HashSet::new()),
    };

    r.mkdir_p(&remote_dir).await?;
    r.mkdir_p(".deploy").await?;

    // Ancien manifeste.
    let old: Manifest = if opts.full_sync {
        Manifest::new()
    } else {
        match r.read(&manifest_path).await {
            Ok(bytes) => manifest::parse(&String::from_utf8_lossy(&bytes)),
            Err(_) => Manifest::new(),
        }
    };
    let plan = manifest::diff(&old, &new);
    let upload_bytes: u64 = plan
        .upload
        .iter()
        .filter_map(|p| std::fs::metadata(local_dir.join(p)).ok())
        .map(|m| m.len())
        .sum();
    on_event(Event::Planned {
        upload: plan.upload.len(),
        delete: plan.delete.len(),
        bytes: upload_bytes,
        total: new.len(),
    });
    let mut report = Report {
        plan: plan.clone(),
        upload_bytes,
        total_files: new.len(),
        server_key: key_line,
        server_fingerprint: fingerprint,
        replace: Replace::Direct,
        dry_run: opts.dry_run,
    };
    if opts.dry_run {
        let _ = session
            .disconnect(russh::Disconnect::ByApplication, "", "fr")
            .await;
        return Ok(report);
    }

    // Mode de remplacement (test sur un fichier de contrôle, comme le script).
    report.replace = r.probe_replace(posix_rename).await;

    if !plan.upload.is_empty() || !plan.delete.is_empty() {
        r.write_file(
            &manifest_path,
            manifest::format(&manifest::stable(&old, &plan)).as_bytes(),
            Replace::Direct,
        )
        .await?;
    }

    // Envoi.
    let total = plan.upload.len();
    let done = std::sync::atomic::AtomicUsize::new(0);
    let parallel = if opts.parallel == 0 { 4 } else { opts.parallel };
    let sem = Arc::new(tokio::sync::Semaphore::new(parallel));
    let mut tasks = Vec::new();
    let r = Arc::new(r);
    for rel in plan.upload.clone() {
        let permit = sem.clone().acquire_owned().await.map_err(sftp_err)?;
        let r = r.clone();
        let local: PathBuf = local_dir.join(&rel);
        let remote = format!("{remote_dir}/{rel}");
        let mode = report.replace;
        tasks.push(tokio::spawn(async move {
            let _permit = permit;
            let data = tokio::fs::read(&local).await?;
            if let Some((parent, _)) = remote.rsplit_once('/') {
                r.mkdir_p(parent).await?;
            }
            r.write_file(&remote, &data, mode).await?;
            Ok::<String, SftpError>(rel)
        }));
    }
    for t in tasks {
        let rel = t.await.map_err(sftp_err)??;
        let n = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        on_event(Event::Uploaded {
            done: n,
            total,
            path: rel,
        });
    }

    // Suppressions (les erreurs « déjà absent » sont ignorées).
    let del_total = plan.delete.len();
    for (i, rel) in plan.delete.iter().enumerate() {
        let _ = r.sftp.remove(format!("{remote_dir}/{rel}")).await;
        on_event(Event::Deleted {
            done: i + 1,
            total: del_total,
        });
    }

    // Manifeste complet.
    r.write_file(
        &manifest_path,
        manifest::format(&new).as_bytes(),
        Replace::Direct,
    )
    .await?;
    on_event(Event::Finished);
    let _ = session
        .disconnect(russh::Disconnect::ByApplication, "", "fr")
        .await;
    Ok(report)
}

struct Remote {
    sftp: Arc<RawSftpSession>,
    made: Mutex<HashSet<String>>,
}

/// Taille des blocs écrits ; 16 écritures en vol par fichier.
const CHUNK: usize = 32 * 1024;
const IN_FLIGHT: usize = 16;

impl Remote {
    async fn mkdir_p(&self, dir: &str) -> Result<(), SftpError> {
        let mut cur = String::new();
        for part in dir.split('/').filter(|p| !p.is_empty() && *p != ".") {
            if !cur.is_empty() {
                cur.push('/');
            }
            cur.push_str(part);
            if self.made.lock().map(|m| m.contains(&cur)).unwrap_or(false) {
                continue;
            }
            if self.sftp.stat(cur.clone()).await.is_err() {
                // Créé entre-temps par une autre tâche : vérifié juste après.
                let _ = self
                    .sftp
                    .mkdir(cur.clone(), FileAttributes::default())
                    .await;
                self.sftp
                    .stat(cur.clone())
                    .await
                    .map_err(|e| SftpError::Sftp(format!("création du dossier {cur} : {e}")))?;
            }
            if let Ok(mut m) = self.made.lock() {
                m.insert(cur.clone());
            }
        }
        Ok(())
    }

    async fn read(&self, path: &str) -> Result<Vec<u8>, SftpError> {
        let h = self
            .sftp
            .open(path.to_string(), OpenFlags::READ, FileAttributes::default())
            .await
            .map_err(sftp_err)?
            .handle;
        let mut out = Vec::new();
        loop {
            match self
                .sftp
                .read(h.clone(), out.len() as u64, CHUNK as u32)
                .await
            {
                Ok(d) if !d.data.is_empty() => out.extend_from_slice(&d.data),
                _ => break,
            }
        }
        let _ = self.sftp.close(h).await;
        Ok(out)
    }

    async fn put(&self, path: &str, data: &[u8]) -> Result<(), SftpError> {
        let h = self
            .sftp
            .open(
                path.to_string(),
                OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
                FileAttributes::default(),
            )
            .await
            .map_err(|e| SftpError::Sftp(format!("{path} : {e}")))?
            .handle;
        let mut offset = 0usize;
        let mut pending = Vec::new();
        while offset < data.len() || !pending.is_empty() {
            while offset < data.len() && pending.len() < IN_FLIGHT {
                let end = (offset + CHUNK).min(data.len());
                let chunk = data[offset..end].to_vec();
                let sftp = self.sftp.clone();
                let hh = h.clone();
                let off = offset as u64;
                pending.push(tokio::spawn(
                    async move { sftp.write(hh, off, chunk).await },
                ));
                offset = end;
            }
            if let Some(t) = pending.first_mut() {
                t.await
                    .map_err(sftp_err)?
                    .map_err(|e| SftpError::Sftp(format!("{path} : {e}")))?;
                pending.remove(0);
            }
        }
        self.sftp
            .close(h)
            .await
            .map_err(|e| SftpError::Sftp(format!("{path} : {e}")))?;
        Ok(())
    }

    async fn posix_rename(&self, from: &str, to: &str) -> Result<(), SftpError> {
        let mut data = Vec::new();
        for s in [from, to] {
            data.extend_from_slice(&(s.len() as u32).to_be_bytes());
            data.extend_from_slice(s.as_bytes());
        }
        match self
            .sftp
            .extended("posix-rename@openssh.com", data)
            .await
            .map_err(sftp_err)?
        {
            Packet::Status(s) if s.status_code == StatusCode::Ok => Ok(()),
            p => Err(SftpError::Sftp(format!("renommage refusé : {p:?}"))),
        }
    }

    async fn write_file(&self, path: &str, data: &[u8], mode: Replace) -> Result<(), SftpError> {
        if mode == Replace::Direct {
            return self.put(path, data).await;
        }
        let (dir, name) = path.rsplit_once('/').unwrap_or(("", path));
        let tmp = if dir.is_empty() {
            format!(".in.{name}")
        } else {
            format!("{dir}/.in.{name}")
        };
        self.put(&tmp, data).await?;
        let r = match mode {
            Replace::PosixRename => self.posix_rename(&tmp, path).await,
            _ => self
                .sftp
                .rename(tmp.clone(), path.to_string())
                .await
                .map(|_| ())
                .map_err(sftp_err),
        };
        if r.is_err() {
            // Repli : suppression puis renommage (non atomique, mais sans fichier partiel).
            let _ = self.sftp.remove(path.to_string()).await;
            self.sftp
                .rename(tmp.clone(), path.to_string())
                .await
                .map_err(|e| SftpError::Sftp(format!("{path} : {e}")))?;
        }
        Ok(())
    }

    async fn probe_replace(&self, posix: bool) -> Replace {
        // Noms propres à cette publication : deux publications simultanées vers le même compte ne
        // se gênent pas.
        let unique = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        let probe = &format!(".deploy/probe-{unique}");
        let tmp = &format!(".deploy/.in.probe-{unique}");
        if self.put(probe, b"ok\n").await.is_err() {
            return Replace::Direct;
        }
        let mut mode = Replace::Direct;
        if posix
            && self.put(tmp, b"ok\n").await.is_ok()
            && self.posix_rename(tmp, probe).await.is_ok()
        {
            mode = Replace::PosixRename;
        } else if self.put(tmp, b"ok\n").await.is_ok()
            && self
                .sftp
                .rename(tmp.to_string(), probe.to_string())
                .await
                .is_ok()
        {
            mode = Replace::Rename;
        }
        let _ = self.sftp.remove(tmp.to_string()).await;
        let _ = self.sftp.remove(probe.to_string()).await;
        mode
    }
}
