//! Informations du site ([`revel_edit::siteinfo`]) dans l'interface :
//! - Paramètres › Général › Informations du site : coordonnées, réseaux… écrites une fois (nom
//!   affiché, valeur, nombre d'endroits qui la reprennent, ajout, retrait) ;
//! - Paramètres › Navigation › Menus du site : listes de liens (texte, adresse, ordre) ;
//! - Styles d'un élément : texte, lien ou image reliés à une information du site, année en
//!   cours ; liste de liens tirée d'un menu (créer un menu avec les liens écrits, en changer) ;
//!   lien d'un menu relié au texte et à l'adresse de chaque lien.

use crate::editor::Ed;
use crate::site::DocSession;
use crate::{Section, Tab, Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::siteinfo::{self as si, InfoValue, MenuLink};
use revel_edit::{Command, Target};
use revel_model::{Attr, Node, Site};
use serde_json::Value;

/// Informations du site (objet `site`) : du document, sinon lues par le cœur dans le fichier de
/// données ; relues quand le document change.
pub fn load(path: String, doc: Signal<Option<Site>>) -> RwSignal<Option<Value>> {
    let out = RwSignal::new(None);
    let path = StoredValue::new(path);
    Effect::new(move |_| {
        let Some(local) = doc.with(|d| d.as_ref().map(|d| d.data.get(si::ROOT).cloned())) else {
            return;
        };
        if let Some(v) = local {
            let _ = out.try_set(Some(v));
            return;
        }
        let p = path.get_value();
        spawn_local(async move {
            if let Ok(d) = api::readable_data(&p, &[vec![si::ROOT.to_string()]], &[]).await {
                let _ = out.try_set(d.values.into_iter().next().flatten());
            }
        });
    });
    out
}

/// Informations du site du document ouvert dans l'éditeur.
pub fn load_ed(ed: Ed) -> RwSignal<Option<Value>> {
    load(ed.path.get_value(), ed.doc.into())
}

/// Nom affiché d'une information : donné dans les paramètres, sinon traduit pour les noms
/// courants (`phone` -> « Téléphone »), sinon tiré de la clé.
pub fn label(ui: Ui, d: Option<&Site>, path: &[String]) -> String {
    if let Some(l) = d.and_then(|d| d.settings.site_labels.get(&path.join("."))) {
        return l.clone();
    }
    path.iter()
        .enumerate()
        .map(|(i, k)| {
            // Nom donné à un parent (« Adresse ») repris.
            if i + 1 < path.len()
                && let Some(l) = d.and_then(|d| d.settings.site_labels.get(&path[..=i].join(".")))
            {
                return l.clone();
            }
            let id = format!("siteinfo-key-{}", k.to_ascii_lowercase());
            let t = ui.now(&id, &[]);
            if t == id || t.is_empty() {
                crate::names::humanize(k)
            } else {
                t
            }
        })
        .collect::<Vec<_>>()
        .join(" › ")
}

fn looks_mail(v: &str) -> bool {
    v.contains('@') && !v.contains(' ') && !v.contains('/')
}

fn looks_phone(v: &str) -> bool {
    let digits = v.chars().filter(|c| c.is_ascii_digit()).count();
    digits >= 6
        && v.chars()
            .all(|c| c.is_ascii_digit() || " +().-/".contains(c))
}

fn looks_url(v: &str) -> bool {
    v.starts_with("http://") || v.starts_with("https://") || v.starts_with('/')
}

fn looks_image(v: &str) -> bool {
    let l = v.to_ascii_lowercase();
    [".jpg", ".jpeg", ".png", ".webp", ".avif", ".gif", ".svg"]
        .iter()
        .any(|e| l.split('?').next().unwrap_or(&l).ends_with(e))
}

// ------------------------------------------------------------------ paramètres

/// Écrit une information (chemin sous `site`) là où elle est rangée.
fn write(s: DocSession, path: Vec<String>, value: Option<Value>) {
    if let Some(cmd) = s
        .doc
        .with_untracked(|d| d.as_ref().map(|d| si::set_value(d, &path, value)))
    {
        s.apply.run(cmd);
    }
}

/// Paramètres › Général › Informations du site.
#[component]
pub fn SiteInfo(s: DocSession, path: String) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let obj = load(path, s.doc.into());
    let new_label = RwSignal::new(String::new());
    let new_value = RwSignal::new(String::new());
    let add = move || {
        let l = new_label.get_untracked().trim().to_string();
        let v = new_value.get_untracked().trim().to_string();
        if l.is_empty() {
            return;
        }
        let taken: Vec<String> = obj.with_untracked(|o| {
            o.as_ref()
                .and_then(Value::as_object)
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default()
        });
        let base = si::key_from(&l);
        let key = if taken.contains(&base) {
            (2..)
                .map(|n| format!("{base}_{n}"))
                .find(|k| !taken.contains(k))
                .unwrap_or(base)
        } else {
            base
        };
        let cmd = s.doc.with_untracked(|d| {
            d.as_ref().map(|d| Command::Batch {
                commands: vec![
                    si::set_value(
                        d,
                        std::slice::from_ref(&key),
                        Some(Value::String(v.clone())),
                    ),
                    Command::SetSiteLabel {
                        path: key.clone(),
                        label: Some(l.clone()),
                    },
                ],
            })
        });
        if let Some(cmd) = cmd {
            s.apply.run(cmd);
            new_label.set(String::new());
            new_value.set(String::new());
        }
    };
    view! {
        <section class="page settings-page" data-sub="site-info">
            <header class="page-head"><h1 title=ui.t("siteinfo-help")>{ui.t("siteinfo-title")}</h1></header>
            <div class="section">
                <p class="muted small">{ui.t("siteinfo-intro")}</p>
                {move || {
                    let vals: Vec<InfoValue> = obj.get().map(|o| si::values(&o)).unwrap_or_default();
                    if vals.is_empty() {
                        return view! { <p class="muted">{ui.t("siteinfo-none")}</p> }.into_any();
                    }
                    view! {
                        <div class="siteinfo-list">
                            {vals.into_iter().map(|v| {
                                let p = StoredValue::new(v.path.clone());
                                let (name, uses) = s.doc.with_untracked(|d| (label(ui, d.as_ref(), &v.path), d.as_ref().map(|d| si::uses(d, &v.path)).unwrap_or(0)));
                                let key = v.path.join(".");
                                view! {
                                    <div class="siteinfo-row" data-info=key.clone()>
                                        <input class="input siteinfo-label" aria-label=ui.t("siteinfo-label") prop:value=name.clone()
                                            title=key.clone()
                                            on:change=move |e| {
                                                let l = event_target_value(&e);
                                                s.apply.run(Command::SetSiteLabel { path: p.get_value().join("."), label: Some(l) });
                                            }/>
                                        <input class="input siteinfo-value" aria-label=name.clone() prop:value=v.value.clone()
                                            on:change=move |e| write(s, p.get_value(), Some(Value::String(event_target_value(&e).trim().to_string())))/>
                                        <span class="siteinfo-uses muted small" title=ui.t("siteinfo-uses-help")>
                                            {ui.now("siteinfo-uses", &[("count", uses.to_string())])}
                                        </span>
                                        <button class="icon-btn icon-btn--small siteinfo-remove" aria-label=ui.t("action-remove")
                                            title={if uses > 0 { ui.tx("siteinfo-remove-used", &[]) } else { ui.tx("action-remove", &[]) }}
                                            disabled={uses > 0}
                                            on:click=move |_| write(s, p.get_value(), None)>{icons::trash()}</button>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }.into_any()
                }}
                <div class="siteinfo-row siteinfo-add">
                    <input class="input siteinfo-label" placeholder=ui.t("siteinfo-new-label") aria-label=ui.t("siteinfo-new-label")
                        prop:value=move || new_label.get() on:input=move |e| new_label.set(event_target_value(&e))/>
                    <input class="input siteinfo-value" placeholder=ui.t("siteinfo-new-value") aria-label=ui.t("siteinfo-new-value")
                        prop:value=move || new_value.get() on:input=move |e| new_value.set(event_target_value(&e))
                        on:keydown=move |e: leptos::ev::KeyboardEvent| if e.key() == "Enter" { add() }/>
                    <button class="btn btn--small" disabled=move || new_label.get().trim().is_empty()
                        on:click=move |_| add()>{icons::plus()}{ui.t("siteinfo-add")}</button>
                </div>
            </div>
        </section>
    }
}

/// Paramètres › Navigation › Menus du site.
#[component]
pub fn SiteMenus(s: DocSession, path: String) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let obj = load(path, s.doc.into());
    let pages = Memo::new(move |_| {
        s.doc.with(|d| {
            d.as_ref()
                .map(|d| {
                    d.pages
                        .iter()
                        .filter(|p| !p.path.contains('{') && !p.draft && p.path.ends_with('/'))
                        .map(|p| p.path.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });
    let new_name = RwSignal::new(String::new());
    let create = move || {
        let n = new_name.get_untracked().trim().to_string();
        if n.is_empty() {
            return;
        }
        let key = si::key_from(&n);
        let cmd = s.doc.with_untracked(|d| {
            d.as_ref().map(|d| Command::Batch {
                commands: vec![
                    si::set_value(
                        d,
                        &[si::MENUS.to_string(), key.clone()],
                        Some(si::menu_value(&[MenuLink {
                            label: ui.now("menus-new", &[]),
                            url: "/".into(),
                        }])),
                    ),
                    Command::SetSiteLabel {
                        path: format!("{}.{key}", si::MENUS),
                        label: Some(n.clone()),
                    },
                ],
            })
        });
        if let Some(cmd) = cmd {
            s.apply.run(cmd);
            new_name.set(String::new());
        }
    };
    view! {
        <section class="page settings-page" data-sub="site-menus">
            <header class="page-head"><h1 title=ui.t("sitemenus-help")>{ui.t("sitemenus-title")}</h1></header>
            {move || {
                let menus = obj.get().map(|o| si::menus(&o)).unwrap_or_default();
                if menus.is_empty() {
                    return view! { <div class="section"><p class="muted">{ui.t("sitemenus-none")}</p></div> }.into_any();
                }
                menus.into_iter().enumerate().map(|(n, (name, links))| {
                    let key = StoredValue::new(name.clone());
                    let list = StoredValue::new(links.clone());
                    let write_links = move |links: Vec<MenuLink>| {
                        write(s, vec![si::MENUS.to_string(), key.get_value()], Some(si::menu_value(&links)))
                    };
                    let (title, uses) = s.doc.with_untracked(|d| {
                        let p = vec![si::MENUS.to_string(), name.clone()];
                        (label(ui, d.as_ref(), &p), d.as_ref().map(|d| si::uses(d, &p)).unwrap_or(0))
                    });
                    let dl = format!("sitemenu-pages-{n}");
                    let count = links.len();
                    view! {
                        <div class="section navlist" data-menu=name.clone()>
                            <h2 class="navlist-head">
                                <input class="input navlist-name" aria-label=ui.t("sitemenus-name") prop:value=title.clone()
                                    on:change=move |e| s.apply.run(Command::SetSiteLabel { path: format!("{}.{}", si::MENUS, key.get_value()), label: Some(event_target_value(&e)) })/>
                                <span class="siteinfo-uses muted small">{ui.now("siteinfo-uses", &[("count", uses.to_string())])}</span>
                                <button class="icon-btn icon-btn--small siteinfo-remove" aria-label=ui.t("sitemenus-remove")
                                    title={if uses > 0 { ui.tx("siteinfo-remove-used", &[]) } else { ui.tx("sitemenus-remove", &[]) }}
                                    disabled={uses > 0}
                                    on:click=move |_| write(s, vec![si::MENUS.to_string(), key.get_value()], None)>{icons::trash()}</button>
                            </h2>
                            <datalist id=dl.clone()>
                                {move || pages.get().into_iter().map(|p| view! { <option value=p></option> }).collect_view()}
                            </datalist>
                            <ul class="navlist-links">
                                {links.into_iter().enumerate().map(|(i, link)| {
                                    let edit = move |f: &dyn Fn(&mut MenuLink)| {
                                        let mut all = list.get_value();
                                        if let Some(x) = all.get_mut(i) {
                                            f(x);
                                        }
                                        write_links(all);
                                    };
                                    let shift = move |d: isize| {
                                        let mut all = list.get_value();
                                        let j = i as isize + d;
                                        if j >= 0 && (j as usize) < all.len() {
                                            all.swap(i, j as usize);
                                            write_links(all);
                                        }
                                    };
                                    view! {
                                        <li class="navlist-link">
                                            <input class="input navlist-label" aria-label=ui.t("menus-label") placeholder=ui.t("menus-label")
                                                prop:value=link.label.clone()
                                                on:change=move |e| { let v = event_target_value(&e); edit(&|x| x.label = v.trim().to_string()) }/>
                                            <input class="input input--mono navlist-url" aria-label=ui.t("menus-url") placeholder="/page/" list=dl.clone()
                                                prop:value=link.url.clone()
                                                on:change=move |e| { let v = event_target_value(&e); edit(&|x| x.url = v.trim().to_string()) }/>
                                            <button class="icon-btn icon-btn--small" title=ui.t("layers-up") aria-label=ui.t("layers-up")
                                                disabled=i == 0 on:click=move |_| shift(-1)>{icons::arrow_up()}</button>
                                            <button class="icon-btn icon-btn--small" title=ui.t("layers-down") aria-label=ui.t("layers-down")
                                                disabled=i + 1 == count on:click=move |_| shift(1)>{icons::arrow_down()}</button>
                                            <button class="icon-btn icon-btn--small" title=ui.t("action-remove") aria-label=ui.t("action-remove")
                                                on:click=move |_| {
                                                    let mut all = list.get_value();
                                                    all.remove(i);
                                                    write_links(all);
                                                }>{icons::trash()}</button>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                            <button class="btn btn--small navlist-add" on:click=move |_| {
                                let mut all = list.get_value();
                                all.push(MenuLink { label: ui.now("menus-new", &[]), url: "/".into() });
                                write_links(all);
                            }>{icons::plus()}{ui.t("menus-add")}</button>
                        </div>
                    }
                }).collect_view().into_any()
            }}
            <div class="section sitemenus-add">
                <input class="input" placeholder=ui.t("sitemenus-new") aria-label=ui.t("sitemenus-new")
                    prop:value=move || new_name.get() on:input=move |e| new_name.set(event_target_value(&e))
                    on:keydown=move |e: leptos::ev::KeyboardEvent| if e.key() == "Enter" { create() }/>
                <button class="btn btn--small" disabled=move || new_name.get().trim().is_empty()
                    on:click=move |_| create()>{icons::plus()}{ui.t("sitemenus-add")}</button>
            </div>
        </section>
    }
}

// ------------------------------------------------------------------ éléments

fn attr<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .and_then(|a| a.value.as_deref())
}

const YEAR: &str = "@year";
const YEAR_EXPR: &str = "now() | date(\"%Y\")";

/// Information reliée par une expression (`site.address.city` -> `address.city`, année en cours).
fn info_of(expr: &str) -> Option<String> {
    let e = expr.trim();
    let squashed: String = e.chars().filter(|c| !c.is_whitespace()).collect();
    if squashed == "now()|date(\"%Y\")" || squashed == "now()|date('%Y')" {
        return Some(YEAR.into());
    }
    let p = e.strip_prefix(&format!("{}.", si::ROOT))?;
    revel_edit::editable::site_path(e).then(|| p.to_string())
}

/// Texte d'un élément : (texte avant, information reliée, texte après) ; `None` si son contenu
/// n'est pas un texte simple.
fn text_parts(children: &[Node]) -> Option<(String, Option<String>, String)> {
    let t = |n: &Node| match n {
        Node::Text { text } => Some(text.clone()),
        _ => None,
    };
    let o = |n: &Node| match n {
        Node::Out { expr } => info_of(expr),
        _ => None,
    };
    match children {
        [] => Some((String::new(), None, String::new())),
        [a] if t(a).is_some() => Some((String::new(), None, String::new())),
        [a] => o(a).map(|k| (String::new(), Some(k), String::new())),
        [a, b] if t(a).is_some() && o(b).is_some() => Some((t(a)?, o(b), String::new())),
        [a, b] if o(a).is_some() && t(b).is_some() => Some((String::new(), o(a), t(b)?)),
        [a, b, c] if t(a).is_some() && o(b).is_some() && t(c).is_some() => {
            Some((t(a)?, o(b), t(c)?))
        }
        _ => None,
    }
}

/// Lien relié : `url:<chemin>`, `mail:<chemin>`, `tel:<chemin>`.
fn link_of(href: &str) -> Option<String> {
    let h = href.trim();
    let inner = |s: &str| -> Option<String> {
        let e = s.strip_prefix("{{")?.strip_suffix("}}")?.trim();
        Some(e.to_string())
    };
    if let Some(rest) = h.strip_prefix("mailto:") {
        return inner(rest)
            .and_then(|e| info_of(&e))
            .map(|p| format!("mail:{p}"));
    }
    if let Some(rest) = h.strip_prefix("tel:") {
        let e = inner(rest)?;
        let e = e
            .strip_suffix("| tel")
            .or_else(|| e.strip_suffix("|tel"))?
            .trim()
            .to_string();
        return info_of(&e).map(|p| format!("tel:{p}"));
    }
    inner(h)
        .and_then(|e| info_of(&e))
        .map(|p| format!("url:{p}"))
}

/// Remplace l'élément `at` par le même avec d'autres enfants (même identifiant).
fn set_children(d: &Site, at: &Target, kids: Vec<Node>) -> Option<Command> {
    let Ok(Node::Element { id, tag, attrs, .. }) = revel_edit::address::node(d, at) else {
        return None;
    };
    let (parent, index) = revel_edit::address::locate(d, at).ok()?;
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
}

/// Réglages d'un élément : texte, lien ou image reliés aux informations du site ; liste de liens
/// tirée d'un menu ; lien d'un menu.
pub fn element_settings(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    // Lien d'une liste de menu : texte et adresse de chaque lien.
    if let Some((each, menu, item)) = ed
        .doc
        .with_untracked(|d| si::enclosing_menu(d.as_ref()?, at))
    {
        return menu_item_settings(ui, ed, at, node, each, menu, item);
    }
    let menu_view = menu_list_settings(ui, ed, at, node);
    let bind_view = value_settings(ui, ed, at, node);
    match (menu_view, bind_view) {
        (None, None) => None,
        (a, b) => Some(view! { {a} {b} }.into_any()),
    }
}

/// Liste de liens : tirée d'un menu du site (le changer, le modifier), ou créer un menu avec ses
/// liens écrits (ou les relier à un menu existant).
fn menu_list_settings(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    let (each_at, current, written) = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let (each_at, current) = match node {
            Node::Each { .. } => (Some(at.clone()), si::is_menu_list(d, at)),
            Node::Element { children, .. } => {
                let found = children.iter().find_map(|c| match c {
                    Node::Each {
                        id: Some(id), expr, ..
                    } => si::menu_of(expr).map(|m| {
                        (
                            Target {
                                owner: at.owner.clone(),
                                node: id.clone(),
                            },
                            m,
                        )
                    }),
                    _ => None,
                });
                match found {
                    Some((t, m)) => (Some(t), Some(m)),
                    None => (None, None),
                }
            }
            _ => (None, None),
        };
        let written = match node {
            Node::Element { .. } => si::link_items(d, at).len(),
            _ => 0,
        };
        Some((each_at, current, written))
    })?;
    if current.is_none() && written < 2 {
        return None;
    }
    let obj = load_ed(ed);
    let at_s = StoredValue::new(at.clone());
    let each_s = StoredValue::new(each_at);
    let new_name = RwSignal::new(String::new());
    let switch = move |name: String| {
        if let Some(each) = each_s.get_value() {
            ed.apply(ui, si::switch_menu(&each, &name), false);
        }
    };
    let link_to = move |name: String| {
        let links = obj.with_untracked(|o| {
            o.as_ref()
                .map(si::menus)
                .and_then(|ms| ms.into_iter().find(|(n, _)| *n == name).map(|(_, l)| l))
        });
        let cmd = ed.doc.with_untracked(|d| {
            si::use_menu(d.as_ref()?, &at_s.get_value(), &name, links.as_deref())
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let create = move || {
        let n = new_name.get_untracked().trim().to_string();
        if n.is_empty() {
            return;
        }
        let key = si::key_from(&n);
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let c = si::use_menu(d, &at_s.get_value(), &key, None)?;
            Some(Command::Batch {
                commands: vec![
                    c,
                    Command::SetSiteLabel {
                        path: format!("{}.{key}", si::MENUS),
                        label: Some(n.clone()),
                    },
                ],
            })
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let menu_names = move || {
        let names: Vec<String> = obj.with(|o| {
            o.as_ref()
                .map(si::menus)
                .unwrap_or_default()
                .into_iter()
                .map(|(n, _)| n)
                .collect()
        });
        ed.doc.with_untracked(|d| {
            names
                .into_iter()
                .map(|n| {
                    let l = label(ui, d.as_ref(), &[si::MENUS.to_string(), n.clone()]);
                    (n, l)
                })
                .collect::<Vec<_>>()
        })
    };
    let cur = current.clone().unwrap_or_default();
    Some(
        view! {
            <div class="insp-group" data-menu-list="">
                <p class="panel-title" title=ui.t("sitemenus-help")>{ui.t("sitemenus-list")}</p>
                {match current {
                    Some(_) => view! {
                        <label class="field" data-menu="current">
                            <span class="label">{ui.t("sitemenus-links-of")}</span>
                            <div class="select">
                                <select class="input" on:change=move |e| switch(event_target_value(&e))>
                                    {move || menu_names().into_iter().map(|(n, l)| {
                                        let on = n == cur;
                                        view! { <option value=n prop:selected=on>{l}</option> }
                                    }).collect_view()}
                                </select>
                            </div>
                        </label>
                        <button type="button" class="link-btn" on:click=move |_| ui.tab(Tab::Settings(Section::Menus))>
                            {ui.t("sitemenus-edit")}
                        </button>
                    }.into_any(),
                    None => view! {
                        <p class="muted small">{ui.now("sitemenus-written", &[("count", written.to_string())])}</p>
                        <div class="field-line" data-menu="create">
                            <input class="input" placeholder=ui.t("sitemenus-new") aria-label=ui.t("sitemenus-new")
                                prop:value=move || new_name.get() on:input=move |e| new_name.set(event_target_value(&e))/>
                            <button type="button" class="btn btn--small" disabled=move || new_name.get().trim().is_empty()
                                on:click=move |_| create()>{ui.t("sitemenus-create")}</button>
                        </div>
                        {move || {
                            let names = menu_names();
                            (!names.is_empty()).then(|| view! {
                                <label class="field" data-menu="link">
                                    <span class="label" title=ui.t("sitemenus-link-help")>{ui.t("sitemenus-link")}</span>
                                    <div class="select">
                                        <select class="input" on:change=move |e| { let v = event_target_value(&e); if !v.is_empty() { link_to(v) } }>
                                            <option value="" selected=true>{ui.t("hover-none")}</option>
                                            {names.into_iter().map(|(n, l)| view! { <option value=n>{l}</option> }).collect_view()}
                                        </select>
                                    </div>
                                </label>
                            })
                        }}
                    }.into_any(),
                }}
            </div>
        }
        .into_any(),
    )
}

/// Lien d'une liste de menu : texte et adresse reliés à chaque lien du menu.
fn menu_item_settings(
    ui: Ui,
    ed: Ed,
    at: &Target,
    node: &Node,
    _each: Target,
    menu: String,
    item: String,
) -> Option<AnyView> {
    let Node::Element {
        tag,
        attrs,
        children,
        ..
    } = node
    else {
        return None;
    };
    let is_link = tag.eq_ignore_ascii_case("a");
    let text_bound = matches!(children.as_slice(), [Node::Out { expr }] if expr.trim() == format!("{item}.label"));
    let text_ok = matches!(children.as_slice(), [] | [Node::Text { .. }]) || text_bound;
    let href_bound =
        attr(attrs, "href").is_some_and(|h| h.trim() == format!("{{{{ {item}.url }}}}"));
    if !is_link && !text_ok {
        return None;
    }
    let at = StoredValue::new(at.clone());
    let item = StoredValue::new(item);
    let set_text = move |bound: bool| {
        let cmd = ed.doc.with_untracked(|d| {
            let kid = if bound {
                Node::Out {
                    expr: format!("{}.label", item.get_value()),
                }
            } else {
                Node::Text {
                    text: ui.now("block-bound-text", &[]),
                }
            };
            set_children(d.as_ref()?, &at.get_value(), vec![kid])
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let set_href = move |bound: bool| {
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: "href".into(),
                value: Some(if bound {
                    format!("{{{{ {}.url }}}}", item.get_value())
                } else {
                    "#".into()
                }),
            },
            false,
        );
    };
    let menu_label = ed
        .doc
        .with_untracked(|d| label(ui, d.as_ref(), &[si::MENUS.to_string(), menu.clone()]));
    Some(
        view! {
            <div class="insp-group" data-menu-item="">
                <p class="panel-title" title=ui.t("sitemenus-item-help")>{ui.now("sitemenus-item", &[("menu", menu_label)])}</p>
                {text_ok.then(|| view! {
                    <label class="field">
                        <span class="label">{ui.t("block-bound-text-label")}</span>
                        <div class="select">
                            <select class="input" on:change=move |e| set_text(event_target_value(&e) == "label")>
                                <option value="" prop:selected=!text_bound>{ui.t("block-bound-written")}</option>
                                <option value="label" prop:selected=text_bound>{ui.t("sitemenus-item-label")}</option>
                            </select>
                        </div>
                    </label>
                })}
                {is_link.then(|| view! {
                    <label class="field">
                        <span class="label">{ui.t("block-bound-link")}</span>
                        <div class="select">
                            <select class="input" on:change=move |e| set_href(event_target_value(&e) == "url")>
                                <option value="" prop:selected=!href_bound>{ui.t("siteinfo-link-written")}</option>
                                <option value="url" prop:selected=href_bound>{ui.t("sitemenus-item-url")}</option>
                            </select>
                        </div>
                    </label>
                })}
                <button type="button" class="link-btn" on:click=move |_| ui.tab(Tab::Settings(Section::Menus))>
                    {ui.t("sitemenus-edit")}
                </button>
            </div>
        }
        .into_any(),
    )
}

/// Texte, lien ou image d'un élément reliés à une information du site (ou à l'année en cours).
fn value_settings(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    let (text, link, image) = match node {
        Node::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            let text = text_parts(children)
                // Texte court seulement (un paragraphe se modifie dans la page).
                .filter(|(b, k, a)| {
                    k.is_some()
                        || children.iter().all(|c| match c {
                            Node::Text { text } => text.chars().count() <= 80,
                            _ => true,
                        }) && b.len() + a.len() <= 80
                })
                .filter(|_| !tag.eq_ignore_ascii_case("html") && !tag.eq_ignore_ascii_case("body"));
            let link = tag
                .eq_ignore_ascii_case("a")
                .then(|| attr(attrs, "href").unwrap_or_default().to_string());
            (text, link, None)
        }
        Node::Image(img) => (
            None,
            None,
            Some(
                img.src
                    .trim()
                    .strip_prefix(&format!("{}.", si::ROOT))
                    .map(str::to_string),
            ),
        ),
        _ => (None, None, None),
    };
    if text.is_none() && link.is_none() && image.is_none() {
        return None;
    }
    // Texte vide d'un élément qui en contient d'autres (bloc) : pas un texte.
    if let Node::Element { children, .. } = node
        && children.is_empty()
        && link.is_none()
    {
        return None;
    }
    let obj = load_ed(ed);
    let at = StoredValue::new(at.clone());
    let text_s = StoredValue::new(text.clone());
    // Valeurs proposées : (chemin, nom, valeur).
    let entries = move || -> Vec<(String, String, String)> {
        let vals = obj.with(|o| o.as_ref().map(si::values).unwrap_or_default());
        ed.doc.with_untracked(|d| {
            vals.into_iter()
                .map(|v| (v.path.join("."), label(ui, d.as_ref(), &v.path), v.value))
                .collect()
        })
    };
    let value_of = move |p: &str| -> String {
        if p == YEAR {
            return js_sys::Date::new_0().get_full_year().to_string();
        }
        entries()
            .into_iter()
            .find(|(k, _, _)| k == p)
            .map(|(_, _, v)| v)
            .unwrap_or_default()
    };
    let set_text = move |choice: String| {
        let (before, cur, after) = text_s.get_value().unwrap_or_default();
        let kids = if choice.is_empty() {
            // Texte écrit : la valeur affichée jusque-là, qu'on peut modifier.
            let shown = cur.as_deref().map(value_of).unwrap_or_default();
            vec![Node::Text {
                text: format!("{before}{shown}{after}"),
            }]
        } else {
            let mut k = Vec::new();
            if !before.is_empty() {
                k.push(Node::Text { text: before });
            }
            k.push(Node::Out {
                expr: if choice == YEAR {
                    YEAR_EXPR.to_string()
                } else {
                    format!("{}.{choice}", si::ROOT)
                },
            });
            if !after.is_empty() {
                k.push(Node::Text { text: after });
            }
            k
        };
        let cmd = ed
            .doc
            .with_untracked(|d| set_children(d.as_ref()?, &at.get_value(), kids));
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let set_affix = move |before: Option<String>, after: Option<String>| {
        let (b, cur, a) = text_s.get_value().unwrap_or_default();
        let Some(cur) = cur else { return };
        let (b, a) = (before.unwrap_or(b), after.unwrap_or(a));
        let mut k = Vec::new();
        if !b.is_empty() {
            k.push(Node::Text { text: b });
        }
        k.push(Node::Out {
            expr: if cur == YEAR {
                YEAR_EXPR.to_string()
            } else {
                format!("{}.{cur}", si::ROOT)
            },
        });
        if !a.is_empty() {
            k.push(Node::Text { text: a });
        }
        let cmd = ed
            .doc
            .with_untracked(|d| set_children(d.as_ref()?, &at.get_value(), k));
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let link_s = StoredValue::new(link.clone());
    let set_link = move |choice: String| {
        let href = match choice.split_once(':') {
            Some(("url", p)) => format!("{{{{ {}.{p} }}}}", si::ROOT),
            Some(("mail", p)) => format!("mailto:{{{{ {}.{p} }}}}", si::ROOT),
            Some(("tel", p)) => format!("tel:{{{{ {}.{p} | tel }}}}", si::ROOT),
            _ => {
                // Adresse écrite : celle qui était produite.
                let cur = link_s.get_value().and_then(|h| link_of(&h));
                match cur.as_deref().and_then(|c| c.split_once(':')) {
                    Some(("mail", p)) => format!("mailto:{}", value_of(p)),
                    Some(("tel", p)) => format!("tel:{}", tel_digits(&value_of(p))),
                    Some((_, p)) => value_of(p),
                    None => "#".into(),
                }
            }
        };
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: "href".into(),
                value: Some(href),
            },
            false,
        );
    };
    let set_image = move |choice: String| {
        let cmd = ed.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            let Ok(Node::Image(mut img)) = revel_edit::address::node(d, &at.get_value()).cloned()
            else {
                return None;
            };
            img.src = if choice.is_empty() {
                revel_edit::canvas::quote(&value_of(
                    img.src.trim().strip_prefix("site.").unwrap_or_default(),
                ))
            } else {
                format!("{}.{choice}", si::ROOT)
            };
            Some(Command::SetImage {
                at: at.get_value(),
                image: img,
            })
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let link_cur = link.as_deref().and_then(link_of).unwrap_or_default();
    let (text_cur, before, after) = text
        .clone()
        .map(|(b, k, a)| (k.unwrap_or_default(), b, a))
        .unwrap_or_default();
    let image_cur = image.clone().flatten().unwrap_or_default();
    let has_text = text.is_some();
    let has_link = link.is_some();
    let has_image = image.is_some();
    let text_bound = !text_cur.is_empty();
    Some(
        view! {
            {move || {
                let es = entries();
                // Sans information du site : seulement l'année en cours, pour un texte.
                if es.is_empty() && !has_text && !text_bound && link_cur.is_empty() && image_cur.is_empty() {
                    return ().into_any();
                }
                let es2 = es.clone();
                let es3 = es.clone();
                let tc = text_cur.clone();
                let lc = link_cur.clone();
                let ic = image_cur.clone();
                view! {
                    <div class="insp-group" data-siteinfo="">
                        <p class="panel-title" title=ui.t("siteinfo-bind-help")>{ui.t("siteinfo-bind")}</p>
                        {has_text.then(|| view! {
                            <label class="field" data-siteinfo="text">
                                <span class="label">{ui.t("block-bound-text-label")}</span>
                                <div class="select">
                                    <select class="input" on:change=move |e| set_text(event_target_value(&e))>
                                        <option value="" prop:selected=tc.is_empty()>{ui.t("block-bound-written")}</option>
                                        {es.iter().map(|(k, l, _)| {
                                            let on = *k == tc;
                                            view! { <option value=k.clone() prop:selected=on>{l.clone()}</option> }
                                        }).collect_view()}
                                        <option value=YEAR prop:selected=tc == YEAR>{ui.t("siteinfo-year")}</option>
                                    </select>
                                </div>
                            </label>
                        })}
                        {(has_text && text_bound).then(|| view! {
                            <div class="gap-pair">
                                <label class="field">
                                    <span class="label">{ui.t("block-bound-before")}</span>
                                    <input class="input" prop:value=before.clone() on:change=move |e| set_affix(Some(event_target_value(&e)), None)/>
                                </label>
                                <label class="field">
                                    <span class="label">{ui.t("block-bound-after")}</span>
                                    <input class="input" prop:value=after.clone() on:change=move |e| set_affix(None, Some(event_target_value(&e)))/>
                                </label>
                            </div>
                        })}
                        {has_link.then(|| view! {
                            <label class="field" data-siteinfo="link">
                                <span class="label">{ui.t("block-bound-link")}</span>
                                <div class="select">
                                    <select class="input" on:change=move |e| set_link(event_target_value(&e))>
                                        <option value="" prop:selected=lc.is_empty()>{ui.t("siteinfo-link-written")}</option>
                                        {es2.iter().flat_map(|(k, l, v)| {
                                            let mut o = Vec::new();
                                            if looks_mail(v) { o.push((format!("mail:{k}"), ui.now("siteinfo-link-mail", &[("name", l.clone())]))); }
                                            if looks_phone(v) { o.push((format!("tel:{k}"), ui.now("siteinfo-link-tel", &[("name", l.clone())]))); }
                                            if looks_url(v) { o.push((format!("url:{k}"), l.clone())); }
                                            o
                                        }).map(|(val, l)| {
                                            let on = val == lc;
                                            view! { <option value=val prop:selected=on>{l}</option> }
                                        }).collect_view()}
                                    </select>
                                </div>
                            </label>
                        })}
                        {has_image.then(|| view! {
                            <label class="field" data-siteinfo="image">
                                <span class="label">{ui.t("block-bound-image")}</span>
                                <div class="select">
                                    <select class="input" on:change=move |e| set_image(event_target_value(&e))>
                                        <option value="" prop:selected=ic.is_empty()>{ui.t("siteinfo-image-written")}</option>
                                        {es3.iter().filter(|(_, _, v)| looks_image(v)).map(|(k, l, _)| {
                                            let on = *k == ic;
                                            view! { <option value=k.clone() prop:selected=on>{l.clone()}</option> }
                                        }).collect_view()}
                                    </select>
                                </div>
                            </label>
                        })}
                        <button type="button" class="link-btn" on:click=move |_| ui.tab(Tab::Settings(Section::SiteInfo))>
                            {ui.t("siteinfo-edit")}
                        </button>
                    </div>
                }.into_any()
            }}
        }
        .into_any(),
    )
}

/// Numéro d'un lien d'appel (comme le filtre `tel` du rendu).
fn tel_digits(v: &str) -> String {
    let t = v.replace("(0)", "");
    t.trim()
        .chars()
        .enumerate()
        .filter(|(i, c)| c.is_ascii_digit() || (*c == '+' && *i == 0))
        .map(|(_, c)| c)
        .collect()
}

/// Informations du site proposées dans les conditions (« Informations du site › Fax » rempli).
pub fn condition_fields(ed: Ed) -> Vec<(String, String)> {
    let Some(ui) = use_context::<Ui>() else {
        return Vec::new();
    };
    ed.doc.with_untracked(|d| {
        let Some(d) = d.as_ref() else {
            return Vec::new();
        };
        let Some(obj) = d.data.get(si::ROOT) else {
            return Vec::new();
        };
        let title = ui.now("siteinfo-title", &[]);
        si::values(obj)
            .into_iter()
            .map(|v| {
                (
                    format!("{}.{}", si::ROOT, v.path.join(".")),
                    format!("{title} › {}", label(ui, Some(d), &v.path)),
                )
            })
            .collect()
    })
}
