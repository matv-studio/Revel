//! Hébergeurs proposés à la première mise en ligne (5.2) : serveur, port et dossier du site
//! habituels, espace client, étapes pour obtenir les accès SFTP ; lecture des accès dans un
//! message de l'hébergeur collé par l'utilisateur.
//!
//! Sources (aides officielles consultées le 16/10/2026) : voir `docs/HEBERGEURS.md`.

/// Hébergeur connu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostPreset {
    /// Identifiant stable, enregistré avec l'hébergement (`HostingProfile::preset`).
    pub id: &'static str,
    pub name: &'static str,
    /// Publication par SFTP (sinon GitHub Pages).
    pub sftp: bool,
    /// Exemple d'adresse du serveur (champ vide).
    pub host_example: &'static str,
    /// Fins d'adresse des serveurs de l'hébergeur (reconnaissance d'un message collé).
    pub host_suffixes: &'static [&'static str],
    pub port: u16,
    /// Dossier du site habituel, relatif au dossier personnel (vide : à choisir sur le serveur).
    pub web_root: &'static str,
    /// Espace client (en français, puis pour les autres langues).
    pub console_fr: &'static str,
    pub console: &'static str,
    /// Étapes pour obtenir les accès SFTP (clés de traduction).
    pub steps: &'static [&'static str],
}

/// Hébergeurs proposés, dans l'ordre de la liste.
pub const PRESETS: &[HostPreset] = &[
    HostPreset {
        id: "ovh",
        name: "OVHcloud",
        sftp: true,
        host_example: "ftp.cluster000.hosting.ovh.net",
        host_suffixes: &[".hosting.ovh.net"],
        port: 22,
        web_root: "www/",
        console_fr: "https://www.ovh.com/manager/",
        console: "https://www.ovh.com/manager/",
        steps: &["host-ovh-sftp", "host-ovh-access"],
    },
    HostPreset {
        id: "o2switch",
        name: "o2switch",
        sftp: true,
        host_example: "serveur.o2switch.net",
        host_suffixes: &[".o2switch.net"],
        port: 22,
        web_root: "public_html/",
        console_fr: "https://clients.o2switch.fr/",
        console: "https://clients.o2switch.fr/",
        steps: &["host-o2switch-ip", "host-o2switch-access"],
    },
    HostPreset {
        id: "infomaniak",
        name: "Infomaniak",
        sftp: true,
        host_example: "abcd.ftp.infomaniak.com",
        host_suffixes: &[".ftp.infomaniak.com"],
        port: 22,
        web_root: "",
        console_fr: "https://manager.infomaniak.com/v3/ng/products/web/hosting",
        console: "https://manager.infomaniak.com/v3/ng/products/web/hosting",
        steps: &["host-infomaniak-access"],
    },
    HostPreset {
        id: "hostinger",
        name: "Hostinger",
        sftp: true,
        host_example: "123.45.67.89",
        host_suffixes: &[],
        port: 65002,
        web_root: "public_html/",
        console_fr: "https://hpanel.hostinger.com/websites",
        console: "https://hpanel.hostinger.com/websites",
        steps: &["host-hostinger-ssh", "host-hostinger-access"],
    },
    HostPreset {
        id: "ionos",
        name: "IONOS",
        sftp: true,
        host_example: "access000000000.webspace-data.io",
        host_suffixes: &[".webspace-data.io"],
        port: 22,
        web_root: "",
        console_fr: "https://my.ionos.fr/",
        console: "https://my.ionos.com/",
        steps: &["host-ionos-create", "host-ionos-access"],
    },
    HostPreset {
        id: "github",
        name: "GitHub Pages",
        sftp: false,
        host_example: "",
        host_suffixes: &[],
        port: 0,
        web_root: "",
        console_fr: "https://github.com/",
        console: "https://github.com/",
        steps: &[],
    },
];

/// Hébergeur par identifiant.
pub fn preset(id: &str) -> Option<&'static HostPreset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// Hébergeur reconnu d'après l'adresse de son serveur.
pub fn preset_for_host(host: &str) -> Option<&'static HostPreset> {
    let h = host.trim().to_ascii_lowercase();
    PRESETS
        .iter()
        .find(|p| p.host_suffixes.iter().any(|s| h.ends_with(s)))
}

/// Accès lus dans un message collé (courriel d'installation, page de l'espace client).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PastedAccess {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub password: Option<String>,
}

impl PastedAccess {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Field {
    Host,
    Port,
    User,
    Password,
}

/// Libellé d'une ligne `libellé : valeur` (français, anglais) -> champ.
fn field_of(label: &str) -> Option<Field> {
    let l = label
        .trim()
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    let has = |w: &[&str]| w.iter().any(|w| l.contains(w));
    if l.is_empty() || l.len() > 48 {
        return None;
    }
    // Ordre : « mot de passe FTP » avant « FTP », « port SFTP » avant « serveur SFTP ».
    if has(&["mot de passe", "password", "passwort", "mdp", "pass"]) {
        Some(Field::Password)
    } else if l == "port" || l.starts_with("port ") || l.ends_with(" port") {
        Some(Field::Port)
    } else if has(&[
        "login",
        "identifiant",
        "utilisateur",
        "user",
        "benutzer",
        "account",
        "compte",
    ]) {
        Some(Field::User)
    } else if has(&[
        "serveur", "server", "host", "hôte", "hote", "adresse", "address", "hostname", "ip",
    ]) {
        Some(Field::Host)
    } else {
        None
    }
}

/// Adresse de serveur plausible (nom avec un point, ou IPv4), sans protocole ni chemin.
fn clean_host(v: &str) -> Option<String> {
    let v = v
        .trim()
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '<' | '>' | '`'));
    let v = v.split_once("://").map(|(_, r)| r).unwrap_or(v);
    let v = v.rsplit_once('@').map(|(_, h)| h).unwrap_or(v);
    let v = v.split(['/', ' ']).next().unwrap_or_default();
    let v = v.split_once(':').map(|(h, _)| h).unwrap_or(v);
    let v = v.trim_end_matches('.').to_ascii_lowercase();
    let ok = v.contains('.')
        && v.len() <= 253
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        && !v.starts_with(['.', '-']);
    ok.then_some(v)
}

/// Port d'une adresse `hote:port` ou `sftp://hote:port`.
fn port_of(v: &str) -> Option<u16> {
    let v = v.trim();
    let v = v.split_once("://").map(|(_, r)| r).unwrap_or(v);
    let v = v.rsplit_once('@').map(|(_, h)| h).unwrap_or(v);
    let v = v.split('/').next().unwrap_or_default();
    v.split_once(':')?.1.trim().parse().ok().filter(|p| *p > 0)
}

/// Valeur d'une ligne : un seul mot (guillemets retirés) ; une phrase (« non affiché ») ou un
/// masque (`********`) ne sont pas des valeurs.
fn word(v: &str) -> Option<String> {
    let v = v
        .trim()
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '`' | '«' | '»'))
        .trim();
    let mut words = v.split_whitespace();
    let w = words.next()?;
    let masked = w.chars().all(|c| matches!(c, '*' | '•' | '.'));
    (words.next().is_none() && !masked && !w.starts_with('(')).then(|| w.to_string())
}

/// Lit les accès SFTP dans un texte collé : lignes `Serveur : …`, `Login : …`, `Mot de passe : …`,
/// adresse `sftp://utilisateur@serveur:port`, ou adresse d'un serveur d'hébergeur connu.
pub fn parse_access(text: &str) -> PastedAccess {
    let mut out = PastedAccess::default();
    // Libellé seul sur sa ligne, valeur sur la suivante.
    let mut pending: Option<Field> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        // URL sftp://utilisateur@serveur:port
        if let Some(i) = line.find("sftp://") {
            let url = &line[i..];
            let url = url.split_whitespace().next().unwrap_or(url);
            let rest = &url["sftp://".len()..];
            if let Some((u, _)) = rest.split('/').next().unwrap_or_default().rsplit_once('@') {
                out.user.get_or_insert_with(|| u.to_string());
            }
            if let Some(h) = clean_host(url) {
                out.host.get_or_insert(h);
            }
            if let Some(p) = port_of(url) {
                out.port.get_or_insert(p);
            }
            pending = None;
            continue;
        }
        let split = line
            .find([':', '='])
            .map(|i| (&line[..i], &line[i + 1..]))
            .or_else(|| line.split_once('\t'));
        let (field, value) = match split {
            Some((label, value)) => match field_of(label) {
                // Libellé suivi de « : » seul : valeur sur la ligne suivante.
                Some(f) if value.trim().is_empty() => {
                    pending = Some(f);
                    continue;
                }
                Some(f) => (Some(f), value.trim()),
                None => (None, line),
            },
            None => match field_of(line) {
                // Libellé seul : valeur sur la ligne suivante.
                Some(f) if line.split_whitespace().count() <= 5 => {
                    pending = Some(f);
                    continue;
                }
                _ => (pending.take(), line),
            },
        };
        let field = field.or(pending.take());
        match field {
            Some(Field::Host) => {
                if let Some(h) = clean_host(value) {
                    if let Some(p) = port_of(value) {
                        out.port.get_or_insert(p);
                    }
                    out.host.get_or_insert(h);
                }
            }
            Some(Field::Port) => {
                if let Some(p) = word(value).and_then(|w| w.parse().ok()).filter(|p| *p > 0) {
                    out.port.get_or_insert(p);
                }
            }
            Some(Field::User) => {
                if let Some(w) = word(value) {
                    out.user.get_or_insert(w);
                }
            }
            Some(Field::Password) => {
                if let Some(w) = word(value) {
                    out.password.get_or_insert(w);
                }
            }
            None => {
                // Adresse d'un serveur d'hébergeur connu au milieu d'une phrase.
                if out.host.is_none() {
                    out.host = line
                        .split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '(' | ')'))
                        .filter_map(clean_host)
                        .find(|h| preset_for_host(h).is_some());
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courriel_ovh() {
        let a = parse_access(
            "Bonjour,\n\nVotre hébergement est installé.\n\nServeur FTP et SFTP : ftp.cluster031.hosting.ovh.net\nLogin principal : alunitp\nMot de passe : Xy7-kQ2pLm\n\nCordialement",
        );
        assert_eq!(a.host.as_deref(), Some("ftp.cluster031.hosting.ovh.net"));
        assert_eq!(a.user.as_deref(), Some("alunitp"));
        assert_eq!(a.password.as_deref(), Some("Xy7-kQ2pLm"));
        assert_eq!(a.port, None);
        assert_eq!(
            preset_for_host(a.host.as_deref().unwrap()).map(|p| p.id),
            Some("ovh")
        );
    }

    #[test]
    fn libelles_anglais_et_valeur_a_la_ligne() {
        let a = parse_access(
            "SFTP details\nHost\n185.12.34.56\nPort\n65002\nUsername\nu12345678\nPassword:\n\"s3cr3t!\"",
        );
        assert_eq!(a.host.as_deref(), Some("185.12.34.56"));
        assert_eq!(a.port, Some(65002));
        assert_eq!(a.user.as_deref(), Some("u12345678"));
        assert_eq!(a.password.as_deref(), Some("s3cr3t!"));
    }

    #[test]
    fn adresse_sftp() {
        let a = parse_access("Connexion : sftp://u987654@access123456789.webspace-data.io:22/");
        assert_eq!(a.host.as_deref(), Some("access123456789.webspace-data.io"));
        assert_eq!(a.user.as_deref(), Some("u987654"));
        assert_eq!(a.port, Some(22));
        assert_eq!(
            preset_for_host("ACCESS1.webspace-data.io").map(|p| p.id),
            Some("ionos")
        );
    }

    #[test]
    fn serveur_dans_une_phrase() {
        let a = parse_access(
            "Connectez-vous à abcd.ftp.infomaniak.com avec l'utilisateur ci-dessous.\nUtilisateur = abcd_site",
        );
        assert_eq!(a.host.as_deref(), Some("abcd.ftp.infomaniak.com"));
        assert_eq!(a.user.as_deref(), Some("abcd_site"));
        assert!(parse_access("Merci de votre confiance.").is_empty());
        let a = parse_access("Login : alunitp\nMot de passe : non affiché\nPassword: ********");
        assert_eq!(a.user.as_deref(), Some("alunitp"));
        assert_eq!(a.password, None);
    }
}
