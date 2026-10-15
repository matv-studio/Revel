//! Commandes d'édition. Chaque commande appliquée renvoie la commande inverse.

use revel_model::ids::{node_id, walk};
use revel_model::{Attr, Breakpoint, Decl, ImageNode, Node, Site, StyleItem};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::EditError;
use crate::address::{self, Parent, Target};

/// Rangement de l'éditeur concerné par [`Command::SetShelf`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShelfKind {
    Media,
    Components,
}

/// Modification du document de site. Sérialisable : la même commande vient de l'interface, de la
/// ligne de commande ou du serveur MCP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    /// Donne une valeur à un attribut (`None` : attribut booléen), créé à la fin s'il n'existe pas.
    SetAttr {
        at: Target,
        name: String,
        #[serde(default)]
        value: Option<String>,
    },
    /// Retire un attribut (par son nom).
    RemoveAttr { at: Target, name: String },
    /// Insère un attribut à une position (inverse d'un retrait).
    InsertAttr {
        at: Target,
        index: usize,
        attr: Attr,
    },
    /// Retire l'attribut d'une position (inverse d'une insertion).
    RemoveAttrAt { at: Target, index: usize },
    /// Change la balise d'un élément (`div` -> `section`).
    SetTag { at: Target, tag: String },
    /// Remplace un nœud texte.
    SetText {
        parent: Parent,
        index: usize,
        text: String,
    },
    /// Remplace un nœud quelconque à une position (expression, HTML, affectation…).
    Replace {
        parent: Parent,
        index: usize,
        node: Node,
    },
    /// Remplace les paramètres d'une image (l'identifiant est conservé).
    SetImage { at: Target, image: ImageNode },
    /// Change l'expression d'une condition ou d'une boucle (`cond` ou `expr`/`item`).
    SetLogic {
        at: Target,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cond: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expr: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        item: Option<String>,
    },
    /// Insère un nœud (et ses enfants) à une position.
    Insert {
        parent: Parent,
        index: usize,
        node: Node,
    },
    /// Supprime un nœud structurel.
    Remove { at: Target },
    /// Supprime le nœud d'une position.
    RemoveAt { parent: Parent, index: usize },
    /// Déplace un nœud ; `index` est la position dans la liste d'arrivée, après le retrait.
    Move {
        at: Target,
        to: Parent,
        index: usize,
    },
    /// Variable d'une page (`None` : supprimée).
    SetPageProp {
        page: String,
        key: String,
        #[serde(default)]
        value: Option<Value>,
    },
    /// Suite de commandes appliquées ensemble (annulées ensemble).
    Batch { commands: Vec<Command> },
    /// Duplique un nœud (copie insérée après lui, nouveaux identifiants).
    Duplicate { at: Target },
    /// Enveloppe un nœud dans un nouvel élément.
    Wrap { at: Target, tag: String },
    /// Insère un élément du panneau « Ajouter » ; `text` : texte par défaut (traduit).
    InsertPreset {
        parent: Parent,
        index: usize,
        preset: crate::make::Preset,
        #[serde(default)]
        text: String,
    },
    /// Déclaration d'une règle de style (dernière occurrence de la propriété) : valeur changée sur
    /// place, ajoutée à la fin si absente, retirée si `value` vaut `None`.
    SetDecl {
        rule: String,
        prop: String,
        #[serde(default)]
        value: Option<String>,
        #[serde(default)]
        important: bool,
    },
    /// Insère une déclaration à une position (inverse d'un retrait).
    InsertDecl {
        rule: String,
        index: usize,
        decl: Decl,
    },
    /// Retire la déclaration d'une position (inverse d'un ajout).
    RemoveDeclAt { rule: String, index: usize },
    /// Change le sélecteur d'une règle.
    SetSelector { rule: String, selector: String },
    /// Insère un élément de la feuille de style (règle, commentaire, bloc brut) à une position.
    InsertStyle { index: usize, item: StyleItem },
    /// Retire l'élément de la feuille de style d'une position.
    RemoveStyleAt { index: usize },
    /// Supprime une règle de style.
    RemoveRule { rule: String },
    /// Texte d'une valeur liée (D-19) : fichier de données, en-tête d'un contenu, variable de page
    /// ou de gabarit. Appliquée par le cœur, qui écrit le fichier (`apply` la refuse).
    SetBound {
        binding: crate::canvas::Binding,
        value: String,
    },
    /// Champ de l'en-tête d'un contenu de collection (`value: None` : champ retiré). Appliquée par
    /// le cœur, qui réécrit seulement ce champ dans le fichier ; inverse : `WriteEntry` (texte
    /// exact du fichier avant la modification).
    SetEntryField {
        folder: String,
        file: String,
        key: String,
        #[serde(default)]
        value: Option<Value>,
    },
    /// Texte (Markdown) d'un contenu, après son en-tête. Appliquée par le cœur.
    SetEntryBody {
        folder: String,
        file: String,
        body: String,
    },
    /// Nouveau contenu : en-tête (`data`, objet) et texte. Appliquée par le cœur ; refusée si le
    /// fichier existe.
    CreateEntry {
        folder: String,
        file: String,
        data: Value,
        #[serde(default)]
        body: String,
    },
    /// Fichier d'un contenu remplacé par un texte exact (`None` : supprimé) : suppression, et
    /// inverse des commandes de contenu (fichier rétabli octet pour octet). Appliquée par le cœur.
    WriteEntry {
        folder: String,
        file: String,
        #[serde(default)]
        text: Option<String>,
    },
    /// Champ d'une collection dans `.pages.yml`, le schéma partagé avec Pages CMS (D-27) :
    /// modification fusionnée (objets fusionnés clé par clé, `null` : clé retirée) ; le nom du
    /// champ ne change pas. Appliquée par le cœur, qui
    /// ajoute au besoin la collection au fichier (ou le crée) ; inverse : `WriteCmsConfig` (texte
    /// exact du fichier avant la modification).
    SetCmsField {
        folder: String,
        field: String,
        patch: Value,
    },
    /// Nouveau champ (définition de Pages CMS) à la position donnée, sinon à la fin.
    AddCmsField {
        folder: String,
        def: Value,
        #[serde(default)]
        index: Option<usize>,
    },
    /// Champ retiré du schéma ; les valeurs restent dans les contenus.
    RemoveCmsField { folder: String, field: String },
    /// Champ déplacé à la position donnée de la liste.
    MoveCmsField {
        folder: String,
        field: String,
        index: usize,
    },
    /// Réglages d'une collection (libellé, colonnes et tri du tableau…) : modification fusionnée
    /// comme pour `SetCmsField`.
    SetCmsCollection { folder: String, patch: Value },
    /// Nouvelle collection de contenus : collection du site (dossier `folder`, sans page par
    /// contenu) et sa description dans `.pages.yml` (libellé, champs). Appliquée par le cœur ;
    /// inverse : `RemoveCmsCollection`.
    CreateCmsCollection {
        id: String,
        label: String,
        folder: String,
        fields: Value,
    },
    /// Collection retirée du site (ses contenus restent sur le disque) et `.pages.yml` remplacé
    /// par un texte exact (`None` : supprimé) : inverse de `CreateCmsCollection`.
    RemoveCmsCollection {
        id: String,
        #[serde(default)]
        config: Option<String>,
    },
    /// `.pages.yml` remplacé par un texte exact (`None` : supprimé) : inverse des commandes de
    /// schéma. Appliquée par le cœur.
    WriteCmsConfig {
        #[serde(default)]
        text: Option<String>,
    },
    /// Nouveau gabarit, inséré à la position `index` de la liste (à la fin si elle est plus
    /// courte). Refusé si l'identifiant est pris.
    AddLayout {
        layout: revel_model::Layout,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
    /// Gabarit retiré ; refusé s'il sert de parent à un autre gabarit ou à une page.
    RemoveLayout { id: String },
    /// Rangement de l'éditeur remplacé (dossiers et éléments rangés des médias ou des composants).
    SetShelf {
        kind: ShelfKind,
        shelf: revel_model::Shelf,
    },
    /// Largeur d'un point de rupture changée ; les règles de style écrites pour lui suivent
    /// (`@media` réécrite). La largeur reste entre celles des points voisins du même sens (ordre
    /// de la cascade inchangé).
    ResizeBreakpoint { id: String, width: u32 },
    /// Remplace la liste des points de rupture du site (la base `desktop` reste la première).
    SetBreakpoints { breakpoints: Vec<Breakpoint> },
    /// Nouvelle règle pour `selector` et `media`, placée dans la cascade (après les règles du même
    /// sélecteur pour des écrans plus larges).
    AddRule {
        selector: String,
        #[serde(default)]
        media: Option<String>,
        #[serde(default)]
        decls: Vec<Decl>,
    },
}

/// Remplace une commande « d'intention » (dupliquer, envelopper, ajouter un élément ou une règle) par les commandes
/// concrètes équivalentes, avec leurs nouveaux identifiants. C'est la forme concrète qui est
/// enregistrée dans l'historique : rétablir redonne exactement les mêmes identifiants.
pub fn expand(site: &Site, cmd: &Command) -> Result<Command, EditError> {
    Ok(match cmd {
        Command::Duplicate { at } => crate::make::duplicate(site, at)?.0,
        Command::Wrap { at, tag } => {
            if !valid_tag(tag) {
                return Err(EditError::InvalidTag(tag.clone()));
            }
            crate::make::wrap(site, at, tag)?.0
        }
        Command::InsertPreset {
            parent,
            index,
            preset,
            text,
        } => Command::Insert {
            parent: parent.clone(),
            index: *index,
            node: preset.node(site, text),
        },
        Command::AddRule {
            selector,
            media,
            decls,
        } => {
            let selector = crate::style::checked_selector(selector)?;
            let media = media
                .as_deref()
                .map(crate::style::checked_media)
                .transpose()?;
            let items = crate::style::items(site)?;
            let index = revel_style::insertion_index(items, &selector, media.as_deref());
            Command::InsertStyle {
                index,
                item: StyleItem::Rule(revel_model::StyleRule {
                    id: Some(revel_model::ids::fresh(&site.node_ids())),
                    selector,
                    media,
                    decls: decls.clone(),
                }),
            }
        }
        Command::Batch { commands } => {
            // Les intentions d'une suite sont développées au fur et à mesure (identifiants libres
            // compte tenu des commandes précédentes) : développement sur une copie.
            let mut scratch = site.clone();
            let mut out = Vec::with_capacity(commands.len());
            for c in commands {
                let e = expand(&scratch, c)?;
                // Commande appliquée par le cœur (fichiers du projet) : sans effet sur la copie.
                match apply(&mut scratch, &e) {
                    Ok(_) | Err(EditError::External) => {}
                    Err(err) => return Err(err),
                }
                out.push(e);
            }
            Command::Batch { commands: out }
        }
        other => other.clone(),
    })
}

fn element_attrs<'a>(site: &'a mut Site, at: &Target) -> Result<&'a mut Vec<Attr>, EditError> {
    match address::node_mut(site, at)? {
        Node::Element { attrs, .. } => Ok(attrs),
        _ => Err(EditError::NotAnElement(at.node.clone())),
    }
}

/// Identifiants contenus dans un nœud (lui compris).
fn ids_of(n: &Node) -> Vec<String> {
    let mut v = Vec::new();
    walk(std::slice::from_ref(n), &mut |x| {
        if let Some(id) = node_id(x) {
            v.push(id.to_string());
        }
    });
    v
}

/// Vérifie qu'un nœud à insérer n'introduit aucun identifiant déjà présent dans le site.
fn check_new_ids(site: &Site, n: &Node) -> Result<(), EditError> {
    let taken = site.node_ids();
    let mut seen = std::collections::HashSet::new();
    for id in ids_of(n) {
        if taken.contains(&id) || !seen.insert(id.clone()) {
            return Err(EditError::DuplicateId(id));
        }
    }
    Ok(())
}

/// Applique une commande et renvoie son inverse. En cas d'échec, le site est inchangé.
pub fn apply(site: &mut Site, cmd: &Command) -> Result<Command, EditError> {
    match cmd {
        Command::SetAttr { at, name, value } => {
            let attrs = element_attrs(site, at)?;
            match attrs.iter_mut().position(|a| &a.name == name) {
                Some(i) => {
                    let old = std::mem::replace(&mut attrs[i].value, value.clone());
                    Ok(Command::SetAttr {
                        at: at.clone(),
                        name: name.clone(),
                        value: old,
                    })
                }
                None => {
                    attrs.push(Attr {
                        name: name.clone(),
                        value: value.clone(),
                        when: None,
                    });
                    Ok(Command::RemoveAttrAt {
                        at: at.clone(),
                        index: attrs.len() - 1,
                    })
                }
            }
        }
        Command::RemoveAttr { at, name } => {
            let attrs = element_attrs(site, at)?;
            let i = attrs
                .iter()
                .position(|a| &a.name == name)
                .ok_or_else(|| EditError::NotFound(format!("attribut {name}")))?;
            let attr = attrs.remove(i);
            Ok(Command::InsertAttr {
                at: at.clone(),
                index: i,
                attr,
            })
        }
        Command::InsertAttr { at, index, attr } => {
            let attrs = element_attrs(site, at)?;
            if *index > attrs.len() {
                return Err(EditError::Index(*index));
            }
            attrs.insert(*index, attr.clone());
            Ok(Command::RemoveAttrAt {
                at: at.clone(),
                index: *index,
            })
        }
        Command::RemoveAttrAt { at, index } => {
            let attrs = element_attrs(site, at)?;
            if *index >= attrs.len() {
                return Err(EditError::Index(*index));
            }
            let attr = attrs.remove(*index);
            Ok(Command::InsertAttr {
                at: at.clone(),
                index: *index,
                attr,
            })
        }
        Command::SetTag { at, tag } => {
            if !valid_tag(tag) {
                return Err(EditError::InvalidTag(tag.clone()));
            }
            match address::node_mut(site, at)? {
                Node::Element { tag: t, .. } => Ok(Command::SetTag {
                    at: at.clone(),
                    tag: std::mem::replace(t, tag.clone()),
                }),
                _ => Err(EditError::NotAnElement(at.node.clone())),
            }
        }
        Command::SetText {
            parent,
            index,
            text,
        } => {
            let list = address::list_mut(site, parent)?;
            match list.get_mut(*index) {
                Some(Node::Text { text: t }) => Ok(Command::SetText {
                    parent: parent.clone(),
                    index: *index,
                    text: std::mem::replace(t, text.clone()),
                }),
                Some(_) => Err(EditError::NotText(*index)),
                None => Err(EditError::Index(*index)),
            }
        }
        Command::Replace {
            parent,
            index,
            node,
        } => {
            let old = address::list(site, parent)?
                .get(*index)
                .cloned()
                .ok_or(EditError::Index(*index))?;
            // Les identifiants du nœud remplacé sont libérés avant la vérification.
            let freed: std::collections::HashSet<String> = ids_of(&old).into_iter().collect();
            let taken = site.node_ids();
            for id in ids_of(node) {
                if taken.contains(&id) && !freed.contains(&id) {
                    return Err(EditError::DuplicateId(id));
                }
            }
            let list = address::list_mut(site, parent)?;
            list[*index] = node.clone();
            Ok(Command::Replace {
                parent: parent.clone(),
                index: *index,
                node: old,
            })
        }
        Command::SetImage { at, image } => match address::node_mut(site, at)? {
            Node::Image(img) => {
                let mut new = image.clone();
                new.id = img.id.clone();
                let old = std::mem::replace(img, new);
                Ok(Command::SetImage {
                    at: at.clone(),
                    image: old,
                })
            }
            _ => Err(EditError::WrongKind(at.node.clone())),
        },
        Command::SetLogic {
            at,
            cond,
            expr,
            item,
        } => match address::node_mut(site, at)? {
            Node::If { cond: c, .. } => {
                let new = cond.clone().ok_or(EditError::WrongKind(at.node.clone()))?;
                Ok(Command::SetLogic {
                    at: at.clone(),
                    cond: Some(std::mem::replace(c, new)),
                    expr: None,
                    item: None,
                })
            }
            Node::Each {
                expr: e, item: i, ..
            } => {
                let old_e = expr.as_ref().map(|x| std::mem::replace(e, x.clone()));
                let old_i = item.as_ref().map(|x| std::mem::replace(i, x.clone()));
                Ok(Command::SetLogic {
                    at: at.clone(),
                    cond: None,
                    expr: old_e,
                    item: old_i,
                })
            }
            _ => Err(EditError::WrongKind(at.node.clone())),
        },
        Command::Insert {
            parent,
            index,
            node,
        } => {
            check_new_ids(site, node)?;
            let list = address::list_mut(site, parent)?;
            if *index > list.len() {
                return Err(EditError::Index(*index));
            }
            list.insert(*index, node.clone());
            Ok(Command::RemoveAt {
                parent: parent.clone(),
                index: *index,
            })
        }
        Command::Remove { at } => {
            let (parent, index) = address::locate(site, at)?;
            apply(site, &Command::RemoveAt { parent, index })
        }
        Command::RemoveAt { parent, index } => {
            let list = address::list_mut(site, parent)?;
            if *index >= list.len() {
                return Err(EditError::Index(*index));
            }
            let node = list.remove(*index);
            Ok(Command::Insert {
                parent: parent.clone(),
                index: *index,
                node,
            })
        }
        Command::Move { at, to, index } => {
            let (from, from_index) = address::locate(site, at)?;
            // Interdit : déplacer un nœud dans lui-même ou dans un de ses descendants.
            if let Some(dest) = &to.node {
                let moved = address::node(site, at)?;
                if ids_of(moved).iter().any(|i| i == dest) {
                    return Err(EditError::InvalidMove);
                }
            }
            address::list(site, to)?; // destination valide avant toute modification
            let node = address::list_mut(site, &from)?.remove(from_index);
            let dest = match address::list_mut(site, to) {
                Ok(d) if *index <= d.len() => d,
                Ok(_) | Err(_) => {
                    // Remise en place si la position d'arrivée est invalide.
                    address::list_mut(site, &from)?.insert(from_index, node);
                    return Err(EditError::Index(*index));
                }
            };
            dest.insert(*index, node);
            // Le nœud appartient désormais au propriétaire d'arrivée (page, gabarit, composant).
            Ok(Command::Move {
                at: Target {
                    owner: to.owner.clone(),
                    node: at.node.clone(),
                },
                to: from,
                index: from_index,
            })
        }
        Command::SetPageProp { page, key, value } => {
            let p = site
                .pages
                .iter_mut()
                .find(|p| &p.id == page)
                .ok_or_else(|| EditError::NotFound(format!("page {page}")))?;
            let old = match value {
                Some(v) => p.props.insert(key.clone(), v.clone()),
                None => p.props.remove(key),
            };
            Ok(Command::SetPageProp {
                page: page.clone(),
                key: key.clone(),
                value: old,
            })
        }
        Command::SetDecl {
            rule,
            prop,
            value,
            important,
        } => {
            let new = match value.as_deref() {
                Some(v) if v.trim().is_empty() => {
                    return Err(EditError::InvalidCss(format!("{prop}:")));
                }
                Some(v) => Some(crate::style::checked_decl(prop, v, *important)?),
                None => None,
            };
            let r = crate::style::rule_mut(site, rule)?;
            let pos = r.decls.iter().rposition(|d| &d.prop == prop);
            match (pos, new) {
                (Some(i), Some(d)) => {
                    let old = std::mem::replace(&mut r.decls[i], d);
                    Ok(Command::SetDecl {
                        rule: rule.clone(),
                        prop: prop.clone(),
                        value: Some(old.value),
                        important: old.important,
                    })
                }
                (Some(i), None) => Ok(Command::InsertDecl {
                    rule: rule.clone(),
                    index: i,
                    decl: r.decls.remove(i),
                }),
                (None, Some(d)) => {
                    r.decls.push(d);
                    Ok(Command::RemoveDeclAt {
                        rule: rule.clone(),
                        index: r.decls.len() - 1,
                    })
                }
                (None, None) => Err(EditError::NotFound(format!("propriété {prop}"))),
            }
        }
        Command::InsertDecl { rule, index, decl } => {
            let d = crate::style::checked_decl(&decl.prop, &decl.value, decl.important)?;
            let r = crate::style::rule_mut(site, rule)?;
            if *index > r.decls.len() {
                return Err(EditError::Index(*index));
            }
            r.decls.insert(*index, d);
            Ok(Command::RemoveDeclAt {
                rule: rule.clone(),
                index: *index,
            })
        }
        Command::RemoveDeclAt { rule, index } => {
            let r = crate::style::rule_mut(site, rule)?;
            if *index >= r.decls.len() {
                return Err(EditError::Index(*index));
            }
            Ok(Command::InsertDecl {
                rule: rule.clone(),
                index: *index,
                decl: r.decls.remove(*index),
            })
        }
        Command::SetSelector { rule, selector } => {
            let selector = crate::style::checked_selector(selector)?;
            let r = crate::style::rule_mut(site, rule)?;
            Ok(Command::SetSelector {
                rule: rule.clone(),
                selector: std::mem::replace(&mut r.selector, selector),
            })
        }
        Command::InsertStyle { index, item } => {
            let item = crate::style::checked_item(site, item)?;
            crate::style::insert_item(site, *index, item)?;
            Ok(Command::RemoveStyleAt { index: *index })
        }
        Command::RemoveStyleAt { index } => Ok(Command::InsertStyle {
            index: *index,
            item: crate::style::remove_item(site, *index)?,
        }),
        Command::RemoveRule { rule } => {
            let index = crate::style::rule_index(site, rule)?;
            apply(site, &Command::RemoveStyleAt { index })
        }
        Command::SetBound { .. }
        | Command::SetEntryField { .. }
        | Command::SetEntryBody { .. }
        | Command::CreateEntry { .. }
        | Command::WriteEntry { .. }
        | Command::SetCmsField { .. }
        | Command::AddCmsField { .. }
        | Command::RemoveCmsField { .. }
        | Command::MoveCmsField { .. }
        | Command::SetCmsCollection { .. }
        | Command::CreateCmsCollection { .. }
        | Command::RemoveCmsCollection { .. }
        | Command::WriteCmsConfig { .. } => Err(EditError::External),
        Command::ResizeBreakpoint { id, width } => {
            let bad = |m: &str| EditError::InvalidBreakpoints(m.to_string());
            let i = site
                .breakpoints
                .iter()
                .position(|b| b.id == *id)
                .ok_or_else(|| bad(id))?;
            let old = site.breakpoints[i].clone();
            let (old_w, is_max) = match (old.max_width, old.min_width) {
                (Some(w), _) => (w, true),
                (None, Some(w)) => (w, false),
                _ => return Err(bad(id)),
            };
            if *width == old_w {
                return Ok(cmd.clone());
            }
            // Points du même sens : la nouvelle largeur ne les franchit pas.
            let same_side = site
                .breakpoints
                .iter()
                .filter(|b| b.id != *id)
                .filter_map(|b| if is_max { b.max_width } else { b.min_width });
            let keeps_order = same_side
                .into_iter()
                .all(|w| w != *width && (w > old_w) == (w > *width));
            if !keeps_order || !(100..100_000).contains(width) {
                return Err(bad(id));
            }
            let old_media = old.media();
            let mut new = old;
            if is_max {
                new.max_width = Some(*width);
            } else {
                new.min_width = Some(*width);
            }
            let new_media = new.media();
            site.breakpoints[i] = new;
            if let Some(sheet) = site.stylesheet.as_mut() {
                for item in &mut sheet.items {
                    match item {
                        revel_model::StyleItem::Rule(r) if r.media == old_media => {
                            r.media = new_media.clone()
                        }
                        revel_model::StyleItem::Comment { media, .. } if *media == old_media => {
                            *media = new_media.clone()
                        }
                        _ => {}
                    }
                }
            }
            Ok(Command::ResizeBreakpoint {
                id: id.clone(),
                width: old_w,
            })
        }
        Command::AddLayout { layout, index } => {
            if site.layouts.iter().any(|l| l.id == layout.id) {
                return Err(EditError::DuplicateId(layout.id.clone()));
            }
            let used = site.node_ids();
            let mut fresh = Vec::new();
            walk(&layout.body, &mut |n| {
                if let Some(id) = node_id(n) {
                    fresh.push(id.to_string());
                }
            });
            if let Some(id) = fresh.iter().find(|i| used.contains(*i)) {
                return Err(EditError::DuplicateId(id.clone()));
            }
            let i = index.unwrap_or(usize::MAX).min(site.layouts.len());
            site.layouts.insert(i, layout.clone());
            Ok(Command::RemoveLayout {
                id: layout.id.clone(),
            })
        }
        Command::RemoveLayout { id } => {
            let i = site
                .layouts
                .iter()
                .position(|l| &l.id == id)
                .ok_or_else(|| EditError::NotFound(id.clone()))?;
            let used = site.layouts.iter().any(|l| l.parent.as_deref() == Some(id))
                || site.pages.iter().any(|p| p.layout.as_deref() == Some(id))
                || site
                    .collections
                    .iter()
                    .any(|c| c.layout.as_deref() == Some(id));
            if used {
                return Err(EditError::WrongKind(id.clone()));
            }
            let layout = site.layouts.remove(i);
            Ok(Command::AddLayout {
                layout,
                index: Some(i),
            })
        }
        Command::SetShelf { kind, shelf } => {
            let slot = match kind {
                ShelfKind::Media => &mut site.library.media,
                ShelfKind::Components => &mut site.library.components,
            };
            Ok(Command::SetShelf {
                kind: *kind,
                shelf: std::mem::replace(slot, shelf.clone()),
            })
        }
        Command::SetBreakpoints { breakpoints } => {
            check_breakpoints(breakpoints)?;
            Ok(Command::SetBreakpoints {
                breakpoints: std::mem::replace(&mut site.breakpoints, breakpoints.clone()),
            })
        }
        Command::Duplicate { .. }
        | Command::Wrap { .. }
        | Command::InsertPreset { .. }
        | Command::AddRule { .. } => {
            let concrete = expand(site, cmd)?;
            apply(site, &concrete)
        }
        Command::Batch { commands } => {
            let mut inverses: Vec<Command> = Vec::with_capacity(commands.len());
            for c in commands {
                match apply(site, c) {
                    Ok(inv) => inverses.push(inv),
                    Err(e) => {
                        // Annulation de ce qui a déjà été appliqué : la suite est atomique.
                        for inv in inverses.iter().rev() {
                            let _ = apply(site, inv);
                        }
                        return Err(e);
                    }
                }
            }
            inverses.reverse();
            Ok(Command::Batch { commands: inverses })
        }
    }
}

/// Points de rupture valides : la base `desktop` (sans largeur) en premier, identifiants uniques,
/// une seule largeur par point, largeurs strictement positives et toutes différentes.
fn check_breakpoints(bps: &[Breakpoint]) -> Result<(), EditError> {
    let bad = |m: &str| EditError::InvalidBreakpoints(m.to_string());
    match bps.first() {
        Some(b) if b.id == "desktop" && b.max_width.is_none() && b.min_width.is_none() => {}
        _ => return Err(bad("desktop")),
    }
    let mut ids = std::collections::HashSet::new();
    let mut widths = std::collections::HashSet::new();
    for b in bps {
        if b.id.is_empty() || !ids.insert(b.id.as_str()) {
            return Err(bad(&b.id));
        }
        if b.id == "desktop" {
            continue;
        }
        match (b.max_width, b.min_width) {
            (Some(w), None) | (None, Some(w)) if w > 0 && w < 100_000 => {
                if !widths.insert((b.max_width.is_some(), w)) {
                    return Err(bad(&b.id));
                }
            }
            _ => return Err(bad(&b.id)),
        }
    }
    Ok(())
}

/// Nom de balise HTML acceptable (lettres, chiffres, tirets ; commence par une lettre).
fn valid_tag(t: &str) -> bool {
    let mut c = t.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic())
        && c.all(|x| x.is_ascii_alphanumeric() || x == '-')
}

impl Command {
    /// Nœud principal visé (pour la sélection après une commande).
    pub fn target(&self) -> Option<&Target> {
        match self {
            Self::SetAttr { at, .. }
            | Self::RemoveAttr { at, .. }
            | Self::InsertAttr { at, .. }
            | Self::RemoveAttrAt { at, .. }
            | Self::SetTag { at, .. }
            | Self::SetImage { at, .. }
            | Self::SetLogic { at, .. }
            | Self::Remove { at }
            | Self::Move { at, .. }
            | Self::Duplicate { at }
            | Self::Wrap { at, .. } => Some(at),
            Self::Batch { commands } => commands.iter().find_map(Self::target),
            _ => None,
        }
    }

    /// Deux commandes successives peuvent-elles être fusionnées dans l'historique (saisie continue
    /// dans un champ) ? Même genre et même cible.
    pub fn same_field(&self, other: &Command) -> bool {
        match (self, other) {
            (Self::SetAttr { at: a, name: n, .. }, Self::SetAttr { at: b, name: m, .. }) => {
                a == b && n == m
            }
            (
                Self::SetText {
                    parent: a,
                    index: i,
                    ..
                },
                Self::SetText {
                    parent: b,
                    index: j,
                    ..
                },
            ) => a == b && i == j,
            (
                Self::SetPageProp {
                    page: a, key: k, ..
                },
                Self::SetPageProp {
                    page: b, key: l, ..
                },
            ) => a == b && k == l,
            (Self::SetLogic { at: a, .. }, Self::SetLogic { at: b, .. }) => a == b,
            (
                Self::SetDecl {
                    rule: a, prop: p, ..
                },
                Self::SetDecl {
                    rule: b, prop: q, ..
                },
            ) => a == b && p == q,
            (Self::SetSelector { rule: a, .. }, Self::SetSelector { rule: b, .. }) => a == b,
            (Self::SetBound { binding: a, .. }, Self::SetBound { binding: b, .. }) => a == b,
            (
                Self::SetEntryBody {
                    folder: a, file: f, ..
                },
                Self::SetEntryBody {
                    folder: b, file: g, ..
                },
            ) => a == b && f == g,
            _ => false,
        }
    }
}
