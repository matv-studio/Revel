//! Modèles de site (D-128) : sites prêts à adapter, par métier ou par besoin (vitrine, blog,
//! portfolio, boutique, restaurant…), gardés sous une forme écrite dans le dossier `templates/` du
//! dépôt de Revel (texte seul), lus en ligne et gardés sur l'ordinateur depuis D-194 (plus rien
//! n'est compilé dans le programme), et construits à la création du site dans la forme de l'interface
//! (composants à propriétés et à emplacement, collections du CMS pour les pages entières,
//! informations du site, couleurs et mode sombre, polices).
//!
//! Un modèle est un dossier `templates/<id>/` :
//! - `template.json` : nom, catégorie, description, aperçu, couleurs, polices, réglages,
//!   informations du site (menus, coordonnées), composants (propriétés, variantes), gabarits,
//!   collections, pages ;
//! - `site.css` : styles du modèle (après ceux des composants communs utilisés) ;
//! - `layouts/<id>.njk`, `components/<id>.njk`, `pages/<id>.njk` : contenus en balisage
//!   (voir [`markup`]) ;
//! - `files/…` : fichiers écrits tels quels dans le site (contenus du CMS, `.pages.yml`).
//!
//! `templates/_kit/` : composants communs (en-tête, pied de page, boutons, cartes…) et leurs
//! styles (`components/<id>.css`), base des styles (`base.css`) et gabarit de base, repris par
//! les modèles qui les nomment (`"kit": […]`).
//!
//! Dans tous les fichiers : `[[texte français||English text]]` selon la langue du site,
//! `@@rôle:largeur@@` : adresse d'une photo du modèle (`photos.json`, photos libres
//! d'Unsplash liées, copiables dans le site depuis la médiathèque), `@@alt:rôle@@` : sa
//! description.

pub mod markup;

use revel_model::{Collection, Component, Layout, Node, Page, Passthrough, PropDef, Site, Variant};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Fichiers des modèles en mémoire : (chemin dans `templates/`, texte). Ils ne sont plus compilés
/// dans le programme (D-194) : ils viennent du dossier `templates/` du dépôt de Revel, lu en ligne
/// et gardé sur l'ordinateur, ou d'un dossier local (essais, développement).
type Files = &'static [(&'static str, &'static str)];
static STORE: std::sync::RwLock<Option<Files>> = std::sync::RwLock::new(None);

/// Variable d'environnement : dossier local des modèles, lu à la place du catalogue en ligne.
pub const DIR_ENV: &str = "REVEL_TEMPLATES_DIR";
/// Index du dossier : `{ "format": 1, "revision": "…", "files": ["agency/template.json", …] }`.
pub const INDEX: &str = "index.json";

/// Index d'un dossier de modèles.
#[derive(Debug, Clone, Default, Deserialize, serde::Serialize, PartialEq)]
pub struct Index {
    #[serde(default)]
    pub format: u32,
    /// Empreinte des chemins et des contenus : change dès qu'un fichier change.
    #[serde(default)]
    pub revision: String,
    #[serde(default)]
    pub files: Vec<String>,
}

/// Remplace les modèles en mémoire. Les textes vivent jusqu'à la fin du programme (un jeu par
/// chargement : au lancement, puis seulement quand le catalogue a changé).
pub fn set_files(mut files: Vec<(String, String)>) {
    files.sort();
    let leaked: Vec<(&'static str, &'static str)> = files
        .into_iter()
        .map(|(p, t)| {
            (
                &*Box::leak(p.into_boxed_str()),
                &*Box::leak(t.into_boxed_str()),
            )
        })
        .collect();
    *STORE.write().unwrap() = Some(Box::leak(leaked.into_boxed_slice()));
}

/// Fichiers texte d'un dossier de modèles (chemins relatifs, séparés par `/`), sans son index.
pub fn read_dir(dir: &std::path::Path) -> Vec<(String, String)> {
    fn walk(d: &std::path::Path, base: &std::path::Path, out: &mut Vec<(String, String)>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for p in rd.flatten().map(|e| e.path()) {
            if p.is_dir() {
                walk(&p, base, out);
            } else if let (Ok(rel), Ok(text)) = (p.strip_prefix(base), std::fs::read_to_string(&p))
            {
                let rel = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("/");
                if rel != INDEX {
                    out.push((rel, text));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// Empreinte d'un jeu de fichiers (chemins et contenus, fins de ligne normalisées).
pub fn revision(files: &[(String, String)]) -> String {
    let mut h = blake3::Hasher::new();
    for (p, t) in files {
        h.update(p.as_bytes());
        h.update(&[0]);
        h.update(t.replace("\r\n", "\n").as_bytes());
        h.update(&[0]);
    }
    h.finalize().to_hex()[..16].to_string()
}

/// Index d'un dossier de modèles tel qu'il doit être écrit dans son `index.json`.
pub fn index_of(dir: &std::path::Path) -> Index {
    let files = read_dir(dir);
    Index {
        format: 1,
        revision: revision(&files),
        files: files.into_iter().map(|(p, _)| p).collect(),
    }
}

/// Écrit `index.json` dans un dossier de modèles (à refaire après chaque changement : un essai
/// le contrôle). Rend le nombre de fichiers listés.
pub fn write_index(dir: &std::path::Path) -> std::io::Result<usize> {
    let index = index_of(dir);
    let text = serde_json::to_string_pretty(&index).map_err(std::io::Error::other)?;
    std::fs::write(dir.join(INDEX), text + "\n")?;
    Ok(index.files.len())
}

/// Charge un dossier de modèles ; faux s'il n'en contient pas.
pub fn load_dir(dir: &std::path::Path) -> bool {
    let files = read_dir(dir);
    if !files.iter().any(|(p, _)| p.ends_with("/template.json")) {
        return false;
    }
    set_files(files);
    true
}

/// Dossier local des modèles : `REVEL_TEMPLATES_DIR`, sinon le dossier `templates/` des sources
/// quand le programme tourne à côté d'elles (développement, essais, intégration continue).
pub fn local_dir() -> Option<std::path::PathBuf> {
    if let Some(d) = std::env::var_os(DIR_ENV).filter(|d| !d.is_empty()) {
        return Some(d.into());
    }
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates");
    src.join("_kit").is_dir().then_some(src)
}

fn store() -> Files {
    if let Some(f) = *STORE.read().unwrap() {
        return f;
    }
    if !local_dir().is_some_and(|d| load_dir(&d)) {
        let mut w = STORE.write().unwrap();
        if w.is_none() {
            *w = Some(&[]);
        }
    }
    STORE.read().unwrap().unwrap_or(&[])
}

/// Vrai si aucun modèle n'est chargé (catalogue jamais lu, ordinateur hors ligne).
pub fn is_empty() -> bool {
    ids().is_empty()
}

/// Texte d'un fichier des modèles (`restaurant/template.json`).
fn file(path: &str) -> Option<&'static str> {
    store().iter().find(|(p, _)| *p == path).map(|(_, t)| *t)
}

/// Fichiers d'un dossier des modèles (chemins relatifs à ce dossier).
fn files_under(dir: &str) -> Vec<(&'static str, &'static str)> {
    let prefix = format!("{dir}/");
    store()
        .iter()
        .filter_map(|(p, t)| p.strip_prefix(prefix.as_str()).map(|r| (r, *t)))
        .collect()
}

/// Thème des couleurs du nouveau site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// Couleurs claires seulement.
    Light,
    /// Couleurs sombres seulement.
    Dark,
    /// Claires, sombres selon l'appareil du visiteur (`prefers-color-scheme`).
    Auto,
}

impl Scheme {
    pub fn parse(s: &str) -> Self {
        match s {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::Auto,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Def {
    name: String,
    category: String,
    #[serde(default)]
    order: i32,
    #[serde(default)]
    description: String,
    /// Photo de la vignette (rôle dans `photos.json`).
    #[serde(default)]
    preview: Option<String>,
    #[serde(default)]
    palette: Vec<String>,
    #[serde(default)]
    features: Vec<String>,
    #[serde(default)]
    fonts: Vec<FontDef>,
    #[serde(default)]
    settings: Value,
    #[serde(default)]
    site: Value,
    #[serde(default)]
    kit: Vec<String>,
    #[serde(default)]
    components: Vec<CompDef>,
    #[serde(default)]
    layouts: Vec<LayoutDef>,
    #[serde(default)]
    collections: Vec<Value>,
    #[serde(default)]
    pages: Vec<PageDef>,
    #[serde(default)]
    library: Value,
    /// Valeurs de la barre de navigation du gabarit commun (`cta`, `cta_link`, `show_cta`).
    #[serde(default)]
    navbar: BTreeMap<String, Value>,
}

/// Police de Google Fonts copiée dans le site à sa création (si la connexion le permet).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct FontDef {
    pub family: String,
    #[serde(default)]
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CompDef {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    props: Vec<PropDef>,
    #[serde(default)]
    variants: Vec<Variant>,
}

#[derive(Debug, Clone, Deserialize)]
struct LayoutDef {
    id: String,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    fields: Vec<PropDef>,
    #[serde(default)]
    props: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct PageDef {
    id: String,
    path: String,
    name: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    layout: Option<String>,
    #[serde(default)]
    props: BTreeMap<String, Value>,
}

/// Modèle proposé à la création d'un site.
#[derive(Debug, Clone, PartialEq)]
pub struct Info {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub preview: Option<String>,
    pub palette: Vec<String>,
    pub features: Vec<String>,
    pub pages: usize,
    pub order: i32,
}

/// Site construit d'après un modèle : document, fichiers à écrire, polices à copier.
pub struct Built {
    pub site: Site,
    pub files: Vec<(String, String)>,
    pub fonts: Vec<FontDef>,
}

/// Identifiants des modèles, dans l'ordre de leur dossier.
pub fn ids() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (p, _) in store() {
        if let Some(id) = p.strip_suffix("/template.json")
            && !id.contains('/')
            && !id.starts_with('_')
        {
            out.push(id.to_string());
        }
    }
    out
}

/// Choix de la langue : `[[français||English]]`.
fn localize(text: &str, fr: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find("[[") {
        let Some(j) = rest[i..].find("]]") else {
            break;
        };
        let inner = &rest[i + 2..i + j];
        let Some((a, b)) = inner.split_once("||") else {
            out.push_str(&rest[..i + j + 2]);
            rest = &rest[i + j + 2..];
            continue;
        };
        out.push_str(&rest[..i]);
        out.push_str(if fr { a } else { b });
        rest = &rest[i + j + 2..];
    }
    out.push_str(rest);
    out
}

/// Photos des modèles : `photos.json` → modèle → rôle → (adresse, description fr, en).
fn photos() -> BTreeMap<String, BTreeMap<String, (String, String, String)>> {
    #[derive(Deserialize)]
    struct Photo {
        role: String,
        url: String,
        #[serde(default)]
        alt_fr: String,
        #[serde(default)]
        alt_en: String,
    }
    let raw: BTreeMap<String, Vec<Photo>> = file("photos.json")
        .and_then(|t| serde_json::from_str(t).ok())
        .unwrap_or_default();
    raw.into_iter()
        .map(|(k, list)| {
            (
                k,
                list.into_iter()
                    .map(|p| (p.role, (p.url, p.alt_fr, p.alt_en)))
                    .collect(),
            )
        })
        .collect()
}

/// Adresse d'une photo d'Unsplash à une largeur (JPEG, recadrée, qualité 80).
pub fn photo_url(base: &str, width: u32) -> String {
    format!("{base}?auto=format&fm=jpg&fit=crop&w={width}&q=80")
}

/// `@@rôle:largeur@@` et `@@alt:rôle@@` remplacés (photos du modèle `id`, sinon communes).
fn with_photos(text: &str, id: &str, fr: bool, all: &PhotoMap) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find("@@") {
        let Some(j) = rest[i + 2..].find("@@") else {
            break;
        };
        let key = &rest[i + 2..i + 2 + j];
        out.push_str(&rest[..i]);
        // `rôle` : photo du modèle ; `autre/rôle` : photo d'un autre modèle.
        let find = |role: &str| {
            let (owner, role) = role.split_once('/').unwrap_or((id, role));
            all.get(owner)
                .and_then(|m| m.get(role))
                .or_else(|| all.get("_common").and_then(|m| m.get(role)))
                .ok_or_else(|| format!("{id} : photo « {role} » inconnue"))
        };
        if let Some(name) = key.strip_prefix("icon:") {
            // Icône de Revel, comme l'insère l'élément « Icône ».
            let (_, body) = revel_edit::icons::ICONS
                .iter()
                .find(|(n, _)| *n == name)
                .ok_or_else(|| format!("{id} : icône « {name} » inconnue"))?;
            out.push_str(&format!(
                r#"<svg class="icon" data-icon="{name}" viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{body}</svg>"#
            ));
        } else if let Some(role) = key.strip_prefix("alt:") {
            let (_, a_fr, a_en) = find(role)?;
            out.push_str(if fr { a_fr } else { a_en });
        } else {
            let (role, width) = key.split_once(':').unwrap_or((key, "1600"));
            let (url, _, _) = find(role)?;
            out.push_str(&photo_url(url, width.parse().unwrap_or(1600)));
        }
        rest = &rest[i + 2 + j + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

type PhotoMap = BTreeMap<String, BTreeMap<String, (String, String, String)>>;

fn def(id: &str, fr: bool, all: &PhotoMap) -> Result<Def, String> {
    let raw = file(&format!("{id}/template.json")).ok_or(format!("modèle inconnu : {id}"))?;
    let text = with_photos(&localize(raw, fr), id, fr, all)?;
    serde_json::from_str(&text).map_err(|e| format!("{id}/template.json : {e}"))
}

/// Modèles proposés, dans l'ordre de leur catégorie puis de `order`.
pub fn catalogue(lang: &str) -> Vec<Info> {
    let fr = lang.to_ascii_lowercase().starts_with("fr");
    let all = photos();
    let mut out: Vec<Info> = ids()
        .iter()
        .filter_map(|id| {
            let d = def(id, fr, &all).ok()?;
            let preview = d.preview.as_deref().and_then(|role| {
                all.get(id.as_str())
                    .and_then(|m| m.get(role))
                    .map(|(u, _, _)| photo_url(u, 720))
            });
            Some(Info {
                id: id.clone(),
                name: d.name,
                category: d.category,
                description: d.description,
                preview,
                palette: d.palette,
                features: d.features,
                pages: d.pages.len(),
                order: d.order,
            })
        })
        .collect();
    out.sort_by_key(|i| i.order);
    out
}

/// Texte d'un fichier du modèle `id` (ou commun, `_kit/…`), langue et photos résolues.
fn text(path: &str, id: &str, fr: bool, all: &PhotoMap) -> Result<Option<String>, String> {
    match file(path) {
        Some(t) => with_photos(&localize(t, fr), id, fr, all).map(Some),
        None => Ok(None),
    }
}

fn nodes_of(path: &str, src: &str) -> Result<Vec<Node>, String> {
    markup::parse(src).map_err(|e| format!("{path} : {e}"))
}

/// Feuille des couleurs selon le thème choisi : `:root` (clair) et le bloc
/// `@media (prefers-color-scheme: dark)` gardés, l'un ou l'autre seul.
fn apply_scheme(css: &str, scheme: Scheme) -> String {
    const DARK: &str = "@media (prefers-color-scheme: dark)";
    let Some(start) = css.find(DARK) else {
        return css.to_string();
    };
    // Fin du bloc : accolades équilibrées.
    let mut depth = 0;
    let mut end = css.len();
    for (i, c) in css[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    let block = &css[start..end];
    let without = format!("{}{}", &css[..start], &css[end..]);
    match scheme {
        Scheme::Auto => css.to_string(),
        Scheme::Light => without,
        Scheme::Dark => {
            // Valeurs sombres reportées dans le `:root` clair (variable par variable).
            let inner = block
                .find(":root")
                .and_then(|i| block[i..].find('{').map(|j| i + j + 1))
                .and_then(|i| block[i..].find('}').map(|j| &block[i..i + j]))
                .unwrap_or("");
            let mut out = without;
            for decl in inner.split(';') {
                let Some((k, v)) = decl.split_once(':') else {
                    continue;
                };
                let (k, v) = (k.trim(), v.trim());
                if !k.starts_with("--") {
                    if k == "color-scheme" {
                        out = out.replacen("color-scheme: light", &format!("color-scheme: {v}"), 1);
                    }
                    continue;
                }
                let pat = format!("{k}:");
                if let Some(i) = out.find(&pat) {
                    let vs = i + pat.len();
                    if let Some(e) = out[vs..].find([';', '}']) {
                        out = format!("{} {v}{}", &out[..vs], &out[vs + e..]);
                    }
                }
            }
            out
        }
    }
}

/// Construit le site du modèle `id` nommé `name`, en langue `lang`.
pub fn build(id: &str, name: &str, lang: &str, scheme: Scheme) -> Result<Built, String> {
    let fr = lang.to_ascii_lowercase().starts_with("fr");
    let all = photos();
    let d = def(id, fr, &all)?;
    let mut s = Site::new(name, lang);
    // Informations du site : celles du modèle, nom et langue choisis.
    let mut info = match d.site.clone() {
        Value::Object(m) => m,
        _ => Default::default(),
    };
    info.insert("title".into(), json!(name));
    info.insert("lang".into(), json!(lang));
    info.entry("description").or_insert(json!(""));
    s.data.insert("site".into(), Value::Object(info));
    s.passthrough = vec![Passthrough {
        from: "media".into(),
        to: "media".into(),
    }];
    if !d.settings.is_null() {
        let mut base = serde_json::to_value(&s.settings).unwrap_or(json!({}));
        merge(&mut base, &d.settings);
        s.settings = serde_json::from_value(base).map_err(|e| format!("{id} : réglages : {e}"))?;
    }
    if !d.library.is_null() {
        let mut base = serde_json::to_value(&s.library).unwrap_or(json!({}));
        merge(&mut base, &d.library);
        s.library = serde_json::from_value(base).map_err(|e| format!("{id} : rangement : {e}"))?;
    }
    // Styles : base commune, composants communs utilisés, styles du modèle.
    let mut css = text("_kit/base.css", id, fr, &all)?.unwrap_or_default();
    // Composants communs (`_kit/kit.json`) puis ceux du modèle.
    let kit: Vec<CompDef> = text("_kit/kit.json", id, fr, &all)?
        .map(|t| serde_json::from_str(&t).map_err(|e| format!("_kit/kit.json : {e}")))
        .transpose()?
        .unwrap_or_default();
    for k in &d.kit {
        let c = kit
            .iter()
            .find(|c| &c.id == k)
            .ok_or_else(|| format!("{id} : composant commun inconnu « {k} »"))?;
        let path = format!("_kit/components/{k}.njk");
        let body = text(&path, id, fr, &all)?.ok_or(format!("{path} absent"))?;
        s.components.push(component(c, nodes_of(&path, &body)?));
        if let Some(c) = text(&format!("_kit/components/{k}.css"), id, fr, &all)? {
            css.push_str(&c);
        }
    }
    for c in &d.components {
        let path = format!("{id}/components/{}.njk", c.id);
        let body = text(&path, id, fr, &all)?.ok_or(format!("{path} absent"))?;
        s.components.push(component(c, nodes_of(&path, &body)?));
    }
    if let Some(own) = text(&format!("{id}/site.css"), id, fr, &all)? {
        css.push_str(&own);
    }
    let css = apply_scheme(&css, scheme);
    s.stylesheet = Some(revel_model::Stylesheet {
        url: "/assets/site.css".into(),
        source: None,
        items: revel_style::parse(&css).map_err(|e| format!("{id} : styles : {e}"))?,
    });
    // Gabarits : ceux du modèle, sinon le gabarit commun.
    let layouts = if d.layouts.is_empty() {
        vec![LayoutDef {
            id: "base".into(),
            parent: None,
            fields: Vec::new(),
            props: BTreeMap::new(),
        }]
    } else {
        d.layouts.clone()
    };
    for l in &layouts {
        let own = format!("{id}/layouts/{}.njk", l.id);
        let (path, body) = match text(&own, id, fr, &all)? {
            Some(b) => (own, b),
            None => {
                let k = format!("_kit/layouts/{}.njk", l.id);
                let mut b = text(&k, id, fr, &all)?.ok_or(format!("{own} absent"))?;
                if !d.navbar.is_empty() {
                    let mut args = String::new();
                    for (name, v) in &d.navbar {
                        match v {
                            Value::String(t) => args.push_str(&format!(
                                " {name}=\"{}\"",
                                t.replace('&', "&amp;").replace('"', "&quot;")
                            )),
                            other => args.push_str(&format!(" x-{name}=\"{other}\"")),
                        }
                    }
                    b = b.replace(
                        "<x-use c=\"navbar\">",
                        &format!("<x-use c=\"navbar\"{args}>"),
                    );
                }
                (k, b)
            }
        };
        s.layouts.push(Layout {
            id: l.id.clone(),
            parent: l.parent.clone(),
            props: l.props.clone(),
            fields: l.fields.clone(),
            body: nodes_of(&path, &body)?,
        });
    }
    for c in &d.collections {
        let col: Collection =
            serde_json::from_value(c.clone()).map_err(|e| format!("{id} : collection : {e}"))?;
        s.collections.push(col);
    }
    for p in &d.pages {
        let path = format!("{id}/pages/{}.njk", p.id);
        let body = text(&path, id, fr, &all)?.ok_or(format!("{path} absent"))?;
        let mut props = p.props.clone();
        let mut seo = serde_json::Map::new();
        if let Some(t) = &p.title {
            seo.insert("title".into(), json!(t));
        }
        if let Some(t) = &p.description {
            seo.insert("description".into(), json!(t));
        }
        if !seo.is_empty() {
            props.insert("seo".into(), Value::Object(seo));
        }
        s.pages.push(Page {
            id: p.id.clone(),
            path: p.path.clone(),
            layout: Some(p.layout.clone().unwrap_or_else(|| "base".into())),
            props,
            computed: Default::default(),
            body: nodes_of(&path, &body)?,
            draft: false,
            code: Default::default(),
            name: Some(p.name.clone()),
        });
    }
    s.ensure_ids();
    let mut out_files = Vec::new();
    for (rel, _) in files_under(&format!("{id}/files")) {
        let full = format!("{id}/files/{rel}");
        if let Some(t) = text(&full, id, fr, &all)? {
            out_files.push((rel.to_string(), t));
        }
    }
    Ok(Built {
        site: s,
        files: out_files,
        fonts: d.fonts,
    })
}

fn component(c: &CompDef, body: Vec<Node>) -> Component {
    Component {
        id: c.id.clone(),
        name: c.name.clone(),
        props: c.props.clone(),
        variants: c.variants.clone(),
        body,
    }
}

/// Fusion récursive des objets JSON (`over` l'emporte).
fn merge(base: &mut Value, over: &Value) {
    match (base, over) {
        (Value::Object(b), Value::Object(o)) => {
            for (k, v) in o {
                merge(b.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (b, o) => *b = o.clone(),
    }
}

#[cfg(test)]
mod tests;
