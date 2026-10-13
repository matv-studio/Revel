//! `revel-render` : génération du site statique (HTML/CSS) à partir du modèle.
//!
//! Interface stable : le trait [`Renderer`]. L'implémentation [`HtmlRenderer`]
//! est volontairement minimale à l'étape 0 ; elle sera complétée à l'étape 1.

use revel_model::{Element, ElementKind, Page, Site};
use std::fmt::Write as _;

/// Fichier produit par la génération, chemin relatif à la racine publiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

pub trait Renderer {
    fn render(&self, site: &Site) -> Result<Vec<OutputFile>, RenderError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("chemin de page invalide : {0}")]
    InvalidPath(String),
}

#[derive(Debug, Default)]
pub struct HtmlRenderer;

impl Renderer for HtmlRenderer {
    fn render(&self, site: &Site) -> Result<Vec<OutputFile>, RenderError> {
        site.pages
            .iter()
            .map(|page| {
                Ok(OutputFile {
                    path: output_path(&page.path)?,
                    bytes: render_page(site, page).into_bytes(),
                })
            })
            .collect()
    }
}

/// `/agence/` -> `agence/index.html` ; `/` -> `index.html`.
fn output_path(path: &str) -> Result<String, RenderError> {
    if !path.starts_with('/') || path.contains("..") {
        return Err(RenderError::InvalidPath(path.into()));
    }
    let trimmed = path.trim_matches('/');
    Ok(if trimmed.is_empty() {
        "index.html".into()
    } else if trimmed.ends_with(".html") {
        trimmed.into()
    } else {
        format!("{trimmed}/index.html")
    })
}

fn render_page(site: &Site, page: &Page) -> String {
    let title = page.seo.title.as_deref().unwrap_or(&page.title);
    let mut out = String::new();
    let _ = write!(
        out,
        "<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{}</title>\n",
        escape(&site.default_locale),
        escape(title)
    );
    if let Some(desc) = &page.seo.description {
        let _ = writeln!(
            out,
            "<meta name=\"description\" content=\"{}\">",
            escape(desc)
        );
    }
    if page.seo.noindex {
        out.push_str("<meta name=\"robots\" content=\"noindex\">\n");
    }
    out.push_str("</head>\n<body>\n");
    render_element(&page.root, &mut out);
    out.push_str("\n</body>\n</html>\n");
    out
}

fn render_element(el: &Element, out: &mut String) {
    let class = if el.classes.is_empty() {
        String::new()
    } else {
        format!(" class=\"{}\"", escape(&el.classes.join(" ")))
    };
    match &el.kind {
        ElementKind::Section | ElementKind::Container => {
            let tag = if matches!(el.kind, ElementKind::Section) {
                "section"
            } else {
                "div"
            };
            let _ = write!(out, "<{tag}{class}>");
            for child in &el.children {
                render_element(child, out);
            }
            let _ = write!(out, "</{tag}>");
        }
        ElementKind::Heading { level, text } => {
            let l = (*level).clamp(1, 6);
            let _ = write!(out, "<h{l}{class}>{}</h{l}>", escape(text));
        }
        ElementKind::Text { text } => {
            let _ = write!(out, "<p{class}>{}</p>", escape(text));
        }
        ElementKind::Link { href, text } => {
            let _ = write!(
                out,
                "<a{class} href=\"{}\">{}</a>",
                escape(href),
                escape(text)
            );
        }
        ElementKind::Image { src, alt } => {
            let _ = write!(
                out,
                "<img{class} src=\"{}\" alt=\"{}\">",
                escape(src),
                escape(alt)
            );
        }
        ElementKind::RawHtml { html } => out.push_str(html),
    }
}

/// Échappement HTML pour le texte et les attributs entre guillemets doubles.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use revel_model::Seo;

    fn page(path: &str) -> Page {
        Page {
            id: "p".into(),
            path: path.into(),
            title: "Titre <test>".into(),
            seo: Seo {
                noindex: true,
                ..Default::default()
            },
            root: Element {
                id: "r".into(),
                kind: ElementKind::Text {
                    text: "a & b".into(),
                },
                classes: vec![],
                children: vec![],
            },
        }
    }

    #[test]
    fn chemins_de_sortie() {
        assert_eq!(output_path("/").unwrap(), "index.html");
        assert_eq!(output_path("/agence/").unwrap(), "agence/index.html");
        assert!(output_path("/../x").is_err());
    }

    #[test]
    fn page_echappee_et_noindex() {
        let mut site = Site::new("Test", "fr");
        site.pages.push(page("/agence/"));
        let files = HtmlRenderer.render(&site).unwrap();
        let html = String::from_utf8(files[0].bytes.clone()).unwrap();
        assert!(html.contains("<title>Titre &lt;test&gt;</title>"));
        assert!(html.contains("<p>a &amp; b</p>"));
        assert!(html.contains("noindex"));
    }
}
