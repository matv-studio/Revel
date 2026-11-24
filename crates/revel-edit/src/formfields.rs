//! Champs d'un formulaire (audit M7, D-199) : liste des champs d'un `<form>`, champ ajouté
//! (texte, e-mail, téléphone, texte long, liste déroulante, case, boutons radio, date, fichier),
//! valeurs d'une liste déroulante. Un champ ajouté reprend les classes d'un champ déjà là, pour
//! garder son aspect.

use crate::EditError;
use crate::address::{self, Branch, Parent, Target};
use crate::command::Command;
use revel_model::ids::{child_lists, fresh, node_id};
use revel_model::{Attr, Node, Site};

/// Sorte de champ proposée par « Ajouter un champ ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldKind {
    Text,
    Email,
    Phone,
    LongText,
    Dropdown,
    Checkbox,
    Radio,
    Date,
    File,
}

impl FieldKind {
    pub const ALL: [FieldKind; 9] = [
        FieldKind::Text,
        FieldKind::Email,
        FieldKind::Phone,
        FieldKind::LongText,
        FieldKind::Dropdown,
        FieldKind::Checkbox,
        FieldKind::Radio,
        FieldKind::Date,
        FieldKind::File,
    ];
    /// Clé courte (traductions `form-field-<clé>`, essais).
    pub fn key(self) -> &'static str {
        match self {
            FieldKind::Text => "text",
            FieldKind::Email => "email",
            FieldKind::Phone => "phone",
            FieldKind::LongText => "long-text",
            FieldKind::Dropdown => "dropdown",
            FieldKind::Checkbox => "checkbox",
            FieldKind::Radio => "radio",
            FieldKind::Date => "date",
            FieldKind::File => "file",
        }
    }
}

/// Champ d'un formulaire tel que le montre la liste du panneau.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
    /// Élément du champ dans le formulaire (son `<label>`, son groupe, ou la saisie seule).
    pub id: String,
    /// Saisie du champ (`input`, `textarea`, `select`) : obligatoire, valeurs.
    pub control: String,
    pub label: String,
    pub kind: FieldKind,
    pub name: String,
    pub required: bool,
    /// Valeurs d'une liste déroulante.
    pub options: Vec<String>,
}

fn attr<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_deref().unwrap_or_default())
}

fn first_text(n: &Node) -> Option<String> {
    match n {
        Node::Text { text } if !text.trim().is_empty() => Some(text.trim().to_string()),
        Node::Element { tag, children, .. }
            if !matches!(tag.as_str(), "select" | "textarea" | "option") =>
        {
            children.iter().find_map(first_text)
        }
        _ => None,
    }
}

/// Première saisie d'un sous-arbre : (identifiant, balise, attributs, enfants).
fn control(n: &Node) -> Option<(&str, &str, &[Attr], &[Node])> {
    if let Node::Element {
        id: Some(id),
        tag,
        attrs,
        children,
    } = n
        && matches!(tag.as_str(), "input" | "textarea" | "select")
        && !matches!(attr(attrs, "type"), Some("hidden" | "submit" | "button"))
    {
        return Some((id, tag, attrs, children));
    }
    child_lists(n)
        .into_iter()
        .flat_map(|l| l.iter())
        .find_map(control)
}

fn kind_of(tag: &str, attrs: &[Attr]) -> FieldKind {
    match (tag, attr(attrs, "type").unwrap_or("text")) {
        ("textarea", _) => FieldKind::LongText,
        ("select", _) => FieldKind::Dropdown,
        (_, "email") => FieldKind::Email,
        (_, "tel") => FieldKind::Phone,
        (_, "checkbox") => FieldKind::Checkbox,
        (_, "radio") => FieldKind::Radio,
        (_, "date") => FieldKind::Date,
        (_, "file") => FieldKind::File,
        _ => FieldKind::Text,
    }
}

fn collect(n: &Node, out: &mut Vec<Field>) {
    let Node::Element {
        id, tag, children, ..
    } = n
    else {
        return;
    };
    // Un champ : un `<label>` ou un groupe (`fieldset`) qui porte une saisie, ou la saisie seule.
    let holder = matches!(tag.as_str(), "label" | "fieldset");
    let bare = matches!(tag.as_str(), "input" | "textarea" | "select");
    if (holder || bare)
        && let (Some(id), Some((cid, ctag, cattrs, ckids))) = (id, control(n))
    {
        let kind = kind_of(ctag, cattrs);
        let name = attr(cattrs, "name").unwrap_or_default().to_string();
        let label = first_text(n)
            .or_else(|| attr(cattrs, "aria-label").map(str::to_string))
            .or_else(|| attr(cattrs, "placeholder").map(str::to_string))
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| name.clone());
        let options = ckids
            .iter()
            .filter_map(|o| match o {
                Node::Element { tag, children, .. } if tag == "option" => {
                    Some(children.iter().find_map(|c| match c {
                        Node::Text { text } => Some(text.trim().to_string()),
                        _ => None,
                    })?)
                }
                _ => None,
            })
            .collect();
        out.push(Field {
            id: id.clone(),
            control: cid.to_string(),
            label,
            kind,
            name,
            required: attr(cattrs, "required").is_some(),
            options,
        });
        return;
    }
    for c in children {
        collect(c, out);
    }
}

/// Champs du formulaire, dans l'ordre de la page.
pub fn fields(form: &Node) -> Vec<Field> {
    let mut out = Vec::new();
    if let Node::Element { children, .. } = form {
        for c in children {
            collect(c, &mut out);
        }
    }
    out
}

/// Classes d'un champ déjà là : (du champ, de son libellé, de sa saisie).
fn classes(form: &Node) -> (Option<String>, Option<String>, Option<String>) {
    fn find(n: &Node) -> Option<(Option<String>, Option<String>, Option<String>)> {
        let Node::Element {
            tag,
            attrs,
            children,
            ..
        } = n
        else {
            return None;
        };
        if tag == "label"
            && let Some((_, ctag, cattrs, _)) = control(n)
            && !matches!(
                kind_of(ctag, cattrs),
                FieldKind::Checkbox | FieldKind::Radio
            )
        {
            let span = children.iter().find_map(|c| match c {
                Node::Element { tag, attrs, .. } if tag == "span" => {
                    attr(attrs, "class").map(str::to_string)
                }
                _ => None,
            });
            return Some((
                attr(attrs, "class").map(str::to_string),
                span,
                attr(cattrs, "class").map(str::to_string),
            ));
        }
        children.iter().find_map(find)
    }
    find(form).unwrap_or((None, None, None))
}

fn a(name: &str, value: &str) -> Attr {
    Attr {
        name: name.into(),
        value: Some(value.into()),
        when: None,
    }
}

/// Nom du champ dans le message reçu : tiré du libellé, sans doublon dans le formulaire.
fn free_name(form: &Node, label: &str) -> String {
    let base = revel_model::slugify(label).replace('-', "_");
    let base = if base.is_empty() {
        "champ".to_string()
    } else {
        base
    };
    let taken: Vec<String> = fields(form).into_iter().map(|f| f.name).collect();
    if !taken.contains(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}_{n}"))
        .find(|n| !taken.contains(n))
        .unwrap_or(base)
}

/// Champ ajouté au formulaire `form`, avant son bouton d'envoi : libellé `label`, valeurs
/// `options` (liste déroulante, boutons radio). Renvoie la commande et l'identifiant du champ.
pub fn add_field(
    site: &Site,
    form: &Target,
    kind: FieldKind,
    label: &str,
    options: &[String],
) -> Result<(Command, String), EditError> {
    let host = address::node(site, form)?;
    let Node::Element { tag, children, .. } = host else {
        return Err(EditError::NoBranch);
    };
    if tag != "form" {
        return Err(EditError::NotFound("formulaire".into()));
    }
    let (field_class, label_class, control_class) = classes(host);
    let mut taken = site.node_ids();
    let mut id = || {
        let i = fresh(&taken);
        taken.insert(i.clone());
        Some(i)
    };
    let name = free_name(host, label);
    let class = |c: &Option<String>| c.as_deref().map(|c| a("class", c)).into_iter();
    let el = |id: Option<String>, tag: &str, attrs: Vec<Attr>, children: Vec<Node>| Node::Element {
        id,
        tag: tag.into(),
        attrs,
        children,
    };
    let txt = |t: &str| Node::Text { text: t.into() };
    let caption =
        |id: Option<String>| el(id, "span", class(&label_class).collect(), vec![txt(label)]);
    let field_id = id();
    let node = match kind {
        FieldKind::Checkbox => {
            let mut attrs: Vec<Attr> = class(&field_class).collect();
            attrs.push(a("data-revel-field", "checkbox"));
            el(
                field_id.clone(),
                "label",
                attrs,
                vec![
                    el(
                        id(),
                        "input",
                        vec![a("type", "checkbox"), a("name", &name), a("value", "1")],
                        Vec::new(),
                    ),
                    el(id(), "span", Vec::new(), vec![txt(label)]),
                ],
            )
        }
        FieldKind::Radio => {
            let mut attrs: Vec<Attr> = class(&field_class).collect();
            attrs.push(a("data-revel-field", "radio"));
            let mut kids = vec![el(
                id(),
                "legend",
                class(&label_class).collect(),
                vec![txt(label)],
            )];
            for o in options {
                kids.push(el(
                    id(),
                    "label",
                    Vec::new(),
                    vec![
                        el(
                            id(),
                            "input",
                            vec![a("type", "radio"), a("name", &name), a("value", o)],
                            Vec::new(),
                        ),
                        el(id(), "span", Vec::new(), vec![txt(o)]),
                    ],
                ));
            }
            el(field_id.clone(), "fieldset", attrs, kids)
        }
        _ => {
            let mut cattrs: Vec<Attr> = class(&control_class).collect();
            let (ctag, kids) = match kind {
                FieldKind::LongText => {
                    cattrs.push(a("rows", "5"));
                    ("textarea", Vec::new())
                }
                FieldKind::Dropdown => (
                    "select",
                    options
                        .iter()
                        .map(|o| el(id(), "option", Vec::new(), vec![txt(o)]))
                        .collect(),
                ),
                other => {
                    cattrs.push(a(
                        "type",
                        match other {
                            FieldKind::Email => "email",
                            FieldKind::Phone => "tel",
                            FieldKind::Date => "date",
                            FieldKind::File => "file",
                            _ => "text",
                        },
                    ));
                    ("input", Vec::new())
                }
            };
            cattrs.push(a("name", &name));
            match kind {
                FieldKind::Email => cattrs.push(a("autocomplete", "email")),
                FieldKind::Phone => cattrs.push(a("autocomplete", "tel")),
                _ => {}
            }
            el(
                field_id.clone(),
                "label",
                class(&field_class).collect(),
                vec![caption(id()), el(id(), ctag, cattrs, kids)],
            )
        }
    };
    // Avant le bouton d'envoi (enfant direct), sinon à la fin.
    let index = children
        .iter()
        .position(|c| {
            matches!(c, Node::Element { tag, attrs, .. }
                if (tag == "button" && attr(attrs, "type") != Some("button"))
                    || (tag == "input" && attr(attrs, "type") == Some("submit")))
        })
        .unwrap_or(children.len());
    Ok((
        Command::Insert {
            parent: Parent::of(form, Branch::Children),
            index,
            node,
        },
        field_id.unwrap_or_default(),
    ))
}

/// Champ obligatoire ou non (sur sa saisie).
pub fn set_required(control: &Target, on: bool) -> Command {
    if on {
        Command::SetAttr {
            at: control.clone(),
            name: "required".into(),
            value: None,
        }
    } else {
        Command::RemoveAttr {
            at: control.clone(),
            name: "required".into(),
        }
    }
}

/// Valeurs d'une liste déroulante remplacées par `options`.
pub fn set_options(site: &Site, select: &Target, options: &[String]) -> Result<Command, EditError> {
    let (parent, index) = address::locate(site, select)?;
    let mut node = address::node(site, select)?.clone();
    let Node::Element { tag, children, .. } = &mut node else {
        return Err(EditError::NoBranch);
    };
    if tag != "select" {
        return Err(EditError::NotFound("liste déroulante".into()));
    }
    let mut taken = site.node_ids();
    *children = options
        .iter()
        .map(|o| {
            let i = fresh(&taken);
            taken.insert(i.clone());
            Node::Element {
                id: Some(i),
                tag: "option".into(),
                attrs: Vec::new(),
                children: vec![Node::Text { text: o.clone() }],
            }
        })
        .collect();
    let _ = node_id(&node);
    Ok(Command::Replace {
        parent,
        index,
        node,
    })
}
