//! Plan d'une page pour un assistant : arbre des éléments (gabarits emboîtés, puis contenu de la
//! page), une ligne par nœud avec son identifiant, sa balise, ses classes et un extrait de texte.

use revel_model::{Node, Site};
use revel_protocol::EditorPageInfo;

/// Lignes au plus (le reste est résumé).
const MAX_LINES: usize = 900;

fn excerpt(s: &str, n: usize) -> String {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > n {
        format!("{}…", t.chars().take(n).collect::<String>())
    } else {
        t
    }
}

/// Valeur d'une expression : texte entre guillemets s'il s'agit d'une chaîne, sinon
/// `{{ expression }}`.
fn value(expr: &str, n: usize) -> String {
    let e = expr.trim();
    let quoted = e.len() >= 2
        && ((e.starts_with('"') && e.ends_with('"')) || (e.starts_with('\'') && e.ends_with('\'')))
        && !e[1..e.len() - 1].contains(e.chars().next().unwrap_or('"'));
    if quoted {
        format!("\"{}\"", excerpt(&e[1..e.len() - 1], n))
    } else {
        format!("{{{{ {} }}}}", excerpt(e, n))
    }
}

fn attr<'a>(n: &'a Node, name: &str) -> Option<&'a str> {
    match n {
        Node::Element { attrs, .. } => attrs
            .iter()
            .find(|a| a.name == name)
            .and_then(|a| a.value.as_deref()),
        _ => None,
    }
}

/// Texte direct d'un élément (enfants texte et expressions).
fn own_text(children: &[Node]) -> String {
    let mut s = String::new();
    for c in children {
        match c {
            Node::Text { text } => s.push_str(text),
            Node::Out { expr } => s.push_str(&format!("{{{{ {expr} }}}}")),
            _ => {}
        }
    }
    s
}

struct Out<'a> {
    site: &'a Site,
    lines: Vec<String>,
    /// Corps à mettre à la place de chaque emplacement `Slot`, du gabarit extérieur vers la page.
    inner: Vec<(String, &'a [Node])>,
}

impl<'a> Out<'a> {
    fn push(&mut self, depth: usize, line: String) {
        if self.lines.len() <= MAX_LINES {
            self.lines.push(format!("{}{line}", "  ".repeat(depth)));
        }
    }

    fn nodes(&mut self, nodes: &'a [Node], depth: usize) {
        for n in nodes {
            self.node(n, depth);
        }
    }

    fn node(&mut self, n: &'a Node, depth: usize) {
        match n {
            Node::Element {
                id,
                tag,
                attrs: _,
                children,
            } => {
                // Classes : `.a.b` ; classe calculée (expression) : écrite telle quelle.
                let class = attr(n, "class").unwrap_or_default();
                let dynamic = class.contains("{{") || class.contains("{%");
                let dotted = if dynamic {
                    String::new()
                } else {
                    class
                        .split_whitespace()
                        .map(|x| format!(".{x}"))
                        .collect::<String>()
                };
                let mut line = format!("<{tag}{dotted}>");
                if let Some(id) = id {
                    line.push_str(&format!(" #{id}"));
                }
                if dynamic {
                    line.push_str(&format!(" class=\"{}\"", excerpt(class, 60)));
                }
                // En-tête du document (balises meta, liens) : résumé, détail avec get_node.
                if tag == "head" {
                    let n = children
                        .iter()
                        .filter(|c| !matches!(c, Node::Text { .. }))
                        .count();
                    line.push_str(&format!(" ({n} items: meta, links; see get_node)"));
                    self.push(depth, line);
                    return;
                }
                for a in ["href", "src", "alt", "aria-label", "type"] {
                    if let Some(v) = attr(n, a) {
                        line.push_str(&format!(" {a}=\"{}\"", excerpt(v, 60)));
                    }
                }
                let t = own_text(children);
                if !t.trim().is_empty() {
                    line.push_str(&format!(" « {} »", excerpt(&t, 80)));
                }
                self.push(depth, line);
                let kids: Vec<&Node> = children
                    .iter()
                    .filter(|c| !matches!(c, Node::Text { .. } | Node::Out { .. }))
                    .collect();
                for k in kids {
                    self.node(k, depth + 1);
                }
            }
            Node::Image(img) => {
                let mut line = format!("<image src={}", value(&img.src, 80));
                if let Some(a) = &img.alt {
                    line.push_str(&format!(" alt={}", value(a, 60)));
                }
                if let Some(c) = &img.class {
                    line.push_str(&format!(" class={}", value(c, 60)));
                }
                line.push('>');
                if let Some(id) = &img.id {
                    line.push_str(&format!(" #{id}"));
                }
                self.push(depth, line);
            }
            Node::Text { text } if !text.trim().is_empty() => {
                self.push(depth, format!("« {} »", excerpt(text, 80)));
            }
            Node::Text { .. } => {}
            Node::Out { expr } => self.push(depth, format!("{{{{ {} }}}}", excerpt(expr, 80))),
            Node::Html { html } => {
                if !html.trim().is_empty() && !html.trim_start().starts_with("<!--") {
                    self.push(depth, format!("html: {}", excerpt(html, 80)));
                }
            }
            Node::If {
                id,
                cond,
                then,
                otherwise,
            } => {
                let id = id.as_deref().map(|i| format!(" #{i}")).unwrap_or_default();
                self.push(depth, format!("{{% if {} %}}{id}", excerpt(cond, 80)));
                self.nodes(then, depth + 1);
                if !otherwise.is_empty() {
                    self.push(depth, "{% else %}".into());
                    self.nodes(otherwise, depth + 1);
                }
            }
            Node::Each {
                id,
                expr,
                item,
                children,
                empty,
            } => {
                let id = id.as_deref().map(|i| format!(" #{i}")).unwrap_or_default();
                self.push(
                    depth,
                    format!("{{% for {item} in {} %}}{id} (repeated)", excerpt(expr, 80)),
                );
                self.nodes(children, depth + 1);
                if !empty.is_empty() {
                    self.push(depth, "{% else (empty list) %}".into());
                    self.nodes(empty, depth + 1);
                }
            }
            Node::Set { name, expr } => {
                self.push(depth, format!("{{% set {name} = {} %}}", excerpt(expr, 60)))
            }
            Node::SetBlock { name, .. } => self.push(depth, format!("{{% set {name} %}}…")),
            Node::Use {
                id,
                component,
                args,
                variant,
                attrs,
            } => {
                let label = self
                    .site
                    .component(component)
                    .map(|c| c.label())
                    .unwrap_or_else(|| component.clone());
                let id = id.as_deref().map(|i| format!(" #{i}")).unwrap_or_default();
                let mut args: Vec<String> = args
                    .iter()
                    .map(|(k, v)| format!("{k}={}", excerpt(v, 40)))
                    .collect();
                if let Some(v) = variant {
                    args.push(format!("variant={v}"));
                }
                args.extend(attrs.iter().map(|a| match &a.value {
                    Some(v) => format!("@{}={}", a.name, excerpt(v, 40)),
                    None => format!("@{}", a.name),
                }));
                self.push(
                    depth,
                    format!(
                        "[component {label} ({component})]{id}{}",
                        if args.is_empty() {
                            String::new()
                        } else {
                            format!(" {}", args.join(" "))
                        }
                    ),
                );
            }
            Node::Slot => {
                if self.inner.is_empty() {
                    self.push(depth, "[page content]".into());
                } else {
                    let (label, body) = self.inner.remove(0);
                    self.push(depth, format!("— {label} —"));
                    self.nodes(body, depth);
                }
            }
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }
}

/// Plan d'une page (gabarits emboîtés, contenu de la page à leur emplacement).
pub fn page(site: &Site, info: &EditorPageInfo) -> String {
    // Chaîne des gabarits, de l'extérieur vers l'intérieur.
    let mut chain = Vec::new();
    let mut cur = info.layout.clone();
    while let Some(id) = cur {
        let Some(l) = site.layouts.iter().find(|l| l.id == id) else {
            break;
        };
        if chain.iter().any(|x: &&revel_model::Layout| x.id == l.id) {
            break;
        }
        chain.push(l);
        cur = l.parent.clone();
    }
    chain.reverse();
    let page = info
        .page
        .as_ref()
        .and_then(|id| site.pages.iter().find(|p| &p.id == id));
    let mut inner: Vec<(String, &[Node])> = chain
        .iter()
        .skip(1)
        .map(|l| (format!("layout {}", l.id), l.body.as_slice()))
        .collect();
    if let Some(p) = page {
        inner.push((
            format!("page {} (owner of the nodes below)", p.id),
            p.body.as_slice(),
        ));
    }
    let mut out = Out {
        site,
        lines: Vec::new(),
        inner,
    };
    let mut head = format!("Page {}", info.url);
    if !info.title.is_empty() {
        head.push_str(&format!(" — « {} »", info.title));
    }
    if let Some(c) = &info.collection {
        head.push_str(&format!(
            " (collection entry {c}/{}; edit its fields with set_entry_field)",
            info.file.clone().unwrap_or_default()
        ));
    }
    out.lines.push(head);
    match (chain.first(), page) {
        (Some(l), _) => {
            out.lines.push(format!("— layout {} —", l.id));
            out.nodes(&l.body, 0);
        }
        (None, Some(p)) => out.nodes(&p.body, 0),
        (None, None) => {}
    }
    // Contenu de page sans emplacement dans les gabarits : à la suite.
    let rest: Vec<(String, &[Node])> = std::mem::take(&mut out.inner);
    for (label, body) in rest {
        out.lines.push(format!("— {label} —"));
        out.nodes(body, 0);
    }
    if out.lines.len() > MAX_LINES {
        out.lines.truncate(MAX_LINES);
        out.lines
            .push("… (truncated: use get_node on an id to see more)".into());
    }
    out.lines.join("\n")
}
