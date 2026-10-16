//! `revel-render` : génération du site statique à partir du modèle.
//!
//! - Les expressions sont évaluées par [minijinja](https://docs.rs/minijinja) (syntaxe Jinja).
//! - Les variables suivent la cascade : données globales < gabarits < collection < page.
//! - `{% set %}` met à jour la variable existante la plus proche (sinon la crée localement),
//!   comme Nunjucks quand une liste est complétée dans une boucle.
//! - Les images sont confiées à un [`AssetHooks`] (variantes générées par `revel-assets`).

use chrono::{DateTime, Utc};
use minijinja::Value;
use revel_edit::canvas::{Binding, Seg, Slot, Source};
use revel_model::{CollectionSource, Node, Site, SortKind};
use std::collections::BTreeMap;
use std::sync::Arc;

pub mod animations;
mod eval;
mod forms;
mod html;
pub mod rebase;
mod typography;

pub use html::escape;

/// Entrée de collection lue sur disque (fichier Markdown avec front matter).
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Dossier d'origine relatif au projet (ex. `content/projets`).
    pub folder: String,
    /// Nom du fichier sans extension.
    pub file_stem: String,
    /// Front matter (objet JSON).
    pub data: serde_json::Value,
    /// Corps Markdown.
    pub body: String,
}

/// Paramètres d'image évalués.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageArgs {
    pub src: String,
    pub alt: String,
    pub sizes: String,
    pub class: String,
    pub ratio: String,
    pub loading: String,
    pub widths: Vec<u32>,
}

/// Accès aux médias pendant la génération.
pub trait AssetHooks: Send + Sync {
    /// HTML de l'image, ou `None` si le fichier n'existe pas (le composant
    /// `image-placeholder` est alors utilisé s'il existe).
    fn image(&self, args: &ImageArgs) -> Option<String>;
    fn media_exists(&self, src: &str) -> bool;
    /// URL absolue d'une image de partage (réseaux sociaux).
    fn share_image(&self, src: &str, base: &str) -> String;
}

/// Médias sans traitement : `<img>` simple, fichiers supposés présents.
#[derive(Debug, Default)]
pub struct PlainAssets;

impl AssetHooks for PlainAssets {
    fn image(&self, a: &ImageArgs) -> Option<String> {
        if a.src.is_empty() {
            return None;
        }
        let mut s = format!("<img src=\"{}\" alt=\"{}\"", escape(&a.src), escape(&a.alt));
        if !a.class.is_empty() {
            s.push_str(&format!(" class=\"{}\"", escape(&a.class)));
        }
        s.push_str(&format!(
            " loading=\"{}\" decoding=\"async\">",
            if a.loading.is_empty() {
                "lazy"
            } else {
                &a.loading
            }
        ));
        Some(s)
    }

    fn media_exists(&self, _src: &str) -> bool {
        true
    }

    fn share_image(&self, src: &str, base: &str) -> String {
        eval::absolute_url(src, base)
    }
}

/// Données et contenus d'une autre langue du site (3.8), lus dans les dossiers de la langue.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LocaleInput {
    pub code: String,
    /// Fichiers du dossier de données de la langue (`_data/en/`), par nom (comme `Site::data`).
    pub data: BTreeMap<String, serde_json::Value>,
    /// Contenus des dossiers de la langue ; `folder` : dossier du fichier (`content/en/projets`).
    pub entries: Vec<Entry>,
}

pub struct BuildInput<'a> {
    pub site: &'a Site,
    pub entries: Vec<Entry>,
    /// Données et contenus des autres langues (vide sans autre langue).
    pub locales: Vec<LocaleInput>,
    /// Nom de l'environnement (`production`, `staging`…), clé de `settings.environments`.
    pub environment: String,
    /// Identifiant de version (paramètre de cache des CSS/JS).
    pub version: String,
    pub hooks: Arc<dyn AssetHooks>,
    /// Date de génération (fonction `now()`).
    pub now: DateTime<Utc>,
}

/// Fichier produit, chemin relatif à la racine publiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct BuildOutput {
    pub files: Vec<OutputFile>,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("chemin de page invalide : {0}")]
    InvalidPath(String),
    #[error("{context} : expression `{expr}` : {msg}")]
    Expr {
        context: String,
        expr: String,
        msg: String,
    },
    #[error("gabarit introuvable : {0}")]
    MissingLayout(String),
    #[error("{0}")]
    Markup(String),
}

/// Type de document produit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    Html,
    Xml,
    Text,
}

impl DocKind {
    pub fn of_path(path: &str) -> Self {
        let last = path.rsplit('/').next().unwrap_or("");
        if path.ends_with('/') || last.ends_with(".html") || last.ends_with(".htm") {
            Self::Html
        } else if [".xml", ".rss", ".atom", ".svg"]
            .iter()
            .any(|e| last.ends_with(e))
        {
            Self::Xml
        } else {
            Self::Text
        }
    }
}

/// `/agence/` -> `agence/index.html` ; `/` -> `index.html` ; `/sitemap.xml` -> `sitemap.xml`.
pub fn output_path(path: &str) -> Result<String, RenderError> {
    if !path.starts_with('/') || path.split('/').any(|s| s == "..") {
        return Err(RenderError::InvalidPath(path.into()));
    }
    let trimmed = &path[1..];
    Ok(if trimmed.is_empty() || trimmed.ends_with('/') {
        format!("{trimmed}index.html")
    } else {
        trimmed.to_string()
    })
}

/// Entrée de collection en cours de constitution : données, valeur, page éventuelle, nom de fichier.
type CollectedItem<'s> = (BTreeMap<String, Value>, Value, Option<Job<'s>>, String);

/// Page à produire (page fixe ou entrée de collection).
struct Job<'s> {
    url: String,
    layout: Option<String>,
    data: BTreeMap<String, Value>,
    /// Origine des variables de `data` (éditeur, D-19).
    origins: BTreeMap<String, Binding>,
    body: Option<&'s [Node]>,
    /// HTML du corps Markdown (entrées de collection).
    content: Option<String>,
    file_slug: String,
    /// Page fixe : son identifiant (propriétaire des nœuds de `body`).
    page_id: Option<String>,
    /// Contenu de collection : identifiant de la collection.
    collection: Option<String>,
}

/// Collections, pages à produire et données globales, prêtes pour le rendu.
struct Prepared<'s> {
    jobs: Vec<Job<'s>>,
    globals: eval::FrameRef,
    /// Éléments de collection : `fileSlug` -> (dossier, fichier, adresse).
    entries: std::collections::HashMap<String, Vec<(String, String, Value)>>,
}

/// Options du rendu pour l'éditeur.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditorOptions {
    /// Exécuter les scripts du site dans le canevas (sinon `type="text/plain"`).
    pub scripts: bool,
}

/// Page rendue pour l'éditeur : HTML avec repères (`data-rv`, `<!--rv:N-->`) et liste des repères.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorPage {
    pub html: String,
    pub slots: Vec<Slot>,
    /// Valeurs des affectations `{% set %}` de la page, de ses gabarits et de ses composants.
    pub vars: Vec<revel_edit::canvas::VarValue>,
    pub warnings: Vec<String>,
}

/// Génère tout le site : langue principale, puis chaque autre langue publiée (adresses
/// préfixées, 3.8).
pub fn build(input: &BuildInput) -> Result<BuildOutput, RenderError> {
    let site = input.site;
    let all = languages(site, false);
    let mut warnings = Vec::new();
    let mut files = Vec::new();
    let mut pages: Vec<(String, bool)> = Vec::new();
    // Formulaires reçus par l'hébergement (5.8) trouvés dans les pages ; pages animées (6.1).
    let mut forms = 0usize;
    let mut animated = false;
    {
        let mut b = eval::Builder::new(input);
        b.lang = eval::Lang {
            current: None,
            all: all.clone(),
            ..Default::default()
        };
        build_pages(
            &b,
            input,
            &mut files,
            &mut pages,
            &mut forms,
            &mut animated,
            &mut warnings,
        )?;
    }
    for l in site.locales.iter().filter(|l| !l.draft) {
        let (lsite, entries, primary_data) = localize(input, l);
        let linput = BuildInput {
            site: &lsite,
            entries,
            locales: Vec::new(),
            environment: input.environment.clone(),
            version: input.version.clone(),
            hooks: input.hooks.clone(),
            now: input.now,
        };
        let mut b = eval::Builder::new(&linput);
        b.lang = eval::Lang {
            current: Some(l.code.clone()),
            all: all.clone(),
            primary_data,
        };
        build_pages(
            &b,
            &linput,
            &mut files,
            &mut pages,
            &mut forms,
            &mut animated,
            &mut warnings,
        )?;
    }
    if all.len() > 1 && site.pages.iter().any(|p| p.path == "/sitemap.xml") {
        warnings.push(
            "plan du site propre au site : les pages des autres langues n'y sont pas (les liens hreflang des pages les signalent aux moteurs de recherche)"
                .into(),
        );
    }
    if all.len() > 1 && site.base_url.is_none() {
        warnings.push(
            "liens entre les langues (hreflang) non produits : adresse du site absente (paramètres du site)"
                .into(),
        );
    }
    seo_files(site, &pages, &mut files, &mut warnings);
    forms::handler_file(site, forms, &mut files, &mut warnings);
    animations::script_file(animated, &mut files);
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(BuildOutput { files, warnings })
}

/// Pages d'une langue : fichiers produits, adresses HTML (adresse, `noindex`) pour le plan du site.
fn build_pages(
    b: &eval::Builder,
    input: &BuildInput,
    files: &mut Vec<OutputFile>,
    pages: &mut Vec<(String, bool)>,
    forms: &mut usize,
    animated: &mut bool,
    warnings: &mut Vec<String>,
) -> Result<(), RenderError> {
    let prepared = prepare(b, input, false)?;
    for j in &prepared.jobs {
        let kind = DocKind::of_path(&j.url);
        let html = b.render_job(&prepared.globals, j, kind, warnings)?;
        let html = if kind == DocKind::Html {
            pages.push((j.url.clone(), truthy(&j.data, "noindex")));
            let scripts = b.editor.as_ref().is_none_or(|e| e.scripts);
            let (html, n) = forms::wire(input.site, finish_html(b, input.site, j, html), scripts);
            *forms += n;
            let (html, a) = animations::wire(html, scripts);
            *animated |= a;
            html
        } else {
            html
        };
        files.push(OutputFile {
            path: output_path(&j.url)?,
            bytes: html.into_bytes(),
        });
    }
    warnings.extend(b.take_warnings());
    Ok(())
}

/// Code ajouté (3.9) : celui du site puis celui de la page, avant `</head>` et `</body>`. Pas
/// dans le canevas de l'éditeur (scripts arrêtés), seulement dans l'aperçu et le site publié.
fn custom_code(b: &eval::Builder, site: &Site, j: &Job, html: String) -> String {
    if b.editor.as_ref().is_some_and(|e| !e.scripts) {
        return html;
    }
    let page = j
        .page_id
        .as_deref()
        .and_then(|id| site.pages.iter().find(|p| p.id == id))
        .map(|p| &p.code);
    let join = |f: fn(&revel_model::CustomCode) -> &str| -> String {
        [Some(&site.settings.code), page]
            .into_iter()
            .flatten()
            .map(f)
            .filter(|c| !c.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let head = join(|c| &c.head);
    let body = join(|c| &c.body);
    let mut html = html;
    if !head.is_empty() {
        html = insert_before(&html, "</head>", &head, false);
    }
    if !body.is_empty() {
        html = insert_before(&html, "</body>", &body, true);
    }
    html
}

/// Insère `code` avant la dernière balise fermante `tag` (casse ignorée) ; sans elle : au début
/// (`at_end` faux) ou à la fin du document.
pub(crate) fn insert_before(html: &str, tag: &str, code: &str, at_end: bool) -> String {
    let lower = html.to_ascii_lowercase();
    match lower.rfind(tag) {
        Some(i) => format!("{}{code}\n{}", &html[..i], &html[i..]),
        None if at_end => format!("{html}\n{code}"),
        None => format!("{code}\n{html}"),
    }
}

/// Dernières retouches d'une page HTML : `noindex`, typographie, langues, code ajouté.
fn finish_html(b: &eval::Builder, site: &Site, j: &Job, html: String) -> String {
    let html = noindex_meta(j, html);
    let html = stylesheet_link(site, html);
    // Typographie française : pour la langue principale et les autres langues françaises.
    let html = if site.settings.typography.as_deref() == Some("fr")
        && b.lang
            .current
            .as_deref()
            .is_none_or(|c| c.starts_with("fr"))
    {
        typography::french(&html)
    } else {
        html
    };
    // Code ajouté en dernier : gardé tel qu'écrit (ni typographie ni liens de langue).
    let html = localize_html(b, site, &j.url, html);
    custom_code(b, site, j, html)
}

/// Page qui existe dans chaque langue : page HTML, sauf la page d'erreur 404.
pub fn localizable(url: &str) -> bool {
    revel_model::localizable_path(url)
}

/// Langues des pages : (code, nom, préfixe), la principale d'abord ; `drafts` : langues en
/// préparation comprises (éditeur, aperçu).
fn languages(site: &Site, drafts: bool) -> Vec<(String, String, String)> {
    if site.locales.is_empty() {
        return Vec::new();
    }
    let mut v = vec![(
        site.default_locale.clone(),
        revel_model::language_name(&site.default_locale),
        String::new(),
    )];
    v.extend(
        site.locales
            .iter()
            .filter(|l| drafts || !l.draft)
            .map(|l| (l.code.clone(), l.label(), format!("/{}", l.code))),
    );
    v
}

/// Fusion d'une valeur de la langue sur celle de la langue principale (objets fusionnés clé par
/// clé ; le reste remplacé).
fn deep_merge(base: &mut serde_json::Value, over: &serde_json::Value) {
    match (base, over) {
        (serde_json::Value::Object(b), serde_json::Value::Object(o)) => {
            for (k, v) in o {
                match b.get_mut(k) {
                    Some(x) => deep_merge(x, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (b, o) => *b = o.clone(),
    }
}

/// Site et contenus vus dans une autre langue : textes traduits, données de la langue fusionnées
/// sur celles de la langue principale, contenus de la langue (ou ceux de la langue principale,
/// placés dans le dossier de la langue où ils seront écrits une fois traduits).
fn localize(input: &BuildInput, l: &revel_model::Locale) -> (Site, Vec<Entry>, PrimaryData) {
    let site = input.site;
    let mut s = revel_edit::locale::translate_site(site, &l.texts);
    let li = input.locales.iter().find(|x| x.code == l.code);
    if let Some(li) = li {
        for (k, v) in &li.data {
            match s.data.get_mut(k) {
                Some(b) => deep_merge(b, v),
                None => {
                    s.data.insert(k.clone(), v.clone());
                }
            }
        }
    }
    let mut folders: Vec<&str> = site
        .collections
        .iter()
        .filter(|c| !c.folder.is_empty())
        .map(|c| c.folder.as_str())
        .collect();
    folders.sort();
    folders.dedup();
    let mut entries = Vec::new();
    let mut primary = PrimaryData::new();
    for f in folders {
        let lf = revel_model::locale_folder(f, &l.code);
        let own: Vec<&Entry> = li
            .map(|li| li.entries.iter().filter(|e| e.folder == lf).collect())
            .unwrap_or_default();
        for e in input.entries.iter().filter(|e| e.folder == f) {
            primary.insert((lf.clone(), e.file_stem.clone()), e.data.clone());
            let mut x = Entry {
                folder: lf.clone(),
                ..e.clone()
            };
            if let Some(o) = own.iter().find(|o| o.file_stem == e.file_stem) {
                deep_merge(&mut x.data, &o.data);
                if !o.body.trim().is_empty() {
                    x.body = o.body.clone();
                }
            }
            entries.push(x);
        }
        for o in own {
            if !input
                .entries
                .iter()
                .any(|e| e.folder == f && e.file_stem == o.file_stem)
            {
                entries.push(o.clone());
            }
        }
    }
    for c in &mut s.collections {
        if !c.folder.is_empty() {
            c.folder = revel_model::locale_folder(&c.folder, &l.code);
        }
    }
    (s, entries, primary)
}

/// En-têtes de la langue principale des contenus d'une autre langue.
type PrimaryData = std::collections::HashMap<(String, String), serde_json::Value>;

/// Page d'une autre langue : `<html lang>` de la langue, liens vers les pages du site préfixés ;
/// toute page d'un site à plusieurs langues : liens `hreflang` vers ses versions.
fn localize_html(b: &eval::Builder, site: &Site, url: &str, html: String) -> String {
    if b.lang.all.len() < 2 {
        return html;
    }
    let mut html = html;
    if let Some(code) = &b.lang.current {
        html = set_html_lang(&html, code);
        html = prefix_links(&html, &format!("/{code}"), site);
    }
    if localizable(url)
        && let Some(base) = site.base_url.as_deref().map(|b| b.trim_end_matches('/'))
    {
        let primary = b.lang.primary_url(url);
        let mut links = String::new();
        for (code, _, prefix) in &b.lang.all {
            links.push_str(&format!(
                "<link rel=\"alternate\" hreflang=\"{}\" href=\"{}\">",
                html::escape(code),
                html::escape(&format!("{base}{prefix}{primary}"))
            ));
        }
        links.push_str(&format!(
            "<link rel=\"alternate\" hreflang=\"x-default\" href=\"{}\">",
            html::escape(&format!("{base}{primary}"))
        ));
        let lower = html.to_ascii_lowercase();
        if let Some(i) = lower.find("</head")
            && !lower[..i].contains("hreflang=")
        {
            html = format!("{}{links}{}", &html[..i], &html[i..]);
        }
    }
    html
}

/// `<html lang="…">` réglé sur la langue de la page.
fn set_html_lang(html: &str, code: &str) -> String {
    let re = regex::Regex::new(r#"(?i)<html\b[^>]*>"#).expect("regex");
    let Some(m) = re.find(html) else {
        return html.to_string();
    };
    let tag = m.as_str();
    let lang = regex::Regex::new(r#"(?i)(\slang\s*=\s*)("[^"]*"|'[^']*'|[^\s>]+)"#).expect("regex");
    let new = if lang.is_match(tag) {
        lang.replace(tag, |c: &regex::Captures| format!("{}\"{code}\"", &c[1]))
            .into_owned()
    } else {
        format!("{} lang=\"{code}\"{}", &tag[..5], &tag[5..])
    };
    format!("{}{new}{}", &html[..m.start()], &html[m.end()..])
}

/// Liens `<a href="/…">` vers une page du site (adresse sans extension ou `.html`, hors fichiers
/// copiés et autres langues) préfixés par la langue ; un lien qui porte `hreflang` (sélecteur de
/// langue) est gardé.
fn prefix_links(html: &str, prefix: &str, site: &Site) -> String {
    let tag = regex::Regex::new(r#"(?i)<a\b[^>]*>"#).expect("regex");
    let href = regex::Regex::new(r#"(\shref\s*=\s*)"(/[^"]*)""#).expect("regex");
    let copied: Vec<String> = site
        .passthrough
        .iter()
        .map(|p| format!("/{}", p.to.trim_matches('/')))
        .filter(|p| p.len() > 1)
        .collect();
    let locales: Vec<String> = site
        .locales
        .iter()
        .map(|l| format!("/{}", l.code))
        .collect();
    let page_link = |u: &str| {
        let path = u.split(['?', '#']).next().unwrap_or(u);
        if path.starts_with("//") {
            return false;
        }
        let under = |p: &String| path == p || path.starts_with(&format!("{p}/"));
        if locales.iter().any(under) || copied.iter().any(under) {
            return false;
        }
        let last = path.rsplit('/').next().unwrap_or("");
        !last.contains('.') || last.ends_with(".html") || last.ends_with(".htm")
    };
    tag.replace_all(html, |c: &regex::Captures| {
        let t = &c[0];
        if t.to_ascii_lowercase().contains("hreflang") {
            return t.to_string();
        }
        href.replace(t, |h: &regex::Captures| {
            if page_link(&h[2]) {
                format!("{}\"{prefix}{}\"", &h[1], &h[2])
            } else {
                h[0].to_string()
            }
        })
        .into_owned()
    })
    .into_owned()
}

/// Variable de page vraie (`noindex: true`, `"true"`, `1`).
fn truthy(data: &BTreeMap<String, Value>, key: &str) -> bool {
    data.get(key).is_some_and(|v| v.is_true())
}

/// Feuille de style du site reliée à la page si son gabarit (ou la page elle-même) ne la relie
/// pas : un site créé sans gabarit, ou dont la feuille a été créée au premier style, l'applique.
fn stylesheet_link(site: &Site, html: String) -> String {
    let Some(sheet) = &site.stylesheet else {
        return html;
    };
    let lower = html.to_ascii_lowercase();
    if !lower.contains("</head") || html.contains(&format!("href=\"{}", sheet.url)) {
        return html;
    }
    let link = format!(
        "<link rel=\"stylesheet\" href=\"{}\">",
        html::escape(&sheet.url)
    );
    insert_before(&html, "</head>", &link, false)
}

/// Page `noindex` : balise `<meta name="robots" content="noindex">` ajoutée dans `<head>` si le
/// gabarit n'en écrit pas déjà une.
fn noindex_meta(j: &Job, html: String) -> String {
    if !truthy(&j.data, "noindex") {
        return html;
    }
    let lower = html.to_ascii_lowercase();
    if lower.contains("name=\"robots\"") || lower.contains("name=robots") {
        return html;
    }
    match lower.find("</head") {
        Some(i) => format!(
            "{}<meta name=\"robots\" content=\"noindex\">{}",
            &html[..i],
            &html[i..]
        ),
        None => html,
    }
}

/// Plan du site (`sitemap.xml`) et `robots.txt` produits quand aucune page du site n'a déjà ces
/// adresses (réglages de référencement du site).
fn seo_files(
    site: &Site,
    pages: &[(String, bool)],
    files: &mut Vec<OutputFile>,
    warnings: &mut Vec<String>,
) {
    let has_sitemap = files.iter().any(|f| f.path == "sitemap.xml");
    let has_robots = files.iter().any(|f| f.path == "robots.txt");
    let base = site
        .base_url
        .as_deref()
        .map(|b| b.trim_end_matches('/').to_string());
    let mut sitemap = has_sitemap;
    if !sitemap && !site.settings.seo.no_sitemap {
        match &base {
            Some(base) => {
                let mut xml = String::from(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
                );
                let mut urls: Vec<&str> = pages
                    .iter()
                    .filter(|(_, noindex)| !noindex)
                    .filter(|(u, _)| !u.ends_with("404.html") && !u.ends_with("/404/"))
                    .map(|(u, _)| u.as_str())
                    .collect();
                urls.sort();
                urls.dedup();
                for u in urls {
                    xml.push_str(&format!(
                        "  <url><loc>{}{}</loc></url>\n",
                        html::escape(base),
                        html::escape(u)
                    ));
                }
                xml.push_str("</urlset>\n");
                files.push(OutputFile {
                    path: "sitemap.xml".into(),
                    bytes: xml.into_bytes(),
                });
                sitemap = true;
            }
            None => warnings.push(
                "plan du site non produit : adresse du site absente (paramètres du site)".into(),
            ),
        }
    }
    if !has_robots {
        let mut text = site
            .settings
            .seo
            .robots
            .clone()
            .unwrap_or_else(|| "User-agent: *\nAllow: /\n".into());
        if !text.ends_with('\n') {
            text.push('\n');
        }
        if sitemap
            && let Some(base) = &base
            && !text.to_ascii_lowercase().contains("sitemap:")
        {
            text.push_str(&format!("Sitemap: {base}/sitemap.xml\n"));
        }
        files.push(OutputFile {
            path: "robots.txt".into(),
            bytes: text.into_bytes(),
        });
    }
}

/// Rend une page HTML pour l'éditeur (étape 2.2) : même HTML que la génération, avec les repères
/// des nœuds. `url` : adresse de la page (`/agence/`). `None` si aucune page n'a cette adresse.
pub fn render_editor(
    input: &BuildInput,
    url: &str,
    opts: &EditorOptions,
) -> Result<Option<EditorPage>, RenderError> {
    let site = input.site;
    let wanted = normalize_url(url);
    let all = languages(site, true);
    // Page d'une autre langue (`/en/agence/`) : site vu dans cette langue.
    if let Some((l, _)) = site.locale_of_url(&wanted) {
        let (lsite, entries, primary_data) = localize(input, l);
        let linput = BuildInput {
            site: &lsite,
            entries,
            locales: Vec::new(),
            environment: input.environment.clone(),
            version: input.version.clone(),
            hooks: input.hooks.clone(),
            now: input.now,
        };
        let mut b = eval::Builder::new(&linput);
        b.lang = eval::Lang {
            current: Some(l.code.clone()),
            all,
            primary_data,
        };
        return editor_page(b, &linput, &wanted, opts);
    }
    let mut b = eval::Builder::new(input);
    b.lang = eval::Lang {
        current: None,
        all,
        ..Default::default()
    };
    editor_page(b, input, &wanted, opts)
}

fn editor_page(
    mut b: eval::Builder,
    input: &BuildInput,
    wanted: &str,
    opts: &EditorOptions,
) -> Result<Option<EditorPage>, RenderError> {
    let prepared = prepare(&b, input, true)?;
    let Some(job) = prepared
        .jobs
        .iter()
        .find(|j| normalize_url(&j.url) == wanted && DocKind::of_path(&j.url) == DocKind::Html)
    else {
        return Ok(None);
    };
    b.editor = Some(eval::Editor {
        slots: Default::default(),
        vars: Default::default(),
        scripts: opts.scripts,
        entries: prepared.entries.clone(),
    });
    let mut warnings = Vec::new();
    let html = b.render_job(&prepared.globals, job, DocKind::Html, &mut warnings)?;
    let html = finish_html(&b, input.site, job, html);
    warnings.extend(b.take_warnings());
    let (slots, vars) = b
        .editor
        .take()
        .map(|e| (e.slots.into_inner(), e.vars.into_inner()))
        .unwrap_or_default();
    Ok(Some(EditorPage {
        html,
        slots,
        vars,
        warnings,
    }))
}

/// Syntaxe d'une expression (`a | f(b)`, `x if c else y`…). Les variables et filtres inconnus
/// ne sont pas des erreurs ici : ils sont résolus au rendu.
pub fn check_expression(expr: &str) -> Result<(), String> {
    minijinja::Environment::new()
        .compile_expression(expr)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Page HTML du site, pour la liste des pages de l'éditeur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRef {
    pub url: String,
    /// Titre (variable `title` ou `seo.title` de la page, sinon vide).
    pub title: String,
    /// Page fixe du site : son identifiant ; sinon contenu de collection.
    pub page: Option<String>,
    /// Contenu de collection : identifiant de la collection.
    pub collection: Option<String>,
    pub layout: Option<String>,
    /// Contenu de collection : nom de son fichier, sans extension (`wattr`).
    pub file: Option<String>,
    /// Page en brouillon (non publiée).
    pub draft: bool,
    /// Nom de la page dans Revel.
    pub name: Option<String>,
    /// Page fixe : référencement (titre, description, image, indexation) et sa source.
    pub seo: Option<PageSeo>,
}

/// Référencement d'une page fixe : valeurs de `seo` (`title`, `description`, `image`) et
/// `noindex`, avec l'endroit où `seo` s'écrit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSeo {
    pub title: String,
    pub description: String,
    pub image: String,
    pub noindex: bool,
    pub source: SeoSource,
}

/// Endroit où s'écrivent les valeurs de `seo` d'une page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeoSource {
    /// Variable de la page (créée au besoin).
    Page,
    /// Fichier de données : chemin des clés de l'objet `seo` (`pages`, `agence`, `seo`).
    Data(Vec<String>),
    /// Expression calculée (pas un simple chemin) : non modifiable ici.
    Computed,
}

/// Pages HTML du site (pages fixes puis contenus de collection), dans l'ordre de génération.
pub fn list_pages(input: &BuildInput) -> Result<Vec<PageRef>, RenderError> {
    let b = eval::Builder::new(input);
    let prepared = prepare(&b, input, true)?;
    let page_of = |j: &Job| {
        j.page_id
            .as_ref()
            .and_then(|id| input.site.pages.iter().find(|p| &p.id == id))
    };
    let title = |d: &BTreeMap<String, Value>| -> String {
        let text = |v: Option<Value>| v.filter(|v| v.as_str().is_some()).map(|v| v.to_string());
        text(d.get("title").cloned())
            .or_else(|| text(d.get("seo").and_then(|x| x.get_attr("title").ok())))
            .unwrap_or_default()
    };
    Ok(prepared
        .jobs
        .iter()
        .filter(|j| DocKind::of_path(&j.url) == DocKind::Html)
        .map(|j| PageRef {
            url: j.url.clone(),
            title: title(&j.data),
            page: j.page_id.clone(),
            collection: j.collection.clone(),
            layout: j.layout.clone(),
            file: j.collection.as_ref().map(|_| j.file_slug.clone()),
            draft: page_of(j).is_some_and(|p| p.draft),
            name: page_of(j).and_then(|p| p.name.clone()),
            seo: page_of(j).map(|p| page_seo(p, &j.data)),
        })
        .collect())
}

/// Référencement d'une page fixe.
fn page_seo(p: &revel_model::Page, data: &BTreeMap<String, Value>) -> PageSeo {
    let seo = data.get("seo");
    let text = |k: &str| {
        seo.and_then(|s| s.get_attr(k).ok())
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default()
    };
    let source = if p.props.contains_key("seo") {
        SeoSource::Page
    } else if let Some(e) = p.computed.get("seo") {
        match revel_edit::canvas::path_expr(e) {
            Some((head, segs)) if segs.iter().all(|s| matches!(s, Seg::Key(_))) => {
                let mut keys = vec![head];
                keys.extend(segs.into_iter().filter_map(|s| match s {
                    Seg::Key(k) => Some(k),
                    Seg::Index(_) => None,
                }));
                SeoSource::Data(keys)
            }
            _ => SeoSource::Computed,
        }
    } else {
        SeoSource::Page
    };
    PageSeo {
        title: text("title"),
        description: text("description"),
        image: text("image"),
        noindex: truthy(data, "noindex"),
        source,
    }
}

/// HTML du canevas sans les repères de l'éditeur (`data-rv="…"`, `<!--rv:N-->`, `<!--/rv:N-->`) :
/// identique à la page générée quand les scripts du site sont gardés (vérification).
pub fn strip_editor_marks(html: &str) -> String {
    let re = regex::Regex::new(r#"<!--/?rv:\d+-->| data-rv="[0-9a-z]+""#).expect("regex");
    re.replace_all(html, "").into_owned()
}

/// `/agence` et `/agence/index.html` -> `/agence/`.
fn normalize_url(u: &str) -> String {
    let u = u.split(['?', '#']).next().unwrap_or(u);
    let u = u.strip_suffix("index.html").unwrap_or(u);
    if u.ends_with('/') || u.rsplit('/').next().is_some_and(|l| l.contains('.')) {
        u.to_string()
    } else {
        format!("{u}/")
    }
}

/// Collections et pages à produire.
/// Gabarit d'un contenu : `layout` de son en-tête (design propre au contenu), sinon celui de la
/// collection. Vide : celui de la collection ; introuvable : celui de la collection, signalé.
fn entry_layout(
    b: &eval::Builder,
    site: &Site,
    col: &revel_model::Collection,
    data: &BTreeMap<String, Value>,
    folder: &str,
    file: &str,
) -> Option<String> {
    let own = data
        .get("layout")
        .and_then(|v| v.as_str().map(|s| s.trim().to_string()))
        .filter(|l| !l.is_empty());
    match own {
        Some(l) if site.layout(&l).is_some() => Some(l),
        Some(l) => {
            b.warn(format!(
                "contenu {folder}/{file} : gabarit « {l} » introuvable, gabarit de la collection utilisé"
            ));
            col.layout.clone()
        }
        None => col.layout.clone(),
    }
}

/// `drafts` : pages en brouillon comprises (éditeur, aperçu), sinon écartées (génération).
fn prepare<'s>(
    b: &eval::Builder<'s>,
    input: &'s BuildInput,
    drafts: bool,
) -> Result<Prepared<'s>, RenderError> {
    let site = input.site;
    let mut entries_index: std::collections::HashMap<String, Vec<(String, String, Value)>> =
        std::collections::HashMap::new();
    // 1. Collections issues de dossiers.
    let mut collections: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut jobs: Vec<Job> = Vec::new();
    let base_globals = b.base_globals();
    for col in site
        .collections
        .iter()
        .filter(|c| c.source == CollectionSource::Folder)
    {
        let mut items: Vec<CollectedItem> = Vec::new();
        for e in input.entries.iter().filter(|e| e.folder == col.folder) {
            let mut data: BTreeMap<String, Value> = col
                .props
                .iter()
                .map(|(k, v)| (k.clone(), crate::eval::from_json(v)))
                .collect();
            if let serde_json::Value::Object(m) = &e.data {
                for (k, v) in m {
                    data.insert(k.clone(), crate::eval::from_json(v));
                }
            }
            let scope = eval::Scope::new(base_globals.clone(), data.clone());
            if let Some(f) = &col.filter
                && !b
                    .eval(&scope, f, &format!("filtre de {}", col.id))?
                    .is_true()
            {
                continue;
            }
            let url = match &col.path {
                Some(p) => {
                    // Autre langue : adresse calculée avec l'en-tête de la langue principale.
                    let primary = b
                        .lang
                        .primary_data
                        .get(&(e.folder.clone(), e.file_stem.clone()));
                    let pscope = match primary {
                        Some(serde_json::Value::Object(m)) => {
                            let mut d: BTreeMap<String, Value> = col
                                .props
                                .iter()
                                .map(|(k, v)| (k.clone(), crate::eval::from_json(v)))
                                .collect();
                            for (k, v) in m {
                                d.insert(k.clone(), crate::eval::from_json(v));
                            }
                            eval::Scope::new(base_globals.clone(), d)
                        }
                        _ => scope.clone(),
                    };
                    Some(b.render_inline(&pscope, p, &format!("chemin de {}", col.id))?)
                }
                None => None,
            };
            // Autre langue : adresse préfixée (`/en/projets/wattr/`).
            let url = url.map(|u| localized_url(b, u));
            let content = markdown(&e.body);
            let date = data.get("date").cloned().unwrap_or(Value::UNDEFINED);
            let mut item_map: BTreeMap<String, Value> = BTreeMap::new();
            item_map.insert(
                "url".into(),
                url.clone().map(Value::from).unwrap_or(Value::from(false)),
            );
            item_map.insert("data".into(), Value::from(data.clone()));
            item_map.insert("content".into(), Value::from_safe_string(content.clone()));
            item_map.insert("fileSlug".into(), Value::from(e.file_stem.clone()));
            item_map.insert("date".into(), date);
            entries_index.entry(e.file_stem.clone()).or_default().push((
                e.folder.clone(),
                e.file_stem.clone(),
                url.clone().map(Value::from).unwrap_or(Value::from(false)),
            ));
            // Variables de l'en-tête du contenu : reliées au fichier (D-19).
            let origins: BTreeMap<String, Binding> = match &e.data {
                serde_json::Value::Object(m) => m
                    .keys()
                    .map(|k| {
                        (
                            k.clone(),
                            Binding {
                                source: Source::Entry {
                                    folder: e.folder.clone(),
                                    file: e.file_stem.clone(),
                                },
                                path: vec![Seg::Key(k.clone())],
                            },
                        )
                    })
                    .collect(),
                _ => BTreeMap::new(),
            };
            let job = url.map(|url| Job {
                url,
                layout: entry_layout(b, site, col, &data, &e.folder, &e.file_stem),
                data: data.clone(),
                origins,
                body: None,
                content: Some(content),
                file_slug: e.file_stem.clone(),
                page_id: None,
                collection: Some(col.id.clone()),
            });
            items.push((data, Value::from(item_map), job, e.file_stem.clone()));
        }
        // Ordre de base (comme Eleventy) : date croissante, puis nom de fichier ;
        // le tri de la collection s'applique ensuite (tri stable).
        items.sort_by(|a, b| {
            let da =
                a.0.get("date")
                    .and_then(|v| eval::parse_date(&v.to_string()));
            let db =
                b.0.get("date")
                    .and_then(|v| eval::parse_date(&v.to_string()));
            match (da, db) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
            .then_with(|| a.3.cmp(&b.3))
        });
        if let Some(sort) = &col.sort {
            items.sort_by(|a, b| {
                let ord = compare(a.0.get(&sort.field), b.0.get(&sort.field), sort.kind);
                if sort.descending { ord.reverse() } else { ord }
            });
        }
        let mut list = Vec::with_capacity(items.len());
        for (_, item, job, _) in items {
            list.push(item);
            if let Some(j) = job {
                jobs.push(j);
            }
        }
        collections.insert(col.id.clone(), list);
    }

    // 2. Pages fixes.
    let partial_globals = b.globals(collections.clone());
    for p in site.pages.iter().filter(|p| drafts || !p.draft) {
        // Autre langue : pages HTML seulement (plan du site, flux, 404 : langue principale).
        if b.lang.current.is_some() && !localizable(&p.path) {
            continue;
        }
        let mut data: BTreeMap<String, Value> = BTreeMap::new();
        let mut origins: BTreeMap<String, Binding> = BTreeMap::new();
        let key = |source: Source, k: &str| Binding {
            source,
            path: vec![Seg::Key(k.to_string())],
        };
        for l in b.layout_chain(p.layout.as_deref())?.iter().rev() {
            for (k, v) in &l.props {
                data.insert(k.clone(), crate::eval::from_json(v));
                origins.insert(
                    k.clone(),
                    key(
                        Source::Layout {
                            layout: l.id.clone(),
                        },
                        k,
                    ),
                );
            }
        }
        for (k, v) in &p.props {
            data.insert(k.clone(), crate::eval::from_json(v));
            origins.insert(k.clone(), key(Source::Page { page: p.id.clone() }, k));
        }
        for (k, e) in &p.computed {
            let scope = eval::Scope::new(partial_globals.clone(), data.clone());
            let v = b.eval(&scope, e, &format!("variable calculée {k} de {}", p.path))?;
            data.insert(k.clone(), v);
            origins.remove(k);
        }
        jobs.push(Job {
            url: localized_url(b, p.path.clone()),
            layout: p.layout.clone(),
            data,
            origins,
            body: Some(&p.body),
            content: None,
            file_slug: p.id.rsplit('/').next().unwrap_or(&p.id).to_string(),
            page_id: Some(p.id.clone()),
            collection: None,
        });
    }

    // 3. Collections de pages (plan du site…).
    for col in site
        .collections
        .iter()
        .filter(|c| c.source == CollectionSource::Pages)
    {
        let mut list = Vec::new();
        let mut ordered: Vec<&Job> = jobs.iter().collect();
        ordered.sort_by(|a, b| a.url.cmp(&b.url));
        for j in ordered {
            let mut m: BTreeMap<String, Value> = BTreeMap::new();
            m.insert("url".into(), Value::from(j.url.clone()));
            m.insert("data".into(), Value::from(j.data.clone()));
            m.insert(
                "date".into(),
                j.data.get("date").cloned().unwrap_or(Value::UNDEFINED),
            );
            let item = Value::from(m.clone());
            if let Some(f) = &col.filter {
                let scope = eval::Scope::new(base_globals.clone(), m);
                if !b
                    .eval(&scope, f, &format!("filtre de {}", col.id))?
                    .is_true()
                {
                    continue;
                }
            }
            list.push(item);
        }
        collections.insert(col.id.clone(), list);
    }

    let globals = b.globals(collections);
    Ok(Prepared {
        jobs,
        globals,
        entries: entries_index,
    })
}

/// Adresse d'une page dans la langue rendue (`/agence/` -> `/en/agence/`).
fn localized_url(b: &eval::Builder, url: String) -> String {
    let prefix = b.lang.prefix();
    if prefix.is_empty() || !url.starts_with('/') || url.starts_with(&format!("{prefix}/")) {
        url
    } else {
        format!("{prefix}{url}")
    }
}

fn compare(a: Option<&Value>, b: Option<&Value>, kind: SortKind) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => match kind {
            SortKind::Number => {
                let x = f64::try_from(a.clone()).unwrap_or(0.0);
                let y = f64::try_from(b.clone()).unwrap_or(0.0);
                x.partial_cmp(&y).unwrap_or(Ordering::Equal)
            }
            SortKind::Date => {
                let x = eval::parse_date(&a.to_string());
                let y = eval::parse_date(&b.to_string());
                x.cmp(&y)
            }
            SortKind::Text => a.to_string().cmp(&b.to_string()),
        },
    }
}

/// Markdown -> HTML (CommonMark, tableaux, texte barré ; HTML brut autorisé).
pub fn markdown(src: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};
    if src.trim().is_empty() {
        return String::new();
    }
    let mut out = String::new();
    html::push_html(
        &mut out,
        Parser::new_ext(src, Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH),
    );
    out
}

#[cfg(test)]
mod tests;
