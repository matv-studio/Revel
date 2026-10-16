//! Nom de domaine (5.7) : vérification par Revel, sans service extérieur. Le résolveur du
//! système donne les adresses du domaine ; une requête HTTPS (certificat vérifié) puis HTTP
//! montre s'il répond ; la page d'accueil reçue est comparée à celle de la dernière publication
//! sur l'hébergement (même empreinte que le fichier envoyé).

use crate::{Core, CoreError, Result};
use revel_protocol::{DomainStatus, HostingTarget};
use std::net::ToSocketAddrs;
use std::path::Path;
use std::time::Duration;

/// Adresses de GitHub Pages pour un domaine racine (A et AAAA).
pub const GITHUB_PAGES_IPS: &[&str] = &[
    "185.199.108.153",
    "185.199.109.153",
    "185.199.110.153",
    "185.199.111.153",
    "2606:50c0:8000::153",
    "2606:50c0:8001::153",
    "2606:50c0:8002::153",
    "2606:50c0:8003::153",
];

/// `https://www.Exemple.fr/page` -> `www.exemple.fr` ; `None` si ce n'est pas un nom de domaine.
pub fn normalize_domain(s: &str) -> Option<String> {
    let s = s.trim();
    let s = s.split_once("://").map(|(_, r)| r).unwrap_or(s);
    let s = s.split(['/', '?', '#']).next().unwrap_or_default();
    let s = s.rsplit_once('@').map(|(_, h)| h).unwrap_or(s);
    let s = s.split(':').next().unwrap_or_default();
    let d = s.trim_end_matches('.').to_ascii_lowercase();
    let ok = d.contains('.')
        && d.len() <= 253
        && d.split('.').all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });
    ok.then_some(d)
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .user_agent(concat!("Revel/", env!("CARGO_PKG_VERSION")))
        .timeout_global(Some(Duration::from_secs(15)))
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

/// Corps de la page d'accueil (au plus 5 Mo) ; erreur de connexion ou de certificat en texte.
fn fetch(agent: &ureq::Agent, url: &str) -> std::result::Result<(u16, Vec<u8>), String> {
    let mut res = agent.get(url).call().map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let body = res
        .body_mut()
        .with_config()
        .limit(5 * 1024 * 1024)
        .read_to_vec()
        .unwrap_or_default();
    Ok((status, body))
}

impl Core {
    /// Vérifie qu'un domaine mène au site publié sur un hébergement.
    pub async fn domain_check(
        &self,
        site: &Path,
        profile_id: &str,
        domain: &str,
    ) -> Result<DomainStatus> {
        let domain = normalize_domain(domain)
            .ok_or_else(|| CoreError::Other(format!("nom de domaine invalide : {domain}")))?;
        let profile = self
            .profiles(&site.display().to_string())
            .into_iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| CoreError::UnknownProfile(profile_id.to_string()))?;
        let expected: Vec<String> = match profile.target {
            HostingTarget::GithubPages { .. } => {
                GITHUB_PAGES_IPS.iter().map(|s| s.to_string()).collect()
            }
            HostingTarget::Sftp { .. } => Vec::new(),
        };
        // Page d'accueil publiée (empreinte du fichier envoyé).
        let published = crate::rollback::versions(site, profile_id)
            .pop()
            .and_then(|v| v.files.get("index.html").cloned());
        let d = domain.clone();
        tokio::task::spawn_blocking(move || {
            let mut addresses: Vec<String> = (d.as_str(), 443)
                .to_socket_addrs()
                .map(|it| it.map(|a| a.ip().to_string()).collect())
                .unwrap_or_default();
            addresses.sort();
            addresses.dedup();
            let agent = agent();
            let mut status = DomainStatus {
                domain: d.clone(),
                addresses,
                expected,
                ..Default::default()
            };
            if status.addresses.is_empty() {
                return status;
            }
            let mut body = None;
            match fetch(&agent, &format!("https://{d}/")) {
                Ok((code, b)) => {
                    status.https = code < 500;
                    body = Some(b);
                }
                Err(e) => status.https_error = Some(e),
            }
            if let Ok((code, b)) = fetch(&agent, &format!("http://{d}/")) {
                status.http = code < 500;
                if body.is_none() {
                    body = Some(b);
                }
            }
            status.serves_site = match (published, body) {
                (Some(hash), Some(b)) => Some(blake3::hash(&b).to_hex().to_string() == hash),
                _ => None,
            };
            status
        })
        .await
        .map_err(|e| CoreError::Other(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domaines() {
        assert_eq!(
            normalize_domain("https://www.Alunites.com/agence/").as_deref(),
            Some("www.alunites.com")
        );
        assert_eq!(
            normalize_domain(" alunites.com. ").as_deref(),
            Some("alunites.com")
        );
        assert_eq!(normalize_domain("alunites"), None);
        assert_eq!(normalize_domain("-a.com"), None);
        assert_eq!(normalize_domain("a_b.com"), None);
    }

    /// Serveur local : adresse, réponse HTTP, page d'accueil comparée.
    #[tokio::test(flavor = "multi_thread")]
    async fn domaine_local() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for s in listener.incoming().flatten().take(4) {
                let mut s = s;
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf);
                let body = "<h1>Accueil</h1>";
                let _ = write!(
                    s,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        let agent = agent();
        let (code, body) = fetch(&agent, &format!("http://127.0.0.1:{port}/")).unwrap();
        assert_eq!(
            (code, body.as_slice()),
            (200, b"<h1>Accueil</h1>".as_slice())
        );
        assert!(fetch(&agent, &format!("https://127.0.0.1:{port}/")).is_err());
    }
}
