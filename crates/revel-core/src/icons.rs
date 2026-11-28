//! Bibliothèques d'icônes en ligne (D-205) : dépôts GitHub publics de fichiers SVG (Lucide, Tabler,
//! Phosphor…). La liste des noms est lue une fois, puis gardée dans le dossier des réglages
//! (relue au-delà d'une semaine, gardée faute de connexion) ; chaque dessin est lu quand il
//! s'affiche, puis gardé. Une icône choisie est copiée dans le site : le site ne dépend jamais de
//! la bibliothèque en ligne.

use crate::{Core, CoreError, Result};
use revel_protocol::{IconDrawing, IconIndex, IconLibrary};
use std::path::PathBuf;

/// Liste relue au-delà de ce délai (secondes).
const MAX_AGE: u64 = 7 * 24 * 3600;
/// Taille maximale d'un fichier SVG lu.
const SVG_MAX: u64 = 256 * 1024;
/// Dessins lus à la fois.
const PARALLEL: usize = 8;

fn err(detail: &str) -> CoreError {
    CoreError::Template("error-icon-library", detail.to_string())
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Dépôt `propriétaire/nom` et dossier vérifiés (pas de `..`, caractères sûrs).
fn checked(lib: &IconLibrary) -> Result<(String, String)> {
    let ok = |s: &str| {
        !s.is_empty()
            && !s.starts_with('.')
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    let repo = lib.repo.trim().trim_matches('/');
    let (o, n) = repo.split_once('/').ok_or_else(|| err(repo))?;
    if !ok(o) || !ok(n) || n.contains('/') {
        return Err(err(repo));
    }
    let dir = lib.dir.trim().trim_matches('/');
    if !dir.is_empty() && !dir.split('/').all(ok) {
        return Err(err(dir));
    }
    Ok((repo.to_string(), dir.to_string()))
}

/// Nom d'icône sûr (nom de fichier sans `.svg`).
fn safe_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 120
        && !n.starts_with('.')
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
}

impl Core {
    fn icons_dir(&self, repo: &str, dir: &str) -> PathBuf {
        let key = format!("{repo}/{dir}").replace('/', "__");
        self.config_dir.join("icon-libraries").join(key)
    }

    /// Client de l'API de GitHub : jeton du compte connecté s'il y en a un (limite d'appels plus
    /// haute), sinon anonyme (dépôt public).
    fn icons_client(&self) -> revel_git::github::Client {
        let token = self
            .github_token_raw()
            .ok()
            .flatten()
            .map(|t| t.access_token)
            .unwrap_or_default();
        revel_git::github::Client::new(&token)
    }

    /// Noms des icônes d'une bibliothèque : liste gardée si elle a moins d'une semaine (ou
    /// `refresh` faux et pas de connexion), sinon relue sur GitHub.
    pub fn icon_library(&self, lib: &IconLibrary, refresh: bool) -> Result<IconIndex> {
        let (repo, dir) = checked(lib)?;
        let file = self.icons_dir(&repo, &dir).join("index.json");
        let kept: Option<IconIndex> = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        if let Some(k) = &kept
            && !refresh
            && now().saturating_sub(k.fetched) < MAX_AGE
        {
            return Ok(k.clone());
        }
        match self.read_icon_names(&repo, &dir) {
            Ok(idx) => {
                if let Some(p) = file.parent() {
                    std::fs::create_dir_all(p)?;
                }
                std::fs::write(&file, serde_json::to_vec(&idx).unwrap_or_default())?;
                Ok(idx)
            }
            Err(e) => match kept {
                Some(mut k) => {
                    k.offline = true;
                    Ok(k)
                }
                None => Err(e),
            },
        }
    }

    /// Lecture en ligne : dépôt (branche par défaut, licence), puis arbre du dossier, segment par
    /// segment (pas d'arbre complet : la liste d'un gros dépôt serait tronquée).
    fn read_icon_names(&self, repo: &str, dir: &str) -> Result<IconIndex> {
        let gh = self.icons_client();
        let get = |path: &str| -> Result<serde_json::Value> {
            let (s, v) = gh
                .call("GET", path, None)
                .map_err(|e| err(&e.to_string()))?;
            match s {
                200 => Ok(v),
                404 => Err(err(&format!("404 {path}"))),
                _ => Err(err(&format!("{s} {path}"))),
            }
        };
        let info = get(&format!("/repos/{repo}"))?;
        let branch = info["default_branch"]
            .as_str()
            .unwrap_or("main")
            .to_string();
        let license = info["license"]["spdx_id"]
            .as_str()
            .filter(|l| *l != "NOASSERTION")
            .unwrap_or_default()
            .to_string();
        let mut tree = get(&format!("/repos/{repo}/git/trees/{branch}"))?;
        for seg in dir.split('/').filter(|s| !s.is_empty()) {
            let sha = tree["tree"]
                .as_array()
                .and_then(|l| {
                    l.iter()
                        .find(|e| e["path"].as_str() == Some(seg) && e["type"] == "tree")
                })
                .and_then(|e| e["sha"].as_str())
                .ok_or_else(|| err(&format!("{repo}/{dir}")))?
                .to_string();
            tree = get(&format!("/repos/{repo}/git/trees/{sha}"))?;
        }
        let mut names: Vec<String> = tree["tree"]
            .as_array()
            .map(|l| {
                l.iter()
                    .filter(|e| e["type"] == "blob")
                    .filter_map(|e| e["path"].as_str()?.strip_suffix(".svg"))
                    .filter(|n| safe_name(n))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        if names.is_empty() {
            return Err(err(&format!("{repo}/{dir} : 0 SVG")));
        }
        names.sort();
        Ok(IconIndex {
            names,
            license,
            fetched: now(),
            offline: false,
        })
    }

    /// Dessins de quelques icônes (celles affichées) : gardés, sinon lus sur GitHub. Une icône
    /// illisible est omise.
    pub fn icon_drawings(&self, lib: &IconLibrary, names: &[String]) -> Result<Vec<IconDrawing>> {
        let (repo, dir) = checked(lib)?;
        let cache = self.icons_dir(&repo, &dir).join("svg");
        let names: Vec<&String> = names.iter().filter(|n| safe_name(n)).take(400).collect();
        let mut texts: Vec<Option<String>> = names
            .iter()
            .map(|n| std::fs::read_to_string(cache.join(format!("{n}.svg"))).ok())
            .collect();
        let missing: Vec<usize> = (0..names.len()).filter(|i| texts[*i].is_none()).collect();
        if !missing.is_empty() {
            let agent = crate::fonts::agent(30);
            let fetched: Vec<(usize, Option<String>)> = std::thread::scope(|s| {
                missing
                    .chunks(missing.len().div_ceil(PARALLEL))
                    .map(|chunk| {
                        let agent = agent.clone();
                        let names = &names;
                        let (repo, dir) = (&repo, &dir);
                        s.spawn(move || {
                            chunk
                                .iter()
                                .map(|i| {
                                    let path = if dir.is_empty() {
                                        format!("{}.svg", names[*i])
                                    } else {
                                        format!("{dir}/{}.svg", names[*i])
                                    };
                                    let url = crate::sharing::raw_url(repo, &path);
                                    (*i, read_text(&agent, &url))
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .flat_map(|h| h.join().unwrap_or_default())
                    .collect()
            });
            let _ = std::fs::create_dir_all(&cache);
            for (i, t) in fetched {
                if let Some(t) = t {
                    let _ = std::fs::write(cache.join(format!("{}.svg", names[i])), &t);
                    texts[i] = Some(t);
                }
            }
        }
        Ok(names
            .iter()
            .zip(texts)
            .filter_map(|(n, t)| drawing(n, &t?))
            .collect())
    }
}

fn read_text(agent: &ureq::Agent, url: &str) -> Option<String> {
    let mut res = agent.get(url).call().ok()?;
    if res.status().as_u16() != 200 {
        return None;
    }
    res.body_mut()
        .with_config()
        .limit(SVG_MAX)
        .read_to_string()
        .ok()
}

/// Dessin prêt à copier : `viewBox`, contenu nettoyé, peinture du `<svg>` d'origine.
pub(crate) fn drawing(name: &str, text: &str) -> Option<IconDrawing> {
    let (view_box, inner) = revel_edit::icons::clean_svg(text)?;
    Some(IconDrawing {
        name: name.to_string(),
        view_box,
        inner,
        paint: revel_edit::icons::svg_paint(text),
    })
}
