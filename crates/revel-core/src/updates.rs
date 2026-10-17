//! Mises à jour de Revel : préversions (6.6, D-86).
//!
//! L'adresse inscrite dans l'application (`https://github.com/<compte>/<dépôt>/releases/latest/
//! download/latest.json`) ne mène qu'à la dernière version publiée hors préversion : GitHub ne
//! marque jamais une préversion comme « dernière ». Avec « Recevoir les préversions », la liste des
//! versions publiées du même dépôt est lue (API publique, sans compte) et la plus récente,
//! préversion ou non, qui porte un `latest.json` est retenue. Les brouillons ne sont jamais
//! visibles sans compte, donc jamais proposés.

use revel_import::crawl::{Fetch, Http, get_json};

/// Version publiée retenue pour les mises à jour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feed {
    /// Adresse de son `latest.json`.
    pub url: String,
    /// Numéro de version (sans le `v` de l'étiquette).
    pub version: String,
    pub prerelease: bool,
}

/// Liste des versions publiées du dépôt GitHub d'une adresse de mise à jour
/// (`https://github.com/<compte>/<dépôt>/releases/…`) ; `None` hors GitHub.
pub fn releases_api(endpoint: &str) -> Option<String> {
    let rest = endpoint.strip_prefix("https://github.com/")?;
    let mut parts = rest.split('/');
    let (owner, repo, kind) = (parts.next()?, parts.next()?, parts.next()?);
    let name = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    (name(owner) && name(repo) && kind == "releases")
        .then(|| format!("https://api.github.com/repos/{owner}/{repo}/releases?per_page=30"))
}

/// Plus récente version (numéro de version le plus élevé) parmi les versions publiées qui portent
/// un `latest.json`, préversions comprises ; brouillons et étiquettes hors numéro de version
/// écartés.
pub fn newest(releases: &serde_json::Value) -> Option<Feed> {
    releases
        .as_array()?
        .iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let tag = r["tag_name"].as_str()?;
            let version = semver::Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok()?;
            let url = r["assets"]
                .as_array()?
                .iter()
                .find(|a| a["name"] == "latest.json")?["browser_download_url"]
                .as_str()?
                .to_string();
            let feed = Feed {
                url,
                version: version.to_string(),
                prerelease: r["prerelease"].as_bool().unwrap_or(false),
            };
            Some((version, feed))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, feed)| feed)
}

/// Version à proposer quand les préversions sont demandées ; `None` (adresse hors GitHub, accès
/// impossible, aucune version qui convienne) : l'adresse inscrite dans l'application sert seule.
pub fn prerelease_feed(fetch: &dyn Fetch, endpoint: &str) -> Option<Feed> {
    let api = releases_api(endpoint)?;
    let headers = [
        ("Accept", "application/vnd.github+json"),
        ("X-GitHub-Api-Version", "2022-11-28"),
    ];
    match get_json(fetch, &api, &headers) {
        Ok(list) => newest(&list),
        Err((status, detail)) => {
            eprintln!("préversions : liste des versions illisible ({status}) {detail}");
            None
        }
    }
}

/// [`prerelease_feed`] en ligne.
pub fn prerelease_feed_online(endpoint: &str) -> Option<Feed> {
    prerelease_feed(&Http::new(), endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ENDPOINT: &str =
        "https://github.com/matv-studio/revel/releases/latest/download/latest.json";

    fn release(tag: &str, draft: bool, pre: bool, with_feed: bool) -> serde_json::Value {
        let mut assets =
            vec![json!({"name": "Revel.dmg", "browser_download_url": "https://x/Revel.dmg"})];
        if with_feed {
            assets.push(json!({
                "name": "latest.json",
                "browser_download_url": format!("https://github.com/matv-studio/revel/releases/download/{tag}/latest.json")
            }));
        }
        json!({"tag_name": tag, "draft": draft, "prerelease": pre, "assets": assets})
    }

    #[test]
    fn adresse_de_la_liste() {
        assert_eq!(
            releases_api(ENDPOINT).as_deref(),
            Some("https://api.github.com/repos/matv-studio/revel/releases?per_page=30")
        );
        assert_eq!(releases_api("https://exemple.fr/latest.json"), None);
        assert_eq!(
            releases_api("https://github.com/matv-studio/revel/tree/main"),
            None
        );
        assert_eq!(
            releases_api("https://github.com/a b/revel/releases/x"),
            None
        );
    }

    #[test]
    fn plus_recente_preversion_comprise() {
        let list = json!([
            release("v0.7.0", true, true, true),
            release("v0.6.2-beta.1", false, true, true),
            release("v0.6.1", false, false, true),
            release("v0.6.3", false, true, false),
            release("step-7", false, false, true),
            release("v0.6.0", false, true, true),
        ]);
        let feed = newest(&list).unwrap();
        assert_eq!(feed.version, "0.6.2-beta.1");
        assert!(feed.prerelease);
        assert!(feed.url.ends_with("/v0.6.2-beta.1/latest.json"));
        // Une version normale plus récente l'emporte sur la préversion.
        let list = json!([
            release("v0.6.2-beta.1", false, true, true),
            release("v0.6.2", false, false, true)
        ]);
        let feed = newest(&list).unwrap();
        assert_eq!((feed.version.as_str(), feed.prerelease), ("0.6.2", false));
        assert_eq!(newest(&json!([])), None);
        assert_eq!(newest(&json!({"message": "API rate limit exceeded"})), None);
    }

    struct Api(u16, &'static str);

    impl Fetch for Api {
        fn get(
            &self,
            url: &revel_import::crawl::Url,
        ) -> Result<revel_import::crawl::Fetched, String> {
            assert!(
                url.as_str()
                    .starts_with("https://api.github.com/repos/matv-studio/revel/releases")
            );
            Ok(revel_import::crawl::Fetched {
                status: self.0,
                url: url.clone(),
                content_type: "application/json".into(),
                body: self.1.as_bytes().to_vec(),
            })
        }
    }

    #[test]
    fn lecture_de_la_liste() {
        let body = r#"[{"tag_name":"v0.6.1","draft":false,"prerelease":true,"assets":[{"name":"latest.json","browser_download_url":"https://github.com/matv-studio/revel/releases/download/v0.6.1/latest.json"}]}]"#;
        let feed = prerelease_feed(&Api(200, body), ENDPOINT).unwrap();
        assert_eq!((feed.version.as_str(), feed.prerelease), ("0.6.1", true));
        // Limite de l'API atteinte : l'adresse inscrite dans l'application sert seule.
        assert_eq!(
            prerelease_feed(&Api(403, r#"{"message":"rate limit"}"#), ENDPOINT),
            None
        );
    }
}
