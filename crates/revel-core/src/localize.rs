//! Images en ligne copiées dans le site (D-128) : les modèles lient des photos libres
//! d'Unsplash ; un site importé peut lier des images d'un autre serveur. « Copier dans le site »
//! les télécharge dans les médias (`photos/`) et remplace leurs adresses partout où elles sont
//! écrites : éléments (`src`, `poster`, `srcset`), images, valeurs de composant (utilisations
//! et valeurs par défaut), informations du site, contenus du CMS. Une seule étape d'annulation ;
//! les images de Revel (AVIF, WebP) sont ensuite produites comme pour les autres médias.

use crate::{Core, CoreError, Result};
use revel_edit::{Command, EditState, Owner, Parent};
use revel_model::{Node, Site};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Serveurs d'images reconnus même sans extension dans l'adresse.
const IMAGE_HOSTS: &[&str] = &[
    "images.unsplash.com",
    "plus.unsplash.com",
    "images.pexels.com",
    "cdn.pixabay.com",
    "picsum.photos",
];

/// Adresse d'une image en ligne (serveur d'images connu, ou extension d'image).
pub fn is_online_image(url: &str) -> bool {
    let u = url.trim();
    let Some(rest) = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))
    else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    // Images d'attente d'un modèle partagé (D-132) : à remplacer, pas à copier.
    if host == "placehold.co" {
        return false;
    }
    if IMAGE_HOSTS.contains(&host) {
        return true;
    }
    let path = rest
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    [".jpg", ".jpeg", ".png", ".webp", ".avif", ".gif", ".svg"]
        .iter()
        .any(|e| path.ends_with(e))
}

/// Adresses d'images en ligne dans un texte (adresse entière, jusqu'au premier guillemet, espace,
/// parenthèse ou chevron).
pub fn urls_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("http") {
        let tail = &rest[i..];
        let end = tail
            .find(|c: char| c.is_whitespace() || "\"'<>)`,".contains(c))
            .unwrap_or(tail.len());
        let u = &tail[..end];
        if is_online_image(u) && !out.iter().any(|x| x == u) {
            out.push(u.to_string());
        }
        rest = &tail[end.max(4)..];
    }
    out
}

/// Expression qui est une adresse écrite (`"https://…"`).
fn literal(expr: &str) -> Option<String> {
    serde_json::from_str::<String>(expr.trim()).ok()
}

fn nodes_urls(nodes: &[Node], out: &mut BTreeSet<String>) {
    revel_model::ids::walk(nodes, &mut |n| match n {
        Node::Element { attrs, .. } => {
            for a in attrs {
                if let Some(v) = &a.value {
                    out.extend(urls_in(v));
                }
            }
        }
        Node::Image(i) => {
            if let Some(u) = literal(&i.src).filter(|u| is_online_image(u)) {
                out.insert(u);
            }
        }
        Node::Use { args, attrs, .. } => {
            for v in args.values() {
                if let Some(u) = literal(v).filter(|u| is_online_image(u)) {
                    out.insert(u);
                }
            }
            for a in attrs {
                if let Some(v) = &a.value {
                    out.extend(urls_in(v));
                }
            }
        }
        _ => {}
    });
}

fn value_urls(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::String(s) if is_online_image(s) => {
            out.insert(s.clone());
        }
        Value::Array(a) => a.iter().for_each(|x| value_urls(x, out)),
        Value::Object(m) => m.values().for_each(|x| value_urls(x, out)),
        _ => {}
    }
}

/// Contenus du CMS (dossier de collection, nom du fichier sans extension) et leur texte.
fn entries(dir: &Path, site: &Site) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for c in &site.collections {
        let Ok(rd) = std::fs::read_dir(dir.join(&c.folder)) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
            if !matches!(ext, "md" | "markdown") {
                continue;
            }
            if let (Some(stem), Ok(text)) = (
                p.file_stem().and_then(|s| s.to_str()),
                std::fs::read_to_string(&p),
            ) {
                out.push((c.folder.clone(), stem.to_string(), text));
            }
        }
    }
    out
}

/// Images en ligne du site : document et contenus du CMS.
pub fn online_images(dir: &Path, site: &Site) -> Vec<String> {
    let mut out = BTreeSet::new();
    for p in &site.pages {
        nodes_urls(&p.body, &mut out);
    }
    for l in &site.layouts {
        nodes_urls(&l.body, &mut out);
    }
    for c in &site.components {
        nodes_urls(&c.body, &mut out);
        for d in &c.props {
            if let Some(v) = &d.default {
                value_urls(v, &mut out);
            }
        }
    }
    for v in site.data.values() {
        value_urls(v, &mut out);
    }
    for (_, _, text) in entries(dir, site) {
        out.extend(urls_in(&text));
    }
    out.into_iter().collect()
}

/// Nom de fichier d'une image en ligne : dernier morceau de l'adresse, largeur demandée,
/// empreinte courte de l'adresse entière (deux tailles d'une même photo restent distinctes).
fn file_name(url: &str, ext: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let last = path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("image");
    let stem = last.rsplit_once('.').map(|(s, _)| s).unwrap_or(last);
    let mut slug = revel_model::slugify(stem);
    if slug.is_empty() {
        slug = "image".into();
    }
    slug.truncate(48);
    let width = url
        .split(['?', '&'])
        .find_map(|p| p.strip_prefix("w="))
        .map(|w| format!("-{w}"))
        .unwrap_or_default();
    let hash = blake3::hash(url.as_bytes()).to_hex();
    format!("{slug}{width}-{}.{ext}", &hash[..6])
}

/// Extension d'après le type de la réponse, sinon d'après l'adresse.
fn extension(content_type: &str, url: &str) -> String {
    let t = content_type.to_ascii_lowercase();
    for (m, e) in [
        ("jpeg", "jpg"),
        ("png", "png"),
        ("webp", "webp"),
        ("avif", "avif"),
        ("gif", "gif"),
        ("svg", "svg"),
    ] {
        if t.contains(m) {
            return e.into();
        }
    }
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    path.rsplit_once('.')
        .map(|(_, e)| e.to_string())
        .filter(|e| e.len() <= 4)
        .unwrap_or_else(|| "jpg".into())
}

fn swap_str(s: &str, map: &BTreeMap<String, String>) -> String {
    let mut out = s.to_string();
    for (from, to) in map {
        if out.contains(from.as_str()) {
            out = out.replace(from.as_str(), to);
        }
    }
    out
}

fn swap_nodes(nodes: &mut [Node], map: &BTreeMap<String, String>) {
    for n in nodes.iter_mut() {
        match n {
            Node::Element { attrs, .. } => {
                for a in attrs.iter_mut() {
                    if let Some(v) = &mut a.value {
                        *v = swap_str(v, map);
                    }
                }
            }
            Node::Image(i) => {
                if let Some(u) = literal(&i.src)
                    && let Some(to) = map.get(&u)
                {
                    i.src = serde_json::to_string(to).unwrap_or_default();
                }
            }
            Node::Use { args, attrs, .. } => {
                for v in args.values_mut() {
                    if let Some(u) = literal(v)
                        && let Some(to) = map.get(&u)
                    {
                        *v = serde_json::to_string(to).unwrap_or_default();
                    }
                }
                for a in attrs.iter_mut() {
                    if let Some(v) = &mut a.value {
                        *v = swap_str(v, map);
                    }
                }
            }
            _ => {}
        }
        for l in revel_model::ids::child_lists_mut(n) {
            swap_nodes(l, map);
        }
    }
}

fn swap_value(v: &mut Value, map: &BTreeMap<String, String>) {
    match v {
        Value::String(s) => {
            if let Some(to) = map.get(s.as_str()) {
                *s = to.clone();
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| swap_value(x, map)),
        Value::Object(m) => m.values_mut().for_each(|x| swap_value(x, map)),
        _ => {}
    }
}

/// Commandes qui remplacent les adresses de `map` dans le document et les contenus.
pub fn commands(dir: &Path, site: &Site, map: &BTreeMap<String, String>) -> Vec<Command> {
    let mut out = Vec::new();
    for p in &site.pages {
        let mut body = p.body.clone();
        swap_nodes(&mut body, map);
        for (i, (old, new)) in p.body.iter().zip(body).enumerate() {
            if *old != new {
                out.push(Command::Replace {
                    parent: Parent::root(Owner::Page(p.id.clone())),
                    index: i,
                    node: new,
                });
            }
        }
    }
    for l in &site.layouts {
        let mut body = l.body.clone();
        swap_nodes(&mut body, map);
        if body != l.body {
            out.push(Command::SetLayoutBody {
                id: l.id.clone(),
                body,
            });
        }
    }
    for c in &site.components {
        let mut body = c.body.clone();
        swap_nodes(&mut body, map);
        if body != c.body {
            out.push(Command::SetComponentBody {
                id: c.id.clone(),
                body,
            });
        }
        let mut props = c.props.clone();
        for d in &mut props {
            if let Some(v) = &mut d.default {
                swap_value(v, map);
            }
        }
        if props != c.props {
            out.push(Command::SetComponentProps {
                id: c.id.clone(),
                props,
            });
        }
    }
    for (k, v) in &site.data {
        let mut new = v.clone();
        swap_value(&mut new, map);
        if new != *v {
            out.push(Command::SetGlobal {
                path: vec![k.clone()],
                value: Some(new),
            });
        }
    }
    for (folder, file, text) in entries(dir, site) {
        for u in urls_in(&text) {
            if let Some(to) = map.get(&u) {
                out.push(Command::ReplaceInEntry {
                    folder: folder.clone(),
                    file: file.clone(),
                    find: u,
                    replace: to.clone(),
                });
            }
        }
    }
    out
}

impl Core {
    /// Images en ligne du site ouvert (adresses).
    pub fn media_online(&self, path: &Path) -> Result<Vec<String>> {
        let site = self.edit_site(path)?;
        Ok(online_images(path, &site))
    }

    /// Copie les images en ligne dans les médias du site (`photos/`) et remplace leurs adresses
    /// (une étape d'annulation). Une image qui ne se télécharge pas garde son adresse ; renvoie
    /// l'état du site et le nombre d'images copiées.
    pub fn media_localize(&self, path: &Path) -> Result<(EditState, usize)> {
        let site = self.edit_site(path)?;
        let urls = online_images(path, &site);
        if urls.is_empty() {
            return Err(CoreError::Other("aucune image en ligne".into()));
        }
        let (media, prefix) = self.media_dir(path)?;
        let stage = path
            .join(revel_project::CACHE_DIR)
            .join("tmp")
            .join("photos");
        let _ = std::fs::remove_dir_all(&stage);
        std::fs::create_dir_all(&stage)?;
        let a = crate::fonts::agent(60);
        let mut map = BTreeMap::new();
        let mut moves = Vec::new();
        for u in &urls {
            use std::io::Read;
            let Ok(res) = a.get(u.as_str()).call() else {
                continue;
            };
            let ct = res
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            if !ct.is_empty() && !ct.starts_with("image/") {
                continue;
            }
            let mut data = Vec::new();
            if res
                .into_body()
                .into_reader()
                .take(40 * 1024 * 1024)
                .read_to_end(&mut data)
                .is_err()
                || data.is_empty()
            {
                continue;
            }
            let name = file_name(u, &extension(&ct, u));
            let staged = stage.join(&name);
            std::fs::write(&staged, &data)?;
            let target = media.join("photos").join(&name);
            if !target.exists() {
                let rel = |p: &Path| {
                    p.strip_prefix(path)
                        .ok()
                        .map(|r| r.to_string_lossy().replace('\\', "/"))
                };
                if let (Some(f), Some(t)) = (rel(&staged), rel(&target)) {
                    moves.push((f, t));
                }
            }
            map.insert(u.clone(), format!("{prefix}/photos/{name}"));
        }
        if map.is_empty() {
            return Err(CoreError::Other(
                "aucune image n'a pu être téléchargée (connexion ?)".into(),
            ));
        }
        let mut list = Vec::new();
        if !moves.is_empty() {
            list.push(Command::MoveFiles { moves });
        }
        list.extend(commands(path, &site, &map));
        let n = map.len();
        let state = self.edit_apply(path, Command::Batch { commands: list }, false)?;
        Ok((state, n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresses_d_images() {
        assert!(is_online_image(
            "https://images.unsplash.com/photo-1?w=800&q=80"
        ));
        assert!(is_online_image("https://exemple.fr/a/b.JPG"));
        assert!(!is_online_image("https://exemple.fr/page/"));
        assert!(!is_online_image("/media/a.jpg"));
        let t = r#"image: "https://images.unsplash.com/photo-1?w=800&q=80"
<img src="https://x.org/a.png"> https://x.org/page"#;
        assert_eq!(
            urls_in(t),
            vec![
                "https://images.unsplash.com/photo-1?w=800&q=80".to_string(),
                "https://x.org/a.png".to_string()
            ]
        );
        let n = file_name(
            "https://images.unsplash.com/photo-123-abc?fm=jpg&w=1600&q=80",
            "jpg",
        );
        assert!(
            n.starts_with("photo-123-abc-1600-") && n.ends_with(".jpg"),
            "{n}"
        );
    }
}
