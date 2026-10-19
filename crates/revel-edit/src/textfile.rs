//! Fichiers du serveur (`.htaccess`, `robots.txt`, `.php`) : texte écrit avec les balises du
//! gabarit, montré et modifié comme tel dans les paramètres de la page.
//!
//! - valeur : `{{ site.url }}`, `{{ env.htpasswd }}`, `{{ j.data.pdf }}` ;
//! - condition : `{% if env.staging %}…{% else %}…{% endif %}` ;
//! - liste : `{% for j in collections.offres %}…{% endfor %}` ;
//! - variable : `{% set x = … %}`.
//!
//! Un fichier se modifie ici quand son texte relu redonne le même fichier ([`round_trips`]) ;
//! un fichier qui contient des éléments, des images ou des composants reste réglé dans les
//! calques.

use crate::address::{Owner, Parent};
use crate::command::Command;
use revel_model::ids::{child_lists_mut, node_id_mut};
use revel_model::{Node, Site};
use std::collections::HashSet;

/// Adresse d'un fichier du serveur (ni page HTML, ni fichier XML).
pub fn is_server_file(path: &str) -> bool {
    !(path.ends_with('/') || path.ends_with(".html") || path.ends_with(".xml"))
}

/// Texte écrit tel quel ; balises du gabarit protégées par `{% raw %}`.
fn literal(t: &str, out: &mut String) -> Option<()> {
    if t.contains("{{") || t.contains("{%") || t.contains("{#") {
        if t.contains("{% endraw") || t.contains("{%endraw") {
            return None;
        }
        out.push_str("{% raw %}");
        out.push_str(t);
        out.push_str("{% endraw %}");
    } else {
        out.push_str(t);
    }
    Some(())
}

fn write(nodes: &[Node], out: &mut String) -> Option<()> {
    for n in nodes {
        match n {
            Node::Html { html } => literal(html, out)?,
            Node::Text { text } => literal(text, out)?,
            Node::Out { expr } => {
                out.push_str("{{ ");
                out.push_str(expr.trim());
                out.push_str(" }}");
            }
            Node::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                out.push_str(&format!("{{% if {} %}}", cond.trim()));
                write(then, out)?;
                if !otherwise.is_empty() {
                    out.push_str("{% else %}");
                    write(otherwise, out)?;
                }
                out.push_str("{% endif %}");
            }
            Node::Each {
                expr,
                item,
                children,
                empty,
                ..
            } => {
                out.push_str(&format!("{{% for {} in {} %}}", item.trim(), expr.trim()));
                write(children, out)?;
                if !empty.is_empty() {
                    out.push_str("{% else %}");
                    write(empty, out)?;
                }
                out.push_str("{% endfor %}");
            }
            Node::Set { name, expr } => {
                out.push_str(&format!("{{% set {} = {} %}}", name.trim(), expr.trim()));
            }
            Node::SetBlock { name, children, .. } => {
                out.push_str(&format!("{{% set {} %}}", name.trim()));
                write(children, out)?;
                out.push_str("{% endset %}");
            }
            // Éléments, images, composants, emplacement : pas de forme écrite ici.
            _ => return None,
        }
    }
    Some(())
}

/// Texte du fichier avec ses balises ; `None` si un nœud n'a pas de forme écrite (élément,
/// image, composant).
pub fn source(nodes: &[Node]) -> Option<String> {
    let mut out = String::new();
    write(nodes, &mut out)?;
    Some(out)
}

/// Nœuds d'un texte avec balises (sans identifiants).
pub fn parse(src: &str) -> Result<Vec<Node>, String> {
    let parsed = revel_markup::parse(
        src,
        &revel_markup::Options {
            dialect: revel_markup::Dialect::Jinja,
            mode: revel_markup::Mode::Text,
        },
    )
    .map_err(|e| e.to_string())?;
    if let Some(w) = parsed.warnings.first() {
        return Err(w.clone());
    }
    Ok(parsed.nodes)
}

/// Le texte du fichier relu redonne-t-il le même fichier ?
pub fn round_trips(nodes: &[Node]) -> bool {
    let Some(s) = source(nodes) else {
        return false;
    };
    parse(&s).ok().and_then(|n| source(&n)).as_deref() == Some(s.as_str())
}

/// Identifiants nouveaux pour les nœuds qui n'en ont pas.
fn give_ids(nodes: &mut [Node], taken: &mut HashSet<String>) {
    for n in nodes {
        if let Some(slot) = node_id_mut(n)
            && slot.is_none()
        {
            let id = revel_model::ids::fresh(taken);
            taken.insert(id.clone());
            *slot = Some(id);
        }
        for list in child_lists_mut(n) {
            give_ids(list, taken);
        }
    }
}

/// Commande qui remplace le contenu de la page `page` par le texte `src` (annulable d'un coup).
pub fn set_source(site: &Site, page: &str, src: &str) -> Result<Command, String> {
    let p = site
        .pages
        .iter()
        .find(|p| p.id == page)
        .ok_or_else(|| format!("page introuvable : {page}"))?;
    let mut nodes = parse(src)?;
    let mut taken = site.node_ids();
    give_ids(&mut nodes, &mut taken);
    let root = Parent::root(Owner::Page(page.to_string()));
    let mut commands: Vec<Command> = (0..p.body.len())
        .rev()
        .map(|index| Command::RemoveAt {
            parent: root.clone(),
            index,
        })
        .collect();
    commands.extend(
        nodes
            .into_iter()
            .enumerate()
            .map(|(index, node)| Command::Insert {
                parent: root.clone(),
                index,
                node,
            }),
    );
    Ok(Command::Batch { commands })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fichier_du_serveur_relu() {
        let src = "# Recette\n{% if env.staging %}Header set X-Robots-Tag \"noindex\"\n{% if env.htpasswd %}AuthUserFile \"{{ env.htpasswd | safe }}\"{% endif %}{% else %}Sitemap: {{ site.url }}/sitemap.xml{% endif %}\n{% for j in collections.offres %}RewriteCond %{QUERY_STRING} job={{ j.data.slug }}\n{% else %}# aucune{% endfor %}\n";
        let nodes = parse(src).unwrap();
        assert_eq!(source(&nodes).as_deref(), Some(src));
        assert!(round_trips(&nodes));
        // Texte qui ressemble à une balise : protégé.
        let nodes = vec![Node::Html {
            html: "a {{ b }} c".into(),
        }];
        let s = source(&nodes).unwrap();
        assert_eq!(s, "{% raw %}a {{ b }} c{% endraw %}");
        assert!(round_trips(&nodes));
        // Élément : pas de forme écrite.
        assert!(source(&[Node::Slot]).is_none());
    }

    #[test]
    fn contenu_remplace_d_un_coup() {
        let mut s = Site::new("Essai", "fr");
        s.pages.push(revel_model::Page {
            id: "robots".into(),
            path: "/robots.txt".into(),
            layout: None,
            props: Default::default(),
            computed: Default::default(),
            body: vec![Node::Html {
                html: "User-agent: *\n".into(),
            }],
            draft: false,
            name: None,
            code: Default::default(),
        });
        let cmd = set_source(&s, "robots", "{% if env.staging %}Disallow: /{% endif %}\n").unwrap();
        crate::command::apply(&mut s, &cmd).unwrap();
        let body = &s.pages[0].body;
        assert!(matches!(&body[0], Node::If { id: Some(_), .. }));
        assert_eq!(
            source(body).as_deref(),
            Some("{% if env.staging %}Disallow: /{% endif %}\n")
        );
    }
}
