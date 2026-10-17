//! Import d'un site HTML statique (7.1) : un dossier de pages `.html`, de feuilles de style et de
//! fichiers, fait à la main, exporté d'un outil (Webflow) ou produit par un générateur.
//!
//! - chaque page garde son adresse (`index.html` → `/`, `a/index.html` → `/a/`, `b.html` →
//!   `/b.html`) ; les adresses relatives des pages sont réécrites depuis la racine, ce qui permet de
//!   partager le gabarit entre pages de profondeurs différentes ;
//! - gabarit `base` : éléments d'en-tête présents dans toutes les pages, titre et description lus
//!   dans le référencement de chaque page (`seo.title`, `seo.description`), attributs de `<html>`
//!   et `<body>`, scripts communs de fin de corps ;
//! - gabarit `site` (enfant de `base`) : début et fin du corps communs au plus grand groupe de
//!   pages (navigation, pied de page), dans les enveloppes communes ;
//! - feuilles de style locales liées par toutes les pages : fusionnées dans la feuille du site
//!   (D-18) ;
//! - le reste du dossier est copié tel quel (passthrough).
//!
//! L'arbre HTML est lu par html5ever (via `dom_query`), comme un navigateur : balises non
//! fermées, `<tbody>` implicite, attributs sans guillemets. `{{` et `{%` restent du texte.

use crate::Report;
use crate::adapt::{self, Tool};
use crate::eleventy::ImportError;
use dom_query::{Document, NodeData, NodeRef};
use revel_model::{Attr, CustomCode, Layout, Node, Page, Passthrough, Site, Stylesheet};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// Dossiers jamais lus ni copiés.
const SKIP_DIRS: &[&str] = &["node_modules", ".git", ".revel", ".github", "__MACOSX"];

/// Fichiers de dépôt non publiés (à la racine).
fn repo_file(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    let stem = l.split('.').next().unwrap_or("");
    matches!(
        stem,
        "readme" | "license" | "licence" | "changelog" | "contributing"
    ) || matches!(
        l.as_str(),
        "package.json"
            | "package-lock.json"
            | "yarn.lock"
            | "pnpm-lock.yaml"
            | "bun.lock"
            | "bun.lockb"
            | "site.revel.json"
            | "revel-import.json"
            | "gemfile"
            | "gemfile.lock"
    )
}

/// Pages HTML d'un dossier (chemins relatifs, `/` comme séparateur), accueil en premier.
pub fn scan(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            let path = e.path();
            let r = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            if path.is_dir() {
                stack.push((path, r));
            } else if is_html(&name) {
                out.push(r);
            }
        }
    }
    out.sort_by(|a, b| {
        let key = |s: &str| (s != "index.html", s.matches('/').count(), s.to_string());
        key(a).cmp(&key(b))
    });
    out
}

fn is_html(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    l.ends_with(".html") || l.ends_with(".htm")
}

/// Le dossier contient des pages HTML à sa racine.
pub fn is_static(root: &Path) -> bool {
    std::fs::read_dir(root).is_ok_and(|rd| {
        rd.flatten()
            .any(|e| e.path().is_file() && is_html(&e.file_name().to_string_lossy()))
    })
}

/// Réglages de l'import.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Nom du site si aucun n'est trouvé dans les pages (`og:site_name`) : en général le nom du
    /// dossier.
    pub name: String,
    /// Langue si les pages n'en déclarent pas (`<html lang>`).
    pub lang: String,
    /// Blocs `<style>` de l'en-tête des pages repris dans la feuille du site (règles en double
    /// fusionnées) : site lu à son adresse (7.4), dont les styles sont écrits dans les pages.
    pub merge_styles: bool,
    /// Fichiers CSV des collections du CMS de Webflow (7.3), en plus de ceux du dossier.
    pub cms_csv: Vec<PathBuf>,
}

/// Page lue.
struct Doc {
    url: String,
    doctype: Option<String>,
    html_attrs: Vec<Attr>,
    head: Vec<Node>,
    body_attrs: Vec<Attr>,
    body: Vec<Node>,
    title: Option<String>,
    description: Option<String>,
}

/// Importe le site HTML statique situé dans `root`.
pub fn import(root: &Path, opts: &Options) -> Result<(Site, Report), ImportError> {
    let files = scan(root);
    if files.is_empty() {
        return Err(ImportError::File {
            path: root.display().to_string(),
            msg: "aucune page HTML".into(),
        });
    }
    let mut report = Report::default();
    let mut docs = Vec::new();
    for rel in &files {
        let path = root.join(rel);
        let bytes = std::fs::read(&path).map_err(|e| ImportError::File {
            path: path.display().to_string(),
            msg: e.to_string(),
        })?;
        let text = match String::from_utf8(bytes) {
            Ok(t) => t,
            Err(e) => {
                report.warnings.push(format!(
                    "{rel} : texte qui n'est pas en UTF-8, caractères non reconnus remplacés"
                ));
                String::from_utf8_lossy(e.as_bytes()).into_owned()
            }
        };
        docs.push(read_page(
            rel,
            text.trim_start_matches('\u{feff}'),
            &mut report,
        ));
    }

    // Outil d'origine : moteur retiré, animations et formulaires repris (7.3, 7.7).
    let tool = adapt::tool(&docs[0].head, &docs[0].html_attrs);
    report.tool = tool.map(|t| t.name().to_string());
    let mut extra_css = String::new();
    match tool {
        Some(Tool::Framer) => {
            for d in &mut docs {
                report.animations += adapt::framer(&mut d.head, &mut d.body);
            }
            report.warnings.push(
                "Framer : menus, carrousels et effets au survol produits par le moteur de Framer non repris ; animations d'apparition reprises dans le panneau Interactions".into(),
            );
        }
        Some(Tool::Webflow) => {
            let ix2 = webflow_interactions(root, &docs);
            for d in &mut docs {
                let (a, f) = adapt::webflow(&mut d.body, ix2.as_ref());
                report.animations += a;
                report.forms += f;
            }
            if report.forms > 0 {
                extra_css.push_str(adapt::WEBFLOW_FORM_CSS);
                extra_css.push('\n');
            }
        }
        _ => {}
    }
    for d in &mut docs {
        report.animations += adapt::aos(&mut d.body);
    }
    if opts.merge_styles
        && let Some(css) = take_inline_styles(&mut docs, &mut report)
    {
        extra_css.insert_str(0, &css);
    }

    let mut site = Site::new(
        site_name(&docs).unwrap_or_else(|| opts.name.clone()),
        docs[0]
            .html_attrs
            .iter()
            .find(|a| a.name == "lang")
            .and_then(|a| a.value.clone())
            .filter(|l| !l.trim().is_empty())
            .unwrap_or_else(|| {
                if opts.lang.is_empty() {
                    "en".into()
                } else {
                    opts.lang.clone()
                }
            }),
    );
    site.base_url = base_url(&docs);
    site.data.insert(
        "site".into(),
        json!({ "title": site.name, "lang": site.default_locale, "description": "" }),
    );

    // En-tête commun à toutes les pages (ordre de la première).
    let keys: Vec<HashSet<String>> = docs
        .iter()
        .map(|d| d.head.iter().map(key).collect())
        .collect();
    let mut common_head: Vec<Node> = docs[0]
        .head
        .iter()
        .filter(|n| keys.iter().all(|k| k.contains(&key(n))))
        .cloned()
        .collect();
    // Feuilles de style locales communes : fusionnées dans la feuille du site.
    merge_stylesheets(root, &mut common_head, &extra_css, &mut site, &mut report);
    let common_keys: HashSet<String> = docs[0]
        .head
        .iter()
        .filter(|n| keys.iter().all(|k| k.contains(&key(n))))
        .map(key)
        .collect();

    // Fin du corps : scripts communs (gabarit) et scripts propres à une page (code ajouté).
    let tails: Vec<(usize, Vec<Node>)> = docs.iter().map(|d| trailing_scripts(&d.body)).collect();
    let tail_keys: Vec<HashSet<String>> = tails
        .iter()
        .map(|t| t.1.iter().map(key).collect())
        .collect();
    let common_tail: Vec<Node> = tails[0]
        .1
        .iter()
        .filter(|n| tail_keys.iter().all(|k| k.contains(&key(n))))
        .cloned()
        .collect();
    let common_tail_keys: HashSet<String> = common_tail.iter().map(key).collect();
    let mains: Vec<Vec<Node>> = docs
        .iter()
        .zip(&tails)
        .map(|(d, t)| significant(&d.body[..t.0]))
        .collect();

    // Attributs de <html> et <body> : écrits dans le gabarit, ou variables de page.
    let (html_attrs, html_props) = shared_attrs(
        &docs
            .iter()
            .map(|d| d.html_attrs.as_slice())
            .collect::<Vec<_>>(),
        "html",
    );
    let (body_attrs, body_props) = shared_attrs(
        &docs
            .iter()
            .map(|d| d.body_attrs.as_slice())
            .collect::<Vec<_>>(),
        "body",
    );

    // Gabarit de base : document, en-tête, corps.
    let mut head = Vec::new();
    let mut seo_done = false;
    for n in common_head {
        let lead = is_head_lead(&n);
        if !seo_done && !lead {
            head.extend(seo_nodes());
            seo_done = true;
        }
        head.push(n);
    }
    if !seo_done {
        head.extend(seo_nodes());
    }
    let mut body = vec![Node::Slot];
    body.extend(common_tail.iter().cloned());
    let mut base = Vec::new();
    if let Some(d) = &docs[0].doctype {
        base.push(Node::Html { html: d.clone() });
        base.push(nl());
    }
    base.push(Node::Element {
        id: None,
        tag: "html".into(),
        attrs: html_attrs,
        children: vec![
            nl(),
            element("head", Vec::new(), lines(head)),
            nl(),
            element("body", body_attrs, lines(body)),
            nl(),
        ],
    });
    site.layouts.push(Layout {
        id: "base".into(),
        parent: None,
        props: Default::default(),
        body: base,
    });

    // Gabarit des pages : début et fin du corps communs au plus grand groupe de pages.
    // Les liens de navigation marqués « page en cours » (aria-current, classe `w--current`,
    // `active`…) diffèrent d'une page à l'autre : comparés sans ces marques, puis écrits dans le
    // gabarit avec une condition sur l'adresse de la page.
    let stripped: Vec<Vec<Node>> = mains.iter().map(|m| strip_current(m)).collect();
    let group = largest_group(&stripped);
    let mut shell: Option<Shell> = None;
    if group.len() >= 2 {
        let urls: Vec<&str> = group.iter().map(|&i| docs[i].url.as_str()).collect();
        let lists: Vec<&[Node]> = group.iter().map(|&i| stripped[i].as_slice()).collect();
        let mut s = shell_of(&lists);
        let originals: Vec<&[Node]> = group.iter().map(|&i| mains[i].as_slice()).collect();
        let shared = s.shared(&originals, &urls).or_else(|| {
            // Marques inexpliquées : comparaison exacte.
            s = shell_of(&originals);
            s.shared(&originals, &urls)
        });
        if let Some((before, after)) = shared
            && (!s.wrappers.is_empty() || s.prefix + s.suffix > 0)
        {
            site.layouts.push(Layout {
                id: "site".into(),
                parent: Some("base".into()),
                props: Default::default(),
                body: s.layout(before, after),
            });
            shell = Some(s);
        }
    }
    report.layouts = site.layouts.len();

    // Pages.
    let mut ids = HashSet::new();
    for (i, d) in docs.iter().enumerate() {
        let in_group = shell.is_some() && group.contains(&i);
        let mut content: Vec<Node> = Vec::new();
        let mut head_code = String::new();
        for n in &d.head {
            if common_keys.contains(&key(n)) {
                continue;
            }
            if is_visual_head(n) {
                content.push(n.clone());
                content.push(nl());
            } else {
                head_code.push_str(&to_html(std::slice::from_ref(n)));
                head_code.push('\n');
            }
        }
        let middle = match (&shell, in_group) {
            (Some(s), true) => s.middle(&mains[i]),
            _ => mains[i].clone(),
        };
        content.extend(lines(middle));
        let body_code: String = tails[i]
            .1
            .iter()
            .filter(|n| !common_tail_keys.contains(&key(n)))
            .map(|n| to_html(std::slice::from_ref(n)))
            .collect::<Vec<_>>()
            .join("\n");
        let mut props = BTreeMap::new();
        let mut seo = serde_json::Map::new();
        if let Some(t) = &d.title {
            seo.insert("title".into(), Value::String(t.clone()));
        }
        if let Some(t) = &d.description {
            seo.insert("description".into(), Value::String(t.clone()));
        }
        if !seo.is_empty() {
            props.insert("seo".into(), Value::Object(seo));
        }
        for (prop, values) in html_props.iter().chain(body_props.iter()) {
            if let Some(v) = &values[i] {
                props.insert(prop.clone(), Value::String(v.clone()));
            }
        }
        let mut id = page_id(&d.url);
        while !ids.insert(id.clone()) {
            id.push('-');
        }
        site.pages.push(Page {
            id,
            path: d.url.clone(),
            layout: Some(if in_group { "site" } else { "base" }.into()),
            props,
            computed: Default::default(),
            body: content,
            draft: false,
            name: None,
            code: CustomCode {
                head: head_code.trim_end().to_string(),
                body: body_code,
            },
        });
    }
    report.pages = site.pages.len();
    page_names(&mut site);

    // Fichiers copiés tels quels, références du site (robots.txt, plan du site).
    site.passthrough = passthrough(root, &files);
    if let Ok(robots) = std::fs::read_to_string(root.join("robots.txt")) {
        site.settings.seo.robots = Some(robots);
    }
    if root.join("sitemap.xml").is_file() {
        site.settings.seo.no_sitemap = true;
    }
    Ok((site, report))
}

/// Noms des pages dans Revel (sans effet sur le site) : titre sans la fin répétée sur toutes les
/// pages (« | Nom du site ») ; titre absent, vide ou partagé par plusieurs pages : adresse.
fn page_names(site: &mut Site) {
    const SEPS: &[&str] = &[" | ", " — ", " – ", " - ", " · ", " :: "];
    let titles: Vec<Option<String>> = site
        .pages
        .iter()
        .map(|p| {
            p.props
                .get("seo")
                .and_then(|s| s.get("title"))
                .and_then(Value::as_str)
                .map(|t| t.trim().to_string())
        })
        .collect();
    // Fin répétée la plus fréquente (au moins deux pages et un tiers des pages).
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for t in titles.iter().flatten() {
        if let Some(i) = SEPS.iter().filter_map(|s| t.rfind(s)).max() {
            *counts.entry(t[i..].to_string()).or_default() += 1;
        }
    }
    let suffix = counts
        .into_iter()
        .filter(|(_, n)| *n >= 2 && *n * 3 >= titles.len())
        .max_by_key(|(_, n)| *n)
        .map(|(s, _)| s);
    let names: Vec<String> = titles
        .iter()
        .map(|t| {
            let t = t.clone().unwrap_or_default();
            let t = match &suffix {
                Some(s) => t.strip_suffix(s.as_str()).unwrap_or(&t).trim().to_string(),
                None => t,
            };
            if t == site.name { String::new() } else { t }
        })
        .collect();
    let fr = site.default_locale.starts_with("fr");
    for (i, page) in site.pages.iter_mut().enumerate() {
        let n = &names[i];
        let shared = names.iter().filter(|m| *m == n).count() > 1;
        let name = if n.is_empty() || shared {
            path_name(&page.path, fr)
        } else {
            n.clone()
        };
        if titles[i].as_deref() != Some(name.as_str()) {
            page.name = Some(name);
        }
    }
}

/// Nom lisible d'une adresse (`/nos-offres/` → `Nos offres`, `/` → `Accueil` ou `Home`).
fn path_name(path: &str, fr: bool) -> String {
    let last = path
        .trim_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim_end_matches(".html")
        .trim_end_matches(".htm");
    if last.is_empty() {
        return if fr { "Accueil" } else { "Home" }.into();
    }
    let s = last.replace(['-', '_'], " ");
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => s,
    }
}

// -------------------------------------------------------------------------------------------------
// Lecture d'une page

fn read_page(rel: &str, text: &str, report: &mut Report) -> Doc {
    let doc = Document::from(text);
    let url = page_url(rel);
    let dir = match url.rfind('/') {
        Some(i) => url[..=i].to_string(),
        None => "/".into(),
    };
    let cx = Cx { dir: &dir };
    let mut d = Doc {
        url,
        doctype: None,
        html_attrs: Vec::new(),
        head: Vec::new(),
        body_attrs: Vec::new(),
        body: Vec::new(),
        title: None,
        description: None,
    };
    for n in doc.root().children() {
        if let Some(dt) = n
            .query(|t| match &t.data {
                NodeData::Doctype { name, .. } => Some(format!("<!DOCTYPE {name}>")),
                _ => None,
            })
            .flatten()
        {
            d.doctype = Some(dt);
        }
    }
    let html = doc.html_root();
    d.html_attrs = attrs_of(&html, &cx);
    for part in html.element_children() {
        match part.node_name().as_deref() {
            Some("head") => {
                for n in part.children() {
                    let Some(node) = convert(&n, &cx) else {
                        continue;
                    };
                    if is_blank(&node) {
                        continue;
                    }
                    match &node {
                        Node::Element { tag, children, .. } if tag == "title" => {
                            if d.title.is_none() {
                                d.title = Some(plain_text(children).trim().to_string());
                            }
                            continue;
                        }
                        Node::Element { tag, attrs, .. }
                            if tag == "meta"
                                && attr(attrs, "name")
                                    .is_some_and(|n| n.eq_ignore_ascii_case("description")) =>
                        {
                            if d.description.is_none() {
                                d.description = attr(attrs, "content").map(str::to_string);
                            }
                            continue;
                        }
                        Node::Element { tag, .. } if tag == "base" => {
                            report.warnings.push(format!(
                                "{rel} : élément <base> gardé ; les adresses relatives sont lues depuis l'adresse de la page"
                            ));
                        }
                        _ => {}
                    }
                    d.head.push(node);
                }
            }
            Some("body") => {
                d.body_attrs = attrs_of(&part, &cx);
                d.body = part
                    .children()
                    .iter()
                    .filter_map(|n| convert(n, &cx))
                    .collect();
            }
            _ => {}
        }
    }
    d
}

/// Éléments d'un morceau de HTML (contenu d'un article, d'une page de CMS), lus comme dans une
/// page à la racine du site : `{{` reste du texte.
pub fn fragment(html: &str) -> Vec<Node> {
    let doc = Document::from(format!(
        "<!DOCTYPE html><html><head></head><body>{html}</body></html>"
    ));
    let cx = Cx { dir: "/" };
    doc.select("body")
        .nodes()
        .first()
        .map(|b| {
            b.children()
                .iter()
                .filter_map(|n| convert(n, &cx))
                .collect()
        })
        .unwrap_or_default()
}

/// Outil qui a produit les pages du dossier (accueil lu), pour l'annoncer avant l'import.
pub fn origin(root: &Path) -> Option<Tool> {
    let first = scan(root).into_iter().next()?;
    let text = std::fs::read_to_string(root.join(&first)).ok()?;
    let d = read_page(&first, &text, &mut Report::default());
    adapt::tool(&d.head, &d.html_attrs)
}

/// Adresse publiée d'un fichier de page.
fn page_url(rel: &str) -> String {
    let lower = rel.to_ascii_lowercase();
    if lower == "index.html" || lower == "index.htm" {
        return "/".into();
    }
    for idx in ["/index.html", "/index.htm"] {
        if lower.ends_with(idx) {
            return format!("/{}/", &rel[..rel.len() - idx.len()]);
        }
    }
    format!("/{rel}")
}

/// Identifiant de page d'après son adresse.
fn page_id(url: &str) -> String {
    let s = revel_model::slugify(url.trim_matches('/').trim_end_matches(".html"));
    if s.is_empty() { "index".into() } else { s }
}

struct Cx<'a> {
    /// Dossier de la page (`/a/`), pour les adresses relatives.
    dir: &'a str,
}

/// Éléments au contenu brut (non échappé à la génération).
const RAW: &[&str] = &[
    "script",
    "style",
    "xmp",
    "iframe",
    "noembed",
    "noframes",
    "plaintext",
];

fn convert(n: &NodeRef, cx: &Cx) -> Option<Node> {
    let data = n.query(|t| match &t.data {
        NodeData::Text { contents } => Some(Ok(contents.to_string())),
        NodeData::Comment { contents } => Some(Err(format!("<!--{contents}-->"))),
        NodeData::Element(_) => None,
        _ => Some(Err(String::new())),
    })?;
    // Texte et commentaire.
    if let Some(r) = data {
        return match r {
            Ok(text) => Some(Node::Text { text }),
            Err(html) if html.is_empty() => None,
            Err(html) => Some(Node::Html { html }),
        };
    }
    convert_element(n, cx)
}

fn convert_element(n: &NodeRef, cx: &Cx) -> Option<Node> {
    let (tag, template) = n.query(|t| match &t.data {
        NodeData::Element(e) => Some((
            match &e.name.prefix {
                Some(p) if !p.is_empty() => format!("{p}:{}", e.name.local),
                _ => e.name.local.to_string(),
            },
            e.template_contents,
        )),
        _ => None,
    })??;
    let lower = tag.to_ascii_lowercase();
    let attrs = attrs_of(n, cx);
    let kids: Vec<NodeRef> = match template {
        Some(id) => NodeRef::new(id, n.tree).children(),
        None => n.children(),
    };
    let children = if RAW.contains(&lower.as_str()) {
        let raw: String = kids
            .iter()
            .filter_map(|k| {
                k.query(|t| match &t.data {
                    NodeData::Text { contents } => Some(contents.to_string()),
                    _ => None,
                })
                .flatten()
            })
            .collect();
        let raw = if lower == "style" {
            rewrite_css_urls(&raw, cx.dir)
        } else {
            raw
        };
        if raw.is_empty() {
            Vec::new()
        } else {
            vec![Node::Html { html: raw }]
        }
    } else {
        kids.iter().filter_map(|k| convert(k, cx)).collect()
    };
    Some(Node::Element {
        id: None,
        tag,
        attrs,
        children,
    })
}

/// Attributs d'un élément : adresses locales réécrites depuis la racine, `{{` gardé en texte.
fn attrs_of(n: &NodeRef, cx: &Cx) -> Vec<Attr> {
    n.query(|t| match &t.data {
        NodeData::Element(e) => e
            .attrs
            .iter()
            .map(|a| {
                let name = match &a.name.prefix {
                    Some(p) if !p.is_empty() => format!("{p}:{}", a.name.local),
                    _ => a.name.local.to_string(),
                };
                let value = a.value.to_string();
                let value = match name.as_str() {
                    "href" | "src" | "poster" | "action" | "formaction" | "data" | "xlink:href"
                    | "background" => absolute(&value, cx.dir),
                    "srcset" | "imagesrcset" => srcset(&value, cx.dir),
                    "style" => rewrite_css_urls(&value, cx.dir),
                    _ => value,
                };
                Attr {
                    name,
                    value: Some(literal(&value)),
                    when: None,
                }
            })
            .collect(),
        _ => Vec::new(),
    })
    .unwrap_or_default()
}

/// Valeur d'attribut écrite telle quelle : `{{`, `{%` et `{#` deviennent des chaînes (les valeurs
/// d'attributs de Revel peuvent contenir des expressions).
fn literal(v: &str) -> String {
    if !v.contains("{{") && !v.contains("{%") {
        return v.to_string();
    }
    let mut out = String::new();
    let mut rest = v;
    while let Some(i) = ["{{", "{%", "{#"].iter().filter_map(|p| rest.find(p)).min() {
        out.push_str(&rest[..i]);
        out.push_str(&format!("{{{{ '{}' }}}}", &rest[i..i + 2]));
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

/// Adresse locale relative réécrite depuis la racine du site (`img/a.png` dans `/b/` →
/// `/b/img/a.png`) ; les adresses absolues, externes, ancres et schémas restent telles quelles.
fn absolute(v: &str, dir: &str) -> String {
    let t = v.trim();
    if t.is_empty()
        || t.starts_with('/')
        || t.starts_with('#')
        || t.starts_with('?')
        || t.starts_with("{{")
        || t.contains("://")
        || t.split_once(':').is_some_and(|(scheme, _)| {
            !scheme.is_empty()
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        })
    {
        return v.to_string();
    }
    let (path, tail) = match t.find(['?', '#']) {
        Some(i) => (&t[..i], &t[i..]),
        None => (t, ""),
    };
    let mut segs: Vec<&str> = dir
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let trailing = path.ends_with('/') || path.is_empty();
    for s in path.split('/') {
        match s {
            "" | "." => {}
            ".." => {
                segs.pop();
            }
            s => segs.push(s),
        }
    }
    let mut out = format!("/{}", segs.join("/"));
    if trailing && !out.ends_with('/') {
        out.push('/');
    }
    out.push_str(tail);
    out
}

fn srcset(v: &str, dir: &str) -> String {
    v.split(',')
        .map(|part| {
            let p = part.trim();
            match p.split_once(char::is_whitespace) {
                Some((u, d)) => format!("{} {}", absolute(u, dir), d.trim()),
                None => absolute(p, dir),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Adresses `url(…)` et `@import "…"` d'un texte CSS réécrites depuis la racine.
pub fn rewrite_css_urls(css: &str, dir: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    loop {
        let next_url = rest.find("url(");
        let next_import = rest.find("@import");
        let Some(i) = [next_url, next_import].into_iter().flatten().min() else {
            out.push_str(rest);
            return out;
        };
        if Some(i) == next_url {
            out.push_str(&rest[..i + 4]);
            rest = &rest[i + 4..];
            let Some(end) = crate::crawl::url_close(rest) else {
                out.push_str(rest);
                return out;
            };
            let inner = rest[..end].trim();
            let (q, u) = match inner.chars().next() {
                Some(c @ ('"' | '\'')) => (Some(c), inner.trim_matches(c)),
                _ => (None, inner),
            };
            let u = absolute(u, dir);
            match q {
                Some(q) => out.push_str(&format!("{q}{u}{q}")),
                None => out.push_str(&u),
            }
            out.push(')');
            rest = &rest[end + 1..];
        } else {
            out.push_str(&rest[..i + 7]);
            rest = &rest[i + 7..];
            let trimmed = rest.trim_start();
            let ws = &rest[..rest.len() - trimmed.len()];
            if let Some(q @ ('"' | '\'')) = trimmed.chars().next() {
                let body = &trimmed[1..];
                let Some(end) = body.find(q) else {
                    out.push_str(rest);
                    return out;
                };
                out.push_str(ws);
                out.push(q);
                out.push_str(&absolute(&body[..end], dir));
                out.push(q);
                rest = &body[end + 1..];
            }
        }
    }
}

fn attr<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(name))
        .and_then(|a| a.value.as_deref())
}

fn plain_text(nodes: &[Node]) -> String {
    let mut s = String::new();
    for n in nodes {
        match n {
            Node::Text { text } => s.push_str(text),
            Node::Element { children, .. } => s.push_str(&plain_text(children)),
            _ => {}
        }
    }
    s
}

fn is_blank(n: &Node) -> bool {
    matches!(n, Node::Text { text } if text.trim().is_empty())
}

/// Nœuds significatifs (sans les blancs entre éléments).
fn significant(nodes: &[Node]) -> Vec<Node> {
    nodes.iter().filter(|n| !is_blank(n)).cloned().collect()
}

/// Clé de comparaison d'un nœud (structure complète).
fn key(n: &Node) -> String {
    serde_json::to_string(n).unwrap_or_default()
}

/// Clé courte : balise et attributs, sans le contenu.
fn shallow(n: &Node) -> Option<String> {
    match n {
        Node::Element { tag, attrs, .. } => Some(format!(
            "{tag}{}",
            serde_json::to_string(attrs).unwrap_or_default()
        )),
        _ => None,
    }
}

fn element(tag: &str, attrs: Vec<Attr>, children: Vec<Node>) -> Node {
    Node::Element {
        id: None,
        tag: tag.into(),
        attrs,
        children,
    }
}

fn nl() -> Node {
    Node::Text { text: "\n".into() }
}

/// Nœuds séparés par des retours à la ligne (site produit lisible).
fn lines(nodes: Vec<Node>) -> Vec<Node> {
    let mut out = Vec::with_capacity(nodes.len() * 2 + 1);
    for n in nodes {
        out.push(nl());
        out.push(n);
    }
    if !out.is_empty() {
        out.push(nl());
    }
    out
}

/// Éléments de tête d'un document, avant le titre : jeu de caractères, fenêtre, `http-equiv`.
fn is_head_lead(n: &Node) -> bool {
    match n {
        Node::Element { tag, attrs, .. } if tag == "meta" => {
            attr(attrs, "charset").is_some()
                || attr(attrs, "http-equiv").is_some()
                || attr(attrs, "name").is_some_and(|v| v.eq_ignore_ascii_case("viewport"))
        }
        Node::Html { .. } => true,
        _ => false,
    }
}

/// Élément d'en-tête qui change l'apparence (feuille de style, bloc `<style>`) : placé au début
/// du contenu de la page pour rester visible dans le canevas.
fn is_visual_head(n: &Node) -> bool {
    match n {
        Node::Element { tag, attrs, .. } => {
            tag == "style"
                || (tag == "link"
                    && attr(attrs, "rel").is_some_and(|r| {
                        r.split_whitespace()
                            .any(|t| t.eq_ignore_ascii_case("stylesheet"))
                    }))
        }
        _ => false,
    }
}

/// Titre et description lus dans le référencement de la page (D-32).
/// Document de base d'un site produit par un import (Figma) : type de document, langue, en-tête
/// (encodage, écran, titre et description de la page, feuille du site), emplacement de la page.
pub(crate) fn base_document(lang: &str, css: &str) -> Vec<Node> {
    let a = |n: &str, v: &str| Attr {
        name: n.into(),
        value: Some(v.into()),
        when: None,
    };
    let mut head = vec![
        element("meta", vec![a("charset", "utf-8")], Vec::new()),
        element(
            "meta",
            vec![
                a("name", "viewport"),
                a("content", "width=device-width, initial-scale=1"),
            ],
            Vec::new(),
        ),
    ];
    head.extend(seo_nodes());
    head.push(element(
        "link",
        vec![a("rel", "stylesheet"), a("href", css)],
        Vec::new(),
    ));
    vec![
        Node::Html {
            html: "<!DOCTYPE html>".into(),
        },
        nl(),
        element(
            "html",
            vec![a("lang", lang)],
            vec![
                nl(),
                element("head", Vec::new(), lines(head)),
                nl(),
                element("body", Vec::new(), lines(vec![Node::Slot])),
                nl(),
            ],
        ),
    ]
}

fn seo_nodes() -> Vec<Node> {
    vec![
        element(
            "title",
            Vec::new(),
            vec![Node::Out {
                expr: "seo.title if seo and seo.title else site.title".into(),
            }],
        ),
        Node::If {
            id: None,
            cond: "seo and seo.description".into(),
            then: vec![element(
                "meta",
                vec![
                    Attr {
                        name: "name".into(),
                        value: Some("description".into()),
                        when: None,
                    },
                    Attr {
                        name: "content".into(),
                        value: Some("{{ seo.description }}".into()),
                        when: None,
                    },
                ],
                Vec::new(),
            )],
            otherwise: Vec::new(),
        },
    ]
}

/// Scripts de fin de corps (suite d'éléments `<script>` et `<noscript>`, blancs compris) : indice
/// de début dans `body` et scripts, dans l'ordre.
fn trailing_scripts(body: &[Node]) -> (usize, Vec<Node>) {
    let mut n = 0;
    for node in body.iter().rev() {
        match node {
            Node::Element { tag, .. } if tag == "script" || tag == "noscript" => n += 1,
            Node::Text { text } if text.trim().is_empty() => n += 1,
            _ => break,
        }
    }
    let start = body.len() - n;
    (start, significant(&body[start..]))
}

/// HTML d'origine de nœuds lus dans une page (code ajouté d'une page).
pub(crate) fn to_html(nodes: &[Node]) -> String {
    const VOID: &[&str] = &[
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
        "track", "wbr",
    ];
    let mut out = String::new();
    for n in nodes {
        match n {
            Node::Text { text } => out.push_str(
                &text
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;"),
            ),
            Node::Html { html } => out.push_str(html),
            Node::Out { expr } => {
                out.push_str("{{ ");
                out.push_str(expr);
                out.push_str(" }}");
            }
            Node::Element {
                tag,
                attrs,
                children,
                ..
            } => {
                out.push('<');
                out.push_str(tag);
                for a in attrs {
                    out.push(' ');
                    out.push_str(&a.name);
                    if let Some(v) = &a.value {
                        let v = v
                            .replace("{{ '{{' }}", "{{")
                            .replace("{{ '{%' }}", "{%")
                            .replace("{{ '{#' }}", "{#");
                        out.push_str("=\"");
                        out.push_str(&v.replace('&', "&amp;").replace('"', "&quot;"));
                        out.push('"');
                    }
                }
                out.push('>');
                if !VOID.contains(&tag.to_ascii_lowercase().as_str()) {
                    out.push_str(&to_html(children));
                    out.push_str("</");
                    out.push_str(tag);
                    out.push('>');
                }
            }
            _ => {}
        }
    }
    out
}

/// Attributs de `<html>` ou `<body>` : identiques partout → gabarit ; sinon variable de page
/// `<prefixe>_<nom>` (valeur par page, `None` si absente).
#[allow(clippy::type_complexity)]
fn shared_attrs(
    lists: &[&[Attr]],
    prefix: &str,
) -> (Vec<Attr>, Vec<(String, Vec<Option<String>>)>) {
    let mut names: Vec<String> = Vec::new();
    for l in lists {
        for a in *l {
            if !names.contains(&a.name) {
                names.push(a.name.clone());
            }
        }
    }
    let mut attrs = Vec::new();
    let mut props = Vec::new();
    for name in names {
        let values: Vec<Option<String>> = lists
            .iter()
            .map(|l| {
                l.iter()
                    .find(|a| a.name == name)
                    .map(|a| a.value.clone().unwrap_or_default())
            })
            .collect();
        if values.iter().all(|v| *v == values[0]) {
            attrs.push(Attr {
                name: name.clone(),
                value: values[0].clone(),
                when: None,
            });
        } else {
            let prop = format!(
                "{prefix}_{}",
                name.chars()
                    .map(|c| if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '_'
                    })
                    .collect::<String>()
            );
            attrs.push(Attr {
                name,
                value: Some(format!("{{{{ {prop} }}}}")),
                when: Some(format!("{prop} is defined")),
            });
            props.push((prop, values));
        }
    }
    (attrs, props)
}

/// Plus grand groupe de pages commençant par le même élément (balise et attributs) ; à égalité,
/// celui de l'accueil.
fn largest_group(mains: &[Vec<Node>]) -> Vec<usize> {
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, m) in mains.iter().enumerate() {
        let Some(k) = m.iter().find_map(shallow) else {
            continue;
        };
        match groups.iter_mut().find(|(g, _)| *g == k) {
            Some((_, v)) => v.push(i),
            None => groups.push((k, vec![i])),
        }
    }
    groups
        .into_iter()
        .max_by(|a, b| {
            a.1.len()
                .cmp(&b.1.len())
                .then_with(|| b.1.first().cmp(&a.1.first()))
        })
        .map(|(_, v)| v)
        .unwrap_or_default()
}

/// Structure commune d'un groupe de pages : enveloppes (balise et attributs), puis nombre
/// d'éléments communs au début et à la fin.
struct Shell {
    wrappers: Vec<(String, Vec<Attr>)>,
    prefix: usize,
    suffix: usize,
}

/// Contenu sous les enveloppes communes.
fn unwrap_level(list: &[Node], depth: usize) -> Vec<Node> {
    let mut cur = significant(list);
    for _ in 0..depth {
        cur = match cur.as_slice() {
            [Node::Element { children, .. }] => significant(children),
            _ => return cur,
        };
    }
    cur
}

fn shell_of(lists: &[&[Node]]) -> Shell {
    let mut wrappers = Vec::new();
    let mut levels: Vec<Vec<Node>> = lists.iter().map(|l| significant(l)).collect();
    while let [Node::Element { tag, attrs, .. }] = levels[0].as_slice() {
        let first = (tag.clone(), attrs.clone());
        let same = levels.iter().all(|l| {
            matches!(l.as_slice(), [Node::Element { tag, attrs, .. }] if *tag == first.0 && *attrs == first.1)
        });
        if !same {
            break;
        }
        wrappers.push(first);
        levels = levels
            .into_iter()
            .map(|l| match l.into_iter().next() {
                Some(Node::Element { children, .. }) => significant(&children),
                _ => Vec::new(),
            })
            .collect();
    }
    let min = levels.iter().map(Vec::len).min().unwrap_or(0);
    let mut prefix = 0;
    while prefix < min
        && levels
            .iter()
            .all(|l| key(&l[prefix]) == key(&levels[0][prefix]))
    {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < min - prefix
        && levels
            .iter()
            .all(|l| key(&l[l.len() - 1 - suffix]) == key(&levels[0][levels[0].len() - 1 - suffix]))
    {
        suffix += 1;
    }
    Shell {
        wrappers,
        prefix,
        suffix,
    }
}

impl Shell {
    /// Début et fin communs, avec les marques « page en cours » écrites sous condition ; `None` :
    /// marques impossibles à exprimer par l'adresse de la page.
    fn shared(&self, lists: &[&[Node]], urls: &[&str]) -> Option<(Vec<Node>, Vec<Node>)> {
        let inner: Vec<Vec<Node>> = lists
            .iter()
            .map(|l| unwrap_level(l, self.wrappers.len()))
            .collect();
        let at = |k: usize| -> Option<Node> {
            let nodes: Vec<&Node> = inner.iter().map(|l| &l[k]).collect();
            with_current(&nodes, urls)
        };
        let before = (0..self.prefix).map(at).collect::<Option<Vec<_>>>()?;
        let after = (0..self.suffix)
            .map(|j| {
                let nodes: Vec<&Node> = inner
                    .iter()
                    .map(|l| &l[l.len() - self.suffix + j])
                    .collect();
                with_current(&nodes, urls)
            })
            .collect::<Option<Vec<_>>>()?;
        Some((before, after))
    }

    /// Corps du gabarit `site` : enveloppes, début commun, contenu de la page, fin commune.
    fn layout(&self, before: Vec<Node>, after: Vec<Node>) -> Vec<Node> {
        let mut nodes = before;
        nodes.push(Node::Slot);
        nodes.extend(after);
        let mut nodes = lines(nodes);
        for (tag, attrs) in self.wrappers.iter().rev() {
            nodes = lines(vec![element(tag, attrs.clone(), nodes)]);
        }
        nodes
    }

    /// Contenu propre à une page du groupe.
    fn middle(&self, list: &[Node]) -> Vec<Node> {
        let inner = unwrap_level(list, self.wrappers.len());
        inner[self.prefix..inner.len() - self.suffix].to_vec()
    }
}

/// Classes qui marquent le lien de la page en cours (Webflow, Bootstrap, WordPress…).
const CURRENT_CLASSES: &[&str] = &[
    "w--current",
    "current",
    "active",
    "is-active",
    "is-current",
    "selected",
    "current-menu-item",
    "current_page_item",
    "current-menu-ancestor",
    "current-page-ancestor",
    "current_page_parent",
];

/// Élément qui peut porter la marque « page en cours ».
fn marks_current(tag: &str) -> bool {
    matches!(tag, "a" | "li")
}

/// Nœuds sans les marques « page en cours » (`aria-current`, classes de `CURRENT_CLASSES`) des
/// liens et éléments de liste.
fn strip_current(nodes: &[Node]) -> Vec<Node> {
    nodes
        .iter()
        .map(|n| match n {
            Node::Element {
                id,
                tag,
                attrs,
                children,
            } => {
                let attrs = if marks_current(tag) {
                    attrs
                        .iter()
                        .filter(|a| a.name != "aria-current")
                        .filter_map(|a| {
                            if a.name != "class" {
                                return Some(a.clone());
                            }
                            let v = a.value.as_deref().unwrap_or("");
                            let kept: Vec<&str> = v
                                .split_whitespace()
                                .filter(|t| !CURRENT_CLASSES.contains(t))
                                .collect();
                            (!kept.is_empty() || v.trim().is_empty()).then(|| Attr {
                                name: a.name.clone(),
                                value: Some(kept.join(" ")),
                                when: None,
                            })
                        })
                        .collect()
                } else {
                    attrs.clone()
                };
                Node::Element {
                    id: id.clone(),
                    tag: tag.clone(),
                    attrs,
                    children: strip_current(children),
                }
            }
            n => n.clone(),
        })
        .collect()
}

/// Adresse visée par un lien (ou par le premier lien d'un élément de liste), sans `index.html`.
fn link_target(tag: &str, attrs: &[Attr], children: &[Node]) -> Option<String> {
    let href = if tag == "a" {
        attr(attrs, "href").map(str::to_string)
    } else {
        children.iter().find_map(|c| match c {
            Node::Element { tag, attrs, .. } if tag == "a" => {
                attr(attrs, "href").map(str::to_string)
            }
            Node::Element {
                children,
                tag,
                attrs,
                ..
            } => link_target(tag, attrs, children),
            _ => None,
        })
    }?;
    let h = href.split(['?', '#']).next().unwrap_or("").to_string();
    if !h.starts_with('/') {
        return None;
    }
    Some(match h.strip_suffix("index.html") {
        Some(dir) => dir.to_string(),
        None => h,
    })
}

/// Nœud commun à plusieurs pages (même structure sans les marques « page en cours ») écrit avec
/// ses marques sous condition : `page.url == "…"` (page visée) ou `page.url is startingwith("…")`
/// (rubrique) ; `None` si une marque ne suit aucune de ces règles.
fn with_current(nodes: &[&Node], urls: &[&str]) -> Option<Node> {
    let Node::Element {
        tag,
        attrs,
        children,
        ..
    } = nodes[0]
    else {
        return Some(nodes[0].clone());
    };
    let per: Vec<(&[Attr], &[Node])> = nodes
        .iter()
        .map(|n| match n {
            Node::Element {
                attrs, children, ..
            } => (attrs.as_slice(), children.as_slice()),
            _ => (&[][..], &[][..]),
        })
        .collect();
    let mut out_attrs = attrs.clone();
    if marks_current(tag) && per.iter().any(|(a, _)| *a != attrs.as_slice()) {
        let target = link_target(tag, attrs, children);
        // Règle d'une marque : pages où elle est présente.
        let rule = |present: &[bool]| -> Option<String> {
            if present.iter().all(|p| *p) || !present.iter().any(|p| *p) {
                return Some(String::new());
            }
            let on: Vec<&str> = urls
                .iter()
                .zip(present)
                .filter(|(_, p)| **p)
                .map(|(u, _)| *u)
                .collect();
            if on.len() == 1 && urls.iter().filter(|u| **u == on[0]).count() == 1 {
                return Some(format!("page.url == {:?}", on[0]));
            }
            let t = target.as_deref()?;
            let starts: Vec<bool> = urls.iter().map(|u| u.starts_with(t)).collect();
            (starts == present).then(|| format!("page.url is startingwith({t:?})"))
        };
        let aria: Vec<Option<String>> = per
            .iter()
            .map(|(a, _)| attr(a, "aria-current").map(str::to_string))
            .collect();
        let classes: Vec<Vec<String>> = per
            .iter()
            .map(|(a, _)| {
                attr(a, "class")
                    .unwrap_or("")
                    .split_whitespace()
                    .map(str::to_string)
                    .collect()
            })
            .collect();
        out_attrs = strip_current(std::slice::from_ref(nodes[0]))
            .into_iter()
            .next()
            .and_then(|n| match n {
                Node::Element { attrs, .. } => Some(attrs),
                _ => None,
            })
            .unwrap_or_default();
        // aria-current.
        if let Some(v) = aria.iter().flatten().next() {
            let present: Vec<bool> = aria.iter().map(Option::is_some).collect();
            let cond = rule(&present)?;
            out_attrs.push(Attr {
                name: "aria-current".into(),
                value: Some(v.clone()),
                when: (!cond.is_empty()).then_some(cond),
            });
        }
        // Classes : chaque marque à sa place, sous condition.
        let mut tokens: Vec<String> = Vec::new();
        for list in &classes {
            for t in list {
                if !tokens.contains(t) {
                    tokens.push(t.clone());
                }
            }
        }
        if !tokens.is_empty() {
            let mut value = String::new();
            for t in &tokens {
                let present: Vec<bool> = classes.iter().map(|l| l.contains(t)).collect();
                let cond = if CURRENT_CLASSES.contains(&t.as_str()) {
                    rule(&present)?
                } else if present.iter().all(|p| *p) {
                    String::new()
                } else {
                    return None;
                };
                let sep = if value.is_empty() { "" } else { " " };
                if cond.is_empty() {
                    value.push_str(sep);
                    value.push_str(t);
                } else {
                    value.push_str(&format!(
                        "{{{{ {:?} if {cond} else \"\" }}}}",
                        format!("{sep}{t}")
                    ));
                }
            }
            match out_attrs.iter_mut().find(|a| a.name == "class") {
                Some(a) => a.value = Some(value),
                None => out_attrs.push(Attr {
                    name: "class".into(),
                    value: Some(value),
                    when: None,
                }),
            }
        }
    } else if per.iter().any(|(a, _)| *a != attrs.as_slice()) {
        return None;
    }
    let mut out_children = Vec::with_capacity(children.len());
    for (k, c) in children.iter().enumerate() {
        let same: Vec<&Node> = per.iter().map(|(_, ch)| ch.get(k).unwrap_or(c)).collect();
        out_children.push(with_current(&same, urls)?);
    }
    Some(Node::Element {
        id: None,
        tag: tag.clone(),
        attrs: out_attrs,
        children: out_children,
    })
}

// -------------------------------------------------------------------------------------------------
// Feuilles de style

/// Feuilles locales liées par toutes les pages, sans `media` restrictif : fusionnées dans l'ordre
/// dans la feuille du site, à l'adresse de la première ; leurs liens retirés sauf le premier.
fn merge_stylesheets(
    root: &Path,
    head: &mut Vec<Node>,
    extra: &str,
    site: &mut Site,
    report: &mut Report,
) {
    let mut found: Vec<(usize, String, PathBuf)> = Vec::new();
    for (i, n) in head.iter().enumerate() {
        let Node::Element { tag, attrs, .. } = n else {
            continue;
        };
        if tag != "link"
            || !attr(attrs, "rel").is_some_and(|r| {
                r.split_whitespace()
                    .any(|t| t.eq_ignore_ascii_case("stylesheet"))
            })
            || attr(attrs, "media").is_some_and(|m| {
                let m = m.trim().to_ascii_lowercase();
                !(m.is_empty() || m == "all" || m == "screen")
            })
        {
            continue;
        }
        let Some(href) = attr(attrs, "href") else {
            continue;
        };
        let url = href.split(['?', '#']).next().unwrap_or("").to_string();
        if !url.starts_with('/') || url.starts_with("//") {
            continue;
        }
        let file = root.join(percent_decode(url.trim_start_matches('/')));
        if file.is_file() {
            found.push((i, url, file));
        }
    }
    if found.is_empty() && extra.trim().is_empty() {
        return;
    }
    let mut css = String::new();
    for (_, url, file) in &found {
        match std::fs::read_to_string(file) {
            Ok(text) => {
                let dir = match url.rfind('/') {
                    Some(i) => &url[..=i],
                    None => "/",
                };
                css.push_str(&rewrite_css_urls(text.trim_start_matches('\u{feff}'), dir));
                css.push('\n');
            }
            Err(e) => {
                report.warnings.push(format!(
                    "feuille de style « {url} » illisible ({e}) : copiée telle quelle"
                ));
                return;
            }
        }
    }
    css.push_str(extra);
    let items = match revel_style::parse(&css) {
        Ok(items) => items,
        Err(e) => {
            report.warnings.push(format!(
                "feuilles de style non reprises dans le panneau Styles ({e}) : copiées telles quelles"
            ));
            return;
        }
    };
    let (url, source) = match found.first() {
        Some((first, url, file)) => {
            if let Node::Element { attrs, .. } = &mut head[*first]
                && let Some(a) = attrs.iter_mut().find(|a| a.name == "href")
            {
                a.value = Some(url.clone());
            }
            for (i, _, _) in found.iter().skip(1).rev() {
                head.remove(*i);
            }
            let source = file
                .strip_prefix(root)
                .unwrap_or(file)
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            (url.clone(), Some(source))
        }
        None => {
            // Styles écrits dans les pages seulement : nouvelle feuille, liée à la fin de
            // l'en-tête commun.
            let mut url = "/assets/styles.css".to_string();
            let mut n = 2;
            while root.join(url.trim_start_matches('/')).exists() {
                url = format!("/assets/styles-{n}.css");
                n += 1;
            }
            head.push(element(
                "link",
                vec![
                    Attr {
                        name: "rel".into(),
                        value: Some("stylesheet".into()),
                        when: None,
                    },
                    Attr {
                        name: "href".into(),
                        value: Some(url.clone()),
                        when: None,
                    },
                ],
                Vec::new(),
            ));
            (url, None)
        }
    };
    site.breakpoints = revel_style::breakpoints_from(&items);
    report.style_rules = items
        .iter()
        .filter(|i| matches!(i, revel_model::StyleItem::Rule(_)))
        .count();
    site.stylesheet = Some(Stylesheet { url, source, items });
}

/// Blocs `<style>` de l'en-tête des pages (tous médias) retirés des pages et réunis : règles et
/// blocs `@…` de premier niveau dans l'ordre de première apparition, sans doublons. `None` si
/// aucun bloc ou si Revel ne lit pas ces styles (pages laissées telles quelles).
fn take_inline_styles(docs: &mut [Doc], report: &mut Report) -> Option<String> {
    let is_style = |n: &Node| matches!(n, Node::Element { tag, .. } if tag == "style");
    // Blocs dans l'ordre des pages ; un bloc répété ne garde que sa dernière place (celle qui
    // décide de la cascade : une règle reprise plus loin l'emporte sur celles d'entre-deux).
    let mut all: Vec<String> = Vec::new();
    for d in docs.iter() {
        let dir = match d.url.rfind('/') {
            Some(i) => &d.url[..=i],
            None => "/",
        };
        for n in d.head.iter().filter(|n| is_style(n)) {
            let Node::Element {
                attrs, children, ..
            } = n
            else {
                continue;
            };
            let text: String = children
                .iter()
                .map(|c| match c {
                    Node::Html { html } => html.as_str(),
                    Node::Text { text } => text.as_str(),
                    _ => "",
                })
                .collect();
            let blocks = top_level_blocks(&rewrite_css_urls(&text, dir));
            match attr(attrs, "media")
                .map(str::trim)
                .filter(|m| !m.is_empty() && !m.eq_ignore_ascii_case("all"))
            {
                // `<style media="…">` : ses règles dans un bloc `@media`.
                Some(m) if !blocks.is_empty() => {
                    all.push(format!("@media {m} {{\n{}\n}}", blocks.join("\n")));
                }
                _ => all.extend(blocks),
            }
        }
    }
    let mut seen = HashSet::new();
    let mut kept: Vec<&String> = all
        .iter()
        .rev()
        .filter(|b| seen.insert(b.as_str()))
        .collect();
    kept.reverse();
    let mut css = String::new();
    for b in kept {
        css.push_str(b);
        css.push('\n');
    }
    if css.is_empty() {
        return None;
    }
    if let Err(e) = revel_style::parse(&css) {
        report.warnings.push(format!(
            "styles écrits dans les pages non repris dans le panneau Styles ({e}) : gardés dans les pages"
        ));
        return None;
    }
    for d in docs.iter_mut() {
        d.head.retain(|n| !is_style(n));
    }
    Some(css)
}

/// Règles et blocs `@…` de premier niveau d'un texte CSS (commentaires retirés).
fn top_level_blocks(css: &str) -> Vec<String> {
    let b = css.as_bytes();
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut depth, mut i) = (0usize, 0usize);
    let mut quote: Option<u8> = None;
    while i < b.len() {
        let c = b[i];
        if let Some(q) = quote {
            if c == b'\\' && i + 1 < b.len() {
                cur.push_str(&css[i..i + 2]);
                i += 2;
                continue;
            }
            if c == q {
                quote = None;
            }
        } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i = css[i + 2..].find("*/").map_or(b.len(), |k| i + 2 + k + 2);
            continue;
        } else if c == b'"' || c == b'\'' {
            quote = Some(c);
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                cur.push('}');
                out.push(cur.trim().to_string());
                cur.clear();
                i += 1;
                continue;
            }
        } else if c == b';' && depth == 0 {
            // `@import …;`, `@charset …;`
            cur.push(';');
            let t = cur.trim().to_string();
            if !t.starts_with("@charset") {
                out.push(t);
            }
            cur.clear();
            i += 1;
            continue;
        }
        let ch = css[i..].chars().next().unwrap_or(' ');
        cur.push(ch);
        i += ch.len_utf8();
    }
    out.retain(|s| !s.is_empty() && s != "}");
    out
}

/// Données des interactions de Webflow, lues dans les scripts locaux des pages (`webflow.js`).
fn webflow_interactions(root: &Path, docs: &[Doc]) -> Option<Value> {
    fn scripts<'a>(nodes: &'a [Node], out: &mut Vec<&'a str>) {
        for n in nodes {
            if let Node::Element {
                tag,
                attrs,
                children,
                ..
            } = n
            {
                if tag == "script"
                    && let Some(src) = attr(attrs, "src")
                {
                    out.push(src);
                }
                scripts(children, out);
            }
        }
    }
    let mut srcs = Vec::new();
    for d in docs.iter().take(3) {
        scripts(&d.head, &mut srcs);
        scripts(&d.body, &mut srcs);
    }
    let mut done = HashSet::new();
    for src in srcs {
        let url = src.split(['?', '#']).next().unwrap_or("");
        if !url.starts_with('/') || url.starts_with("//") || !done.insert(url) {
            continue;
        }
        let file = root.join(percent_decode(url.trim_start_matches('/')));
        if std::fs::metadata(&file).is_ok_and(|m| m.is_file() && m.len() < 20_000_000)
            && let Ok(js) = std::fs::read_to_string(&file)
            && let Some(cfg) = adapt::ix2_config(&js)
        {
            return Some(cfg);
        }
    }
    None
}

/// `%20` et autres séquences d'une adresse décodées (nom de fichier).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|d| d as u8)
}

// -------------------------------------------------------------------------------------------------
// Site

/// Nom du site : `og:site_name` de l'accueil.
fn site_name(docs: &[Doc]) -> Option<String> {
    docs[0].head.iter().find_map(|n| match n {
        Node::Element { tag, attrs, .. }
            if tag == "meta" && attr(attrs, "property") == Some("og:site_name") =>
        {
            attr(attrs, "content")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        }
        _ => None,
    })
}

/// Adresse publique : celle de l'adresse officielle (`canonical`) ou de `og:url` de l'accueil.
fn base_url(docs: &[Doc]) -> Option<String> {
    docs[0].head.iter().find_map(|n| {
        let Node::Element { tag, attrs, .. } = n else {
            return None;
        };
        let u = match tag.as_str() {
            "link" if attr(attrs, "rel").is_some_and(|r| r.eq_ignore_ascii_case("canonical")) => {
                attr(attrs, "href")
            }
            "meta" if attr(attrs, "property") == Some("og:url") => attr(attrs, "content"),
            _ => None,
        }?;
        let (scheme, rest) = u.split_once("://")?;
        if scheme != "https" && scheme != "http" {
            return None;
        }
        let host = rest.split('/').next()?;
        (!host.is_empty()).then(|| format!("{scheme}://{host}"))
    })
}

/// Contenu du dossier copié tel quel : dossiers et fichiers de premier niveau, sauf les pages de
/// la racine, `robots.txt` (repris dans les réglages) et les fichiers de dépôt.
fn passthrough(root: &Path, pages: &[String]) -> Vec<Passthrough> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(root) else {
        return out;
    };
    let mut names: Vec<(String, bool)> = rd
        .flatten()
        .map(|e| {
            (
                e.file_name().to_string_lossy().to_string(),
                e.path().is_dir(),
            )
        })
        .collect();
    names.sort();
    for (name, dir) in names {
        let keep_hidden = matches!(name.as_str(), ".htaccess" | ".well-known" | ".nojekyll");
        if (name.starts_with('.') && !keep_hidden)
            || SKIP_DIRS.contains(&name.as_str())
            || (!dir && (pages.contains(&name) || repo_file(&name) || name == "robots.txt"))
        {
            continue;
        }
        out.push(Passthrough {
            from: name.clone(),
            to: name,
        });
    }
    out
}

// -------------------------------------------------------------------------------------------------
// Comparaison (fidélité de l'import)

/// Différences de structure entre une page d'origine et la page produite par Revel (au plus
/// `limit`), lues comme un navigateur : blancs entre éléments et paramètres d'adresse des
/// feuilles de style ignorés, adresses relatives lues depuis la racine, ordre de l'en-tête
/// indifférent ; feuilles de style et blocs `<style>` comptés avec l'en-tête où qu'ils soient.
pub fn compare(original: &str, produced: &str, rel: &str, limit: usize) -> Vec<String> {
    let mut report = Report::default();
    let a = normalized(read_page(rel, original, &mut report));
    let b = normalized(read_page(rel, produced, &mut report));
    let mut out = Vec::new();
    if a.doctype.as_deref().map(str::to_ascii_lowercase)
        != b.doctype.as_deref().map(str::to_ascii_lowercase)
    {
        out.push(format!("doctype : {:?} ≠ {:?}", a.doctype, b.doctype));
    }
    if a.title != b.title {
        out.push(format!("titre : {:?} ≠ {:?}", a.title, b.title));
    }
    if a.description != b.description {
        out.push(format!(
            "description : {:?} ≠ {:?}",
            a.description, b.description
        ));
    }
    let attrs = |l: &[Attr]| {
        let mut v: Vec<String> = l
            .iter()
            .map(|a| format!("{}={:?}", a.name, a.value))
            .collect();
        v.sort();
        v
    };
    if attrs(&a.html_attrs) != attrs(&b.html_attrs) {
        out.push(format!(
            "<html> : {:?} ≠ {:?}",
            attrs(&a.html_attrs),
            attrs(&b.html_attrs)
        ));
    }
    if attrs(&a.body_attrs) != attrs(&b.body_attrs) {
        out.push(format!(
            "<body> : {:?} ≠ {:?}",
            attrs(&a.body_attrs),
            attrs(&b.body_attrs)
        ));
    }
    let mut ha: Vec<String> = a.head.iter().map(key).collect();
    let mut hb: Vec<String> = b.head.iter().map(key).collect();
    ha.sort();
    hb.sort();
    for k in &ha {
        if !hb.contains(k) {
            out.push(format!(
                "en-tête : absent de la page produite : {}",
                short(k)
            ));
        }
    }
    for k in &hb {
        if !ha.contains(k) {
            out.push(format!(
                "en-tête : ajouté dans la page produite : {}",
                short(k)
            ));
        }
    }
    diff_nodes(&a.body, &b.body, "body", &mut out, limit);
    out.truncate(limit);
    out
}

fn short(s: &str) -> String {
    let s: String = s.chars().take(160).collect();
    s
}

/// Page ramenée à sa forme comparable.
fn normalized(mut d: Doc) -> Doc {
    // Feuilles de style et blocs <style> du corps (premier niveau) : comptés avec l'en-tête.
    let mut moved = Vec::new();
    d.body.retain(|n| {
        if is_visual_head(n) {
            moved.push(n.clone());
            false
        } else {
            true
        }
    });
    d.head.extend(moved);
    d.head = d
        .head
        .into_iter()
        .map(|n| norm(n, false))
        .filter(|n| !is_blank(n))
        .collect();
    d.body = d
        .body
        .into_iter()
        .map(|n| norm(n, false))
        .filter(|n| !is_blank(n))
        .collect();
    d
}

/// Blancs réduits, blancs entre éléments retirés, paramètres d'adresse des feuilles retirés.
fn norm(n: Node, pre: bool) -> Node {
    match n {
        Node::Text { text } if !pre => Node::Text {
            text: text.split_whitespace().collect::<Vec<_>>().join(" "),
        },
        Node::Element {
            id,
            tag,
            mut attrs,
            children,
        } => {
            let pre = pre || matches!(tag.as_str(), "pre" | "textarea");
            if tag == "link" {
                for a in attrs.iter_mut().filter(|a| a.name == "href") {
                    if let Some(v) = &mut a.value
                        && let Some(i) = v.find('?')
                    {
                        v.truncate(i);
                    }
                }
            }
            attrs.sort_by(|x, y| x.name.cmp(&y.name));
            let children = children
                .into_iter()
                .map(|c| norm(c, pre))
                .filter(|c| pre || !matches!(c, Node::Text { text } if text.is_empty()))
                .collect();
            Node::Element {
                id,
                tag,
                attrs,
                children,
            }
        }
        n => n,
    }
}

fn label(n: &Node) -> String {
    match n {
        Node::Element { tag, attrs, .. } => {
            let cls = attr(attrs, "class")
                .map(|c| format!(".{}", c.split_whitespace().collect::<Vec<_>>().join(".")))
                .unwrap_or_default();
            format!("<{tag}{cls}>")
        }
        Node::Text { text } => format!("texte {:?}", text.chars().take(60).collect::<String>()),
        Node::Html { html } => format!("{:?}", html.chars().take(60).collect::<String>()),
        _ => "?".into(),
    }
}

fn diff_nodes(a: &[Node], b: &[Node], path: &str, out: &mut Vec<String>, limit: usize) {
    if out.len() >= limit {
        return;
    }
    if a.len() != b.len() {
        let first = a
            .iter()
            .zip(b)
            .position(|(x, y)| key(x) != key(y))
            .unwrap_or(a.len().min(b.len()));
        out.push(format!(
            "{path} : {} enfants ≠ {} ; premier écart : {} / {}",
            a.len(),
            b.len(),
            a.get(first).map(label).unwrap_or_else(|| "rien".into()),
            b.get(first).map(label).unwrap_or_else(|| "rien".into()),
        ));
        return;
    }
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        if key(x) == key(y) {
            continue;
        }
        match (x, y) {
            (
                Node::Element {
                    tag: ta,
                    attrs: aa,
                    children: ca,
                    ..
                },
                Node::Element {
                    tag: tb,
                    attrs: ab,
                    children: cb,
                    ..
                },
            ) if ta == tb => {
                let here = format!("{path} > {}[{i}]", label(x));
                if aa != ab {
                    out.push(format!(
                        "{here} : attributs {:?} ≠ {:?}",
                        aa.iter()
                            .map(|a| format!("{}={}", a.name, a.value.as_deref().unwrap_or("")))
                            .collect::<Vec<_>>(),
                        ab.iter()
                            .map(|a| format!("{}={}", a.name, a.value.as_deref().unwrap_or("")))
                            .collect::<Vec<_>>()
                    ));
                }
                diff_nodes(ca, cb, &here, out, limit);
            }
            _ => out.push(format!("{path}[{i}] : {} ≠ {}", label(x), label(y))),
        }
        if out.len() >= limit {
            return;
        }
    }
}
