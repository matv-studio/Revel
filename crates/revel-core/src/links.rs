//! Vérification des adresses d'autres sites (plan du site, D-107) : vidéos, réseaux sociaux,
//! autres sites liés depuis les pages, les contenus du CMS et les informations du site.
//!
//! - vidéo YouTube ou Vimeo : réponse oEmbed de la plateforme (une page de vidéo supprimée répond
//!   quand même) ; 200 : en ligne, 400/404 : introuvable, 401/403 : privée ou non intégrable ;
//! - autre adresse : premier octet demandé (`Range: bytes=0-0`), redirections suivies ; une page de
//!   connexion ou un refus (403, 429, 999) veut dire que le site refuse les vérifications
//!   automatiques, pas que l'adresse est morte.
//!
//! Résultats gardés dans `.revel/links.json` (hors des versions) : le plan les montre sans
//! refaire la vérification, refaite au-delà d'un jour.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use revel_protocol::{LinkCheck, LinkState};

use crate::{Core, CoreError, Result};

/// Fichier des résultats, relatif au site.
const CACHE: &str = ".revel/links.json";
/// Âge au-delà duquel une vérification est refaite (secondes).
pub const FRESH: i64 = 24 * 3600;
/// Vérifications menées en même temps.
const PARALLEL: usize = 6;

/// Ce que [`Core::links_check`] vérifie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckMode {
    /// Résultats gardés seulement (aucune connexion).
    Cached,
    /// Adresses jamais vérifiées ou vérifiées il y a plus d'un jour.
    Stale,
    /// Toutes les adresses demandées.
    All,
}

impl CheckMode {
    pub fn parse(s: &str) -> Self {
        match s {
            "all" => Self::All,
            "stale" => Self::Stale,
            _ => Self::Cached,
        }
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .user_agent(concat!(
            "Revel/",
            env!("CARGO_PKG_VERSION"),
            " (verification des liens)"
        ))
        .timeout_global(Some(Duration::from_secs(12)))
        .max_redirects(6)
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

/// Valeur d'un paramètre d'adresse encodée (caractères non réservés gardés).
fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Domaine (sans `www.`) et chemin d'une adresse absolue.
fn host_path(url: &str) -> Option<(String, String)> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = rest[..end].to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    let path = rest[end..]
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .to_string();
    Some((host, path))
}

/// Identifiant d'une vidéo YouTube (`watch?v=`, `youtu.be/`, `embed/`, `shorts/`, `live/`).
pub fn youtube_id(url: &str) -> Option<String> {
    let (host, path) = host_path(url)?;
    let ok = |id: &str| {
        (id.len() == 11
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .then(|| id.to_string())
    };
    if host == "youtu.be" {
        return ok(path.trim_start_matches('/').split('/').next()?);
    }
    if !(host.ends_with("youtube.com") || host.ends_with("youtube-nocookie.com")) {
        return None;
    }
    for prefix in ["/embed/", "/shorts/", "/live/", "/v/"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            return ok(rest.split('/').next()?);
        }
    }
    if path == "/watch" {
        let q = url.split_once('?')?.1.split('#').next()?;
        return q
            .split('&')
            .find_map(|kv| kv.strip_prefix("v="))
            .and_then(ok);
    }
    None
}

/// Identifiant d'une vidéo Vimeo (`vimeo.com/123`, `player.vimeo.com/video/123`).
pub fn vimeo_id(url: &str) -> Option<String> {
    let (host, path) = host_path(url)?;
    let digits =
        |s: &str| (!s.is_empty() && s.chars().all(|c| c.is_ascii_digit())).then(|| s.to_string());
    match host.as_str() {
        "player.vimeo.com" => digits(path.strip_prefix("/video/")?.split('/').next()?),
        "vimeo.com" => digits(path.trim_start_matches('/').split('/').next()?),
        _ => None,
    }
}

/// Page de connexion ou de contrôle (le site refuse la vérification automatique).
fn login_page(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    [
        "/login",
        "/signin",
        "/sign-in",
        "/authwall",
        "/checkpoint",
        "/sorry/",
        "/consent",
    ]
    .iter()
    .any(|k| p.contains(k))
}

/// Page déplacée : autre domaine ou autre chemin, sauf pour un fichier (vidéo, image : le
/// fichier est souvent servi par un autre serveur).
fn moved(original: &str, last: &str) -> bool {
    let (Some((h1, p1)), Some((h2, p2))) = (host_path(original), host_path(last)) else {
        return false;
    };
    let file = p1.rsplit('/').next().is_some_and(|s| s.contains('.'));
    !file && (h1 != h2 || p1.trim_end_matches('/') != p2.trim_end_matches('/'))
}

/// État d'une réponse : code, adresse finale.
pub fn classify(url: &str, status: u16, last: &str) -> (LinkState, Option<String>) {
    let last_path = host_path(last).map(|(_, p)| p).unwrap_or_default();
    match status {
        200..=299 | 416 if login_page(&last_path) => (LinkState::Blocked, None),
        200..=299 | 416 if moved(url, last) => (LinkState::Redirected, Some(last.to_string())),
        200..=299 | 416 => (LinkState::Ok, None),
        404 | 410 => (LinkState::Missing, None),
        401 | 403 | 429 | 999 => (LinkState::Blocked, None),
        _ => (LinkState::Failing, None),
    }
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Vérifie une adresse (connexion).
pub fn check_url(agent: &ureq::Agent, url: &str) -> LinkCheck {
    let mut c = LinkCheck {
        url: url.to_string(),
        state: LinkState::Unreachable,
        status: None,
        final_url: None,
        error: None,
        checked: now(),
    };
    // Vidéo : réponse oEmbed de la plateforme.
    let oembed = if let Some(id) = youtube_id(url) {
        Some(format!(
            "https://www.youtube.com/oembed?format=json&url={}",
            encode(&format!("https://www.youtube.com/watch?v={id}"))
        ))
    } else {
        vimeo_id(url).map(|id| {
            format!(
                "https://vimeo.com/api/oembed.json?url={}",
                encode(&format!("https://vimeo.com/{id}"))
            )
        })
    };
    if let Some(o) = oembed {
        match agent.get(&o).call() {
            Ok(res) => {
                let status = res.status().as_u16();
                c.status = Some(status);
                c.state = match status {
                    200..=299 => LinkState::Ok,
                    400 | 404 | 410 => LinkState::Missing,
                    401 | 403 => LinkState::Restricted,
                    429 => LinkState::Blocked,
                    _ => LinkState::Failing,
                };
            }
            Err(e) => c.error = Some(e.to_string()),
        }
        return c;
    }
    match agent.get(url).header("Range", "bytes=0-0").call() {
        Ok(res) => {
            use ureq::ResponseExt;
            let status = res.status().as_u16();
            let last = res.get_uri().to_string();
            let (state, final_url) = classify(url, status, &last);
            c.status = Some(status);
            c.state = state;
            c.final_url = final_url;
        }
        Err(e) => c.error = Some(e.to_string()),
    }
    c
}

/// Vérifie des adresses, quelques-unes à la fois.
pub fn check_all(urls: &[String]) -> Vec<LinkCheck> {
    let agent = agent();
    let mut out = Vec::with_capacity(urls.len());
    for chunk in urls.chunks(PARALLEL) {
        let found: Vec<LinkCheck> = std::thread::scope(|sc| {
            let jobs: Vec<_> = chunk
                .iter()
                .map(|u| {
                    let a = agent.clone();
                    sc.spawn(move || check_url(&a, u))
                })
                .collect();
            jobs.into_iter()
                .zip(chunk)
                .map(|(j, u)| {
                    j.join().unwrap_or_else(|_| LinkCheck {
                        url: u.clone(),
                        state: LinkState::Unreachable,
                        status: None,
                        final_url: None,
                        error: Some("vérification interrompue".into()),
                        checked: now(),
                    })
                })
                .collect()
        });
        out.extend(found);
    }
    out
}

fn read_cache(site: &Path) -> BTreeMap<String, LinkCheck> {
    std::fs::read(site.join(CACHE))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn write_cache(site: &Path, cache: &BTreeMap<String, LinkCheck>) -> Result<()> {
    let p = site.join(CACHE);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    let text = serde_json::to_vec_pretty(cache).map_err(|e| CoreError::Other(e.to_string()))?;
    std::fs::write(p, text)?;
    Ok(())
}

/// État des adresses `urls` (dans le même ordre ; `None` : jamais vérifiée et non vérifiée
/// ici). Les résultats sont gardés pour le site.
pub fn check_site(site: &Path, urls: &[String], mode: CheckMode) -> Result<Vec<Option<LinkCheck>>> {
    let mut cache = read_cache(site);
    let t = now();
    let mut todo: Vec<String> = urls
        .iter()
        .filter(|u| match mode {
            CheckMode::Cached => false,
            CheckMode::All => true,
            CheckMode::Stale => cache.get(*u).is_none_or(|c| t - c.checked > FRESH),
        })
        .cloned()
        .collect();
    todo.sort();
    todo.dedup();
    if !todo.is_empty() {
        for c in check_all(&todo) {
            cache.insert(c.url.clone(), c);
        }
        write_cache(site, &cache)?;
    }
    Ok(urls.iter().map(|u| cache.get(u).cloned()).collect())
}

impl Core {
    /// Vérifie des adresses d'autres sites (voir [`check_site`]) sans bloquer l'interface.
    pub async fn links_check(
        &self,
        site: &Path,
        urls: Vec<String>,
        mode: CheckMode,
    ) -> Result<Vec<Option<LinkCheck>>> {
        let site = site.to_path_buf();
        tokio::task::spawn_blocking(move || check_site(&site, &urls, mode))
            .await
            .map_err(|e| CoreError::Other(e.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn identifiants_des_videos() {
        assert_eq!(
            youtube_id("https://www.youtube.com/watch?v=nGzy-rbno-c&t=3").as_deref(),
            Some("nGzy-rbno-c")
        );
        assert_eq!(
            youtube_id("https://youtu.be/nGzy-rbno-c").as_deref(),
            Some("nGzy-rbno-c")
        );
        assert_eq!(
            youtube_id("https://www.youtube-nocookie.com/embed/nGzy-rbno-c?rel=0").as_deref(),
            Some("nGzy-rbno-c")
        );
        assert_eq!(youtube_id("https://www.youtube.com/@alunites"), None);
        assert_eq!(
            vimeo_id("https://vimeo.com/22439234").as_deref(),
            Some("22439234")
        );
        assert_eq!(
            vimeo_id("https://player.vimeo.com/video/22439234?dnt=1").as_deref(),
            Some("22439234")
        );
        assert_eq!(
            vimeo_id("https://player.vimeo.com/progressive_redirect/playback/1/file.mp4"),
            None
        );
        assert_eq!(encode("https://a.b/c?d=1"), "https%3A%2F%2Fa.b%2Fc%3Fd%3D1");
    }

    #[test]
    fn etats_des_reponses() {
        let u = "https://exemple.fr/page/";
        assert_eq!(classify(u, 200, u).0, LinkState::Ok);
        assert_eq!(
            classify(u, 206, "https://www.exemple.fr/page").0,
            LinkState::Ok
        );
        assert_eq!(
            classify(u, 200, "https://exemple.fr/nouvelle/"),
            (
                LinkState::Redirected,
                Some("https://exemple.fr/nouvelle/".into())
            )
        );
        // Fichier servi par un autre serveur : pas une page déplacée.
        let f = "https://player.vimeo.com/progressive_redirect/playback/1/file.mp4?s=1";
        assert_eq!(
            classify(f, 206, "https://cdn.vimeocdn.com/x/y?token=1").0,
            LinkState::Ok
        );
        assert_eq!(classify(u, 404, u).0, LinkState::Missing);
        assert_eq!(classify(u, 999, u).0, LinkState::Blocked);
        assert_eq!(
            classify(
                "https://www.instagram.com/a/",
                200,
                "https://www.instagram.com/accounts/login/?next=x"
            )
            .0,
            LinkState::Blocked
        );
        assert_eq!(classify(u, 503, u).0, LinkState::Failing);
    }

    /// Serveur local : `/ok` répond 200, `/vieux` redirige vers `/neuf`, le reste 404.
    fn server() -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        std::thread::spawn(move || {
            for s in l.incoming().take(8) {
                let Ok(mut s) = s else { continue };
                let mut buf = [0u8; 2048];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                let resp = match path.as_str() {
                    "/ok" | "/neuf" => {
                        "HTTP/1.1 200 OK\r\nContent-Length: 1\r\nConnection: close\r\n\r\nx"
                            .to_string()
                    }
                    "/vieux" => format!(
                        "HTTP/1.1 301 Moved\r\nLocation: http://{addr}/neuf\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    ),
                    _ => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_string(),
                };
                let _ = s.write_all(resp.as_bytes());
            }
        });
        format!("http://{addr}")
    }

    #[test]
    fn verification_et_resultats_gardes() {
        let base = server();
        let dir = tempfile::tempdir().unwrap();
        let urls = vec![
            format!("{base}/ok"),
            format!("{base}/vieux"),
            format!("{base}/rien"),
        ];
        // Rien de gardé : aucune connexion.
        let none = check_site(dir.path(), &urls, CheckMode::Cached).unwrap();
        assert!(none.iter().all(Option::is_none));
        let r = check_site(dir.path(), &urls, CheckMode::Stale).unwrap();
        let states: Vec<LinkState> = r.iter().map(|c| c.as_ref().unwrap().state).collect();
        assert_eq!(
            states,
            vec![LinkState::Ok, LinkState::Redirected, LinkState::Missing]
        );
        assert_eq!(
            r[1].as_ref().unwrap().final_url.as_deref(),
            Some(format!("{base}/neuf").as_str())
        );
        // Gardés : relus sans connexion.
        let again = check_site(dir.path(), &urls, CheckMode::Cached).unwrap();
        assert_eq!(again, r);
        assert!(dir.path().join(CACHE).exists());
    }
}
