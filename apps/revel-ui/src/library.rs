//! Médias et composants rangés en dossiers (rangement de l'éditeur, `Site::library`) : les
//! dossiers ne déplacent aucun fichier et ne changent aucune adresse ; ils s'enregistrent dans le
//! document (commande `SetShelf`, annulable).
//!
//! - Médias : fichiers des dossiers copiés tels quels, filtrés par type, triés (récents, nom,
//!   type, taille), en grille ; on entre dans un dossier (fil d'Ariane).
//! - Composants : arbre de dossiers repliables, nombre d'utilisations.
//!
//! Un élément se range en le glissant sur un dossier (ou le fil d'Ariane), ou par son menu
//! contextuel ; un dossier se crée, se renomme, se supprime (ses éléments remontent d'un niveau).

use crate::editor::Ed;
use crate::panels::{PlannedButton, Search};
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::{Command, ShelfKind};
use revel_model::{Node, Shelf};
use revel_protocol::{MediaFile, MediaKind};
use std::collections::BTreeMap;

// ------------------------------------------------------------------ dossiers (sans interface)

/// Nom de dossier accepté (sans `/`, 60 caractères au plus), espaces de bord retirés.
pub fn valid_name(name: &str) -> Option<String> {
    let n = name.trim();
    (!n.is_empty() && n.chars().count() <= 60 && !n.contains('/') && n != "." && n != "..")
        .then(|| n.to_string())
}

fn join(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

pub fn parent_of(path: &str) -> &str {
    path.rsplit_once('/').map(|(p, _)| p).unwrap_or("")
}

pub fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// `path` est `folder` ou l'un de ses sous-dossiers.
fn within(path: &str, folder: &str) -> bool {
    path == folder
        || path
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Sous-dossiers directs de `parent` (`""` : racine), par nom.
pub fn children(s: &Shelf, parent: &str) -> Vec<String> {
    let mut v: Vec<String> = s
        .folders
        .iter()
        .filter(|f| parent_of(f) == parent)
        .cloned()
        .collect();
    v.sort_by_key(|f| name_of(f).to_lowercase());
    v
}

/// Dossier d'un élément (`""` : racine, aussi quand son dossier n'existe plus).
pub fn folder_of<'a>(s: &'a Shelf, item: &str) -> &'a str {
    s.items
        .get(item)
        .filter(|f| s.folders.contains(f))
        .map(String::as_str)
        .unwrap_or("")
}

/// Nouveau dossier `name` dans `parent` ; `None` : nom refusé ou déjà pris.
pub fn create(s: &Shelf, parent: &str, name: &str) -> Option<(Shelf, String)> {
    let path = join(parent, &valid_name(name)?);
    if s.folders.iter().any(|f| f.eq_ignore_ascii_case(&path))
        || (!parent.is_empty() && !s.folders.iter().any(|f| f == parent))
    {
        return None;
    }
    let mut t = s.clone();
    t.folders.push(path.clone());
    Some((t, path))
}

/// Dossier renommé (sous-dossiers et éléments suivent) ; `None` : nom refusé, pris ou inchangé.
pub fn rename(s: &Shelf, path: &str, name: &str) -> Option<(Shelf, String)> {
    let new = join(parent_of(path), &valid_name(name)?);
    if new == path
        || s.folders
            .iter()
            .any(|f| f != path && f.eq_ignore_ascii_case(&new))
    {
        return None;
    }
    let moved = |f: &str| {
        if within(f, path) {
            format!("{new}{}", &f[path.len()..])
        } else {
            f.to_string()
        }
    };
    let t = Shelf {
        folders: s.folders.iter().map(|f| moved(f)).collect(),
        items: s.items.iter().map(|(k, f)| (k.clone(), moved(f))).collect(),
    };
    Some((t, new))
}

/// Dossier supprimé avec ses sous-dossiers ; leurs éléments remontent dans le dossier parent.
pub fn remove(s: &Shelf, path: &str) -> Shelf {
    let up = parent_of(path);
    Shelf {
        folders: s
            .folders
            .iter()
            .filter(|f| !within(f, path))
            .cloned()
            .collect(),
        items: s
            .items
            .iter()
            .filter_map(|(k, f)| {
                let f = if within(f, path) { up } else { f.as_str() };
                (!f.is_empty()).then(|| (k.clone(), f.to_string()))
            })
            .collect(),
    }
}

/// Élément rangé dans `folder` (`""` : racine).
pub fn put(s: &Shelf, item: &str, folder: &str) -> Shelf {
    let mut t = s.clone();
    if folder.is_empty() {
        t.items.remove(item);
    } else {
        t.items.insert(item.to_string(), folder.to_string());
    }
    t
}

// ------------------------------------------------------------------ interface commune

/// État partagé d'un panneau rangé en dossiers.
#[derive(Clone, Copy)]
struct Shelves {
    ui: Ui,
    ed: Ed,
    kind: ShelfKind,
    shelf: Memo<Shelf>,
    /// Dossier en cours de renommage.
    renaming: RwSignal<Option<String>>,
    /// Dossier parent où un nouveau dossier est en cours de saisie.
    creating: RwSignal<Option<String>>,
    /// Élément glissé, dossier survolé.
    dragging: RwSignal<Option<String>>,
    drop_on: RwSignal<Option<String>>,
}

impl Shelves {
    fn new(ui: Ui, ed: Ed, kind: ShelfKind) -> Self {
        let shelf = Memo::new(move |_| {
            ed.doc.with(|d| {
                d.as_ref()
                    .map(|d| match kind {
                        ShelfKind::Media => d.library.media.clone(),
                        ShelfKind::Components => d.library.components.clone(),
                        ShelfKind::Pages => d.library.pages.clone(),
                    })
                    .unwrap_or_default()
            })
        });
        Self {
            ui,
            ed,
            kind,
            shelf,
            renaming: RwSignal::new(None),
            creating: RwSignal::new(None),
            dragging: RwSignal::new(None),
            drop_on: RwSignal::new(None),
        }
    }

    fn set(self, shelf: Shelf) {
        if self.shelf.with_untracked(|s| *s != shelf) {
            self.ed.apply(
                self.ui,
                Command::SetShelf {
                    kind: self.kind,
                    shelf,
                },
                false,
            );
        }
    }

    fn put(self, item: &str, folder: &str) {
        let s = self.shelf.with_untracked(|s| put(s, item, folder));
        self.set(s);
    }

    /// Menu d'un dossier : ouvrir, nouveau sous-dossier, renommer, supprimer.
    fn folder_menu(
        self,
        e: &web_sys::MouseEvent,
        path: String,
        open: impl Fn() + Send + Sync + 'static,
    ) {
        use crate::ctxmenu::{CtxItem, open as show};
        let ui = self.ui;
        let (p1, p2, p3) = (path.clone(), path.clone(), path);
        let items = vec![
            CtxItem::new(ui.now("library-open", &[]), open).icon(icons::folder),
            CtxItem::new(ui.now("library-new-subfolder", &[]), move || {
                self.creating.set(Some(p1.clone()))
            })
            .icon(icons::folder_plus),
            CtxItem::new(ui.now("library-rename", &[]), move || {
                self.renaming.set(Some(p2.clone()))
            })
            .icon(icons::edit)
            .sep(),
            CtxItem::new(ui.now("library-delete-folder", &[]), move || {
                let s = self.shelf.with_untracked(|s| remove(s, &p3));
                self.set(s);
            })
            .icon(icons::trash)
            .danger()
            .sep(),
        ];
        show(ui, e, items);
    }

    /// Lignes « Déplacer dans … » d'un élément (dossiers autres que le sien) et « Retirer du
    /// dossier ».
    fn move_items(self, item: &str) -> Vec<crate::ctxmenu::CtxItem> {
        use crate::ctxmenu::CtxItem;
        let ui = self.ui;
        let (folders, cur) = self.shelf.with_untracked(|s| {
            let mut f = s.folders.clone();
            f.sort_by_key(|x| x.to_lowercase());
            (f, folder_of(s, item).to_string())
        });
        let mut out = Vec::new();
        for (i, f) in folders
            .into_iter()
            .filter(|f| *f != cur)
            .take(12)
            .enumerate()
        {
            let it = item.to_string();
            let label = ui.now("library-move-to", &[("folder", f.replace('/', " › "))]);
            let mut c = CtxItem::new(label, move || self.put(&it, &f)).icon(icons::folder);
            if i == 0 {
                c = c.sep();
            }
            out.push(c);
        }
        if !cur.is_empty() {
            let it = item.to_string();
            out.push(
                CtxItem::new(ui.now("library-move-out", &[]), move || self.put(&it, ""))
                    .icon(icons::arrow_up),
            );
        }
        out
    }

    /// Cible de dépôt (dossier ou racine) : survol et dépôt d'un élément glissé.
    fn drop_zone(self, folder: String) -> impl Fn(web_sys::DragEvent, bool) + Copy + 'static {
        let folder = StoredValue::new(folder);
        move |e: web_sys::DragEvent, dropped: bool| {
            let Some(item) = self.dragging.get_untracked() else {
                return;
            };
            e.prevent_default();
            let f = folder.get_value();
            if dropped {
                self.drop_on.set(None);
                self.dragging.set(None);
                self.put(&item, &f);
            } else if self.drop_on.get_untracked().as_deref() != Some(f.as_str()) {
                self.drop_on.set(Some(f));
            }
        }
    }
}

/// Champ du nom d'un dossier (création, renommage) : Entrée valide, Échap annule ; refusé
/// (nom pris, invalide) : le champ reste ouvert, marqué.
fn folder_input(
    ui: Ui,
    initial: String,
    commit: impl Fn(String) -> bool + Copy + 'static,
    cancel: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let value = RwSignal::new(initial);
    let bad = RwSignal::new(false);
    let node = NodeRef::<leptos::html::Input>::new();
    Effect::new(move |_| {
        if let Some(el) = node.get() {
            let _ = el.focus();
            el.select();
        }
    });
    // Saisie terminée : la sortie du champ retiré ne la traite pas une seconde fois.
    let closed = StoredValue::new(false);
    let finish = move |ok: bool| {
        if closed.try_get_value().unwrap_or(true) {
            return;
        }
        let v = value.get_untracked();
        if !ok || v.trim().is_empty() {
            closed.set_value(true);
            cancel();
        } else if commit(v) {
            closed.set_value(true);
        } else {
            bad.set(true);
        }
    };
    view! {
        <input class="folder-input" class:is-bad=move || bad.get() node_ref=node
            aria-label=ui.t("library-folder-name") placeholder=ui.t("library-folder-name")
            prop:value=move || value.get()
            on:input=move |e| { value.set(event_target_value(&e)); bad.set(false); }
            on:click=|e| e.stop_propagation()
            on:keydown=move |e: leptos::ev::KeyboardEvent| {
                e.stop_propagation();
                match e.key().as_str() {
                    "Enter" => { e.prevent_default(); finish(true) }
                    "Escape" => { e.prevent_default(); finish(false) }
                    _ => {}
                }
            }
            // Sortie du champ : nom valide appliqué, sinon abandon.
            on:blur=move |_| finish(!bad.try_get_untracked().unwrap_or(true))/>
    }
}

/// Ligne de saisie d'un nouveau dossier dans `parent`.
fn new_folder_row(sh: Shelves, parent: String, depth: usize) -> impl IntoView {
    let p = StoredValue::new(parent);
    let commit = move |name: String| match sh
        .shelf
        .with_untracked(|s| create(s, &p.get_value(), &name))
    {
        Some((s, _)) => {
            sh.creating.set(None);
            sh.set(s);
            true
        }
        None => false,
    };
    view! {
        <div class="list-row list-folder is-editing" style=format!("--depth:{depth}")>
            <span class="list-chevron"></span>
            <span class="list-icon">{icons::folder()}</span>
            {folder_input(sh.ui, String::new(), commit, move || sh.creating.set(None))}
        </div>
    }
}

/// Ligne d'un dossier : ouverture, renommage sur place, menu, dépôt.
fn folder_row(
    sh: Shelves,
    path: String,
    depth: usize,
    count: usize,
    chevron: Option<Signal<bool>>,
    open: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let path = StoredValue::new(path);
    let drop = sh.drop_zone(path.get_value());
    let renaming = move || sh.renaming.get().as_deref() == Some(path.get_value().as_str());
    let commit = move |name: String| match sh
        .shelf
        .with_untracked(|s| rename(s, &path.get_value(), &name))
    {
        Some((s, _)) => {
            sh.renaming.set(None);
            sh.set(s);
            true
        }
        None => name.trim() == name_of(&path.get_value()),
    };
    view! {
        <div class="list-row list-folder" role="button" tabindex="0"
            style=format!("--depth:{depth}")
            data-folder=path.get_value()
            title=path.get_value()
            class:is-drop=move || sh.drop_on.get().as_deref() == Some(path.get_value().as_str())
            on:click=move |_| if !renaming() { open() }
            on:keydown=move |e: leptos::ev::KeyboardEvent| if e.key() == "Enter" && !renaming() { open() }
            on:contextmenu=move |e: leptos::ev::MouseEvent| sh.folder_menu(&e, path.get_value(), open)
            on:dragover=move |e: leptos::ev::DragEvent| drop(e, false)
            on:dragleave=move |_| sh.drop_on.set(None)
            on:drop=move |e: leptos::ev::DragEvent| drop(e, true)>
            <span class="list-chevron">{chevron.map(|c| move || if c.get() { icons::chevron_down() } else { icons::chevron_right() })}</span>
            <span class="list-icon">{icons::folder()}</span>
            {move || if renaming() {
                folder_input(sh.ui, name_of(&path.get_value()).to_string(), commit, move || sh.renaming.set(None)).into_any()
            } else {
                view! { <span class="list-label">{name_of(&path.get_value()).to_string()}</span> }.into_any()
            }}
            <span class="list-count">{count}</span>
        </div>
    }
}

// ------------------------------------------------------------------ médias

/// Ordre des médias.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MediaSort {
    Recent,
    Oldest,
    Name,
    NameDesc,
    Kind,
    Size,
}

impl MediaSort {
    const ALL: [MediaSort; 6] = [
        MediaSort::Recent,
        MediaSort::Oldest,
        MediaSort::Name,
        MediaSort::NameDesc,
        MediaSort::Kind,
        MediaSort::Size,
    ];
    fn id(self) -> &'static str {
        match self {
            MediaSort::Recent => "recent",
            MediaSort::Oldest => "oldest",
            MediaSort::Name => "name",
            MediaSort::NameDesc => "name-desc",
            MediaSort::Kind => "type",
            MediaSort::Size => "size",
        }
    }
    fn parse(s: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|m| m.id() == s)
            .unwrap_or(MediaSort::Recent)
    }
}

const KINDS: [MediaKind; 7] = [
    MediaKind::Image,
    MediaKind::Video,
    MediaKind::Audio,
    MediaKind::Document,
    MediaKind::Font,
    MediaKind::Animation,
    MediaKind::Other,
];

fn kind_id(k: MediaKind) -> &'static str {
    match k {
        MediaKind::Image => "image",
        MediaKind::Video => "video",
        MediaKind::Audio => "audio",
        MediaKind::Document => "document",
        MediaKind::Font => "font",
        MediaKind::Animation => "animation",
        MediaKind::Other => "other",
    }
}

fn kind_icon(k: MediaKind) -> AnyView {
    match k {
        MediaKind::Image => icons::image(),
        MediaKind::Video => icons::video(),
        MediaKind::Audio => icons::audio(),
        MediaKind::Document => icons::file(),
        MediaKind::Font => icons::font(),
        MediaKind::Animation => icons::play(),
        MediaKind::Other => icons::box_(),
    }
}

fn file_name(url: &str) -> String {
    url.rsplit('/').next().unwrap_or(url).to_string()
}

/// Taille lisible (`12 Ko`, `3,4 Mo`).
fn human_size(ui: Ui, n: u64) -> String {
    let (v, unit) = if n >= 1_000_000 {
        (n as f64 / 1_000_000.0, "size-mb")
    } else {
        ((n as f64 / 1000.0).max(1.0), "size-kb")
    };
    let num = if v >= 10.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    };
    let num = if ui.lang() == "fr" {
        num.replace('.', ",")
    } else {
        num
    };
    ui.now(unit, &[("n", num)])
}

/// Largeur des vignettes de médias (px).
const THUMB_MIN: u32 = 44;
const THUMB_MAX: u32 = 160;

/// Médias du site, rangés en dossiers.
#[component]
pub fn Media() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let sh = Shelves::new(ui, ed, ShelfKind::Media);
    let files = RwSignal::new(Vec::<MediaFile>::new());
    let loaded = RwSignal::new(false);
    let q = RwSignal::new(String::new());
    let folder = RwSignal::new(String::new());
    let kind = RwSignal::new(None::<MediaKind>);
    let sort = RwSignal::new(MediaSort::parse(
        &crate::pref_get("media.sort").unwrap_or_default(),
    ));
    // Taille des vignettes (px), gardée d'une session à l'autre.
    let thumb = RwSignal::new(
        crate::pref_get("media.thumb")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(72)
            .clamp(THUMB_MIN, THUMB_MAX),
    );
    // Liste relue à l'ouverture du panneau et après une modification de fichiers hors de Revel.
    let path = ed.path.get_value();
    Effect::new(move |_| {
        ed.load.track();
        let path = path.clone();
        spawn_local(async move {
            if let Ok(m) = api::edit_media(&path).await
                && files.with_untracked(|f| *f != m)
            {
                files.set(m);
            }
            loaded.set(true);
        });
    });
    // Dossier affiché supprimé ou renommé : retour au plus proche qui existe.
    Effect::new(move |_| {
        let cur = folder.get();
        if cur.is_empty() {
            return;
        }
        sh.shelf.with(|s| {
            let mut f = cur.as_str();
            while !f.is_empty() && !s.folders.iter().any(|x| x == f) {
                f = parent_of(f);
            }
            if f != cur {
                folder.set(f.to_string());
            }
        });
    });
    let searching = move || !q.with(|q| q.trim().is_empty());
    let shown = Memo::new(move |_| {
        let ql = q.get().trim().to_lowercase();
        let cur = folder.get();
        let k = kind.get();
        let mut v: Vec<MediaFile> = sh.shelf.with(|s| {
            files.with(|fs| {
                fs.iter()
                    .filter(|f| k.is_none_or(|k| f.kind == k))
                    .filter(|f| {
                        if ql.is_empty() {
                            folder_of(s, &f.url) == cur
                        } else {
                            file_name(&f.url).to_lowercase().contains(&ql)
                        }
                    })
                    .cloned()
                    .collect()
            })
        });
        let name = |f: &MediaFile| file_name(&f.url).to_lowercase();
        match sort.get() {
            MediaSort::Recent => {
                v.sort_by(|a, b| b.modified.cmp(&a.modified).then(name(a).cmp(&name(b))))
            }
            MediaSort::Oldest => {
                v.sort_by(|a, b| a.modified.cmp(&b.modified).then(name(a).cmp(&name(b))))
            }
            MediaSort::Name => v.sort_by_key(name),
            MediaSort::NameDesc => v.sort_by_key(|f| std::cmp::Reverse(name(f))),
            MediaSort::Kind => v.sort_by(|a, b| a.kind.cmp(&b.kind).then(name(a).cmp(&name(b)))),
            MediaSort::Size => v.sort_by_key(|f| std::cmp::Reverse(f.size)),
        }
        v
    });
    // Nombre de médias de chaque dossier (sous-dossiers compris), au filtre de type près.
    let counts = Memo::new(move |_| {
        let k = kind.get();
        sh.shelf.with(|s| {
            let mut m: BTreeMap<String, usize> = BTreeMap::new();
            files.with(|fs| {
                for f in fs.iter().filter(|f| k.is_none_or(|k| f.kind == k)) {
                    let mut d = folder_of(s, &f.url);
                    while !d.is_empty() {
                        *m.entry(d.to_string()).or_default() += 1;
                        d = parent_of(d);
                    }
                }
            });
            m
        })
    });
    let base = move || ed.base.get().unwrap_or_default();
    let tile = move |f: MediaFile| {
        let url = StoredValue::new(f.url.clone());
        let name = file_name(&f.url);
        let ext = name
            .rsplit_once('.')
            .map(|(_, e)| e.to_uppercase())
            .unwrap_or_default();
        let tip = format!("{}\n{}", f.url, human_size(ui, f.size));
        let src = if f.url.starts_with("http") {
            f.url.clone()
        } else {
            format!("{}{}", base(), f.url.trim_start_matches('/'))
        };
        let menu = move |e: leptos::ev::MouseEvent| {
            use crate::ctxmenu::{CtxItem, open};
            let u = url.get_value();
            let mut items = vec![
                CtxItem::new(ui.now("media-copy-url", &[]), move || api::copy_text(&u))
                    .icon(icons::link),
            ];
            items.extend(sh.move_items(&url.get_value()));
            open(ui, &e, items);
        };
        view! {
            <figure class="asset" title=tip draggable="true" data-url=f.url.clone()
                class:is-dragging=move || sh.dragging.get().as_deref() == Some(url.get_value().as_str())
                on:contextmenu=menu
                on:dragstart=move |e: leptos::ev::DragEvent| {
                    if let Some(dt) = e.data_transfer() {
                        let _ = dt.set_data("text/plain", &url.get_value());
                        dt.set_effect_allowed("move");
                    }
                    sh.dragging.set(Some(url.get_value()));
                }
                on:dragend=move |_| { sh.dragging.set(None); sh.drop_on.set(None); }>
                <div class="asset-thumb" class:is-file=f.kind != MediaKind::Image>
                    {if f.kind == MediaKind::Image {
                        view! { <img src=src alt="" loading="lazy" decoding="async" draggable="false"/> }.into_any()
                    } else {
                        view! { <span class="asset-icon">{kind_icon(f.kind)}<span class="asset-ext">{ext}</span></span> }.into_any()
                    }}
                </div>
                <figcaption>{name}</figcaption>
            </figure>
        }
    };
    let crumbs = move || {
        let cur = folder.get();
        let mut parts: Vec<String> = Vec::new();
        let mut acc = String::new();
        for p in cur.split('/').filter(|p| !p.is_empty()) {
            acc = join(&acc, p);
            parts.push(acc.clone());
        }
        let crumb = move |target: String, label: String, last: bool| {
            let drop = sh.drop_zone(target.clone());
            let t2 = target.clone();
            view! {
                <button class="media-crumb" class:is-last=last
                    class:is-drop=move || sh.drop_on.get().as_deref() == Some(t2.as_str())
                    on:click=move |_| folder.set(target.clone())
                    on:dragover=move |e: leptos::ev::DragEvent| drop(e, false)
                    on:dragleave=move |_| sh.drop_on.set(None)
                    on:drop=move |e: leptos::ev::DragEvent| drop(e, true)>{label}</button>
            }
        };
        (!cur.is_empty()).then(|| {
            let n = parts.len();
            view! {
                <nav class="media-crumbs" aria-label=ui.t("panel-assets")>
                    {crumb(String::new(), ui.now("panel-assets", &[]), false)}
                    {parts.into_iter().enumerate().map(|(i, p)| {
                        let label = name_of(&p).to_string();
                        view! { <span class="media-crumb-sep">"›"</span>{crumb(p, label, i + 1 == n)} }
                    }).collect_view()}
                </nav>
            }
        })
    };
    view! {
        <div class="panel-list media">
            <Search value=q key="assets-search"/>
            <div class="media-tools">
                <label class="mini-select" title=ui.t("media-type-filter")>
                    {move || kind.get().map(kind_icon).unwrap_or_else(icons::grid)}
                    <select aria-label=ui.t("media-type-filter")
                        on:change=move |e| {
                            let v = event_target_value(&e);
                            kind.set(KINDS.into_iter().find(|k| kind_id(*k) == v));
                        }>
                        <option value="" selected=move || kind.get().is_none()>{ui.t("media-all-types")}</option>
                        {KINDS.into_iter().map(|k| view! {
                            <option value=kind_id(k) selected=move || kind.get() == Some(k)>{ui.t(match k {
                                MediaKind::Image => "media-kind-image",
                                MediaKind::Video => "media-kind-video",
                                MediaKind::Audio => "media-kind-audio",
                                MediaKind::Document => "media-kind-document",
                                MediaKind::Font => "media-kind-font",
                                MediaKind::Animation => "media-kind-animation",
                                MediaKind::Other => "media-kind-other",
                            })}</option>
                        }).collect_view()}
                    </select>
                </label>
                <label class="mini-select" title=ui.t("media-sort")>
                    {icons::sort()}
                    <select aria-label=ui.t("media-sort")
                        on:change=move |e| {
                            let s = MediaSort::parse(&event_target_value(&e));
                            crate::pref_set("media.sort", s.id());
                            sort.set(s);
                        }>
                        {MediaSort::ALL.into_iter().map(|m| view! {
                            <option value=m.id() selected=move || sort.get() == m>{ui.t(match m {
                                MediaSort::Recent => "media-sort-recent",
                                MediaSort::Oldest => "media-sort-oldest",
                                MediaSort::Name => "media-sort-name",
                                MediaSort::NameDesc => "media-sort-name-desc",
                                MediaSort::Kind => "media-sort-type",
                                MediaSort::Size => "media-sort-size",
                            })}</option>
                        }).collect_view()}
                    </select>
                </label>
            </div>
            {crumbs}
            <div class="list-scroll"
                on:dragover=move |e: leptos::ev::DragEvent| { if sh.dragging.get_untracked().is_some() { e.prevent_default() } }>
                <Show when=move || !searching()>
                    {move || sh.creating.get().filter(|p| *p == folder.get()).map(|p| new_folder_row(sh, p, 0))}
                    {move || {
                        let cur = folder.get();
                        let subs = sh.shelf.with(|s| children(s, &cur));
                        subs.into_iter().map(|p| {
                            let n = counts.with(|c| c.get(&p).copied().unwrap_or(0));
                            let target = StoredValue::new(p.clone());
                            folder_row(sh, p, 0, n, None, move || folder.set(target.get_value()))
                        }).collect_view()
                    }}
                </Show>
                {move || {
                    let v = shown.get();
                    if v.is_empty() && loaded.get() {
                        let empty_site = files.with(Vec::is_empty);
                        let has_folders = !searching() && sh.shelf.with(|s| !children(s, &folder.get()).is_empty());
                        return (!has_folders).then(|| view! {
                            <p class="panel-empty muted">{ui.t(if empty_site { "media-none" } else { "media-no-match" })}</p>
                        }).into_any();
                    }
                    view! {
                        <div class="asset-grid" class:is-small=move || thumb.get() < 60
                            style=move || format!("--thumb:{}px", thumb.get())>{v.into_iter().map(tile).collect_view()}</div>
                    }.into_any()
                }}
            </div>
            <footer class="panel-foot">
                <span class="panel-foot-text muted">{move || ui.tx("media-count", &[("count", shown.with(Vec::len).to_string())])}</span>
                <input class="thumb-size" type="range" min=THUMB_MIN max=THUMB_MAX step="4"
                    title=ui.t("media-thumb-size") aria-label=ui.t("media-thumb-size")
                    prop:value=move || thumb.get().to_string()
                    on:input=move |e| {
                        if let Ok(v) = event_target_value(&e).parse::<u32>() {
                            thumb.set(v.clamp(THUMB_MIN, THUMB_MAX));
                            crate::pref_set("media.thumb", &v.to_string());
                        }
                    }/>
                <button class="icon-btn icon-btn--small" title=ui.t("library-new-folder") aria-label=ui.t("library-new-folder")
                    on:click=move |_| { q.set(String::new()); sh.creating.set(Some(folder.get_untracked())) }>{icons::folder_plus()}</button>
                <PlannedButton key="assets-import" step=3 icon=icons::upload/>
            </footer>
        </div>
    }
}

// ------------------------------------------------------------------ composants

/// Composants du site rangés en dossiers, avec leur nombre d'utilisations.
#[component]
pub fn Components() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let sh = Shelves::new(ui, ed, ShelfKind::Components);
    let q = RwSignal::new(String::new());
    let closed = RwSignal::new(Vec::<String>::new());
    // Composants (identifiant, utilisations), par nom.
    let all = Memo::new(move |_| {
        ed.doc.with(|d| {
            let Some(d) = d.as_ref() else {
                return Vec::new();
            };
            let mut uses: BTreeMap<String, usize> = BTreeMap::new();
            for (_, _, body) in d.owners() {
                revel_model::ids::walk(body, &mut |n| {
                    if let Node::Use { component, .. } = n {
                        *uses.entry(component.clone()).or_default() += 1;
                    }
                });
            }
            let mut v: Vec<(String, usize)> = d
                .components
                .iter()
                .map(|c| (c.id.clone(), uses.get(&c.id).copied().unwrap_or(0)))
                .collect();
            v.sort_by_key(|(id, _)| name_of(id).to_lowercase());
            v
        })
    });
    let count_in = move |folder: &str| {
        sh.shelf.with(|s| {
            all.with(|a| {
                a.iter()
                    .filter(|(id, _)| {
                        within(folder_of(s, id), folder) && !folder_of(s, id).is_empty()
                    })
                    .count()
            })
        })
    };
    let row = move |(id, n): (String, usize), depth: usize| {
        let item = StoredValue::new(id.clone());
        let menu = move |e: leptos::ev::MouseEvent| {
            use crate::ctxmenu::open;
            open(ui, &e, sh.move_items(&item.get_value()));
        };
        view! {
            <div class="list-row component-row" draggable="true" data-component=id.clone()
                style=format!("--depth:{depth}")
                title=move || format!("{}\n{}", item.get_value(), ui.tx("components-uses", &[("count", n.to_string())]))
                class:is-dragging=move || sh.dragging.get().as_deref() == Some(item.get_value().as_str())
                on:contextmenu=menu
                on:dragstart=move |e: leptos::ev::DragEvent| {
                    if let Some(dt) = e.data_transfer() {
                        let _ = dt.set_data("text/plain", &item.get_value());
                        dt.set_effect_allowed("move");
                    }
                    sh.dragging.set(Some(item.get_value()));
                }
                on:dragend=move |_| { sh.dragging.set(None); sh.drop_on.set(None); }>
                <span class="list-chevron"></span>
                <span class="list-icon">{icons::component()}</span>
                <span class="list-label">{name_of(&id).to_string()}</span>
                <span class="list-count">{n}</span>
            </div>
        }
    };
    // Arbre : sous-dossiers puis composants de `folder` (récursif).
    fn level(
        sh: Shelves,
        folder: String,
        depth: usize,
        closed: RwSignal<Vec<String>>,
        all: Memo<Vec<(String, usize)>>,
        count_in: impl Fn(&str) -> usize + Copy + Send + Sync + 'static,
        row: impl Fn((String, usize), usize) -> AnyView + Copy + Send + Sync + 'static,
    ) -> AnyView {
        let subs = sh.shelf.with(|s| children(s, &folder));
        let items: Vec<(String, usize)> = sh.shelf.with(|s| {
            all.with(|a| {
                a.iter()
                    .filter(|(id, _)| folder_of(s, id) == folder)
                    .cloned()
                    .collect()
            })
        });
        let creating = sh.creating.get().filter(|p| *p == folder);
        view! {
            {creating.map(|p| new_folder_row(sh, p, depth))}
            {subs.into_iter().map(|p| {
                let key = StoredValue::new(p.clone());
                let is_open = Signal::derive(move || !closed.with(|c| key.with_value(|k| c.contains(k))));
                let toggle = move || closed.update(|c| {
                    let k = key.get_value();
                    if let Some(i) = c.iter().position(|x| *x == k) { c.remove(i); } else { c.push(k); }
                });
                let inner = p.clone();
                view! {
                    {folder_row(sh, p.clone(), depth, count_in(&p), Some(is_open), toggle)}
                    {move || is_open.get().then(|| level(sh, inner.clone(), depth + 1, closed, all, count_in, row))}
                }
            }).collect_view()}
            {items.into_iter().map(|it| row(it, depth)).collect_view()}
        }
        .into_any()
    }
    let row_any = move |it: (String, usize), depth: usize| row(it, depth).into_any();
    let found = move || {
        let ql = q.get().trim().to_lowercase();
        all.with(|a| {
            a.iter()
                .filter(|(id, _)| id.to_lowercase().contains(&ql))
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    let root_drop = sh.drop_zone(String::new());
    view! {
        <div class="panel-list components">
            <Search value=q key="components-search"/>
            <div class="list-scroll"
                class:is-drop=move || sh.drop_on.get().as_deref() == Some("")
                on:dragover=move |e: leptos::ev::DragEvent| root_drop(e, false)
                on:drop=move |e: leptos::ev::DragEvent| root_drop(e, true)>
                {move || if q.with(|q| q.trim().is_empty()) {
                    level(sh, String::new(), 0, closed, all, count_in, row_any)
                } else {
                    found().into_iter().map(|it| row(it, 0)).collect_view().into_any()
                }}
            </div>
            <footer class="panel-foot">
                <span class="panel-foot-text muted">{move || ui.tx("planned-edit", &[("step", "3".to_string())])}</span>
                <button class="icon-btn icon-btn--small" title=ui.t("library-new-folder") aria-label=ui.t("library-new-folder")
                    on:click=move |_| { q.set(String::new()); sh.creating.set(Some(String::new())) }>{icons::folder_plus()}</button>
                <PlannedButton key="components-new" step=3 icon=icons::plus/>
            </footer>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dossiers() {
        let s = Shelf::default();
        let (s, a) = create(&s, "", " Logos ").unwrap();
        assert_eq!(a, "Logos");
        assert!(
            create(&s, "", "logos").is_none(),
            "nom déjà pris (casse ignorée)"
        );
        assert!(create(&s, "", "a/b").is_none() && create(&s, "absent", "x").is_none());
        let (s, b) = create(&s, "Logos", "Clients").unwrap();
        assert_eq!(b, "Logos/Clients");
        let s = put(&s, "/m/a.png", "Logos/Clients");
        let s = put(&s, "/m/b.png", "Logos");
        assert_eq!(children(&s, ""), vec!["Logos"]);
        assert_eq!(children(&s, "Logos"), vec!["Logos/Clients"]);
        // Renommer : sous-dossiers et éléments suivent.
        let (s, r) = rename(&s, "Logos", "Marques").unwrap();
        assert_eq!(r, "Marques");
        assert_eq!(s.folders, vec!["Marques", "Marques/Clients"]);
        assert_eq!(folder_of(&s, "/m/a.png"), "Marques/Clients");
        assert!(rename(&s, "Marques", "Marques").is_none());
        // Supprimer : les éléments remontent d'un niveau.
        let t = remove(&s, "Marques/Clients");
        assert_eq!(folder_of(&t, "/m/a.png"), "Marques");
        let t = remove(&s, "Marques");
        assert!(t.folders.is_empty() && t.items.is_empty());
        // Élément rangé dans un dossier disparu : à la racine.
        let mut u = put(&s, "/m/c.png", "Marques");
        u.folders.clear();
        assert_eq!(folder_of(&u, "/m/c.png"), "");
        assert!(!within("Marques2", "Marques") && within("Marques/x", "Marques"));
    }
}
