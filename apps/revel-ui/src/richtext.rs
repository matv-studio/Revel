//! Texte riche (étape 3.2) : éditeur visuel — paragraphes, titres, gras, italique, liens,
//! listes, citations — pour les textes des contenus, qu'ils soient écrits en Markdown (texte du
//! contenu) ou en HTML (champs importés de WordPress).
//!
//! Le texte n'est réécrit que s'il a changé : à l'ouverture, la page éditée est sérialisée une
//! première fois ; à la sortie de l'éditeur, la nouvelle sérialisation lui est comparée. Le HTML
//! produit ne garde que des balises de texte (liste blanche, sans styles ni scripts) ; le Markdown
//! garde en HTML ce qu'il ne sait pas écrire.

use crate::{Ui, icons};
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, Node};

/// Format du texte enregistré.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Html,
    Markdown,
}

/// Texte qui ressemble à du HTML (champ importé : `<p>…</p>`).
pub fn looks_like_html(s: &str) -> bool {
    let t = s.trim_start();
    t.starts_with('<') && t.contains("</")
}

fn md_to_html(md: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};
    let mut out = String::new();
    html::push_html(
        &mut out,
        Parser::new_ext(md, Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH),
    );
    out
}

fn esc_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\u{a0}', "&nbsp;")
}

fn esc_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
}

const BLOCKS: &[&str] = &[
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "ul",
    "ol",
    "li",
    "blockquote",
    "pre",
    "hr",
    "div",
    "figure",
    "figcaption",
    "table",
    "thead",
    "tbody",
    "tr",
    "td",
    "th",
];
const ALLOWED: &[&str] = &[
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "strong",
    "em",
    "a",
    "ul",
    "ol",
    "li",
    "blockquote",
    "br",
    "img",
    "code",
    "pre",
    "s",
    "u",
    "sup",
    "sub",
    "hr",
    "figure",
    "figcaption",
    "table",
    "thead",
    "tbody",
    "tr",
    "td",
    "th",
];
const VOID: &[&str] = &["br", "img", "hr"];

/// Balise d'un élément, ramenée à sa forme retenue (`b` -> `strong`, `div` -> `p`…).
fn tag_of(el: &Element) -> String {
    match el.tag_name().to_lowercase().as_str() {
        "b" => "strong".into(),
        "i" => "em".into(),
        "strike" | "del" => "s".into(),
        "div" => "p".into(),
        t => t.to_string(),
    }
}

fn children(n: &Node) -> Vec<Node> {
    let list = n.child_nodes();
    (0..list.length()).filter_map(|i| list.get(i)).collect()
}

/// HTML nettoyé d'un nœud (liste blanche de balises et d'attributs).
fn ser_html(n: &Node, out: &mut String) {
    match n.node_type() {
        Node::TEXT_NODE => out.push_str(&esc_text(&n.text_content().unwrap_or_default())),
        Node::ELEMENT_NODE => {
            let Some(el) = n.dyn_ref::<Element>() else {
                return;
            };
            let tag = tag_of(el);
            if !ALLOWED.contains(&tag.as_str()) {
                for c in children(n) {
                    ser_html(&c, out);
                }
                return;
            }
            out.push('<');
            out.push_str(&tag);
            let keep: &[&str] = match tag.as_str() {
                "a" => &["href", "title", "target", "rel"],
                "img" => &["src", "alt", "width", "height"],
                "td" | "th" => &["colspan", "rowspan"],
                _ => &[],
            };
            for a in keep {
                if let Some(v) = el.get_attribute(a) {
                    out.push_str(&format!(" {a}=\"{}\"", esc_attr(&v)));
                }
            }
            out.push('>');
            if VOID.contains(&tag.as_str()) {
                return;
            }
            for c in children(n) {
                ser_html(&c, out);
            }
            out.push_str(&format!("</{tag}>"));
        }
        _ => {}
    }
}

/// HTML du contenu de l'éditeur : un bloc par ligne au premier niveau.
fn to_html(root: &Node) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut inline = String::new();
    for c in children(root) {
        let block = c
            .dyn_ref::<Element>()
            .is_some_and(|e| BLOCKS.contains(&e.tag_name().to_lowercase().as_str()));
        if block {
            if !inline.trim().is_empty() {
                parts.push(format!("<p>{}</p>", inline.trim()));
            }
            inline.clear();
            let mut s = String::new();
            ser_html(&c, &mut s);
            parts.push(s);
        } else {
            ser_html(&c, &mut inline);
        }
    }
    if !inline.trim().is_empty() {
        parts.push(format!("<p>{}</p>", inline.trim()));
    }
    parts.join("\n")
}

/// Échappement minimal du texte en Markdown.
fn esc_md(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '*' | '_' | '`' | '[' | ']' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Markdown en ligne d'un nœud (texte, gras, italique, lien, code, image ; le reste en HTML).
fn md_inline(n: &Node, out: &mut String) {
    match n.node_type() {
        Node::TEXT_NODE => {
            let t = n
                .text_content()
                .unwrap_or_default()
                .replace(['\n', '\r'], " ");
            out.push_str(&esc_md(&t));
        }
        Node::ELEMENT_NODE => {
            let Some(el) = n.dyn_ref::<Element>() else {
                return;
            };
            let inner = |out: &mut String| {
                for c in children(n) {
                    md_inline(&c, out);
                }
            };
            match tag_of(el).as_str() {
                "strong" => {
                    let mut s = String::new();
                    inner(&mut s);
                    if !s.trim().is_empty() {
                        out.push_str(&format!("**{}**", s.trim()));
                    }
                }
                "em" => {
                    let mut s = String::new();
                    inner(&mut s);
                    if !s.trim().is_empty() {
                        out.push_str(&format!("*{}*", s.trim()));
                    }
                }
                "s" => {
                    let mut s = String::new();
                    inner(&mut s);
                    out.push_str(&format!("~~{s}~~"));
                }
                "code" => out.push_str(&format!("`{}`", el.text_content().unwrap_or_default())),
                "a" => {
                    let mut s = String::new();
                    inner(&mut s);
                    let href = el.get_attribute("href").unwrap_or_default();
                    match el.get_attribute("title") {
                        Some(t) => out.push_str(&format!("[{s}]({href} \"{t}\")")),
                        None => out.push_str(&format!("[{s}]({href})")),
                    }
                }
                "img" => out.push_str(&format!(
                    "![{}]({})",
                    el.get_attribute("alt").unwrap_or_default(),
                    el.get_attribute("src").unwrap_or_default()
                )),
                "br" => out.push_str("  \n"),
                "span" | "font" => inner(out),
                _ => {
                    // Balise sans équivalent Markdown (souligné, exposant…) : gardée en HTML.
                    let mut s = String::new();
                    ser_html(n, &mut s);
                    out.push_str(&s);
                }
            }
        }
        _ => {}
    }
}

/// Markdown d'un bloc, avec le retrait des listes imbriquées.
fn md_block(n: &Node, out: &mut String, indent: usize) {
    let Some(el) = n.dyn_ref::<Element>() else {
        let mut s = String::new();
        md_inline(n, &mut s);
        if !s.trim().is_empty() {
            out.push_str(&format!("{}\n\n", s.trim()));
        }
        return;
    };
    let pad = " ".repeat(indent);
    let tag = tag_of(el);
    match tag.as_str() {
        "p" => {
            let mut s = String::new();
            for c in children(n) {
                md_inline(&c, &mut s);
            }
            if !s.trim().is_empty() {
                out.push_str(&format!("{pad}{}\n\n", s.trim()));
            }
        }
        h if h.len() == 2 && h.starts_with('h') => {
            let level = h[1..].parse::<usize>().unwrap_or(2);
            let mut s = String::new();
            for c in children(n) {
                md_inline(&c, &mut s);
            }
            out.push_str(&format!("{pad}{} {}\n\n", "#".repeat(level), s.trim()));
        }
        "ul" | "ol" => {
            let mut i = 1;
            for li in children(n) {
                let Some(lie) = li.dyn_ref::<Element>() else {
                    continue;
                };
                if lie.tag_name().to_lowercase() != "li" {
                    continue;
                }
                let marker = if tag == "ol" {
                    format!("{i}. ")
                } else {
                    "- ".into()
                };
                i += 1;
                let mut line = String::new();
                let mut nested = String::new();
                for c in children(&li) {
                    let sub = c
                        .dyn_ref::<Element>()
                        .map(|e| e.tag_name().to_lowercase())
                        .filter(|t| t == "ul" || t == "ol");
                    if sub.is_some() {
                        md_block(&c, &mut nested, indent + marker.len());
                    } else if c
                        .dyn_ref::<Element>()
                        .is_some_and(|e| e.tag_name().eq_ignore_ascii_case("p"))
                    {
                        for cc in children(&c) {
                            md_inline(&cc, &mut line);
                        }
                    } else {
                        md_inline(&c, &mut line);
                    }
                }
                out.push_str(&format!("{pad}{marker}{}\n", line.trim()));
                out.push_str(nested.trim_end_matches('\n'));
                if !nested.is_empty() {
                    out.push('\n');
                }
            }
            if indent == 0 {
                out.push('\n');
            }
        }
        "blockquote" => {
            let mut s = String::new();
            for c in children(n) {
                md_block(&c, &mut s, 0);
            }
            for l in s.trim_end().lines() {
                out.push_str(&format!("{pad}> {l}\n").replace("> \n", ">\n"));
            }
            out.push('\n');
        }
        "pre" => out.push_str(&format!(
            "```\n{}\n```\n\n",
            el.text_content().unwrap_or_default().trim_end()
        )),
        "hr" => out.push_str("---\n\n"),
        "div" => {
            for c in children(n) {
                md_block(&c, out, indent);
            }
        }
        _ => {
            if BLOCKS.contains(&tag.as_str()) {
                // Tableau, figure : gardés en HTML.
                let mut s = String::new();
                ser_html(n, &mut s);
                out.push_str(&format!("{s}\n\n"));
            } else {
                let mut s = String::new();
                md_inline(n, &mut s);
                if !s.trim().is_empty() {
                    out.push_str(&format!("{pad}{}\n\n", s.trim()));
                }
            }
        }
    }
}

fn to_markdown(root: &Node) -> String {
    let mut out = String::new();
    let mut inline = String::new();
    for c in children(root) {
        let block = c
            .dyn_ref::<Element>()
            .is_some_and(|e| BLOCKS.contains(&e.tag_name().to_lowercase().as_str()));
        if block {
            if !inline.trim().is_empty() {
                out.push_str(&format!("{}\n\n", inline.trim()));
                inline.clear();
            }
            md_block(&c, &mut out, 0);
        } else {
            md_inline(&c, &mut inline);
        }
    }
    if !inline.trim().is_empty() {
        out.push_str(&format!("{}\n\n", inline.trim()));
    }
    let t = out.trim_end().to_string();
    if t.is_empty() { t } else { t + "\n" }
}

fn doc() -> Option<web_sys::HtmlDocument> {
    document().dyn_into::<web_sys::HtmlDocument>().ok()
}

/// Commande d'édition du navigateur sur la sélection (gras, liste, titre…).
fn exec(cmd: &str, value: &str) {
    if let Some(d) = doc() {
        let _ = d.exec_command_with_show_ui_and_value(cmd, false, value);
    }
}

/// Éditeur de texte riche. `value` : texte enregistré ; `on_commit` reçoit le nouveau texte
/// (même format) à la sortie de l'éditeur, s'il a changé.
#[component]
pub fn RichText(
    value: Signal<String>,
    format: Format,
    on_commit: Callback<String>,
    #[prop(optional)] placeholder: Option<String>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let body = NodeRef::<html::Div>::new();
    let initial = StoredValue::new(String::new());
    let focused = RwSignal::new(false);
    let source = RwSignal::new(false);
    let link_open = RwSignal::new(false);
    let link_url = RwSignal::new(String::new());
    let saved = StoredValue::new_local(None::<web_sys::Range>);
    let block = RwSignal::new("p".to_string());

    let serialize = move |root: &Node| match format {
        Format::Html => to_html(root),
        Format::Markdown => to_markdown(root),
    };
    // Valeur chargée dans l'éditeur (hors saisie en cours).
    Effect::new(move |_| {
        let v = value.get();
        let Some(b) = body.get() else {
            return;
        };
        if focused.get_untracked() {
            return;
        }
        let html = match format {
            Format::Html => v,
            Format::Markdown => md_to_html(&v),
        };
        b.set_inner_html(&html);
        initial.set_value(serialize(&b));
    });
    let commit = move || {
        let Some(b) = body.get_untracked() else {
            return;
        };
        let now = serialize(&b);
        if now != initial.get_value() {
            initial.set_value(now.clone());
            on_commit.run(now);
        }
    };
    // Type du bloc où se trouve le curseur (liste des formats).
    let track_block = move || {
        if let Some(d) = doc()
            && let Ok(v) = d.query_command_value("formatBlock")
        {
            let v = v.to_lowercase();
            block.set(if v.is_empty() || v == "div" {
                "p".into()
            } else {
                v
            });
        }
    };
    let save_range = move || {
        let r = window()
            .get_selection()
            .ok()
            .flatten()
            .filter(|s| s.range_count() > 0)
            .and_then(|s| s.get_range_at(0).ok());
        saved.set_value(r);
    };
    let restore_range = move || {
        if let Some(b) = body.get_untracked() {
            let _ = b.focus();
        }
        if let (Some(r), Some(sel)) = (saved.get_value(), window().get_selection().ok().flatten()) {
            let _ = sel.remove_all_ranges();
            let _ = sel.add_range(&r);
        }
    };
    let tool = move |key: &'static str, icon: fn() -> AnyView, run: fn()| {
        view! {
            <button type="button" class="rt-btn" title=ui.t(key) aria-label=ui.t(key) tabindex="-1"
                on:mousedown=move |e: ev::MouseEvent| { e.prevent_default(); run(); track_block(); }>{icon()}</button>
        }
    };
    let open_link = move || {
        save_range();
        // Lien existant sous le curseur : son adresse proposée.
        let href = window()
            .get_selection()
            .ok()
            .flatten()
            .and_then(|s| s.anchor_node())
            .and_then(|n| {
                let el = if n.node_type() == Node::ELEMENT_NODE {
                    n.dyn_into::<Element>().ok()
                } else {
                    n.parent_element()
                };
                el?.closest("a").ok().flatten()
            })
            .and_then(|a| a.get_attribute("href"))
            .unwrap_or_default();
        link_url.set(href);
        link_open.set(true);
    };
    let apply_link = move || {
        let url = link_url.get_untracked().trim().to_string();
        link_open.set(false);
        restore_range();
        if url.is_empty() {
            exec("unlink", "");
        } else {
            exec("createLink", &url);
        }
        commit();
    };
    let ph = placeholder.unwrap_or_default();

    view! {
        <div class="rt" class:is-focused=move || focused.get() class:is-source=move || source.get()>
            <div class="rt-bar" role="toolbar" aria-label=ui.t("rt-toolbar")>
                <div class="select rt-block" title=ui.t("rt-block")>
                    <select class="input" tabindex="-1" aria-label=ui.t("rt-block")
                        prop:disabled=move || source.get()
                        on:mousedown=move |_| save_range()
                        on:change=move |e| {
                            let v = event_target_value(&e);
                            restore_range();
                            exec("formatBlock", &v);
                            block.set(v);
                        }>
                        {[("p", "rt-p"), ("h2", "rt-h2"), ("h3", "rt-h3"), ("h4", "rt-h4"), ("blockquote", "rt-quote")]
                            .into_iter().map(|(v, k)| view! {
                                <option value=v selected=move || block.get() == v>{ui.t(k)}</option>
                            }).collect_view()}
                    </select>
                </div>
                {tool("rt-bold", icons::bold, || exec("bold", ""))}
                {tool("rt-italic", icons::italic, || exec("italic", ""))}
                <button type="button" class="rt-btn" class:is-on=move || link_open.get() title=ui.t("rt-link") aria-label=ui.t("rt-link") tabindex="-1"
                    on:mousedown=move |e: ev::MouseEvent| { e.prevent_default(); open_link(); }>{icons::link()}</button>
                {tool("rt-ul", icons::list, || exec("insertUnorderedList", ""))}
                {tool("rt-ol", icons::list_ordered, || exec("insertOrderedList", ""))}
                {tool("rt-clear", icons::reset, || { exec("removeFormat", ""); exec("formatBlock", "p"); })}
                <span class="toolbar-spacer"></span>
                <button type="button" class="rt-btn rt-source" class:is-on=move || source.get() tabindex="-1"
                    title=ui.t(if format == Format::Markdown { "rt-source-md" } else { "rt-source-html" })
                    aria-label=ui.t(if format == Format::Markdown { "rt-source-md" } else { "rt-source-html" })
                    on:mousedown=move |e: ev::MouseEvent| e.prevent_default()
                    on:click=move |_| source.update(|s| *s = !*s)>{icons::code()}</button>
            </div>
            <Show when=move || link_open.get()>
                <div class="rt-link">
                    <input class="input" type="url" placeholder="https://… ou /page/" prop:value=move || link_url.get()
                        on:input=move |e| link_url.set(event_target_value(&e))
                        on:keydown=move |e: ev::KeyboardEvent| match e.key().as_str() {
                            "Enter" => { e.prevent_default(); apply_link(); }
                            "Escape" => { e.prevent_default(); e.stop_propagation(); link_open.set(false); restore_range(); }
                            _ => {}
                        }/>
                    <button type="button" class="btn btn--small" on:click=move |_| apply_link()>{ui.t("rt-link-apply")}</button>
                </div>
            </Show>
            <div class="rt-body" node_ref=body contenteditable="true" spellcheck="true"
                class:is-hidden=move || source.get()
                data-placeholder=ph
                on:focus=move |_| {
                    focused.set(true);
                    exec("defaultParagraphSeparator", "p");
                }
                on:blur=move |_| {
                    focused.set(false);
                    commit();
                }
                on:keyup=move |_| track_block()
                on:mouseup=move |_| track_block()
                on:keydown=move |e: ev::KeyboardEvent| {
                    if (e.ctrl_key() || e.meta_key()) && e.key().eq_ignore_ascii_case("k") {
                        e.prevent_default();
                        open_link();
                    }
                }
                on:paste=move |e: ev::ClipboardEvent| {
                    // Collé en texte brut (pas de styles venus d'un traitement de texte).
                    if let Some(dt) = e.clipboard_data()
                        && let Ok(text) = dt.get_data("text/plain")
                    {
                        e.prevent_default();
                        exec("insertText", &text);
                    }
                }></div>
            <Show when=move || source.get()>
                <textarea class="input rt-code" spellcheck="false" rows="10"
                    prop:value=move || value.get()
                    on:change=move |e| on_commit.run(event_target_value(&e))></textarea>
            </Show>
        </div>
    }
}
