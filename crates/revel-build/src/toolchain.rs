//! Outils de construction téléchargés par Revel (Node.js, Hugo), rangés dans le dossier de
//! l'application : version retenue d'après les publications officielles, archive vérifiée par
//! l'empreinte SHA-256 publiée avec elle, puis décompressée.
//!
//! - Node.js : dernière version LTS de <https://nodejs.org/dist/index.json>, empreintes dans
//!   `SHASUMS256.txt` du même dossier.
//! - Hugo : dernière publication du dépôt `gohugoio/hugo` (API GitHub), édition « extended »,
//!   empreintes dans `hugo_<version>_checksums.txt`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::BuildError;

/// Accès réseau (remplaçable dans les essais).
pub trait Net: Sync {
    /// Texte d'une adresse (JSON, liste d'empreintes).
    fn text(&self, url: &str) -> Result<String, String>;
    /// Taille d'un fichier en ligne (en-tête `Content-Length`), si le serveur la donne.
    fn size(&self, url: &str) -> Option<u64> {
        let _ = url;
        None
    }
    /// Télécharge `url` dans `to` ; renvoie l'empreinte SHA-256 (hexadécimal) du fichier.
    fn download(
        &self,
        url: &str,
        to: &Path,
        progress: &mut dyn FnMut(u64, Option<u64>),
        cancel: &AtomicBool,
    ) -> Result<String, BuildError>;
}

/// Accès réseau réel (certificats du système).
pub struct Http {
    agent: ureq::Agent,
}

impl Default for Http {
    fn default() -> Self {
        Self::new()
    }
}

impl Http {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .user_agent(concat!("Revel/", env!("CARGO_PKG_VERSION"), " (build)"))
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_recv_response(Some(Duration::from_secs(60)))
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                    .build(),
            )
            .build()
            .into();
        Self { agent }
    }
}

pub(crate) fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

impl Net for Http {
    fn text(&self, url: &str) -> Result<String, String> {
        let mut r = self
            .agent
            .get(url)
            .header("Accept", "application/json, text/plain, */*")
            .call()
            .map_err(|e| e.to_string())?;
        r.body_mut()
            .with_config()
            .limit(32 * 1024 * 1024)
            .read_to_string()
            .map_err(|e| e.to_string())
    }

    fn size(&self, url: &str) -> Option<u64> {
        let r = self.agent.head(url).call().ok()?;
        r.headers()
            .get("content-length")?
            .to_str()
            .ok()?
            .parse()
            .ok()
    }

    fn download(
        &self,
        url: &str,
        to: &Path,
        progress: &mut dyn FnMut(u64, Option<u64>),
        cancel: &AtomicBool,
    ) -> Result<String, BuildError> {
        let resp = self
            .agent
            .get(url)
            .call()
            .map_err(|e| BuildError::Download(format!("{url} : {e}")))?;
        let total = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        let mut reader = resp.into_body().into_reader();
        let mut out = std::fs::File::create(to)?;
        let mut h = Sha256::new();
        let mut buf = vec![0u8; 256 * 1024];
        let mut got = 0u64;
        loop {
            if cancel.load(Ordering::Relaxed) {
                drop(out);
                let _ = std::fs::remove_file(to);
                return Err(BuildError::Cancelled);
            }
            let n = reader
                .read(&mut buf)
                .map_err(|e| BuildError::Download(format!("{url} : {e}")))?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
            out.write_all(&buf[..n])?;
            got += n as u64;
            progress(got, total);
        }
        out.flush()?;
        Ok(hex(&h.finalize()))
    }
}

/// Outil téléchargeable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Node,
    Hugo,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Node => "Node.js",
            Kind::Hugo => "Hugo",
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Kind::Node => "node-",
            Kind::Hugo => "hugo-",
        }
    }
}

/// Version publiée d'un outil, pour ce système.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub kind: Kind,
    /// Numéro de version sans `v` (`24.22.0`).
    pub version: String,
    /// Adresse de l'archive.
    pub url: String,
    /// Nom de l'archive (`node-v24.22.0-linux-x64.tar.gz`).
    pub archive: String,
    /// Adresse de la liste des empreintes.
    pub sums: String,
    /// Taille de l'archive, quand la publication la donne.
    pub size: Option<u64>,
}

/// Système et processeur de cet ordinateur, aux noms de Node.js (`linux`, `darwin`, `win` ;
/// `x64`, `arm64`).
fn node_platform() -> Option<(&'static str, &'static str)> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "darwin",
        "windows" => "win",
        _ => return None,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => return None,
    };
    Some((os, arch))
}

/// Dernière version LTS de Node.js pour `os`/`arch` d'après `index.json` (versions les plus
/// récentes en tête).
pub fn node_release_for(index: &str, os: &str, arch: &str) -> Option<Release> {
    let list: Value = serde_json::from_str(index).ok()?;
    // Clé des fichiers dans `index.json` : `linux-x64`, `osx-arm64-tar`, `win-x64-zip`.
    let key = match os {
        "linux" => format!("linux-{arch}"),
        "darwin" => format!("osx-{arch}-tar"),
        _ => format!("win-{arch}-zip"),
    };
    let r = list.as_array()?.iter().find(|r| {
        r.get("lts").is_some_and(|l| l.as_str().is_some())
            && r.get("files")
                .and_then(Value::as_array)
                .is_some_and(|f| f.iter().any(|x| x.as_str() == Some(key.as_str())))
    })?;
    let v = r.get("version")?.as_str()?;
    let ext = if os == "win" { "zip" } else { "tar.gz" };
    let archive = format!("node-{v}-{os}-{arch}.{ext}");
    Some(Release {
        kind: Kind::Node,
        version: v.trim_start_matches('v').to_string(),
        url: format!("https://nodejs.org/dist/{v}/{archive}"),
        archive,
        sums: format!("https://nodejs.org/dist/{v}/SHASUMS256.txt"),
        size: None,
    })
}

/// Nom de l'archive de Hugo pour ce système (`linux-amd64`, `darwin-universal`,
/// `windows-amd64`).
fn hugo_platform() -> Option<&'static str> {
    Some(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-amd64",
        ("linux", "aarch64") => "linux-arm64",
        ("macos", _) => "darwin-universal",
        ("windows", "x86_64") => "windows-amd64",
        ("windows", "aarch64") => "windows-arm64",
        _ => return None,
    })
}

/// Édition « extended » de Hugo pour `platform` d'après la publication (API GitHub).
pub fn hugo_release_for(release_json: &str, platform: &str) -> Option<Release> {
    let r: Value = serde_json::from_str(release_json).ok()?;
    let assets = r.get("assets")?.as_array()?;
    let pick = |pred: &dyn Fn(&str) -> bool| {
        assets.iter().find_map(|a| {
            let name = a.get("name")?.as_str()?;
            pred(name).then(|| {
                (
                    name.to_string(),
                    a.get("browser_download_url")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    a.get("size").and_then(Value::as_u64),
                )
            })
        })
    };
    let (archive, url, size) = pick(&|n: &str| {
        n.starts_with("hugo_extended_")
            && !n.contains("withdeploy")
            && (n.ends_with(&format!("_{platform}.tar.gz"))
                || n.ends_with(&format!("_{platform}.zip")))
    })?;
    let version = archive
        .strip_prefix("hugo_extended_")?
        .split('_')
        .next()?
        .to_string();
    let (_, sums, _) = pick(&|n: &str| n.starts_with("hugo_") && n.ends_with("checksums.txt"))?;
    Some(Release {
        kind: Kind::Hugo,
        version,
        url,
        archive,
        sums,
        size,
    })
}

/// Dernière version publiée de l'outil pour ce système.
pub fn latest(kind: Kind, net: &dyn Net) -> Result<Release, BuildError> {
    match kind {
        Kind::Node => {
            let (os, arch) = node_platform().ok_or(BuildError::Platform)?;
            let index = net
                .text("https://nodejs.org/dist/index.json")
                .map_err(BuildError::Download)?;
            node_release_for(&index, os, arch).ok_or(BuildError::Platform)
        }
        Kind::Hugo => {
            let platform = hugo_platform().ok_or(BuildError::Platform)?;
            let json = net
                .text("https://api.github.com/repos/gohugoio/hugo/releases/latest")
                .map_err(BuildError::Download)?;
            hugo_release_for(&json, platform).ok_or(BuildError::Platform)
        }
    }
}

/// Empreinte d'un fichier dans une liste `empreinte  nom`.
pub fn sum_for(list: &str, file: &str) -> Option<String> {
    list.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        let sum = it.next()?;
        let name = it.next()?.trim_start_matches('*');
        (name == file).then(|| sum.to_ascii_lowercase())
    })
}

/// Outil installé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub kind: Kind,
    pub version: String,
    pub dir: PathBuf,
}

impl Installed {
    /// Programme principal (`node`, `hugo`).
    pub fn program(&self) -> PathBuf {
        let win = cfg!(windows);
        match self.kind {
            Kind::Node if win => self.dir.join("node.exe"),
            Kind::Node => self.dir.join("bin").join("node"),
            Kind::Hugo if win => self.dir.join("hugo.exe"),
            Kind::Hugo => self.dir.join("hugo"),
        }
    }

    /// Dossier à ajouter au `PATH` des commandes lancées.
    pub fn bin_dir(&self) -> PathBuf {
        self.program()
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.dir.clone())
    }

    /// Programme de npm livré avec Node.js.
    pub fn npm_cli(&self) -> PathBuf {
        let base = if cfg!(windows) {
            self.dir.join("node_modules")
        } else {
            self.dir.join("lib").join("node_modules")
        };
        base.join("npm").join("bin").join("npm-cli.js")
    }
}

const READY: &str = ".revel-pret";

/// Outils installés dans `tools` (les plus récents en premier).
pub fn installed(tools: &Path) -> Vec<Installed> {
    let mut out: Vec<Installed> = std::fs::read_dir(tools)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().join(READY).is_file())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let (kind, version) = [Kind::Node, Kind::Hugo]
                        .into_iter()
                        .find_map(|k| name.strip_prefix(k.prefix()).map(|v| (k, v.to_string())))?;
                    Some(Installed {
                        kind,
                        version,
                        dir: e.path(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let key = |i: &Installed| semver::Version::parse(&i.version).ok();
    out.sort_by_key(|a| std::cmp::Reverse(key(a)));
    out
}

/// Outil `kind` installé le plus récent.
pub fn find(tools: &Path, kind: Kind) -> Option<Installed> {
    installed(tools).into_iter().find(|i| i.kind == kind)
}

/// Taille d'un dossier (octets).
pub fn dir_size(dir: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            match e.file_type() {
                Ok(t) if t.is_dir() => stack.push(e.path()),
                Ok(t) if t.is_file() => total += e.metadata().map(|m| m.len()).unwrap_or(0),
                _ => {}
            }
        }
    }
    total
}

/// Décompresse `archive` (`.tar.gz` ou `.zip`) dans `dest`, sans le dossier de tête commun aux
/// entrées (Node.js en a un, Hugo non).
pub fn extract(archive: &Path, dest: &Path) -> Result<(), BuildError> {
    std::fs::create_dir_all(dest)?;
    let name = archive
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let strip = |names: &[String]| -> usize {
        let first: Vec<&str> = names
            .iter()
            .filter_map(|n| n.split('/').find(|s| !s.is_empty() && *s != "."))
            .collect();
        let all_in_one = !first.is_empty()
            && first.iter().all(|f| *f == first[0])
            && names.iter().any(|n| n.trim_matches('/').contains('/'));
        usize::from(all_in_one)
    };
    let target = |path: &str, skip: usize| -> Option<PathBuf> {
        let parts: Vec<&str> = path
            .split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .skip(skip)
            .collect();
        if parts.is_empty() || parts.iter().any(|p| *p == ".." || p.contains('\\')) {
            return None;
        }
        Some(parts.iter().fold(dest.to_path_buf(), |acc, p| acc.join(p)))
    };
    if name.ends_with(".zip") {
        let f = std::fs::File::open(archive)?;
        let mut z = zip::ZipArchive::new(f).map_err(|e| BuildError::Archive(e.to_string()))?;
        let names: Vec<String> = (0..z.len())
            .filter_map(|i| z.by_index(i).ok().map(|e| e.name().to_string()))
            .collect();
        let skip = strip(&names);
        for i in 0..z.len() {
            let mut e = z
                .by_index(i)
                .map_err(|e| BuildError::Archive(e.to_string()))?;
            let Some(out) = target(e.name(), skip) else {
                continue;
            };
            if e.is_dir() {
                std::fs::create_dir_all(&out)?;
                continue;
            }
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut w = std::fs::File::create(&out)?;
            std::io::copy(&mut e, &mut w)?;
            #[cfg(unix)]
            if let Some(mode) = e.unix_mode() {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&out, std::fs::Permissions::from_mode(mode & 0o777))?;
            }
        }
        return Ok(());
    }
    let open = || -> Result<tar::Archive<flate2::read::GzDecoder<std::fs::File>>, BuildError> {
        Ok(tar::Archive::new(flate2::read::GzDecoder::new(
            std::fs::File::open(archive)?,
        )))
    };
    let names: Vec<String> = open()?
        .entries()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            e.path()
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
        })
        .collect();
    let skip = strip(&names);
    let mut a = open()?;
    a.set_preserve_permissions(true);
    for e in a.entries()? {
        let mut e = e?;
        let path = e.path()?.to_string_lossy().replace('\\', "/");
        let Some(out) = target(&path, skip) else {
            continue;
        };
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p)?;
        }
        e.unpack(&out)?;
    }
    Ok(())
}

/// Télécharge, vérifie et décompresse `release` dans `tools` ; renvoie l'outil installé.
pub fn install(
    release: &Release,
    tools: &Path,
    net: &dyn Net,
    progress: &mut dyn FnMut(u64, Option<u64>),
    cancel: &AtomicBool,
) -> Result<Installed, BuildError> {
    std::fs::create_dir_all(tools)?;
    let sums = net.text(&release.sums).map_err(BuildError::Download)?;
    let expected = sum_for(&sums, &release.archive)
        .ok_or_else(|| BuildError::Integrity(format!("{} : empreinte absente", release.archive)))?;
    let part = tools.join(format!("{}.part", release.archive));
    let got = net.download(&release.url, &part, progress, cancel)?;
    if got != expected {
        let _ = std::fs::remove_file(&part);
        return Err(BuildError::Integrity(format!(
            "{} : empreinte {got} au lieu de {expected}",
            release.archive
        )));
    }
    // L'archive garde son nom (extension lue par `extract`).
    let archive = tools.join(&release.archive);
    std::fs::rename(&part, &archive)?;
    let dir = tools.join(format!("{}{}", release.kind.prefix(), release.version));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    let r = extract(&archive, &dir);
    let _ = std::fs::remove_file(&archive);
    if let Err(e) = r {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(e);
    }
    let tool = Installed {
        kind: release.kind,
        version: release.version.clone(),
        dir: dir.clone(),
    };
    if !tool.program().is_file() {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(BuildError::Archive(format!(
            "{} : {} absent",
            release.archive,
            tool.program().display()
        )));
    }
    std::fs::write(dir.join(READY), &release.archive)?;
    Ok(tool)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = r#"[
      {"version":"v26.12.0","lts":false,"files":["linux-x64","osx-arm64-tar","win-x64-zip"]},
      {"version":"v24.22.0","lts":"Krypton","files":["linux-x64","linux-arm64","osx-arm64-tar","osx-x64-tar","win-x64-zip"]},
      {"version":"v22.30.0","lts":"Jod","files":["linux-x64"]}
    ]"#;

    #[test]
    fn node_lts_par_systeme() {
        let r = node_release_for(INDEX, "linux", "x64").unwrap();
        assert_eq!(r.version, "24.22.0");
        assert_eq!(r.archive, "node-v24.22.0-linux-x64.tar.gz");
        assert_eq!(
            r.url,
            "https://nodejs.org/dist/v24.22.0/node-v24.22.0-linux-x64.tar.gz"
        );
        assert_eq!(r.sums, "https://nodejs.org/dist/v24.22.0/SHASUMS256.txt");
        assert_eq!(
            node_release_for(INDEX, "darwin", "arm64").unwrap().archive,
            "node-v24.22.0-darwin-arm64.tar.gz"
        );
        assert_eq!(
            node_release_for(INDEX, "win", "x64").unwrap().archive,
            "node-v24.22.0-win-x64.zip"
        );
        assert_eq!(node_release_for(INDEX, "win", "arm64"), None);
    }

    #[test]
    fn hugo_extended() {
        let json = r#"{"tag_name":"v0.152.2","assets":[
          {"name":"hugo_0.152.2_linux-amd64.tar.gz","size":1,"browser_download_url":"https://x/a"},
          {"name":"hugo_extended_0.152.2_linux-amd64.tar.gz","size":25000000,"browser_download_url":"https://x/b"},
          {"name":"hugo_extended_withdeploy_0.152.2_linux-amd64.tar.gz","size":3,"browser_download_url":"https://x/c"},
          {"name":"hugo_extended_0.152.2_windows-amd64.zip","size":4,"browser_download_url":"https://x/d"},
          {"name":"hugo_0.152.2_checksums.txt","size":5,"browser_download_url":"https://x/sums"}]}"#;
        let r = hugo_release_for(json, "linux-amd64").unwrap();
        assert_eq!(
            (r.version.as_str(), r.url.as_str(), r.size),
            ("0.152.2", "https://x/b", Some(25000000))
        );
        assert_eq!(r.sums, "https://x/sums");
        assert_eq!(
            hugo_release_for(json, "windows-amd64").unwrap().archive,
            "hugo_extended_0.152.2_windows-amd64.zip"
        );
        assert_eq!(hugo_release_for(json, "darwin-universal"), None);
    }

    #[test]
    fn liste_des_empreintes() {
        let l = "aa11  node-v24.22.0-linux-x64.tar.gz\nBB22 *node-v24.22.0-win-x64.zip\n";
        assert_eq!(
            sum_for(l, "node-v24.22.0-win-x64.zip").as_deref(),
            Some("bb22")
        );
        assert_eq!(sum_for(l, "autre"), None);
    }

    /// Réseau simulé : archives fabriquées dans l'essai.
    struct Fake {
        sums: String,
        file: PathBuf,
    }

    impl Net for Fake {
        fn text(&self, _url: &str) -> Result<String, String> {
            Ok(self.sums.clone())
        }
        fn download(
            &self,
            _url: &str,
            to: &Path,
            progress: &mut dyn FnMut(u64, Option<u64>),
            _cancel: &AtomicBool,
        ) -> Result<String, BuildError> {
            std::fs::copy(&self.file, to)?;
            let data = std::fs::read(to)?;
            progress(data.len() as u64, Some(data.len() as u64));
            Ok(hex(&Sha256::digest(&data)))
        }
    }

    fn tar_gz(path: &Path, entries: &[(&str, &[u8], u32)]) {
        let f = std::fs::File::create(path).unwrap();
        let gz = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
        let mut b = tar::Builder::new(gz);
        for (name, data, mode) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_size(data.len() as u64);
            h.set_mode(*mode);
            h.set_cksum();
            b.append_data(&mut h, name, *data).unwrap();
        }
        b.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn installation_verifiee() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src.tar.gz");
        let node = if cfg!(windows) {
            "node-v1.2.3/node.exe"
        } else {
            "node-v1.2.3/bin/node"
        };
        tar_gz(
            &src,
            &[
                (node, b"#!/bin/sh\n", 0o755),
                ("node-v1.2.3/README.md", b"x", 0o644),
            ],
        );
        let sum = hex(&Sha256::digest(std::fs::read(&src).unwrap()));
        let rel = Release {
            kind: Kind::Node,
            version: "1.2.3".into(),
            url: "https://x/node.tar.gz".into(),
            archive: "node-v1.2.3-x.tar.gz".into(),
            sums: "https://x/SHASUMS256.txt".into(),
            size: None,
        };
        let tools = d.path().join("tools");
        let cancel = AtomicBool::new(false);
        let mut seen = 0;
        let net = Fake {
            sums: format!("{sum}  node-v1.2.3-x.tar.gz\n"),
            file: src.clone(),
        };
        let t = install(&rel, &tools, &net, &mut |g, _| seen = g, &cancel).unwrap();
        assert!(seen > 0);
        assert!(t.program().is_file(), "{}", t.program().display());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(t.program()).unwrap().permissions().mode() & 0o111,
                0o111
            );
        }
        assert_eq!(
            find(&tools, Kind::Node).map(|i| i.version),
            Some("1.2.3".into())
        );
        assert!(dir_size(&t.dir) > 0);
        // Empreinte différente : refusé, rien d'installé.
        let bad = Fake {
            sums: "00  node-v1.2.3-x.tar.gz\n".into(),
            file: src,
        };
        let tools2 = d.path().join("tools2");
        assert!(matches!(
            install(&rel, &tools2, &bad, &mut |_, _| {}, &cancel),
            Err(BuildError::Integrity(_))
        ));
        assert!(installed(&tools2).is_empty());
    }

    #[test]
    fn archive_zip_sans_dossier_de_tete() {
        let d = tempfile::tempdir().unwrap();
        let z = d.path().join("hugo.zip");
        {
            let f = std::fs::File::create(&z).unwrap();
            let mut w = zip::ZipWriter::new(f);
            let o: zip::write::SimpleFileOptions = Default::default();
            w.start_file("hugo.exe", o).unwrap();
            w.write_all(b"MZ").unwrap();
            w.start_file("LICENSE", o).unwrap();
            w.write_all(b"x").unwrap();
            w.start_file("../evasion", o).unwrap();
            w.write_all(b"x").unwrap();
            w.finish().unwrap();
        }
        let out = d.path().join("out");
        extract(&z, &out).unwrap();
        assert!(out.join("hugo.exe").is_file() && out.join("LICENSE").is_file());
        assert!(!d.path().join("evasion").exists());
    }
}
