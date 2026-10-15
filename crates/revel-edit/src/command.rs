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
    Pages,
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
    /// Nouvelle page, insérée à la position `index` de la liste (à la fin si elle est plus
    /// courte). Refusée si l'identifiant ou l'adresse est pris, ou si un nœud reprend un
    /// identifiant existant.
    AddPage {
        page: revel_model::Page,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
    /// Page supprimée (inverse : `AddPage` à sa place).
    RemovePage { id: String },
    /// Adresse d'une page (refusée si invalide ou prise par une autre page).
    SetPagePath { id: String, path: String },
    /// Page en brouillon (non publiée) ou publiée.
    SetPageDraft { id: String, draft: bool },
    /// Nom d'une page dans Revel (`None` : titre ou adresse).
    SetPageName { id: String, name: Option<String> },
    /// Redirections du site remplacées.
    SetRedirects {
        redirects: Vec<revel_model::Redirect>,
    },
    /// Référencement du site (plan du site, `robots.txt`).
    SetSeoSettings { seo: revel_model::SeoSettings },
    /// Identité du site (3.9) : nom, adresse publique (`https://…`, sans `/` final ; vide :
    /// aucune), langue principale (code valide, différent des autres langues), typographie
    /// (`fr` ou aucune).
    SetSiteInfo {
        name: String,
        #[serde(default)]
        base_url: Option<String>,
        default_locale: String,
        #[serde(default)]
        typography: Option<String>,
    },
    /// Fichiers du site déplacés (chemins relatifs au dossier du site : de, vers). Appliquée par
    /// le cœur : polices mises en place ou rangées dans `.revel/`, de façon annulable.
    MoveFiles { moves: Vec<(String, String)> },
    /// Code ajouté (3.9) : au site (`page` absent) ou à une page.
    SetCode {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        page: Option<String>,
        code: revel_model::CustomCode,
    },
    /// Valeur d'un fichier de données (`binding` : source `data`, chemin jusqu'à la clé), clé
    /// créée au besoin, retirée avec `None`. Appliquée par le cœur ; inverse : `WriteDataFile`.
    SetDataValue {
        binding: crate::canvas::Binding,
        #[serde(default)]
        value: Option<Value>,
    },
    /// Fichier de données (chemin relatif au projet) remplacé par un texte exact (`None` : fichier
    /// retiré). Appliquée par le cœur.
    WriteDataFile {
        file: String,
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
    /// Texte alternatif par défaut d'un média (`None` ou vide : retiré).
    SetMediaAlt { url: String, alt: Option<String> },
    /// Nouveau composant, inséré à la position `index` de la liste (à la fin si elle est plus
    /// courte). Refusé si l'identifiant est pris ou si un nœud reprend un identifiant existant.
    AddComponent {
        component: revel_model::Component,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
    /// Composant retiré ; refusé s'il est encore utilisé.
    RemoveComponent { id: String },
    /// Nom d'un composant dans Revel (`None` : fin de l'identifiant).
    SetComponentName { id: String, name: Option<String> },
    /// Contenu d'un composant remplacé (mêmes identifiants pour les nœuds gardés).
    SetComponentBody { id: String, body: Vec<Node> },
    /// Propriétés d'un composant remplacées (noms valides, chacun une seule fois).
    SetComponentProps {
        id: String,
        props: Vec<revel_model::PropDef>,
    },
    /// Valeur d'une propriété pour une utilisation de composant (expression : `"Titre"`,
    /// `p.data.title`, `true`) ; `None` : valeur par défaut du composant.
    SetUseArg {
        at: Target,
        name: String,
        value: Option<String>,
    },
    /// Crée le composant `id` à partir d'un nœud : le nœud devient son contenu et une utilisation
    /// du composant prend sa place.
    MakeComponent {
        at: Target,
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// Utilisation de composant remplacée par une copie de son contenu (nouveaux identifiants),
    /// valeurs des propriétés écrites dedans.
    Detach { at: Target },
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
    /// Nouvelle langue du site (3.8), insérée à la position `index` (à la fin par défaut). Code
    /// valide, différent de la langue principale et des autres langues.
    AddLocale {
        locale: revel_model::Locale,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
    /// Langue retirée, avec ses textes traduits (les fichiers de la langue restent sur le disque).
    RemoveLocale { code: String },
    /// Nom affiché et état d'une langue (`draft` : non publiée).
    SetLocale {
        code: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        draft: bool,
    },
    /// Traduction d'un texte écrit dans les pages (`source` : texte de la langue principale) ;
    /// `None` ou vide : traduction retirée.
    SetTranslation {
        locale: String,
        source: String,
        #[serde(default)]
        text: Option<String>,
        /// Traduction automatique, à relire (D-36) ; saisie : marque retirée.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        machine: bool,
    },
    /// Marques « traduction automatique à relire » posées (`on`) ou retirées (clés de
    /// [`revel_model::Locale::machine`]).
    MarkMachine {
        locale: String,
        keys: Vec<String>,
        on: bool,
    },
    /// Mots gardés tels quels par la traduction automatique (réglages du site).
    SetNoTranslate { terms: Vec<String> },
    /// Traduction du texte écrit à un endroit (canevas, inspecteur), remplacée par
    /// `SetTranslation` avec le texte de la langue principale.
    Translate {
        locale: String,
        place: crate::locale::TextPlace,
        text: String,
    },
}

/// Remplace une commande « d'intention » (dupliquer, envelopper, ajouter un élément ou une règle) par les commandes
/// concrètes équivalentes, avec leurs nouveaux identifiants. C'est la forme concrète qui est
/// enregistrée dans l'historique : rétablir redonne exactement les mêmes identifiants.
pub fn expand(site: &Site, cmd: &Command) -> Result<Command, EditError> {
    Ok(match cmd {
        Command::Duplicate { at } => crate::make::duplicate(site, at)?.0,
        Command::MakeComponent { at, id, name } => {
            crate::make::make_component(site, at, id, name.clone())?.0
        }
        Command::Detach { at } => crate::make::detach(site, at)?,
        Command::Translate {
            locale,
            place,
            text,
        } => {
            if site.locale(locale).is_none() {
                return Err(EditError::InvalidLocale(locale.clone()));
            }
            Command::SetTranslation {
                locale: locale.clone(),
                source: crate::locale::source_at(site, place)?,
                text: Some(text.clone()),
                machine: false,
            }
        }
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
        } => {
            let insert = Command::Insert {
                parent: parent.clone(),
                index: *index,
                node: preset.node(site, text),
            };
            // Règles de style de l'élément que le site n'a pas encore (après l'insertion : leurs
            // identifiants tiennent compte des nouveaux nœuds).
            let rules: Vec<Command> = if site.stylesheet.is_some() {
                preset
                    .rules()
                    .into_iter()
                    .filter(|(sel, _)| {
                        !site
                            .style_rules()
                            .any(|r| r.selector == *sel && r.media.is_none())
                    })
                    .map(|(sel, decls)| Command::AddRule {
                        selector: sel.to_string(),
                        media: None,
                        decls: decls
                            .into_iter()
                            .map(|(p, v)| Decl {
                                prop: p.into(),
                                value: v.into(),
                                important: false,
                            })
                            .collect(),
                    })
                    .collect()
            } else {
                Vec::new()
            };
            if rules.is_empty() {
                insert
            } else {
                let mut commands = vec![insert];
                commands.extend(rules);
                expand(site, &Command::Batch { commands })?
            }
        }
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

/// Adresse de page valide et libre (`except` : la page qui la porte déjà).
fn check_page_path(site: &Site, path: &str, except: Option<&String>) -> Result<(), EditError> {
    if !revel_model::valid_page_path(path) {
        return Err(EditError::InvalidPath(path.to_string()));
    }
    // Adresses des autres langues réservées (`/en/…`).
    if site.locale_of_url(path).is_some() {
        return Err(EditError::PathTaken(path.to_string()));
    }
    if site
        .pages
        .iter()
        .any(|p| p.path == path && Some(&p.id) != except)
    {
        return Err(EditError::PathTaken(path.to_string()));
    }
    Ok(())
}

/// Un composant ne peut pas être placé dans lui-même (ni dans un composant qu'il utilise).
fn check_self_use(site: &Site, owner: &crate::Owner, n: &Node) -> Result<(), EditError> {
    let mut bad = None;
    walk(std::slice::from_ref(n), &mut |x| {
        if let Node::Use { component, .. } = x
            && !crate::make::can_use_in(site, owner, component)
        {
            bad.get_or_insert_with(|| component.clone());
        }
    });
    match bad {
        Some(c) => Err(EditError::SelfUse(c)),
        None => Ok(()),
    }
}

/// Propriétés d'un composant : noms valides, chacun une seule fois.
fn check_props(props: &[revel_model::PropDef]) -> Result<(), EditError> {
    let mut seen = std::collections::HashSet::new();
    for p in props {
        if !revel_model::valid_prop_name(&p.name) || !seen.insert(p.name.as_str()) {
            return Err(EditError::InvalidProp(p.name.clone()));
        }
    }
    Ok(())
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
            check_self_use(site, &parent.owner, node)?;
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
            if to.owner != at.owner {
                check_self_use(site, &to.owner, address::node(site, at)?)?;
            }
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
        | Command::WriteCmsConfig { .. }
        | Command::SetDataValue { .. }
        | Command::MoveFiles { .. }
        | Command::WriteDataFile { .. } => Err(EditError::External),
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
        Command::AddPage { page, index } => {
            if site.pages.iter().any(|p| p.id == page.id) {
                return Err(EditError::DuplicateId(page.id.clone()));
            }
            check_page_path(site, &page.path, None)?;
            let used = site.node_ids();
            let mut fresh = Vec::new();
            walk(&page.body, &mut |n| {
                if let Some(id) = node_id(n) {
                    fresh.push(id.to_string());
                }
            });
            if let Some(id) = fresh.iter().find(|i| used.contains(*i)) {
                return Err(EditError::DuplicateId(id.clone()));
            }
            let i = index.unwrap_or(usize::MAX).min(site.pages.len());
            site.pages.insert(i, page.clone());
            Ok(Command::RemovePage {
                id: page.id.clone(),
            })
        }
        Command::RemovePage { id } => {
            let i = site
                .pages
                .iter()
                .position(|p| &p.id == id)
                .ok_or_else(|| EditError::NotFound(format!("page {id}")))?;
            let page = site.pages.remove(i);
            Ok(Command::AddPage {
                page,
                index: Some(i),
            })
        }
        Command::SetPagePath { id, path } => {
            check_page_path(site, path, Some(id))?;
            let p = site
                .pages
                .iter_mut()
                .find(|p| &p.id == id)
                .ok_or_else(|| EditError::NotFound(format!("page {id}")))?;
            Ok(Command::SetPagePath {
                id: id.clone(),
                path: std::mem::replace(&mut p.path, path.clone()),
            })
        }
        Command::SetPageDraft { id, draft } => {
            let p = site
                .pages
                .iter_mut()
                .find(|p| &p.id == id)
                .ok_or_else(|| EditError::NotFound(format!("page {id}")))?;
            Ok(Command::SetPageDraft {
                id: id.clone(),
                draft: std::mem::replace(&mut p.draft, *draft),
            })
        }
        Command::SetPageName { id, name } => {
            let p = site
                .pages
                .iter_mut()
                .find(|p| &p.id == id)
                .ok_or_else(|| EditError::NotFound(format!("page {id}")))?;
            let name = name
                .as_ref()
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty());
            Ok(Command::SetPageName {
                id: id.clone(),
                name: std::mem::replace(&mut p.name, name),
            })
        }
        Command::SetRedirects { redirects } => {
            for r in redirects {
                if !r.from.starts_with('/') || r.from == r.to || r.to.trim().is_empty() {
                    return Err(EditError::InvalidRedirect(r.from.clone()));
                }
            }
            Ok(Command::SetRedirects {
                redirects: std::mem::replace(&mut site.redirects, redirects.clone()),
            })
        }
        Command::SetSeoSettings { seo } => Ok(Command::SetSeoSettings {
            seo: std::mem::replace(&mut site.settings.seo, seo.clone()),
        }),
        Command::SetSiteInfo {
            name,
            base_url,
            default_locale,
            typography,
        } => {
            let name = name.trim();
            if name.is_empty() {
                return Err(EditError::InvalidSite("name".into()));
            }
            let url = match base_url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
                None => None,
                Some(u) => Some(clean_url(u).ok_or_else(|| EditError::InvalidSite(u.to_string()))?),
            };
            let code = default_locale.trim();
            if !code.eq_ignore_ascii_case(&site.default_locale)
                && (!revel_model::valid_locale_code(code)
                    || site
                        .locales
                        .iter()
                        .any(|l| l.code.eq_ignore_ascii_case(code)))
            {
                return Err(EditError::InvalidLocale(code.to_string()));
            }
            let typo = typography
                .as_deref()
                .filter(|t| *t == "fr")
                .map(str::to_string);
            Ok(Command::SetSiteInfo {
                name: std::mem::replace(&mut site.name, name.to_string()),
                base_url: std::mem::replace(&mut site.base_url, url),
                default_locale: std::mem::replace(&mut site.default_locale, code.to_string()),
                typography: std::mem::replace(&mut site.settings.typography, typo),
            })
        }
        Command::SetCode { page, code } => {
            let clean = revel_model::CustomCode {
                head: code.head.trim_end().to_string(),
                body: code.body.trim_end().to_string(),
            };
            let slot = match page {
                None => &mut site.settings.code,
                Some(id) => {
                    &mut site
                        .pages
                        .iter_mut()
                        .find(|p| p.id == *id)
                        .ok_or_else(|| EditError::NotFound(id.clone()))?
                        .code
                }
            };
            Ok(Command::SetCode {
                page: page.clone(),
                code: std::mem::replace(slot, clean),
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
        Command::AddLocale { locale, index } => {
            crate::locale::check_new_locale(site, &locale.code)?;
            let i = index.unwrap_or(site.locales.len()).min(site.locales.len());
            site.locales.insert(i, locale.clone());
            Ok(Command::RemoveLocale {
                code: locale.code.clone(),
            })
        }
        Command::RemoveLocale { code } => {
            let i = site
                .locales
                .iter()
                .position(|l| l.code == *code)
                .ok_or_else(|| EditError::InvalidLocale(code.clone()))?;
            let locale = site.locales.remove(i);
            Ok(Command::AddLocale {
                locale,
                index: Some(i),
            })
        }
        Command::SetLocale { code, name, draft } => {
            let l = site
                .locales
                .iter_mut()
                .find(|l| l.code == *code)
                .ok_or_else(|| EditError::InvalidLocale(code.clone()))?;
            let name = name
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .map(str::to_string);
            Ok(Command::SetLocale {
                code: code.clone(),
                name: std::mem::replace(&mut l.name, name),
                draft: std::mem::replace(&mut l.draft, *draft),
            })
        }
        Command::SetTranslation {
            locale,
            source,
            text,
            machine,
        } => {
            let l = site
                .locales
                .iter_mut()
                .find(|l| l.code == *locale)
                .ok_or_else(|| EditError::InvalidLocale(locale.clone()))?;
            let key = crate::locale::normalize(source);
            if key.is_empty() {
                return Err(EditError::Bound(source.clone()));
            }
            let text = text
                .as_deref()
                .map(crate::locale::normalize)
                .filter(|t| !t.is_empty());
            let mark = crate::locale::text_key(&key);
            let was_machine = if *machine && text.is_some() {
                !l.machine.insert(mark)
            } else {
                l.machine.remove(&mark)
            };
            let old = match text {
                Some(t) => l.texts.insert(key.clone(), t),
                None => l.texts.remove(&key),
            };
            Ok(Command::SetTranslation {
                locale: locale.clone(),
                source: key,
                machine: was_machine && old.is_some(),
                text: old,
            })
        }
        Command::MarkMachine { locale, keys, on } => {
            let l = site
                .locales
                .iter_mut()
                .find(|l| l.code == *locale)
                .ok_or_else(|| EditError::InvalidLocale(locale.clone()))?;
            // Seules les marques changées sont rétablies à l'annulation.
            let changed: Vec<String> = keys
                .iter()
                .filter(|k| {
                    if *on {
                        l.machine.insert((*k).clone())
                    } else {
                        l.machine.remove(*k)
                    }
                })
                .cloned()
                .collect();
            Ok(Command::MarkMachine {
                locale: locale.clone(),
                keys: changed,
                on: !on,
            })
        }
        Command::SetNoTranslate { terms } => {
            let clean: Vec<String> = terms
                .iter()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .fold(Vec::new(), |mut v, t| {
                    if !v.contains(&t) {
                        v.push(t);
                    }
                    v
                });
            Ok(Command::SetNoTranslate {
                terms: std::mem::replace(&mut site.settings.no_translate, clean),
            })
        }
        Command::SetMediaAlt { url, alt } => {
            let old = match alt.as_ref().map(|a| a.trim()).filter(|a| !a.is_empty()) {
                Some(a) => site.library.alt.insert(url.clone(), a.to_string()),
                None => site.library.alt.remove(url),
            };
            Ok(Command::SetMediaAlt {
                url: url.clone(),
                alt: old,
            })
        }
        Command::SetShelf { kind, shelf } => {
            let slot = match kind {
                ShelfKind::Media => &mut site.library.media,
                ShelfKind::Components => &mut site.library.components,
                ShelfKind::Pages => &mut site.library.pages,
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
        Command::AddComponent { component, index } => {
            if site.components.iter().any(|c| c.id == component.id) {
                return Err(EditError::DuplicateId(component.id.clone()));
            }
            let used = site.node_ids();
            let mut seen = std::collections::HashSet::new();
            let mut dup = None;
            walk(&component.body, &mut |n| {
                if let Some(id) = node_id(n)
                    && (used.contains(id) || !seen.insert(id.to_string()))
                {
                    dup.get_or_insert_with(|| id.to_string());
                }
            });
            if let Some(id) = dup {
                return Err(EditError::DuplicateId(id));
            }
            check_props(&component.props)?;
            let i = index.unwrap_or(usize::MAX).min(site.components.len());
            site.components.insert(i, component.clone());
            Ok(Command::RemoveComponent {
                id: component.id.clone(),
            })
        }
        Command::RemoveComponent { id } => {
            let i = site
                .components
                .iter()
                .position(|c| &c.id == id)
                .ok_or_else(|| EditError::NotFound(id.clone()))?;
            if crate::make::uses_of(site, id) > 0 {
                return Err(EditError::InUse(id.clone()));
            }
            let component = site.components.remove(i);
            Ok(Command::AddComponent {
                component,
                index: Some(i),
            })
        }
        Command::SetComponentName { id, name } => {
            let c = site
                .components
                .iter_mut()
                .find(|c| &c.id == id)
                .ok_or_else(|| EditError::NotFound(id.clone()))?;
            let name = name
                .as_ref()
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty());
            Ok(Command::SetComponentName {
                id: id.clone(),
                name: std::mem::replace(&mut c.name, name),
            })
        }
        Command::SetComponentBody { id, body } => {
            let i = site
                .components
                .iter()
                .position(|c| &c.id == id)
                .ok_or_else(|| EditError::NotFound(id.clone()))?;
            // Identifiants nouveaux du contenu : pas déjà pris ailleurs dans le site.
            let old_ids: std::collections::HashSet<String> = {
                let mut v = std::collections::HashSet::new();
                walk(&site.components[i].body, &mut |n| {
                    if let Some(x) = node_id(n) {
                        v.insert(x.to_string());
                    }
                });
                v
            };
            let taken = site.node_ids();
            let mut dup = None;
            walk(body, &mut |n| {
                if let Some(x) = node_id(n)
                    && !old_ids.contains(x)
                    && taken.contains(x)
                {
                    dup.get_or_insert_with(|| x.to_string());
                }
            });
            if let Some(x) = dup {
                return Err(EditError::DuplicateId(x));
            }
            Ok(Command::SetComponentBody {
                id: id.clone(),
                body: std::mem::replace(&mut site.components[i].body, body.clone()),
            })
        }
        Command::SetComponentProps { id, props } => {
            check_props(props)?;
            let c = site
                .components
                .iter_mut()
                .find(|c| &c.id == id)
                .ok_or_else(|| EditError::NotFound(id.clone()))?;
            Ok(Command::SetComponentProps {
                id: id.clone(),
                props: std::mem::replace(&mut c.props, props.clone()),
            })
        }
        Command::SetUseArg { at, name, value } => match address::node_mut(site, at)? {
            Node::Use { args, .. } => {
                let old = match value {
                    Some(v) => args.insert(name.clone(), v.clone()),
                    None => args.remove(name),
                };
                Ok(Command::SetUseArg {
                    at: at.clone(),
                    name: name.clone(),
                    value: old,
                })
            }
            _ => Err(EditError::WrongKind(at.node.clone())),
        },
        Command::Duplicate { .. }
        | Command::Wrap { .. }
        | Command::InsertPreset { .. }
        | Command::MakeComponent { .. }
        | Command::Detach { .. }
        | Command::Translate { .. }
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
            (Self::SetUseArg { at: a, name: m, .. }, Self::SetUseArg { at: b, name: n, .. }) => {
                a == b && m == n
            }
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

/// Adresse publique d'un site : `http(s)://hôte[/chemin]`, sans `/` final.
pub fn clean_url(u: &str) -> Option<String> {
    let u = u.trim().trim_end_matches('/');
    let rest = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))?;
    let host = rest.split('/').next().unwrap_or_default();
    let ok = !host.is_empty()
        && !rest.contains(char::is_whitespace)
        && host
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'));
    ok.then(|| u.to_string())
}
