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

pub mod actions;
pub mod animations;
mod eval;
pub mod flatten;
mod forms;
mod head;
mod html;
pub mod lottie;
pub mod rebase;
mod redirects;
pub mod transitions;
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
    // `//dossier/` (adresse d'origine mal formée) ou `\` : chemin absolu une fois joint au
    // dossier de sortie, refusé.
    if !path.starts_with('/')
        || path.starts_with("//")
        || path.contains('\\')
        || path.split('/').any(|s| s == "..")
    {
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
    let mut pages: Vec<SitemapPage> = Vec::new();
    // Formulaires reçus par l'hébergement (5.8) trouvés dans les pages ; pages animées (6.1).
    let mut forms = 0usize;
    let mut used = Scripts::default();
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
            &mut used,
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
            &mut used,
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
    feeds(input, &mut files, &mut warnings)?;
    seo_files(site, &pages, &mut files, &mut warnings);
    redirects::redirect_files(site, &mut files, &mut warnings);
    forms::handler_file(site, forms, &mut files, &mut warnings);
    animations::script_file(used.animations, &mut files);
    lottie::script_file(used.lottie, &mut files);
    actions::script_file(used.actions, &mut files);
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(BuildOutput { files, warnings })
}

/// Environnements de publication d'un site (`production` d'abord, puis ceux des paramètres du
/// site) : un site reconstruit doit donner les mêmes fichiers dans chacun.
pub(crate) fn environments_of(site: &Site, first: &str) -> Vec<String> {
    let mut v = vec![first.to_string()];
    for k in site.settings.environments.keys() {
        if !v.contains(k) {
            v.push(k.clone());
        }
    }
    v
}

/// Fichiers produits pour chaque environnement de publication (chemin préfixé par
/// l'environnement hors du premier : `staging:index.html`).
pub(crate) fn build_all_envs(input: &BuildInput, site: &Site) -> Option<Vec<OutputFile>> {
    let mut all = Vec::new();
    for (i, env) in environments_of(input.site, &input.environment)
        .into_iter()
        .enumerate()
    {
        let inp = BuildInput {
            site,
            entries: input.entries.clone(),
            locales: input.locales.clone(),
            environment: env.clone(),
            version: input.version.clone(),
            hooks: input.hooks.clone(),
            now: input.now,
        };
        let out = build(&inp).ok()?;
        for mut f in out.files {
            if i > 0 {
                f.path = format!("{env}:{}", f.path);
            }
            all.push(f);
        }
    }
    Some(all)
}

/// Flux RSS des collections (paramètres du site) : contenus publiés dans l'ordre de la
/// collection ; résumé tiré du premier champ rempli parmi `description`, `excerpt`, `summary`,
/// `chapeau`, `intro`, sans balises, 300 caractères au plus.
pub(crate) fn feeds(
    input: &BuildInput,
    files: &mut Vec<OutputFile>,
    warnings: &mut Vec<String>,
) -> Result<(), RenderError> {
    let site = input.site;
    if site.settings.feeds.is_empty() {
        return Ok(());
    }
    let b = eval::Builder::new(input);
    let prepared = prepare(&b, input, false)?;
    let cols = prepared
        .globals
        .get("collections")
        .cloned()
        .unwrap_or(Value::UNDEFINED);
    let site_data = |k: &str| {
        site.data
            .get("site")
            .and_then(|s| s.get(k))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };
    let base = site
        .base_url
        .clone()
        .or_else(|| site_data("url"))
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_string();
    if base.is_empty() {
        warnings.push("flux RSS non produit : adresse du site absente (paramètres du site)".into());
        return Ok(());
    }
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    let strip = regex::Regex::new(r"<[^>]*>").expect("regex");
    let last_word = regex::Regex::new(r"\s+\S*$").expect("regex");
    for f in &site.settings.feeds {
        let Ok(items) = cols.get_attr(&f.collection) else {
            continue;
        };
        let name = f
            .title
            .clone()
            .or_else(|| site_data("title"))
            .unwrap_or_else(|| site.name.clone());
        let link = format!("{base}{}", f.link.as_deref().unwrap_or("/"));
        let mut out = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\">\n  <channel>\n    <title>{}</title>\n    <link>{}</link>\n    <atom:link href=\"{}{}\" rel=\"self\" type=\"application/rss+xml\"/>\n",
            esc(&name),
            esc(&link),
            esc(&base),
            esc(&f.path)
        );
        if let Some(d) = f
            .description
            .clone()
            .or_else(|| site_data("description"))
            .filter(|d| !d.trim().is_empty())
        {
            out.push_str(&format!("    <description>{}</description>\n", esc(&d)));
        }
        out.push_str(&format!(
            "    <language>{}</language>\n",
            esc(&og_locale(&site.default_locale).replace('_', "-"))
        ));
        // Les plus récents d'abord (usage des flux) quand chaque contenu a une date.
        let mut list: Vec<(Option<DateTime<Utc>>, Value)> = items
            .try_iter()
            .into_iter()
            .flatten()
            .map(|it| {
                let d = it
                    .get_attr("date")
                    .ok()
                    .filter(|d| !d.is_undefined() && !d.is_none())
                    .and_then(|d| eval::parse_date(&d.to_string()));
                (d, it)
            })
            .collect();
        if list.iter().all(|(d, _)| d.is_some()) {
            list.sort_by_key(|a| std::cmp::Reverse(a.0));
        }
        for it in list
            .into_iter()
            .map(|(_, it)| it)
            .take(f.limit.unwrap_or(20))
        {
            let data = it.get_attr("data").unwrap_or(Value::UNDEFINED);
            let text = |k: &str| {
                data.get_attr(k)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .filter(|s| !s.trim().is_empty())
            };
            let url = format!(
                "{base}{}",
                it.get_attr("url")
                    .ok()
                    .and_then(|u| u.as_str().map(str::to_string))
                    .unwrap_or_default()
            );
            out.push_str("    <item>\n");
            out.push_str(&format!(
                "      <title>{}</title>\n",
                esc(&text("title").unwrap_or_default())
            ));
            out.push_str(&format!(
                "      <link>{}</link>\n      <guid>{}</guid>\n",
                esc(&url),
                esc(&url)
            ));
            let date = it
                .get_attr("date")
                .ok()
                .and_then(|d| eval::parse_date(&d.to_string()));
            if let Some(d) = date {
                out.push_str(&format!(
                    "      <pubDate>{}</pubDate>\n",
                    d.format("%a, %d %b %Y %H:%M:%S GMT")
                ));
            }
            let summary = ["description", "excerpt", "summary", "chapeau", "intro"]
                .iter()
                .find_map(|k| text(k))
                .map(|s| {
                    // Texte sans balises (blancs réduits), 300 caractères au plus, coupé entre deux
                    // mots avec « … ».
                    let t = strip.replace_all(&s, " ");
                    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
                    let chars: Vec<char> = t.chars().collect();
                    if chars.len() > 300 {
                        let cut: String = chars[..299].iter().collect();
                        format!("{}…", last_word.replace(&cut, ""))
                    } else {
                        t
                    }
                });
            if let Some(sm) = summary {
                out.push_str(&format!("      <description>{}</description>\n", esc(&sm)));
            }
            out.push_str("    </item>\n");
        }
        out.push_str("  </channel>\n</rss>\n");
        files.push(OutputFile {
            path: f.path.trim_start_matches('/').to_string(),
            bytes: out.into_bytes(),
        });
    }
    Ok(())
}

/// Scripts du site utilisés par au moins une page (produits une fois pour le site).
#[derive(Default)]
struct Scripts {
    animations: bool,
    lottie: bool,
    actions: bool,
}

/// Page HTML pour le plan du site : adresse, `noindex`, date de publication (ISO 8601).
type SitemapPage = (String, bool, Option<String>);

/// Pages d'une langue : fichiers produits, adresses HTML (adresse, `noindex`, date) pour le plan
/// du site.
fn build_pages(
    b: &eval::Builder,
    input: &BuildInput,
    files: &mut Vec<OutputFile>,
    pages: &mut Vec<SitemapPage>,
    forms: &mut usize,
    used: &mut Scripts,
    warnings: &mut Vec<String>,
) -> Result<(), RenderError> {
    let prepared = prepare(b, input, false)?;
    for j in &prepared.jobs {
        let kind = DocKind::of_path(&j.url);
        let html = b.render_job(&prepared.globals, j, kind, warnings)?;
        let html = if kind == DocKind::Html {
            let date = j
                .data
                .get("date")
                .filter(|v| !v.is_undefined() && !v.is_none())
                .and_then(|v| eval::parse_date(&v.to_string()))
                .map(|d| d.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string());
            pages.push((j.url.clone(), truthy(&j.data, "noindex"), date));
            let scripts = b.editor.as_ref().is_none_or(|e| e.scripts);
            let (html, n) = forms::wire(input.site, finish_html(b, input.site, j, html), scripts);
            *forms += n;
            let (html, a) = animations::wire(html, scripts);
            used.animations |= a;
            let (html, l) = lottie::wire(html);
            used.lottie |= l;
            let (html, i) = actions::wire(html, scripts);
            used.actions |= i;
            transitions::wire(
                html,
                input.site.settings.page_transition.as_deref(),
                scripts,
            )
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

/// Valeur (`content`) de la balise `<meta>` de nom ou propriété `key` dans `head` (en
/// minuscules).
fn meta_content(head: &str, key: &str) -> Option<String> {
    let tag = regex::Regex::new(r"<meta\s[^>]*>").expect("regex");
    let content = regex::Regex::new(r#"\scontent\s*=\s*"([^"]*)""#).expect("regex");
    tag.find_iter(head)
        .map(|m| m.as_str())
        .find(|t| {
            t.contains(&format!("property=\"{key}\"")) || t.contains(&format!("name=\"{key}\""))
        })
        .and_then(|t| content.captures(t))
        .map(|c| c[1].trim().to_string())
}

/// Code ajouté à l'en-tête (site, puis page), en minuscules : ce qu'il contient n'est pas écrit
/// une seconde fois d'après les réglages.
fn added_head_code(site: &Site, j: &Job) -> String {
    let page = j
        .page_id
        .as_deref()
        .and_then(|id| site.pages.iter().find(|p| p.id == id))
        .map(|p| p.code.head.as_str())
        .unwrap_or("");
    format!("{}\n{page}", site.settings.code.head).to_ascii_lowercase()
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
    let html = page_look(site, j, html);
    let html = seo_meta(site, j, html);
    let html = noindex_meta(site, j, html);
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

/// Texte d'une variable de la page (chaîne non vide).
/// Première classe écrite d'une valeur `class` (avant toute condition).
pub(crate) fn style_base(class: &str) -> Option<String> {
    class
        .split("{%")
        .next()?
        .split_whitespace()
        .next()
        .map(str::to_string)
}

fn data_text(data: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    data.get(key)
        .and_then(|v| v.as_str().map(str::to_string))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Liens vers la page affichée : `aria-current="page"` (état « Page actuelle » des styles), comme
/// la classe `w--current` de Webflow. Avec `data-revel-current="section"`, aussi sur les pages
/// dont l'adresse commence par celle du lien (sauf l'accueil). Le réglage n'est pas publié.
fn mark_current(html: &str, url: &str) -> String {
    if !html.contains("<a ") {
        return html.to_string();
    }
    let tag_re = regex::Regex::new(r"(?s)<a\s[^>]*>").expect("regex");
    let href_re = regex::Regex::new(r#"\shref="([^"]*)""#).expect("regex");
    let section_re = regex::Regex::new(r#"\s+data-revel-current="[^"]*""#).expect("regex");
    tag_re
        .replace_all(html, |c: &regex::Captures| {
            let tag = &c[0];
            let section = tag.contains(r#"data-revel-current="section""#);
            let mut out = section_re.replace_all(tag, "").into_owned();
            if out.contains(" aria-current=") {
                return out;
            }
            let Some(h) = href_re.captures(&out) else {
                return out;
            };
            let path = h[1]
                .replace("&amp;", "&")
                .split(['?', '#'])
                .next()
                .unwrap_or_default()
                .to_string();
            if !path.starts_with('/') || path.starts_with("//") {
                return out;
            }
            let hit = path == url || (section && path != "/" && url.starts_with(&path));
            if hit {
                let at = if out.ends_with("/>") {
                    out.len() - 2
                } else {
                    out.len() - 1
                };
                out.insert_str(at, r#" aria-current="page""#);
            }
            out
        })
        .into_owned()
}

/// Apparence de la page réglée dans ses paramètres : classe du corps de page (ajoutée aux
/// classes de `<body>`) et thème (`data-theme` de `<html>`) ; sinon ceux de son gabarit (puis du
/// gabarit parent).
/// Réglage de la page (`bodyClass`, `theme`, `ogType`) : celui de la page, sinon de son gabarit
/// (puis du gabarit parent).
fn page_setting(site: &Site, j: &Job, key: &str) -> Option<String> {
    data_text(&j.data, key).or_else(|| {
        let mut at = j.layout.clone();
        let mut seen = 0;
        while let Some(id) = at.take() {
            let l = site.layouts.iter().find(|l| l.id == id)?;
            if let Some(v) = l
                .props
                .get(key)
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
            {
                return Some(v);
            }
            seen += 1;
            if seen > 16 {
                break;
            }
            at = l.parent.clone();
        }
        None
    })
}

fn page_look(site: &Site, j: &Job, html: String) -> String {
    let mut html = mark_current(&html, &j.url);
    let look = |key: &str| page_setting(site, j, key);
    if let Some(cls) = look("bodyClass") {
        html = add_attr_class(&html, "body", &cls);
    }
    if let Some(theme) = look("theme") {
        html = set_attr(&html, "html", "data-theme", &theme);
    }
    html
}

/// Première balise `<tag …>` : position de son début et de sa fin (`>`).
fn open_tag(html: &str, tag: &str) -> Option<(usize, usize)> {
    let lower = html.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(&format!("<{tag}")) {
        let at = from + i;
        let next = lower.as_bytes().get(at + tag.len() + 1).copied();
        if matches!(next, Some(b' ' | b'>' | b'\n' | b'\t' | b'\r')) {
            let end = at + html[at..].find('>')?;
            return Some((at, end));
        }
        from = at + 1;
    }
    None
}

/// Classes ajoutées à la première balise `tag` (sans doublon).
fn add_attr_class(html: &str, tag: &str, cls: &str) -> String {
    let Some((start, end)) = open_tag(html, tag) else {
        return html.to_string();
    };
    let head = &html[start..end];
    let re = regex::Regex::new(r#"\sclass="([^"]*)""#).expect("regex");
    let new_head = match re.captures(head) {
        Some(c) => {
            let mut list: Vec<String> = c[1].split_whitespace().map(str::to_string).collect();
            for x in cls.split_whitespace() {
                if !list.iter().any(|y| y == x) {
                    list.push(x.to_string());
                }
            }
            re.replace(head, format!(" class=\"{}\"", list.join(" ")).as_str())
                .into_owned()
        }
        None => format!("{head} class=\"{}\"", escape_attr_value(cls)),
    };
    format!("{}{new_head}{}", &html[..start], &html[end..])
}

/// Attribut de la première balise `tag` remplacé ou ajouté.
fn set_attr(html: &str, tag: &str, name: &str, value: &str) -> String {
    let Some((start, end)) = open_tag(html, tag) else {
        return html.to_string();
    };
    let head = &html[start..end];
    let re = regex::Regex::new(&format!(r#"\s{}="[^"]*""#, regex::escape(name))).expect("regex");
    let attr = format!(" {name}=\"{}\"", escape_attr_value(value));
    let new_head = if re.is_match(head) {
        re.replace(head, attr.as_str()).into_owned()
    } else {
        format!("{head}{attr}")
    };
    format!("{}{new_head}{}", &html[..start], &html[end..])
}

fn escape_attr_value(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

/// Référencement écrit par Revel quand le gabarit ne l'écrit pas (comme Webflow) : description,
/// adresse officielle, aperçu de partage (Open Graph, X) tirés des paramètres de la page et du
/// site.
fn seo_meta(site: &Site, j: &Job, html: String) -> String {
    let lower = html.to_ascii_lowercase();
    let Some(head_end) = lower.find("</head") else {
        return html;
    };
    // Balises déjà là : dans le gabarit ou dans le code ajouté à l'en-tête (site, page).
    let code = added_head_code(site, j);
    let has = |needle: &str| lower[..head_end].contains(needle) || code.contains(needle);
    let seo = j.data.get("seo");
    let seo_text = |k: &str| {
        seo.and_then(|v| v.get_attr(k).ok())
            .and_then(|v| v.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
    };
    let site_text = |k: &str| {
        site.data
            .get("site")
            .and_then(|s| s.get(k))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let cfg = &site.settings.seo;
    let opt = |v: &Option<String>| {
        v.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let site_title = opt(&cfg.site_title);
    let title = seo_text("title").or_else(|| data_text(&j.data, "title"));
    // Titre complet (« Page | Nom du site ») quand le nom affiché du site est réglé.
    let full_title = match (&title, &site_title) {
        (Some(t), Some(st)) if t != st => Some(format!("{t} | {st}")),
        (None, Some(st)) => Some(st.clone()),
        (t, _) => t.clone(),
    };
    let description = seo_text("description")
        .or_else(|| data_text(&j.data, "description"))
        .or_else(|| opt(&cfg.description))
        .or_else(|| site_text("description"));
    let base = site
        .base_url
        .clone()
        .or_else(|| site_text("url"))
        .unwrap_or_default();
    let base = base.trim_end_matches('/');
    let url = (!base.is_empty()).then(|| format!("{base}{}", j.url));
    let image = seo_text("image")
        .or_else(|| data_text(&j.data, "cover"))
        .or_else(|| opt(&cfg.share_image))
        .or_else(|| site_text("share_image"))
        .map(|i| {
            if i.starts_with("http") || base.is_empty() {
                i
            } else {
                format!("{base}/{}", i.trim_start_matches('/'))
            }
        });
    let mut tags = String::new();
    let mut add = |cond: bool, tag: String| {
        if cond {
            tags.push_str(&tag);
        }
    };
    let e = escape_attr_value;
    // Titre de la page : écrit par Revel quand le gabarit n'en a pas.
    if let Some(t) = full_title
        .clone()
        .or_else(|| Some(site.name.clone()))
        .filter(|_| !has("<title"))
    {
        add(
            true,
            format!("<title>{}</title>", crate::html::escape_text(&t)),
        );
    }
    if let Some(d) = &description {
        add(
            !has("name=\"description\""),
            format!("<meta name=\"description\" content=\"{}\">", e(d)),
        );
        add(
            !has("og:description"),
            format!("<meta property=\"og:description\" content=\"{}\">", e(d)),
        );
        add(
            !has("twitter:description"),
            format!("<meta name=\"twitter:description\" content=\"{}\">", e(d)),
        );
    }
    if let Some(t) = &full_title {
        add(
            !has("og:title"),
            format!("<meta property=\"og:title\" content=\"{}\">", e(t)),
        );
        add(
            !has("twitter:title"),
            format!("<meta name=\"twitter:title\" content=\"{}\">", e(t)),
        );
    }
    if let Some(st) = &site_title {
        add(
            !has("og:site_name"),
            format!("<meta property=\"og:site_name\" content=\"{}\">", e(st)),
        );
    }
    add(
        !has("og:locale"),
        format!(
            "<meta property=\"og:locale\" content=\"{}\">",
            e(&og_locale(&j_locale(site, j)))
        ),
    );
    if let Some(u) = &url {
        add(
            !has("rel=\"canonical\""),
            format!("<link rel=\"canonical\" href=\"{}\">", e(u)),
        );
        add(
            !has("og:url"),
            format!("<meta property=\"og:url\" content=\"{}\">", e(u)),
        );
    }
    if let Some(i) = &image {
        add(
            !has("og:image"),
            format!("<meta property=\"og:image\" content=\"{}\">", e(i)),
        );
        add(
            !has("twitter:card"),
            "<meta name=\"twitter:card\" content=\"summary_large_image\">".to_string(),
        );
        add(
            !has("twitter:image"),
            format!("<meta name=\"twitter:image\" content=\"{}\">", e(i)),
        );
    }
    // Type déjà annoncé par la page : celui qui compte (date d'article seulement pour un
    // article).
    let written =
        meta_content(&lower[..head_end], "og:type").or_else(|| meta_content(&code, "og:type"));
    let og_type = written
        .or_else(|| page_setting(site, j, "ogType"))
        .unwrap_or_else(|| {
            if j.collection.is_some() {
                "article"
            } else {
                "website"
            }
            .to_string()
        });
    add(
        !has("og:type"),
        format!(
            "<meta property=\"og:type\" content=\"{}\">",
            escape_attr_value(&og_type)
        ),
    );
    // Article : date de publication.
    let published = j
        .data
        .get("date")
        .map(|d| d.to_string())
        .and_then(|d| eval::parse_date(&d))
        .map(|d| d.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string());
    if og_type == "article"
        && let Some(p) = &published
    {
        add(
            !has("article:published_time"),
            format!(
                "<meta property=\"article:published_time\" content=\"{}\">",
                e(p)
            ),
        );
    }
    // Aperçus d'image en grand dans les résultats de recherche (page indexée).
    add(
        !has("name=\"robots\"") && !j.data.get("noindex").is_some_and(|v| v.is_true()),
        "<meta name=\"robots\" content=\"max-image-preview:large\">".to_string(),
    );
    // Navigateur : couleur, icônes.
    if let Some(c) = opt(&cfg.theme_color) {
        add(
            !has("theme-color"),
            format!("<meta name=\"theme-color\" content=\"{}\">", e(&c)),
        );
    }
    if !has("rel=\"icon\"") {
        if let Some(i) = opt(&cfg.favicon) {
            let kind = if i.ends_with(".svg") {
                "image/svg+xml"
            } else {
                "image/png"
            };
            add(
                true,
                format!("<link rel=\"icon\" href=\"{}\" type=\"{kind}\">", e(&i)),
            );
        }
        if let Some(i) = opt(&cfg.favicon_png) {
            add(
                true,
                format!(
                    "<link rel=\"icon\" href=\"{}\" type=\"image/png\" sizes=\"192x192\">",
                    e(&i)
                ),
            );
        }
    }
    if let Some(i) = opt(&cfg.touch_icon) {
        add(
            !has("apple-touch-icon"),
            format!("<link rel=\"apple-touch-icon\" href=\"{}\">", e(&i)),
        );
    }
    // Flux RSS du site.
    for f in &site.settings.feeds {
        let t = f
            .title
            .clone()
            .or_else(|| site_title.clone())
            .unwrap_or_else(|| site.name.clone());
        add(
            !has(&format!("href=\"{}\"", f.path.to_ascii_lowercase())),
            format!(
                "<link rel=\"alternate\" type=\"application/rss+xml\" title=\"{}\" href=\"{}\">",
                e(&t),
                e(&f.path)
            ),
        );
    }
    // Données structurées : organisation, article.
    if let Some(o) = &cfg.organization
        && !has("\"@type\":\"organization\"")
    {
        add(
            true,
            format!(
                "<script type=\"application/ld+json\">{}</script>",
                organization_ld(o, base)
            ),
        );
        if og_type == "article"
            && let (Some(t), Some(p), Some(u)) = (
                data_text(&j.data, "title").or(title.clone()),
                &published,
                &url,
            )
        {
            let ld = serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "headline": t,
                "datePublished": p,
                "mainEntityOfPage": u,
                "publisher": { "@id": format!("{base}/#organisation") },
            });
            add(
                true,
                format!(
                    "<script type=\"application/ld+json\">{}</script>",
                    ld.to_string().replace("</", "<\\/")
                ),
            );
        }
    }
    if tags.is_empty() {
        return html;
    }
    format!("{}{tags}{}", &html[..head_end], &html[head_end..])
}

/// Langue d'une page (préfixe de langue de son adresse, sinon celle du site).
fn j_locale(site: &Site, j: &Job) -> String {
    site.locales
        .iter()
        .find(|l| j.url.starts_with(&format!("/{}/", l.code)))
        .map(|l| l.code.clone())
        .unwrap_or_else(|| site.default_locale.clone())
}

/// Langue au format du partage (`fr` -> `fr_FR`).
fn og_locale(code: &str) -> String {
    let c = code.replace('-', "_");
    if c.contains('_') {
        return c;
    }
    let region = match c.as_str() {
        "en" => "US",
        "pt" => "PT",
        "zh" => "CN",
        "ja" => "JP",
        "ko" => "KR",
        "sv" => "SE",
        "da" => "DK",
        "cs" => "CZ",
        "el" => "GR",
        "uk" => "UA",
        "ar" => "SA",
        "he" => "IL",
        "hi" => "IN",
        "vi" => "VN",
        "et" => "EE",
        "ca" => "ES",
        other => return format!("{other}_{}", other.to_ascii_uppercase()),
    };
    format!("{c}_{region}")
}

/// Organisation en données structurées (`schema.org/Organization`), adresses absolues.
fn organization_ld(o: &revel_model::Organization, base: &str) -> String {
    let abs = |u: &str| {
        if u.starts_with("http") || base.is_empty() {
            u.to_string()
        } else {
            format!("{base}/{}", u.trim_start_matches('/'))
        }
    };
    let mut m = serde_json::Map::new();
    m.insert("@context".into(), "https://schema.org".into());
    m.insert("@type".into(), "Organization".into());
    if !base.is_empty() {
        m.insert("@id".into(), format!("{base}/#organisation").into());
    }
    let mut put = |k: &str, v: &str| {
        if !v.trim().is_empty() {
            m.insert(k.into(), v.trim().into());
        }
    };
    put("name", &o.name);
    put("alternateName", &o.alternate_name);
    if !base.is_empty() {
        put("url", &format!("{base}/"));
    }
    if !o.logo.is_empty() {
        put("logo", &abs(&o.logo));
    }
    put("email", &o.email);
    put(
        "telephone",
        &o.telephone
            .replace([' ', '(', ')'], "")
            .replace("+330", "+33"),
    );
    put("foundingDate", &o.founding_date);
    if !(o.street.is_empty() && o.city.is_empty()) {
        let mut a = serde_json::Map::new();
        a.insert("@type".into(), "PostalAddress".into());
        for (k, v) in [
            ("streetAddress", &o.street),
            ("postalCode", &o.postcode),
            ("addressLocality", &o.city),
            ("addressCountry", &o.country),
        ] {
            if !v.trim().is_empty() {
                a.insert(k.into(), v.trim().into());
            }
        }
        m.insert("address".into(), serde_json::Value::Object(a));
    }
    if !o.same_as.is_empty() {
        m.insert("sameAs".into(), o.same_as.clone().into());
    }
    serde_json::Value::Object(m)
        .to_string()
        .replace("</", "<\\/")
}

/// Page `noindex` : balise `<meta name="robots" content="noindex">` ajoutée dans `<head>` si le
/// gabarit n'en écrit pas déjà une.
fn noindex_meta(site: &Site, j: &Job, html: String) -> String {
    if !truthy(&j.data, "noindex") {
        return html;
    }
    let lower = html.to_ascii_lowercase();
    let code = added_head_code(site, j);
    if [&lower, &code]
        .iter()
        .any(|t| t.contains("name=\"robots\"") || t.contains("name=robots"))
    {
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
    pages: &[SitemapPage],
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
                let mut urls: Vec<(&str, Option<&str>)> = pages
                    .iter()
                    .filter(|(_, noindex, _)| !noindex)
                    .filter(|(u, _, _)| !u.ends_with("404.html") && !u.ends_with("/404/"))
                    .map(|(u, _, d)| (u.as_str(), d.as_deref()))
                    .collect();
                urls.sort();
                urls.dedup_by(|a, b| a.0 == b.0);
                for (u, d) in urls {
                    let lastmod = match d {
                        Some(d) if site.settings.seo.sitemap_lastmod => {
                            format!("<lastmod>{}</lastmod>", html::escape(d))
                        }
                        _ => String::new(),
                    };
                    xml.push_str(&format!(
                        "  <url><loc>{}{}</loc>{lastmod}</url>\n",
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
    // Scripts du site : animations Lottie aussi dans le canevas (elles s'y voient) ; animations,
    // interactions et transitions dans l'aperçu seulement (`scripts`).
    let (html, _) = animations::wire(html, opts.scripts);
    let (html, _) = lottie::wire(html);
    let (html, _) = actions::wire(html, opts.scripts);
    let html = transitions::wire(
        html,
        input.site.settings.page_transition.as_deref(),
        opts.scripts,
    );
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

/// Contenu non publié : `draft: true` ou `published: false` dans son en-tête.
fn entry_draft(data: &serde_json::Value) -> bool {
    data.get("draft").and_then(serde_json::Value::as_bool) == Some(true)
        || data.get("published").and_then(serde_json::Value::as_bool) == Some(false)
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
            // Contenu en brouillon (`draft: true`, `published: false`, comme le montre la vue
            // CMS) : écarté du site généré, gardé dans l'éditeur et l'aperçu.
            if !drafts && entry_draft(&e.data) {
                continue;
            }
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
            let defaults = l
                .fields
                .iter()
                .filter_map(|f| f.default.as_ref().map(|d| (&f.name, d)))
                .filter(|(k, _)| !l.props.contains_key(*k));
            for (k, v) in l.props.iter().chain(defaults) {
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
