//! `revel-deploy` : publication du site chez un hébergeur.
//!
//! Chaque hébergeur implémente [`Provider`] et déclare ses [`Capabilities`].
//! L'éditeur compare ces capacités aux besoins du site ([`SiteNeeds`]) pour
//! avertir l'utilisateur **avant** la publication ([`check`]).
//!
//! Les implémentations d'envoi arrivent à l'étape 1 (SFTP, GitHub Pages).

use serde::Serialize;

pub mod manifest;
pub mod providers;
#[cfg(feature = "sftp")]
pub mod sftp;

/// Ce qu'un hébergeur sait faire. Sources des valeurs : voir `providers.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Capabilities {
    pub id: &'static str,
    /// Vraies redirections HTTP 301 côté serveur.
    pub redirects_301: bool,
    /// Exécution de PHP (formulaire `contact.php`, par exemple).
    pub php: bool,
    /// En-têtes HTTP personnalisés (`X-Robots-Tag`, cache, sécurité).
    pub custom_headers: bool,
    /// Fichier `.htaccess` Apache interprété.
    pub htaccess: bool,
    /// Taille maximale du site publié, en octets (`None` = pas de limite connue).
    pub max_site_bytes: Option<u64>,
    /// Taille maximale d'un fichier, en octets.
    pub max_file_bytes: Option<u64>,
    /// Nombre maximal de fichiers publiés.
    pub max_files: Option<u64>,
    /// Le dépôt source peut-il rester privé sur une offre gratuite ?
    pub private_source_on_free_plan: Option<bool>,
}

/// Besoins du site, calculés à partir du modèle et des fichiers générés.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiteNeeds {
    pub redirects_301: usize,
    pub php_files: usize,
    pub custom_headers: bool,
    pub htaccess: bool,
    pub total_bytes: u64,
    pub largest_file_bytes: u64,
    pub file_count: u64,
    pub wants_private_source: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// La publication fonctionnera, mais une fonction sera dégradée.
    Warning,
    /// La publication échouera ou le site sera cassé.
    Blocking,
}

/// Avertissement présenté à l'utilisateur. `key` est une clé de traduction
/// (`revel-i18n`), `args` ses paramètres.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    pub severity: Severity,
    pub key: &'static str,
    pub args: Vec<(&'static str, String)>,
}

/// Compare les besoins du site aux capacités de l'hébergeur.
pub fn check(caps: &Capabilities, needs: &SiteNeeds) -> Vec<Warning> {
    let mut out = Vec::new();
    let w = |severity, key, args| Warning {
        severity,
        key,
        args,
    };
    if needs.redirects_301 > 0 && !caps.redirects_301 {
        out.push(w(
            Severity::Warning,
            "deploy-warn-no-301",
            vec![("count", needs.redirects_301.to_string())],
        ));
    }
    if needs.php_files > 0 && !caps.php {
        out.push(w(
            Severity::Warning,
            "deploy-warn-no-php",
            vec![("count", needs.php_files.to_string())],
        ));
    }
    if needs.custom_headers && !caps.custom_headers {
        out.push(w(Severity::Warning, "deploy-warn-no-headers", vec![]));
    }
    if needs.htaccess && !caps.htaccess {
        out.push(w(Severity::Warning, "deploy-warn-no-htaccess", vec![]));
    }
    if let Some(max) = caps.max_site_bytes
        && needs.total_bytes > max
    {
        out.push(w(
            Severity::Blocking,
            "deploy-warn-site-too-big",
            vec![
                ("size", needs.total_bytes.to_string()),
                ("max", max.to_string()),
            ],
        ));
    }
    if let Some(max) = caps.max_file_bytes
        && needs.largest_file_bytes > max
    {
        out.push(w(
            Severity::Blocking,
            "deploy-warn-file-too-big",
            vec![
                ("size", needs.largest_file_bytes.to_string()),
                ("max", max.to_string()),
            ],
        ));
    }
    if let Some(max) = caps.max_files
        && needs.file_count > max
    {
        out.push(w(
            Severity::Blocking,
            "deploy-warn-too-many-files",
            vec![
                ("count", needs.file_count.to_string()),
                ("max", max.to_string()),
            ],
        ));
    }
    if needs.wants_private_source && caps.private_source_on_free_plan == Some(false) {
        out.push(w(Severity::Warning, "deploy-warn-public-repo", vec![]));
    }
    out
}

/// Besoins d'un site déjà généré (dossier de sortie).
pub fn needs_from_dir(dir: &std::path::Path) -> std::io::Result<SiteNeeds> {
    let mut n = SiteNeeds::default();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d)? {
            let e = e?;
            let p = e.path();
            if e.file_type()?.is_dir() {
                stack.push(p);
                continue;
            }
            let len = e.metadata()?.len();
            n.total_bytes += len;
            n.largest_file_bytes = n.largest_file_bytes.max(len);
            n.file_count += 1;
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".php") {
                n.php_files += 1;
            }
            if name == ".htaccess" {
                n.htaccess = true;
                let text = std::fs::read_to_string(&p).unwrap_or_default();
                for l in text.lines().map(str::trim).filter(|l| !l.starts_with('#')) {
                    if l.contains("R=301")
                        || l.starts_with("Redirect 301")
                        || l.starts_with("RedirectMatch 301")
                        || l.starts_with("RedirectPermanent")
                    {
                        n.redirects_301 += 1;
                    }
                    if l.starts_with("Header ") {
                        n.custom_headers = true;
                    }
                }
            }
        }
    }
    Ok(n)
}

/// Interface commune des hébergeurs. Les méthodes d'envoi seront ajoutées
/// à l'étape 1 sans modifier `capabilities`.
pub trait Provider {
    fn capabilities(&self) -> Capabilities;
}

#[cfg(all(test, feature = "github-pages", feature = "sftp"))]
mod tests {
    use super::*;
    use providers::{github_pages, sftp_apache};

    #[test]
    fn alunites_sur_github_pages_avertit() {
        // Profil proche du site Alunites : redirections, contact.php, .htaccess, ~806 Mo.
        let needs = SiteNeeds {
            redirects_301: 40,
            php_files: 1,
            custom_headers: true,
            htaccess: true,
            total_bytes: 806 * 1_000_000,
            largest_file_bytes: 30 * 1_000_000,
            file_count: 4_537,
            wants_private_source: true,
        };
        let keys: Vec<_> = check(&github_pages(), &needs)
            .iter()
            .map(|w| w.key)
            .collect();
        assert!(keys.contains(&"deploy-warn-no-301"));
        assert!(keys.contains(&"deploy-warn-no-php"));
        assert!(keys.contains(&"deploy-warn-public-repo"));
        assert!(!keys.contains(&"deploy-warn-site-too-big"));
        assert!(check(&sftp_apache(), &needs).is_empty());
    }

    #[test]
    fn besoins_d_un_dossier() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join(".htaccess"),
            "# commentaire R=301\nRewriteRule ^a$ /b [R=301,L]\nRedirect 301 /c /d\nHeader set X-Robots-Tag noindex\n",
        )
        .unwrap();
        std::fs::write(d.path().join("contact.php"), "<?php").unwrap();
        let n = needs_from_dir(d.path()).unwrap();
        assert_eq!((n.redirects_301, n.php_files, n.file_count), (2, 1, 2));
        assert!(n.htaccess && n.custom_headers);
    }

    #[test]
    fn site_trop_gros_bloquant() {
        let needs = SiteNeeds {
            total_bytes: 2_000_000_000,
            ..Default::default()
        };
        let warnings = check(&github_pages(), &needs);
        assert_eq!(warnings[0].severity, Severity::Blocking);
    }
}
