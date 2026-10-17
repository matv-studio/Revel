//! Import d'un site WordPress (7.6) depuis son fichier d'export (Outils › Exporter, format WXR) :
//!
//! - **articles** : collection `articles` (fichiers Markdown dans `content/articles`, texte en
//!   HTML), adresses d'origine gardées (`permalink`), image mise en avant, catégories,
//!   étiquettes, auteur, brouillons ;
//! - **pages** : pages Revel à leur adresse d'origine, contenu modifiable dans le canevas ;
//! - **menu** : premier menu de navigation du site (sinon les pages), dans `site.menu` ;
//! - **médias** : images utilisées par les articles et les pages téléchargées depuis le site
//!   (`/wp-content/uploads/…`, mêmes adresses), adresses réécrites ;
//! - contenu nettoyé : commentaires de l'éditeur de blocs retirés, paragraphes de l'éditeur
//!   classique, légendes (`[caption]`) en `<figure>`.
//!
//! Le site part du modèle Blog de Revel (gabarits, styles), fourni par l'appelant : le thème
//! WordPress (PHP) n'est pas dans le fichier d'export.

use crate::Report;
use crate::crawl::Fetch;
use crate::eleventy::ImportError;
use regex::Regex;
use revel_model::{Attr, Node, Page, Passthrough, Site};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use url::Url;

/// Élément du fichier d'export (article, page, fichier joint, lien de menu…).
#[derive(Debug, Clone, Default)]
pub struct Item {
    pub id: u64,
    pub title: String,
    pub link: String,
    pub slug: String,
    /// `AAAA-MM-JJ HH:MM:SS`.
    pub date: String,
    /// `publish`, `draft`, `pending`, `private`, `future`, `inherit`, `trash`…
    pub status: String,
    pub post_type: String,
    pub parent: u64,
    pub menu_order: i64,
    pub content: String,
    pub excerpt: String,
    /// Identifiant de connexion de l'auteur.
    pub creator: String,
    pub attachment_url: String,
    /// (taxonomie, slug, nom) : `category`, `post_tag`, `nav_menu`…
    pub terms: Vec<(String, String, String)>,
    pub meta: Vec<(String, String)>,
    pub comments: usize,
}

impl Item {
    fn meta(&self, key: &str) -> Option<&str> {
        self.meta
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    fn is_content(&self) -> bool {
        matches!(
            self.status.as_str(),
            "publish" | "draft" | "pending" | "private" | "future"
        )
    }
}

/// Fichier d'export lu.
#[derive(Debug, Clone, Default)]
pub struct Wxr {
    pub title: String,
    pub description: String,
    /// Adresse du site (`wp:base_blog_url`, sinon `link`).
    pub url: String,
    pub language: String,
    /// Nom affiché des auteurs par identifiant de connexion.
    pub authors: HashMap<String, String>,
    pub items: Vec<Item>,
}

impl Wxr {
    pub fn pages(&self) -> impl Iterator<Item = &Item> {
        self.items
            .iter()
            .filter(|i| i.post_type == "page" && i.is_content())
    }

    pub fn posts(&self) -> impl Iterator<Item = &Item> {
        self.items
            .iter()
            .filter(|i| i.post_type == "post" && i.is_content())
    }

    pub fn attachments(&self) -> impl Iterator<Item = &Item> {
        self.items.iter().filter(|i| i.post_type == "attachment")
    }
}

/// Le texte est un fichier d'export WordPress.
pub fn is_wxr(text: &str) -> bool {
    let head: String = text.chars().take(4000).collect();
    head.contains("<rss") && head.contains("wordpress.org/export/")
}

/// Lit un fichier d'export WordPress.
pub fn parse(text: &str) -> Result<Wxr, String> {
    if !is_wxr(text) {
        return Err("ce fichier n'est pas un export WordPress (WXR)".into());
    }
    let doc = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            nodes_limit: u32::MAX,
            ..Default::default()
        },
    )
    .map_err(|e| format!("fichier d'export illisible : {e}"))?;
    let channel = doc
        .descendants()
        .find(|n| n.has_tag_name("channel"))
        .ok_or("élément <channel> absent")?;
    // Préfixe de l'élément : `wp:post_id` lu par son nom local, quel que soit l'espace de noms
    // (versions 1.0 à 1.2 du format).
    let local = |n: &roxmltree::Node| n.tag_name().name().to_string();
    let text_of = |n: &roxmltree::Node| -> String {
        n.children()
            .filter(|c| c.is_text())
            .filter_map(|c| c.text())
            .collect::<String>()
    };
    let child = |n: &roxmltree::Node, name: &str, ns_hint: Option<&str>| -> String {
        n.children()
            .filter(|c| c.is_element() && c.tag_name().name() == name)
            .find(|c| {
                ns_hint.is_none_or(|h| c.tag_name().namespace().is_some_and(|ns| ns.contains(h)))
            })
            .map(|c| text_of(&c))
            .unwrap_or_default()
    };
    let mut w = Wxr {
        title: child(&channel, "title", None).trim().to_string(),
        description: child(&channel, "description", None).trim().to_string(),
        language: child(&channel, "language", None).trim().to_string(),
        ..Default::default()
    };
    let blog = child(&channel, "base_blog_url", Some("wordpress.org"));
    w.url = if blog.trim().is_empty() {
        child(&channel, "link", None).trim().to_string()
    } else {
        blog.trim().to_string()
    };
    for n in channel.children().filter(|c| c.is_element()) {
        match local(&n).as_str() {
            "author" => {
                let login = child(&n, "author_login", None);
                let name = child(&n, "author_display_name", None);
                if !login.is_empty() {
                    w.authors.insert(
                        login.trim().to_string(),
                        if name.trim().is_empty() {
                            login.trim().to_string()
                        } else {
                            name.trim().to_string()
                        },
                    );
                }
            }
            "item" => {
                let mut it = Item::default();
                for c in n.children().filter(|c| c.is_element()) {
                    let ns = c.tag_name().namespace().unwrap_or("");
                    let v = || text_of(&c);
                    match (c.tag_name().name(), ns) {
                        ("title", _) => it.title = v().trim().to_string(),
                        ("link", _) => it.link = v().trim().to_string(),
                        ("creator", _) => it.creator = v().trim().to_string(),
                        ("encoded", ns) if ns.contains("content") => it.content = v(),
                        ("encoded", ns) if ns.contains("excerpt") => it.excerpt = v(),
                        ("post_id", _) => it.id = v().trim().parse().unwrap_or(0),
                        ("post_date", _) => it.date = v().trim().to_string(),
                        ("post_name", _) => it.slug = v().trim().to_string(),
                        ("status", _) => it.status = v().trim().to_string(),
                        ("post_parent", _) => it.parent = v().trim().parse().unwrap_or(0),
                        ("menu_order", _) => it.menu_order = v().trim().parse().unwrap_or(0),
                        ("post_type", _) => it.post_type = v().trim().to_string(),
                        ("attachment_url", _) => it.attachment_url = v().trim().to_string(),
                        ("category", _) => it.terms.push((
                            c.attribute("domain").unwrap_or("category").to_string(),
                            c.attribute("nicename").unwrap_or("").to_string(),
                            v().trim().to_string(),
                        )),
                        ("postmeta", _) => {
                            let k = child(&c, "meta_key", None);
                            let val = child(&c, "meta_value", None);
                            it.meta.push((k.trim().to_string(), val));
                        }
                        ("comment", _) => it.comments += 1,
                        _ => {}
                    }
                }
                w.items.push(it);
            }
            _ => {}
        }
    }
    Ok(w)
}

/// Réglages de l'import.
#[derive(Default)]
pub struct Options<'a> {
    /// Images téléchargées depuis le site d'origine (sinon adresses d'origine gardées).
    pub media: bool,
    /// Textes de l'interface en français (menu, titres).
    pub fr: bool,
    pub cancel: Option<&'a AtomicBool>,
}

/// Collection des articles décrite pour Pages CMS (et la vue CMS de Revel).
pub fn pages_yml(fr: bool) -> String {
    let l = |a: &'static str, b: &'static str| if fr { a } else { b };
    format!(
        "# Configuration de Pages CMS (https://pagescms.org), lue aussi par Revel.\n\
media:\n  input: media\n  output: /media\n\
content:\n  - name: articles\n    label: Articles\n    type: collection\n    path: content/articles\n    filename: \"{{fields.slug}}.md\"\n    view:\n      fields: [title, date]\n      primary: title\n      default:\n        sort: date\n        order: desc\n    fields:\n\
      - name: title\n        label: {}\n        type: string\n        required: true\n\
      - name: slug\n        label: {}\n        type: string\n        required: true\n\
      - name: date\n        label: Date\n        type: date\n\
      - name: description\n        label: {}\n        type: text\n\
      - name: image\n        label: Image\n        type: image\n\
      - name: categories\n        label: {}\n        type: string\n        list: true\n\
      - name: tags\n        label: {}\n        type: string\n        list: true\n\
      - name: author\n        label: {}\n        type: string\n\
      - name: permalink\n        label: {}\n        type: string\n\
      - name: draft\n        label: {}\n        type: boolean\n\
      - name: body\n        label: {}\n        type: rich-text\n",
        l("Titre", "Title"),
        l("Adresse de la page", "Page address"),
        l("Résumé", "Summary"),
        l("Catégories", "Categories"),
        l("Étiquettes", "Tags"),
        l("Auteur", "Author"),
        l("Adresse d'origine", "Original address"),
        l("Brouillon", "Draft"),
        l("Texte", "Text"),
    )
}

/// Chemin de la collection : adresse d'origine de l'article, sinon `/articles/<slug>/`.
pub const POST_PATH: &str = "{{ permalink if permalink else \"/articles/\" ~ slug ~ \"/\" }}";

/// Complète `site` (modèle Blog : gabarit `base` avec `site.menu`, gabarit `article`, page
/// d'accueil `index`, page `articles`, collection `articles`) avec le contenu de `wxr`. Les
/// fichiers (articles, médias) sont écrits dans `dest`. `progress` : fichiers téléchargés.
pub fn fill(
    site: &mut Site,
    dest: &Path,
    wxr: &Wxr,
    fetch: &dyn Fetch,
    opts: &Options,
    progress: &mut dyn FnMut(usize),
) -> Result<Report, ImportError> {
    let mut report = Report {
        tool: Some("WordPress".into()),
        ..Default::default()
    };
    let l = |a: &'static str, b: &'static str| if opts.fr { a } else { b };
    let base = Url::parse(&wxr.url).ok();
    let hosts: HashSet<String> = base
        .iter()
        .filter_map(|u| u.host_str())
        .flat_map(|h| {
            let bare = h.trim_start_matches("www.").to_string();
            [bare.clone(), format!("www.{bare}")]
        })
        .collect();
    // Préfixe de chemin du site (WordPress installé dans un sous-dossier).
    let prefix = base
        .as_ref()
        .map(|u| u.path().trim_end_matches('/').to_string())
        .unwrap_or_default();
    let local = |link: &str| -> Option<String> {
        let u = Url::parse(link).ok()?;
        if !u.host_str().is_some_and(|h| hosts.contains(h)) {
            return None;
        }
        let p = u.path();
        let p = p.strip_prefix(prefix.as_str()).unwrap_or(p);
        // Adresse sans réécriture (`?page_id=12`) : pas d'adresse lisible.
        if u.query().is_some() && (p.is_empty() || p == "/") {
            return None;
        }
        // Lettres accentuées en clair (nom du dossier produit), barres doublées réduites.
        let decoded = percent_encoding::percent_decode_str(p).decode_utf8_lossy();
        let mut out = String::from("/");
        for seg in decoded
            .split('/')
            .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        {
            out.push_str(seg);
            out.push('/');
        }
        if !p.ends_with('/') && out.len() > 1 {
            out.pop();
        }
        Some(out)
    };

    // Images utilisées : contenus, images mises en avant.
    let attachments: HashMap<u64, &Item> = wxr.attachments().map(|a| (a.id, a)).collect();
    let featured = |it: &Item| -> Option<String> {
        let id: u64 = it.meta("_thumbnail_id")?.trim().parse().ok()?;
        let a = attachments.get(&id)?;
        (!a.attachment_url.is_empty()).then(|| a.attachment_url.clone())
    };
    // Médias : `…/wp-content/uploads/…` du site, ou serveur de médias de WordPress.com
    // (`<site>.files.wordpress.com`).
    let upload_re =
        Regex::new(r#"(?:https?:)?//[A-Za-z0-9.-]+(?::\d+)?/[^\s"'<>(),\[\]]+"#).expect("regex");
    let is_media = |x: &Url| {
        let h = x.host_str().unwrap_or("");
        (hosts.contains(h) && x.path().contains("/wp-content/uploads/"))
            || h.ends_with(".files.wordpress.com")
    };
    let contents: Vec<&Item> = wxr.pages().chain(wxr.posts()).collect();
    let mut wanted: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    for it in &contents {
        let mut found: Vec<String> = upload_re
            .find_iter(&it.content)
            .map(|m| m.as_str().to_string())
            .collect();
        if let Some(f) = featured(it) {
            found.push(f);
        }
        for u in found {
            let u = if u.starts_with("//") {
                format!("https:{u}")
            } else {
                u
            };
            if Url::parse(&u).ok().is_some_and(|x| is_media(&x)) && seen.insert(u.clone()) {
                wanted.push(u);
            }
        }
    }
    // Téléchargement : même chemin dans le site (`wp-content/uploads/…`).
    let mut mapped: HashMap<String, String> = HashMap::new();
    if opts.media && !wanted.is_empty() {
        let jobs: Vec<(Url, String)> = wanted
            .iter()
            .filter_map(|u| {
                let x = Url::parse(u).ok()?;
                let path = percent_encoding::percent_decode_str(x.path())
                    .decode_utf8_lossy()
                    .to_string();
                let mut rel = match path.find("wp-content/uploads/") {
                    Some(i) => path[i..].to_string(),
                    None => format!("wp-content/uploads{path}"),
                };
                if rel.split('/').any(|s| s == "..") || rel.ends_with('/') {
                    return None;
                }
                // Variante d'une même image (`?w=300`) : nom distinct.
                if let Some(q) = x.query() {
                    let h = q.bytes().fold(0xcbf29ce484222325u64, |h, b| {
                        (h ^ b as u64).wrapping_mul(0x100000001b3)
                    });
                    let tag = format!("-q{:08x}", h as u32);
                    match rel.rfind('.').filter(|i| *i > rel.rfind('/').unwrap_or(0)) {
                        Some(i) => rel.insert_str(i, &tag),
                        None => rel.push_str(&tag),
                    }
                }
                Some((x, rel))
            })
            .collect();
        let done = crate::crawl::download(fetch, &jobs, dest, 6, opts.cancel, &mut |n| progress(n));
        if opts.cancel.is_some_and(|c| c.load(Ordering::SeqCst)) {
            return Err(ImportError::File {
                path: dest.display().to_string(),
                msg: "import annulé".into(),
            });
        }
        let mut failed = 0;
        for ((u, rel), ok) in jobs.iter().zip(&done) {
            if *ok {
                mapped.insert(u.to_string(), format!("/{rel}"));
            } else {
                failed += 1;
            }
        }
        // Adresses d'origine (sans normalisation de `Url`) reliées aux fichiers locaux.
        for w in &wanted {
            if let Ok(x) = Url::parse(w)
                && let Some(p) = mapped.get(x.as_str()).cloned()
            {
                mapped.insert(w.clone(), p);
            }
        }
        report.files = done.iter().filter(|d| **d).count();
        if failed > 0 {
            report.warnings.push(format!(
                "{failed} image(s) non téléchargée(s) : adresses du site d'origine gardées"
            ));
        }
        if report.files > 0 {
            site.passthrough.push(Passthrough {
                from: "wp-content".into(),
                to: "wp-content".into(),
            });
        }
    } else if !wanted.is_empty() {
        report.warnings.push(format!(
            "{} image(s) gardée(s) sur le site d'origine (non téléchargées)",
            wanted.len()
        ));
    }
    let clean = |html: &str, report: &mut Report| -> String {
        let html = clean_content(html, report);
        rewrite_links(&html, &mapped, &hosts, &prefix)
    };

    // Articles.
    let folder = dest.join("content").join("articles");
    std::fs::create_dir_all(&folder).map_err(|e| ImportError::File {
        path: folder.display().to_string(),
        msg: e.to_string(),
    })?;
    let mut slugs = HashSet::new();
    let mut comments = 0;
    for it in wxr.posts() {
        comments += it.comments;
        let mut slug = if it.slug.is_empty() {
            revel_model::slugify(&it.title)
        } else {
            revel_model::slugify(
                &percent_encoding::percent_decode_str(&it.slug).decode_utf8_lossy(),
            )
        };
        if slug.is_empty() {
            slug = format!("article-{}", it.id);
        }
        while !slugs.insert(slug.clone()) {
            slug.push_str("-2");
        }
        let body = clean(&it.content, &mut report);
        let mut data = Map::new();
        data.insert("title".into(), Value::String(it.title.clone()));
        data.insert("slug".into(), Value::String(slug.clone()));
        if let Some(d) = it
            .date
            .get(..10)
            .filter(|d| d.len() == 10 && !d.starts_with("0000"))
        {
            data.insert("date".into(), Value::String(d.to_string()));
        }
        let desc = if it.excerpt.trim().is_empty() {
            summary(&body)
        } else {
            summary(&it.excerpt)
        };
        if !desc.is_empty() {
            data.insert("description".into(), Value::String(desc));
        }
        if let Some(f) = featured(it) {
            let v = mapped.get(&f).cloned().unwrap_or(f);
            data.insert("image".into(), Value::String(v));
        }
        let terms = |tax: &str| -> Vec<Value> {
            it.terms
                .iter()
                .filter(|(t, _, n)| {
                    t == tax && !n.is_empty() && n != "Uncategorized" && n != "Non classé"
                })
                .map(|(_, _, n)| Value::String(n.clone()))
                .collect()
        };
        let cats = terms("category");
        if !cats.is_empty() {
            data.insert("categories".into(), Value::Array(cats));
        }
        let tags = terms("post_tag");
        if !tags.is_empty() {
            data.insert("tags".into(), Value::Array(tags));
        }
        if let Some(a) = wxr
            .authors
            .get(&it.creator)
            .or(Some(&it.creator))
            .filter(|a| !a.is_empty())
        {
            data.insert("author".into(), Value::String(a.clone()));
        }
        if let Some(p) = local(&it.link).filter(|p| p != "/") {
            data.insert("permalink".into(), Value::String(p));
        }
        if it.status != "publish" {
            data.insert("draft".into(), Value::Bool(true));
        }
        let text = revel_project::entries::entry_text(&Value::Object(data), &body);
        let file = folder.join(format!("{slug}.md"));
        std::fs::write(&file, text).map_err(|e| ImportError::File {
            path: file.display().to_string(),
            msg: e.to_string(),
        })?;
        report.entries += 1;
    }
    report.collections = 1;
    if let Some(c) = site.collections.iter_mut().find(|c| c.id == "articles") {
        c.path = Some(POST_PATH.into());
    }
    // Page d'un article : le résumé (souvent le début du texte) sert aux listes, pas répété sous
    // le titre (comme les thèmes WordPress).
    if let Some(l) = site.layouts.iter_mut().find(|l| l.id == "article") {
        fn drop_summary(nodes: &mut Vec<Node>) {
            nodes.retain(|n| !matches!(n, Node::If { cond, .. } if cond.trim() == "description"));
            for n in nodes.iter_mut() {
                if let Node::Element { children, .. } = n {
                    drop_summary(children);
                }
            }
        }
        drop_summary(&mut l.body);
    }

    // Pages.
    let mut page_paths: HashMap<u64, String> = HashMap::new();
    let mut taken: HashSet<String> = site.pages.iter().map(|p| p.path.clone()).collect();
    let mut front: Option<Vec<Node>> = None;
    let mut pages: Vec<&Item> = wxr.pages().collect();
    pages.sort_by_key(|p| (p.parent != 0, p.menu_order, p.id));
    for it in pages {
        comments += it.comments;
        let body_html = clean(&it.content, &mut report);
        let content = crate::html::fragment(&body_html);
        let path = local(&it.link).unwrap_or_else(|| {
            let s = revel_model::slugify(if it.slug.is_empty() {
                &it.title
            } else {
                &it.slug
            });
            format!(
                "/{}/",
                if s.is_empty() {
                    format!("page-{}", it.id)
                } else {
                    s
                }
            )
        });
        if path == "/" && front.is_none() {
            front = Some(content);
            page_paths.insert(it.id, path);
            continue;
        }
        let mut path = path;
        while taken.contains(&path) {
            path = format!("{}-2/", path.trim_end_matches('/'));
        }
        taken.insert(path.clone());
        page_paths.insert(it.id, path.clone());
        let mut id = revel_model::slugify(path.trim_matches('/'));
        if id.is_empty() || site.pages.iter().any(|p| p.id == id) {
            id = format!("page-{}", it.id);
        }
        let mut props = BTreeMap::new();
        props.insert("seo".to_string(), json!({ "title": it.title }));
        site.pages.push(Page {
            id,
            path,
            layout: Some("base".into()),
            props,
            computed: Default::default(),
            body: vec![page_main(&it.title, content)],
            draft: it.status != "publish",
            name: Some(it.title.clone()),
            code: Default::default(),
        });
        report.pages += 1;
    }
    if comments > 0 {
        report.warnings.push(format!(
            "{comments} commentaire(s) non repris (pas de commentaires dans un site statique)"
        ));
    }

    // Accueil : page d'accueil de WordPress, sinon titre du site et derniers articles.
    let posts = report.entries;
    if let Some(index) = site.pages.iter_mut().find(|p| p.id == "index")
        && let Some(Node::Element { children, .. }) = index.body.first_mut()
    {
        let intro = revel_markup::parse(
            "<section class=\"bloc\"><div class=\"bloc__inner\"><h1 class=\"bloc-titre\">{{ site.title }}</h1>{% if site.description %}<p class=\"bloc-texte\">{{ site.description }}</p>{% endif %}</div></section>",
            &revel_markup::Options {
                dialect: revel_markup::Dialect::Nunjucks,
                ..Default::default()
            },
        )
        .map(|p| p.nodes)
        .unwrap_or_default();
        let list = children.pop();
        children.clear();
        match front {
            Some(nodes) => children.extend(nodes),
            None => children.extend(intro),
        }
        if posts > 0
            && let Some(l) = list
        {
            children.push(l);
        }
    }
    report.pages += 1;
    if posts == 0 {
        site.pages.retain(|p| p.id != "articles");
    } else {
        report.pages += 1;
    }

    // Menu : menu de navigation de WordPress, sinon accueil, pages et articles.
    let menu = nav_menu(wxr, &page_paths, &local, opts.fr).unwrap_or_else(|| {
        let mut m = vec![json!({ "label": l("Accueil", "Home"), "url": "/" })];
        let mut top: Vec<&Item> = wxr
            .pages()
            .filter(|p| p.parent == 0 && p.status == "publish")
            .collect();
        top.sort_by_key(|p| (p.menu_order, p.id));
        for p in top.into_iter().take(6) {
            if let Some(u) = page_paths.get(&p.id).filter(|u| *u != "/") {
                m.push(json!({ "label": p.title, "url": u }));
            }
        }
        if posts > 0 {
            m.push(json!({ "label": "Articles", "url": "/articles/" }));
        }
        m
    });
    let lang = wxr
        .language
        .split(['-', '_'])
        .next()
        .filter(|l| !l.is_empty())
        .unwrap_or(&site.default_locale)
        .to_ascii_lowercase();
    site.default_locale = lang.clone();
    if !wxr.title.is_empty() {
        site.name = wxr.title.clone();
    }
    site.data.insert(
        "site".into(),
        json!({ "title": site.name, "lang": lang, "description": wxr.description, "menu": menu }),
    );
    if let Some(b) = &base {
        site.base_url = Some(b.origin().ascii_serialization());
    }
    report.warnings.push(
        "WordPress : thème du site non repris (absent du fichier d'export) ; mise en page du modèle Blog de Revel".into(),
    );
    site.ensure_ids();
    Ok(report)
}

/// `<main class="page"><article class="article">` : titre et contenu d'une page.
fn page_main(title: &str, content: Vec<Node>) -> Node {
    let el = |tag: &str, class: &str, children: Vec<Node>| Node::Element {
        id: None,
        tag: tag.into(),
        attrs: vec![Attr {
            name: "class".into(),
            value: Some(class.into()),
            when: None,
        }],
        children,
    };
    el(
        "main",
        "page",
        vec![el(
            "article",
            "article",
            vec![
                el(
                    "h1",
                    "article__titre",
                    vec![Node::Text { text: title.into() }],
                ),
                el("div", "article__corps", content),
            ],
        )],
    )
}

/// Menu de navigation le plus long du fichier : liens de premier niveau, dans l'ordre.
fn nav_menu(
    wxr: &Wxr,
    page_paths: &HashMap<u64, String>,
    local: &dyn Fn(&str) -> Option<String>,
    fr: bool,
) -> Option<Vec<Value>> {
    let mut menus: BTreeMap<String, Vec<&Item>> = BTreeMap::new();
    for it in wxr.items.iter().filter(|i| i.post_type == "nav_menu_item") {
        if let Some((_, slug, _)) = it.terms.iter().find(|(t, _, _)| t == "nav_menu") {
            menus.entry(slug.clone()).or_default().push(it);
        }
    }
    let (_, mut items) = menus.into_iter().max_by_key(|(_, v)| v.len())?;
    items.retain(|i| {
        i.meta("_menu_item_menu_item_parent")
            .is_none_or(|p| p.trim() == "0")
    });
    items.sort_by_key(|i| i.menu_order);
    let by_id: HashMap<u64, &Item> = wxr.items.iter().map(|i| (i.id, i)).collect();
    let mut out = Vec::new();
    for it in items {
        let target: u64 = it
            .meta("_menu_item_object_id")
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        let (url, label) = match it.meta("_menu_item_type").unwrap_or("") {
            "custom" => {
                let u = it.meta("_menu_item_url").unwrap_or("").trim().to_string();
                (local(&u).unwrap_or(u), it.title.clone())
            }
            "post_type" => {
                let t = by_id.get(&target);
                let url = page_paths
                    .get(&target)
                    .cloned()
                    .or_else(|| t.and_then(|t| local(&t.link)))
                    .unwrap_or_else(|| "/".into());
                let label = if it.title.is_empty() {
                    t.map(|t| t.title.clone()).unwrap_or_default()
                } else {
                    it.title.clone()
                };
                (url, label)
            }
            "post_type_archive" => (
                "/articles/".into(),
                if it.title.is_empty() {
                    "Articles".into()
                } else {
                    it.title.clone()
                },
            ),
            _ => continue,
        };
        if label.trim().is_empty() || url.is_empty() {
            continue;
        }
        out.push(json!({ "label": label, "url": url }));
    }
    let _ = fr;
    (!out.is_empty()).then_some(out)
}

/// Résumé : texte sans balises, 160 caractères au plus, coupé à un mot.
fn summary(html: &str) -> String {
    let re = Regex::new(r"<[^>]*>|\[[^\]]*\]").expect("regex");
    let t = re.replace_all(html, " ");
    let t = decode_entities(&t);
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() <= 160 {
        return t;
    }
    let cut: String = t.chars().take(160).collect();
    match cut.rfind(' ') {
        Some(i) => format!("{}…", cut[..i].trim_end_matches([',', ';', ':', '.'])),
        None => format!("{cut}…"),
    }
}

fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&#8217;", "’")
        .replace("&#8216;", "‘")
        .replace("&#8220;", "“")
        .replace("&#8221;", "”")
        .replace("&#8230;", "…")
        .replace("&#8211;", "–")
        .replace("&#8212;", "—")
        .replace("&rsquo;", "’")
        .replace("&hellip;", "…")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Contenu nettoyé : commentaires de l'éditeur de blocs, légendes, intégrations, paragraphes de
/// l'éditeur classique ; lignes sans retrait ni ligne vide (texte lu comme du HTML).
pub fn clean_content(html: &str, report: &mut Report) -> String {
    let blocks = Regex::new(r"<!--\s*/?wp:[^>]*?-->").expect("regex");
    let mut s = blocks.replace_all(html, "").into_owned();
    let caption = Regex::new(r"(?s)\[caption([^\]]*)\](.*?)\[/caption\]").expect("regex");
    s = caption
        .replace_all(&s, |c: &regex::Captures| {
            let inner = c[2].trim();
            let (media, text) = match inner.rfind('>') {
                Some(i) => (&inner[..=i], inner[i + 1..].trim()),
                None => (inner, ""),
            };
            let align = Regex::new(r#"align="([a-z]+)""#)
                .expect("regex")
                .captures(&c[1])
                .map(|a| format!(" wp-caption {}", &a[1]))
                .unwrap_or_else(|| " wp-caption".into());
            format!(
                "<figure class=\"{}\">{media}<figcaption>{text}</figcaption></figure>",
                align.trim()
            )
        })
        .into_owned();
    let embed = Regex::new(r"(?s)\[embed[^\]]*\](.*?)\[/embed\]").expect("regex");
    s = embed
        .replace_all(&s, |c: &regex::Captures| {
            let u = c[1].trim();
            format!("<p><a href=\"{u}\">{u}</a></p>")
        })
        .into_owned();
    // Codes courts restants (`[gallery]`, `[contact-form-7]`…) : signalés.
    let short = Regex::new(r"\[([a-z][a-z0-9_-]{1,40})(?:\s[^\]]*)?\]").expect("regex");
    let mut names: Vec<String> = short
        .captures_iter(&s)
        .map(|c| c[1].to_string())
        .filter(|n| !matches!(n.as_str(), "x" | "i" | "a" | "b"))
        .collect();
    names.sort();
    names.dedup();
    for n in names {
        let w =
            format!("code court WordPress [{n}] gardé tel quel (extension WordPress non reprise)");
        if !report.warnings.contains(&w) {
            report.warnings.push(w);
        }
    }
    // Éditeur classique : paragraphes séparés par une ligne vide, sans balises <p>.
    if !s.contains("<p") && s.contains("\n\n")
        || !s.contains("<p") && !s.trim_start().starts_with('<')
    {
        s = autop(&s);
    }
    // Pas de ligne vide ni de retrait (bloc de code en Markdown), sauf dans <pre>.
    let mut out = String::with_capacity(s.len());
    let mut pre = false;
    for line in s.lines() {
        let l = line.to_ascii_lowercase();
        if pre {
            out.push_str(line);
            out.push('\n');
        } else {
            let t = line.trim();
            if !t.is_empty() {
                out.push_str(t);
                out.push('\n');
            }
        }
        if l.contains("<pre") {
            pre = !l.contains("</pre");
        } else if l.contains("</pre") {
            pre = false;
        }
    }
    out.trim_end().to_string()
}

/// Paragraphes de l'éditeur classique (`wpautop`, simplifié) : blocs séparés par une ligne vide
/// en `<p>`, retours à la ligne en `<br>`, éléments de bloc laissés tels quels.
fn autop(s: &str) -> String {
    const BLOCK: &[&str] = &[
        "<p",
        "<div",
        "<h1",
        "<h2",
        "<h3",
        "<h4",
        "<h5",
        "<h6",
        "<ul",
        "<ol",
        "<li",
        "<table",
        "<blockquote",
        "<pre",
        "<figure",
        "<hr",
        "<form",
        "<section",
        "<iframe",
        "<dl",
        "<address",
    ];
    let s = s.replace("\r\n", "\n");
    let mut out = Vec::new();
    for block in s.split("\n\n") {
        let b = block.trim();
        if b.is_empty() {
            continue;
        }
        let l = b.to_ascii_lowercase();
        if BLOCK.iter().any(|t| l.starts_with(t)) {
            out.push(b.to_string());
        } else {
            out.push(format!("<p>{}</p>", b.replace('\n', "<br>\n")));
        }
    }
    out.join("\n")
}

/// Adresses du site d'origine : médias téléchargés vers leur copie, liens internes depuis la
/// racine.
fn rewrite_links(
    html: &str,
    mapped: &HashMap<String, String>,
    hosts: &HashSet<String>,
    prefix: &str,
) -> String {
    let attr =
        Regex::new(r#"(?i)\b(href|src|srcset|data-src|data-srcset|poster)=("[^"]*"|'[^']*')"#)
            .expect("regex");
    let url = Regex::new(r#"(?:https?:)?//([A-Za-z0-9.-]+)(?::\d+)?(/[^\s"',]*)?"#).expect("regex");
    attr.replace_all(html, |c: &regex::Captures| {
        let v = &c[2];
        let q = &v[..1];
        let inner = &v[1..v.len() - 1];
        let new = url.replace_all(inner, |u: &regex::Captures| {
            let whole = u.get(0).map_or("", |m| m.as_str());
            let full = if whole.starts_with("//") {
                format!("https:{whole}")
            } else {
                whole.to_string()
            };
            if let Some(p) = mapped.get(&full).or_else(|| mapped.get(whole)) {
                return p.clone();
            }
            if hosts.contains(&u[1]) {
                let p = u.get(2).map_or("/", |m| m.as_str());
                if p.contains("wp-content/") || p.contains("wp-includes/") {
                    return whole.to_string();
                }
                let p = p.strip_prefix(prefix).unwrap_or(p);
                return if p.is_empty() {
                    "/".into()
                } else {
                    p.to_string()
                };
            }
            whole.to_string()
        });
        format!("{}={q}{new}{q}", &c[1])
    })
    .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    pub const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<rss version="2.0" xmlns:excerpt="http://wordpress.org/export/1.2/excerpt/" xmlns:content="http://purl.org/rss/1.0/modules/content/" xmlns:wfw="http://wellformedweb.org/CommentAPI/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:wp="http://wordpress.org/export/1.2/">
<channel>
<title>Atelier Lumen</title>
<link>https://atelier.example</link>
<description>Céramiques &amp; objets</description>
<language>fr-FR</language>
<wp:wxr_version>1.2</wp:wxr_version>
<wp:base_site_url>https://atelier.example</wp:base_site_url>
<wp:base_blog_url>https://atelier.example</wp:base_blog_url>
<wp:author><wp:author_id>1</wp:author_id><wp:author_login><![CDATA[marie]]></wp:author_login><wp:author_display_name><![CDATA[Marie Durand]]></wp:author_display_name></wp:author>
<item>
<title>Bols d'hiver</title>
<link>https://atelier.example/2024/01/bols-d-hiver/</link>
<dc:creator><![CDATA[marie]]></dc:creator>
<content:encoded><![CDATA[<!-- wp:paragraph -->
<p>Une série de <a href="https://atelier.example/a-propos/">bols</a> {{ émaillés }}.</p>
<!-- /wp:paragraph -->
<!-- wp:image -->
<figure class="wp-block-image"><img src="https://atelier.example/wp-content/uploads/2024/01/bol-1024x768.jpg" srcset="https://atelier.example/wp-content/uploads/2024/01/bol-1024x768.jpg 1024w, https://atelier.example/wp-content/uploads/2024/01/bol-300x225.jpg 300w" alt=""/></figure>
<!-- /wp:image -->
[gallery ids="4,5"]]]></content:encoded>
<excerpt:encoded><![CDATA[]]></excerpt:encoded>
<wp:post_id>10</wp:post_id>
<wp:post_date><![CDATA[2024-01-15 09:30:00]]></wp:post_date>
<wp:post_name><![CDATA[bols-d-hiver]]></wp:post_name>
<wp:status><![CDATA[publish]]></wp:status>
<wp:post_parent>0</wp:post_parent>
<wp:menu_order>0</wp:menu_order>
<wp:post_type><![CDATA[post]]></wp:post_type>
<category domain="category" nicename="ceramique"><![CDATA[Céramique]]></category>
<category domain="post_tag" nicename="hiver"><![CDATA[hiver]]></category>
<wp:postmeta><wp:meta_key><![CDATA[_thumbnail_id]]></wp:meta_key><wp:meta_value><![CDATA[20]]></wp:meta_value></wp:postmeta>
<wp:comment><wp:comment_id>1</wp:comment_id></wp:comment>
</item>
<item>
<title>Brouillon</title>
<link>https://atelier.example/?p=11</link>
<content:encoded><![CDATA[Premier paragraphe.

Deuxième
ligne.]]></content:encoded>
<wp:post_id>11</wp:post_id>
<wp:post_date><![CDATA[2024-02-01 10:00:00]]></wp:post_date>
<wp:post_name><![CDATA[]]></wp:post_name>
<wp:status><![CDATA[draft]]></wp:status>
<wp:post_type><![CDATA[post]]></wp:post_type>
</item>
<item>
<title>À propos</title>
<link>https://atelier.example/a-propos/</link>
<content:encoded><![CDATA[<p>L'atelier <strong>Lumen</strong>.</p>
[caption id="attachment_20" align="aligncenter" width="300"]<img src="https://atelier.example/wp-content/uploads/2024/01/four.jpg" alt="Four" /> Le four[/caption]]]></content:encoded>
<wp:post_id>12</wp:post_id>
<wp:post_name><![CDATA[a-propos]]></wp:post_name>
<wp:status><![CDATA[publish]]></wp:status>
<wp:post_parent>0</wp:post_parent>
<wp:menu_order>2</wp:menu_order>
<wp:post_type><![CDATA[page]]></wp:post_type>
</item>
<item>
<title>bol</title>
<link>https://atelier.example/bol/</link>
<wp:post_id>20</wp:post_id>
<wp:status><![CDATA[inherit]]></wp:status>
<wp:post_type><![CDATA[attachment]]></wp:post_type>
<wp:attachment_url><![CDATA[https://atelier.example/wp-content/uploads/2024/01/bol.jpg]]></wp:attachment_url>
</item>
<item>
<title></title>
<wp:post_id>30</wp:post_id>
<wp:status><![CDATA[publish]]></wp:status>
<wp:menu_order>1</wp:menu_order>
<wp:post_type><![CDATA[nav_menu_item]]></wp:post_type>
<category domain="nav_menu" nicename="principal"><![CDATA[Principal]]></category>
<wp:postmeta><wp:meta_key><![CDATA[_menu_item_type]]></wp:meta_key><wp:meta_value><![CDATA[post_type]]></wp:meta_value></wp:postmeta>
<wp:postmeta><wp:meta_key><![CDATA[_menu_item_menu_item_parent]]></wp:meta_key><wp:meta_value><![CDATA[0]]></wp:meta_value></wp:postmeta>
<wp:postmeta><wp:meta_key><![CDATA[_menu_item_object_id]]></wp:meta_key><wp:meta_value><![CDATA[12]]></wp:meta_value></wp:postmeta>
</item>
<item>
<title>Boutique</title>
<wp:post_id>31</wp:post_id>
<wp:status><![CDATA[publish]]></wp:status>
<wp:menu_order>2</wp:menu_order>
<wp:post_type><![CDATA[nav_menu_item]]></wp:post_type>
<category domain="nav_menu" nicename="principal"><![CDATA[Principal]]></category>
<wp:postmeta><wp:meta_key><![CDATA[_menu_item_type]]></wp:meta_key><wp:meta_value><![CDATA[custom]]></wp:meta_value></wp:postmeta>
<wp:postmeta><wp:meta_key><![CDATA[_menu_item_url]]></wp:meta_key><wp:meta_value><![CDATA[https://boutique.example/]]></wp:meta_value></wp:postmeta>
</item>
</channel>
</rss>"#;

    #[test]
    fn lecture_du_fichier() {
        let w = parse(SAMPLE).unwrap();
        assert_eq!(w.title, "Atelier Lumen");
        assert_eq!(w.description, "Céramiques & objets");
        assert_eq!(w.language, "fr-FR");
        assert_eq!(
            w.authors.get("marie").map(String::as_str),
            Some("Marie Durand")
        );
        assert_eq!(w.posts().count(), 2);
        assert_eq!(w.pages().count(), 1);
        assert_eq!(w.attachments().count(), 1);
        let p = w.posts().next().unwrap();
        assert_eq!(p.slug, "bols-d-hiver");
        assert_eq!(p.meta("_thumbnail_id"), Some("20"));
        assert_eq!(p.comments, 1);
        assert_eq!(p.terms.len(), 2);
        assert!(parse("<html></html>").is_err());
    }

    #[test]
    fn contenu_nettoye() {
        let mut r = Report::default();
        let w = parse(SAMPLE).unwrap();
        let p = w.posts().next().unwrap();
        let c = clean_content(&p.content, &mut r);
        assert!(!c.contains("wp:paragraph"));
        assert!(c.starts_with("<p>Une série"));
        assert!(r.warnings.iter().any(|w| w.contains("[gallery]")));
        let page = w.pages().next().unwrap();
        let c = clean_content(&page.content, &mut r);
        assert!(
            c.contains("<figure class=\"wp-caption aligncenter\"><img"),
            "{c}"
        );
        assert!(c.contains("<figcaption>Le four</figcaption>"));
        let draft = w.posts().nth(1).unwrap();
        assert_eq!(
            clean_content(&draft.content, &mut r),
            "<p>Premier paragraphe.</p>\n<p>Deuxième<br>\nligne.</p>"
        );
        let mapped: HashMap<String, String> = [(
            "https://atelier.example/wp-content/uploads/2024/01/bol-300x225.jpg".to_string(),
            "/wp-content/uploads/2024/01/bol-300x225.jpg".to_string(),
        )]
        .into();
        let hosts: HashSet<String> = ["atelier.example".to_string()].into();
        let out = rewrite_links(
            r#"<a href="https://atelier.example/contact/">c</a><img srcset="https://atelier.example/wp-content/uploads/2024/01/bol-300x225.jpg 300w, https://atelier.example/wp-content/uploads/2024/01/x.jpg 2x"><a href="https://autre.example/">x</a>"#,
            &mapped,
            &hosts,
            "",
        );
        assert_eq!(
            out,
            r#"<a href="/contact/">c</a><img srcset="/wp-content/uploads/2024/01/bol-300x225.jpg 300w, https://atelier.example/wp-content/uploads/2024/01/x.jpg 2x"><a href="https://autre.example/">x</a>"#
        );
    }
}
