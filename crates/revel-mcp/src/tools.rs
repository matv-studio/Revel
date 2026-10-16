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
currently open in Revel (or the one named by the optional `site` argument, an absolute folder path). \
Every edit is applied with Revel's own commands: it shows up immediately in Revel's editor, and the user \
can undo it (tool `undo`, or Ctrl+Z in Revel). Start with `get_site`, then `get_page` for the page you want \
to work on: it lists every element with its id (#id). Change text with `set_text`, attributes (class, \
href, src, alt…) with `set_attributes`, CSS with `set_styles` (styles belong to classes, per breakpoint), \
add markup with `insert_html`, remove with `remove_node`. Texts shown as {{ expressions }} come from the \
site's data files or CMS entries: resolve the variables set by {% set … %} (e.g. `{% set a = pages.agence %}` \
then `{{ a.title }}` is the data path `pages.agence.title`), read them with `get_data` and change them with \
`set_data`; CMS entries (`list_collections`) with `get_entry` and `set_entry_field`. Animations are \
attributes set with `set_attributes`: `data-revel-appear` (fade, up, down, left, right, zoom-in, zoom-out, blur, \
pop, flip, rotate, reveal), `data-revel-text` (fade, up, blur, pop, flip, typewriter), `data-revel-hover` (grow, \
shrink, lift, shadow, dim, tilt, brighten, underline), `data-revel-press` (press, squish, push), \
`data-revel-loop` (spin, pulse, float, swing, shake, blink, bounce, heartbeat, ticker) and \
`data-revel-scroll` (parallax, fade-in, zoom-in, rotate, slide-x, fade-out); value: the animation, then optional \
settings such as `dur=0.8 delay=0.2 ease=smooth|bounce|ease|linear dist=32 stagger=0.1 trigger=view|load \
replay split=chars|words|lines amount=100 mirror=on|off dir=left|right pause` (e.g. `up dur=0.8`). Keep the \
site's language and tone.";

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
    /// Pseudo-class state, e.g. ":hover"; omit for the normal state.
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

    async fn collections(&self, site: &Option<String>) -> Result<Vec<CmsCollection>, String> {
        let v = self.call("cms_collections", site, json!({})).await?;
        serde_json::from_value(v).map_err(|e| e.to_string())
    }
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
        let full = format!(
            "{}{}",
            a.selector.trim(),
            a.state.clone().unwrap_or_default()
        );
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
        let mut nodes = match revel_markup::parse(
            &a.html,
            &revel_markup::Options {
                dialect: revel_markup::Dialect::Nunjucks,
                ..Default::default()
            },
        ) {
            Ok(p) => p.nodes,
            Err(e) => return fail(format!("HTML not understood: {e}")),
        };
        nodes.retain(|n| !matches!(n, Node::Text { text } if text.trim().is_empty()));
        if nodes.is_empty() {
            return fail("Nothing to insert.");
        }
        let mut taken = d.node_ids();
        for n in &mut nodes {
            revel_edit::make::refresh_ids(n, &mut taken);
        }
        let place = if let Some(id) = &a.inside {
            target_of(&d, id).and_then(|t| match address::node(&d, &t).ok()? {
                Node::Element { children, .. } => {
                    Some((Parent::of(&t, Branch::Children), children.len()))
                }
                _ => None,
            })
        } else if let Some(id) = a.before.as_ref().or(a.after.as_ref()) {
            target_of(&d, id).and_then(|t| {
                let (p, i) = address::locate(&d, &t).ok()?;
                Some((p, if a.after.is_some() { i + 1 } else { i }))
            })
        } else if let Some(url) = &a.page {
            let pages = self.pages(&a.site).await.unwrap_or_default();
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
        };
        let Some((parent, index)) = place else {
            return fail(
                "Say where to insert: `inside`, `before` or `after` an element id, or `page`.",
            );
        };
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
        self.apply(&a.site, cmd, &what).await
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
