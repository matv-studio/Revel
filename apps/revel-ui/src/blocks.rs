//! Réglages des éléments de 3.7 : icône (choix du dessin), code HTML, vidéo (lecture), liste de
//! collection (collection, filtre, tri, nombre, comme la « Collection List » de Webflow) et
//! éléments d'une liste reliés aux champs du contenu (texte, image, lien).

use crate::editor::Ed;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::collection::{FieldList, Query};
use revel_edit::make::Preset;
use revel_edit::{Branch, Command, Parent, Target};
use revel_model::{Attr, Node, Site};
use revel_protocol::CmsCollection;

fn attr<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a Attr> {
    attrs.iter().find(|a| a.name == name)
}

/// Collections du site (schéma de Pages CMS), relues à la demande.
pub(crate) fn collections(ed: Ed) -> RwSignal<Vec<CmsCollection>> {
    let cols = RwSignal::new(Vec::new());
    let path = ed.path.get_value();
    // Listes qui lisent plusieurs collections : une collection réunie (`projets+actualites`),
    // aux champs des deux, pour relier leurs éléments.
    let keys: Vec<Vec<String>> = ed.doc.with_untracked(|d| {
        let mut keys: Vec<Vec<String>> = Vec::new();
        if let Some(d) = d.as_ref() {
            for (_, _, nodes) in d.owners() {
                revel_model::ids::walk(nodes, &mut |n| {
                    if let Node::Each { expr, .. } = n
                        && let Some(q) = Query::parse(expr)
                        && !q.also.is_empty()
                    {
                        let ids: Vec<String> = std::iter::once(q.collection.clone())
                            .chain(q.also.clone())
                            .collect();
                        if !keys.contains(&ids) {
                            keys.push(ids);
                        }
                    }
                });
            }
        }
        keys
    });
    spawn_local(async move {
        if let Ok(mut c) = api::cms_collections(&path).await {
            for ids in keys {
                let parts: Vec<CmsCollection> = ids
                    .iter()
                    .filter_map(|id| c.iter().find(|x| &x.id == id).cloned())
                    .collect();
                let Some(first) = parts.first().cloned() else {
                    continue;
                };
                let mut merged = first;
                merged.id = ids.join("+");
                merged.label = Some(parts.iter().map(label_of).collect::<Vec<_>>().join(" + "));
                for p in &parts[1..] {
                    for f in &p.fields {
                        if !merged.fields.iter().any(|m| m.name == f.name) {
                            merged.fields.push(f.clone());
                        }
                    }
                }
                c.push(merged);
            }
            let _ = cols.try_set(c);
        }
    });
    cols
}

/// Noms lisibles d'un champ lu dans un contenu (D-184) : collection (par son dossier) et champ
/// (libellé du schéma), sinon le nom du champ mis en forme.
pub(crate) fn cms_names(cols: &[CmsCollection], folder: &str, expr: &str) -> (String, String) {
    let col = cols
        .iter()
        .find(|c| !c.id.contains('+') && c.folder.trim_matches('/') == folder.trim_matches('/'));
    let head = revel_edit::overrides::split(expr)
        .map(|(_, e)| e)
        .unwrap_or(expr);
    let head = head.split('|').next().unwrap_or("").trim();
    let name = head.rsplit('.').next().unwrap_or(head);
    let field = col
        .and_then(|c| c.fields.iter().find(|f| f.name == name))
        .and_then(|f| f.label.clone())
        .unwrap_or_else(|| revel_edit::overrides::field_label(expr));
    let collection = col.map(label_of).unwrap_or_else(|| {
        let last = folder
            .trim_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(folder);
        crate::names::humanize(last)
    });
    (collection, field)
}

pub(crate) fn label_of(c: &CmsCollection) -> String {
    c.label
        .clone()
        .unwrap_or_else(|| crate::site::capitalize(&c.id))
}

/// Réglages propres à l'élément sélectionné (en plus des réglages communs).
pub fn extra_settings(ui: Ui, ed: Ed, at: Target, node: Node) -> Option<AnyView> {
    let mut parts: Vec<AnyView> = Vec::new();
    // Zone du site (en-tête, pied) : montrée ou non sur la page affichée (D-182).
    if let Some(v) = zone_settings(ui, ed, &at) {
        parts.push(v);
    }
    // Outil à plusieurs fonctions (D-172) : la fonction se choisit ici, pas dans « Ajouter ».
    if let Some(v) = tool_function(ui, ed, &at, &node) {
        parts.push(v);
    }
    // Élément construit fermé (D-159) : « Décomposer » en tête de ses réglages.
    let widget = revel_model::is_widget(&node);
    if let Node::Element {
        id,
        tag,
        attrs,
        children,
    } = &node
    {
        let lower = tag.to_ascii_lowercase();
        if lower == "svg"
            && let Some(name) = attr(attrs, "data-icon").and_then(|a| a.value.clone())
        {
            let at = at.clone();
            parts.push(view! { <IconPicker at current=name/> }.into_any());
        }
        if let [Node::Html { html }] = children.as_slice()
            && lower != "svg"
        {
            parts.push(embed_code(ui, ed, at.clone(), html.clone()));
        }
        if lower == "video" {
            parts.push(video_options(ui, ed, at.clone(), attrs.clone()));
        }
        // Champ de formulaire : type, nom, obligatoire, texte d'exemple, saisie automatique.
        if crate::inspector::FIELD_TAGS.contains(&lower.as_str()) {
            parts.push(field_settings(ui, ed, at.clone(), &lower, attrs));
        }
        // Média au survol : le média qui change, et les éléments qui le changent.
        if let Some(v) = crate::hovermedia::display_settings(ui, ed, &at, &lower, attrs) {
            parts.push(v);
        }
        if let Some(v) = crate::hovermedia::trigger_settings(ui, ed, &at, &lower, attrs) {
            parts.push(v);
        }
        if attr(attrs, "data-lottie").is_some() {
            parts.push(lottie_options(ui, ed, at.clone(), attrs.clone()));
        }
        // Modèle 3D et scène Marmoset (D-160).
        if let Some(v) = crate::model3d::settings(ui, ed, &at, &node) {
            parts.push(v);
        }
        if let Some(v) = crate::model3d::mview_settings(ui, ed, &at, &node) {
            parts.push(v);
        }
        if let Some(v) = attr(attrs, "data-revel-video") {
            parts.push(video_player_options(
                ui,
                ed,
                at.clone(),
                v.value.clone().unwrap_or_default(),
            ));
        }
        // Défilement (animation en boucle « ticker »), carrousel et diaporama.
        if attr(attrs, "data-revel-loop")
            .and_then(|a| a.value.as_deref())
            .is_some_and(|v| v.split_whitespace().next() == Some("ticker"))
        {
            let at = at.clone();
            parts.push(view! { <crate::interactions::TickerSettings at=at/> }.into_any());
        }
        if let Some(v) = attr(attrs, "data-revel-tabs") {
            parts.push(tabs_options(
                ui,
                ed,
                at.clone(),
                v.value.clone().unwrap_or_default(),
            ));
        }
        // Formulaire envoyé par le site.
        if let Some(v) = form_settings(ui, ed, &at, &lower, attrs) {
            parts.push(v);
        }
        // Bloc « Formulaire » ou « Panier » (D-177) : les réglages de l'outil qu'il porte sont
        // montrés sur le bloc, sans avoir à chercher l'élément dans les calques.
        let block = attr(attrs, "class")
            .and_then(|a| a.value.clone())
            .unwrap_or_default();
        if block.split_whitespace().any(|c| c == "bloc-formulaire")
            && let Some((id, Node::Element { attrs: fa, .. })) =
                crate::tools::inner_tool(&node, &|t, a| {
                    t == "form" && attr(a, "data-revel-form").is_some()
                })
            && let Some(v) = form_settings(
                ui,
                ed,
                &Target {
                    owner: at.owner.clone(),
                    node: id,
                },
                "form",
                fa,
            )
        {
            parts.push(v);
        }
        if block.split_whitespace().any(|c| c == "bloc-panier")
            && let Some((id, Node::Element { attrs: ca, .. })) =
                crate::tools::inner_tool(&node, &|_, a| attr(a, "data-revel-cart").is_some())
            && let Some(v) = cart_settings(
                ui,
                ed,
                &Target {
                    owner: at.owner.clone(),
                    node: id,
                },
                ca,
            )
        {
            parts.push(v);
        }
        // Panier (D-129) : monnaie du panier ; produit et prix d'un bouton d'achat ; compteur.
        if let Some(v) = cart_settings(ui, ed, &at, attrs) {
            parts.push(v);
        }
        // Rôle dans un élément interactif (filtres, consentement, interrupteur, onglets,
        // carrousel, lecteur, formulaire) : un seul choix « Rôle » tant que l'élément n'en a
        // pas ; ensuite, les réglages de son rôle.
        let has_role = attrs.iter().any(|a| ROLE_ATTRS.contains(&a.name.as_str()));
        if !has_role {
            // (Rôle à donner : réglage avancé, voir `role_settings`.)
            // Texte d'état d'un interrupteur (élément dans un interrupteur).
            if let Some(v) = switch_settings(ui, ed, &at, &lower, attrs, false) {
                parts.push(v);
            }
        } else {
            // Filtres de liste : groupe, bouton, élément filtré, message sans résultat ; fermés
            // (D-159) : leurs boutons aussi.
            if widget {
                if let Some(v) = crate::widgets::filters_settings(ui, ed, &at, &node) {
                    parts.push(v);
                }
            } else if let Some(v) = filter_settings(ui, ed, &at, attrs) {
                parts.push(v);
            }
            // Consentement : bandeau (identifiant de mesure), boutons, lien qui le rouvre ; pas
            // proposé à un autre élément construit (interrupteur).
            if (!widget || attr(attrs, "data-revel-consent").is_some())
                && let Some(v) = consent_settings(ui, ed, &at, &lower, attrs)
            {
                parts.push(v);
            }
            // Interrupteur des sons ou de la musique, et texte de son état ; fermé (D-159) :
            // ce qu'il commande et les textes de son état, en un seul groupe.
            if widget {
                if let Some(v) = crate::widgets::switch_settings(ui, ed, &at, &node) {
                    parts.push(v);
                }
            } else if let Some(v) = switch_settings(ui, ed, &at, &lower, attrs, true) {
                parts.push(v);
            }
            // Élément dans des onglets : onglet (bouton ou lien) ou panneau.
            if attr(attrs, "data-revel-tabs").is_none()
                && (attr(attrs, "data-revel-tab").is_some()
                    || attr(attrs, "data-revel-panel").is_some())
            {
                let role = if attr(attrs, "data-revel-tab").is_some() {
                    "tab"
                } else {
                    "panel"
                };
                parts.push(tab_role(ui, ed, at.clone(), role));
            }
        }
        // Horloge et toile de pixels (D-158).
        if let Some(v) = crate::livetools::clock_settings(ui, ed, &at, attrs) {
            parts.push(v);
        }
        if let Some(v) = crate::livetools::pixels_settings(ui, ed, &at, attrs) {
            parts.push(v);
        }
        // Image révélée au survol (D-125) : manière, halo, les deux images.
        if let Some(v) = attr(attrs, "data-revel-reveal") {
            parts.push(reveal_settings(
                ui,
                ed,
                at.clone(),
                v.value.clone().unwrap_or_default(),
            ));
        }
        // Sommaire (D-172) : niveaux de titres, zone de la page.
        if let Some(v) = attr(attrs, "data-revel-toc") {
            parts.push(toc_settings(
                ui,
                ed,
                at.clone(),
                v.value.clone().unwrap_or_default(),
            ));
        }
        // Accordéon (`details`) : ouvert au chargement, un seul ouvert à la fois (D-123).
        if lower == "details" {
            parts.push(accordion_settings(ui, ed, at.clone(), attrs));
        }
        // Menu de liens (barre de navigation) : liens réglés ici (D-123).
        if lower == "nav"
            && let Some(menu) = links_box(id.as_deref(), children)
        {
            let t = Target {
                owner: at.owner.clone(),
                node: menu,
            };
            parts.push(view! { <crate::items::ItemsList at=t links=true/> }.into_any());
        }
        // Accordéon et barre de navigation (D-176), textes des parties des autres outils fermés.
        if let Some(v) = crate::tools::accordion(ui, ed, &at, &node) {
            parts.push(v);
        }
        if let Some(v) = crate::tools::navbar(ui, ed, &at, &node) {
            parts.insert(0, v);
        }
        if let Some(v) = crate::tools::texts(ui, ed, &at, &node) {
            parts.push(v);
        }
        if let Some(v) = crate::tools::locales(ui, ed, &at, &node) {
            parts.push(v);
        }
        if let Some(v) = crate::tools::video_player(ui, ed, &at, &node) {
            parts.push(v);
        }
        // Carrousel (D-174) : fonction, images, affichage, commandes, défilement.
        if attr(attrs, "data-revel-slider").is_some()
            && let Some(v) = crate::carousel::settings(ui, ed, &at, &node)
        {
            parts.push(v);
        }
        // Liste de collection : son conteneur (une seule boucle dedans).
        let loops: Vec<&Node> = children
            .iter()
            .filter(|c| matches!(c, Node::Each { .. }))
            .collect();
        if let [
            Node::Each {
                id: Some(id), expr, ..
            },
        ] = loops.as_slice()
            && revel_edit::siteinfo::menu_of(expr).is_none()
            && expr.trim() != "locales"
        {
            parts.push(collection_list(
                ui,
                ed,
                Target {
                    owner: at.owner.clone(),
                    node: id.clone(),
                },
                expr.clone(),
            ));
        }
    }
    if let Node::Each {
        expr, id: Some(_), ..
    } = &node
        && revel_edit::siteinfo::menu_of(expr).is_none()
    {
        parts.push(collection_list(ui, ed, at.clone(), expr.clone()));
    }
    // Utilisation d'un composant dans une liste : élément filtré par les filtres de la page.
    if let Node::Use { attrs, .. } = &node
        && let Some(v) = filter_settings(ui, ed, &at, attrs)
    {
        parts.push(v);
    }
    // Utilisation d'un composant qui est un onglet ou un panneau : ce qu'il montre.
    if matches!(node, Node::Use { .. })
        && let Some(v) = tab_pairing(ui, ed, &at)
    {
        parts.push(view! { <div class="insp-group">{v}</div> }.into_any());
    }
    if let Some(b) = item_bindings(ui, ed, &at, &node) {
        parts.push(b);
    }
    // Informations du site (texte, lien, image, menu) et liens d'un menu.
    if let Some(b) = crate::siteinfo::element_settings(ui, ed, &at, &node) {
        parts.push(b);
    }
    if widget {
        // Images de ses parties (couverture d'un lecteur) ; celles d'une image révélée ont
        // leurs propres réglages.
        let reveal = matches!(&node, Node::Element { attrs, .. } if attr(attrs, "data-revel-reveal").is_some());
        // Carrousel et défilement : leurs images sont dans leur liste (un clic choisit l'image).
        let listed = matches!(&node, Node::Element { attrs, .. }
            if attr(attrs, "data-revel-slider").is_some() || attr(attrs, "data-revel-loop").is_some());
        if !reveal
            && !listed
            && let Some(v) = crate::widgets::images_settings(ui, ed, &at, &node)
        {
            parts.push(v);
        }
    }
    (!parts.is_empty()).then(|| parts.into_iter().collect_view().into_any())
}

// ------------------------------------------------------------------ sommaire

/// Réglages d'un sommaire (`data-revel-toc="h2 h3 in:.zone"`) : titres repris, zone lue.
fn toc_settings(ui: Ui, ed: Ed, at: Target, value: String) -> AnyView {
    let at = StoredValue::new(at);
    let value = RwSignal::new(value);
    let levels = move || {
        let v = value.get();
        let l: Vec<String> = v
            .split_whitespace()
            .filter(|o| o.len() == 2 && o.starts_with('h'))
            .map(str::to_string)
            .collect();
        if l.is_empty() {
            vec!["h2".to_string()]
        } else {
            l
        }
    };
    let zone = move || {
        value
            .get()
            .split_whitespace()
            .find_map(|o| o.strip_prefix("in:").map(str::to_string))
            .unwrap_or_default()
    };
    let write = move |levels: Vec<String>, zone: String| {
        let mut parts = levels;
        if !zone.is_empty() {
            parts.push(format!("in:{zone}"));
        }
        let v = parts.join(" ");
        value.set(v.clone());
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: "data-revel-toc".into(),
                value: Some(v),
            },
            false,
        );
    };
    let classes = ed.doc.with_untracked(|d| {
        d.as_ref()
            .map(|d| {
                let mut c: Vec<String> = d
                    .style_rules()
                    .filter_map(|r| {
                        let c = r.selector.trim().strip_prefix('.')?;
                        c.chars()
                            .all(|x| x.is_alphanumeric() || x == '-' || x == '_')
                            .then(|| c.to_string())
                    })
                    .collect();
                c.sort();
                c.dedup();
                c
            })
            .unwrap_or_default()
    });
    view! {
        <div class="insp-group toc-settings">
            <p class="panel-title">{ui.t("insert-toc")}</p>
            <div class="style-row">
                <span class="style-label" title=ui.t("toc-levels-help")><span class="style-label-text">{ui.t("toc-levels")}</span></span>
                <div class="style-control">
                    <div class="seg" role="group">
                        {["h2", "h3", "h4"].into_iter().map(|h| {
                            let on = move || levels().iter().any(|l| l == h);
                            view! {
                                <button type="button" class:is-on=on aria-pressed=move || on().to_string()
                                    on:click=move |_| {
                                        let mut l = levels();
                                        if l.iter().any(|x| x == h) {
                                            if l.len() > 1 { l.retain(|x| x != h); }
                                        } else {
                                            l.push(h.to_string());
                                            l.sort();
                                        }
                                        write(l, zone());
                                    }>{h.to_uppercase()}</button>
                            }
                        }).collect_view()}
                    </div>
                </div>
            </div>
            <div class="style-row">
                <span class="style-label" title=ui.t("toc-zone-help")><span class="style-label-text">{ui.t("toc-zone")}</span></span>
                <div class="style-control">
                    <div class="select">
                        <select class="input" on:change=move |e| write(levels(), event_target_value(&e))>
                            <option value="" selected=move || zone().is_empty()>{ui.t("toc-zone-page")}</option>
                            {classes.into_iter().map(|c| {
                                let v = format!(".{c}");
                                let sel = { let v = v.clone(); move || zone() == v };
                                view! { <option value=v.clone() selected=sel>{v.clone()}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                </div>
            </div>
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ fonction d'un outil

/// Famille d'un élément construit par un outil à plusieurs fonctions, et sa fonction actuelle.
/// Fonctions d'une famille d'outils : préréglage, libellé, aide.
type ToolFunctions = &'static [(Preset, &'static str, &'static str)];

fn tool_family(node: &Node) -> Option<(ToolFunctions, Preset)> {
    const VIDEO: &[(Preset, &str, &str)] = &[
        (Preset::Video, "fn-video", "fn-video-help"),
        (
            Preset::VideoPlayer,
            "fn-video-player",
            "fn-video-player-help",
        ),
        (Preset::HoverMedia, "fn-hover-media", "fn-hover-media-help"),
    ];
    const IMAGE: &[(Preset, &str, &str)] = &[
        (Preset::Image, "fn-image", "fn-image-help"),
        (Preset::Reveal, "fn-reveal", "fn-reveal-help"),
    ];
    match node {
        Node::Image(_) => Some((IMAGE, Preset::Image)),
        Node::Element { tag, attrs, .. } => {
            if attr(attrs, "data-revel-reveal").is_some() {
                Some((IMAGE, Preset::Reveal))
            } else if attr(attrs, "data-revel-video").is_some() {
                Some((VIDEO, Preset::VideoPlayer))
            } else if tag.eq_ignore_ascii_case("video") {
                if attr(attrs, revel_edit::hover::DISPLAY).is_some() {
                    Some((VIDEO, Preset::HoverMedia))
                } else {
                    Some((VIDEO, Preset::Video))
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Choix « Fonction » d'un outil : l'élément est remplacé par l'autre forme de l'outil, son
/// média repris (D-172).
fn tool_function(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    let (family, current) = tool_family(node)?;
    // Image dans une liste, un lecteur ou une image révélée : pas de changement de fonction.
    let inside = ed.doc.with_untracked(|d| {
        d.as_ref().is_some_and(|d| {
            inside_attr(d, at, "data-revel-video")
                || inside_attr(d, at, "data-revel-reveal")
                || in_loop(d, at)
        })
    });
    if inside {
        return None;
    }
    let at = StoredValue::new(at.clone());
    let buttons = family
        .iter()
        .map(|&(p, label, help)| {
            let on = p == current;
            view! {
                <button type="button" role="radio" class:is-on=on aria-checked=on.to_string()
                    title=ui.t(help) data-fn=label
                    on:click=move |_| {
                        if on {
                            return;
                        }
                        let place = ed.doc.with_untracked(|d| {
                            d.as_ref().and_then(|d| revel_edit::address::locate(d, &at.get_value()).ok())
                        });
                        if let Some((parent, index)) = place {
                            ed.apply(
                                ui,
                                Command::ConvertPreset { parent, index, preset: p, text: if text_of(p).is_empty() { String::new() } else { ui.now(text_of(p), &[]) } },
                                false,
                            );
                        }
                    }>{ui.t(label)}</button>
            }
        })
        .collect_view();
    Some(view! {
        <div class="insp-group tool-function">
            <div class="style-row">
                <span class="style-label" title=ui.t("fn-help")><span class="style-label-text">{ui.t("fn-title")}</span></span>
                <div class="style-control">
                    <div class="segmented segmented--small" role="radiogroup" aria-label=ui.t("fn-title")>{buttons}</div>
                </div>
            </div>
        </div>
    }.into_any())
}

/// Texte par défaut d'une forme d'outil (libellé du bouton de lecture).
fn text_of(p: Preset) -> &'static str {
    match p {
        Preset::VideoPlayer => "insert-text-video-player",
        Preset::Image => "insert-text-image",
        _ => "",
    }
}

// ------------------------------------------------------------------ icône

/// Choix d'icône (D-125) : onglets (flèches, interface, contact, médias, objets, création) et
/// onglet des icônes importées (fichiers SVG des médias du site) avec « Importer ».
#[component]
pub(crate) fn IconPicker(
    at: Target,
    current: String,
    /// Autres icônes changées en même temps (la même dans chaque volet d'un accordéon).
    #[prop(optional)]
    also: Vec<Target>,
) -> impl IntoView {
    const CUSTOM: &str = "icons-custom";
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let at = StoredValue::new(at);
    let also = StoredValue::new(also);
    let all = move || {
        let mut l = vec![at.get_value()];
        l.extend(also.get_value());
        l
    };
    let current = StoredValue::new(current);
    // Onglet ouvert : celui de l'icône en place, sinon le premier.
    let first = current.with_value(|c| {
        if c.starts_with("file:") {
            CUSTOM
        } else {
            revel_edit::icons::GROUPS
                .iter()
                .find(|(_, l)| l.contains(&c.as_str()))
                .map(|(k, _)| *k)
                .unwrap_or(revel_edit::icons::GROUPS[0].0)
        }
    });
    let tab = RwSignal::new(first);
    let choose = move |name: &'static str, path: &'static str| {
        let cmd = ed.doc.with_untracked(|d| {
            d.as_ref().map(|d| Command::Batch {
                commands: all()
                    .iter()
                    .map(|at| revel_edit::icons::set_icon(d, at, name, "0 0 24 24", path, true))
                    .collect(),
            })
        });
        if let Some(c) = cmd {
            ed.apply(ui, c, false);
        }
    };
    // Icônes importées : fichiers SVG des médias du site.
    let svgs = RwSignal::new(Vec::<String>::new());
    let reload = move || {
        let path = ed.path.get_value();
        spawn_local(async move {
            if let Ok(list) = api::edit_media(&path).await
                && !svgs.is_disposed()
            {
                svgs.set(
                    list.into_iter()
                        .map(|m| m.url)
                        .filter(|u| u.to_ascii_lowercase().ends_with(".svg"))
                        .collect(),
                );
            }
        });
    };
    reload();
    let base = move || ed.base.get().unwrap_or_default();
    let choose_file = move |url: String| {
        let src = crate::library::thumb_src(&base(), &url);
        spawn_local(async move {
            let Some(text) = fetch_text(&src).await else {
                return;
            };
            let Some((vb, inner)) = revel_edit::icons::clean_svg(&text) else {
                ui.fail(revel_protocol::Failure::new("error-icon-not-svg"));
                return;
            };
            let cmd = ed.doc.with_untracked(|d| {
                d.as_ref().map(|d| Command::Batch {
                    commands: all()
                        .iter()
                        .map(|at| {
                            revel_edit::icons::set_icon(
                                d,
                                at,
                                &format!("file:{url}"),
                                &vb,
                                &inner,
                                false,
                            )
                        })
                        .collect(),
                })
            });
            if let Some(c) = cmd {
                ed.apply(ui, c, false);
            }
        });
    };
    let import = move |_| {
        crate::files::pick(".svg,image/svg+xml", true, move |fs| {
            let path = ed.path.get_value();
            spawn_local(async move {
                let mut first = None;
                for f in fs {
                    let Ok(bytes) = crate::files::read(&f).await else {
                        continue;
                    };
                    if revel_edit::icons::clean_svg(&String::from_utf8_lossy(&bytes)).is_none() {
                        ui.fail(revel_protocol::Failure::new("error-icon-not-svg"));
                        continue;
                    }
                    match api::edit_import_media(&path, &f.name(), &bytes).await {
                        Ok(m) => {
                            first.get_or_insert(m.url);
                        }
                        Err(e) => ui.fail(e),
                    }
                }
                reload();
                tab.set(CUSTOM);
                if let Some(u) = first {
                    choose_file(u);
                }
            });
        });
    };
    view! {
        <div class="insp-group icon-picker">
            <p class="panel-title">{ui.t("block-icon")}</p>
            <div class="icon-tabs" role="tablist" aria-label=ui.t("block-icon")>
                {revel_edit::icons::GROUPS.iter().map(|(k, _)| *k).chain(std::iter::once(CUSTOM)).map(|k| view! {
                    <button role="tab" class="icon-tab" data-tab=k class:is-on=move || tab.get() == k
                        aria-selected=move || (tab.get() == k).to_string()
                        on:click=move |_| tab.set(k)>{ui.t(k)}</button>
                }).collect_view()}
            </div>
            {move || {
                let t = tab.get();
                if t == CUSTOM {
                    let list = svgs.get();
                    view! {
                        <div class="icon-grid icon-grid--custom" role="listbox" aria-label=ui.t(CUSTOM)>
                            {list.into_iter().map(|u| {
                                let on = current.with_value(|c| c.strip_prefix("file:") == Some(u.as_str()));
                                let name = u.rsplit('/').next().unwrap_or(&u).to_string();
                                let u2 = u.clone();
                                view! {
                                    <button type="button" class="icon-choice" class:is-on=on title=name.clone() aria-label=name data-file=u.clone()
                                        on:click=move |_| choose_file(u2.clone())>
                                        <img alt="" src=crate::library::thumb_src(&base(), &u)/>
                                    </button>
                                }
                            }).collect_view()}
                            <button type="button" class="icon-choice icon-import" data-action="icons-import" title=ui.t("icons-import")
                                aria-label=ui.t("icons-import") on:click=import>{icons::upload()}</button>
                        </div>
                    }.into_any()
                } else {
                    let names = revel_edit::icons::GROUPS.iter().find(|(k, _)| *k == t).map(|(_, l)| *l).unwrap_or(&[]);
                    view! {
                        <div class="icon-grid" role="listbox" aria-label=ui.t(t)>
                            {names.iter().filter_map(|n| revel_edit::icons::ICONS.iter().find(|(x, _)| x == n)).map(|(name, path)| {
                                let on = current.with_value(|c| c == name);
                                view! {
                                    <button type="button" class="icon-choice" class:is-on=on title=*name aria-label=*name data-icon=*name
                                        on:click=move |_| choose(name, path)>
                                        <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2"
                                            stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" inner_html=*path></svg>
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Texte d'un fichier servi par le canevas (icône SVG importée).
async fn fetch_text(url: &str) -> Option<String> {
    use wasm_bindgen::JsCast;
    let resp = wasm_bindgen_futures::JsFuture::from(window().fetch_with_str(url))
        .await
        .ok()?;
    let resp: web_sys::Response = resp.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().ok()?)
        .await
        .ok()?;
    text.as_string()
}

// ------------------------------------------------------------------ code HTML

fn embed_code(ui: Ui, ed: Ed, at: Target, html: String) -> AnyView {
    let at = StoredValue::new(at);
    view! {
        <div class="insp-group">
            <p class="panel-title" title=ui.t("block-embed-help")>{ui.t("block-embed")}</p>
            <textarea class="input embed-code" rows="7" spellcheck="false" prop:value=html
                aria-label=ui.t("block-embed")
                on:change=move |e| {
                    let at = at.get_value();
                    ed.apply(ui, Command::Replace {
                        parent: Parent::of(&at, Branch::Children),
                        index: 0,
                        node: Node::Html { html: event_target_value(&e) },
                    }, false);
                }></textarea>
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ vidéo

fn video_options(ui: Ui, ed: Ed, at: Target, attrs: Vec<Attr>) -> AnyView {
    let has = |n: &str| attr(&attrs, n).is_some();
    let at = StoredValue::new(at);
    // Attribut booléen ajouté ou retiré (lecture automatique : sans le son, sinon refusée).
    let toggle = move |names: &'static [&'static str], on: bool| {
        let at = at.get_value();
        let present = ed.doc.with_untracked(|d| {
            d.as_ref()
                .and_then(|d| revel_edit::address::node(d, &at).ok())
                .map(|n| match n {
                    Node::Element { attrs, .. } => {
                        attrs.iter().map(|a| a.name.clone()).collect::<Vec<_>>()
                    }
                    _ => Vec::new(),
                })
                .unwrap_or_default()
        });
        let commands: Vec<Command> = names
            .iter()
            .filter(|n| present.iter().any(|p| p == *n) != on)
            .map(|n| {
                if on {
                    Command::SetAttr {
                        at: at.clone(),
                        name: n.to_string(),
                        value: None,
                    }
                } else {
                    Command::RemoveAttr {
                        at: at.clone(),
                        name: n.to_string(),
                    }
                }
            })
            .collect();
        if !commands.is_empty() {
            ed.apply(ui, Command::Batch { commands }, false);
        }
    };
    let row = move |key: &'static str, names: &'static [&'static str], on: bool| {
        view! {
            <label class="view-line">
                <span>{ui.t(key)}</span>
                <span class="switch">
                    <input type="checkbox" prop:checked=on on:change=move |e| toggle(names, event_target_checked(&e))/>
                    <span class="switch-track"></span>
                </span>
            </label>
        }
    };
    // Source (fichier du site ou adresse) : `data-src` quand la vidéo est chargée tard, sinon `src`.
    let src_attr: &'static str = if has("data-src") { "data-src" } else { "src" };
    let value_of = |n: &str| {
        attr(&attrs, n)
            .and_then(|a| a.value.clone())
            .unwrap_or_default()
    };
    let src = value_of(src_attr);
    let poster = value_of("poster");
    let preload = value_of("preload");
    let set_attr = move |name: &'static str, v: String| {
        let at = at.get_value();
        ed.apply(
            ui,
            if v.trim().is_empty() {
                Command::RemoveAttr {
                    at,
                    name: name.into(),
                }
            } else {
                Command::SetAttr {
                    at,
                    name: name.into(),
                    value: Some(v.trim().to_string()),
                }
            },
            false,
        );
    };
    let base = crate::library::canvas_base(ui, &ui.site_path().unwrap_or_default());
    let poster_thumb = {
        let p = poster.clone();
        move || {
            (!p.trim().is_empty())
                .then(|| base.get().map(|b| crate::library::thumb_src(&b, p.trim())))
                .flatten()
        }
    };
    view! {
        <div class="insp-group">
            <p class="panel-title">{ui.t("block-video")}</p>
            <div class="field">
                <span class="label" title=ui.t("block-video-src-help")>{ui.t("block-video-src")}</span>
                <span class="field-line">
                    <input class="input" type="text" spellcheck="false" prop:value=src placeholder="/media/film.mp4"
                        on:change=move |e| set_attr(src_attr, event_target_value(&e))/>
                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("picker-choose-video") aria-label=ui.t("picker-choose-video")
                        on:click=move |_| crate::library::pick_kind(ui, revel_protocol::MediaKind::Video, move |u| set_attr(src_attr, u))>{icons::video()}</button>
                </span>
            </div>
            <div class="field">
                <span class="label" title=ui.t("block-video-poster-help")>{ui.t("block-video-poster")}</span>
                <span class="field-line">
                    {move || poster_thumb().map(|s| view! { <img class="cms-thumb" src=s alt=""/> })}
                    <input class="input" type="text" spellcheck="false" prop:value=poster
                        on:change=move |e| set_attr("poster", event_target_value(&e))/>
                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("picker-choose-image") aria-label=ui.t("picker-choose-image")
                        on:click=move |_| crate::library::pick_media(ui, true, None, move |u| set_attr("poster", u))>{icons::image()}</button>
                </span>
            </div>
            {row("block-video-controls", &["controls"], has("controls"))}
            {row("block-video-autoplay", &["autoplay", "muted", "playsinline"], has("autoplay"))}
            {row("block-video-loop", &["loop"], has("loop"))}
            {row("block-video-muted", &["muted"], has("muted"))}
            <label class="field">
                <span class="label" title=ui.t("block-video-preload-help")>{ui.t("block-video-preload")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| set_attr("preload", event_target_value(&e))>
                        {[("", "block-video-preload-default"), ("none", "block-video-preload-none"), ("metadata", "block-video-preload-metadata"), ("auto", "block-video-preload-auto")]
                            .into_iter().map(|(v, k)| view! { <option value=v prop:selected=preload == v>{ui.t(k)}</option> }).collect_view()}
                    </select>
                </div>
            </label>
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ lecteur vidéo

/// Lecteur vidéo : adresse YouTube, Vimeo ou fichier du site (`data-revel-video`), lancée à
/// l'appui sur la couverture (image et bouton : calques ordinaires).
fn video_player_options(ui: Ui, ed: Ed, at: Target, url: String) -> AnyView {
    let at = StoredValue::new(at);
    let set = move |v: String| {
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: "data-revel-video".into(),
                value: Some(v.trim().to_string()),
            },
            false,
        );
    };
    let pick = move || crate::library::pick_media(ui, false, crate::library::media_shelf(ed), set);
    let kind = if url.contains("youtu") {
        "YouTube"
    } else if url.contains("vimeo.com") {
        "Vimeo"
    } else if url.is_empty() {
        ""
    } else {
        "file"
    };
    view! {
        <div class="insp-group">
            <p class="panel-title">{ui.t("block-video-player")}</p>
            <div class="field">
                <span class="label" title=ui.t("block-video-player-help")>{ui.t("block-video-player-url")}</span>
                <span class="field-line">
                    <input class="input input--mono" spellcheck="false" prop:value=url.clone()
                        placeholder="https://www.youtube.com/watch?v=…"
                        on:change=move |e| set(event_target_value(&e))/>
                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("block-video-player-file")
                        aria-label=ui.t("block-video-player-file") on:click=move |_| pick()>{crate::icons::play()}</button>
                </span>
            </div>
            {(!kind.is_empty()).then(|| view! {
                <p class="muted small">{if kind == "file" { ui.tx("block-video-player-file", &[]) } else { kind.to_string() }}</p>
            })}
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ filtres de liste

/// Le site contient un élément qui porte l'attribut `name` (groupe de filtres, bandeau).
fn site_has_attr(d: &Site, name: &str) -> bool {
    let mut found = false;
    let mut look = |nodes: &[Node]| {
        revel_model::ids::walk(nodes, &mut |n| {
            if let Node::Element { attrs, .. } = n
                && attrs.iter().any(|a| a.name == name)
            {
                found = true;
            }
        });
    };
    for p in &d.pages {
        look(&p.body);
    }
    for l in &d.layouts {
        look(&l.body);
    }
    for c in &d.components {
        look(&c.body);
    }
    found
}

/// Rôle d'un élément dans les filtres de liste (« Filtres » de Finsweet pour Webflow) : groupe
/// (paramètre de l'adresse), bouton (valeur écrite ou champ), élément filtré (champ liste du
/// contenu), message sans résultat. Proposé quand l'élément a un rôle ou que le site a un groupe.
fn filter_settings(ui: Ui, ed: Ed, at: &Target, attrs: &[Attr]) -> Option<AnyView> {
    let value = |n: &str| attr(attrs, n).map(|a| a.value.clone().unwrap_or_default());
    let role = if attr(attrs, "data-revel-filters").is_some() {
        "group"
    } else if attr(attrs, "data-revel-filter").is_some() {
        "button"
    } else if attr(attrs, "data-revel-tags").is_some() {
        "item"
    } else if attr(attrs, "data-revel-filter-empty").is_some() {
        "empty"
    } else {
        ""
    };
    let (has_group, list, in_list) = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        Some((
            site_has_attr(d, "data-revel-filters"),
            binding_context(d, at),
            enclosing_list(d, at).is_some(),
        ))
    })?;
    // Sans rôle : proposé aux éléments d'une liste quand la page a des filtres (les boutons et le
    // groupe viennent de l'élément « Filtres de liste » du panneau Ajouter).
    // (Pas sur une utilisation de composant : le rôle se règle sur un élément, D-181.)
    let is_use = ed.doc.with_untracked(|d| {
        d.as_ref()
            .and_then(|d| revel_edit::address::node(d, at).ok())
            .is_some_and(|n| matches!(n, Node::Use { .. }))
    });
    if role.is_empty() && (is_use || !(has_group && in_list)) {
        return None;
    }
    let names = [
        ("group", "data-revel-filters"),
        ("button", "data-revel-filter"),
        ("item", "data-revel-tags"),
        ("empty", "data-revel-filter-empty"),
    ];
    let at = StoredValue::new(at.clone());
    let set_role = move |v: String| {
        let at = at.get_value();
        let mut commands = Vec::new();
        for (r, name) in names {
            if r == role && v != r {
                commands.push(Command::RemoveAttr {
                    at: at.clone(),
                    name: name.into(),
                });
            }
            if r == v && role != r {
                commands.push(Command::SetAttr {
                    at: at.clone(),
                    name: name.into(),
                    value: match r {
                        "group" => Some("filtre".into()),
                        "button" | "item" => Some(String::new()),
                        _ => None,
                    },
                });
            }
        }
        if !commands.is_empty() {
            ed.apply(ui, Command::Batch { commands }, false);
        }
    };
    let set_value = move |name: &'static str, v: String| {
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: name.into(),
                value: Some(v),
            },
            false,
        );
    };
    // Champs du contenu de la liste qui entoure l'élément (valeur d'un bouton, étiquettes).
    let cols = collections(ed);
    let fields = move |many: bool| -> Vec<(String, String)> {
        let Some((col, prefix, _)) = list.clone() else {
            return Vec::new();
        };
        cols.with(|cs| {
            cs.iter()
                .find(|c| c.id == col)
                .map(|c| {
                    c.fields
                        .iter()
                        .filter(|f| f.list == many)
                        .map(|f| {
                            (
                                format!("{prefix}{}", f.name),
                                f.label.clone().unwrap_or_else(|| f.name.clone()),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
    };
    let inner = |v: &str| {
        v.trim()
            .strip_prefix("{{")
            .and_then(|x| x.strip_suffix("}}"))
            .map(|x| x.trim().to_string())
    };
    let detail = match role {
        "group" => {
            let param = value("data-revel-filters").unwrap_or_default();
            Some(view! {
                <label class="field" title=ui.t("filters-param-help")>
                    <span class="label">{ui.t("filters-param")}</span>
                    <input class="input input--mono" spellcheck="false" prop:value=param
                        on:change=move |e| set_value("data-revel-filters", event_target_value(&e).trim().to_string())/>
                </label>
            }.into_any())
        }
        "button" => {
            let v = value("data-revel-filter").unwrap_or_default();
            let bound = inner(&v);
            let written = if bound.is_some() {
                String::new()
            } else {
                v.clone()
            };
            let cur = bound.clone().unwrap_or_default();
            Some(view! {
                <label class="field">
                    <span class="label" title=ui.t("filters-value-help")>{ui.t("filters-value")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| {
                            let f = event_target_value(&e);
                            set_value("data-revel-filter", if f.is_empty() { String::new() } else { format!("{{{{ {f} }}}}") });
                        }>
                            <option value="" prop:selected=cur.is_empty()>{ui.t("filters-value-written")}</option>
                            {move || fields(false).into_iter().map(|(e, l)| {
                                let on = e == cur;
                                view! { <option value=e prop:selected=on>{l}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                </label>
                {bound.is_none().then(|| view! {
                    <input class="input" prop:value=written placeholder=ui.t("filters-value-all")
                        aria-label=ui.t("filters-value")
                        on:change=move |e| set_value("data-revel-filter", event_target_value(&e).trim().to_string())/>
                })}
            }.into_any())
        }
        "item" => {
            let v = value("data-revel-tags").unwrap_or_default();
            let cur = inner(&v)
                .and_then(|e| {
                    let (h, t) = e.split_once('|')?;
                    t.trim().starts_with("join").then(|| h.trim().to_string())
                })
                .unwrap_or_default();
            Some(view! {
                <label class="field">
                    <span class="label" title=ui.t("filters-tags-help")>{ui.t("filters-tags")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| {
                            let f = event_target_value(&e);
                            set_value("data-revel-tags", if f.is_empty() { String::new() } else { format!("{{{{ {f} | join(' ') }}}}") });
                        }>
                            <option value="" prop:selected=cur.is_empty()>"—"</option>
                            {move || fields(true).into_iter().map(|(e, l)| {
                                let on = e == cur;
                                view! { <option value=e prop:selected=on>{l}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                </label>
            }.into_any())
        }
        _ => None,
    };
    Some(
        view! {
            <div class="insp-group">
                <label class="field" title=ui.t("filters-role-help")>
                    <span class="label">{ui.t("filters-role")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| set_role(event_target_value(&e))>
                            <option value="" prop:selected=role.is_empty()>{ui.t("filters-role-none")}</option>
                            <option value="group" prop:selected={role == "group"}>{ui.t("filters-role-group")}</option>
                            <option value="button" prop:selected={role == "button"}>{ui.t("filters-role-button")}</option>
                            <option value="item" prop:selected={role == "item"}>{ui.t("filters-role-item")}</option>
                            <option value="empty" prop:selected={role == "empty"}>{ui.t("filters-role-empty")}</option>
                        </select>
                    </div>
                </label>
                {detail}
            </div>
        }
        .into_any(),
    )
}

// ------------------------------------------------------------------ consentement

/// Rôle d'un élément dans le consentement aux cookies : bandeau (affiché tant que le visiteur
/// n'a pas choisi ; identifiant Google Analytics 4 chargé après accord), bouton Accepter,
/// bouton Refuser, lien qui rouvre le bandeau.
fn consent_settings(ui: Ui, ed: Ed, at: &Target, tag: &str, attrs: &[Attr]) -> Option<AnyView> {
    let choice = attr(attrs, "data-revel-consent-choice").and_then(|a| a.value.clone());
    let role = if attr(attrs, "data-revel-consent").is_some() {
        "banner"
    } else if choice.as_deref() == Some("accept") {
        "accept"
    } else if choice.as_deref() == Some("refuse") {
        "refuse"
    } else if attr(attrs, "data-revel-consent-open").is_some() {
        "open"
    } else {
        ""
    };
    let has_banner = ed.doc.with_untracked(|d| {
        d.as_ref()
            .is_some_and(|d| site_has_attr(d, "data-revel-consent"))
    });
    // Sans rôle : proposé aux boutons et aux liens quand le site a un bandeau (le bandeau vient
    // de l'élément « Consentement » du panneau Ajouter).
    if role.is_empty() && !(has_banner && matches!(tag, "a" | "button")) {
        return None;
    }
    let ga = attr(attrs, "data-revel-consent")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let at = StoredValue::new(at.clone());
    let set_role = move |v: String| {
        let at = at.get_value();
        let mut commands = Vec::new();
        let current_attr = match role {
            "banner" => Some("data-revel-consent"),
            "accept" | "refuse" => Some("data-revel-consent-choice"),
            "open" => Some("data-revel-consent-open"),
            _ => None,
        };
        if let Some(n) = current_attr {
            commands.push(Command::RemoveAttr {
                at: at.clone(),
                name: n.into(),
            });
        }
        let (name, value) = match v.as_str() {
            "banner" => ("data-revel-consent", Some(String::new())),
            "accept" => ("data-revel-consent-choice", Some("accept".to_string())),
            "refuse" => ("data-revel-consent-choice", Some("refuse".to_string())),
            "open" => ("data-revel-consent-open", None),
            _ => ("", None),
        };
        if !name.is_empty() {
            commands.push(Command::SetAttr {
                at: at.clone(),
                name: name.into(),
                value,
            });
        }
        if !commands.is_empty() {
            ed.apply(ui, Command::Batch { commands }, false);
        }
    };
    view! {
        <div class="insp-group">
            <label class="field" title=ui.t("consent-role-help")>
                <span class="label">{ui.t("consent-role")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| set_role(event_target_value(&e))>
                        <option value="" prop:selected=role.is_empty()>{ui.t("consent-role-none")}</option>
                        <option value="banner" prop:selected={role == "banner"}>{ui.t("consent-role-banner")}</option>
                        <option value="accept" prop:selected={role == "accept"}>{ui.t("consent-role-accept")}</option>
                        <option value="refuse" prop:selected={role == "refuse"}>{ui.t("consent-role-refuse")}</option>
                        <option value="open" prop:selected={role == "open"}>{ui.t("consent-role-open")}</option>
                    </select>
                </div>
            </label>
            {(role == "banner").then(|| view! {
                <label class="field" title=ui.t("consent-ga-help")>
                    <span class="label">{ui.t("consent-ga")}</span>
                    <input class="input input--mono" spellcheck="false" placeholder="G-XXXXXXXXXX" prop:value=ga
                        on:change=move |e| {
                            ed.apply(ui, Command::SetAttr {
                                at: at.get_value(),
                                name: "data-revel-consent".into(),
                                value: Some(event_target_value(&e).trim().to_string()),
                            }, false);
                        }/>
                </label>
            })}
        </div>
    }
    .into_any()
    .into()
}

/// Le site joue des sons ou de la musique (actions, onglets).
fn site_has_sound(d: &Site) -> bool {
    let mut found = false;
    let mut look = |nodes: &[Node]| {
        revel_model::ids::walk(nodes, &mut |n| {
            if let Node::Element { attrs, .. } | Node::Use { attrs, .. } = n
                && attrs.iter().any(|a| {
                    let v = a.value.as_deref().unwrap_or_default();
                    (a.name == "data-revel-on" && (v.contains("sound:") || v.contains("music:")))
                        || (a.name == "data-revel-tabs" && v.contains("sound="))
                })
            {
                found = true;
            }
        });
    };
    for p in &d.pages {
        look(&p.body);
    }
    for l in &d.layouts {
        look(&l.body);
    }
    for c in &d.components {
        look(&c.body);
    }
    found
}

/// Interrupteur (`data-revel-switch` : musique ou sons, allumé par défaut, choix gardé par le
/// navigateur) et texte de son état (`data-revel-switch-text` : « On | Off »). Proposé aux
/// boutons quand le site joue des sons, et à ce qui est dans un interrupteur.
fn switch_settings(
    ui: Ui,
    ed: Ed,
    at: &Target,
    tag: &str,
    attrs: &[Attr],
    role_select: bool,
) -> Option<AnyView> {
    let channel = attr(attrs, "data-revel-switch").map(|a| a.value.clone().unwrap_or_default());
    let text = attr(attrs, "data-revel-switch-text").map(|a| a.value.clone().unwrap_or_default());
    let (sound, inside, dark) = ed.doc.with_untracked(|d| {
        d.as_ref().map_or((false, false, false), |d| {
            (
                site_has_sound(d),
                inside_attr(d, at, "data-revel-switch"),
                // Couleurs pour le mode sombre (Paramètres › Couleurs) : interrupteur du thème.
                revel_edit::style::find_rule(d, ":root", &Some(crate::sitecolors::DARK.into()))
                    .is_some(),
            )
        })
    });
    let offer_switch =
        role_select && (channel.is_some() || ((sound || dark) && matches!(tag, "button" | "a")));
    let offer_text = text.is_some() || inside;
    if !offer_switch && !offer_text {
        return None;
    }
    let at = StoredValue::new(at.clone());
    let set = move |name: &'static str, v: Option<String>| {
        let at = at.get_value();
        ed.apply(
            ui,
            match v {
                Some(v) => Command::SetAttr {
                    at,
                    name: name.into(),
                    value: Some(v),
                },
                None => Command::RemoveAttr {
                    at,
                    name: name.into(),
                },
            },
            false,
        );
    };
    let cur = channel.clone().unwrap_or_default();
    let parts: Vec<String> = text
        .clone()
        .unwrap_or_else(|| "On|Off".into())
        .split('|')
        .map(str::to_string)
        .collect();
    let (on_text, off_text) = (
        parts.first().cloned().unwrap_or_default(),
        parts.get(1).cloned().unwrap_or_default(),
    );
    let texts = StoredValue::new((on_text.clone(), off_text.clone()));
    let write_text = move |on: Option<String>, off: Option<String>| {
        let (a, b) = texts.get_value();
        let v = format!("{}|{}", on.unwrap_or(a), off.unwrap_or(b));
        set("data-revel-switch-text", Some(v));
    };
    Some(
        view! {
            <div class="insp-group">
                {offer_switch.then(|| view! {
                    <label class="field" title=ui.t("switch-role-help")>
                        <span class="label">{ui.t("switch-role")}</span>
                        <div class="select">
                            <select class="input" on:change=move |e| {
                                let v = event_target_value(&e);
                                set("data-revel-switch", (!v.is_empty()).then_some(v));
                            }>
                                <option value="" prop:selected=cur.is_empty()>{ui.t("switch-none")}</option>
                                <option value="music" prop:selected={cur == "music"}>{ui.t("switch-music")}</option>
                                <option value="sounds" prop:selected={cur == "sounds"}>{ui.t("switch-sounds")}</option>
                                <option value="theme" prop:selected={cur == "theme"}>{ui.t("switch-theme")}</option>
                            </select>
                        </div>
                    </label>
                })}
                {offer_text.then(|| match text.is_some() {
                    true => view! {
                        <label class="field">
                            <span class="label">{ui.t("switch-text-on")}</span>
                            <input class="input" prop:value=on_text.clone()
                                on:change=move |e| write_text(Some(event_target_value(&e)), None)/>
                        </label>
                        <label class="field">
                            <span class="label">{ui.t("switch-text-off")}</span>
                            <input class="input" prop:value=off_text.clone()
                                on:change=move |e| write_text(None, Some(event_target_value(&e)))/>
                        </label>
                        <button class="link-btn" on:click=move |_| set("data-revel-switch-text", None)>{ui.t("switch-text-remove")}</button>
                    }.into_any(),
                    false => view! {
                        <button class="link-btn" title=ui.t("switch-text-help")
                            on:click=move |_| set("data-revel-switch-text", Some("On|Off".into()))>{ui.t("switch-text-add")}</button>
                    }.into_any(),
                })}
            </div>
        }
        .into_any(),
    )
}

// ------------------------------------------------------------------ onglets

/// Un parent de `at` porte `data-revel-tabs`.
fn inside_tabs(d: &Site, at: &Target) -> bool {
    inside_attr(d, at, "data-revel-tabs")
}

/// Un parent de `at` porte l'attribut `name`.
fn inside_attr(d: &Site, at: &Target, name: &str) -> bool {
    let mut cur = at.clone();
    for _ in 0..64 {
        let Ok((parent, _)) = revel_edit::address::locate(d, &cur) else {
            return false;
        };
        let Some(id) = parent.node else {
            return false;
        };
        let t = Target {
            owner: at.owner.clone(),
            node: id,
        };
        if let Ok(Node::Element { attrs, .. }) = revel_edit::address::node(d, &t)
            && attr(attrs, name).is_some()
        {
            return true;
        }
        cur = t;
    }
    false
}

/// Onglets : changement au survol (sinon à l'appui), touches du clavier dans toute la page
/// (flèches, W/A/S/D), son joué à chaque changement.
fn tabs_options(ui: Ui, ed: Ed, at: Target, value: String) -> AnyView {
    let at = StoredValue::new(at);
    let opts = StoredValue::new(value.clone());
    let has = |k: &str| value.split_whitespace().any(|o| o == k);
    let (hover, keys) = (has("hover"), has("keys"));
    let sound = value
        .split_whitespace()
        .find_map(|o| o.strip_prefix("sound="))
        .map(|u| u.replace("%20", " "));
    // Réglage `key` (drapeau, ou `key=valeur`) remplacé ou retiré.
    let put = move |key: &'static str, v: Option<String>| {
        let mut list: Vec<String> = opts
            .get_value()
            .split_whitespace()
            .filter(|o| *o != key && !o.starts_with(&format!("{key}=")))
            .map(str::to_string)
            .collect();
        if let Some(v) = v {
            list.push(if v.is_empty() {
                key.to_string()
            } else {
                format!("{key}={}", revel_edit::actions::url_token(&v))
            });
        }
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: "data-revel-tabs".into(),
                value: Some(list.join(" ")),
            },
            false,
        );
    };
    let flag = move |key: &'static str, title: &'static str, help: &'static str, on: bool| {
        view! {
            <label class="view-line" title=ui.t(help)>
                <span>{ui.t(title)}</span>
                <span class="switch">
                    <input type="checkbox" prop:checked=on
                        on:change=move |e| put(key, event_target_checked(&e).then(String::new))/>
                    <span class="switch-track"></span>
                </span>
            </label>
        }
    };
    // Nom du fichier, sans le volume (`~50`).
    let sound_name = sound.as_deref().map(|u| {
        let u = u.rsplit_once('~').map_or(u, |(a, _)| a);
        u.rsplit('/').next().unwrap_or(u).to_string()
    });
    view! {
        <div class="insp-group">
            <p class="panel-title">{ui.t("tabs-title")}</p>
            {flag("hover", "tabs-hover", "tabs-hover-help", hover)}
            {flag("keys", "tabs-keys", "tabs-keys-help", keys)}
            <div class="view-line" title=ui.t("tabs-sound-help")>
                <span>{ui.t("tabs-sound")}</span>
                <span class="act-sound-name">{sound_name.map(|n| view! { {icons::audio()}<span>{n}</span> })}</span>
                <button class="icon-btn icon-btn--small" title=ui.t("act-sound-add") aria-label=ui.t("act-sound-add")
                    on:click=move |_| crate::library::pick_audio(ui, crate::library::media_shelf(ed), move |url| put("sound", Some(url)))>
                    {icons::audio()}</button>
                {sound.is_some().then(|| view! {
                    <button class="icon-btn icon-btn--small" title=ui.t("act-sound-remove") aria-label=ui.t("act-sound-remove")
                        on:click=move |_| put("sound", None)>{icons::close()}</button>
                })}
            </div>
            {crate::tools::tabs(ui, ed, at.get_value(), opts.get_value())}
        </div>
    }
    .into_any()
}

/// Rôle d'un élément dans des onglets : onglet, panneau ou aucun.
fn tab_role(ui: Ui, ed: Ed, at: Target, role: &'static str) -> AnyView {
    let at = StoredValue::new(at);
    let set = move |v: String| {
        let at = at.get_value();
        let mut commands = Vec::new();
        for (name, key) in [("data-revel-tab", "tab"), ("data-revel-panel", "panel")] {
            if v == key && role != key {
                commands.push(Command::SetAttr {
                    at: at.clone(),
                    name: name.into(),
                    value: None,
                });
            } else if v != key && role == key {
                commands.push(Command::RemoveAttr {
                    at: at.clone(),
                    name: name.into(),
                });
            }
        }
        if commands.is_empty() {
            return;
        }
        ed.apply(ui, Command::Batch { commands }, false);
    };
    let pairing = tab_pairing(ui, ed, &at.get_value());
    view! {
        <div class="insp-group">
            <label class="field" title=ui.t("tabs-role-help")>
                <span class="label">{ui.t("tabs-role")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| set(event_target_value(&e))>
                        <option value="" prop:selected=role.is_empty()>{ui.t("tabs-role-none")}</option>
                        <option value="tab" prop:selected={role == "tab"}>{ui.t("tabs-role-tab")}</option>
                        <option value="panel" prop:selected={role == "panel"}>{ui.t("tabs-role-panel")}</option>
                    </select>
                </div>
            </label>
            {pairing}
        </div>
    }
    .into_any()
}

/// Onglet ou panneau sélectionné : ce qu'il montre ou ce qui le montre (le n-ième onglet montre
/// le n-ième panneau), chacun sélectionnable ; nombres qui ne correspondent pas signalés.
pub(crate) fn tab_pairing(ui: Ui, ed: Ed, at: &Target) -> Option<AnyView> {
    ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let boite = revel_edit::tabs::tabs_box(d, at)?;
        let m = revel_edit::tabs::map(d, &boite);
        let (is_tab, i) = revel_edit::tabs::place_of(d, &m, at)?;
        let here = at.owner.clone();
        let (nt, np) = (m.tabs.len(), m.panels.len());
        let (title, others) = if is_tab {
            (
                ui.now(
                    "tabs-pair-tab",
                    &[("n", (i + 1).to_string()), ("count", nt.to_string())],
                ),
                m.panels_of(i),
            )
        } else {
            (
                ui.now(
                    "tabs-pair-panel",
                    &[("n", (i + 1).to_string()), ("count", np.to_string())],
                ),
                m.tab_of(i).into_iter().collect(),
            )
        };
        let rows: Vec<(Target, String)> = others
            .into_iter()
            .map(|t| {
                let l = crate::hovermedia::label_of(ui, d, &t, &here);
                (t, l)
            })
            .collect();
        let empty = rows.is_empty();
        let mismatch = nt > 0 && np % nt != 0;
        Some(
            view! {
                <div class="field hover-links" data-tabs="pairing">
                    <span class="label">{title}</span>
                    {(!empty).then(|| view! {
                        <ul class="hover-list">
                            {rows.into_iter().map(|(t, l)| view! {
                                <li class="hover-row"><span class="hover-row-name">{l}</span>{crate::hovermedia::select_btn(ui, ed, t)}</li>
                            }).collect_view()}
                        </ul>
                    })}
                    {empty.then(|| icons::empty_tip(ui.tx("map-none", &[]), ui.tx("tabs-pair-none", &[])))}
                    {m.repeated.then(|| icons::note_tip(ui.tx("tabs-pair-repeated", &[])))}
                    {mismatch.then(|| view! {
                        <p class="muted small insp-warn">{ui.now("tabs-pair-mismatch", &[("tabs", nt.to_string()), ("panels", np.to_string())])}</p>
                    })}
                </div>
            }
            .into_any(),
        )
    })
}

// ------------------------------------------------------------------ rôles des parties

/// Rôles d'un élément dans un carrousel ou diaporama (piste, précédent, suivant, compteur,
/// points), dans un lecteur vidéo (couverture qui lance la vidéo), dans un formulaire du site
/// (message après l'envoi) ; nom d'un formulaire du site (`data-revel-form`).
/// Attributs qui donnent un rôle à un élément dans un élément interactif.
const ROLE_ATTRS: &[&str] = &[
    "data-revel-filters",
    "data-revel-filter",
    "data-revel-tags",
    "data-revel-filter-empty",
    "data-revel-consent",
    "data-revel-consent-choice",
    "data-revel-consent-open",
    "data-revel-switch",
    "data-revel-tab",
    "data-revel-panel",
    "data-revel-track",
    "data-revel-prev",
    "data-revel-next",
    "data-revel-count",
    "data-revel-dots",
    "data-revel-play",
    "data-revel-form-status",
    "data-revel-cart-add",
    "data-revel-cart-count",
    "data-revel-cart-item",
    "data-revel-cart-name",
    "data-revel-cart-qty",
    "data-revel-cart-price",
    "data-revel-cart-plus",
    "data-revel-cart-minus",
    "data-revel-cart-remove",
    "data-revel-cart-total",
    "data-revel-cart-empty",
    "data-revel-cart-field",
];

/// Rôle proposé : famille (titre), libellé, attribut posé et sa valeur.
type Offer = (
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
);

/// Rôle de l'élément dans un élément interactif (paramètres avancés) : le choisir s'il n'en a
/// pas ; partie d'un carrousel, d'un lecteur vidéo, d'un formulaire.
pub fn role_settings(ui: Ui, ed: Ed, at: &Target, tag: &str, attrs: &[Attr]) -> Option<AnyView> {
    let lower = tag.to_ascii_lowercase();
    if attrs.iter().any(|a| ROLE_ATTRS.contains(&a.name.as_str())) {
        part_roles(ui, ed, at, attrs)
    } else {
        role_picker(ui, ed, at, &lower, attrs)
    }
}

/// Choix unique du rôle d'un élément qui n'en a pas : rôles possibles à cet endroit, groupés
/// par élément interactif (filtres de liste, consentement, interrupteur, onglets, carrousel,
/// lecteur vidéo, formulaire).
fn role_picker(ui: Ui, ed: Ed, at: &Target, tag: &str, attrs: &[Attr]) -> Option<AnyView> {
    let (cart, in_cart) = ed.doc.with_untracked(|d| {
        d.as_ref().map(|d| {
            (
                site_has_attr(d, "data-revel-cart"),
                attr(attrs, "data-revel-cart").is_none() && inside_attr(d, at, "data-revel-cart"),
            )
        })
    })?;
    let (filters, consent, sound, tabs, slider, player, form) = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let inside = |n: &str| attr(attrs, n).is_none() && inside_attr(d, at, n);
        Some((
            site_has_attr(d, "data-revel-filters") && enclosing_list(d, at).is_some(),
            site_has_attr(d, "data-revel-consent") && matches!(tag, "a" | "button"),
            site_has_sound(d) && matches!(tag, "a" | "button"),
            attr(attrs, "data-revel-tabs").is_none() && inside_tabs(d, at),
            inside("data-revel-slider"),
            inside("data-revel-video"),
            inside("data-revel-form"),
        ))
    })?;
    let mut offers: Vec<Offer> = Vec::new();
    if tabs {
        offers.push(("tabs-role", "tabs-role-tab", "data-revel-tab", None));
        offers.push(("tabs-role", "tabs-role-panel", "data-revel-panel", None));
    }
    if slider {
        offers.extend(
            SLIDER_ROLES
                .iter()
                .map(|(a, k)| ("role-slider", *k, *a, None)),
        );
    }
    if player {
        offers.extend(
            PLAYER_ROLES
                .iter()
                .map(|(a, k)| ("role-player", *k, *a, None)),
        );
    }
    if form {
        offers.extend(FORM_ROLES.iter().map(|(a, k)| ("role-form", *k, *a, None)));
    }
    if in_cart {
        offers.extend(CART_ROLES.iter().map(|(a, k)| ("role-cart", *k, *a, None)));
    } else if cart {
        // Site avec un panier : bouton d'achat (fiche produit) et compteur (barre de navigation).
        if matches!(tag, "a" | "button") {
            offers.push(("role-cart", "role-cart-add", "data-revel-cart-add", None));
        }
        offers.push((
            "role-cart",
            "role-cart-count",
            "data-revel-cart-count",
            None,
        ));
    }
    if filters {
        offers.push((
            "filters-role",
            "filters-role-group",
            "data-revel-filters",
            Some("filtre"),
        ));
        offers.push((
            "filters-role",
            "filters-role-button",
            "data-revel-filter",
            Some(""),
        ));
        offers.push((
            "filters-role",
            "filters-role-item",
            "data-revel-tags",
            Some(""),
        ));
        offers.push((
            "filters-role",
            "filters-role-empty",
            "data-revel-filter-empty",
            None,
        ));
    }
    if consent {
        offers.push((
            "consent-role",
            "consent-role-banner",
            "data-revel-consent",
            Some(""),
        ));
        offers.push((
            "consent-role",
            "consent-role-accept",
            "data-revel-consent-choice",
            Some("accept"),
        ));
        offers.push((
            "consent-role",
            "consent-role-refuse",
            "data-revel-consent-choice",
            Some("refuse"),
        ));
        offers.push((
            "consent-role",
            "consent-role-open",
            "data-revel-consent-open",
            None,
        ));
    }
    if sound {
        offers.push((
            "switch-role",
            "switch-music",
            "data-revel-switch",
            Some("music"),
        ));
        offers.push((
            "switch-role",
            "switch-sounds",
            "data-revel-switch",
            Some("sounds"),
        ));
    }
    if offers.is_empty() {
        return None;
    }
    let at = StoredValue::new(at.clone());
    let list = StoredValue::new(offers.clone());
    let mut groups: Vec<&'static str> = Vec::new();
    for (g, ..) in &offers {
        if !groups.contains(g) {
            groups.push(g);
        }
    }
    Some(
        view! {
            <div class="insp-group" data-role="picker">
                <label class="field" title=ui.t("role-picker-help")>
                    <span class="label">{ui.t("role-picker")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| {
                            let Ok(i) = event_target_value(&e).parse::<usize>() else { return };
                            let Some((_, _, name, value)) = list.with_value(|l| l.get(i).cloned()) else { return };
                            ed.apply(ui, Command::SetAttr {
                                at: at.get_value(),
                                name: name.into(),
                                value: value.map(str::to_string),
                            }, false);
                        }>
                            <option value="" selected=true>{ui.t("tabs-role-none")}</option>
                            {groups.into_iter().map(|g| view! {
                                <optgroup label=ui.t(g)>
                                    {offers.iter().enumerate().filter(|(_, o)| o.0 == g).map(|(i, o)| view! {
                                        <option value=i.to_string()>{ui.t(o.1)}</option>
                                    }).collect_view()}
                                </optgroup>
                            }).collect_view()}
                        </select>
                    </div>
                </label>
            </div>
        }
        .into_any(),
    )
}

const SLIDER_ROLES: &[(&str, &str)] = &[
    ("data-revel-track", "role-slider-track"),
    ("data-revel-prev", "role-slider-prev"),
    ("data-revel-next", "role-slider-next"),
    ("data-revel-count", "role-slider-count"),
    ("data-revel-dots", "role-slider-dots"),
];
const PLAYER_ROLES: &[(&str, &str)] = &[("data-revel-play", "role-player-cover")];
const FORM_ROLES: &[(&str, &str)] = &[("data-revel-form-status", "role-form-status")];
/// Parties d'un panier (D-129).
const CART_ROLES: &[(&str, &str)] = &[
    ("data-revel-cart-item", "role-cart-item"),
    ("data-revel-cart-name", "role-cart-name"),
    ("data-revel-cart-qty", "role-cart-qty"),
    ("data-revel-cart-price", "role-cart-price"),
    ("data-revel-cart-plus", "role-cart-plus"),
    ("data-revel-cart-minus", "role-cart-minus"),
    ("data-revel-cart-remove", "role-cart-remove"),
    ("data-revel-cart-total", "role-cart-total"),
    ("data-revel-cart-empty", "role-cart-empty"),
    ("data-revel-cart-field", "role-cart-field"),
];

fn part_roles(ui: Ui, ed: Ed, at: &Target, attrs: &[Attr]) -> Option<AnyView> {
    let mut groups: Vec<AnyView> = Vec::new();
    let inside = |name: &str| {
        attr(attrs, name).is_none()
            && ed
                .doc
                .with_untracked(|d| d.as_ref().is_some_and(|d| inside_attr(d, at, name)))
    };
    let has = |roles: &[(&str, &str)]| roles.iter().any(|(a, _)| attr(attrs, a).is_some());
    for (box_attr, title, help, roles) in [
        (
            "data-revel-slider",
            "role-slider",
            "role-slider-help",
            SLIDER_ROLES,
        ),
        (
            "data-revel-video",
            "role-player",
            "role-player-help",
            PLAYER_ROLES,
        ),
        ("data-revel-form", "role-form", "role-form-help", FORM_ROLES),
        ("data-revel-cart", "role-cart", "role-cart-help", CART_ROLES),
    ] {
        if inside(box_attr) && has(roles) {
            groups.push(role_select(ui, ed, at.clone(), attrs, title, help, roles));
        }
    }
    (!groups.is_empty()).then(|| groups.into_iter().collect_view().into_any())
}

/// Formulaire : envoyé par le site (nom du formulaire, Paramètres › Formulaires), ou ordinaire.
fn form_settings(ui: Ui, ed: Ed, at: &Target, tag: &str, attrs: &[Attr]) -> Option<AnyView> {
    let mut groups: Vec<AnyView> = Vec::new();
    if tag == "form" {
        let name = attr(attrs, "data-revel-form").map(|a| a.value.clone().unwrap_or_default());
        let at = StoredValue::new(at.clone());
        let set = move |v: Option<String>| {
            let at = at.get_value();
            ed.apply(
                ui,
                match v {
                    Some(v) => Command::SetAttr {
                        at,
                        name: "data-revel-form".into(),
                        value: Some(v),
                    },
                    None => Command::RemoveAttr {
                        at,
                        name: "data-revel-form".into(),
                    },
                },
                false,
            );
        };
        let on = name.is_some();
        groups.push(
            view! {
                <div class="insp-group" data-role="form">
                    <p class="panel-title" title=ui.t("role-form-help")>{ui.t("role-form")}</p>
                    <label class="view-line" title=ui.t("role-form-send-help")>
                        <span>{ui.t("role-form-send")}</span>
                        <span class="switch">
                            <input type="checkbox" prop:checked=on
                                on:change=move |e| set(event_target_checked(&e).then(|| "contact".to_string()))/>
                            <span class="switch-track"></span>
                        </span>
                    </label>
                    {on.then(|| view! {
                        <label class="field">
                            <span class="label" title=ui.t("role-form-name-help")>{ui.t("role-form-name")}</span>
                            <input class="input" prop:value=name.clone().unwrap_or_default()
                                on:change=move |e| { let v = event_target_value(&e).trim().to_string(); set(Some(if v.is_empty() { "contact".into() } else { v })) }/>
                        </label>
                        <button type="button" class="link-btn" on:click=move |_| ui.tab(crate::Tab::Settings(crate::Section::Forms))>{ui.t("role-form-settings")}</button>
                    })}
                </div>
            }
            .into_any(),
        );
    }
    (!groups.is_empty()).then(|| groups.into_iter().collect_view().into_any())
}

/// Valeur d'un attribut réglée par un champ lisible : texte (ou nombre) écrit, ou valeur reliée
/// (champ du contenu, propriété du composant, information du site) avec le même éditeur que
/// l'inspecteur ; bouton pour relier ou délier.
#[allow(clippy::too_many_arguments)]
fn attr_value_field(
    ui: Ui,
    ed: Ed,
    at: &Target,
    attrs: &[Attr],
    name: &'static str,
    label: &'static str,
    help: &'static str,
    number: bool,
) -> AnyView {
    let value = attr(attrs, name)
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    let bound = crate::boundattr::attr_editor(ui, ed, at.clone(), name.into(), value.clone());
    let at2 = StoredValue::new(at.clone());
    let head = |button: Option<AnyView>| {
        view! {
            <span class="field-head">
                <span class="label" title=ui.t(help)>{ui.t(label)}</span>
                {button}
            </span>
        }
    };
    match bound {
        Some(b) => {
            let unlink = crate::boundattr::unlink_button(ui, ed, at.clone(), name.into(), value);
            view! { <div class="field field--bound" data-attr=name>{head(unlink)}{b}</div> }
                .into_any()
        }
        None => {
            let link =
                crate::boundattr::link_button(ui, ed, at.clone(), name.into(), value.clone());
            view! {
                <div class="field" data-attr=name>
                    {head(Some(link))}
                    <input class="input" type=if number { "number" } else { "text" } step="any" prop:value=value
                        on:change=move |e| {
                            let v = event_target_value(&e).trim().to_string();
                            ed.apply(ui, Command::SetAttr { at: at2.get_value(), name: name.into(), value: Some(v) }, false);
                        }/>
                </div>
            }
            .into_any()
        }
    }
}

/// Types d'un champ de saisie proposés (les autres restent affichés tels quels).
const INPUT_TYPES: &[(&str, &str)] = &[
    ("text", "field-type-text"),
    ("email", "field-type-email"),
    ("tel", "field-type-tel"),
    ("url", "field-type-url"),
    ("number", "field-type-number"),
    ("date", "field-type-date"),
    ("checkbox", "field-type-checkbox"),
    ("radio", "field-type-radio"),
    ("file", "field-type-file"),
    ("password", "field-type-password"),
    ("hidden", "field-type-hidden"),
];
/// Saisie automatique proposée par le navigateur (valeurs de `autocomplete`).
const AUTOCOMPLETE: &[(&str, &str)] = &[
    ("", "field-auto-default"),
    ("off", "field-auto-off"),
    ("name", "field-auto-name"),
    ("given-name", "field-auto-given-name"),
    ("family-name", "field-auto-family-name"),
    ("email", "field-auto-email"),
    ("tel", "field-auto-tel"),
    ("organization", "field-auto-organization"),
    ("street-address", "field-auto-street"),
    ("postal-code", "field-auto-postal-code"),
    ("address-level2", "field-auto-city"),
    ("country-name", "field-auto-country"),
];

/// Champ de formulaire (`input`, `textarea`, `select`) : type, nom envoyé, obligatoire, texte
/// d'exemple, saisie automatique, lignes d'une zone de texte.
fn field_settings(ui: Ui, ed: Ed, at: Target, tag: &str, attrs: &[Attr]) -> AnyView {
    let at = StoredValue::new(at);
    let value_of = |n: &str| {
        attr(attrs, n)
            .and_then(|a| a.value.clone())
            .unwrap_or_default()
    };
    let set = move |name: &'static str, v: Option<String>| {
        let at = at.get_value();
        ed.apply(
            ui,
            match v {
                Some(v) => Command::SetAttr {
                    at,
                    name: name.into(),
                    value: Some(v),
                },
                None => Command::RemoveAttr {
                    at,
                    name: name.into(),
                },
            },
            false,
        )
    };
    let kind = if tag == "input" {
        let t = value_of("type");
        if t.is_empty() {
            "text".to_string()
        } else {
            t.to_ascii_lowercase()
        }
    } else {
        String::new()
    };
    let required = attr(attrs, "required").is_some();
    let has_placeholder = !matches!(
        kind.as_str(),
        "checkbox" | "radio" | "file" | "hidden" | "date"
    ) && tag != "select";
    let name = value_of("name");
    let placeholder = value_of("placeholder");
    let auto = value_of("autocomplete");
    let rows = value_of("rows");
    let kind2 = kind.clone();
    view! {
        <div class="insp-group" data-role="field">
            <p class="panel-title" title=ui.t("field-group-help")>{ui.t("field-group")}</p>
            {(tag == "input").then(|| {
                let known = INPUT_TYPES.iter().any(|(v, _)| *v == kind2);
                view! {
                    <label class="field">
                        <span class="label">{ui.t("field-type")}</span>
                        <div class="select">
                            <select class="input" on:change=move |e| set("type", Some(event_target_value(&e)))>
                                {(!known).then(|| view! { <option value=kind2.clone() selected=true>{kind2.clone()}</option> })}
                                {INPUT_TYPES.iter().map(|(v, k)| view! { <option value=*v prop:selected=kind == *v>{ui.t(k)}</option> }).collect_view()}
                            </select>
                        </div>
                    </label>
                }
            })}
            <label class="field">
                <span class="label" title=ui.t("field-name-help")>{ui.t("field-name")}</span>
                <input class="input input--mono" prop:value=name spellcheck="false"
                    on:change=move |e| {
                        let v = event_target_value(&e).trim().to_string();
                        set("name", (!v.is_empty()).then_some(v))
                    }/>
            </label>
            <label class="view-line">
                <span title=ui.t("field-required-help")>{ui.t("field-required")}</span>
                <span class="switch">
                    <input type="checkbox" prop:checked=required on:change=move |e| set("required", event_target_checked(&e).then(String::new))/>
                    <span class="switch-track"></span>
                </span>
            </label>
            {has_placeholder.then(|| view! {
                <label class="field">
                    <span class="label" title=ui.t("field-placeholder-help")>{ui.t("field-placeholder")}</span>
                    <input class="input" prop:value=placeholder
                        on:change=move |e| { let v = event_target_value(&e); set("placeholder", (!v.trim().is_empty()).then_some(v)) }/>
                </label>
            })}
            {(tag == "textarea").then(|| view! {
                <label class="field">
                    <span class="label">{ui.t("field-rows")}</span>
                    <input class="input" type="number" min="1" max="40" prop:value=rows
                        on:change=move |e| { let v = event_target_value(&e).trim().to_string(); set("rows", (!v.is_empty()).then_some(v)) }/>
                </label>
            })}
            {(!matches!(kind.as_str(), "checkbox" | "radio" | "file" | "hidden")).then(|| view! {
                <label class="field">
                    <span class="label" title=ui.t("field-auto-help")>{ui.t("field-auto")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| { let v = event_target_value(&e); set("autocomplete", (!v.is_empty()).then_some(v)) }>
                            {(!AUTOCOMPLETE.iter().any(|(v, _)| *v == auto)).then(|| view! { <option value=auto.clone() selected=true>{auto.clone()}</option> })}
                            {AUTOCOMPLETE.iter().map(|(v, k)| view! { <option value=*v prop:selected=auto == *v>{ui.t(k)}</option> }).collect_view()}
                        </select>
                    </div>
                </label>
            })}
        </div>
    }
    .into_any()
}

/// Monnaies proposées pour un panier (codes ISO 4217).
const CURRENCIES: &[&str] = &[
    "EUR", "USD", "GBP", "CHF", "CAD", "AUD", "JPY", "SEK", "NOK", "DKK",
];

/// Panier (D-129) : monnaie de la boîte du panier ; produit et prix d'un bouton d'achat ;
/// compteur d'articles ; « Retirer ce rôle » pour le bouton et le compteur.
fn cart_settings(ui: Ui, ed: Ed, at: &Target, attrs: &[Attr]) -> Option<AnyView> {
    let mut groups: Vec<AnyView> = Vec::new();
    let atv = StoredValue::new(at.clone());
    let remove = move |names: &'static [&'static str]| {
        ed.apply(
            ui,
            Command::Batch {
                commands: names
                    .iter()
                    .map(|n| Command::RemoveAttr {
                        at: atv.get_value(),
                        name: (*n).into(),
                    })
                    .collect(),
            },
            false,
        )
    };
    let remove_btn = move |names: &'static [&'static str]| {
        view! {
            <button type="button" class="icon-btn icon-btn--small" title=ui.t("role-remove") aria-label=ui.t("role-remove")
                on:click=move |_| remove(names)>{icons::close()}</button>
        }
    };
    if let Some(cur) = attr(attrs, "data-revel-cart").map(|a| a.value.clone().unwrap_or_default()) {
        let cur = if cur.trim().is_empty() {
            "EUR".to_string()
        } else {
            cur.trim().to_uppercase()
        };
        let mut list: Vec<String> = CURRENCIES.iter().map(|c| c.to_string()).collect();
        if !list.contains(&cur) {
            list.insert(0, cur.clone());
        }
        groups.push(
            view! {
                <div class="insp-group" data-role="cart">
                    <p class="panel-title" title=ui.t("role-cart-help")>{ui.t("role-cart")}</p>
                    <label class="field">
                        <span class="label" title=ui.t("role-cart-currency-help")>{ui.t("role-cart-currency")}</span>
                        <div class="select">
                            <select class="input" on:change=move |e| ed.apply(ui, Command::SetAttr {
                                at: atv.get_value(), name: "data-revel-cart".into(), value: Some(event_target_value(&e)),
                            }, false)>
                                {list.into_iter().map(|c| { let on = c == cur; view! { <option value=c.clone() prop:selected=on>{c.clone()}</option> } }).collect_view()}
                            </select>
                        </div>
                    </label>
                </div>
            }
            .into_any(),
        );
    }
    if attr(attrs, "data-revel-cart-add").is_some() {
        groups.push(
            view! {
                <div class="insp-group" data-role="cart-add">
                    <div class="insp-group-head">
                        <p class="panel-title" title=ui.t("role-cart-add-help")>{ui.t("role-cart-add")}</p>
                        {remove_btn(&["data-revel-cart-add", "data-revel-product", "data-revel-price"])}
                    </div>
                    {attr_value_field(ui, ed, at, attrs, "data-revel-product", "role-cart-product", "role-cart-product-help", false)}
                    {attr_value_field(ui, ed, at, attrs, "data-revel-price", "role-cart-price-value", "role-cart-price-help", true)}
                </div>
            }
            .into_any(),
        );
    }
    if attr(attrs, "data-revel-cart-count").is_some() {
        groups.push(
            view! {
                <div class="insp-group" data-role="cart-count">
                    <div class="insp-group-head">
                        <p class="panel-title" title=ui.t("role-cart-count-help")>{ui.t("role-cart-count")}</p>
                        {remove_btn(&["data-revel-cart-count"])}
                    </div>
                </div>
            }
            .into_any(),
        );
    }
    (!groups.is_empty()).then(|| groups.into_iter().collect_view().into_any())
}

/// Choix d'un rôle parmi des attributs (un seul à la fois).
fn role_select(
    ui: Ui,
    ed: Ed,
    at: Target,
    attrs: &[Attr],
    title: &'static str,
    help: &'static str,
    roles: &'static [(&'static str, &'static str)],
) -> AnyView {
    let cur = roles
        .iter()
        .find(|(a, _)| attr(attrs, a).is_some())
        .map(|(a, _)| *a)
        .unwrap_or("");
    let at = StoredValue::new(at);
    let set = move |v: String| {
        let at = at.get_value();
        let present: Vec<&str> = roles
            .iter()
            .map(|(a, _)| *a)
            .filter(|a| ed.doc.with_untracked(|d| {
                d.as_ref()
                    .and_then(|d| revel_edit::address::node(d, &at).ok())
                    .is_some_and(|n| matches!(n, Node::Element { attrs, .. } if attrs.iter().any(|x| x.name == *a)))
            }))
            .collect();
        let mut commands: Vec<Command> = present
            .iter()
            .filter(|a| **a != v)
            .map(|a| Command::RemoveAttr {
                at: at.clone(),
                name: a.to_string(),
            })
            .collect();
        if !v.is_empty() && !present.contains(&v.as_str()) {
            commands.push(Command::SetAttr {
                at: at.clone(),
                name: v,
                value: None,
            });
        }
        if !commands.is_empty() {
            ed.apply(ui, Command::Batch { commands }, false);
        }
    };
    view! {
        <div class="insp-group" data-role=title>
            <label class="field" title=ui.t(help)>
                <span class="label">{ui.t(title)}</span>
                <div class="select">
                    <select class="input" on:change=move |e| set(event_target_value(&e))>
                        <option value="" prop:selected=cur.is_empty()>{ui.t("tabs-role-none")}</option>
                        {roles.iter().map(|(a, k)| view! {
                            <option value=*a prop:selected=cur == *a>{ui.t(k)}</option>
                        }).collect_view()}
                    </select>
                </div>
            </label>
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ carrousel, diaporama

/// Image révélée au survol (D-125) : `fade` (en entier), `wipe` (jusqu'au pointeur), `spot`
/// (halo autour du pointeur, rayon et douceur du bord) ; images de dessous et révélée.
fn reveal_settings(ui: Ui, ed: Ed, at: Target, value: String) -> AnyView {
    let at = StoredValue::new(at);
    let mut parts = value.split_whitespace();
    let mode = parts.next().unwrap_or("fade").to_string();
    let opt = |k: &str, d: &str| {
        value
            .split_whitespace()
            .find_map(|o| o.strip_prefix(&format!("{k}=")).map(str::to_string))
            .unwrap_or_else(|| d.to_string())
    };
    let (radius, soft) = (opt("radius", "140"), opt("soft", "40"));
    let write = move |mode: String, radius: String, soft: String| {
        let v = if mode == "spot" {
            format!("spot radius={radius} soft={soft}")
        } else {
            mode
        };
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: "data-revel-reveal".into(),
                value: Some(v),
            },
            false,
        );
    };
    let (m1, r1, s1) = (mode.clone(), radius.clone(), soft.clone());
    let (m2, s2) = (mode.clone(), soft.clone());
    let (m3, r3) = (mode.clone(), radius.clone());
    let num = |n: String| {
        n.trim()
            .parse::<u32>()
            .ok()
            .map(|v| v.min(2000).to_string())
    };
    view! {
        <div class="insp-group reveal-settings">
            <p class="panel-title">{ui.t("insert-reveal")}</p>
            <div class="seg reveal-mode" role="radiogroup" aria-label=ui.t("reveal-mode")>
                {[("fade", "reveal-fade", "reveal-fade-help"), ("wipe", "reveal-wipe", "reveal-wipe-help"), ("spot", "reveal-spot", "reveal-spot-help")]
                    .into_iter().map(|(v, k, h)| {
                        let on = mode == v;
                        let (r, s) = (r1.clone(), s1.clone());
                        view! {
                            <button role="radio" data-mode=v class:is-on=on aria-checked=on.to_string() title=ui.t(h)
                                on:click=move |_| write(v.to_string(), r.clone(), s.clone())>{ui.t(k)}</button>
                        }
                    }).collect_view()}
            </div>
            {(m1 == "spot").then(|| view! {
                <div class="style-row" title=ui.t("reveal-radius-help")>
                    <span class="style-label"><span class="style-label-text">{ui.t("reveal-radius")}</span></span>
                    <div class="style-control">
                        <div class="nbox has-units">
                            <input class="nbox-input" type="text" inputmode="numeric" data-reveal="radius" prop:value=radius.clone()
                                on:change=move |e| if let Some(v) = num(event_target_value(&e)) { write(m2.clone(), v, s2.clone()) }/>
                            <span class="nbox-suffix" aria-hidden="true">"px"</span>
                        </div>
                    </div>
                </div>
                <div class="style-row" title=ui.t("reveal-soft-help")>
                    <span class="style-label"><span class="style-label-text">{ui.t("reveal-soft")}</span></span>
                    <div class="style-control">
                        <div class="nbox has-units">
                            <input class="nbox-input" type="text" inputmode="numeric" data-reveal="soft" prop:value=soft.clone()
                                on:change=move |e| if let Some(v) = num(event_target_value(&e)) { write(m3.clone(), r3.clone(), v) }/>
                            <span class="nbox-suffix" aria-hidden="true">"px"</span>
                        </div>
                    </div>
                </div>
            })}
            <crate::items::ItemsList at=at.get_value() fixed=true/>
        </div>
    }
    .into_any()
}

/// Réglages d'un volet d'accordéon (`details`) : ouvert au chargement (`open`) ; un seul volet
/// ouvert à la fois parmi ses voisins (`name` commun, accordéon exclusif du navigateur : Chrome et
/// Edge 120, Firefox 130, Safari 17.2) ; « Ajouter un volet » (copie après celui-ci).
fn accordion_settings(ui: Ui, ed: Ed, at: Target, attrs: &[Attr]) -> AnyView {
    let open = attr(attrs, "open").is_some();
    let group = attr(attrs, "name").and_then(|a| a.value.clone());
    let at = StoredValue::new(at);
    // Volets voisins (même parent) : identifiants.
    let siblings = move || -> Vec<String> {
        ed.doc.with_untracked(|d| {
            let Some(d) = d.as_ref() else {
                return Vec::new();
            };
            let Ok((parent, _)) = revel_edit::address::locate(d, &at.get_value()) else {
                return Vec::new();
            };
            revel_edit::address::list(d, &parent)
                .map(|l| {
                    l.iter()
                        .filter_map(|n| match n {
                            Node::Element {
                                id: Some(id), tag, ..
                            } if tag == "details" => Some(id.clone()),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
    };
    let many = siblings().len() > 1;
    let set_open = move |on: bool| {
        let at = at.get_value();
        ed.apply(
            ui,
            if on {
                Command::SetAttr {
                    at,
                    name: "open".into(),
                    value: None,
                }
            } else {
                Command::RemoveAttr {
                    at,
                    name: "open".into(),
                }
            },
            false,
        );
    };
    let set_group = move |on: bool| {
        let owner = at.get_value().owner;
        let name = format!("accordeon-{}", at.get_value().node);
        let commands: Vec<Command> = siblings()
            .into_iter()
            .map(|node| {
                let at = Target {
                    owner: owner.clone(),
                    node,
                };
                if on {
                    Command::SetAttr {
                        at,
                        name: "name".into(),
                        value: Some(name.clone()),
                    }
                } else {
                    Command::RemoveAttr {
                        at,
                        name: "name".into(),
                    }
                }
            })
            .collect();
        if !commands.is_empty() {
            ed.apply(ui, Command::Batch { commands }, false);
        }
    };
    view! {
        <div class="insp-group accordion-settings">
            <p class="panel-title">{ui.t("insert-accordion")}</p>
            <label class="view-line" title=ui.t("accordion-open-help")>
                <span>{ui.t("accordion-open")}</span>
                <span class="switch">
                    <input type="checkbox" prop:checked=open on:change=move |e| set_open(event_target_checked(&e))/>
                    <span class="switch-track"></span>
                </span>
            </label>
            {many.then(|| view! {
                <label class="view-line" title=ui.t("accordion-single-help")>
                    <span>{ui.t("accordion-single")}</span>
                    <span class="switch">
                        <input type="checkbox" prop:checked=group.is_some() on:change=move |e| set_group(event_target_checked(&e))/>
                        <span class="switch-track"></span>
                    </span>
                </label>
            })}
            <button class="btn btn--small items-add" data-action="accordion-add"
                on:click=move |_| ed.apply_keep(ui, Command::Duplicate { at: at.get_value() })>
                {icons::plus()}<span>{ui.t("accordion-add")}</span>
            </button>
        </div>
    }
    .into_any()
}

/// Élément d'un menu dont les enfants sont des liens (deux au moins) : le menu lui-même
/// (`own`) ou un de ses descendants.
fn links_box(own: Option<&str>, children: &[Node]) -> Option<String> {
    let links = children
        .iter()
        .filter(|n| matches!(n, Node::Element { tag, .. } if tag == "a"))
        .count();
    if links >= 2 {
        return own.map(str::to_string);
    }
    children.iter().find_map(|n| match n {
        Node::Element { id, children, .. } => links_box(id.as_deref(), children),
        _ => None,
    })
}

// ------------------------------------------------------------------ animation Lottie

const LOTTIE_PLAY: &[(&str, &str)] = &[
    ("auto", "lottie-play-auto"),
    ("hover", "lottie-play-hover"),
    ("click", "lottie-play-click"),
    ("scroll", "lottie-play-scroll"),
];
const LOTTIE_SPEED: &[&str] = &["0.5", "0.75", "1", "1.5", "2"];

/// Animation Lottie (6.2) : fichier, lecture (à l'écran, au survol, à l'appui, au défilement),
/// boucle, vitesse.
fn lottie_options(ui: Ui, ed: Ed, at: Target, attrs: Vec<Attr>) -> AnyView {
    let value = |n: &str| attr(&attrs, n).and_then(|a| a.value.clone());
    let src = value("data-lottie").unwrap_or_default();
    let play = value("data-lottie-play").unwrap_or_else(|| "auto".into());
    let looped = attr(&attrs, "data-lottie-loop").is_some();
    let speed = value("data-lottie-speed").unwrap_or_else(|| "1".into());
    let at = StoredValue::new(at);
    let set = move |name: &'static str, v: Option<String>| {
        let at = at.get_value();
        let cmd = match v {
            Some(v) => Command::SetAttr {
                at,
                name: name.into(),
                value: Some(v),
            },
            None => Command::RemoveAttr {
                at,
                name: name.into(),
            },
        };
        ed.apply(ui, cmd, false);
    };
    let pick = move || {
        crate::library::pick_animation(ui, crate::library::media_shelf(ed), move |url| {
            set("data-lottie", Some(url))
        })
    };
    let file = src.rsplit('/').next().unwrap_or_default().to_string();
    view! {
        <div class="insp-group">
            <p class="panel-title">{ui.t("block-lottie")}</p>
            <div class="field">
                <span class="label">{ui.t("block-lottie-file")}</span>
                <span class="field-line">
                    <input class="input input--mono" readonly=true prop:value=file.clone()
                        placeholder=ui.t("block-lottie-none") title=src.clone() on:click=move |_| pick()/>
                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("block-lottie-choose")
                        aria-label=ui.t("block-lottie-choose") on:click=move |_| pick()>{crate::icons::sparkle()}</button>
                </span>
            </div>
            <label class="field">
                <span class="label">{ui.t("block-lottie-play")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| set("data-lottie-play", Some(event_target_value(&e)))>
                        {LOTTIE_PLAY.iter().map(|(v, k)| view! { <option value=*v selected=*v == play title=ui.tx(&format!("{k}-help"), &[])>{ui.t(k)}</option> }).collect_view()}
                    </select>
                </div>
            </label>
            <label class="field">
                <span class="label">{ui.t("block-lottie-speed")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| {
                        let v = event_target_value(&e);
                        set("data-lottie-speed", (v != "1").then_some(v));
                    }>
                        {LOTTIE_SPEED.iter().map(|v| view! { <option value=*v selected=*v == speed>{format!("{}×", v.replace('.', ","))}</option> }).collect_view()}
                    </select>
                </div>
            </label>
            <label class="view-line">
                <span>{ui.t("block-lottie-loop")}</span>
                <span class="switch">
                    <input type="checkbox" prop:checked=looped on:change=move |e| {
                        set("data-lottie-loop", event_target_checked(&e).then(String::new))
                    }/>
                    <span class="switch-track"></span>
                </span>
            </label>
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ liste de collection

/// Champs proposés pour filtrer ou trier (chemin dans le contenu, libellé).
fn fields_of(ui: Ui, c: Option<&CmsCollection>) -> Vec<(String, String)> {
    let mut v = vec![("date".to_string(), ui.now("block-list-date", &[]))];
    if let Some(c) = c {
        for f in &c.fields {
            if matches!(
                f.kind.as_deref(),
                Some("object" | "block" | "rich_text" | "code" | "image" | "file")
            ) {
                continue;
            }
            v.push((
                format!("data.{}", f.name),
                f.label.clone().unwrap_or_else(|| f.name.clone()),
            ));
        }
    }
    v
}

/// Valeurs proposées pour un champ d'une collection (filtre, « contient ») : choix du schéma,
/// contenus reliés (par titre), oui/non ; `true` : liste fermée (sinon suggestions tirées des
/// valeurs déjà écrites dans les contenus).
fn field_values(
    ui: Ui,
    cols: &[CmsCollection],
    c: Option<&CmsCollection>,
    path: &str,
) -> (Vec<(String, String)>, bool) {
    let Some(c) = c else {
        return (Vec::new(), false);
    };
    let name = path.strip_prefix("data.").unwrap_or(path);
    let f = c.fields.iter().find(|f| f.name == name);
    match f.and_then(|f| f.kind.as_deref()) {
        Some("select") => return (crate::cmsfields::choices(f.unwrap()), true),
        Some("reference") => return (crate::cmsfields::reference_choices(cols, f.unwrap()), true),
        Some("boolean") => {
            return (
                vec![
                    ("true".into(), ui.now("block-list-value-yes", &[])),
                    ("false".into(), ui.now("block-list-value-no", &[])),
                ],
                true,
            );
        }
        _ => {}
    }
    let mut seen: Vec<(String, String)> = Vec::new();
    for e in &c.entries {
        let vals: Vec<String> = match e.data.get(name) {
            Some(serde_json::Value::String(s)) => vec![s.clone()],
            Some(serde_json::Value::Array(a)) => a
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect(),
            Some(serde_json::Value::Number(n)) => vec![n.to_string()],
            Some(serde_json::Value::Bool(b)) => vec![b.to_string()],
            _ => Vec::new(),
        };
        for v in vals {
            if !v.is_empty() && v.len() < 80 && !seen.iter().any(|(x, _)| *x == v) {
                seen.push((v.clone(), v));
            }
        }
        if seen.len() >= 60 {
            break;
        }
    }
    seen.sort_by_key(|a| a.1.to_lowercase());
    (seen, false)
}

/// Valeur cherchée d'un filtre : liste des choix (champ à choix, référence, oui/non), sinon
/// texte avec les valeurs déjà écrites en suggestions.
fn value_picker(
    ui: Ui,
    options: (Vec<(String, String)>, bool),
    current: String,
    placeholder: &'static str,
    on_set: impl Fn(String) + Clone + 'static,
) -> AnyView {
    let (list, closed) = options;
    if closed {
        let known = current.is_empty() || list.iter().any(|(v, _)| *v == current);
        view! {
            <div class="select">
                <select class="input" on:change=move |e| on_set(event_target_value(&e))>
                    <option value="" prop:selected=current.is_empty()>{ui.t(placeholder)}</option>
                    {(!known).then(|| view! { <option value=current.clone() selected=true>{current.clone()}</option> })}
                    {list.into_iter().map(|(v, l)| { let on = v == current; view! { <option value=v prop:selected=on>{l}</option> } }).collect_view()}
                </select>
            </div>
        }
        .into_any()
    } else {
        let id = format!("dl-{}", js_sys::Math::random().to_string().replace('.', ""));
        view! {
            <input class="input" prop:value=current placeholder=ui.t(placeholder) list=id.clone()
                on:change=move |e| on_set(event_target_value(&e))/>
            <datalist id=id>{list.into_iter().map(|(v, _)| view! { <option value=v></option> }).collect_view()}</datalist>
        }
        .into_any()
    }
}

/// Champs à plusieurs valeurs du contenu qui entoure une liste (liste dans une liste) : nom,
/// libellé, collection désignée (référence).
fn list_fields(c: Option<&CmsCollection>) -> Vec<(String, String, Option<String>)> {
    c.map(|c| {
        c.fields
            .iter()
            .filter(|f| f.list && !matches!(f.kind.as_deref(), Some("object" | "block")))
            .map(|f| {
                let target = (f.kind.as_deref() == Some("reference"))
                    .then(|| {
                        f.options
                            .get("collection")
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                    })
                    .flatten();
                (
                    f.name.clone(),
                    f.label.clone().unwrap_or_else(|| f.name.clone()),
                    target,
                )
            })
            .collect()
    })
    .unwrap_or_default()
}

/// Champs « liste d'images » du contenu auquel l'élément peut se relier (galerie d'un projet) :
/// expression et nom affiché (D-174).
pub(crate) fn image_list_fields(ed: Ed, at: &Target) -> Vec<(String, String)> {
    let Some((col, prefix, _)) = ed.doc.with_untracked(|d| binding_context(d.as_ref()?, at)) else {
        return Vec::new();
    };
    collections(ed).with_untracked(|cs| {
        cs.iter()
            .find(|c| c.id == col)
            .map(|c| {
                c.fields
                    .iter()
                    .filter(|f| f.list && matches!(f.kind.as_deref(), Some("image")))
                    .map(|f| {
                        (
                            format!("{prefix}{}", f.name),
                            f.label.clone().unwrap_or_else(|| f.name.clone()),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// Source d'une liste : une collection, ou un champ du contenu qui l'entoure.
fn list_source(ui: Ui, ed: Ed, each_at: Target, expr: String) -> Option<AnyView> {
    let outer = ed
        .doc
        .with_untracked(|d| binding_context(d.as_ref()?, &each_at));
    let field = FieldList::parse(&expr);
    if outer.is_none() && field.is_none() {
        return None;
    }
    let cols = collections(ed);
    let at = StoredValue::new(each_at);
    let (outer_col, prefix) = outer.map(|(c, p, _)| (c, p)).unwrap_or_default();
    let outer_col = StoredValue::new(outer_col);
    let prefix = StoredValue::new(prefix);
    // Valeur du choix : `col:<id>` ou `field:<nom>`.
    let cur = match (&field, Query::parse(&expr)) {
        (Some(FieldList::Values { field }), _) | (Some(FieldList::Refs { field, .. }), _) => {
            let p = prefix.get_value();
            format!("field:{}", field.strip_prefix(p.as_str()).unwrap_or(field))
        }
        (None, Some(q)) => format!("col:{}", q.collection),
        _ => String::new(),
    };
    let choose = move |v: String| {
        let p = prefix.get_value();
        // Dans une liste : nom de l'élément différent de celui de la liste qui l'entoure.
        let mut item = None;
        let new = if let Some(id) = v.strip_prefix("col:") {
            Query {
                collection: id.to_string(),
                ..Default::default()
            }
            .to_expr()
        } else if let Some(name) = v.strip_prefix("field:") {
            let target = cols.with_untracked(|cs| {
                list_fields(cs.iter().find(|c| outer_col.with_value(|o| c.id == *o)))
                    .into_iter()
                    .find(|(n, _, _)| n == name)
                    .and_then(|(_, _, t)| t)
            });
            let nested = !p.is_empty();
            match target {
                Some(collection) => {
                    item = nested.then(|| "ref".to_string());
                    FieldList::Refs {
                        collection,
                        field: format!("{p}{name}"),
                    }
                    .to_expr()
                }
                None => {
                    item = nested.then(|| "valeur".to_string());
                    FieldList::Values {
                        field: format!("{p}{name}"),
                    }
                    .to_expr()
                }
            }
        } else {
            return;
        };
        ed.apply(
            ui,
            Command::SetLogic {
                at: at.get_value(),
                cond: None,
                expr: Some(new),
                item,
            },
            false,
        );
    };
    Some(
        view! {
            <label class="field">
                <span class="label">{ui.t("block-list-source")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| choose(event_target_value(&e))>
                        {move || {
                            let mut opts: Vec<(String, String)> = cols.with(|cs| {
                                let mut v: Vec<(String, String)> = list_fields(cs.iter().find(|c| outer_col.with_value(|o| c.id == *o)))
                                    .into_iter()
                                    .map(|(n, l, _)| (format!("field:{n}"), ui.now("block-list-of-field", &[("field", l)])))
                                    .collect();
                                v.extend(cs.iter().map(|c| (format!("col:{}", c.id), label_of(c))));
                                v
                            });
                            if !cur.is_empty() && !opts.iter().any(|(v, _)| *v == cur) {
                                let name = cur.split_once(':').map(|(_, n)| n.to_string()).unwrap_or_default();
                                opts.insert(0, (cur.clone(), name));
                            }
                            opts.into_iter().map(|(v, l)| {
                                let on = v == cur;
                                view! { <option value=v prop:selected=on>{l}</option> }
                            }).collect_view()
                        }}
                    </select>
                </div>
            </label>
        }
        .into_any(),
    )
}

/// Classes d'un élément selon le contenu (paramètres avancés) : classe ajoutée quand un champ est
/// rempli (`person__photo--swap` si « Photo au survol »), ou tirée de la valeur d'un champ. Dans
/// une liste de contenus, le modèle des pages d'une collection ou un composant seulement.
pub(crate) fn content_classes(ui: Ui, ed: Ed, at: Target, node: &Node) -> Option<AnyView> {
    let fields = condition_fields(ed, &at)?;
    if fields.with_untracked(Vec::is_empty) {
        return None;
    }
    class_editors(ui, ed, StoredValue::new(at), node, fields)
}

/// Classes d'un élément relié : selon des conditions, ou tirée de la valeur d'un champ.
fn class_editors(
    ui: Ui,
    ed: Ed,
    at: StoredValue<Target>,
    node: &Node,
    fields: Signal<Vec<(String, String)>>,
) -> Option<AnyView> {
    let Node::Element { attrs, .. } = node else {
        return None;
    };
    let v = attr(attrs, "class")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    if let Some((base, prefix, field, default)) = revel_edit::style::value_class(&v) {
        return Some(value_class_editor(
            ui, ed, at, base, prefix, field, default, fields,
        ));
    }
    let (base, conds) = revel_edit::style::cond_classes(&v)?;
    Some(cond_classes_editor(ui, ed, at, base, conds, fields))
}

/// Classe tirée d'un champ : début écrit autour, préfixe, champ, valeur par défaut.
type ValueClass = (String, String, String, Option<String>);

/// Classe tirée de la valeur d'un champ (`block--{{ b.style or 'plein' }}`) : champ, début de
/// la classe, valeur quand le champ est vide.
#[allow(clippy::too_many_arguments)]
fn value_class_editor(
    ui: Ui,
    ed: Ed,
    at: StoredValue<Target>,
    base: String,
    prefix: String,
    field: String,
    default: Option<String>,
    fields: Signal<Vec<(String, String)>>,
) -> AnyView {
    let state = StoredValue::new((base, prefix.clone(), field.clone(), default.clone()));
    let write = move |f: Box<dyn Fn(&mut ValueClass)>| {
        let mut st = state.get_value();
        f(&mut st);
        state.set_value(st.clone());
        let (b, p, fi, d) = st;
        let value = if fi.is_empty() {
            b
        } else {
            revel_edit::style::value_class_expr(&b, &p, &fi, d.as_deref())
        };
        let cmd = if value.trim().is_empty() {
            Command::RemoveAttr {
                at: at.get_value(),
                name: "class".into(),
            }
        } else {
            Command::SetAttr {
                at: at.get_value(),
                name: "class".into(),
                value: Some(value.trim().to_string()),
            }
        };
        ed.apply(ui, cmd, false);
    };
    let cur = field.clone();
    view! {
        <div class="field" data-bound="block-bound-value-class">
            <div class="field-line">
                <span class="label" title=ui.t("block-bound-value-class-help")>{ui.t("block-bound-value-class")}</span>
                <button type="button" class="icon-btn icon-btn--small" title=ui.t("cond-class-remove") aria-label=ui.t("cond-class-remove")
                    on:click=move |_| write(Box::new(|s| s.2 = String::new()))>{icons::close()}</button>
            </div>
            <div class="select">
                <select class="input" on:change=move |e| {
                    let v = event_target_value(&e);
                    write(Box::new(move |s| s.2 = v.clone()));
                }>
                    <option value="">{ui.t("block-bound-value-class-none")}</option>
                    {move || {
                        let known = fields.with(|fs| fs.iter().any(|(e, _)| *e == cur));
                        let extra = (!known).then(|| view! { <option value=cur.clone() selected=true>{cur.clone()}</option> });
                        view! {
                            {extra}
                            {fields.with(|fs| fs.iter().map(|(e, l)| {
                                let on = *e == cur;
                                view! { <option value=e.clone() selected=on>{l.clone()}</option> }
                            }).collect_view())}
                        }
                    }}
                </select>
            </div>
            <div class="field-pair">
                <label class="field">
                    <span class="label">{ui.t("block-bound-value-class-prefix")}</span>
                    <input class="input input--mono" spellcheck="false" prop:value=prefix
                        on:change=move |e| { let v = event_target_value(&e).trim().to_string(); write(Box::new(move |s| s.1 = v.clone())) }/>
                </label>
                <label class="field">
                    <span class="label">{ui.t("block-bound-value-class-default")}</span>
                    <input class="input input--mono" spellcheck="false" prop:value=default.unwrap_or_default()
                        on:change=move |e| {
                            let v = event_target_value(&e).trim().to_string();
                            write(Box::new(move |s| s.3 = (!v.is_empty()).then(|| v.clone())))
                        }/>
                </label>
            </div>
        </div>
    }
    .into_any()
}

/// Classes ajoutées selon des conditions (`carte{% if item.data.une %} carte--une{% endif %}`) :
/// une ligne par classe, avec ses conditions (champ rempli, égal à…).
fn cond_classes_editor(
    ui: Ui,
    ed: Ed,
    at: StoredValue<Target>,
    base: String,
    conds: Vec<(String, String)>,
    fields: Signal<Vec<(String, String)>>,
) -> AnyView {
    let rows = StoredValue::new(conds.clone());
    let base = StoredValue::new(base);
    let write = move |list: Vec<(String, String)>| {
        let base = base.get_value();
        rows.set_value(list.clone());
        let mut value = base.clone();
        for (c, cls) in list
            .iter()
            .filter(|(c, cls)| !c.trim().is_empty() && !cls.trim().is_empty())
        {
            value.push_str(&format!("{{% if {c} %}} {}{{% endif %}}", cls.trim()));
        }
        let cmd = if value.trim().is_empty() {
            Command::RemoveAttr {
                at: at.get_value(),
                name: "class".into(),
            }
        } else {
            Command::SetAttr {
                at: at.get_value(),
                name: "class".into(),
                value: Some(value.trim_start().to_string()),
            }
        };
        ed.apply(ui, cmd, false);
    };
    let first_class = conds_base_first(&rows);
    let no_rows = rows.with_value(|r| r.is_empty());
    view! {
        <div class="field" data-bound="block-bound-class">
            <span class="label" title=ui.t("block-bound-class-help")>{ui.t("block-bound-class")}</span>
            {conds.into_iter().enumerate().map(|(i, (c, cls))| {
                let set_cond = move |nc: Option<String>| {
                    if let Some(nc) = nc {
                        let mut list = rows.get_value();
                        if let Some(r) = list.get_mut(i) { r.0 = nc; }
                        write(list);
                    }
                };
                view! {
                    <div class="cond-class">
                        <div class="field-line">
                            <input class="input input--mono" prop:value=cls spellcheck="false" aria-label=ui.t("block-bound-class-name")
                                on:change=move |e| {
                                    let v = event_target_value(&e).trim().trim_start_matches('.').to_string();
                                    let mut list = rows.get_value();
                                    if let Some(r) = list.get_mut(i) { r.1 = v; }
                                    write(list);
                                }/>
                            <button type="button" class="icon-btn icon-btn--small" title=ui.t("cond-class-remove") aria-label=ui.t("cond-class-remove")
                                on:click=move |_| {
                                    let mut list = rows.get_value();
                                    if i < list.len() { list.remove(i); }
                                    write(list);
                                }>{icons::close()}</button>
                        </div>
                        {crate::visibility::condition_editor(ui, fields, Some(c), false, set_cond)}
                    </div>
                }
            }).collect_view()}
            <div class="cond-class-adds">
            <button type="button" class="btn btn--small cond-add" on:click=move |_| {
                let Some(f) = fields.with_untracked(|fs| fs.first().map(|(e, _)| e.clone())) else { return };
                let mut list = rows.get_value();
                list.push((f, format!("{}--option", first_class.clone())));
                write(list);
            }>{icons::plus()}<span>{ui.t("cond-class-add")}</span></button>
            {no_rows.then(|| view! {
                <button type="button" class="btn btn--small cond-add" on:click=move |_| {
                    let Some(f) = fields.with_untracked(|fs| fs.first().map(|(e, _)| e.clone())) else { return };
                    let b = base.get_value();
                    let first = b.split_whitespace().next().unwrap_or("classe").to_string();
                    ed.apply(ui, Command::SetAttr {
                        at: at.get_value(),
                        name: "class".into(),
                        value: Some(revel_edit::style::value_class_expr(&b, &format!("{first}--"), &f, None)),
                    }, false);
                }>{icons::plus()}<span>{ui.t("block-bound-value-class-add")}</span></button>
            })}
            </div>
        </div>
    }
    .into_any()
}

fn conds_base_first(rows: &StoredValue<Vec<(String, String)>>) -> String {
    rows.with_value(|r| {
        r.first()
            .and_then(|(_, c)| c.split("--").next().map(str::to_string))
    })
    .unwrap_or_else(|| "classe".into())
}

/// Contenus choisis à la main, dans l'ordre : monter, descendre, retirer, ajouter (titres des
/// contenus de la collection).
fn picks_editor(
    ui: Ui,
    entries: RwSignal<Vec<revel_protocol::PickEntry>>,
    list: Vec<String>,
    set: impl Fn(Vec<String>) + Copy + Send + Sync + 'static,
) -> AnyView {
    let picked = StoredValue::new(list.clone());
    let n = list.len();
    view! {
        <ul class="pick-list list-picks">
            {list.into_iter().enumerate().map(|(i, slug)| {
                // Contenu introuvable (supprimé) : marqué une fois les contenus lus.
                let slug3 = slug.clone();
                let missing = move || entries.with(|es| !es.is_empty() && !es.iter().any(|e| e.slug == slug3));
                let shift = move |d: isize| {
                    let mut all = picked.get_value();
                    let j = i as isize + d;
                    if j >= 0 && (j as usize) < all.len() {
                        all.swap(i, j as usize);
                        set(all);
                    }
                };
                let slug2 = slug.clone();
                view! {
                    <li class="pick-item" class:is-missing=missing title=slug.clone()>
                        <span class="pick-title">{move || entries.with(|es| es.iter().find(|e| e.slug == slug2).map(|e| e.title.clone())).unwrap_or_else(|| slug2.clone())}</span>
                        <button class="icon-btn icon-btn--small" title=ui.t("layers-up") aria-label=ui.t("layers-up")
                            disabled=i == 0 on:click=move |_| shift(-1)>{icons::arrow_up()}</button>
                        <button class="icon-btn icon-btn--small" title=ui.t("layers-down") aria-label=ui.t("layers-down")
                            disabled=i + 1 == n on:click=move |_| shift(1)>{icons::arrow_down()}</button>
                        <button class="icon-btn icon-btn--small" title=ui.t("action-remove") aria-label=ui.t("action-remove")
                            on:click=move |_| { let mut all = picked.get_value(); all.remove(i); set(all); }>{icons::close()}</button>
                    </li>
                }
            }).collect_view()}
        </ul>
        <div class="select pick-add">
            <select class="input" aria-label=ui.t("picks-add") on:change=move |e| {
                let v = event_target_value(&e);
                if !v.is_empty() {
                    let mut all = picked.get_value();
                    all.push(v);
                    set(all);
                }
            }>
                <option value="" selected=true>{ui.t("picks-add")}</option>
                {move || entries.with(|es| es.iter()
                    .filter(|e| !picked.with_value(|p| p.contains(&e.slug)))
                    .map(|e| view! { <option value=e.slug.clone()>{e.title.clone()}</option> })
                    .collect_view())}
            </select>
        </div>
    }
    .into_any()
}

fn collection_list(ui: Ui, ed: Ed, each_at: Target, expr: String) -> AnyView {
    let source = list_source(ui, ed, each_at.clone(), expr.clone());
    let Some(q) = Query::parse(&expr) else {
        // Liste tirée d'un champ : sa source seulement.
        if let Some(src) = source {
            return view! {
                <div class="insp-group collection-list">
                    <p class="panel-title">{ui.t("block-list")}</p>
                    {src}
                </div>
            }
            .into_any();
        }
        return view! {
            <div class="insp-group">
                <p class="panel-title">{ui.t("block-list")}</p>
                <p class="muted small" title=ui.t("block-list-computed")>{ui.t("block-list-computed-short")}</p>
            </div>
        }
        .into_any();
    };
    let nested = source.is_some();
    let cols = collections(ed);
    let at = StoredValue::new(each_at);

    let q = StoredValue::new(q);
    let write = move |f: Box<dyn Fn(&mut Query)>| {
        let mut nq = q.get_value();
        f(&mut nq);
        ed.apply(
            ui,
            Command::SetLogic {
                at: at.get_value(),
                cond: None,
                expr: Some(nq.to_expr()),
                item: None,
            },
            false,
        );
    };
    let current =
        move || cols.with(|cs| q.with_value(|q| cs.iter().find(|c| c.id == q.key()).cloned()));
    let fields = move || fields_of(ui, current().as_ref());
    let filter_field = q.with_value(|q| q.filter.as_ref().map(|f| f.0.clone()).unwrap_or_default());
    let filter_value = q.with_value(|q| {
        q.filter
            .as_ref()
            .and_then(|f| f.1.clone())
            .map(|v| revel_edit::canvas::literal(&v).unwrap_or(v))
            .unwrap_or_default()
    });
    let sort_field = q.with_value(|q| q.sort.as_ref().map(|s| s.0.clone()).unwrap_or_default());
    let desc = q.with_value(|q| q.sort.as_ref().is_some_and(|s| s.1));
    let limit = q.with_value(|q| q.limit.map(|n| n.to_string()).unwrap_or_default());
    // Premier contenu montré (D-171) : 1 = depuis le début.
    let start = q.with_value(|q| {
        if q.offset > 0 {
            (q.offset + 1).to_string()
        } else {
            String::new()
        }
    });
    let exclude = q.with_value(|q| q.exclude_current);
    // Affichage progressif (D-114) : élément qui contient la liste, et nombre montré d'abord.
    let holder = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let at = at.get_value();
        let (parent, _) = revel_edit::address::locate(d, &at).ok()?;
        let t = Target {
            owner: at.owner.clone(),
            node: parent.node?,
        };
        let more = match revel_edit::address::node(d, &t).ok()? {
            Node::Element { attrs, .. } => attrs
                .iter()
                .find(|a| a.name == "data-revel-more")
                .and_then(|a| a.value.clone())
                .unwrap_or_default(),
            _ => return None,
        };
        Some((t, more))
    });
    // Sur la page d'un contenu (modèle de collection) : liste sans le contenu affiché.
    let on_template = exclude
        || ed.doc.with_untracked(|d| {
            d.as_ref()
                .and_then(|d| template_collection(d, &at.get_value()))
                .is_some()
        });
    let has_filter = !filter_field.is_empty();
    let has_sort = !sort_field.is_empty();
    let (ff, sf) = (filter_field.clone(), sort_field.clone());
    // Contenus choisis à la main (ordre affiché) : contenus de la collection proposés par titre.
    let picked = q.with_value(|q| q.pick.clone());
    let picking = RwSignal::new(!picked.is_empty());
    let entries = RwSignal::new(Vec::<revel_protocol::PickEntry>::new());
    {
        let ids: Vec<String> = q.with_value(|q| {
            std::iter::once(q.collection.clone())
                .chain(q.also.clone())
                .collect()
        });
        let path = ed.path.get_value();
        spawn_local(async move {
            if let Ok(d) = crate::api::readable_data(&path, &[], &ids).await {
                let _ = entries.try_set(d.entries);
            }
        });
    }
    let set_pick = move |v: Vec<String>| write(Box::new(move |q| q.pick = v.clone()));
    let pick_view = move || {
        picking
            .get()
            .then(|| picks_editor(ui, entries, picked.clone(), set_pick))
    };
    // Contenus mis en premier : champ du contenu affiché (liste de contenus reliés) ou
    // contenus choisis, puis les autres.
    let first = q.with_value(|q| q.first.clone());
    let first_picks = first
        .as_deref()
        .and_then(revel_edit::collection::first_picks);
    let first_value = match (&first, &first_picks) {
        (None, _) => String::new(),
        (Some(_), Some(_)) => "pick".to_string(),
        (Some(f), None) => format!("field:{f}"),
    };
    let first_fields: Vec<(String, String)> = ed.doc.with_untracked(|d| {
        d.as_ref()
            .and_then(|d| template_collection(d, &at.get_value()))
            .map(|col| {
                cols.with_untracked(|cs| {
                    list_fields(cs.iter().find(|c| c.id == col))
                        .into_iter()
                        .map(|(n, l, _)| (n, l))
                        .collect()
                })
            })
            .unwrap_or_default()
    });
    let first_fields = StoredValue::new(first_fields);
    let set_first = move |v: Vec<String>| {
        write(Box::new(move |q| {
            q.first = Some(revel_edit::collection::first_expr(&v))
        }))
    };
    let first_view = first_picks.map(|list| picks_editor(ui, entries, list, set_first));
    // Filtre « contient » (champ liste, comme « Projets dont les expertises contiennent la page
    // affichée ») : sur la page d'un contenu, la page affichée ; sinon une valeur écrite.
    let contains = q.with_value(|q| q.contains.clone());
    let contains_field = contains.as_ref().map(|c| c.0.clone()).unwrap_or_default();
    let contains_field2 = contains_field.clone();
    let contains_value = contains.map(|c| c.1).unwrap_or_default();
    view! {
        <div class="insp-group collection-list">
            <p class="panel-title">{ui.t("block-list")}</p>
            {source}
            <label class="field" class:is-hidden=nested>
                <span class="label">{ui.t("block-list-collection")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| {
                        let v = event_target_value(&e);
                        write(Box::new(move |q| {
                            q.collection = v.clone();
                            q.filter = None;
                            q.sort = q.sort.take().filter(|s| s.0 == "date");
                        }))
                    }>
                        {move || {
                            let cur = q.with_value(|q| q.collection.clone());
                            let mut list: Vec<(String, String)> = cols.with(|cs| cs.iter().filter(|c| !c.id.contains('+')).map(|c| (c.id.clone(), label_of(c))).collect());
                            if !list.iter().any(|(id, _)| *id == cur) {
                                list.insert(0, (cur.clone(), cur.clone()));
                            }
                            list.into_iter().map(|(id, l)| {
                                let on = id == cur;
                                view! { <option value=id prop:selected=on>{l}</option> }
                            }).collect_view()
                        }}
                    </select>
                </div>
            </label>
            // Autres collections lues à la suite (contenus de plusieurs collections).
            <div class="list-also" class:is-hidden=nested title=ui.t("block-list-also-help")>
                {move || q.with_value(|q| q.also.clone()).into_iter().map(|id| {
                    let shown = cols.with(|cs| cs.iter().find(|c| c.id == id).map(label_of)).unwrap_or_else(|| id.clone());
                    let gone = id.clone();
                    view! {
                        <span class="prop-chip prop-chip--field">
                            {icons::database()}<span>{format!("+ {shown}")}</span>
                            <button class="chip-x" title=ui.t("block-list-also-remove") aria-label=ui.t("block-list-also-remove")
                                on:click=move |_| { let g = gone.clone(); write(Box::new(move |q| q.also.retain(|x| *x != g))) }>{icons::close()}</button>
                        </span>
                    }
                }).collect_view()}
                <div class="select select--small">
                    <select class="input" aria-label=ui.t("block-list-also-add") on:change=move |e| {
                        let v = event_target_value(&e);
                        if !v.is_empty() {
                            write(Box::new(move |q| if !q.also.contains(&v) && q.collection != v { q.also.push(v.clone()) }));
                        }
                    }>
                        <option value="" selected=true>{ui.t("block-list-also-add")}</option>
                        {move || {
                            let (main, also) = q.with_value(|q| (q.collection.clone(), q.also.clone()));
                            cols.with(|cs| cs.iter()
                                .filter(|c| !c.id.contains('+') && c.id != main && !also.contains(&c.id))
                                .map(|c| (c.id.clone(), label_of(c)))
                                .collect::<Vec<_>>())
                                .into_iter()
                                .map(|(id, l)| view! { <option value=id>{l}</option> })
                                .collect_view()
                        }}
                    </select>
                </div>
            </div>
            <div class="field" class:is-hidden=nested data-list="mode">
                <span class="label">{ui.t("block-list-items")}</span>
                // Automatique (tri, nombre, filtre) ou choix manuel des contenus, dans l'ordre voulu (D-182).
                <div class="segmented list-mode" role="radiogroup">
                    <button type="button" role="radio" data-mode="auto" class:is-on=move || !picking.get()
                        aria-checked=move || (!picking.get()).to_string()
                        on:click=move |_| { if picking.get_untracked() { picking.set(false); set_pick(Vec::new()); } }>{ui.t("block-list-items-all")}</button>
                    <button type="button" role="radio" data-mode="pick" class:is-on=move || picking.get()
                        aria-checked=move || picking.get().to_string()
                        on:click=move |_| picking.set(true)>{ui.t("block-list-items-pick")}</button>
                </div>
            </div>
            {pick_view}
            <div class="stack list-auto" class:is-hidden=move || picking.get()>
            <label class="field" class:is-hidden=nested title=ui.t("block-list-first-help")>
                <span class="label">{ui.t("block-list-first")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| {
                        let v = event_target_value(&e);
                        write(Box::new(move |q| {
                            q.first = if v.is_empty() {
                                None
                            } else if v == "pick" {
                                Some("[]".to_string())
                            } else {
                                v.strip_prefix("field:").map(str::to_string)
                            };
                        }))
                    }>
                        <option value="" prop:selected=first_value.is_empty()>{ui.t("block-list-first-none")}</option>
                        <option value="pick" prop:selected={first_value == "pick"}>{ui.t("block-list-first-pick")}</option>
                        {first_fields.get_value().into_iter().map(|(n, l)| {
                            let v = format!("field:{n}");
                            let on = v == first_value;
                            view! { <option value=v prop:selected=on>{ui.tx("block-list-first-field", &[("field", l)])}</option> }
                        }).collect_view()}
                        {(first_value.starts_with("field:") && !first_fields.with_value(|f| f.iter().any(|(n, _)| format!("field:{n}") == first_value))).then(|| {
                            let v = first_value.clone();
                            view! { <option value=v.clone() prop:selected=true>{v[6..].to_string()}</option> }
                        })}
                    </select>
                </div>
            </label>
            {first_view}
            {(!first_value.is_empty()).then(|| {
                let full = q.with_value(|q| q.first_full);
                view! {
                    <label class="view-line" title=ui.t("block-list-first-full-help")>
                        <span>{ui.t("block-list-first-full")}</span>
                        <span class="switch">
                            <input type="checkbox" prop:checked=full on:change=move |e| {
                                let on = event_target_checked(&e);
                                write(Box::new(move |q| q.first_full = on))
                            }/>
                            <span class="switch-track"></span>
                        </span>
                    </label>
                }
            })}
            <label class="field">
                <span class="label">{ui.t("block-list-contains")}</span>
                <div class="select">
                    <select class="input" title=ui.t("block-list-contains-help") on:change=move |e| {
                        let v = event_target_value(&e);
                        write(Box::new(move |q| {
                            q.contains = (!v.is_empty()).then(|| (v.clone(), if on_template { "page.fileSlug".into() } else { "\"\"".into() }))
                        }))
                    }>
                        <option value="" prop:selected=contains_field.is_empty()>{ui.t("block-list-contains-none")}</option>
                        {move || fields().into_iter().filter(|(p, _)| p != "date").map(|(p, l)| {
                            let on = p == contains_field;
                            view! { <option value=p prop:selected=on>{l}</option> }
                        }).collect_view()}
                    </select>
                </div>
            </label>
            {(!contains_value.is_empty()).then(|| {
                let shown = if contains_value == "page.fileSlug" {
                    String::new()
                } else {
                    revel_edit::canvas::literal(&contains_value).unwrap_or(contains_value.clone())
                };
                let opts = cols.with(|cs| field_values(ui, cs, current().as_ref(), &contains_field2));
                view! {
                    <div class="field">
                        <span class="label" title=ui.t("block-list-contains-value-help")>{ui.t("block-list-contains-value")}</span>
                        {value_picker(ui, opts, shown, "block-list-contains-current", move |v: String| {
                            let v = v.trim().to_string();
                            write(Box::new(move |q| {
                                if let Some(c) = q.contains.as_mut() {
                                    c.1 = if v.is_empty() { "page.fileSlug".into() } else { revel_edit::canvas::quote(&v) };
                                }
                            }))
                        })}
                    </div>
                }
            })}
            <label class="field">
                <span class="label">{ui.t("block-list-filter")}</span>
                <div class="select">
                    <select class="input" on:change=move |e| {
                        let v = event_target_value(&e);
                        write(Box::new(move |q| q.filter = (!v.is_empty()).then(|| (v.clone(), None))))
                    }>
                        <option value="" prop:selected=!has_filter>{ui.t("block-list-all")}</option>
                        {move || fields().into_iter().filter(|(p, _)| p != "date").map(|(p, l)| {
                            let on = p == ff;
                            view! { <option value=p prop:selected=on>{l}</option> }
                        }).collect_view()}
                    </select>
                </div>
            </label>
            {has_filter.then(|| {
                let opts = cols.with(|cs| field_values(ui, cs, current().as_ref(), &filter_field));
                view! {
                <div class="field">
                    <span class="label">{ui.t("block-list-filter-value")}</span>
                    {value_picker(ui, opts, filter_value.clone(), "block-list-filter-any", move |v: String| {
                            let v = v.trim().to_string();
                            write(Box::new(move |q| {
                                if let Some(f) = q.filter.as_mut() {
                                    f.1 = if v.is_empty() {
                                        None
                                    } else if matches!(v.as_str(), "true" | "false") || v.parse::<f64>().is_ok() {
                                        Some(v.clone())
                                    } else {
                                        Some(revel_edit::canvas::quote(&v))
                                    };
                                }
                            }))
                        })}
                </div>
                }
            })}
            <div class="field-pair">
                <label class="field">
                    <span class="label">{ui.t("block-list-sort")}</span>
                    <div class="select">
                        <select class="input" data-list="sort" on:change=move |e| {
                            let v = event_target_value(&e);
                            write(Box::new(move |q| {
                                let desc = q.sort.as_ref().map(|s| s.1).unwrap_or(v == "date");
                                q.sort = (!v.is_empty()).then(|| (v.clone(), desc));
                            }))
                        }>
                            <option value="" prop:selected=!has_sort>{ui.t("block-list-sort-default")}</option>
                            {move || fields().into_iter().map(|(p, l)| {
                                let on = p == sf;
                                view! { <option value=p prop:selected=on>{l}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                </label>
                <label class="field">
                    <span class="label">{ui.t("block-list-order")}</span>
                    <div class="select">
                        <select class="input" disabled=!has_sort on:change=move |e| {
                            let d = event_target_value(&e) == "desc";
                            write(Box::new(move |q| { if let Some(s) = q.sort.as_mut() { s.1 = d; } }))
                        }>
                            <option value="asc" prop:selected=!desc>{ui.t("block-list-asc")}</option>
                            <option value="desc" prop:selected=desc>{ui.t("block-list-desc")}</option>
                        </select>
                    </div>
                </label>
            </div>
            <label class="field">
                <span class="label">{ui.t("block-list-limit")}</span>
                <input class="input" type="number" min="1" step="1" prop:value=limit placeholder=ui.t("block-list-limit-all")
                    on:change=move |e| {
                        let n = event_target_value(&e).trim().parse::<usize>().ok().filter(|n| *n > 0);
                        write(Box::new(move |q| q.limit = n))
                    }/>
            </label>
            <label class="field">
                <span class="label">{ui.t("block-list-start")}{icons::info_tip(ui.tx("block-list-start-help", &[]))}</span>
                <input class="input" type="number" min="1" step="1" data-list="start" prop:value=start placeholder="1"
                    on:change=move |e| {
                        let n = event_target_value(&e).trim().parse::<usize>().ok().filter(|n| *n > 1).map(|n| n - 1).unwrap_or(0);
                        write(Box::new(move |q| q.offset = n))
                    }/>
            </label>
            {holder.map(|(t, more)| {
                let t = StoredValue::new(t);
                view! {
                    <label class="field" title=ui.t("block-list-more-help")>
                        <span class="label">{ui.t("block-list-more")}</span>
                        <input class="input" type="number" min="1" step="1" data-list="more" prop:value=more
                            placeholder=ui.t("block-list-limit-all")
                            on:change=move |e| {
                                let n = event_target_value(&e).trim().parse::<usize>().ok().filter(|n| *n > 0);
                                let at = t.get_value();
                                ed.apply(ui, match n {
                                    Some(n) => Command::SetAttr { at, name: "data-revel-more".into(), value: Some(n.to_string()) },
                                    None => Command::RemoveAttr { at, name: "data-revel-more".into() },
                                }, false);
                            }/>
                    </label>
                }
            })}
            {on_template.then(|| view! {
                <label class="view-line" title=ui.t("block-list-exclude-help")>
                    <span>{ui.t("block-list-exclude")}</span>
                    <span class="switch">
                        <input type="checkbox" prop:checked=exclude on:change=move |e| {
                            let on = event_target_checked(&e);
                            write(Box::new(move |q| q.exclude_current = on))
                        }/>
                        <span class="switch-track"></span>
                    </span>
                </label>
            })}
            </div>
        </div>
    }
    .into_any()
}

// ------------------------------------------------------------------ éléments reliés au contenu

/// Boucle de collection la plus proche qui contient le nœud : sa cible, ses réglages, le nom de
/// sa variable d'élément.
pub(crate) fn enclosing_list(d: &Site, at: &Target) -> Option<(Query, String)> {
    let mut cur = at.clone();
    for _ in 0..64 {
        let (parent, _) = revel_edit::address::locate(d, &cur).ok()?;
        let id = parent.node?;
        let t = Target {
            owner: at.owner.clone(),
            node: id,
        };
        if let Ok(Node::Each { expr, item, .. }) = revel_edit::address::node(d, &t)
            && parent.branch == Branch::Children
        {
            // Liste de collection, ou contenus désignés par un champ (liste dans une liste).
            let q = Query::parse(expr).or_else(|| match FieldList::parse(expr)? {
                FieldList::Refs { collection, .. } => Some(Query {
                    collection,
                    ..Default::default()
                }),
                FieldList::Values { .. } => None,
            })?;
            let item = item.trim();
            return revel_model::valid_prop_name(item).then(|| (q, item.to_string()));
        }
        cur = t;
    }
    None
}

/// Identifiant de la liste de collection la plus proche qui contient le nœud.
pub(crate) fn enclosing_list_id(d: &Site, at: &Target) -> Option<String> {
    let mut cur = at.clone();
    for _ in 0..64 {
        let (parent, _) = revel_edit::address::locate(d, &cur).ok()?;
        let id = parent.node?;
        let t = Target {
            owner: at.owner.clone(),
            node: id.clone(),
        };
        if matches!(revel_edit::address::node(d, &t), Ok(Node::Each { .. }))
            && parent.branch == Branch::Children
        {
            return Some(id);
        }
        cur = t;
    }
    None
}

/// Liste des valeurs d'un champ (`item.data.tags`) la plus proche qui contient le nœud : nom de
/// sa variable (chaque valeur).
fn enclosing_values(d: &Site, at: &Target) -> Option<String> {
    enclosing_values_at(d, at).map(|(v, _, _)| v)
}

/// Champ du contenu relié à une propriété liste d'un composant, d'après ses utilisations dans
/// les pages et les gabarits : (collection, nom du champ). Sert à connaître les sous-champs
/// d'une liste parcourue dans le composant (blocs d'un projet).
pub(crate) fn list_prop_source(d: &Site, cid: &str, prop: &str) -> Option<(String, String)> {
    let mut found: Option<(String, String)> = None;
    let owners = d
        .layouts
        .iter()
        .map(|l| (revel_edit::Owner::Layout(l.id.clone()), &l.body))
        .chain(
            d.pages
                .iter()
                .map(|p| (revel_edit::Owner::Page(p.id.clone()), &p.body)),
        );
    for (owner, body) in owners {
        if found.is_some() {
            break;
        }
        revel_model::ids::walk(body, &mut |n| {
            if found.is_some() {
                return;
            }
            if let Node::Use {
                id: Some(id),
                component,
                args,
                ..
            } = n
                && component == cid
                && let Some(e) = args.get(prop)
            {
                let t = Target {
                    owner: owner.clone(),
                    node: id.clone(),
                };
                if let Some((col, prefix, _)) = binding_context(d, &t)
                    && let Some(f) = e.trim().strip_prefix(prefix.as_str())
                    && revel_model::valid_prop_name(f)
                {
                    found = Some((col, f.to_string()));
                }
            }
        });
    }
    found
}

/// Liste des valeurs d'un champ qui contient le nœud : variable, champ (`activities`,
/// `item.data.tags`) et boucle.
fn enclosing_values_at(d: &Site, at: &Target) -> Option<(String, String, Target)> {
    let mut cur = at.clone();
    for _ in 0..64 {
        let (parent, _) = revel_edit::address::locate(d, &cur).ok()?;
        let id = parent.node?;
        let t = Target {
            owner: at.owner.clone(),
            node: id,
        };
        if let Ok(Node::Each { expr, item, .. }) = revel_edit::address::node(d, &t)
            && parent.branch == Branch::Children
        {
            let item = item.trim().to_string();
            return match FieldList::parse(expr) {
                Some(FieldList::Values { field }) if revel_model::valid_prop_name(&item) => {
                    Some((item, field, t))
                }
                _ => None,
            };
        }
        cur = t;
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    Text,
    Src,
    Href,
    /// Image de fond d'un bloc (`style="background-image: url(…)"`).
    Bg,
}

/// Texte d'un élément relié à un champ : texte écrit avant et après, champ, écriture d'une date,
/// texte riche (`| safe`), expression non reconnue.
#[derive(Clone, Default, PartialEq)]
struct TextBind {
    before: String,
    after: String,
    field: String,
    /// Champ lu quand le premier est vide (`(a or b)`).
    fallback: Option<String>,
    format: Option<String>,
    safe: bool,
    /// Nombre de valeurs d'un champ liste (`x | length`).
    count: bool,
    /// Temps de lecture en minutes d'un texte ou des blocs (`x | reading_time`, D-172).
    reading: bool,
    /// Mots entre astérisques mis en valeur : classe (`x | emphasis("accent")`).
    emphasis: Option<String>,
    /// Valeurs d'un champ liste séparées (`x | join(", ")`) : séparateur.
    join: Option<String>,
    /// Texte écrit quand le champ (et son repli) est vide (`(x or "À venir")`).
    fallback_text: Option<String>,
    raw: Option<String>,
}

/// Lit le contenu d'un élément : vide, texte écrit, ou `[texte] {{ champ | filtre }} [texte]`.
fn text_bind(children: &[Node], field_of: &dyn Fn(&str) -> Option<String>) -> Option<TextBind> {
    let (before, out, after) = match children {
        [] | [Node::Text { .. }] => return Some(TextBind::default()),
        [Node::Out { expr }] => ("", expr, ""),
        [Node::Text { text }, Node::Out { expr }] => (text.as_str(), expr, ""),
        [Node::Out { expr }, Node::Text { text }] => ("", expr, text.as_str()),
        [
            Node::Text { text: a },
            Node::Out { expr },
            Node::Text { text: b },
        ] => (a.as_str(), expr, b.as_str()),
        _ => return None,
    };
    let mut b = TextBind {
        before: before.to_string(),
        after: after.to_string(),
        ..Default::default()
    };
    let mut b_fallback = None;
    let (head, tail) = match out.split_once('|') {
        Some((h, t)) => (h.trim(), Some(t.trim())),
        None => (out.trim(), None),
    };
    // Champ, sinon un autre : `(a or b)`.
    let inner = head
        .strip_prefix('(')
        .and_then(|h| h.strip_suffix(')'))
        .unwrap_or(head);
    let head = match inner
        .split(" or ")
        .map(str::trim)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [a, b] if field_of(a).is_some() && field_of(b).is_some() => {
            b_fallback = field_of(b);
            *a
        }
        // Champ, sinon un texte écrit : `(a or "À venir")`.
        [a, t] if field_of(a).is_some() && revel_edit::canvas::literal(t).is_some() => {
            b.fallback_text = revel_edit::canvas::literal(t);
            *a
        }
        [a, f2, t]
            if field_of(a).is_some()
                && field_of(f2).is_some()
                && revel_edit::canvas::literal(t).is_some() =>
        {
            b_fallback = field_of(f2);
            b.fallback_text = revel_edit::canvas::literal(t);
            *a
        }
        _ => head,
    };
    b.fallback = b_fallback;
    match (field_of(head), tail) {
        (Some(f), None) => b.field = f,
        (Some(f), Some("safe")) => {
            b.field = f;
            b.safe = true;
        }
        (Some(f), Some("length")) => {
            b.field = f;
            b.count = true;
        }
        (Some(f), Some("reading_time")) => {
            b.field = f;
            b.reading = true;
        }
        (Some(f), Some(t)) if t.starts_with("emphasis(") && t.ends_with(')') => {
            b.field = f;
            let arg = t["emphasis(".len()..t.len() - 1].trim();
            b.emphasis = Some(revel_edit::canvas::literal(arg).unwrap_or_else(|| "accent".into()));
        }
        (Some(f), Some(t)) if revel_edit::dates::format_of(t).is_some() => {
            b.field = f;
            b.format = revel_edit::dates::format_of(t);
        }
        // Valeurs d'un champ liste séparées.
        (Some(f), Some(t))
            if t.starts_with("join(")
                && t.ends_with(')')
                && revel_edit::canvas::literal(&t[5..t.len() - 1]).is_some() =>
        {
            b.field = f;
            b.join = revel_edit::canvas::literal(&t[5..t.len() - 1]);
        }
        // Écriture propre au site (filtre à une valeur : « Prénom NOM »).
        (Some(f), Some(t))
            if !t.is_empty() && t.chars().all(|c| c.is_alphanumeric() || c == '_') =>
        {
            b.field = f;
            b.format = Some(t.to_string());
        }
        _ => b.raw = Some(out.trim().to_string()),
    }
    Some(b)
}

/// Contenu d'un élément relié : texte avant, valeur du champ, texte après.
fn text_children(b: &TextBind, prefix: &str) -> Vec<Node> {
    let mut v = Vec::new();
    if !b.before.is_empty() {
        v.push(Node::Text {
            text: b.before.clone(),
        });
    }
    let text = b.fallback_text.as_deref().map(revel_edit::canvas::quote);
    let mut expr = match (&b.fallback, &text) {
        (Some(fb), Some(t)) => format!("({prefix}{} or {prefix}{fb} or {t})", b.field),
        (Some(fb), None) => format!("({prefix}{} or {prefix}{fb})", b.field),
        (None, Some(t)) => format!("({prefix}{} or {t})", b.field),
        (None, None) => format!("{prefix}{}", b.field),
    };
    if b.count {
        expr.push_str(" | length");
    } else if b.reading {
        expr.push_str(" | reading_time");
    } else if let Some(sep) = &b.join {
        expr.push_str(&format!(" | join({})", revel_edit::canvas::quote(sep)));
    } else if let Some(c) = &b.emphasis {
        expr.push_str(&format!(" | emphasis({})", revel_edit::canvas::quote(c)));
    } else if let Some(f) = &b.format {
        expr.push_str(" | ");
        expr.push_str(f);
    } else if b.safe {
        expr.push_str(" | safe");
    }
    v.push(Node::Out { expr });
    if !b.after.is_empty() {
        v.push(Node::Text {
            text: b.after.clone(),
        });
    }
    v
}

/// Image de fond reliée : `background-image: url('{{ expr }}')` (seule déclaration du style).
fn bg_expr(style: &str) -> Option<String> {
    let v = style.trim().trim_end_matches(';').trim();
    let inner = v
        .strip_prefix("background-image:")?
        .trim()
        .strip_prefix("url(")?
        .strip_suffix(')')?
        .trim()
        .trim_matches(|c| c == '\'' || c == '"');
    let e = inner.strip_prefix("{{")?.strip_suffix("}}")?.trim();
    (!e.is_empty()).then(|| e.to_string())
}

/// Condition d'affichage d'un élément relié : enveloppe `If` qui ne contient que lui.
fn visibility_of(d: &Site, at: &Target) -> Option<(Target, String)> {
    let (parent, _) = revel_edit::address::locate(d, at).ok()?;
    let id = parent.node.clone()?;
    if parent.branch != Branch::Then {
        return None;
    }
    let t = Target {
        owner: at.owner.clone(),
        node: id,
    };
    match revel_edit::address::node(d, &t).ok()? {
        Node::If {
            cond,
            then,
            otherwise,
            ..
        } if then.len() == 1 && otherwise.is_empty() => Some((t, cond.clone())),
        _ => None,
    }
}

/// Contenu auquel un élément peut se relier : collection, début des expressions de ses champs,
/// expression de l'adresse de sa page. Dans une liste de collection : l'élément de la liste
/// (`item.data.client`, `item.url`) ; dans le modèle d'une collection (page de collection de
/// Webflow) : le contenu affiché (`client`, `page.url`).
fn binding_context(d: &Site, at: &Target) -> Option<(String, String, String)> {
    if let Some((q, item)) = enclosing_list(d, at) {
        return Some((q.key(), format!("{item}.data."), format!("{item}.url")));
    }
    // Composant qui a une propriété « Contenu » (D-174) : champs de sa collection.
    if let revel_edit::Owner::Component(cid) = &at.owner
        && let Some(p) = d.component(cid).and_then(|c| {
            c.props
                .iter()
                .find(|p| p.kind == revel_model::PropKind::Entry)
        })
        && let Some(col) = p.collection.clone()
    {
        return Some((col, format!("{}.data.", p.name), format!("{}.url", p.name)));
    }
    template_collection(d, at).map(|c| (c, String::new(), "page.url".into()))
}

/// Zone du site (D-182) : en-tête (`false`) ou pied (`true`) d'un gabarit — élément `header` ou
/// `footer`, ou utilisation d'un composant dont la racine l'est.
pub(crate) fn zone_of(d: &Site, at: &Target) -> Option<bool> {
    if !matches!(at.owner, revel_edit::Owner::Layout(_)) {
        return None;
    }
    let tag = match revel_edit::address::node(d, at).ok()? {
        Node::Element { tag, .. } => tag.clone(),
        Node::Use { component, .. } => {
            d.component(component)?.body.iter().find_map(|n| match n {
                Node::Element { tag, .. } => Some(tag.clone()),
                _ => None,
            })?
        }
        _ => return None,
    };
    match tag.to_ascii_lowercase().as_str() {
        "header" => Some(false),
        "footer" => Some(true),
        _ => None,
    }
}

/// Clé de la page qui masque la zone (propriété du gabarit, interrupteur éteint par défaut).
fn zone_key(footer: bool) -> &'static str {
    if footer { "sans_pied" } else { "sans_entete" }
}

/// Gabarits de la page affichée, du sien à ses parents.
fn page_layouts(d: &Site, layout: Option<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = layout;
    while let Some(id) = cur {
        if out.contains(&id) {
            break;
        }
        cur = d.layout(&id).and_then(|l| l.parent.clone());
        out.push(id);
    }
    out
}

/// Outil « En-tête » ou « Pied » : zone déjà là (son identifiant, à sélectionner), sinon l'endroit
/// où la poser dans le gabarit de la page (début ou fin de `body`).
pub(crate) fn zone_place(
    ed: Ed,
    footer: bool,
) -> Option<Result<String, (revel_edit::Parent, usize)>> {
    let info = ed.page_ref_untracked(&ed.page.get_untracked())?;
    ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let layout = info
            .page
            .as_ref()
            .and_then(|id| d.pages.iter().find(|p| p.id == *id))
            .and_then(|p| p.layout.clone())
            .or(info.layout.clone());
        let layouts = page_layouts(d, layout);
        for lid in &layouts {
            let owner = revel_edit::Owner::Layout(lid.clone());
            let mut found = None;
            revel_model::ids::walk(&d.layout(lid)?.body, &mut |n| {
                let id = match n {
                    Node::Element { id, .. } | Node::Use { id, .. } => id.clone(),
                    _ => None,
                };
                if found.is_none()
                    && let Some(id) = id
                    && zone_of(
                        d,
                        &Target {
                            owner: owner.clone(),
                            node: id.clone(),
                        },
                    ) == Some(footer)
                {
                    found = Some(id);
                }
            });
            if let Some(id) = found {
                return Some(Ok(id));
            }
        }
        // Gabarit le plus haut qui a un `body`, sinon celui de la page.
        for lid in layouts.iter().rev() {
            let mut body = None;
            revel_model::ids::walk(&d.layout(lid)?.body, &mut |n| {
                if let Node::Element {
                    id: Some(id),
                    tag,
                    children,
                    ..
                } = n
                    && tag.eq_ignore_ascii_case("body")
                    && body.is_none()
                {
                    body = Some((id.clone(), children.len()));
                }
            });
            if let Some((id, n)) = body {
                return Some(Err((
                    revel_edit::Parent {
                        owner: revel_edit::Owner::Layout(lid.clone()),
                        node: Some(id),
                        branch: Default::default(),
                    },
                    if footer { n } else { 0 },
                )));
            }
        }
        let lid = layouts.first()?;
        let n = d.layout(lid)?.body.len();
        Some(Err((
            revel_edit::Parent {
                owner: revel_edit::Owner::Layout(lid.clone()),
                node: None,
                branch: Default::default(),
            },
            if footer { n } else { 0 },
        )))
    })
}

/// Réglage d'une zone du site : « Montré sur cette page ». Éteint : la page reçoit la valeur qui
/// masque la zone ; la première fois, le gabarit reçoit l'interrupteur et la zone sa condition.
fn zone_settings(ui: Ui, ed: Ed, at: &Target) -> Option<AnyView> {
    let footer = ed.doc.with_untracked(|d| zone_of(d.as_ref()?, at))?;
    let key = zone_key(footer);
    // Page du site (pas un modèle ni un gabarit vu seul).
    let page = ed.page_ref_untracked(&ed.page.get_untracked())?.page?;
    let shown = ed.doc.with_untracked(|d| {
        let p = d.as_ref()?.pages.iter().find(|p| p.id == page)?;
        Some(p.props.get(key).and_then(serde_json::Value::as_bool) != Some(true))
    })?;
    let at = StoredValue::new(at.clone());
    let page = StoredValue::new(page);
    let name = if footer { "zone-footer" } else { "zone-header" };
    let toggle = move |on: bool| {
        let at = at.get_value();
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let mut commands = Vec::new();
            if !on {
                let revel_edit::Owner::Layout(lid) = &at.owner else {
                    return None;
                };
                let l = d.layout(lid)?;
                if !revel_edit::editable::layout_fields(d, Some(lid))
                    .iter()
                    .any(|f| f.name == key)
                {
                    let mut fields = l.fields.clone();
                    fields.push(revel_model::PropDef {
                        name: key.into(),
                        label: Some(ui.now(
                            if footer {
                                "zone-hide-footer"
                            } else {
                                "zone-hide-header"
                            },
                            &[],
                        )),
                        kind: revel_model::PropKind::Switch,
                        default: None,
                        collection: None,
                    });
                    commands.push(Command::SetLayoutFields {
                        id: lid.clone(),
                        fields,
                    });
                }
                let cond = format!("not {key}");
                let wrapped = visibility_of(d, &at).is_some_and(|(_, c)| c.trim() == cond);
                if !wrapped {
                    let node = revel_edit::address::node(d, &at).ok()?.clone();
                    let (parent, index) = revel_edit::address::locate(d, &at).ok()?;
                    commands.push(Command::Replace {
                        parent,
                        index,
                        node: Node::If {
                            id: Some(revel_model::ids::fresh(&d.node_ids())),
                            cond,
                            then: vec![node],
                            otherwise: Vec::new(),
                        },
                    });
                }
            }
            commands.push(Command::SetPageProp {
                page: page.get_value(),
                key: key.into(),
                value: (!on).then_some(serde_json::Value::Bool(true)),
            });
            Some(Command::Batch { commands })
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    Some(
        view! {
            <div class="insp-group zone-tool" data-zone=if footer { "footer" } else { "header" }>
                <p class="panel-title tool-head">
                    <span class="tool-icon">{icons::template()}</span><span>{ui.t(name)}</span>
                    <span class="layer-badge" title=ui.t("zone-site-help")>{ui.t("zone-site")}</span>
                </p>
                <label class="view-line">
                    <span>{ui.t("zone-shown")}</span>
                    <span class="switch">
                        <input type="checkbox" class="zone-shown" prop:checked=shown on:change=move |e| toggle(event_target_checked(&e))/>
                        <span class="switch-track"></span>
                    </span>
                </label>
            </div>
        }
        .into_any(),
    )
}

/// Élément « Champ CMS » (D-182) : son seul contenu est un champ lu dans une collection (texte)
/// ou son image en vient. Noms lisibles de la collection et du champ, pour les calques.
pub(crate) fn cms_label(d: &Site, cols: &[CmsCollection], at: &Target) -> Option<(String, String)> {
    let expr = match revel_edit::address::node(d, at).ok()? {
        Node::Element { children, .. } => match children.as_slice() {
            [Node::Out { expr }] => expr.trim().trim_start_matches('(').to_string(),
            _ => return None,
        },
        Node::Image(img) if revel_edit::canvas::literal(&img.src).is_none() => {
            img.src.trim().to_string()
        }
        _ => return None,
    };
    let mut ctxs: Vec<(String, String)> = binding_context(d, at)
        .map(|(c, p, _)| (c, p))
        .into_iter()
        .collect();
    ctxs.extend(
        enclosing_collection_loops(d, at)
            .into_iter()
            .map(|(v, c)| (c, format!("{v}.data."))),
    );
    let (col, prefix) = ctxs
        .into_iter()
        .filter(|(_, p)| expr.starts_with(p.as_str()))
        .max_by_key(|(_, p)| p.len())?;
    let name: String = expr[prefix.len()..]
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if name.is_empty() {
        return None;
    }
    let c = cols.iter().find(|c| c.id == col);
    let field = c
        .and_then(|c| c.fields.iter().find(|f| f.name == name))
        .and_then(|f| f.label.clone())
        .unwrap_or_else(|| revel_edit::overrides::field_label(&name));
    let collection = c
        .map(label_of)
        .unwrap_or_else(|| crate::site::capitalize(&col));
    Some((collection, field))
}

/// Élément « Champ CMS » prêt à poser dans `parent` : texte relié au premier champ de texte de
/// la collection lisible à cet endroit (titre de préférence). `None` hors d'un tel endroit ou
/// avant le chargement des collections : l'outil pose alors un texte écrit.
pub(crate) fn cms_field_node(ed: Ed, parent: &revel_edit::Parent) -> Option<Node> {
    let (col, prefix, id) = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let node = parent.node.clone()?;
        let at = Target {
            owner: parent.owner.clone(),
            node,
        };
        // L'endroit visé : le parent lui-même peut être la liste.
        let ctx = match revel_edit::address::node(d, &at) {
            Ok(Node::Each { expr, item, .. }) => {
                Query::parse(expr).map(|q| (q.key(), format!("{}.data.", item.trim())))
            }
            _ => None,
        }
        .or_else(|| binding_context(d, &at).map(|(c, p, _)| (c, p)))?;
        Some((ctx.0, ctx.1, revel_model::ids::fresh(&d.node_ids())))
    })?;
    let field = d_fields(ed, &col)?;
    Some(Node::Element {
        id: Some(id),
        tag: "p".into(),
        attrs: Vec::new(),
        children: vec![Node::Out {
            expr: format!("{prefix}{field}"),
        }],
    })
}

/// Premier champ de texte d'une collection (schéma du site), « title » de préférence.
fn d_fields(ed: Ed, col: &str) -> Option<String> {
    let _ = ed;
    let cols = crate::textfield::cached_cols();
    let c = cols.iter().find(|c| c.id == col)?;
    let texts: Vec<&revel_protocol::CmsField> = c
        .fields
        .iter()
        .filter(|f| {
            !f.list && !f.hidden && matches!(f.kind.as_deref(), Some("string" | "text") | None)
        })
        .collect();
    texts
        .iter()
        .find(|f| f.name == "title")
        .or(texts.first())
        .map(|f| f.name.clone())
}

/// Collection dont le modèle de page est le gabarit propriétaire de `at`.
pub(crate) fn template_collection(d: &Site, at: &Target) -> Option<String> {
    let revel_edit::Owner::Layout(lid) = &at.owner else {
        return None;
    };
    d.collections
        .iter()
        .find(|c| {
            c.layout.as_deref().is_some_and(|t| {
                t == lid || lid.strip_prefix(t).is_some_and(|r| r.starts_with("--"))
            })
        })
        .map(|c| c.id.clone())
}

/// Texte, image ou lien d'un élément relié à un champ du contenu (« Get text from » de
/// Webflow) : dans une liste de collection ou dans le modèle des pages d'une collection. Date :
/// écriture choisie ; texte : écrit avant et après la valeur ; affichage selon un champ rempli
/// ou vide (« Conditional visibility »).
fn item_bindings(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    // Lien d'un menu du site : réglé par les informations du site.
    if ed
        .doc
        .with_untracked(|d| revel_edit::siteinfo::enclosing_menu(d.as_ref()?, at))
        .is_some()
    {
        return None;
    }
    if let Some(var) = ed.doc.with_untracked(|d| enclosing_values(d.as_ref()?, at)) {
        let main = value_binding(ui, ed, at, node, var);
        return main;
    }
    // Collections lisibles ici (D-182, outil « Champ CMS ») : celle de l'endroit (liste, contenu
    // du composant, modèle de pages), puis celles des listes qui l'entourent.
    let ctxs: Vec<(String, String, String)> = ed.doc.with_untracked(|d| {
        let Some(d) = d.as_ref() else {
            return Vec::new();
        };
        let mut out: Vec<(String, String, String)> = binding_context(d, at).into_iter().collect();
        for (var, col) in enclosing_collection_loops(d, at) {
            let prefix = format!("{var}.data.");
            if !out.iter().any(|c| c.1 == prefix) {
                out.push((col, prefix, format!("{var}.url")));
            }
        }
        out
    });
    if ctxs.is_empty() {
        return None;
    }
    // Collection de la valeur déjà reliée (préfixe le plus long), sinon celle de l'endroit.
    let exprs: Vec<String> = match node {
        Node::Element {
            attrs, children, ..
        } => children
            .iter()
            .filter_map(|c| match c {
                Node::Out { expr } => Some(expr.trim().trim_start_matches('(').to_string()),
                _ => None,
            })
            .chain(
                attr(attrs, "href")
                    .and_then(|a| a.value.clone())
                    .and_then(|h| h.split_once("{{").map(|(_, r)| r.trim().to_string())),
            )
            .collect(),
        Node::Image(img) => vec![img.src.trim().to_string()],
        _ => Vec::new(),
    };
    let cur = ctxs
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.1.is_empty() && exprs.iter().any(|e| e.starts_with(&c.1) || *e == c.2))
        .max_by_key(|(_, c)| c.1.len())
        .map_or(0, |(i, _)| i);
    let pick = RwSignal::new(cur);
    let (at, node) = (at.clone(), node.clone());
    let ctxs = StoredValue::new(ctxs);
    Some(
        (move || {
            let ctx = ctxs.with_value(|c| c[pick.get().min(c.len() - 1)].clone());
            bindings_in(ui, ed, &at, &node, ctx, ctxs, pick)
        })
        .into_any(),
    )
}

/// Réglages « Champ CMS » d'un élément pour une collection lisible à son endroit.
fn bindings_in(
    ui: Ui,
    ed: Ed,
    at: &Target,
    node: &Node,
    (collection_id, prefix, url_expr): (String, String, String),
    ctxs: StoredValue<Vec<(String, String, String)>>,
    pick: RwSignal<usize>,
) -> Option<AnyView> {
    let is_field = |f: &str| {
        !f.is_empty()
            && f.chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    };
    let pre = prefix.clone();
    let field_of = move |e: &str| {
        let e = e.trim();
        // Texte riche (HTML) : affiché tel quel (`| safe`).
        let e = e.strip_suffix("| safe").map(str::trim_end).unwrap_or(e);
        e.strip_prefix(pre.as_str())
            .filter(|f| is_field(f))
            .map(str::to_string)
    };
    let mut slots: Vec<(Slot, Option<String>)> = Vec::new();
    let mut text: Option<TextBind> = None;
    let mut link_affix: Option<(String, String)> = None;
    match node {
        Node::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            let is_link = tag.eq_ignore_ascii_case("a");
            if let Some(b) = text_bind(children, &field_of)
                && !(is_link && children.is_empty())
            {
                let cur = b.raw.clone().or_else(|| {
                    (!b.field.is_empty()).then(|| {
                        if b.count {
                            format!("count:{}", b.field)
                        } else if b.reading {
                            format!("read:{}", b.field)
                        } else if b.join.is_some() {
                            format!("join:{}", b.field)
                        } else {
                            b.field.clone()
                        }
                    })
                });
                slots.push((Slot::Text, cur));
                text = Some(b);
            }
            if is_link {
                let href = attr(attrs, "href")
                    .and_then(|a| a.value.clone())
                    .unwrap_or_default();
                // Adresse : `avant{{ champ }}après` (texte écrit autour du champ).
                let (inner, affix) = match (href.find("{{"), href.rfind("}}")) {
                    (Some(i), Some(j))
                        if i < j && !href[..i].contains('{') && !href[j + 2..].contains('{') =>
                    {
                        (
                            Some(href[i + 2..j].trim().to_string()),
                            (href[..i].to_string(), href[j + 2..].to_string()),
                        )
                    }
                    _ => (None, (String::new(), String::new())),
                };
                link_affix = Some(affix);
                let cur = match inner {
                    Some(e) if e == url_expr => Some("@url".to_string()),
                    Some(e) => Some(field_of(&e).unwrap_or(e)),
                    None => None,
                };
                slots.push((Slot::Href, cur));
            }
            // Bloc (pas un texte) : image de fond reliée.
            let style = attr(attrs, "style").and_then(|a| a.value.clone());
            let bound_bg = style.as_deref().and_then(bg_expr);
            if text.is_none() && (style.is_none() || bound_bg.is_some()) {
                slots.push((Slot::Bg, bound_bg.map(|e| field_of(&e).unwrap_or(e))));
            }
        }
        Node::Image(img) => {
            let cur = revel_edit::canvas::literal(&img.src)
                .is_none()
                .then(|| field_of(&img.src).unwrap_or_else(|| img.src.clone()));
            slots.push((Slot::Src, cur));
        }
        _ => {}
    }
    if slots.is_empty() {
        return None;
    }
    let cols = collections(ed);
    let at = StoredValue::new(at.clone());
    let prefix = StoredValue::new(prefix.clone());
    let url_expr = StoredValue::new(url_expr);
    let link_affix = StoredValue::new(link_affix.unwrap_or_default());
    let collection = StoredValue::new(collection_id);
    let text = StoredValue::new(text);
    // Écritures propres au site (filtres à une valeur) : nom, libellé.
    let site_formats: Vec<(String, String)> = ed.doc.with_untracked(|d| {
        d.as_ref()
            .map(|d| {
                d.settings
                    .filters
                    .iter()
                    .filter(|(_, f)| f.params.len() == 1)
                    .map(|(n, f)| {
                        (
                            n.clone(),
                            f.description.clone().unwrap_or_else(|| n.clone()),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    });
    let site_lang = ed.doc.with_untracked(|d| {
        d.as_ref()
            .map(|d| d.default_locale.clone())
            .unwrap_or_else(|| "fr".into())
    });
    // Lecture non suivie : appelée pendant la construction du panneau (une lecture suivie le
    // reconstruirait à l'arrivée des collections, avec une nouvelle liste vide).
    let field_kind = move |name: &str| -> Option<String> {
        cols.with_untracked(|cs| {
            cs.iter()
                .find(|c| collection.with_value(|id| c.id == *id))
                .and_then(|c| c.fields.iter().find(|f| f.name == name))
                .and_then(|f| f.kind.clone())
        })
    };
    let fields = move |slot: Slot| -> Vec<(String, String)> {
        cols.with(|cs| {
            cs.iter()
                .find(|c| collection.with_value(|id| c.id == *id))
                .map(|c| {
                    c.fields
                        .iter()
                        // Champ à plusieurs valeurs : pas un texte ni une image seule.
                        .filter(|f| !f.list)
                        .filter(|f| match slot {
                            Slot::Src | Slot::Bg => {
                                matches!(f.kind.as_deref(), Some("image" | "file") | None)
                            }
                            _ => !matches!(
                                f.kind.as_deref(),
                                Some("object" | "block" | "image" | "file" | "boolean")
                            ),
                        })
                        .map(|f| {
                            (
                                f.name.clone(),
                                f.label.clone().unwrap_or_else(|| f.name.clone()),
                            )
                        })
                        // Texte : aussi le nombre de valeurs d'un champ liste.
                        .chain(
                            c.fields
                                .iter()
                                .filter(|f| f.list && slot == Slot::Text)
                                .map(|f| {
                                    (
                                        format!("count:{}", f.name),
                                        ui.now(
                                            "block-bound-count",
                                            &[(
                                                "field",
                                                f.label.clone().unwrap_or_else(|| f.name.clone()),
                                            )],
                                        ),
                                    )
                                }),
                        )
                        // Texte : temps de lecture d'un texte riche ou des blocs (minutes).
                        .chain(
                            c.fields
                                .iter()
                                .filter(|f| {
                                    slot == Slot::Text
                                        && (matches!(
                                            f.kind.as_deref(),
                                            Some("rich-text" | "rich_text" | "markdown" | "text")
                                        ) || (f.list
                                            && matches!(
                                                f.kind.as_deref(),
                                                Some("object" | "block")
                                            )))
                                })
                                .map(|f| {
                                    (
                                        format!("read:{}", f.name),
                                        ui.now(
                                            "block-bound-reading",
                                            &[(
                                                "field",
                                                f.label.clone().unwrap_or_else(|| f.name.clone()),
                                            )],
                                        ),
                                    )
                                }),
                        )
                        // Texte : les valeurs d'un champ liste séparées (« Film, Identité »).
                        .chain(
                            c.fields
                                .iter()
                                .filter(|f| {
                                    f.list
                                        && slot == Slot::Text
                                        && !matches!(
                                            f.kind.as_deref(),
                                            Some("object" | "block" | "image" | "file")
                                        )
                                })
                                .map(|f| {
                                    (
                                        format!("join:{}", f.name),
                                        ui.now(
                                            "block-bound-join",
                                            &[(
                                                "field",
                                                f.label.clone().unwrap_or_else(|| f.name.clone()),
                                            )],
                                        ),
                                    )
                                }),
                        )
                        .collect()
                })
                .unwrap_or_default()
        })
    };
    // Élément entier remplacé par le même avec un autre contenu (même identifiant).
    let set_children = move |kids: Vec<Node>| {
        let at = at.get_value();
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let Ok(Node::Element { id, tag, attrs, .. }) = revel_edit::address::node(d, &at) else {
                return None;
            };
            let (parent, index) = revel_edit::address::locate(d, &at).ok()?;
            Some(Command::Replace {
                parent,
                index,
                node: Node::Element {
                    id: id.clone(),
                    tag: tag.clone(),
                    attrs: attrs.clone(),
                    children: kids,
                },
            })
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let set_text = move |f: Box<dyn Fn(&mut TextBind)>| {
        let mut b = text.get_value().unwrap_or_default();
        f(&mut b);
        if b.field.is_empty() {
            set_children(vec![Node::Text {
                text: ui.now("block-bound-text", &[]),
            }]);
        } else {
            set_children(text_children(&b, &prefix.get_value()));
        }
    };
    let set = move |slot: Slot, v: String| {
        let at = at.get_value();
        let prefix = prefix.get_value();
        let expr = |f: &str| format!("{prefix}{f}");
        let cmd = match slot {
            Slot::Text => {
                let kind = field_kind(&v);
                let rich = matches!(
                    kind.as_deref(),
                    Some("rich-text" | "rich_text" | "markdown")
                );
                let date = matches!(kind.as_deref(), Some("date" | "datetime"));
                let reading = v.strip_prefix("read:").map(str::to_string);
                let (field, count, join) = match (v.strip_prefix("count:"), v.strip_prefix("join:"))
                {
                    (Some(f), _) => (f.to_string(), true, false),
                    (_, Some(f)) => (f.to_string(), false, true),
                    _ => (reading.clone().unwrap_or_else(|| v.clone()), false, false),
                };
                let reading = reading.is_some();
                let rich = rich && !reading;
                set_text(Box::new(move |b| {
                    b.field = field.clone();
                    b.raw = None;
                    b.count = count;
                    b.reading = reading;
                    b.join = join.then(|| ", ".to_string());
                    b.safe = rich && !count && !join;
                    b.format = (date && !count && !join)
                        .then(|| revel_edit::dates::FORMATS[0].0.to_string());
                }));
                return;
            }
            Slot::Href => {
                let (before, after) = link_affix.get_value();
                Command::SetAttr {
                    at: at.clone(),
                    name: "href".into(),
                    value: Some(match v.as_str() {
                        "" => "#".into(),
                        "@url" => format!("{{{{ {} }}}}", url_expr.get_value()),
                        f => format!("{before}{{{{ {} }}}}{after}", expr(f)),
                    }),
                }
            }
            Slot::Bg if v.is_empty() => Command::RemoveAttr {
                at: at.clone(),
                name: "style".into(),
            },
            Slot::Bg => Command::SetAttr {
                at: at.clone(),
                name: "style".into(),
                value: Some(format!("background-image: url('{{{{ {} }}}}')", expr(&v))),
            },
            Slot::Src => {
                let Some(Node::Image(mut img)) = ed.doc.with_untracked(|d| {
                    d.as_ref()
                        .and_then(|d| revel_edit::address::node(d, &at).ok().cloned())
                }) else {
                    return;
                };
                img.src = if v.is_empty() {
                    "\"\"".into()
                } else {
                    expr(&v)
                };
                Command::SetImage { at, image: img }
            }
        };
        ed.apply(ui, cmd, false);
    };
    // Affichage : toujours, ou selon un champ rempli / vide (enveloppe `If`).
    let vis = ed
        .doc
        .with_untracked(|d| d.as_ref().and_then(|d| visibility_of(d, &at.get_value())));
    let vis_cond = vis.as_ref().map(|(_, c)| c.clone());
    let set_vis = move |cond: Option<String>| {
        let at = at.get_value();
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            // Enveloppe relue à chaque fois : le panneau n'est pas refait quand elle change.
            let target = visibility_of(d, &at).map(|(t, _)| t);
            match (target, cond) {
                // Condition changée.
                (Some(t), Some(c)) => Some(Command::SetLogic {
                    at: t,
                    cond: Some(c),
                    expr: None,
                    item: None,
                }),
                // Toujours affiché : l'enveloppe retirée.
                (Some(t), None) => {
                    let node = revel_edit::address::node(d, &at).ok()?.clone();
                    let (parent, index) = revel_edit::address::locate(d, &t).ok()?;
                    Some(Command::Replace {
                        parent,
                        index,
                        node,
                    })
                }
                // Nouvelle condition : l'élément enveloppé.
                (None, Some(c)) => {
                    let node = revel_edit::address::node(d, &at).ok()?.clone();
                    let (parent, index) = revel_edit::address::locate(d, &at).ok()?;
                    let id = revel_model::ids::fresh(&d.node_ids());
                    Some(Command::Replace {
                        parent,
                        index,
                        node: Node::If {
                            id: Some(id),
                            cond: c,
                            then: vec![node],
                            otherwise: Vec::new(),
                        },
                    })
                }
                (None, None) => None,
            }
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let cond_fields =
        condition_fields(ed, &at.get_value()).unwrap_or_else(|| Signal::derive(Vec::new));
    // Champ principal de l'élément (texte, image, sinon lien) : sa condition « rempli ».
    let main_field = slots
        .iter()
        .filter(|(s, _)| *s != Slot::Bg)
        .filter_map(|(_, c)| c.clone())
        .map(|c| {
            ["count:", "read:", "join:"]
                .iter()
                .find_map(|p| c.strip_prefix(p).map(str::to_string))
                .unwrap_or(c)
        })
        .find(|c| c != "@url" && is_field(c));
    let hide_cond = StoredValue::new(main_field.map(|f| format!("{}{f}", prefix.get_value())));
    let mode = match (&vis_cond, hide_cond.get_value()) {
        (None, _) => "keep",
        (Some(c), Some(h)) if c.trim() == h => "hide",
        _ => "cond",
    };
    let custom = RwSignal::new(mode == "cond");
    let vis_cond = StoredValue::new(vis_cond);
    // Affiché en texte, en image ou en lien : élément sans autre élément dedans.
    let shape: Option<&'static str> = match node {
        Node::Image(_) => Some("image"),
        Node::Element { tag, children, .. }
            if children
                .iter()
                .all(|c| matches!(c, Node::Text { .. } | Node::Out { .. })) =>
        {
            let leaf = text.with_value(Option::is_some) || children.is_empty();
            leaf.then_some(if tag.eq_ignore_ascii_case("a") {
                "link"
            } else {
                "text"
            })
        }
        _ => None,
    };
    let set_shape = move |to: &'static str| {
        let at = at.get_value();
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let old = revel_edit::address::node(d, &at).ok()?.clone();
            let (parent, index) = revel_edit::address::locate(d, &at).ok()?;
            let (id, class, kids) = match &old {
                Node::Element {
                    id,
                    attrs,
                    children,
                    ..
                } => (
                    id.clone(),
                    attr(attrs, "class").and_then(|a| a.value.clone()),
                    children.clone(),
                ),
                Node::Image(i) => (i.id.clone(), i.class.clone(), Vec::new()),
                _ => return None,
            };
            let kids = if kids.is_empty() {
                vec![Node::Text {
                    text: ui.now("block-bound-text", &[]),
                }]
            } else {
                kids
            };
            let mut attrs: Vec<Attr> = class
                .clone()
                .map(|c| Attr {
                    name: "class".into(),
                    value: Some(c),
                    when: None,
                })
                .into_iter()
                .collect();
            let node = match to {
                "image" => {
                    let first = cols.with_untracked(|cs| {
                        cs.iter()
                            .find(|c| collection.with_value(|id| c.id == *id))
                            .and_then(|c| {
                                c.fields.iter().find(|f| {
                                    !f.list && matches!(f.kind.as_deref(), Some("image" | "file"))
                                })
                            })
                            .map(|f| f.name.clone())
                    });
                    Node::Image(revel_model::ImageNode {
                        id,
                        src: first.map_or_else(
                            || "\"\"".to_string(),
                            |f| format!("{}{f}", prefix.get_value()),
                        ),
                        class,
                        ..Default::default()
                    })
                }
                "link" => {
                    attrs.push(Attr {
                        name: "href".into(),
                        value: Some(format!("{{{{ {} }}}}", url_expr.get_value())),
                        when: None,
                    });
                    Node::Element {
                        id,
                        tag: "a".into(),
                        attrs,
                        children: kids,
                    }
                }
                _ => Node::Element {
                    id,
                    tag: "p".into(),
                    attrs,
                    children: kids,
                },
            };
            Some(Command::Replace {
                parent,
                index,
                node,
            })
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let text_extra = text
        .with_value(|t| t.clone())
        .filter(|t| !t.field.is_empty() && t.raw.is_none());
    Some(
        view! {
            <div class="insp-group cms-field-tool">
                <p class="panel-title tool-head"><span class="tool-icon">{icons::database()}</span><span>{ui.t("insert-cms-field")}</span></p>
                <label class="field" data-bound="cms-field-collection">
                    <span class="label">{ui.t("cms-field-collection")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| { if let Ok(i) = event_target_value(&e).parse::<usize>() { pick.set(i) } }>
                            {move || ctxs.with_value(|c| c.clone()).into_iter().enumerate().map(|(i, (col, _, _))| {
                                let name = cols.with(|cs| cs.iter().find(|c| c.id == col).map(label_of))
                                    .unwrap_or_else(|| crate::site::capitalize(&col));
                                view! { <option value=i.to_string() prop:selected={i == pick.get_untracked()}>{name}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                </label>
                {shape.map(|cur| view! {
                    <div class="field" data-bound="cms-field-as">
                        <span class="label">{ui.t("cms-field-as")}</span>
                        <div class="segmented" role="radiogroup">
                            {[("text", "block-bound-text-label"), ("image", "block-bound-image"), ("link", "block-bound-link")].into_iter().map(|(v, k)| view! {
                                <button type="button" role="radio" data-as=v class:is-on={cur == v} aria-checked={(cur == v).to_string()}
                                    on:click=move |_| { if cur != v { set_shape(v) } }>{ui.t(k)}</button>
                            }).collect_view()}
                        </div>
                    </div>
                })}
                {slots.into_iter().map(|(slot, cur)| {
                    let key = match slot {
                        Slot::Text => "block-bound-text-label",
                        Slot::Src => "block-bound-image",
                        Slot::Href => "block-bound-link",
                        Slot::Bg => "block-bound-bg",
                    };
                    let cur = cur.unwrap_or_default();
                    let cur2 = cur.clone();
                    let cur3 = cur.clone();
                    view! {
                        <label class="field" data-bound=key>
                            <span class="label">{ui.t(if shape.is_some() && matches!(slot, Slot::Text | Slot::Src) { "cms-field-field" } else { key })}</span>
                            <div class="select">
                                <select class="input" on:change=move |e| set(slot, event_target_value(&e))>
                                    <option value="" prop:selected=cur.is_empty()>{ui.t(if slot == Slot::Text { "block-bound-written" } else { "block-bound-none" })}</option>
                                    {(slot == Slot::Href).then(|| view! {
                                        <option value="@url" prop:selected={cur == "@url"}>{ui.t("block-bound-page")}</option>
                                    })}
                                    {move || {
                                        let list = fields(slot);
                                        let known = cur2.is_empty() || cur2 == "@url" || list.iter().any(|(n, _)| *n == cur2);
                                        let extra = (!known).then(|| {
                                            let c = cur2.clone();
                                            // Valeur d'une autre forme : nommée, jamais montrée en expression.
                                            view! { <option value=c.clone() prop:selected=true>{revel_edit::overrides::field_label(&c)}</option> }
                                        });
                                        view! {
                                            {extra}
                                            {list.into_iter().map(|(n, l)| {
                                                let on = n == cur2;
                                                view! { <option value=n prop:selected=on>{l}</option> }
                                            }).collect_view()}
                                        }
                                    }}
                                </select>
                            </div>
                        </label>
                        {(slot == Slot::Href && !cur.is_empty() && cur != "@url").then(|| {
                            let (before, after) = link_affix.get_value();
                            let write_affix = move |b: Option<String>, a: Option<String>| {
                                let (b0, a0) = link_affix.get_value();
                                let (b1, a1) = (b.unwrap_or(b0), a.unwrap_or(a0));
                                link_affix.set_value((b1, a1));
                                set(Slot::Href, cur3.clone());
                            };
                            let w2 = write_affix.clone();
                            view! {
                                <div class="field-pair">
                                    <label class="field">
                                        <span class="label">{ui.t("block-bound-before")}</span>
                                        <input class="input input--mono" spellcheck="false" prop:value=before
                                            on:change=move |e| write_affix(Some(event_target_value(&e)), None)/>
                                    </label>
                                    <label class="field">
                                        <span class="label">{ui.t("block-bound-after")}</span>
                                        <input class="input input--mono" spellcheck="false" prop:value=after
                                            on:change=move |e| w2(None, Some(event_target_value(&e)))/>
                                    </label>
                                </div>
                            }
                        })}
                    }
                }).collect_view()}
                {text_extra.map(|b| {
                    let has_format = b.format.is_some();
                    let field = b.field.clone();
                    // Champ date : suivi (les collections arrivent après la construction).
                    let is_date = move || has_format || cols.with(|cs| {
                        cs.iter()
                            .find(|c| collection.with_value(|id| c.id == *id))
                            .and_then(|c| c.fields.iter().find(|f| f.name == field))
                            .is_some_and(|f| matches!(f.kind.as_deref(), Some("date" | "datetime")))
                    });
                    let cur_fmt = b.format.clone().unwrap_or_default();
                    let lang = site_lang.clone();
                    let fb = b.fallback.clone().unwrap_or_default();
                    let main = b.field.clone();
                    view! {
                        <label class="field" data-bound="block-bound-fallback">
                            <span class="label" title=ui.t("block-bound-fallback-help")>{ui.t("block-bound-fallback")}</span>
                            <div class="select">
                                <select class="input" on:change=move |e| {
                                    let v = event_target_value(&e);
                                    set_text(Box::new(move |b| b.fallback = (!v.is_empty()).then(|| v.clone())));
                                }>
                                    <option value="" prop:selected=fb.is_empty()>"—"</option>
                                    {move || fields(Slot::Text).into_iter().filter(|(n, _)| *n != main && !n.contains(':')).map(|(n, l)| {
                                        let on = n == fb;
                                        view! { <option value=n prop:selected=on>{l}</option> }
                                    }).collect_view()}
                                </select>
                            </div>
                        </label>
                        <label class="field" data-bound="block-bound-fallback-text">
                            <span class="label" title=ui.t("block-bound-fallback-text-help")>{ui.t("block-bound-fallback-text")}</span>
                            <input class="input" prop:value=b.fallback_text.clone().unwrap_or_default() on:change=move |e| {
                                let v = event_target_value(&e);
                                set_text(Box::new(move |b| b.fallback_text = (!v.is_empty()).then(|| v.clone())));
                            }/>
                        </label>
                        {b.join.clone().map(|sep| view! {
                            <label class="field" data-bound="block-bound-join">
                                <span class="label">{ui.t("block-bound-join-sep")}</span>
                                <input class="input input--mono" prop:value=sep on:change=move |e| {
                                    let v = event_target_value(&e);
                                    set_text(Box::new(move |b| b.join = Some(v.clone())));
                                }/>
                            </label>
                        })}
                        {move || {
                            let date = is_date();
                            let formats = site_formats.clone();
                            (date || !formats.is_empty()).then(|| {
                            let cur_fmt = cur_fmt.clone();
                            let lang = lang.clone();
                            view! {
                            <label class="field" data-bound="block-bound-date">
                                <span class="label" title=ui.t("block-bound-format-help")>{ui.t(if date { "block-bound-date" } else { "block-bound-format" })}</span>
                                <div class="select">
                                    <select class="input" on:change=move |e| {
                                        let v = event_target_value(&e);
                                        set_text(Box::new(move |b| b.format = (!v.is_empty()).then(|| v.clone())));
                                    }>
                                        <option value="" prop:selected=cur_fmt.is_empty()>{ui.t("block-bound-date-raw")}</option>
                                        {date.then(|| revel_edit::dates::FORMATS.iter().map(|(f, _)| {
                                            let on = *f == cur_fmt;
                                            view! { <option value=*f prop:selected=on>{revel_edit::dates::example(f, &lang)}</option> }
                                        }).collect_view())}
                                        {formats.into_iter().map(|(f, label)| {
                                            let on = f == cur_fmt;
                                            view! { <option value=f prop:selected=on>{label}</option> }
                                        }).collect_view()}
                                    </select>
                                </div>
                            </label>
                        }})}}
                        // Mots entre *astérisques* mis en valeur (classe réglable).
                        {(b.format.is_none() && !b.safe).then(|| {
                            let on = b.emphasis.is_some();
                            let cls = b.emphasis.clone().unwrap_or_else(|| "accent".into());
                            view! {
                                <label class="view-line" title=ui.t("block-bound-emphasis-help")>
                                    <span>{ui.t("block-bound-emphasis")}</span>
                                    <span class="switch">
                                        <input type="checkbox" prop:checked=on on:change=move |e| {
                                            let on = event_target_checked(&e);
                                            set_text(Box::new(move |b| b.emphasis = on.then(|| "accent".to_string())));
                                        }/>
                                        <span class="switch-track"></span>
                                    </span>
                                </label>
                                {on.then(|| view! {
                                    <label class="field">
                                        <span class="label">{ui.t("block-bound-emphasis-class")}</span>
                                        <input class="input input--mono" prop:value=cls spellcheck="false" on:change=move |e| {
                                            let v = event_target_value(&e).trim().trim_start_matches('.').to_string();
                                            set_text(Box::new(move |b| b.emphasis = Some(if v.is_empty() { "accent".into() } else { v.clone() })));
                                        }/>
                                    </label>
                                })}
                            }
                        })}
                        <div class="field-pair">
                            <label class="field">
                                <span class="label">{ui.t("block-bound-before")}</span>
                                <input class="input" prop:value=b.before.clone() on:change=move |e| {
                                    let v = event_target_value(&e);
                                    set_text(Box::new(move |b| b.before = v.clone()));
                                }/>
                            </label>
                            <label class="field">
                                <span class="label">{ui.t("block-bound-after")}</span>
                                <input class="input" prop:value=b.after.clone() on:change=move |e| {
                                    let v = event_target_value(&e);
                                    set_text(Box::new(move |b| b.after = v.clone()));
                                }/>
                            </label>
                        </div>
                    }
                })}
                // Champ vide : élément gardé, masqué (affichage selon ce champ), ou selon une autre
                // condition (réglage complet dessous).
                <label class="field" data-bound="cms-field-empty">
                    <span class="label">{ui.t("cms-field-empty")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| { match event_target_value(&e).as_str() {
                            "keep" => { custom.set(false); set_vis(None) }
                            "hide" => { custom.set(false); if let Some(c) = hide_cond.get_value() { set_vis(Some(c)) } }
                            _ => custom.set(true),
                        } }>
                            <option value="keep" prop:selected={mode == "keep"}>{ui.t("cms-field-empty-keep")}</option>
                            {hide_cond.with_value(Option::is_some).then(|| view! {
                                <option value="hide" prop:selected={mode == "hide"}>{ui.t("cms-field-empty-hide")}</option>
                            })}
                            <option value="cond" prop:selected={mode == "cond"}>{ui.t("cms-field-empty-cond")}</option>
                        </select>
                    </div>
                </label>
                <Show when=move || custom.get()>
                    <div class="field" data-bound="block-bound-visibility">
                        <span class="label">{ui.t("block-bound-visibility")}</span>
                        {crate::visibility::condition_editor(ui, cond_fields, vis_cond.get_value(), true, set_vis)}
                    </div>
                </Show>
            </div>
        }
        .into_any(),
    )
}

/// Texte d'un élément dans une liste des valeurs d'un champ (`{% for tag in item.data.tags %}`) :
/// écrit, ou la valeur.
fn value_binding(ui: Ui, ed: Ed, at: &Target, node: &Node, var: String) -> Option<AnyView> {
    let Node::Element { children, .. } = node else {
        return None;
    };
    // Sous-champs d'une liste d'objets (« Activités » : titre, texte).
    let subs: Vec<(String, String, bool)> = ed
        .doc
        .with_untracked(|d| {
            let d = d.as_ref()?;
            let (_, field, each) = enclosing_values_at(d, at)?;
            match binding_context(d, &each) {
                Some((col, prefix, _)) => {
                    let name = field
                        .strip_prefix(prefix.as_str())
                        .unwrap_or(&field)
                        .to_string();
                    Some((col, name))
                }
                // Liste d'une propriété du composant : champ relié à ses utilisations.
                None => match &at.owner {
                    revel_edit::Owner::Component(cid) => list_prop_source(d, cid, &field),
                    _ => None,
                },
            }
        })
        .and_then(|(col, name)| {
            collections(ed).with_untracked(|cs| {
                let f = cs
                    .iter()
                    .find(|c| c.id == col)?
                    .fields
                    .iter()
                    .find(|f| f.name == name)?
                    .clone();
                Some(
                    f.fields
                        .iter()
                        .filter(|x| !x.list)
                        .map(|x| {
                            (
                                x.name.clone(),
                                x.label.clone().unwrap_or_else(|| x.name.clone()),
                                matches!(
                                    x.kind.as_deref(),
                                    Some("rich-text" | "rich_text" | "markdown")
                                ),
                            )
                        })
                        .collect(),
                )
            })
        })
        .unwrap_or_default();
    // Valeur reliée : la valeur, ou un sous-champ (`a.title`, `a.body | safe`).
    let cur: Option<String> = match children.as_slice() {
        [Node::Out { expr }] => {
            let e = expr.trim().trim_end_matches("| safe").trim();
            if e == var {
                Some(String::new())
            } else {
                e.strip_prefix(&format!("{var}.")).map(str::to_string)
            }
        }
        [] | [Node::Text { .. }] => None,
        _ => return None,
    };
    let bound = cur.as_deref() == Some("");
    let at = StoredValue::new(at.clone());
    let rich = StoredValue::new(subs.clone());
    let set = move |choice: String| {
        let at = at.get_value();
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let Ok(Node::Element { id, tag, attrs, .. }) = revel_edit::address::node(d, &at) else {
                return None;
            };
            let (parent, index) = revel_edit::address::locate(d, &at).ok()?;
            let kid = match choice.as_str() {
                "" => Node::Text {
                    text: ui.now("block-bound-text", &[]),
                },
                "value" => Node::Out { expr: var.clone() },
                sub => {
                    // Texte riche d'un objet : écrit tel quel (HTML).
                    let long = rich.with_value(|r| r.iter().any(|(n, _, rich)| n == sub && *rich));
                    Node::Out {
                        expr: if long {
                            format!("{var}.{sub} | safe")
                        } else {
                            format!("{var}.{sub}")
                        },
                    }
                }
            };
            Some(Command::Replace {
                parent,
                index,
                node: Node::Element {
                    id: id.clone(),
                    tag: tag.clone(),
                    attrs: attrs.clone(),
                    children: vec![kid],
                },
            })
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    Some(
        view! {
            <div class="insp-group">
                <p class="panel-title">{ui.t("block-bound")}</p>
                <label class="field" data-bound="block-bound-text-label">
                    <span class="label">{ui.t("block-bound-text-label")}</span>
                    <div class="select">
                        <select class="input" on:change=move |e| set(event_target_value(&e))>
                            <option value="" prop:selected=cur.is_none()>{ui.t("block-bound-written")}</option>
                            {subs.is_empty().then(|| view! {
                                <option value="value" prop:selected=bound>{ui.t("block-bound-value")}</option>
                            })}
                            {subs.iter().map(|(n, l, _)| {
                                let on = cur.as_deref() == Some(n.as_str());
                                view! { <option value=n.clone() prop:selected=on>{l.clone()}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                </label>
            </div>
        }
        .into_any(),
    )
}

/// Champs qu'une condition peut tester à l'endroit de `at` : valeur d'une liste tirée d'un
/// champ (et ses sous-champs), champs du contenu (liste, modèle de collection), propriétés du
/// composant ouvert. (expression, libellé).
/// `at` est dans une boucle (liste de collection, de valeurs…) du même propriétaire.
/// Boucles sur une collection qui entourent l'endroit (marque d'un projet :
/// `{% for m in collections.marques | by_slugs(…) %}`) : (variable, collection) (D-179).
fn enclosing_collection_loops(d: &Site, at: &Target) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut cur = at.clone();
    for _ in 0..64 {
        let Ok((parent, _)) = revel_edit::address::locate(d, &cur) else {
            break;
        };
        let Some(node) = parent.node else {
            break;
        };
        cur = Target {
            owner: at.owner.clone(),
            node,
        };
        if let Ok(Node::Each { expr, item, .. }) = revel_edit::address::node(d, &cur)
            && let Some(rest) = expr.trim().strip_prefix("collections.")
        {
            let col: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            let item = item.trim();
            if !col.is_empty() && revel_model::valid_prop_name(item) {
                out.push((item.to_string(), col));
            }
        }
    }
    out
}

fn in_loop(d: &Site, at: &Target) -> bool {
    let mut cur = at.clone();
    for _ in 0..64 {
        let Ok((parent, _)) = revel_edit::address::locate(d, &cur) else {
            return false;
        };
        let Some(node) = parent.node else {
            return false;
        };
        cur = Target {
            owner: at.owner.clone(),
            node,
        };
        if matches!(revel_edit::address::node(d, &cur), Ok(Node::Each { .. })) {
            return true;
        }
    }
    false
}

pub fn condition_fields(ed: Ed, at: &Target) -> Option<Signal<Vec<(String, String)>>> {
    condition_fields_with(ed, at, collections(ed))
}

/// Comme [`condition_fields`], avec des collections déjà chargées : un appelant qui lit aussi
/// les collections partage le même chargement (deux chargements finissent dans n'importe quel
/// ordre, et la liste montrait un temps les seules valeurs du second).
pub fn condition_fields_with(
    ed: Ed,
    at: &Target,
    cols: RwSignal<Vec<CmsCollection>>,
) -> Option<Signal<Vec<(String, String)>>> {
    // Endroit : liste de valeurs (variable, collection, champ), contenu relié, propriétés.
    let (values, content, props, loops) = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let values = enclosing_values_at(d, at).map(|(var, field, each)| {
            let col = binding_context(d, &each)
                .map(|(col, prefix, _)| {
                    let name = field
                        .strip_prefix(prefix.as_str())
                        .unwrap_or(&field)
                        .to_string();
                    (col, name)
                })
                .or_else(|| match &at.owner {
                    revel_edit::Owner::Component(cid) => list_prop_source(d, cid, &field),
                    _ => None,
                });
            (var, col)
        });
        let content = binding_context(d, at);
        let mut props: Vec<(String, String)> = match &at.owner {
            revel_edit::Owner::Component(cid) => d
                .components
                .iter()
                .find(|c| c.id == *cid)
                .map(|c| {
                    c.props
                        .iter()
                        .map(|p| {
                            (
                                p.name.clone(),
                                p.label.clone().unwrap_or_else(|| p.name.clone()),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            // Gabarit : ses propriétés (valeurs choisies par chaque page, D-96).
            revel_edit::Owner::Layout(lid) => revel_edit::editable::layout_fields(d, Some(lid))
                .iter()
                .map(|p| (p.name.clone(), p.label().to_string()))
                .collect(),
            _ => Vec::new(),
        };
        // Informations du site (« Informations du site › Fax »).
        props.extend(crate::siteinfo::condition_fields(ed));
        let loops = enclosing_collection_loops(d, at);
        // Dans une liste : premier ou dernier élément (`loop.first`, `loop.last`).
        if in_loop(d, at) {
            let ui = expect_context::<Ui>();
            props.push(("loop.first".into(), ui.now("condition-field-first", &[])));
            props.push(("loop.last".into(), ui.now("condition-field-last", &[])));
        }
        // Variables des environnements de publication (« recette » : `env.staging`).
        props.extend(
            revel_edit::editable::env_fields(d)
                .into_iter()
                .map(|v| (format!("env.{v}"), format!("Environnement › {v}"))),
        );
        Some((values, content, props, loops))
    })?;
    if values.is_none() && content.is_none() && props.is_empty() && loops.is_empty() {
        return None;
    }
    let label = |f: &revel_protocol::CmsField| f.label.clone().unwrap_or_else(|| f.name.clone());
    let url_label = expect_context::<Ui>().now("condition-field-page", &[]);
    Some(Signal::derive(move || {
        let mut out: Vec<(String, String)> = Vec::new();
        // Endroit lié à une collection : rien tant que les collections ne sont pas chargées, pour
        // que ses champs viennent toujours en premier.
        if (content.is_some()
            || !loops.is_empty()
            || values.as_ref().is_some_and(|(_, c)| c.is_some()))
            && cols.with(Vec::is_empty)
        {
            return out;
        }
        cols.with(|cs| {
            if let Some((var, col)) = &values {
                out.push((var.clone(), var.clone()));
                if let Some((col, name)) = col
                    && let Some(f) = cs
                        .iter()
                        .find(|c| c.id == *col)
                        .and_then(|c| c.fields.iter().find(|f| f.name == *name))
                {
                    for sub in &f.fields {
                        out.push((
                            format!("{var}.{}", sub.name),
                            format!("{var} › {}", label(sub)),
                        ));
                    }
                }
            }
            if let Some((col, prefix, url)) = &content
                && let Some(c) = cs.iter().find(|c| c.id == *col)
            {
                for f in &c.fields {
                    out.push((format!("{prefix}{}", f.name), label(f)));
                }
                // Le contenu a une page (« a une page » : rempli).
                out.push((url.clone(), url_label.clone()));
            }
            // Contenus d'une autre collection lus en boucle (marque d'un projet) : leurs champs,
            // « Marques › Logo ».
            for (var, col) in &loops {
                if let Some(c) = cs.iter().find(|c| c.id == *col) {
                    let name = c
                        .label
                        .clone()
                        .unwrap_or_else(|| crate::site::capitalize(&c.id));
                    for f in &c.fields {
                        out.push((
                            format!("{var}.data.{}", f.name),
                            format!("{name} › {}", label(f)),
                        ));
                    }
                }
            }
        });
        out.extend(props.iter().cloned());
        out
    }))
}

/// Condition d'un bloc « Si » : clauses sur les champs de l'endroit (rempli, vide, égal à…) ;
/// l'expression reste modifiable dessous.
pub fn readable_condition(ui: Ui, ed: Ed, at: Target, cond: String) -> Option<AnyView> {
    let fields = condition_fields(ed, &at)?;
    let at = StoredValue::new(at);
    let set = move |c: Option<String>| {
        if let Some(c) = c {
            ed.apply(
                ui,
                Command::SetLogic {
                    at: at.get_value(),
                    cond: Some(c),
                    expr: None,
                    item: None,
                },
                false,
            );
        }
    };
    Some(
        view! {
            <div class="field" data-bound="block-bound-visibility">
                <span class="label">{ui.t("block-bound-visibility")}</span>
                {crate::visibility::condition_editor(ui, fields, Some(cond), false, set)}
            </div>
        }
        .into_any(),
    )
}
