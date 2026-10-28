//! Panneau « Ajouter » (étapes 2.5, 2.12) : éléments de base, par famille, à glisser sur la page
//! (emplacement montré par le canevas) ou à insérer d'un clic dans l'élément sélectionné (s'il peut
//! contenir des enfants) ou juste après lui. Les éléments prévus sont visibles, désactivés.
//!
//! Résolution des dépôts du canevas (`DropTarget`) en emplacement du document, pour une insertion
//! ou un déplacement.

use crate::editor::{DragItem, DropTarget, Ed, Place, owner_of};
use crate::{Ui, icons};
use leptos::ev;
use leptos::prelude::*;
use revel_edit::make::Preset;
use revel_edit::{Branch, Command, Owner, Parent, Target};
use revel_model::Node;
use revel_model::ids::OwnerKind;
use wasm_bindgen::JsCast;

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Identifiant de la page fixe affichée.
fn page_id(ed: Ed) -> Option<String> {
    let url = ed.page.get_untracked();
    ed.pages.with_untracked(|ps| {
        ps.iter()
            .find(|p| p.url == url)
            .and_then(|p| p.page.clone())
    })
}

/// Emplacement d'insertion : dans l'élément sélectionné, après lui, ou à la fin de la page.
pub fn insertion_point(ed: Ed) -> Option<(Parent, usize)> {
    let target = ed.selected_target();
    let page = page_id(ed);
    ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        match target {
            Some(t) => {
                let node = revel_edit::address::node(d, &t).ok()?;
                let (p, i) = match node {
                    // Paragraphe ou titre : ne contient que du texte (un bloc dedans serait
                    // sorti par le navigateur) ; l'ajout vient après.
                    Node::Element { tag, children, .. }
                        if !VOID.contains(&tag.to_ascii_lowercase().as_str())
                            && !matches!(
                                tag.to_ascii_lowercase().as_str(),
                                "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                            ) =>
                    {
                        (Parent::of(&t, Branch::Children), children.len())
                    }
                    // Utilisation au contenu propre (emplacement rempli, D-118) : à la fin de ce
                    // contenu.
                    Node::Use { children, .. } if !children.is_empty() => {
                        (Parent::of(&t, Branch::Children), children.len())
                    }
                    _ => {
                        let (p, i) = revel_edit::address::locate(d, &t).ok()?;
                        (p, i + 1)
                    }
                };
                Some(into_page(d, page.as_deref(), p, i))
            }
            None => {
                // Sans sélection : à la fin du corps de la page affichée (page fixe), ou du
                // modèle d'une page de contenu (page de collection).
                if let Some(id) = page {
                    let body = d.pages.iter().find(|p| p.id == id)?.body.len();
                    return Some((Parent::root(Owner::Page(id)), body));
                }
                let url = ed.page.get_untracked();
                let layout = ed
                    .page_ref_untracked(&url)
                    .filter(|p| p.collection.is_some())
                    .and_then(|p| p.layout)?;
                let body = d.layout(&layout)?.body.len();
                Some((Parent::root(Owner::Layout(layout)), body))
            }
        }
    })
}

/// Emplacement d'insertion corrigé pour qu'un ajout arrive là où on l'attend :
/// - autour de l'emplacement du contenu d'un gabarit (dans la liste qui le contient, ou dans
///   `html`, `body`, `main`… qui le contiennent) : dans la page affichée (au début avant
///   l'emplacement, à la fin après), pas dans le gabarit partagé par toutes les pages ;
/// - dans l'en-tête du document (`head` et ses éléments, invisibles) : à la fin de la page (ou du
///   `body` d'une page sans gabarit).
fn into_page(d: &revel_model::Site, page: Option<&str>, p: Parent, i: usize) -> (Parent, usize) {
    let page_end = |page: &str| {
        d.pages
            .iter()
            .find(|x| x.id == page)
            .map(|x| (Parent::root(Owner::Page(page.to_string())), x.body.len()))
    };
    let Ok(list) = revel_edit::address::list(d, &p) else {
        return (p, i);
    };
    if let (Owner::Layout(_), Some(page)) = (&p.owner, page)
        && let Some(slot) = list.iter().position(has_slot)
        && let Some((to, end)) = page_end(page)
    {
        return (to, if i <= slot { 0 } else { end });
    }
    let root = revel_edit::address::list(d, &Parent::root(p.owner.clone()));
    if let (Some(id), Ok(root)) = (&p.node, root)
        && in_head(root, id, false) == Some(true)
    {
        if let (Owner::Layout(_), Some(page)) = (&p.owner, page)
            && let Some(end) = page_end(page)
        {
            return end;
        }
        if let Some((body, n)) = find_body(root) {
            return (
                Parent {
                    owner: p.owner.clone(),
                    node: Some(body),
                    branch: Branch::Children,
                },
                n,
            );
        }
    }
    (p, i)
}

/// Le nœud est ou contient l'emplacement du contenu.
fn has_slot(n: &Node) -> bool {
    matches!(n, Node::Slot)
        || revel_model::ids::child_lists(n)
            .into_iter()
            .any(|l| l.iter().any(has_slot))
}

/// Le nœud `id` est dans l'en-tête du document (`head`) : `None` s'il n'est pas dans la liste.
fn in_head(nodes: &[Node], id: &str, head: bool) -> Option<bool> {
    for n in nodes {
        let h = head || matches!(n, Node::Element { tag, .. } if tag.eq_ignore_ascii_case("head"));
        if revel_model::ids::node_id(n) == Some(id) {
            return Some(h);
        }
        for l in revel_model::ids::child_lists(n) {
            if let Some(r) = in_head(l, id, h) {
                return Some(r);
            }
        }
    }
    None
}

/// Élément `body` (identifiant, nombre d'enfants).
fn find_body(nodes: &[Node]) -> Option<(String, usize)> {
    nodes.iter().find_map(|n| match n {
        Node::Element {
            tag, children, id, ..
        } if tag.eq_ignore_ascii_case("body") => id.clone().map(|i| (i, children.len())),
        _ => revel_model::ids::child_lists(n)
            .into_iter()
            .find_map(|l| find_body(l)),
    })
}

/// Adresse d'un nœud (son propriétaire retrouvé dans le document).
fn target_of(d: &revel_model::Site, id: &str) -> Option<Target> {
    let (kind, owner) = owner_of(d, id)?;
    Some(Target {
        owner: match kind {
            OwnerKind::Page => Owner::Page(owner),
            OwnerKind::Layout => Owner::Layout(owner),
            OwnerKind::Component => Owner::Component(owner),
        },
        node: id.to_string(),
    })
}

/// Emplacement (liste, position) d'un dépôt du canevas.
pub fn drop_point(ed: Ed, t: &DropTarget) -> Option<(Parent, usize)> {
    let page = page_id(ed);
    ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let target = target_of(d, &t.at)?;
        let (p, i) = match t.place {
            Place::Before | Place::After => {
                let (p, i) = revel_edit::address::locate(d, &target).ok()?;
                (p, if t.place == Place::After { i + 1 } else { i })
            }
            Place::Inside => match revel_edit::address::node(d, &target).ok()? {
                Node::Element { children, .. } => {
                    (Parent::of(&target, Branch::Children), children.len())
                }
                // Utilisation d'un composant à emplacement : son contenu propre (D-118).
                Node::Use {
                    component,
                    children,
                    ..
                } if d
                    .component(component)
                    .is_some_and(|c| revel_edit::make::slot_of(c).is_some()) =>
                {
                    (Parent::of(&target, Branch::Children), children.len())
                }
                _ => return None,
            },
        };
        Some(into_page(d, page.as_deref(), p, i))
    })
}

/// Commande d'un élément de la page glissé ailleurs (ou copié avec Alt). Pas de commande si
/// l'emplacement est le sien, s'il est dans l'élément lui-même ou chez un autre propriétaire
/// (page, gabarit, composant) : comme dans les calques.
pub fn move_command(ed: Ed, id: &str, t: &DropTarget, copy: bool) -> Option<Command> {
    let (to, mut index) = drop_point(ed, t)?;
    ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let at = target_of(d, id)?;
        let node = revel_edit::address::node(d, &at).ok()?.clone();
        // Pas dans lui-même ni dans un de ses descendants.
        if let Some(dest) = &to.node {
            let mut inside = false;
            revel_model::ids::walk(std::slice::from_ref(&node), &mut |n| {
                if revel_model::ids::node_id(n) == Some(dest.as_str()) {
                    inside = true;
                }
            });
            if inside {
                return None;
            }
        }
        if copy {
            let mut n = node;
            let mut taken = d.node_ids();
            revel_edit::make::refresh_ids(&mut n, &mut taken);
            return Some(Command::Insert {
                parent: to,
                index,
                node: n,
            });
        }
        if to.owner != at.owner {
            return None;
        }
        let (from, from_index) = revel_edit::address::locate(d, &at).ok()?;
        if to == from && from_index < index {
            index -= 1;
        }
        if to == from && index == from_index {
            return None;
        }
        Some(Command::Move { at, to, index })
    })
}

/// Élément proposé : préréglage (`None` : prévu), libellé, aide, texte par défaut (clés de
/// traduction), icône, balise produite et nature (bloc), étape prévue.
struct Item {
    preset: Option<Preset>,
    key: &'static str,
    help: &'static str,
    text: &'static str,
    icon: fn() -> AnyView,
    tag: &'static str,
    block: bool,
    step: u8,
}

const fn item(
    preset: Preset,
    key: &'static str,
    help: &'static str,
    text: &'static str,
    icon: fn() -> AnyView,
    tag: &'static str,
    block: bool,
) -> Item {
    Item {
        preset: Some(preset),
        key,
        help,
        text,
        icon,
        tag,
        block,
        step: 0,
    }
}

// Éléments prévus (visibles, désactivés, avec l'étape qui les apportera) : aucun pour l'instant.
#[allow(dead_code)]
const fn planned(key: &'static str, icon: fn() -> AnyView, step: u8) -> Item {
    Item {
        preset: None,
        key,
        help: "",
        text: "",
        icon,
        tag: "div",
        block: true,
        step,
    }
}

/// Familles d'éléments (comme le panneau « Add » de Webflow).
fn groups() -> Vec<(&'static str, Vec<Item>)> {
    vec![
        (
            "insert-group-structure",
            vec![
                item(
                    Preset::Section,
                    "insert-section",
                    "insert-help-section",
                    "",
                    icons::slot,
                    "section",
                    true,
                ),
                item(
                    Preset::Container,
                    "insert-container",
                    "insert-help-container",
                    "",
                    icons::box_,
                    "div",
                    true,
                ),
                item(
                    Preset::Grid,
                    "insert-grid",
                    "insert-help-grid",
                    "",
                    icons::grid,
                    "div",
                    true,
                ),
                item(
                    Preset::Columns,
                    "insert-columns",
                    "insert-help-columns",
                    "",
                    icons::layout,
                    "div",
                    true,
                ),
            ],
        ),
        // Éléments interactifs (Navbar, Slider, Tabs de Webflow ; Ticker, Carousel, Slideshow de
        // Framer) : leurs parties sont des calques ordinaires, leurs réglages en tête de Styles.
        (
            "insert-group-interactive",
            vec![
                item(
                    Preset::Navbar,
                    "insert-navbar",
                    "insert-help-navbar",
                    "insert-text-navbar",
                    icons::navbar,
                    "nav",
                    true,
                ),
                item(
                    Preset::Ticker,
                    "insert-ticker",
                    "insert-help-ticker",
                    "insert-text-ticker",
                    icons::ticker,
                    "div",
                    true,
                ),
                item(
                    Preset::Carousel,
                    "insert-carousel",
                    "insert-help-carousel",
                    "insert-text-slider",
                    icons::carousel,
                    "div",
                    true,
                ),
                item(
                    Preset::Slideshow,
                    "insert-slideshow",
                    "insert-help-slideshow",
                    "insert-text-slider",
                    icons::slideshow,
                    "div",
                    true,
                ),
                item(
                    Preset::Consent,
                    "insert-consent",
                    "insert-help-consent",
                    "insert-text-consent",
                    icons::info,
                    "div",
                    true,
                ),
                item(
                    Preset::Switch,
                    "insert-switch",
                    "insert-help-switch",
                    "insert-text-switch",
                    icons::toggle,
                    "button",
                    true,
                ),
                item(
                    Preset::ThemeSwitch,
                    "insert-theme-switch",
                    "insert-help-theme-switch",
                    "insert-text-theme-switch",
                    icons::moon,
                    "button",
                    true,
                ),
                item(
                    Preset::ListFilters,
                    "insert-filters",
                    "insert-help-filters",
                    "insert-text-filters",
                    icons::filter,
                    "div",
                    true,
                ),
                item(
                    Preset::Tabs,
                    "insert-tabs",
                    "insert-help-tabs",
                    "insert-text-tabs",
                    icons::tabs,
                    "div",
                    true,
                ),
                item(
                    Preset::Accordion,
                    "insert-accordion",
                    "insert-help-accordion",
                    "insert-text-accordion",
                    icons::chevron_down,
                    "details",
                    true,
                ),
                item(
                    Preset::ScrollProgress,
                    "insert-progress",
                    "insert-help-progress",
                    "",
                    icons::progress,
                    "div",
                    false,
                ),
                item(
                    Preset::Clock,
                    "insert-clock",
                    "insert-help-clock",
                    "",
                    icons::clock,
                    "p",
                    false,
                ),
                item(
                    Preset::Pixels,
                    "insert-pixels",
                    "insert-help-pixels",
                    "",
                    icons::pixels,
                    "div",
                    false,
                ),
            ],
        ),
        (
            "insert-group-text",
            vec![
                item(
                    Preset::Heading,
                    "insert-heading",
                    "insert-help-heading",
                    "insert-text-heading",
                    icons::text,
                    "h2",
                    true,
                ),
                item(
                    Preset::Paragraph,
                    "insert-paragraph",
                    "insert-help-paragraph",
                    "insert-text-paragraph",
                    icons::text_left,
                    "p",
                    true,
                ),
                item(
                    Preset::Link,
                    "insert-link",
                    "insert-help-link",
                    "insert-text-link",
                    icons::link,
                    "a",
                    false,
                ),
                item(
                    Preset::Button,
                    "insert-button",
                    "insert-help-button",
                    "insert-text-button",
                    icons::wrap,
                    "a",
                    false,
                ),
                item(
                    Preset::List,
                    "insert-list",
                    "insert-help-list",
                    "insert-text-list",
                    icons::list,
                    "ul",
                    true,
                ),
                item(
                    Preset::RichText,
                    "insert-rich-text",
                    "insert-help-rich-text",
                    "insert-text-rich",
                    icons::text,
                    "div",
                    true,
                ),
            ],
        ),
        (
            "insert-group-media",
            vec![
                item(
                    Preset::Image,
                    "insert-image",
                    "insert-help-image",
                    "insert-text-image",
                    icons::image,
                    "img",
                    false,
                ),
                item(
                    Preset::Video,
                    "insert-video",
                    "insert-help-video",
                    "",
                    icons::play,
                    "video",
                    true,
                ),
                item(
                    Preset::VideoPlayer,
                    "insert-video-player",
                    "insert-help-video-player",
                    "insert-text-video-player",
                    icons::play,
                    "figure",
                    true,
                ),
                item(
                    Preset::HoverMedia,
                    "insert-hover-media",
                    "insert-help-hover-media",
                    "",
                    icons::video,
                    "video",
                    false,
                ),
                item(
                    Preset::Reveal,
                    "insert-reveal",
                    "insert-help-reveal",
                    "",
                    icons::reveal,
                    "div",
                    false,
                ),
                item(
                    Preset::Icon,
                    "insert-icon",
                    "insert-help-icon",
                    "",
                    icons::bolt,
                    "svg",
                    false,
                ),
                item(
                    Preset::Lottie,
                    "insert-lottie",
                    "insert-help-lottie",
                    "",
                    icons::sparkle,
                    "div",
                    true,
                ),
            ],
        ),
        (
            "insert-group-blocks",
            vec![
                item(
                    Preset::BlockHero,
                    "insert-block-hero",
                    "insert-help-block",
                    "",
                    icons::layout,
                    "section",
                    true,
                ),
                item(
                    Preset::BlockFeatures,
                    "insert-block-features",
                    "insert-help-block",
                    "",
                    icons::grid,
                    "section",
                    true,
                ),
                item(
                    Preset::BlockTextImage,
                    "insert-block-text-image",
                    "insert-help-block",
                    "",
                    icons::image,
                    "section",
                    true,
                ),
                item(
                    Preset::BlockCta,
                    "insert-block-cta",
                    "insert-help-block",
                    "",
                    icons::bolt,
                    "section",
                    true,
                ),
                item(
                    Preset::BlockTestimonial,
                    "insert-block-testimonial",
                    "insert-help-block",
                    "",
                    icons::text,
                    "section",
                    true,
                ),
                item(
                    Preset::BlockFaq,
                    "insert-block-faq",
                    "insert-help-block",
                    "",
                    icons::list,
                    "section",
                    true,
                ),
                item(
                    Preset::BlockContact,
                    "insert-block-contact",
                    "insert-help-block",
                    "",
                    icons::globe,
                    "section",
                    true,
                ),
            ],
        ),
        (
            "insert-group-cms",
            vec![
                item(
                    Preset::CollectionList,
                    "insert-collection-list",
                    "insert-help-collection-list",
                    "insert-text-collection",
                    icons::database,
                    "div",
                    true,
                ),
                item(
                    Preset::LocaleSwitch,
                    "insert-locale-switch",
                    "insert-help-locale-switch",
                    "insert-text-locale-switch",
                    icons::globe,
                    "nav",
                    true,
                ),
            ],
        ),
        (
            "insert-group-forms",
            vec![
                item(
                    Preset::BlockForm,
                    "insert-form",
                    "insert-help-form",
                    "",
                    icons::page,
                    "section",
                    true,
                ),
                item(
                    Preset::BlockCart,
                    "insert-cart",
                    "insert-help-cart",
                    "",
                    icons::cart,
                    "section",
                    true,
                ),
                item(
                    Preset::CartAdd,
                    "insert-cart-add",
                    "insert-help-cart-add",
                    "insert-text-cart-add",
                    icons::cart,
                    "button",
                    false,
                ),
            ],
        ),
        (
            "insert-group-code",
            vec![item(
                Preset::Embed,
                "insert-embed",
                "insert-help-embed",
                "insert-text-embed",
                icons::code,
                "div",
                true,
            )],
        ),
    ]
}

/// Élément à ajouter proposé par la palette de commandes : famille, libellé, texte par défaut,
/// icône, préréglage.
pub type PaletteItem = (
    &'static str,
    &'static str,
    &'static str,
    fn() -> AnyView,
    Preset,
);

/// Éléments à ajouter proposés par la palette de commandes (éléments prévus exclus).
pub fn palette_items() -> Vec<PaletteItem> {
    groups()
        .into_iter()
        .flat_map(|(g, items)| {
            items
                .into_iter()
                .filter_map(move |it| Some((g, it.key, it.text, it.icon, it.preset?)))
        })
        .collect()
}

/// Ajoute un élément dans l'élément sélectionné (ou juste après lui), sinon à la fin de la page.
pub fn add_preset(ui: Ui, ed: Ed, preset: Preset, text_key: &'static str) {
    let Some((parent, index)) = insertion_point(ed) else {
        ui.fail(revel_protocol::Failure::new("insert-no-place"));
        return;
    };
    insert_at(ui, ed, preset, text_key, parent, index);
}

#[component]
pub fn Insert() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let add = move |preset: Preset, text_key: &'static str| {
        let Some((parent, index)) = insertion_point(ed) else {
            ui.fail(revel_protocol::Failure::new("insert-no-place"));
            return;
        };
        insert_at(ui, ed, preset, text_key, parent, index);
    };

    let drag = CanvasDrag::new(ui, ed);
    let on_down = move |e: ev::PointerEvent| drag.down(&e);
    let on_move = move |e: ev::PointerEvent, it: DragItem| {
        drag.moved(&e, it);
    };
    let finish = move |e: Option<ev::PointerEvent>| match drag.finish(e.as_ref()) {
        Some((it, Some((parent, index)))) => {
            insert_item(ui, ed, &it, parent, index);
            true
        }
        Some((_, None)) => true,
        None => false,
    };
    // Échap : glisser annulé.
    let on_key = window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" && ed.drag.get_untracked().is_some() {
            ed.drop.set(None);
            finish(None);
        }
    });
    on_cleanup(move || on_key.remove());

    view! {
        <div class="insert">
            {groups().into_iter().map(|(title, items)| view! {
                <section class="insert-group">
                    <p class="insert-group-title">{ui.t(title)}</p>
                    <div class="insert-grid">
                        {items.into_iter().map(|it| match it.preset {
                            None => {
                                let step = it.step;
                                let key = it.key;
                                view! {
                                    <button class="insert-item is-planned" disabled=true data-item=key
                                        title=move || format!("{} — {}", ui.tx(key, &[]), ui.tx("planned-step", &[("step", step.to_string())]))>
                                        {(it.icon)()}<span>{ui.t(key)}</span>
                                    </button>
                                }.into_any()
                            }
                            Some(preset) => {
                                let (key, help, text_key, tag, block) = (it.key, it.help, it.text, it.tag, it.block);
                                let drag_item = move || DragItem { preset, text_key, label: ui.now(key, &[]), tag, block, node: None };
                                view! {
                                    <button class="insert-item" data-item=key
                                        title=move || ui.tx(help, &[])
                                        on:pointerdown=on_down
                                        on:pointermove=move |e| on_move(e, drag_item())
                                        on:pointerup=move |e| { finish(Some(e)); }
                                        // Double-clic : ajouté à l'emplacement de la sélection (un clic seul
                                        // n'ajoute rien, comme dans Webflow ; glisser le pose où l'on veut).
                                        on:dblclick=move |_| add(preset, text_key)
                                        on:pointercancel=move |_| { finish(None); }
                                        on:keydown=move |e: ev::KeyboardEvent| if e.key() == "Enter" || e.key() == " " { e.prevent_default(); add(preset, text_key) }>
                                        {(it.icon)()}<span>{ui.t(key)}</span>
                                    </button>
                                }.into_any()
                            }
                        }).collect_view()}
                    </div>
                </section>
            }).collect_view()}
        </div>
    }
}

/// Glisser vers la page depuis un panneau (Ajouter, Médias) : appui, puis déplacement de plus de
/// 4 px. Le pointeur est capturé par l'élément du panneau ; au-dessus du cadre, sa position
/// (pixels de la page) est transmise au canevas, qui montre l'emplacement (`ed.drop`).
#[derive(Clone, Copy)]
pub struct CanvasDrag {
    ed: Ed,
    press: StoredValue<Option<(f64, f64)>>,
}

impl CanvasDrag {
    pub fn new(_ui: Ui, ed: Ed) -> Self {
        Self {
            ed,
            press: StoredValue::new(None),
        }
    }

    /// Point de la page sous le pointeur (coordonnées de la fenêtre), s'il est sur le cadre.
    pub fn frame_point(self, cx: f64, cy: f64) -> Option<(f64, f64)> {
        let f = self.ed.frame_el()?;
        let r = f.get_bounding_client_rect();
        let inside = cx >= r.left() && cx <= r.right() && cy >= r.top() && cy <= r.bottom();
        let s = self.ed.scale();
        inside.then(|| ((cx - r.left()) / s, (cy - r.top()) / s))
    }

    pub fn down(self, e: &ev::PointerEvent) {
        if e.button() != 0 {
            return;
        }
        if let Some(t) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = t.set_pointer_capture(e.pointer_id());
        }
        self.press
            .set_value(Some((f64::from(e.client_x()), f64::from(e.client_y()))));
    }

    /// Déplacement : glisser commencé après 4 px ; `true` pendant un glisser.
    pub fn moved(self, e: &ev::PointerEvent, it: DragItem) -> bool {
        let ed = self.ed;
        let Some((sx, sy)) = self.press.get_value() else {
            return false;
        };
        let (cx, cy) = (f64::from(e.client_x()), f64::from(e.client_y()));
        if ed.drag.get_untracked().is_none() {
            if (cx - sx).hypot(cy - sy) < 4.0 {
                return false;
            }
            ed.drop.set(None);
            ed.post(serde_json::json!({ "type": "drag", "phase": "start", "tag": it.tag, "block": it.block }));
            ed.drag.set(Some(it));
        }
        ed.drag_pos.set((cx, cy));
        match self.frame_point(cx, cy) {
            Some((x, y)) => {
                ed.post(serde_json::json!({ "type": "drag", "phase": "move", "x": x, "y": y }))
            }
            None => {
                // Hors du cadre : plus d'emplacement.
                if ed.drop.get_untracked().is_some() {
                    ed.post(serde_json::json!({ "type": "drag", "phase": "out" }));
                    ed.drop.set(None);
                }
            }
        }
        true
    }

    /// Fin : `None` sans glisser (simple clic) ; sinon l'élément glissé et l'emplacement dans la
    /// page s'il a été relâché sur le cadre.
    pub fn finish(
        self,
        e: Option<&ev::PointerEvent>,
    ) -> Option<(DragItem, Option<(Parent, usize)>)> {
        let ed = self.ed;
        self.press.set_value(None);
        let it = ed.drag.get_untracked()?;
        ed.drag.set(None);
        ed.post(serde_json::json!({ "type": "drag", "phase": "end" }));
        let over = e.is_some_and(|e| {
            self.frame_point(f64::from(e.client_x()), f64::from(e.client_y()))
                .is_some()
        });
        let at = if over {
            ed.drop.get_untracked().and_then(|t| drop_point(ed, &t))
        } else {
            None
        };
        ed.drop.set(None);
        Some((it, at))
    }
}

/// Insère l'élément glissé (prédéfini ou nœud fourni) à un emplacement.
pub fn insert_item(ui: Ui, ed: Ed, it: &DragItem, parent: Parent, index: usize) {
    match &it.node {
        Some(n) => {
            let mut node = n.clone();
            let mut taken = ed
                .doc
                .with_untracked(|d| d.as_ref().map(|d| d.node_ids()))
                .unwrap_or_default();
            revel_edit::make::refresh_ids(&mut node, &mut taken);
            ed.apply(
                ui,
                Command::Insert {
                    parent,
                    index,
                    node,
                },
                false,
            );
        }
        None => insert_at(ui, ed, it.preset, it.text_key, parent, index),
    }
}

/// Insère un élément du panneau à un emplacement.
fn insert_at(ui: Ui, ed: Ed, preset: Preset, text_key: &str, parent: Parent, index: usize) {
    let text = if text_key.is_empty() {
        String::new()
    } else {
        ui.now(text_key, &[])
    };
    ed.apply(
        ui,
        Command::InsertPreset {
            parent,
            index,
            preset,
            text,
        },
        false,
    );
}
