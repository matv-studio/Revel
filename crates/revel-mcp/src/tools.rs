//! Outils MCP de Revel. Descriptions en anglais (langue la mieux comprise des modèles) ; les
//! textes du site restent dans leur langue. Lecture : site, plan d'une page, nœud, styles,
//! contenus du CMS, page rendue. Écriture : texte, attributs, styles, HTML inséré, nœud retiré,
//! commandes de Revel, champ d'un contenu ; annuler, rétablir. Chaque écriture est une étape
//! d'annulation de Revel.

use crate::{Invoke, failure_text, outline};
use revel_edit::address::{self, Branch, Owner, Parent, Target};
use revel_edit::{Command, EditState};
use revel_model::{Node, Site};
use revel_protocol::{CmsCollection, EditorPageInfo};
use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer};
use rmcp::{ServerHandler, schemars, tool, tool_handler, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

const INSTRUCTIONS: &str = "Revel is a desktop visual website builder. These tools read and edit the website \
currently open in Revel (or the one named by the optional `site` argument, an absolute folder path). Every edit \
is applied with Revel's own commands: it shows up immediately in Revel's editor and the user can undo it \
(`undo`, or Ctrl+Z). Start with `get_site`, then `get_page` (every element with its id #id).\n\n\
BUILD ONLY WHAT THE USER CAN EDIT IN REVEL'S INTERFACE. The user must be able to change everything you make from \
the panels (Layers, Styles, Add, CMS), without code: never write `{% set %}`, computed expressions, filters other \
than those listed here, or site scripts. After your changes call `check_site`: it must list no issue for what you \
built; rebuild anything it lists with the forms below. Content that exists in the CMS (titles, dates, clients, \
covers, videos of projects or articles) is read from it with a collection list and fields, never retyped in the \
layers; elements that belong together (an item and its media) are linked by a setting, never by their position.\n\
- Structure: elements with classes; texts written in the elements (`set_text`); CSS on classes per breakpoint and \
state (`set_styles`, states :hover :active :focus-visible [aria-current=\"page\"] .is-active [aria-pressed=\"false\"], or \
inside an active element `.is-active .x`); text-shadow, transform-origin and clip-path are panel controls. Prefer `insert_preset` (Add panel) \
for sections, grids, buttons, images, video, video player (YouTube/Vimeo/file with cover), icons, navbar, ticker \
(endless scroll of logos/team), carousel, slideshow, tabs, accordion, list filters, collection list, blocks.\n\
- Repeated content (projects, team, logos, articles): a CMS collection and a collection list: `{% for item in \
(collections.<id> | selectattr(\"data.<field>\") | where_contains(\"data.<listfield>\", <value>) | without(page.url) | \
sort(attribute=\"<path>\", reverse=true) | pick_first(<field or [\"slug\",…]>))[:<n>] %}` (each part optional, in this \
order; `collections.<id> | by_slugs([\"a\",\"b\"])` for chosen entries; several collections read one after the other: \
`(collections.<a> + collections.<b>) | by_slugs([…])`; `pick_first(<field>, <n>)` puts the entries named by the field first \
only when they fill the list, otherwise the plain list). Inside: `{{ item.data.<field> }}` alone in an \
element (or with a date format `| date(\"%d.%m.%Y\")`, `| safe` for rich text, `| length` for a count, `| emphasis(\"accent\")` \
for *starred* words), `href=\"{{ item.url }}\"`, images bound to a field. A list field: `{% for v in item.data.<field> %}`, \
a reference list: `{% for r in collections.<id> | by_slugs(item.data.<field>) %}`. In a collection template, \
fields are read without prefix (`{{ title }}`).\n\
- Conditions: `{% if %}` with clauses joined by `and` (or all by `or`): `field`, `not field`, `field == \"v\"`, \
`field != \"v\"`, `\"v\" in field`, `field|length > n`. Classes on a condition: \
`class=\"card{% if item.data.featured %} card--featured{% endif %}\"`; class from a value: `class=\"tag tag--{{ item.data.color }}\"`.\n\
- Values in other attributes (Inspector › Attributes, bound without braces): one value with written text around \
(`aria-label=\"Play {{ item.data.title }}\"`, `href=\"/projets/?expertise={{ slug }}\"`), or written only when filled: \
`aria-label=\"Play the video{% if label %}: {{ label }}{% endif %}\"`; style properties bound to a value: \
`style=\"--dot:{{ item.data.color }};--count:{{ collections.<id>|length }}\"` (custom properties read by the classes' styles).\n\
- Site information (Settings › Site information: e-mail, phone, address, social links, menus) is shared by every page: \
`{{ site.<key> }}`, `href=\"mailto:{{ site.email }}\"`, `href=\"tel:{{ site.phone | tel }}\"`, \
`href=\"https://maps.google.com/?q={{ site.address | map_query }}\"`, menus `{% for lien in site.menus.<name> %}<a \
href=\"{{ lien.url }}\">{{ lien.label }}</a>{% endfor %}`; set them with `apply_commands` \
`{\"cmd\": \"set_global\", \"path\": [\"site\", \"email\"], \"value\": \"…\"}`.\n\
- Server files (`.htaccess`, `robots.txt`) are text pages edited as text in the page settings; they may contain \
`{{ site.url }}`, `{% if env.staging %}…{% endif %}` (publishing environment) and `{% for e in collections.<id> %}…{% endfor %}`.\n\
- Components: repeated blocks become components with properties (text, image, link, visibility, list) that each \
use sets; variants for alternate looks.\n\
- Behaviour without code: `data-revel-on` on the triggering element, `<trigger> <action> <target>` separated by \
`;`: trigger click|hover|scroll:<px>|view|load, action toggle|show|hide|class:<name>|play|dismiss|random:<ms>|spin:<deg>, \
target self|parent|next|body|.<class>|#<id> (e.g. `click class:menu-open body; scroll:40 class:is-solid self`); \
sound: `hover sound:/media/a.wav~50 self` (several files `a.wav|b.wav`: one at random; `~50` volume %), music on a loop: \
`click music:/media/x.mp3|/media/y.mp3 self` (spaces as %20); music/sound switches: `<button data-revel-switch=\"music|sounds\">` \
with a `data-revel-switch-text=\"On|Off\"` child (insert_preset switch). \
Tabs: `data-revel-tabs` (options `hover`, `keys` for arrows/WASD anywhere, `sound=<url>`) with `data-revel-tab` \
triggers and `data-revel-panel` panels (several series of panels each follow the tabs in order; the active tab and \
filter get the class `is-active`); media on hover (a background video showing the hovered item's video, e.g. a \
hero driven by a list of featured projects): `insert_preset` HoverMedia (`<video muted playsinline loop autoplay \
data-revel-hover-media=\"<name>\">`), and on one element of each list item (its link) `data-revel-media-for=\"<name>\"`, \
`data-revel-media=\"{{ item.data.<video field> }}\"`, optional `data-revel-media-poster=\"{{ item.data.<image field> }}\"` \
(chosen elements outside a list: addresses written in these attributes); the first item shows on load, the shown \
item gets `is-active`, the started video `is-ready`; slider: `data-revel-slider` (`autoplay=5 loop`) with `data-revel-track`, `data-revel-prev`, `data-revel-next`, \
`data-revel-count`; video player: `data-revel-video=\"<url>\"` with a `data-revel-play` cover; list filters: \
`data-revel-filters=\"<url param>\"` group of `data-revel-filter=\"<value>\"` buttons, items `data-revel-tags`, \
`data-revel-filter-empty`; cookie consent: `data-revel-consent=\"G-…\"` banner, `data-revel-consent-choice=accept|refuse`, \
`data-revel-consent-open`; forms: `<form data-revel-form=\"contact\">` (sent by e-mail, Settings › Forms). \
Animations: `data-revel-appear|text|hover|press|loop|scroll|active` (e.g. `data-revel-appear=\"up dur=0.8\"`, \
`data-revel-active=\"pop\"` plays when its tab, filter or switch becomes active), \
`data-revel-loop=\"ticker speed=60 pause\"`). Lottie: `data-lottie=\"/media/x.json\"` with `data-lottie-play`.\n\
- Page settings (title, description, share image, body class, theme, share type, publication date) and site settings (site name, \
icons, organisation, RSS feeds, forms) are settings, not markup: Revel writes the `<head>`. Redirections \
(old address, with or without `?parameters`, to a new one) are a site setting: `apply_commands` with \
`{\"cmd\": \"set_redirects\", \"redirects\": [{\"from\": \"/old/\", \"to\": \"/new/\"}]}` (the whole list); Revel writes the \
`.htaccess` rules and a redirect page at the old address. A value that changes from page to page inside a \
layout (header, footer: hide a block, a subtitle) is a layout property: `apply_commands` with \
`{\"cmd\": \"set_layout_fields\", \"id\": \"<layout>\", \"fields\": [{\"name\": \"hideCta\", \"label\": \"Hide the call to action\", \"type\": \"switch\"}]}` \
(types text, image, link, switch), read in the layout like a component property (`{% if not hideCta %}`), each \
page giving its value with `{\"cmd\": \"set_page_prop\", \"page\": \"<page id>\", \"key\": \"hideCta\", \"value\": true}`.\n\
Existing sites may still contain {{ expressions }} from data files: read them with `get_data`/`set_data`; CMS entries \
with `list_collections`, `get_entry`, `set_entry_field`. Keep the site's language and tone.";

/// Assistant et outil d'une modification (dernières actions des assistants).
#[derive(Debug, Clone, serde::Serialize)]
struct Via {
    client: String,
    tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
}

tokio::task_local! {
    /// Appel d'outil en cours.
    static VIA: Via;
}

/// Nom de l'assistant : informations du protocole 2026-07-28 (`_meta`), sinon de l'ouverture.
fn client_of(ctx: &RequestContext<RoleServer>) -> String {
    ctx.meta
        .client_info()
        .or_else(|| ctx.peer.peer_info().map(|p| p.client_info.clone()))
        .map(|i| i.title.filter(|t| !t.trim().is_empty()).unwrap_or(i.name))
        .unwrap_or_default()
}

/// Paramètre commun : site visé.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub struct SiteArg {
    /// Absolute path of the site folder. Omit to use the site open in Revel.
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PageArg {
    /// Page address, e.g. "/" or "/about/".
    pub url: String,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NodeArg {
    /// Element id, as listed by get_page (without the #).
    pub id: String,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StylesArg {
    /// Only rules whose selector contains this text (e.g. ".hero"). Omit for all rules (long).
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetTextArg {
    /// Element id (its text is replaced; the element must contain only text).
    pub id: String,
    /// New text (plain text, no HTML).
    pub text: String,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetAttrsArg {
    /// Element id.
    pub id: String,
    /// Attributes to set; null removes the attribute. Example: {"class": "button button--primary", "href": "/contact/"}.
    pub attributes: BTreeMap<String, Option<String>>,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetStylesArg {
    /// Class selector the styles belong to, e.g. ".hero__title" (or a combination ".button.is-large").
    pub selector: String,
    /// CSS properties to set; null removes the property. Example: {"font-size": "48px", "margin-top": null}.
    pub properties: BTreeMap<String, Option<String>>,
    /// Breakpoint id from get_site (e.g. "max-900"); omit for the base (desktop) styles.
    #[serde(default)]
    pub breakpoint: Option<String>,
    /// State, e.g. ":hover", ".is-active" (active tab, filter or switch), "[aria-pressed=\"false\"]"
    /// (switch off); starting with "^", a state of an enclosing element ("^.is-active": inside the
    /// active tab). Omit for the normal state.
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InsertHtmlArg {
    /// HTML to insert (Nunjucks expressions allowed). Use the site's existing classes when possible.
    pub html: String,
    /// Insert at the end of this element (id).
    #[serde(default)]
    pub inside: Option<String>,
    /// Insert just before this element (id).
    #[serde(default)]
    pub before: Option<String>,
    /// Insert just after this element (id).
    #[serde(default)]
    pub after: Option<String>,
    /// Otherwise: at the end of this page's own content (address, e.g. "/about/").
    #[serde(default)]
    pub page: Option<String>,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InsertPresetArg {
    /// Element of Revel's Add panel: section, container, heading, paragraph, link, button, image,
    /// list, grid, columns, rich_text, video, icon, embed, lottie, tabs, accordion,
    /// collection_list, locale_switch, navbar, ticker, carousel, slideshow, block_hero,
    /// block_features, block_text_image, block_cta, block_testimonial, block_faq, block_contact,
    /// block_form, video_player, list_filters, consent, switch.
    pub preset: String,
    /// Default text of the new element (site's language). navbar: "Brand|Menu|Link 1|Link 2|…";
    /// carousel/slideshow: "Slide|Previous|Next"; switch: "Label|On|Off"; icon: icon name
    /// (fleche-droite, chevron-droite, plus, croix, coche, menu, etoile, coeur…).
    #[serde(default)]
    pub text: Option<String>,
    /// Insert at the end of this element (id).
    #[serde(default)]
    pub inside: Option<String>,
    /// Insert just before this element (id).
    #[serde(default)]
    pub before: Option<String>,
    /// Insert just after this element (id).
    #[serde(default)]
    pub after: Option<String>,
    /// Otherwise: at the end of this page's own content (address, e.g. "/about/").
    #[serde(default)]
    pub page: Option<String>,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LayerArg {
    /// Element id.
    pub id: String,
    /// Layer name shown in Revel's Layers panel (empty: automatic name).
    #[serde(default)]
    pub name: Option<String>,
    /// Hidden layer: not on the published site, hidden on the canvas.
    #[serde(default)]
    pub hidden: Option<bool>,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CommandsArg {
    /// Revel edit commands (JSON objects tagged by "cmd"), applied together as one undo step.
    /// Example: [{"cmd": "set_attr", "at": {"owner": {"kind": "page", "id": "index"}, "node": "abc123"}, "name": "class", "value": "hero"}].
    pub commands: Vec<Value>,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EntryArg {
    /// Collection id (from list_collections).
    pub collection: String,
    /// Entry file name without extension (from list_collections).
    pub file: String,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EntryFieldArg {
    /// Collection id (from list_collections).
    pub collection: String,
    /// Entry file name without extension.
    pub file: String,
    /// Front matter field name (e.g. "title"); use "body" for the Markdown text after the front matter.
    pub field: String,
    /// New value (string, number, boolean, list or object as JSON); null removes the field.
    pub value: Value,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DataArg {
    /// Path in the site's data files, dot-separated, list items by number: "pages.agence.title",
    /// "site.nav.0.label". A shorter path returns the whole object.
    pub path: String,
    #[serde(default)]
    pub site: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetDataArg {
    /// Path of a text value in the site's data files, e.g. "pages.agence.title".
    pub path: String,
    /// New text (HTML allowed where the page shows it with `| safe`).
    pub value: String,
    #[serde(default)]
    pub site: Option<String>,
}

/// Chemin de données découpé (`pages.agence.values.0.title`).
fn data_path(path: &str) -> Vec<String> {
    path.split('.')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Résultat texte.
fn ok(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(text.into())])
}

/// Résultat d'erreur (lisible par le modèle).
fn fail(text: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(text.into())])
}

/// Adresse d'un nœud (son propriétaire est retrouvé dans le document).
fn target_of(d: &Site, id: &str) -> Option<Target> {
    for (kind, owner, body) in d.owners() {
        let mut found = false;
        revel_model::ids::walk(body, &mut |n| {
            found |= revel_model::ids::node_id(n) == Some(id);
        });
        if found {
            let owner = match kind {
                revel_model::ids::OwnerKind::Page => Owner::Page(owner.to_string()),
                revel_model::ids::OwnerKind::Layout => Owner::Layout(owner.to_string()),
                revel_model::ids::OwnerKind::Component => Owner::Component(owner.to_string()),
            };
            return Some(Target {
                owner,
                node: id.to_string(),
            });
        }
    }
    None
}

#[derive(Clone)]
pub struct Revel {
    inv: Arc<dyn Invoke>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl Revel {
    pub fn new(inv: Arc<dyn Invoke>) -> Self {
        Self {
            inv,
            tool_router: Self::tool_router(),
        }
    }

    async fn call(
        &self,
        cmd: &str,
        site: &Option<String>,
        mut args: Value,
    ) -> Result<Value, String> {
        if let Some(o) = args.as_object_mut() {
            if let Some(s) = site {
                o.insert("site".into(), Value::String(s.clone()));
            }
            // Modification : assistant et outil transmis (relevés par l'application).
            if crate::activity::WRITES.contains(&cmd)
                && let Ok(v) = VIA.try_with(Via::clone)
            {
                o.insert("via".into(), json!(v));
            }
        }
        self.inv
            .invoke(cmd, args)
            .await
            .map_err(|f| failure_text(&f))
    }

    async fn doc(&self, site: &Option<String>) -> Result<Site, String> {
        let v = self.call("edit_open", site, json!({})).await?;
        serde_json::from_value(v.get("site").cloned().unwrap_or(Value::Null))
            .map_err(|e| e.to_string())
    }

    async fn pages(&self, site: &Option<String>) -> Result<Vec<EditorPageInfo>, String> {
        let v = self.call("edit_pages", site, json!({})).await?;
        serde_json::from_value(v).map_err(|e| e.to_string())
    }

    /// Applique une commande (une étape d'annulation) et décrit le résultat.
    async fn apply(&self, site: &Option<String>, cmd: Command, what: &str) -> CallToolResult {
        let v = match self
            .call(
                "edit_apply",
                site,
                json!({ "command": cmd, "coalesce": false }),
            )
            .await
        {
            Ok(v) => v,
            Err(e) => return fail(e),
        };
        match serde_json::from_value::<EditState>(v) {
            Ok(st) if st.applied.is_some() => ok(format!(
                "{what}. Visible in Revel now; undo with the `undo` tool (revision {}).",
                st.revision
            )),
            Ok(_) => ok("Nothing changed (the value was already set)."),
            Err(e) => fail(e.to_string()),
        }
    }

    /// Emplacement d'insertion : dans un élément, avant ou après lui, ou à la fin d'une page.
    async fn place(
        &self,
        d: &Site,
        site: &Option<String>,
        inside: &Option<String>,
        before: &Option<String>,
        after: &Option<String>,
        page: &Option<String>,
    ) -> Option<(Parent, usize)> {
        if let Some(id) = inside {
            target_of(d, id).and_then(|t| match address::node(d, &t).ok()? {
                Node::Element { children, .. } => {
                    Some((Parent::of(&t, Branch::Children), children.len()))
                }
                _ => None,
            })
        } else if let Some(id) = before.as_ref().or(after.as_ref()) {
            target_of(d, id).and_then(|t| {
                let (p, i) = address::locate(d, &t).ok()?;
                Some((p, if after.is_some() { i + 1 } else { i }))
            })
        } else if let Some(url) = page {
            let pages = self.pages(site).await.unwrap_or_default();
            pages
                .iter()
                .find(|p| &p.url == url)
                .and_then(|p| p.page.clone())
                .and_then(|id| {
                    let n = d.pages.iter().find(|p| p.id == id)?.body.len();
                    Some((Parent::root(Owner::Page(id)), n))
                })
        } else {
            None
        }
    }

    /// Résultat complété des points que l'interface ne sait pas régler, apparus avec la
    /// modification : l'assistant reconstruit avec les outils de l'interface.
    async fn with_audit(
        &self,
        site: &Option<String>,
        before: Vec<revel_edit::editable::Issue>,
        result: CallToolResult,
    ) -> CallToolResult {
        if result.is_error == Some(true) {
            return result;
        }
        let Ok(d) = self.doc(site).await else {
            return result;
        };
        let new: Vec<_> = revel_edit::editable::audit(&d)
            .into_iter()
            .filter(|i| !before.contains(i))
            .collect();
        if new.is_empty() {
            return result;
        }
        let list = new
            .iter()
            .take(20)
            .map(|i| format!("- {} {:?}: {}", i.owner, i.kind, i.detail))
            .collect::<Vec<_>>()
            .join("\n");
        ok(format!(
            "{}\n\nNot editable in Revel's interface (the user cannot change these without code); rebuild them with interface structures — literal text in elements, component properties, collection lists (`insert_preset` CollectionList), interactive presets, `data-revel-*` attributes:\n{list}",
            text_of(&result)
        ))
    }

    async fn collections(&self, site: &Option<String>) -> Result<Vec<CmsCollection>, String> {
        let v = self.call("cms_collections", site, json!({})).await?;
        serde_json::from_value(v).map_err(|e| e.to_string())
    }
}

/// Texte par défaut d'un élément du panneau Ajouter, dans la langue du site (celui de
/// l'interface).
fn preset_text(p: revel_edit::make::Preset, locale: &str) -> String {
    use revel_edit::make::Preset as P;
    let key = match p {
        P::Heading => "insert-text-heading",
        P::Paragraph => "insert-text-paragraph",
        P::Link => "insert-text-link",
        P::Button => "insert-text-button",
        P::List => "insert-text-list",
        P::RichText => "insert-text-rich",
        P::Image => "insert-text-image",
        P::Tabs => "insert-text-tabs",
        P::Accordion => "insert-text-accordion",
        P::Navbar => "insert-text-navbar",
        P::Ticker => "insert-text-ticker",
        P::Carousel | P::Slideshow => "insert-text-slider",
        P::VideoPlayer => "insert-text-video-player",
        P::ListFilters => "insert-text-filters",
        P::Consent => "insert-text-consent",
        P::Switch => "insert-text-switch",
        _ => return String::new(),
    };
    revel_i18n::Localizer::new(locale).t(key, &[])
}

/// Arguments d'un outil appelé sans MCP.
fn args_of<T: serde::de::DeserializeOwned>(v: Value) -> Result<Parameters<T>, String> {
    serde_json::from_value(v)
        .map(Parameters)
        .map_err(|e| format!("Arguments not understood: {e}"))
}

impl Revel {
    /// Outil appelé sans MCP (interface HTTP décrite en OpenAPI) : mêmes outils, mêmes résultats,
    /// modifications relevées pour `client`. `None` : outil inconnu.
    pub async fn run(&self, name: &str, args: Value, client: &str) -> Option<CallToolResult> {
        let args = if args.is_null() { json!({}) } else { args };
        let via = Via {
            client: client.to_string(),
            tool: name.to_string(),
            target: ["id", "selector", "path", "url", "page", "file"]
                .iter()
                .find_map(|k| args.get(*k).and_then(Value::as_str).map(str::to_string)),
        };
        macro_rules! go {
            ($f:ident) => {
                match args_of(args) {
                    Ok(a) => self.$f(a).await,
                    Err(e) => fail(e),
                }
            };
        }
        let run = async move {
            Some(match name {
                "get_site" => go!(get_site),
                "get_page" => go!(get_page),
                "get_node" => go!(get_node),
                "get_styles" => go!(get_styles),
                "get_page_html" => go!(get_page_html),
                "set_text" => go!(set_text),
                "set_attributes" => go!(set_attributes),
                "set_styles" => go!(set_styles),
                "insert_html" => go!(insert_html),
                "insert_preset" => go!(insert_preset),
                "set_layer" => go!(set_layer),
                "check_site" => go!(check_site),
                "remove_node" => go!(remove_node),
                "apply_commands" => go!(apply_commands),
                "undo" => go!(undo),
                "redo" => go!(redo),
                "get_data" => go!(get_data),
                "set_data" => go!(set_data),
                "list_collections" => go!(list_collections),
                "get_entry" => go!(get_entry),
                "set_entry_field" => go!(set_entry_field),
                _ => return None,
            })
        };
        VIA.scope(via, run).await
    }

    /// Description OpenAPI 3.1 des outils (`POST /api/<outil>`, corps : arguments de l'outil).
    pub fn openapi(server: &str) -> Value {
        let mut paths = serde_json::Map::new();
        for t in Self::tool_router().list_all() {
            let mut schema = Value::Object((*t.input_schema).clone());
            if let Some(o) = schema.as_object_mut() {
                o.remove("$schema");
            }
            let description = t.description.as_deref().unwrap_or_default();
            let summary = description
                .split(". ")
                .next()
                .unwrap_or_default()
                .trim_end_matches('.');
            paths.insert(
                format!("/api/{}", t.name),
                json!({ "post": {
                    "operationId": t.name,
                    "summary": summary,
                    "description": description,
                    "requestBody": { "required": true, "content": { "application/json": { "schema": schema } } },
                    "responses": {
                        "200": { "description": "Result (text for the model)", "content": { "application/json": { "schema": {
                            "type": "object", "properties": { "result": { "type": "string" } }, "required": ["result"] } } } },
                        "422": { "description": "The tool could not do it (readable reason)", "content": { "application/json": { "schema": {
                            "type": "object", "properties": { "error": { "type": "string" } }, "required": ["error"] } } } },
                        "401": { "description": "Missing or wrong access key" }
                    }
                } }),
            );
        }
        json!({
            "openapi": "3.1.0",
            "info": { "title": "Revel", "version": env!("CARGO_PKG_VERSION"), "description": INSTRUCTIONS },
            "servers": [{ "url": server }],
            "components": { "securitySchemes": { "bearer": { "type": "http", "scheme": "bearer" } } },
            "security": [{ "bearer": [] }],
            "paths": paths,
        })
    }
}

/// Texte d'un résultat d'outil.
pub(crate) fn text_of(r: &CallToolResult) -> String {
    r.content
        .iter()
        .filter_map(|c| c.as_text().map(|t| t.text.clone()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[tool_router]
impl Revel {
    #[tool(
        description = "Overview of the website open in Revel: name, folder, address, languages, breakpoints (screen sizes), pages (address, title, id), collections and components. Call this first."
    )]
    async fn get_site(&self, Parameters(a): Parameters<SiteArg>) -> CallToolResult {
        let (d, pages) = match (self.doc(&a.site).await, self.pages(&a.site).await) {
            (Ok(d), Ok(p)) => (d, p),
            (Err(e), _) | (_, Err(e)) => return fail(e),
        };
        let path = self
            .call("site_path", &a.site, json!({}))
            .await
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        let mut out = vec![format!("Site « {} » — folder {path}", d.name)];
        if let Some(u) = &d.base_url {
            out.push(format!("Address: {u}"));
        }
        let mut langs = vec![format!("{} (main)", d.default_locale)];
        langs.extend(d.locales.iter().map(|l| l.code.clone()));
        out.push(format!("Languages: {}", langs.join(", ")));
        out.push(format!(
            "Breakpoints: {}",
            d.breakpoints
                .iter()
                .map(|b| match (b.max_width, b.min_width) {
                    (Some(w), _) => format!("{} (≤ {w}px)", b.id),
                    (_, Some(w)) => format!("{} (≥ {w}px)", b.id),
                    _ => format!("{} (base)", b.id),
                })
                .collect::<Vec<_>>()
                .join(", ")
        ));
        out.push("Pages:".into());
        for p in pages.iter().filter(|p| p.page.is_some()) {
            let name = p
                .name
                .clone()
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| p.title.clone());
            out.push(format!(
                "  {}{}{}{}",
                p.url,
                if name.trim().is_empty() {
                    String::new()
                } else {
                    format!(" — « {} »", name.trim())
                },
                p.page
                    .as_ref()
                    .map(|i| format!(" (page id {i})"))
                    .unwrap_or_default(),
                if p.draft { " [draft]" } else { "" }
            ));
        }
        let entries = pages.iter().filter(|p| p.collection.is_some()).count();
        if entries > 0 {
            out.push(format!(
                "  + {entries} collection entry pages (see list_collections)"
            ));
        }
        if !d.collections.is_empty() {
            out.push(format!(
                "Collections: {}",
                d.collections
                    .iter()
                    .map(|c| c.id.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !d.components.is_empty() {
            out.push(format!(
                "Components: {}",
                d.components
                    .iter()
                    .map(|c| format!("{} ({})", c.label(), c.id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !d.redirects.is_empty() {
            out.push(format!(
                "Redirections: {}",
                d.redirects
                    .iter()
                    .map(|r| format!("{} → {}", r.from, r.to))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        ok(out.join("\n"))
    }

    #[tool(
        description = "Structure of a page: every element (layouts and page content) with its tag, classes, id (#id) and a text excerpt. Ids are needed by the editing tools."
    )]
    async fn get_page(&self, Parameters(a): Parameters<PageArg>) -> CallToolResult {
        let (d, pages) = match (self.doc(&a.site).await, self.pages(&a.site).await) {
            (Ok(d), Ok(p)) => (d, p),
            (Err(e), _) | (_, Err(e)) => return fail(e),
        };
        let Some(info) = pages.iter().find(|p| p.url == a.url) else {
            return fail(format!(
                "No page at {}. Pages: {}",
                a.url,
                pages
                    .iter()
                    .filter(|p| p.page.is_some())
                    .map(|p| p.url.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        };
        ok(outline::page(&d, info))
    }

    #[tool(
        description = "Full JSON of one element (attributes, children) and the template that owns it."
    )]
    async fn get_node(&self, Parameters(a): Parameters<NodeArg>) -> CallToolResult {
        let d = match self.doc(&a.site).await {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        let Some(t) = target_of(&d, &a.id) else {
            return fail(format!("No element #{} (ids come from get_page).", a.id));
        };
        match address::node(&d, &t) {
            Ok(n) => ok(
                serde_json::to_string_pretty(&json!({ "at": t, "node": n })).unwrap_or_default()
            ),
            Err(e) => fail(e.to_string()),
        }
    }

    #[tool(
        description = "CSS rules of the site's stylesheet (selector, media query, declarations), optionally filtered by selector text."
    )]
    async fn get_styles(&self, Parameters(a): Parameters<StylesArg>) -> CallToolResult {
        let d = match self.doc(&a.site).await {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        let q = a.selector.unwrap_or_default();
        let mut out = Vec::new();
        for r in d.style_rules() {
            if !q.is_empty() && !r.selector.contains(&q) {
                continue;
            }
            let decls: Vec<String> = r
                .decls
                .iter()
                .map(|x| {
                    format!(
                        "{}: {}{};",
                        x.prop,
                        x.value,
                        if x.important { " !important" } else { "" }
                    )
                })
                .collect();
            let rule = format!("{} {{ {} }}", r.selector, decls.join(" "));
            out.push(match &r.media {
                Some(m) => format!("@media {m} {{ {rule} }}"),
                None => rule,
            });
            if out.len() >= 400 {
                out.push("… (more rules: filter with `selector`)".into());
                break;
            }
        }
        if out.is_empty() {
            return ok("No matching rule.");
        }
        let bps: Vec<String> = d
            .breakpoints
            .iter()
            .filter_map(|b| b.media().map(|m| format!("{} = {m}", b.id)))
            .collect();
        ok(format!(
            "{}\n\nBreakpoints: {}",
            out.join("\n"),
            bps.join(", ")
        ))
    }

    #[tool(
        description = "HTML of a page as it will be published (rendered by Revel). Long pages are cut."
    )]
    async fn get_page_html(&self, Parameters(a): Parameters<PageArg>) -> CallToolResult {
        match self
            .call("page_html", &a.site, json!({ "url": a.url }))
            .await
        {
            Ok(Value::String(h)) => {
                let max = 60_000;
                if h.len() > max {
                    let mut end = max;
                    while !h.is_char_boundary(end) {
                        end -= 1;
                    }
                    ok(format!(
                        "{}\n<!-- … cut ({} bytes in all) -->",
                        &h[..end],
                        h.len()
                    ))
                } else {
                    ok(h)
                }
            }
            Ok(_) => fail(format!("No page at {}.", a.url)),
            Err(e) => fail(e),
        }
    }

    #[tool(
        description = "Replace the text of an element that contains only text (heading, paragraph, button, link…)."
    )]
    async fn set_text(&self, Parameters(a): Parameters<SetTextArg>) -> CallToolResult {
        let d = match self.doc(&a.site).await {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        let Some(t) = target_of(&d, &a.id) else {
            return fail(format!("No element #{}.", a.id));
        };
        let Ok(Node::Element { children, .. }) = address::node(&d, &t) else {
            return fail(format!("#{} is not an HTML element.", a.id));
        };
        let parent = Parent::of(&t, Branch::Children);
        let texts: Vec<usize> = children
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c, Node::Text { .. }))
            .map(|(i, _)| i)
            .collect();
        let only_text = children.iter().all(|c| match c {
            Node::Text { .. } => true,
            Node::Html { html } => html.trim().is_empty(),
            _ => false,
        });
        let cmd = match (texts.as_slice(), only_text) {
            ([], true) => Command::Insert {
                parent,
                index: 0,
                node: Node::Text { text: a.text },
            },
            ([i], true) => Command::SetText {
                parent,
                index: *i,
                text: a.text,
            },
            _ => {
                return fail(format!(
                    "#{} contains other elements or expressions: set the text of a child element, or replace its content with insert_html / remove_node.",
                    a.id
                ));
            }
        };
        self.apply(&a.site, cmd, &format!("Text of #{} replaced", a.id))
            .await
    }

    #[tool(
        description = "Set or remove attributes of an element (class, href, src, alt, id, aria-*, data-*…)."
    )]
    async fn set_attributes(&self, Parameters(a): Parameters<SetAttrsArg>) -> CallToolResult {
        let d = match self.doc(&a.site).await {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        let Some(t) = target_of(&d, &a.id) else {
            return fail(format!("No element #{}.", a.id));
        };
        let mut commands: Vec<Command> = a
            .attributes
            .into_iter()
            .map(|(name, value)| Command::SetAttr {
                at: t.clone(),
                name,
                value,
            })
            .collect();
        if commands.is_empty() {
            return ok("No attribute given.");
        }
        let cmd = if commands.len() == 1 {
            commands.remove(0)
        } else {
            Command::Batch { commands }
        };
        self.apply(&a.site, cmd, &format!("Attributes of #{} set", a.id))
            .await
    }

    #[tool(
        description = "Set or remove CSS properties of a class, for the base styles or one breakpoint and state. Every element with this class changes (like Webflow classes)."
    )]
    async fn set_styles(&self, Parameters(a): Parameters<SetStylesArg>) -> CallToolResult {
        let d = match self.doc(&a.site).await {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        if d.stylesheet.is_none() {
            return fail("This site has no stylesheet managed by Revel.");
        }
        let media = match &a.breakpoint {
            None => None,
            Some(id) => match d.breakpoints.iter().find(|b| &b.id == id) {
                Some(b) => b.media(),
                None => {
                    return fail(format!(
                        "No breakpoint {id}. Breakpoints: {}",
                        d.breakpoints
                            .iter()
                            .map(|b| b.id.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            },
        };
        let state = a.state.clone().unwrap_or_default();
        let full = match state.strip_prefix('^') {
            Some(outer) => format!("{} {}", outer.trim(), a.selector.trim()),
            None => format!("{}{state}", a.selector.trim()),
        };
        let rule = revel_edit::style::find_rule(&d, &full, &media);
        match revel_edit::style::style_command(
            rule,
            full.clone(),
            media,
            a.properties.into_iter().collect(),
        ) {
            Some(cmd) => {
                self.apply(&a.site, cmd, &format!("Styles of {full} set"))
                    .await
            }
            None => ok("Nothing changed (the values were already set)."),
        }
    }

    #[tool(
        description = "Insert HTML (one or more elements) inside an element, before or after an element, or at the end of a page's content. Returns the new elements' ids."
    )]
    async fn insert_html(&self, Parameters(a): Parameters<InsertHtmlArg>) -> CallToolResult {
        let d = match self.doc(&a.site).await {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        let place = self
            .place(&d, &a.site, &a.inside, &a.before, &a.after, &a.page)
            .await;
        let Some((parent, index)) = place else {
            return fail(
                "Say where to insert: `inside`, `before` or `after` an element id, or `page`.",
            );
        };
        // Page XML (flux RSS) ou texte (`.htaccess`) : lue comme telle, comme à l'import.
        let mode = match &parent.owner {
            Owner::Page(id) => d
                .pages
                .iter()
                .find(|p| &p.id == id)
                .map_or(revel_markup::Mode::Html, |p| page_mode(&p.path)),
            _ => revel_markup::Mode::Html,
        };
        let mut nodes = match revel_markup::parse(
            &a.html,
            &revel_markup::Options {
                dialect: revel_markup::Dialect::Nunjucks,
                mode,
            },
        ) {
            Ok(p) => p.nodes,
            Err(e) => return fail(format!("HTML not understood: {e}")),
        };
        if mode == revel_markup::Mode::Html {
            nodes.retain(|n| !matches!(n, Node::Text { text } if text.trim().is_empty()));
        }
        // Gabarit : `{{ content }}` devient l'emplacement de la page (la page reste dans les
        // calques, comme un gabarit fait dans Revel).
        if matches!(parent.owner, Owner::Layout(_)) {
            revel_model::content_to_slot(&mut nodes);
        }
        if nodes.is_empty() {
            return fail("Nothing to insert.");
        }
        let mut taken = d.node_ids();
        for n in &mut nodes {
            revel_edit::make::refresh_ids(n, &mut taken);
        }
        let ids: Vec<String> = nodes
            .iter()
            .filter_map(|n| revel_model::ids::node_id(n).map(str::to_string))
            .collect();
        let mut commands: Vec<Command> = nodes
            .into_iter()
            .enumerate()
            .map(|(i, node)| Command::Insert {
                parent: parent.clone(),
                index: index + i,
                node,
            })
            .collect();
        let cmd = if commands.len() == 1 {
            commands.remove(0)
        } else {
            Command::Batch { commands }
        };
        let what = format!(
            "Inserted ({})",
            ids.iter()
                .map(|i| format!("#{i}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let before = revel_edit::editable::audit(&d);
        let result = self.apply(&a.site, cmd, &what).await;
        self.with_audit(&a.site, before, result).await
    }

    #[tool(
        description = "Insert an element of Revel's Add panel, exactly as the interface does (with its style rules): structure, text, media, collection list, interactive elements (navbar with mobile menu, ticker, carousel, slideshow, tabs, accordion) and ready-made blocks. Prefer this to insert_html for anything the panel offers."
    )]
    async fn insert_preset(&self, Parameters(a): Parameters<InsertPresetArg>) -> CallToolResult {
        let d = match self.doc(&a.site).await {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        let want: String = a
            .preset
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        let Some(preset) = revel_edit::make::Preset::ALL
            .into_iter()
            .find(|p| format!("{p:?}").to_ascii_lowercase() == want)
        else {
            return fail(format!("Unknown preset `{}`.", a.preset));
        };
        let Some((parent, index)) = self
            .place(&d, &a.site, &a.inside, &a.before, &a.after, &a.page)
            .await
        else {
            return fail(
                "Say where to insert: `inside`, `before` or `after` an element id, or `page`.",
            );
        };
        let text = a
            .text
            .unwrap_or_else(|| preset_text(preset, &d.default_locale));
        let before = revel_edit::editable::audit(&d);
        let result = self
            .apply(
                &a.site,
                Command::InsertPreset {
                    parent,
                    index,
                    preset,
                    text,
                },
                &format!("Inserted {preset:?} (see its ids with get_page)"),
            )
            .await;
        self.with_audit(&a.site, before, result).await
    }

    #[tool(
        description = "Rename a layer (name shown in Revel's Layers panel, never written in the pages) and/or hide it (hidden layers are not published)."
    )]
    async fn set_layer(&self, Parameters(a): Parameters<LayerArg>) -> CallToolResult {
        let d = match self.doc(&a.site).await {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        let Some(at) = target_of(&d, &a.id) else {
            return fail(format!("No element #{}.", a.id));
        };
        let mut commands = Vec::new();
        if let Some(n) = &a.name {
            match revel_edit::layer::rename(&d, &at, n) {
                Ok(c) => commands.extend(c),
                Err(e) => return fail(e.to_string()),
            }
        }
        if let Some(h) = a.hidden {
            match revel_edit::layer::set_hidden(&d, &at, h) {
                Ok(c) => commands.extend(c),
                Err(e) => return fail(e.to_string()),
            }
        }
        if commands.is_empty() {
            return ok("Nothing changed.");
        }
        self.apply(
            &a.site,
            Command::Batch { commands },
            &format!("Layer #{} updated", a.id),
        )
        .await
    }

    #[tool(
        description = "List what Revel's interface cannot edit in the site (variables, computed expressions, written conditions and lists, computed attributes…). Everything a user builds in the interface has zero issues: rebuild each listed place with interface structures (literal text, component properties, collection lists, presets, data-revel-* attributes)."
    )]
    async fn check_site(&self, Parameters(a): Parameters<SiteArg>) -> CallToolResult {
        let d = match self.doc(&a.site).await {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        let issues = revel_edit::editable::audit(&d);
        if issues.is_empty() {
            return ok("Everything in this site is editable in Revel's interface.");
        }
        ok(serde_json::to_string_pretty(&json!({
            "summary": revel_edit::editable::summary(&issues)
                .into_iter()
                .map(|(k, n)| json!({ "kind": k, "count": n }))
                .collect::<Vec<_>>(),
            "issues": issues,
        }))
        .unwrap_or_default())
    }

    #[tool(description = "Remove an element (and everything inside it).")]
    async fn remove_node(&self, Parameters(a): Parameters<NodeArg>) -> CallToolResult {
        let d = match self.doc(&a.site).await {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        let Some(at) = target_of(&d, &a.id) else {
            return fail(format!("No element #{}.", a.id));
        };
        self.apply(
            &a.site,
            Command::Remove { at },
            &format!("#{} removed", a.id),
        )
        .await
    }

    #[tool(
        description = "Advanced: apply Revel edit commands (the same JSON commands the Revel interface uses) as one undo step. Prefer the simpler tools."
    )]
    async fn apply_commands(&self, Parameters(a): Parameters<CommandsArg>) -> CallToolResult {
        let mut commands = Vec::new();
        for (i, c) in a.commands.into_iter().enumerate() {
            match serde_json::from_value::<Command>(c) {
                Ok(c) => commands.push(c),
                Err(e) => return fail(format!("Command {i} not understood: {e}")),
            }
        }
        if commands.is_empty() {
            return ok("No command.");
        }
        let cmd = if commands.len() == 1 {
            commands.remove(0)
        } else {
            Command::Batch { commands }
        };
        self.apply(&a.site, cmd, "Commands applied").await
    }

    #[tool(description = "Undo the last change made in Revel (by the user or an assistant).")]
    async fn undo(&self, Parameters(a): Parameters<SiteArg>) -> CallToolResult {
        match self.call("edit_undo", &a.site, json!({})).await {
            Ok(v) => {
                let st: Option<EditState> = serde_json::from_value(v).ok();
                match st {
                    Some(s) if s.applied.is_some() => {
                        ok(format!("Undone (revision {}).", s.revision))
                    }
                    _ => ok("Nothing to undo."),
                }
            }
            Err(e) => fail(e),
        }
    }

    #[tool(description = "Redo the last undone change.")]
    async fn redo(&self, Parameters(a): Parameters<SiteArg>) -> CallToolResult {
        match self.call("edit_redo", &a.site, json!({})).await {
            Ok(v) => {
                let st: Option<EditState> = serde_json::from_value(v).ok();
                match st {
                    Some(s) if s.applied.is_some() => {
                        ok(format!("Redone (revision {}).", s.revision))
                    }
                    _ => ok("Nothing to redo."),
                }
            }
            Err(e) => fail(e),
        }
    }

    #[tool(
        description = "Read a value from the site's data files (texts, lists and settings used by the pages), by data path."
    )]
    async fn get_data(&self, Parameters(a): Parameters<DataArg>) -> CallToolResult {
        let segs = data_path(&a.path);
        if segs.is_empty() {
            return fail("Empty path.");
        }
        let v = match self
            .call(
                "readable_data",
                &a.site,
                json!({ "paths": [segs], "collections": [] }),
            )
            .await
        {
            Ok(v) => v,
            Err(e) => return fail(e),
        };
        let value = serde_json::from_value::<revel_protocol::ReadableData>(v)
            .ok()
            .and_then(|d| d.values.into_iter().next().flatten());
        match value {
            Some(v) => {
                let mut t = serde_json::to_string_pretty(&v).unwrap_or_default();
                if t.len() > 20_000 {
                    let mut end = 20_000;
                    while !t.is_char_boundary(end) {
                        end -= 1;
                    }
                    t.truncate(end);
                    t.push_str("\n… (cut: ask for a longer path)");
                }
                ok(t)
            }
            None => fail(format!("No data at {}.", a.path)),
        }
    }

    #[tool(
        description = "Change a text value in the site's data files (written in its YAML or JSON file, formatting kept)."
    )]
    async fn set_data(&self, Parameters(a): Parameters<SetDataArg>) -> CallToolResult {
        use revel_edit::canvas::{Binding, Seg, Source};
        let segs = data_path(&a.path);
        if segs.is_empty() {
            return fail("Empty path.");
        }
        let path = segs
            .into_iter()
            .map(|s| match s.parse::<usize>() {
                Ok(i) => Seg::Index(i),
                Err(_) => Seg::Key(s),
            })
            .collect();
        let cmd = Command::SetBound {
            binding: Binding {
                source: Source::Data,
                path,
            },
            value: a.value,
        };
        self.apply(&a.site, cmd, &format!("Data {} set", a.path))
            .await
    }

    #[tool(
        description = "CMS collections of the site (projects, articles…): their fields and entries (file name, title, page address)."
    )]
    async fn list_collections(&self, Parameters(a): Parameters<SiteArg>) -> CallToolResult {
        let cols = match self.collections(&a.site).await {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        if cols.is_empty() {
            return ok("No collection.");
        }
        let mut out = Vec::new();
        for c in cols {
            out.push(format!(
                "Collection {}{} — folder {} — fields: {}",
                c.id,
                c.label
                    .as_ref()
                    .map(|l| format!(" (« {l} »)"))
                    .unwrap_or_default(),
                c.folder,
                c.fields
                    .iter()
                    .map(|f| format!("{} ({})", f.name, f.kind.as_deref().unwrap_or("text")))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            for e in c.entries.iter().take(200) {
                let title = e
                    .data
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                out.push(format!(
                    "  {} — « {title} »{}",
                    e.file,
                    e.url
                        .as_ref()
                        .map(|u| format!(" — {u}"))
                        .unwrap_or_default()
                ));
            }
        }
        ok(out.join("\n"))
    }

    #[tool(description = "Fields (front matter) and text (Markdown body) of one CMS entry.")]
    async fn get_entry(&self, Parameters(a): Parameters<EntryArg>) -> CallToolResult {
        let cols = match self.collections(&a.site).await {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let Some(c) = cols.iter().find(|c| c.id == a.collection) else {
            return fail(format!("No collection {}.", a.collection));
        };
        let Some(e) = c.entries.iter().find(|e| e.file == a.file) else {
            return fail(format!("No entry {} in {}.", a.file, a.collection));
        };
        ok(
            serde_json::to_string_pretty(
                &json!({ "fields": e.data, "body": e.body, "url": e.url }),
            )
            .unwrap_or_default(),
        )
    }

    #[tool(
        description = "Set one field of a CMS entry (or its Markdown body with field \"body\"). The entry file is rewritten like Pages CMS would."
    )]
    async fn set_entry_field(&self, Parameters(a): Parameters<EntryFieldArg>) -> CallToolResult {
        let cols = match self.collections(&a.site).await {
            Ok(c) => c,
            Err(e) => return fail(e),
        };
        let Some(c) = cols.iter().find(|c| c.id == a.collection) else {
            return fail(format!("No collection {}.", a.collection));
        };
        if !c.entries.iter().any(|e| e.file == a.file) {
            return fail(format!("No entry {} in {}.", a.file, a.collection));
        }
        let cmd = if a.field == "body" {
            Command::SetEntryBody {
                folder: c.folder.clone(),
                file: a.file.clone(),
                body: a.value.as_str().unwrap_or_default().to_string(),
            }
        } else {
            Command::SetEntryField {
                folder: c.folder.clone(),
                file: a.file.clone(),
                key: a.field.clone(),
                value: (!a.value.is_null()).then_some(a.value),
            }
        };
        self.apply(
            &a.site,
            cmd,
            &format!("Field {} of {}/{} set", a.field, a.collection, a.file),
        )
        .await
    }
}

#[tool_handler]
impl ServerHandler for Revel {
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let target = request.arguments.as_ref().and_then(|a| {
            ["id", "selector", "path", "url", "page", "file"]
                .iter()
                .find_map(|k| a.get(*k).and_then(Value::as_str).map(str::to_string))
        });
        let via = Via {
            client: client_of(&context),
            tool: request.name.to_string(),
            target,
        };
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        VIA.scope(via, self.tool_router.call(tcc)).await
    }

    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("revel", env!("CARGO_PKG_VERSION")).with_title("Revel"),
            )
            .with_instructions(INSTRUCTIONS)
    }
}

/// Lecture d'un contenu inséré dans une page : HTML, XML (`.xml`, `.rss`, `.atom`, `.svg`) ou
/// texte (autres fichiers : `.htaccess`, `robots.txt`), comme `revel_render::DocKind`.
fn page_mode(path: &str) -> revel_markup::Mode {
    let last = path.rsplit('/').next().unwrap_or("");
    if path.ends_with('/') || last.ends_with(".html") || last.ends_with(".htm") {
        revel_markup::Mode::Html
    } else if [".xml", ".rss", ".atom", ".svg"]
        .iter()
        .any(|e| last.ends_with(e))
    {
        revel_markup::Mode::Xml
    } else {
        revel_markup::Mode::Text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Reply;
    use revel_protocol::Failure;

    struct Nothing;

    impl Invoke for Nothing {
        fn invoke(&self, _: &str, _: Value) -> Reply {
            Box::pin(async { Err(Failure::new("error-mcp-no-site")) })
        }
    }

    /// Contenu inséré lu selon la page : flux RSS en XML (`<link>…</link>` gardé), `.htaccess` en
    /// texte, pages en HTML.
    #[test]
    fn lecture_selon_la_page() {
        use revel_markup::Mode;
        assert_eq!(page_mode("/"), Mode::Html);
        assert_eq!(page_mode("/agence/"), Mode::Html);
        assert_eq!(page_mode("/404.html"), Mode::Html);
        assert_eq!(page_mode("/feed.xml"), Mode::Xml);
        assert_eq!(page_mode("/.htaccess"), Mode::Text);
        assert_eq!(page_mode("/robots.txt"), Mode::Text);
    }

    /// Chaque outil du protocole est aussi appelable sans MCP, et décrit sans référence externe.
    #[tokio::test]
    async fn outils_sans_mcp() {
        let r = Revel::new(Arc::new(Nothing));
        for t in Revel::tool_router().list_all() {
            assert!(r.run(&t.name, json!({}), "t").await.is_some(), "{}", t.name);
        }
        assert!(r.run("inconnu", json!({}), "t").await.is_none());
        let doc = Revel::openapi("http://127.0.0.1:1").to_string();
        assert!(!doc.contains("\"$ref\""), "{doc}");
    }
}
