//! Capacités déclarées par hébergeur.
//!
//! Chaque valeur est sourcée ; la revérifier avant toute modification.

use crate::{Capabilities, Provider};

#[allow(dead_code)] // inutilisées si certains hébergeurs sont désactivés
const GB: u64 = 1_000_000_000;
#[allow(dead_code)]
const MIB: u64 = 1024 * 1024;

/// GitHub Pages (vérifié le 13/10/2026) :
/// - site publié de 1 Go maximum ; pas de PHP ni de `.htaccess` ;
/// - dépôt public obligatoire sur un compte gratuit ;
/// - fichiers de plus de 100 Mio refusés par GitHub (limite des dépôts).
///
/// Sources : <https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits>,
/// <https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github>
#[cfg(feature = "github-pages")]
pub fn github_pages() -> Capabilities {
    Capabilities {
        id: "github-pages",
        redirects_301: false,
        php: false,
        custom_headers: false,
        htaccess: false,
        max_site_bytes: Some(GB),
        max_file_bytes: Some(100 * MIB),
        max_files: None,
        private_source_on_free_plan: Some(false),
    }
}

/// Hébergement mutualisé Apache par SFTP (profil OVH vérifié le 13/10/2026 :
/// `.htaccess` et PHP actifs ; FTPS refusé, SFTP sur le port 22).
#[cfg(feature = "sftp")]
pub fn sftp_apache() -> Capabilities {
    Capabilities {
        id: "sftp-apache",
        redirects_301: true,
        php: true,
        custom_headers: true,
        htaccess: true,
        max_site_bytes: None,
        max_file_bytes: None,
        max_files: None,
        private_source_on_free_plan: None,
    }
}

/// Cloudflare Workers, fichiers statiques (vérifié le 13/10/2026) :
/// - 25 Mio par fichier ; 20 000 fichiers par version (offre gratuite) ;
/// - redirections 301 via `_redirects` (2 000 statiques maximum), en-têtes via `_headers` ;
/// - pas de PHP.
///
/// Sources : <https://developers.cloudflare.com/workers/platform/limits/>,
/// <https://developers.cloudflare.com/workers/static-assets/redirects/>
#[cfg(feature = "cloudflare")]
pub fn cloudflare() -> Capabilities {
    Capabilities {
        id: "cloudflare",
        redirects_301: true,
        php: false,
        custom_headers: true,
        htaccess: false,
        max_site_bytes: None,
        max_file_bytes: Some(25 * MIB),
        max_files: Some(20_000),
        private_source_on_free_plan: None,
    }
}

/// Liste des hébergeurs compilés (selon les fonctions activées).
#[allow(clippy::vec_init_then_push, unused_mut)] // la liste dépend des fonctions compilées
pub fn all() -> Vec<Capabilities> {
    let mut v = Vec::new();
    #[cfg(feature = "github-pages")]
    v.push(github_pages());
    #[cfg(feature = "sftp")]
    v.push(sftp_apache());
    #[cfg(feature = "cloudflare")]
    v.push(cloudflare());
    v
}

/// Hébergeur décrit uniquement par ses capacités (utile pour les tests et l'aperçu).
pub struct Static(pub Capabilities);

impl Provider for Static {
    fn capabilities(&self) -> Capabilities {
        self.0.clone()
    }
}
