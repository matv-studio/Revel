//! Pages du site (D-28) : pages fixes (dossiers d'après leur adresse) et collections du CMS.
//!
//! Une collection est un dossier : son modèle (gabarit de la collection) se modifie en ouvrant
//! n'importe lequel de ses contenus ; replié, le dossier est mis en évidence et montre seulement
//! le contenu affiché. Un contenu peut recevoir un design propre : copie du modèle (nouveaux
//! identifiants) enregistrée comme gabarit du site et désignée par `layout` dans l'en-tête du
//! contenu ; les modifications du modèle ne s'y appliquent plus. « Revenir au modèle » retire la
//! clé et le gabarit (annulable). Un design propre dont le contenu a disparu est signalé (à
//! supprimer).
//!
//! Le même arbre sert au panneau Pages et, quand ce panneau est fermé, au menu des pages de la
//! barre d'outils.

use crate::editor::Ed;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::Command;
use revel_protocol::EditorPageInfo;
use serde_json::json;
use std::collections::BTreeSet;

/// Ouvre une page dans le canevas.
pub fn open_page(ed: Ed, url: String) {
    if ed.page.get_untracked() != url {
        ed.page.set(url);
        ed.scroll.set((0.0, 0.0));
        ed.selected.set(None);
    }
}

/// Titre d'une page ; sans titre : d'après son adresse (`/mentions-legales/` -> « Mentions
/// legales », `/` -> « Accueil »).
fn title_of(p: &EditorPageInfo) -> String {
    if !p.title.is_empty() {
        return p.title.clone();
    }
    let last = p
        .url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .trim_end_matches(".html");
    if last.is_empty() {
        return "/".into();
    }
    label_of(&last.replace(['-', '_'], " "))
}

fn matches(q: &str, texts: &[&str]) -> bool {
    let q = q.trim().to_lowercase();
    q.is_empty() || texts.iter().any(|t| t.to_lowercase().contains(&q))
}

/// Collection et ses contenus.
#[derive(Clone, PartialEq)]
pub struct CollectionInfo {
    pub id: String,
    pub folder: String,
    /// Gabarit de la collection (le modèle) ; `None` : contenus sans page.
    pub template: Option<String>,
    pub entries: Vec<EditorPageInfo>,
    /// Designs propres sans contenu (gabarits `<modèle>--…` que plus rien n'utilise).
    pub orphans: Vec<String>,
}

impl CollectionInfo {
    /// Le contenu a un design propre (gabarit autre que le modèle).
    pub fn own(&self, p: &EditorPageInfo) -> bool {
        self.template.is_some() && p.layout.is_some() && p.layout != self.template
    }
}

/// Collections du site, avec leurs contenus et les designs propres orphelins.
pub fn collections(ed: Ed) -> Vec<CollectionInfo> {
    ed.doc.with(|d| {
        let Some(d) = d.as_ref() else {
            return Vec::new();
        };
        ed.pages.with(|ps| {
            let used: BTreeSet<&str> = ps
                .iter()
                .filter_map(|p| p.layout.as_deref())
                .chain(d.layouts.iter().filter_map(|l| l.parent.as_deref()))
                .chain(d.collections.iter().filter_map(|c| c.layout.as_deref()))
                .chain(d.pages.iter().filter_map(|p| p.layout.as_deref()))
                .collect();
            d.collections
                .iter()
                .map(|c| {
                    let orphans = c
                        .layout
                        .as_ref()
                        .map(|t| {
                            let prefix = format!("{t}--");
                            d.layouts
                                .iter()
                                .filter(|l| {
                                    l.id.starts_with(&prefix) && !used.contains(l.id.as_str())
                                })
                                .map(|l| l.id.clone())
                                .collect()
                        })
                        .unwrap_or_default();
                    CollectionInfo {
                        id: c.id.clone(),
                        folder: c.folder.clone(),
                        template: c.layout.clone(),
                        entries: ps
                            .iter()
                            .filter(|p| p.collection.as_deref() == Some(c.id.as_str()))
                            .cloned()
                            .collect(),
                        orphans,
                    }
                })
                .filter(|c| !c.entries.is_empty() || !c.orphans.is_empty())
                .collect()
        })
    })
}

/// Libellé d'une collection (`projets` -> `Projets`).
fn label_of(id: &str) -> String {
    let mut c = id.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// Contenu affiché et sa collection.
pub fn current_entry(ed: Ed) -> Option<(CollectionInfo, EditorPageInfo)> {
    let url = ed.page.get();
    let p = ed
        .pages
        .with(|ps| ps.iter().find(|p| p.url == url).cloned())?;
    p.collection.as_ref()?;
    let c = collections(ed)
        .into_iter()
        .find(|c| Some(&c.id) == p.collection.as_ref())?;
    Some((c, p))
}

/// Crée le design propre d'un contenu : copie du modèle, clé `layout` de son en-tête ; si la
/// collection a un schéma Pages CMS qui ne garde pas les clés non déclarées, la clé y est
/// déclarée (champ caché), sinon Pages CMS l'effacerait à l'enregistrement du contenu.
pub fn detach(ui: Ui, ed: Ed, c: &CollectionInfo, p: &EditorPageInfo) {
    let (Some(file), Some(template)) = (p.file.clone(), c.template.clone()) else {
        return;
    };
    if c.own(p) {
        return;
    }
    let made = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let id = revel_edit::make::own_layout_id(d, &template, &file);
        revel_edit::make::layout_copy(d, &template, &id)
            .ok()
            .map(|l| (id, l))
    });
    let Some((id, layout)) = made else {
        return;
    };
    let folder = c.folder.clone();
    let path = ed.path.get_value();
    spawn_local(async move {
        let mut commands = vec![Command::AddLayout {
            layout,
            index: None,
        }];
        if let Ok(cols) = api::cms_collections(&path).await
            && let Some(col) = cols.iter().find(|x| x.folder == folder)
            && col.schema.is_some()
            && !col.merge
            && !col.fields.iter().any(|f| f.name == "layout")
        {
            commands.push(Command::AddCmsField {
                folder: folder.clone(),
                def: json!({
                    "name": "layout",
                    "label": ui.now("design-field-label", &[]),
                    "type": "string",
                    "hidden": true,
                }),
                index: None,
            });
        }
        commands.push(Command::SetEntryField {
            folder,
            file,
            key: "layout".into(),
            value: Some(json!(id)),
        });
        ed.apply(ui, Command::Batch { commands }, false);
    });
}

/// Retire le design propre d'un contenu (il reprend le modèle) ; le gabarit est supprimé s'il ne
/// sert à aucun autre contenu.
pub fn attach(ui: Ui, ed: Ed, c: &CollectionInfo, p: &EditorPageInfo) {
    let (Some(file), Some(own)) = (p.file.clone(), p.layout.clone()) else {
        return;
    };
    if !c.own(p) {
        return;
    }
    let mut commands = vec![Command::SetEntryField {
        folder: c.folder.clone(),
        file,
        key: "layout".into(),
        value: None,
    }];
    let shared = ed.pages.with_untracked(|ps| {
        ps.iter()
            .any(|x| x.url != p.url && x.layout.as_deref() == Some(own.as_str()))
    });
    let removable = ed.doc.with_untracked(|d| {
        d.as_ref().is_some_and(|d| {
            !d.layouts
                .iter()
                .any(|l| l.parent.as_deref() == Some(own.as_str()))
                && !d
                    .pages
                    .iter()
                    .any(|x| x.layout.as_deref() == Some(own.as_str()))
        })
    });
    if !shared && removable {
        commands.push(Command::RemoveLayout { id: own });
    }
    ed.apply(ui, Command::Batch { commands }, false);
}

/// Dossier de pages fixes (premier segment de l'adresse) et ses pages.
type Folder = (String, Vec<EditorPageInfo>);

/// Pages fixes : pages de premier niveau, puis dossiers (adresses à plusieurs segments).
fn fixed_tree(ps: &[EditorPageInfo], q: &str) -> (Vec<EditorPageInfo>, Vec<Folder>) {
    let mut top = Vec::new();
    let mut folders: Vec<Folder> = Vec::new();
    for p in ps
        .iter()
        .filter(|p| p.page.is_some())
        .filter(|p| matches(q, &[&p.title, &p.url]))
    {
        let segs: Vec<&str> = p.url.split('/').filter(|s| !s.is_empty()).collect();
        if segs.len() >= 2 {
            match folders.iter_mut().find(|(f, _)| f == segs[0]) {
                Some((_, v)) => v.push(p.clone()),
                None => folders.push((segs[0].to_string(), vec![p.clone()])),
            }
        } else {
            top.push(p.clone());
        }
    }
    // Page d'index d'un dossier (`/legal/`) : rangée avec lui, en premier.
    top.retain(|p| {
        let seg = p.url.trim_matches('/').to_string();
        match folders.iter_mut().find(|(f, _)| *f == seg) {
            Some((_, v)) => {
                v.insert(0, p.clone());
                false
            }
            None => true,
        }
    });
    folders.sort_by(|a, b| a.0.cmp(&b.0));
    (top, folders)
}

/// Arbre des pages. `on_pick` : appelé après l'ouverture d'une page (menu de la barre d'outils).
#[component]
pub fn PageTree(
    q: RwSignal<String>,
    #[prop(optional)] on_pick: Option<Callback<()>>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    // Dossiers dépliés (pages fixes : `p:<segment>`, collections : `c:<id>`).
    let open = RwSignal::new(Vec::<String>::new());
    let toggle = move |k: String| {
        open.update(|o| {
            if let Some(i) = o.iter().position(|x| *x == k) {
                o.remove(i);
            } else {
                o.push(k);
            }
        })
    };
    let go = move |url: String| {
        open_page(ed, url);
        if let Some(cb) = on_pick {
            cb.run(());
        }
    };
    let page_menu = move |e: &web_sys::MouseEvent, url: String| {
        use crate::ctxmenu::{CtxItem, open as show};
        let u2 = url.clone();
        show(
            ui,
            e,
            vec![
                CtxItem::new(ui.now("library-open", &[]), move || go(url.clone()))
                    .icon(icons::page),
                CtxItem::new(ui.now("media-copy-url", &[]), move || api::copy_text(&u2))
                    .icon(icons::link),
            ],
        );
    };
    let entry_menu = move |e: &web_sys::MouseEvent, c: CollectionInfo, p: EditorPageInfo| {
        use crate::ctxmenu::{CtxItem, open as show};
        let url = p.url.clone();
        let own = c.own(&p);
        let (c2, p2) = (c.clone(), p.clone());
        let mut items = vec![
            CtxItem::new(ui.now("library-open", &[]), move || go(url.clone())).icon(icons::text),
        ];
        if c.template.is_some() {
            items.push(if own {
                CtxItem::new(ui.now("design-attach", &[]), move || {
                    attach(ui, ed, &c2, &p2)
                })
                .icon(icons::reset)
                .sep()
            } else {
                CtxItem::new(ui.now("design-detach", &[]), move || {
                    detach(ui, ed, &c2, &p2)
                })
                .icon(icons::layout)
                .sep()
            });
        }
        show(ui, e, items);
    };
    let orphan_menu = move |e: &web_sys::MouseEvent, id: String| {
        use crate::ctxmenu::{CtxItem, open as show};
        show(
            ui,
            e,
            vec![
                CtxItem::new(ui.now("design-orphan-delete", &[]), move || {
                    ed.apply(ui, Command::RemoveLayout { id: id.clone() }, false)
                })
                .icon(icons::trash)
                .danger(),
            ],
        );
    };
    let page_row = move |p: EditorPageInfo, depth: usize| {
        let url = p.url.clone();
        let (u1, u2, u3) = (url.clone(), url.clone(), url.clone());
        view! {
            <button class="list-row page-row" class:is-on=move || ed.page.get() == u1
                style=format!("--depth:{depth}") title=p.url.clone() data-url=p.url.clone()
                on:click=move |_| go(u2.clone())
                on:contextmenu=move |e: leptos::ev::MouseEvent| page_menu(&e, u3.clone())>
                <span class="list-chevron"></span>
                <span class="list-icon">{icons::page()}</span>
                <span class="list-label">{title_of(&p)}</span>
                <span class="list-meta">{p.url.clone()}</span>
            </button>
        }
    };
    let entry_row = move |c: CollectionInfo, p: EditorPageInfo| {
        let url = p.url.clone();
        let (u1, u2) = (url.clone(), url.clone());
        let own = c.own(&p);
        let (c2, p2) = (c.clone(), p.clone());
        view! {
            <button class="list-row page-row entry-row" class:is-on=move || ed.page.get() == u1
                style="--depth:1" title=p.url.clone() data-url=p.url.clone()
                on:click=move |_| go(u2.clone())
                on:contextmenu=move |e: leptos::ev::MouseEvent| entry_menu(&e, c2.clone(), p2.clone())>
                <span class="list-chevron"></span>
                <span class="list-icon">{icons::text()}</span>
                <span class="list-label">{title_of(&p)}</span>
                {own.then(|| view! { <span class="list-badge" title=ui.t("design-own-help")>{icons::layout()}</span> })}
            </button>
        }
    };
    let fixed = Memo::new(move |_| {
        let q = q.get();
        ed.pages.with(|ps| fixed_tree(ps, &q))
    });
    // Libellés des collections (`.pages.yml`), par dossier.
    let labels = RwSignal::new(std::collections::BTreeMap::<String, String>::new());
    let path = ed.path.get_value();
    spawn_local(async move {
        if let Ok(cols) = api::cms_collections(&path).await {
            labels.set(
                cols.into_iter()
                    .filter_map(|c| Some((c.folder, c.label?)))
                    .collect(),
            );
        }
    });
    let cols = Memo::new(move |_| collections(ed));
    view! {
        <div class="page-tree" role="tree">
            {move || {
                let (top, folders) = fixed.get();
                view! {
                    {top.into_iter().map(|p| page_row(p, 0)).collect_view()}
                    {folders.into_iter().map(|(f, pages)| {
                        let key = format!("p:{f}");
                        let k1 = key.clone();
                        let is_open = Memo::new(move |_| !q.with(|q| q.trim().is_empty()) || open.with(|o| o.contains(&k1)));
                        let urls: Vec<String> = pages.iter().map(|p| p.url.clone()).collect();
                        let has_current = Memo::new(move |_| { let u = ed.page.get(); urls.contains(&u) });
                        let n = pages.len();
                        view! {
                            <button class="list-row list-folder" style="--depth:0" class:is-current=move || has_current.get() && !is_open.get()
                                data-folder=f.clone() on:click=move |_| toggle(key.clone())>
                                <span class="list-chevron">{move || if is_open.get() { icons::chevron_down() } else { icons::chevron_right() }}</span>
                                <span class="list-icon">{icons::folder()}</span>
                                <span class="list-label">{f.clone()}</span>
                                <span class="list-count">{n}</span>
                            </button>
                            {move || {
                                let shown: Vec<EditorPageInfo> = if is_open.get() {
                                    pages.clone()
                                } else {
                                    let u = ed.page.get();
                                    pages.iter().filter(|p| p.url == u).cloned().collect()
                                };
                                shown.into_iter().map(|p| page_row(p, 1)).collect_view()
                            }}
                        }
                    }).collect_view()}
                }
            }}
            {move || cols.get().into_iter().map(|c| {
                let key = format!("c:{}", c.id);
                let k1 = key.clone();
                let ql = q.get();
                let entries: Vec<EditorPageInfo> = c.entries.iter()
                    .filter(|p| matches(&ql, &[&p.title, &p.url, &c.id]))
                    .cloned()
                    .collect();
                if entries.is_empty() && !ql.trim().is_empty() {
                    return ().into_any();
                }
                let searching = !ql.trim().is_empty();
                let is_open = Memo::new(move |_| searching || open.with(|o| o.contains(&k1)));
                let id = c.id.clone();
                let has_current = Memo::new(move |_| current_entry(ed).is_some_and(|(cc, _)| cc.id == id));
                let n = c.entries.len();
                let c2 = c.clone();
                let first = c.entries.iter().find(|p| !c.own(p)).or(c.entries.first()).map(|p| p.url.clone());
                let template_tip = match &c.template {
                    Some(t) => ui.now("design-template-help", &[("layout", t.clone())]),
                    None => ui.now("design-no-template", &[]),
                };
                view! {
                    <div class="list-row list-folder collection-row" style="--depth:0" role="treeitem"
                        class:is-current=move || has_current.get()
                        data-collection=c.id.clone() title=template_tip
                        on:click=move |_| {
                            // Le modèle s'ouvre sur le contenu affiché, sinon le premier qui l'utilise.
                            if !has_current.get_untracked() && let Some(u) = first.clone() { go(u) }
                        }>
                        <button class="list-chevron" tabindex="-1" aria-label=ui.t("pages-toggle")
                            on:click=move |e| { e.stop_propagation(); toggle(key.clone()) }>
                            {move || if is_open.get() { icons::chevron_down() } else { icons::chevron_right() }}
                        </button>
                        <span class="list-icon">{icons::database()}</span>
                        <span class="list-label">{
                            let (id, folder) = (c.id.clone(), c.folder.clone());
                            move || labels.with(|l| l.get(&folder).cloned()).unwrap_or_else(|| label_of(&id))
                        }</span>
                        <span class="list-count">{n}</span>
                    </div>
                    {move || {
                        let shown: Vec<EditorPageInfo> = if is_open.get() {
                            entries.clone()
                        } else {
                            // Replié : seulement le contenu affiché.
                            let u = ed.page.get();
                            entries.iter().filter(|p| p.url == u).cloned().collect()
                        };
                        let c3 = c2.clone();
                        view! {
                            {shown.into_iter().map(|p| entry_row(c3.clone(), p)).collect_view()}
                            {is_open.get().then(|| c3.orphans.iter().map(|o| {
                                let o2 = o.clone();
                                view! {
                                    <div class="list-row entry-row is-orphan" style="--depth:1" title=ui.t("design-orphan-help")
                                        on:contextmenu=move |e: leptos::ev::MouseEvent| orphan_menu(&e, o2.clone())>
                                        <span class="list-chevron"></span>
                                        <span class="list-icon">{icons::warning()}</span>
                                        <span class="list-label">{o.rsplit("--").next().unwrap_or(o).to_string()}</span>
                                    </div>
                                }
                            }).collect_view())}
                        }
                    }}
                }.into_any()
            }).collect_view()}
        </div>
    }
}

/// Panneau Pages.
#[component]
pub fn Pages() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let q = RwSignal::new(String::new());
    view! {
        <div class="panel-list pages">
            <crate::panels::Search value=q key="pages-search"/>
            <div class="list-scroll"><PageTree q=q/></div>
            <footer class="panel-foot">
                <span class="panel-foot-text muted">{move || ui.tx("pages-count", &[("count", ed.pages.with(|p| p.iter().filter(|p| p.page.is_some()).count()).to_string())])}</span>
                <crate::panels::PlannedButton key="pages-new" step=3 icon=icons::plus/>
            </footer>
        </div>
    }
}

/// Barre d'outils : page affichée (menu des pages quand le panneau Pages n'est pas visible) et,
/// pour un contenu de collection, modèle ou design propre.
#[component]
pub fn PageContext() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let dock = expect_context::<crate::dock::Dock>();
    let panel_shown = move || {
        dock.layout
            .with(|l| !l.hidden && l.group_of(crate::dock::PanelId::Pages).is_some())
    };
    let q = RwSignal::new(String::new());
    let menu = NodeRef::<leptos::html::Details>::new();
    let close = Callback::new(move |_| {
        if let Some(d) = menu.get_untracked() {
            d.set_open(false);
        }
    });
    let current = move || {
        let u = ed.page.get();
        ed.pages
            .with(|ps| ps.iter().find(|p| p.url == u).map(title_of))
            .unwrap_or(u)
    };
    let entry = Memo::new(move |_| current_entry(ed));
    view! {
        <Show when=move || !panel_shown()>
            <details class="bp-menu page-pick" node_ref=menu>
                <summary class="page-pick-btn" title=ui.t("editor-page") aria-label=ui.t("editor-page")>
                    <span class="list-icon">{move || if entry.get().is_some() { icons::text() } else { icons::page() }}</span>
                    <span class="page-pick-label">{current}</span>
                    {icons::chevron_down()}
                </summary>
                <div class="bp-pop page-pop">
                    <crate::panels::Search value=q key="pages-search"/>
                    <div class="page-pop-list"><PageTree q=q on_pick=close/></div>
                </div>
            </details>
        </Show>
        {move || entry.get().filter(|(c, _)| c.template.is_some()).map(|(c, p)| {
            let own = c.own(&p);
            let (c2, p2) = (c.clone(), p.clone());
            let label = label_of(&c.id);
            view! {
                <div class="design-chip" class:is-own=own
                    title=if own { ui.now("design-own-help", &[]) } else { ui.now("design-template-help", &[("layout", c.template.clone().unwrap_or_default())]) }>
                    <span class="design-chip-icon">{if own { icons::layout() } else { icons::database() }}</span>
                    <span class="design-chip-label">{if own { ui.now("design-own", &[]) } else { ui.now("design-template", &[("collection", label)]) }}</span>
                    {if own {
                        view! {
                            <button class="icon-btn icon-btn--small" title=ui.t("design-attach") aria-label=ui.t("design-attach")
                                on:click=move |_| attach(ui, ed, &c2, &p2)>{icons::reset()}</button>
                        }.into_any()
                    } else {
                        view! {
                            <button class="icon-btn icon-btn--small" title=ui.t("design-detach-help") aria-label=ui.t("design-detach")
                                on:click=move |_| detach(ui, ed, &c2, &p2)>{icons::layout()}</button>
                        }.into_any()
                    }}
                </div>
            }
        })}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(url: &str, fixed: bool) -> EditorPageInfo {
        EditorPageInfo {
            url: url.into(),
            title: String::new(),
            page: fixed.then(|| url.into()),
            collection: (!fixed).then(|| "projets".into()),
            layout: None,
            file: None,
        }
    }

    #[test]
    fn pages_rangees_par_dossier() {
        let ps = vec![
            page("/", true),
            page("/agence/", true),
            page("/legal/", true),
            page("/legal/mentions/", true),
            page("/legal/cgv/", true),
            page("/projet/a/", false),
        ];
        let (top, folders) = fixed_tree(&ps, "");
        let urls: Vec<&str> = top.iter().map(|p| p.url.as_str()).collect();
        assert_eq!(urls, vec!["/", "/agence/"]);
        assert_eq!(folders.len(), 1);
        let (f, inside) = &folders[0];
        assert_eq!(f, "legal");
        let inside: Vec<&str> = inside.iter().map(|p| p.url.as_str()).collect();
        assert_eq!(inside, vec!["/legal/", "/legal/mentions/", "/legal/cgv/"]);
        let (top, folders) = fixed_tree(&ps, "cgv");
        assert!(top.is_empty() && folders[0].1.len() == 1);
        assert_eq!(label_of("projets"), "Projets");
    }
}
