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
pub(crate) use revel_protocol::EditorPageInfo;
use serde_json::json;
use std::collections::BTreeSet;

/// Ouvre une page dans le canevas.
pub fn open_page(ed: Ed, url: String) {
    ed.close_comp();
    if ed.page.get_untracked() != url {
        ed.page.set(url);
        ed.scroll.set((0.0, 0.0));
        ed.selected.set(None);
    }
}

/// Titre d'une page ; sans titre : d'après son adresse (`/mentions-legales/` -> « Mentions
/// legales », `/` -> « Accueil »).
pub(crate) fn title_of(p: &EditorPageInfo) -> String {
    if let Some(n) = p.name.as_ref().filter(|n| !n.trim().is_empty()) {
        return n.clone();
    }
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
pub(crate) fn label_of(id: &str) -> String {
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

/// Dossier de pages fixes : chemin d'adresse (`""` : racine), sous-dossiers, pages (la page
/// d'index du dossier, `/legal/`, en premier).
#[derive(Clone, Debug, PartialEq, Default)]
struct FolderNode {
    path: String,
    folders: Vec<FolderNode>,
    pages: Vec<EditorPageInfo>,
}

impl FolderNode {
    fn count(&self) -> usize {
        self.pages.len() + self.folders.iter().map(FolderNode::count).sum::<usize>()
    }
    fn contains(&self, url: &str) -> bool {
        self.pages.iter().any(|p| p.url == url) || self.folders.iter().any(|f| f.contains(url))
    }
}

/// Arbre des pages fixes : dossiers du rangement et des adresses ; recherche : dossiers sans
/// page trouvée retirés.
fn page_tree(ps: &[EditorPageInfo], folders: &[String], q: &str) -> FolderNode {
    let searching = !q.trim().is_empty();
    let mut placed: Vec<(String, bool, EditorPageInfo)> = Vec::new();
    for p in ps.iter().filter(|p| p.page.is_some()).filter(|p| {
        matches(
            q,
            &[&p.title, &p.url, p.name.as_deref().unwrap_or_default()],
        )
    }) {
        let own = p.url.trim_matches('/').to_string();
        let index = p.url != "/" && p.url.ends_with('/') && folders.contains(&own);
        let dir = if index {
            own
        } else {
            crate::pageops::folder_of(&p.url)
        };
        placed.push((dir, index, p.clone()));
    }
    fn build(
        path: &str,
        folders: &[String],
        placed: &[(String, bool, EditorPageInfo)],
        searching: bool,
    ) -> FolderNode {
        let mut pages: Vec<EditorPageInfo> = placed
            .iter()
            .filter(|(d, i, _)| d == path && *i)
            .map(|(_, _, p)| p.clone())
            .collect();
        pages.extend(
            placed
                .iter()
                .filter(|(d, i, _)| d == path && !*i)
                .map(|(_, _, p)| p.clone()),
        );
        let mut subs: Vec<FolderNode> = folders
            .iter()
            .filter(|f| crate::pageops::folder_of(&format!("/{f}/")) == path)
            .map(|f| build(f, folders, placed, searching))
            .filter(|f| !searching || f.count() > 0)
            .collect();
        subs.sort_by(|a, b| a.path.cmp(&b.path));
        FolderNode {
            path: path.to_string(),
            folders: subs,
            pages,
        }
    }
    build("", folders, &placed, searching)
}

/// État partagé de l'arbre (dossiers dépliés, saisies en cours, actions du panneau).
#[derive(Clone, Copy)]
struct TreeCtx {
    ui: Ui,
    ed: Ed,
    open: RwSignal<Vec<String>>,
    q: RwSignal<String>,
    on_pick: Option<Callback<()>>,
    on_settings: Option<Callback<String>>,
    /// Dossier où une nouvelle page ou un nouveau dossier est en cours de saisie.
    creating_page: RwSignal<Option<String>>,
    creating_folder: RwSignal<Option<String>>,
    renaming: RwSignal<Option<String>>,
    /// Glisser pour ranger l'affichage : dossier parent et clé de la ligne glissée.
    dragging: RwSignal<Option<(String, String)>>,
    /// Ligne survolée pendant le glisser et côté (`true` : après).
    drop_at: RwSignal<Option<(String, bool)>>,
}

/// Clés de l'ordre d'affichage choisi (rangement des pages de l'éditeur).
fn page_key(p: &EditorPageInfo) -> String {
    format!("page:{}", p.page.as_deref().unwrap_or(&p.url))
}
fn folder_key(path: &str) -> String {
    format!("folder:{path}")
}
fn collection_key(id: &str) -> String {
    format!("collection:{id}")
}

/// Ligne de l'arbre rangeable (page ou dossier ; collection à la racine).
#[derive(Clone)]
enum Child {
    Page(EditorPageInfo),
    Folder(FolderNode),
    Collection(CollectionInfo),
}

impl Child {
    fn key(&self) -> String {
        match self {
            Child::Page(p) => page_key(p),
            Child::Folder(f) => folder_key(&f.path),
            Child::Collection(c) => collection_key(&c.id),
        }
    }
}

/// Ordre d'affichage : clés rangées d'abord (dans l'ordre choisi), les autres à la suite dans
/// l'ordre habituel.
fn arrange(ed: Ed, mut items: Vec<Child>) -> Vec<Child> {
    ed.doc.with(|d| {
        if let Some(d) = d {
            let shelf = &d.library.pages;
            items.sort_by_key(|c| shelf.rank(&c.key()).map_or((1, 0), |r| (0, r)));
        }
    });
    items
}

impl TreeCtx {
    /// Rangement par glisser possible (pas pendant une recherche).
    fn can_drag(self) -> bool {
        self.q.with_untracked(|q| q.trim().is_empty())
    }

    fn drag_start(self, parent: String, key: String, e: &leptos::ev::DragEvent) {
        if !self.can_drag() {
            e.prevent_default();
            return;
        }
        if let Some(dt) = e.data_transfer() {
            let _ = dt.set_data("text/plain", &key);
            dt.set_effect_allowed("move");
        }
        self.dragging.set(Some((parent, key)));
    }

    fn drag_over(self, parent: &str, key: &str, e: &leptos::ev::DragEvent) {
        let ok = self
            .dragging
            .with_untracked(|d| d.as_ref().is_some_and(|(p, k)| p == parent && k != key));
        if !ok {
            return;
        }
        e.prevent_default();
        e.stop_propagation();
        let after = e
            .current_target()
            .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok())
            .map(|el| {
                let r = el.get_bounding_client_rect();
                f64::from(e.client_y()) > r.top() + r.height() / 2.0
            })
            .unwrap_or(false);
        let next = Some((key.to_string(), after));
        if self.drop_at.get_untracked() != next {
            self.drop_at.set(next);
        }
    }

    fn drag_end(self) {
        self.dragging.set(None);
        self.drop_at.set(None);
    }

    /// Dépôt : `siblings` (clés des lignes du même dossier, dans l'ordre affiché) rangées à nouveau.
    fn drop_on(self, key: &str, siblings: &[String], e: &leptos::ev::DragEvent) {
        e.prevent_default();
        let src = self.dragging.get_untracked().map(|(_, k)| k);
        let after = self.drop_at.get_untracked().is_some_and(|(_, a)| a);
        self.drag_end();
        let Some(src) = src.filter(|s| s != key) else {
            return;
        };
        let mut keys: Vec<String> = siblings.iter().filter(|k| **k != src).cloned().collect();
        let Some(at) = keys.iter().position(|k| k == key) else {
            return;
        };
        keys.insert(if after { at + 1 } else { at }, src);
        let Some(mut shelf) = self
            .ed
            .doc
            .with_untracked(|d| d.as_ref().map(|d| d.library.pages.clone()))
        else {
            return;
        };
        let before = shelf.clone();
        shelf.reorder(&keys);
        if shelf != before {
            self.ed.apply(
                self.ui,
                Command::SetShelf {
                    kind: revel_edit::ShelfKind::Pages,
                    shelf,
                },
                false,
            );
        }
    }

    fn is_dragged(self, key: String) -> impl Fn() -> bool + Copy + 'static {
        let key = StoredValue::new(key);
        move || {
            self.dragging
                .with(|d| d.as_ref().is_some_and(|(_, k)| key.with_value(|x| x == k)))
        }
    }

    fn is_drop(self, key: String, after: bool) -> impl Fn() -> bool + Copy + 'static {
        let key = StoredValue::new(key);
        move || {
            self.drop_at.with(|d| {
                d.as_ref()
                    .is_some_and(|(k, a)| key.with_value(|x| x == k) && *a == after)
            })
        }
    }

    /// Lignes d'un dossier (ou de la racine), dans l'ordre d'affichage choisi.
    fn children(
        self,
        parent: &str,
        items: Vec<Child>,
        depth: usize,
        collection: impl Fn(CollectionInfo, String, StoredValue<Vec<String>>) -> AnyView,
    ) -> AnyView {
        let items = arrange(self.ed, items);
        let siblings = StoredValue::new(items.iter().map(Child::key).collect::<Vec<_>>());
        let parent = parent.to_string();
        items
            .into_iter()
            .map(|c| match c {
                Child::Page(p) => self.page_row(p, depth, parent.clone(), siblings),
                Child::Folder(f) => self.folder(f, depth, parent.clone(), siblings),
                Child::Collection(c) => collection(c, parent.clone(), siblings),
            })
            .collect_view()
            .into_any()
    }

    fn toggle(self, k: String) {
        self.open.update(|o| {
            if let Some(i) = o.iter().position(|x| *x == k) {
                o.remove(i);
            } else {
                o.push(k);
            }
        })
    }

    fn go(self, url: String) {
        open_page(self.ed, url);
        if let Some(cb) = self.on_pick {
            cb.run(());
        }
    }

    fn page_menu(self, e: &web_sys::MouseEvent, p: &EditorPageInfo) {
        use crate::ctxmenu::{CtxItem, open as show};
        let (ui, ed) = (self.ui, self.ed);
        let Some(id) = p.page.clone() else { return };
        let home = p.url == "/";
        let url = p.url.clone();
        let mut items = vec![
            CtxItem::new(ui.now("library-open", &[]), {
                let u = url.clone();
                move || self.go(u.clone())
            })
            .icon(icons::page),
        ];
        if let Some(cb) = self.on_settings {
            let i = id.clone();
            items.push(
                CtxItem::new(ui.now("pages-settings", &[]), move || cb.run(i.clone()))
                    .icon(icons::settings),
            );
        }
        items.push(
            CtxItem::new(ui.now("pages-duplicate", &[]), {
                let i = id.clone();
                move || crate::pageops::duplicate(ui, ed, &i)
            })
            .icon(icons::copy)
            .sep(),
        );
        if !home {
            let i = id.clone();
            let draft = p.draft;
            items.push(
                CtxItem::new(
                    ui.now(
                        if draft {
                            "pages-publish"
                        } else {
                            "pages-to-draft"
                        },
                        &[],
                    ),
                    move || crate::pageops::set_draft(ui, ed, &i, !draft),
                )
                .icon(icons::draft),
            );
            let here = crate::pageops::folder_of(&p.url);
            let folders = ed
                .doc
                .with_untracked(|d| d.as_ref().map(crate::pageops::folders))
                .unwrap_or_default();
            for (n, f) in folders
                .into_iter()
                .filter(|f| *f != here && *f != url.trim_matches('/'))
                .take(10)
                .enumerate()
            {
                let i = id.clone();
                let label = ui.now("library-move-to", &[("folder", f.replace('/', " › "))]);
                let mut c = CtxItem::new(label, move || crate::pageops::move_to(ui, ed, &i, &f))
                    .icon(icons::folder);
                if n == 0 {
                    c = c.sep();
                }
                items.push(c);
            }
            if !here.is_empty() {
                let i = id.clone();
                items.push(
                    CtxItem::new(ui.now("pages-move-root", &[]), move || {
                        crate::pageops::move_to(ui, ed, &i, "")
                    })
                    .icon(icons::arrow_up),
                );
            }
        }
        items.push(
            CtxItem::new(ui.now("media-copy-url", &[]), {
                let u = url.clone();
                move || api::copy_text(&u)
            })
            .icon(icons::link)
            .sep(),
        );
        if !home {
            let i = id.clone();
            items.push(
                CtxItem::new(ui.now("pages-delete", &[]), move || {
                    crate::pageops::delete(ui, ed, &i)
                })
                .icon(icons::trash)
                .danger()
                .sep(),
            );
        }
        show(ui, e, items);
    }

    fn folder_menu(self, e: &web_sys::MouseEvent, node: &FolderNode) {
        use crate::ctxmenu::{CtxItem, open as show};
        let (ui, ed) = (self.ui, self.ed);
        let path = node.path.clone();
        let empty = node.count() == 0;
        let (p1, p2, p3, p4) = (path.clone(), path.clone(), path.clone(), path.clone());
        let items = vec![
            CtxItem::new(ui.now("pages-new-here", &[]), move || {
                self.open.update(|o| {
                    if !o.contains(&format!("p:{p1}")) {
                        o.push(format!("p:{p1}"))
                    }
                });
                self.creating_page.set(Some(p1.clone()))
            })
            .icon(icons::plus),
            CtxItem::new(ui.now("library-new-subfolder", &[]), move || {
                self.creating_folder.set(Some(p2.clone()))
            })
            .icon(icons::folder_plus),
            CtxItem::new(ui.now("library-rename", &[]), move || {
                self.renaming.set(Some(p3.clone()))
            })
            .icon(icons::edit)
            .sep(),
            CtxItem::new(
                ui.now(
                    if empty {
                        "library-delete-folder"
                    } else {
                        "pages-folder-not-empty"
                    },
                    &[],
                ),
                move || crate::pageops::delete_folder(ui, ed, &p4),
            )
            .icon(icons::trash)
            .danger()
            .disabled(!empty)
            .sep(),
        ];
        show(ui, e, items);
    }

    fn page_row(
        self,
        p: EditorPageInfo,
        depth: usize,
        parent: String,
        siblings: StoredValue<Vec<String>>,
    ) -> AnyView {
        let (ui, ed) = (self.ui, self.ed);
        let key = page_key(&p);
        let (k1, k2, k3) = (key.clone(), key.clone(), key.clone());
        let (before, after) = (
            self.is_drop(key.clone(), false),
            self.is_drop(key.clone(), true),
        );
        let dragged = self.is_dragged(key);
        let par = parent.clone();
        let url = p.url.clone();
        let (u1, u2) = (url.clone(), url.clone());
        let p2 = p.clone();
        let id = p.page.clone().unwrap_or_default();
        let settings = self.on_settings.map(|cb| {
            view! {
                <span class="row-action" role="button" title=ui.t("pages-settings") aria-label=ui.t("pages-settings")
                    on:click=move |e| { e.stop_propagation(); cb.run(id.clone()) }>{icons::settings()}</span>
            }
        });
        view! {
            <button class="list-row page-row" class:is-on=move || ed.page.get() == u1 class:is-draft=p.draft
                class:drop-before=before class:drop-after=after class:is-dragging=dragged
                style=format!("--depth:{depth}") title=p.url.clone() data-url=p.url.clone()
                draggable="true"
                on:dragstart=move |e| self.drag_start(parent.clone(), k1.clone(), &e)
                on:dragover=move |e| self.drag_over(&par, &k2, &e)
                on:drop=move |e| siblings.with_value(|s| self.drop_on(&k3, s, &e))
                on:dragend=move |_| self.drag_end()
                on:click=move |_| self.go(u2.clone())
                on:contextmenu=move |e: leptos::ev::MouseEvent| self.page_menu(&e, &p2)>
                <span class="list-chevron"></span>
                <span class="list-icon">{if p.url == "/" { icons::grid() } else { icons::page() }}</span>
                <span class="list-label">{title_of(&p)}</span>
                {p.draft.then(|| view! { <span class="list-tag">{ui.t("pages-draft-tag")}</span> })}
                <span class="list-meta">{p.url.clone()}</span>
                {settings}
            </button>
        }
        .into_any()
    }

    fn inputs(self, folder: &str, depth: usize) -> AnyView {
        let f1 = folder.to_string();
        let f2 = folder.to_string();
        view! {
            {move || self.creating_folder.get().filter(|p| *p == f1).map(|p| {
                let parent = StoredValue::new(p);
                crate::library::input_row(self.ui, icons::folder(), depth, "library-folder-name",
                    move |name: String| crate::pageops::create_folder(self.ui, self.ed, &parent.get_value(), &name)
                        .then(|| self.creating_folder.set(None)).is_some(),
                    move || self.creating_folder.set(None))
            })}
            {move || self.creating_page.get().filter(|p| *p == f2).map(|p| {
                let parent = StoredValue::new(p);
                crate::library::input_row(self.ui, icons::page(), depth, "pages-name",
                    move |name: String| crate::pageops::create(self.ui, self.ed, &name, &parent.get_value())
                        .then(|| self.creating_page.set(None)).is_some(),
                    move || self.creating_page.set(None))
            })}
        }
        .into_any()
    }

    fn folder(
        self,
        node: FolderNode,
        depth: usize,
        parent: String,
        siblings: StoredValue<Vec<String>>,
    ) -> AnyView {
        let (ui, ed) = (self.ui, self.ed);
        let dkey = folder_key(&node.path);
        let (d1, d2, d3) = (dkey.clone(), dkey.clone(), dkey.clone());
        let (before, after) = (
            self.is_drop(dkey.clone(), false),
            self.is_drop(dkey.clone(), true),
        );
        let dragged = self.is_dragged(dkey);
        let (par1, par2) = (parent.clone(), parent);
        let key = format!("p:{}", node.path);
        let k1 = key.clone();
        let q = self.q;
        let is_open = Memo::new(move |_| {
            !q.with(|q| q.trim().is_empty()) || self.open.with(|o| o.contains(&k1))
        });
        let n2 = node.clone();
        let has_current = Memo::new(move |_| {
            let u = ed.page.get();
            n2.contains(&u)
        });
        let n = node.count();
        let path = StoredValue::new(node.path.clone());
        let name = node.path.rsplit('/').next().unwrap_or_default().to_string();
        let node_menu = node.clone();
        let renaming = move || self.renaming.get().as_deref() == Some(path.get_value().as_str());
        let row = view! {
            <div class="list-row list-folder" role="treeitem" tabindex="0" style=format!("--depth:{depth}")
                class:is-current=move || has_current.get() && !is_open.get()
                class:drop-before=before class:drop-after=after class:is-dragging=dragged
                data-folder=node.path.clone()
                draggable=move || (!renaming()).to_string()
                on:dragstart=move |e| self.drag_start(par1.clone(), d1.clone(), &e)
                on:dragover=move |e| self.drag_over(&par2, &d2, &e)
                on:drop=move |e| siblings.with_value(|s| self.drop_on(&d3, s, &e))
                on:dragend=move |_| self.drag_end()
                on:click=move |_| if !renaming() { self.toggle(key.clone()) }
                on:contextmenu=move |e: leptos::ev::MouseEvent| self.folder_menu(&e, &node_menu)>
                <span class="list-chevron">{move || if is_open.get() { icons::chevron_down() } else { icons::chevron_right() }}</span>
                <span class="list-icon">{icons::folder()}</span>
                {move || if renaming() {
                    crate::library::folder_name_input(ui, name.clone(),
                        move |v: String| crate::pageops::rename_folder(ui, ed, &path.get_value(), &v)
                            .then(|| self.renaming.set(None)).is_some(),
                        move || self.renaming.set(None)).into_any()
                } else {
                    view! { <span class="list-label">{name.clone()}</span> }.into_any()
                }}
                <span class="list-count">{n}</span>
            </div>
        };
        let inside = node.clone();
        view! {
            {row}
            {move || {
                if is_open.get() {
                    let f = inside.clone();
                    view! {
                        {self.inputs(&f.path, depth + 1)}
                        {self.children(&f.path,
                            f.folders.into_iter().map(Child::Folder).chain(f.pages.into_iter().map(Child::Page)).collect(),
                            depth + 1, |_, _, _| ().into_any())}
                    }.into_any()
                } else {
                    // Replié : seulement la page affichée.
                    let u = ed.page.get();
                    let mut cur = Vec::new();
                    fn find(f: &FolderNode, u: &str, out: &mut Vec<EditorPageInfo>) {
                        out.extend(f.pages.iter().filter(|p| p.url == u).cloned());
                        for s in &f.folders { find(s, u, out); }
                    }
                    find(&inside, &u, &mut cur);
                    let none = StoredValue::new(Vec::new());
                    cur.into_iter().map(|p| self.page_row(p, depth + 1, String::new(), none)).collect_view().into_any()
                }
            }}
        }
        .into_any()
    }
}

/// Arbre des pages. `on_pick` : appelé après l'ouverture d'une page (menu de la barre d'outils) ;
/// `on_settings` : paramètres d'une page (panneau Pages).
#[component]
pub fn PageTree(
    q: RwSignal<String>,
    #[prop(optional)] on_pick: Option<Callback<()>>,
    #[prop(optional)] on_settings: Option<Callback<String>>,
    #[prop(optional)] creating_page: Option<RwSignal<Option<String>>>,
    #[prop(optional)] creating_folder: Option<RwSignal<Option<String>>>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    // Dossiers dépliés (pages fixes : `p:<chemin>`, collections : `c:<id>`).
    let open = RwSignal::new(Vec::<String>::new());
    let tc = TreeCtx {
        ui,
        ed,
        open,
        q,
        on_pick,
        on_settings,
        creating_page: creating_page.unwrap_or_else(|| RwSignal::new(None)),
        creating_folder: creating_folder.unwrap_or_else(|| RwSignal::new(None)),
        renaming: RwSignal::new(None),
        dragging: RwSignal::new(None),
        drop_at: RwSignal::new(None),
    };
    let go = move |url: String| tc.go(url);
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
        .into_any()
    };
    let fixed = Memo::new(move |_| {
        let q = q.get();
        let folders = ed
            .doc
            .with(|d| d.as_ref().map(crate::pageops::folders))
            .unwrap_or_default();
        ed.pages.with(|ps| page_tree(ps, &folders, &q))
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
    // Collections sans page par contenu : proposées en bas de l'arbre, avec la création de
    // leurs pages (modèle commun, comme la page de collection de Webflow).
    let pageless = Memo::new(move |_| {
        ed.doc.with(|d| {
            d.as_ref()
                .map(|d| {
                    d.collections
                        .iter()
                        .filter(|c| {
                            c.path.is_none() && c.source.is_folder() && !c.folder.is_empty()
                        })
                        .map(|c| (c.id.clone(), c.folder.clone()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });
    let create_pages = move |id: String, folder: String| {
        let base = folder.rsplit('/').next().unwrap_or(&id).to_string();
        let Some(layout) = ed.doc.with_untracked(|d| {
            d.as_ref()
                .map(|d| revel_edit::make::collection_template(d, &id, "title"))
        }) else {
            return;
        };
        let lid = layout.id.clone();
        ed.apply(
            ui,
            Command::Batch {
                commands: vec![
                    Command::AddLayout {
                        layout,
                        index: None,
                    },
                    Command::SetCollectionPages {
                        id,
                        path: Some(format!("/{base}/{{{{ slug }}}}/")),
                        layout: Some(lid),
                    },
                ],
            },
            false,
        );
    };
    let collection_row =
        move |c: CollectionInfo, parent: String, siblings: StoredValue<Vec<String>>| {
            collection_view(tc, c, parent, siblings, labels, entry_row, orphan_menu)
        };
    view! {
        <div class="page-tree" role="tree">
            {move || tc.inputs("", 0)}
            {move || {
                // Racine : pages, dossiers et collections, dans l'ordre d'affichage choisi.
                let root = fixed.get();
                let items: Vec<Child> = root.pages.into_iter().map(Child::Page)
                    .chain(root.folders.into_iter().map(Child::Folder))
                    .chain(cols.get().into_iter().map(Child::Collection))
                    .collect();
                tc.children("", items, 0, collection_row)
            }}
            {move || pageless.get().into_iter().map(|(id, folder)| {
                let (id2, f2) = (id.clone(), folder.clone());
                let name = labels.with(|l| l.get(&folder).cloned()).unwrap_or_else(|| label_of(&id));
                view! {
                    <div class="list-row collection-row is-pageless" style="--depth:0" role="treeitem"
                        data-collection=id.clone() title=ui.t("pages-collection-none")>
                        <span class="list-chevron"></span>
                        <span class="list-icon">{icons::database()}</span>
                        <span class="list-label">{name}</span>
                        <button class="icon-btn icon-btn--small list-action" title=ui.t("pages-collection-create")
                            aria-label=ui.t("pages-collection-create")
                            on:click=move |_| create_pages(id2.clone(), f2.clone())>{icons::plus()}</button>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

/// Ligne d'une collection et ses contenus (racine de l'arbre des pages).
#[allow(clippy::too_many_arguments)]
fn collection_view(
    tc: TreeCtx,
    c: CollectionInfo,
    parent: String,
    siblings: StoredValue<Vec<String>>,
    labels: RwSignal<std::collections::BTreeMap<String, String>>,
    entry_row: impl Fn(CollectionInfo, EditorPageInfo) -> AnyView + Copy + Send + Sync + 'static,
    orphan_menu: impl Fn(&web_sys::MouseEvent, String) + Copy + Send + Sync + 'static,
) -> AnyView {
    let (ui, ed, q, open) = (tc.ui, tc.ed, tc.q, tc.open);
    let go = move |url: String| tc.go(url);
    let toggle = move |k: String| tc.toggle(k);
    let ckey = collection_key(&c.id);
    let (d1, d2, d3) = (ckey.clone(), ckey.clone(), ckey.clone());
    let (before, after) = (
        tc.is_drop(ckey.clone(), false),
        tc.is_drop(ckey.clone(), true),
    );
    let dragged = tc.is_dragged(ckey);
    let (par1, par2) = (parent.clone(), parent);
    {
        {
            let key = format!("c:{}", c.id);
            let k1 = key.clone();
            let ql = q.get();
            let entries: Vec<EditorPageInfo> = c
                .entries
                .iter()
                .filter(|p| matches(&ql, &[&p.title, &p.url, &c.id]))
                .cloned()
                .collect();
            if entries.is_empty() && !ql.trim().is_empty() {
                return ().into_any();
            }
            let searching = !ql.trim().is_empty();
            let is_open = Memo::new(move |_| searching || open.with(|o| o.contains(&k1)));
            let id = c.id.clone();
            let has_current =
                Memo::new(move |_| current_entry(ed).is_some_and(|(cc, _)| cc.id == id));
            let n = c.entries.len();
            let c2 = c.clone();
            let first = c
                .entries
                .iter()
                .find(|p| !c.own(p))
                .or(c.entries.first())
                .map(|p| p.url.clone());
            let template_tip = match &c.template {
                Some(t) => ui.now("design-template-help", &[("layout", t.clone())]),
                None => ui.now("design-no-template", &[]),
            };
            view! {
                    <div class="list-row list-folder collection-row" style="--depth:0" role="treeitem"
                        class:is-current=move || has_current.get()
                        class:drop-before=before class:drop-after=after class:is-dragging=dragged
                        data-collection=c.id.clone() title=template_tip
                        draggable="true"
                        on:dragstart=move |e| tc.drag_start(par1.clone(), d1.clone(), &e)
                        on:dragover=move |e| tc.drag_over(&par2, &d2, &e)
                        on:drop=move |e| siblings.with_value(|s| tc.drop_on(&d3, s, &e))
                        on:dragend=move |_| tc.drag_end()
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
                        // Réglages du modèle des pages (apparence, partage), comme ceux d'une page.
                        {tc.on_settings.zip(c.template.clone()).map(|(cb, t)| view! {
                            <span class="row-action" role="button" title=ui.t("template-settings") aria-label=ui.t("template-settings")
                                on:click=move |e| { e.stop_propagation(); cb.run(format!("layout:{t}")) }>{icons::settings()}</span>
                        })}
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
        }
    }
}

/// Panneau Pages : arbre des pages, ou paramètres d'une page.
#[component]
pub fn Pages() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let q = RwSignal::new(String::new());
    let settings = RwSignal::new(None::<String>);
    let creating_page = RwSignal::new(None::<String>);
    let creating_folder = RwSignal::new(None::<String>);
    // Paramètres d'une page : par-dessus toute la colonne (ou la fenêtre) du panneau, pour avoir
    // la hauteur nécessaire (comme Webflow).
    let root = NodeRef::<leptos::html::Div>::new();
    let rect = RwSignal::new(None::<(f64, f64, f64, f64)>);
    let measure = move || {
        let r = root.get_untracked().and_then(|el| {
            let host = el.closest(".dock-col, .dock-float").ok().flatten()?;
            let b = host.get_bounding_client_rect();
            Some((b.left(), b.top(), b.width(), b.height()))
        });
        rect.set(r);
    };
    let on_resize = window_event_listener(leptos::ev::resize, move |_| {
        if settings.get_untracked().is_some() {
            measure()
        }
    });
    on_cleanup(move || on_resize.remove());
    let on_settings = Callback::new(move |id: String| {
        measure();
        settings.set(Some(id))
    });
    let close = Callback::new(move |_| settings.set(None));
    view! {
        <div class="panel-list pages" node_ref=root>
            {move || settings.get().map(|id| view! {
                <div class="page-settings-overlay" style=move || rect.get().map(|(x, y, w, h)| format!("left:{x}px;top:{y}px;width:{w}px;height:{h}px")).unwrap_or_default()>
                    {match id.strip_prefix("layout:") {
                        Some(l) => view! { <crate::pagesettings::TemplateSettings layout=l.to_string() on_close=close/> }.into_any(),
                        None => view! { <crate::pagesettings::PageSettings id=id on_close=close/> }.into_any(),
                    }}
                </div>
            })}
            {move || match settings.get() {
                Some(_) => ().into_any(),
                None => view! {
                    <crate::panels::Search value=q key="pages-search"/>
                    <div class="list-scroll">
                        <PageTree q=q on_settings=on_settings creating_page=creating_page creating_folder=creating_folder/>
                    </div>
                    <footer class="panel-foot">
                        <span class="panel-foot-text muted">{move || ui.tx("pages-count", &[("count", ed.pages.with(|p| p.iter().filter(|p| p.page.is_some()).count()).to_string())])}</span>
                        <button class="icon-btn icon-btn--small" title=ui.t("library-new-folder") aria-label=ui.t("library-new-folder")
                            on:click=move |_| { q.set(String::new()); creating_folder.set(Some(String::new())) }>{icons::folder_plus()}</button>
                        <button class="icon-btn icon-btn--small" title=ui.t("pages-new") aria-label=ui.t("pages-new")
                            on:click=move |_| { q.set(String::new()); creating_page.set(Some(String::new())) }>{icons::plus()}</button>
                    </footer>
                }.into_any(),
            }}
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
    // Libellés des collections (`.pages.yml`, « Projets »), par dossier.
    let labels = RwSignal::new(std::collections::BTreeMap::<String, String>::new());
    let path = ed.path.get_value();
    spawn_local(async move {
        if let Ok(cols) = api::cms_collections(&path).await {
            let _ = labels.try_set(
                cols.into_iter()
                    .filter_map(|c| Some((c.folder, c.label?)))
                    .collect(),
            );
        }
    });
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
            let label = labels.with(|l| l.get(&c.folder).cloned()).unwrap_or_else(|| label_of(&c.id));
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
            draft: false,
            name: None,
            seo: None,
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
            page("/emplois/2026/stage/", true),
            page("/projet/a/", false),
        ];
        let folders = vec![
            "emplois".to_string(),
            "emplois/2026".to_string(),
            "legal".to_string(),
            "vide".to_string(),
        ];
        let t = page_tree(&ps, &folders, "");
        let urls: Vec<&str> = t.pages.iter().map(|p| p.url.as_str()).collect();
        assert_eq!(urls, vec!["/", "/agence/"]);
        let names: Vec<&str> = t.folders.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(names, vec!["emplois", "legal", "vide"]);
        let legal: Vec<&str> = t.folders[1].pages.iter().map(|p| p.url.as_str()).collect();
        assert_eq!(legal, vec!["/legal/", "/legal/mentions/", "/legal/cgv/"]);
        assert_eq!(t.folders[0].folders[0].pages[0].url, "/emplois/2026/stage/");
        assert_eq!(t.folders[0].count(), 1);
        // Recherche : dossiers sans résultat retirés.
        let t = page_tree(&ps, &folders, "cgv");
        assert!(t.pages.is_empty() && t.folders.len() == 1 && t.folders[0].pages.len() == 1);
        assert_eq!(label_of("projets"), "Projets");
    }
}
