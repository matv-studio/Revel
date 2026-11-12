//! Flux reliés à une collection (D-167) : lecture d'un flux RSS 2.0 (ou Atom 1.0) d'un autre
//! site et contenus de la collection tirés de ses articles. Chaque article devient un contenu
//! (fichier Markdown) reconnu par son identifiant (`guid`) : un article déjà repris est mis à
//! jour (champs du flux remplacés, autres champs gardés), un article nouveau est ajouté ; un
//! contenu dont l'article a quitté le flux reste (un flux ne garde que ses derniers articles).
//!
//! Champs écrits : `title`, `date` (AAAA-MM-JJ), `summary` (texte, 300 caractères au plus),
//! `image` (pièce jointe image, `media:content`, `media:thumbnail`, sinon première image du
//! texte), `tags` (catégories), `author`, `link` (article d'origine), `guid`, `source` (origine
//! donnée au flux) ; texte du contenu : `content:encoded` (sinon la description), en HTML.
//!
//! Plusieurs flux peuvent remplir une collection, repris dans leur ordre : un article dont le
//! titre est déjà celui d'un autre contenu (publié par deux sites) n'est repris qu'une fois, la
//! première version gardée.
//!
//! Spécifications : RSS 2.0 (https://www.rssboard.org/rss-specification), module de contenu
//! (http://purl.org/rss/1.0/modules/content/), Media RSS (http://search.yahoo.com/mrss/),
//! Dublin Core (`dc:creator`), Atom 1.0 (RFC 4287).

use crate::crawl::Fetch;
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use url::Url;

const NS_CONTENT: &str = "http://purl.org/rss/1.0/modules/content/";
const NS_MEDIA: &str = "http://search.yahoo.com/mrss/";
const NS_DC: &str = "http://purl.org/dc/elements/1.1/";
const NS_ATOM: &str = "http://www.w3.org/2005/Atom";

/// Article d'un flux.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeedItem {
    pub guid: String,
    pub title: String,
    pub link: String,
    /// `AAAA-MM-JJ`, vide si le flux n'en donne pas.
    pub date: String,
    pub summary: String,
    /// Texte de l'article, en HTML.
    pub content: String,
    pub image: String,
    pub tags: Vec<String>,
    pub author: String,
}

/// Flux lu : titre et articles, dans l'ordre du flux.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Feed {
    pub title: String,
    pub items: Vec<FeedItem>,
}

/// Contenu existant de la collection : nom du fichier et en-tête.
#[derive(Debug, Clone)]
pub struct Existing {
    pub file: String,
    pub data: Value,
    pub body: String,
}

/// Ce que la synchronisation écrit : contenus nouveaux et contenus modifiés.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    /// (fichier, en-tête, texte).
    pub create: Vec<(String, Value, String)>,
    /// (fichier, en-tête fusionné, texte).
    pub update: Vec<(String, Value, String)>,
    /// Articles déjà à jour.
    pub unchanged: usize,
    /// Articles déjà repris depuis un autre flux (même titre).
    pub duplicates: usize,
}

/// Titre comparé pour les doublons : lettres et chiffres, en minuscules.
fn same_title(t: &str) -> String {
    t.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Lit un flux RSS 2.0 ou Atom 1.0 ; adresses relatives résolues depuis `base`.
pub fn parse(xml: &str, base: Option<&Url>) -> Result<Feed, String> {
    let doc = roxmltree::Document::parse_with_options(
        xml.trim_start_matches('\u{feff}'),
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    let root = doc.root_element();
    let abs = |u: &str| -> String {
        let u = u.trim();
        if u.is_empty() {
            return String::new();
        }
        match base.and_then(|b| b.join(u).ok()) {
            Some(x) => x.to_string(),
            None => u.to_string(),
        }
    };
    let mut feed = Feed::default();
    match root.tag_name().name() {
        "rss" | "RDF" => {
            let channel = root
                .children()
                .find(|n| n.has_tag_name("channel"))
                .ok_or("flux sans <channel>")?;
            feed.title = text_of(channel, "title");
            // RSS 1.0 (RDF) : articles à côté du canal ; RSS 2.0 : dans le canal.
            let items: Vec<_> = if root.tag_name().name() == "RDF" {
                root.children()
                    .filter(|n| n.tag_name().name() == "item")
                    .collect()
            } else {
                channel
                    .children()
                    .filter(|n| n.has_tag_name("item"))
                    .collect()
            };
            for it in items {
                feed.items.push(rss_item(it, &abs));
            }
        }
        "feed" if root.tag_name().namespace() == Some(NS_ATOM) => {
            feed.title = text_of(root, "title");
            for e in root
                .children()
                .filter(|n| n.has_tag_name((NS_ATOM, "entry")))
            {
                feed.items.push(atom_entry(e, &abs));
            }
        }
        other => {
            return Err(format!(
                "ce document n'est pas un flux RSS ou Atom (<{other}>)"
            ));
        }
    }
    Ok(feed)
}

fn rss_item(it: roxmltree::Node, abs: &dyn Fn(&str) -> String) -> FeedItem {
    let mut item = FeedItem {
        title: clean_text(&text_of(it, "title")),
        link: abs(&text_of(it, "link")),
        ..Default::default()
    };
    let desc = text_of(it, "description");
    let content = it
        .children()
        .find(|n| n.has_tag_name((NS_CONTENT, "encoded")))
        .map(|n| all_text(n))
        .unwrap_or_default();
    item.content = if content.trim().is_empty() {
        desc.clone()
    } else {
        content
    };
    item.summary = summary(if desc.trim().is_empty() {
        &item.content
    } else {
        &desc
    });
    item.guid = text_of(it, "guid");
    if item.guid.is_empty() {
        item.guid = item.link.clone();
    }
    if item.guid.is_empty() {
        item.guid = format!("{}|{}", item.title, text_of(it, "pubDate"));
    }
    item.date = rfc822_date(&text_of(it, "pubDate"))
        .or_else(|| {
            it.children()
                .find(|n| n.has_tag_name((NS_DC, "date")))
                .and_then(|n| iso_date(&all_text(n)))
        })
        .unwrap_or_default();
    item.author = it
        .children()
        .find(|n| n.has_tag_name((NS_DC, "creator")))
        .map(|n| all_text(n).trim().to_string())
        .unwrap_or_else(|| author_name(&text_of(it, "author")));
    item.tags = it
        .children()
        .filter(|n| n.has_tag_name("category"))
        .map(|n| clean_text(&all_text(n)))
        .filter(|t| !t.is_empty())
        .collect();
    item.image = rss_image(it, abs).unwrap_or_else(|| first_img(&item.content, abs));
    item
}

/// Image d'un article : pièce jointe image, `media:content` image, `media:thumbnail`.
fn rss_image(it: roxmltree::Node, abs: &dyn Fn(&str) -> String) -> Option<String> {
    let is_img = |n: &roxmltree::Node| {
        let t = n.attribute("type").unwrap_or("");
        let m = n.attribute("medium").unwrap_or("");
        let u = n.attribute("url").unwrap_or("").to_ascii_lowercase();
        t.starts_with("image/")
            || m == "image"
            || [".jpg", ".jpeg", ".png", ".webp", ".avif", ".gif"]
                .iter()
                .any(|e| u.split(['?', '#']).next().unwrap_or("").ends_with(e))
    };
    let pick = |n: roxmltree::Node| n.attribute("url").map(abs);
    it.children()
        .filter(|n| n.has_tag_name("enclosure") && is_img(n))
        .find_map(pick)
        .or_else(|| {
            it.descendants()
                .filter(|n| n.has_tag_name((NS_MEDIA, "content")) && is_img(n))
                .find_map(pick)
        })
        .or_else(|| {
            it.descendants()
                .filter(|n| n.has_tag_name((NS_MEDIA, "thumbnail")))
                .find_map(pick)
        })
}

fn atom_entry(e: roxmltree::Node, abs: &dyn Fn(&str) -> String) -> FeedItem {
    let child = |name: &str| e.children().find(|n| n.has_tag_name((NS_ATOM, name)));
    let text = |name: &str| child(name).map(|n| atom_text(n)).unwrap_or_default();
    let link = e
        .children()
        .filter(|n| n.has_tag_name((NS_ATOM, "link")))
        .find(|n| matches!(n.attribute("rel"), None | Some("alternate")))
        .and_then(|n| n.attribute("href"))
        .map(abs)
        .unwrap_or_default();
    let content = text("content");
    let summ = text("summary");
    let mut item = FeedItem {
        guid: text("id"),
        title: clean_text(&text("title")),
        link,
        date: iso_date(&text("published"))
            .or_else(|| iso_date(&text("updated")))
            .unwrap_or_default(),
        content: if content.trim().is_empty() {
            summ.clone()
        } else {
            content
        },
        author: child("author")
            .and_then(|a| a.children().find(|n| n.has_tag_name((NS_ATOM, "name"))))
            .map(|n| all_text(n).trim().to_string())
            .unwrap_or_default(),
        tags: e
            .children()
            .filter(|n| n.has_tag_name((NS_ATOM, "category")))
            .filter_map(|n| n.attribute("label").or(n.attribute("term")))
            .map(clean_text)
            .filter(|t| !t.is_empty())
            .collect(),
        ..Default::default()
    };
    if item.guid.is_empty() {
        item.guid = item.link.clone();
    }
    item.summary = summary(if summ.trim().is_empty() {
        &item.content
    } else {
        &summ
    });
    item.image = e
        .children()
        .filter(|n| n.has_tag_name((NS_ATOM, "link")) && n.attribute("rel") == Some("enclosure"))
        .filter(|n| n.attribute("type").unwrap_or("").starts_with("image/"))
        .find_map(|n| n.attribute("href").map(abs))
        .or_else(|| {
            e.descendants()
                .filter(|n| {
                    n.has_tag_name((NS_MEDIA, "thumbnail")) || n.has_tag_name((NS_MEDIA, "content"))
                })
                .find_map(|n| n.attribute("url").map(abs))
        })
        .unwrap_or_else(|| first_img(&item.content, abs));
    item
}

/// Texte d'un élément Atom : `type="xhtml"` : le balisage de son `<div>` ; sinon le texte.
fn atom_text(n: roxmltree::Node) -> String {
    if n.attribute("type") == Some("xhtml") {
        let doc = n.document().input_text();
        return n
            .children()
            .find(|c| c.is_element())
            .map(|d| {
                let r = d.range();
                let inner = &doc[r];
                // Contenu du <div> seulement.
                let start = inner.find('>').map(|i| i + 1).unwrap_or(0);
                let end = inner.rfind("</").unwrap_or(inner.len());
                inner[start..end.max(start)].to_string()
            })
            .unwrap_or_default();
    }
    all_text(n)
}

fn text_of(n: roxmltree::Node, name: &str) -> String {
    n.children()
        .find(|c| {
            c.is_element() && c.tag_name().name() == name && c.tag_name().namespace().is_none()
        })
        .map(|c| all_text(c))
        .unwrap_or_default()
}

/// Texte d'un élément (CDATA compris).
fn all_text(n: roxmltree::Node) -> String {
    n.descendants()
        .filter(|d| d.is_text())
        .filter_map(|d| d.text())
        .collect::<String>()
}

/// « nom@exemple.fr (Nom) » -> « Nom » ; adresse seule : retirée.
fn author_name(s: &str) -> String {
    let s = s.trim();
    if let (Some(a), Some(b)) = (s.find('('), s.rfind(')'))
        && a < b
    {
        return s[a + 1..b].trim().to_string();
    }
    if s.contains('@') {
        String::new()
    } else {
        s.to_string()
    }
}

/// Texte sans balises ni espaces en trop, entités courantes rendues.
pub fn clean_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut tag = false;
    for c in html.chars() {
        match c {
            '<' => {
                tag = true;
                out.push(' ');
            }
            '>' if tag => tag = false,
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    let out = out
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#8217;", "’")
        .replace("&rsquo;", "’")
        .replace("&hellip;", "…")
        .replace("&#8230;", "…");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Résumé : texte de la description, 300 caractères au plus (coupé à un mot).
fn summary(html: &str) -> String {
    let t = clean_text(html);
    if t.chars().count() <= 300 {
        return t;
    }
    let cut: String = t.chars().take(300).collect();
    let cut = cut.rsplit_once(' ').map(|(a, _)| a).unwrap_or(&cut);
    format!("{}…", cut.trim_end_matches([',', ';', ':', '.', ' ']))
}

fn first_img(html: &str, abs: &dyn Fn(&str) -> String) -> String {
    let lower = html.to_ascii_lowercase();
    let Some(i) = lower.find("<img") else {
        return String::new();
    };
    let rest = &html[i..];
    let end = rest.find('>').unwrap_or(rest.len());
    let tag = &rest[..end];
    let lt = tag.to_ascii_lowercase();
    let Some(j) = lt.find("src=") else {
        return String::new();
    };
    let v = &tag[j + 4..];
    let (q, v) = match v.chars().next() {
        Some(c @ ('"' | '\'')) => (c, &v[1..]),
        _ => (' ', v),
    };
    let src = v.split(q).next().unwrap_or("").trim();
    if src.starts_with("data:") {
        return String::new();
    }
    abs(&src.replace("&amp;", "&"))
}

/// Date RFC 822 (RSS) -> `AAAA-MM-JJ`.
fn rfc822_date(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    chrono::DateTime::parse_from_rfc2822(s)
        .ok()
        .map(|d| d.format("%Y-%m-%d").to_string())
        .or_else(|| iso_date(s))
}

/// Date ISO 8601 (Atom, `dc:date`) -> `AAAA-MM-JJ`.
fn iso_date(s: &str) -> Option<String> {
    let s = s.trim();
    let d = s.get(..10)?;
    let ok = d.len() == 10
        && d.as_bytes()[4] == b'-'
        && d.as_bytes()[7] == b'-'
        && d.bytes()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit());
    ok.then(|| d.to_string())
}

/// En-tête d'un contenu tiré d'un article (champs vides omis).
pub fn item_data(it: &FeedItem, source: &str) -> Map<String, Value> {
    let mut m = Map::new();
    let mut put = |k: &str, v: &str| {
        if !v.is_empty() {
            m.insert(k.into(), json!(v));
        }
    };
    put("title", &it.title);
    put("date", &it.date);
    put("summary", &it.summary);
    put("image", &it.image);
    put("author", &it.author);
    put("link", &it.link);
    put("guid", &it.guid);
    put("source", source);
    if !it.tags.is_empty() {
        m.insert("tags".into(), json!(it.tags));
    }
    m
}

/// Champs que le flux écrit (retirés d'un contenu existant quand l'article ne les donne plus).
pub const FIELDS: [&str; 9] = [
    "title", "date", "summary", "image", "author", "link", "guid", "source", "tags",
];

/// Contenus à créer ou mettre à jour pour que la collection reprenne les articles du flux
/// (`source` : origine écrite dans les contenus). `existing` : contenus de la collection, y
/// compris ceux qu'un flux précédent de la même reprise va créer.
pub fn plan(feed: &Feed, existing: &[Existing], source: &str) -> Plan {
    let mut out = Plan::default();
    let mut taken: HashSet<String> = existing.iter().map(|e| e.file.clone()).collect();
    let mut seen: HashSet<&str> = HashSet::new();
    let mut titles: HashSet<String> = existing
        .iter()
        .filter_map(|e| e.data.get("title").and_then(Value::as_str))
        .map(same_title)
        .filter(|t| !t.is_empty())
        .collect();
    for it in &feed.items {
        if it.guid.is_empty() || !seen.insert(it.guid.as_str()) {
            continue;
        }
        let fresh = item_data(it, source);
        let body = it.content.trim().to_string();
        let found = existing
            .iter()
            .find(|e| e.data.get("guid").and_then(Value::as_str) == Some(it.guid.as_str()));
        match found {
            Some(e) => {
                let mut data = e.data.as_object().cloned().unwrap_or_default();
                for k in FIELDS {
                    match fresh.get(k) {
                        Some(v) => {
                            data.insert(k.into(), v.clone());
                        }
                        None => {
                            data.remove(k);
                        }
                    }
                }
                let data = Value::Object(data);
                if data == e.data && body == e.body.trim() {
                    out.unchanged += 1;
                } else {
                    out.update.push((e.file.clone(), data, body));
                }
            }
            None => {
                let t = same_title(&it.title);
                if !t.is_empty() && !titles.insert(t) {
                    out.duplicates += 1;
                    continue;
                }
                let file = unique_stem(&stem_of(it), &mut taken);
                out.create.push((file, Value::Object(fresh), body));
            }
        }
    }
    out
}

/// Nom de fichier d'un article : titre, sinon fin de son adresse, sinon `article`.
fn stem_of(it: &FeedItem) -> String {
    let s = revel_model::slugify(&it.title);
    if !s.is_empty() {
        return s
            .chars()
            .take(80)
            .collect::<String>()
            .trim_end_matches('-')
            .to_string();
    }
    let tail = it
        .link
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .split(['?', '#', '.'])
        .next()
        .unwrap_or("");
    let s = revel_model::slugify(tail);
    if s.is_empty() { "article".into() } else { s }
}

fn unique_stem(base: &str, taken: &mut HashSet<String>) -> String {
    let mut name = base.to_string();
    let mut n = 2;
    while taken.contains(&name) {
        name = format!("{base}-{n}");
        n += 1;
    }
    taken.insert(name.clone());
    name
}

/// Lit le flux à une adresse (pages HTML refusées : l'adresse doit être celle du flux).
pub fn fetch(fetch: &dyn Fetch, url: &str) -> Result<Feed, String> {
    let u = Url::parse(url.trim()).map_err(|e| format!("{url} : {e}"))?;
    if !matches!(u.scheme(), "http" | "https") {
        return Err(format!("{url} : adresse http ou https attendue"));
    }
    let r = fetch.get_with(
        &u,
        &[(
            "Accept",
            "application/rss+xml, application/atom+xml, application/xml;q=0.9, text/xml;q=0.8, */*;q=0.1",
        )],
    )?;
    if !(200..300).contains(&r.status) {
        return Err(format!("{url} : réponse {}", r.status));
    }
    let text = String::from_utf8_lossy(&r.body);
    parse(&text, Some(&r.url))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:media="http://search.yahoo.com/mrss/">
<channel><title>Studio Nord</title><link>https://nord.example/</link><description>Projets</description>
<item><title>Maison &amp; jardin</title><link>/projets/maison/</link>
<guid isPermaLink="false">p-12</guid><pubDate>Tue, 03 Nov 2026 09:30:00 +0100</pubDate>
<dc:creator>Léa Martin</dc:creator><category>Architecture</category><category>Paysage</category>
<description><![CDATA[<p>Une maison <b>ouverte</b> sur le jardin.</p>]]></description>
<content:encoded><![CDATA[<p>Texte complet.</p><img src="/img/maison.jpg" alt="">]]></content:encoded>
<media:content url="https://cdn.example/maison-large.jpg" medium="image"/></item>
<item><title>Note sans image</title><link>https://nord.example/notes/1/</link>
<description>Courte note.</description><author>contact@nord.example (Paul)</author></item>
</channel></rss>"#;

    #[test]
    fn lit_un_flux_rss() {
        let base = Url::parse("https://nord.example/feed.xml").unwrap();
        let f = parse(RSS, Some(&base)).unwrap();
        assert_eq!(f.title, "Studio Nord");
        assert_eq!(f.items.len(), 2);
        let a = &f.items[0];
        assert_eq!(a.title, "Maison & jardin");
        assert_eq!(a.link, "https://nord.example/projets/maison/");
        assert_eq!(a.guid, "p-12");
        assert_eq!(a.date, "2026-11-03");
        assert_eq!(a.author, "Léa Martin");
        assert_eq!(a.tags, ["Architecture", "Paysage"]);
        assert_eq!(a.summary, "Une maison ouverte sur le jardin.");
        assert!(a.content.contains("Texte complet."));
        assert_eq!(a.image, "https://cdn.example/maison-large.jpg");
        let b = &f.items[1];
        assert_eq!(b.guid, "https://nord.example/notes/1/");
        assert_eq!(b.author, "Paul");
        assert_eq!(b.image, "");
        assert_eq!(b.content, "Courte note.");
    }

    #[test]
    fn lit_un_flux_atom() {
        let xml = r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>Blog</title>
<entry><title>Premier</title><id>urn:1</id><link rel="alternate" href="https://b.example/1/"/>
<published>2026-10-01T10:00:00Z</published><category term="news"/>
<author><name>Ana</name></author><summary>Résumé</summary>
<content type="html">&lt;p&gt;Corps &lt;img src="https://b.example/i.png"&gt;&lt;/p&gt;</content></entry></feed>"#;
        let f = parse(xml, None).unwrap();
        let e = &f.items[0];
        assert_eq!(
            (e.guid.as_str(), e.date.as_str(), e.author.as_str()),
            ("urn:1", "2026-10-01", "Ana")
        );
        assert_eq!(e.tags, ["news"]);
        assert_eq!(e.summary, "Résumé");
        assert_eq!(e.image, "https://b.example/i.png");
    }

    #[test]
    fn refuse_une_page_html() {
        assert!(parse("<html><body>x</body></html>", None).is_err());
    }

    #[test]
    fn plan_ajoute_puis_met_a_jour() {
        let f = parse(RSS, None).unwrap();
        let p = plan(&f, &[], "nord");
        assert_eq!(p.create.len(), 2);
        assert_eq!(p.create[0].0, "maison-jardin");
        assert_eq!(p.create[0].1["guid"], "p-12");
        assert_eq!(p.create[0].1["source"], "nord");
        // Contenu repris, puis modifié par l'auteur (champ à lui) : ses champs restent.
        let mut data = p.create[0].1.clone();
        data["featured"] = json!(true);
        data["title"] = json!("Ancien titre");
        let existing = vec![
            Existing {
                file: "maison-jardin".into(),
                data,
                body: p.create[0].2.clone(),
            },
            Existing {
                file: "note-sans-image".into(),
                data: p.create[1].1.clone(),
                body: p.create[1].2.clone(),
            },
        ];
        let p2 = plan(&f, &existing, "nord");
        assert!(p2.create.is_empty());
        assert_eq!(p2.unchanged, 1);
        assert_eq!(p2.update.len(), 1);
        assert_eq!(p2.update[0].1["title"], "Maison & jardin");
        assert_eq!(p2.update[0].1["featured"], true);
    }

    #[test]
    fn noms_de_fichiers_uniques() {
        let mut f = parse(RSS, None).unwrap();
        f.items[1].title = "Maison et jardin".into();
        let existing = vec![Existing {
            file: "maison-jardin".into(),
            data: json!({}),
            body: String::new(),
        }];
        let p = plan(&f, &existing, "");
        let names: Vec<_> = p.create.iter().map(|c| c.0.as_str()).collect();
        assert_eq!(names, ["maison-jardin-2", "maison-et-jardin"]);
        assert!(p.create[0].1.get("source").is_none());
    }

    #[test]
    fn doublon_d_un_autre_flux_ignore() {
        let f = parse(RSS, None).unwrap();
        // Même article déjà repris d'un autre site (autre identifiant, même titre).
        let existing = vec![Existing {
            file: "maison-jardin".into(),
            data: json!({ "title": "Maison & Jardin !", "guid": "autre-1", "source": "sud" }),
            body: String::new(),
        }];
        let p = plan(&f, &existing, "nord");
        assert_eq!((p.create.len(), p.duplicates), (1, 1));
        assert_eq!(p.create[0].0, "note-sans-image");
    }
}
