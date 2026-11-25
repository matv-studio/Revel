//! Filtres de liste reliés à un champ (audit M8, D-202) : on choisit la liste de la page et le
//! champ ; les boutons viennent des valeurs du champ (une boucle sur `filter_values`), chaque
//! contenu de la liste porte ses étiquettes (`filter_tags`), et la liste est marquée comme celle
//! que le groupe filtre. Bouton « Tout » facultatif, plusieurs valeurs à la fois en option.

use crate::EditError;
use crate::address::{self, Branch, Owner, Parent, Target};
use crate::collection::Query;
use crate::command::Command;
use revel_model::ids::{child_lists, fresh, node_id};
use revel_model::{Attr, Node, Site};

/// Groupe de filtres : nom du paramètre de l'adresse.
pub const GROUP: &str = "data-revel-filters";
/// Bouton : valeur comparée (vide : tout).
pub const BUTTON: &str = "data-revel-filter";
/// Champ dont viennent les boutons.
pub const FIELD: &str = "data-revel-filter-field";
/// Plusieurs valeurs à la fois.
pub const MULTI: &str = "data-revel-filter-multi";
/// Élément qui contient la liste filtrée (valeur : paramètre du groupe).
pub const SCOPE: &str = "data-revel-filter-scope";
/// Étiquettes d'un contenu de la liste.
pub const TAGS: &str = "data-revel-tags";

/// Liste de collection d'un propriétaire (page, gabarit, composant).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListOnPage {
    /// Boucle de la liste.
    pub each: String,
    pub collection: String,
    /// Variable de chaque contenu (`item`).
    pub item: String,
}

/// État d'un groupe de filtres tel que le montre le panneau.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    pub param: String,
    /// Champ dont viennent les boutons ; aucun : boutons écrits à la main.
    pub field: Option<String>,
    /// Boucle de la liste filtrée.
    pub list: Option<String>,
    /// Libellé du bouton « Tout », s'il y en a un.
    pub all: Option<String>,
    pub multi: bool,
    /// Boutons écrits à la main (valeur non vide hors boucle).
    pub written: usize,
}

fn a(name: &str, value: &str) -> Attr {
    Attr {
        name: name.into(),
        value: Some(value.into()),
        when: None,
    }
}

fn attr<'a>(n: &'a Node, name: &str) -> Option<&'a Attr> {
    match n {
        Node::Element { attrs, .. } | Node::Use { attrs, .. } => {
            attrs.iter().find(|x| x.name == name)
        }
        _ => None,
    }
}

fn walk<'a>(
    nodes: &'a [Node],
    f: &mut impl FnMut(&'a Node, Option<&'a Node>),
    parent: Option<&'a Node>,
) {
    for n in nodes {
        f(n, parent);
        for l in child_lists(n) {
            walk(l, f, Some(n));
        }
    }
}

/// Listes de collection du propriétaire, dans l'ordre du document.
pub fn lists(site: &Site, owner: &Owner) -> Vec<ListOnPage> {
    let Ok(body) = address::body(site, owner) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    walk(
        body,
        &mut |n, _| {
            if let Node::Each {
                id: Some(id),
                expr,
                item,
                ..
            } = n
                && let Some(q) = Query::parse(expr)
                && revel_model::valid_prop_name(item.trim())
            {
                out.push(ListOnPage {
                    each: id.clone(),
                    collection: q.collection,
                    item: item.trim().to_string(),
                });
            }
        },
        None,
    );
    out
}

/// Boucle des boutons tirés d'un champ (`collections.c | filter_values("champ")`).
fn is_values_loop(n: &Node) -> bool {
    matches!(n, Node::Each { expr, .. } if expr.contains("filter_values("))
}

/// État du groupe de filtres.
pub fn state(site: &Site, group: &Target) -> Result<State, EditError> {
    let g = address::node(site, group)?;
    let Node::Element { children, .. } = g else {
        return Err(EditError::NoBranch);
    };
    let value = |n: &str| attr(g, n).and_then(|x| x.value.clone());
    let param = value(GROUP).unwrap_or_default();
    let mut st = State {
        param: param.clone(),
        field: value(FIELD).filter(|f| !f.is_empty()),
        multi: attr(g, MULTI).is_some(),
        ..Default::default()
    };
    for c in children {
        if let Some(v) = attr(c, BUTTON) {
            if v.value.as_deref().unwrap_or("").is_empty() {
                if st.all.is_none() {
                    st.all = Some(first_text(c));
                }
            } else {
                st.written += 1;
            }
        }
    }
    if !param.is_empty()
        && let Ok(body) = address::body(site, &group.owner)
    {
        walk(
            body,
            &mut |n, _| {
                if st.list.is_none()
                    && attr(n, SCOPE).and_then(|x| x.value.as_deref()) == Some(param.as_str())
                    && let Node::Element { children, .. } = n
                    && let Some(id) = children
                        .iter()
                        .find(|c| matches!(c, Node::Each { .. }))
                        .and_then(node_id)
                {
                    st.list = Some(id.to_string());
                }
            },
            None,
        );
    }
    Ok(st)
}

fn first_text(n: &Node) -> String {
    let mut out = None;
    walk(
        std::slice::from_ref(n),
        &mut |x, _| {
            if out.is_none()
                && let Node::Text { text } = x
                && !text.trim().is_empty()
            {
                out = Some(text.trim().to_string());
            }
        },
        None,
    );
    out.unwrap_or_default()
}

/// Attributs d'un bouton du groupe, repris d'un bouton déjà là (classe, partie d'un élément
/// construit, type) sans sa valeur ; à défaut, `class="filtre" type="button"`.
fn button_attrs(children: &[Node]) -> Vec<Attr> {
    let inner = children
        .iter()
        .filter(|c| is_values_loop(c))
        .flat_map(|c| match c {
            Node::Each { children, .. } => children.iter().collect::<Vec<_>>(),
            _ => Vec::new(),
        });
    children
        .iter()
        .chain(inner)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .find_map(|c| match c {
            Node::Element { attrs, .. } if attrs.iter().any(|x| x.name == BUTTON) => Some(
                attrs
                    .iter()
                    .filter(|x| x.name != BUTTON)
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_else(|| vec![a("class", "filtre"), a("type", "button")])
}

fn with_value(mut attrs: Vec<Attr>, value: &str) -> Vec<Attr> {
    attrs.push(a(BUTTON, value));
    attrs
}

/// Relie le groupe à une liste de la page et à un champ : boutons remplacés par « Tout » (si
/// `all`) et une boucle sur les valeurs du champ ; liste marquée ; étiquettes posées sur chaque
/// contenu (et retirées de la liste reliée auparavant).
pub fn bind(
    site: &Site,
    group: &Target,
    list: &str,
    field: &str,
    all: Option<&str>,
) -> Result<Command, EditError> {
    let st = state(site, group)?;
    let g = address::node(site, group)?;
    let Node::Element { children, .. } = g else {
        return Err(EditError::NoBranch);
    };
    let owner = group.owner.clone();
    let lst = lists(site, &owner)
        .into_iter()
        .find(|l| l.each == list)
        .ok_or_else(|| EditError::NotFound(format!("liste {list}")))?;
    let param = if st.param.trim().is_empty() {
        revel_model::slugify(field)
    } else {
        st.param.clone()
    };
    let base = button_attrs(children);
    let mut taken = site.node_ids();
    let mut id = || {
        let i = fresh(&taken);
        taken.insert(i.clone());
        Some(i)
    };
    let mut cmds = vec![
        Command::SetAttr {
            at: group.clone(),
            name: GROUP.into(),
            value: Some(param.clone()),
        },
        Command::SetAttr {
            at: group.clone(),
            name: FIELD.into(),
            value: Some(field.into()),
        },
    ];
    // Boutons remplacés (de la fin vers le début).
    let inside = Parent::of(group, Branch::Children);
    for i in (0..children.len()).rev() {
        cmds.push(Command::RemoveAt {
            parent: inside.clone(),
            index: i,
        });
    }
    let button = |id: Option<String>, value: &str, text: Node| Node::Element {
        id,
        tag: "button".into(),
        attrs: with_value(base.clone(), value),
        children: vec![text],
    };
    let mut index = 0;
    if let Some(label) = all.or(st.all.as_deref()) {
        cmds.push(Command::Insert {
            parent: inside.clone(),
            index,
            node: button(id(), "", Node::Text { text: label.into() }),
        });
        index += 1;
    }
    let b = id();
    cmds.push(Command::Insert {
        parent: inside,
        index,
        node: Node::Each {
            id: id(),
            expr: format!(
                "collections.{} | filter_values(\"{field}\")",
                lst.collection
            ),
            item: "v".into(),
            children: vec![button(
                b,
                "{{ v.value }}",
                Node::Out {
                    expr: "v.label".into(),
                },
            )],
            empty: Vec::new(),
        },
    });
    // Liste filtrée marquée (son élément parent) ; ancienne liste démarquée.
    let each_at = Target {
        owner: owner.clone(),
        node: lst.each.clone(),
    };
    if let Some(old) = st.list.as_ref().filter(|o| **o != lst.each) {
        let old_at = Target {
            owner: owner.clone(),
            node: old.clone(),
        };
        if let Some(p) = address::locate(site, &old_at)
            .ok()
            .and_then(|(p, _)| p.node)
        {
            cmds.push(Command::RemoveAttr {
                at: Target {
                    owner: owner.clone(),
                    node: p,
                },
                name: SCOPE.into(),
            });
        }
        cmds.extend(tags(site, &old_at, None)?);
    }
    let (parent, _) = address::locate(site, &each_at)?;
    let scope = parent.node.ok_or(EditError::NoBranch)?;
    cmds.push(Command::SetAttr {
        at: Target {
            owner: owner.clone(),
            node: scope,
        },
        name: SCOPE.into(),
        value: Some(param),
    });
    cmds.extend(tags(
        site,
        &each_at,
        Some(&format!("{{{{ {} | filter_tags('{field}') }}}}", lst.item)),
    )?);
    Ok(Command::Batch { commands: cmds })
}

/// Étiquettes posées (ou retirées) sur les éléments racines de chaque contenu de la liste.
fn tags(site: &Site, each: &Target, value: Option<&str>) -> Result<Vec<Command>, EditError> {
    let Node::Each { children, .. } = address::node(site, each)? else {
        return Err(EditError::NoBranch);
    };
    let mut out = Vec::new();
    // Contenu dessiné par un élément ou par une utilisation de composant (attributs posés sur
    // sa racine).
    for c in children {
        let (Node::Element { id: Some(id), .. } | Node::Use { id: Some(id), .. }) = c else {
            continue;
        };
        let at = Target {
            owner: each.owner.clone(),
            node: id.clone(),
        };
        match value {
            Some(v) => out.push(Command::SetAttr {
                at,
                name: TAGS.into(),
                value: Some(v.into()),
            }),
            None if attr(c, TAGS).is_some() => out.push(Command::RemoveAttr {
                at,
                name: TAGS.into(),
            }),
            None => {}
        }
    }
    Ok(out)
}

/// Bouton « Tout » ajouté (en tête), renommé, ou retiré (`None`).
pub fn set_all(site: &Site, group: &Target, label: Option<&str>) -> Result<Command, EditError> {
    let Node::Element { children, .. } = address::node(site, group)? else {
        return Err(EditError::NoBranch);
    };
    let inside = Parent::of(group, Branch::Children);
    let pos = children
        .iter()
        .position(|c| attr(c, BUTTON).is_some_and(|x| x.value.as_deref().unwrap_or("").is_empty()));
    match (pos, label) {
        (Some(i), None) => Ok(Command::RemoveAt {
            parent: inside,
            index: i,
        }),
        (Some(i), Some(l)) => {
            let Node::Element {
                id: Some(bid),
                children: kids,
                ..
            } = &children[i]
            else {
                return Err(EditError::NoBranch);
            };
            let t = kids
                .iter()
                .position(|k| matches!(k, Node::Text { .. }))
                .ok_or(EditError::NotText(0))?;
            Ok(Command::SetText {
                parent: Parent::of(
                    &Target {
                        owner: group.owner.clone(),
                        node: bid.clone(),
                    },
                    Branch::Children,
                ),
                index: t,
                text: l.into(),
            })
        }
        (None, Some(l)) => {
            let taken = site.node_ids();
            Ok(Command::Insert {
                parent: inside,
                index: 0,
                node: Node::Element {
                    id: Some(fresh(&taken)),
                    tag: "button".into(),
                    attrs: with_value(button_attrs(children), ""),
                    children: vec![Node::Text { text: l.into() }],
                },
            })
        }
        (None, None) => Ok(Command::Batch {
            commands: Vec::new(),
        }),
    }
}

/// Plusieurs valeurs à la fois.
pub fn set_multi(group: &Target, on: bool) -> Command {
    if on {
        Command::SetAttr {
            at: group.clone(),
            name: MULTI.into(),
            value: None,
        }
    } else {
        Command::RemoveAttr {
            at: group.clone(),
            name: MULTI.into(),
        }
    }
}
